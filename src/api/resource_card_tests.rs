//! The JSON protocol exposes a physical payment choice independently of targets.
use super::test_support::{apply, request, views};
use super::*;
use crate::game::test_support::{position, ready_unit};
use crate::play::PlayChoices;

fn session(actor: usize, card: usize) -> (Session, u32) {
    let mut s = Session::new(ResetOptions::default()).unwrap();
    s.game = position(actor, 0, 451002);
    let unit = ready_unit(&mut s.game, actor, 450);
    s.game.players[actor].hand = vec![card];
    s.game.players[actor].rune_deck = vec![Domain::Calm; 3];
    (s, unit)
}

fn cast(card: usize, unit: u32) -> Action {
    Action::WithPlayChoices {
        action: Box::new(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        }),
        choices: PlayChoices {
            bonus: card == 452,
            exhaust_unit: Some(unit),
            ..Default::default()
        },
    }
}

#[test]
fn irelia_resources_api_candidates_payment_and_draw_visibility_are_separate() {
    for actor in 0..2 {
        for card in [452, 453] {
            let (mut s, unit) = session(actor, card);
            let action = cast(card, unit);
            let json_action = action_to_json(&action);
            assert_eq!(parse_action(&json_action).unwrap(), action);
            let before = views(&s);
            let revision = s.revision();
            for command in ["play_choices", "validate", "quote_payment"] {
                let response = request(&mut s, command, json_action.clone());
                assert_eq!(response["ok"], true, "{response}");
                if command == "play_choices" {
                    assert_eq!(
                        response["result"]["unit_cost"],
                        json!({"type":"exhaust_unit", "optional":card == 452, "candidates":[unit], "is_target":false})
                    );
                    assert_eq!(
                        response["result"]["costs"]["object_costs"],
                        json!([{"type":"exhaust_unit", "player":actor, "unit_id":unit}])
                    );
                }
                assert_eq!(views(&s), before);
                assert_eq!(s.revision(), revision);
            }
            let targets = s.handle(json!({"command":"target_candidates", "player":actor, "revision":revision, "execution":0, "action":json_action}));
            assert_eq!(targets["ok"], true, "{targets}");
            assert_eq!(targets["result"]["candidates"], json!([]));
            apply(&mut s, action);
            assert!(!s.game.players[actor].units[0].ready);
            assert!(s.game.players[actor].hand.is_empty());
            assert_eq!(
                s.game.chain.len(),
                1,
                "paying with Jae must not add his draw trigger"
            );
            let shared = views(&s);
            assert_eq!(shared[0]["chain"], shared[1]["chain"]);
            assert_eq!(shared[0]["chain"][0]["targets"], json!([]));
            apply(&mut s, Action::Pass);
            apply(&mut s, Action::Pass);
            let resolved = views(&s);
            let draws = if card == 452 { 2 } else { 1 };
            assert_eq!(
                resolved[actor]["players"][actor]["hand"],
                json!(vec![71; draws])
            );
            assert!(resolved[1 - actor]["players"][actor]["hand"].is_null());
            assert_eq!(resolved[1 - actor]["players"][actor]["hand_count"], draws);
        }
    }
}

#[test]
fn irelia_resources_api_invalid_stale_and_other_seat_requests_are_atomic() {
    for actor in 0..2 {
        let (mut s, unit) = session(actor, 453);
        let before = views(&s);
        let revision = s.revision();
        for value in [
            json!(-1),
            json!(u64::from(u32::MAX) + 1),
            json!("unit"),
            json!(u32::MAX),
            Value::Null,
        ] {
            let mut action = action_to_json(&cast(453, unit));
            action["choices"]["exhaust_unit"] = value;
            let response = request(&mut s, "apply", action);
            assert_eq!(response["ok"], false, "{response}");
            assert_eq!(views(&s), before);
            assert_eq!(s.revision(), revision);
        }
        for (player, supplied_revision) in [(1 - actor, revision), (actor, revision + 1)] {
            let response = s.handle(json!({"command":"apply", "player":player, "revision":supplied_revision, "action":action_to_json(&cast(453, unit))}));
            assert_eq!(response["ok"], false, "{response}");
            assert_eq!(views(&s), before);
            assert_eq!(s.revision(), revision);
        }
    }
}
