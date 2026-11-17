use super::*;
use crate::game::test_support::{decline_legend_action, position, ready_unit};
use test_support::{apply, request, views};

#[test]
fn movement_groups_api_queries_are_read_only_and_use_the_shared_constraints() {
    for actor in 0..2 {
        for card in 488..=490 {
            let mut s = Session {
                game: position(actor, 0, 489100),
                ..Session::default()
            };
            let owner = if card == 488 { actor } else { 1 - actor };
            let id = ready_unit(&mut s.game, owner, 13);
            s.game.players[owner].units.last_mut().unwrap().lane = Some(0);
            let target = Target::Unit {
                player: owner,
                unit_id: id,
            };
            s.game.players[actor].hand = vec![card];
            let lane = if card == 489 { Some(1) } else { None };
            let action = Action::PlayCardWithTargets {
                hand_index: 0,
                lane,
                targets: vec![target],
            };
            let before = views(&s);
            let revision = s.revision();
            let query=s.handle(json!({"command":"target_candidates","player":actor,"revision":revision,"execution":0,"action":action_to_json(&Action::PlayCard {hand_index:0,lane,target:None})}));
            assert_eq!(query["ok"], true, "{query}");
            assert!(
                query["result"]["candidates"]
                    .as_array()
                    .unwrap()
                    .contains(&target_to_json(&target))
            );
            for command in ["validate", "play_choices", "quote_payment", "cost_plan"] {
                assert_eq!(
                    request(&mut s, command, action_to_json(&action))["ok"],
                    true
                );
            }
            assert_eq!(views(&s), before);
            assert_eq!(s.revision(), revision);
            let wrong = Target::Unit {
                player: 1 - owner,
                unit_id: id,
            };
            let invalid = Action::PlayCardWithTargets {
                hand_index: 0,
                lane,
                targets: vec![wrong],
            };
            assert_eq!(
                request(&mut s, "apply", action_to_json(&invalid))["ok"],
                false
            );
            assert_eq!(views(&s), before);
            assert_eq!(s.revision(), revision);
            let metadata = targeting::play_targets_to_json(card);
            assert_eq!(
                metadata["condition"]["controller"],
                if card == 488 { "friendly" } else { "enemy" }
            );
            if card == 489 {
                assert_eq!(metadata["condition"]["maximum_total_might"], 8);
            }
            apply(&mut s, action);
            for _ in 0..60 {
                if s.game.chain.is_empty() && s.game.decision.is_none() {
                    break;
                }
                let next = decline_legend_action(&s.game).unwrap_or(Action::Pass);
                apply(&mut s, next);
            }
            assert!(s.game.chain.is_empty() && s.game.decision.is_none());
            assert_eq!(
                s.game.players[owner]
                    .units
                    .iter()
                    .find(|u| u.id == id)
                    .unwrap()
                    .lane,
                lane
            );
        }
    }
}

#[test]
fn tricksy_api_requires_the_controller_to_choose_a_legal_remaining_subset() {
    for actor in 0..2 {
        let mut s = Session {
            game: position(actor, 0, 489101),
            ..Session::default()
        };
        let mut targets = Vec::new();
        for card in [13, 71] {
            let id = ready_unit(&mut s.game, 1 - actor, card);
            s.game.players[1 - actor].units.last_mut().unwrap().lane = Some(0);
            targets.push(Target::Unit {
                player: 1 - actor,
                unit_id: id,
            });
        }
        s.game.players[actor].hand = vec![489];
        s.game.players[1 - actor].hand = vec![7];
        apply(
            &mut s,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: targets.clone(),
            },
        );
        if s.game.priority() == actor {
            apply(&mut s, Action::Pass);
        }
        apply(
            &mut s,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![targets[0]],
            },
        );
        for _ in 0..60 {
            if matches!(s.game.decision, Some(Decision::ResolutionSubset { .. })) {
                break;
            }
            let next = decline_legend_action(&s.game).unwrap_or(Action::Pass);
            apply(&mut s, next);
        }
        assert!(matches!(
            s.game.decision,
            Some(Decision::ResolutionSubset { maximum: 8, .. })
        ));
        let before = views(&s);
        let revision = s.revision();
        assert!(
            before[actor]
                .to_string()
                .contains("requires_maximal_subset")
        );
        let action = Action::ChooseResolutionSubset {
            targets: vec![targets[1]],
        };
        let wrong=s.handle(json!({"command":"apply","player":1-actor,"revision":revision,"action":action_to_json(&action)}));
        assert_eq!(wrong["ok"], false);
        assert_eq!(
            request(
                &mut s,
                "apply",
                action_to_json(&Action::ChooseResolutionSubset { targets: vec![] })
            )["ok"],
            false
        );
        assert_eq!(views(&s), before);
        assert_eq!(s.revision(), revision);
        assert_eq!(
            request(&mut s, "validate", action_to_json(&action))["ok"],
            true
        );
        assert_eq!(views(&s), before);
        apply(&mut s, action);
        assert_eq!(
            s.game.players[1 - actor]
                .units
                .iter()
                .find(|u| u.id
                    == match targets[1] {
                        Target::Unit { unit_id, .. } => unit_id,
                        _ => unreachable!(),
                    })
                .unwrap()
                .lane,
            None
        );
    }
}
