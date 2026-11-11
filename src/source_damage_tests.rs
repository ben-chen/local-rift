//! C27 actual cards in controlled positions, followed by public engine actions.
//! Numeric/null boundary fixtures are explicitly identified below.
use super::*;

fn position(actor: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 270027, Difficulty::Tactical);
    g.start_match().unwrap();
    g.active = actor;
    g.priority_player = actor;
    g.battlefields = [0, 0];
    g.battlefield_owner = [Some(1 - actor), None];
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
        p.energy_pool = 100;
        p.energy = 100;
        p.power_pool = [20; 6];
        p.any_power = 0;
    }
    for l in &mut g.legend {
        l.ready = false;
    }
    g
}
fn unit(g: &mut Game, player: usize, card: CardId, lane: Option<usize>) -> Target {
    let id = g.create_unit(player, card, false, 0);
    let u = g.players[player].units.last_mut().unwrap();
    u.lane = lane;
    u.ready = true;
    Target::Unit {
        player,
        unit_id: id,
    }
}
fn owner(t: Target) -> usize {
    let Target::Unit { player, .. } = t else {
        unreachable!()
    };
    player
}
fn move_to(g: &mut Game, t: Target, lane: Option<usize>) {
    let Target::Unit { unit_id, .. } = t else {
        unreachable!()
    };
    g.apply(Action::Move { unit_id, lane }).unwrap();
}
fn cast(g: &mut Game, card: CardId, lane: Option<usize>, targets: Vec<Target>) {
    let p = g.priority();
    let hand_index = g.players[p].hand.len();
    g.players[p].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane,
        targets,
    })
    .unwrap();
}
fn focus(g: &mut Game, p: usize) {
    if g.priority() != p {
        g.apply(Action::Pass).unwrap();
    }
    assert_eq!(g.priority(), p);
}
fn pair(g: &mut Game) {
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn settle(g: &mut Game) {
    for _ in 0..60 {
        match g.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => g
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            None if !g.chain.is_empty() => g.apply(Action::Pass).unwrap(),
            None => return,
            other => panic!("Unexpected decision {other:?}"),
        }
    }
    panic!("chain did not settle");
}
fn reject(g: &mut Game, a: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(a).is_err());
    assert_eq!(format!("{g:?}"), before);
}
fn choose(g: &mut Game, t: Target) {
    g.apply(Action::ChooseTriggerTargets { targets: vec![t] })
        .unwrap();
}
fn damage(g: &Game, t: Target) -> u64 {
    g.unit_ref(t).unwrap().damage
}
fn attach(g: &mut Game, t: Target, card: CardId) {
    let p = owner(t);
    let gear_id = 10000 + g.players[p].gear.len() as u32;
    g.players[p].gear.push(GearState {
        id: gear_id,
        card_id: card,
        lane: None,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    g.apply(Action::Equip { gear_id, target: t }).unwrap();
    settle(g);
}
fn attack(card: CardId, p: usize) -> (Game, Target, Target) {
    let mut g = position(p);
    let s = unit(&mut g, p, card, None);
    let e = unit(&mut g, 1 - p, 70, Some(0));
    move_to(&mut g, s, Some(0));
    (g, s, e)
}

#[test]
fn yasuo_stupefy_reads_live_might_after_response_in_both_seats() {
    for p in 0..2 {
        let (mut g, s, e) = attack(413, p);
        choose(&mut g, e);
        assert_eq!(damage(&g, e), 0);
        focus(&mut g, 1 - p);
        cast(&mut g, 63, None, vec![s]);
        settle(&mut g);
        assert!(!g.unit_ref(s).unwrap().stunned);
        assert_eq!(g.unit_might(g.unit_ref(s).unwrap()).unwrap(), 5);
        assert_eq!(damage(&g, e), 5);
    }
}

#[test]
fn chosen_live_here_rechecks_source_and_target_after_flash() {
    for p in 0..2 {
        for card in [413, 415] {
            for moving_source in [false, true] {
                let (mut g, s, e) = attack(card, p);
                choose(&mut g, e);
                let t = if moving_source { s } else { e };
                focus(&mut g, owner(t));
                cast(&mut g, 9, None, vec![t]);
                settle(&mut g);
                assert_eq!(damage(&g, e), 0);
            }
        }
    }
}

#[test]
fn targeted_source_damage_is_counterable_and_pays_deflect_before_effect() {
    for p in 0..2 {
        for card in [413, 415] {
            let (mut g, _, e) = attack(card, p);
            choose(&mut g, e);
            let chain_id = g.chain.last().unwrap().id;
            focus(&mut g, 1 - p);
            cast(&mut g, 12, None, vec![Target::Spell { chain_id }]);
            settle(&mut g);
            assert_eq!(damage(&g, e), 0);
            let mut g = position(p);
            let s = unit(&mut g, p, card, None);
            let e = unit(&mut g, 1 - p, 213, Some(0));
            move_to(&mut g, s, Some(0));
            let before = g.players[p].power_pool.iter().sum::<u16>();
            choose(&mut g, e);
            assert!(g.players[p].power_pool.iter().sum::<u16>() < before);
            assert_eq!(damage(&g, e), 0);
        }
    }
}

#[test]
fn graves_attack_and_defend_hit_all_enemies_here_without_targets_or_deflect() {
    for p in 0..2 {
        for defending in [false, true] {
            let a = if defending { 1 - p } else { p };
            let mut g = position(a);
            let s = unit(&mut g, p, 414, defending.then_some(0));
            let e = unit(&mut g, 1 - p, 70, (!defending).then_some(0));
            let deflect = unit(&mut g, 1 - p, 213, Some(0));
            g.unit_mut(deflect).unwrap().buff = 20;
            let protected = unit(&mut g, 1 - p, 70, Some(0));
            g.unit_mut(protected).unwrap().protected = true;
            let ally = unit(&mut g, p, 70, Some(0));
            let far = unit(&mut g, 1 - p, 70, Some(1));
            let base = unit(&mut g, 1 - p, 70, None);
            g.players[p].power_pool = [0; 6];
            g.players[p].any_power = 0;
            move_to(&mut g, if defending { e } else { s }, Some(0));
            assert!(g.decision.is_none());
            let item = g.chain.last().unwrap();
            assert_eq!(item.card_id, 414);
            assert!(item.targets.is_empty());
            let chain_id = item.id;
            focus(&mut g, 1 - p);
            let hand_index = g.players[1 - p].hand.len();
            g.players[1 - p].hand.push(12);
            reject(
                &mut g,
                Action::PlayCardWithTargets {
                    hand_index,
                    lane: None,
                    targets: vec![Target::Spell { chain_id }],
                },
            );
            settle(&mut g);
            for t in [e, deflect, protected] {
                assert_eq!(damage(&g, t), 4);
            }
            for t in [ally, far, base] {
                assert_eq!(damage(&g, t), 0);
            }
            let c = g.unit_combat_contribution(g.unit_ref(s).unwrap()).unwrap();
            assert_eq!(
                (c.might, c.damage, c.stunned, c.prevents_combat_damage),
                (4, 0, false, true)
            );
            assert_eq!(g.players[p].power_pool, [0; 6]);
        }
    }
}

#[test]
fn graves_uses_live_might_and_current_recipients_including_reaction_entry() {
    for p in 0..2 {
        let mut g = position(1 - p);
        let s = unit(&mut g, p, 414, Some(0));
        let e = unit(&mut g, 1 - p, 70, None);
        move_to(&mut g, e, Some(0));
        focus(&mut g, 1 - p);
        cast(&mut g, 411, Some(0), vec![]);
        let r = g.players[1 - p]
            .units
            .iter()
            .find(|u| u.card_id == 411)
            .unwrap()
            .id;
        cast(&mut g, 63, None, vec![s]);
        settle(&mut g);
        assert_eq!(damage(&g, e), 3);
        assert_eq!(
            damage(
                &g,
                Target::Unit {
                    player: 1 - p,
                    unit_id: r
                }
            ),
            3
        );
        assert_eq!(g.unit_might(g.unit_ref(s).unwrap()).unwrap(), 3);
    }
}

#[test]
fn graves_departure_rechecks_here_instead_of_old_battlefield() {
    for p in 0..2 {
        let (mut g, s, e) = attack(414, p);
        focus(&mut g, p);
        cast(&mut g, 9, None, vec![s]);
        settle(&mut g);
        assert_eq!(damage(&g, e), 0);
        assert_eq!(g.unit_ref(s).unwrap().lane, None);
    }
}

#[test]
fn graves_applies_every_packet_before_any_death_replacement_choice() {
    for p in 0..2 {
        let mut g = position(1 - p);
        let fragile = unit(&mut g, 1 - p, 71, None);
        attach(&mut g, fragile, 10);
        attach(&mut g, fragile, 10);
        g.unit_mut(fragile).unwrap().lane = Some(0);
        g.battlefield_owner[0] = Some(1 - p);
        let survivor = unit(&mut g, 1 - p, 70, Some(0));
        g.active = p;
        g.priority_player = p;
        let s = unit(&mut g, p, 414, None);
        g.unit_mut(s).unwrap().buff = 4;
        move_to(&mut g, s, Some(0));
        pair(&mut g);
        assert!(matches!(
            g.decision,
            Some(Decision::DeathReplacement { .. })
        ));
        assert_eq!(damage(&g, survivor), 8);
    }
}

#[test]
fn source_damage_applies_ability_bonuses_to_each_positive_packet() {
    for p in 0..2 {
        let mut g = position(p);
        g.battlefields[0] = 3;
        unit(&mut g, p, 259, None);
        let s = unit(&mut g, p, 414, None);
        let a = unit(&mut g, 1 - p, 70, Some(0));
        let b = unit(&mut g, 1 - p, 70, Some(0));
        move_to(&mut g, s, Some(0));
        settle(&mut g);
        assert_eq!(damage(&g, a), 6);
        assert_eq!(damage(&g, b), 6);
    }
}

#[test]
fn lucian_assault_sum_is_not_might_and_includes_multiple_printed_and_granted_values() {
    for p in 0..2 {
        let mut g = position(p);
        let s = unit(&mut g, p, 415, None);
        attach(&mut g, s, 58);
        attach(&mut g, s, 123);
        attach(&mut g, s, 123);
        cast(&mut g, 293, None, vec![s]);
        settle(&mut g);
        let e = unit(&mut g, 1 - p, 70, Some(0));
        g.unit_mut(e).unwrap().buff = 20;
        assert_eq!(g.unit_keywords(g.unit_ref(s).unwrap()).unwrap().assault, 8);
        assert_eq!(g.unit_might(g.unit_ref(s).unwrap()).unwrap(), 4);
        move_to(&mut g, s, Some(0));
        choose(&mut g, e);
        focus(&mut g, p);
        cast(&mut g, 58, None, vec![]);
        g.apply(Action::ChooseTarget { target: Some(s) }).unwrap();
        settle(&mut g);
        assert_eq!(damage(&g, e), 8);
        assert!(g.unit_might(g.unit_ref(s).unwrap()).unwrap() > 8);
    }
}

#[test]
fn tusker_optional_move_targets_either_side_at_any_battlefield_but_not_base() {
    for p in 0..2 {
        for target_owner in 0..2 {
            for lane in 0..2 {
                let mut g = position(p);
                let s = unit(&mut g, p, 416, None);
                let t = unit(&mut g, target_owner, 70, Some(lane));
                let base = unit(&mut g, 1 - p, 70, None);
                move_to(&mut g, s, Some(0));
                assert!(matches!(
                    g.decision,
                    Some(Decision::TriggerTargets { card_id: 416, .. })
                ));
                reject(
                    &mut g,
                    Action::ChooseTriggerTargets {
                        targets: vec![base],
                    },
                );
                choose(&mut g, t);
                settle(&mut g);
                assert_eq!(damage(&g, t), 3);
            }
        }
    }
    for p in 0..2 {
        let (mut g, _, e) = attack(416, p);
        g.apply(Action::ChooseTriggerTargets { targets: vec![] })
            .unwrap();
        assert!(g.chain.is_empty());
        assert_eq!(damage(&g, e), 0);
    }
}

#[test]
fn tusker_live_shield_grants_stack_after_move_trigger_finalizes() {
    for p in 0..2 {
        let mut g = position(p);
        let s = unit(&mut g, p, 416, None);
        attach(&mut g, s, 124);
        attach(&mut g, s, 124);
        let t = unit(&mut g, 1 - p, 70, Some(1));
        g.unit_mut(t).unwrap().buff = 20;
        move_to(&mut g, s, Some(0));
        choose(&mut g, t);
        focus(&mut g, p);
        // Controlled cross-domain response: no current legal Body/Mind legend
        // admits Tusker plus Cloth Armor; legal deck coverage uses Ntofo.
        cast(&mut g, 120, None, vec![]);
        g.apply(Action::ChooseTarget { target: Some(s) }).unwrap();
        settle(&mut g);
        assert_eq!(g.unit_keywords(g.unit_ref(s).unwrap()).unwrap().shield, 9);
        assert_eq!(damage(&g, t), 9);
        assert_eq!(g.unit_might(g.unit_ref(s).unwrap()).unwrap(), 4); // Shield remains a reference outside defense.
    }
}

#[test]
fn tusker_target_moving_to_base_invalidates_any_battlefield_choice() {
    for p in 0..2 {
        let (mut g, _, t) = attack(416, p);
        choose(&mut g, t);
        focus(&mut g, 1 - p);
        cast(&mut g, 9, None, vec![t]);
        settle(&mut g);
        assert_eq!(damage(&g, t), 0);
    }
}

#[test]
fn source_loss_never_reads_same_name_replacement_or_bonus_damage() {
    for p in 0..2 {
        for card in [413, 414, 415, 416] {
            let (mut g, s, e) = attack(card, p);
            if card != 414 {
                choose(&mut g, e);
            }
            let item = g.chain.last().unwrap().clone();
            let Some(ChainAbility::CardTrigger {
                rule_index,
                context,
            }) = item.ability
            else {
                panic!("not trigger")
            };
            // Controlled physical-source departure/replacement, not an admitted control-transfer card.
            g.bounce(s);
            unit(&mut g, p, card, Some(0));
            unit(&mut g, p, 259, None);
            g.battlefields[0] = 3;
            g.resolve_card_trigger(p, card, rule_index, context, &item.targets)
                .unwrap();
            assert_eq!(damage(&g, e), 0);
        }
    }
}

#[test]
fn zero_and_negative_live_might_do_not_generate_bonus_packets() {
    for p in 0..2 {
        for card in [413, 414] {
            for delta in [-6, -100] {
                let (mut g, s, e) = attack(card, p);
                if card != 414 {
                    choose(&mut g, e);
                }
                let item = g.chain.last().unwrap().clone();
                let Some(ChainAbility::CardTrigger {
                    rule_index,
                    context,
                }) = item.ability
                else {
                    panic!("not trigger")
                };
                // Controlled live numeric boundary: undamaged zero-Might units remain
                // legal. An actual Eclipse response is covered separately below.
                g.unit_mut(s).unwrap().temporary_buff = delta;
                unit(&mut g, p, 259, None);
                g.battlefields[0] = 3;
                assert_eq!(g.unit_might(g.unit_ref(s).unwrap()).unwrap(), 0);
                g.resolve_card_trigger(p, card, rule_index, context, &item.targets)
                    .unwrap();
                assert_eq!(damage(&g, e), 0);
            }
        }
    }
}

#[test]
fn graves_packet_capacity_failure_rolls_back_the_entire_public_resolution() {
    for p in 0..2 {
        let (mut g, _, first) = attack(414, p);
        let second = unit(&mut g, 1 - p, 70, Some(0));
        // Representable marked-damage boundary; no cleanup runs before resolving
        // the already-finalized ability. The second packet overflows after first.
        g.unit_mut(second).unwrap().damage = u64::MAX - 2;
        g.apply(Action::Pass).unwrap();
        let before = format!("{g:?}");
        let err = g.apply(Action::Pass).unwrap_err();
        assert!(err.contains("marked damage"), "{err}");
        assert_eq!(format!("{g:?}"), before);
        assert_eq!(damage(&g, first), 0);
    }
}

#[test]
fn actual_eclipse_then_flurry_kills_source_before_its_ability_without_last_known_damage() {
    for p in 0..2 {
        for card in [414, 415, 416] {
            let (mut g, source, enemy) = attack(card, p);
            if card != 414 {
                choose(&mut g, enemy);
            }
            focus(&mut g, 1 - p);
            cast(&mut g, 52, None, vec![source]);
            pair(&mut g);
            assert!(matches!(g.decision, Some(Decision::Predict { .. })));
            g.apply(Action::Choose { accept: true }).unwrap();
            assert_eq!(g.unit_might(g.unit_ref(source).unwrap()).unwrap(), 0);
            focus(&mut g, 1 - p);
            cast(&mut g, 297, None, vec![]); // Flurry deals1 to every battlefield unit.
            settle(&mut g);
            assert!(g.unit_ref(source).is_none());
            assert_eq!(damage(&g, enemy), 1); // Only Flurry; the source ability has no live referent.
        }
    }
}

#[test]
fn tusker_move_to_base_still_references_shield_and_can_damage_remote_field() {
    for p in 0..2 {
        let mut g = position(p);
        g.battlefield_owner = [Some(p), Some(1 - p)];
        let source = unit(&mut g, p, 416, Some(0));
        let target = unit(&mut g, 1 - p, 70, Some(1));
        cast(&mut g, 9, None, vec![source]);
        pair(&mut g);
        assert_eq!(g.unit_ref(source).unwrap().lane, None);
        choose(&mut g, target);
        settle(&mut g);
        assert_eq!(damage(&g, target), 3);
        assert_eq!(g.unit_might(g.unit_ref(source).unwrap()).unwrap(), 4);
    }
}

#[test]
fn actual_eclipse_zero_might_survives_and_graves_deals_no_bonus_damage() {
    for p in 0..2 {
        let (mut g, source, enemy) = attack(414, p);
        g.battlefields[0] = 3;
        unit(&mut g, p, 259, None);
        focus(&mut g, 1 - p);
        cast(&mut g, 52, None, vec![source]);
        pair(&mut g);
        g.apply(Action::Choose { accept: true }).unwrap();
        assert_eq!(g.unit_might(g.unit_ref(source).unwrap()).unwrap(), 0);
        settle(&mut g);
        assert!(g.unit_ref(source).is_some());
        assert_eq!(damage(&g, enemy), 0);
    }
}
