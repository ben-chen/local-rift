//! Protocol timing, physical source identity and private draw observations.
use super::test_support::{apply, request, views};
use super::*;
use crate::game::test_support::{decline_legend_action, position, ready_unit};

fn session(actor: usize) -> Session {
    let mut s = Session::new(ResetOptions::default()).unwrap();
    s.game = position(actor, 0, 454002);
    let unit_id = ready_unit(&mut s.game, actor, 71);
    s.game.players[actor].hand = vec![454, 8];
    apply(
        &mut s,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        },
    );
    apply(
        &mut s,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: actor,
                unit_id,
            }],
        },
    );
    while let Some(action) = decline_legend_action(&s.game) {
        apply(&mut s, action);
    }
    s
}

fn pay(s: &Session) -> Action {
    let Some(Decision::TriggerPayment { choice_id, .. }) = s.game.decision else {
        panic!("missing payment")
    };
    Action::ResolveTriggerPayment {
        choice_id,
        pay: true,
    }
}

#[test]
fn irelia_wheel_api_exposes_physical_cost_and_separate_private_draw() {
    for actor in 0..2 {
        let mut s = session(actor);
        let gear = s.game.players[actor].gear[0].id;
        let observation = s.observe(actor).unwrap();
        assert_eq!(observation["decision"]["timing"], "finalization");
        assert_eq!(
            observation["decision"]["object_cost"],
            json!({"type":"exhaust_gear",
            "player":actor,"gear_id":gear,"paid_before_responses":true,"is_target":false})
        );
        let action = pay(&s);
        let before = views(&s);
        let revision = s.revision();
        for command in ["cost_plan", "validate", "quote_payment"] {
            let response = request(&mut s, command, action_to_json(&action));
            assert_eq!(response["ok"], true, "{response}");
            if command == "cost_plan" {
                assert_eq!(response["result"]["energy"], 1);
                assert_eq!(
                    response["result"]["object_costs"],
                    json!([{"type":"exhaust_gear","player":actor,"gear_id":gear}])
                );
            }
            assert_eq!(views(&s), before);
            assert_eq!(s.revision(), revision);
        }
        apply(&mut s, action);
        assert!(!s.game.players[actor].gear[0].ready);
        assert!(s.game.players[actor].hand.is_empty());
        assert!(s.game.chain.last().unwrap().targets.is_empty());
        apply(&mut s, Action::Pass);
        apply(&mut s, Action::Pass);
        let v = views(&s);
        assert_eq!(v[actor]["players"][actor]["hand"], json!([71]));
        assert!(v[1 - actor]["players"][actor]["hand"].is_null());
        assert_eq!(v[1 - actor]["players"][actor]["hand_count"], 1);
        assert_eq!(s.game.chain.len(), 1);
    }
}

#[test]
fn irelia_wheel_api_rejects_stale_wrong_seat_and_exhausted_source_atomically() {
    for actor in 0..2 {
        let mut s = session(actor);
        let action = action_to_json(&pay(&s));
        let before = views(&s);
        let revision = s.revision();
        for (player, revision) in [(1 - actor, revision), (actor, revision + 1)] {
            let response = s.handle(
                json!({"command":"apply","player":player,"revision":revision,"action":action}),
            );
            assert_eq!(response["ok"], false);
            assert_eq!(views(&s), before);
        }
        s.game.players[actor].gear[0].ready = false;
        let before = views(&s);
        let response = request(&mut s, "apply", action);
        assert_eq!(response["ok"], false);
        assert_eq!(views(&s), before);
        assert_eq!(s.revision(), revision);
    }
}
