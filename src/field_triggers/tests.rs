//! Actual C28 cards and fields in controlled positions; actions remain public.
//! Direct event scheduling below isolates timing checkpoints from deck draws.
use super::*;
use crate::legends::LegendKind;

fn position(player: usize, field: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 280028, Difficulty::Tactical);
    g.start_match().unwrap();
    g.active = player;
    g.priority_player = player;
    g.battlefields = [field, 0];
    g.battlefield_owner = [Some(player), None];
    for p in &mut g.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.deck = vec![71; 30];
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
fn unit_id(target: Target) -> u32 {
    let Target::Unit { unit_id, .. } = target else {
        unreachable!()
    };
    unit_id
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
fn settle(g: &mut Game) {
    for _ in 0..60 {
        match g.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => g
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            None if !g.chain.is_empty() => g.apply(Action::Pass).unwrap(),
            _ => return,
        }
    }
    panic!("did not settle");
}
fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action).is_err());
    assert_eq!(before, format!("{g:?}"));
}
fn dais(g: &mut Game, player: usize, target: Target) -> u32 {
    g.queue_dais_cost(player, 0);
    g.after_resolution().unwrap();
    let Some(Decision::BattlefieldCost { choice_id, .. }) = g.decision else {
        panic!("{:?}", g.decision)
    };
    assert!(g.chain.is_empty());
    g.apply(Action::SelectBattlefieldCost {
        choice_id,
        unit_id: Some(unit_id(target)),
    })
    .unwrap();
    choice_id
}

#[test]
fn dais_composite_cost_precedes_responses_and_controlled_counter_does_not_refund() {
    for p in 0..2 {
        let mut g = position(p, 11);
        let u = unit(&mut g, p, 71, Some(0));
        let choice_id = dais(&mut g, p, u);
        let energy = g.players[p].energy;
        g.apply(Action::ResolveBattlefieldPayment {
            choice_id,
            pay: true,
        })
        .unwrap();
        assert!(g.unit_ref(u).is_none());
        assert_eq!(g.players[p].hand, [71]);
        assert_eq!(g.players[p].energy, energy - 1);
        assert!(g.pending_token.is_none());
        let item = g.chain.last().unwrap();
        assert!(item.targets.is_empty());
        assert_eq!(
            g.chain_source(item),
            ChainSource::Battlefield {
                lane: 0,
                battlefield_id: 11
            }
        );
        let chain_id = item.id;
        if g.priority() != 1 - p {
            g.apply(Action::Pass).unwrap();
        }
        // No admitted counter can choose an untargeted battlefield ability:
        // Not So Fast requires it to choose one of the countering player's
        // units/gear. The generic counter executor is a controlled boundary.
        g.counter_spell(chain_id, false);
        g.after_resolution().unwrap();
        assert!(g.chain.is_empty());
        assert!(g.pending_token.is_none());
        assert_eq!(g.players[p].hand, [71]);
        assert_eq!(g.players[p].energy, energy - 1);
        assert!(g.players[p].units.is_empty());
    }
}

#[test]
fn dais_resolves_token_only_after_response_and_retains_actual_field_source() {
    for p in 0..2 {
        let mut g = position(p, 11);
        unit(&mut g, p, 265, None); // Rek'Sai makes token Accelerate optional.
        let u = unit(&mut g, p, 71, Some(0));
        let choice_id = dais(&mut g, p, u);
        g.apply(Action::ResolveBattlefieldPayment {
            choice_id,
            pay: true,
        })
        .unwrap();
        settle(&mut g);
        assert!(matches!(g.decision, Some(Decision::TokenPlay { .. })));
        let token = g.pending_token.as_ref().unwrap();
        assert_eq!(
            token.source,
            Some(ChainSource::Battlefield {
                lane: 0,
                battlefield_id: 11
            })
        );
        g.apply(Action::FinalizeTokenPlay {
            lane: Some(0),
            accelerate: false,
        })
        .unwrap();
        assert_eq!(g.players[p].units.len(), 2);
        let u = g.players[p].units.iter().find(|u| u.token).unwrap();
        assert!(u.token);
        assert_eq!(u.lane, Some(0));
        assert_eq!(g.unit_might(u).unwrap(), 2);
    }
}

#[test]
fn dais_zero_resource_component_still_returns_unit_and_can_decline_composite_cost() {
    for p in 0..2 {
        let mut g = position(p, 11);
        let u = unit(&mut g, p, 71, Some(0));
        let choice_id = dais(&mut g, p, u);
        // Controlled discount boundary; there is no admitted field-cost reducer.
        if let Some(Decision::BattlefieldPayment { cost, .. }) = &mut g.decision {
            cost.energy = 0;
        }
        let before = format!("{g:?}");
        g.apply(Action::BeginPayment {
            action: Box::new(Action::ResolveBattlefieldPayment {
                choice_id,
                pay: true,
            }),
        })
        .unwrap();
        assert!(g.can_cancel_payment().unwrap());
        assert!(g.can_refuse_payment().unwrap());
        g.apply(Action::Choose { accept: false }).unwrap();
        assert_eq!(before, format!("{g:?}"));
        g.apply(Action::ResolveBattlefieldPayment {
            choice_id,
            pay: false,
        })
        .unwrap();
        assert!(g.unit_ref(u).is_some());
        assert!(g.chain.is_empty());
        let choice_id = dais(&mut g, p, u);
        if let Some(Decision::BattlefieldPayment { cost, .. }) = &mut g.decision {
            cost.energy = 0;
        }
        let energy = g.players[p].energy;
        g.apply(Action::ResolveBattlefieldPayment {
            choice_id,
            pay: true,
        })
        .unwrap();
        assert!(g.unit_ref(u).is_none());
        assert_eq!(g.players[p].energy, energy);
    }
}

#[test]
fn spring_consumes_first_nontoken_play_even_when_declined_and_excludes_enterer() {
    for p in 0..2 {
        let mut g = position(p, 12);
        let other = unit(&mut g, p, 71, Some(0));
        cast(&mut g, 71, Some(0), vec![]);
        settle(&mut g);
        let Some(Decision::BattlefieldTargets {
            choice_id,
            kind: BattlefieldTargetKind::StarSpring { played_unit_id },
            ..
        }) = g.decision
        else {
            panic!("{:?}", g.decision)
        };
        assert_eq!(g.battlefield_choice_candidates(), vec![other]);
        assert!(g.star_spring_used()[p][0]);
        reject(
            &mut g,
            Action::ResolveBattlefieldTargets {
                choice_id,
                targets: vec![Target::Unit {
                    player: p,
                    unit_id: played_unit_id,
                }],
            },
        );
        g.apply(Action::ResolveBattlefieldTargets {
            choice_id,
            targets: vec![],
        })
        .unwrap();
        cast(&mut g, 71, Some(0), vec![]);
        settle(&mut g);
        assert!(g.decision.is_none());
        assert_eq!(g.unit_ref(other).unwrap().lane, Some(0));
    }
}

#[test]
fn peak_delays_selection_and_uses_exact_rune_objects_from_either_player() {
    for p in 0..2 {
        let mut g = position(p, 14);
        for state in &mut g.players {
            state
                .set_rune_setup([1, 0, 0, 0, 0, 0], [1, 0, 0, 0, 0, 0])
                .unwrap();
        }
        g.queue_ability(
            p,
            0,
            vec![],
            ChainAbility::Battlefield(BattlefieldAbility::TargonPeakDelay { lane: 0 }),
        );
        g.after_resolution().unwrap();
        settle(&mut g);
        assert!(g.decision.is_none());
        assert_eq!(g.delayed_battlefield_triggers().len(), 1);
        let a = g.players[p].rune_objects()[0].id;
        let b = g.players[1 - p].rune_objects()[0].id;
        g.begin_ending();
        g.after_resolution().unwrap();
        let Some(Decision::BattlefieldTargets {
            choice_id,
            kind: BattlefieldTargetKind::TargonPeakReady,
            ..
        }) = g.decision
        else {
            panic!("{:?}", g.decision)
        };
        g.apply(Action::ResolveBattlefieldTargets {
            choice_id,
            targets: vec![
                Target::Rune {
                    player: p,
                    rune_id: a,
                },
                Target::Rune {
                    player: 1 - p,
                    rune_id: b,
                },
            ],
        })
        .unwrap();
        // A response recycles one selected object; a new same-domain channel
        // must not inherit the pending target. Controlled direct response here
        // isolates identity from the unrelated channel-card instructions.
        g.activate_rune_object(p, a, true).unwrap();
        g.players[p].rune_deck = vec![Domain::Fury];
        g.channel_runes(p, 1, true).unwrap();
        let replacement = g.players[p].rune_objects()[0].id;
        assert_ne!(a, replacement);
        // Resolve this ability without finishing Ending/Awaken, which would
        // legitimately ready the next player's entire board.
        g.resolve_top_chain().unwrap();
        assert!(!g.players[p].rune_objects()[0].ready);
        assert!(g.players[1 - p].rune_objects()[0].ready);
    }
}

#[test]
fn peak_created_after_ending_boundary_does_not_schedule_next_turn() {
    let mut g = position(0, 14);
    g.ending_turn = true;
    g.resolve_new_battlefield_ability(0, BattlefieldAbility::TargonPeakDelay { lane: 0 }, &[])
        .unwrap();
    assert!(g.delayed_battlefield_triggers().is_empty());
}

#[test]
fn zaun_keeps_resolving_parent_during_discard_and_draws_even_from_empty_hand() {
    for p in 0..2 {
        for has_card in [false, true] {
            let mut g = position(p, 16);
            if has_card {
                g.players[p].hand.push(70);
            }
            g.queue_ability(
                p,
                0,
                vec![],
                ChainAbility::Battlefield(BattlefieldAbility::ZaunWarrensLoot { lane: 0 }),
            );
            g.after_resolution().unwrap();
            settle(&mut g);
            assert!(matches!(
                g.decision,
                Some(Decision::Discard {
                    count: 1,
                    draw_after: 1,
                    ..
                })
            ));
            assert_eq!(g.chain.len(), 1);
            g.apply(Action::SelectCards {
                indices: if has_card { vec![0] } else { vec![] },
            })
            .unwrap();
            assert!(g.chain.is_empty());
            assert_eq!(g.players[p].hand, [71]);
            assert_eq!(g.players[p].discard.contains(&70), has_card);
        }
    }
}

#[test]
fn rune_legacy_and_exact_actions_preserve_identity_and_atomic_errors() {
    for p in 0..2 {
        let mut g = position(p, 0);
        g.players[p]
            .set_rune_setup([2, 0, 0, 0, 0, 0], [1, 0, 0, 0, 0, 0])
            .unwrap();
        g.players[p].refresh_rune_energy().unwrap();
        let old = g.players[p].rune_objects().to_vec();
        g.apply(Action::ActivateRune {
            domain: Domain::Fury,
            recycle: false,
        })
        .unwrap();
        assert_eq!(
            g.players[p]
                .rune_objects()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
            old.iter().map(|r| r.id).collect::<Vec<_>>()
        );
        assert!(g.players[p].rune_objects().iter().all(|r| !r.ready));
        reject(
            &mut g,
            Action::ActivateRuneObject {
                rune_id: old[1].id,
                recycle: false,
            },
        );
        g.apply(Action::ActivateRuneObject {
            rune_id: old[1].id,
            recycle: true,
        })
        .unwrap();
        assert_eq!(g.players[p].rune_objects().len(), 1);
        assert_eq!(g.players[p].rune_objects()[0].id, old[0].id);
        reject(
            &mut g,
            Action::ActivateRuneObject {
                rune_id: old[1].id,
                recycle: true,
            },
        );
    }
}

#[test]
fn field_target_selection_notifies_each_chosen_observer_once_and_dais_cost_never_does() {
    for p in 0..2 {
        for kind in [
            BattlefieldTargetKind::AbandonedHall,
            BattlefieldTargetKind::StarSpring {
                played_unit_id: u32::MAX,
            },
        ] {
            let mut g = position(p, 13);
            g.legend[p].kind = LegendKind::Irelia;
            let u = unit(&mut g, p, 0, Some(0)); // Irelia, Fervent and the Irelia legend both observe.
            g.queue_field_targets(p, 0, kind);
            g.after_resolution().unwrap();
            let Some(Decision::BattlefieldTargets { choice_id, .. }) = g.decision else {
                panic!()
            };
            g.apply(Action::ResolveBattlefieldTargets {
                choice_id,
                targets: vec![u],
            })
            .unwrap();
            let Some(Decision::OrderTriggers { ref options, .. }) = g.decision else {
                panic!("{:?}", g.decision)
            };
            assert_eq!(
                options.len(),
                2,
                "one Irelia unit and one Irelia legend occurrence"
            );
            assert_eq!(g.pending_triggers.len(), 2);
        }
        let mut g = position(p, 11);
        g.legend[p].kind = LegendKind::Irelia;
        let u = unit(&mut g, p, 0, Some(0));
        let choice_id = dais(&mut g, p, u);
        g.apply(Action::ResolveBattlefieldPayment {
            choice_id,
            pay: true,
        })
        .unwrap();
        assert!(g.pending_triggers.is_empty());
        assert!(g.decision.is_none());
        assert_eq!(g.chain.len(), 1);
    }
}

#[test]
fn faefolk_moves_to_captured_destination_after_source_flash_and_rejects_same_location() {
    for p in 0..2 {
        let mut g = position(p, 0);
        let source = unit(&mut g, p, 417, None);
        let enemy = unit(&mut g, 1 - p, 71, Some(1));
        let already = unit(&mut g, 1 - p, 71, Some(0));
        g.battlefield_owner[0] = Some(1 - p);
        g.apply(Action::Move {
            unit_id: unit_id(source),
            lane: Some(0),
        })
        .unwrap();
        assert!(matches!(
            g.decision,
            Some(Decision::TriggerTargets { card_id: 417, .. })
        ));
        reject(
            &mut g,
            Action::ChooseTriggerTargets {
                targets: vec![already],
            },
        );
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![enemy],
        })
        .unwrap();
        cast(&mut g, 9, None, vec![source]);
        settle(&mut g);
        assert_eq!(g.unit_ref(source).unwrap().lane, None);
        assert_eq!(g.unit_ref(enemy).unwrap().lane, Some(0));
    }
}

#[test]
fn kinkou_resolution_rechecks_other_live_might_and_excludes_itself() {
    for p in 0..2 {
        for boost in [false, true] {
            let mut g = position(p, 0);
            let other = unit(&mut g, p, 71, None); // Phantom 5.
            if boost {
                g.unit_mut(other).unwrap().temporary_buff = -1;
            }
            cast(&mut g, 418, None, vec![]);
            assert_eq!(g.chain.last().unwrap().card_id, 418);
            // Controlled continuous-stat response isolates the threshold's execution
            // timing; both source and other units remain physically present.
            g.unit_mut(other).unwrap().temporary_buff = if boost { 0 } else { -1 };
            let before = g.players[p].hand.len();
            settle(&mut g);
            assert_eq!(g.players[p].hand.len(), before + usize::from(boost));
        }
    }
}

#[test]
fn pyke_optional_power_cost_is_a_respondable_play_trigger_not_accelerate() {
    for p in 0..2 {
        for paid in [false, true] {
            let mut g = position(p, 0);
            let power = g.players[p].power_pool[0];
            g.players[p].hand.push(420);
            g.apply(Action::WithPlayChoices {
                action: Box::new(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                }),
                choices: PlayChoices {
                    bonus: paid,
                    ..PlayChoices::default()
                },
            })
            .unwrap();
            let u = &g.players[p].units[0];
            assert!(!u.ready);
            assert_eq!(g.unit_might(u).unwrap(), 2);
            assert_eq!(g.players[p].power_pool[0], power - u16::from(paid));
            assert_eq!(g.chain.len(), usize::from(paid));
            settle(&mut g);
            let u = &g.players[p].units[0];
            assert_eq!(u.ready, paid);
            assert_eq!(g.unit_might(u).unwrap(), if paid { 4 } else { 2 });
        }
    }
}

#[test]
fn nidalee_combat_result_draw_precedes_conquer_and_requires_live_winner() {
    for p in 0..2 {
        let mut g = position(p, 0);
        g.battlefield_owner[0] = Some(1 - p);
        let cat = unit(&mut g, p, 419, None);
        unit(&mut g, 1 - p, 56, Some(0)); // Inferna defends with Might 1.
        g.apply(Action::Move {
            unit_id: unit_id(cat),
            lane: Some(0),
        })
        .unwrap();
        g.apply(Action::Pass).unwrap();
        g.apply(Action::Pass).unwrap();
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
        assert!(g.chain.iter().any(|i| i.card_id == 419));
        assert_eq!(g.players[p].score, 0);
        assert_eq!(g.players[p].hand.len(), 0);
        assert_eq!(g.battlefield_owner[0], Some(1 - p));
        settle(&mut g);
        assert_eq!(g.players[p].hand.len(), 1);
        assert_eq!(g.players[p].score, 1);
    }
}

#[test]
fn dais_direct_add_then_staged_undo_and_decline_restore_exact_rune_and_choice() {
    for p in 0..2 {
        let mut g = position(p, 11);
        let u = unit(&mut g, p, 71, Some(0));
        g.players[p]
            .set_rune_setup([1, 0, 0, 0, 0, 0], [0; 6])
            .unwrap();
        g.players[p].energy_pool = 0;
        g.players[p].refresh_rune_energy().unwrap();
        let choice_id = dais(&mut g, p, u);
        let state = g.players[p].clone();
        let rune_id = g.players[p].rune_objects()[0].id;
        g.apply(Action::ActivateRuneObject {
            rune_id,
            recycle: false,
        })
        .unwrap();
        let context = g.battlefield_payment_context().unwrap();
        let before_plan = format!("{g:?}");
        g.apply(Action::BeginPayment {
            action: Box::new(Action::ResolveBattlefieldPayment {
                choice_id,
                pay: true,
            }),
        })
        .unwrap();
        assert_eq!(g.battlefield_payment_context(), Some(context));
        assert!(g.can_cancel_payment().unwrap());
        g.apply(Action::Choose { accept: false }).unwrap();
        assert_eq!(format!("{g:?}"), before_plan);
        g.apply(Action::ResolveBattlefieldPayment {
            choice_id,
            pay: false,
        })
        .unwrap();
        assert_eq!(g.players[p], state);
        assert!(g.unit_ref(u).is_some());
        assert!(g.chain.is_empty());
    }
}

#[test]
fn hall_belongs_to_spell_controller_triggers_only_after_success_and_expires() {
    for p in 0..2 {
        let mut g = position(p, 13);
        g.battlefield_owner[0] = Some(1 - p);
        let ally = unit(&mut g, p, 71, Some(0));
        let enemy = unit(&mut g, 1 - p, 71, Some(0));
        cast(&mut g, 63, None, vec![enemy]);
        assert!(g.decision.is_none());
        let id = g.chain.last().unwrap().id;
        g.apply(Action::Pass).unwrap();
        cast(&mut g, 12, None, vec![Target::Spell { chain_id: id }]);
        settle(&mut g);
        // Countering Stupefy still completes Not So Fast for the opponent.
        let Some(Decision::BattlefieldTargets {
            choice_id,
            player,
            kind: BattlefieldTargetKind::AbandonedHall,
            ..
        }) = g.decision
        else {
            panic!("{:?}", g.decision)
        };
        assert_eq!(player, 1 - p);
        g.apply(Action::ResolveBattlefieldTargets {
            choice_id,
            targets: vec![],
        })
        .unwrap();
        settle(&mut g);
        assert!(g.decision.is_none());
        if g.priority() != p {
            g.apply(Action::Pass).unwrap();
        }
        cast(&mut g, 63, None, vec![enemy]);
        settle(&mut g);
        let Some(Decision::BattlefieldTargets {
            choice_id, player, ..
        }) = g.decision
        else {
            panic!()
        };
        assert_eq!(player, p);
        assert_eq!(g.battlefield_choice_candidates(), vec![ally]);
        let might = g.unit_might(g.unit_ref(ally).unwrap()).unwrap();
        g.apply(Action::ResolveBattlefieldTargets {
            choice_id,
            targets: vec![ally],
        })
        .unwrap();
        assert_eq!(g.unit_might(g.unit_ref(ally).unwrap()).unwrap(), might);
        settle(&mut g);
        assert_eq!(g.unit_might(g.unit_ref(ally).unwrap()).unwrap(), might + 1);
        assert_eq!(g.unit_ref(ally).unwrap().temporary_buff, 1);
        // Execute the canonical expiration checkpoint after this-turn effects.
        g.finish_ending().unwrap();
        assert_eq!(g.unit_ref(ally).unwrap().temporary_buff, 0);
        assert_eq!(g.unit_might(g.unit_ref(ally).unwrap()).unwrap(), might);
    }
}

#[test]
fn pyke_selected_optional_cost_still_counts_when_ezreal_reduces_it_to_zero() {
    for p in 0..2 {
        let mut g = position(p, 0);
        unit(&mut g, p, 35, None);
        g.players[p].hand.push(420);
        let action = Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: None,
        };
        let choices = PlayChoices {
            bonus: true,
            ..PlayChoices::default()
        };
        let view = g
            .play_choices(&Action::WithPlayChoices {
                action: Box::new(action.clone()),
                choices: choices.clone(),
            })
            .unwrap();
        let mut choices = choices;
        choices.cost_order = view
            .costs
            .modifiers
            .iter()
            .map(|m| CostSelection {
                modifier_id: m.id,
                use_power: true,
                power_allocations: vec![],
            })
            .collect();
        let view = g
            .play_choices(&Action::WithPlayChoices {
                action: Box::new(action.clone()),
                choices: choices.clone(),
            })
            .unwrap();
        assert_eq!(view.costs.power.iter().map(|p| p.amount).sum::<u16>(), 0);
        let before = g.players[p].power_pool;
        g.apply(Action::WithPlayChoices {
            action: Box::new(action),
            choices,
        })
        .unwrap();
        assert_eq!(g.players[p].power_pool, before);
        assert!(g.chain.iter().any(|i| i.card_id == 420));
        settle(&mut g);
        let pyke = g.players[p]
            .units
            .iter()
            .find(|u| u.card_id == 420)
            .unwrap();
        assert!(pyke.ready);
        assert_eq!(g.unit_might(pyke).unwrap(), 4);
    }
}

#[test]
fn physical_rune_capacity_and_mirror_mismatch_are_atomic_public_errors() {
    let mut g = position(0, 0);
    g.players[0]
        .set_rune_setup([1, 0, 0, 0, 0, 0], [0; 6])
        .unwrap();
    let rune_id = g.players[0].rune_objects()[0].id;
    g.players[0].power_pool[0] = u16::MAX;
    let before = format!("{g:?}");
    assert!(crate::numeric::is_capacity(
        &g.apply(Action::ActivateRuneObject {
            rune_id,
            recycle: true
        })
        .unwrap_err()
    ));
    assert_eq!(format!("{g:?}"), before);
    g.players[0].power_pool[0] = 0;
    g.players[0].runes[0] = 2; // Deliberately malformed mirror, never repaired by an action.
    let before = format!("{g:?}");
    assert!(
        g.apply(Action::ActivateRuneObject {
            rune_id,
            recycle: false
        })
        .unwrap_err()
        .contains("Rune object/count mismatch")
    );
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn field_ai_chooses_own_exhausted_runes_in_both_seats_and_declines_harmful_return() {
    for p in 0..2 {
        for difficulty in [Difficulty::Easy, Difficulty::Tactical] {
            let mut g = position(p, 14);
            g.difficulty = difficulty;
            for state in &mut g.players {
                state
                    .set_rune_setup([3, 0, 0, 0, 0, 0], [2, 0, 0, 0, 0, 0])
                    .unwrap();
            }
            g.queue_field_targets(p, 0, BattlefieldTargetKind::TargonPeakReady);
            g.after_resolution().unwrap();
            let action = g.choose_ai_action().unwrap();
            let Action::ResolveBattlefieldTargets { ref targets, .. } = action else {
                panic!("{action:?}")
            };
            assert_eq!(targets.len(), 2);
            assert!(
                targets
                    .iter()
                    .all(|t| matches!(t,Target::Rune { player,.. } if *player==p))
            );
            g.apply(action).unwrap();
            settle(&mut g);
            assert!(g.players[p].rune_objects().iter().all(|r| r.ready));
            assert_eq!(g.players[1 - p].exhausted_runes[0], 2);
            let mut g = position(p, 11);
            g.difficulty = difficulty;
            unit(&mut g, p, 70, Some(0));
            g.queue_dais_cost(p, 0);
            g.after_resolution().unwrap();
            assert!(matches!(
                g.choose_ai_action().unwrap(),
                Action::SelectBattlefieldCost { unit_id: None, .. }
            ));
        }
    }
}

#[test]
fn faefolk_event_destination_survives_actual_source_death_response() {
    for p in 0..2 {
        let mut g = position(p, 0);
        g.battlefield_owner[0] = Some(1 - p);
        let source = unit(&mut g, p, 417, None);
        let enemy = unit(&mut g, 1 - p, 71, Some(1));
        unit(&mut g, 1 - p, 71, Some(0));
        g.apply(Action::Move {
            unit_id: unit_id(source),
            lane: Some(0),
        })
        .unwrap();
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![enemy],
        })
        .unwrap();
        g.apply(Action::Pass).unwrap();
        cast(&mut g, 297, None, vec![]);
        settle(&mut g);
        assert!(g.unit_ref(source).is_none());
        assert_eq!(g.unit_ref(enemy).unwrap().lane, Some(0));
    }
}

#[test]
fn kinkou_threshold_uses_actual_might_response_and_quick_draw_equipment() {
    for p in 0..2 {
        for increase in [false, true] {
            let mut g = position(p, 0);
            let other = unit(&mut g, p, 71, None);
            if increase {
                g.unit_mut(other).unwrap().temporary_buff = -1;
            }
            cast(&mut g, 418, None, vec![]);
            if increase {
                cast(&mut g, 58, None, vec![]);
                g.apply(Action::ChooseTarget {
                    target: Some(other),
                })
                .unwrap();
            } else {
                g.apply(Action::Pass).unwrap();
                cast(&mut g, 63, None, vec![other]);
            }
            let cards = g.players[p].hand.len();
            settle(&mut g);
            assert_eq!(g.players[p].hand.len(), cards + usize::from(increase));
        }
    }
}

#[test]
fn nidalee_mutual_wipe_and_recall_are_no_result_with_no_draw() {
    for p in 0..2 {
        for stunned in [false, true] {
            let mut g = position(p, 0);
            g.battlefield_owner[0] = Some(1 - p);
            let a = unit(&mut g, p, 419, None);
            let d = unit(&mut g, 1 - p, 419, Some(0));
            g.unit_mut(a).unwrap().stunned = stunned;
            g.unit_mut(d).unwrap().stunned = stunned;
            g.apply(Action::Move {
                unit_id: unit_id(a),
                lane: Some(0),
            })
            .unwrap();
            g.apply(Action::Pass).unwrap();
            g.apply(Action::Pass).unwrap();
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
            assert!(g.chain.is_empty());
            assert!(g.decision.is_none());
            assert!(g.players.iter().all(|p| p.hand.is_empty()));
            assert!(g.log.iter().any(|line| line.contains("no result")));
            if stunned {
                assert_eq!(g.unit_ref(a).unwrap().lane, None);
            } else {
                assert!(g.unit_ref(a).is_none());
            }
        }
    }
}

#[test]
fn nidalee_ambush_after_damage_before_result_inherits_winning_combat() {
    for p in 0..2 {
        let mut g = position(p, 0);
        g.battlefield_owner[0] = Some(1 - p);
        let a = unit(&mut g, p, 71, None);
        unit(&mut g, 1 - p, 15, Some(0)); // Scuttle Deathknell suspends result.
        g.apply(Action::Move {
            unit_id: unit_id(a),
            lane: Some(0),
        })
        .unwrap();
        g.apply(Action::Pass).unwrap();
        g.apply(Action::Pass).unwrap();
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
        assert_eq!(
            g.combat_resolution().unwrap().step,
            CombatStep::DetermineResult
        );
        if g.priority() != p {
            g.apply(Action::Pass).unwrap();
        }
        cast(&mut g, 419, Some(0), vec![]);
        let cat = g.players[p]
            .units
            .iter()
            .find(|u| u.card_id == 419)
            .unwrap()
            .id;
        settle(&mut g);
        assert!(matches!(g.decision, Some(Decision::RevealHand { .. })));
        g.apply(Action::Choose { accept: true }).unwrap();
        assert!(g.chain.iter().any(|i| i.card_id == 419));
        assert!(
            g.unit_ref(Target::Unit {
                player: p,
                unit_id: cat
            })
            .is_some()
        );
        assert_eq!(g.players[p].score, 0);
        settle(&mut g);
        assert_eq!(g.players[p].hand.len(), 1);
    }
}

#[test]
fn shadow_temple_hold_burn_is_respondable_and_burnout_follows_canonical_draw_rules() {
    for p in 0..2 {
        let mut g = position(p, 15);
        g.players[p].deck = vec![70, 71, 72, 73];
        g.battlefield_on_hold(p, 0);
        g.after_resolution().unwrap();
        assert_eq!(g.players[p].deck.len(), 4);
        assert_eq!(g.chain.len(), 1);
        settle(&mut g);
        assert_eq!(g.players[p].deck, [70]);
        assert_eq!(g.players[p].discard, [73, 72, 71]);
        g.players[p].deck = vec![71];
        g.players[p].discard = vec![70, 72, 73, 74];
        let score = g.players[1 - p].score;
        g.battlefield_on_hold(p, 0);
        g.after_resolution().unwrap();
        settle(&mut g);
        assert_eq!(g.players[1 - p].score, score + 1);
    }
}

#[test]
fn pyke_hidden_waives_only_base_cost_and_retains_optional_fury_payment() {
    for p in 0..2 {
        for paid in [false, true] {
            let mut g = position(p, 0);
            unit(&mut g, p, 71, Some(0));
            g.players[p].hand.push(420);
            g.apply(Action::Hide {
                hand_index: 0,
                lane: 0,
            })
            .unwrap();
            let hidden_id = g.players[p].hidden[0].id;
            // Controlled prior-turn facedown position; Hide itself above is a
            // real public action. No claim that this setup is a complete turn.
            g.players[p].hidden[0].hidden_turn = g.turn - 1;
            g.players[p].energy_pool = 0;
            g.players[p].energy = 0;
            g.players[p].power_pool = [1, 0, 0, 0, 0, 0];
            g.apply(Action::WithPlayChoices {
                action: Box::new(Action::PlayHidden {
                    hidden_id,
                    targets: vec![],
                }),
                choices: PlayChoices {
                    bonus: paid,
                    ..PlayChoices::default()
                },
            })
            .unwrap();
            let pyke = g.players[p]
                .units
                .iter()
                .find(|u| u.card_id == 420)
                .unwrap();
            assert_eq!(pyke.lane, Some(0));
            assert!(!pyke.ready);
            assert_eq!(g.unit_might(pyke).unwrap(), 2);
            assert_eq!(g.players[p].energy, 0);
            assert_eq!(g.players[p].power_pool[0], u16::from(!paid));
            assert_eq!(
                g.chain.iter().filter(|i| i.card_id == 420).count(),
                usize::from(paid)
            );
            settle(&mut g);
            let pyke = g.players[p]
                .units
                .iter()
                .find(|u| u.card_id == 420)
                .unwrap();
            assert_eq!(pyke.ready, paid);
            assert_eq!(g.unit_might(pyke).unwrap(), if paid { 4 } else { 2 });
        }
    }
}

#[test]
fn spring_ignores_movement_and_tokens_and_tracks_each_physical_field_until_next_turn() {
    for p in 0..2 {
        let mut g = position(p, 12);
        g.battlefields[1] = 12;
        g.battlefield_owner = [Some(p), Some(p)];
        unit(&mut g, p, 71, Some(0));
        unit(&mut g, p, 71, Some(1));
        let mover = unit(&mut g, p, 71, None);
        g.apply(Action::Move {
            unit_id: unit_id(mover),
            lane: Some(0),
        })
        .unwrap();
        assert_eq!(g.star_spring_used(), [[false; 2]; 2]);
        // Canonical fixed token creation, without inventing a non-token play.
        g.queue_token_play(p, 178, 2, Some(Some(0)), false);
        g.after_resolution().unwrap();
        assert!(
            g.players[p]
                .units
                .iter()
                .any(|u| u.token && u.lane == Some(0))
        );
        assert_eq!(g.star_spring_used(), [[false; 2]; 2]);
        for lane in 0..2 {
            cast(&mut g, 71, Some(lane), vec![]);
            let Some(Decision::BattlefieldTargets {
                choice_id, source, ..
            }) = g.decision
            else {
                panic!("{:?}", g.decision)
            };
            assert_eq!(source.lane, lane);
            g.apply(Action::ResolveBattlefieldTargets {
                choice_id,
                targets: vec![],
            })
            .unwrap();
            assert!(g.star_spring_used()[p][lane]);
        }
        assert_eq!(g.star_spring_used()[1 - p], [false; 2]);
        // Canonical start-turn checkpoint resets both players' opportunities.
        g.start_turn().unwrap();
        assert_eq!(g.star_spring_used(), [[false; 2]; 2]);
    }
}

#[test]
fn peak_ready_energy_capacity_failure_preserves_the_entire_chain_and_rune_state() {
    for p in 0..2 {
        let mut g = position(p, 14);
        g.players[p]
            .set_rune_setup([1, 0, 0, 0, 0, 0], [1, 0, 0, 0, 0, 0])
            .unwrap();
        g.players[p].energy_pool = u16::MAX;
        g.players[p].energy = u16::MAX;
        let rune_id = g.players[p].rune_objects()[0].id;
        g.queue_field_targets(p, 0, BattlefieldTargetKind::TargonPeakReady);
        g.after_resolution().unwrap();
        let Some(Decision::BattlefieldTargets { choice_id, .. }) = g.decision else {
            panic!()
        };
        g.apply(Action::ResolveBattlefieldTargets {
            choice_id,
            targets: vec![Target::Rune { player: p, rune_id }],
        })
        .unwrap();
        g.apply(Action::Pass).unwrap();
        let before = format!("{g:?}");
        assert!(g.apply(Action::Pass).unwrap_err().contains("capacity"));
        assert_eq!(format!("{g:?}"), before);
        assert!(!g.players[p].rune_objects()[0].ready);
        g.players[p].energy_pool -= 1;
        g.players[p].energy -= 1;
        g.apply(Action::Pass).unwrap();
        assert!(g.players[p].rune_objects()[0].ready);
        assert_eq!(g.players[p].energy, u16::MAX);
    }
}

#[test]
fn faefolk_ai_accepts_safe_pulls_prefers_hold_denial_and_declines_harmful_moves() {
    for p in 0..2 {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            for scenario in 0..6 {
                let mut g = position(p, 0);
                g.difficulty = difficulty;
                let ally = unit(&mut g, p, if scenario == 4 { 414 } else { 71 }, Some(0));
                if scenario == 3 {
                    g.unit_mut(ally).unwrap().stunned = true;
                }
                let source = unit(&mut g, p, 417, None);
                let base = unit(
                    &mut g,
                    1 - p,
                    match scenario {
                        2 => 70,
                        3 | 4 => 71,
                        _ => 56,
                    },
                    None,
                );
                if scenario == 5 {
                    g.unit_mut(base).unwrap().protected = true;
                }
                let preferred = if scenario == 1 {
                    g.battlefield_owner[1] = Some(1 - p);
                    unit(&mut g, 1 - p, 56, Some(1))
                } else {
                    base
                };
                g.apply(Action::Move {
                    unit_id: unit_id(source),
                    lane: Some(0),
                })
                .unwrap();
                assert!(matches!(
                    g.decision,
                    Some(Decision::TriggerTargets { card_id: 417, .. })
                ));
                let before = format!("{g:?}");
                let action = g.choose_ai_action().unwrap();
                assert_eq!(format!("{g:?}"), before, "the evaluator must be read-only");
                let expected = if scenario < 2 {
                    vec![preferred]
                } else {
                    vec![]
                };
                assert_eq!(
                    action,
                    Action::ChooseTriggerTargets {
                        targets: expected.clone()
                    },
                    "seat{p} {difficulty:?} scenario{scenario}"
                );
                g.apply(action).unwrap();
                settle(&mut g);
                assert_eq!(
                    g.unit_ref(preferred).unwrap().lane,
                    if scenario < 2 { Some(0) } else { None }
                );
                if scenario == 1 {
                    assert_eq!(g.battlefield_owner[1], None);
                }
            }
        }
    }
}

#[test]
fn faefolk_policy_is_invariant_to_private_hands_and_deck_order_in_every_ai_level() {
    for p in 0..2 {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let mut a = position(p, 0);
            a.difficulty = difficulty;
            unit(&mut a, p, 71, Some(0));
            let source = unit(&mut a, p, 417, None);
            let victim = unit(&mut a, 1 - p, 56, None);
            a.players[p].hand = vec![71, 70];
            a.players[1 - p].hand = vec![9, 297, 63];
            a.players[0].deck = vec![71, 70, 56, 63];
            a.players[1].deck = vec![63, 56, 70, 71];
            a.apply(Action::Move {
                unit_id: unit_id(source),
                lane: Some(0),
            })
            .unwrap();
            let mut b = a.clone();
            b.players[p].hand = vec![297, 63];
            b.players[1 - p].hand = vec![70, 71, 418];
            b.players[0].deck.reverse();
            b.players[1].deck.reverse();
            let expected = Action::ChooseTriggerTargets {
                targets: vec![victim],
            };
            assert_eq!(a.choose_ai_action().unwrap(), expected);
            assert_eq!(b.choose_ai_action().unwrap(), expected);
        }
    }
}
