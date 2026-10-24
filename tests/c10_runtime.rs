//! XP, Level, entry replacements and event history through public game actions.
use local_rift::game::{Action, ChainAbility, Decision, Difficulty, Game, Target, Unit};
use local_rift::play::{ExecutionChoices, PlayChoices, RepeatSelection};
use local_rift::rules::{self, EntryReadyRule};

fn game() -> Game {
    let mut g = Game::new_for_agents(3, 3, 92234, Difficulty::Tactical);
    g.start_match().unwrap();
    g.battlefields = [2, 2];
    g.battlefield_owner = [None; 2];
    for p in 0..2 {
        g.legend[p].ready = false;
        let state = &mut g.players[p];
        state.hand.clear();
        state.units.clear();
        state.gear.clear();
        state.hidden.clear();
        state.discard.clear();
        state.champion = None;
        state.runes = [0; 6];
        state.exhausted_runes = [0; 6];
        state.energy = 50;
        state.energy_pool = 50;
        state.power_pool = [20; 6];
        state.any_power = 0;
    }
    g
}
fn unit(g: &mut Game, player: usize, id: u32, card_id: usize, lane: Option<usize>) -> Target {
    g.players[player].units.push(Unit {
        id,
        card_id,
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
fn u(g: &Game, target: Target) -> &Unit {
    let Target::Unit { player, unit_id } = target else {
        panic!()
    };
    g.players[player]
        .units
        .iter()
        .find(|u| u.id == unit_id)
        .unwrap()
}
fn draft(g: &mut Game, card_id: usize, targets: Vec<Target>) -> Action {
    let p = g.priority();
    let hand_index = g.players[p].hand.len();
    g.players[p].hand.push(card_id);
    Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    }
}
fn cast(g: &mut Game, card_id: usize, targets: Vec<Target>) {
    let a = draft(g, card_id, targets);
    g.apply(a).unwrap();
}
fn resolve(g: &mut Game) {
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn settle(g: &mut Game) {
    for _ in 0..100 {
        let a = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::Predict { .. }) => Action::Choose { accept: false },
            Some(Decision::PredictCards {
                choice_id, cards, ..
            }) => Action::ResolvePredict {
                choice_id: *choice_id,
                recycle: vec![],
                top_order: (0..cards.len()).collect(),
            },
            Some(other) => panic!("unexpected decision {other:?}"),
            None if g.in_window() => Action::Pass,
            None => return,
        };
        g.apply(a).unwrap();
    }
    panic!("unsettled");
}
fn hunt(g: &mut Game, card: usize) {
    g.battlefields[0] = 3; // no simultaneous Conquer ability to order with Hunt
    unit(g, 0, 1000, card, None);
    g.apply(Action::Move {
        unit_id: 1000,
        lane: Some(0),
    })
    .unwrap();
    resolve(g); // empty-field showdown completes and Conquer finalizes Hunt
    assert!(
        g.chain
            .iter()
            .any(|i| matches!(i.ability, Some(ChainAbility::Hunt { .. })))
    );
}
fn academy(g: &mut Game) {
    g.battlefields[0] = 4;
    unit(g, 0, 5000, 69, Some(0));
    g.apply(Action::EndTurn).unwrap();
    g.apply(Action::EndTurn).unwrap();
    settle(g);
    // Replenish the explicit resource fixture after normal turn pool expiry.
    g.players[0].energy_pool = 50;
    g.players[0].energy = 50;
    g.players[0].runes = [0; 6];
    g.players[0].exhausted_runes = [0; 6];
    g.players[0].power_pool = [20; 6];
}
fn repeat(g: &Game, action: Action, targets: Vec<Target>) -> Action {
    let options = g.play_choices(&action).unwrap();
    assert_eq!(options.repeats.len(), 1);
    Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats: vec![RepeatSelection {
                option_id: options.repeats[0].id,
                choices: ExecutionChoices {
                    targets,
                    mode: None,
                },
            }],
            ..Default::default()
        },
    }
}

#[test]
fn actual_hunt_crosses_255_and_reaches_the_exact_host_maximum() {
    for (xp, expected) in [(254, 256), (u64::MAX - 2, u64::MAX)] {
        let mut g = game();
        g.players[0].xp = xp;
        hunt(&mut g, 22);
        resolve(&mut g);
        assert_eq!(g.players[0].xp, expected);
        assert!(g.gained_xp_this_turn(0));
    }
}
#[test]
fn overflow_restores_the_entire_attempted_public_resolution() {
    let mut g = game();
    g.players[0].xp = u64::MAX - 1;
    hunt(&mut g, 22);
    g.apply(Action::Pass).unwrap();
    let before = format!("{g:?}");
    let err = g.apply(Action::Pass).unwrap_err();
    assert!(err.starts_with("engine_capacity:"), "{err}");
    assert_eq!(format!("{g:?}"), before);
    assert!(!g.gained_xp_this_turn(0));
    // This is a controlled numeric boundary fixture, not a game XP-spend action.
    g.players[0].xp = u64::MAX - 2;
    g.apply(Action::Pass).unwrap();
    assert_eq!(g.players[0].xp, u64::MAX);
}
#[test]
fn scrutinizing_sergeant_counts_every_friendly_unit_beyond_255() {
    let mut g = game();
    for id in 1000..1260 {
        unit(&mut g, 0, id, 69, None);
    }
    cast(&mut g, 155, vec![]);
    resolve(&mut g);
    assert_eq!(g.players[0].units.len(), 261);
    assert_eq!(g.players[0].xp, 261);
    assert!(g.gained_xp_this_turn(0));
}
#[test]
fn checked_overflow_also_rolls_back_metadata_gain_and_legacy_death_gain() {
    // Sergeant exercises the metadata resolver, with a pop and earlier Pass.
    let mut g = game();
    g.players[0].xp = u64::MAX;
    cast(&mut g, 155, vec![]);
    g.apply(Action::Pass).unwrap();
    let before = format!("{g:?}");
    assert!(
        g.apply(Action::Pass)
            .unwrap_err()
            .starts_with("engine_capacity:")
    );
    assert_eq!(format!("{g:?}"), before);
    // Scuttle Crab's death ability also reveals a hand after gaining XP.
    let mut g = game();
    g.players[0].xp = u64::MAX;
    let crab = unit(&mut g, 0, 1000, 15, None);
    cast(&mut g, 294, vec![crab]);
    resolve(&mut g);
    assert!(!g.chain.is_empty());
    g.apply(Action::Pass).unwrap();
    let before = format!("{g:?}");
    assert!(
        g.apply(Action::Pass)
            .unwrap_err()
            .starts_with("engine_capacity:")
    );
    assert_eq!(format!("{g:?}"), before);
    assert!(!g.hidden_view[0]);
}
#[test]
fn all_new_hunts_use_canonical_captured_values() {
    for (card, amount) in [(337, 1), (338, 2), (339, 2), (340, 2), (341, 1)] {
        let mut g = game();
        hunt(&mut g, card);
        assert!(
            g.chain.iter().any(
                |i| matches!(i.ability, Some(ChainAbility::Hunt { amount: a, .. }) if a==amount)
            )
        );
        resolve(&mut g);
        assert_eq!(g.players[0].xp, u64::from(amount));
    }
}
#[test]
fn live_level_passives_follow_exact_thresholds_and_fall_when_xp_falls() {
    for (card, threshold, base) in [(337, 6, 2), (338, 3, 3), (339, 3, 3), (340, 3, 3)] {
        let mut g = game();
        let t = unit(&mut g, 0, 1000, card, None);
        g.players[0].xp = threshold - 1;
        assert_eq!(g.unit_might(u(&g, t)), base);
        assert!(!g.unit_keywords(u(&g, t)).ganking);
        assert_eq!(g.unit_keywords(u(&g, t)).deflect, 0);
        g.players[0].xp = threshold;
        assert_eq!(g.unit_might(u(&g, t)), base + 1);
        assert_eq!(g.unit_keywords(u(&g, t)).ganking, card == 338);
        assert_eq!(g.unit_keywords(u(&g, t)).deflect, u8::from(card == 339));
        g.players[0].xp = 0; // controlled live-passive fixture
        assert_eq!(g.unit_might(u(&g, t)), base);
    }
}
#[test]
fn conditional_entry_is_not_a_ready_event_and_does_not_later_exhaust() {
    for card in [334, 340] {
        assert_eq!(
            rules::entry_ready_rule(card),
            Some(EntryReadyRule::ControllerXpAtLeast(3))
        );
        for xp in [2, 3] {
            let mut g = game();
            g.players[0].xp = xp;
            // Hecarim observes every friendly Ready action; entry must not fire it.
            unit(&mut g, 0, 1000, 196, None);
            assert_eq!(g.printed_entry_ready(0, card), xp == 3);
            cast(&mut g, card, vec![]);
            let entering = g.players[0]
                .units
                .iter()
                .find(|u| u.card_id == card)
                .unwrap();
            assert_eq!(entering.ready, xp == 3);
            assert_eq!(entering.temporary_buff, 0);
            assert!(g.chain.is_empty());
            g.players[0].xp = 0;
            assert_eq!(g.players[0].units.last().unwrap().ready, xp == 3);
        }
    }
    assert_eq!(rules::entry_ready_rule(233), Some(EntryReadyRule::Always));
}
#[test]
fn wuju_draw_is_captured_at_completed_play_and_survives_later_xp_loss() {
    for xp in [5, 6] {
        let mut g = game();
        g.players[0].xp = xp;
        let before = g.players[0].hand.len();
        cast(&mut g, 341, vec![]);
        assert_eq!(g.players[0].hand.len(), before);
        assert_eq!(g.chain.len(), usize::from(xp == 6));
        g.players[0].xp = 0; // no XP-spend Reaction is implemented in this slice
        settle(&mut g);
        assert_eq!(g.players[0].hand.len(), before + usize::from(xp == 6));
    }
}
#[test]
fn wily_uses_actual_gain_history_before_or_after_entry_and_resets_each_turn() {
    for enters_before in [false, true] {
        let mut g = game();
        g.players[0].xp = 8;
        if enters_before {
            cast(&mut g, 342, vec![]);
        }
        assert!(!g.gained_xp_this_turn(0));
        cast(&mut g, 152, vec![]);
        resolve(&mut g); // actual Highlander gain1
        if !enters_before {
            cast(&mut g, 342, vec![]);
        }
        let t = Target::Unit {
            player: 0,
            unit_id: g.players[0]
                .units
                .iter()
                .find(|u| u.card_id == 342)
                .unwrap()
                .id,
        };
        assert_eq!(g.unit_might(u(&g, t)), 5);
        assert!(g.unit_keywords(u(&g, t)).ganking);
        g.players[0].xp = 0; // balance reduction does not erase an already recorded event
        assert!(g.gained_xp_this_turn(0));
        assert_eq!(g.unit_might(u(&g, t)), 5);
        g.apply(Action::EndTurn).unwrap();
        assert!(!g.gained_xp_this_turn(0));
        assert_eq!(g.unit_might(u(&g, t)), 4);
        assert!(!g.unit_keywords(u(&g, t)).ganking);
    }
}
#[test]
fn concentrate_uses_one_exclusive_tier_in_query_direct_and_staged_payment() {
    assert_eq!(rules::xp_energy_discount_tiers(336).len(), 2);
    for (xp, cost) in [(5, 5), (6, 3), (10, 3), (11, 1)] {
        let mut g = game();
        g.players[0].xp = xp;
        let a = draft(&mut g, 336, vec![]);
        let before = format!("{g:?}");
        let plan = g.cost_plan(&a).unwrap();
        assert_eq!(plan.energy, cost);
        assert_eq!(
            plan.modifiers
                .iter()
                .filter(|m| m.label == "Printed discount")
                .count(),
            usize::from(xp >= 6)
        );
        assert_eq!(format!("{g:?}"), before);
        let mut staged = g.clone();
        staged
            .apply(Action::BeginPayment {
                action: Box::new(a.clone()),
            })
            .unwrap();
        assert_eq!(staged.payment_plan_view(0).unwrap().energy, cost);
        staged.apply(Action::CompletePayment).unwrap();
        g.apply(a).unwrap();
        assert_eq!(g.players[0].energy, 50 - cost as u16);
        assert_eq!(staged.players[0].energy, g.players[0].energy);
        resolve(&mut g);
        assert_eq!(g.players[0].hand.len(), 2);
    }
}
#[test]
fn concentrate_freezes_tier_before_payment_and_reduces_total_repeat_cost_once() {
    let mut g = game();
    g.players[0].xp = 11;
    let a = draft(&mut g, 336, vec![]);
    g.apply(Action::BeginPayment {
        action: Box::new(a),
    })
    .unwrap();
    g.players[0].xp = 0; // controlled cost-freeze fixture; resource Add does not change XP yet
    g.apply(Action::CompletePayment).unwrap();
    assert_eq!(g.players[0].energy, 49);
    let mut g = game();
    academy(&mut g);
    g.players[0].xp = 11;
    let a = draft(&mut g, 336, vec![]);
    let a = repeat(&g, a, vec![]);
    assert_eq!(g.cost_plan(&a).unwrap().energy, 6); // 5+5-4, not (5-4)+(5-4)
    let before = g.players[0].hand.len();
    g.apply(a).unwrap();
    resolve(&mut g);
    assert_eq!(g.players[0].hand.len(), before - 1 + 4);
    assert_eq!(
        g.players[0].discard.iter().filter(|&&id| id == 336).count(),
        1
    );
}
#[test]
fn combat_experience_reads_live_controller_xp_and_pays_enemy_deflect() {
    for (xp, amount) in [(5, 1), (6, 3)] {
        let mut g = game();
        g.players[0].xp = xp;
        let enemy = unit(&mut g, 1, 1000, 339, None);
        g.players[1].xp = 3;
        let a = draft(&mut g, 335, vec![enemy]);
        assert_eq!(
            g.cost_plan(&a)
                .unwrap()
                .power
                .iter()
                .map(|p| p.amount)
                .sum::<u16>(),
            1
        );
        g.apply(a).unwrap();
        resolve(&mut g);
        assert_eq!(u(&g, enemy).temporary_buff, amount);
    }
    let mut g = game();
    g.players[0].xp = 6;
    let t = unit(&mut g, 0, 1000, 69, None);
    cast(&mut g, 335, vec![t]);
    g.players[0].xp = 5;
    resolve(&mut g);
    assert_eq!(u(&g, t).temporary_buff, 1);
}
#[test]
fn real_move_xp_response_changes_already_finalized_combat_experience() {
    let mut g = game();
    g.players[0].xp = 5;
    let nilah = unit(&mut g, 0, 1000, 344, Some(0));
    cast(&mut g, 335, vec![nilah]);
    // The caster keeps priority and may immediately add its own Flash.
    cast(&mut g, 9, vec![nilah]);
    resolve(&mut g);
    assert_eq!(u(&g, nilah).lane, None);
    assert_eq!(g.players[0].xp, 5); // Nilah's Move trigger is respondable
    resolve(&mut g);
    assert_eq!(g.players[0].xp, 6);
    resolve(&mut g);
    assert_eq!(u(&g, nilah).temporary_buff, 3);
}
#[test]
fn repeated_combat_experience_executes_full_live_instruction_for_each_target() {
    let mut g = game();
    academy(&mut g);
    g.players[0].xp = 6;
    let a = unit(&mut g, 0, 1000, 69, None);
    let b = unit(&mut g, 1, 1001, 70, None);
    let action = draft(&mut g, 335, vec![a]);
    let action = repeat(&g, action, vec![b]);
    g.apply(action).unwrap();
    resolve(&mut g);
    assert_eq!(u(&g, a).temporary_buff, 3);
    assert_eq!(u(&g, b).temporary_buff, 3);
    assert_eq!(
        g.players[0].discard.iter().filter(|&&id| id == 335).count(),
        1
    );
}
#[test]
fn countered_combat_experience_never_changes_might() {
    let mut g = game();
    g.players[0].xp = 6;
    let t = unit(&mut g, 0, 1000, 69, None);
    cast(&mut g, 335, vec![t]);
    let id = g.chain[0].id;
    cast(&mut g, 86, vec![Target::Spell { chain_id: id }]);
    resolve(&mut g);
    assert_eq!(u(&g, t).temporary_buff, 0);
    assert!(g.chain.is_empty());
}
#[test]
fn root_and_nilah_move_events_are_separate_from_entry_and_destination_specific() {
    for (card, to_field, to_base) in [(343, 2, 0), (344, 1, 1)] {
        let mut g = game();
        let a = draft(&mut g, card, vec![]);
        g.apply(Action::WithPlayChoices {
            action: Box::new(a),
            choices: PlayChoices {
                accelerate: true,
                ..Default::default()
            },
        })
        .unwrap();
        let id = g.players[0].units[0].id;
        assert!(g.players[0].units[0].ready);
        assert_eq!(g.players[0].xp, 0);
        g.apply(Action::Move {
            unit_id: id,
            lane: Some(0),
        })
        .unwrap();
        settle(&mut g);
        assert_eq!(g.players[0].xp, to_field);
        // Flash moves the exhausted unit without pretending it was readied.
        let target = Target::Unit {
            player: 0,
            unit_id: id,
        };
        cast(&mut g, 9, vec![target]);
        settle(&mut g);
        assert_eq!(u(&g, target).lane, None);
        assert_eq!(g.players[0].xp, to_field + to_base);
        assert!(!u(&g, target).ready);
    }
}
#[test]
fn recall_and_blocked_owner_movement_do_not_fabricate_move_xp() {
    let mut g = game();
    let root = unit(&mut g, 0, 1000, 343, Some(0));
    let foe = unit(&mut g, 1, 1001, 70, None);
    cast(&mut g, 16, vec![root, foe]);
    settle(&mut g);
    assert_eq!(g.players[0].xp, 0);
    assert!(g.players[0].hand.contains(&343));
    let mut g = game();
    unit(&mut g, 1, 1000, 20, Some(0));
    cast(&mut g, 344, vec![]);
    settle(&mut g);
    let id = g.players[0].units[0].id;
    // Ready fixture isolates Vex's movement lock from ordinary exhaustion.
    g.players[0].units[0].ready = true;
    let before = format!("{g:?}");
    assert!(
        g.apply(Action::Move {
            unit_id: id,
            lane: Some(1)
        })
        .is_err()
    );
    assert_eq!(format!("{g:?}"), before);
    assert_eq!(g.players[0].xp, 0);
}

#[test]
fn payment_complete_and_rollback_restore_outer_transaction_protection() {
    for complete in [false, true] {
        let mut g = game();
        let t = unit(&mut g, 0, 1000, 69, None);
        let action = draft(&mut g, 335, vec![t]);
        if !complete {
            g.players[0].energy = 0;
            g.players[0].energy_pool = 0;
        }
        g.apply(Action::BeginPayment {
            action: Box::new(action),
        })
        .unwrap();
        if complete {
            g.apply(Action::CompletePayment).unwrap();
            resolve(&mut g);
        } else {
            assert!(g.can_cancel_payment());
            g.apply(Action::Choose { accept: false }).unwrap();
        }
        g.players[0].energy = 50;
        g.players[0].energy_pool = 50;
        g.players[0].xp = u64::MAX;
        cast(&mut g, 155, vec![]);
        g.apply(Action::Pass).unwrap();
        let before = format!("{g:?}");
        assert!(
            g.apply(Action::Pass)
                .unwrap_err()
                .starts_with("engine_capacity:")
        );
        assert_eq!(format!("{g:?}"), before);
    }
}
#[test]
fn ai_capacity_failure_preserves_pass_count_counters_and_complete_state() {
    let mut g = game();
    g.players[0].xp = u64::MAX - 1;
    hunt(&mut g, 22);
    g.apply(Action::Pass).unwrap();
    let before = format!("{g:?}");
    assert!(g.step_ai().unwrap_err().starts_with("engine_capacity:"));
    assert_eq!(format!("{g:?}"), before);
}
