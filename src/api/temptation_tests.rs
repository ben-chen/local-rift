use super::*;
use crate::game::test_support::{decline_legend_action, position, ready_unit};
use test_support::{apply, request, views};

#[test]
fn temptation_api_reports_destination_restrictions_and_rejects_empty_locations_atomically() {
    for actor in 0..2 {
        for occupied_by_enemy in [false, true] {
            let mut game = position(actor, 0, 487010);
            let id = ready_unit(&mut game, 1 - actor, 71);
            game.players[1 - actor].units.last_mut().unwrap().lane = Some(0);
            let target = Target::Unit {
                player: 1 - actor,
                unit_id: id,
            };
            let anchor_controller = if occupied_by_enemy { 1 - actor } else { actor };
            ready_unit(&mut game, anchor_controller, 71);
            game.players[anchor_controller]
                .units
                .last_mut()
                .unwrap()
                .lane = Some(1);
            game.players[actor].hand = vec![487];
            let mut session = Session {
                game,
                ..Session::default()
            };
            let action = Action::PlayCardWithTargets {
                hand_index: 0,
                lane: Some(1),
                targets: vec![target],
            };
            let before = views(&session);
            let revision = session.revision();
            let query = session.handle(json!({"command":"target_candidates","player":actor,
            "revision":revision,"execution":0,"action":action_to_json(&Action::PlayCard {
                hand_index:0, lane:Some(1), target:None
            })}));
            assert_eq!(query["ok"], true, "{query}");
            assert_eq!(
                query["result"]["candidates"]
                    .as_array()
                    .unwrap()
                    .contains(&target_to_json(&target)),
                occupied_by_enemy
            );
            assert_eq!(
                targeting::play_targets_to_json(487)["destination_condition"],
                json!({
                    "type":"unit_with_same_controller", "rechecked_on_resolution":true,
                    "invalid_destination":"recall"
                })
            );
            assert_eq!(views(&session), before);
            if !occupied_by_enemy {
                assert_eq!(
                    request(&mut session, "apply", action_to_json(&action))["ok"],
                    false
                );
                assert_eq!(views(&session), before);
                assert_eq!(session.revision(), revision);
                continue;
            }
            for command in ["validate", "play_choices", "quote_payment", "cost_plan"] {
                assert_eq!(
                    request(&mut session, command, action_to_json(&action))["ok"],
                    true
                );
            }
            assert_eq!(views(&session), before);
            let power = session.game.players[actor].power_pool;
            apply(&mut session, action);
            for _ in 0..40 {
                if session.game.chain.is_empty() && session.game.decision.is_none() {
                    break;
                }
                let action = decline_legend_action(&session.game).unwrap_or(Action::Pass);
                apply(&mut session, action);
            }
            assert!(session.game.chain.is_empty());
            assert!(session.game.decision.is_none());
            assert_eq!(session.game.players[actor].energy_pool, 18);
            assert_eq!(session.game.players[actor].power_pool, power);
            assert_eq!(
                session.game.players[1 - actor]
                    .units
                    .iter()
                    .find(|u| u.id == id)
                    .unwrap()
                    .lane,
                Some(1)
            );
        }
    }
}
