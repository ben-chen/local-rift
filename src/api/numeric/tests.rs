//! Protocol-level numeric checks use controlled public board positions and then
//! actual JSON commands. Extreme fixtures test host arithmetic, not deck reachability.
use super::*;
use crate::game::Unit;

fn position() -> Session {
    position_with_decks([1, 1])
}
fn position_with_decks(decks: [usize; 2]) -> Session {
    let mut session = Session::new(ResetOptions {
        decks,
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
        player.hidden.clear();
        player.discard.clear();
        player.champion = None;
        player.runes = [0; 6];
        player.exhausted_runes = [0; 6];
        player.energy_pool = 30;
        player.energy = 30;
        player.power_pool = [0; 6];
        player.any_power = 30;
        player.xp = 0;
    }
    session.game.legend[0].ready = false;
    session.game.legend[1].ready = false;
    session
}
fn unit(id: u32, might: i64, lane: Option<usize>) -> Unit {
    Unit {
        id,
        card_id: 273,
        lane,
        ready: true,
        damage: 0,
        buff: 0,
        temporary_buff: might,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    }
}
fn views(session: &Session) -> [Value; 2] {
    [session.observe(0).unwrap(), session.observe(1).unwrap()]
}
fn request(session: &mut Session, command: &str, action: Value) -> Value {
    session.handle(json!({"command":command,"revision":session.revision(),
        "player":session.game.priority(),"action":action}))
}
fn apply(session: &mut Session, action: Value) {
    let reply = request(session, "apply", action);
    assert_eq!(reply["ok"], true, "{reply}");
}
fn submit(session: &mut Session, action: Action) {
    apply(session, action_to_json(&action));
}
fn combat(might: i64, opposing: i64) -> Session {
    // Irelia has no derived lone-defender bonus, so the combat fixture's
    // controlled Might is also its exact damage total.
    let mut session = position_with_decks([0, 0]);
    session.game.players[0].units = vec![unit(100, might, None)];
    session.game.players[1].units = vec![unit(200, opposing, Some(0))];
    session.game.battlefield_owner[0] = Some(1);
    submit(
        &mut session,
        Action::Move {
            unit_id: 100,
            lane: Some(0),
        },
    );
    submit(&mut session, Action::Pass);
    submit(&mut session, Action::Pass);
    assert!(matches!(
        session.game.decision,
        Some(Decision::CombatDamage { player: 0, .. })
    ));
    session
}
fn reject_unchanged(session: &mut Session, action: Value, code: &str) {
    let before = views(session);
    let state = format!("{:?}", session.game);
    let revision = session.revision();
    for command in ["validate", "apply"] {
        let reply = request(session, command, action.clone());
        assert_eq!(reply["ok"], false, "{reply}");
        assert_eq!(reply["error"]["code"], code, "{reply}");
        assert_eq!(views(session), before);
        assert_eq!(format!("{:?}", session.game), state);
        assert_eq!(session.revision(), revision);
    }
}

#[test]
fn damage_parser_accepts_exact_unsigned_boundaries_without_float_conversion() {
    for amount in [
        0_u64,
        1,
        255,
        256,
        32768,
        65536,
        MAX_EXACT_JSON_INTEGER,
        MAX_EXACT_JSON_INTEGER + 1,
        (1_u64 << 53) + 1,
        i64::MAX as u64,
        u64::MAX,
    ] {
        assert_eq!(parse_damage(&json!(amount)).unwrap(), amount);
        assert_eq!(parse_damage(&json!(amount.to_string())).unwrap(), amount);
    }
}

#[test]
fn damage_parser_rejects_noncanonical_and_out_of_range_values() {
    for value in [
        json!(""),
        json!("00"),
        json!("01"),
        json!("+1"),
        json!("-0"),
        json!("-1"),
        json!(" 1"),
        json!("1 "),
        json!("1\n"),
        json!("1.0"),
        json!("1e3"),
        json!("18446744073709551616"),
        json!("１"),
        json!(-1),
        json!(1.5),
        json!(1.0),
        json!(true),
        Value::Null,
        json!([]),
    ] {
        assert_eq!(
            parse_damage(&value).unwrap_err().code,
            "invalid_request",
            "{value}"
        );
    }
}

#[test]
fn suggested_assignments_roundtrip_large_values_and_keep_small_numeric_shape() {
    for amount in [
        1_u64,
        65536,
        MAX_EXACT_JSON_INTEGER,
        MAX_EXACT_JSON_INTEGER + 1,
        u64::MAX,
    ] {
        let action = Action::AssignDamage {
            assignments: vec![(7, amount)],
        };
        let wire = action_to_json(&action);
        assert_eq!(parse_action(&wire).unwrap(), action);
        if amount <= MAX_EXACT_JSON_INTEGER {
            assert_eq!(wire["assignments"][0][1].as_u64(), Some(amount));
        } else {
            assert_eq!(wire["assignments"][0][1], amount.to_string());
        }
        assert_eq!(wire["assignments"][0][0], 7);
    }
    assert!(exact_assignments(None).is_null());
    assert_eq!(
        exact_assignments(Some(&[(7, u64::MAX)])),
        json!([[7, u64::MAX.to_string()]])
    );
}

#[test]
fn both_seats_observe_signed_might_and_exact_dynamic_fields() {
    let mut session = position();
    session.game.players[0].units = vec![unit(100, (1_i64 << 53) + 1, None), unit(101, -2, None)];
    session.game.players[0].units[0].damage = (1_u64 << 53) + 1;
    for view in views(&session) {
        let units = view["players"][0]["units"].as_array().unwrap();
        assert_eq!(units[0]["might"].as_u64(), Some((1_u64 << 53) + 1));
        for field in ["might", "signed_might", "damage", "buff", "temporary_buff"] {
            assert_eq!(
                units[0][format!("{field}_exact")],
                units[0][field].to_string()
            );
        }
        assert_eq!(units[1]["might"], 0);
        assert_eq!(units[1]["might_exact"], "0");
        assert_eq!(units[1]["signed_might"], -2);
        assert_eq!(units[1]["signed_might_exact"], "-2");
        assert_eq!(units[1]["temporary_buff_exact"], "-2");
        for keyword in ["assault", "shield"] {
            assert_eq!(
                units[0]["keywords"][format!("{keyword}_exact")],
                units[0]["keywords"][keyword].to_string()
            );
        }
        let effect = &units[0]["passive_contributions"][0]["effect"];
        assert_eq!(effect["amount_exact"], effect["amount"].to_string());
    }
    // Exact runtime contributions are additive; printed catalog descriptors stay unchanged.
    let printed = &catalog()["cards"][273]["passive_rules"][0]["effect"];
    assert_eq!(printed["amount"], 7);
    assert!(printed.get("amount_exact").is_none());
}

#[test]
fn actual_combat_uses_full_damage_above_65535_and_javascript_safe_integer_range() {
    for amount in [65536_u64, (1_u64 << 53) + 1, i64::MAX as u64] {
        let mut session = combat(amount as i64, amount as i64 - 1);
        for view in views(&session) {
            assert_eq!(view["decision"]["total"].as_u64(), Some(amount));
            assert_eq!(view["decision"]["total_exact"], amount.to_string());
        }
        let suggestion = session
            .suggestions(0)
            .unwrap()
            .into_iter()
            .find(|s| matches!(s.action, Action::AssignDamage { .. }))
            .unwrap();
        let wire = action_to_json(&suggestion.action);
        assert_eq!(parse_action(&wire).unwrap(), suggestion.action);
        // Explicit exact-string input works even within the ordinary number range.
        apply(
            &mut session,
            json!({"type":"assign_damage","assignments":[[200,amount.to_string()]]}),
        );
        for view in views(&session) {
            assert_eq!(
                view["players"][0]["submitted_combat_damage"],
                json!([[200, amount]])
            );
            assert_eq!(
                view["players"][0]["submitted_combat_damage_exact"],
                json!([[200, amount.to_string()]])
            );
        }
        apply(
            &mut session,
            json!({"type":"assign_damage","assignments":[[100,amount - 1]]}),
        );
        assert!(
            session.game.players[1].units.is_empty(),
            "Full damage must defeat the defender"
        );
        assert!(session.game.players[0].units.iter().any(|u| u.id == 100));
    }
}

#[test]
fn invalid_allocation_values_and_totals_leave_both_views_and_revision_unchanged() {
    let mut session = combat(65536, 65535);
    for action in [
        json!({"type":"assign_damage","assignments":[[200,"65536 "]]}),
        json!({"type":"assign_damage","assignments":[[200,"18446744073709551616"]]}),
        json!({"type":"assign_damage","assignments":[[200,-1]]}),
        json!({"type":"assign_damage","assignments":[[200,65536]],"total_exact":"65536"}),
    ] {
        reject_unchanged(&mut session, action, "invalid_request");
    }
    for assignments in [
        json!([[200, "65535"]]),
        json!([[200, "65537"]]),
        json!([[200, "18446744073709551615"], [200, "18446744073709551615"]]),
        json!([[200, 32768], [200, 32768]]),
        json!([[999, 65536]]),
    ] {
        reject_unchanged(
            &mut session,
            json!({"type":"assign_damage","assignments":assignments}),
            "illegal_action",
        );
    }
}

#[test]
fn overflow_while_resolving_a_real_spell_is_capacity_error_and_rolls_back_the_pass() {
    let mut session = position();
    session.game.players[0].units = vec![unit(100, i64::MAX, None)];
    session.game.players[0].hand = vec![7]; // Discipline: +2 Might, then Draw1.
    submit(
        &mut session,
        Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: Some(Target::Unit {
                player: 0,
                unit_id: 100,
            }),
        },
    );
    submit(&mut session, Action::Pass);
    let before = views(&session);
    let state = format!("{:?}", session.game);
    let revision = session.revision();
    let suggestions =
        session.handle(json!({"command":"suggest", "player":session.game.priority()}));
    assert_eq!(suggestions["ok"], false, "{suggestions}");
    assert_eq!(suggestions["error"]["code"], "engine_capacity");
    assert_eq!(views(&session), before);
    assert_eq!(format!("{:?}", session.game), state);
    assert_eq!(session.revision(), revision);
    reject_unchanged(
        &mut session,
        action_to_json(&Action::Pass),
        "engine_capacity",
    );
    assert_eq!(session.game.chain.last().unwrap().card_id, 7);
}

#[test]
fn observation_rejects_unrepresentable_state_without_inventing_a_clamped_number() {
    let mut session = position();
    session.game.players[0].units = vec![unit(100, i64::MAX, None)];
    session.game.players[0].units[0].buff = 1;
    let before = format!("{:?}", session.game);
    for player in 0..2 {
        assert_eq!(session.observe(player).unwrap_err().code, "engine_capacity");
    }
    assert_eq!(format!("{:?}", session.game), before);
}

#[test]
fn highest_might_discount_keeps_its_exact_value_without_overflowing_the_payable_cost() {
    let mut session = position_with_decks([3, 1]); // Kai'Sa supports the Fury spell and Mind unit.
    let might = (1_i64 << 53) + 1;
    session.game.players[0].units = vec![unit(100, might, None)];
    session.game.players[0].units[0].card_id = 69; // Mega-Mech's printed8 plus the controlled modifier.
    session.game.players[0].units[0].temporary_buff = might - 8;
    session.game.players[1].units = vec![unit(200, 10, Some(0))];
    session.game.players[0].hand = vec![226]; // Sky Splitter: discount by highest own Might.
    let action = action_to_json(&Action::PlayCard {
        hand_index: 0,
        lane: None,
        target: Some(Target::Unit {
            player: 1,
            unit_id: 200,
        }),
    });
    let before = views(&session);
    let reply = request(&mut session, "cost_plan", action.clone());
    assert_eq!(reply["ok"], true, "{reply}");
    let plan = &reply["result"];
    assert_eq!(plan["energy"], 0);
    let modifier = plan["modifiers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["source"]["card_id"] == 226)
        .unwrap();
    assert_eq!(modifier["energy"].as_u64(), Some(might as u64));
    assert_eq!(modifier["energy_exact"], might.to_string());
    assert_eq!(views(&session), before);
    let quote = request(&mut session, "quote_payment", action.clone());
    assert_eq!(quote["ok"], true, "{quote}");
    assert_eq!(views(&session), before);
    let energy = session.game.players[0].energy;
    apply(
        &mut session,
        json!({"type":"with_power_payment","action":action,
        "payment":quote["result"]["suggested"]}),
    );
    assert_eq!(session.game.players[0].energy, energy);
    assert_eq!(session.game.chain.last().unwrap().card_id, 226);
}
