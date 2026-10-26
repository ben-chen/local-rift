//! Actual public actions for the C8 spell, trigger and continuous-rule families.
use local_rift::activated::AbilitySource;
use local_rift::empower::BoardObject;
use local_rift::game::play::{ExecutionChoices, PlayChoices, RepeatSelection};
use local_rift::game::{Action, Decision, Difficulty, Game, Showdown, Target, Unit};
use local_rift::gear::GearState;
use local_rift::rules::{self, PlayTargetKind};

fn game() -> Game {
    let mut g = Game::new_for_agents(3, 3, 87331, Difficulty::Tactical);
    g.start_match().unwrap();
    g.battlefields = [2, 2]; // Minefield has no passive damage modifier.
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
        state.energy_pool = 60;
        state.energy = 60;
        state.power_pool = [20; 6];
        state.any_power = 0;
    }
    g.battlefield_owner = [None; 2];
    g
}
fn unit(g: &mut Game, p: usize, id: u32, card: usize, lane: Option<usize>) -> Target {
    g.players[p].units.push(Unit {
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
        g.battlefield_owner[lane] = Some(p);
    }
    Target::Unit {
        player: p,
        unit_id: id,
    }
}
fn u(g: &Game, t: Target) -> &Unit {
    let Target::Unit { player, unit_id } = t else {
        panic!()
    };
    g.players[player]
        .units
        .iter()
        .find(|u| u.id == unit_id)
        .unwrap()
}
fn um(g: &mut Game, t: Target) -> &mut Unit {
    let Target::Unit { player, unit_id } = t else {
        panic!()
    };
    g.players[player]
        .units
        .iter_mut()
        .find(|u| u.id == unit_id)
        .unwrap()
}
fn gear(g: &mut Game, p: usize, id: u32, card_id: usize, attached_to: Option<u32>) {
    g.players[p].gear.push(GearState {
        id,
        card_id,
        lane: None,
        attached_to,
        ready: true,
        temporary: false,
        token: false,
    });
}
fn cast(g: &mut Game, card: usize, targets: Vec<Target>) {
    let p = g.priority();
    let hand_index = g.players[p].hand.len();
    g.players[p].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
}
fn resolve(g: &mut Game) {
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn drain(g: &mut Game) {
    for _ in 0..100 {
        let action = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::PredictCards {
                choice_id, cards, ..
            }) => Action::ResolvePredict {
                choice_id: *choice_id,
                recycle: vec![],
                top_order: (0..cards.len()).collect(),
            },
            Some(Decision::TokenPlacement { .. }) => Action::PlaceToken { lane: None },
            Some(Decision::TokenPlay { .. }) => Action::FinalizeTokenPlay {
                lane: None,
                accelerate: false,
            },
            Some(Decision::BattlefieldOrder { lanes, .. }) => {
                Action::ChooseBattlefield { lane: lanes[0] }
            }
            Some(other) => panic!("unexpected choice {other:?}"),
            None if g.chain.is_empty() => return,
            None => Action::Pass,
        };
        g.apply(action).unwrap();
    }
    panic!("unsettled");
}
fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action).is_err());
    assert_eq!(format!("{g:?}"), before);
}
fn portal(g: &mut Game) {
    let id = 7000 + g.players[0].gear.len() as u32;
    gear(g, 0, id, 256, None);
    g.apply(Action::ActivateAbility {
        source: AbilitySource::Gear {
            player: 0,
            gear_id: id,
        },
        ability_index: 0,
        targets: vec![],
    })
    .unwrap();
    resolve(g);
}
fn repeated(g: &mut Game, card: usize, base: Vec<Target>, extra: Vec<Vec<Target>>) {
    for _ in &extra {
        portal(g);
    }
    let hand_index = g.players[0].hand.len();
    g.players[0].hand.push(card);
    let action = Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets: base,
    };
    let view = g.play_choices(&action).unwrap();
    assert_eq!(view.repeats.len(), extra.len());
    let repeats = view
        .repeats
        .iter()
        .zip(extra)
        .map(|(option, targets)| RepeatSelection {
            option_id: option.id,
            choices: ExecutionChoices {
                targets,
                mode: None,
            },
        })
        .collect();
    g.apply(Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats,
            ..Default::default()
        },
    })
    .unwrap();
}
fn combat(g: &mut Game, lane: usize, attacker: usize) {
    g.showdown = Some(Showdown {
        lane,
        attacker,
        focus: attacker,
        passes: 0,
        combat: true,
    });
}

#[test]
fn target_metadata_candidates_and_bad_explicit_fields_are_consistent() {
    let mut g = game();
    assert_eq!(
        rules::play_target_spec(299).kind,
        Some(PlayTargetKind::Battlefield)
    );
    assert_eq!(rules::play_target_spec(297).maximum, 0);
    assert_eq!(
        rules::play_target_spec(12).kind,
        Some(PlayTargetKind::Spell)
    );
    assert_eq!(rules::play_target_spec(41).required, 0);
    assert_eq!(rules::play_target_spec(23).maximum, 0);
    assert_eq!(
        g.card_target_candidates(0, 299, None, &[]).unwrap(),
        vec![
            Target::Battlefield { lane: 0 },
            Target::Battlefield { lane: 1 }
        ]
    );
    assert!(
        g.card_target_candidates(0, 299, None, &[Target::Battlefield { lane: 0 }])
            .unwrap()
            .is_empty()
    );
    assert!(
        g.card_target_candidates(5, 299, None, &[])
            .unwrap()
            .is_empty()
    );
    assert!(
        g.card_target_candidates(0, usize::MAX, None, &[])
            .unwrap()
            .is_empty()
    );
    assert!(
        g.card_target_candidates(0, 299, Some(9), &[])
            .unwrap()
            .is_empty()
    );
    let target = unit(&mut g, 0, 1000, 69, None);
    g.players[0].hand.push(299);
    for targets in [
        vec![],
        vec![target],
        vec![Target::Battlefield { lane: 2 }],
        vec![
            Target::Battlefield { lane: 0 },
            Target::Battlefield { lane: 1 },
        ],
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
    reject(
        &mut g,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: Some(0),
            targets: vec![Target::Battlefield { lane: 0 }],
        },
    );
    cast(&mut g, 299, vec![Target::Battlefield { lane: 1 }]);
    resolve(&mut g);
    assert!(g.players[0].discard.contains(&299));
}
#[test]
fn supernova_draws_with_invalid_target_but_base_movement_alone_is_valid() {
    let mut g = game();
    let target = unit(&mut g, 1, 1000, 301, Some(0));
    cast(&mut g, 291, vec![target]);
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 11, vec![target]); // Gust returns the chosen object to hand.
    resolve(&mut g);
    let before = g.players[0].hand.len();
    resolve(&mut g);
    assert!(g.players[1].units.is_empty());
    assert!(g.players[1].hand.contains(&301));
    assert_eq!(g.players[0].hand.len(), before + 2);
    let mut g = game();
    let target = unit(&mut g, 1, 1000, 69, Some(0));
    um(&mut g, target).temporary_buff = 20;
    cast(&mut g, 291, vec![target]);
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 9, vec![target]);
    resolve(&mut g);
    resolve(&mut g);
    assert_eq!(u(&g, target).lane, None);
    assert_eq!(u(&g, target).damage, 10);
}
#[test]
fn supernova_countered_does_not_draw_and_repeated_draws_keep_parent_out_of_burnout() {
    let mut g = game();
    let target = unit(&mut g, 1, 1000, 69, None);
    cast(&mut g, 291, vec![target]);
    g.apply(Action::Pass).unwrap();
    let chain_id = g.chain[0].id;
    cast(&mut g, 86, vec![Target::Spell { chain_id }]);
    resolve(&mut g);
    assert!(g.players[0].hand.is_empty());
    assert_eq!(u(&g, target).damage, 0);
    let mut g = game();
    let target = unit(&mut g, 1, 1000, 69, None);
    um(&mut g, target).temporary_buff = 30;
    repeated(&mut g, 291, vec![target], vec![vec![target]]);
    g.players[0].deck = vec![70];
    g.players[0].discard = vec![71, 72, 73];
    resolve(&mut g);
    assert_eq!(g.players[0].hand.len(), 4);
    assert!(!g.players[0].hand.contains(&291));
    assert_eq!(g.players[0].discard, vec![291]);
    assert_eq!(u(&g, target).damage, 20);
}
#[test]
fn final_spark_uses_enemy_deflect_and_successful_printed_cost_lux_trigger() {
    let mut g = game();
    g.legend[0] = local_rift::legends::LegendState::from_legend(6).unwrap();
    let target = unit(&mut g, 1, 1000, 92, None);
    um(&mut g, target).temporary_buff = 20;
    g.players[0].hand.push(290);
    let quote = g
        .cost_plan(&Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        })
        .unwrap();
    assert_eq!(quote.power.iter().map(|p| p.amount).sum::<u16>(), 1);
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![target],
    })
    .unwrap();
    resolve(&mut g);
    assert_eq!(u(&g, target).damage, 8);
    assert_eq!(g.chain.len(), 1); // Lux completion trigger.
    assert!(matches!(
        g.chain_source(&g.chain[0]),
        local_rift::game::ChainSource::Legend { legend_id: 6, .. }
    ));
}
#[test]
fn rune_prison_stuns_without_exhausting_or_locking_movement() {
    let mut g = game();
    let target = unit(&mut g, 0, 1000, 69, None);
    cast(&mut g, 292, vec![target]);
    resolve(&mut g);
    assert!(u(&g, target).stunned && u(&g, target).ready);
    g.apply(Action::Move {
        unit_id: 1000,
        lane: Some(0),
    })
    .unwrap();
    assert_eq!(g.lane_might(0, 0).unwrap(), 0);
    assert_eq!(
        g.unit_might(u(&g, target)).unwrap(),
        u64::from(g.catalog[69].might)
    );
}
#[test]
fn cleave_is_temporary_stacking_assault_not_a_base_buff() {
    let mut g = game();
    let target = unit(&mut g, 0, 1000, 301, None);
    cast(&mut g, 293, vec![target]);
    resolve(&mut g);
    assert_eq!(g.unit_keywords(u(&g, target)).unwrap().assault, 4);
    assert_eq!(g.unit_might(u(&g, target)).unwrap(), 2);
    um(&mut g, target).lane = Some(0);
    combat(&mut g, 0, 0);
    assert_eq!(g.unit_might(u(&g, target)).unwrap(), 6);
    g.showdown.as_mut().unwrap().attacker = 1;
    assert_eq!(g.unit_might(u(&g, target)).unwrap(), 2);
    g.showdown = None;
    um(&mut g, target).lane = None;
    g.apply(Action::EndTurn).unwrap();
    drain(&mut g);
    assert_eq!(g.unit_keywords(u(&g, target)).unwrap().assault, 1);
}
#[test]
fn soul_harvest_rechecks_might_and_battlefield_at_resolution() {
    let mut g = game();
    let target = unit(&mut g, 1, 1000, 301, Some(0));
    cast(&mut g, 295, vec![target]);
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 7, vec![target]);
    resolve(&mut g);
    resolve(&mut g);
    assert_eq!(g.unit_might(u(&g, target)).unwrap(), 4);
    assert!(!g.players[1].discard.contains(&301));
    let mut g = game();
    let target = unit(&mut g, 1, 1000, 301, Some(0));
    cast(&mut g, 296, vec![target]);
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 9, vec![target]);
    resolve(&mut g);
    resolve(&mut g);
    assert_eq!(u(&g, target).lane, None);
    let mut g = game();
    let target = unit(&mut g, 1, 1000, 69, Some(0));
    g.players[0].hand.push(295);
    reject(
        &mut g,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        },
    );
}
#[test]
fn direct_kill_of_zero_might_has_no_damage_and_offers_controller_replacements() {
    let mut g = game();
    let target = unit(&mut g, 1, 1000, 15, None);
    gear(&mut g, 1, 1001, 10, Some(1000));
    gear(&mut g, 1, 1002, 21, None);
    cast(&mut g, 294, vec![target]);
    resolve(&mut g);
    assert_eq!(u(&g, target).damage, 0);
    assert!(matches!(
        g.decision,
        Some(Decision::DeathReplacement { player: 1, .. })
    ));
    assert!(g.resolution_progress().is_some());
    assert!(g.chain.iter().any(|i| i.card_id == 294));
    g.apply(Action::ChooseDeathReplacement {
        unit_id: 1000,
        gear_id: Some(1002),
    })
    .unwrap();
    assert_eq!(u(&g, target).damage, 0);
    assert!(!u(&g, target).ready);
    assert!(g.players[1].discard.contains(&21));
    assert!(g.players[0].discard.contains(&294));
}
#[test]
fn repeated_kills_resume_once_and_recall_only_invalidates_battlefield_restriction() {
    for card in [294, 296] {
        let mut g = game();
        let target = unit(&mut g, 1, 1000, 69, Some(0));
        gear(&mut g, 1, 1001, 10, Some(1000));
        gear(&mut g, 1, 1002, 21, None);
        repeated(&mut g, card, vec![target], vec![vec![target]]);
        resolve(&mut g);
        g.apply(Action::ChooseDeathReplacement {
            unit_id: 1000,
            gear_id: Some(1002),
        })
        .unwrap();
        assert_eq!(
            g.players[1].gear.iter().any(|g| g.card_id == 10),
            card == 296
        );
        assert_eq!(u(&g, target).lane, None);
        assert!(g.players[0].discard.contains(&card));
        assert!(g.resolution_progress().is_none());
    }
}
#[test]
fn firestorm_targets_a_field_not_protected_deflect_recipients_or_irelia() {
    let mut g = game();
    let enemy = unit(&mut g, 1, 1000, 0, Some(0));
    um(&mut g, enemy).protected = true;
    um(&mut g, enemy).temporary_buff = 5;
    let friendly = unit(&mut g, 0, 1001, 271, None);
    g.players[0].hand.push(299);
    let action = Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Battlefield { lane: 0 }],
    };
    let plan = g.cost_plan(&action).unwrap();
    assert_eq!(plan.energy, 6);
    assert_eq!(plan.power.iter().map(|p| p.amount).sum::<u16>(), 1);
    g.apply(action).unwrap();
    g.apply(Action::Pass).unwrap();
    g.players[1].hand.push(12);
    let chain_id = g.chain[0].id;
    reject(
        &mut g,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Spell { chain_id }],
        },
    );
    g.apply(Action::Pass).unwrap();
    assert_eq!(u(&g, friendly).damage, 0);
    assert_eq!(u(&g, enemy).damage, 3);
    assert_eq!(u(&g, enemy).temporary_buff, 5);
}
#[test]
fn firestorm_recipients_are_live_and_repeat_fields_independent() {
    let mut g = game();
    let a = unit(&mut g, 1, 1000, 69, Some(0));
    let b = unit(&mut g, 1, 1001, 69, Some(1));
    let base = unit(&mut g, 1, 1002, 69, None);
    repeated(
        &mut g,
        299,
        vec![Target::Battlefield { lane: 0 }],
        vec![vec![Target::Battlefield { lane: 1 }]],
    );
    assert_eq!(
        g.chain[0].executions[1].targets,
        vec![Target::Battlefield { lane: 1 }]
    );
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 9, vec![a]);
    resolve(&mut g);
    resolve(&mut g);
    assert_eq!(u(&g, a).damage, 0);
    assert_eq!(u(&g, b).damage, 3);
    assert_eq!(u(&g, base).damage, 0);
}
#[test]
fn untargeted_areas_hit_exact_zones_and_explicit_extra_target_is_rejected() {
    for card in [297, 298] {
        let mut g = game();
        let targets = [
            unit(&mut g, 0, 1000, 69, None),
            unit(&mut g, 1, 1001, 69, None),
            unit(&mut g, 0, 1002, 69, Some(0)),
            unit(&mut g, 1, 1003, 69, Some(1)),
        ];
        g.players[0].hand.push(card);
        reject(
            &mut g,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![targets[0]],
            },
        );
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        })
        .unwrap();
        resolve(&mut g);
        for (i, target) in targets.into_iter().enumerate() {
            assert_eq!(
                u(&g, target).damage,
                if (i < 2) == (card == 298) {
                    if card == 298 { 3 } else { 1 }
                } else {
                    0
                }
            );
        }
    }
}
#[test]
fn repeated_area_damage_defers_cleanup_and_bonuses_are_per_recipient() {
    let mut g = game();
    let annie = unit(&mut g, 0, 1000, 259, None);
    let enemy = unit(&mut g, 1, 1001, 69, None);
    um(&mut g, enemy).temporary_buff = 20;
    gear(&mut g, 0, 1002, 10, Some(1000));
    repeated(&mut g, 298, vec![], vec![vec![]]);
    resolve(&mut g);
    // Annie stays present for both executions, then Guardian saves her once.
    assert_eq!(u(&g, enemy).damage, 8);
    assert_eq!(u(&g, annie).damage, 0);
    assert!(g.players[0].discard.contains(&10));
}
#[test]
fn all_area_recipients_use_same_pre_deal_annie_board_independent_of_vector_order() {
    for reverse in [false, true] {
        let mut g = game();
        let annie = unit(&mut g, 0, 1000, 259, None);
        let friend = unit(&mut g, 0, 1001, 69, None);
        let enemy = unit(&mut g, 1, 1002, 69, None);
        if reverse {
            g.players[0].units.reverse();
        }
        cast(&mut g, 298, vec![]);
        resolve(&mut g);
        assert!(g.players[0].units.iter().all(|u| u.id != 1000));
        assert_eq!(u(&g, friend).damage, 4);
        assert_eq!(u(&g, enemy).damage, 4);
        let _ = annie;
    }
}
#[test]
fn two_lethal_area_victims_share_one_hourglass_controller_choice() {
    let mut g = game();
    let a = unit(&mut g, 1, 1000, 301, None);
    let b = unit(&mut g, 1, 1001, 301, None);
    gear(&mut g, 1, 1002, 21, None);
    cast(&mut g, 298, vec![]);
    resolve(&mut g);
    let Some(Decision::DeathReplacement {
        player: 1,
        ref options,
    }) = g.decision
    else {
        panic!("{:?}", g.decision)
    };
    assert_eq!(options.len(), 2);
    assert!(g.resolution_progress().is_none());
    g.apply(Action::ChooseDeathReplacement {
        unit_id: 1001,
        gear_id: Some(1002),
    })
    .unwrap();
    assert_eq!(u(&g, b).damage, 0);
    assert!(g.players[1].units.iter().all(|u| u.id != 1000));
    let _ = a;
}
#[test]
fn taric_recall_replacement_does_not_merge_newly_lethal_observer_into_original_deaths() {
    let mut g = game();
    unit(&mut g, 0, 900, 69, Some(0));
    let taric = unit(&mut g, 1, 1000, 307, Some(0));
    let original = unit(&mut g, 1, 1001, 300, Some(0));
    let observer = unit(&mut g, 1, 1002, 176, Some(0));
    gear(&mut g, 1, 1003, 10, Some(1000));
    combat(&mut g, 0, 0);
    um(&mut g, taric).damage = 5;
    um(&mut g, original).damage = 1;
    um(&mut g, observer).damage = 3;
    cast(&mut g, 297, vec![]);
    resolve(&mut g);
    drain(&mut g);
    // Taric is recalled before the original Scout dies. Viktor sees that death,
    // then dies in a separate cleanup after losing the local Shield bonus.
    assert_eq!(u(&g, taric).lane, None);
    assert!(
        g.players[1]
            .units
            .iter()
            .all(|u| u.id != 1001 && u.id != 1002)
    );
    assert_eq!(g.players[1].units.iter().filter(|u| u.token).count(), 1);
}
#[test]
fn replacement_choice_keeps_original_victim_even_when_recall_raises_its_might() {
    let mut g = game();
    unit(&mut g, 0, 900, 69, Some(0));
    let taric = unit(&mut g, 1, 1000, 307, Some(0));
    let water = unit(&mut g, 1, 1001, 305, Some(0));
    gear(&mut g, 1, 1002, 10, Some(1000));
    gear(&mut g, 1, 1003, 10, Some(1000));
    combat(&mut g, 0, 0);
    assert_eq!(g.unit_might(u(&g, taric)).unwrap(), 7);
    assert_eq!(g.unit_might(u(&g, water)).unwrap(), 3);
    um(&mut g, taric).damage = 6;
    um(&mut g, water).damage = 2;
    cast(&mut g, 297, vec![]);
    resolve(&mut g);
    let Some(Decision::DeathReplacement {
        player: 1,
        ref options,
    }) = g.decision
    else {
        panic!("{:?}", g.decision)
    };
    assert_eq!(options.len(), 2);
    assert_eq!(u(&g, water).damage, 3);
    g.apply(Action::ChooseDeathReplacement {
        unit_id: 1000,
        gear_id: Some(1002),
    })
    .unwrap();
    // The recall would raise Water to 4 Might: alone +2 replaces Taric's +1
    // Shield. Its already-created Kill event still happens (CR323.4/373.1.a).
    assert_eq!(u(&g, taric).lane, None);
    assert!(g.players[1].units.iter().all(|unit| unit.id != 1001));
    assert!(g.players[1].discard.contains(&305));
}

#[test]
fn live_combat_group_predicates_count_noncontributors_and_exclude_neutral_showdowns() {
    let mut g = game();
    let pigeons = unit(&mut g, 0, 1000, 304, Some(0));
    let other = unit(&mut g, 0, 1001, 15, Some(0));
    um(&mut g, other).stunned = true;
    um(&mut g, other).ready = false;
    combat(&mut g, 0, 0);
    assert_eq!(g.unit_might(u(&g, pigeons)).unwrap(), 5);
    g.showdown.as_mut().unwrap().attacker = 1;
    assert_eq!(g.unit_might(u(&g, pigeons)).unwrap(), 3);
    g.showdown.as_mut().unwrap().combat = false;
    assert_eq!(g.unit_might(u(&g, pigeons)).unwrap(), 3);
    let mut g = game();
    let water = unit(&mut g, 0, 1000, 305, Some(0));
    combat(&mut g, 0, 1);
    assert_eq!(g.unit_might(u(&g, water)).unwrap(), 4);
    let other = unit(&mut g, 0, 1001, 15, Some(0));
    um(&mut g, other).stunned = true;
    assert_eq!(g.unit_might(u(&g, water)).unwrap(), 2);
    um(&mut g, other).lane = None;
    assert_eq!(g.unit_might(u(&g, water)).unwrap(), 4);
    g.showdown.as_mut().unwrap().combat = false;
    assert_eq!(g.unit_might(u(&g, water)).unwrap(), 2);
}
#[test]
fn local_aura_sources_exclude_self_stack_and_keep_opponents_bases_distinct() {
    for (card, assault, shield) in [(306, 1, 0), (307, 0, 2)] {
        let mut g = game();
        let a = unit(&mut g, 0, 1000, card, None);
        let b = unit(&mut g, 0, 1001, card, None);
        let enemy = unit(&mut g, 1, 1002, 69, None);
        assert_eq!(g.unit_keywords(u(&g, a)).unwrap().assault, assault);
        assert_eq!(g.unit_keywords(u(&g, a)).unwrap().shield, shield);
        assert_eq!(g.unit_keywords(u(&g, enemy)).unwrap().assault, 0);
        assert_eq!(g.unit_keywords(u(&g, enemy)).unwrap().shield, 0);
        let contributions = g
            .passive_contributions(BoardObject::Unit {
                player: 0,
                unit_id: 1000,
            })
            .unwrap();
        assert_eq!(contributions.len(), 1);
        assert_eq!(
            contributions[0].source,
            BoardObject::Unit {
                player: 0,
                unit_id: 1001
            }
        );
        um(&mut g, b).lane = Some(0);
        assert_eq!(g.unit_keywords(u(&g, a)).unwrap().assault, 0);
        assert_eq!(
            g.unit_keywords(u(&g, a)).unwrap().shield,
            u64::from(card == 307)
        );
    }
}
#[test]
fn new_channel_and_predict_triggers_execute_on_real_play_and_death() {
    let mut g = game();
    let before = g.players[0].total_runes();
    cast(&mut g, 302, vec![]);
    drain(&mut g);
    assert_eq!(g.players[0].total_runes(), before + 1);
    assert_eq!(g.players[0].exhausted_runes.iter().sum::<u8>(), 1);
    assert_eq!(g.players[0].energy, 53);
    for card in [300, 301, 303] {
        let mut g = game();
        let target = unit(&mut g, 0, 1000, card, None);
        let before = g.players[0].total_runes();
        cast(&mut g, 294, vec![target]);
        resolve(&mut g);
        drain(&mut g);
        assert_eq!(g.players[0].total_runes(), before + u8::from(card != 303));
        assert!(g.players[0].discard.contains(&card));
    }
}

#[test]
fn legal_actions_exposes_both_field_slots_even_after_the_policy_candidate_cap() {
    let mut g = game();
    // Enough earlier plays to fill the bounded optional-cost policy candidates.
    g.players[0].hand = vec![4; 30];
    g.players[0].hand.push(299);
    let actions = g.legal_actions().unwrap();
    for lane in 0..2 {
        assert!(actions.iter().any(|action| matches!(action,
            Action::PlayCardWithTargets { hand_index: 30, lane: None, targets }
                if targets == &vec![Target::Battlefield { lane }])));
    }
    let mut affordable = game();
    affordable.players[0].hand.push(299);
    affordable.players[0].energy_pool = 0;
    affordable.players[0].energy = 0;
    assert!(
        !affordable
            .legal_actions()
            .unwrap()
            .iter()
            .any(|action| matches!(action,
        Action::PlayCardWithTargets { targets, .. }
            if matches!(targets.as_slice(), [Target::Battlefield { .. }])))
    );
}
