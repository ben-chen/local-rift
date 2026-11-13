//! Controlled board/resource fixtures followed by real card and choice actions.
//! The fixtures are not claimed to be complete registered tournament decks.
use super::*;
const GRIP: CardId = 447;

fn position(actor: usize) -> Game {
    let mut game = Game::new_for_agents(3, 3, 447163, Difficulty::Tactical);
    game.choose_starting_player(actor).unwrap();
    game.start_match().unwrap();
    game.battlefields = vec![9, 9];
    game.battlefield_owner = vec![None; 2];
    for player in &mut game.players {
        player.units.clear();
        player.gear.clear();
        player.hand.clear();
        player.hidden.clear();
        player.discard.clear();
        player.banish.clear();
        player.champion = None;
        player.deck = vec![71; 20];
        player.set_rune_setup([0; 6], [0; 6]).unwrap();
        player.energy = 100;
        player.energy_pool = 100;
        player.power_pool = [20; 6];
    }
    for legend in &mut game.legend {
        legend.ready = false;
    }
    game
}

fn unit(game: &mut Game, player: usize, lane: Option<usize>) -> Target {
    let unit_id = game.create_unit(player, 69, false, 0);
    let unit = game.players[player].units.last_mut().unwrap();
    unit.lane = lane;
    unit.ready = true;
    Target::Unit { player, unit_id }
}

fn pair(game: &mut Game) {
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
}

fn drain(game: &mut Game) {
    for _ in 0..30 {
        assert!(game.decision.is_none(), "{:?}", game.decision);
        if game.chain.is_empty() {
            return;
        }
        game.apply(Action::Pass).unwrap();
    }
    panic!("Chain did not finish");
}

fn cast(game: &mut Game, targets: Vec<Target>) {
    let actor = game.priority();
    let hand_index = game.players[actor].hand.len();
    game.players[actor].hand.push(GRIP);
    game.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
}

#[test]
fn both_units_are_distinct_friendly_targets_and_kill_waits_for_responses() {
    for actor in 0..2 {
        let mut g = position(actor);
        let sacrifice = unit(&mut g, actor, None);
        let recipient = unit(&mut g, actor, Some(0));
        let enemy = unit(&mut g, 1 - actor, Some(1));
        g.players[actor].hand.push(GRIP);
        for targets in [
            vec![sacrifice],
            vec![sacrifice, sacrifice],
            vec![enemy, recipient],
            vec![sacrifice, enemy],
        ] {
            let before = format!("{g:?}");
            assert!(
                g.apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets
                })
                .is_err()
            );
            assert_eq!(format!("{g:?}"), before);
        }
        let cost = g.players[actor].energy;
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![sacrifice, recipient],
        })
        .unwrap();
        assert_eq!(g.players[actor].energy, cost - 2);
        assert!(g.unit_ref(sacrifice).is_some());
        assert_eq!(g.unit_ref(recipient).unwrap().temporary_buff, 0);
        // A real response changes the amount before the Kill instruction.
        g.players[actor].hand.push(7); // Response: +2 Might and Draw1.
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![sacrifice],
        })
        .unwrap();
        pair(&mut g);
        let amount = g.unit_might(g.unit_ref(sacrifice).unwrap()).unwrap();
        drain(&mut g);
        assert!(g.unit_ref(sacrifice).is_none());
        assert_eq!(g.unit_ref(recipient).unwrap().temporary_buff, amount as i64);
        assert_eq!(g.players[actor].hand, vec![71, 71]);
        assert!(g.pending_deathgrip.is_none());
        let ledger = g.last_damage_ledger().unwrap();
        assert!(ledger.damage.is_empty());
        assert_eq!(ledger.kills.len(), 1);
        assert_eq!(ledger.kills[0].target, sacrifice);
    }
}

#[test]
fn either_target_can_leave_without_reassigning_slots_and_draw_is_independent() {
    for actor in 0..2 {
        for lost_slot in 0..2 {
            let mut g = position(actor);
            let targets = [unit(&mut g, actor, Some(0)), unit(&mut g, actor, Some(0))];
            cast(&mut g, targets.to_vec());
            g.players[actor].hand.push(377); // Retreat returns a friendly unit to hand.
            g.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![targets[lost_slot]],
            })
            .unwrap();
            drain(&mut g);
            assert_eq!(g.players[actor].hand, vec![69, 71]);
            if lost_slot == 0 {
                assert_eq!(g.unit_ref(targets[1]).unwrap().temporary_buff, 0);
                assert!(g.last_damage_ledger().unwrap().kills.is_empty());
            } else {
                assert!(g.unit_ref(targets[0]).is_none());
                assert_eq!(g.last_damage_ledger().unwrap().kills.len(), 1);
            }
        }
    }
}

#[test]
fn ordered_death_replacement_pauses_draw_and_prevents_might_transfer() {
    for actor in 0..2 {
        let mut g = position(actor);
        let sacrifice = unit(&mut g, actor, Some(0));
        let recipient = unit(&mut g, actor, None);
        g.gear_play(actor, 21, None).unwrap();
        g.gear_play(actor, 21, None).unwrap();
        cast(&mut g, vec![sacrifice, recipient]);
        pair(&mut g);
        let Some(Decision::DeathReplacement { player, options }) = g.decision.clone() else {
            panic!("{:?}", g.decision)
        };
        assert_eq!(player, actor);
        assert_eq!(options.len(), 2);
        assert!(g.players[actor].hand.is_empty());
        assert!(g.pending_deathgrip.is_some());
        let before = format!("{g:?}");
        assert!(
            g.apply(Action::ChooseDeathReplacement {
                unit_id: u32::MAX,
                gear_id: options[0].gear_id
            })
            .is_err()
        );
        assert_eq!(format!("{g:?}"), before);
        g.apply(Action::ChooseDeathReplacement {
            unit_id: options[0].unit_id,
            gear_id: options[0].gear_id,
        })
        .unwrap();
        drain(&mut g);
        assert_eq!(g.unit_ref(sacrifice).unwrap().lane, None);
        assert_eq!(g.unit_ref(recipient).unwrap().temporary_buff, 0);
        assert_eq!(g.players[actor].hand, vec![71]);
        assert!(g.pending_deathgrip.is_none());
    }
}

#[test]
fn capacity_failure_restores_death_might_draw_and_suspended_choice() {
    for actor in 0..2 {
        for mode in 0..3 {
            let mut g = position(actor);
            let sacrifice = unit(&mut g, actor, Some(0));
            let recipient = unit(&mut g, actor, None);
            if mode == 0 {
                g.unit_mut(recipient).unwrap().temporary_buff = i64::MAX - 8;
            } else {
                g.trigger_state.draws[actor] = u64::MAX;
            }
            if mode == 2 {
                g.gear_play(actor, 21, None).unwrap();
                g.gear_play(actor, 21, None).unwrap();
            }
            cast(&mut g, vec![sacrifice, recipient]);
            g.apply(Action::Pass).unwrap();
            let action = if mode == 2 {
                g.apply(Action::Pass).unwrap();
                let Some(Decision::DeathReplacement { options, .. }) = &g.decision else {
                    panic!()
                };
                Action::ChooseDeathReplacement {
                    unit_id: options[0].unit_id,
                    gear_id: options[0].gear_id,
                }
            } else {
                Action::Pass
            };
            let before = format!("{g:?}");
            let error = g.apply(action).unwrap_err();
            assert!(error.starts_with("engine_capacity:"), "{error}");
            assert_eq!(format!("{g:?}"), before);
        }
    }
}

#[test]
fn current_geared_might_is_used_and_negative_might_transfers_zero() {
    for actor in 0..2 {
        for negative in [false, true] {
            let mut g = position(actor);
            let sacrifice = unit(&mut g, actor, Some(0));
            let recipient = unit(&mut g, actor, Some(1));
            let Target::Unit { unit_id, .. } = sacrifice else {
                unreachable!()
            };
            g.gear_play(actor, 3, None).unwrap(); // Recurve Bow's attached +2.
            g.players[actor].gear.last_mut().unwrap().attached_to = Some(unit_id);
            g.players[actor].gear.last_mut().unwrap().lane = Some(0);
            if negative {
                g.unit_mut(sacrifice).unwrap().temporary_buff = -20;
            }
            let amount = g.unit_might(g.unit_ref(sacrifice).unwrap()).unwrap();
            assert_eq!(amount, if negative { 0 } else { 10 });
            cast(&mut g, vec![sacrifice, recipient]);
            drain(&mut g);
            assert_eq!(g.unit_ref(recipient).unwrap().temporary_buff, amount as i64);
            assert_eq!(g.players[actor].hand, vec![71]);
            assert_eq!(g.players[actor].gear[0].attached_to, None);
            g.apply(Action::EndTurn).unwrap();
            assert_eq!(g.unit_ref(recipient).unwrap().temporary_buff, 0);
        }
    }
}

#[test]
fn repeat_uses_its_own_targets_and_draws_even_when_original_sacrifice_is_gone() {
    use crate::play::{ExecutionChoices, PlayChoices, RepeatSelection};
    for actor in 0..2 {
        for same_sacrifice in [true, false] {
            let mut g = position(actor);
            let first = unit(&mut g, actor, None);
            let second = unit(&mut g, actor, None);
            let recipient = unit(&mut g, actor, Some(0));
            let might = g.unit_might(g.unit_ref(first).unwrap()).unwrap();
            g.play_grants.push(PlayGrant {
                id: 44799,
                player: actor,
                source: ChainSource::Card { card_id: 443 },
                expires_turn: g.turn,
                kind: PlayGrantKind::RepeatNextSpell,
            });
            g.players[actor].hand.push(GRIP);
            let action = Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![first, recipient],
            };
            let option_id = g.play_choices(&action).unwrap().repeats[0].id;
            g.apply(Action::WithPlayChoices {
                action: Box::new(action),
                choices: PlayChoices {
                    repeats: vec![RepeatSelection {
                        option_id,
                        choices: ExecutionChoices {
                            targets: vec![if same_sacrifice { first } else { second }, recipient],
                            ..Default::default()
                        },
                    }],
                    ..Default::default()
                },
            })
            .unwrap();
            drain(&mut g);
            assert_eq!(g.players[actor].hand, vec![71, 71]);
            assert_eq!(
                g.unit_ref(recipient).unwrap().temporary_buff,
                (might * if same_sacrifice { 1 } else { 2 }) as i64
            );
            assert_eq!(g.unit_ref(second).is_some(), same_sacrifice);
            assert!(g.pending_deathgrip.is_none());
        }
    }
}
