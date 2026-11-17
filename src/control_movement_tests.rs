//! Captured ability actors and physical movement destinations survive control.
use super::*;
use test_support::{position, settle_unopposed_chain};

#[test]
fn pending_ezreal_uses_the_ability_controllers_base_then_instead_recalls() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 468501);
        let unit_id = g.create_unit(actor, 412, false, 0);
        let original = Target::Unit {
            player: actor,
            unit_id,
        };
        g.unit_mut(original).unwrap().lane = Some(0);
        g.unit_mut(original).unwrap().ready = true;
        g.unit_mut(original).unwrap().damage = 1;
        g.battlefield_owner[0] = Some(actor);
        g.apply(Action::ActivateAbility {
            source: AbilitySource::Unit {
                player: actor,
                unit_id,
            },
            ability_index: 0,
            targets: vec![],
        })
        .unwrap();
        let current = g
            .take_control_of_battlefield_unit(1 - actor, original)
            .unwrap();
        settle_unopposed_chain(&mut g);
        let unit = g.unit_ref(current).unwrap();
        assert_eq!(unit.lane, None, "the physical source must still resolve");
        assert!(unit.ready);
        assert_eq!(unit.damage, 1);
        assert!(
            !g.unit_moved_this_turn(1 - actor, unit_id),
            "an invalid destination instead Recalls (449.2.c)"
        );
        assert_eq!(g.board_owner(current), Some(actor));
    }
}

#[test]
fn tideturner_preserves_both_physical_locations_after_its_source_changes_control() {
    for actor in 0..2 {
        for recall_source in [false, true] {
            for blocked in [false, true] {
                for transfer_before_choice in [false, true] {
                    let mut g = position(actor, 0, 468502);
                    let other = g.create_unit(actor, 71, false, 0);
                    let target = Target::Unit {
                        player: actor,
                        unit_id: other,
                    };
                    g.unit_mut(target).unwrap().lane = recall_source.then_some(1);
                    g.unit_mut(target).unwrap().ready = true;
                    g.battlefield_owner[0] = Some(actor);
                    g.players[actor].hand = vec![18];
                    g.apply(Action::PlayCardWithTargets {
                        hand_index: 0,
                        lane: Some(0),
                        targets: vec![],
                    })
                    .unwrap();
                    let source_id = g.players[actor]
                        .units
                        .iter()
                        .find(|u| u.card_id == 18)
                        .unwrap()
                        .id;
                    assert!(matches!(g.decision, Some(Decision::SwapUnit { .. })));
                    if !transfer_before_choice {
                        g.apply(Action::ChooseTarget {
                            target: Some(target),
                        })
                        .unwrap();
                    }
                    let source = g
                        .take_control_of_battlefield_unit(
                            1 - actor,
                            Target::Unit {
                                player: actor,
                                unit_id: source_id,
                            },
                        )
                        .unwrap();
                    if recall_source {
                        g.recall_unit(1 - actor, source_id, false, false);
                    }
                    if transfer_before_choice {
                        g.apply(Action::ChooseTarget {
                            target: Some(target),
                        })
                        .unwrap();
                    }
                    if blocked {
                        g.movement_locks.push((actor, source_id));
                    }
                    settle_unopposed_chain(&mut g);
                    let source_destination = if blocked {
                        (!recall_source).then_some(0)
                    } else {
                        recall_source.then_some(1)
                    };
                    assert_eq!(g.unit_ref(source).unwrap().lane, source_destination);
                    assert_eq!(
                        g.unit_ref(target).unwrap().lane,
                        (!recall_source).then_some(0)
                    );
                    assert_eq!(
                        g.unit_moved_this_turn(1 - actor, source_id),
                        recall_source && !blocked
                    );
                    assert_eq!(g.unit_moved_this_turn(actor, other), !recall_source);
                    assert!(g.unit_ref(target).unwrap().ready);
                }
            }
        }
    }
}

#[test]
fn forced_movement_captures_actor_and_physical_locations_before_control_changes() {
    use board_targets::BoardLocation;
    for actor in 0..2 {
        let controller = 1 - actor;
        let mut g = position(actor, 0, 468503);
        let unit_id = g.create_unit(controller, 288, false, 0); // Akali's Move trigger.
        let target = Target::Unit {
            player: controller,
            unit_id,
        };
        g.move_effect(actor, target, Some(0), false).unwrap();
        g.after_resolution().unwrap();
        let Some(Decision::TriggerTargets {
            player, context, ..
        }) = g.decision
        else {
            panic!("expected Akali's real Move trigger: {:?}", g.decision);
        };
        assert_eq!(player, controller);
        let expected = triggers::MoveFacts {
            actor,
            origin: BoardLocation::Base { player: controller },
            destination: BoardLocation::Battlefield { lane: 0 },
        };
        assert_eq!(context.movement, Some(expected));
        let current = g.take_control_of_battlefield_unit(actor, target).unwrap();
        g.recall_unit(actor, unit_id, false, false);
        assert_eq!(g.unit_ref(current).unwrap().lane, None);
        let Some(Decision::TriggerTargets { context, .. }) = g.decision else {
            unreachable!();
        };
        assert_eq!(
            context.movement,
            Some(expected),
            "captured base does not change with the controller"
        );
    }
}

#[test]
fn prevented_move_and_invalid_destination_are_distinct_and_ready_still_executes() {
    use board_targets::BoardLocation;
    for actor in 0..2 {
        let controller = 1 - actor;
        let mut g = position(actor, 0, 468504);
        let unit_id = g.create_unit(controller, 71, false, 0);
        let target = Target::Unit {
            player: controller,
            unit_id,
        };
        g.unit_mut(target).unwrap().lane = Some(0);
        g.unit_mut(target).unwrap().stunned = true;
        g.movement_locks.push((actor, unit_id));
        let destination = BoardLocation::Base { player: actor };
        g.move_effect_at(actor, target, destination, true).unwrap();
        assert_eq!(
            g.unit_ref(target).unwrap().lane,
            Some(0),
            "a prohibited Move cannot turn into a Recall"
        );
        assert!(g.unit_ref(target).unwrap().ready);
        g.movement_locks.clear();
        g.move_effect_at(actor, target, destination, false).unwrap();
        assert_eq!(g.unit_ref(target).unwrap().lane, None);
        assert!(g.unit_ref(target).unwrap().stunned);
        assert!(!g.unit_moved_this_turn(controller, unit_id));
        g.unit_mut(target).unwrap().ready = false;
        g.move_effect_at(
            actor,
            target,
            BoardLocation::Base { player: controller },
            true,
        )
        .unwrap();
        assert!(
            g.unit_ref(target).unwrap().ready,
            "Ready still executes when already at the destination"
        );
        assert!(!g.unit_moved_this_turn(controller, unit_id));
        assert!(g.chain.is_empty());
    }
}
