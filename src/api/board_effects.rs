//! Public protocol regressions for global instructions and physical effect sources.
use super::*;
use crate::game::Unit;
use crate::gear::GearState;

fn position() -> Session {
    let mut session = Session::new(ResetOptions {
        decks: [3, 3],
        ..Default::default()
    })
    .unwrap();
    session.start(0).unwrap();
    session.game.battlefields = [3, 3];
    session.game.battlefield_owner = [None; 2];
    for player in &mut session.game.players {
        player.hand.clear();
        player.units.clear();
        player.gear.clear();
        player.discard.clear();
        player.champion = None;
        player.runes = [0; 6];
        player.exhausted_runes = [0; 6];
        player.energy_pool = 50;
        player.energy = 50;
        player.power_pool = [20; 6];
        player.xp = 0;
    }
    session
}

fn unit(id: u32, card_id: usize, lane: Option<usize>) -> Unit {
    Unit {
        id,
        card_id,
        lane,
        ready: true,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    }
}

fn gear(id: u32, card_id: usize, bearer: Option<u32>) -> GearState {
    GearState {
        id,
        card_id,
        attached_to: bearer,
        lane: None,
        ready: true,
        temporary: false,
        token: false,
    }
}

fn apply(session: &mut Session, action: Action) {
    let response = session.handle(json!({"command":"apply", "revision":session.revision(),
        "player":session.game.priority(), "action":action_to_json(&action)}));
    assert_eq!(response["ok"], true, "{response}");
}

fn play(id: usize, session: &mut Session) {
    session.game.players[0].hand = vec![id];
    apply(
        session,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        },
    );
}

fn pass_until(session: &mut Session, done: impl Fn(&Session) -> bool) {
    for _ in 0..40 {
        if done(session) {
            return;
        }
        assert!(
            session.game.decision.is_none(),
            "{:?}",
            session.game.decision
        );
        apply(session, Action::Pass);
    }
    panic!("Response window did not reach the expected state");
}

fn views(session: &Session) -> [Value; 2] {
    [session.observe(0).unwrap(), session.observe(1).unwrap()]
}

#[test]
fn catalog_preserves_condition_strings_and_exposes_thresholds_and_recipients() {
    let catalog = catalog();
    for (id, condition, threshold, recipients) in [
        (321, "buffed", None, "self_unit"),
        (322, "buffed", None, "self_unit"),
        (323, "other_friendly_here", None, "self_unit"),
        (324, "always", None, "friendly_token_units"),
        (325, "controller_runes_at_least", Some(8), "self_unit"),
        (326, "controller_xp_at_least", Some(11), "self_unit"),
        (330, "controller_xp_at_least", Some(3), "attached_bearer"),
        (331, "always", None, "friendly_units"),
    ] {
        let rule = &catalog["cards"][id]["passive_rules"][0];
        assert_eq!(rule["condition"], condition);
        assert_eq!(rule["condition_threshold"], json!(threshold));
        assert_eq!(rule["recipients"], recipients);
    }
    for card in catalog["cards"].as_array().unwrap() {
        for rule in card["passive_rules"].as_array().unwrap() {
            assert!(
                rule.as_object()
                    .unwrap()
                    .contains_key("condition_threshold")
            );
        }
    }
    for id in 308..314 {
        assert_eq!(
            catalog["cards"][id]["play_targets"],
            json!({"kind":null,"maximum":0,"required":0})
        );
    }
    assert_eq!(
        catalog["cards"][304]["passive_rules"][0]["condition"],
        "attacking_with_another"
    );
}

#[test]
fn both_seats_observe_live_rune_xp_and_distinct_attached_sword_contributions() {
    let mut session = position();
    session.game.players[0].units = vec![unit(900, 325, None), unit(901, 326, Some(0))];
    session.game.players[0].gear = vec![gear(902, 330, Some(900)), gear(903, 330, Some(900))];
    session.game.players[0].runes[1] = 8;
    session.game.players[0].exhausted_runes[1] = 1;
    session.game.players[0].xp = 3;
    for view in views(&session) {
        let yi = &view["players"][0]["units"][0];
        assert_eq!(yi["might"], 12);
        for id in [902, 903] {
            assert!(yi["passive_contributions"].as_array().unwrap().iter().any(
                |effect| effect["source"] == json!({"type":"gear","player":0,"gear_id":id})
                    && effect["active"] == true
                    && effect["effect"] == json!({"type":"might","amount":1,"amount_exact":"1"})
            ));
        }
        assert_eq!(view["players"][0]["units"][1]["might"], 6);
    }
    session.game.players[0].runes[1] = 7;
    session.game.players[0].xp = 2;
    for view in views(&session) {
        assert_eq!(view["players"][0]["units"][0]["might"], 6);
        assert_eq!(view["players"][0]["units"][0]["keywords"]["hunt"], 0);
    }
    session.game.players[0].xp = 11;
    session.game.players[0].gear[0].attached_to = None;
    for view in views(&session) {
        assert_eq!(view["players"][0]["units"][0]["might"], 6);
        assert_eq!(view["players"][0]["units"][1]["might"], 10);
    }
}

#[test]
fn token_aura_and_another_here_observations_respect_physical_identity_and_ownership() {
    let mut session = position();
    let mut recruit = unit(902, 87, Some(1));
    recruit.token = true;
    recruit.buff = 1;
    session.game.players[0].units = vec![unit(900, 323, None), unit(901, 324, Some(0)), recruit];
    session.game.players[1].units = vec![unit(903, 323, None)];
    for view in views(&session) {
        assert_eq!(view["players"][0]["units"][0]["might"], 2);
        assert_eq!(view["players"][0]["units"][2]["might"], 2);
        assert_eq!(view["players"][1]["units"][0]["might"], 2);
        let effects = view["players"][0]["units"][2]["passive_contributions"]
            .as_array()
            .unwrap();
        assert!(
            effects
                .iter()
                .any(|e| e["source"]["unit_id"] == 901 && e["active"] == true)
        );
    }
    session.game.players[0].units[1].lane = None;
    session.game.players[0].units[1].ready = false;
    session.game.players[0].units[1].stunned = true;
    for view in views(&session) {
        assert_eq!(view["players"][0]["units"][0]["might"], 3);
        assert_eq!(view["players"][1]["units"][0]["might"], 2);
    }
}

#[test]
fn hunt_observations_capture_one_sum_and_a_truthful_bearer_source() {
    for (card_id, copies, token, expected) in
        [(22, 2, false, 4), (153, 1, false, 4), (87, 2, true, 2)]
    {
        let mut session = position();
        let mut bearer = unit(900, card_id, None);
        bearer.token = token;
        bearer.buff = u64::from(token);
        session.game.players[0].units.push(bearer);
        for index in 0..copies {
            session.game.players[0]
                .gear
                .push(gear(901 + index, 328, Some(900)));
        }
        assert_eq!(
            session.observe(0).unwrap()["players"][0]["units"][0]["keywords"]["hunt"],
            expected
        );
        apply(
            &mut session,
            Action::Move {
                unit_id: 900,
                lane: Some(0),
            },
        );
        pass_until(&mut session, |s| {
            s.game
                .chain
                .iter()
                .any(|c| matches!(c.ability, Some(ChainAbility::Hunt { .. })))
        });
        let before = views(&session);
        for view in &before {
            let abilities: Vec<_> = view["chain"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| c["ability"]["type"] == "hunt")
                .collect();
            assert_eq!(abilities.len(), 1);
            let item = abilities[0];
            assert_eq!(
                item["source"],
                json!({"type":"unit","player":0,"unit_id":900})
            );
            assert_eq!(item["ability"]["source"], item["source"]);
            assert_eq!(item["ability"]["amount"], expected);
            if token {
                assert_eq!(item["ability"]["source_name"], "Recruit token");
            }
        }
        // Once finalized, the public ability survives absence of its physical source.
        session.game.players[0].units.clear();
        session.game.players[0].gear.clear();
        for view in views(&session) {
            assert_eq!(view["chain"], before[0]["chain"]);
        }
        pass_until(&mut session, |s| s.game.chain.is_empty());
        assert_eq!(session.game.players[0].xp, expected);
    }
}

#[test]
fn token_entry_instruction_is_public_and_cannot_be_overridden_by_a_client() {
    for (creator, ready) in [(318, false), (319, true)] {
        let mut session = position();
        // The live Accelerate grant exposes a genuine token-finalization choice.
        session.game.players[0].units.push(unit(990, 265, None));
        play(creator, &mut session);
        pass_until(&mut session, |s| {
            matches!(
                s.game.decision,
                Some(Decision::TokenPlay { .. } | Decision::TokenPlacement { .. })
            )
        });
        let before = views(&session);
        for view in &before {
            let pending = &view["pending_token_plays"][0];
            assert_eq!(pending["entry_ready"], ready);
            assert_eq!(pending["destination"], json!({"mode":"fixed","lane":null}));
        }
        for action in [
            json!({"type":"place_token","lane":null,"entry_ready":!ready}),
            json!({"type":"finalize_token_play","lane":null,"accelerate":false,"entry_ready":!ready}),
        ] {
            let response = session.handle(
                json!({"command":"apply","player":0,"revision":session.revision(),"action":action}),
            );
            assert_eq!(response["ok"], false, "{response}");
            assert_eq!(views(&session), before);
        }
        apply(
            &mut session,
            Action::FinalizeTokenPlay {
                lane: None,
                accelerate: false,
            },
        );
        pass_until(&mut session, |s| !s.game.in_window());
        for view in views(&session) {
            let token = view["players"][0]["units"]
                .as_array()
                .unwrap()
                .iter()
                .find(|u| u["token"] == true)
                .unwrap();
            assert_eq!(token["name"], "Mech token");
            assert_eq!(token["ready"], ready);
            assert_eq!(token["might"], 3);
            assert_eq!(token["lane"], Value::Null);
        }
    }
}

#[test]
fn partial_channel_quotes_are_read_only_and_fallback_draws_only_once() {
    for (id, remaining, expected_channels, expected_draw) in [
        (309, 0, 0, 1),
        (309, 1, 1, 0),
        (310, 0, 0, 1),
        (310, 1, 1, 1),
        (310, 2, 2, 0),
    ] {
        let mut session = position();
        session.game.players[0].rune_deck = vec![Domain::Body; remaining];
        session.game.players[0].deck = vec![0, 1, 8];
        session.game.players[0].hand = vec![id];
        let action = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        };
        let before = views(&session);
        for command in ["validate", "play_choices", "cost_plan", "quote_payment"] {
            let response = session.handle(json!({"command":command,"revision":session.revision(),"player":0,"action":action_to_json(&action)}));
            assert_eq!(response["ok"], true, "{response}");
            assert_eq!(views(&session), before);
        }
        apply(&mut session, action);
        pass_until(&mut session, |s| !s.game.in_window());
        assert_eq!(session.game.players[0].runes[3], expected_channels);
        assert_eq!(
            session.game.players[0].exhausted_runes[3],
            expected_channels
        );
        assert_eq!(session.game.players[0].hand.len(), expected_draw);
        assert_eq!(
            session.game.players[0].energy,
            50 - u16::from(session.game.catalog[id].cost)
        );
        assert_eq!(session.game.players[0].discard, vec![id]);
    }
}

#[test]
fn ruination_keeps_its_physical_parent_during_replacement_and_rejects_invalid_choice() {
    let mut session = position();
    session.game.players[1].units = vec![unit(900, 2, None), unit(901, 69, Some(1))];
    session.game.players[1].gear.push(gear(902, 21, None));
    play(308, &mut session);
    pass_until(&mut session, |s| {
        matches!(s.game.decision, Some(Decision::DeathReplacement { .. }))
    });
    let before = views(&session);
    for view in &before {
        assert_eq!(
            view["chain"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| c["card_id"] == 308 && c["ability"].is_null())
                .count(),
            1
        );
        assert!(
            !view["players"][0]["discard"]
                .as_array()
                .unwrap()
                .contains(&json!(308))
        );
        assert_eq!(view["decision"]["player"], 1);
    }
    let invalid = Action::ChooseDeathReplacement {
        unit_id: 9999,
        gear_id: Some(902),
    };
    assert!(
        session
            .apply_action(1, session.revision(), invalid)
            .is_err()
    );
    assert_eq!(views(&session), before);
    apply(
        &mut session,
        Action::ChooseDeathReplacement {
            unit_id: 900,
            gear_id: Some(902),
        },
    );
    pass_until(&mut session, |s| !s.game.in_window());
    assert_eq!(session.game.players[1].units.len(), 1);
    assert_eq!(session.game.players[1].units[0].id, 900);
    assert!(session.game.players[1].discard.contains(&69));
    assert!(session.game.players[1].discard.contains(&21));
    assert_eq!(session.game.players[0].discard, vec![308]);
}

#[test]
fn untargeted_globals_reject_supplied_targets_in_previews_validation_and_application() {
    for id in 308..314 {
        let mut session = position();
        session.game.players[0].hand = vec![id];
        session.game.players[1].units.push(unit(900, 2, Some(0)));
        let before = views(&session);
        for target in [
            json!({"type":"unit","player":1,"unit_id":900}),
            json!({"type":"battlefield","lane":0}),
        ] {
            let action = json!({"type":"play_card_targets","hand_index":0,"targets":[target]});
            for command in [
                "play_choices",
                "cost_plan",
                "quote_payment",
                "validate",
                "apply",
            ] {
                let response = session.handle(json!({"command":command,"revision":session.revision(),"player":0,"action":action}));
                assert_eq!(response["ok"], false, "card {id} {command}: {response}");
                assert_eq!(views(&session), before);
            }
        }
    }
}

#[test]
fn granted_temporary_is_observed_as_active_after_equip_and_expires_before_hold() {
    let mut session = position();
    session.game.players[0].units.push(unit(900, 69, Some(0)));
    session.game.battlefield_owner[0] = Some(0);
    session.game.players[0].gear.push(gear(901, 3, None));
    session.game.players[0].hand = vec![66];
    apply(
        &mut session,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Gear {
                player: 0,
                gear_id: 901,
            }],
        },
    );
    pass_until(&mut session, |s| !s.game.in_window());
    for view in views(&session) {
        assert_eq!(
            view["players"][0]["gear"][0]["temporary_status"],
            json!({"intrinsic":false,"granted":true,"active":true})
        );
    }
    apply(
        &mut session,
        Action::Equip {
            gear_id: 901,
            target: Target::Unit {
                player: 0,
                unit_id: 900,
            },
        },
    );
    pass_until(&mut session, |s| !s.game.in_window());
    for view in views(&session) {
        let observed = &view["players"][0]["gear"][0];
        assert_eq!(observed["temporary"], true);
        assert_eq!(observed["attached_to"], 900);
        assert_eq!(
            observed["temporary_status"],
            json!({"intrinsic":false,"granted":true,"active":true})
        );
    }
    apply(&mut session, Action::EndTurn);
    pass_until(&mut session, |s| !s.game.in_window());
    apply(&mut session, Action::EndTurn);
    assert_eq!(session.game.players[0].score, 0);
    assert!(session.game.chain.iter().any(|item| matches!(
        item.ability,
        Some(ChainAbility::Temporary {
            source: crate::empower::BoardObject::Gear {
                player: 0,
                gear_id: 901
            },
            ..
        })
    )));
    assert_eq!(session.game.players[0].gear.len(), 1);
    pass_until(&mut session, |s| !s.game.in_window());
    assert!(session.game.players[0].gear.is_empty());
    assert!(session.game.players[0].discard.contains(&3));
    assert_eq!(session.game.players[0].score, 1);
    for view in views(&session) {
        assert!(view["players"][0]["gear"].as_array().unwrap().is_empty());
        assert_eq!(view["players"][0]["units"][0]["might"], 8);
    }
}
