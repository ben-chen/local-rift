use super::*;
use crate::game::test_support::{decline_legend_action, position, ready_unit};
use test_support::{apply, request, views};

#[test]
fn twilight_api_exposes_live_targets_hand_and_flow_costs_without_preview_mutation() {
    for actor in 0..2 {
        for flow in [false, true] {
            let mut game = position(actor, 0, 486210);
            let mut targets = Vec::new();
            for controller in 0..2 {
                let unit_id = ready_unit(&mut game, controller, 71);
                let unit = game.players[controller].units.last_mut().unwrap();
                unit.ready = false;
                unit.lane = Some(0);
                unit.temporary_buff = -2;
                targets.push(Target::Unit {
                    player: controller,
                    unit_id,
                });
            }
            let too_large = Target::Unit {
                player: actor,
                unit_id: ready_unit(&mut game, actor, 71),
            };
            game.players[actor].units.last_mut().unwrap().lane = Some(0);
            if flow {
                game.players[actor].discard = vec![486];
            } else {
                game.players[actor].hand = vec![486];
            }
            let mut session = Session {
                game,
                ..Session::default()
            };
            let action = |target| {
                if flow {
                    Action::PlayFromTrash {
                        trash_index: 0,
                        lane: None,
                        target,
                    }
                } else {
                    Action::PlayCard {
                        hand_index: 0,
                        lane: None,
                        target,
                    }
                }
            };
            let before = views(&session);
            let revision = session.revision();
            let query = session.handle(json!({"command":"target_candidates","player":actor,
                "revision":revision,"execution":0,"action":action_to_json(&action(None))}));
            assert_eq!(query["ok"], true, "{query}");
            let candidates = query["result"]["candidates"].as_array().unwrap();
            for target in &targets {
                assert!(candidates.contains(&target_to_json(target)));
            }
            assert!(!candidates.contains(&target_to_json(&too_large)));
            assert_eq!(
                targeting::play_targets_to_json(486)["condition"],
                json!({
                    "type":"maximum_might","maximum_might":3,"controller":"any","rechecked_on_resolution":true
                })
            );
            let target = targets[1 - actor];
            for command in ["validate", "play_choices", "quote_payment", "cost_plan"] {
                let result = request(&mut session, command, action_to_json(&action(Some(target))));
                assert_eq!(result["ok"], true, "{result}");
            }
            assert_eq!(
                session
                    .cost_plan(actor, revision, action(Some(target)))
                    .unwrap()
                    .energy,
                if flow { 4 } else { 2 }
            );
            let rejected = request(
                &mut session,
                "apply",
                action_to_json(&action(Some(too_large))),
            );
            assert_eq!(rejected["ok"], false);
            assert_eq!(session.revision(), revision);
            assert_eq!(views(&session), before);
            apply(&mut session, action(Some(target)));
            for _ in 0..40 {
                if session.game.chain.is_empty() && session.game.decision.is_none() {
                    break;
                }
                let action = decline_legend_action(&session.game).unwrap_or(Action::Pass);
                apply(&mut session, action);
            }
            assert!(session.game.chain.is_empty());
            assert!(session.game.decision.is_none());
            let unit = &session.game.players[1 - actor].units[0];
            assert_eq!(unit.lane, None);
            assert!(!unit.ready);
            assert_eq!(session.game.players[actor].banish.contains(&486), flow);
            assert_eq!(session.game.players[actor].discard.contains(&486), !flow);
        }
    }
}
