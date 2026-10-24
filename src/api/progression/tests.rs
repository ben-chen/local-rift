//! Independent protocol checks; fixtures establish positions, and actual
//! requests perform plays, moves, priority passes and payments.
use super::*;
use crate::game::Unit;

fn position() -> Session {
    let mut session = Session::new(ResetOptions {
        decks: [1, 1],
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
        player.energy_pool = 50;
        player.energy = 50;
        player.power_pool = [20; 6];
        player.xp = 0;
    }
    session
}

fn unit(id: u32, card_id: usize) -> Unit {
    Unit {
        id,
        card_id,
        lane: None,
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

fn views(session: &Session) -> [Value; 2] {
    [session.observe(0).unwrap(), session.observe(1).unwrap()]
}

fn request(session: &mut Session, command: &str, action: &Action) -> Value {
    session.handle(json!({"command":command,"revision":session.revision(),
        "player":session.game.priority(),"action":action_to_json(action)}))
}

fn apply(session: &mut Session, action: Action) {
    let response = request(session, "apply", &action);
    assert_eq!(response["ok"], true, "{response}");
}

fn pass_until(session: &mut Session, done: impl Fn(&Session) -> bool) {
    for _ in 0..60 {
        if done(session) {
            return;
        }
        if let Some(Decision::OrderTriggers { options, .. }) = &session.game.decision {
            let trigger_id = options[0].id;
            apply(session, Action::OrderTrigger { trigger_id });
        } else {
            assert!(
                session.game.decision.is_none(),
                "{:?}",
                session.game.decision
            );
            apply(session, Action::Pass);
        }
    }
    panic!("Priority did not reach the expected state");
}

fn finish(session: &mut Session) {
    pass_until(session, |session| !session.game.in_window());
}

fn card_action(targets: Vec<Target>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets,
    }
}

#[test]
fn catalog_explains_entry_and_nonstacking_xp_discounts_without_replacing_old_facts() {
    let catalog = catalog();
    for id in [334, 340] {
        assert_eq!(
            catalog["cards"][id]["entry_ready"],
            json!({"condition":"controller_xp_at_least","threshold":3})
        );
    }
    assert_eq!(
        catalog["cards"][319]["entry_ready"],
        json!({"condition":"always"})
    );
    assert_eq!(catalog["cards"][308]["entry_ready"], Value::Null);
    assert_eq!(catalog["cards"][336]["energy"], 5);
    assert_eq!(
        catalog["cards"][336]["xp_energy_discounts"],
        json!([{"minimum_xp":6,"energy":2},{"minimum_xp":11,"energy":4}])
    );
    assert_eq!(catalog["cards"][335]["xp_energy_discounts"], json!([]));
    for rule in catalog["cards"][342]["passive_rules"].as_array().unwrap() {
        assert_eq!(rule["condition"], "controller_gained_xp_this_turn");
        assert_eq!(rule["condition_threshold"], Value::Null);
    }
    assert_eq!(
        catalog["cards"][325]["passive_rules"][0]["condition_threshold"],
        8
    );
    for card in catalog["cards"].as_array().unwrap() {
        assert!(card.as_object().unwrap().contains_key("entry_ready"));
        assert!(card["xp_energy_discounts"].is_array());
    }
}

#[test]
fn xp_observations_preserve_exact_integers_for_both_seats_and_reject_client_overrides() {
    let mut session = position();
    for amount in [
        0_u64,
        255,
        256,
        (1_u64 << 53) - 1,
        1_u64 << 53,
        (1_u64 << 53) + 1,
        u64::MAX,
    ] {
        session.game.players[0].xp = amount;
        session.game.players[1].xp = amount;
        let revision = session.revision();
        for view in views(&session) {
            for owner in 0..2 {
                assert_eq!(view["players"][owner]["xp"].as_u64(), Some(amount));
                assert_eq!(view["players"][owner]["xp_exact"], amount.to_string());
                assert_eq!(view["players"][owner]["xp_gained_this_turn"], false);
            }
            let encoded = serde_json::to_string(&view).unwrap();
            let decoded: Value = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, view);
        }
        assert_eq!(session.revision(), revision);
    }
    let before = views(&session);
    let revision = session.revision();
    for command in ["validate", "apply"] {
        let rejected = session.handle(json!({"command":command,"player":0,"revision":revision,
            "action":{"type":"pass","xp_exact":"0","xp_gained_this_turn":true}}));
        assert_eq!(rejected["ok"], false);
        assert_eq!(rejected["error"]["code"], "invalid_request");
        assert_eq!(views(&session), before);
        assert_eq!(session.revision(), revision);
    }
}

#[test]
fn actual_hunt_crosses_255_and_history_applies_to_newtfish_before_and_after_entry() {
    let mut session = position();
    session.game.players[0].xp = 254;
    session.game.players[0].units = vec![unit(900, 22), unit(901, 342)];
    apply(
        &mut session,
        Action::Move {
            unit_id: 900,
            lane: Some(0),
        },
    );
    finish(&mut session);
    assert_eq!(session.game.players[0].xp, 256);
    session.game.players[0].hand = vec![342];
    apply(&mut session, card_action(vec![]));
    finish(&mut session);
    // A controlled balance change tests event history. No included action spends
    // XP in this slice, so this is deliberately not described as a legal cost.
    session.game.players[0].xp = 0;
    for view in views(&session) {
        assert_eq!(view["players"][0]["xp_gained_this_turn"], true);
        assert_eq!(view["players"][1]["xp_gained_this_turn"], false);
        let fish: Vec<_> = view["players"][0]["units"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|unit| unit["card_id"] == 342)
            .collect();
        assert_eq!(fish.len(), 2);
        for unit in fish {
            assert_eq!(unit["might"], 5);
            assert_eq!(unit["keywords"]["ganking"], true);
        }
    }
    apply(&mut session, Action::EndTurn);
    finish(&mut session);
    for view in views(&session) {
        assert_eq!(view["players"][0]["xp_gained_this_turn"], false);
        assert_eq!(view["players"][1]["xp_gained_this_turn"], false);
        for unit in view["players"][0]["units"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|unit| unit["card_id"] == 342)
        {
            assert_eq!(unit["might"], 4);
            assert_eq!(unit["keywords"]["ganking"], false);
        }
    }
}

#[test]
fn overflow_is_a_distinct_atomic_error_while_exact_u64_maximum_can_resolve() {
    for (initial, overflow) in [(u64::MAX - 1, true), (u64::MAX - 2, false)] {
        let mut session = position();
        session.game.players[0].xp = initial;
        session.game.players[0].units.push(unit(900, 22));
        apply(
            &mut session,
            Action::Move {
                unit_id: 900,
                lane: Some(0),
            },
        );
        pass_until(&mut session, |session| {
            session
                .game
                .chain
                .iter()
                .any(|item| matches!(item.ability, Some(ChainAbility::Hunt { amount: 2, .. })))
        });
        if !overflow {
            finish(&mut session);
            for view in views(&session) {
                assert_eq!(view["players"][0]["xp_exact"], u64::MAX.to_string());
                assert_eq!(view["players"][0]["xp_gained_this_turn"], true);
            }
            continue;
        }
        let mut rejected = false;
        for _ in 0..4 {
            let before = views(&session);
            let revision = session.revision();
            let validation = request(&mut session, "validate", &Action::Pass);
            assert_eq!(views(&session), before);
            assert_eq!(session.revision(), revision);
            if validation["ok"] == false {
                assert_eq!(validation["error"]["code"], "engine_capacity");
                let response = request(&mut session, "apply", &Action::Pass);
                assert_eq!(response["ok"], false);
                assert_eq!(response["error"]["code"], "engine_capacity");
                assert_eq!(views(&session), before);
                assert_eq!(session.revision(), revision);
                let response = session.handle(json!({"command":"step_ai",
                    "revision":revision,"player":session.game.priority()}));
                assert_eq!(response["ok"], false);
                assert_eq!(response["error"]["code"], "engine_capacity");
                assert_eq!(views(&session), before);
                assert_eq!(session.revision(), revision);
                rejected = true;
                break;
            }
            apply(&mut session, Action::Pass);
        }
        assert!(rejected, "The overflowing gain must fail explicitly");
        assert_eq!(session.game.players[0].xp, initial);
        assert!(!session.game.gained_xp_this_turn(0));
    }
}

#[test]
fn conditional_ready_entry_and_live_might_are_observed_separately() {
    for (card, xp, ready, might) in [
        (334, 2, false, 5),
        (334, 3, true, 5),
        (340, 2, false, 3),
        (340, 3, true, 4),
    ] {
        let mut session = position();
        session.game.players[0].xp = xp;
        session.game.players[0].hand = vec![card];
        apply(&mut session, card_action(vec![]));
        finish(&mut session);
        for view in views(&session) {
            assert_eq!(view["players"][0]["units"][0]["ready"], ready);
            assert_eq!(view["players"][0]["units"][0]["might"], might);
        }
        session.game.players[0].xp = 0; // Controlled threshold-loss fixture.
        assert_eq!(
            session.observe(0).unwrap()["players"][0]["units"][0]["ready"],
            ready
        );
    }
}

#[test]
fn concentrate_quotes_are_pure_and_staged_payment_keeps_one_selected_discount() {
    for (xp, energy) in [(5, 5_u16), (6, 3), (10, 3), (11, 1)] {
        let mut session = position();
        session.game.players[0].xp = xp;
        session.game.players[0].hand = vec![336];
        let action = card_action(vec![]);
        let before = views(&session);
        let revision = session.revision();
        for command in ["validate", "play_choices", "cost_plan", "quote_payment"] {
            assert_eq!(request(&mut session, command, &action)["ok"], true);
            assert_eq!(views(&session), before);
            assert_eq!(session.revision(), revision);
        }
        assert_eq!(
            session
                .cost_plan(0, revision, action.clone())
                .unwrap()
                .energy,
            u32::from(energy)
        );
        apply(
            &mut session,
            Action::BeginPayment {
                action: Box::new(action),
            },
        );
        assert_eq!(
            session.game.payment_plan_view(0).unwrap().energy,
            u32::from(energy)
        );
        // This controlled fixture checks the fixed-cost contract; ordinary
        // resource generation cannot currently change XP during a payment.
        session.game.players[0].xp = 0;
        assert_eq!(
            session.game.payment_plan_view(0).unwrap().energy,
            u32::from(energy)
        );
        apply(&mut session, Action::CompletePayment);
        finish(&mut session);
        assert_eq!(session.game.players[0].energy, 50 - energy);
        assert_eq!(session.game.players[0].hand.len(), 2);
        assert!(session.game.players[0].discard.contains(&336));
    }
}

#[test]
fn wuju_completion_trigger_survives_a_later_fixture_balance_change() {
    for (xp, draws) in [(5, 0_usize), (6, 1)] {
        let mut session = position();
        session.game.players[0].xp = xp;
        session.game.players[0].hand = vec![341];
        let deck_count = session.game.players[0].deck.len();
        apply(&mut session, card_action(vec![]));
        if draws == 1 {
            assert!(!session.game.chain.is_empty());
        }
        session.game.players[0].xp = 0;
        finish(&mut session);
        assert_eq!(session.game.players[0].hand.len(), draws);
        assert_eq!(session.game.players[0].deck.len(), deck_count - draws);
        assert_eq!(
            session.observe(1).unwrap()["players"][0]["hand"],
            Value::Null
        );
        assert_eq!(
            session.observe(1).unwrap()["players"][0]["hand_count"],
            draws
        );
    }
}

#[test]
fn combat_experience_uses_resolution_xp_and_replaces_the_lower_bonus() {
    for (announced_xp, resolving_xp, bonus) in [(5, 6, 3), (6, 5, 1)] {
        let mut session = position();
        session.game.players[1].units.push(unit(900, 69));
        session.game.players[0].xp = announced_xp;
        session.game.players[0].hand = vec![335];
        apply(
            &mut session,
            card_action(vec![Target::Unit {
                player: 1,
                unit_id: 900,
            }]),
        );
        assert!(!session.game.chain.is_empty());
        // Controlled balance change isolates live resolution timing. It is not
        // a claimed response or an XP-spending action in this first slice.
        session.game.players[0].xp = resolving_xp;
        finish(&mut session);
        for view in views(&session) {
            assert_eq!(view["players"][1]["units"][0]["might"], 8 + bonus);
        }
    }
}

#[test]
fn friendly_unit_xp_gain_counts_every_public_unit_beyond_255() {
    let mut session = position();
    session.game.players[0].units = (1000..1260).map(|id| unit(id, 69)).collect();
    session.game.players[0].hand = vec![155];
    apply(&mut session, card_action(vec![]));
    finish(&mut session);
    assert_eq!(session.game.players[0].units.len(), 261);
    for view in views(&session) {
        assert_eq!(view["players"][0]["xp"], 261);
        assert_eq!(view["players"][0]["xp_exact"], "261");
        assert_eq!(view["players"][0]["xp_gained_this_turn"], true);
    }
}
