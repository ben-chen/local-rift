//! Public-action regressions for the seven reviewed battlefield additions.
use local_rift::activated::AbilitySource;
use local_rift::battlefields::BattlefieldAbility;
use local_rift::game::play::{CostSelection, PlayChoices, PlayGrantKind};
use local_rift::game::{
    Action, ChainAbility, ChainSource, Decision, Difficulty, Game, Target, Unit,
};
use local_rift::gear::GearState;

fn position(fields: [usize; 2]) -> Game {
    let mut game = Game::new_for_agents(3, 3, 44071, Difficulty::Tactical);
    game.choose_battlefield(0, fields[0]).unwrap();
    game.choose_battlefield(1, fields[1]).unwrap();
    game.start_match().unwrap();
    game.battlefield_owner = vec![None; 2];
    for p in 0..2 {
        game.legend[p].ready = false;
        let player = &mut game.players[p];
        player.hand.clear();
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.discard.clear();
        player.champion = None;
        player.set_rune_setup([0; 6], [0; 6]).unwrap();
        player.energy_pool = 30;
        player.energy = 30;
        player.power_pool = [10; 6];
        player.any_power = 0;
    }
    game
}
fn unit(g: &mut Game, player: usize, id: u32, card: usize, lane: Option<usize>) -> Target {
    g.players[player].units.push(Unit {
        id,
        card_id: card,
        lane,
        ready: true,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    });
    if let Some(lane) = lane {
        g.battlefield_owner[lane] = Some(player);
    }
    Target::Unit {
        player,
        unit_id: id,
    }
}
fn gear(g: &mut Game, id: u32, card: usize) -> AbilitySource {
    g.players[0].gear.push(GearState {
        id,
        card_id: card,
        lane: None,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    AbilitySource::Gear {
        player: 0,
        gear_id: id,
    }
}
fn hand(g: &mut Game, card: usize, lane: Option<usize>) -> Action {
    let player = g.priority();
    let index = g.players[player].hand.len();
    g.players[player].hand.push(card);
    Action::PlayCardWithTargets {
        hand_index: index,
        lane,
        targets: vec![],
    }
}
fn settle(g: &mut Game) {
    for _ in 0..100 {
        let action = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Some(Action::OrderTrigger {
                trigger_id: options[0].id,
            }),
            Some(Decision::TokenPlacement { .. }) => Some(Action::PlaceToken { lane: None }),
            Some(Decision::TokenPlay { .. }) => Some(Action::FinalizeTokenPlay {
                lane: None,
                accelerate: false,
            }),
            Some(_) => Some(Action::Choose { accept: false }),
            None if g.in_window() => Some(Action::Pass),
            None => None,
        };
        let Some(action) = action else {
            return;
        };
        g.apply(action).unwrap();
    }
    panic!("field response window did not settle: {:?}", g.decision);
}
fn field_grant_on_chain(g: &Game, expected: PlayGrantKind) -> bool {
    g.chain.iter().any(|item| {
        matches!(item.ability,
        Some(ChainAbility::Battlefield(BattlefieldAbility::Grant { kind, .. })) if kind == expected)
    })
}

#[test]
fn academy_hold_queues_respondable_repeat_before_channel_and_draw() {
    let mut g = position([4, 3]);
    unit(&mut g, 0, 9901, 69, Some(0));
    g.apply(Action::EndTurn).unwrap();
    let hand_count = g.players[0].hand.len();
    let rune_count = g.players[0].total_runes();
    g.apply(Action::EndTurn).unwrap();
    assert!(g.play_grants.is_empty());
    assert!(field_grant_on_chain(&g, PlayGrantKind::RepeatNextSpell));
    assert_eq!(g.players[0].hand.len(), hand_count);
    assert_eq!(g.players[0].total_runes(), rune_count);
    settle(&mut g);
    assert_eq!(g.players[0].hand.len(), hand_count + 1);
    let grant = g
        .play_grants
        .iter()
        .find(|v| v.kind == PlayGrantKind::RepeatNextSpell)
        .unwrap();
    assert_eq!(
        grant.source,
        ChainSource::Battlefield {
            lane: 0,
            battlefield_id: 4
        }
    );
    let action = hand(&mut g, 263, None);
    let choices = g.play_choices(&action).unwrap();
    assert_eq!(choices.repeats.len(), 1);
    assert_eq!(choices.repeats[0].cost.energy, 2);
    assert_eq!(choices.repeats[0].cost.power, 0);
}

#[test]
fn hideout_triggers_on_conquest_but_never_on_unscored_reconquest() {
    let mut g = position([5, 3]);
    unit(&mut g, 0, 9902, 69, None);
    g.apply(Action::Move {
        unit_id: 9902,
        lane: Some(0),
    })
    .unwrap();
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
    assert!(field_grant_on_chain(&g, PlayGrantKind::AccelerateNextUnit));
    assert!(g.play_grants.is_empty());
    settle(&mut g);
    assert_eq!(g.play_grants.len(), 1);
    g.players[0].units[0].ready = true;
    g.apply(Action::Move {
        unit_id: 9902,
        lane: None,
    })
    .unwrap();
    g.players[0].units[0].ready = true;
    g.apply(Action::Move {
        unit_id: 9902,
        lane: Some(0),
    })
    .unwrap();
    settle(&mut g);
    assert_eq!(g.play_grants.len(), 1);
    let action = hand(&mut g, 69, None);
    let choices = g.play_choices(&action).unwrap();
    assert_eq!(choices.accelerate.unwrap().cost.energy, 1);
}

#[test]
fn hideout_final_point_draw_still_triggers_and_grant_expires_at_turn_end() {
    let mut g = position([5, 3]);
    g.players[0].score = 7;
    unit(&mut g, 0, 9903, 69, None);
    let hand_count = g.players[0].hand.len();
    g.apply(Action::Move {
        unit_id: 9903,
        lane: Some(0),
    })
    .unwrap();
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
    assert_eq!(g.players[0].score, 7);
    assert_eq!(g.players[0].hand.len(), hand_count + 1);
    settle(&mut g);
    assert_eq!(g.play_grants.len(), 1);
    g.apply(Action::EndTurn).unwrap();
    assert!(g.play_grants.is_empty());
}

#[test]
fn ornn_forge_discounts_first_nontoken_gear_even_if_control_was_gained_later() {
    let mut g = position([6, 6]);
    unit(&mut g, 0, 9910, 69, Some(0));
    unit(&mut g, 0, 9911, 69, Some(1));
    let first = hand(&mut g, 117, None);
    assert_eq!(g.cost_plan(&first).unwrap().energy, 2); // both fields apply to the same first gear
    g.apply(first).unwrap();
    settle(&mut g);
    assert_eq!(g.play_history.non_token_gears[0], 1);
    let second = hand(&mut g, 117, None);
    assert_eq!(g.cost_plan(&second).unwrap().energy, 4);
    g.battlefield_owner = vec![None, None];
    assert_eq!(g.cost_plan(&second).unwrap().energy, 4);
    g.battlefield_owner = vec![Some(0), Some(0)];
    assert_eq!(g.cost_plan(&second).unwrap().energy, 4); // reacquiring control never resets history
}

#[test]
fn zero_energy_first_gear_uses_forge_allowance_but_a_gold_token_does_not() {
    let mut g = position([6, 3]);
    unit(&mut g, 0, 9912, 69, Some(0));
    let gold_spell = hand(&mut g, 262, None);
    g.apply(gold_spell).unwrap();
    settle(&mut g);
    assert!(g.players[0].gear.iter().any(|v| v.token));
    assert_eq!(g.play_history.non_token_gears[0], 0);
    let zero = hand(&mut g, 127, None);
    assert_eq!(g.cost_plan(&zero).unwrap().energy, 0);
    g.apply(zero).unwrap();
    settle(&mut g);
    let next = hand(&mut g, 117, None);
    assert_eq!(g.cost_plan(&next).unwrap().energy, 4);
}

#[test]
fn hunters_circle_tracks_each_player_lane_and_turn_without_granting_deploy() {
    let mut g = position([7, 7]);
    unit(&mut g, 0, 9913, 69, Some(0));
    unit(&mut g, 0, 9914, 69, Some(1));
    let first = hand(&mut g, 69, Some(0));
    assert_eq!(g.cost_plan(&first).unwrap().energy, 6);
    g.apply(first).unwrap();
    settle(&mut g);
    let second = hand(&mut g, 69, Some(0));
    assert_eq!(g.cost_plan(&second).unwrap().energy, 7);
    let other = hand(&mut g, 69, Some(1));
    assert_eq!(g.cost_plan(&other).unwrap().energy, 6);
    let illegal = hand(&mut g, 117, Some(0));
    assert!(
        g.apply(illegal).is_err(),
        "Circle does not let ordinary gear deploy to fields"
    );
}

#[test]
fn piltovan_first_ability_discount_uses_global_history_including_zero_cost_add() {
    let mut g = position([8, 8]);
    unit(&mut g, 0, 9915, 69, Some(0));
    unit(&mut g, 0, 9916, 69, Some(1));
    let tools = gear(&mut g, 9917, 253);
    let empower = Action::ActivateAbility {
        source: tools,
        ability_index: 0,
        targets: vec![],
    };
    assert_eq!(g.cost_plan(&empower).unwrap().energy, 0);
    let egg = gear(&mut g, 9918, 252);
    g.apply(Action::ActivateAbility {
        source: egg,
        ability_index: 1,
        targets: vec![],
    })
    .unwrap();
    assert_eq!(g.play_history.gear_activations[0], 1);
    assert_eq!(g.cost_plan(&empower).unwrap().energy, 2);
    g.apply(empower).unwrap();
    assert_eq!(g.play_history.gear_activations[0], 2);
}

#[test]
fn altar_discount_requires_controller_unit_here_and_explicit_resource_choice() {
    let mut g = position([9, 3]);
    unit(&mut g, 0, 9920, 45, Some(0));
    let action = Action::EmpowerUnit {
        unit_id: 9920,
        pay_power: false,
    };
    let plan = g.cost_plan(&action).unwrap();
    let altar = plan
        .modifiers
        .iter()
        .find(|v| v.label == "Risen Altar")
        .unwrap();
    assert!(altar.choose_energy_or_power);
    let id = altar.id;
    let chosen = Action::WithPlayChoices {
        action: Box::new(action.clone()),
        choices: PlayChoices {
            cost_order: vec![CostSelection {
                modifier_id: id,
                use_power: true,
                power_allocations: vec![],
            }],
            ..PlayChoices::default()
        },
    };
    let power_plan = g.cost_plan(&chosen).unwrap();
    assert_eq!(power_plan.energy, 2);
    assert_eq!(power_plan.power.iter().map(|p| p.amount).sum::<u16>(), 0);
    g.battlefield_owner[0] = None;
    assert!(
        !g.cost_plan(&action)
            .unwrap()
            .modifiers
            .iter()
            .any(|v| v.label == "Risen Altar")
    );
    g.battlefield_owner[0] = Some(1);
    assert!(
        !g.cost_plan(&action)
            .unwrap()
            .modifiers
            .iter()
            .any(|v| v.label == "Risen Altar")
    );
    g.battlefield_owner[0] = Some(0);
    g.players[0].units[0].lane = None;
    assert!(
        !g.cost_plan(&action)
            .unwrap()
            .modifiers
            .iter()
            .any(|v| v.label == "Risen Altar")
    );
}

#[test]
fn vault_hold_surcharge_is_respondable_persists_without_control_and_expires() {
    let mut g = position([10, 3]);
    unit(&mut g, 0, 9921, 69, Some(0));
    g.apply(Action::EndTurn).unwrap();
    g.apply(Action::EndTurn).unwrap();
    assert!(field_grant_on_chain(
        &g,
        PlayGrantKind::UnitSurchargeThisTurn(1)
    ));
    assert!(g.play_grants.is_empty());
    settle(&mut g);
    let action = hand(&mut g, 69, None);
    assert_eq!(g.cost_plan(&action).unwrap().energy, 8);
    g.players[0].units[0].lane = None;
    g.battlefield_owner[0] = None;
    assert_eq!(g.cost_plan(&action).unwrap().energy, 8);
    g.apply(Action::EndTurn).unwrap();
    assert!(g.play_grants.is_empty());
}

#[test]
fn hunters_circle_applies_without_control_but_not_on_opponents_turn() {
    let mut g = position([7, 3]);
    unit(&mut g, 1, 9922, 69, Some(0));
    let ambush = hand(&mut g, 30, Some(0));
    assert_eq!(g.cost_plan(&ambush).unwrap().energy, 4);
    g.apply(Action::EndTurn).unwrap();
    let spell = hand(&mut g, 263, None);
    g.apply(spell).unwrap();
    g.apply(Action::Pass).unwrap();
    assert_eq!(g.priority(), 0);
    assert_eq!(g.active, 1);
    assert_eq!(g.cost_plan(&ambush).unwrap().energy, 5);
}

#[test]
fn token_play_consumes_circle_first_unit_or_gear_history() {
    let mut g = position([7, 3]);
    unit(&mut g, 0, 9923, 69, Some(0));
    let recruit = hand(&mut g, 87, None);
    g.apply(recruit).unwrap();
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
    assert!(matches!(g.decision, Some(Decision::TokenPlacement { .. })));
    g.apply(Action::PlaceToken { lane: Some(0) }).unwrap();
    settle(&mut g);
    assert_eq!(g.play_history.units_or_gears_here[0][0], 1);
    let next = hand(&mut g, 69, Some(0));
    assert_eq!(g.cost_plan(&next).unwrap().energy, 7);
}

fn circle_position(player: usize) -> Game {
    let mut g = position([7, 7]);
    if player == 1 {
        g.apply(Action::EndTurn).unwrap();
        settle(&mut g);
    }
    g.players[player].set_rune_setup([0; 6], [0; 6]).unwrap();
    g.players[player].energy_pool = 30;
    g.players[player].energy = 30;
    g.players[player].power_pool = [10; 6];
    unit(&mut g, player, 9950, 69, Some(0));
    unit(&mut g, player, 9951, 69, Some(1));
    g
}

#[test]
fn circle_unit_and_deploy_gear_share_one_discount_in_either_order_and_seat() {
    for player in 0..2 {
        for gear_first in [false, true] {
            let mut g = circle_position(player);
            let (first, first_cost, next, next_cost) = if gear_first {
                (401, 0, 69, 7)
            } else {
                (69, 6, 401, 1)
            };
            let play = hand(&mut g, first, Some(0));
            assert_eq!(g.cost_plan(&play).unwrap().energy, first_cost);
            let energy = g.players[player].energy;
            g.apply(play).unwrap();
            assert_eq!(
                u32::from(g.players[player].energy),
                u32::from(energy) - first_cost
            );
            settle(&mut g);
            assert_eq!(g.play_history.units_or_gears_here[player], [1, 0]);
            let play = hand(&mut g, next, Some(0));
            assert_eq!(g.cost_plan(&play).unwrap().energy, next_cost);
            g.apply(play).unwrap();
            settle(&mut g);
            assert_eq!(g.play_history.units_or_gears_here[player], [2, 0]);
            let other_field = hand(&mut g, 69, Some(1));
            assert_eq!(g.cost_plan(&other_field).unwrap().energy, 6);
        }
    }
}

#[test]
fn circle_hiding_is_not_playing_but_a_free_hidden_unit_uses_the_discount() {
    for player in 0..2 {
        let mut g = circle_position(player);
        let hand_index = g.players[player].hand.len();
        g.players[player].hand.push(108); // Pakaa Cub: Hidden, no play trigger.
        g.apply(Action::Hide {
            hand_index,
            lane: 0,
        })
        .unwrap();
        let hidden_id = g.players[player].hidden[0].id;
        assert_eq!(g.play_history.units_or_gears_here[player], [0, 0]);
        let probe = hand(&mut g, 69, Some(0));
        assert_eq!(g.cost_plan(&probe).unwrap().energy, 6);
        for _ in 0..2 {
            g.apply(Action::EndTurn).unwrap();
            settle(&mut g);
        }
        assert_eq!(g.active, player);
        let reveal = Action::PlayHidden {
            hidden_id,
            targets: vec![],
        };
        assert_eq!(g.cost_plan(&reveal).unwrap().energy, 0);
        let energy = g.players[player].energy;
        g.apply(reveal).unwrap();
        settle(&mut g);
        assert_eq!(g.players[player].energy, energy);
        assert_eq!(g.play_history.units_or_gears_here[player], [1, 0]);
        let gear = hand(&mut g, 401, Some(0));
        assert_eq!(g.cost_plan(&gear).unwrap().energy, 1);
    }
}

#[test]
fn circle_each_players_allowance_returns_on_their_next_turn() {
    let mut g = position([7, 7]);
    unit(&mut g, 0, 9960, 69, Some(0));
    unit(&mut g, 1, 9961, 69, Some(1));
    for player in [0, 1, 0] {
        assert_eq!(g.active, player);
        assert_eq!(g.play_history.units_or_gears_here, [[0; 2]; 2]);
        let first = hand(&mut g, 401, Some(player));
        assert_eq!(g.cost_plan(&first).unwrap().energy, 0);
        g.apply(first).unwrap();
        settle(&mut g);
        assert_eq!(g.play_history.units_or_gears_here[player][player], 1);
        g.apply(Action::EndTurn).unwrap();
        settle(&mut g);
    }
}

#[test]
fn circle_unit_and_gear_moves_do_not_use_a_play_discount() {
    let mut g = circle_position(0);
    unit(&mut g, 0, 9962, 69, None);
    g.apply(Action::Move {
        unit_id: 9962,
        lane: Some(0),
    })
    .unwrap();
    settle(&mut g);
    gear(&mut g, 9963, 401);
    g.players[0].gear[0].lane = Some(1);
    let action = hand(&mut g, 410, Some(0));
    let Action::PlayCardWithTargets { hand_index, .. } = action else {
        unreachable!()
    };
    g.apply(Action::WithPlayChoices {
        action: Box::new(Action::PlayCardWithTargets {
            hand_index,
            lane: None,
            targets: vec![
                Target::Gear {
                    player: 0,
                    gear_id: 9963,
                },
                Target::Battlefield { lane: 0 },
            ],
        }),
        choices: PlayChoices {
            mode: Some(0),
            ..Default::default()
        },
    })
    .unwrap();
    settle(&mut g);
    assert_eq!(g.players[0].gear[0].lane, Some(0));
    assert_eq!(g.play_history.units_or_gears_here[0], [0, 0]);
    let first = hand(&mut g, 69, Some(0));
    assert_eq!(g.cost_plan(&first).unwrap().energy, 6);
}

#[test]
fn circle_quotes_and_cancelled_or_failed_payments_do_not_spend_the_allowance() {
    let mut g = circle_position(0);
    let first = hand(&mut g, 69, Some(0));
    let before = format!("{g:?}");
    for _ in 0..3 {
        assert_eq!(g.cost_plan(&first).unwrap().energy, 6);
        assert_eq!(format!("{g:?}"), before);
    }
    g.apply(Action::BeginPayment {
        action: Box::new(first.clone()),
    })
    .unwrap();
    assert_eq!(g.play_history.units_or_gears_here[0], [0, 0]);
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(format!("{g:?}"), before);
    g.players[0].energy_pool = 0;
    g.players[0].energy = 0;
    let unfunded = format!("{g:?}");
    assert!(g.apply(first.clone()).is_err());
    assert_eq!(format!("{g:?}"), unfunded);
    g.players[0].energy_pool = 6;
    g.players[0].energy = 6;
    g.apply(Action::BeginPayment {
        action: Box::new(first),
    })
    .unwrap();
    g.apply(Action::CompletePayment).unwrap();
    settle(&mut g);
    assert_eq!(g.players[0].energy, 0);
    assert_eq!(g.play_history.units_or_gears_here[0], [1, 0]);
}

#[test]
fn circle_instructed_free_replay_uses_the_allowance_at_the_actual_destination() {
    for player in 0..2 {
        let mut g = circle_position(player);
        let hand_index = g.players[player].hand.len();
        g.players[player].hand.push(381); // Temporal Breach replays to the same location.
        g.apply(Action::PlayCardWithTargets {
            hand_index,
            lane: None,
            targets: vec![Target::Unit {
                player,
                unit_id: 9950,
            }],
        })
        .unwrap();
        g.apply(Action::Pass).unwrap();
        g.apply(Action::Pass).unwrap();
        assert!(matches!(
            g.decision,
            Some(Decision::PendingPlay { card_id: 69, .. })
        ));
        assert_eq!(g.play_history.units_or_gears_here[player], [0, 0]);
        let replay = Action::FinalizePendingPlay {
            lane: Some(0),
            targets: vec![],
            repeat_targets: vec![],
            repeat: false,
            accelerate: false,
            bonus: false,
        };
        assert_eq!(g.cost_plan(&replay).unwrap().energy, 0);
        g.apply(replay).unwrap();
        settle(&mut g);
        assert_eq!(g.play_history.units_or_gears_here[player], [1, 0]);
        assert!(!g.players[player].units.iter().any(|unit| unit.id == 9950));
        let next = hand(&mut g, 401, Some(0));
        assert_eq!(g.cost_plan(&next).unwrap().energy, 1);
        let elsewhere = hand(&mut g, 401, Some(1));
        assert_eq!(g.cost_plan(&elsewhere).unwrap().energy, 0);
    }
}

#[test]
fn circle_reduces_hidden_accelerate_energy_after_the_base_cost_is_ignored() {
    let mut g = circle_position(0);
    g.battlefields[1] = 5; // Firelights' Hideout grants Accelerate on conquest.
    g.battlefield_owner[1] = None;
    g.players[0].units.retain(|unit| unit.id != 9951);
    let hand_index = g.players[0].hand.len();
    g.players[0].hand.push(108);
    g.apply(Action::Hide {
        hand_index,
        lane: 0,
    })
    .unwrap();
    let hidden_id = g.players[0].hidden[0].id;
    for _ in 0..2 {
        g.apply(Action::EndTurn).unwrap();
        settle(&mut g);
    }
    unit(&mut g, 0, 9970, 69, None);
    g.apply(Action::Move {
        unit_id: 9970,
        lane: Some(1),
    })
    .unwrap();
    settle(&mut g);
    assert!(
        g.play_grants
            .iter()
            .any(|grant| grant.kind == PlayGrantKind::AccelerateNextUnit)
    );
    // The fixture legend has different rune domains; fund the Cub's printed domain.
    g.players[0].power_pool = [1; 6];
    let action = Action::WithPlayChoices {
        action: Box::new(Action::PlayHidden {
            hidden_id,
            targets: vec![],
        }),
        choices: PlayChoices {
            accelerate: true,
            ..Default::default()
        },
    };
    // CR356: ignore base cost, add Accelerate, then apply Circle's total discount.
    let plan = g.cost_plan(&action).unwrap();
    assert_eq!(plan.energy, 0);
    assert_eq!(plan.power.iter().map(|cost| cost.amount).sum::<u16>(), 1);
    assert!(
        plan.modifiers
            .iter()
            .any(|modifier| modifier.label == "Hunters' Circle")
    );
    g.apply(action).unwrap();
    settle(&mut g);
    assert!(
        g.players[0]
            .units
            .iter()
            .any(|unit| unit.card_id == 108 && unit.ready)
    );
    assert_eq!(g.play_history.units_or_gears_here[0], [1, 0]);
}

#[test]
fn circle_hidden_play_on_opponents_turn_does_not_spend_the_next_own_turn_discount() {
    let mut g = circle_position(0);
    let hand_index = g.players[0].hand.len();
    g.players[0].hand.push(108);
    g.apply(Action::Hide {
        hand_index,
        lane: 0,
    })
    .unwrap();
    let hidden_id = g.players[0].hidden[0].id;
    g.apply(Action::EndTurn).unwrap();
    settle(&mut g);
    let spell_index = g.players[1].hand.len();
    g.players[1].hand.push(7);
    g.apply(Action::PlayCardWithTargets {
        hand_index: spell_index,
        lane: None,
        targets: vec![Target::Unit {
            player: 0,
            unit_id: 9950,
        }],
    })
    .unwrap();
    g.apply(Action::Pass).unwrap();
    assert_eq!(g.priority(), 0);
    let reveal = Action::PlayHidden {
        hidden_id,
        targets: vec![],
    };
    assert!(
        !g.cost_plan(&reveal)
            .unwrap()
            .modifiers
            .iter()
            .any(|modifier| modifier.label == "Hunters' Circle")
    );
    g.apply(reveal).unwrap();
    settle(&mut g);
    assert_eq!(g.play_history.units_or_gears_here[0], [1, 0]);
    g.apply(Action::EndTurn).unwrap();
    settle(&mut g);
    assert_eq!(g.active, 0);
    assert_eq!(g.play_history.units_or_gears_here[0], [0, 0]);
    let first = hand(&mut g, 69, Some(0));
    assert_eq!(g.cost_plan(&first).unwrap().energy, 6);
}
