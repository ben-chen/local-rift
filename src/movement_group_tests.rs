//! Printed movement instructions, public responses and explicit remaining choices.
use super::*;
use test_support::{
    decline_legend_action, position, ready_unit, reject_unchanged, settle_unopposed_chain,
};

fn unit(game: &mut Game, owner: usize, lane: Option<usize>, might: u64) -> Target {
    let unit_id = ready_unit(game, owner, 71);
    let target = Target::Unit {
        player: owner,
        unit_id,
    };
    let printed = game.unit_might(game.unit_ref(target).unwrap()).unwrap();
    game.modify_might(target, i128::from(might) - i128::from(printed), 0)
        .unwrap();
    let state = game.unit_mut(target).unwrap();
    state.lane = lane;
    state.ready = false;
    target
}

fn play(targets: Vec<Target>, lane: Option<usize>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        targets,
        lane,
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

fn stop_at_choice(game: &mut Game) {
    for _ in 0..60 {
        if matches!(
            game.decision,
            Some(Decision::ResolutionSubset { .. } | Decision::ResolutionTargets { .. })
        ) {
            return;
        }
        game.apply(decline_legend_action(game).unwrap_or(Action::Pass))
            .unwrap();
    }
    panic!("missing remaining-target choice");
}

#[test]
fn emperor_divide_accepts_zero_or_many_friendly_units_from_one_battlefield() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 488001);
        let chosen: Vec<_> = (0..4).map(|_| unit(&mut game, actor, Some(0), 3)).collect();
        let other_field = unit(&mut game, actor, Some(1), 3);
        let base = unit(&mut game, actor, None, 3);
        let enemy = unit(&mut game, 1 - actor, Some(0), 3);
        game.players[actor].hand = vec![488];
        for targets in [
            vec![chosen[0], other_field],
            vec![base],
            vec![enemy],
            vec![chosen[0], chosen[0]],
        ] {
            reject_unchanged(&mut game, play(targets, None));
        }
        assert!(
            !game
                .card_target_candidates(actor, 488, None, &[chosen[0]])
                .unwrap()
                .contains(&other_field)
        );
        let mut empty = game.clone();
        empty.apply(play(vec![], None)).unwrap();
        settle_unopposed_chain(&mut empty);
        assert_eq!(empty.unit_ref(chosen[0]).unwrap().lane, Some(0));
        let power = game.players[actor].power_pool;
        game.unit_mut(chosen[0]).unwrap().damage = 1;
        game.apply(play(chosen.clone(), None)).unwrap();
        assert_eq!(game.players[actor].energy_pool, 18);
        assert_eq!(game.players[actor].power_pool, power);
        settle_unopposed_chain(&mut game);
        for target in chosen {
            let state = game.unit_ref(target).unwrap();
            assert_eq!(state.lane, None);
            assert!(!state.ready);
            assert!(game.unit_moved_this_turn(actor, state.id));
        }
        assert_eq!(game.unit_ref(other_field).unwrap().lane, Some(1));
        assert_eq!(game.unit_ref(enemy).unwrap().lane, Some(0));
    }
}

#[test]
fn emperor_divide_siren_response_offers_both_remaining_location_groups() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 488002);
        let first = unit(&mut game, actor, None, 3);
        let second = unit(&mut game, actor, Some(0), 3);
        game.battlefield_owner[0] = Some(actor);
        let id = game.unit_ref(first).unwrap().id;
        game.unit_mut(first).unwrap().ready = true;
        game.apply(Action::Move {
            unit_id: id,
            lane: Some(0),
        })
        .unwrap();
        assert!(game.unit_moved_this_turn(actor, id));
        game.players[actor].hand = vec![488];
        game.players[1 - actor].hand = vec![427];
        game.apply(play(vec![first, second], None)).unwrap();
        respond_as(&mut game, 1 - actor);
        game.apply(play(vec![first], Some(1))).unwrap();
        stop_at_choice(&mut game);
        let Some(Decision::ResolutionTargets { groups, .. }) = &game.decision else {
            panic!("missing location groups");
        };
        assert_eq!(groups.len(), 2);
        let index = groups
            .iter()
            .position(|group| group == &vec![second])
            .unwrap();
        reject_unchanged(&mut game, Action::ChooseResolutionGroup { index: 2 });
        game.apply(Action::ChooseResolutionGroup { index }).unwrap();
        settle_unopposed_chain(&mut game);
        assert_eq!(game.unit_ref(first).unwrap().lane, Some(1));
        assert_eq!(game.unit_ref(second).unwrap().lane, None);
    }
}

#[test]
fn tricksy_group_uses_one_destination_and_preserves_each_units_movement_restriction() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 489001);
        let first = unit(&mut game, 1 - actor, Some(0), 3);
        let second = unit(&mut game, 1 - actor, None, 5);
        let extra = unit(&mut game, 1 - actor, Some(0), 1);
        let friend = unit(&mut game, actor, None, 1);
        game.players[actor].hand = vec![489];
        for targets in [vec![first, second, extra], vec![first, first], vec![friend]] {
            reject_unchanged(&mut game, play(targets, Some(1)));
        }
        reject_unchanged(&mut game, play(vec![second], None));
        let blocked = game.unit_ref(first).unwrap().id;
        game.movement_locks.push((actor, blocked));
        game.unit_mut(second).unwrap().damage = 1;
        game.apply(play(vec![first, second], Some(1))).unwrap();
        assert_eq!(game.players[actor].energy_pool, 16);
        assert_eq!(
            game.players[actor].power_pool[domain_index(Domain::Calm)],
            4
        );
        settle_unopposed_chain(&mut game);
        assert_eq!(game.unit_ref(first).unwrap().lane, Some(0));
        let moved = game.unit_ref(second).unwrap();
        assert_eq!(moved.lane, Some(1));
        assert!(!moved.ready);
        assert_eq!(moved.damage, 1);
        assert!(game.unit_moved_this_turn(1 - actor, moved.id));
        assert!(!game.unit_moved_this_turn(1 - actor, blocked));
    }
}

#[test]
fn tricksy_discipline_response_requires_a_maximal_original_subset_including_zero_might() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 489002);
        let first = unit(&mut game, 1 - actor, Some(0), 3);
        let second = unit(&mut game, 1 - actor, Some(0), 5);
        let zero = unit(&mut game, 1 - actor, Some(0), 0);
        let unchosen = unit(&mut game, 1 - actor, Some(0), 1);
        game.players[actor].hand = vec![489];
        game.players[1 - actor].hand = vec![7];
        game.apply(play(vec![first, second, zero], None)).unwrap();
        respond_as(&mut game, 1 - actor);
        game.apply(play(vec![first], None)).unwrap();
        stop_at_choice(&mut game);
        assert!(matches!(
            game.decision,
            Some(Decision::ResolutionSubset { maximum: 8, .. })
        ));
        for targets in [
            vec![],
            vec![second],
            vec![first, second],
            vec![second, zero, unchosen],
        ] {
            reject_unchanged(&mut game, Action::ChooseResolutionSubset { targets });
        }
        game.apply(Action::ChooseResolutionSubset {
            targets: vec![second, zero],
        })
        .unwrap();
        settle_unopposed_chain(&mut game);
        assert_eq!(game.unit_ref(first).unwrap().lane, Some(0));
        assert_eq!(game.unit_ref(second).unwrap().lane, None);
        assert_eq!(game.unit_ref(zero).unwrap().lane, None);
        assert_eq!(game.unit_ref(unchosen).unwrap().lane, Some(0));
    }
}

#[test]
fn isolate_draws_only_for_an_enemy_alone_after_the_move_even_when_movement_is_prevented() {
    for actor in 0..2 {
        for blocked in [false, true] {
            for enemies in 0..3 {
                for friendly in [false, true] {
                    let mut game = position(actor, 0, 490001);
                    let target = unit(&mut game, 1 - actor, Some(0), 3);
                    for _ in 0..enemies {
                        unit(&mut game, 1 - actor, Some(0), 3);
                    }
                    if friendly {
                        unit(&mut game, actor, Some(0), 3);
                    }
                    let id = game.unit_ref(target).unwrap().id;
                    if blocked {
                        game.movement_locks.push((actor, id));
                    }
                    game.players[actor].hand = vec![490];
                    game.apply(play(vec![target], None)).unwrap();
                    settle_unopposed_chain(&mut game);
                    let expected = !friendly && enemies + usize::from(blocked) == 1;
                    assert_eq!(game.players[actor].hand.len(), usize::from(expected));
                    assert_eq!(
                        game.unit_ref(target).unwrap().lane,
                        if blocked { Some(0) } else { None }
                    );
                    assert_eq!(game.unit_moved_this_turn(1 - actor, id), !blocked);
                }
            }
        }
    }
}

#[test]
fn isolate_flash_response_mistargets_and_ignores_the_linked_draw() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 490002);
        let target = unit(&mut game, 1 - actor, Some(0), 3);
        unit(&mut game, 1 - actor, Some(0), 3);
        game.players[actor].hand = vec![490];
        game.players[1 - actor].hand = vec![9];
        game.apply(play(vec![target], None)).unwrap();
        respond_as(&mut game, 1 - actor);
        game.apply(play(vec![target], None)).unwrap();
        settle_unopposed_chain(&mut game);
        assert_eq!(game.unit_ref(target).unwrap().lane, None);
        assert_eq!(game.lane_count(1 - actor, 0), 1);
        assert!(game.players[actor].hand.is_empty());
    }
}

#[test]
fn emperor_divide_hidden_reaction_uses_only_units_at_its_source_battlefield() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 488003);
        let first = unit(&mut game, actor, Some(0), 3);
        let other = unit(&mut game, actor, Some(1), 3);
        let enemy = unit(&mut game, 1 - actor, None, 3);
        game.battlefield_owner = vec![Some(actor), Some(actor)];
        game.players[actor].hand = vec![488];
        game.players[1 - actor].hand = vec![7];
        game.apply(Action::Hide {
            hand_index: 0,
            lane: 0,
        })
        .unwrap();
        let hidden_id = game.players[actor].hidden[0].id;
        game.apply(Action::EndTurn).unwrap();
        game.apply(play(vec![enemy], None)).unwrap();
        respond_as(&mut game, actor);
        reject_unchanged(
            &mut game,
            Action::PlayHidden {
                hidden_id,
                targets: vec![other],
            },
        );
        let before = game.players[actor].energy_pool;
        game.apply(Action::PlayHidden {
            hidden_id,
            targets: vec![first],
        })
        .unwrap();
        assert_eq!(game.players[actor].energy_pool, before);
        settle_unopposed_chain(&mut game);
        assert_eq!(game.unit_ref(first).unwrap().lane, None);
        assert_eq!(game.unit_ref(other).unwrap().lane, Some(1));
        assert!(game.players[actor].hidden.is_empty());
    }
}

#[test]
fn flash_and_emperor_divide_offer_the_order_of_simultaneous_move_triggers() {
    for actor in 0..2 {
        for spell in [9, 488] {
            let mut game = position(actor, 0, 488004);
            let mut targets = Vec::new();
            for _ in 0..2 {
                let id = ready_unit(&mut game, actor, 17); // Stellacorn Herder: move, draw 1.
                game.players[actor].units.last_mut().unwrap().lane = Some(0);
                targets.push(Target::Unit {
                    player: actor,
                    unit_id: id,
                });
            }
            game.players[actor].hand = vec![spell];
            game.apply(play(targets, None)).unwrap();
            for _ in 0..60 {
                if let Some(Decision::OrderTriggers { options, .. }) = &game.decision
                    && options.iter().filter(|option| option.card_id == 17).count() == 2
                {
                    let triggers: Vec<_> = game
                        .pending_triggers
                        .iter()
                        .filter(|t| t.option.card_id == 17)
                        .collect();
                    assert_eq!(triggers.len(), 2);
                    assert_eq!(triggers[0].batch, triggers[1].batch);
                    break;
                }
                game.apply(decline_legend_action(&game).unwrap_or(Action::Pass))
                    .unwrap();
            }
            let Some(Decision::OrderTriggers { options, .. }) = &game.decision else {
                panic!("both Move triggers must be offered together");
            };
            assert_eq!(
                options.iter().filter(|option| option.card_id == 17).count(),
                2
            );
            settle_unopposed_chain(&mut game);
            assert_eq!(game.players[actor].hand.len(), 2);
        }
    }
}
