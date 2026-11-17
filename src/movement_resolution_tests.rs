//! Movement destinations are finalized choices, not extra unit restrictions.
use super::*;
use test_support::{decline_legend_action, position, settle_unopposed_chain};

fn respond_as(game: &mut Game, player: usize) {
    while let Some(action) = decline_legend_action(game) {
        game.apply(action).unwrap();
    }
    assert!(game.decision.is_none());
    if game.priority() != player {
        game.apply(Action::Pass).unwrap();
    }
    assert_eq!(game.priority(), player);
}

#[test]
fn ride_the_wind_readies_after_flash_reaches_its_chosen_destination() {
    // Core 355.4 fixes a valid destination at finalization. On resolution,
    // 359.3.e.2 rechecks the friendly-unit target, and 359.3.e.6 / .14 allow
    // Ready when the earlier Move cannot happen for a non-target reason.
    for actor in 0..2 {
        let mut game = position(actor, 0, 486101);
        let unit_id = game.create_unit(actor, 71, false, 0);
        let target = Target::Unit {
            player: actor,
            unit_id,
        };
        game.unit_mut(target).unwrap().lane = Some(0);
        game.battlefield_owner[0] = Some(actor);
        game.players[actor].hand = vec![14, 9];
        assert!(!game.unit_ref(target).unwrap().ready);
        game.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        })
        .unwrap();
        respond_as(&mut game, actor);
        game.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        })
        .unwrap();
        respond_as(&mut game, actor);
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert_eq!(game.unit_ref(target).unwrap().lane, None);
        assert!(
            !game.unit_ref(target).unwrap().ready,
            "Flash does not ready"
        );
        assert!(game.chain.iter().any(|item| item.card_id == 14));
        settle_unopposed_chain(&mut game);
        assert!(
            game.unit_ref(target).unwrap().ready,
            "Ride the Wind still readies its legal friendly-unit target"
        );
        assert!(game.players[actor].discard.contains(&14));
        assert!(game.players[actor].discard.contains(&9));
    }
}

#[test]
fn ride_the_wind_readies_after_opposing_sirens_song_reaches_the_chosen_field() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 486102);
        let unit_id = test_support::ready_unit(&mut game, actor, 71);
        let target = Target::Unit {
            player: actor,
            unit_id,
        };
        game.players[actor].hand = vec![14];
        game.players[1 - actor].hand = vec![427];
        game.apply(Action::Move {
            unit_id,
            lane: Some(0),
        })
        .unwrap();
        assert!(game.unit_moved_this_turn(actor, unit_id));
        respond_as(&mut game, actor);
        game.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: Some(1),
            targets: vec![target],
        })
        .unwrap();
        respond_as(&mut game, 1 - actor);
        game.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: Some(1),
            targets: vec![target],
        })
        .unwrap();
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert_eq!(game.unit_ref(target).unwrap().lane, Some(1));
        assert!(!game.unit_ref(target).unwrap().ready);
        settle_unopposed_chain(&mut game);
        assert_eq!(game.unit_ref(target).unwrap().lane, Some(1));
        assert!(game.unit_ref(target).unwrap().ready);
        assert!(game.players[1 - actor].discard.contains(&427));
    }
}

#[test]
fn movement_destinations_must_still_differ_when_declaring_targets() {
    for actor in 0..2 {
        for card in [4, 9, 14, 427] {
            for lane in [None, Some(0)] {
                if card == 9 && lane.is_some() {
                    continue;
                }
                let mut game = position(actor, 0, 486103);
                let controller = if card == 4 || card == 427 {
                    1 - actor
                } else {
                    actor
                };
                let unit_id = game.create_unit(controller, 71, false, 0);
                let target = Target::Unit {
                    player: controller,
                    unit_id,
                };
                game.unit_mut(target).unwrap().lane = lane;
                // This test isolates destination validation from Siren's Song's
                // separate moved-this-turn restriction.
                game.moved_units.push(unit_id);
                game.players[actor].hand = vec![card];
                assert!(
                    !game
                        .valid_card_target(actor, card, lane, target, &[])
                        .unwrap()
                );
                assert!(
                    !game
                        .card_target_candidates(actor, card, lane, &[])
                        .unwrap()
                        .contains(&target)
                );
                test_support::reject_unchanged(
                    &mut game,
                    Action::PlayCardWithTargets {
                        hand_index: 0,
                        lane,
                        targets: vec![target],
                    },
                );
                test_support::reject_unchanged(
                    &mut game,
                    Action::PlayCard {
                        hand_index: 0,
                        lane,
                        target: Some(target),
                    },
                );
            }
        }
    }
}

#[test]
fn a_prevented_move_is_playable_and_still_readies_but_a_departed_target_does_not() {
    for actor in 0..2 {
        for bounce in [false, true] {
            let mut game = position(actor, 0, 486104);
            // Watchful Sentry has 1 Might and is a legal Gust target.
            let unit_id = game.create_unit(actor, 68, false, 0);
            let target = Target::Unit {
                player: actor,
                unit_id,
            };
            game.unit_mut(target).unwrap().lane = Some(0);
            game.battlefield_owner[0] = Some(actor);
            game.players[actor].hand = vec![14];
            game.players[1 - actor].hand = vec![11];
            if !bounce {
                game.movement_locks.push((actor, unit_id));
            }
            assert!(
                game.valid_card_target(actor, 14, None, target, &[])
                    .unwrap()
            );
            game.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![target],
            })
            .unwrap();
            if bounce {
                respond_as(&mut game, 1 - actor);
                game.apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![target],
                })
                .unwrap();
            }
            settle_unopposed_chain(&mut game);
            if bounce {
                assert!(game.unit_ref(target).is_none());
                assert!(game.players[actor].hand.contains(&68));
            } else {
                let unit = game.unit_ref(target).unwrap();
                assert_eq!(unit.lane, Some(0));
                assert!(unit.ready);
                assert!(!game.unit_moved_this_turn(actor, unit_id));
            }
        }
    }
}
