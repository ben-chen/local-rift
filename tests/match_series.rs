use local_rift::{
    api::Session,
    cards::presets,
    deck::CustomDeck,
    game::Difficulty,
    match_series::{MatchSeries, Registration, SeriesPhase},
};
use serde_json::{Value, json};
fn registration() -> Registration {
    Registration::from_preset(5).unwrap()
}
fn series() -> MatchSeries {
    MatchSeries::new([registration(), registration()], 0).unwrap()
}
fn choose(s: &mut MatchSeries, fields: [usize; 2], first: usize) {
    s.choose_battlefield(0, fields[0]).unwrap();
    s.choose_battlefield(1, fields[1]).unwrap();
    let chooser = s.summary()["designated_player"].as_u64().unwrap() as usize;
    s.choose_play_order(chooser, first).unwrap();
}
fn keep(s: &mut MatchSeries) {
    for p in 0..2 {
        let d = CustomDeck::from_json(&s.observe(p).unwrap()["deck"]).unwrap();
        s.submit_sideboard(p, d.cards, d.chosen_champion).unwrap();
    }
}
fn request(s: &mut Session, mut v: Value) -> Value {
    if v["revision"].is_null() {
        v["revision"] = json!(s.revision());
    }
    let r = s.handle(v.clone());
    assert_eq!(r["ok"], true, "{v}: {r}");
    r["result"].clone()
}
fn begin(s: &mut Session) {
    request(
        s,
        json!({"command":"series_begin","registrations":[{"preset":5},{"preset":5}],"designated_player":0,"seed":100}),
    );
}
fn setup(s: &mut Session) {
    request(
        s,
        json!({"command":"series_battlefield","player":0,"battlefield":13}),
    );
    request(
        s,
        json!({"command":"series_battlefield","player":1,"battlefield":0}),
    );
    request(
        s,
        json!({"command":"series_play_order","player":0,"first_player":1}),
    );
}
fn invalid(s: &mut Session, mut v: Value) {
    let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
    let rev = s.revision();
    if v["revision"].is_null() {
        v["revision"] = json!(rev);
    }
    let r = s.handle(v.clone());
    assert_eq!(r["ok"], false, "unexpected success: {v}");
    assert_eq!(rev, s.revision());
    assert_eq!(before, [s.observe(0).unwrap(), s.observe(1).unwrap()]);
}
fn swap(cards: &mut Vec<(usize, u8)>, out: usize, inside: usize) {
    let row = cards.iter_mut().find(|(id, _)| *id == out).unwrap();
    row.1 -= 1;
    cards.retain(|(_, n)| *n > 0);
    if let Some(row) = cards.iter_mut().find(|(id, _)| *id == inside) {
        row.1 += 1;
    } else {
        cards.push((inside, 1));
    }
}

#[test]
fn registration_enforces_exact_main_sideboard_combined_copies_and_three_legal_fields() {
    let r = registration();
    r.validate().unwrap();
    assert_eq!(Registration::from_json(&r.to_json()).unwrap(), r);
    assert!(Registration::from_preset(3).is_err()); // Historical Kai'Sa has a banned field.
    let mut bad = r.clone();
    bad.deck.cards.push((422, 1));
    bad.deck.validate().unwrap();
    assert!(bad.validate().is_err());
    let mut bad = r.clone();
    bad.sideboard.push((432, 2));
    assert!(bad.validate().is_err());
    let mut bad = r.clone();
    bad.sideboard = vec![(432, 2)];
    assert!(bad.validate().is_err());
    let mut bad = r.clone();
    bad.sideboard = vec![(424, 1), (424, 1)];
    assert!(bad.validate().is_err());
    for side in [vec![(99999, 1)], vec![(424, 0)], vec![(56, 1)]] {
        let mut bad = r.clone();
        bad.sideboard = side;
        assert!(bad.validate().is_err());
    }
    let mut bad = r.clone();
    bad.battlefields = [13, 13, 0];
    assert!(bad.validate().is_err());
    let mut bad = r.clone();
    bad.battlefields = [13, 0, 99999];
    assert!(bad.validate().is_err());
    let mut empty = r;
    empty.sideboard.clear();
    empty.validate().unwrap();
}
#[test]
fn setup_is_private_then_revealed_and_does_not_allow_first_game_sideboarding() {
    let mut s = series();
    assert_eq!(s.phase(), SeriesPhase::Battlefields);
    assert!(
        s.submit_sideboard(0, presets()[5].cards.clone(), 0)
            .is_err()
    );
    s.choose_battlefield(1, 14).unwrap();
    assert_eq!(
        s.observe(0).unwrap()["players"][1]["selected_battlefield"],
        Value::Null
    );
    assert_eq!(
        s.observe(1).unwrap()["players"][1]["selected_battlefield"],
        14
    );
    assert!(s.choose_battlefield(1, 0).is_err());
    assert!(s.choose_play_order(0, 0).is_err());
    s.choose_battlefield(0, 13).unwrap();
    assert_eq!(
        s.observe(0).unwrap()["players"][1]["selected_battlefield"],
        14
    );
    assert!(s.choose_play_order(1, 0).is_err());
    s.choose_play_order(0, 1).unwrap();
    let g = s.new_game(1, Difficulty::Easy).unwrap();
    assert_eq!(g.first_player, 1);
    assert_eq!(g.battlefields, [13, 14]);
    assert!(
        g.players
            .iter()
            .all(|p| p.hand.len() == 4 && p.deck.len() == 35 && p.champion == Some(0))
    );
}
#[test]
fn sideboarding_conserves_the_pool_and_reveals_new_champion_before_fields() {
    let mut r = registration();
    r.sideboard.retain(|(id, _)| *id != 1);
    r.sideboard.push((271, 1));
    r.validate().unwrap();
    let mut s = MatchSeries::new([r.clone(), registration()], 0).unwrap();
    choose(&mut s, [13, 0], 0);
    s.concede_game(1).unwrap();
    let before = s.observe(0).unwrap();
    let mut wrong = r.deck.cards.clone();
    swap(&mut wrong, 432, 422);
    assert!(s.submit_sideboard(0, wrong, 0).is_err());
    assert_eq!(before, s.observe(0).unwrap());
    let mut changed = r.deck.cards.clone();
    swap(&mut changed, 0, 271);
    s.submit_sideboard(0, changed, 271).unwrap();
    assert_eq!(s.observe(0).unwrap()["players"][0]["chosen_champion"], 271);
    assert_eq!(s.observe(1).unwrap()["players"][0]["chosen_champion"], 0);
    s.submit_sideboard(1, presets()[5].cards.clone(), 0)
        .unwrap();
    assert_eq!(s.phase(), SeriesPhase::Battlefields);
    assert_eq!(s.observe(1).unwrap()["players"][0]["chosen_champion"], 271);
    assert!(s.choose_battlefield(0, 13).is_err());
    assert!(s.choose_battlefield(1, 0).is_err());
    choose(&mut s, [0, 14], 1);
    let g = s.new_game(2, Difficulty::Easy).unwrap();
    assert_eq!(g.players[0].champion, Some(271));
    assert_eq!(g.first_player, 1);
}
#[test]
fn two_wins_finish_and_three_decisive_games_use_distinct_fields() {
    let mut s = series();
    choose(&mut s, [13, 0], 0);
    s.concede_game(1).unwrap();
    assert_eq!(s.summary()["wins"], json!([1, 0]));
    assert_eq!(s.summary()["designated_player"], 1);
    keep(&mut s);
    choose(&mut s, [0, 14], 1);
    s.concede_game(0).unwrap();
    assert_eq!(s.summary()["wins"], json!([1, 1]));
    assert_eq!(s.summary()["designated_player"], 0);
    keep(&mut s);
    choose(&mut s, [14, 13], 0);
    s.concede_game(1).unwrap();
    assert_eq!(s.phase(), SeriesPhase::Complete);
    assert_eq!(s.summary()["winner"], 0);
    assert_eq!(s.summary()["wins"], json!([2, 1]));
    assert!(s.concede_game(0).is_err());
    assert!(s.new_game(4, Difficulty::Easy).is_err());
}
#[test]
fn drawn_game_keeps_decks_fields_and_starting_player_without_sideboarding() {
    let mut s = series();
    choose(&mut s, [13, 0], 1);
    s.offer_draw(0, false, true).unwrap();
    assert_eq!(s.game_number(), 1);
    s.offer_draw(1, false, true).unwrap();
    assert_eq!(s.game_number(), 2);
    assert_eq!(s.phase(), SeriesPhase::Game);
    assert_eq!(s.summary()["wins"], json!([0, 0]));
    assert!(
        s.submit_sideboard(0, presets()[5].cards.clone(), 0)
            .is_err()
    );
    assert!(s.choose_battlefield(0, 14).is_err());
    assert!(s.choose_play_order(0, 0).is_err());
    let g = s.new_game(3, Difficulty::Easy).unwrap();
    assert_eq!(g.first_player, 1);
    assert_eq!(g.battlefields, [13, 0]);
    assert_eq!(
        s.observe(0).unwrap()["players"][0]["used_battlefields"],
        json!([])
    );
    s.concede_game(1).unwrap();
    assert_eq!(
        s.observe(0).unwrap()["players"][0]["used_battlefields"],
        json!([13])
    );
}
#[test]
fn concession_before_reveal_does_not_leak_or_consume_private_field() {
    let mut s = series();
    s.choose_battlefield(0, 14).unwrap();
    s.concede_match(1).unwrap();
    assert_eq!(
        s.observe(1).unwrap()["players"][0]["selected_battlefield"],
        Value::Null
    );
    let mut s = series();
    s.choose_battlefield(0, 14).unwrap();
    s.concede_game(1).unwrap();
    assert_eq!(
        s.summary()["history"][0]["battlefields"],
        json!([null, null])
    );
    assert_eq!(
        s.observe(1).unwrap()["players"][0]["used_battlefields"],
        json!([])
    );
    keep(&mut s);
    s.choose_battlefield(0, 14).unwrap();
}
#[test]
fn match_draw_requires_both_players_and_does_not_fabricate_game_wins() {
    let mut s = series();
    s.offer_draw(0, true, true).unwrap();
    assert_ne!(s.phase(), SeriesPhase::Complete);
    s.offer_draw(0, true, false).unwrap();
    s.offer_draw(1, true, true).unwrap();
    assert_ne!(s.phase(), SeriesPhase::Complete);
    s.offer_draw(0, true, true).unwrap();
    assert_eq!(s.phase(), SeriesPhase::Complete);
    assert_eq!(s.summary()["winner"], Value::Null);
    assert_eq!(s.summary()["wins"], json!([0, 0]));
}
#[test]
fn api_locks_setup_and_mulligan_order_and_rejects_stale_or_invalid_choices_atomically() {
    let mut s = Session::default();
    begin(&mut s);
    assert!(s.observe(0).unwrap().get("players").is_none());
    assert!(s.suggestions(0).unwrap().is_empty());
    invalid(&mut s, json!({"command":"start"}));
    invalid(&mut s, json!({"command":"step_ai","player":0}));
    invalid(
        &mut s,
        json!({"command":"series_battlefield","player":0,"battlefield":13,"revision":0}),
    );
    invalid(
        &mut s,
        json!({"command":"series_battlefield","player":2,"battlefield":13}),
    );
    invalid(
        &mut s,
        json!({"command":"series_battlefield","player":0,"battlefield":1}),
    );
    setup(&mut s);
    invalid(&mut s, json!({"command":"start"}));
    invalid(
        &mut s,
        json!({"command":"mulligan","player":0,"indices":[]}),
    );
    request(
        &mut s,
        json!({"command":"mulligan","player":1,"indices":[]}),
    );
    request(
        &mut s,
        json!({"command":"mulligan","player":0,"indices":[]}),
    );
    request(&mut s, json!({"command":"start"}));
    let obs = s.observe(0).unwrap();
    assert_eq!(obs["phase"], "playing");
    assert_eq!(obs["first_player"], 1);
    assert_eq!(obs["players"][1]["hand"], Value::Null);
    request(
        &mut s,
        json!({"command":"series_concede","player":0,"scope":"game"}),
    );
    assert_eq!(s.observe(0).unwrap()["series"]["phase"], "sideboarding");
    invalid(
        &mut s,
        json!({"command":"apply","player":1,"action":{"type":"pass"}}),
    );
    invalid(
        &mut s,
        json!({"command":"series_sideboard","player":0,"cards":presets()[5].cards,"chosen_champion":0,"runes":[0,0,0,0,12,0]}),
    );
}
#[test]
fn api_draw_restarts_with_identical_configuration_and_offers_clear_on_game_actions() {
    let mut s = Session::default();
    begin(&mut s);
    setup(&mut s);
    request(
        &mut s,
        json!({"command":"series_draw","player":0,"scope":"game","accept":true}),
    );
    request(
        &mut s,
        json!({"command":"mulligan","player":1,"indices":[]}),
    );
    assert_eq!(
        s.observe(0).unwrap()["series"]["draw_game"],
        json!([false, false])
    );
    request(
        &mut s,
        json!({"command":"series_draw","player":0,"scope":"game","accept":true}),
    );
    request(
        &mut s,
        json!({"command":"series_draw","player":1,"scope":"game","accept":true}),
    );
    let v = s.observe(0).unwrap();
    assert_eq!(v["series"]["game_number"], 2);
    assert_eq!(v["battlefields"], json!([13, 0]));
    assert_eq!(v["first_player"], 1);
    assert_eq!(v["mulligans_used"], json!([false, false]));
    request(&mut s, json!({"command":"reset","decks":[0,1]}));
    assert!(s.observe(0).unwrap().get("series").is_none());
    request(&mut s, json!({"command":"start"}));
}
#[test]
fn api_opponent_pool_and_sideboard_swaps_remain_private() {
    let mut a = Session::default();
    let mut b = Session::default();
    let mut other = registration();
    other.sideboard.retain(|(id, _)| *id != 1);
    other.sideboard.push((422, 1));
    for (s, reg) in [(&mut a, registration()), (&mut b, other)] {
        request(
            s,
            json!({"command":"series_begin","registrations":[{"preset":5},reg.to_json()],"designated_player":0}),
        );
    }
    assert_eq!(a.observe(0).unwrap(), b.observe(0).unwrap());
    for s in [&mut a, &mut b] {
        setup(s);
        request(
            s,
            json!({"command":"series_concede","player":0,"scope":"game"}),
        );
    }
    let plain = presets()[5].cards.clone();
    let mut swapped = plain.clone();
    swap(&mut swapped, 432, 422);
    request(
        &mut a,
        json!({"command":"series_sideboard","player":1,"cards":plain,"chosen_champion":0}),
    );
    request(
        &mut b,
        json!({"command":"series_sideboard","player":1,"cards":swapped,"chosen_champion":0}),
    );
    assert_eq!(a.observe(0).unwrap(), b.observe(0).unwrap());
}
#[test]
fn registered_matches_complete_with_real_ai_games_and_between_game_swaps() {
    let mut completed = 0;
    let mut games = 0;
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for opponent in [1, 4, 5] {
            for seat in 0..2 {
                for first in 0..2 {
                    let rival = if opponent == 5 {
                        registration()
                    } else {
                        let mut deck = CustomDeck::from_preset(opponent).unwrap();
                        deck.battlefield = 1;
                        Registration {
                            deck,
                            sideboard: vec![],
                            battlefields: [1, 2, 3],
                        }
                    };
                    let regs = if seat == 0 {
                        [registration(), rival]
                    } else {
                        [rival, registration()]
                    };
                    let mut s = MatchSeries::new(regs, first).unwrap();
                    while s.phase() != SeriesPhase::Complete {
                        if s.phase() == SeriesPhase::Sideboarding {
                            for p in 0..2 {
                                let mut deck =
                                    CustomDeck::from_json(&s.observe(p).unwrap()["deck"]).unwrap();
                                if p == seat && s.game_number() == 2 {
                                    swap(&mut deck.cards, 432, 424);
                                }
                                s.submit_sideboard(p, deck.cards, deck.chosen_champion)
                                    .unwrap();
                            }
                        }
                        let fields = [0, 1].map(|p| {
                            s.observe(p).unwrap()["available_battlefields"][0]
                                .as_u64()
                                .unwrap() as usize
                        });
                        choose(&mut s, fields, first);
                        let mut g = s
                            .new_game(
                                700000
                                    + (level * 100 + opponent * 4 + seat * 2 + first) as u64
                                    + s.game_number(),
                                difficulty,
                            )
                            .unwrap();
                        g.start_match().unwrap();
                        for _ in 0..2600 {
                            if g.winner.is_some() {
                                break;
                            }
                            assert!(g.step_ai().unwrap());
                        }
                        assert!(
                            g.winner.is_some(),
                            "unfinished {level}/{opponent}/{seat}/{first}"
                        );
                        s.finish_completed_game(&g).unwrap();
                        games += 1;
                    }
                    assert!(
                        s.summary()["wins"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|v| v == 2)
                    );
                    completed += 1;
                }
            }
        }
    }
    assert_eq!(completed, 36);
    assert!((72..=108).contains(&games));
    println!("Completed {completed} full best-of-three matches across {games} actual games.");
}
