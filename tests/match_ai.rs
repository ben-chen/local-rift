use local_rift::{
    api::Session,
    cards::{CardKind, Domain, cards},
    game::{Decision, Difficulty, Game, Phase, Unit},
    gear::GearState,
    hidden::HiddenCard,
    match_ai::{MatchPolicy, Scouting, recommend},
    match_series::{MatchSeries, Registration},
};
use serde_json::{Value, json};

fn registration() -> Registration {
    Registration::from_preset(5).unwrap()
}
fn setup() -> MatchSeries {
    let mut s = MatchSeries::new([registration(), registration()], 0).unwrap();
    s.choose_battlefield(0, 13).unwrap();
    s.choose_battlefield(1, 13).unwrap();
    s.choose_play_order(0, 0).unwrap();
    s
}
fn sideboarding() -> MatchSeries {
    let mut s = setup();
    s.concede_game(1).unwrap();
    s
}
fn game() -> Game {
    let mut g = Game::new(5, 5, 444, Difficulty::Tactical);
    g.players[1].champion = None;
    g
}
fn unit(id: u32, card_id: usize, lane: Option<usize>, token: bool) -> Unit {
    Unit {
        id,
        card_id,
        lane,
        ready: true,
        damage: 0,
        buff: if token { 2 } else { 0 },
        temporary_buff: 0,
        token,
        protected: false,
        stunned: false,
        banish_on_death: false,
    }
}
fn plan(s: &MatchSeries, scout: &Scouting, level: Difficulty) -> local_rift::match_ai::MatchPlan {
    recommend(
        &s.observe(0).unwrap(),
        0,
        scout,
        level,
        MatchPolicy::Adaptive,
    )
    .unwrap()
}

#[test]
fn private_hand_deck_runes_and_facedown_identity_do_not_change_scouting_or_plan() {
    let mut a = game();
    a.players[1].hidden.push(HiddenCard {
        id: 77,
        card_id: 1,
        lane: 0,
        hidden_turn: 1,
        revealed: false,
    });
    let mut b = a.clone();
    b.players[1].hand = vec![424, 421, 11, 1];
    b.players[1].deck.reverse();
    b.players[1].rune_deck.reverse();
    b.players[1].hidden[0].card_id = 424;
    let mut sa = Scouting::default();
    let mut sb = Scouting::default();
    sa.observe_game(&a, 0).unwrap();
    sb.observe_game(&b, 0).unwrap();
    assert_eq!(sa, sb);
    let s = sideboarding();
    assert_eq!(
        plan(&s, &sa, Difficulty::Strategic),
        plan(&s, &sb, Difficulty::Strategic)
    );
    assert_eq!(sa.to_json()["seen_cards"], json!([]));
    // CR 422.4 explicitly makes these identities public at game end.
    a.phase = Phase::Finished;
    b.phase = Phase::Finished;
    sa.observe_game(&a, 0).unwrap();
    sb.observe_game(&b, 0).unwrap();
    assert_eq!(sa.to_json()["seen_cards"], json!([1]));
    assert_eq!(sb.to_json()["seen_cards"], json!([424]));
}

#[test]
fn explicit_reveals_are_remembered_after_permissions_expire() {
    let mut g = game();
    g.players[1].hidden.push(HiddenCard {
        id: 77,
        card_id: 11,
        lane: 0,
        hidden_turn: 1,
        revealed: false,
    });
    let mut scout = Scouting::default();
    g.hidden_view[0] = true;
    g.decision = Some(Decision::RevealHand {
        player: 0,
        opponent: 1,
        cards: vec![424, 421],
    });
    scout.observe_game(&g, 0).unwrap();
    g.hidden_view[0] = false;
    g.decision = None;
    g.players[1].hidden.clear();
    scout.observe_game(&g, 0).unwrap();
    assert_eq!(scout.to_json()["seen_cards"], json!([11, 421, 424]));
    g.players[1].hand = vec![369];
    g.decision = Some(Decision::Sabotage {
        player: 0,
        opponent: 1,
    });
    scout.observe_game(&g, 0).unwrap();
    assert_eq!(scout.to_json()["seen_cards"], json!([11, 369, 421, 424]));
}

#[test]
fn gust_evidence_uses_current_might_at_a_field_and_invalid_observation_is_atomic() {
    let mut g = game();
    let mut u = unit(1, 0, None, false); // Irelia has 4 printed Might.
    u.temporary_buff = -2;
    g.players[1].units.push(u);
    let mut scout = Scouting::default();
    scout.observe_game(&g, 0).unwrap();
    assert_eq!(scout.to_json()["small_field_peak"], 0);
    g.players[1].units[0].lane = Some(0);
    g.players[1].units[0].temporary_buff = 2;
    scout.observe_game(&g, 0).unwrap();
    assert_eq!(scout.to_json()["small_field_peak"], 0);
    g.players[1].units[0].temporary_buff = -2;
    scout.observe_game(&g, 0).unwrap();
    assert_eq!(scout.to_json()["small_field_peak"], 1);
    let before = scout.clone();
    assert!(scout.observe_game(&g, 2).is_err());
    assert_eq!(before, scout);
    g.players[1].units[0].buff = u64::MAX;
    g.players[1].discard.push(424);
    assert!(scout.observe_game(&g, 0).is_err());
    assert_eq!(before, scout);
}

#[test]
fn repeated_snapshots_are_idempotent_and_tokens_do_not_invent_deck_cards() {
    let mut g = game();
    g.players[1].units.push(unit(1, 424, Some(0), true));
    g.players[1].gear.push(GearState {
        id: 2,
        card_id: 14,
        lane: None,
        attached_to: None,
        ready: true,
        temporary: false,
        token: true,
    });
    let mut scout = Scouting::default();
    scout.observe_game(&g, 0).unwrap();
    let initial = scout.clone();
    for _ in 0..30 {
        scout.observe_game(&g, 0).unwrap();
    }
    assert_eq!(initial, scout);
    assert_eq!(scout.to_json()["seen_cards"], json!([]));
    assert_eq!(scout.to_json()["gear_peak"], 0);
    assert_eq!(scout.to_json()["small_field_peak"], 1);
    let p = plan(&sideboarding(), &scout, Difficulty::Strategic);
    assert!(p.swaps.iter().any(|s| s.inside == 11));
    assert!(p.swaps.iter().all(|s| ![424, 369].contains(&s.inside)));
}

#[test]
fn revealed_gear_prompts_legal_swaps_without_changing_registration_or_core_counts() {
    let mut s = sideboarding();
    let before = s.observe(0).unwrap()["registration"].clone();
    let mut g = game();
    g.players[1].discard = cards()
        .iter()
        .filter(|c| c.kind == CardKind::Gear)
        .take(3)
        .map(|c| c.id)
        .collect();
    let mut scout = Scouting::default();
    scout.observe_game(&g, 0).unwrap();
    for level in [Difficulty::Tactical, Difficulty::Strategic] {
        let p = plan(&s, &scout, level);
        assert!(
            p.swaps.iter().any(|s| [424, 369].contains(&s.inside)),
            "{p:?}"
        );
        assert!(p.swaps.len() <= if level == Difficulty::Tactical { 2 } else { 4 });
        assert_eq!(p.deck.card_count(), 40);
        let catalog = cards();
        for kind in [CardKind::Unit, CardKind::Gear] {
            let count = |rows: &[(usize, u8)]| {
                rows.iter()
                    .filter(|(id, _)| catalog[*id].kind == kind)
                    .map(|(_, n)| u32::from(*n))
                    .sum::<u32>()
            };
            assert!(count(&p.deck.cards) >= count(&registration().deck.cards));
        }
        let mut preview = s.clone();
        preview
            .submit_sideboard(0, p.deck.cards, p.deck.chosen_champion)
            .unwrap();
        assert_eq!(preview.observe(0).unwrap()["registration"], before);
    }
    let p = plan(&s, &scout, Difficulty::Strategic);
    s.submit_sideboard(0, p.deck.cards, p.deck.chosen_champion)
        .unwrap();
    assert!(plan(&s, &scout, Difficulty::Strategic).swaps.is_empty());
}

#[test]
fn no_evidence_easy_and_keep_policy_do_not_sideboard_and_fury_is_required_for_decree() {
    let s = sideboarding();
    assert!(
        plan(&s, &Scouting::default(), Difficulty::Strategic)
            .swaps
            .is_empty()
    );
    let mut g = game();
    g.players[1].discard = cards()
        .iter()
        .filter(|c| c.kind == CardKind::Spell && !c.domains.contains(&Domain::Fury))
        .take(7)
        .map(|c| c.id)
        .collect();
    let mut scout = Scouting::default();
    scout.observe_game(&g, 0).unwrap();
    let p = plan(&s, &scout, Difficulty::Strategic);
    assert!(p.swaps.iter().any(|s| s.inside == 1));
    assert!(p.swaps.iter().all(|s| s.inside != 421));
    assert!(plan(&s, &scout, Difficulty::Easy).swaps.is_empty());
    assert!(
        recommend(
            &s.observe(0).unwrap(),
            0,
            &scout,
            Difficulty::Strategic,
            MatchPolicy::Keep
        )
        .unwrap()
        .swaps
        .is_empty()
    );
    g.players[1].discard = cards()
        .iter()
        .filter(|c| c.kind == CardKind::Unit && c.domains.contains(&Domain::Fury))
        .take(5)
        .map(|c| c.id)
        .collect();
    let mut fury = Scouting::default();
    fury.observe_game(&g, 0).unwrap();
    assert!(
        plan(&s, &fury, Difficulty::Strategic)
            .swaps
            .iter()
            .any(|s| s.inside == 421)
    );
}

#[test]
fn first_game_stays_registered_and_opponent_pool_or_commit_does_not_change_plans() {
    let r = registration();
    let mut other = r.clone();
    other
        .sideboard
        .iter_mut()
        .find(|(id, _)| *id == 424)
        .unwrap()
        .1 -= 1;
    other.sideboard.push((0, 1));
    other.validate().unwrap();
    let mut a = MatchSeries::new([r.clone(), r.clone()], 0).unwrap();
    let mut b = MatchSeries::new([r.clone(), other], 0).unwrap();
    a.choose_battlefield(1, 0).unwrap();
    b.choose_battlefield(1, 14).unwrap();
    let pa = plan(&a, &Scouting::default(), Difficulty::Strategic);
    assert_eq!(pa, plan(&b, &Scouting::default(), Difficulty::Strategic));
    assert!(pa.swaps.is_empty());
    assert_eq!(pa.deck, r.deck);
    assert_eq!(pa.battlefield, Some(13));
    assert_eq!(
        plan(&sideboarding(), &Scouting::default(), Difficulty::Strategic).battlefield,
        Some(0)
    );
}

#[test]
fn irelia_keeps_its_engine_and_does_not_accumulate_more_cuts_in_game_three() {
    let mut s = sideboarding();
    let mut g = game();
    g.players[1].discard = cards().iter().map(|c| c.id).collect();
    for id in 1..5 {
        g.players[1].units.push(unit(id, 424, Some(0), true));
    }
    let mut scout = Scouting::default();
    scout.observe_game(&g, 0).unwrap();
    let original = registration();
    for game_number in [2, 3] {
        let plan = plan(&s, &scout, Difficulty::Strategic);
        let count = |rows: &[(usize, u8)], id| {
            rows.iter()
                .find(|(card, _)| *card == id)
                .map_or(0, |(_, n)| *n)
        };
        for id in [5, 6, 7, 14, 17, 18, 425, 430] {
            assert_eq!(count(&original.deck.cards, id), count(&plan.deck.cards, id));
        }
        let distance: usize = plan
            .deck
            .cards
            .iter()
            .map(|(id, n)| usize::from(n.saturating_sub(count(&original.deck.cards, *id))))
            .sum();
        assert!(distance <= 4, "game {game_number}: {distance}");
        assert!(distance > 0);
        s.submit_sideboard(0, plan.deck.cards, plan.deck.chosen_champion)
            .unwrap();
        let r = registration();
        s.submit_sideboard(1, r.deck.cards, r.deck.chosen_champion)
            .unwrap();
        if game_number == 2 {
            s.choose_battlefield(0, 0).unwrap();
            s.choose_battlefield(1, 0).unwrap();
            s.choose_play_order(1, 1).unwrap();
            s.concede_game(0).unwrap();
        }
    }
}

fn request(session: &mut Session, mut v: Value) -> Value {
    if v["command"] != "series_suggest" {
        v["revision"] = json!(session.revision());
    }
    let response = session.handle(v.clone());
    assert_eq!(response["ok"], true, "{v}: {response}");
    response["result"].clone()
}
fn begin(session: &mut Session, level: &str, seed: u64) {
    request(
        session,
        json!({"command":"series_begin","registrations":[{"preset":5},{"preset":5}],"designated_player":0,"difficulty":level,"seed":seed}),
    );
}

#[test]
fn api_suggestions_are_read_only_mutations_require_revision_and_new_matches_clear_memory() {
    let mut s = Session::new(Default::default()).unwrap();
    begin(&mut s, "tactical", 5);
    let before = s.revision();
    let idea = request(&mut s, json!({"command":"series_suggest","player":0}));
    assert_eq!(before, s.revision());
    assert_eq!(idea["plan"]["battlefield"], 13);
    assert_eq!(idea["scouting"]["seen_cards"], json!([]));
    for v in [
        json!({"command":"series_step_ai","player":0,"revision":before-1}),
        json!({"command":"series_step_ai","player":2,"revision":before}),
        json!({"command":"series_step_ai","player":0,"revision":before,"policy":"cheat"}),
    ] {
        assert_eq!(s.handle(v)["ok"], false);
        assert_eq!(s.revision(), before);
    }
    for p in [0, 1, 0] {
        request(&mut s, json!({"command":"series_step_ai","player":p}));
    }
    let observed = request(&mut s, json!({"command":"series_suggest","player":0}));
    assert_eq!(observed["scouting"]["seen_cards"], json!([0]));
    let rev = s.revision();
    assert_eq!(
        s.handle(json!({"command":"series_step_ai","player":0,"revision":rev}))["ok"],
        false
    );
    assert_eq!(rev, s.revision());
    begin(&mut s, "tactical", 6);
    assert_eq!(
        request(&mut s, json!({"command":"series_suggest","player":0}))["scouting"]["seen_cards"],
        json!([])
    );
}

#[test]
fn adaptive_api_finishes_real_matches_and_preserves_each_private_registration() {
    for (i, level) in ["easy", "tactical", "strategic"].iter().enumerate() {
        let mut s = Session::new(Default::default()).unwrap();
        begin(&mut s, level, 81200 + i as u64);
        let original = [
            s.observe(0).unwrap()["series"]["registration"].clone(),
            s.observe(1).unwrap()["series"]["registration"].clone(),
        ];
        let mut complete = false;
        let mut swaps = 0;
        for _ in 0..6000 {
            let view = s.observe(0).unwrap();
            let state = &view["series"];
            match state["phase"].as_str().unwrap() {
                "complete" => {
                    assert_eq!(state["wins"][state["winner"].as_u64().unwrap() as usize], 2);
                    complete = true;
                    break;
                }
                "sideboarding" | "battlefields" => {
                    let ready = if state["phase"] == "sideboarding" {
                        "sideboard_ready"
                    } else {
                        "battlefield_committed"
                    };
                    for p in 0..2 {
                        if state[ready][p] == false {
                            let idea =
                                request(&mut s, json!({"command":"series_suggest","player":p}));
                            swaps += idea["plan"]["swaps"].as_array().unwrap().len();
                            request(&mut s, json!({"command":"series_step_ai","player":p}));
                            assert_eq!(
                                s.observe(p).unwrap()["series"]["registration"],
                                original[p]
                            );
                        }
                    }
                }
                "play_order" => {
                    request(
                        &mut s,
                        json!({"command":"series_step_ai","player":state["designated_player"]}),
                    );
                }
                "game" if view["phase"] == "mulligan" => {
                    let first = view["first_player"].as_u64().unwrap() as usize;
                    for p in [first, 1 - first] {
                        request(
                            &mut s,
                            json!({"command":"mulligan","player":p,"indices":[]}),
                        );
                    }
                    request(&mut s, json!({"command":"start"}));
                }
                "game" => {
                    request(
                        &mut s,
                        json!({"command":"step_ai","player":view["priority"]}),
                    );
                }
                phase => panic!("Unexpected phase {phase}"),
            }
        }
        assert!(complete, "{level}");
        if *level != "easy" {
            assert!(swaps > 0, "{level}: did not exercise adaptive swaps");
        }
        println!("{level}: real match completed; {swaps} suggested swaps");
    }
}
