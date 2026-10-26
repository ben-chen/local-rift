//! Shared play grants, typed costs and resumable spell executions.
use local_rift::activated::AbilitySource;
use local_rift::game::{Action, Decision, Difficulty, Game, Target, Unit};
use local_rift::gear::GearState;
use local_rift::play::{CostScope, CostSelection, ExecutionChoices, PlayChoices, RepeatSelection};

fn position() -> Game {
    let mut game = Game::new_for_agents(3, 3, 3772, Difficulty::Tactical);
    game.start_match().unwrap();
    game.battlefields = [3, 3];
    game.legend[0].ready = false;
    game.legend[1].ready = false;
    game.battlefield_owner = [None; 2];
    for player in &mut game.players {
        player.hand.clear();
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.discard.clear();
        player.champion = None;
        player.runes = [0; 6];
        player.exhausted_runes = [0; 6];
        player.energy_pool = 20;
        player.energy = 20;
        player.power_pool = [8; 6];
        player.any_power = 0;
    }
    game
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
fn gear(g: &mut Game, id: u32, card_id: usize) -> AbilitySource {
    g.players[0].gear.push(GearState {
        id,
        card_id,
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
fn ability(source: AbilitySource, index: usize, targets: Vec<Target>) -> Action {
    Action::ActivateAbility {
        source,
        ability_index: index,
        targets,
    }
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
    assert!(g.decision.is_none(), "{:?}", g.decision);
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn repeat_grants(g: &mut Game, n: u32) {
    for i in 0..n {
        let src = gear(g, 8000 + i, 256);
        g.apply(ability(src, 0, vec![])).unwrap();
        resolve(g);
    }
}
fn draft(g: &mut Game, id: usize, targets: Vec<Target>) -> Action {
    let p = g.priority();
    let hand_index = g.players[p].hand.len();
    g.players[p].hand.push(id);
    Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    }
}
fn repeated(g: &Game, action: Action, targets: Vec<Target>) -> Action {
    let view = g.play_choices(&action).unwrap();
    Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats: view
                .repeats
                .iter()
                .map(|r| RepeatSelection {
                    option_id: r.id,
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
fn repeated_top_three_resumes_each_choice_before_physical_departure() {
    let mut g = position();
    repeat_grants(&mut g, 2);
    let a = draft(&mut g, 39, vec![]);
    let a = repeated(&g, a, vec![]);
    g.apply(a).unwrap();
    resolve(&mut g);
    for execution in 0..3 {
        assert!(matches!(g.decision, Some(Decision::TopThree { .. })));
        assert_eq!(g.resolution_progress().unwrap().execution, execution);
        assert!(g.chain.iter().any(|i| i.card_id == 39));
        assert!(!g.players[0].discard.contains(&39));
        g.apply(Action::SelectCards { indices: vec![] }).unwrap();
    }
    assert!(g.chain.is_empty());
    assert!(g.players[0].discard.contains(&39));
}
#[test]
fn repeated_sabotage_is_sequential_and_does_not_finish_on_first_selection() {
    let mut g = position();
    repeat_grants(&mut g, 2);
    g.players[1].hand = vec![7, 8, 9];
    let a = draft(&mut g, 32, vec![]);
    let a = repeated(&g, a, vec![]);
    g.apply(a).unwrap();
    resolve(&mut g);
    for count in (1..=3).rev() {
        assert!(matches!(g.decision, Some(Decision::Sabotage { .. })));
        assert_eq!(g.players[1].hand.len(), count);
        g.apply(Action::SelectCards { indices: vec![0] }).unwrap();
    }
    assert!(g.players[1].hand.is_empty());
    assert_eq!(
        g.players[0].discard.iter().filter(|&&id| id == 32).count(),
        1
    );
}
#[test]
fn repeated_loot_completes_discard_draw_each_time_and_observes_spell_once() {
    let mut g = position();
    repeat_grants(&mut g, 2);
    g.players[0].hand = vec![69];
    let before = g.trigger_state.completed_cards[0];
    let a = draft(&mut g, 84, vec![]);
    let a = repeated(&g, a, vec![]);
    g.apply(a).unwrap();
    resolve(&mut g);
    for execution in 0..3 {
        assert!(matches!(
            g.decision,
            Some(Decision::Discard {
                count: 1,
                draw_after: 2,
                ..
            })
        ));
        assert_eq!(g.resolution_progress().unwrap().execution, execution);
        g.apply(Action::SelectCards { indices: vec![0] }).unwrap();
    }
    assert_eq!(g.players[0].hand.len(), 4);
    assert_eq!(g.trigger_state.completed_cards[0], before + 1);
}
#[test]
fn two_repeat_instances_keep_their_own_domains_and_deflect_costs() {
    let mut g = position();
    repeat_grants(&mut g, 1);
    let target = unit(&mut g, 1, 9001, 0, Some(0));
    unit(&mut g, 0, 9002, 266, Some(0));
    g.showdown = Some(local_rift::game::Showdown {
        lane: 0,
        attacker: 0,
        focus: 0,
        passes: 0,
        combat: true,
    });
    let a = draft(&mut g, 50, vec![target]);
    let a = repeated(&g, a, vec![target]);
    let plan = g.cost_plan(&a).unwrap();
    assert!(
        plan.power
            .iter()
            .any(|c| c.domains == vec![local_rift::cards::Domain::Chaos] && c.amount == 1)
    );
    assert_eq!(
        plan.components
            .iter()
            .find(|c| c.label == "Deflect")
            .unwrap()
            .cost
            .power,
        4
    ); // initial + printed + Portal + Syndra
    g.apply(a).unwrap();
    assert_eq!(g.chain.last().unwrap().executions.len(), 4);
}
#[test]
fn ezreal_reduces_each_optional_component_before_total_discounts() {
    let mut g = position();
    unit(&mut g, 0, 101, 35, None);
    unit(&mut g, 0, 111, 35, None);
    repeat_grants(&mut g, 1);
    unit(&mut g, 0, 201, 266, Some(0));
    g.showdown = Some(local_rift::game::Showdown {
        lane: 0,
        attacker: 0,
        focus: 0,
        passes: 0,
        combat: false,
    });
    let a = draft(&mut g, 263, vec![]);
    let a = repeated(&g, a, vec![]);
    let plan = g.cost_plan(&a).unwrap();
    let ids: std::collections::HashSet<_> = plan.modifiers.iter().map(|m| m.id).collect();
    assert_eq!(ids.len(), plan.modifiers.len());
    assert_eq!(plan.energy, 2);
    assert_eq!(
        plan.modifiers
            .iter()
            .filter(|m| matches!(m.scope, CostScope::Component { .. }))
            .count(),
        4
    );
}
#[test]
fn explicitly_reversing_component_and_total_discount_stages_is_atomic() {
    let mut g = position();
    unit(&mut g, 0, 101, 35, None);
    unit(&mut g, 0, 102, 271, None);
    repeat_grants(&mut g, 1);
    let a = draft(
        &mut g,
        7,
        vec![Target::Unit {
            player: 0,
            unit_id: 102,
        }],
    );
    let a = repeated(
        &g,
        a,
        vec![Target::Unit {
            player: 0,
            unit_id: 102,
        }],
    );
    let plan = g.cost_plan(&a).unwrap();
    let total = plan
        .modifiers
        .iter()
        .find(|m| matches!(m.scope, CostScope::Total))
        .unwrap()
        .id;
    let component = plan
        .modifiers
        .iter()
        .find(|m| matches!(m.scope, CostScope::Component { .. }))
        .unwrap()
        .id;
    let Action::WithPlayChoices {
        action,
        mut choices,
    } = a
    else {
        panic!()
    };
    choices.cost_order = vec![
        CostSelection {
            modifier_id: total,
            use_power: false,
            power_allocations: vec![],
        },
        CostSelection {
            modifier_id: component,
            use_power: false,
            power_allocations: vec![],
        },
    ];
    let a = Action::WithPlayChoices { action, choices };
    let before = format!("{g:?}");
    assert!(g.apply(a).is_err());
    assert_eq!(format!("{g:?}"), before);
}
#[test]
fn optional_discount_power_preference_applies_to_every_repeat_component() {
    let mut g = position();
    unit(&mut g, 0, 101, 35, None);
    repeat_grants(&mut g, 2);
    g.apply(Action::SetOptionalDiscount { power: 1 }).unwrap();
    let a = draft(&mut g, 50, vec![]);
    let a = repeated(&g, a, vec![]);
    let p = g.cost_plan(&a).unwrap();
    assert_eq!(p.energy, 4);
    assert_eq!(p.power.len(), 1);
    assert_eq!(p.power[0].amount, 1);
    assert_eq!(p.power[0].domains, vec![local_rift::cards::Domain::Mind]);
}
#[test]
fn ready_entry_precedes_unit_play_observers_without_emitting_ready_event() {
    let mut g = position();
    unit(&mut g, 0, 700, 264, None);
    unit(&mut g, 0, 701, 154, None);
    let a = draft(&mut g, 153, vec![]);
    g.apply(a).unwrap();
    assert!(g.players[0].units.last().unwrap().ready);
    assert!(g.chain.is_empty());
}
#[test]
fn hidden_independent_repeats_reject_targets_outside_original_field() {
    let mut g = position();
    repeat_grants(&mut g, 1);
    unit(&mut g, 0, 701, 69, Some(0));
    let target = unit(&mut g, 1, 702, 69, Some(1));
    g.players[0].hidden.push(local_rift::hidden::HiddenCard {
        id: 400,
        card_id: 53,
        lane: 0,
        hidden_turn: 0,
        revealed: false,
    });
    let a = Action::PlayHidden {
        hidden_id: 400,
        targets: vec![],
    };
    let r = g.play_choices(&a).unwrap().repeats[0].id;
    let a = Action::WithPlayChoices {
        action: Box::new(a),
        choices: PlayChoices {
            repeats: vec![RepeatSelection {
                option_id: r,
                choices: ExecutionChoices {
                    targets: vec![target],
                    mode: None,
                },
            }],
            ..Default::default()
        },
    };
    assert!(g.apply(a).is_err());
    assert_eq!(g.players[0].hidden.len(), 1);
}
#[test]
fn token_accelerate_recycles_any_domain_and_enters_ready_without_ready_trigger() {
    let mut g = position();
    unit(&mut g, 0, 700, 265, None);
    cast(&mut g, 87, vec![]);
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::TokenPlay { remaining: 4, .. })
    ));
    let before = g.players[0].energy;
    g.apply(Action::FinalizeTokenPlay {
        lane: None,
        accelerate: true,
    })
    .unwrap();
    assert_eq!(g.players[0].energy, before - 1);
    assert!(g.players[0].units.last().unwrap().ready);
    g.apply(Action::FinalizeTokenPlay {
        lane: None,
        accelerate: false,
    })
    .unwrap();
    assert!(!g.players[0].units.last().unwrap().ready);
}

#[test]
fn independent_counter_taxes_suspend_the_same_parent_then_resume_next_execution() {
    let mut g = position();
    repeat_grants(&mut g, 1);
    let mine = unit(&mut g, 0, 6001, 69, None);
    let theirs = unit(&mut g, 1, 6002, 69, None);
    let heart = gear(&mut g, 6003, 230);
    g.apply(ability(heart, 0, vec![mine])).unwrap();
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 7, vec![theirs]);
    let target = Target::Spell {
        chain_id: g.chain.last().unwrap().id,
    };
    g.apply(Action::Pass).unwrap();
    let a = draft(&mut g, 37, vec![target]);
    let a = repeated(&g, a, vec![target]);
    g.apply(a).unwrap();
    let parent = g.chain.last().unwrap().id;
    resolve(&mut g);
    let before = g.players[1].energy;
    for n in 0..3 {
        assert!(matches!(
            g.decision,
            Some(Decision::CounterTax {
                player: 1,
                cost: 2,
                ..
            })
        ));
        assert_eq!(g.resolution_progress().unwrap().execution, n);
        assert!(g.chain.iter().any(|i| i.id == parent));
        g.apply(Action::Choose { accept: true }).unwrap();
    }
    assert_eq!(g.players[1].energy, before - 6);
    assert!(!g.chain.iter().any(|i| i.id == parent));
    assert!(g.chain.iter().any(|i| i.id
        == match target {
            Target::Spell { chain_id } => chain_id,
            _ => 0,
        }));
}
#[test]
fn repeated_abandon_predicts_again_when_original_spell_target_has_left() {
    let mut g = position();
    repeat_grants(&mut g, 1);
    let mine = unit(&mut g, 0, 6001, 69, None);
    let theirs = unit(&mut g, 1, 6002, 69, None);
    let heart = gear(&mut g, 6003, 230);
    g.apply(ability(heart, 0, vec![mine])).unwrap();
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 7, vec![theirs]);
    let target = Target::Spell {
        chain_id: g.chain.last().unwrap().id,
    };
    g.apply(Action::Pass).unwrap();
    let a = draft(&mut g, 1, vec![target]);
    let a = repeated(&g, a, vec![target]);
    g.apply(a).unwrap();
    resolve(&mut g);
    for execution in 0..2 {
        assert!(matches!(
            g.decision,
            Some(Decision::Predict { player: 0, .. })
        ));
        assert_eq!(g.resolution_progress().unwrap().execution, execution);
        g.apply(Action::Choose { accept: false }).unwrap();
    }
    assert!(g.players[1].hand.contains(&7));
    assert_eq!(
        g.players[0].discard.iter().filter(|&&id| id == 1).count(),
        1
    );
}
#[test]
fn failed_payment_restores_reserved_grants_and_pending_physical_card() {
    let mut g = position();
    repeat_grants(&mut g, 1);
    let grant = g.play_grants[0].clone();
    g.players[0].energy = 0;
    g.players[0].energy_pool = 0;
    g.players[0].power_pool = [0; 6];
    let action = draft(&mut g, 77, vec![]);
    g.apply(Action::BeginPayment {
        action: Box::new(action),
    })
    .unwrap();
    assert!(g.play_grants.is_empty());
    assert_eq!(g.payment_plan_view(0).unwrap().pending_card, Some(77));
    assert!(g.apply(Action::CompletePayment).is_err());
    g.apply(Action::Choose { accept: false }).unwrap();
    assert!(g.players[0].hand.contains(&77));
    assert!(g.play_grants.contains(&grant));
}
#[test]
fn stargazer_uses_exact_trash_objects_flow_for_instructed_play_optional_costs() {
    let mut g = position();
    unit(&mut g, 0, 7001, 269, None);
    let target = unit(&mut g, 1, 7002, 69, Some(0));
    g.players[0].discard = vec![67, 67];
    g.decision = Some(Decision::GrantFlow { player: 0 });
    g.apply(Action::SelectCards { indices: vec![0] }).unwrap();
    resolve(&mut g);
    repeat_grants(&mut g, 1);
    g.decision = Some(Decision::ReplaySpell { player: 0 });
    let make = |index| Action::PlayFromTrashWithTargets {
        trash_index: index,
        lane: None,
        targets: vec![target],
    };
    let a = repeated(&g, make(0), vec![target]);
    let b = repeated(&g, make(1), vec![target]);
    assert_eq!(g.cost_plan(&a).unwrap().energy, 1);
    assert_eq!(g.cost_plan(&b).unwrap().energy, 3);
}
#[test]
fn explicit_power_reduction_allocates_across_distinct_repeat_components() {
    let mut g = position();
    g.battlefields = [0, 0];
    unit(&mut g, 0, 7001, 258, Some(0));
    cast(&mut g, 9, vec![]);
    resolve(&mut g);
    resolve(&mut g);
    unit(&mut g, 0, 7002, 266, Some(0));
    g.showdown = Some(local_rift::game::Showdown {
        lane: 0,
        attacker: 0,
        focus: 0,
        passes: 0,
        combat: false,
    });
    let a = draft(&mut g, 50, vec![]);
    let a = repeated(&g, a, vec![]);
    let plan = g.cost_plan(&a).unwrap();
    let modifier = plan.modifiers.iter().find(|m| m.power == 2).unwrap().id;
    let chaos = plan
        .components
        .iter()
        .find(|c| c.cost.domains == vec![local_rift::cards::Domain::Chaos])
        .unwrap()
        .id;
    let Action::WithPlayChoices {
        action,
        mut choices,
    } = a
    else {
        panic!()
    };
    choices.cost_order = vec![CostSelection {
        modifier_id: modifier,
        use_power: false,
        power_allocations: vec![
            local_rift::play::PowerReduction {
                component_id: 1,
                amount: 1,
            },
            local_rift::play::PowerReduction {
                component_id: chaos,
                amount: 1,
            },
        ],
    }];
    let a = Action::WithPlayChoices { action, choices };
    let plan = g.cost_plan(&a).unwrap();
    assert_eq!(plan.power.len(), 1);
    assert_eq!(plan.power[0].domains, vec![local_rift::cards::Domain::Mind]);
    assert_eq!(plan.power[0].amount, 1);
    g.apply(a).unwrap();
}
#[test]
fn minimum_energy_applies_at_each_discount_step_not_as_a_global_floor() {
    let mut g = position();
    let researcher = unit(&mut g, 0, 7001, 267, None);
    let Target::Unit { unit_id, .. } = researcher else {
        panic!()
    };
    g.apply(ability(
        AbilitySource::Unit { player: 0, unit_id },
        0,
        vec![],
    ))
    .unwrap();
    resolve(&mut g);
    unit(&mut g, 0, 7002, 271, None);
    let a = draft(
        &mut g,
        7,
        vec![Target::Unit {
            player: 0,
            unit_id: 7002,
        }],
    );
    let p = g.cost_plan(&a).unwrap();
    let researchers = p
        .modifiers
        .iter()
        .find(|m| m.minimum_energy == 1)
        .unwrap()
        .id;
    let irelia = p
        .modifiers
        .iter()
        .find(|m| m.choose_energy_or_power)
        .unwrap()
        .id;
    let select = |id| CostSelection {
        modifier_id: id,
        use_power: false,
        power_allocations: vec![],
    };
    let after = Action::WithPlayChoices {
        action: Box::new(a.clone()),
        choices: PlayChoices {
            cost_order: vec![select(researchers), select(irelia)],
            ..Default::default()
        },
    };
    let before = Action::WithPlayChoices {
        action: Box::new(a),
        choices: PlayChoices {
            cost_order: vec![select(irelia), select(researchers)],
            ..Default::default()
        },
    };
    assert_eq!(g.cost_plan(&after).unwrap().energy, 0);
    assert_eq!(g.cost_plan(&before).unwrap().energy, 1);
}

#[test]
fn hidden_deflect_uses_the_same_total_discount_plan_as_hand_plays() {
    let mut g = position();
    unit(&mut g, 0, 7001, 267, None);
    g.apply(ability(
        AbilitySource::Unit {
            player: 0,
            unit_id: 7001,
        },
        0,
        vec![],
    ))
    .unwrap();
    resolve(&mut g);
    let target = unit(&mut g, 1, 7002, 0, Some(0));
    unit(&mut g, 0, 7003, 69, Some(0));
    g.players[0].hidden.push(local_rift::hidden::HiddenCard {
        id: 400,
        card_id: 53,
        lane: 0,
        hidden_turn: 0,
        revealed: false,
    });
    g.players[0].power_pool = [0; 6];
    let a = Action::PlayHidden {
        hidden_id: 400,
        targets: vec![target],
    };
    assert!(g.cost_plan(&a).unwrap().power.is_empty());
    g.apply(a).unwrap();
    assert!(g.chain.iter().any(|item| item.card_id == 53));
}
#[test]
fn cost_increases_reject_discount_payloads_and_roundtrip_the_default_plan() {
    let mut g = position();
    g.players[1].gear.push(GearState {
        id: 7001,
        card_id: 268,
        lane: None,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    let a = draft(&mut g, 77, vec![]);
    let plan = g.cost_plan(&a).unwrap();
    let increase = plan.modifiers.iter().find(|m| m.increase).unwrap().id;
    for (use_power, power_allocations) in [
        (true, vec![]),
        (
            false,
            vec![local_rift::play::PowerReduction {
                component_id: 1,
                amount: 1,
            }],
        ),
    ] {
        let bad = Action::WithPlayChoices {
            action: Box::new(a.clone()),
            choices: PlayChoices {
                cost_order: vec![CostSelection {
                    modifier_id: increase,
                    use_power,
                    power_allocations,
                }],
                ..Default::default()
            },
        };
        let before = format!("{g:?}");
        assert!(g.cost_plan(&bad).is_err());
        assert!(g.apply(bad).is_err());
        assert_eq!(format!("{g:?}"), before);
    }
    let roundtrip = Action::WithPlayChoices {
        action: Box::new(a),
        choices: PlayChoices {
            cost_order: plan.applied_order,
            ..Default::default()
        },
    };
    assert_eq!(g.cost_plan(&roundtrip).unwrap().energy, plan.energy);
    g.apply(roundtrip).unwrap();
}
#[test]
fn ai_candidates_obey_current_unit_surcharge() {
    let mut g = position();
    g.players[0].hand = vec![56];
    g.players[0].energy = 4;
    g.players[0].energy_pool = 4;
    g.play_grants.push(local_rift::play::PlayGrant {
        id: 7001,
        player: 0,
        source: local_rift::game::ChainSource::Battlefield {
            lane: 0,
            battlefield_id: 10,
        },
        expires_turn: g.turn,
        kind: local_rift::play::PlayGrantKind::UnitSurchargeThisTurn(1),
    });
    for action in g.legal_actions().unwrap() {
        assert!(g.clone().apply(action.clone()).is_ok(), "{action:?}");
    }
    g.step_ai().unwrap();
}

#[test]
fn repeated_rampage_reexecutes_the_paid_conditional_buff_on_a_shared_target() {
    let mut g = position();
    repeat_grants(&mut g, 1);
    let friend = unit(&mut g, 0, 991, 69, None);
    let enemy = unit(&mut g, 1, 992, 15, None);
    let action = draft(&mut g, 29, vec![friend, enemy]);
    let mut action = repeated(&g, action, vec![friend, enemy]);
    if let Action::WithPlayChoices { choices, .. } = &mut action {
        choices.bonus = true;
    }
    let power_before = g.players[0].power_pool[3];
    g.apply(action).unwrap();
    assert_eq!(
        g.players[0].power_pool[3],
        power_before - 1,
        "Rampage's additional cost is paid once"
    );
    resolve(&mut g);
    assert_eq!(
        g.players[0]
            .units
            .iter()
            .find(|u| u.id == 991)
            .unwrap()
            .temporary_buff,
        4
    );
}

#[test]
fn repeated_rampage_buffs_each_executions_selected_friend_only_if_paid() {
    for paid in [false, true] {
        let mut g = position();
        repeat_grants(&mut g, 1);
        let first = unit(&mut g, 0, 991, 69, None);
        let second = unit(&mut g, 0, 992, 69, None);
        let enemy = unit(&mut g, 1, 993, 15, None);
        let action = draft(&mut g, 29, vec![first, enemy]);
        let mut action = repeated(&g, action, vec![second, enemy]);
        if let Action::WithPlayChoices { choices, .. } = &mut action {
            choices.bonus = paid;
        }
        g.apply(action).unwrap();
        resolve(&mut g);
        for id in [991, 992] {
            assert_eq!(
                g.players[0]
                    .units
                    .iter()
                    .find(|u| u.id == id)
                    .unwrap()
                    .temporary_buff,
                if paid { 2 } else { 0 }
            );
        }
    }
}

#[test]
fn eclipse_predict_defers_lethal_cleanup_until_the_physical_spell_finishes() {
    let mut g = position();
    let target = unit(&mut g, 0, 991, 69, None);
    g.players[0].units[0].damage = 4;
    cast(&mut g, 52, vec![target]);
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::Predict { player: 0, .. })
    ));
    assert!(g.resolution_progress().is_some());
    let lethal = g.players[0].units.iter().find(|u| u.id == 991).unwrap();
    assert!(lethal.damage >= g.unit_might(lethal).unwrap());
    assert!(g.chain.iter().any(|i| i.card_id == 52));
    assert!(!g.players[0].discard.contains(&52));
    g.apply(Action::Choose { accept: false }).unwrap();
    assert!(g.resolution_progress().is_none());
    assert!(!g.players[0].units.iter().any(|u| u.id == 991));
    assert!(g.players[0].discard.contains(&52));
    assert!(g.players[0].discard.contains(&69));
}

#[test]
fn repeated_eclipse_finishes_both_predicts_before_death_replacement_choice() {
    let mut g = position();
    repeat_grants(&mut g, 1);
    let target = unit(&mut g, 0, 991, 69, None);
    g.players[0].units[0].damage = 4;
    gear(&mut g, 992, 21);
    gear(&mut g, 993, 21);
    let action = draft(&mut g, 52, vec![target]);
    let action = repeated(&g, action, vec![target]);
    g.apply(action).unwrap();
    resolve(&mut g);
    for execution in 0..2 {
        assert!(matches!(
            g.decision,
            Some(Decision::Predict { player: 0, .. })
        ));
        assert_eq!(g.resolution_progress().unwrap().execution, execution);
        assert_eq!(g.players[0].units[0].damage, 4);
        assert!(g.players[0].gear.iter().any(|gear| gear.id == 992));
        assert!(g.players[0].gear.iter().any(|gear| gear.id == 993));
        g.apply(Action::Choose { accept: false }).unwrap();
    }
    assert!(g.resolution_progress().is_none());
    assert!(g.players[0].discard.contains(&52));
    assert!(matches!(
        g.decision,
        Some(Decision::DeathReplacement { player: 0, .. })
    ));
    g.apply(Action::ChooseDeathReplacement {
        unit_id: 991,
        gear_id: Some(992),
    })
    .unwrap();
    assert_eq!(
        g.players[0]
            .units
            .iter()
            .find(|u| u.id == 991)
            .unwrap()
            .damage,
        0
    );
    assert!(!g.players[0].gear.iter().any(|gear| gear.id == 992));
    assert!(g.players[0].gear.iter().any(|gear| gear.id == 993));
}

#[test]
fn strategic_seed117_cleans_lethal_vex_after_pending_eclipse_predict() {
    let mut g = Game::new(0, 3, 117, Difficulty::Strategic);
    g.choose_starting_player(0).unwrap();
    g.start_match().unwrap();
    for _ in 0..34 {
        g.step_ai().unwrap();
    }
    assert!(matches!(
        g.decision,
        Some(Decision::Predict { player: 1, .. })
    ));
    assert!(g.resolution_progress().is_some());
    let vex = g.players[0].units.iter().find(|u| u.id == 2).unwrap();
    assert_eq!(vex.card_id, 20);
    assert_eq!(vex.damage, 4);
    assert_eq!(g.unit_might(vex).unwrap(), 2);
    g.step_ai().unwrap();
    assert!(g.resolution_progress().is_none());
    assert!(!g.players[0].units.iter().any(|u| u.id == 2));
    assert!(g.players[0].discard.contains(&20));
}
