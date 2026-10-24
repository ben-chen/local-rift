//! Object costs and nested Add conversion through public engine actions.
use local_rift::activated::{AbilitySource, ConversionDirection, activated_rules};
use local_rift::game::{Action, Decision, Difficulty, Game, Target, Unit};
use local_rift::gear::GearState;
use local_rift::payment::PowerPayment;

fn position() -> Game {
    let mut game = Game::new_for_agents(3, 3, 3772, Difficulty::Tactical);
    game.start_match().unwrap();
    game.battlefields = [0, 0];
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
fn source(target: Target) -> AbilitySource {
    let Target::Unit { player, unit_id } = target else {
        panic!()
    };
    AbilitySource::Unit { player, unit_id }
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
fn rejected(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action).is_err());
    assert_eq!(format!("{g:?}"), before);
}

fn costs(source: AbilitySource, targets: Vec<Target>, trash_indices: Vec<usize>) -> Action {
    Action::ActivateAbilityWithCosts {
        source,
        ability_index: 0,
        targets,
        trash_indices,
    }
}
fn clear_resources(g: &mut Game) {
    for p in &mut g.players {
        p.energy = 0;
        p.energy_pool = 0;
        p.power_pool = [0; 6];
        p.any_power = 0;
    }
}

#[test]
fn grabber_requires_exact_distinct_trash_cost_and_recycles_before_responses() {
    let mut g = position();
    let source = gear(&mut g, 900, 240);
    g.players[0].discard = vec![69, 70, 25, 26];
    assert_eq!(g.ability_cost_candidates(source, 0), vec![0, 1, 2, 3]);
    rejected(&mut g, costs(source, vec![], vec![0, 0, 1]));
    rejected(&mut g, costs(source, vec![], vec![0, 1]));
    rejected(&mut g, costs(source, vec![], vec![0, 1, 99]));
    let deck = g.players[0].deck.len();
    let hand = g.players[0].hand.len();
    g.apply(costs(source, vec![], vec![0, 2, 3])).unwrap();
    assert_eq!(g.players[0].discard, vec![70]);
    assert_eq!(g.players[0].deck.len(), deck + 3);
    assert_eq!(g.players[0].energy, 19);
    assert!(!g.players[0].gear[0].ready);
    assert_eq!(g.players[0].hand.len(), hand);
    resolve(&mut g);
    assert_eq!(g.players[0].hand.len(), hand + 1);
}
#[test]
fn assembly_recycles_only_a_unit_and_pays_both_resources_before_mech_entry() {
    let mut g = position();
    let source = gear(&mut g, 901, 241);
    g.players[0].discard = vec![7, 69];
    assert_eq!(g.ability_cost_candidates(source, 0), vec![1]);
    rejected(&mut g, costs(source, vec![], vec![0]));
    g.apply(costs(source, vec![], vec![1])).unwrap();
    assert_eq!(g.players[0].energy, 19);
    assert_eq!(g.players[0].power_pool[0], 7);
    assert!(g.players[0].units.is_empty());
    resolve(&mut g);
    assert_eq!(g.players[0].units.len(), 1);
    let mech = &g.players[0].units[0];
    assert!(mech.token);
    assert_eq!(mech.lane, None);
    assert_eq!(g.unit_might(mech), 3);
}
#[test]
fn shells_vision_uses_the_entering_gear_identity_only() {
    let mut g = position();
    gear(&mut g, 902, 242);
    cast(&mut g, 242, vec![]);
    assert_eq!(
        g.chain.len(),
        1,
        "Only the newly played Shells triggers Vision"
    );
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::Predict { player: 0, .. })
    ));
    g.apply(Action::Choose { accept: false }).unwrap();
    assert!(g.chain.is_empty());
}
#[test]
fn shells_vision_on_an_empty_deck_does_not_burn_out() {
    let mut g = position();
    g.players[0].deck.clear();
    cast(&mut g, 242, vec![]);
    resolve(&mut g);
    assert!(g.decision.is_none());
    assert_eq!(g.players[1].score, 0);
}
#[test]
fn shells_sacrifices_itself_as_cost_and_its_fixed_effect_survives() {
    let mut g = position();
    let source = gear(&mut g, 903, 242);
    let target = unit(&mut g, 0, 904, 69, None);
    let might = g.unit_might(g.players[0].units.last().unwrap());
    g.apply(ability(source, 0, vec![target])).unwrap();
    assert!(g.players[0].gear.is_empty());
    assert!(g.players[0].discard.contains(&242));
    resolve(&mut g);
    assert_eq!(g.unit_might(&g.players[0].units[0]), might + 2);
}
#[test]
fn formula_enters_exhausted_and_can_empower_enemy_attached_gear_but_not_itself() {
    let mut g = position();
    cast(&mut g, 243, vec![]);
    let formula = g.players[0].gear[0].id;
    let source = AbilitySource::Gear {
        player: 0,
        gear_id: formula,
    };
    let own = Target::Gear {
        player: 0,
        gear_id: formula,
    };
    rejected(&mut g, ability(source, 0, vec![own]));
    g.players[0].gear[0].ready = true;
    rejected(&mut g, ability(source, 0, vec![own]));
    unit(&mut g, 1, 905, 69, None);
    g.players[1].gear.push(GearState {
        id: 906,
        card_id: 58,
        attached_to: Some(905),
        ready: false,
        temporary: false,
        token: false,
    });
    let enemy = Target::Gear {
        player: 1,
        gear_id: 906,
    };
    assert!(g.ability_targets(source, 0).contains(&enemy));
    g.apply(ability(source, 0, vec![enemy])).unwrap();
    resolve(&mut g);
    assert!(g.unit_empowered(906));
    assert!(!g.unit_empowered(905));
}
#[test]
fn formula_can_empower_gold_and_an_already_empowered_gear_is_a_legal_noop() {
    let mut g = position();
    let source = gear(&mut g, 907, 243);
    g.players[0].gear.push(GearState {
        id: 908,
        card_id: 13,
        attached_to: None,
        ready: true,
        temporary: false,
        token: true,
    });
    let target = Target::Gear {
        player: 0,
        gear_id: 908,
    };
    g.apply(ability(source, 0, vec![target])).unwrap();
    resolve(&mut g);
    assert!(g.unit_empowered(908));
    g.players[0]
        .gear
        .iter_mut()
        .find(|x| x.id == 907)
        .unwrap()
        .ready = true;
    g.apply(ability(source, 0, vec![target])).unwrap();
    resolve(&mut g);
    assert!(g.unit_empowered(908));
}
#[test]
fn henge_defers_only_conversion_payment_and_zero_preserves_exhaust() {
    let mut g = position();
    let source = gear(&mut g, 909, 244);
    clear_resources(&mut g);
    g.apply(ability(source, 0, vec![])).unwrap();
    assert!(!g.players[0].gear[0].ready);
    assert!(g.chain.is_empty());
    assert_eq!(g.conversion_available(), Some(0));
    assert!(matches!(
        g.decision,
        Some(Decision::ResourceConversion {
            direction: ConversionDirection::EnergyToPower,
            ..
        })
    ));
    rejected(&mut g, Action::ResolveConversion { amount: 1 });
    g.apply(Action::Choose { accept: false }).unwrap();
    assert!(g.decision.is_none());
    assert!(!g.players[0].gear[0].ready);
}
#[test]
fn henge_can_wait_for_dragonsoul_and_restores_the_original_payment() {
    let mut g = position();
    clear_resources(&mut g);
    let henge = gear(&mut g, 910, 244);
    let sage = source(unit(&mut g, 0, 911, 236, None));
    g.decision = Some(Decision::CounterTax {
        player: 0,
        chain_id: 999,
        cost: 2,
    });
    g.apply(ability(henge, 0, vec![])).unwrap();
    assert_eq!(g.resource_continuations()[0].task, "CounterTax");
    g.apply(ability(sage, 0, vec![])).unwrap();
    assert_eq!(g.conversion_available(), Some(1));
    g.apply(Action::ResolveConversion { amount: 1 }).unwrap();
    assert!(matches!(g.decision, Some(Decision::CounterTax { .. })));
    assert_eq!(g.players[0].any_power, 1);
    assert_eq!(g.players[0].energy, 0);
}
#[test]
fn nested_anomalies_complete_inside_out_and_preserve_the_outer_amount_choice() {
    let mut g = position();
    clear_resources(&mut g);
    g.players[0].any_power = 3;
    let outer = gear(&mut g, 912, 245);
    let inner = gear(&mut g, 913, 245);
    g.apply(ability(outer, 0, vec![])).unwrap();
    g.apply(ability(inner, 0, vec![])).unwrap();
    assert_eq!(g.resource_continuations().len(), 1);
    assert_eq!(g.resource_continuations()[0].task, "ResourceConversion");
    g.apply(Action::ResolveConversion { amount: 1 }).unwrap();
    assert_eq!(g.conversion_available(), Some(2));
    g.apply(Action::ResolveConversion { amount: 2 }).unwrap();
    assert!(g.decision.is_none());
    assert_eq!(g.players[0].energy, 3);
    assert_eq!(g.players[0].any_power, 0);
}
#[test]
fn anomaly_uses_exact_generic_power_and_excludes_spell_only_power() {
    let mut g = position();
    clear_resources(&mut g);
    let source = gear(&mut g, 914, 245);
    g.legend[0].spell_power = 7;
    g.players[0].power_pool[0] = 2;
    g.players[0].power_pool[2] = 3;
    g.apply(ability(source, 0, vec![])).unwrap();
    assert_eq!(g.conversion_available(), Some(5));
    let mut payment = PowerPayment::default();
    payment.colored[2] = 3;
    g.apply(Action::WithPowerPayment {
        action: Box::new(Action::ResolveConversion { amount: 3 }),
        payment,
    })
    .unwrap();
    assert_eq!(g.players[0].power_pool[0], 2);
    assert_eq!(g.players[0].power_pool[2], 0);
    assert_eq!(g.legend[0].spell_power, 7);
    assert_eq!(g.players[0].energy, 3);
}
#[test]
fn conversions_preserve_more_than_255_resources_without_clipping() {
    let mut g = position();
    clear_resources(&mut g);
    let henge = gear(&mut g, 915, 244);
    g.players[0].energy = 400;
    g.players[0].energy_pool = 400;
    g.apply(ability(henge, 0, vec![])).unwrap();
    assert_eq!(g.conversion_available(), Some(400));
    g.apply(Action::ResolveConversion { amount: 300 }).unwrap();
    assert_eq!(g.players[0].any_power, 300);
    assert_eq!(g.players[0].energy, 100);
    // Resource representation fixture: the two converters' domains do not
    // share a supported legend; legal-deck pairings are tested separately.
    let anomaly = gear(&mut g, 916, 245);
    g.apply(ability(anomaly, 0, vec![])).unwrap();
    let payment = PowerPayment {
        any: 300,
        ..Default::default()
    };
    g.apply(Action::WithPowerPayment {
        action: Box::new(Action::ResolveConversion { amount: 300 }),
        payment,
    })
    .unwrap();
    assert_eq!(g.players[0].energy, 400);
    assert_eq!(g.players[0].any_power, 0);
}
#[test]
fn conversion_metadata_is_explicit_and_invalid_sources_fail_safely() {
    let mut g = position();
    assert_eq!(
        activated_rules(244)[0].conversion_direction(),
        Some(ConversionDirection::EnergyToPower)
    );
    assert_eq!(
        activated_rules(245)[0].conversion_direction(),
        Some(ConversionDirection::PowerToEnergy)
    );
    rejected(
        &mut g,
        ability(
            AbilitySource::Gear {
                player: 99,
                gear_id: 1,
            },
            0,
            vec![],
        ),
    );
    assert!(
        g.ability_cost_candidates(
            AbilitySource::Gear {
                player: 99,
                gear_id: 1
            },
            0
        )
        .is_empty()
    );
}

#[test]
fn nested_conversion_completion_events_wait_and_keep_distinct_trigger_batches() {
    let mut g = position();
    clear_resources(&mut g);
    unit(&mut g, 0, 920, 215, None);
    unit(&mut g, 0, 921, 215, None);
    let outer = gear(&mut g, 922, 245);
    let inner = gear(&mut g, 923, 245);
    g.apply(ability(outer, 0, vec![])).unwrap();
    g.apply(ability(inner, 0, vec![])).unwrap();
    g.apply(Action::ResolveConversion { amount: 0 }).unwrap();
    assert!(g.chain.is_empty());
    assert!(matches!(
        g.decision,
        Some(Decision::ResourceConversion { .. })
    ));
    g.apply(Action::ResolveConversion { amount: 0 }).unwrap();
    let Some(Decision::OrderTriggers { options, .. }) = &g.decision else {
        panic!("{:?}", g.decision)
    };
    assert_eq!(
        options.len(),
        2,
        "The inner use's two simultaneous observers precede the outer use's separate event"
    );
}
