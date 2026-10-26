//! Actual registered cards in controlled positions, driven through real actions.
use super::*;
use crate::gear::GearState;

fn position(player: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 240024, Difficulty::Tactical);
    g.start_match().unwrap();
    g.active = player;
    g.priority_player = player;
    g.battlefields = [3, 3];
    g.battlefield_owner = [None; 2];
    for p in &mut g.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.deck = vec![71; 12];
        p.runes = [0; 6];
        p.exhausted_runes = [0; 6];
        p.energy_pool = 100;
        p.energy = 100;
        p.power_pool = [20; 6];
        p.any_power = 0;
    }
    for legend in &mut g.legend {
        legend.ready = false;
    }
    g.trigger_state.draws = [0; 2];
    g
}
fn unit(g: &mut Game, player: usize, card_id: usize, lane: Option<usize>) -> Target {
    let unit_id = g.create_unit(player, card_id, false, 0);
    let t = Target::Unit { player, unit_id };
    let unit = g.unit_mut(t).unwrap();
    unit.lane = lane;
    unit.ready = true;
    t
}
fn gear(g: &mut Game, player: usize, card_id: usize, lane: Option<usize>) -> Target {
    let gear_id = g.next_unit_id;
    g.next_unit_id += 1;
    g.players[player].gear.push(GearState {
        id: gear_id,
        card_id,
        lane,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    Target::Gear { player, gear_id }
}
fn gid(t: Target) -> u32 {
    if let Target::Gear { gear_id, .. } = t {
        gear_id
    } else {
        panic!()
    }
}
fn cast(g: &mut Game, id: usize, lane: Option<usize>, targets: Vec<Target>, mode: Option<usize>) {
    let p = g.priority();
    let hand_index = g.players[p].hand.len();
    g.players[p].hand.push(id);
    let action = Action::PlayCardWithTargets {
        hand_index,
        lane,
        targets,
    };
    let action = if mode.is_some() {
        Action::WithPlayChoices {
            action: Box::new(action),
            choices: PlayChoices {
                mode,
                ..Default::default()
            },
        }
    } else {
        action
    };
    g.apply(action).unwrap_or_else(|e| panic!("cast {id}: {e}"));
}
fn pass_pair(g: &mut Game) {
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn drive(g: &mut Game) {
    for _ in 0..100 {
        match g.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => g
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            Some(Decision::TokenPlay { .. } | Decision::TokenPlacement { .. }) => {
                g.apply(Action::PlaceToken { lane: None }).unwrap()
            }
            Some(Decision::Legend { .. }) => g.apply(Action::Choose { accept: false }).unwrap(),
            None if !g.chain.is_empty() => g.apply(Action::Pass).unwrap(),
            None => return,
            other => panic!("unexpected decision {other:?}"),
        }
    }
    panic!("did not settle")
}
fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action).is_err());
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn deploy_enters_every_field_without_contesting_controlling_or_scoring() {
    for player in 0..2 {
        for lane in 0..2 {
            for owner in [None, Some(player), Some(1 - player)] {
                let mut g = position(player);
                g.battlefield_owner[lane] = owner;
                if let Some(owner) = owner {
                    unit(&mut g, owner, 71, Some(lane));
                }
                g.players[player].hand.push(401);
                reject(
                    &mut g,
                    Action::PlayCard {
                        hand_index: 0,
                        lane: None,
                        target: None,
                    },
                );
                g.apply(Action::PlayCard {
                    hand_index: 0,
                    lane: Some(lane),
                    target: None,
                })
                .unwrap();
                assert_eq!(g.players[player].gear[0].lane, Some(lane));
                assert_eq!(
                    g.gear_location(player, g.players[player].gear[0].id),
                    Some(lane)
                );
                assert_eq!(g.battlefield_owner[lane], owner);
                assert!(!g.contested(lane));
                assert!(g.showdown.is_none());
                assert_eq!(g.players[player].score, 0);
            }
        }
    }
}

#[test]
fn opponent_hold_kills_exact_gear_once_even_after_it_moves_but_conquer_does_not() {
    for player in 0..2 {
        let mut g = position(player);
        let first = gear(&mut g, player, 401, Some(0));
        let second = gear(&mut g, player, 401, Some(1));
        unit(&mut g, 1 - player, 71, Some(0));
        unit(&mut g, 1 - player, 71, Some(0));
        g.battlefield_owner[0] = Some(1 - player);
        g.score_triggers(1 - player, 0, false).unwrap();
        g.after_resolution().unwrap();
        assert!(g.chain.is_empty());
        g.score_triggers(1 - player, 0, true).unwrap();
        g.after_resolution().unwrap();
        assert_eq!(g.chain.iter().filter(|i| i.card_id == 401).count(), 1);
        assert!(g.board_target_facts(first).is_some());
        g.players[player].gear[0].lane = Some(1); // response-time move; identity stays pinned
        drive(&mut g);
        assert!(g.board_target_facts(first).is_none());
        assert!(g.board_target_facts(second).is_some());
        assert_eq!(g.players[player].discard, vec![401]);
    }
}

#[test]
fn local_passives_stack_and_traveler_uses_existential_characteristic_not_printed_activity() {
    for p in 0..2 {
        let mut g = position(p);
        let traveler = unit(&mut g, p, 409, Some(0));
        let other = unit(&mut g, p, 71, Some(1));
        let banner = gear(&mut g, p, 401, Some(0));
        gear(&mut g, p, 401, Some(0));
        gear(&mut g, p, 404, Some(0));
        gear(&mut g, p, 406, Some(0));
        assert_eq!(
            g.try_unit_stats(g.unit_ref(traveler).unwrap())
                .unwrap()
                .might,
            3
        );
        let keywords = g.unit_keywords(g.unit_ref(traveler).unwrap()).unwrap();
        assert_eq!(keywords.assault, 2);
        assert_eq!(keywords.shield, 2);
        assert!(keywords.ganking);
        assert_eq!(
            g.unit_keywords(g.unit_ref(other).unwrap()).unwrap().assault,
            0
        );
        g.players[p].gear.retain(|x| x.id == gid(banner));
        let Target::Unit { unit_id, .. } = traveler else {
            unreachable!()
        };
        g.players[p].gear[0].attached_to = Some(unit_id);
        let status = g.gear_deploy_status(p, gid(banner)).unwrap();
        assert!(status.characteristic);
        assert!(!status.printed_abilities_active);
        assert!(!status.opponent_hold_trigger_active);
        assert_eq!(
            g.try_unit_stats(g.unit_ref(traveler).unwrap())
                .unwrap()
                .might,
            3
        );
        assert_eq!(
            g.unit_keywords(g.unit_ref(traveler).unwrap())
                .unwrap()
                .assault,
            0
        );
        g.unit_mut(traveler).unwrap().lane = None;
        assert_eq!(
            g.try_unit_stats(g.unit_ref(traveler).unwrap())
                .unwrap()
                .might,
            2
        );
    }
}

#[test]
fn poster_payment_precedes_responses_and_decline_creates_no_token_or_chain() {
    for p in 0..2 {
        for accept in [false, true] {
            let mut g = position(p);
            cast(&mut g, 403, Some(0), vec![], None);
            let Some(Decision::TriggerPayment {
                choice_id,
                ref base_cost,
                ..
            }) = g.decision
            else {
                panic!("{:?}", g.decision)
            };
            assert_eq!(base_cost.energy, 1);
            assert!(g.chain.is_empty());
            assert!(g.players[p].units.is_empty());
            let energy = g.players[p].energy;
            reject(
                &mut g,
                Action::ResolveTriggerPayment {
                    choice_id: choice_id + 100,
                    pay: true,
                },
            );
            g.apply(Action::ResolveTriggerPayment {
                choice_id,
                pay: accept,
            })
            .unwrap();
            assert_eq!(g.players[p].energy, energy - u16::from(accept));
            assert_eq!(g.chain.len(), usize::from(accept));
            assert!(g.players[p].units.is_empty());
            drive(&mut g);
            assert_eq!(g.players[p].units.len(), usize::from(accept));
            if accept {
                let u = &g.players[p].units[0];
                assert!(u.token);
                assert_eq!(u.lane, None);
                assert_eq!(g.unit_might(u).unwrap(), 1);
                assert!(g.unit_has_tag(u, "Recruit"));
            }
        }
    }
}

#[test]
fn portal_decline_before_targets_and_pinned_payment_rechecks_live_here() {
    for p in 0..2 {
        let mut g = position(p);
        let t = unit(&mut g, 1 - p, 71, Some(0));
        g.unit_mut(t).unwrap().ready = false;
        cast(&mut g, 406, Some(0), vec![], None);
        assert!(matches!(
            g.decision,
            Some(Decision::TriggerTargets { card_id: 406, .. })
        ));
        let mut declined = g.clone();
        declined
            .apply(Action::ChooseTriggerTargets { targets: vec![] })
            .unwrap();
        assert!(declined.chain.is_empty());
        g.apply(Action::ChooseTriggerTargets { targets: vec![t] })
            .unwrap();
        let Some(Decision::TriggerPayment {
            choice_id,
            ref targets,
            ref base_cost,
            ..
        }) = g.decision
        else {
            panic!()
        };
        assert_eq!(targets, &[t]);
        assert_eq!(base_cost.domains, vec![Domain::Body]);
        assert!(!g.unit_ref(t).unwrap().ready);
        let power = g.players[p].power_pool[domain_index(Domain::Body)];
        g.apply(Action::ResolveTriggerPayment {
            choice_id,
            pay: true,
        })
        .unwrap();
        assert_eq!(
            g.players[p].power_pool[domain_index(Domain::Body)],
            power - 1
        );
        assert!(!g.unit_ref(t).unwrap().ready);
        g.players[p].gear[0].lane = Some(1); // Here is live at effect execution.
        pass_pair(&mut g);
        assert!(!g.unit_ref(t).unwrap().ready);
        assert_eq!(
            g.players[p].power_pool[domain_index(Domain::Body)],
            power - 1
        );
    }
}

#[test]
fn trigger_payment_plan_cancel_preserves_exact_choice_and_rolls_back_add() {
    for p in 0..2 {
        let mut g = position(p);
        let gold = gear(&mut g, p, 13, None);
        g.players[p].gear[0].token = true;
        let target = unit(&mut g, 1 - p, 287, Some(0)); // Deflect2: base Body1 + generic2
        cast(&mut g, 406, Some(0), vec![], None);
        g.players[p].power_pool = [0; 6];
        g.players[p].any_power = 0;
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target],
        })
        .unwrap();
        let original = g.trigger_payment_context().unwrap();
        let Decision::TriggerPayment { choice_id, .. } = original else {
            panic!()
        };
        let energy = g.players[p].energy;
        g.apply(Action::BeginPayment {
            action: Box::new(Action::ResolveTriggerPayment {
                choice_id,
                pay: true,
            }),
        })
        .unwrap();
        assert_eq!(g.trigger_payment_context(), Some(original.clone()));
        g.apply(Action::ActivateGear { gear_id: gid(gold) })
            .unwrap();
        assert!(g.board_target_facts(gold).is_none());
        assert_eq!(g.trigger_payment_context(), Some(original.clone()));
        assert!(g.can_cancel_payment().unwrap());
        g.apply(Action::Choose { accept: false }).unwrap();
        assert_eq!(g.decision, Some(original));
        assert!(g.board_target_facts(gold).is_some());
        assert_eq!(g.players[p].energy, energy);
        assert_eq!(g.players[p].any_power, 0);
        g.players[p].power_pool[domain_index(Domain::Body)] = 3;
        g.apply(Action::BeginPayment {
            action: Box::new(Action::ResolveTriggerPayment {
                choice_id,
                pay: true,
            }),
        })
        .unwrap();
        g.apply(Action::CompletePayment).unwrap();
        assert_eq!(g.chain.len(), 1);
        assert_eq!(g.players[p].power_pool[domain_index(Domain::Body)], 0);
    }
}

#[test]
fn command_queries_mixed_slots_moves_exact_copy_and_rejects_invalid_modes_atomically() {
    for p in 0..2 {
        let mut g = position(p);
        let first = gear(&mut g, p, 401, Some(0));
        let second = gear(&mut g, p, 401, Some(0));
        g.players[p].hand.push(410);
        let draft = |targets| Action::WithPlayChoices {
            action: Box::new(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets,
            }),
            choices: PlayChoices {
                mode: Some(0),
                ..Default::default()
            },
        };
        let v = g.play_target_view(&draft(vec![]), 0).unwrap();
        assert_eq!(v.next_slot.unwrap().kind, rules::PlayTargetKind::Gear);
        assert_eq!(v.candidates, vec![first, second]);
        let v = g.play_target_view(&draft(vec![second]), 0).unwrap();
        assert_eq!(
            v.next_slot.unwrap().kind,
            rules::PlayTargetKind::Battlefield
        );
        assert_eq!(v.candidates, vec![Target::Battlefield { lane: 1 }]);
        reject(&mut g, draft(vec![second, Target::Battlefield { lane: 0 }]));
        reject(
            &mut g,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![second, Target::Battlefield { lane: 1 }],
            },
        );
        g.apply(draft(vec![second, Target::Battlefield { lane: 1 }]))
            .unwrap();
        assert_eq!(g.chain[0].executions[0].mode, Some(0));
        pass_pair(&mut g);
        assert_eq!(g.gear_location(p, gid(first)), Some(0));
        assert_eq!(g.gear_location(p, gid(second)), Some(1));
        assert_eq!(g.battlefield_owner, [None; 2]);
    }
}

#[test]
fn command_draw_inspects_five_occurrences_and_records_only_real_draw() {
    for p in 0..2 {
        let mut g = position(p);
        g.players[p].deck = vec![71, 401, 71, 401, 71, 403];
        cast(&mut g, 410, None, vec![], Some(1));
        pass_pair(&mut g);
        let Some(Decision::DrawLook {
            choice_id,
            ref cards,
            ref eligible,
            kind,
            ..
        }) = g.decision
        else {
            panic!()
        };
        assert_eq!(kind, DrawLookKind::CommandAttack);
        assert_eq!(cards, &[403, 71, 401, 71, 401]);
        assert_eq!(eligible, &[0, 2, 4]);
        assert_eq!(g.trigger_state.draws[p], 0);
        g.apply(Action::ResolveDrawLook {
            choice_id,
            index: Some(4),
        })
        .unwrap();
        assert_eq!(g.trigger_state.draws[p], 1);
        assert_eq!(g.players[p].hand, vec![401]);
        assert!(matches!(g.decision, Some(Decision::RevealedCard { .. })));
        g.apply(Action::Choose { accept: true }).unwrap();
        assert!(g.chain.is_empty());
        assert_eq!(g.players[p].deck.len(), 5);
    }
}

#[test]
fn attached_deploy_is_a_legal_command_target_but_cannot_move_separately() {
    for p in 0..2 {
        let mut g = position(p);
        let bearer = unit(&mut g, p, 71, Some(1));
        let source = gear(&mut g, p, 401, Some(0));
        let Target::Unit { unit_id, .. } = bearer else {
            unreachable!()
        };
        g.players[p].gear[0].attached_to = Some(unit_id);
        g.players[p].hand.push(410);
        let action = Action::WithPlayChoices {
            action: Box::new(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![source, Target::Battlefield { lane: 0 }],
            }),
            choices: PlayChoices {
                mode: Some(0),
                ..Default::default()
            },
        };
        g.apply(action).unwrap();
        pass_pair(&mut g);
        assert_eq!(g.gear_location(p, gid(source)), Some(1));
        assert_eq!(g.unit_ref(bearer).unwrap().lane, Some(1));
        assert_eq!(g.players[p].discard, vec![410]);
    }
}

#[test]
fn hunting_blind_requires_a_different_live_battlefield_and_moves_only_chosen_enemy() {
    for p in 0..2 {
        for source_leaves in [false, true] {
            let mut g = position(p);
            let here = unit(&mut g, 1 - p, 71, Some(0));
            let elsewhere = unit(&mut g, 1 - p, 71, None);
            cast(&mut g, 404, Some(0), vec![], None);
            assert!(!g.valid_trigger_target(here, &[]).unwrap());
            assert!(g.valid_trigger_target(elsewhere, &[]).unwrap());
            reject(
                &mut g,
                Action::ChooseTriggerTargets {
                    targets: vec![here],
                },
            );
            g.apply(Action::ChooseTriggerTargets {
                targets: vec![elsewhere],
            })
            .unwrap();
            if source_leaves {
                let source = Target::Gear {
                    player: p,
                    gear_id: g.players[p].gear[0].id,
                };
                g.return_board_targets(&[source]);
            }
            drive(&mut g);
            assert_eq!(
                g.unit_ref(elsewhere).unwrap().lane,
                if source_leaves { None } else { Some(0) }
            );
            assert_eq!(g.unit_ref(here).unwrap().lane, Some(0));
        }
    }
}

#[test]
fn portal_cost_remains_paid_when_not_so_fast_counters_its_finalized_trigger() {
    for p in 0..2 {
        let mut g = position(p);
        let target = unit(&mut g, 1 - p, 71, Some(0));
        g.unit_mut(target).unwrap().ready = false;
        cast(&mut g, 406, Some(0), vec![], None);
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target],
        })
        .unwrap();
        let Some(Decision::TriggerPayment { choice_id, .. }) = g.decision else {
            panic!()
        };
        let power = g.players[p].power_pool[domain_index(Domain::Body)];
        g.apply(Action::ResolveTriggerPayment {
            choice_id,
            pay: true,
        })
        .unwrap();
        let id = g.chain[0].id;
        if g.priority() == p {
            g.apply(Action::Pass).unwrap();
        }
        assert_eq!(g.priority(), 1 - p);
        cast(&mut g, 12, None, vec![Target::Spell { chain_id: id }], None);
        drive(&mut g);
        assert!(!g.unit_ref(target).unwrap().ready);
        assert_eq!(
            g.players[p].power_pool[domain_index(Domain::Body)],
            power - 1
        );
    }
}

#[test]
fn portal_accepts_ready_targets_but_never_spell_only_power_for_its_base_cost() {
    for p in 0..2 {
        let mut g = position(p);
        let target = unit(&mut g, p, 71, Some(0));
        cast(&mut g, 406, Some(0), vec![], None);
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target],
        })
        .unwrap();
        let Some(Decision::TriggerPayment { choice_id, .. }) = g.decision else {
            panic!()
        };
        g.players[p].power_pool = [0; 6];
        g.legend[p].spell_power = 3;
        reject(
            &mut g,
            Action::ResolveTriggerPayment {
                choice_id,
                pay: true,
            },
        );
        g.players[p].power_pool[domain_index(Domain::Body)] = 1;
        g.apply(Action::ResolveTriggerPayment {
            choice_id,
            pay: true,
        })
        .unwrap();
        drive(&mut g);
        assert!(g.unit_ref(target).unwrap().ready);
        assert_eq!(g.legend[p].spell_power, 3);
    }
}

#[test]
fn dreamroot_waits_for_combat_result_and_draw_finishes_before_conquer() {
    for p in 0..2 {
        let mut g = position(p);
        let attacker = unit(&mut g, p, 71, None);
        unit(&mut g, 1 - p, 56, Some(0));
        let root = gear(&mut g, p, 402, Some(0));
        g.battlefield_owner[0] = Some(1 - p);
        let Target::Unit { unit_id, .. } = attacker else {
            unreachable!()
        };
        g.apply(Action::Move {
            unit_id,
            lane: Some(0),
        })
        .unwrap();
        pass_pair(&mut g);
        for _ in 0..2 {
            if let Some(Decision::CombatDamage {
                player,
                lane,
                total,
            }) = g.decision
            {
                let assignments = g.automatic_damage_assignment(player, lane, total).unwrap();
                g.apply(Action::AssignDamage { assignments }).unwrap();
            }
        }
        assert_eq!(
            g.combat_resolution().unwrap().step,
            CombatStep::EstablishControl
        );
        assert_eq!(g.chain.iter().filter(|i| i.card_id == 402).count(), 1);
        assert_eq!(g.trigger_state.draws[p], 0);
        assert_eq!(g.battlefield_owner[0], Some(1 - p));
        assert_eq!(g.players[p].score, 0);
        g.return_board_targets(&[root]); // event condition was captured; Draw needs no live Here
        drive(&mut g);
        assert_eq!(g.trigger_state.draws[p], 1);
        assert_eq!(g.battlefield_owner[0], Some(p));
        assert_eq!(g.players[p].score, 1);
        assert!(g.combat_resolution().is_none());
    }
}

#[test]
fn dreamroot_does_not_trigger_for_uncontested_conquest_or_mutual_wipe() {
    for p in 0..2 {
        for combat in [false, true] {
            let mut g = position(p);
            let attacker = unit(&mut g, p, 71, None);
            gear(&mut g, p, 402, Some(0));
            if combat {
                unit(&mut g, 1 - p, 71, Some(0));
                g.battlefield_owner[0] = Some(1 - p);
            }
            let Target::Unit { unit_id, .. } = attacker else {
                unreachable!()
            };
            g.apply(Action::Move {
                unit_id,
                lane: Some(0),
            })
            .unwrap();
            if combat {
                pass_pair(&mut g);
                for _ in 0..2 {
                    if let Some(Decision::CombatDamage {
                        player,
                        lane,
                        total,
                    }) = g.decision
                    {
                        let assignments =
                            g.automatic_damage_assignment(player, lane, total).unwrap();
                        g.apply(Action::AssignDamage { assignments }).unwrap();
                    }
                }
            }
            drive(&mut g);
            assert_eq!(g.trigger_state.draws[p], 0);
            assert!(g.chain.is_empty());
        }
    }
}

#[test]
fn armory_buffs_friendly_units_anywhere_on_play_and_its_field_conquest() {
    for p in 0..2 {
        let mut g = position(p);
        let base = unit(&mut g, p, 71, None);
        let other = unit(&mut g, p, 71, Some(1));
        cast(&mut g, 405, Some(0), vec![], None);
        assert!(g.trigger_target_candidates().unwrap().contains(&base));
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![base],
        })
        .unwrap();
        drive(&mut g);
        assert_eq!(g.unit_ref(base).unwrap().buff, 1);
        g.score_triggers(p, 1, false).unwrap();
        g.after_resolution().unwrap();
        assert!(g.decision.is_none());
        assert!(g.chain.is_empty());
        g.score_triggers(p, 0, false).unwrap();
        g.after_resolution().unwrap();
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![other],
        })
        .unwrap();
        drive(&mut g);
        assert_eq!(g.unit_ref(other).unwrap().buff, 1);
    }
}

#[test]
fn extractor_condition_is_checked_at_own_ending_and_gold_enters_exhausted_at_base() {
    for p in 0..2 {
        let mut g = position(p);
        gear(&mut g, p, 407, Some(0));
        unit(&mut g, p, 71, Some(0));
        g.battlefield_owner[0] = Some(p);
        g.emit_event(EventSnapshot::simple(Occurrence::ControllerEnding, 1 - p));
        g.after_resolution().unwrap();
        assert!(g.chain.is_empty());
        g.emit_event(EventSnapshot::simple(Occurrence::ControllerEnding, p));
        g.after_resolution().unwrap();
        assert_eq!(g.chain.len(), 1);
        g.players[p].gear[0].lane = Some(1); // no repeated resolution-side control condition
        drive(&mut g);
        let gold = g.players[p].gear.iter().find(|g| g.token).unwrap();
        assert!(!gold.ready);
        assert_eq!(gold.lane, None);
        assert!(!g.gear_deploy_status(p, gold.id).unwrap().characteristic);
    }
}

#[test]
fn good_stuff_discards_then_draws_and_hold_returns_the_exact_source() {
    for p in 0..2 {
        let mut g = position(p);
        g.players[p].hand = vec![71];
        cast(&mut g, 408, Some(0), vec![], None);
        pass_pair(&mut g);
        assert!(matches!(
            g.decision,
            Some(Decision::Discard {
                count: 1,
                draw_after: 3,
                ..
            })
        ));
        assert_eq!(g.trigger_state.draws[p], 0);
        g.apply(Action::SelectCards { indices: vec![0] }).unwrap();
        assert_eq!(g.trigger_state.draws[p], 3);
        assert_eq!(g.players[p].hand.len(), 3);
        let source = Target::Gear {
            player: p,
            gear_id: g.players[p].gear[0].id,
        };
        let other = gear(&mut g, p, 408, Some(1));
        g.score_triggers(p, 0, true).unwrap();
        g.after_resolution().unwrap();
        assert!(g.board_target_facts(source).is_some());
        drive(&mut g);
        assert!(g.board_target_facts(source).is_none());
        assert!(g.board_target_facts(other).is_some());
        assert_eq!(g.players[p].hand.iter().filter(|&&c| c == 408).count(), 1);
    }
}

#[test]
fn command_repeat_preserves_independent_modes_targets_and_private_draw_continuation() {
    for p in 0..2 {
        let mut g = position(p);
        let source = gear(&mut g, p, 401, Some(0));
        g.players[p].hand = vec![410];
        g.grant_play(
            p,
            ChainSource::Card { card_id: 254 },
            PlayGrantKind::RepeatNextSpell,
        );
        let action = Action::WithPlayChoices {
            action: Box::new(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![source, Target::Battlefield { lane: 1 }],
            }),
            choices: PlayChoices {
                mode: Some(0),
                ..Default::default()
            },
        };
        let repeat = g.play_choices(&action).unwrap().repeats[0].id;
        let Action::WithPlayChoices {
            action,
            mut choices,
        } = action
        else {
            unreachable!()
        };
        choices.repeats.push(RepeatSelection {
            option_id: repeat,
            choices: ExecutionChoices {
                mode: Some(1),
                targets: vec![],
            },
        });
        g.apply(Action::WithPlayChoices { action, choices })
            .unwrap();
        assert_eq!(
            g.chain[0]
                .executions
                .iter()
                .map(|e| e.mode)
                .collect::<Vec<_>>(),
            vec![Some(0), Some(1)]
        );
        pass_pair(&mut g);
        assert_eq!(g.gear_location(p, gid(source)), Some(1));
        let Some(Decision::DrawLook { choice_id, .. }) = g.decision else {
            panic!()
        };
        assert_eq!(g.resolution_progress().unwrap().execution, 1);
        g.apply(Action::ResolveDrawLook {
            choice_id,
            index: None,
        })
        .unwrap();
        assert!(g.chain.is_empty());
        assert_eq!(g.players[p].discard, vec![410]);
        assert_eq!(g.trigger_state.draws[p], 0);
    }
}

#[test]
fn hidden_gear_enters_its_field_then_normal_cleanup_recalls_it_without_deploy() {
    for p in 0..2 {
        let mut g = position(p);
        // Zhonya's Hourglass is actual Hidden gear. Entry observes the field, then the
        // explicit normal cleanup recalls it; playing at Base would lose facts.
        g.trigger_state.hidden_play = true;
        g.gear_play_at(p, 21, Some(1)).unwrap();
        g.trigger_state.hidden_play = false;
        let id = g.players[p].gear[0].id;
        assert_eq!(g.gear_location(p, id), Some(1));
        assert_eq!(g.play_history.units_or_gears_here[p][1], 0); // primitive entry does not fake a Play action
        assert!(!g.gear_deploy_status(p, id).unwrap().characteristic);
        g.after_resolution().unwrap();
        assert_eq!(g.gear_location(p, id), None);
        assert!(g.players[p].discard.is_empty());
    }
}

#[test]
fn simple_ai_selects_useful_portal_targets_and_explicit_command_modes() {
    for p in 0..2 {
        let mut g = position(p);
        let ours = unit(&mut g, p, 71, Some(0));
        g.unit_mut(ours).unwrap().ready = false;
        unit(&mut g, 1 - p, 70, Some(0));
        cast(&mut g, 406, Some(0), vec![], None);
        let chosen = g.choose_heuristic_action().unwrap();
        assert!(
            matches!(&chosen, Action::ChooseTriggerTargets { targets } if *targets == vec![ours])
        );
        g.apply(chosen).unwrap();
        let payment = g.choose_heuristic_action().unwrap();
        assert!(matches!(
            payment,
            Action::ResolveTriggerPayment { pay: true, .. }
        ));
        g.apply(payment).unwrap();
        assert!(!g.unit_ref(ours).unwrap().ready);
        drive(&mut g);
        assert!(g.unit_ref(ours).unwrap().ready);

        let mut g = position(p);
        g.players[p].hand.push(410);
        let chosen = g.choose_heuristic_action().unwrap();
        assert!(
            matches!(&chosen, Action::WithPlayChoices { choices, .. } if choices.mode == Some(1))
        );
        g.apply(chosen).unwrap();
        assert_eq!(g.chain.last().unwrap().executions[0].mode, Some(1));

        let mut g = position(p);
        unit(&mut g, 1 - p, 70, Some(1));
        cast(&mut g, 404, Some(0), vec![], None);
        assert!(
            matches!(g.choose_heuristic_action().unwrap(),Action::ChooseTriggerTargets{targets} if targets.is_empty())
        );
    }
}

#[test]
fn departing_bearer_leaves_deploy_gear_at_the_field_while_ordinary_gear_recalls() {
    for p in 0..2 {
        let mut g = position(p);
        let bearer = unit(&mut g, p, 71, Some(1));
        let Target::Unit { unit_id, .. } = bearer else {
            unreachable!()
        };
        let deploy = gear(&mut g, p, 401, None);
        let ordinary = gear(&mut g, p, 10, None);
        for attachment in &mut g.players[p].gear {
            attachment.attached_to = Some(unit_id);
        }
        g.gear_unit_left_board(p, unit_id);
        g.players[p].units.retain(|u| u.id != unit_id);
        assert_eq!(g.gear_location(p, gid(deploy)), Some(1));
        assert_eq!(g.gear_location(p, gid(ordinary)), Some(1));
        g.cleanup_gear_locations();
        assert_eq!(g.gear_location(p, gid(deploy)), Some(1));
        assert_eq!(g.gear_location(p, gid(ordinary)), None);
        assert!(
            g.gear_deploy_status(p, gid(deploy))
                .unwrap()
                .opponent_hold_trigger_active
        );
    }
}

#[test]
fn funded_trigger_cost_can_cancel_and_direct_add_decline_restores_resources_only() {
    for p in 0..2 {
        for staged in [false, true] {
            let mut g = position(p);
            let target = unit(&mut g, p, 71, Some(0));
            g.unit_mut(target).unwrap().ready = false;
            g.battlefield_owner[0] = Some(p);
            let gold = g.gear_gold(p, false);
            cast(&mut g, 406, Some(0), vec![], None);
            g.apply(Action::ChooseTriggerTargets {
                targets: vec![target],
            })
            .unwrap();
            g.players[p].power_pool = [0; 6];
            g.players[p].runes[domain_index(Domain::Body)] = 1;
            g.players[p].energy += 1;
            let original = g.decision.clone().unwrap();
            let Decision::TriggerPayment { choice_id, .. } = original else {
                panic!()
            };
            let before_player = format!("{:?}", g.players[p]);
            let before_entry = g.trigger_state.clone();
            if staged {
                g.apply(Action::BeginPayment {
                    action: Box::new(Action::ResolveTriggerPayment {
                        choice_id,
                        pay: true,
                    }),
                })
                .unwrap();
                // Recycling the ready Body rune can already fund this cost.
                assert!(g.can_cancel_payment().unwrap());
            }
            g.apply(Action::ActivateGear { gear_id: gold }).unwrap();
            g.apply(Action::ActivateRune {
                domain: Domain::Body,
                recycle: false,
            })
            .unwrap();
            g.apply(Action::RecycleRune {
                domain: Domain::Body,
                exhausted: true,
            })
            .unwrap();
            assert!(!g.players[p].gear.iter().any(|gear| gear.id == gold));
            assert_eq!(g.trigger_payment_context(), Some(original.clone()));
            if staged {
                assert!(g.can_cancel_payment().unwrap());
                g.apply(Action::Choose { accept: false }).unwrap();
                assert_eq!(g.decision, Some(original.clone()));
                assert_eq!(format!("{:?}", g.players[p]), before_player);
            }
            g.apply(Action::ResolveTriggerPayment {
                choice_id,
                pay: false,
            })
            .unwrap();
            assert!(g.decision.is_none());
            assert!(g.chain.is_empty());
            assert!(g.trigger_payment_origin.is_none());
            assert_eq!(format!("{:?}", g.players[p]), before_player);
            assert_eq!(
                format!("{:?}", g.trigger_state),
                format!("{before_entry:?}")
            );
            assert!(!g.unit_ref(target).unwrap().ready);
            assert!(g.players[p].gear.iter().any(|gear| gear.card_id == 406));
        }
    }
}
