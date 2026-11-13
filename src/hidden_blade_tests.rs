//! Controlled board/resource fixtures followed by real card and choice actions.
//! The fixtures are not claimed to be complete registered tournament decks.
use super::*;
const BLADE: CardId = 446;

fn position(actor: usize) -> Game {
    let mut game = Game::new_for_agents(3, 3, 446213, Difficulty::Tactical);
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

fn cast(game: &mut Game, card: CardId, target: Target) {
    let actor = game.priority();
    let hand_index = game.players[actor].hand.len();
    game.players[actor].hand.push(card);
    game.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets: vec![target],
    })
    .unwrap();
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

#[test]
fn valid_friendly_or_enemy_target_dies_and_its_controller_draws_two() {
    for actor in 0..2 {
        for controller in 0..2 {
            let mut game = position(actor);
            let target = unit(&mut game, controller, Some(0));
            cast(&mut game, BLADE, target);
            assert!(game.unit_ref(target).is_some());
            drain(&mut game);
            assert!(game.unit_ref(target).is_none());
            assert_eq!(game.players[controller].hand, vec![71, 71]);
            assert!(game.players[1 - controller].hand.is_empty());
            let ledger = game.last_damage_ledger().unwrap();
            assert!(ledger.damage.is_empty());
            assert_eq!(ledger.kills.len(), 1);
            assert_eq!(ledger.kills[0].target, target);
            assert_eq!(ledger.kills[0].credits[0].player, actor);
        }
    }
}

#[test]
fn base_target_rejects_and_flash_or_retreat_response_stops_both_instructions() {
    for actor in 0..2 {
        let enemy = 1 - actor;
        let mut game = position(actor);
        let base = unit(&mut game, enemy, None);
        game.players[actor].hand.push(BLADE);
        let before = format!("{game:?}");
        assert!(
            game.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![base],
            })
            .is_err()
        );
        assert_eq!(format!("{game:?}"), before);
        for response in [9, 377] {
            let mut game = position(actor);
            let target = unit(&mut game, enemy, Some(0));
            cast(&mut game, BLADE, target);
            game.apply(Action::Pass).unwrap();
            assert_eq!(game.priority(), enemy);
            cast(&mut game, response, target);
            drain(&mut game);
            assert_eq!(game.players[enemy].hand.len(), usize::from(response == 377));
            assert_eq!(game.players[enemy].deck.len(), 20);
            if response == 9 {
                assert_eq!(game.unit_ref(target).unwrap().lane, None);
            } else {
                assert_eq!(game.players[enemy].hand, vec![69]);
            }
        }
    }
}

#[test]
fn death_replacement_choice_precedes_draw_and_saved_controller_still_draws() {
    for actor in 0..2 {
        let enemy = 1 - actor;
        let mut game = position(actor);
        let target = unit(&mut game, enemy, Some(0));
        game.gear_play(enemy, 21, None).unwrap();
        game.gear_play(enemy, 21, None).unwrap();
        cast(&mut game, BLADE, target);
        pair(&mut game);
        let Some(Decision::DeathReplacement { player, options }) = game.decision.clone() else {
            panic!("Expected replacement ordering: {:?}", game.decision);
        };
        assert_eq!(player, enemy);
        assert_eq!(options.len(), 2);
        assert!(game.players[enemy].hand.is_empty());
        assert_eq!(game.pending_kill_controller_draw, Some((enemy, 2)));
        let before = format!("{game:?}");
        assert!(
            game.apply(Action::ChooseDeathReplacement {
                unit_id: u32::MAX,
                gear_id: options[0].gear_id
            })
            .is_err()
        );
        assert_eq!(format!("{game:?}"), before);
        game.apply(Action::ChooseDeathReplacement {
            unit_id: options[0].unit_id,
            gear_id: options[0].gear_id,
        })
        .unwrap();
        drain(&mut game);
        assert_eq!(game.unit_ref(target).unwrap().lane, None);
        assert_eq!(game.players[enemy].hand, vec![71, 71]);
        assert_eq!(game.players[enemy].gear.len(), 1);
        assert!(game.pending_kill_controller_draw.is_none());
        assert!(
            !game
                .last_damage_ledger()
                .unwrap()
                .kills
                .iter()
                .any(|k| k.target == target)
        );
    }
}

#[test]
fn repeated_execution_does_not_draw_again_for_a_departed_target() {
    use crate::play::{ExecutionChoices, PlayChoices, RepeatSelection};
    for actor in 0..2 {
        let mut game = position(actor);
        let enemy = 1 - actor;
        let target = unit(&mut game, enemy, Some(0));
        game.players[actor].hand.push(BLADE);
        game.play_grants.push(PlayGrant {
            id: 44699,
            player: actor,
            source: ChainSource::Card { card_id: 443 },
            expires_turn: game.turn,
            kind: PlayGrantKind::RepeatNextSpell,
        });
        let action = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        };
        let repeat = game.play_choices(&action).unwrap().repeats[0].id;
        game.apply(Action::WithPlayChoices {
            action: Box::new(action),
            choices: PlayChoices {
                repeats: vec![RepeatSelection {
                    option_id: repeat,
                    choices: ExecutionChoices {
                        targets: vec![target],
                        ..Default::default()
                    },
                }],
                ..Default::default()
            },
        })
        .unwrap();
        drain(&mut game);
        assert_eq!(game.players[enemy].hand, vec![71, 71]);
        assert_eq!(game.last_damage_ledger().unwrap().kills.len(), 1);
    }
}

#[test]
fn hidden_play_waives_base_cost_and_restricts_targets_to_its_own_field() {
    for actor in 0..2 {
        let mut game = position(actor);
        let enemy = 1 - actor;
        let here = unit(&mut game, enemy, Some(0));
        let elsewhere = unit(&mut game, enemy, Some(1));
        unit(&mut game, actor, Some(0));
        game.battlefield_owner[0] = Some(actor);
        // Controlled earlier-turn facedown card. Actual Hidden finalization follows.
        game.players[actor].hidden.push(HiddenCard {
            id: 44600,
            card_id: BLADE,
            lane: 0,
            hidden_turn: 0,
            revealed: false,
        });
        game.players[actor].energy = 0;
        game.players[actor].energy_pool = 0;
        game.players[actor].power_pool = [0; 6];
        let before = format!("{game:?}");
        assert!(
            game.apply(Action::PlayHidden {
                hidden_id: 44600,
                targets: vec![elsewhere]
            })
            .is_err()
        );
        assert_eq!(format!("{game:?}"), before);
        game.apply(Action::PlayHidden {
            hidden_id: 44600,
            targets: vec![here],
        })
        .unwrap();
        drain(&mut game);
        assert!(game.unit_ref(here).is_none());
        assert!(game.unit_ref(elsewhere).is_some());
        assert_eq!(game.players[enemy].hand, vec![71, 71]);
        assert_eq!(game.players[actor].energy_pool, 0);
    }
}

#[test]
fn draw_capacity_failure_restores_kill_and_suspended_replacement_action() {
    for actor in 0..2 {
        for replacement in [false, true] {
            let enemy = 1 - actor;
            let mut game = position(actor);
            let target = unit(&mut game, enemy, Some(0));
            game.trigger_state.draws[enemy] = u64::MAX - 1;
            if replacement {
                game.gear_play(enemy, 21, None).unwrap();
                game.gear_play(enemy, 21, None).unwrap();
            }
            cast(&mut game, BLADE, target);
            game.apply(Action::Pass).unwrap();
            let action = if replacement {
                game.apply(Action::Pass).unwrap();
                let Some(Decision::DeathReplacement { options, .. }) = &game.decision else {
                    panic!()
                };
                Action::ChooseDeathReplacement {
                    unit_id: options[0].unit_id,
                    gear_id: options[0].gear_id,
                }
            } else {
                Action::Pass
            };
            let before = format!("{game:?}");
            let error = game.apply(action).unwrap_err();
            assert!(error.starts_with("engine_capacity:"), "{error}");
            assert_eq!(format!("{game:?}"), before);
        }
    }
}
