//! Printed C1 abilities through real public actions and response windows.
use local_rift::activated::{AbilitySource, activated_rules};
use local_rift::game::{Action, ChainAbility, Decision, Difficulty, Game, Phase, Target, Unit};
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
fn finalize() -> Action {
    Action::FinalizePendingPlay {
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate: false,
        bonus: false,
    }
}

#[test]
fn every_printed_descriptor_has_expected_distinct_source_options() {
    let mut g = position();
    for id in 230..=239 {
        if g.catalog[id].kind == local_rift::cards::CardKind::Gear {
            gear(&mut g, id as u32 + 1000, id);
        } else {
            unit(&mut g, 0, id as u32 + 1000, id, None);
        }
    }
    let options = g.activated_options(0);
    assert_eq!(options.len(), 11);
    assert_eq!(activated_rules(239).len(), 2);
    assert!(activated_rules(9999).is_empty());
    assert!(g.activated_options(99).is_empty());
    let bad = AbilitySource::Unit {
        player: 99,
        unit_id: 100,
    };
    assert!(g.ability_targets(bad, 99).is_empty());
    rejected(&mut g, ability(bad, 99, vec![]));
    rejected(&mut g, ability(options[0].source, 99, vec![]));
    g.players[0].units[0].token = true;
    assert_eq!(
        g.activated_options(0).len(),
        10,
        "token source artwork is not a copied ability"
    );
}

#[test]
fn heart_orb_and_kingpin_resolve_only_after_responses_with_legal_any_owner_targets() {
    for (id, expected) in [(230, 3), (231, -1), (233, 3)] {
        let mut g = position();
        let target = unit(&mut g, 1, 200, 72, None);
        let actor = if id == 233 {
            source(unit(&mut g, 0, 100, id, None))
        } else {
            gear(&mut g, 100, id)
        };
        g.apply(ability(actor, 0, vec![target])).unwrap();
        assert_eq!(g.chain.len(), 1);
        assert_eq!(g.players[1].units[0].temporary_buff, 0);
        assert_eq!(g.priority(), 0);
        rejected(&mut g, ability(actor, 0, vec![target]));
        resolve(&mut g);
        assert_eq!(g.players[1].units[0].temporary_buff, expected);
    }
}

#[test]
fn orb_floor_is_a_resolution_snapshot_not_a_persistent_minimum_or_heal() {
    let mut g = position();
    let target = unit(&mut g, 1, 200, 15, None); // printed zero Might
    let orb = gear(&mut g, 100, 231);
    g.apply(ability(orb, 0, vec![target])).unwrap();
    resolve(&mut g);
    assert_eq!(g.unit_might(&g.players[1].units[0]).unwrap(), 0);
    assert_eq!(g.players[1].units[0].temporary_buff, 0);
}

#[test]
fn iron_enters_exhausted_kingpin_enters_ready_and_ability_reaction_does_not_speed_up_sage_card() {
    let mut g = position();
    cast(&mut g, 232, vec![]);
    assert!(!g.players[0].gear[0].ready);
    cast(&mut g, 233, vec![]);
    assert!(g.players[0].units[0].ready);
    let target = Target::Unit {
        player: 0,
        unit_id: g.players[0].units[0].id,
    };
    cast(&mut g, 7, vec![target]);
    g.players[0].hand.push(236);
    let hand_index = g.players[0].hand.len() - 1;
    rejected(
        &mut g,
        Action::PlayCardWithTargets {
            hand_index,
            lane: None,
            targets: vec![],
        },
    );
    cast(&mut g, 7, vec![target]); // actual Reaction is still legal
    cast(&mut g, 58, vec![]); // actual Quick-Draw is still legal
    assert_eq!(g.players[0].gear.len(), 2);
}

#[test]
fn normal_activations_reject_opponent_priority_and_showdowns_without_spending_costs() {
    let mut g = position();
    let target = unit(&mut g, 0, 100, 72, None);
    let heart = gear(&mut g, 101, 230);
    cast(&mut g, 7, vec![target]);
    rejected(&mut g, ability(heart, 0, vec![target]));
    g.apply(Action::Pass).unwrap();
    rejected(&mut g, ability(heart, 0, vec![target]));
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    assert!(g.showdown.is_some());
    rejected(&mut g, ability(heart, 0, vec![target]));
}

#[test]
fn attached_or_invalid_target_gear_cannot_pay_any_cost() {
    let mut g = position();
    let target = unit(&mut g, 0, 100, 72, None);
    let bar = gear(&mut g, 101, 234);
    rejected(&mut g, ability(bar, 0, vec![target])); // target is ready
    g.players[0].units[0].ready = false;
    g.players[0].gear[0].attached_to = Some(100);
    rejected(&mut g, ability(bar, 0, vec![target]));
    g.players[0].gear[0].attached_to = None;
    rejected(&mut g, ability(bar, 0, vec![target, target]));
    g.apply(ability(bar, 0, vec![target])).unwrap();
    resolve(&mut g);
    assert_eq!(g.players[0].units[0].buff, 1);
}

#[test]
fn arena_bar_rechecks_target_existence_but_still_completes_a_gear_use() {
    let mut g = position();
    unit(&mut g, 0, 100, 215, None);
    let target = unit(&mut g, 0, 102, 72, Some(0));
    g.players[0].units[1].ready = false;
    let bar = gear(&mut g, 101, 234);
    g.apply(ability(bar, 0, vec![target])).unwrap();
    cast(&mut g, 11, vec![target]); // removes the chosen exhausted unit
    resolve(&mut g);
    assert_eq!(g.players[0].units.len(), 1);
    resolve(&mut g);
    assert_eq!(g.chain.last().unwrap().card_id, 215);
    resolve(&mut g);
    assert_eq!(g.players[0].units[0].temporary_buff, 1);
}

#[test]
fn not_so_fast_counters_targeted_gear_ability_without_refunding_or_completion_event() {
    let mut g = position();
    unit(&mut g, 0, 100, 215, None);
    let target = unit(&mut g, 1, 200, 72, Some(1));
    let iron = gear(&mut g, 101, 232);
    g.apply(ability(iron, 0, vec![target])).unwrap();
    let chain_id = g.chain[0].id;
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 12, vec![Target::Spell { chain_id }]);
    resolve(&mut g);
    assert!(g.chain.is_empty());
    assert!(!g.players[0].gear[0].ready);
    assert_eq!(g.players[1].units[0].damage, 0);
    assert_eq!(g.players[0].units[0].temporary_buff, 0);
}

#[test]
fn iron_requires_battlefield_target_and_void_gate_adds_one_damage() {
    let mut g = position();
    g.battlefields[1] = 3;
    let target = unit(&mut g, 1, 200, 69, None);
    let iron = gear(&mut g, 101, 232);
    rejected(&mut g, ability(iron, 0, vec![target]));
    g.players[1].units[0].lane = Some(1);
    g.battlefield_owner[1] = Some(1);
    g.apply(ability(iron, 0, vec![target])).unwrap();
    resolve(&mut g);
    assert_eq!(g.players[1].units[0].damage, 3);
}

#[test]
fn xerath_requires_field_only_on_activation_pays_color_and_each_deflect_atomically() {
    let mut g = position();
    let xerath = source(unit(&mut g, 0, 100, 237, None));
    let target = unit(&mut g, 1, 200, 0, None);
    rejected(&mut g, ability(xerath, 0, vec![target]));
    g.players[0].units[0].lane = Some(0);
    g.battlefield_owner[0] = Some(0);
    g.players[0].power_pool = [0; 6];
    g.players[0].power_pool[0] = 1;
    g.legend[0].spell_power = 2;
    rejected(&mut g, ability(xerath, 0, vec![target]));
    g.players[0].power_pool[4] = 1;
    let mut payment = PowerPayment::default();
    payment.colored[0] = 1;
    payment.colored[4] = 1;
    g.apply(Action::WithPowerPayment {
        action: Box::new(ability(xerath, 0, vec![target])),
        payment,
    })
    .unwrap();
    assert_eq!(g.players[0].power_pool, [0; 6]);
    assert_eq!(g.legend[0].spell_power, 2);
    cast(
        &mut g,
        9,
        vec![Target::Unit {
            player: 0,
            unit_id: 100,
        }],
    );
    resolve(&mut g);
    assert_eq!(g.players[0].units[0].lane, None);
    resolve(&mut g);
    assert_eq!(g.players[1].units[0].damage, 3);
}

#[test]
fn caitlyn_reads_live_might_and_location_condition_is_not_a_resolution_gate() {
    for recall in [false, true] {
        let mut g = position();
        let caitlyn = unit(&mut g, 0, 100, 238, Some(0));
        let target = unit(&mut g, 1, 200, 69, Some(1));
        assert!(g.unit_keywords(&g.players[0].units[0]).unwrap().backline);
        g.apply(ability(source(caitlyn), 0, vec![target])).unwrap();
        cast(&mut g, if recall { 9 } else { 7 }, vec![caitlyn]);
        resolve(&mut g);
        resolve(&mut g);
        assert_eq!(g.players[1].units[0].damage, if recall { 3 } else { 5 });
    }
}

#[test]
fn absent_caitlyn_has_no_last_known_might_but_absent_xerath_keeps_fixed_damage() {
    for (card, expected) in [(238, 0), (237, 3)] {
        let mut g = position();
        let actor = unit(&mut g, 0, 100, card, Some(0));
        let target = unit(&mut g, 1, 200, 69, Some(1));
        g.apply(ability(source(actor), 0, vec![target])).unwrap();
        g.apply(Action::Pass).unwrap();
        let helper = unit(&mut g, 1, 201, 72, None);
        cast(&mut g, 16, vec![helper, actor]);
        resolve(&mut g);
        assert!(g.players[0].units.is_empty());
        resolve(&mut g);
        assert_eq!(g.players[1].units[0].damage, expected);
    }
}

#[test]
fn armory_parent_finishes_before_three_separate_recruit_finalizations() {
    let mut g = position();
    unit(&mut g, 0, 100, 72, Some(0));
    let armory = gear(&mut g, 101, 235);
    g.apply(ability(armory, 0, vec![])).unwrap();
    assert_eq!(g.players[0].units.len(), 1);
    resolve(&mut g);
    assert!(g.chain.is_empty());
    for (i, lane) in [None, Some(0), None].into_iter().enumerate() {
        assert!(
            matches!(g.decision, Some(Decision::TokenPlacement { remaining, .. }) if remaining == 3-i as u8)
        );
        assert_eq!(g.players[0].units.len(), 1 + i);
        g.apply(Action::PlaceToken { lane }).unwrap();
    }
    assert!(g.decision.is_none());
    assert_eq!(g.players[0].units.iter().filter(|u| u.token).count(), 3);
    assert!(
        g.activated_options(0)
            .iter()
            .all(|o| matches!(o.source, AbilitySource::Gear { .. }))
    );
}

#[test]
fn dragonsoul_can_pay_hard_bargain_when_current_energy_is_insufficient() {
    let mut g = position();
    let sage = source(unit(&mut g, 0, 100, 236, None));
    let target = unit(&mut g, 0, 101, 72, None);
    unit(&mut g, 0, 102, 215, None);
    g.players[0].energy = 3;
    g.players[0].energy_pool = 3;
    cast(&mut g, 7, vec![target]);
    let chain_id = g.chain[0].id;
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 37, vec![Target::Spell { chain_id }]);
    let bargain_id = g.chain.last().unwrap().id;
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::CounterTax {
            player: 0,
            cost: 2,
            ..
        })
    ));
    let decision = g.decision.clone();
    assert_eq!(g.players[0].energy, 1);
    g.step_ai().unwrap(); // prepares Sage instead of auto-declining an affordable tax
    assert_eq!(g.decision, decision);
    assert_eq!(g.players[0].energy, 2);
    assert!(!g.players[0].units[0].ready);
    assert_eq!(
        g.chain
            .iter()
            .map(|item| (item.id, item.card_id))
            .collect::<Vec<_>>(),
        vec![(chain_id, 7), (bargain_id, 37)],
        "the physical counterspell stays on the chain throughout its payment choice"
    );
    assert!(!g.players[1].discard.contains(&37));
    assert!(matches!(sage, AbilitySource::Unit { .. }));
    g.step_ai().unwrap();
    assert!(g.decision.is_none());
    assert_eq!(g.players[0].energy, 0);
    assert_eq!(g.chain.len(), 1);
    assert_eq!((g.chain[0].id, g.chain[0].card_id), (chain_id, 7));
    assert!(g.players[1].discard.contains(&37));
    resolve(&mut g);
    assert_eq!(g.players[0].units[1].temporary_buff, 2);
    assert_eq!(
        g.players[0].units[2].temporary_buff, 0,
        "unit activation is not gear use"
    );
}

#[test]
fn dragonsoul_is_reaction_immediate_and_cannot_interrupt_non_payment_choices() {
    let mut g = position();
    let sage = source(unit(&mut g, 0, 100, 236, None));
    let target = unit(&mut g, 0, 101, 72, None);
    cast(&mut g, 7, vec![target]);
    let chain = g.chain.clone();
    g.apply(ability(sage, 0, vec![])).unwrap();
    assert_eq!(g.chain, chain);
    assert_eq!(g.priority(), 0);
    assert_eq!(g.players[0].energy, 19);
    g.players[0].units[0].ready = true;
    g.decision = Some(Decision::Discard {
        player: 0,
        count: 1,
        draw_after: 2,
    });
    rejected(&mut g, ability(sage, 0, vec![]));
}

#[test]
fn last_rites_pending_child_cannot_cancel_when_sage_can_fund_its_energy() {
    let mut g = position();
    let sage = source(unit(&mut g, 0, 100, 236, None));
    g.players[0].discard = vec![103]; // three energy, no power
    g.decision = Some(Decision::OptionalPlay {
        player: 0,
        card_id: 38,
        gear_id: Some(101),
    });
    g.apply(Action::Choose { accept: true }).unwrap();
    g.apply(Action::SelectCards { indices: vec![0] }).unwrap();
    g.players[0].energy = 2;
    g.players[0].energy_pool = 2;
    g.legend[0].ready = false;
    resolve(&mut g);
    assert!(matches!(g.decision, Some(Decision::PendingPlay { .. })));
    assert!(!g.can_decline_trash_play().unwrap());
    rejected(&mut g, Action::Choose { accept: false });
    assert!(
        g.legal_actions()
            .unwrap()
            .contains(&ability(sage, 0, vec![]))
    );
    g.apply(ability(sage, 0, vec![])).unwrap();
    g.apply(finalize()).unwrap();
    assert_eq!(g.players[0].energy, 0);
    assert!(g.players[0].units.iter().any(|u| u.card_id == 103));
}

#[test]
fn renata_draw_is_repeatable_without_exhaustion_and_point_is_not_conquest() {
    let mut g = position();
    let renata = source(unit(&mut g, 0, 100, 239, Some(0)));
    let hand = g.players[0].hand.len();
    for _ in 0..2 {
        g.apply(ability(renata, 0, vec![])).unwrap();
        assert!(g.players[0].units[0].ready);
        resolve(&mut g);
    }
    assert_eq!(g.players[0].hand.len(), hand + 2);
    assert_eq!(g.players[0].energy, 18);
    assert_eq!(g.players[0].power_pool[2], 6);
    g.players[0].score = 7;
    let owners = g.battlefield_owner;
    g.apply(ability(renata, 1, vec![])).unwrap();
    assert_eq!(g.players[0].score, 7);
    assert!(!g.players[0].units[0].ready);
    resolve(&mut g);
    assert_eq!(g.players[0].score, 8);
    assert_eq!(g.winner, Some(0));
    assert_eq!(g.phase, Phase::Finished);
    assert_eq!(g.battlefield_owner, owners);
}

#[test]
fn simple_ai_uses_renata_to_win_and_targeted_damage_only_against_an_enemy() {
    for difficulty in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        let mut g = position();
        g.difficulty = difficulty;
        unit(&mut g, 0, 100, 239, Some(0));
        g.players[0].score = 7;
        g.step_ai().unwrap();
        assert!(matches!(
            g.chain.last().and_then(|item| item.ability.as_ref()),
            Some(ChainAbility::Activated {
                card_id: 239,
                ability_index: 1,
                ..
            })
        ));
        let mut g = position();
        g.difficulty = difficulty;
        unit(&mut g, 0, 100, 72, None);
        g.players[0].units[0].ready = false; // no competing free conquest by this unit
        let target = unit(&mut g, 1, 200, 88, Some(1));
        gear(&mut g, 101, 232);
        let mut saw_damage = false;
        for _ in 0..12 {
            g.step_ai().unwrap();
            if let Some(item) = g.chain.last()
                && matches!(
                    item.ability,
                    Some(ChainAbility::Activated { card_id: 232, .. })
                )
            {
                assert_eq!(item.targets, vec![target]);
                saw_damage = true;
                break;
            }
        }
        assert!(
            saw_damage,
            "{difficulty:?} should find the lethal Iron Ballista activation: {:?}",
            g.log
        );
    }
}

#[test]
fn simple_ai_preserves_kingpin_movement_and_avoids_useless_sage_conversion() {
    let mut g = position();
    unit(&mut g, 0, 100, 233, None);
    unit(&mut g, 0, 101, 72, None);
    unit(&mut g, 0, 102, 236, None);
    g.step_ai().unwrap();
    assert!(
        g.chain
            .iter()
            .all(|item| !matches!(item.ability, Some(ChainAbility::Activated { .. })))
    );
    assert!(
        g.showdown.is_some(),
        "an empty field is more valuable than a temporary self buff"
    );
    assert_eq!(g.players[0].energy_pool, 20);
}

fn conflicted_combat(two_caitlyns: bool, power: u8) -> Game {
    let mut g = position();
    unit(&mut g, 0, 100, 69, None);
    g.players[0].units[0].temporary_buff = i64::from(power) - 8;
    unit(&mut g, 1, 200, 238, Some(0));
    unit(&mut g, 1, 201, 72, Some(0));
    unit(&mut g, 1, 202, if two_caitlyns { 238 } else { 72 }, Some(0));
    for (id, bearer) in [(300, 200)]
        .into_iter()
        .chain(two_caitlyns.then_some((301, 202)))
    {
        g.players[1].gear.push(GearState {
            id,
            card_id: 121,
            lane: None,
            attached_to: Some(bearer),
            ready: true,
            temporary: false,
            token: false,
        });
    }
    g.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::CombatDamage { player: 0, .. })
    ));
    g
}

#[test]
fn caitlyn_with_tank_may_be_assigned_first_or_last_but_not_between_ordinary_units() {
    let game = conflicted_combat(false, 10);
    for assignments in [
        vec![(200, 4), (201, 3), (202, 3)],
        vec![(201, 3), (202, 3), (200, 4)],
    ] {
        let mut g = game.clone();
        g.apply(Action::AssignDamage { assignments }).unwrap();
        assert!(matches!(
            g.decision,
            Some(Decision::CombatDamage { player: 1, .. })
        ));
    }
    let mut g = game;
    rejected(
        &mut g,
        Action::AssignDamage {
            assignments: vec![(201, 3), (200, 4), (202, 3)],
        },
    );
    let mut partial = conflicted_combat(false, 3);
    partial
        .apply(Action::AssignDamage {
            assignments: vec![(201, 3)],
        })
        .unwrap();
}

#[test]
fn separate_tank_backline_conflicts_can_choose_different_priorities() {
    let game = conflicted_combat(true, 11);
    for assignments in [
        vec![(200, 4), (201, 3), (202, 4)],
        vec![(202, 4), (200, 4), (201, 3)],
        vec![(201, 3), (200, 4), (202, 4)],
    ] {
        let mut g = game.clone();
        g.apply(Action::AssignDamage { assignments }).unwrap();
    }
}

#[test]
fn arena_bar_target_readied_by_irelia_before_resolution_is_no_longer_eligible() {
    let mut g = position();
    g.legend[0] = local_rift::legends::LegendState::new(0);
    let target = unit(&mut g, 0, 100, 72, None);
    g.players[0].units[0].ready = false;
    let bar = gear(&mut g, 101, 234);
    g.apply(ability(bar, 0, vec![target])).unwrap();
    assert!(matches!(
        g.decision,
        Some(Decision::Legend { player: 0, .. })
    ));
    g.apply(Action::Choose { accept: true }).unwrap();
    assert_eq!(g.chain.len(), 2);
    resolve(&mut g);
    assert!(g.players[0].units[0].ready);
    resolve(&mut g);
    assert_eq!(g.players[0].units[0].buff, 0);
}

#[test]
fn caitlyn_does_not_damage_a_target_recalled_before_resolution() {
    let mut g = position();
    let caitlyn = unit(&mut g, 0, 100, 238, Some(0));
    let target = unit(&mut g, 1, 200, 69, Some(1));
    g.apply(ability(source(caitlyn), 0, vec![target])).unwrap();
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 9, vec![target]);
    resolve(&mut g);
    assert_eq!(g.players[1].units[0].lane, None);
    resolve(&mut g);
    assert_eq!(g.players[1].units[0].damage, 0);
}
