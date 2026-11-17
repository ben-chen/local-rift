//! Temptation's fixed destination is restricted independently of its unit target.
use super::*;
use crate::play::{ExecutionChoices, MoveDestination, PlayChoices, RepeatSelection};
use test_support::{decline_legend_action, position, reject_unchanged, settle_unopposed_chain};

fn unit(game: &mut Game, controller: usize, lane: Option<usize>) -> Target {
    let unit_id = game.create_unit(controller, 71, false, 0);
    let target = Target::Unit {
        player: controller,
        unit_id,
    };
    let unit = game.unit_mut(target).unwrap();
    unit.lane = lane;
    unit.ready = false;
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
    if game.priority() != actor {
        game.apply(Action::Pass).unwrap();
    }
    assert_eq!(game.priority(), actor);
}

#[test]
fn temptation_requires_a_different_location_with_a_unit_of_the_target_controller() {
    for actor in 0..2 {
        for destination in [None, Some(1)] {
            for matching_controller in [false, true] {
                let mut game = position(actor, 0, 487001);
                let target = unit(&mut game, 1 - actor, Some(0));
                let friendly = unit(&mut game, actor, Some(0));
                game.players[actor].hand = vec![487];
                reject_unchanged(&mut game, play(target, destination));
                unit(
                    &mut game,
                    if matching_controller {
                        1 - actor
                    } else {
                        actor
                    },
                    destination,
                );
                assert_eq!(
                    game.card_target_candidates(actor, 487, destination, &[])
                        .unwrap()
                        .contains(&target),
                    matching_controller
                );
                reject_unchanged(&mut game, play(friendly, destination));
                reject_unchanged(&mut game, play(target, Some(0)));
                if !matching_controller {
                    reject_unchanged(&mut game, play(target, destination));
                    continue;
                }
                let power = game.players[actor].power_pool;
                game.apply(play(target, destination)).unwrap();
                assert_eq!(game.players[actor].energy_pool, 18);
                assert_eq!(game.players[actor].power_pool, power);
                settle_unopposed_chain(&mut game);
                let moved = game.unit_ref(target).unwrap();
                assert_eq!(moved.lane, destination);
                assert!(!moved.ready);
                assert_eq!(moved.damage, 1);
                assert!(game.unit_moved_this_turn(1 - actor, moved.id));
            }
        }
    }
}

#[test]
fn temptation_losing_its_anchor_to_flash_recalls_unless_movement_is_prevented() {
    for actor in 0..2 {
        for blocked in [false, true] {
            let mut game = position(actor, 0, 487002);
            let target = unit(&mut game, 1 - actor, Some(0));
            let anchor = unit(&mut game, 1 - actor, Some(1));
            let id = game.unit_ref(target).unwrap().id;
            if blocked {
                game.movement_locks.push((actor, id));
            }
            game.players[actor].hand = vec![487];
            game.players[1 - actor].hand = vec![9];
            game.apply(play(target, Some(1))).unwrap();
            respond_as(&mut game, 1 - actor);
            game.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![anchor],
            })
            .unwrap();
            settle_unopposed_chain(&mut game);
            assert_eq!(game.unit_ref(anchor).unwrap().lane, None);
            let target = game.unit_ref(target).unwrap();
            assert_eq!(target.lane, if blocked { Some(0) } else { None });
            assert!(!target.ready);
            assert_eq!(target.damage, 1);
            assert!(
                !game.unit_moved_this_turn(1 - actor, id),
                "Recall is not a Move; prevention is not Recall"
            );
        }
    }
}

#[test]
fn temptation_repeat_rechecks_occupancy_after_the_original_execution() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 487003);
        let first = unit(&mut game, 1 - actor, Some(0));
        let second = unit(&mut game, 1 - actor, Some(1));
        game.players[actor].hand = vec![487];
        let original = play(first, Some(1));
        let repeat = game.play_choices(&original).unwrap().repeats[0].id;
        game.apply(Action::WithPlayChoices {
            action: Box::new(original),
            choices: PlayChoices {
                repeats: vec![RepeatSelection {
                    option_id: repeat,
                    choices: ExecutionChoices {
                        targets: vec![second],
                        destination: Some(MoveDestination::Battlefield { lane: 0 }),
                        ..Default::default()
                    },
                }],
                ..Default::default()
            },
        })
        .unwrap();
        assert_eq!(game.players[actor].energy_pool, 16);
        let spell = game.chain.iter().find(|item| item.card_id == 487).unwrap();
        assert_eq!(spell.executions.len(), 2);
        assert_eq!(
            spell.executions[0].destination,
            Some(MoveDestination::Battlefield { lane: 1 })
        );
        assert_eq!(
            spell.executions[1].destination,
            Some(MoveDestination::Battlefield { lane: 0 })
        );
        settle_unopposed_chain(&mut game);
        assert_eq!(game.unit_ref(first).unwrap().lane, Some(1));
        assert_eq!(game.unit_ref(second).unwrap().lane, None);
        assert!(game.unit_moved_this_turn(1 - actor, game.unit_ref(first).unwrap().id));
        assert!(!game.unit_moved_this_turn(1 - actor, game.unit_ref(second).unwrap().id));
    }
}

#[test]
fn temptation_accepts_unit_tokens_as_anchors_and_ignores_plain_gear() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 487004);
        let target = unit(&mut game, 1 - actor, Some(0));
        game.players[actor].hand = vec![487];
        game.gear_play_at(1 - actor, 10, None).unwrap();
        reject_unchanged(&mut game, play(target, None));
        // A unit anchor does not require Might, readiness, or a non-token card.
        let anchor = unit(&mut game, 1 - actor, Some(1));
        let state = game.unit_mut(anchor).unwrap();
        state.token = true;
        state.buff = 0;
        state.damage = 0;
        assert_eq!(game.unit_might(game.unit_ref(anchor).unwrap()).unwrap(), 0);
        game.apply(play(target, Some(1))).unwrap();
        settle_unopposed_chain(&mut game);
        assert_eq!(game.unit_ref(target).unwrap().lane, Some(1));
    }
}

#[test]
fn temptation_is_an_ordinary_spell_even_with_repeat() {
    for actor in 0..2 {
        let (mut game, _, enemy) = test_support::combat_position(actor, 487005);
        unit(&mut game, 1 - actor, None);
        game.players[actor].hand = vec![487];
        let action = play(enemy, None);
        let repeat = game.play_choices(&action).unwrap().repeats[0].id;
        reject_unchanged(&mut game, action.clone());
        reject_unchanged(
            &mut game,
            Action::WithPlayChoices {
                action: Box::new(action),
                choices: PlayChoices {
                    repeats: vec![RepeatSelection {
                        option_id: repeat,
                        choices: ExecutionChoices {
                            targets: vec![enemy],
                            destination: Some(MoveDestination::Base),
                            ..Default::default()
                        },
                    }],
                    ..Default::default()
                },
            },
        );
    }
}
