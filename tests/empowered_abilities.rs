//! Empower costs and live activation instructions through public engine actions.
use local_rift::activated::{AbilitySource, AbilityTiming, activated_rules};
use local_rift::game::{Action, Decision, Difficulty, Game, Target, Unit};
use local_rift::gear::GearState;

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

fn gear_target(source: AbilitySource) -> Target {
    match source {
        AbilitySource::Gear { player, gear_id } => Target::Gear { player, gear_id },
        _ => panic!("expected gear"),
    }
}
fn empower_with_formula(g: &mut Game, target: AbilitySource) {
    let id = 20_000 + g.players[0].gear.len() as u32;
    let formula = gear(g, id, 243);
    g.apply(ability(formula, 0, vec![gear_target(target)]))
        .unwrap();
    resolve(g);
}
fn next_own_turn(g: &mut Game) {
    assert!(g.chain.is_empty());
    g.apply(Action::EndTurn).unwrap();
    g.apply(Action::EndTurn).unwrap();
    assert_eq!(g.priority(), 0);
}
fn remove_gear_resolution_fixture(g: &mut Game, target: AbilitySource) {
    // No currently included Reaction removes an arbitrary base gear. This
    // explicit resolution-state fixture models source departure, not a spell.
    let AbilitySource::Gear { player, gear_id } = target else {
        panic!()
    };
    let index = g.players[player]
        .gear
        .iter()
        .position(|gear| gear.id == gear_id)
        .unwrap();
    let gear = g.players[player].gear.remove(index);
    g.players[player].discard.push(gear.card_id);
}

#[test]
fn eight_descriptors_distinguish_costs_timing_and_targetless_self_empower() {
    for id in 246..254 {
        assert_eq!(activated_rules(id).len(), if id >= 250 { 2 } else { 1 });
    }
    for id in 250..254 {
        let rule = &activated_rules(id)[0];
        assert!(rule.only_if_not_empowered);
        assert!(!rule.disempower_source);
        assert_eq!(rule.targets.max, 0);
        assert_eq!(rule.timing, AbilityTiming::Main);
        assert_eq!(
            rule.energy,
            match id {
                252 => 1,
                253 => 2,
                _ => 0,
            }
        );
        assert_eq!(rule.exhaust, id != 253);
    }
    assert!(activated_rules(250)[1].disempower_source);
    assert!(activated_rules(251)[1].disempower_source);
    assert!(activated_rules(252)[1].adds_resources());
    assert_eq!(activated_rules(252)[1].timing, AbilityTiming::Reaction);
    assert!(!activated_rules(252)[0].adds_resources());
    for id in 230..246 {
        assert!(
            activated_rules(id)
                .iter()
                .all(|r| !r.only_if_not_empowered && !r.disempower_source)
        );
    }
}

#[test]
fn four_self_empower_abilities_pay_exact_costs_then_resolve_without_targets() {
    for id in 250..254 {
        let mut g = position();
        let source = gear(&mut g, 900, id);
        let target = gear_target(source);
        rejected(&mut g, ability(source, 0, vec![target]));
        let energy = g.players[0].energy;
        g.apply(ability(source, 0, vec![])).unwrap();
        assert!(!g.unit_empowered(900), "Empower is an effect, not its cost");
        assert_eq!(
            g.players[0].energy,
            energy - u16::from(activated_rules(id)[0].energy)
        );
        assert_eq!(g.players[0].gear[0].ready, id == 253);
        assert!(g.chain[0].targets.is_empty());
        assert!(g.decision.is_none());
        resolve(&mut g);
        assert!(g.unit_empowered(900));
        rejected(&mut g, ability(source, 0, vec![]));
    }
}

#[test]
fn self_empower_completion_not_activation_triggers_prize_and_source_departure_is_noop() {
    for removed in [false, true] {
        let mut g = position();
        let source = gear(&mut g, 900, 253);
        unit(&mut g, 0, 901, 215, None);
        g.apply(ability(source, 0, vec![])).unwrap();
        assert_eq!(g.chain.len(), 1);
        assert_eq!(g.players[0].units[0].temporary_buff, 0);
        if removed {
            remove_gear_resolution_fixture(&mut g, source);
            assert!(!g.players[0].gear.iter().any(|gear| gear.id == 900));
            // A later copy is a different object; it must not receive the effect.
            gear(&mut g, 902, 253);
        }
        resolve(&mut g);
        assert_eq!(g.unit_empowered(900), !removed);
        assert!(!g.unit_empowered(902));
        assert_eq!(g.chain.last().unwrap().card_id, 215);
        resolve(&mut g);
        assert_eq!(g.players[0].units[0].temporary_buff, 1);
    }
}

#[test]
fn resolving_self_empower_on_an_already_empowered_source_is_a_completed_noop() {
    let mut g = position();
    let source = gear(&mut g, 900, 253);
    unit(&mut g, 0, 901, 215, None);
    g.apply(ability(source, 0, vec![])).unwrap();
    // Resolution-state fixture: the current included pool has no reaction
    // that Empowers an arbitrary base gear. Do not invent such a card.
    g.empowered_units.push(900);
    resolve(&mut g);
    assert_eq!(g.empowered_units.iter().filter(|&&id| id == 900).count(), 1);
    assert_eq!(g.chain.last().unwrap().card_id, 215);
}

#[test]
fn disempower_draw_and_mech_costs_are_paid_before_response_and_do_not_need_source_liveness() {
    for id in [250, 251] {
        for removed in [false, true] {
            let mut g = position();
            let source = gear(&mut g, 900, id);
            rejected(&mut g, ability(source, 1, vec![]));
            empower_with_formula(&mut g, source);
            let energy = g.players[0].energy;
            let hand = g.players[0].hand.len();
            g.apply(ability(source, 1, vec![])).unwrap();
            assert!(!g.unit_empowered(900));
            assert!(!g.players[0].gear[0].ready);
            assert_eq!(g.players[0].energy, energy - 1);
            assert_eq!(g.players[0].hand.len(), hand);
            assert!(g.players[0].units.is_empty());
            if removed {
                remove_gear_resolution_fixture(&mut g, source);
            }
            resolve(&mut g);
            if id == 250 {
                assert_eq!(g.players[0].hand.len(), hand + 1);
            } else {
                let token = g.players[0].units.iter().find(|u| u.token).unwrap();
                assert_eq!(g.unit_might(token).unwrap(), 3);
                assert_eq!(token.lane, None);
                assert_eq!(
                    local_rift::rules::token_spec(token.card_id).unwrap().tag,
                    Some("Mech")
                );
            }
            assert!(!g.unit_empowered(900));
        }
    }
}

#[test]
fn disempower_payment_recording_does_not_consume_status_and_undo_restores_exact_state() {
    let mut g = position();
    let source = gear(&mut g, 900, 250);
    empower_with_formula(&mut g, source);
    g.players[0].energy = 0;
    g.players[0].energy_pool = 0;
    let before = format!("{g:?}");
    g.apply(Action::BeginPayment {
        action: Box::new(ability(source, 1, vec![])),
    })
    .unwrap();
    assert!(g.unit_empowered(900));
    assert!(g.players[0].gear[0].ready);
    rejected(&mut g, Action::CompletePayment);
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(format!("{g:?}"), before);
    let sage = AbilitySource::Unit {
        player: 0,
        unit_id: 901,
    };
    unit(&mut g, 0, 901, 236, None);
    g.apply(Action::BeginPayment {
        action: Box::new(ability(source, 1, vec![])),
    })
    .unwrap();
    g.apply(ability(sage, 0, vec![])).unwrap();
    g.apply(Action::CompletePayment).unwrap();
    assert!(!g.unit_empowered(900));
    assert!(!g.players[0].gear[0].ready);
    assert_eq!(g.players[0].energy, 0);
    resolve(&mut g);
}

#[test]
fn egg_enters_exhausted_and_its_ability_reaction_does_not_speed_up_card_play() {
    let mut g = position();
    cast(&mut g, 252, vec![]);
    let egg = g.players[0].gear[0].id;
    assert!(!g.players[0].gear[0].ready);
    let src = AbilitySource::Gear {
        player: 0,
        gear_id: egg,
    };
    rejected(&mut g, ability(src, 1, vec![]));
    next_own_turn(&mut g);
    assert!(g.players[0].gear[0].ready);
    let target = unit(&mut g, 0, 901, 72, None);
    cast(&mut g, 7, vec![target]);
    let p = g.priority();
    g.players[p].hand.push(252);
    let hand_index = g.players[p].hand.len() - 1;
    rejected(
        &mut g,
        Action::PlayCardWithTargets {
            hand_index,
            lane: None,
            targets: vec![],
        },
    );
    let before = g.players[0].energy;
    let chain = g.chain.clone();
    g.apply(ability(src, 1, vec![])).unwrap();
    assert_eq!(g.players[0].energy, before + 1);
    assert_eq!(g.chain, chain);
    assert_eq!(g.priority(), 0);
}

#[test]
fn egg_adds_one_or_two_not_both_and_restores_nested_conversion_payment() {
    for empowered in [false, true] {
        let mut g = position();
        let egg = gear(&mut g, 900, 252);
        let henge = gear(&mut g, 901, 244);
        if empowered {
            empower_with_formula(&mut g, egg);
        }
        g.players[0].energy_pool = 0;
        g.players[0].energy = 0;
        g.apply(ability(henge, 0, vec![])).unwrap();
        let outer = g.decision.clone();
        g.apply(ability(egg, 1, vec![])).unwrap();
        let amount = if empowered { 2 } else { 1 };
        assert_eq!(g.players[0].energy, amount);
        assert_eq!(g.decision, outer);
        assert!(matches!(
            g.decision,
            Some(Decision::ResourceConversion { .. })
        ));
        g.apply(Action::ResolveConversion { amount }).unwrap();
        assert_eq!(g.players[0].any_power, amount);
        assert_eq!(g.players[0].energy, 0);
        assert!(g.decision.is_none());
        assert!(!g.players[0].gear[0].ready);
    }
}

#[test]
fn eggs_own_add_cannot_pay_the_exhaust_cost_of_its_self_empower() {
    let mut g = position();
    let egg = gear(&mut g, 900, 252);
    g.players[0].energy_pool = 0;
    g.players[0].energy = 0;
    let before = format!("{g:?}");
    g.apply(Action::BeginPayment {
        action: Box::new(ability(egg, 0, vec![])),
    })
    .unwrap();
    rejected(&mut g, ability(egg, 0, vec![]));
    g.apply(ability(egg, 1, vec![])).unwrap();
    assert_eq!(g.players[0].energy, 1);
    rejected(&mut g, Action::CompletePayment);
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn tools_uses_live_empowered_status_and_no_last_known_bonus_after_source_dies() {
    for empowered in [false, true] {
        let mut g = position();
        let tools = gear(&mut g, 900, 253);
        let target = unit(&mut g, 0, 901, 72, None);
        if empowered {
            empower_with_formula(&mut g, tools);
        }
        g.apply(ability(tools, 1, vec![target])).unwrap();
        resolve(&mut g);
        assert_eq!(
            g.players[0].units[0].temporary_buff,
            if empowered { 4 } else { 2 }
        );
    }
    let mut g = position();
    let tools = gear(&mut g, 900, 253);
    let target = unit(&mut g, 0, 901, 72, None);
    empower_with_formula(&mut g, tools);
    g.apply(ability(tools, 1, vec![target])).unwrap();
    remove_gear_resolution_fixture(&mut g, tools);
    assert!(
        g.unit_empowered(900),
        "fixture retains old status to test liveness, not just cleanup"
    );
    resolve(&mut g);
    assert_eq!(g.players[0].units[0].temporary_buff, 2);
}

#[test]
fn tools_checks_empowered_on_resolution_not_finalization() {
    let mut g = position();
    let tools = gear(&mut g, 900, 253);
    let target = unit(&mut g, 0, 901, 72, None);
    g.apply(ability(tools, 1, vec![target])).unwrap();
    // A direct resolution-state fixture; no unsupported reaction is invented.
    g.empowered_units.push(900);
    resolve(&mut g);
    assert_eq!(g.players[0].units[0].temporary_buff, 4);
}

#[test]
fn avian_buffs_unbuffed_units_or_adds_temporary_might_exclusively() {
    let mut g = position();
    let avian = source(unit(&mut g, 0, 900, 248, Some(0)));
    let target = unit(&mut g, 0, 901, 72, None);
    g.apply(ability(avian, 0, vec![target])).unwrap();
    resolve(&mut g);
    let target_unit = g.players[0].units.iter().find(|u| u.id == 901).unwrap();
    assert_eq!(target_unit.buff, 1);
    assert_eq!(target_unit.temporary_buff, 0);
    // Awakening is a real turn transition; the permanent Buff persists.
    next_own_turn(&mut g);
    g.apply(ability(avian, 0, vec![target])).unwrap();
    resolve(&mut g);
    let target_unit = g.players[0].units.iter().find(|u| u.id == 901).unwrap();
    assert_eq!(target_unit.buff, 1);
    assert_eq!(target_unit.temporary_buff, 2);
}

#[test]
fn avian_buff_has_the_activating_actor_and_temporary_might_is_not_another_buff() {
    let mut g = position();
    let avian = source(unit(&mut g, 0, 900, 248, Some(0)));
    let enemy = unit(&mut g, 1, 901, 158, None); // Simian: when YOU buff me, ready me.
    g.players[1].units[0].ready = false;
    g.apply(ability(avian, 0, vec![enemy])).unwrap();
    resolve(&mut g);
    assert_eq!(g.players[1].units[0].buff, 1);
    assert!(!g.players[1].units[0].ready);
    assert!(g.chain.is_empty());
}

#[test]
fn avian_can_choose_self_and_pays_enemy_deflect_without_counting_token_might_as_buff() {
    let mut g = position();
    let actor = unit(&mut g, 0, 900, 248, None);
    g.apply(ability(source(actor), 0, vec![actor])).unwrap();
    resolve(&mut g);
    assert_eq!(g.players[0].units[0].buff, 1);
    let other = source(unit(&mut g, 0, 901, 248, Some(0)));
    let target = unit(&mut g, 1, 902, 0, None);
    let before = g.players[0].power_pool;
    g.apply(ability(other, 0, vec![target])).unwrap();
    assert_eq!(
        before.iter().sum::<u16>() - g.players[0].power_pool.iter().sum::<u16>(),
        1
    );
    resolve(&mut g);
    assert_eq!(g.players[1].units[0].buff, 1);
    let third = source(unit(&mut g, 0, 903, 248, Some(0)));
    let token = unit(&mut g, 0, 904, 249, None);
    let t = g.players[0].units.iter_mut().find(|u| u.id == 904).unwrap();
    t.token = true;
    t.buff = 1; // Token's printed1 Might, not a Buff counter.
    g.apply(ability(third, 0, vec![token])).unwrap();
    resolve(&mut g);
    let t = g.players[0].units.iter().find(|u| u.id == 904).unwrap();
    assert_eq!(g.unit_might(t).unwrap(), 2);
    assert_eq!(t.temporary_buff, 0);
    assert!(g.buffed_units.contains(&904));
}

#[test]
fn ultrasoft_restricts_activation_not_later_effect_and_birds_finalize_after_parent() {
    let mut g = position();
    let poro = unit(&mut g, 0, 900, 249, None);
    rejected(&mut g, ability(source(poro), 0, vec![]));
    g.apply(Action::Move {
        unit_id: 900,
        lane: Some(0),
    })
    .unwrap();
    while g.showdown.is_some() {
        g.apply(Action::Pass).unwrap();
    }
    // Standard movement exhausted it; First Mate readies it in place.
    cast(&mut g, 23, vec![]);
    g.apply(Action::ChooseTarget { target: Some(poro) })
        .unwrap();
    resolve(&mut g);
    g.apply(ability(source(poro), 0, vec![])).unwrap();
    cast(&mut g, 9, vec![poro]); // Flash is a real owner Reaction, moving the source to base.
    resolve(&mut g);
    assert_eq!(
        g.players[0]
            .units
            .iter()
            .find(|u| u.id == 900)
            .unwrap()
            .lane,
        None
    );
    resolve(&mut g);
    assert!(g.chain.is_empty());
    assert!(matches!(
        g.decision,
        Some(Decision::TokenPlacement { remaining: 2, .. })
    ));
    assert!(g.players[0].units.iter().all(|u| !u.token));
    g.apply(Action::PlaceToken { lane: None }).unwrap();
    assert_eq!(g.players[0].units.iter().filter(|u| u.token).count(), 1);
    g.apply(Action::PlaceToken { lane: None }).unwrap();
    let birds: Vec<_> = g.players[0].units.iter().filter(|u| u.token).collect();
    assert_eq!(birds.len(), 2);
    for bird in birds {
        assert_eq!(g.unit_might(bird).unwrap(), 1);
        assert_eq!(g.unit_keywords(bird).unwrap().deflect, 1);
        assert_eq!(
            local_rift::rules::token_spec(bird.card_id).unwrap().tag,
            Some("Bird")
        );
    }
}

#[test]
fn syren_moves_friendly_field_unit_without_readying_or_recall_and_rechecks_target() {
    let mut g = position();
    let syren = gear(&mut g, 900, 247);
    let own_base = unit(&mut g, 0, 901, 72, None);
    let enemy = unit(&mut g, 1, 902, 72, Some(1));
    rejected(&mut g, ability(syren, 0, vec![own_base]));
    rejected(&mut g, ability(syren, 0, vec![enemy]));
    let moving = unit(&mut g, 0, 903, 72, Some(0));
    g.players[0]
        .units
        .iter_mut()
        .find(|u| u.id == 903)
        .unwrap()
        .ready = false;
    let energy = g.players[0].energy;
    g.apply(ability(syren, 0, vec![moving])).unwrap();
    resolve(&mut g);
    let moved = g.players[0].units.iter().find(|u| u.id == 903).unwrap();
    assert_eq!(moved.lane, None);
    assert!(!moved.ready);
    assert_eq!(g.players[0].energy, energy - 1);

    let second = gear(&mut g, 904, 247);
    let moving = unit(&mut g, 0, 905, 72, Some(0));
    g.apply(ability(second, 0, vec![moving])).unwrap();
    cast(&mut g, 9, vec![moving]);
    resolve(&mut g);
    resolve(&mut g);
    assert_eq!(
        g.players[0]
            .units
            .iter()
            .find(|u| u.id == 905)
            .unwrap()
            .lane,
        None
    );
}

#[test]
fn poro_snax_new_copy_draw_and_sacrificed_activation_have_separate_windows() {
    let mut g = position();
    gear(&mut g, 900, 246);
    cast(&mut g, 246, vec![]);
    assert_eq!(g.chain.len(), 1, "only entering copy triggers");
    assert_eq!(g.chain[0].card_id, 246);
    let hand = g.players[0].hand.len();
    resolve(&mut g);
    assert_eq!(g.players[0].hand.len(), hand + 1);
    let source = AbilitySource::Gear {
        player: 0,
        gear_id: 900,
    };
    let energy = g.players[0].energy;
    let calm = g.players[0].power_pool[1];
    g.apply(ability(source, 0, vec![])).unwrap();
    assert!(!g.players[0].gear.iter().any(|gear| gear.id == 900));
    assert!(g.players[0].discard.contains(&246));
    assert_eq!(g.players[0].energy, energy - 1);
    assert_eq!(g.players[0].power_pool[1], calm - 1);
    assert_eq!(g.players[0].hand.len(), hand + 1);
    resolve(&mut g);
    assert_eq!(g.players[0].hand.len(), hand + 2);
}

#[test]
fn poro_snax_burnout_can_recycle_its_already_sacrificed_source() {
    let mut g = position();
    let snax = gear(&mut g, 900, 246);
    g.players[0].deck.clear();
    g.players[0].discard.clear();
    g.apply(ability(snax, 0, vec![])).unwrap();
    assert_eq!(g.players[0].discard, vec![246]);
    resolve(&mut g);
    assert_eq!(g.players[0].hand, vec![246]);
    assert_eq!(g.players[1].score, 1);
}

#[test]
fn syren_performs_a_real_move_event_and_respects_vexs_movement_lock() {
    let mut g = position();
    let syren = gear(&mut g, 900, 247);
    let herder = unit(&mut g, 0, 901, 17, Some(0));
    g.players[0].units[0].ready = false;
    let hand = g.players[0].hand.len();
    g.apply(ability(syren, 0, vec![herder])).unwrap();
    resolve(&mut g);
    assert_eq!(g.players[0].units[0].lane, None);
    assert!(!g.players[0].units[0].ready);
    assert_eq!(g.players[0].hand.len(), hand);
    assert_eq!(g.chain.last().unwrap().card_id, 17);
    resolve(&mut g);
    assert_eq!(g.players[0].hand.len(), hand + 1);

    let mut g = position();
    let syren = gear(&mut g, 900, 247);
    unit(&mut g, 0, 901, 72, Some(0));
    unit(&mut g, 1, 902, 20, Some(1));
    g.players[0].hand.push(69);
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: Some(0),
        targets: vec![],
    })
    .unwrap();
    let target = g.players[0]
        .units
        .iter()
        .find(|u| u.card_id == 69)
        .unwrap()
        .id;
    resolve(&mut g); // Actual Vex trigger, no fixture-only movement lock.
    assert!(g.movement_locked(0, target));
    g.apply(ability(
        syren,
        0,
        vec![Target::Unit {
            player: 0,
            unit_id: target,
        }],
    ))
    .unwrap();
    resolve(&mut g);
    assert_eq!(
        g.players[0]
            .units
            .iter()
            .find(|u| u.id == target)
            .unwrap()
            .lane,
        Some(0)
    );
    assert!(
        !g.players[0].gear[0].ready,
        "a resolved blocked move does not refund Exhaust"
    );
}

#[test]
fn syren_ai_returns_an_already_scored_ready_unit_to_claim_the_winning_other_field() {
    for difficulty in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        let mut g = position();
        g.difficulty = difficulty;
        let syren = gear(&mut g, 900, 247);
        unit(&mut g, 0, 901, 72, Some(0));
        g.players[0].score = 7;
        g.player_scored_this_turn[0][0] = true;
        g.scored_this_turn[0] = true;
        g.step_ai().unwrap();
        assert!(
            matches!(g.chain.last().and_then(|item| item.ability.as_ref()),
            Some(local_rift::game::ChainAbility::Activated { source, card_id: 247, .. }) if *source == syren),
            "{difficulty:?}: {:?}",
            g.chain
        );
        for _ in 0..10 {
            if g.winner.is_some() {
                break;
            }
            g.step_ai().unwrap();
        }
        assert_eq!(g.winner, Some(0), "{difficulty:?}");
    }
}

#[test]
fn snax_rejected_resource_payment_does_not_sacrifice_or_exhaust_it() {
    let mut g = position();
    let snax = gear(&mut g, 900, 246);
    g.players[0].power_pool = [0; 6];
    rejected(&mut g, ability(snax, 0, vec![]));
    assert!(g.players[0].gear[0].ready);
    assert!(g.players[0].discard.is_empty());
}

#[test]
fn avians_countered_ability_refunds_no_exhaust_and_buffs_nothing() {
    let mut g = position();
    let avian = source(unit(&mut g, 0, 900, 248, Some(0)));
    let target = unit(&mut g, 1, 901, 72, None);
    g.apply(ability(avian, 0, vec![target])).unwrap();
    let chain_id = g.chain.last().unwrap().id;
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 12, vec![Target::Spell { chain_id }]); // Not So Fast protects its friendly chosen unit.
    resolve(&mut g);
    assert!(g.chain.is_empty());
    assert_eq!(g.players[1].units[0].buff, 0);
    assert_eq!(g.players[1].units[0].temporary_buff, 0);
    assert!(!g.players[0].units[0].ready);
}

#[test]
fn syren_ai_preserves_an_unscored_lone_holder_and_does_not_predict_an_illegal_final_conquest() {
    let mut g = position();
    gear(&mut g, 900, 247);
    unit(&mut g, 0, 901, 72, Some(0));
    g.players[0].score = 7;
    g.step_ai().unwrap();
    assert!(
        g.players[0].gear[0].ready,
        "do not abandon the lone unscored holder"
    );
    assert_eq!(g.players[0].units[0].lane, Some(0));

    let mut g = position();
    gear(&mut g, 900, 247);
    unit(&mut g, 0, 901, 72, Some(0));
    unit(&mut g, 0, 902, 70, Some(0));
    g.players[0].score = 7;
    for _ in 0..10 {
        if g.player_scored_this_turn[0][1] {
            break;
        }
        g.step_ai().unwrap();
    }
    assert!(g.player_scored_this_turn[0][1]);
    assert!(!g.player_scored_this_turn[0][0]);
    assert_eq!(g.battlefield_owner, [Some(0), Some(0)]);
    assert_eq!(
        g.players[0].score, 7,
        "other field was not scored this turn: draw replaces final conquest point"
    );
    assert_eq!(g.winner, None);
}
