//! Registered C26 cards in controlled positions; actions use the public engine.
use super::*;
use crate::activated::{AbilitySource, AbilityTiming, activated_rules};

fn position(actor: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 260026, Difficulty::Tactical);
    g.start_match().unwrap();
    g.active = actor;
    g.priority_player = actor;
    g.battlefields = vec![0, 0];
    g.battlefield_owner = vec![Some(1 - actor), None];
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
    for legend in &mut g.legend {
        legend.ready = false;
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
fn enter(g: &mut Game, target: Target) {
    let Target::Unit { unit_id, .. } = target else {
        unreachable!()
    };
    g.apply(Action::Move {
        unit_id,
        lane: Some(0),
    })
    .unwrap();
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
fn pair(g: &mut Game) {
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn settle_chain(g: &mut Game) {
    for _ in 0..50 {
        match g.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => g
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            None if !g.chain.is_empty() => g.apply(Action::Pass).unwrap(),
            None => return,
            other => panic!("unexpected choice {other:?}"),
        }
    }
    panic!("chain did not settle")
}
fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action).is_err());
    assert_eq!(format!("{g:?}"), before);
}
fn ability(t: Target) -> Action {
    let Target::Unit { player, unit_id } = t else {
        unreachable!()
    };
    Action::ActivateAbility {
        source: AbilitySource::Unit { player, unit_id },
        ability_index: 0,
        targets: vec![],
    }
}
fn ezreal_combat(owner: usize, defending: bool) -> (Game, Target, Target) {
    let attacker = if defending { 1 - owner } else { owner };
    let mut g = position(attacker);
    let ez = unit(&mut g, owner, 412, defending.then_some(0));
    let enemy = unit(&mut g, 1 - owner, 70, (!defending).then_some(0));
    enter(&mut g, if defending { enemy } else { ez });
    assert!(matches!(
        g.decision,
        Some(Decision::TriggerTargets { card_id: 412, .. })
    ));
    (g, ez, enemy)
}
fn choose_ezreal(g: &mut Game, enemy: Target) {
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![enemy],
    })
    .unwrap();
    assert_eq!(g.chain.last().unwrap().card_id, 412);
}

#[test]
fn rengar_attacking_player_permission_survives_last_friendly_departure_in_both_seats() {
    for actor in 0..2 {
        let mut g = position(actor);
        let attacker = unit(&mut g, actor, 71, None);
        unit(&mut g, 1 - actor, 70, Some(0));
        assert!(g.validate_play_structure(411, Some(0)).is_err());
        enter(&mut g, attacker);
        cast(&mut g, 9, None, vec![attacker]); // Flash: return sole attacker to Base.
        pair(&mut g);
        assert_eq!(g.lane_count(actor, 0), 0);
        assert_eq!(g.battlefield_owner[0], Some(1 - actor));
        g.apply(Action::Pass).unwrap();
        assert_eq!(g.priority(), actor);
        assert!(g.validate_play_structure(411, Some(0)).is_ok());
        assert!(g.validate_play_structure(411, Some(1)).is_err());
        cast(&mut g, 411, Some(0), vec![]);
        // Permanents enter when their Play finalizes; unlike spells, the unit
        // itself does not wait on the response chain.
        let rengar = g.players[actor]
            .units
            .iter()
            .find(|u| u.card_id == 411)
            .unwrap();
        assert_eq!(rengar.lane, Some(0));
        assert!(!rengar.ready);
        assert_eq!(g.unit_might(rengar).unwrap(), 5);
        assert_eq!(g.unit_combat_contribution(rengar).unwrap().damage, 5);
    }
}

#[test]
fn rengar_permission_is_not_ambush_a_staged_field_or_a_defender_permission() {
    for actor in 0..2 {
        for combat in [false, true] {
            let mut g = position(actor);
            unit(&mut g, actor, 71, Some(0));
            unit(&mut g, 1 - actor, 70, Some(0));
            g.battlefield_owner = vec![None; 2];
            g.showdown = Some(Showdown {
                lane: 0,
                attacker: 1 - actor,
                focus: actor,
                passes: 0,
                combat,
            });
            assert!(g.validate_play_structure(411, Some(0)).is_err());
            g.showdown.as_mut().unwrap().attacker = actor;
            assert_eq!(g.validate_play_structure(411, Some(0)).is_ok(), combat);
            assert!(g.validate_play_structure(411, None).is_ok());
            g.showdown = None;
            g.pending_entries.push((0, actor));
            assert!(g.validate_play_structure(411, Some(0)).is_err());
        }
    }
}

#[test]
fn rengar_respects_costs_closed_assignments_and_mandatory_destinations() {
    for actor in 0..2 {
        let mut g = position(actor);
        let a = unit(&mut g, actor, 71, None);
        unit(&mut g, 1 - actor, 70, Some(0));
        enter(&mut g, a);
        g.players[actor].hand.push(411);
        let play = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: Some(0),
            targets: vec![],
        };
        let energy = g.players[actor].energy;
        g.players[actor].energy = 0;
        g.players[actor].energy_pool = 0;
        reject(&mut g, play.clone());
        g.players[actor].energy = energy;
        g.players[actor].energy_pool = energy;
        for destination in [PlayDestination::Base, PlayDestination::Exact(Some(1))] {
            let instruction = PlayInstruction {
                energy_discount: 0,
                cost: BaseCostWaiver::All,
                destination,
            };
            assert!(
                instruction
                    .validate_destination(Some(0), g.battlefield_count())
                    .is_err()
            );
        }
        pair(&mut g);
        assert!(matches!(g.decision, Some(Decision::CombatDamage { .. })));
        reject(&mut g, play);
    }
}

#[test]
fn rengar_instructed_play_does_not_replace_a_required_base_or_exact_destination() {
    for actor in 0..2 {
        for (kind, destination) in [
            (InstructedPlayKind::TemporalBreach, Some(1)),
            (InstructedPlayKind::PortalRescue, None),
        ] {
            let mut g = position(actor);
            let attacker = unit(&mut g, actor, 71, None);
            unit(&mut g, 1 - actor, 70, Some(0));
            enter(&mut g, attacker);
            // Controlled pending-child boundary using the actual printed
            // instruction's destination; finalization below is public.
            g.players[actor].banish.push(411);
            g.queue_banish_play(actor, 0, kind, Some(1));
            g.after_resolution().unwrap();
            assert!(matches!(
                g.decision,
                Some(Decision::PendingPlay { card_id: 411, .. })
            ));
            let finalize = |lane| Action::FinalizePendingPlay {
                lane,
                targets: vec![],
                repeat_targets: vec![],
                repeat: false,
                accelerate: false,
                bonus: false,
            };
            reject(&mut g, finalize(Some(0)));
            g.apply(finalize(destination)).unwrap();
            let rengar = g.players[actor]
                .units
                .iter()
                .find(|u| u.card_id == 411)
                .unwrap();
            assert_eq!(rengar.lane, destination);
            assert_eq!(g.unit_might(rengar).unwrap(), 3);
            assert!(g.pending_cards().is_empty());
        }
    }
}

#[test]
fn rengar_can_enter_during_conquer_responses_without_repeating_finished_combat() {
    for actor in 0..2 {
        let mut g = position(actor);
        let attacker = unit(&mut g, actor, 56, None); // Inferna, Assault 2.
        let Target::Unit { unit_id, .. } = attacker else {
            unreachable!()
        };
        g.players[actor].gear.push(GearState {
            id: 60000,
            card_id: 201,
            lane: None,
            attached_to: Some(unit_id),
            ready: true,
            temporary: false,
            token: false,
        }); // Cull supplies a real Conquer response chain.
        unit(&mut g, 1 - actor, 374, Some(0));
        enter(&mut g, attacker);
        pair(&mut g);
        for _ in 0..2 {
            let Some(Decision::CombatDamage {
                player,
                lane,
                total,
            }) = g.decision
            else {
                break;
            };
            let assignments = g.automatic_damage_assignment(player, lane, total).unwrap();
            g.apply(Action::AssignDamage { assignments }).unwrap();
        }
        g.apply(Action::ChooseTriggerTargets { targets: vec![] })
            .unwrap();
        let result = g.combat_resolution().unwrap();
        assert_eq!(result.step, CombatStep::End);
        assert_eq!(result.result, Some(CombatResult::Winner { player: actor }));
        assert_eq!(g.players[actor].score, 1);
        if g.priority() != actor {
            g.apply(Action::Pass).unwrap();
        }
        cast(&mut g, 9, None, vec![attacker]);
        pair(&mut g);
        assert_eq!(g.lane_count(actor, 0), 0);
        // Control remains established while this combat's Conquer effects
        // finish, even after the winning unit leaves.
        assert_eq!(g.battlefield_owner[0], Some(actor));
        if g.priority() != actor {
            g.apply(Action::Pass).unwrap();
        }
        assert!(!g.showdown_focus_open());
        cast(&mut g, 411, Some(0), vec![]);
        let rengar = g.players[actor]
            .units
            .iter()
            .find(|u| u.card_id == 411)
            .unwrap();
        assert_eq!(g.unit_might(rengar).unwrap(), 5);
        assert_eq!(g.combat_resolution().unwrap().result, result.result);
        assert_eq!(g.players[actor].score, 1);
        settle_chain(&mut g);
        assert!(g.combat_resolution().is_none());
        assert!(g.showdown.is_none());
        assert_eq!(g.players[actor].score, 1);
        let rengar = g.players[actor]
            .units
            .iter()
            .find(|u| u.card_id == 411)
            .unwrap();
        assert_eq!(g.unit_might(rengar).unwrap(), 3);
        assert_eq!(
            g.log
                .iter()
                .filter(|line| line.contains(" combat:"))
                .count(),
            1
        );
    }
}

#[test]
fn ezreal_unit_card_is_not_action_or_reaction_despite_its_activated_action() {
    for actor in 0..2 {
        let mut g = position(actor);
        assert!(g.validate_play_structure(412, None).is_ok());
        let a = unit(&mut g, actor, 71, None);
        unit(&mut g, 1 - actor, 70, Some(0));
        enter(&mut g, a);
        assert!(g.validate_play_structure(412, None).is_err());
        cast(&mut g, 7, None, vec![a]);
        assert!(g.validate_play_structure(412, None).is_err());
        assert_eq!(activated_rules(412)[0].timing, AbilityTiming::Action);
    }
}

#[test]
fn ezreal_attack_and_defend_damage_uses_live_might_and_remains_respondable() {
    for owner in 0..2 {
        for defending in [false, true] {
            let (mut g, ez, enemy) = ezreal_combat(owner, defending);
            choose_ezreal(&mut g, enemy);
            assert_eq!(g.unit_ref(enemy).unwrap().damage, 0);
            // A real Reaction changes source Might before the queued trigger resolves.
            cast(&mut g, 7, None, vec![ez]);
            settle_chain(&mut g);
            let might = g.unit_might(g.unit_ref(ez).unwrap()).unwrap();
            assert!(might > 3);
            assert_eq!(g.unit_ref(enemy).unwrap().damage, might);
            let c = g.unit_combat_contribution(g.unit_ref(ez).unwrap()).unwrap();
            assert_eq!(
                (c.might, c.damage, c.stunned, c.prevents_combat_damage),
                (might, 0, false, true)
            );
        }
    }
}

#[test]
fn ezreal_here_and_source_identity_recheck_after_movement_or_departure() {
    for owner in 0..2 {
        for defending in [false, true] {
            for move_source in [false, true] {
                let (mut g, ez, enemy) = ezreal_combat(owner, defending);
                choose_ezreal(&mut g, enemy);
                let moving = if move_source { ez } else { enemy };
                // Flash may choose only a friendly unit; give its controller priority.
                let Target::Unit { player, .. } = moving else {
                    unreachable!()
                };
                if g.priority() != player {
                    g.apply(Action::Pass).unwrap();
                }
                cast(&mut g, 9, None, vec![moving]);
                settle_chain(&mut g);
                assert_eq!(g.unit_ref(enemy).unwrap().damage, 0);
            }
        }
    }
    for owner in 0..2 {
        let (mut g, ez, enemy) = ezreal_combat(owner, false);
        choose_ezreal(&mut g, enemy);
        // Controlled source removal plus a same-name new object must not supply
        // the departed source's Here or Might to its already-finalized trigger.
        g.bounce(ez);
        unit(&mut g, owner, 412, Some(0));
        g.sync_combat_triggers();
        // Inspect the original immutable physical-source context directly;
        // the new object's own trigger is independent and remains pending.
        let item = g.chain.last().unwrap().clone();
        let Some(ChainAbility::CardTrigger {
            rule_index,
            context,
        }) = item.ability
        else {
            panic!("{item:?}")
        };
        g.resolve_card_trigger(owner, 412, rule_index, context, &[enemy])
            .unwrap();
        assert_eq!(g.unit_ref(enemy).unwrap().damage, 0);
    }
}

#[test]
fn ezreal_trigger_counter_and_deflect_are_finalization_costs() {
    for owner in 0..2 {
        let (mut g, _, enemy) = ezreal_combat(owner, false);
        choose_ezreal(&mut g, enemy);
        let id = g.chain.last().unwrap().id;
        if g.priority() == owner {
            g.apply(Action::Pass).unwrap();
        }
        cast(&mut g, 12, None, vec![Target::Spell { chain_id: id }]);
        settle_chain(&mut g);
        assert_eq!(g.unit_ref(enemy).unwrap().damage, 0);
        let mut g = position(owner);
        let ez = unit(&mut g, owner, 412, None);
        let target = unit(&mut g, 1 - owner, 213, Some(0)); // Storm Raptor: Deflect.
        assert!(
            g.unit_keywords(g.unit_ref(target).unwrap())
                .unwrap()
                .deflect
                > 0
        );
        enter(&mut g, ez);
        let before = g.players[owner].any_power + g.players[owner].power_pool.iter().sum::<u16>();
        choose_ezreal(&mut g, target);
        let after = g.players[owner].any_power + g.players[owner].power_pool.iter().sum::<u16>();
        assert!(after < before);
        assert_eq!(g.unit_ref(target).unwrap().damage, 0);
    }
}

#[test]
fn ezreal_zero_contribution_is_independent_of_stun_zero_might_and_teammates() {
    for owner in 0..2 {
        let mut g = position(owner);
        let ez = unit(&mut g, owner, 412, Some(0));
        let ally = unit(&mut g, owner, 71, Some(0));
        assert_eq!(g.lane_might(owner, 0).unwrap(), 5);
        g.unit_mut(ez).unwrap().stunned = true;
        let c = g.unit_combat_contribution(g.unit_ref(ez).unwrap()).unwrap();
        assert_eq!(
            (c.might, c.damage, c.stunned, c.prevents_combat_damage),
            (3, 0, true, true)
        );
        g.unit_mut(ez).unwrap().stunned = false;
        assert_eq!(g.lane_might(owner, 0).unwrap(), 5);
        g.unit_mut(ally).unwrap().stunned = true;
        assert_eq!(g.lane_might(owner, 0).unwrap(), 0);
        g.unit_mut(ez).unwrap().temporary_buff = -3;
        let c = g.unit_combat_contribution(g.unit_ref(ez).unwrap()).unwrap();
        assert_eq!(
            (c.might, c.damage, c.stunned, c.prevents_combat_damage),
            (0, 0, false, true)
        );
        g.unit_mut(ally).unwrap().stunned = false;
        assert_eq!(g.lane_might(owner, 0).unwrap(), 5);
    }
}

#[test]
fn ezreal_receives_combat_damage_against_full_might_and_still_deals_fight_damage() {
    for owner in 0..2 {
        let (mut g, ez, enemy) = ezreal_combat(owner, false);
        choose_ezreal(&mut g, enemy);
        pair(&mut g);
        pair(&mut g);
        let Some(Decision::CombatDamage { player, total, .. }) = g.decision else {
            panic!("{:?}", g.decision)
        };
        assert_eq!(player, 1 - owner);
        assert_eq!(total, 10);
        assert!(g.log.iter().any(|line| line
            == "Battlefield 1 combat: 0 damage vs 10 damage. Assign damage before either side is hit."));
        let Target::Unit { unit_id, .. } = ez else {
            unreachable!()
        };
        assert_eq!(
            g.automatic_damage_assignment(player, 0, total).unwrap(),
            vec![(unit_id, total)]
        );
        g.apply(Action::AssignDamage {
            assignments: vec![(unit_id, total)],
        })
        .unwrap();
        assert!(g.unit_ref(ez).is_none());
        assert!(g.unit_ref(enemy).is_some());
        assert_eq!(g.players[owner].score, 0);

        let mut g = position(owner);
        let ez = unit(&mut g, owner, 412, None);
        let enemy = unit(&mut g, 1 - owner, 71, None);
        cast(&mut g, 82, None, vec![ez, enemy]); // Challenge is noncombat Unit damage.
        settle_chain(&mut g);
        assert!(g.unit_ref(ez).is_none());
        assert_eq!(g.unit_ref(enemy).unwrap().damage, 3);
    }
}

#[test]
fn ezreal_self_move_is_paid_respondable_action_without_exhaust_or_recall() {
    for owner in 0..2 {
        let mut g = position(owner);
        let ez = unit(&mut g, owner, 412, None);
        reject(&mut g, ability(ez)); // Base-to-Base is not a legal Move.
        g.unit_mut(ez).unwrap().lane = Some(0);
        g.unit_mut(ez).unwrap().ready = false;
        g.battlefield_owner[0] = Some(owner);
        let before = g.players[owner].power_pool[domain_index(Domain::Mind)];
        g.apply(ability(ez)).unwrap();
        assert_eq!(
            g.players[owner].power_pool[domain_index(Domain::Mind)],
            before - 1
        );
        assert_eq!(g.unit_ref(ez).unwrap().lane, Some(0));
        assert!(!g.unit_ref(ez).unwrap().ready);
        assert_eq!(g.chain.len(), 1);
        reject(&mut g, ability(ez)); // Action cannot be added to an open chain.
        settle_chain(&mut g);
        assert_eq!(g.unit_ref(ez).unwrap().lane, None);
        assert!(!g.unit_ref(ez).unwrap().ready);
    }
}

#[test]
fn ezreal_untargeted_move_rejects_not_so_fast_and_departure_or_lock_does_not_refund() {
    for owner in 0..2 {
        for mode in 0..3 {
            let mut g = position(owner);
            let ez = unit(&mut g, owner, 412, Some(0));
            g.battlefield_owner[0] = Some(owner);
            let before = g.players[owner].power_pool[domain_index(Domain::Mind)];
            g.apply(ability(ez)).unwrap();
            match mode {
                0 => {
                    let id = g.chain.last().unwrap().id;
                    let actor = g.priority();
                    let hand_index = g.players[actor].hand.len();
                    g.players[actor].hand.push(12);
                    reject(
                        &mut g,
                        Action::PlayCardWithTargets {
                            hand_index,
                            lane: None,
                            targets: vec![Target::Spell { chain_id: id }],
                        },
                    );
                }
                1 => {
                    g.bounce(ez);
                    unit(&mut g, owner, 412, Some(0));
                }
                _ => {
                    let Target::Unit { unit_id, .. } = ez else {
                        unreachable!()
                    };
                    g.movement_locks.push((owner, unit_id));
                }
            }
            settle_chain(&mut g);
            assert!(
                g.players[owner]
                    .units
                    .iter()
                    .any(|u| u.card_id == 412 && u.lane == if mode == 0 { None } else { Some(0) })
            );
            assert_eq!(
                g.players[owner].power_pool[domain_index(Domain::Mind)],
                before - 1
            );
        }
    }
}

#[test]
fn ezreal_first_designation_is_once_per_physical_unit_per_combat() {
    for owner in 0..2 {
        let (mut g, ez, enemy) = ezreal_combat(owner, false);
        choose_ezreal(&mut g, enemy);
        pair(&mut g);
        assert_eq!(g.unit_ref(enemy).unwrap().damage, 3);
        g.apply(ability(ez)).unwrap();
        pair(&mut g);
        g.apply(Action::Pass).unwrap();
        cast(&mut g, 14, Some(0), vec![ez]);
        pair(&mut g);
        assert!(g.decision.is_none());
        assert_eq!(g.unit_ref(enemy).unwrap().damage, 3);
    }
}

#[test]
fn ezreal_zero_pools_complete_cleanup_and_recall_without_damage_choices() {
    for owner in 0..2 {
        let (mut g, ez, enemy) = ezreal_combat(owner, false);
        g.unit_mut(enemy).unwrap().stunned = true;
        choose_ezreal(&mut g, enemy);
        pair(&mut g);
        assert_eq!(g.unit_ref(enemy).unwrap().damage, 3);
        pair(&mut g);
        assert!(g.decision.is_none());
        assert!(g.showdown.is_none());
        assert_eq!(g.unit_ref(ez).unwrap().lane, None);
        assert_eq!(g.unit_ref(enemy).unwrap().damage, 0);
        assert_eq!(g.players[owner].score, 0);
        assert!(!g.unit_ref(ez).unwrap().stunned);
        assert!(
            g.unit_combat_contribution(g.unit_ref(ez).unwrap())
                .unwrap()
                .prevents_combat_damage
        );
    }
}

#[test]
fn ezreal_no_legal_first_designation_target_is_not_rechecked_later_in_combat() {
    for owner in 0..2 {
        for defending in [false, true] {
            let attacker = if defending { 1 - owner } else { owner };
            let mut g = position(attacker);
            let ez = unit(&mut g, owner, 412, Some(0));
            g.showdown = Some(Showdown {
                lane: 0,
                attacker,
                focus: attacker,
                passes: 0,
                combat: true,
            });
            // Controlled boundary fixture: no valid target at the first actual
            // designation checkpoint. A later entry is not another first check.
            g.sync_combat_triggers();
            g.after_resolution().unwrap();
            assert!(g.decision.is_none());
            assert!(g.chain.is_empty());
            unit(&mut g, 1 - owner, 70, Some(0));
            g.sync_combat_triggers();
            g.after_resolution().unwrap();
            assert!(g.decision.is_none());
            assert!(g.chain.is_empty());
            let Target::Unit { unit_id, .. } = ez else {
                unreachable!()
            };
            assert!(g.trigger_state.combat_seen.contains(&unit_id));
        }
    }
}

#[test]
fn ezreal_move_staged_payment_and_tactical_escape_use_the_existing_action_contract() {
    for owner in 0..2 {
        let (mut g, ez, enemy) = ezreal_combat(owner, false);
        choose_ezreal(&mut g, enemy);
        pair(&mut g);
        assert_eq!(g.choose_ai_action().unwrap(), ability(ez));
        let before = g.players[owner].power_pool[domain_index(Domain::Mind)];
        g.apply(Action::BeginPayment {
            action: Box::new(ability(ez)),
        })
        .unwrap();
        assert!(g.chain.is_empty());
        assert_eq!(g.unit_ref(ez).unwrap().lane, Some(0));
        g.apply(Action::CompletePayment).unwrap();
        assert_eq!(
            g.players[owner].power_pool[domain_index(Domain::Mind)],
            before - 1
        );
        assert_eq!(g.unit_ref(ez).unwrap().lane, Some(0));
        pair(&mut g);
        assert_eq!(g.unit_ref(ez).unwrap().lane, None);
    }
}
