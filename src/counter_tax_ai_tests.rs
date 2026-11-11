//! Controlled actual-card positions; counter-tax creation and payments use actions.
use super::*;
use crate::activated::activated_rules;

fn pending_tax(actor: usize, legend: usize) -> Game {
    let ids = if actor == 0 { [legend, 3] } else { [3, legend] };
    let mut g = Game::new_with_custom(
        [0, 0],
        ids.map(|id| Some(crate::deck::CustomDeck::from_legend(id).unwrap())),
        533101,
        Difficulty::Tactical,
        false,
    )
    .unwrap();
    assert_eq!(g.legend_id(actor), legend);
    g.start_match().unwrap();
    g.active = actor;
    g.priority_player = actor;
    g.battlefield_owner = [None; 2];
    for p in &mut g.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy = 4;
        p.energy_pool = 4;
        p.power_pool = [0; 6];
        p.any_power = 0;
    }
    for l in &mut g.legend {
        l.ready = false;
    }
    let unit_id = g.create_unit(1 - actor, 71, false, 0);
    g.players[actor].hand.push(52); // Eclipse: 3 Energy, leaves one for a two-Energy tax.
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player: 1 - actor,
            unit_id,
        }],
    })
    .unwrap();
    let chain_id = g.chain.last().unwrap().id;
    g.apply(Action::Pass).unwrap(); // The caster retains priority until passing.
    g.players[1 - actor].hand.push(37); // Hard Bargain.
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Spell { chain_id }],
    })
    .unwrap();
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
    assert!(matches!(g.decision,Some(Decision::CounterTax{player,cost:2,..}) if player==actor));
    assert_eq!(g.players[actor].energy, 1);
    g
}
fn source(g: &mut Game, actor: usize, card: CardId, ready: bool) -> AbilitySource {
    match g.catalog[card].kind {
        CardKind::Unit => {
            let id = g.create_unit(actor, card, false, 0);
            g.players[actor].units.last_mut().unwrap().ready = ready;
            AbilitySource::Unit {
                player: actor,
                unit_id: id,
            }
        }
        CardKind::Gear => {
            let id = g.next_unit_id;
            g.next_unit_id += 1;
            g.players[actor].gear.push(GearState {
                id,
                card_id: card,
                lane: None,
                attached_to: None,
                ready,
                temporary: false,
                token: false,
            });
            AbilitySource::Gear {
                player: actor,
                gear_id: id,
            }
        }
        CardKind::Spell => panic!("not a resource permanent"),
    }
}
fn gold(g: &mut Game, actor: usize, count: usize) {
    for _ in 0..count {
        g.gear_gold(actor, false);
    }
}
fn pay_with_ai(g: &mut Game, actor: usize) {
    for _ in 0..20 {
        let action = g.choose_heuristic_action().unwrap();
        assert_ne!(
            action,
            Action::Choose { accept: false },
            "a feasible tax was declined"
        );
        g.apply(action).unwrap();
        if !matches!(
            g.decision,
            Some(Decision::CounterTax { .. } | Decision::ResourceConversion { .. })
        ) {
            assert!(g.chain.iter().any(|item| item.card_id == 52));
            return;
        }
        assert_eq!(g.priority(), actor);
    }
    panic!("feasible tax did not finish")
}

#[test]
fn power_only_tax_policy_is_pure_and_preserves_gold_runes_and_exact_engine_proofs() {
    for actor in 0..2 {
        let mut g = pending_tax(actor, 3);
        gold(&mut g, actor, 7);
        g.players[actor]
            .set_rune_setup([0, 0, 7, 0, 0, 0], [0, 0, 7, 0, 0, 0])
            .unwrap();
        source(&mut g, actor, 126, true); // Seal of Insight adds Power, not new Energy.
        let before = format!("{g:?}");
        assert!(g.ai_counter_tax_decline_without_add(actor).unwrap());
        assert_eq!(
            g.choose_heuristic_action().unwrap(),
            Action::Choose { accept: false }
        );
        assert_eq!(format!("{g:?}"), before);
        // The exact API remains exact; use a smaller equivalent resource board
        // to avoid reintroducing the intentionally pathological test workload.
        let mut small = pending_tax(actor, 3);
        gold(&mut small, actor, 1);
        assert!(
            small
                .resource_witness(actor, |s| crate::numeric::legal_result(
                    s.clone().apply(Action::Choose { accept: true })
                ))
                .unwrap()
                .is_none()
        );
        g.apply(Action::Choose { accept: false }).unwrap();
        assert_eq!(
            g.players[actor]
                .gear
                .iter()
                .filter(|x| x.is_gold() && x.ready)
                .count(),
            7
        );
        assert_eq!(g.players[actor].runes[2], 7);
        assert!(!g.chain.iter().any(|item| item.card_id == 52));
    }
}

#[test]
fn all_latent_energy_or_unknown_resource_descriptors_force_exact_fallback() {
    for actor in 0..2 {
        let g = pending_tax(actor, 3);
        let ids: Vec<_> = g
            .catalog
            .iter()
            .filter(|c| {
                activated_rules(c.id).iter().any(|r| {
                    r.adds_resources() && r.fixed_resource_output().is_none_or(|o| o.energy > 0)
                })
            })
            .map(|c| c.id)
            .collect();
        for expected in [236, 244, 245, 252, 347] {
            assert!(ids.contains(&expected));
        }
        for id in ids {
            let mut p = g.clone();
            source(&mut p, actor, id, false);
            p.players[actor].xp = 0; // Ignore exhaustion, XP and Empower restrictions conservatively.
            assert!(
                !p.ai_counter_tax_decline_without_add(actor).unwrap(),
                "{actor}/{id}"
            );
        }
        let mut p = g.clone();
        source(&mut p, actor, 130, false); // Legacy rune-conduit Energy.
        assert!(!p.ai_counter_tax_decline_without_add(actor).unwrap());
        let mut p = pending_tax(actor, 4);
        p.legend[actor].ready = false;
        assert!(!p.ai_counter_tax_decline_without_add(actor).unwrap());
    }
}

#[test]
fn actual_energy_unit_legacy_gear_empowered_egg_conversion_and_darius_taxes_remain_payable() {
    for actor in 0..2 {
        for card in [236, 130, 252, 245] {
            let mut g = pending_tax(actor, 3);
            let src = source(&mut g, actor, card, true);
            if card == 252 {
                g.empowered_units.push(match src {
                    AbilitySource::Gear { gear_id, .. } => gear_id,
                    _ => unreachable!(),
                });
            }
            if card == 245 {
                g.players[actor].any_power = 1;
            }
            assert!(!g.ai_counter_tax_decline_without_add(actor).unwrap());
            pay_with_ai(&mut g, actor);
        }
        let mut g = pending_tax(actor, 4);
        g.legend[actor].ready = true;
        pay_with_ai(&mut g, actor);
    }
}

#[test]
fn nested_conversion_and_frozen_payment_never_use_the_optional_tax_shortcut() {
    for actor in 0..2 {
        let mut g = pending_tax(actor, 3);
        let src = source(&mut g, actor, 245, true);
        g.players[actor].any_power = 1;
        g.apply(Action::ActivateAbility {
            source: src,
            ability_index: 0,
            targets: vec![],
        })
        .unwrap();
        assert!(matches!(
            g.decision,
            Some(Decision::ResourceConversion {
                direction: ConversionDirection::PowerToEnergy,
                ..
            })
        ));
        assert!(!g.resource_frames.is_empty());
        assert!(!g.ai_counter_tax_decline_without_add(actor).unwrap());
        let before = format!("{g:?}");
        assert!(g.apply(Action::RefusePayment).is_err());
        assert_eq!(format!("{g:?}"), before);
        g.apply(Action::ResolveConversion { amount: 1 }).unwrap();
        assert_eq!(g.players[actor].energy, 2);
        assert!(!g.ai_counter_tax_decline_without_add(actor).unwrap());
        g.apply(Action::Choose { accept: true }).unwrap();
        let mut g = pending_tax(actor, 3);
        g.apply(Action::BeginPayment {
            action: Box::new(Action::Choose { accept: true }),
        })
        .unwrap();
        assert!(g.payment_plan.is_some());
        assert!(!g.ai_counter_tax_decline_without_add(actor).unwrap());
    }
}

#[test]
fn queued_gear_observers_do_not_run_while_the_counter_tax_remains_pending() {
    for actor in 0..2 {
        let mut g = pending_tax(actor, 3);
        let src = source(&mut g, actor, 215, true);
        gold(&mut g, actor, 1);
        let AbilitySource::Unit { unit_id, .. } = src else {
            panic!("Prize is a unit")
        };
        let before = g
            .unit_might(
                g.players[actor]
                    .units
                    .iter()
                    .find(|u| u.id == unit_id)
                    .unwrap(),
            )
            .unwrap();
        let id = g.players[actor]
            .gear
            .iter()
            .find(|x| x.is_gold())
            .unwrap()
            .id;
        g.apply(Action::ActivateGear { gear_id: id }).unwrap();
        assert!(matches!(g.decision, Some(Decision::CounterTax { .. })));
        assert_eq!(
            g.unit_might(
                g.players[actor]
                    .units
                    .iter()
                    .find(|u| u.id == unit_id)
                    .unwrap()
            )
            .unwrap(),
            before
        );
        assert!(g.ai_counter_tax_decline_without_add(actor).unwrap());
    }
}

#[test]
fn capacity_errors_from_actual_state_actions_and_fallback_search_are_preserved() {
    for actor in 0..2 {
        let mut g = pending_tax(actor, 3);
        let unit = &mut g.players[1 - actor].units[0];
        unit.buff = i64::MAX as u64;
        let before = format!("{g:?}");
        assert!(
            g.ai_counter_tax_decline_without_add(actor)
                .unwrap_err()
                .starts_with("engine_capacity:")
        );
        assert!(
            g.apply(Action::Choose { accept: false })
                .unwrap_err()
                .starts_with("engine_capacity:")
        );
        assert_eq!(format!("{g:?}"), before);
        let mut g = pending_tax(actor, 3);
        let src = source(&mut g, actor, 347, true);
        g.players[actor].any_power = u16::MAX;
        g.players[actor].xp = 6;
        assert!(!g.ai_counter_tax_decline_without_add(actor).unwrap());
        let before = format!("{g:?}");
        assert!(
            g.apply(Action::ActivateAbility {
                source: src,
                ability_index: 1,
                targets: vec![]
            })
            .unwrap_err()
            .starts_with("engine_capacity:")
        );
        assert_eq!(format!("{g:?}"), before);
        assert!(
            g.resource_witness(actor, |s| crate::numeric::legal_result(
                s.clone().apply(Action::Choose { accept: true })
            ))
            .unwrap_err()
            .starts_with("engine_capacity:")
        );
    }
}
