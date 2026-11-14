//! OGN-070; CR355.17.b explicitly makes Warden override play permissions.
use super::*;
use test_support::{position, ready_unit, reject_unchanged};

const WARDEN: CardId = 464;

fn board(actor: usize) -> (Game, u32) {
    let mut g = position(actor, 0, 464001);
    let warden = ready_unit(&mut g, 1 - actor, WARDEN);
    g.unit_mut(Target::Unit {
        player: 1 - actor,
        unit_id: warden,
    })
    .unwrap()
    .lane = Some(1);
    let holder = ready_unit(&mut g, actor, 71);
    g.unit_mut(Target::Unit {
        player: actor,
        unit_id: holder,
    })
    .unwrap()
    .lane = Some(0);
    g.battlefield_owner = vec![Some(actor), Some(1 - actor)];
    (g, warden)
}

fn hand(lane: Option<usize>, targets: Vec<Target>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane,
        targets,
    }
}

fn settle(g: &mut Game) {
    for _ in 0..50 {
        if g.decision.is_none() && g.chain.is_empty() && g.showdown.is_none() {
            return;
        }
        g.apply(test_support::decline_legend_action(g).unwrap_or(Action::Pass))
            .unwrap();
    }
    panic!("resolution did not finish: {:?}", g.decision);
}

#[test]
fn warden_blocks_unit_plays_at_every_field_but_allows_base_gear_and_movement() {
    for actor in 0..2 {
        let (mut g, _) = board(actor);
        g.players[actor].hand = vec![71, 401];
        for lane in 0..2 {
            reject_unchanged(&mut g, hand(Some(lane), vec![]));
            assert!(
                !g.legal_actions()
                    .unwrap()
                    .contains(&hand(Some(lane), vec![]))
            );
            g.players[actor].champion = Some(0);
            reject_unchanged(
                &mut g,
                Action::PlayChampion {
                    lane: Some(lane),
                    target: None,
                },
            );
        }
        g.apply(hand(None, vec![])).unwrap();
        assert_eq!(g.players[actor].units.last().unwrap().lane, None);
        g.apply(hand(Some(0), vec![])).unwrap();
        assert_eq!(g.players[actor].gear[0].lane, Some(0));
        let runner = ready_unit(&mut g, actor, 71);
        g.apply(Action::Move {
            unit_id: runner,
            lane: Some(0),
        })
        .unwrap();
        settle(&mut g);
        assert_eq!(
            g.unit_ref(Target::Unit {
                player: actor,
                unit_id: runner
            })
            .unwrap()
            .lane,
            Some(0)
        );
    }
}

#[test]
fn warden_restrictions_follow_live_location_controller_and_all_sources() {
    for actor in 0..2 {
        let (mut g, warden) = board(actor);
        let second = ready_unit(&mut g, 1 - actor, WARDEN);
        g.unit_mut(Target::Unit {
            player: 1 - actor,
            unit_id: second,
        })
        .unwrap()
        .lane = Some(0);
        assert_eq!(g.restrictions_affecting(actor).len(), 4);
        assert!(g.restrictions_affecting(1 - actor).is_empty());
        g.players[actor].hand = vec![71];
        g.move_effect(
            1 - actor,
            Target::Unit {
                player: 1 - actor,
                unit_id: warden,
            },
            None,
            false,
        )
        .unwrap();
        assert_eq!(g.restrictions_affecting(actor).len(), 2);
        reject_unchanged(&mut g, hand(Some(0), vec![]));
        g.move_effect(
            1 - actor,
            Target::Unit {
                player: 1 - actor,
                unit_id: second,
            },
            None,
            false,
        )
        .unwrap();
        assert!(g.restrictions_affecting(actor).is_empty());
        g.apply(hand(Some(0), vec![])).unwrap();
    }
}

#[test]
fn warden_blocks_hidden_units_and_fixed_tokens_without_redirecting_them() {
    for actor in 0..2 {
        let (mut g, _) = board(actor);
        g.players[actor].hand = vec![108];
        g.apply(Action::Hide {
            hand_index: 0,
            lane: 0,
        })
        .unwrap();
        g.players[actor].hidden[0].hidden_turn = 0;
        let hidden_id = g.players[actor].hidden[0].id;
        reject_unchanged(
            &mut g,
            Action::PlayHidden {
                hidden_id,
                targets: vec![],
            },
        );
        g.queue_token_play(actor, 49, 2, Some(Some(0)), false);
        g.after_resolution().unwrap();
        assert!(g.pending_tokens().is_empty());
        assert_eq!(g.players[actor].units.len(), 1);
        g.queue_token_play(actor, 49, 2, None, false);
        g.after_resolution().unwrap();
        assert!(
            g.legal_actions()
                .unwrap()
                .contains(&Action::PlaceToken { lane: None })
        );
        for lane in 0..2 {
            reject_unchanged(&mut g, Action::PlaceToken { lane: Some(lane) });
        }
        g.apply(Action::PlaceToken { lane: None }).unwrap();
        assert_eq!(g.players[actor].units.len(), 2);
    }
}

#[test]
fn warden_prevents_ready_but_not_independent_might_and_draw_effects() {
    for actor in 0..2 {
        for card in [312, 425] {
            let (mut g, _) = board(actor);
            let unit_id = ready_unit(&mut g, actor, 71);
            let target = Target::Unit {
                player: actor,
                unit_id,
            };
            g.unit_mut(target).unwrap().ready = false;
            let might = g.actual_unit_might(g.unit_ref(target).unwrap()).unwrap();
            g.players[actor].hand = vec![card];
            g.apply(hand(None, if card == 312 { vec![] } else { vec![target] }))
                .unwrap();
            settle(&mut g);
            assert!(!g.unit_ref(target).unwrap().ready);
            if card == 312 {
                assert_eq!(
                    g.actual_unit_might(g.unit_ref(target).unwrap()).unwrap(),
                    might + 3
                );
            } else {
                assert_eq!(g.players[actor].hand.len(), 1);
                assert!(g.movement_locked(actor, unit_id));
            }
        }
    }
}

#[test]
fn warden_irelia_costs_are_paid_but_prevented_ready_has_no_growth_event() {
    for actor in 0..2 {
        let (mut g, _) = board(actor);
        let unit_id = ready_unit(&mut g, actor, 0);
        let target = Target::Unit {
            player: actor,
            unit_id,
        };
        g.unit_mut(target).unwrap().ready = false;
        g.legend[actor].ready = true;
        g.players[actor].hand = vec![7];
        g.apply(hand(None, vec![target])).unwrap();
        for _ in 0..10 {
            if matches!(g.decision, Some(Decision::Legend { .. })) {
                break;
            }
            let order = g
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|a| matches!(a, Action::OrderTrigger { .. }))
                .unwrap();
            g.apply(order).unwrap();
        }
        assert!(matches!(g.decision, Some(Decision::Legend { .. })));
        let power = g.players[actor].power_pool.iter().sum::<u16>();
        g.apply(Action::Choose { accept: true }).unwrap();
        assert!(!g.legend[actor].ready);
        assert_eq!(g.players[actor].power_pool.iter().sum::<u16>(), power - 1);
        settle(&mut g);
        assert!(!g.unit_ref(target).unwrap().ready);
        assert_eq!(
            g.actual_unit_might(g.unit_ref(target).unwrap()).unwrap(),
            4 + 1 + 2,
            "printed Might, one chosen trigger, Discipline; no Ready trigger"
        );
    }
}

#[test]
fn warden_does_not_block_awakening_or_accelerated_entry() {
    for actor in 0..2 {
        let (mut g, _) = board(actor);
        g.players[actor].hand = vec![64];
        g.apply(Action::WithPlayChoices {
            action: Box::new(hand(None, vec![])),
            choices: PlayChoices {
                accelerate: true,
                ..Default::default()
            },
        })
        .unwrap();
        settle(&mut g);
        assert!(
            g.players[actor].units.last().unwrap().ready,
            "entering ready is not a Ready action"
        );
        g.players[actor].units.last_mut().unwrap().ready = false;
        let gear_id = g.gear_gold(actor, true);
        let gear = Target::Gear {
            player: actor,
            gear_id,
        };
        g.ready_object_by(actor, gear);
        assert!(
            !g.players[actor].gear[0].ready,
            "effect Ready is prohibited"
        );
        g.apply(Action::EndTurn).unwrap();
        settle(&mut g);
        g.apply(Action::EndTurn).unwrap();
        settle(&mut g);
        assert_eq!(g.active, actor);
        assert!(g.players[actor].units.last().unwrap().ready);
        assert!(g.players[actor].gear[0].ready);
    }
}

#[test]
fn warden_leaving_in_response_allows_the_waiting_ready_to_resolve() {
    for actor in 0..2 {
        let (mut g, warden) = board(actor);
        let id = ready_unit(&mut g, actor, 71);
        let target = Target::Unit {
            player: actor,
            unit_id: id,
        };
        g.unit_mut(target).unwrap().ready = false;
        g.players[actor].hand = vec![425];
        g.players[1 - actor].hand = vec![9]; // Flash: move own Warden to base.
        g.apply(hand(None, vec![target])).unwrap();
        while let Some(action) = test_support::decline_legend_action(&g) {
            g.apply(action).unwrap();
        }
        g.apply(Action::Pass).unwrap();
        assert_eq!(g.priority(), 1 - actor);
        g.apply(hand(
            None,
            vec![Target::Unit {
                player: 1 - actor,
                unit_id: warden,
            }],
        ))
        .unwrap();
        settle(&mut g);
        assert!(g.unit_ref(target).unwrap().ready);
        assert!(g.restrictions_affecting(actor).is_empty());
        assert_eq!(g.players[actor].hand.len(), 1);
    }
}

#[test]
fn warden_stops_a_fixed_instructed_replay_but_leaves_the_banish_intact() {
    for actor in 0..2 {
        let (mut g, _) = board(actor);
        let target = Target::Unit {
            player: actor,
            unit_id: g.players[actor].units[0].id,
        };
        g.players[actor].hand = vec![381];
        g.apply(hand(None, vec![target])).unwrap();
        for _ in 0..24 {
            if matches!(g.decision, Some(Decision::PendingPlay { .. })) {
                break;
            }
            g.apply(test_support::decline_legend_action(&g).unwrap_or(Action::Pass))
                .unwrap();
        }
        assert!(matches!(g.decision, Some(Decision::PendingPlay { .. })));
        assert_eq!(
            g.legal_actions().unwrap(),
            vec![Action::Choose { accept: false }]
        );
        for lane in [None, Some(0), Some(1)] {
            reject_unchanged(
                &mut g,
                Action::FinalizePendingPlay {
                    lane,
                    targets: vec![],
                    repeat_targets: vec![],
                    repeat: false,
                    accelerate: false,
                    bonus: false,
                },
            );
        }
        g.apply(Action::Choose { accept: false }).unwrap();
        assert_eq!(g.players[actor].banish, vec![71]);
        assert!(g.unit_ref(target).is_none());
    }
}

#[test]
fn warden_ai_declines_a_useless_irelia_ready_without_peeking() {
    for actor in 0..2 {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let (mut g, _) = board(actor);
            g.difficulty = difficulty;
            let id = ready_unit(&mut g, actor, 71);
            g.unit_mut(Target::Unit {
                player: actor,
                unit_id: id,
            })
            .unwrap()
            .ready = false;
            g.legend[actor].ready = true;
            g.players[actor].hand = vec![7];
            g.apply(hand(
                None,
                vec![Target::Unit {
                    player: actor,
                    unit_id: id,
                }],
            ))
            .unwrap();
            assert!(matches!(g.decision, Some(Decision::Legend { .. })));
            assert_eq!(
                g.choose_ai_action().unwrap(),
                Action::Choose { accept: false }
            );
            g.players[1 - actor].hand = vec![9, 425];
            g.players[1 - actor].deck.reverse();
            assert_eq!(
                g.choose_ai_action().unwrap(),
                Action::Choose { accept: false }
            );
        }
    }
}
