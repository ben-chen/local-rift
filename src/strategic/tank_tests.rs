//! Independent two-defender combat oracle: spend Block only for a useful trade,
//! and let the opponent choose its most damaging legal allocation.
use super::*;

#[test]
fn block_tank_benchmark_strategic_damage_can_remove_the_larger_threat() {
    for actor in 0..2 {
        let mut g = test_support::position(actor, 0, 461701);
        g.difficulty = Difficulty::Strategic;
        let attacker = unit(&mut g, actor, 71, None);
        let small = unit(&mut g, 1 - actor, 110, Some(0));
        let large = unit(&mut g, 1 - actor, 71, Some(0));
        g.battlefield_owner[0] = Some(1 - actor);
        g.apply(Action::Move {
            unit_id: id(attacker),
            lane: Some(0),
        })
        .unwrap();
        g.apply(Action::Pass).unwrap();
        g.apply(Action::Pass).unwrap();
        assert!(
            matches!(g.decision, Some(Decision::CombatDamage { player, total: 5, .. }) if player == actor)
        );
        let action = g.choose_ai_action().unwrap();
        let mut forecast = g.clone();
        forecast.strategic_settle(g.turn).unwrap();
        assert!(forecast.unit_ref(small).is_some());
        assert!(forecast.unit_ref(large).is_none());
        // Five damage can kill the 5-Might Phantom. Killing the 3-Might
        // defending Poro instead leaves the larger opposing threat alive.
        let outcome = finish(g, action.clone(), actor, small, large, attacker);
        assert!(
            outcome.0 && !outcome.1 && !outcome.2,
            "seat {actor}: {action:?} leaves {outcome:?}; should remove Phantom and leave Poro"
        );
    }
}

fn start_combat(g: &mut Game, attacker: Target) {
    g.apply(Action::Move {
        unit_id: id(attacker),
        lane: Some(0),
    })
    .unwrap();
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
    assert!(matches!(g.decision, Some(Decision::CombatDamage { .. })));
}

#[test]
fn strategic_damage_trades_multiple_units_and_uses_existing_damage() {
    for actor in 0..2 {
        for damaged in [false, true] {
            let mut g = test_support::position(actor, 0, 461702);
            g.difficulty = Difficulty::Strategic;
            let attacker = unit(&mut g, actor, 71, None);
            let small = [
                unit(&mut g, 1 - actor, 88, Some(0)),
                unit(&mut g, 1 - actor, 88, Some(0)),
            ];
            let large = unit(&mut g, 1 - actor, 71, Some(0));
            if damaged {
                g.unit_mut(large).unwrap().damage = 2;
            }
            g.battlefield_owner[0] = Some(1 - actor);
            start_combat(&mut g, attacker);
            let before = format!("{g:?}");
            let action = g.choose_ai_action().unwrap();
            assert_eq!(format!("{g:?}"), before, "planning mutated the real game");
            g.apply(action).unwrap();
            g.strategic_settle(g.turn).unwrap();
            let small_left = small.iter().filter(|&&t| g.unit_ref(t).is_some()).count();
            if damaged {
                // Three damage finishes Phantom; two finishes one Poro.
                assert_eq!(small_left, 1);
                assert!(g.unit_ref(large).is_none());
            } else {
                // Two Poros together are more valuable than the single Phantom.
                assert_eq!(small_left, 0);
                assert!(g.unit_ref(large).is_some());
            }
        }
    }
}

#[test]
fn strategic_damage_obeys_tank_backline_and_keeps_both_dual_orders() {
    use std::collections::BTreeSet;
    for actor in 0..2 {
        for marked in [0, 1] {
            for total in 1..=24 {
                let mut g = test_support::position(actor, 0, 461703);
                let attacker = unit(&mut g, actor, 71, None);
                g.unit_mut(attacker).unwrap().temporary_buff = total - 5;
                let tank = unit(&mut g, 1 - actor, 91, Some(0)); // 6, Tank.
                let ordinary = unit(&mut g, 1 - actor, 71, Some(0)); // 5.
                let backline = unit(&mut g, 1 - actor, 13, Some(0)); // 3, Backline.
                let dual = unit(&mut g, 1 - actor, 13, Some(0)); // 3, Tank + Backline.
                g.grant_turn_keyword(dual, crate::turn_keywords::TurnKeyword::Tank);
                for target in [tank, ordinary, backline, dual] {
                    g.unit_mut(target).unwrap().damage = marked;
                }
                g.battlefield_owner[0] = Some(1 - actor);
                start_combat(&mut g, attacker);
                let targets = [tank, ordinary, backline, dual];
                let mut expected = BTreeSet::new();
                // Independent brute-force order oracle for four physical units.
                // No AI ordering/ranking or priority helpers are used here.
                for a in 0..4 {
                    for b in 0..4 {
                        for c in 0..4 {
                            for d in 0..4 {
                                let order = [a, b, c, d];
                                if order.iter().copied().collect::<BTreeSet<_>>().len() != 4 {
                                    continue;
                                }
                                let mut remaining = u64::try_from(total).unwrap();
                                let mut assignments = Vec::new();
                                for (index, target_index) in order.into_iter().enumerate() {
                                    if remaining == 0 {
                                        break;
                                    }
                                    let lethal = [6, 5, 3, 3][target_index] - marked;
                                    let amount = if index == 3 {
                                        remaining
                                    } else {
                                        remaining.min(lethal)
                                    };
                                    assignments.push((id(targets[target_index]), amount));
                                    remaining -= amount;
                                }
                                if g.clone()
                                    .apply(Action::AssignDamage {
                                        assignments: assignments.clone(),
                                    })
                                    .is_ok()
                                {
                                    expected.insert(assignments);
                                }
                            }
                        }
                    }
                }
                let candidates = g.strategic_damage_candidates().unwrap();
                let actual: BTreeSet<_> = candidates
                    .into_iter()
                    .map(|action| {
                        let Action::AssignDamage { assignments } = action else {
                            panic!("damage action")
                        };
                        assignments
                    })
                    .collect();
                assert_eq!(
                    actual, expected,
                    "seat {actor}, marked {marked}, total {total}"
                );
                if total == 1 {
                    assert_eq!(
                        actual,
                        BTreeSet::from([vec![(id(tank), 1)], vec![(id(dual), 1)]])
                    );
                }
                if total == 24 {
                    assert!(actual.iter().any(|order| order[0].0 == id(dual)));
                    assert!(
                        actual
                            .iter()
                            .any(|order| order.last().unwrap().0 == id(dual))
                    );
                    for order in actual {
                        let pos = |t| order.iter().position(|&(u, _)| u == id(t)).unwrap();
                        assert!(pos(tank) < pos(ordinary) && pos(ordinary) < pos(backline));
                    }
                }
            }
        }
    }
}

#[test]
fn strategic_damage_dual_keyword_cannot_go_between_ordinary_units() {
    for actor in 0..2 {
        let mut g = test_support::position(actor, 0, 461705);
        let attacker = unit(&mut g, actor, 71, None);
        g.unit_mut(attacker).unwrap().buff = 20;
        unit(&mut g, 1 - actor, 71, Some(0));
        let dual = unit(&mut g, 1 - actor, 13, Some(0));
        unit(&mut g, 1 - actor, 71, Some(0));
        g.grant_turn_keyword(dual, crate::turn_keywords::TurnKeyword::Tank);
        g.battlefield_owner[0] = Some(1 - actor);
        start_combat(&mut g, attacker);
        let mut positions = std::collections::BTreeSet::new();
        for action in g.strategic_damage_candidates().unwrap() {
            let Action::AssignDamage { assignments } = action else {
                panic!("damage action")
            };
            positions.insert(
                assignments
                    .iter()
                    .position(|&(u, _)| u == id(dual))
                    .unwrap(),
            );
        }
        assert_eq!(positions, std::collections::BTreeSet::from([0, 2]));
    }
}

#[test]
fn strategic_damage_crowded_board_is_bounded_and_private_information_independent() {
    for actor in 0..2 {
        let mut g = test_support::position(actor, 0, 461704);
        g.difficulty = Difficulty::Strategic;
        let attacker = unit(&mut g, actor, 71, None);
        for _ in 0..16 {
            unit(&mut g, 1 - actor, 88, Some(0));
        }
        unit(&mut g, 1 - actor, 71, Some(0));
        g.players[1 - actor].hand = vec![7, 8, 9];
        g.battlefield_owner[0] = Some(1 - actor);
        start_combat(&mut g, attacker);
        let candidates = g.strategic_damage_candidates().unwrap();
        assert!(!candidates.is_empty() && candidates.len() <= 24);
        for action in &candidates {
            g.clone().apply(action.clone()).unwrap();
        }
        let chosen = g.choose_ai_action().unwrap();
        let mut hidden = g.clone();
        hidden.players[1 - actor].hand = vec![53, 6, 6];
        hidden.players[1 - actor].deck.reverse();
        hidden.rng = Rng::new(47);
        assert_eq!(chosen, hidden.choose_ai_action().unwrap());
        g.apply(chosen.clone()).unwrap();
        hidden.apply(chosen).unwrap();
        for state in [&mut g, &mut hidden] {
            state.strategic_settle(state.turn).unwrap();
        }
        assert_eq!(g.players[1 - actor].units.len(), 15);
        assert_eq!(g.players[1 - actor].units, hidden.players[1 - actor].units);
        assert_eq!(g.battlefield_owner, hidden.battlefield_owner);
    }
}

fn unit(g: &mut Game, owner: usize, card: CardId, lane: Option<usize>) -> Target {
    let unit_id = test_support::ready_unit(g, owner, card);
    let target = Target::Unit {
        player: owner,
        unit_id,
    };
    g.unit_mut(target).unwrap().lane = lane;
    target
}
fn id(target: Target) -> u32 {
    let Target::Unit { unit_id, .. } = target else {
        panic!("unit required")
    };
    unit_id
}

// All legal lethal-first orders for this fixture (one or two opposing units).
// Choose the order removing the most printed Might; do not reuse AI scoring.
fn damage(g: &Game, player: usize, lane: usize, total: u64) -> Action {
    let units: Vec<_> = g.players[1 - player]
        .units
        .iter()
        .filter(|u| u.lane == Some(lane))
        .collect();
    assert!(!units.is_empty() && units.len() <= 2);
    let mut orders = vec![units.clone()];
    if units.len() == 2 {
        orders.push(vec![units[1], units[0]]);
    }
    orders
        .into_iter()
        .filter_map(|order| {
            let mut remaining = total;
            let mut assignments = Vec::new();
            let mut removed_value = 0;
            for (index, unit) in order.iter().enumerate() {
                if remaining == 0 {
                    break;
                }
                let lethal = g
                    .unit_might(unit)
                    .unwrap()
                    .saturating_sub(unit.damage)
                    .max(1);
                let amount = if index + 1 == order.len() {
                    remaining
                } else {
                    remaining.min(lethal)
                };
                assignments.push((unit.id, amount));
                remaining -= amount;
                if amount >= lethal {
                    removed_value += u64::from(g.catalog[unit.card_id].might) + 1;
                }
            }
            let action = Action::AssignDamage { assignments };
            let mut copy = g.clone();
            copy.apply(action.clone())
                .ok()
                .map(|_| (removed_value, action))
        })
        .max_by_key(|(value, _)| *value)
        .expect("legal combat order")
        .1
}

fn finish(
    mut g: Game,
    action: Action,
    actor: usize,
    small: Target,
    large: Target,
    attacker: Target,
) -> (bool, bool, bool, Option<usize>, bool) {
    g.apply(action).unwrap();
    for _ in 0..50 {
        if !g.in_window() && g.decision.is_none() {
            break;
        }
        let action = if let Some(action) = test_support::decline_legend_action(&g) {
            action
        } else if let Some(Decision::CombatDamage {
            player,
            lane,
            total,
        }) = g.decision
        {
            damage(&g, player, lane, total)
        } else {
            assert!(g.decision.is_none(), "{:?}", g.decision);
            Action::Pass
        };
        g.apply(action).unwrap();
    }
    assert!(!g.in_window() && g.decision.is_none());
    (
        g.unit_ref(small).is_some(),
        g.unit_ref(large).is_some(),
        g.unit_ref(attacker).is_some(),
        g.battlefield_owner[0],
        g.players[actor].hand.contains(&461),
    )
}

#[test]
fn block_tank_benchmark_checks_useful_casts_and_target_tradeoffs() {
    let mut correct = [0; 3];
    let mut total = [0; 3];
    for actor in 0..2 {
        for (level, difficulty) in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ]
        .into_iter()
        .enumerate()
        {
            for attack in 1..=13 {
                let mut g = test_support::position(1 - actor, 0, 461700);
                g.difficulty = difficulty;
                let small = unit(&mut g, actor, 110, Some(0)); // 2 printed Might, Shield 1 -> defender 3.
                let large = unit(&mut g, actor, 71, Some(0)); // 5 printed Might, no other ability.
                let attacker = unit(&mut g, 1 - actor, 71, None);
                g.unit_mut(attacker).unwrap().temporary_buff = attack - 5;
                g.players[actor].hand = vec![461];
                // Hand size is public; change only unknown identities below.
                g.players[1 - actor].hand = vec![7, 7, 7];
                g.battlefield_owner[0] = Some(actor);
                for p in &mut g.players {
                    p.energy_pool = 2;
                    p.energy = 2;
                    p.power_pool = [0; 6];
                }
                g.apply(Action::Move {
                    unit_id: id(attacker),
                    lane: Some(0),
                })
                .unwrap();
                g.apply(Action::Pass).unwrap();
                assert_eq!(g.priority(), actor);
                let cast = |target| Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![target],
                };
                let pass = finish(g.clone(), Action::Pass, actor, small, large, attacker);
                let on_small = finish(g.clone(), cast(small), actor, small, large, attacker);
                let on_large = finish(g.clone(), cast(large), actor, small, large, attacker);
                // Fixed mathematical oracle: opponent kills as much printed Might
                // as possible, Tank forces that unit first, and damage is simultaneous.
                let expected_pass = match attack {
                    1..=2 => (true, true, false, Some(actor), true),
                    3..=4 => (false, true, false, Some(actor), true),
                    5..=7 => (true, false, false, Some(actor), true),
                    8 => (false, false, false, None, true),
                    _ => (false, false, true, Some(1 - actor), true),
                };
                let expected_small = match attack {
                    1..=5 => (true, true, false, Some(actor), false),
                    6..=10 => (false, true, false, Some(actor), false),
                    11 => (false, false, false, None, false),
                    _ => (false, false, true, Some(1 - actor), false),
                };
                let expected_large = match attack {
                    1..=7 => (true, true, false, Some(actor), false),
                    8..=10 => (true, false, false, Some(actor), false),
                    11 => (false, false, false, None, false),
                    _ => (false, false, true, Some(1 - actor), false),
                };
                assert_eq!(pass, expected_pass, "pass, attack {attack}");
                assert_eq!(on_small, expected_small, "small, attack {attack}");
                assert_eq!(on_large, expected_large, "large, attack {attack}");
                let action = g.choose_ai_action().unwrap();
                let actual = finish(g.clone(), action.clone(), actor, small, large, attacker);
                let expected = match attack {
                    1..=2 | 12..=13 => expected_pass,
                    6..=7 => expected_large,
                    _ => expected_small,
                };
                let good = actual == expected;
                correct[level] += usize::from(good);
                total[level] += 1;
                let mut hidden = g.clone();
                hidden.players[1 - actor].hand = vec![6, 8, 9];
                hidden.players[1 - actor].deck.reverse();
                assert_eq!(
                    action,
                    hidden.choose_ai_action().unwrap(),
                    "private cards changed decision"
                );
                println!(
                    "TANK_BENCHMARK {}",
                    serde_json::json!({"seat":actor,"difficulty":format!("{difficulty:?}"),"attack":attack,"action":crate::api::action_to_json(&action),"correct":good,"pass":pass,"on_small":on_small,"on_large":on_large,"actual":actual})
                );
            }
        }
    }
    println!(
        "TANK_SUMMARY {}",
        serde_json::json!({"correct":correct,"total":total})
    );
    assert_eq!(
        correct[2], total[2],
        "Strategic missed a proven Block trade"
    );
}
