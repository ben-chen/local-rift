//! Official-face scenarios: one own-trash card; live attacker/Stunned checks.
use super::*;
use test_support::{position, ready_unit, reject_unchanged, settle_unopposed_chain};

fn play(targets: Vec<Target>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets,
    }
}

pub(crate) fn combat(actor: usize) -> (Game, Target, Target) {
    let (mut g, attacker, defender) = test_support::combat_position(1 - actor, 48301);
    g.apply(Action::Pass).unwrap();
    assert_eq!(g.priority(), actor);
    g.players[actor].hand = vec![483];
    (g, attacker, defender)
}

fn repeated(target: Target, again: Target) -> Action {
    Action::PlayCardOptions {
        hand_index: 0,
        lane: None,
        targets: vec![target],
        repeat_targets: vec![again],
        repeat: true,
        accelerate: false,
        bonus: false,
    }
}

#[test]
fn morbid_return_requires_one_own_unit_and_returns_without_draw_or_play() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 48201);
        g.players[actor].hand = vec![482];
        g.players[actor].discard = vec![71, 3, 71];
        g.players[1 - actor].discard = vec![71];
        let first = g.trash_target(actor, 0).unwrap();
        let gear = g.trash_target(actor, 1).unwrap();
        let second = g.trash_target(actor, 2).unwrap();
        let enemy = g.trash_target(1 - actor, 0).unwrap();
        assert_eq!(
            g.card_target_candidates(actor, 482, None, &[]).unwrap(),
            vec![first, second]
        );
        for invalid in [vec![], vec![gear], vec![enemy], vec![first, second]] {
            reject_unchanged(&mut g, play(invalid));
        }
        let draws = g.trigger_state.draws;
        g.apply(play(vec![second])).unwrap();
        assert_eq!(g.chain.last().unwrap().targets, vec![second]);
        settle_unopposed_chain(&mut g);
        assert_eq!(g.players[actor].hand, vec![71]);
        assert_eq!(g.players[actor].discard, vec![71, 3, 482]);
        assert_eq!(g.trigger_state.draws, draws);
        assert!(g.players.iter().all(|p| p.units.is_empty()));
        assert_eq!(g.players[1 - actor].discard, vec![71]);
    }
}

#[test]
fn morbid_return_does_not_substitute_an_identical_trash_copy_after_burnout() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 48202);
        g.players[actor].hand = vec![482, 7];
        g.players[actor].discard = vec![71];
        g.players[actor].deck.clear();
        let unit_id = ready_unit(&mut g, actor, 71);
        let target = g.trash_target(actor, 0).unwrap();
        g.apply(play(vec![target])).unwrap();
        assert_eq!(g.priority(), actor, "the caster retains priority");
        g.apply(play(vec![Target::Unit {
            player: actor,
            unit_id,
        }]))
        .unwrap();
        settle_unopposed_chain(&mut g);
        assert!(g.trash_target_index(target).is_none());
        assert_eq!(
            g.players[actor].hand,
            vec![71],
            "Discipline draws the recycled card; Morbid cannot recover it again"
        );
        assert!(g.players[actor].discard.contains(&482));
    }
}

#[test]
fn dread_targets_only_current_enemy_attackers() {
    for actor in 0..2 {
        let (mut g, attacker, defender) = combat(actor);
        let base = Target::Unit {
            player: 1 - actor,
            unit_id: ready_unit(&mut g, 1 - actor, 71),
        };
        let elsewhere = Target::Unit {
            player: 1 - actor,
            unit_id: ready_unit(&mut g, 1 - actor, 71),
        };
        g.unit_mut(elsewhere).unwrap().lane = Some(1);
        assert_eq!(
            g.card_target_candidates(actor, 483, None, &[]).unwrap(),
            vec![attacker]
        );
        for target in [defender, base, elsewhere] {
            reject_unchanged(&mut g, play(vec![target]));
        }
        let mut defending_enemy = g.clone();
        defending_enemy.showdown.as_mut().unwrap().attacker = actor;
        assert!(
            defending_enemy
                .card_target_candidates(actor, 483, None, &[])
                .unwrap()
                .is_empty()
        );
        reject_unchanged(&mut defending_enemy, play(vec![attacker]));
        g.showdown = None;
        assert!(
            g.card_target_candidates(actor, 483, None, &[])
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn dread_checks_stunned_at_resolution_without_exhausting_or_killing() {
    for actor in 0..2 {
        for stunned in [false, true] {
            let (mut g, attacker, _) = combat(actor);
            g.unit_mut(attacker).unwrap().stunned = stunned;
            g.apply(play(vec![attacker])).unwrap();
            settle_unopposed_chain(&mut g);
            if stunned {
                assert!(g.unit_ref(attacker).is_none());
                assert_eq!(g.players[1 - actor].hand, vec![71]);
                assert!(!g.players[1 - actor].discard.contains(&71));
            } else {
                let unit = g.unit_ref(attacker).unwrap();
                assert!(unit.stunned && unit.ready);
                assert_eq!(unit.damage, 0);
            }
        }
    }
}

#[test]
fn dread_repeat_pays_two_energy_and_checks_each_execution_separately() {
    for actor in 0..2 {
        for same in [false, true] {
            let (mut g, first, _) = combat(actor);
            let second = Target::Unit {
                player: 1 - actor,
                unit_id: ready_unit(&mut g, 1 - actor, 71),
            };
            g.unit_mut(second).unwrap().lane = Some(0);
            let energy = g.players[actor].energy_pool;
            let power = g.players[actor].power_pool[Domain::Chaos as usize];
            g.apply(repeated(first, if same { first } else { second }))
                .unwrap();
            assert_eq!(g.players[actor].energy_pool, energy - 3);
            assert_eq!(
                g.players[actor].power_pool[Domain::Chaos as usize],
                power - 1
            );
            settle_unopposed_chain(&mut g);
            if same {
                assert!(g.unit_ref(first).is_none());
                assert!(!g.unit_ref(second).unwrap().stunned);
                assert_eq!(g.players[1 - actor].hand, vec![71]);
            } else {
                assert!(g.unit_ref(first).unwrap().stunned);
                assert!(g.unit_ref(second).unwrap().stunned);
                assert!(g.players[1 - actor].hand.is_empty());
            }
        }
    }
}

#[test]
fn dread_rechecks_attacking_after_a_flash_response() {
    for actor in 0..2 {
        let (mut g, attacker, _) = combat(actor);
        g.players[1 - actor].hand = vec![9];
        g.apply(play(vec![attacker])).unwrap();
        g.apply(Action::Pass).unwrap();
        assert_eq!(g.priority(), 1 - actor);
        g.apply(play(vec![attacker])).unwrap();
        settle_unopposed_chain(&mut g);
        let saved = g.unit_ref(attacker).unwrap();
        assert_eq!(saved.lane, None);
        assert!(!saved.stunned);
        assert!(g.players[actor].discard.contains(&483));
    }
}

#[test]
fn dread_return_preserves_owner_and_detaches_gear_and_tokens_vanish() {
    for actor in 0..2 {
        for token in [false, true] {
            let (mut g, attacker, _) = combat(actor);
            let Target::Unit { unit_id, player } = attacker else {
                unreachable!()
            };
            g.unit_mut(attacker).unwrap().stunned = true;
            g.unit_mut(attacker).unwrap().token = token;
            if !token {
                g.board_owner_overrides.insert(unit_id, actor);
            }
            g.players[player].gear.push(crate::gear::GearState {
                id: 48390,
                card_id: 3,
                attached_to: Some(unit_id),
                lane: None,
                temporary: false,
                ready: true,
                token: false,
            });
            g.apply(play(vec![attacker])).unwrap();
            settle_unopposed_chain(&mut g);
            assert!(g.unit_ref(attacker).is_none());
            assert_eq!(g.players[player].gear[0].attached_to, None);
            assert!(g.players[player].hand.is_empty());
            assert_eq!(g.players[actor].hand, if token { vec![] } else { vec![71] });
            assert!(!g.players.iter().any(|p| p.discard.contains(&71)));
        }
    }
}

#[test]
fn dread_is_action_speed_and_cannot_be_used_as_a_response_from_hand() {
    for actor in 0..2 {
        let (mut g, attacker, _) = combat(actor);
        g.players[actor].hand.insert(0, 7);
        g.apply(play(vec![attacker])).unwrap();
        assert_eq!(g.priority(), actor);
        reject_unchanged(&mut g, play(vec![attacker]));
    }
}

#[test]
fn dread_ai_can_cast_and_complete_defense_at_each_difficulty() {
    for actor in 0..2 {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let (mut g, _, _) = combat(actor);
            g.difficulty = difficulty;
            let candidates = g.legal_actions().unwrap();
            assert!(candidates.iter().any(|a| matches!(
                a,
                Action::PlayCardOptions { .. }
                    | Action::PlayCardWithTargets { .. }
                    | Action::PlayCard { .. }
            )));
            let action = g.choose_ai_action().unwrap();
            g.apply(action).unwrap();
            assert!(g.chain.iter().any(|item| item.card_id == 483));
            settle_unopposed_chain(&mut g);
        }
    }
}

#[test]
fn dread_sees_a_stun_added_by_a_hidden_response() {
    for actor in 0..2 {
        let (mut g, attacker, _) = combat(actor);
        g.turn = 3;
        g.players[actor].hidden.push(HiddenCard {
            id: 48350,
            card_id: 428,
            lane: 0,
            hidden_turn: 1,
            revealed: false,
        });
        g.apply(play(vec![attacker])).unwrap();
        assert_eq!(g.priority(), actor);
        g.apply(Action::PlayHidden {
            hidden_id: 48350,
            targets: vec![attacker],
        })
        .unwrap();
        settle_unopposed_chain(&mut g);
        assert!(g.unit_ref(attacker).is_none());
        assert_eq!(g.players[1 - actor].hand, vec![71]);
    }
}
