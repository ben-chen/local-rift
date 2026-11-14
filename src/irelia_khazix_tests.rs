//! Kha'Zix's printed condition is checked at first Attack/Defend designation.
//! Explicit midgame fixtures; movement and responses use real public actions.
use super::*;

const KHAZIX: CardId = 449;

fn position(actor: usize) -> Game {
    let mut game = Game::new_for_agents(3, 3, 449001, Difficulty::Tactical);
    game.start_match().unwrap();
    game.active = actor;
    game.priority_player = actor;
    game.battlefields = vec![3, 3];
    game.battlefield_owner = vec![Some(1 - actor), None];
    for state in &mut game.players {
        state.units.clear();
        state.gear.clear();
        state.hidden.clear();
        state.hand.clear();
        state.discard.clear();
        state.banish.clear();
        state.champion = None;
        state.deck = vec![71; 20];
        state.set_rune_setup([0; 6], [0; 6]).unwrap();
        state.energy_pool = 20;
        state.energy = 20;
        state.power_pool = [5; 6];
        state.any_power = 0;
    }
    for legend in &mut game.legend {
        legend.ready = false;
    }
    game
}

fn unit(game: &mut Game, owner: usize, card: CardId, lane: Option<usize>) -> u32 {
    let id = game.create_unit(owner, card, false, 0);
    let unit = game.players[owner].units.last_mut().unwrap();
    unit.lane = lane;
    unit.ready = true;
    id
}

fn settle_chain(game: &mut Game) {
    for _ in 0..40 {
        match game.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => game
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            None if !game.chain.is_empty() => game.apply(Action::Pass).unwrap(),
            None => return,
            other => panic!("unexpected choice {other:?}"),
        }
    }
    panic!("trigger chain did not settle")
}

fn combat(owner: usize, defending: bool, enemies: usize, company: bool) -> (Game, u32) {
    let actor = if defending { 1 - owner } else { owner };
    let mut game = position(actor);
    let source = unit(&mut game, owner, KHAZIX, defending.then_some(0));
    if company {
        unit(&mut game, owner, 59, Some(0));
    }
    let mut mover = source;
    for index in 0..enemies {
        let at_base = defending && index == 0;
        let enemy = unit(&mut game, 1 - owner, 59, (!at_base).then_some(0));
        if at_base {
            mover = enemy;
        }
    }
    game.apply(Action::Move {
        unit_id: mover,
        lane: Some(0),
    })
    .unwrap();
    (game, source)
}

fn might(game: &Game, owner: usize, id: u32) -> u64 {
    game.unit_might(
        game.players[owner]
            .units
            .iter()
            .find(|unit| unit.id == id)
            .unwrap(),
    )
    .unwrap()
}

fn cast(game: &mut Game, owner: usize, card: CardId, lane: Option<usize>, targets: Vec<Target>) {
    if game.priority() != owner {
        game.apply(Action::Pass).unwrap();
    }
    assert_eq!(game.priority(), owner);
    let hand_index = game.players[owner].hand.len();
    game.players[owner].hand.push(card);
    game.apply(Action::PlayCardWithTargets {
        hand_index,
        lane,
        targets,
    })
    .unwrap();
}

fn finish_windows(game: &mut Game) {
    for _ in 0..60 {
        if game.decision.is_none() && !game.in_window() {
            return;
        }
        match game.decision.clone() {
            Some(Decision::CombatDamage {
                player,
                lane,
                total,
            }) => game
                .apply(Action::AssignDamage {
                    assignments: game
                        .automatic_damage_assignment(player, lane, total)
                        .unwrap(),
                })
                .unwrap(),
            Some(Decision::OrderTriggers { options, .. }) => game
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            None => game.apply(Action::Pass).unwrap(),
            other => panic!("unexpected end-of-combat choice {other:?}"),
        }
    }
    panic!("combat did not finish")
}

#[test]
fn irelia_khazix_attack_and_defend_reward_one_enemy_even_with_friendly_company() {
    for owner in 0..2 {
        for defending in [false, true] {
            for company in [false, true] {
                let (mut game, id) = combat(owner, defending, 1, company);
                let xp = game.players[owner].xp;
                settle_chain(&mut game);
                assert_eq!(
                    might(&game, owner, id),
                    6,
                    "owner={owner} defending={defending} company={company}"
                );
                assert_eq!(game.players[owner].xp, xp + 2);
                assert_eq!(
                    game.players[owner]
                        .units
                        .iter()
                        .find(|unit| unit.id == id)
                        .unwrap()
                        .temporary_buff,
                    2
                );
            }
        }
    }
}

#[test]
fn irelia_khazix_does_not_reward_a_pair_of_enemies_or_an_empty_field() {
    for owner in 0..2 {
        for defending in [false, true] {
            let (mut game, id) = combat(owner, defending, 2, true);
            let xp = game.players[owner].xp;
            settle_chain(&mut game);
            assert_eq!(might(&game, owner, id), 4);
            assert_eq!(game.players[owner].xp, xp);
        }
        let (mut game, id) = combat(owner, false, 0, false);
        let xp = game.players[owner].xp;
        settle_chain(&mut game);
        assert_eq!(might(&game, owner, id), 4);
        assert_eq!(game.players[owner].xp, xp);
    }
}

#[test]
fn irelia_khazix_does_not_recheck_a_trigger_after_an_enemy_ambushes() {
    for owner in 0..2 {
        for defending in [false, true] {
            let (mut game, id) = combat(owner, defending, 1, true);
            let xp = game.players[owner].xp;
            assert!(!game.chain.is_empty());
            cast(&mut game, 1 - owner, 94, Some(0), vec![]); // Soulspinner's actual Ambush.
            assert_eq!(game.lane_count(1 - owner, 0), 2);
            settle_chain(&mut game);
            assert_eq!(might(&game, owner, id), 6);
            assert_eq!(game.players[owner].xp, xp + 2);
        }
    }
}

#[test]
fn irelia_khazix_source_departure_does_not_cancel_the_independent_xp() {
    for owner in 0..2 {
        for defending in [false, true] {
            let (mut game, id) = combat(owner, defending, 1, true);
            let xp = game.players[owner].xp;
            cast(
                &mut game,
                owner,
                377,
                None,
                vec![Target::Unit {
                    player: owner,
                    unit_id: id,
                }],
            );
            settle_chain(&mut game);
            assert!(!game.players[owner].units.iter().any(|unit| unit.id == id));
            assert!(game.players[owner].hand.contains(&KHAZIX));
            assert_eq!(game.players[owner].xp, xp + 2);
        }
    }
}

#[test]
fn irelia_khazix_failed_first_condition_cannot_trigger_later_in_the_same_combat() {
    for owner in 0..2 {
        for defending in [false, true] {
            let (mut game, id) = combat(owner, defending, 2, true);
            let xp = game.players[owner].xp;
            let enemy = game.players[1 - owner].units[0].id;
            cast(
                &mut game,
                1 - owner,
                377,
                None,
                vec![Target::Unit {
                    player: 1 - owner,
                    unit_id: enemy,
                }],
            );
            settle_chain(&mut game);
            assert_eq!(game.lane_count(1 - owner, 0), 1);
            assert_eq!(might(&game, owner, id), 4);
            assert_eq!(game.players[owner].xp, xp);
            assert!(game.trigger_state.combat_seen.contains(&id));
        }
    }
}

#[test]
fn irelia_khazix_separate_combats_stack_might_until_turn_end_and_keep_xp() {
    for owner in 0..2 {
        let (mut game, id) = combat(owner, false, 1, false);
        let xp = game.players[owner].xp;
        settle_chain(&mut game);
        finish_windows(&mut game);
        unit(&mut game, 1 - owner, 59, Some(1));
        cast(
            &mut game,
            owner,
            14,
            Some(1),
            vec![Target::Unit {
                player: owner,
                unit_id: id,
            }],
        );
        settle_chain(&mut game);
        assert_eq!(might(&game, owner, id), 8);
        assert_eq!(game.players[owner].xp, xp + 4);
        finish_windows(&mut game);
        game.apply(Action::EndTurn).unwrap();
        assert_eq!(might(&game, owner, id), 4);
        assert_eq!(game.players[owner].xp, xp + 4);
    }
}

#[test]
fn irelia_khazix_ambush_is_a_paid_reaction_only_at_a_friendly_occupied_field() {
    for owner in 0..2 {
        for actor in 0..2 {
            let mut game = position(actor);
            unit(&mut game, owner, 59, Some(0));
            unit(&mut game, 1 - owner, 59, Some(1));
            let chosen = unit(&mut game, actor, 59, None);
            cast(
                &mut game,
                actor,
                7,
                None,
                vec![Target::Unit {
                    player: actor,
                    unit_id: chosen,
                }],
            );
            if game.priority() != owner {
                game.apply(Action::Pass).unwrap();
            }
            game.players[owner].hand = vec![KHAZIX];
            for lane in [None, Some(1)] {
                let before = format!("{game:?}");
                assert!(
                    game.apply(Action::PlayCardWithTargets {
                        hand_index: 0,
                        lane,
                        targets: vec![]
                    })
                    .is_err()
                );
                assert_eq!(format!("{game:?}"), before);
            }
            let energy = game.players[owner].energy;
            let power = game.players[owner].power_pool[domain_index(Domain::Chaos)];
            game.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: Some(0),
                targets: vec![],
            })
            .unwrap();
            assert!(
                game.players[owner]
                    .units
                    .iter()
                    .any(|unit| unit.card_id == KHAZIX && unit.lane == Some(0) && !unit.ready)
            );
            assert_eq!(game.players[owner].energy, energy - 4);
            assert_eq!(
                game.players[owner].power_pool[domain_index(Domain::Chaos)],
                power - 1
            );
            assert!(
                !game.chain.is_empty(),
                "the original spell still awaits resolution"
            );
        }
    }
}
