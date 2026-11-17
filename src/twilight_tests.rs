//! Twilight Step: evaluated Might, declared destinations and ordinary Flow.
use super::*;
use test_support::{decline_legend_action, position, reject_unchanged, settle_unopposed_chain};

fn unit(game: &mut Game, controller: usize, might: i64) -> Target {
    let unit_id = game.create_unit(controller, 71, false, 0);
    let target = Target::Unit {
        player: controller,
        unit_id,
    };
    let unit = game.unit_mut(target).unwrap();
    unit.lane = Some(0);
    unit.temporary_buff = might - 5;
    unit.damage = 1;
    target
}

fn play(target: Target, lane: Option<usize>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane,
        targets: vec![target],
    }
}

fn respond_as(game: &mut Game, actor: usize) {
    while let Some(action) = decline_legend_action(game) {
        game.apply(action).unwrap();
    }
    assert!(game.decision.is_none());
    if game.priority() != actor {
        game.apply(Action::Pass).unwrap();
    }
    assert_eq!(game.priority(), actor);
}

#[test]
fn twilight_uses_evaluated_might_for_either_controller_and_does_not_ready() {
    for actor in 0..2 {
        for controller in 0..2 {
            for might in [3, 4] {
                let mut game = position(actor, 0, 486201);
                let target = unit(&mut game, controller, might);
                game.players[actor].hand = vec![486];
                let action = play(target, Some(1));
                assert_eq!(
                    game.valid_card_target(actor, 486, Some(1), target, &[])
                        .unwrap(),
                    might <= 3
                );
                assert_eq!(
                    game.card_target_candidates(actor, 486, Some(1), &[])
                        .unwrap()
                        .contains(&target),
                    might <= 3
                );
                if might > 3 {
                    reject_unchanged(&mut game, action);
                    continue;
                }
                let energy = game.players[actor].energy_pool;
                let chaos = game.players[actor].power_pool[domain_index(Domain::Chaos)];
                game.apply(action).unwrap();
                assert_eq!(game.players[actor].energy_pool, energy - 2);
                assert_eq!(
                    game.players[actor].power_pool[domain_index(Domain::Chaos)],
                    chaos - 1
                );
                settle_unopposed_chain(&mut game);
                let unit = game.unit_ref(target).unwrap();
                assert_eq!(unit.lane, Some(1));
                assert!(!unit.ready);
                assert_eq!(unit.damage, 1);
                assert!(game.unit_moved_this_turn(controller, unit.id));
                assert!(game.players[actor].discard.contains(&486));
            }
        }
    }
}

#[test]
fn twilight_rechecks_might_after_a_real_discipline_response() {
    for actor in 0..2 {
        for controller in 0..2 {
            let mut game = position(actor, 0, 486202);
            let target = unit(&mut game, controller, 3);
            game.players[actor].hand = vec![486];
            game.players[1 - actor].hand = vec![7];
            game.apply(play(target, Some(1))).unwrap();
            respond_as(&mut game, 1 - actor);
            game.apply(play(target, None)).unwrap();
            settle_unopposed_chain(&mut game);
            let unit = game.unit_ref(target).unwrap();
            assert_eq!(game.unit_might(unit).unwrap(), 5);
            assert_eq!(unit.lane, Some(0));
            assert!(!unit.ready);
            assert!(!game.unit_moved_this_turn(controller, unit.id));
            assert!(game.players[actor].discard.contains(&486));
        }
    }
}

#[test]
fn twilight_flow_pays_its_distinct_cost_and_banishes_after_resolving() {
    for actor in 0..2 {
        for controller in 0..2 {
            let mut game = position(actor, 0, 486203);
            let target = unit(&mut game, controller, 3);
            game.players[actor].discard = vec![486];
            let action = Action::PlayFromTrash {
                trash_index: 0,
                lane: None,
                target: Some(target),
            };
            assert_eq!(game.cost_plan(&action).unwrap().energy, 4);
            let chaos = game.players[actor].power_pool[domain_index(Domain::Chaos)];
            game.apply(action).unwrap();
            assert_eq!(game.players[actor].energy_pool, 16);
            assert_eq!(
                game.players[actor].power_pool[domain_index(Domain::Chaos)],
                chaos - 1
            );
            settle_unopposed_chain(&mut game);
            assert_eq!(game.unit_ref(target).unwrap().lane, None);
            assert!(!game.unit_ref(target).unwrap().ready);
            assert_eq!(game.players[actor].banish, vec![486]);
            assert!(game.players[actor].discard.is_empty());
        }
    }
}

#[test]
fn twilight_and_its_flow_keep_normal_spell_timing_and_destination_legality() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 486204);
        let target = unit(&mut game, actor, 3);
        game.players[actor].hand = vec![486, 7];
        game.players[actor].discard = vec![486];
        reject_unchanged(&mut game, play(target, Some(0)));
        game.apply(Action::PlayCardWithTargets {
            hand_index: 1,
            lane: None,
            targets: vec![target],
        })
        .unwrap();
        respond_as(&mut game, actor);
        reject_unchanged(&mut game, play(target, Some(1)));
        reject_unchanged(
            &mut game,
            Action::PlayFromTrash {
                trash_index: 0,
                lane: Some(1),
                target: Some(target),
            },
        );

        let (mut game, target, _) = test_support::combat_position(actor, 486205);
        game.unit_mut(target).unwrap().temporary_buff = -2;
        game.players[actor].hand = vec![486];
        game.players[actor].discard = vec![486];
        reject_unchanged(&mut game, play(target, Some(1)));
        reject_unchanged(
            &mut game,
            Action::PlayFromTrash {
                trash_index: 0,
                lane: Some(1),
                target: Some(target),
            },
        );
    }
}

#[test]
fn twilight_selected_base_survives_control_changes_without_creating_move_history() {
    // Authored control-change boundary: this exercises the shared control
    // primitive while the spell waits, not an invented Reaction on Possession.
    for actor in 0..2 {
        for controller in 0..2 {
            for blocked in [false, true] {
                let mut game = position(actor, 0, 486206);
                let target = unit(&mut game, controller, 3);
                let Target::Unit { unit_id, .. } = target else {
                    unreachable!()
                };
                game.players[actor].hand = vec![486];
                if blocked {
                    game.movement_locks.push((actor, unit_id));
                }
                game.apply(play(target, None)).unwrap();
                respond_as(&mut game, actor);
                assert_eq!(
                    game.chain
                        .iter()
                        .find(|i| i.card_id == 486)
                        .unwrap()
                        .executions[0]
                        .targets,
                    vec![target]
                );
                let current = game
                    .take_control_of_battlefield_unit(1 - controller, target)
                    .unwrap();
                settle_unopposed_chain(&mut game);
                let unit = game.unit_ref(current).unwrap();
                assert_eq!(unit.lane, if blocked { Some(0) } else { None });
                assert!(!unit.ready);
                assert_eq!(unit.damage, 1);
                assert!(
                    !game.unit_moved_this_turn(1 - controller, unit_id),
                    "the selected foreign base instead Recalls; a prevented Move stays put"
                );
                assert_eq!(game.board_owner(current), Some(controller));
            }
        }
    }
}
