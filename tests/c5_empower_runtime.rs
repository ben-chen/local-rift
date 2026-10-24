//! Empower costs and live activation instructions through public engine actions.
use local_rift::activated::{AbilitySource, AbilityTiming, activated_rules};
use local_rift::cards::Domain;
use local_rift::empower::BoardObject;
use local_rift::game::ChainAbility;
use local_rift::game::{Action, Decision, Difficulty, Game, Target, Unit};
use local_rift::gear::GearState;
use local_rift::legends::{LegendAbility, LegendState};

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

fn settle(g: &mut Game) {
    for _ in 0..150 {
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
            Some(other) => panic!("unexpected {other:?}"),
            None if g.in_window() => Action::Pass,
            None => return,
        };
        g.apply(action).unwrap();
    }
    panic!("did not settle");
}
fn object(target: Target) -> BoardObject {
    match target {
        Target::Unit { player, unit_id } => BoardObject::Unit { player, unit_id },
        Target::Gear { player, gear_id } => BoardObject::Gear { player, gear_id },
        _ => panic!(),
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

#[test]
fn every_new_empower_activation_pays_printed_cost_and_changes_only_after_resolution() {
    for card in 272..=289 {
        let mut g = position();
        let source = if card == 281 {
            gear(&mut g, 1000, card)
        } else {
            source(unit(&mut g, 0, 1000, card, None))
        };
        let before = g.players[0].energy;
        let rule = activated_rules(card)[0];
        assert_eq!(rule.timing, AbilityTiming::Main);
        let action = ability(source, 0, vec![]);
        let quoted = g.cost_plan(&action).unwrap().energy;
        g.apply(action).unwrap();
        assert_eq!(g.players[0].energy, before - quoted as u16, "{card}");
        assert!(!g.unit_empowered(1000));
        assert!(g.players[0].units.iter().all(|u| u.ready));
        settle(&mut g);
        assert!(g.unit_empowered(1000), "{card}");
        rejected(&mut g, ability(source, 0, vec![]));
    }
}
#[test]
fn conditional_keywords_might_and_aura_replacement_are_live() {
    for (card, might, assault, shield, deflect, ganking) in [
        (272, 2, 3, 0, 0, false),
        (273, 7, 0, 0, 1, false),
        (274, 3, 0, 3, 1, false),
        (275, 5, 0, 0, 0, true),
        (276, 6, 0, 0, 0, true),
        (277, 4, 0, 0, 2, false),
        (278, 6, 0, 0, 0, false),
        (279, 4, 0, 3, 1, false),
        (280, 6, 2, 0, 1, false),
        (283, 4, 0, 0, 0, false),
        (286, 6, 0, 0, 0, false),
        (287, 8, 0, 0, 2, false),
        (288, 4, 0, 0, 0, false),
        (289, 5, 2, 0, 0, false),
    ] {
        let mut g = position();
        let t = unit(&mut g, 0, 1000, card, None);
        g.apply(ability(source(t), 0, vec![])).unwrap();
        settle(&mut g);
        let k = g.unit_keywords(u(&g, t));
        assert_eq!(
            (
                g.unit_might(u(&g, t)),
                k.assault,
                k.shield,
                k.deflect,
                k.ganking
            ),
            (might, assault, shield, deflect, ganking),
            "{card}"
        );
        assert_eq!(u(&g, t).buff, 0);
        assert_eq!(u(&g, t).temporary_buff, 0);
        assert!(g.passive_contributions(object(t)).iter().all(|c| c.active));
    }
    let mut g = position();
    let target = unit(&mut g, 0, 1000, 72, None);
    let rage = gear(&mut g, 1001, 281);
    assert_eq!(g.unit_might(u(&g, target)), g.catalog[72].might + 1);
    g.apply(ability(rage, 0, vec![])).unwrap();
    settle(&mut g);
    assert_eq!(g.unit_might(u(&g, target)), g.catalog[72].might + 2);
    let contributions = g.passive_contributions(object(target));
    assert_eq!(contributions.iter().filter(|c| c.active).count(), 1);
}
#[test]
fn aurok_stacks_on_empowered_friends_including_tokens_but_not_unempowered_units() {
    let mut g = position();
    let a = unit(&mut g, 0, 1000, 282, None);
    let b = unit(&mut g, 0, 1001, 282, None);
    let normal = unit(&mut g, 0, 1002, 72, None);
    let token = unit(&mut g, 0, 1003, 282, None);
    g.players[0].units.last_mut().unwrap().token = true;
    g.players[0].units.last_mut().unwrap().buff = 1;
    g.empowered_units.push(1003);
    for t in [a, b] {
        g.apply(ability(source(t), 0, vec![])).unwrap();
        settle(&mut g);
    }
    assert_eq!(g.unit_might(u(&g, a)), 9);
    assert_eq!(g.unit_might(u(&g, b)), 9);
    assert_eq!(g.unit_might(u(&g, normal)), g.catalog[72].might);
    assert_eq!(g.unit_might(u(&g, token)), 5);
}
#[test]
fn matriarch_observes_actor_empowering_enemy_gear_and_readies_enemy_unit_with_exact_costs() {
    let mut g = position();
    g.legend[0] = LegendState::from_legend(7).unwrap();
    g.legend[1] = LegendState::from_legend(7).unwrap();
    let formula = gear(&mut g, 1000, 243);
    g.players[1].gear.push(GearState {
        id: 1001,
        card_id: 42,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    g.apply(ability(
        formula,
        0,
        vec![Target::Gear {
            player: 1,
            gear_id: 1001,
        }],
    ))
    .unwrap();
    resolve(&mut g);
    assert!(g.object_empowered(BoardObject::Gear {
        player: 1,
        gear_id: 1001
    }));
    assert!(!g.legend[0].empowered);
    assert!(!g.legend[1].empowered);
    assert!(g.chain.iter().any(|c| matches!(
        c.ability,
        Some(ChainAbility::Legend(LegendAbility::MatriarchEmpower))
    )));
    settle(&mut g);
    assert!(g.legend[0].empowered);
    assert!(!g.legend[1].empowered);
    let enemy = unit(&mut g, 1, 1002, 0, None);
    g.players[1].units.last_mut().unwrap().ready = false;
    g.players[0].power_pool = [0; 6];
    g.players[0].any_power = 1;
    rejected(
        &mut g,
        Action::ActivateLegend {
            target: Some(enemy),
        },
    );
    g.players[0].any_power = 2;
    g.apply(Action::ActivateLegend {
        target: Some(enemy),
    })
    .unwrap();
    assert_eq!(g.players[0].any_power, 0);
    assert!(!g.legend[0].ready);
    assert!(!g.legend[0].empowered);
    assert!(!u(&g, enemy).ready);
    settle(&mut g);
    assert!(u(&g, enemy).ready);
    assert_eq!(u(&g, enemy).temporary_buff, 0);
}
#[test]
fn apprentice_predict_two_preserves_deck_until_atomic_partition_and_order() {
    let mut g = position();
    let t = unit(&mut g, 0, 1000, 283, None);
    g.players[0].deck = vec![69, 70, 71, 72];
    g.apply(ability(source(t), 0, vec![])).unwrap();
    resolve(&mut g);
    assert!(g.decision.is_none());
    resolve(&mut g);
    let Some(Decision::PredictCards {
        choice_id,
        ref cards,
        ..
    }) = g.decision
    else {
        panic!("{:?}", g.decision)
    };
    assert_eq!(cards, &vec![72, 71]);
    assert_eq!(g.players[0].deck, vec![69, 70, 71, 72]);
    rejected(
        &mut g,
        Action::ResolvePredict {
            choice_id,
            recycle: vec![0],
            top_order: vec![0],
        },
    );
    rejected(
        &mut g,
        Action::ResolvePredict {
            choice_id: choice_id + 1,
            recycle: vec![],
            top_order: vec![0, 1],
        },
    );
    g.apply(Action::ResolvePredict {
        choice_id,
        recycle: vec![],
        top_order: vec![1, 0],
    })
    .unwrap();
    assert_eq!(g.players[0].deck, vec![69, 70, 72, 71]);
}
#[test]
fn predict_short_deck_never_burns_out_and_recycle_is_seeded() {
    for n in 0..=2 {
        let mut g = position();
        let t = unit(&mut g, 0, 1000, 283, None);
        g.players[0].deck = (69..69 + n).collect();
        g.apply(ability(source(t), 0, vec![])).unwrap();
        resolve(&mut g);
        resolve(&mut g);
        if n == 0 {
            assert!(g.decision.is_none());
        } else {
            let Some(Decision::PredictCards { choice_id, .. }) = g.decision else {
                panic!()
            };
            let mut other = g.clone();
            let action = Action::ResolvePredict {
                choice_id,
                recycle: (0..n).collect(),
                top_order: vec![],
            };
            g.apply(action.clone()).unwrap();
            other.apply(action).unwrap();
            assert_eq!(g.players[0].deck, other.players[0].deck);
        }
        assert_eq!(g.players[1].score, 0);
        assert_eq!(g.players[0].deck.len(), n);
    }
}
#[test]
fn rune_dependent_prices_use_controlled_runes_and_stay_fixed_during_payment() {
    for card in [278, 279, 280] {
        for n in [0, 4, 5, 12] {
            let mut g = position();
            let t = unit(&mut g, 0, 1000, card, None);
            g.players[0].runes = [n, 0, 0, 0, 0, 0];
            g.players[0].exhausted_runes = [n, 0, 0, 0, 0, 0];
            let expected = if card == 280 {
                if n <= 4 { 2 } else { 5 }
            } else {
                12 - n
            };
            assert_eq!(
                g.cost_plan(&ability(source(t), 0, vec![])).unwrap().energy,
                u32::from(expected)
            );
        }
    }
    let mut g = position();
    let t = unit(&mut g, 0, 1000, 278, None);
    g.players[0].runes = [5, 0, 0, 0, 0, 0];
    g.players[0].exhausted_runes = [5, 0, 0, 0, 0, 0];
    g.apply(Action::BeginPayment {
        action: Box::new(ability(source(t), 0, vec![])),
    })
    .unwrap();
    assert_eq!(g.payment_plan_view(0).unwrap().energy, 7);
    g.apply(Action::RecycleRune {
        domain: Domain::Fury,
        exhausted: true,
    })
    .unwrap();
    assert_eq!(g.payment_plan_view(0).unwrap().energy, 7);
    let before = g.players[0].energy;
    g.apply(Action::CompletePayment).unwrap();
    assert_eq!(g.players[0].energy, before - 7);
    settle(&mut g);
    assert!(g.unit_empowered(1000));
}

#[test]
fn aura_loss_causes_a_second_death_pass_and_captures_empowered_deathknell_once() {
    let mut g = position();
    let aurok = unit(&mut g, 0, 1000, 282, None);
    let emissary = unit(&mut g, 0, 1001, 285, None);
    g.empowered_units.extend([1000, 1001]);
    g.players[0].units[0].damage = 1;
    g.players[0].units[1].damage = 3;
    assert_eq!(g.unit_might(u(&g, emissary)), 4);
    cast(&mut g, 61, vec![aurok]);
    resolve(&mut g);
    assert!(g.players[0].units.is_empty());
    assert!(g.players[0].discard.contains(&282));
    assert!(g.players[0].discard.contains(&285));
    assert_eq!(g.chain.iter().filter(|i| i.card_id == 285).count(), 1);
    settle(&mut g);
    assert_eq!(
        g.players[0]
            .units
            .iter()
            .filter(|u| u.token && u.card_id == 285)
            .count(),
        2
    );
}
#[test]
fn witherclaw_death_channels_available_runes_exhausted_and_banish_does_not() {
    for available in 0..=2 {
        let mut g = position();
        let target = unit(&mut g, 0, 1000, 286, None);
        g.empowered_units.push(1000);
        g.players[0].rune_deck = vec![Domain::Body; available];
        let energy = g.players[0].energy;
        cast(&mut g, 61, vec![target]);
        resolve(&mut g);
        assert_eq!(g.players[0].runes, [0; 6]);
        settle(&mut g);
        assert_eq!(g.players[0].runes[3], available as u8);
        assert_eq!(g.players[0].exhausted_runes[3], available as u8);
        assert_eq!(g.players[0].energy, energy - g.catalog[61].cost as u16);
    }
    let mut g = position();
    let t = unit(&mut g, 0, 1000, 285, Some(0));
    g.empowered_units.push(1000);
    cast(&mut g, 62, vec![t]);
    settle(&mut g);
    assert!(g.players[0].banish.contains(&285));
    assert!(!g.players[0].units.iter().any(|u| u.token));
}
#[test]
fn replacement_prevents_empowered_deathknell() {
    let mut g = position();
    let t = unit(&mut g, 0, 1000, 285, None);
    g.empowered_units.push(1000);
    gear(&mut g, 1001, 10);
    g.players[0].gear[0].attached_to = Some(1000);
    cast(&mut g, 61, vec![t]);
    settle(&mut g);
    assert!(g.players[0].units.iter().any(|u| u.id == 1000));
    assert!(!g.players[0].units.iter().any(|u| u.token));
    assert!(g.players[0].discard.contains(&10));
}
#[test]
fn covert_move_trigger_survives_later_disempower() {
    let mut g = position();
    let t = unit(&mut g, 0, 1000, 284, None);
    g.empowered_units.push(1000);
    let before = g.players[0].hand.len();
    g.apply(Action::Move {
        unit_id: 1000,
        lane: Some(0),
    })
    .unwrap();
    assert!(g.chain.iter().any(|c| c.card_id == 284));
    // Explicit post-finalization state fixture: dependent text was active at
    // the event. Later status loss cannot erase an already created ability.
    g.empowered_units.clear();
    resolve(&mut g);
    assert_eq!(g.players[0].hand.len(), before + 1);
    assert_eq!(u(&g, t).lane, Some(0));
}
#[test]
fn akali_uses_recorded_move_battlefields_even_after_source_leaves() {
    let mut g = position();
    let akali = unit(&mut g, 0, 1000, 288, None);
    let target = unit(&mut g, 1, 1001, 69, Some(0));
    g.apply(Action::Move {
        unit_id: 1000,
        lane: Some(0),
    })
    .unwrap();
    assert!(matches!(
        g.decision,
        Some(Decision::TriggerTargets { card_id: 288, .. })
    ));
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![target],
    })
    .unwrap();
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 11, vec![akali]);
    resolve(&mut g);
    assert!(g.players[0].hand.contains(&288));
    resolve(&mut g);
    assert_eq!(
        u(&g, target).damage,
        2,
        "base1 plus Void Gate1; departing source supplies no Empowered bonus"
    );
}
#[test]
fn akali_empowered_damage_is_live_and_origin_field_is_eligible() {
    let mut g = position();
    let akali = unit(&mut g, 0, 1000, 288, Some(0));
    let target = unit(&mut g, 1, 1001, 69, Some(0));
    gear(&mut g, 1002, 3);
    g.players[0].gear[0].attached_to = Some(1000);
    g.empowered_units.push(1000);
    g.apply(Action::Move {
        unit_id: 1000,
        lane: Some(1),
    })
    .unwrap();
    assert!(g.valid_trigger_target(target, &[]));
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![target],
    })
    .unwrap();
    resolve(&mut g);
    assert_eq!(u(&g, target).damage, 3);
    assert_eq!(u(&g, akali).lane, Some(1));
}
#[test]
fn empowered_nasus_conquest_point_is_independent_and_can_finish_after_replaced_final_conquest() {
    let mut g = position();
    unit(&mut g, 0, 1000, 287, None);
    g.empowered_units.push(1000);
    g.players[0].score = 7;
    let hand = g.players[0].hand.len();
    g.apply(Action::Move {
        unit_id: 1000,
        lane: Some(0),
    })
    .unwrap();
    settle(&mut g);
    assert_eq!(g.winner, Some(0));
    assert_eq!(g.players[0].score, 8);
    assert_eq!(g.players[0].hand.len(), hand + 1);
}
#[test]
fn ambessa_attack_kill_compares_live_assault_might_and_revalidates_here() {
    for move_source in [false, true] {
        let mut g = position();
        let ambessa = unit(&mut g, 0, 1000, 289, None);
        let enemy = unit(&mut g, 1, 1001, 71, Some(0));
        g.empowered_units.push(1000);
        // Playful Phantom has5 printed Might; empowered attacking Ambessa has7.
        assert_eq!(g.catalog[71].might, 5);
        g.apply(Action::Move {
            unit_id: 1000,
            lane: Some(0),
        })
        .unwrap();
        assert!(g.valid_trigger_target(enemy, &[]));
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![enemy],
        })
        .unwrap();
        if move_source {
            cast(&mut g, 9, vec![ambessa]);
            resolve(&mut g);
        }
        resolve(&mut g);
        assert_eq!(g.players[1].units.iter().any(|u| u.id == 1001), move_source);
        if !move_source {
            assert!(g.players[1].discard.contains(&71));
        }
    }
}
#[test]
fn ambessa_ai_readies_a_friendly_exhausted_unit_instead_of_an_enemy() {
    for difficulty in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        let mut g = position();
        g.difficulty = difficulty;
        g.legend[0] = LegendState::from_legend(7).unwrap();
        g.legend[0].empowered = true;
        let own = unit(&mut g, 0, 1000, 72, None);
        unit(&mut g, 1, 1001, 69, None);
        g.players[0].units[0].ready = false;
        g.step_ai().unwrap();
        assert!(g.chain.iter().any(|c|matches!(c.ability,Some(ChainAbility::Legend(LegendAbility::MatriarchReady{target})) if target==own)),"{difficulty:?} {:?}",g.log);
        settle(&mut g);
        assert!(u(&g, own).ready);
    }
}

#[test]
fn enemy_apprentice_observes_tornado_empower_but_matriarch_follows_actor() {
    let mut g = position();
    g.legend[0] = LegendState::from_legend(7).unwrap();
    g.legend[1] = LegendState::from_legend(7).unwrap();
    unit(&mut g, 0, 1000, 72, Some(0));
    let enemy = unit(&mut g, 1, 1001, 283, Some(0));
    g.battlefield_owner[0] = Some(0);
    // A Reaction window after an enemy has entered our defended battlefield.
    g.players[0].hidden.push(local_rift::hidden::HiddenCard {
        id: 1002,
        card_id: 46,
        lane: 0,
        hidden_turn: 0,
        revealed: false,
    });
    g.apply(Action::PlayHidden {
        hidden_id: 1002,
        targets: vec![],
    })
    .unwrap();
    assert!(matches!(g.decision, Some(Decision::EmpowerHere { .. })));
    g.apply(Action::ChooseTarget {
        target: Some(enemy),
    })
    .unwrap();
    resolve(&mut g);
    assert!(g.unit_empowered(1001));
    assert!(!g.legend[1].empowered);
    assert!(g.chain.iter().any(|i| i.player == 0
        && matches!(
            i.ability,
            Some(ChainAbility::Legend(LegendAbility::MatriarchEmpower))
        )));
    assert!(g.chain.iter().any(|i| i.player == 1 && i.card_id == 283));
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::PredictCards { player: 1, .. })
    ));
}
#[test]
fn no_op_empower_does_not_trigger_matriarch_but_later_real_transition_does() {
    let mut g = position();
    g.legend[0] = LegendState::from_legend(7).unwrap();
    let formula = gear(&mut g, 1000, 243);
    let target = gear(&mut g, 1001, 42);
    let AbilitySource::Gear { player, gear_id } = target else {
        panic!()
    };
    g.empowered_units.push(gear_id);
    g.apply(ability(formula, 0, vec![Target::Gear { player, gear_id }]))
        .unwrap();
    resolve(&mut g);
    assert!(g.chain.is_empty());
    assert!(!g.legend[0].empowered);
    // New source and an unempowered object give a real transition, even while
    // the observer is already Empowered: its resulting trigger is a no-op.
    let formula2 = gear(&mut g, 1002, 243);
    let target2 = gear(&mut g, 1003, 42);
    g.legend[0].empowered = true;
    let AbilitySource::Gear { player, gear_id } = target2 else {
        panic!()
    };
    g.apply(ability(formula2, 0, vec![Target::Gear { player, gear_id }]))
        .unwrap();
    resolve(&mut g);
    assert_eq!(g.chain.len(), 1);
    resolve(&mut g);
    assert!(g.chain.is_empty());
}
#[test]
fn consecutive_predict_choices_have_distinct_instance_ids_even_for_identical_cards() {
    let mut g = position();
    let first = unit(&mut g, 0, 1000, 283, None);
    let second = unit(&mut g, 0, 1001, 283, None);
    g.players[0].deck = vec![72, 72, 72];
    let mut ids = Vec::new();
    for target in [first, second] {
        g.apply(ability(source(target), 0, vec![])).unwrap();
        resolve(&mut g);
        resolve(&mut g);
        let Some(Decision::PredictCards { choice_id, .. }) = g.decision else {
            panic!()
        };
        ids.push(choice_id);
        g.apply(Action::ResolvePredict {
            choice_id,
            recycle: vec![],
            top_order: vec![0, 1],
        })
        .unwrap();
    }
    assert_ne!(ids[0], ids[1]);
}
#[test]
fn conditional_deflect_is_paid_for_each_choice_and_failed_payment_is_atomic() {
    let mut g = position();
    let target = unit(&mut g, 1, 1000, 277, None);
    g.empowered_units.push(1000);
    g.players[0].power_pool = [0; 6];
    g.players[0].any_power = 5;
    g.players[0].hand.push(53);
    let action = Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![target, target],
    };
    // Falling Star costs2 colored Fury plus Deflect2 for each of its two choices.
    rejected(&mut g, action.clone());
    g.players[0].power_pool[0] = 2;
    g.players[0].any_power = 4;
    g.apply(action).unwrap();
    assert_eq!(g.players[0].any_power, 0);
    assert_eq!(g.players[0].power_pool[0], 0);
}
#[test]
fn simultaneous_aurok_deaths_do_not_reclassify_the_other_players_death_set_midpass() {
    let mut g = position();
    let a = unit(&mut g, 0, 1000, 282, None);
    let b = unit(&mut g, 1, 1001, 282, None);
    let e = unit(&mut g, 1, 1002, 285, None);
    g.empowered_units.extend([1000, 1001, 1002]);
    g.players[0].units[0].damage = 1;
    g.players[1].units[0].damage = 1;
    g.players[1].units[1].damage = 3;
    cast(&mut g, 61, vec![a, b]);
    resolve(&mut g);
    assert!(g.players[0].units.is_empty());
    assert!(!g.players[1].units.iter().any(|u| u.id == 1002));
    assert_eq!(g.chain.iter().filter(|i| i.card_id == 285).count(), 1);
    let _ = e;
    settle(&mut g);
    assert_eq!(g.players[1].units.iter().filter(|u| u.token).count(), 2);
}

#[test]
fn ambessas_kill_is_a_real_kill_instruction_with_controller_replacement_choice() {
    let mut g = position();
    unit(&mut g, 0, 1000, 289, None);
    let enemy = unit(&mut g, 1, 1001, 285, Some(0));
    g.empowered_units.extend([1000, 1001]);
    g.players[1].gear.push(GearState {
        id: 1002,
        card_id: 10,
        attached_to: Some(1001),
        ready: true,
        temporary: false,
        token: false,
    });
    g.players[1].gear.push(GearState {
        id: 1003,
        card_id: 21,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    g.apply(Action::Move {
        unit_id: 1000,
        lane: Some(0),
    })
    .unwrap();
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![enemy],
    })
    .unwrap();
    resolve(&mut g);
    let Some(Decision::DeathReplacement {
        player,
        ref options,
    }) = g.decision
    else {
        panic!("{:?}", g.decision)
    };
    assert_eq!(player, 1);
    assert_eq!(options.len(), 2);
    assert_eq!(u(&g, enemy).damage, 0, "Kill does not manufacture damage");
    g.apply(Action::ChooseDeathReplacement {
        unit_id: 1001,
        gear_id: Some(1003),
    })
    .unwrap();
    assert_eq!(u(&g, enemy).lane, None);
    assert!(g.players[1].discard.contains(&21));
    assert!(!g.chain.iter().any(|i| i.card_id == 285));
    assert!(g.players[1].units.iter().all(|u| !u.token));
}
#[test]
fn ambessas_dependent_trigger_remains_but_its_might_comparison_is_live() {
    for card in [71, 73] {
        let mut g = position();
        unit(&mut g, 0, 1000, 289, None);
        let enemy = unit(&mut g, 1, 1001, card, Some(0));
        g.empowered_units.push(1000);
        g.apply(Action::Move {
            unit_id: 1000,
            lane: Some(0),
        })
        .unwrap();
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![enemy],
        })
        .unwrap();
        // Explicit response-state fixture for a future Disempower instruction.
        // No included Reaction currently Disempowers an arbitrary unit.
        g.empowered_units.clear();
        resolve(&mut g);
        assert_eq!(
            g.players[1].units.iter().any(|u| u.id == 1001),
            card == 71,
            "strictly less than live5 Might; trigger existence is already fixed"
        );
    }
}
