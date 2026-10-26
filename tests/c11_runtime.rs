//! XP costs, dependent mixed Add, and per-panel equipment triggers.
use local_rift::activated::{AbilitySource, activated_rules};
use local_rift::game::{Action, ChainAbility, Decision, Difficulty, Game, Target, Unit};
use local_rift::gear::GearState;
use local_rift::payment::PowerPayment;
use local_rift::rules;
use local_rift::trigger_rules::{self, TriggerTextArea};
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

fn source(t: Target) -> AbilitySource {
    let Target::Unit { player, unit_id } = t else {
        panic!()
    };
    AbilitySource::Unit { player, unit_id }
}
fn gear(g: &mut Game, id: u32, card: usize) -> AbilitySource {
    g.players[0].gear.push(GearState {
        id,
        card_id: card,
        ready: true,
        lane: None,
        attached_to: None,
        temporary: false,
        token: false,
    });
    AbilitySource::Gear {
        player: 0,
        gear_id: id,
    }
}
fn ability(s: AbilitySource, index: usize) -> Action {
    Action::ActivateAbility {
        source: s,
        ability_index: index,
        targets: vec![],
    }
}
fn begin(g: &mut Game, a: Action) {
    g.apply(Action::BeginPayment {
        action: Box::new(a),
    })
    .unwrap();
}
fn rejected(g: &mut Game, a: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(a).is_err());
    assert_eq!(format!("{g:?}"), before);
}
fn no_resources(g: &mut Game, p: usize) {
    g.players[p].energy = 0;
    g.players[p].energy_pool = 0;
    g.players[p].power_pool = [0; 6];
    g.players[p].any_power = 0;
    g.players[p].runes = [0; 6];
    g.players[p].exhausted_runes = [0; 6];
    g.legend[p].ready = false;
    g.legend[p].spell_power = 0;
}
fn pass_until(g: &mut Game, predicate: impl Fn(&Game) -> bool) {
    for _ in 0..50 {
        if predicate(g) {
            return;
        }
        let action = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(d) => panic!("unexpected decision {d:?}"),
            None => Action::Pass,
        };
        g.apply(action).unwrap();
    }
    panic!("condition not reached");
}

#[test]
fn typed_costs_outputs_and_text_panels_have_explicit_sources() {
    for card in [345, 346] {
        let r = &activated_rules(card)[0];
        assert_eq!(r.xp_cost, 2);
        assert!(!r.exhaust);
        assert_eq!((r.energy, r.power, r.targets.max), (0, 0, 0));
        assert_eq!(
            rules::static_abilities(card),
            &[rules::StaticAbility::Hunt(1)]
        );
    }
    let r = activated_rules(347);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].minimum_xp, None);
    assert_eq!(r[1].minimum_xp, Some(6));
    for (index, energy) in [(0, 0), (1, 1)] {
        let out = r[index].fixed_resource_output().unwrap();
        assert_eq!((out.energy, out.power, out.domain), (energy, 1, None));
        assert!(r[index].adds_resources());
    }
    let e = rules::equipment(348).unwrap();
    assert_eq!(
        (e.equip_xp, e.equip_energy, e.equip_power, e.might),
        (1, 0, 0, 2)
    );
    assert_eq!(
        trigger_rules::trigger_rules(348)[0].text_area,
        TriggerTextArea::Rules
    );
    for id in (199..=207).chain([329]) {
        assert!(
            trigger_rules::trigger_rules(id)
                .iter()
                .all(|r| r.text_area == TriggerTextArea::Equipment)
        );
    }
    for id in [195, 196, 197, 198, 242, 246] {
        assert!(
            trigger_rules::trigger_rules(id)
                .iter()
                .all(|r| r.text_area == TriggerTextArea::Rules)
        );
    }
}

#[test]
fn xp_only_buff_is_paid_once_direct_explicit_and_staged_without_exhausting() {
    for card in [345, 346] {
        for mode in 0..3 {
            let mut g = game();
            no_resources(&mut g, 0);
            g.players[0].xp = u64::MAX;
            let t = unit(&mut g, 0, 1000, card, None);
            g.players[0].units[0].ready = false;
            let a = ability(source(t), 0);
            let plan = g.cost_plan(&a).unwrap();
            assert_eq!((plan.xp, plan.energy, plan.power.len()), (2, 0, 0));
            assert_eq!(plan.components[0].cost.xp, 2);
            assert!(g.quote_power_payment(&a).unwrap().costs.is_empty());
            if mode == 0 {
                g.apply(a).unwrap();
            } else if mode == 1 {
                g.apply(Action::WithPowerPayment {
                    action: Box::new(a),
                    payment: PowerPayment::default(),
                })
                .unwrap();
            } else {
                begin(&mut g, a);
                assert_eq!(g.players[0].xp, u64::MAX);
                assert_eq!(g.payment_plan_view(0).unwrap().xp, 2);
                g.apply(Action::WithPowerPayment {
                    action: Box::new(Action::CompletePayment),
                    payment: PowerPayment::default(),
                })
                .unwrap();
            }
            assert_eq!(g.players[0].xp, u64::MAX - 2);
            assert_eq!(u(&g, t).buff, 0);
            assert!(!u(&g, t).ready);
            resolve(&mut g);
            assert_eq!(u(&g, t).buff, 1);
            assert_eq!(g.players[0].xp, u64::MAX - 2);
            assert!(!g.gained_xp_this_turn(0));
        }
    }
}

#[test]
fn shortage_can_be_staged_but_not_paid_and_undo_restores_everything() {
    let mut g = game();
    let t = unit(&mut g, 0, 1000, 345, None);
    g.players[0].xp = 1;
    let a = ability(source(t), 0);
    assert_eq!(g.cost_plan(&a).unwrap().xp, 2);
    assert!(g.quote_power_payment(&a).is_err());
    rejected(&mut g, a.clone());
    let original = format!("{g:?}");
    begin(&mut g, a);
    assert_eq!(g.payment_plan_view(1).unwrap().xp, 2);
    assert!(g.payment_plan_view(1).unwrap().action.is_none());
    rejected(&mut g, Action::CompletePayment);
    assert!(g.can_cancel_payment().unwrap());
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(format!("{g:?}"), original);
}

#[test]
fn actual_hunt_gain_over_255_then_spend_preserves_positive_history() {
    let mut g = game();
    g.battlefields[0] = 3;
    g.players[0].xp = 255;
    let t = unit(&mut g, 0, 1000, 345, None);
    g.apply(Action::Move {
        unit_id: 1000,
        lane: Some(0),
    })
    .unwrap();
    pass_until(&mut g, |g| {
        g.chain
            .iter()
            .any(|i| matches!(i.ability, Some(ChainAbility::Hunt { amount: 1, .. })))
    });
    settle(&mut g);
    assert_eq!(g.players[0].xp, 256);
    g.apply(ability(source(t), 0)).unwrap();
    assert_eq!(g.players[0].xp, 254);
    assert!(g.gained_xp_this_turn(0));
    settle(&mut g);
    assert_eq!(u(&g, t).buff, 1);
}

#[test]
fn self_buff_has_no_target_and_no_effect_on_a_returned_source() {
    let mut g = game();
    g.players[0].xp = 2;
    let t = unit(&mut g, 0, 1000, 346, None);
    let enemy = unit(&mut g, 1, 1001, 69, None);
    rejected(
        &mut g,
        Action::ActivateAbility {
            source: source(t),
            ability_index: 0,
            targets: vec![t],
        },
    );
    g.apply(ability(source(t), 0)).unwrap();
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 16, vec![enemy, t]);
    resolve(&mut g);
    assert!(g.players[0].units.is_empty());
    assert!(g.players[0].hand.contains(&346));
    settle(&mut g);
    assert_eq!(g.players[0].xp, 0);
    assert!(!g.buffed_units.contains(&1000));
}

#[test]
fn already_buffed_activation_is_legal_but_ai_does_not_repeat_it() {
    let mut g = game();
    no_resources(&mut g, 0);
    g.players[0].xp = 6;
    let t = unit(&mut g, 0, 1000, 345, None);
    g.players[0].units[0].ready = false;
    g.step_ai().unwrap();
    assert_eq!(g.players[0].xp, 4);
    settle(&mut g);
    assert_eq!(u(&g, t).buff, 1);
    g.apply(ability(source(t), 0)).unwrap();
    settle(&mut g);
    assert_eq!((g.players[0].xp, u(&g, t).buff), (2, 1));
    g.step_ai().unwrap();
    assert_eq!(g.players[0].xp, 2, "policy does not pay for a no-op Buff");
}

#[test]
fn xp_cost_buff_obeys_ordinary_timing_and_cleanup_after_payment() {
    let mut g = game();
    g.players[0].xp = 3;
    let t = unit(&mut g, 0, 1000, 345, None);
    unit(&mut g, 0, 1001, 339, None);
    g.players[0].units[1].damage = 3;
    assert_eq!(g.unit_might(&g.players[0].units[1]).unwrap(), 4);
    g.apply(ability(source(t), 0)).unwrap();
    assert_eq!(g.players[0].xp, 1);
    assert!(!g.players[0].units.iter().any(|u| u.id == 1001));
    assert!(g.players[0].discard.contains(&339));
    rejected(&mut g, ability(source(t), 0));
    settle(&mut g);
    assert_eq!(u(&g, t).buff, 1);
    g.apply(Action::EndTurn).unwrap();
    rejected(&mut g, ability(source(t), 0));
}

#[test]
fn honeyfruit_enters_exhausted_and_its_reaction_text_does_not_change_card_speed() {
    let mut g = game();
    g.players[0].xp = 6;
    cast(&mut g, 347, vec![]);
    let id = g.players[0].gear[0].id;
    assert!(!g.players[0].gear[0].ready);
    rejected(
        &mut g,
        ability(
            AbilitySource::Gear {
                player: 0,
                gear_id: id,
            },
            1,
        ),
    );
    let t = unit(&mut g, 0, 1000, 69, None);
    cast(&mut g, 7, vec![t]);
    let a = draft(&mut g, 347, vec![]);
    rejected(&mut g, a);
}

#[test]
fn honeyfruit_dependent_existence_and_shared_exhaust_cost_are_exact() {
    for xp in [5, 6] {
        for index in 0..2 {
            let mut g = game();
            no_resources(&mut g, 0);
            g.players[0].xp = xp;
            let s = gear(&mut g, 2000, 347);
            let options: Vec<_> = g
                .activated_options(0)
                .into_iter()
                .map(|o| o.ability_index)
                .collect();
            assert_eq!(options, if xp == 5 { vec![0] } else { vec![0, 1] });
            if xp == 5 && index == 1 {
                assert!(g.cost_plan(&ability(s, index)).is_err());
                rejected(&mut g, ability(s, index));
                continue;
            }
            let priority = g.priority();
            g.apply(ability(s, index)).unwrap();
            assert_eq!(
                (g.players[0].energy, g.players[0].any_power),
                (index as u16, 1)
            );
            assert_eq!(g.players[0].xp, xp);
            assert_eq!(g.priority(), priority);
            assert!(g.chain.is_empty());
            rejected(&mut g, ability(s, 1 - index));
        }
    }
}

#[test]
fn mixed_add_checks_capacity_atomically_and_preserves_values_above_255() {
    for (energy, power) in [(u16::MAX, 0), (0, u16::MAX)] {
        let mut g = game();
        no_resources(&mut g, 0);
        g.players[0].xp = 6;
        g.players[0].energy = energy;
        g.players[0].energy_pool = energy;
        g.players[0].any_power = power;
        let s = gear(&mut g, 2000, 347);
        let before = format!("{g:?}");
        assert!(
            g.apply(ability(s, 1))
                .unwrap_err()
                .starts_with("engine_capacity:")
        );
        assert_eq!(format!("{g:?}"), before);
    }
    let mut g = game();
    g.players[0].xp = 6;
    g.players[0].energy = 400;
    g.players[0].energy_pool = 400;
    g.players[0].any_power = 400;
    let s = gear(&mut g, 2000, 347);
    g.apply(ability(s, 1)).unwrap();
    assert_eq!((g.players[0].energy, g.players[0].any_power), (401, 401));
}

#[test]
fn actual_countertax_finds_honeyfruit_second_ability_and_preserves_parent() {
    let mut g = game();
    no_resources(&mut g, 0);
    g.players[0].xp = 6;
    g.players[0].energy = 3;
    g.players[0].energy_pool = 3;
    let s = gear(&mut g, 2000, 347);
    let t = unit(&mut g, 0, 1000, 69, None);
    cast(&mut g, 7, vec![t]);
    let chain_id = g.chain[0].id;
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 37, vec![Target::Spell { chain_id }]);
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::CounterTax {
            player: 0,
            cost: 2,
            ..
        })
    ));
    let before = g.decision.clone();
    let parent_ids: Vec<_> = g.chain.iter().map(|i| i.id).collect();
    g.step_ai().unwrap();
    assert_eq!(g.decision, before);
    assert_eq!((g.players[0].energy, g.players[0].any_power), (2, 1));
    assert_eq!(g.chain.iter().map(|i| i.id).collect::<Vec<_>>(), parent_ids);
    assert!(g.validate_ability(s, 0, &[]).is_err());
    g.step_ai().unwrap();
    settle(&mut g);
    assert_eq!(u(&g, t).temporary_buff, 2);
}

#[test]
fn nested_conversion_keeps_honeyfruit_output_and_gear_completion_separate() {
    let mut g = game();
    no_resources(&mut g, 0);
    g.players[0].xp = 6;
    let honey = gear(&mut g, 2000, 347);
    let converter = gear(&mut g, 2001, 245);
    let prize = unit(&mut g, 0, 1000, 215, None);
    g.apply(ability(converter, 0)).unwrap();
    let d = g.decision.clone();
    g.apply(ability(honey, 1)).unwrap();
    assert_eq!(g.decision, d);
    assert_eq!(g.conversion_available(), Some(1));
    assert!(
        g.chain.is_empty(),
        "ordinary observers wait for the suspended task"
    );
    g.apply(Action::ResolveConversion { amount: 1 }).unwrap();
    assert_eq!((g.players[0].energy, g.players[0].any_power), (2, 0));
    settle(&mut g);
    assert_eq!(
        u(&g, prize).temporary_buff,
        2,
        "two completed gear uses, one each"
    );
}

#[test]
fn failed_xp_payment_restores_honeyfruit_and_generated_power() {
    let mut g = game();
    no_resources(&mut g, 0);
    let t = unit(&mut g, 0, 1000, 345, None);
    let honey = gear(&mut g, 2000, 347);
    g.players[0].xp = 1;
    let before = format!("{g:?}");
    begin(&mut g, ability(source(t), 0));
    g.apply(ability(honey, 0)).unwrap();
    assert_eq!(g.players[0].any_power, 1);
    rejected(&mut g, Action::CompletePayment);
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn staged_mixed_resource_card_payment_keeps_the_exact_power_obligation() {
    let mut g = game();
    no_resources(&mut g, 0);
    g.players[0].xp = 6;
    g.players[0].energy = 2;
    g.players[0].energy_pool = 2;
    let honey = gear(&mut g, 2000, 347);
    let enemy = unit(&mut g, 1, 1000, 69, Some(0));
    let a = draft(&mut g, 227, vec![enemy]); // Shock Blast3E1P
    begin(&mut g, a);
    assert_eq!(g.payment_plan_view(0).unwrap().xp, 0);
    g.apply(ability(honey, 1)).unwrap();
    g.apply(Action::WithPowerPayment {
        action: Box::new(Action::CompletePayment),
        payment: PowerPayment {
            any: 1,
            ..Default::default()
        },
    })
    .unwrap();
    assert_eq!(
        (g.players[0].energy, g.players[0].any_power, g.players[0].xp),
        (0, 0, 6)
    );
    assert_eq!(g.chain.last().unwrap().card_id, 227);
}

#[test]
fn heirloom_play_trigger_uses_exact_gear_identity_then_pays_one_xp_to_equip() {
    let mut g = game();
    no_resources(&mut g, 0);
    g.players[0].energy = 2;
    g.players[0].energy_pool = 2;
    gear(&mut g, 2000, 348); // an older copy must not observe another copy's play
    let t = unit(&mut g, 0, 1000, 69, None);
    cast(&mut g, 348, vec![]);
    assert_eq!(g.players[0].xp, 0);
    let new_id = g.players[0].gear.iter().find(|x| x.id != 2000).unwrap().id;
    assert_eq!(g.chain.len(), 1);
    let Some(ChainAbility::CardTrigger { context, .. }) = &g.chain[0].ability else {
        panic!()
    };
    assert_eq!(context.source.gear_id, Some(new_id));
    assert_eq!(context.source.unit_id, None);
    resolve(&mut g);
    assert_eq!(g.players[0].xp, 1);
    let a = Action::Equip {
        gear_id: new_id,
        target: t,
    };
    let plan = g.cost_plan(&a).unwrap();
    assert_eq!((plan.xp, plan.energy, plan.power.len()), (1, 0, 0));
    let might = g.unit_might(u(&g, t)).unwrap();
    g.apply(Action::WithPowerPayment {
        action: Box::new(a),
        payment: PowerPayment::default(),
    })
    .unwrap();
    assert_eq!(g.players[0].xp, 0);
    assert_eq!(g.unit_might(u(&g, t)).unwrap(), might);
    resolve(&mut g);
    assert_eq!(g.unit_might(u(&g, t)).unwrap(), might + 2);
    assert!(g.chain.is_empty());
    assert_eq!(
        g.players[0].xp, 0,
        "Equip does not create a fresh GearPlayed event"
    );
    assert!(g.gained_xp_this_turn(0));
}

#[test]
fn heirloom_staged_equip_pays_once_and_shortage_can_be_cancelled() {
    for xp in [0, 1] {
        let mut g = game();
        no_resources(&mut g, 0);
        g.players[0].xp = xp;
        let t = unit(&mut g, 0, 1000, 69, None);
        gear(&mut g, 2000, 348);
        let before = format!("{g:?}");
        begin(
            &mut g,
            Action::Equip {
                gear_id: 2000,
                target: t,
            },
        );
        assert_eq!(g.payment_plan_view(0).unwrap().xp, 1);
        assert_eq!(g.players[0].xp, xp);
        if xp == 0 {
            rejected(&mut g, Action::CompletePayment);
            g.apply(Action::Choose { accept: false }).unwrap();
            assert_eq!(format!("{g:?}"), before);
        } else {
            g.apply(Action::CompletePayment).unwrap();
            assert_eq!(g.players[0].xp, 0);
            resolve(&mut g);
            assert_eq!(g.players[0].gear[0].attached_to, Some(1000));
            assert_eq!(g.players[0].xp, 0);
        }
    }
}

#[test]
fn heirloom_invalidated_equip_keeps_cost_and_fires_completed_use_observer() {
    let mut g = game();
    g.players[0].xp = 1;
    let t = unit(&mut g, 0, 1000, 69, None);
    let prize = unit(&mut g, 0, 1002, 215, None);
    let enemy = unit(&mut g, 1, 1001, 69, None);
    gear(&mut g, 2000, 348);
    g.apply(Action::Equip {
        gear_id: 2000,
        target: t,
    })
    .unwrap();
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 16, vec![enemy, t]);
    resolve(&mut g);
    assert_eq!(g.players[0].xp, 0);
    settle(&mut g);
    assert_eq!(g.players[0].gear[0].attached_to, None);
    assert_eq!(u(&g, prize).temporary_buff, 1);
}

#[test]
fn finalized_rules_trigger_survives_attachment_or_source_departure() {
    // Controlled response-state fixtures: ordinary Equip is not a Reaction.
    // The trigger is finalized using actual completed gear play first.
    for attached in [false, true] {
        let mut g = game();
        unit(&mut g, 0, 1000, 69, None);
        cast(&mut g, 348, vec![]);
        if attached {
            g.players[0].gear[0].attached_to = Some(1000);
        } else {
            g.players[0].gear.clear();
        }
        resolve(&mut g);
        assert_eq!(g.players[0].xp, 1);
    }
}

#[test]
fn unattached_equipment_observers_do_not_gain_bearer_events() {
    let mut g = game();
    g.battlefields[0] = 3;
    let t = unit(&mut g, 0, 1000, 69, None);
    gear(&mut g, 2000, 201); // Cull has an equipment-panel Conquer trigger
    gear(&mut g, 2001, 329); // Boneshiver likewise
    g.apply(Action::Move {
        unit_id: 1000,
        lane: Some(0),
    })
    .unwrap();
    settle(&mut g);
    assert!(!g.players[0].gear.iter().any(|x| x.token));
    assert_eq!(g.players[0].runes.iter().sum::<u8>(), 0);
    assert_eq!(u(&g, t).lane, Some(0));
}

#[test]
fn total_energy_discounts_do_not_erase_xp_costs() {
    let mut g = game();
    g.players[0].xp = 1;
    g.battlefields = [8, 8]; // two controlled Piltovan cost passives
    g.battlefield_owner = [Some(0), Some(0)];
    let t = unit(&mut g, 0, 1000, 69, None);
    gear(&mut g, 2000, 348);
    let a = Action::Equip {
        gear_id: 2000,
        target: t,
    };
    let plan = g.cost_plan(&a).unwrap();
    assert_eq!(plan.xp, 1);
    assert_eq!(plan.energy, 0);
    g.apply(a).unwrap();
    assert_eq!(g.players[0].xp, 0);
}

#[test]
fn failed_card_payment_undo_restores_both_mixed_outputs_and_history() {
    let mut g = game();
    no_resources(&mut g, 0);
    g.players[0].xp = 6;
    let honey = gear(&mut g, 2000, 347);
    let enemy = unit(&mut g, 1, 1000, 69, Some(0));
    let a = draft(&mut g, 227, vec![enemy]);
    let before = format!("{g:?}");
    begin(&mut g, a);
    g.apply(ability(honey, 1)).unwrap();
    assert_eq!((g.players[0].energy, g.players[0].any_power), (1, 1));
    rejected(&mut g, Action::CompletePayment);
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn ai_does_not_trade_a_level_survivor_for_one_buff() {
    let mut g = game();
    no_resources(&mut g, 0);
    g.players[0].xp = 3;
    unit(&mut g, 0, 1000, 345, None);
    unit(&mut g, 0, 1001, 339, None);
    for u in &mut g.players[0].units {
        u.ready = false;
    }
    g.players[0].units[1].damage = 3;
    g.step_ai().unwrap();
    assert_eq!(g.players[0].xp, 3);
    assert!(g.players[0].units.iter().any(|u| u.id == 1001));
}

#[test]
fn fixed_xp_payment_defers_level_loss_cleanup_until_finalization() {
    let mut g = game();
    g.players[0].xp = 3;
    let source_unit = unit(&mut g, 0, 1000, 345, None);
    unit(&mut g, 0, 1001, 339, None);
    g.players[0].units[1].damage = 3;
    let honey = gear(&mut g, 2000, 347);
    begin(&mut g, ability(source(source_unit), 0));
    assert_eq!(
        g.players[0].xp, 3,
        "Recording creates an obligation without spending"
    );
    g.apply(ability(honey, 0)).unwrap();
    assert!(g.players[0].units.iter().any(|u| u.id == 1001));
    assert_eq!(g.players[0].xp, 3);
    g.apply(Action::CompletePayment).unwrap();
    assert_eq!(g.players[0].xp, 1);
    assert!(!g.players[0].units.iter().any(|u| u.id == 1001));
    assert_eq!(u(&g, source_unit).buff, 0);
    assert!(g.chain.iter().any(|i| matches!(
        i.ability,
        Some(ChainAbility::Activated { card_id: 345, .. })
    )));
    settle(&mut g);
    assert_eq!(u(&g, source_unit).buff, 1);
}
