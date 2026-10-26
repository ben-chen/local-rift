//! Independent numeric boundaries through existing cards and public actions.
//!
//! Large modifiers are explicit numeric fixtures, not invented Buff actions or
//! claims that an ordinary opening position can reach a host boundary.
use local_rift::activated::AbilitySource;
use local_rift::deck::CustomDeck;
use local_rift::game::{Action, Decision, Difficulty, Game, Target, Unit};
use local_rift::gear::GearState;
use local_rift::play::{ExecutionChoices, PlayChoices, RepeatSelection};

fn position(legends: [usize; 2]) -> Game {
    let mut g = Game::new_with_custom(
        [0, 0],
        legends.map(|id| Some(CustomDeck::from_legend(id).unwrap())),
        0x4e554d45524943,
        Difficulty::Tactical,
        false,
    )
    .unwrap();
    g.start_match().unwrap();
    g.battlefields = [7, 9]; // No automatic scoring/damage instructions.
    g.battlefield_owner = [None; 2];
    for player in 0..2 {
        g.legend[player].ready = false;
        let p = &mut g.players[player];
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.runes = [0; 6];
        p.exhausted_runes = [0; 6];
        p.energy = 100;
        p.energy_pool = 100;
        p.power_pool = [30; 6];
        p.any_power = 0;
    }
    g
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
fn u(g: &Game, t: Target) -> &Unit {
    let Target::Unit { player, unit_id } = t else {
        panic!("unit target")
    };
    g.players[player]
        .units
        .iter()
        .find(|u| u.id == unit_id)
        .unwrap()
}
fn um(g: &mut Game, t: Target) -> &mut Unit {
    let Target::Unit { player, unit_id } = t else {
        panic!("unit target")
    };
    g.players[player]
        .units
        .iter_mut()
        .find(|u| u.id == unit_id)
        .unwrap()
}
fn numeric_might(g: &mut Game, t: Target, signed: i64) {
    let base = i128::from(g.catalog[u(g, t).card_id].might);
    um(g, t).temporary_buff = i64::try_from(i128::from(signed) - base).unwrap();
}
fn stats(g: &Game, t: Target) -> (i64, u64) {
    let s = g.try_unit_stats(u(g, t)).unwrap();
    (s.signed_might, s.might)
}
fn draft(g: &mut Game, card: usize, targets: Vec<Target>) -> Action {
    let p = g.priority();
    let hand_index = g.players[p].hand.len();
    g.players[p].hand.push(card);
    Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    }
}
fn cast(g: &mut Game, card: usize, targets: Vec<Target>) {
    let a = draft(g, card, targets);
    g.apply(a).unwrap();
    decline_optional(g);
}
fn decline_optional(g: &mut Game) {
    // These legal legend homes retain their real optional trigger choices.
    // Decline them explicitly instead of removing the legend's behavior.
    for _ in 0..20 {
        let action = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::Legend { .. } | Decision::Battlefield { .. }) => {
                Action::Choose { accept: false }
            }
            _ => return,
        };
        g.apply(action).unwrap();
    }
    panic!("optional trigger finalization did not finish")
}
fn resolve(g: &mut Game) {
    decline_optional(g);
    g.apply(Action::Pass).unwrap();
    decline_optional(g);
    g.apply(Action::Pass).unwrap();
    decline_optional(g);
}
fn settle(g: &mut Game) {
    for _ in 0..100 {
        let action = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::Legend { .. } | Decision::Battlefield { .. }) => {
                Action::Choose { accept: false }
            }
            Some(other) => panic!("unexpected choice: {other:?}"),
            None if g.in_window() => Action::Pass,
            None => return,
        };
        g.apply(action).unwrap();
    }
    panic!("unsettled numeric fixture")
}
fn capacity(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    let error = g.apply(action).unwrap_err();
    assert!(error.starts_with("engine_capacity:"), "{error}");
    assert_eq!(
        format!("{g:?}"),
        before,
        "capacity is an atomic command failure"
    );
}
fn rejected(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action).is_err());
    assert_eq!(format!("{g:?}"), before);
}
fn academy(g: &mut Game) {
    g.battlefields[0] = 4;
    unit(g, 0, 9000, 70, Some(0));
    g.apply(Action::EndTurn).unwrap();
    g.apply(Action::EndTurn).unwrap();
    settle(g);
    g.players[0].energy = 100;
    g.players[0].energy_pool = 100;
    g.players[0].power_pool = [30; 6];
}
fn all_repeats(g: &Game, action: Action, targets: Vec<Target>) -> Action {
    let options = g.play_choices(&action).unwrap();
    assert!(!options.repeats.is_empty());
    Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats: options
                .repeats
                .into_iter()
                .map(|o| RepeatSelection {
                    option_id: o.id,
                    choices: ExecutionChoices {
                        targets: targets.clone(),
                        mode: None,
                    },
                })
                .collect(),
            ..Default::default()
        },
    }
}

#[test]
fn signed_queries_floor_only_negative_references_and_preserve_the_game() {
    let mut g = position([0, 0]);
    let t = unit(&mut g, 0, 100, 273, Some(0)); // Zero printed Might.
    for n in [
        i64::MIN,
        -32769,
        -32768,
        -1,
        0,
        1,
        255,
        256,
        32767,
        32768,
        65535,
        65536,
        i64::MAX,
    ] {
        numeric_might(&mut g, t, n);
        let before = format!("{g:?}");
        assert_eq!(stats(&g, t), (n, n.max(0) as u64));
        assert_eq!(g.unit_might(u(&g, t)).unwrap(), n.max(0) as u64);
        assert_eq!(g.try_lane_might(0, 0).unwrap(), n.max(0) as u64);
        g.validate_numeric_state().unwrap();
        assert_eq!(format!("{g:?}"), before);
    }
    um(&mut g, t).stunned = true;
    assert_eq!(g.try_lane_might(0, 0).unwrap(), 0);
}

#[test]
fn combat_experience_crosses_each_old_positive_boundary_exactly() {
    for start in [255, 32767, 65535] {
        let mut g = position([0, 0]);
        let t = unit(&mut g, 0, 100, 273, None);
        numeric_might(&mut g, t, start);
        cast(&mut g, 335, vec![t]);
        resolve(&mut g);
        assert_eq!(stats(&g, t), (start + 1, (start + 1) as u64));
    }
}

#[test]
fn frigid_touch_keeps_signed_values_beyond_the_old_negative_boundary() {
    let mut g = position([5, 0]); // Mind spell; opposing Calm unit.
    let t = unit(&mut g, 1, 100, 273, None);
    numeric_might(&mut g, t, -32767);
    cast(&mut g, 136, vec![t]);
    resolve(&mut g);
    assert_eq!(stats(&g, t), (-32769, 0));
    assert!(
        g.players[1].units.iter().any(|v| v.id == 100),
        "zero damage never becomes a Kill"
    );
}

#[test]
fn minimum_might_reduction_neither_wraps_large_values_nor_raises_low_ones() {
    for (start, end) in [(65537, 65533), (4, 1), (1, 1), (0, 0), (-32769, -32769)] {
        let mut g = position([5, 0]);
        let t = unit(&mut g, 1, 100, 273, None);
        numeric_might(&mut g, t, start);
        cast(&mut g, 81, vec![t]);
        resolve(&mut g);
        assert_eq!(stats(&g, t), (end, end.max(0) as u64));
    }
}

#[test]
fn high_note_counts_all_exhausted_tokens_in_its_live_might_change() {
    let mut g = position([5, 0]);
    for id in 1000..1300 {
        let t = unit(&mut g, 0, id, 87, None); // Recruit's actual1-Might token.
        let token = um(&mut g, t);
        token.token = true;
        token.buff = 1;
        token.ready = false;
    }
    let target = unit(&mut g, 1, 100, 273, None);
    numeric_might(&mut g, target, 1000);
    cast(&mut g, 229, vec![target]);
    resolve(&mut g);
    assert_eq!(stats(&g, target), (700, 700));
    assert_eq!(g.players[0].units.len(), 300);
    assert!(
        g.players[0]
            .units
            .iter()
            .all(|unit| g.unit_might(unit).unwrap() == 1)
    );
}

#[test]
fn invalid_derived_might_is_a_pure_error_not_a_clipped_value() {
    let mut g = position([1, 1]);
    let t = unit(&mut g, 0, 100, 70, None);
    // Both stored fields fit, but printed10 plus this modifier does not fit
    // the declared signed-Might representation.
    um(&mut g, t).temporary_buff = i64::MAX;
    let before = format!("{g:?}");
    assert!(
        g.try_unit_stats(u(&g, t))
            .unwrap_err()
            .starts_with("engine_capacity:")
    );
    assert!(
        g.unit_might(u(&g, t))
            .unwrap_err()
            .starts_with("engine_capacity:")
    );
    assert!(
        g.validate_numeric_state()
            .unwrap_err()
            .starts_with("engine_capacity:")
    );
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn switcheroo_can_exchange_signed_extremes_with_wider_intermediate_deltas() {
    for (a, b) in [(i64::MIN, i64::MAX), (-32769, 65536), (-1, 256)] {
        let mut g = position([0, 0]);
        let first = unit(&mut g, 0, 100, 273, Some(0));
        let second = unit(&mut g, 0, 101, 273, Some(0));
        numeric_might(&mut g, first, a);
        numeric_might(&mut g, second, b);
        cast(&mut g, 44, vec![first, second]);
        resolve(&mut g);
        assert_eq!(stats(&g, first), (b, b.max(0) as u64));
        assert_eq!(stats(&g, second), (a, a.max(0) as u64));
        g.apply(Action::EndTurn).unwrap();
        assert_eq!(stats(&g, first), (0, 0));
        assert_eq!(stats(&g, second), (0, 0));
    }
}

#[test]
fn challenge_uses_complete_simultaneous_unit_damage_without_spell_bonuses() {
    for amount in [400, 32768, 65536] {
        let mut g = position([1, 1]); // Calm/Body supports both units and Challenge.
        let first = unit(&mut g, 0, 100, 70, None);
        let second = unit(&mut g, 1, 101, 70, Some(1));
        g.battlefields[1] = 3; // Void Gate must not amplify Unit-dealt Fight damage.
        numeric_might(&mut g, first, amount);
        numeric_might(&mut g, second, amount + 100);
        cast(&mut g, 82, vec![first, second]);
        resolve(&mut g);
        assert!(g.players[0].units.is_empty());
        assert_eq!(u(&g, second).damage, amount as u64);
        assert_eq!(stats(&g, second).1, (amount + 100) as u64);
    }
}

#[test]
fn clash_of_giants_preserves_the_three_hundred_versus_two_eighty_survivor() {
    // Exact bounded pre-fix reproduction: both values used to display255 and
    // both died. The real spell must leave the300-Might unit with280damage.
    let mut g = position([1, 1]);
    let stronger = unit(&mut g, 0, 100, 273, None);
    let weaker = unit(&mut g, 1, 101, 273, None);
    numeric_might(&mut g, stronger, 300);
    numeric_might(&mut g, weaker, 280);
    cast(&mut g, 83, vec![stronger, weaker]);
    resolve(&mut g);
    assert_eq!(stats(&g, stronger), (300, 300));
    assert_eq!(u(&g, stronger).damage, 280);
    assert!(g.players[1].units.is_empty());
}

#[test]
fn caitlyns_current_source_might_is_live_and_not_limited_to_a_byte() {
    let mut g = position([0, 0]);
    let source = unit(&mut g, 0, 100, 238, Some(0));
    let target = unit(&mut g, 1, 101, 71, Some(1));
    numeric_might(&mut g, source, 65536);
    numeric_might(&mut g, target, 100000);
    g.apply(Action::ActivateAbility {
        source: AbilitySource::Unit {
            player: 0,
            unit_id: 100,
        },
        ability_index: 0,
        targets: vec![target],
    })
    .unwrap();
    g.players[0].xp = 6;
    cast(&mut g, 335, vec![source]); // Real Reaction changes the live source.
    resolve(&mut g);
    resolve(&mut g);
    assert_eq!(u(&g, target).damage, 65539);
}

#[test]
fn caitlyn_source_departure_does_not_use_a_stale_large_might_snapshot() {
    let mut g = position([0, 0]);
    let source = unit(&mut g, 0, 100, 238, Some(0));
    let target = unit(&mut g, 1, 101, 71, Some(1));
    let friend = unit(&mut g, 1, 102, 71, None);
    numeric_might(&mut g, source, 65536);
    numeric_might(&mut g, target, 100000);
    g.apply(Action::ActivateAbility {
        source: AbilitySource::Unit {
            player: 0,
            unit_id: 100,
        },
        ability_index: 0,
        targets: vec![target],
    })
    .unwrap();
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 16, vec![friend, source]); // Opponent's real Star-Crossed Reaction.
    settle(&mut g);
    assert!(g.players[0].units.is_empty());
    assert_eq!(u(&g, target).damage, 0);
}

#[test]
fn numeric_target_bound_and_true_kill_use_different_rules() {
    let mut g = position([5, 0]);
    let target = unit(&mut g, 1, 100, 273, Some(0));
    numeric_might(&mut g, target, 65537);
    let kill_small = draft(&mut g, 295, vec![target]);
    rejected(&mut g, kill_small);
    cast(&mut g, 294, vec![target]);
    resolve(&mut g);
    assert!(
        g.players[1].units.is_empty(),
        "Vengeance is a Kill, not capped damage"
    );
}

#[test]
fn combat_allocates_all_seventy_thousand_with_full_lethal_and_order_constraints() {
    let mut g = position([5, 0]);
    let attacker = unit(&mut g, 0, 100, 69, None);
    numeric_might(&mut g, attacker, 70000);
    let tank = unit(&mut g, 1, 200, 71, Some(0));
    let ordinary = unit(&mut g, 1, 201, 71, Some(0));
    let backline = unit(&mut g, 1, 202, 211, Some(0));
    numeric_might(&mut g, tank, 399); // Attached Doran's Shield adds the final1.
    numeric_might(&mut g, ordinary, 500);
    numeric_might(&mut g, backline, 600);
    um(&mut g, tank).damage = 100;
    g.players[1].gear.push(GearState {
        id: 500,
        card_id: 121,
        ready: true,
        lane: None,
        attached_to: Some(200),
        temporary: false,
        token: false,
    });
    g.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::CombatDamage {
            player: 0,
            total: 70000,
            ..
        })
    ));
    let expected = vec![(200, 300), (201, 500), (202, 69200)];
    assert_eq!(
        g.automatic_damage_assignment(0, 0, 70000).unwrap(),
        expected
    );
    for assignments in [
        vec![(201, 500), (200, 300), (202, 69200)],
        vec![(200, 299), (201, 500), (202, 69201)],
        vec![(200, 301), (201, 500), (202, 69199)],
        vec![(200, 300), (201, 500), (202, 69201)],
        vec![(200, 300), (200, 69700)],
    ] {
        rejected(&mut g, Action::AssignDamage { assignments });
    }
    g.apply(Action::AssignDamage {
        assignments: expected.clone(),
    })
    .unwrap();
    assert_eq!(g.submitted_combat_damage(0), Some(expected.as_slice()));
    assert_eq!(
        u(&g, tank).damage,
        100,
        "no packets before both assignments"
    );
    let Some(Decision::CombatDamage {
        player: 1, total, ..
    }) = g.decision
    else {
        panic!("defender assignment")
    };
    assert_eq!(total, 1500);
    g.apply(Action::AssignDamage {
        assignments: vec![(100, total)],
    })
    .unwrap();
    settle(&mut g);
    assert!(g.players[1].units.is_empty());
    assert_eq!(stats(&g, attacker).1, 70000);
    assert_eq!(
        u(&g, attacker).damage,
        0,
        "normal combat cleanup still heals"
    );
}

#[test]
fn signed_capacity_errors_restore_the_resolving_command_and_retry_once() {
    for (start, spell, fix, expected, legends) in [
        (i64::MAX, 335, i64::MAX - 1, i64::MAX, [0, 0]),
        (i64::MIN, 136, i64::MIN + 2, i64::MIN, [5, 0]),
    ] {
        let mut g = position(legends);
        let t = unit(&mut g, 1, 100, 273, None);
        numeric_might(&mut g, t, start);
        cast(&mut g, spell, vec![t]);
        g.apply(Action::Pass).unwrap();
        capacity(&mut g, Action::Pass);
        numeric_might(&mut g, t, fix); // Only the explicit numeric fixture changes.
        g.apply(Action::Pass).unwrap();
        assert_eq!(stats(&g, t), (expected, expected.max(0) as u64));
        assert_eq!(
            g.players[0]
                .discard
                .iter()
                .filter(|&&id| id == spell)
                .count(),
            1
        );
    }
}

#[test]
fn largest_lane_sum_is_exact_and_small_real_increment_overflows_atomically() {
    let mut g = position([0, 0]);
    let first = unit(&mut g, 0, 100, 273, Some(0));
    let second = unit(&mut g, 0, 101, 273, Some(0));
    let last = unit(&mut g, 0, 102, 71, Some(0));
    numeric_might(&mut g, first, i64::MAX);
    numeric_might(&mut g, second, i64::MAX);
    numeric_might(&mut g, last, 1);
    assert_eq!(g.try_lane_might(0, 0).unwrap(), u64::MAX);
    g.validate_numeric_state().unwrap();
    cast(&mut g, 335, vec![last]);
    g.apply(Action::Pass).unwrap();
    capacity(&mut g, Action::Pass);
    assert_eq!(stats(&g, last).1, 1);
}

#[test]
fn three_actual_fights_overflow_only_on_third_packet_and_roll_back_the_whole_parent() {
    let mut g = position([1, 1]);
    academy(&mut g);
    let first = unit(&mut g, 0, 100, 70, None);
    let second = unit(&mut g, 1, 101, 70, Some(1));
    numeric_might(&mut g, first, i64::MAX);
    numeric_might(&mut g, second, i64::MAX);
    let action = draft(&mut g, 217, vec![first, second]);
    assert_eq!(g.play_choices(&action).unwrap().repeats.len(), 2);
    let action = all_repeats(&g, action, vec![first, second]);
    g.apply(action).unwrap();
    assert_eq!(g.chain.last().unwrap().executions.len(), 3);
    g.apply(Action::Pass).unwrap();
    capacity(&mut g, Action::Pass);
    assert_eq!(u(&g, first).damage, 0);
    assert_eq!(u(&g, second).damage, 0);
    numeric_might(&mut g, first, 70000);
    numeric_might(&mut g, second, 70000);
    g.apply(Action::Pass).unwrap();
    assert!(!g.players[0].units.iter().any(|v| v.id == 100));
    assert!(g.players[1].units.is_empty());
    assert_eq!(
        g.players[0].discard.iter().filter(|&&id| id == 217).count(),
        1
    );
}

#[test]
fn repeated_supernova_overflow_restores_earlier_draw_burnout_and_rng() {
    let mut g = position([5, 0]);
    // Academy needs only a controlled holder; this mapped Mind unit is legal here.
    g.battlefields[0] = 4;
    unit(&mut g, 0, 9000, 69, Some(0));
    g.apply(Action::EndTurn).unwrap();
    g.apply(Action::EndTurn).unwrap();
    settle(&mut g);
    g.players[0].energy = 100;
    g.players[0].energy_pool = 100;
    g.players[0].power_pool = [30; 6];
    let target = unit(&mut g, 1, 100, 273, None);
    numeric_might(&mut g, target, i64::MAX);
    g.players[0].deck.clear();
    g.players[0].discard = vec![69, 73, 69, 73];
    let action = draft(&mut g, 291, vec![target]);
    let action = all_repeats(&g, action, vec![target]);
    g.apply(action).unwrap();
    // A controlled captured-bonus boundary, not an invented printed modifier.
    g.chain.last_mut().unwrap().bonus_damage = u64::MAX - 10;
    g.apply(Action::Pass).unwrap();
    capacity(&mut g, Action::Pass);
    assert_eq!(u(&g, target).damage, 0);
    assert_eq!(g.players[0].deck.len(), 0);
    assert_eq!(g.players[0].discard, vec![69, 73, 69, 73]);
}

#[test]
fn bellows_resolution_group_choice_is_restored_after_numeric_failure() {
    let mut g = position([5, 0]);
    let a = unit(&mut g, 1, 100, 71, Some(0));
    let b = unit(&mut g, 1, 101, 71, Some(0));
    let c = unit(&mut g, 1, 102, 71, Some(0));
    cast(&mut g, 50, vec![a, b, c]);
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 9, vec![b, c]); // Actual opponent Flash splits the original group.
    resolve(&mut g);
    resolve(&mut g);
    let Some(Decision::ResolutionTargets { ref groups, .. }) = g.decision else {
        panic!("real split targets must require a choice")
    };
    let index = groups.iter().position(|v| v == &vec![a]).unwrap();
    g.chain.last_mut().unwrap().bonus_damage = u64::MAX;
    capacity(&mut g, Action::ChooseResolutionGroup { index });
    g.chain.last_mut().unwrap().bonus_damage = 0;
    g.apply(Action::ChooseResolutionGroup { index }).unwrap();
    assert_eq!(u(&g, a).damage, 1);
    assert_eq!(u(&g, b).damage, 0);
    assert_eq!(u(&g, c).damage, 0);
    assert!(g.decision.is_none());
}

#[test]
fn complete_payment_numeric_failure_retains_the_plan_and_deliberate_add_steps() {
    let mut g = position([5, 0]); // Mind/Order: Commander, Mega-Mech, and Energy Conduit.
    let target = unit(&mut g, 0, 100, 69, Some(0));
    numeric_might(&mut g, target, i64::MAX - 1);
    g.players[0].energy = 0;
    g.players[0].energy_pool = 0;
    g.players[0].gear.push(GearState {
        id: 500,
        card_id: 130,
        ready: true,
        lane: None,
        attached_to: None,
        temporary: false,
        token: false,
    });
    let hand_index = g.players[0].hand.len();
    g.players[0].hand.push(320);
    g.apply(Action::BeginPayment {
        action: Box::new(Action::PlayCardWithTargets {
            hand_index,
            lane: Some(0),
            targets: vec![],
        }),
    })
    .unwrap();
    g.apply(Action::ActivateGear { gear_id: 500 }).unwrap();
    // Controlled continuation boundary, not a legal in-payment board-changing action.
    numeric_might(&mut g, target, i64::MAX);
    let energy = g.payment_plan_view(0).unwrap().energy as u16;
    g.players[0].energy = energy;
    g.players[0].energy_pool = energy;
    capacity(&mut g, Action::CompletePayment);
    assert_eq!(g.payment_plan_view(0).unwrap().pending_card, Some(320));
    assert!(!g.players[0].gear[0].ready);
    assert!(
        g.can_cancel_payment()
            .unwrap_err()
            .starts_with("engine_capacity:"),
        "capacity is not proof of legal impossibility"
    );
    numeric_might(&mut g, target, i64::MAX - 1);
    g.apply(Action::CompletePayment).unwrap();
    assert!(g.payment_plan_view(0).is_none());
    assert_eq!(stats(&g, target).0, i64::MAX);
    assert_eq!(
        g.players[0]
            .units
            .iter()
            .filter(|u| u.card_id == 320)
            .count(),
        1
    );
}

#[test]
fn required_resolution_choices_preserve_capacity_errors_and_executable_alternatives() {
    let mut g = position([5, 0]);
    let a = unit(&mut g, 1, 100, 71, Some(0));
    let b = unit(&mut g, 1, 101, 71, Some(0));
    let c = unit(&mut g, 1, 102, 71, Some(0));
    cast(&mut g, 50, vec![a, b, c]);
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 9, vec![b, c]);
    resolve(&mut g);
    resolve(&mut g);
    let Some(Decision::ResolutionTargets { ref groups, .. }) = g.decision else {
        panic!("the real Bellows/Flash sequence must require a resolution group")
    };
    let base_group = groups
        .iter()
        .position(|group| group == &vec![b, c])
        .unwrap();
    g.chain.last_mut().unwrap().bonus_damage = u64::MAX;
    let before = format!("{g:?}");
    assert!(
        g.legal_actions()
            .unwrap_err()
            .starts_with("engine_capacity:")
    );
    assert_eq!(format!("{g:?}"), before);
    assert!(g.step_ai().unwrap_err().starts_with("engine_capacity:"));
    assert_eq!(format!("{g:?}"), before);

    // The captured modifier is a numeric fixture. Void Gate makes only the
    // remaining battlefield group overflow; the two units recalled to base
    // can receive the representable packet. A failed alternative is not a veto.
    g.battlefields[0] = 3;
    g.chain.last_mut().unwrap().bonus_damage = u64::MAX - 1;
    let actions = g.legal_actions().unwrap();
    assert_eq!(
        actions,
        vec![Action::ChooseResolutionGroup { index: base_group }]
    );
    assert!(g.step_ai().unwrap());
    assert!(g.decision.is_none());
    assert_eq!(
        g.players[1].units.iter().map(|u| u.id).collect::<Vec<_>>(),
        vec![100]
    );
    assert_eq!(u(&g, a).damage, 0);
}

#[test]
fn mandatory_pending_play_capacity_never_becomes_permission_to_cancel() {
    let mut g = position([5, 0]); // Mind/Order: Last Rites, Garen, and Mega-Mechs.
    let bearer = unit(&mut g, 0, 100, 69, None);
    let base = unit(&mut g, 0, 101, 69, None);
    let other_field = unit(&mut g, 0, 102, 69, Some(1));
    g.players[0].gear.push(GearState {
        id: 500,
        card_id: 38,
        ready: true,
        lane: None,
        attached_to: Some(100),
        temporary: false,
        token: false,
    });
    g.players[0].discard = vec![320];
    g.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    resolve(&mut g);
    assert!(matches!(g.decision, Some(Decision::OptionalPlay { .. })));
    g.apply(Action::Choose { accept: true }).unwrap();
    g.apply(Action::SelectCards { indices: vec![0] }).unwrap();
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::PendingPlay { card_id: 320, .. })
    ));

    // Explicit numeric boundary after the actual trash-play instruction: each
    // legal destination has an ally whose Might Garen's entry would overflow.
    numeric_might(&mut g, bearer, i64::MAX - 2); // Last Rites contributes +2.
    numeric_might(&mut g, base, i64::MAX);
    numeric_might(&mut g, other_field, i64::MAX);
    for target in [bearer, base, other_field] {
        assert_eq!(stats(&g, target).0, i64::MAX);
    }
    g.validate_numeric_state().unwrap();
    let before = format!("{g:?}");
    assert!(
        g.can_decline_trash_play()
            .unwrap_err()
            .starts_with("engine_capacity:")
    );
    assert_eq!(format!("{g:?}"), before);
    assert!(
        g.legal_actions()
            .unwrap_err()
            .starts_with("engine_capacity:")
    );
    assert_eq!(format!("{g:?}"), before);
    capacity(&mut g, Action::Choose { accept: false });
    assert!(g.step_ai().unwrap_err().starts_with("engine_capacity:"));
    assert_eq!(format!("{g:?}"), before);

    // A representable base destination is enough to prove the mandatory play
    // can complete; the other two failing alternatives must not veto it.
    numeric_might(&mut g, base, i64::MAX - 1);
    assert!(!g.can_decline_trash_play().unwrap());
    assert!(g.step_ai().unwrap());
    assert!(g.pending_cards().is_empty());
    assert_eq!(stats(&g, base).0, i64::MAX);
    assert!(
        g.players[0]
            .units
            .iter()
            .any(|u| u.card_id == 320 && u.lane.is_none())
    );
}
