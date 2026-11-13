use super::*;
const ALPHA: CardId = 443;

fn position(actor: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 443443, Difficulty::Tactical);
    g.choose_starting_player(actor).unwrap();
    g.start_match().unwrap();
    g.battlefields = vec![9, 9];
    g.battlefield_owner = vec![None; 2];
    for p in &mut g.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.xp = 0;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy = 100;
        p.energy_pool = 100;
        p.power_pool = [20; 6];
    }
    for legend in &mut g.legend {
        legend.ready = false;
    }
    g
}
fn unit(g: &mut Game, player: usize, might: i64, lane: Option<usize>) -> Target {
    let id = g.create_unit(player, 69, false, 0);
    let u = g.players[player].units.last_mut().unwrap();
    u.lane = lane;
    u.ready = true;
    u.temporary_buff = might - 8;
    Target::Unit {
        player,
        unit_id: id,
    }
}
fn id(target: Target) -> u32 {
    let Target::Unit { unit_id, .. } = target else {
        panic!()
    };
    unit_id
}
fn cast(g: &mut Game, card: CardId, targets: Vec<Target>) {
    let player = g.priority();
    let hand_index = g.players[player].hand.len();
    g.players[player].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
}
fn pair(g: &mut Game) {
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn settle(g: &mut Game) {
    for _ in 0..100 {
        let action = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            None if !g.chain.is_empty() => Action::Pass,
            None => return,
            other => panic!("Unexpected choice {other:?}"),
        };
        g.apply(action).unwrap();
    }
    panic!("Chain failed to settle");
}
fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action).is_err());
    assert_eq!(format!("{g:?}"), before);
}

/// Controlled Repeat grants exercise one real paid parent and its executions.
/// This does not claim that the fixture is a registered tournament deck.
fn cast_repeated(g: &mut Game, executions: Vec<Vec<Target>>) {
    use crate::play::{ExecutionChoices, PlayChoices, RepeatSelection};
    let actor = g.priority();
    g.players[actor].hand = vec![ALPHA];
    for index in 1..executions.len() {
        g.play_grants.push(PlayGrant {
            id: 44390 + index as u32,
            player: actor,
            source: ChainSource::Card { card_id: 443 },
            expires_turn: g.turn,
            kind: PlayGrantKind::RepeatNextSpell,
        });
    }
    let action = Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: executions[0].clone(),
    };
    let options = g.play_choices(&action).unwrap().repeats;
    assert_eq!(options.len(), executions.len() - 1);
    g.apply(Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats: options
                .iter()
                .zip(executions.into_iter().skip(1))
                .map(|(option, targets)| RepeatSelection {
                    option_id: option.id,
                    choices: ExecutionChoices {
                        targets,
                        ..Default::default()
                    },
                })
                .collect(),
            ..Default::default()
        },
    })
    .unwrap();
    assert_eq!(
        g.chain
            .iter()
            .filter(|c| c.card_id == ALPHA && c.ability.is_none())
            .count(),
        1
    );
}

fn finalize_xp_rewards(g: &mut Game, count: usize) {
    while let Some(Decision::OrderTriggers { options, .. }) = g.decision.clone() {
        g.apply(Action::OrderTrigger {
            trigger_id: options[0].id,
        })
        .unwrap();
    }
    assert!(g.decision.is_none());
    assert_eq!(
        g.chain
            .iter()
            .filter(|c| matches!(
                c.ability,
                Some(ChainAbility::Triggered(TriggeredAbility::Xp(1)))
            ))
            .count(),
        count
    );
}

#[test]
fn alpha_repeat_disjoint_kills_belong_to_one_parent_observed_by_each_clause() {
    for actor in 0..2 {
        for executions in [2, 3] {
            let mut g = position(actor);
            let source = unit(&mut g, actor, 1, None);
            let victims: Vec<_> = (0..executions)
                .map(|i| unit(&mut g, 1 - actor, 1, Some(i % 2)))
                .collect();
            cast_repeated(
                &mut g,
                victims.iter().map(|&target| vec![source, target]).collect(),
            );
            pair(&mut g);
            assert!(victims.iter().all(|&target| g.unit_ref(target).is_none()));
            assert_eq!(
                g.players[actor].xp, 0,
                "Each XP remains a separate response-window ability"
            );
            assert_eq!(
                g.players[actor]
                    .discard
                    .iter()
                    .filter(|&&id| id == ALPHA)
                    .count(),
                1
            );
            finalize_xp_rewards(&mut g, executions * executions);
            settle(&mut g);
            assert_eq!(g.players[actor].xp, (executions * executions) as u64);
            assert!(g.alpha_kill_watches.is_empty());
        }
    }
}

#[test]
fn alpha_repeat_combined_damage_credits_one_kill_once_per_clause() {
    for actor in 0..2 {
        let mut g = position(actor);
        let source = unit(&mut g, actor, 3, None);
        let victim = unit(&mut g, 1 - actor, 6, Some(0));
        cast_repeated(&mut g, vec![vec![source, victim], vec![source, victim]]);
        pair(&mut g);
        assert!(g.unit_ref(victim).is_none());
        finalize_xp_rewards(&mut g, 2);
        settle(&mut g);
        assert_eq!(g.players[actor].xp, 2);
    }
}

#[test]
fn alpha_repeat_zero_damage_is_negated_not_an_ignored_reward_clause() {
    for actor in 0..2 {
        for zero_first in [false, true] {
            let mut g = position(actor);
            let shrinking = unit(&mut g, actor, 1, None);
            let source = unit(&mut g, actor, 3, None);
            let victim = unit(&mut g, 1 - actor, 2, Some(0));
            let mut executions = vec![vec![shrinking, victim], vec![source, victim]];
            if !zero_first {
                executions.reverse();
            }
            cast_repeated(&mut g, executions);
            if g.priority() == actor {
                g.apply(Action::Pass).unwrap();
            }
            cast(&mut g, 136, vec![shrinking]); // Actual Frigid Touch response.
            pair(&mut g);
            assert_eq!(g.unit_might(g.unit_ref(shrinking).unwrap()).unwrap(), 0);
            pair(&mut g);
            assert!(g.unit_ref(victim).is_none());
            finalize_xp_rewards(&mut g, 2);
            settle(&mut g);
            assert_eq!(g.players[actor].xp, 2);
        }
    }
}

#[test]
fn alpha_repeat_ignored_source_or_all_recipients_do_not_create_a_reward_clause() {
    for actor in 0..2 {
        for return_source in [false, true] {
            for ignored_first in [false, true] {
                let mut g = position(actor);
                let a = unit(&mut g, actor, 3, None);
                let b = unit(&mut g, actor, 3, None);
                let x = unit(&mut g, 1 - actor, 2, Some(0));
                let y = unit(&mut g, 1 - actor, 2, Some(1));
                let mut executions = vec![vec![a, x], vec![b, y]];
                if !ignored_first {
                    executions.reverse();
                }
                cast_repeated(&mut g, executions);
                let owner = if return_source { actor } else { 1 - actor };
                if g.priority() != owner {
                    g.apply(Action::Pass).unwrap();
                }
                cast(&mut g, 377, vec![if return_source { a } else { x }]);
                pair(&mut g);
                pair(&mut g);
                assert!(g.unit_ref(y).is_none());
                finalize_xp_rewards(&mut g, 1);
                settle(&mut g);
                assert_eq!(g.players[actor].xp, 1);
            }
        }
    }
}

#[test]
fn alpha_repeat_replaced_death_is_excluded_from_every_parent_reward_clause() {
    for actor in 0..2 {
        for copies in [1, 2] {
            let mut g = position(actor);
            let source = unit(&mut g, actor, 4, None);
            let saved = unit(&mut g, 1 - actor, 2, Some(0));
            let dead = unit(&mut g, 1 - actor, 2, Some(1));
            for offset in 0..copies {
                g.players[1 - actor].gear.push(crate::gear::GearState {
                    id: 44388 + offset,
                    card_id: 10,
                    lane: Some(0),
                    attached_to: Some(id(saved)),
                    ready: true,
                    temporary: false,
                    token: false,
                });
            }
            cast_repeated(&mut g, vec![vec![source, saved], vec![source, dead]]);
            pair(&mut g);
            // A sole mandatory replacement resolves automatically. Multiple gear
            // copies suspend cleanup so their owner can choose which to consume.
            if copies == 2 {
                assert!(matches!(
                    g.decision,
                    Some(Decision::DeathReplacement { .. })
                ));
                g.apply(Action::ChooseDeathReplacement {
                    unit_id: id(saved),
                    gear_id: Some(44388),
                })
                .unwrap();
            }
            assert!(g.unit_ref(dead).is_none());
            assert_eq!(g.unit_ref(saved).unwrap().lane, None);
            assert_eq!(g.unit_ref(saved).unwrap().damage, 0);
            assert_eq!(g.players[1 - actor].gear.len(), (copies - 1) as usize);
            finalize_xp_rewards(&mut g, 2);
            settle(&mut g);
            assert_eq!(g.players[actor].xp, 2);
        }
    }
}

#[test]
fn alpha_waits_for_resolution_and_queues_separate_xp_only_after_actual_deaths() {
    for actor in 0..2 {
        let mut g = position(actor);
        let source = unit(&mut g, actor, 8, None);
        let a = unit(&mut g, 1 - actor, 3, Some(0));
        let b = unit(&mut g, 1 - actor, 5, Some(1));
        cast(&mut g, ALPHA, vec![source, a, b]);
        assert!(g.decision.is_none());
        assert_eq!(g.unit_ref(a).unwrap().damage, 0);
        pair(&mut g);
        assert!(matches!(
            g.decision,
            Some(Decision::SplitDamage {
                total: 8,
                keep: 2,
                ..
            })
        ));
        for assignments in [
            vec![],
            vec![(id(a), 8)],
            vec![(id(a), 0), (id(b), 8)],
            vec![(id(a), 4), (id(b), 3)],
            vec![(id(a), 4), (id(a), 4)],
            vec![(id(source), 3), (id(b), 5)],
        ] {
            reject(&mut g, Action::AssignDamage { assignments });
        }
        reject(&mut g, Action::Pass);
        reject(&mut g, Action::EndTurn);
        g.apply(Action::AssignDamage {
            assignments: vec![(id(a), 3), (id(b), 5)],
        })
        .unwrap();
        assert!(g.unit_ref(a).is_none() && g.unit_ref(b).is_none());
        assert_eq!(g.players[actor].xp, 0, "Rewards remain respondable");
        assert!(g.players[actor].discard.contains(&ALPHA));
        settle(&mut g);
        assert_eq!(g.players[actor].xp, 2);
        assert!(g.alpha_kill_watches.is_empty());
    }
}

#[test]
fn alpha_reduced_might_requires_exactly_the_remaining_number_of_recipients() {
    for actor in 0..2 {
        let mut g = position(actor);
        let source = unit(&mut g, actor, 5, None);
        let targets: Vec<_> = (0..4)
            .map(|index| {
                unit(
                    &mut g,
                    1 - actor,
                    if index == 0 { 3 } else { 1 },
                    Some(index % 2),
                )
            })
            .collect();
        cast(
            &mut g,
            ALPHA,
            std::iter::once(source)
                .chain(targets.iter().copied())
                .collect(),
        );
        g.apply(Action::Pass).unwrap();
        cast(&mut g, 136, vec![source]); // Frigid Touch: current Might becomes3.
        pair(&mut g);
        pair(&mut g);
        assert!(matches!(
            g.decision,
            Some(Decision::SplitDamage {
                total: 3,
                keep: 3,
                ..
            })
        ));
        reject(
            &mut g,
            Action::AssignDamage {
                assignments: vec![(id(targets[0]), 3)],
            },
        );
        g.apply(Action::AssignDamage {
            assignments: targets[1..].iter().map(|&target| (id(target), 1)).collect(),
        })
        .unwrap();
        assert_eq!(g.unit_ref(targets[0]).unwrap().damage, 0);
        settle(&mut g);
        assert_eq!(g.players[actor].xp, 3);
    }
}

#[test]
fn alpha_larger_might_is_divided_only_among_original_targets_and_no_combat_order_applies() {
    let mut g = position(0);
    let source = unit(&mut g, 0, 3, None);
    let a = unit(&mut g, 1, 9, Some(0));
    let b = unit(&mut g, 1, 1, Some(1));
    let unchosen = unit(&mut g, 1, 1, Some(1));
    cast(&mut g, ALPHA, vec![source, a, b]);
    cast(&mut g, 135, vec![source]); // Feral Strength: +2M.
    pair(&mut g);
    pair(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::SplitDamage {
            total: 5,
            keep: 2,
            ..
        })
    ));
    reject(
        &mut g,
        Action::AssignDamage {
            assignments: vec![(id(a), 1), (id(unchosen), 4)],
        },
    );
    g.apply(Action::AssignDamage {
        assignments: vec![(id(a), 1), (id(b), 4)],
    })
    .unwrap();
    assert_eq!(g.unit_ref(a).unwrap().damage, 1);
    assert!(g.unit_ref(b).is_none());
    assert_eq!(g.unit_ref(unchosen).unwrap().damage, 0);
    settle(&mut g);
    assert_eq!(g.players[0].xp, 1);
}

#[test]
fn alpha_unit_damage_ignores_spell_prevention_and_spell_bonus_damage() {
    let mut g = position(0);
    let source = unit(&mut g, 0, 3, None);
    let enemy = unit(&mut g, 1, 4, Some(0));
    g.battlefields[0] = 3; // Void Gate's spell/ability bonus must not apply.
    cast(&mut g, 441, vec![]);
    settle(&mut g);
    assert!(g.spell_ability_damage_prevented);
    cast(&mut g, ALPHA, vec![source, enemy]);
    pair(&mut g);
    assert!(g.decision.is_none()); // One recipient needs no allocation prompt.
    assert_eq!(g.unit_ref(enemy).unwrap().damage, 3);
    assert_eq!(g.players[0].xp, 0);
}

#[test]
fn alpha_drops_moved_targets_but_a_moved_source_still_uses_its_current_might() {
    let mut g = position(0);
    let source = unit(&mut g, 0, 3, Some(0));
    let a = unit(&mut g, 1, 5, Some(0));
    let b = unit(&mut g, 1, 5, Some(1));
    cast(&mut g, ALPHA, vec![source, a, b]);
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 9, vec![b]);
    pair(&mut g);
    if g.priority() != 0 {
        g.apply(Action::Pass).unwrap();
    }
    cast(&mut g, 9, vec![source]);
    pair(&mut g);
    pair(&mut g);
    assert_eq!(g.unit_ref(source).unwrap().lane, None);
    assert_eq!(g.unit_ref(b).unwrap().lane, None);
    assert_eq!(g.unit_ref(a).unwrap().damage, 3);
    assert_eq!(g.unit_ref(b).unwrap().damage, 0);
}

#[test]
fn alpha_replaced_death_waits_for_choice_and_never_awards_xp_for_the_survivor() {
    let mut g = position(0);
    let source = unit(&mut g, 0, 6, None);
    let guarded = unit(&mut g, 1, 1, Some(0));
    let killed = unit(&mut g, 1, 3, Some(1));
    for gear_id in [99001, 99002] {
        g.players[1].gear.push(GearState {
            id: gear_id,
            card_id: 10,
            attached_to: Some(id(guarded)),
            lane: None,
            ready: true,
            temporary: false,
            token: false,
        });
    }
    cast(&mut g, ALPHA, vec![source, guarded, killed]);
    pair(&mut g);
    g.apply(Action::AssignDamage {
        assignments: vec![(id(guarded), 3), (id(killed), 3)],
    })
    .unwrap();
    assert!(matches!(
        g.decision,
        Some(Decision::DeathReplacement { .. })
    ));
    assert_eq!(g.players[0].xp, 0);
    g.apply(Action::ChooseDeathReplacement {
        unit_id: id(guarded),
        gear_id: Some(99001),
    })
    .unwrap();
    assert!(g.unit_ref(killed).is_none());
    assert_eq!(g.unit_ref(guarded).unwrap().lane, None);
    assert_eq!(g.unit_ref(guarded).unwrap().damage, 0);
    settle(&mut g);
    assert_eq!(g.players[0].xp, 1);
}

#[test]
fn alpha_banished_instead_of_dying_does_not_award_xp() {
    let mut g = position(0);
    let source = unit(&mut g, 0, 3, None);
    let enemy = unit(&mut g, 1, 2, Some(0));
    g.unit_mut(enemy).unwrap().banish_on_death = true;
    cast(&mut g, ALPHA, vec![source, enemy]);
    pair(&mut g);
    settle(&mut g);
    assert!(g.unit_ref(enemy).is_none());
    assert!(g.players[1].banish.contains(&69));
    assert_eq!(g.players[0].xp, 0);
}

#[test]
fn alpha_source_and_recipient_departures_break_physical_identity() {
    for actor in 0..2 {
        for remove_source in [false, true] {
            let mut g = position(actor);
            let source = unit(&mut g, actor, 5, None);
            let a = unit(&mut g, 1 - actor, 9, Some(0));
            let b = unit(&mut g, 1 - actor, 9, Some(1));
            cast(&mut g, ALPHA, vec![source, a, b]);
            let returning = if remove_source { source } else { b };
            let owner = if remove_source { actor } else { 1 - actor };
            if g.priority() != owner {
                g.apply(Action::Pass).unwrap();
            }
            cast(&mut g, 377, vec![returning]); // Actual Retreat response.
            pair(&mut g);
            assert!(g.unit_ref(returning).is_none());
            assert!(g.players[owner].hand.contains(&69));
            // A controlled re-entry fixture of the same printed card has a new
            // physical identity; this is not a claim of a legal Mega-Mech response.
            g.players[owner].hand.retain(|&card| card != 69);
            let replacement = unit(&mut g, owner, 9, Some(1));
            assert_ne!(id(returning), id(replacement));
            pair(&mut g);
            assert_eq!(g.unit_ref(replacement).unwrap().damage, 0);
            assert_eq!(
                g.unit_ref(a).unwrap().damage,
                if remove_source { 0 } else { 5 }
            );
            assert!(g.decision.is_none());
            assert_eq!(g.players[actor].xp, 0);
            assert!(g.alpha_kill_watches.is_empty());
        }
    }
}

#[test]
fn alpha_zero_might_at_resolution_has_no_allocation_or_deal() {
    for actor in 0..2 {
        let mut g = position(actor);
        let source = unit(&mut g, actor, 1, None);
        let enemy = unit(&mut g, 1 - actor, 5, Some(0));
        cast(&mut g, ALPHA, vec![source, enemy]);
        g.apply(Action::Pass).unwrap();
        cast(&mut g, 136, vec![source]);
        pair(&mut g);
        assert_eq!(g.unit_might(g.unit_ref(source).unwrap()).unwrap(), 0);
        pair(&mut g);
        assert!(g.decision.is_none());
        assert_eq!(g.unit_ref(enemy).unwrap().damage, 0);
        assert_eq!(g.players[actor].xp, 0);
        assert!(g.alpha_kill_watches.is_empty());
    }
}

#[test]
fn alpha_single_target_can_be_repulsed_but_multiple_live_links_cannot() {
    for actor in 0..2 {
        for multiple in [false, true] {
            let mut g = position(actor);
            let source = unit(&mut g, actor, 5, None);
            let a = unit(&mut g, 1 - actor, 3, Some(0));
            let b = unit(&mut g, 1 - actor, 3, Some(1));
            let mut targets = vec![source, a];
            if multiple {
                targets.push(b);
            }
            cast(&mut g, ALPHA, targets);
            let alpha = Target::Spell {
                chain_id: g.chain.last().unwrap().id,
            };
            g.apply(Action::Pass).unwrap();
            g.players[1 - actor].hand = vec![442];
            let counter = Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![a, alpha],
            };
            if multiple {
                reject(&mut g, counter.clone());
                // Moving the second target to base invalidates its damage, but
                // preserves the choosing relationship used by Repulse.
                cast(&mut g, 9, vec![b]);
                pair(&mut g);
                if g.priority() != 1 - actor {
                    g.apply(Action::Pass).unwrap();
                }
                reject(&mut g, counter);
                // Wind Wall can counter the spell without a chosen-unit limit.
                cast(&mut g, 86, vec![alpha]);
            } else {
                g.apply(counter).unwrap();
            }
            pair(&mut g);
            settle(&mut g);
            assert!(g.players[actor].discard.contains(&ALPHA));
            assert_eq!(g.unit_ref(a).unwrap().damage, 0);
            assert_eq!(g.unit_ref(b).unwrap().damage, 0);
            assert_eq!(g.players[actor].xp, 0);
            assert!(g.alpha_kill_watches.is_empty());
        }
    }
}

#[test]
fn alpha_rewards_token_deaths_but_not_later_spells_kills() {
    for token in [false, true] {
        let mut g = position(0);
        let source = unit(&mut g, 0, 3, None);
        let enemy = unit(&mut g, 1, if token { 2 } else { 4 }, Some(0));
        if token {
            let u = g.unit_mut(enemy).unwrap();
            u.token = true;
            u.buff = 2;
            u.temporary_buff = 0;
        }
        cast(&mut g, ALPHA, vec![source, enemy]);
        pair(&mut g);
        settle(&mut g);
        assert_eq!(g.players[0].xp, if token { 1 } else { 0 });
        assert!(g.alpha_kill_watches.is_empty());
        if token {
            assert!(g.unit_ref(enemy).is_none());
            assert!(!g.players[1].discard.contains(&69));
        } else {
            assert_eq!(g.unit_ref(enemy).unwrap().damage, 3);
            cast(&mut g, 53, vec![enemy, enemy]);
            settle(&mut g);
            assert!(g.unit_ref(enemy).is_none());
            assert_eq!(g.players[0].xp, 0, "A later spell owns its own kills");
        }
    }
}

#[test]
fn alpha_repeat_keeps_independent_splits_and_does_not_offer_responses_mid_resolution() {
    use crate::play::{ExecutionChoices, PlayChoices, RepeatSelection};
    for actor in 0..2 {
        let mut g = position(actor);
        let source = unit(&mut g, actor, 3, None);
        let other_source = unit(&mut g, actor, 4, None);
        let a = unit(&mut g, 1 - actor, 9, Some(0));
        let b = unit(&mut g, 1 - actor, 9, Some(1));
        let c = unit(&mut g, 1 - actor, 9, Some(1));
        g.players[actor].hand = vec![ALPHA];
        g.play_grants.push(PlayGrant {
            id: 44399,
            player: actor,
            source: ChainSource::Card { card_id: 443 },
            expires_turn: g.turn,
            kind: PlayGrantKind::RepeatNextSpell,
        });
        let action = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![source, a, b],
        };
        let option_id = g.play_choices(&action).unwrap().repeats[0].id;
        g.apply(Action::WithPlayChoices {
            action: Box::new(action),
            choices: PlayChoices {
                repeats: vec![RepeatSelection {
                    option_id,
                    choices: ExecutionChoices {
                        targets: vec![other_source, b, c],
                        ..Default::default()
                    },
                }],
                ..Default::default()
            },
        })
        .unwrap();
        pair(&mut g);
        assert!(matches!(
            g.decision,
            Some(Decision::SplitDamage { total: 3, .. })
        ));
        g.apply(Action::AssignDamage {
            assignments: vec![(id(a), 1), (id(b), 2)],
        })
        .unwrap();
        assert!(matches!(
            g.decision,
            Some(Decision::SplitDamage { total: 4, .. })
        ));
        assert_eq!(g.unit_ref(a).unwrap().damage, 1);
        assert!(!g.players[actor].discard.contains(&ALPHA));
        reject(&mut g, Action::Pass);
        reject(
            &mut g,
            Action::AssignDamage {
                assignments: vec![(id(a), 1), (id(c), 3)],
            },
        );
        g.apply(Action::AssignDamage {
            assignments: vec![(id(b), 1), (id(c), 3)],
        })
        .unwrap();
        assert_eq!(g.unit_ref(b).unwrap().damage, 3);
        assert_eq!(g.unit_ref(c).unwrap().damage, 3);
        assert!(g.players[actor].discard.contains(&ALPHA));
        assert_eq!(g.players[actor].xp, 0);
        assert!(g.alpha_kill_watches.is_empty());
    }
}

#[test]
fn alpha_initial_bounds_immunity_and_each_hybrid_payment_are_enforced() {
    for actor in 0..2 {
        for domain in [Domain::Calm, Domain::Body] {
            let mut g = position(actor);
            let source = unit(&mut g, actor, 2, None);
            let a = unit(&mut g, 1 - actor, 5, Some(0));
            let b = unit(&mut g, 1 - actor, 5, Some(1));
            let c = unit(&mut g, 1 - actor, 5, Some(0));
            let base = unit(&mut g, 1 - actor, 5, None);
            let immune = unit(&mut g, 1 - actor, 5, Some(1));
            g.unit_mut(immune).unwrap().card_id = 31; // Ruin Runner.
            for target in [a, b] {
                let u = g.unit_mut(target).unwrap();
                u.card_id = 0; // Irelia, Fervent: Deflect1.
                u.temporary_buff = 0;
            }
            g.players[actor].hand = vec![ALPHA];
            g.players[actor].any_power = 0;
            g.players[actor].power_pool = [0; 6];
            g.players[actor].power_pool[domain_index(domain)] = 1;
            for targets in [
                vec![],
                vec![source],
                vec![a, source],
                vec![source, a, a],
                vec![source, a, b, c],
                vec![source, base],
                vec![source, immune],
            ] {
                reject(
                    &mut g,
                    Action::PlayCardWithTargets {
                        hand_index: 0,
                        lane: None,
                        targets,
                    },
                );
            }
            let view = g
                .play_target_view(
                    &Action::PlayCardWithTargets {
                        hand_index: 0,
                        lane: None,
                        targets: vec![source],
                    },
                    0,
                )
                .unwrap();
            assert_eq!(view.spec.maximum, 3);
            assert_eq!(view.spec.required, 2);
            assert!(!view.candidates.contains(&base));
            assert!(!view.candidates.contains(&immune));
            let action = Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![source, a, b],
            };
            reject(&mut g, action.clone()); // Two Deflect payments are still missing.
            g.players[actor].any_power = 2;
            g.apply(action).unwrap();
            assert_eq!(
                g.players[actor].energy, 97,
                "Deflect adds Any Power, not Energy"
            );
            assert_eq!(g.players[actor].power_pool[domain_index(domain)], 0);
            assert_eq!(g.players[actor].any_power, 0);
        }
    }
}

#[test]
fn alpha_split_ai_suggestions_are_legal_at_each_level_and_ignore_hidden_hands() {
    for actor in 0..2 {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let mut g = position(actor);
            g.difficulty = difficulty;
            let source = unit(&mut g, actor, 5, None);
            let a = unit(&mut g, 1 - actor, 2, Some(0));
            let b = unit(&mut g, 1 - actor, 3, Some(1));
            cast(&mut g, ALPHA, vec![source, a, b]);
            pair(&mut g);
            let suggested = g.choose_ai_action().unwrap();
            assert!(matches!(suggested, Action::AssignDamage { .. }));
            let mut hidden_variant = g.clone();
            hidden_variant.players[1 - actor].hand = vec![86, 442, 441, 135];
            hidden_variant.players[1 - actor].deck = vec![136; 20];
            assert_eq!(suggested, hidden_variant.choose_ai_action().unwrap());
            g.apply(suggested).unwrap();
            settle(&mut g);
            assert_eq!(g.players[actor].xp, 2);
        }
    }
}

#[test]
fn alpha_requires_initial_damage_and_a_legal_enemy_recipient() {
    for might in [-1, 0, 1] {
        let mut g = position(0);
        let source = unit(&mut g, 0, might, None);
        let enemy = unit(&mut g, 1, 5, None);
        g.players[0].hand = vec![ALPHA];
        let action = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![source, enemy],
        };
        reject(&mut g, action.clone()); // Enemy in base is never eligible.
        g.unit_mut(enemy).unwrap().lane = Some(0);
        if might <= 0 {
            reject(&mut g, action);
            assert!(g.alpha_candidates(0, &[]).unwrap().is_empty());
        } else {
            g.apply(action).unwrap();
        }
    }
}

#[test]
fn alpha_does_not_claim_an_undamaged_units_death_after_tarics_shield_disappears() {
    let mut g = position(0);
    let source = unit(&mut g, 0, 5, Some(0));
    let taric = unit(&mut g, 1, 4, Some(0));
    let neighbor = unit(&mut g, 1, 3, Some(0));
    let t = g.unit_mut(taric).unwrap();
    t.card_id = 307;
    t.temporary_buff = 0;
    g.showdown = Some(Showdown {
        lane: 0,
        attacker: 0,
        focus: 0,
        passes: 0,
        combat: true,
    });
    g.battlefield_owner[0] = Some(1);
    g.unit_mut(neighbor).unwrap().damage = 3;
    assert_eq!(g.unit_might(g.unit_ref(taric).unwrap()).unwrap(), 5);
    assert_eq!(g.unit_might(g.unit_ref(neighbor).unwrap()).unwrap(), 4);
    cast(&mut g, ALPHA, vec![source, taric]);
    pair(&mut g);
    assert!(g.unit_ref(taric).is_none());
    assert!(g.unit_ref(neighbor).is_none());
    settle(&mut g);
    assert_eq!(
        g.players[0].xp, 1,
        "Only Taric received damage from Alpha Strike"
    );
}

#[test]
fn alpha_repeat_tracks_damaged_victims_through_cleanup_cascades_and_replacements() {
    for actor in 0..2 {
        for save_taric in [false, true] {
            for taric_first in [false, true] {
                let mut g = position(actor);
                let source = unit(&mut g, actor, 6, None);
                let small_source = unit(&mut g, actor, 1, None);
                let taric = unit(&mut g, 1 - actor, 4, Some(0));
                let neighbor = unit(&mut g, 1 - actor, 3, Some(0));
                let untouched = unit(&mut g, 1 - actor, 3, Some(0));
                let t = g.unit_mut(taric).unwrap();
                t.card_id = 307;
                t.temporary_buff = 0;
                g.unit_mut(neighbor).unwrap().damage = 2;
                g.unit_mut(untouched).unwrap().damage = 3;
                g.showdown = Some(Showdown {
                    lane: 0,
                    attacker: actor,
                    focus: actor,
                    passes: 0,
                    combat: true,
                });
                g.battlefield_owner[0] = Some(1 - actor);
                if save_taric {
                    g.players[1 - actor].gear.push(GearState {
                        id: 44387,
                        card_id: 10,
                        lane: Some(0),
                        attached_to: Some(id(taric)),
                        ready: true,
                        temporary: false,
                        token: false,
                    });
                }
                assert_eq!(g.unit_might(g.unit_ref(neighbor).unwrap()).unwrap(), 4);
                let mut executions = vec![vec![source, taric], vec![small_source, neighbor]];
                if !taric_first {
                    executions.reverse();
                }
                cast_repeated(&mut g, executions);
                pair(&mut g);
                assert!(g.unit_ref(neighbor).is_none());
                assert!(g.unit_ref(untouched).is_none());
                if save_taric {
                    let survivor = g.unit_ref(taric).unwrap();
                    assert_eq!((survivor.lane, survivor.damage), (None, 0));
                } else {
                    assert!(g.unit_ref(taric).is_none());
                }
                let expected = if save_taric { 2 } else { 4 };
                finalize_xp_rewards(&mut g, expected);
                settle(&mut g);
                assert_eq!(
                    g.players[actor].xp, expected as u64,
                    "Only damaged actual deaths count; the undamaged cascade victim never does"
                );
                assert!(g.alpha_kill_watches.is_empty());
            }
        }
    }
}

#[test]
fn alpha_choosing_irelia_uses_her_resolved_choose_bonus_even_when_stunned() {
    let mut g = position(0);
    let source = unit(&mut g, 0, 4, None);
    let u = g.unit_mut(source).unwrap();
    u.card_id = 0;
    u.temporary_buff = 0;
    u.stunned = true;
    let a = unit(&mut g, 1, 9, Some(0));
    let b = unit(&mut g, 1, 9, Some(1));
    cast(&mut g, ALPHA, vec![source, a, b]);
    assert!(
        g.chain.last().unwrap().ability.is_some(),
        "Irelia's choosing trigger is above Alpha"
    );
    pair(&mut g);
    assert_eq!(g.unit_might(g.unit_ref(source).unwrap()).unwrap(), 5);
    pair(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::SplitDamage { total: 5, .. })
    ));
    g.apply(Action::AssignDamage {
        assignments: vec![(id(a), 2), (id(b), 3)],
    })
    .unwrap();
    assert_eq!(g.unit_ref(a).unwrap().damage, 2);
    assert_eq!(g.unit_ref(b).unwrap().damage, 3);
}

#[test]
fn alpha_automatic_targets_preserve_a_kill_instead_of_spreading_damage_too_thinly() {
    for actor in 0..2 {
        let mut g = position(actor);
        let source = unit(&mut g, actor, 4, None);
        let a = unit(&mut g, 1 - actor, 4, Some(0));
        let b = unit(&mut g, 1 - actor, 4, Some(1));
        let c = unit(&mut g, 1 - actor, 4, Some(1));
        // This is an open-state position: the uncontested enemy fields already
        // have their controller. Leaving them unclaimed invents new conquest
        // points during the rollout and distorts the comparison with EndTurn.
        g.battlefield_owner = vec![Some(1 - actor); 2];
        assert_eq!(
            g.prepare_targets(actor, ALPHA, None, &[]).unwrap(),
            vec![source, a]
        );
        // Explicitly spreading 4 damage over all three is legal, even though it
        // cannot kill any of them. The recommendation is not a rules restriction.
        assert_eq!(
            g.prepare_targets_explicit(actor, ALPHA, None, &[source, a, b, c])
                .unwrap(),
            vec![source, a, b, c]
        );
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let mut trial = g.clone();
            trial.difficulty = difficulty;
            trial.players[actor].hand = vec![ALPHA];
            let action = trial.choose_ai_action().unwrap();
            trial.apply(action.clone()).unwrap();
            assert!(
                trial
                    .chain
                    .iter()
                    .any(|item| item.card_id == ALPHA && item.ability.is_none()),
                "{difficulty:?}: {action:?}"
            );
            pair(&mut trial);
            settle(&mut trial);
            assert_eq!(trial.players[1 - actor].units.len(), 2, "{difficulty:?}");
            assert_eq!(trial.players[actor].xp, 1);
        }
        // The same budget should still sweep genuinely cheap recipients.
        for enemy in [a, b, c] {
            g.unit_mut(enemy).unwrap().temporary_buff = -7;
        }
        assert_eq!(
            g.prepare_targets(actor, ALPHA, None, &[]).unwrap(),
            vec![source, a, b, c]
        );
    }
}
