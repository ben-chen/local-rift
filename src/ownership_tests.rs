//! Authored foreign-control positions isolate CR056 destinations. These are
//! not evidence that any control-changing card or attachment flow is playable.
use super::*;
use crate::game::test_support::{position, reject_unchanged};

fn controlled_unit(g: &mut Game, owner: usize, card: CardId, token: bool) -> Target {
    let controller = 1 - owner;
    let unit_id = g.create_unit(owner, card, token, if token { 3 } else { 0 });
    let unit = g.players[owner].units.pop().unwrap();
    g.players[controller].units.push(unit);
    g.board_owner_overrides.insert(unit_id, owner);
    Target::Unit {
        player: controller,
        unit_id,
    }
}

fn controlled_gear(g: &mut Game, owner: usize, token: bool) -> Target {
    let controller = 1 - owner;
    if token {
        g.gear_gold(owner, false);
    } else {
        g.gear_play_at(owner, 3, None).unwrap();
    }
    let gear = g.players[owner].gear.pop().unwrap();
    let gear_id = gear.id;
    g.players[controller].gear.push(gear);
    g.board_owner_overrides.insert(gear_id, owner);
    Target::Gear {
        player: controller,
        gear_id,
    }
}

#[test]
fn unit_departures_use_owner_and_remove_physical_ownership_even_for_tokens() {
    for owner in 0..2 {
        for token in [false, true] {
            for departure in 0..4 {
                let mut g = position(owner, 0, 468001);
                let target = controlled_unit(&mut g, owner, 71, token);
                assert_eq!(g.board_owner(target), Some(owner));
                match departure {
                    0 => assert_eq!(g.return_unit_to_hand(target), Some(owner)),
                    1 => assert_eq!(g.banish_unit_directly(target), Some(owner)),
                    2 => g.kill_unit(target).unwrap(),
                    3 => {
                        g.unit_mut(target).unwrap().banish_on_death = true;
                        g.kill_unit(target).unwrap();
                    }
                    _ => unreachable!(),
                }
                let expected = if token { vec![] } else { vec![71] };
                let state = &g.players[owner];
                assert_eq!(
                    match departure {
                        0 => &state.hand,
                        2 => &state.discard,
                        _ => &state.banish,
                    },
                    &expected
                );
                let controller = &g.players[1 - owner];
                assert!(
                    controller.hand.is_empty()
                        && controller.discard.is_empty()
                        && controller.banish.is_empty()
                );
                assert!(g.board_owner_overrides.is_empty());
                assert_eq!(g.board_owner(target), None);
            }
        }
    }
}

#[test]
fn gear_departures_use_owner_and_tokens_never_become_factory_cards() {
    for owner in 0..2 {
        for token in [false, true] {
            for departure in 0..3 {
                let mut g = position(owner, 0, 468002);
                let target = controlled_gear(&mut g, owner, token);
                assert_eq!(g.board_owner(target), Some(owner));
                match departure {
                    0 => g.return_board_targets(&[target]),
                    1 => g.banish_gear_directly(target),
                    2 => {
                        g.kill_gear_target(target);
                    }
                    _ => unreachable!(),
                }
                let expected = if token { vec![] } else { vec![3] };
                let state = &g.players[owner];
                assert_eq!(
                    match departure {
                        0 => &state.hand,
                        1 => &state.banish,
                        _ => &state.discard,
                    },
                    &expected
                );
                let controller = &g.players[1 - owner];
                assert!(
                    controller.hand.is_empty()
                        && controller.discard.is_empty()
                        && controller.banish.is_empty()
                );
                assert!(g.board_owner_overrides.is_empty());
                assert_eq!(g.board_owner(target), None);
            }
        }
    }
}

#[test]
fn recall_and_rejected_actions_preserve_ownership_and_existing_object_status() {
    for owner in 0..2 {
        let mut g = position(owner, 0, 468003);
        let target = controlled_unit(&mut g, owner, 71, false);
        let Target::Unit { player, unit_id } = target else {
            unreachable!()
        };
        let unit = g.unit_mut(target).unwrap();
        unit.lane = Some(0);
        unit.damage = 1;
        unit.buff = 2;
        unit.ready = true;
        g.recall_unit(player, unit_id, false, false);
        let unit = g.unit_ref(target).unwrap();
        assert_eq!(
            (unit.lane, unit.damage, unit.buff, unit.ready),
            (None, 1, 2, true)
        );
        assert_eq!(g.board_owner(target), Some(owner));
        reject_unchanged(
            &mut g,
            Action::Move {
                unit_id,
                lane: Some(900),
            },
        );
        assert_eq!(g.clone().board_owner(target), Some(owner));
        assert_eq!(g.return_unit_to_hand(target), Some(owner));
        let replay = g.create_unit(owner, 71, false, 0);
        assert_ne!(unit_id, replay);
        assert_eq!(
            g.board_owner(Target::Unit {
                player: owner,
                unit_id: replay
            }),
            Some(owner)
        );
        assert!(g.board_owner_overrides.is_empty());
    }
}

#[test]
fn death_trigger_keeps_controller_while_the_card_goes_to_owners_trash() {
    for owner in 0..2 {
        let mut g = position(owner, 0, 468004);
        let target = controlled_unit(&mut g, owner, 68, false);
        g.kill_unit(target).unwrap();
        assert_eq!(g.players[owner].discard, [68]);
        assert!(g.players[1 - owner].discard.is_empty());
        assert!(
            g.pending_triggers
                .iter()
                .any(|trigger| { trigger.player == 1 - owner && trigger.option.card_id == 68 })
        );
        assert!(
            !g.pending_triggers
                .iter()
                .any(|trigger| { trigger.player == owner && trigger.option.card_id == 68 })
        );
    }
}

#[test]
fn death_replacement_that_keeps_the_unit_preserves_its_owner() {
    for owner in 0..2 {
        let controller = 1 - owner;
        let mut g = position(owner, 0, 468006);
        let target = controlled_unit(&mut g, owner, 71, false);
        let Target::Unit { unit_id, .. } = target else {
            unreachable!()
        };
        g.unit_mut(target).unwrap().lane = Some(0);
        g.gear_play_at(controller, 10, None).unwrap();
        g.players[controller].gear.last_mut().unwrap().attached_to = Some(unit_id);
        g.kill_unit(target).unwrap();
        assert_eq!(g.board_owner(target), Some(owner));
        assert_eq!(g.unit_ref(target).unwrap().lane, None);
        assert_eq!(g.players[controller].discard, [10]);
        assert!(g.players[owner].discard.is_empty());
        assert_eq!(g.return_unit_to_hand(target), Some(owner));
        assert_eq!(g.players[owner].hand, [71]);
        assert!(g.board_owner_overrides.is_empty());
    }
}

#[test]
fn banish_replay_uses_original_owner_and_exact_nonboard_occurrence() {
    for owner in 0..2 {
        let mut g = position(owner, 0, 468005);
        g.players[owner].banish = vec![72];
        let target = controlled_unit(&mut g, owner, 71, false);
        g.unit_mut(target).unwrap().lane = Some(1);
        g.banish_and_replay(target, InstructedPlayKind::TemporalBreach);
        let pending = g.pending_cards();
        assert_eq!(pending.len(), 1);
        assert_eq!((pending[0].player, pending[0].card_id), (owner, 71));
        assert_eq!(g.players[owner].banish, [72]);
        assert!(g.players[1 - owner].banish.is_empty());
        assert!(g.board_owner_overrides.is_empty());
    }
}

#[test]
fn borrowed_unit_replay_distinguishes_exact_opposing_base_from_owner_base() {
    use crate::game::board_targets::BoardLocation;
    use crate::game::test_support::decline_legend_action;
    for owner in 0..2 {
        for spell in [380, 381] {
            let controller = 1 - owner;
            let mut g = position(controller, 0, 468006);
            let target = controlled_unit(&mut g, owner, 71, false);
            g.players[owner].banish = vec![72];
            g.players[controller].hand = vec![spell];
            g.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![target],
            })
            .unwrap();
            for _ in 0..80 {
                if matches!(g.decision, Some(Decision::PendingPlay { .. })) {
                    break;
                }
                let action = if let Some(action) = decline_legend_action(&g) {
                    action
                } else if let Some(Decision::OrderTriggers { options, .. }) = &g.decision {
                    Action::OrderTrigger {
                        trigger_id: options[0].id,
                    }
                } else {
                    assert!(g.decision.is_none(), "unexpected {:?}", g.decision);
                    Action::Pass
                };
                g.apply(action).unwrap();
            }
            assert!(
                matches!(g.decision, Some(Decision::PendingPlay { player, .. }) if player == owner)
            );
            assert_eq!(g.players[owner].banish, [72]);
            assert!(g.players[controller].units.is_empty());
            let finalize = |lane| Action::FinalizePendingPlay {
                lane,
                targets: vec![],
                repeat_targets: vec![],
                repeat: false,
                accelerate: false,
                bonus: false,
            };
            if spell == 381 {
                assert_eq!(
                    g.pending_cards()[0].instruction.destination,
                    PlayDestination::Exact(BoardLocation::Base { player: controller })
                );
                for lane in [None, Some(0), Some(1)] {
                    reject_unchanged(&mut g, finalize(lane));
                }
                assert!(g.can_decline_trash_play().unwrap());
                assert!(
                    g.legal_actions()
                        .unwrap()
                        .contains(&Action::Choose { accept: false })
                );
                let mut replay = g.clone();
                let resources = (
                    g.players[owner].energy,
                    g.players[owner].power_pool,
                    g.players[owner].xp,
                );
                g.apply(Action::Choose { accept: false }).unwrap();
                replay.apply(Action::Choose { accept: false }).unwrap();
                assert_eq!(format!("{g:?}"), format!("{replay:?}"));
                assert_eq!(
                    (
                        g.players[owner].energy,
                        g.players[owner].power_pool,
                        g.players[owner].xp
                    ),
                    resources
                );
                assert_eq!(g.players[owner].banish, [72, 71]);
                assert!(g.players[owner].units.is_empty());
            } else {
                assert_eq!(
                    g.pending_cards()[0].instruction.destination,
                    PlayDestination::Base
                );
                assert!(!g.can_decline_trash_play().unwrap());
                g.apply(finalize(None)).unwrap();
                let fresh = &g.players[owner].units[0];
                assert_eq!((fresh.card_id, fresh.lane), (71, None));
                assert_eq!(
                    g.board_owner(Target::Unit {
                        player: owner,
                        unit_id: fresh.id
                    }),
                    Some(owner)
                );
            }
        }
    }
}
