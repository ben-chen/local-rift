//! Reckless Innovator: actual exhaustion, frozen modes and resolving instruction costs.
use super::*;

const INNOVATOR: usize = 429;

fn position(owner: usize) -> Game {
    position_with_preset(owner, 3)
}

fn position_with_preset(owner: usize, preset: usize) -> Game {
    let mut g = Game::new_for_agents(preset, preset, 429100, Difficulty::Tactical);
    g.choose_starting_player(owner).unwrap();
    g.start_match().unwrap();
    g.battlefields = [2, 2];
    g.battlefield_owner = [None; 2];
    for p in &mut g.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.deck = vec![71; 8];
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 20;
        p.energy = 20;
        p.power_pool = [4; 6];
        p.any_power = 0;
    }
    g
}

#[test]
fn innovator_irelia_ready_trigger_allows_a_second_exhaust_choice_that_turn() {
    for owner in 0..2 {
        let mut g = position_with_preset(owner, 0);
        g.battlefield_owner[0] = Some(owner);
        let unit_id = ready_copy(&mut g, owner);
        g.apply(Action::Move {
            unit_id,
            lane: Some(0),
        })
        .unwrap();
        mode(&mut g, 0);
        settle(&mut g);
        assert_eq!(g.players[owner].gear.len(), 1);
        assert!(
            !g.unit_ref(Target::Unit {
                player: owner,
                unit_id
            })
            .unwrap()
            .ready
        );
        g.players[owner].hand = vec![7];
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: owner,
                unit_id,
            }],
        })
        .unwrap();
        assert!(matches!(g.decision, Some(Decision::Legend {
            player, trigger: LegendTrigger::IreliaChoose { unit_id: chosen }
        }) if player == owner && chosen == unit_id));
        g.apply(Action::Choose { accept: true }).unwrap();
        assert!(!g.legend[owner].ready);
        assert!(
            !g.unit_ref(Target::Unit {
                player: owner,
                unit_id
            })
            .unwrap()
            .ready
        );
        settle(&mut g);
        assert!(
            g.unit_ref(Target::Unit {
                player: owner,
                unit_id
            })
            .unwrap()
            .ready
        );
        // Discipline itself draws one; Innovator's later mode adds one more.
        assert_eq!(g.players[owner].hand, vec![71]);
        g.apply(Action::Move {
            unit_id,
            lane: None,
        })
        .unwrap();
        mode(&mut g, 1);
        reach_payment(&mut g);
        g.apply(pay_action(&g, true)).unwrap();
        settle(&mut g);
        assert_eq!(g.players[owner].gear.len(), 1);
        assert_eq!(g.players[owner].hand, vec![71, 71]);
        assert!(
            !g.unit_ref(Target::Unit {
                player: owner,
                unit_id
            })
            .unwrap()
            .ready
        );
    }
}

#[test]
fn innovator_effect_exhausts_before_lethal_damage_and_preserves_only_actual_occurrences() {
    for owner in 0..2 {
        for chosen in 0..2 {
            let mut g = position(owner);
            let ready = ready_copy(&mut g, owner);
            let already_exhausted = g.create_unit(owner, INNOVATOR, false, 0);
            let enemy = ready_copy(&mut g, 1 - owner);
            for (player, unit_id) in [
                (owner, ready),
                (owner, already_exhausted),
                (1 - owner, enemy),
            ] {
                g.unit_mut(Target::Unit { player, unit_id }).unwrap().lane = Some(0);
            }
            g.players[owner].hand = vec![313];
            g.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            })
            .unwrap();
            g.apply(Action::Pass).unwrap();
            g.apply(Action::Pass).unwrap();
            assert!(g.players.iter().all(|player| player.units.is_empty()));
            assert!(
                matches!(g.decision, Some(Decision::TriggerMode { player, context, .. })
                if player == owner && context.source.unit_id == Some(ready))
            );
            assert!(g.pending_triggers.is_empty());
            mode(&mut g, chosen);
            if chosen == 1 {
                reach_payment(&mut g);
                g.apply(pay_action(&g, true)).unwrap();
            }
            settle(&mut g);
            assert_eq!(g.players[owner].gear.len(), usize::from(chosen == 0));
            assert_eq!(g.players[owner].hand.len(), chosen);
            assert!(g.players[1 - owner].gear.is_empty() && g.players[1 - owner].hand.is_empty());
        }
    }
}

fn settle(g: &mut Game) {
    if g.chain.is_empty() && g.decision.is_none() {
        g.after_resolution().unwrap();
    }
    for _ in 0..100 {
        let action = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::TriggerPayment { .. }) => pay_action(g, true),
            None if g.in_window() => Action::Pass,
            None => return,
            other => panic!("Unexpected decision {other:?}"),
        };
        g.apply(action).unwrap();
    }
    panic!("Unsettled chain");
}

fn ready_copy(g: &mut Game, owner: usize) -> u32 {
    let id = g.create_unit(owner, INNOVATOR, false, 0);
    g.ready_unit(owner, id);
    id
}

fn exhaust(g: &mut Game, owner: usize, id: u32) -> u32 {
    g.exhaust_unit(owner, id);
    g.after_resolution().unwrap();
    let Some(Decision::TriggerMode {
        choice_id, player, ..
    }) = g.decision
    else {
        panic!("No mode choice: {:?}", g.decision);
    };
    assert_eq!(player, owner);
    assert_eq!(g.priority(), owner);
    choice_id
}

fn mode(g: &mut Game, mode: usize) {
    let Some(Decision::TriggerMode { choice_id, .. }) = g.decision else {
        panic!("No mode choice");
    };
    g.apply(Action::ChooseTriggerMode { choice_id, mode })
        .unwrap();
}

fn reach_payment(g: &mut Game) {
    for _ in 0..12 {
        if matches!(
            g.decision,
            Some(Decision::TriggerPayment {
                resolving_chain_id: Some(_),
                ..
            })
        ) {
            assert!(g.resolution_progress().is_some());
            return;
        }
        assert!(g.decision.is_none(), "Unexpected choice: {:?}", g.decision);
        assert!(!g.chain.is_empty());
        g.apply(Action::Pass).unwrap();
    }
    panic!("Payment was never reached");
}

fn pay_action(g: &Game, pay: bool) -> Action {
    let Some(Decision::TriggerPayment { choice_id, .. }) = g.decision else {
        panic!("No payment choice: {:?}", g.decision);
    };
    Action::ResolveTriggerPayment { choice_id, pay }
}

fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action.clone()).is_err(), "Accepted {action:?}");
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn innovator_normal_and_accelerated_entries_pay_printed_cost_and_do_not_trigger() {
    for owner in 0..2 {
        for accelerate in [false, true] {
            let mut g = position(owner);
            g.players[owner].hand = vec![INNOVATOR];
            let action = Action::PlayCardOptions {
                hand_index: 0,
                lane: None,
                targets: vec![],
                repeat_targets: vec![],
                repeat: false,
                accelerate,
                bonus: false,
            };
            let options = g.play_choices(&action).unwrap();
            assert!(options.accelerate.is_some());
            g.apply(action).unwrap();
            settle(&mut g);
            assert_eq!(g.players[owner].energy, if accelerate { 16 } else { 17 });
            assert_eq!(
                g.players[owner].power_pool[4],
                if accelerate { 3 } else { 4 }
            );
            let unit = &g.players[owner].units[0];
            assert_eq!(unit.card_id, INNOVATOR);
            assert_eq!(unit.ready, accelerate);
            assert_eq!(g.unit_might(unit).unwrap(), 3);
            assert!(g.players[owner].gear.is_empty());
            assert!(g.decision.is_none());
            assert!(g.pending_triggers.is_empty());
            assert_eq!(g.catalog[INNOVATOR].tags, &["Piltover"]);
        }
    }
}

#[test]
fn innovator_gold_reminder_does_not_grant_reaction_timing_to_unit() {
    let mut g = position(0);
    let unit_id = g.create_unit(0, 73, false, 0);
    g.players[0].hand = vec![7, INNOVATOR];
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit { player: 0, unit_id }],
    })
    .unwrap();
    assert!(g.in_window());
    reject(
        &mut g,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        },
    );
}

#[test]
fn innovator_standard_move_requires_mode_and_gold_resolves_after_responses() {
    for owner in 0..2 {
        let mut g = position(owner);
        let id = ready_copy(&mut g, owner);
        g.apply(Action::MoveGroup {
            unit_ids: vec![id],
            lane: Some(0),
        })
        .unwrap();
        assert!(
            matches!(g.decision, Some(Decision::TriggerMode { player, .. }) if player == owner)
        );
        reject(&mut g, Action::Pass);
        reject(&mut g, Action::Choose { accept: false });
        assert!(g.chain.is_empty());
        mode(&mut g, 0);
        assert!(g.players[owner].gear.is_empty());
        let item = g.chain.last().unwrap();
        assert_eq!(item.card_id, INNOVATOR);
        assert!(
            matches!(item.ability, Some(ChainAbility::CardTrigger { context, .. }) if context.mode == Some(0))
        );
        settle(&mut g);
        assert!(g.players[owner].hand.is_empty());
        assert_eq!(g.players[owner].gear.len(), 1);
        let gold = &g.players[owner].gear[0];
        assert!(gold.is_gold() && !gold.ready && gold.lane.is_none());
        let gear_id = gold.id;
        reject(&mut g, Action::ActivateGear { gear_id });
    }
}

#[test]
fn innovator_both_selected_effects_survive_source_departure() {
    for owner in 0..2 {
        for chosen in 0..2 {
            let mut g = position(owner);
            let id = ready_copy(&mut g, owner);
            exhaust(&mut g, owner, id);
            mode(&mut g, chosen);
            assert_eq!(g.players[owner].power_pool[4], 4);
            assert!(g.players[owner].hand.is_empty());
            assert_eq!(g.chain.len(), 1);
            g.banish_unit_directly(Target::Unit {
                player: owner,
                unit_id: id,
            });
            settle(&mut g);
            assert_eq!(g.players[owner].banish, vec![INNOVATOR]);
            assert_eq!(g.players[owner].gear.len(), usize::from(chosen == 0));
            assert_eq!(g.players[owner].hand.len(), chosen);
            assert_eq!(
                g.players[owner].power_pool[4],
                if chosen == 1 { 3 } else { 4 }
            );
        }
    }
}

#[test]
fn innovator_already_exhausted_does_not_trigger_and_readying_allows_repeat_same_turn() {
    for owner in 0..2 {
        let mut g = position(owner);
        let id = g.create_unit(owner, INNOVATOR, false, 0);
        g.exhaust_unit(owner, id);
        settle(&mut g);
        assert!(g.players[owner].gear.is_empty());
        for count in 1..=3 {
            g.ready_unit(owner, id);
            exhaust(&mut g, owner, id);
            mode(&mut g, 0);
            settle(&mut g);
            assert_eq!(g.players[owner].gear.len(), count);
            g.exhaust_unit(owner, id);
            settle(&mut g);
            assert_eq!(g.players[owner].gear.len(), count);
        }
    }
}

#[test]
fn innovator_paid_draw_requires_chaos_or_any_power_and_excludes_spell_only_power() {
    for source in 0..5 {
        let mut g = position(0);
        let id = ready_copy(&mut g, 0);
        g.players[0].power_pool = [0; 6];
        match source {
            0 => g.players[0].power_pool[4] = 1,
            1 => g.players[0].any_power = 1,
            2 => g.players[0].power_pool[1] = 1,
            3 => g.legend[0].spell_power = 1,
            _ => {}
        }
        exhaust(&mut g, 0, id);
        mode(&mut g, 1);
        reach_payment(&mut g);
        let action = pay_action(&g, true);
        if source <= 1 {
            g.apply(action).unwrap();
            settle(&mut g);
            assert_eq!(g.players[0].hand, vec![71]);
            assert_eq!(g.players[0].power_pool[4] + g.players[0].any_power, 0);
        } else {
            reject(&mut g, action);
            g.apply(pay_action(&g, false)).unwrap();
            settle(&mut g);
            assert!(g.players[0].hand.is_empty());
        }
        assert!(g.players[0].gear.is_empty());
        assert_eq!(g.players[0].energy, 20);
    }
}

#[test]
fn innovator_mode_choices_are_read_only_and_bad_or_stale_actions_are_atomic() {
    let mut g = position(0);
    let id = ready_copy(&mut g, 0);
    let choice_id = exhaust(&mut g, 0, id);
    let before = format!("{g:?}");
    assert_eq!(g.trigger_mode_options().len(), 2);
    let actions = g.legal_actions().unwrap();
    assert!(actions.contains(&Action::ChooseTriggerMode { choice_id, mode: 0 }));
    assert!(actions.contains(&Action::ChooseTriggerMode { choice_id, mode: 1 }));
    assert_eq!(format!("{g:?}"), before);
    reject(
        &mut g,
        Action::ChooseTriggerMode {
            choice_id: choice_id + 1,
            mode: 0,
        },
    );
    reject(&mut g, Action::ChooseTriggerMode { choice_id, mode: 2 });
    mode(&mut g, 1);
    reject(&mut g, Action::ChooseTriggerMode { choice_id, mode: 0 });
    reach_payment(&mut g);
    g.apply(pay_action(&g, false)).unwrap();
    settle(&mut g);
    assert!(g.players[0].gear.is_empty());
    assert!(g.players[0].hand.is_empty());
}

#[test]
fn innovator_payment_undo_restores_but_refusal_keeps_generated_power_without_gold() {
    for owner in 0..2 {
        let mut g = position(owner);
        let id = ready_copy(&mut g, owner);
        g.players[owner]
            .set_rune_setup([0, 0, 0, 0, 1, 0], [0; 6])
            .unwrap();
        g.players[owner].energy = 21;
        g.players[owner].power_pool = [0; 6];
        exhaust(&mut g, owner, id);
        mode(&mut g, 1);
        reach_payment(&mut g);
        let before = format!("{g:?}");
        g.apply(Action::BeginPayment {
            action: Box::new(pay_action(&g, true)),
        })
        .unwrap();
        g.apply(Action::ActivateRune {
            domain: Domain::Chaos,
            recycle: true,
        })
        .unwrap();
        g.apply(Action::Choose { accept: false }).unwrap();
        assert_eq!(format!("{g:?}"), before);
        g.apply(Action::ActivateRune {
            domain: Domain::Chaos,
            recycle: true,
        })
        .unwrap();
        g.apply(pay_action(&g, false)).unwrap();
        settle(&mut g);
        assert_eq!(g.players[owner].runes[4], 0);
        assert_eq!(g.players[owner].power_pool[4], 1);
        assert!(!g.players[owner].units[0].ready);
        assert!(g.players[owner].gear.is_empty());
        assert!(g.players[owner].hand.is_empty());
    }
}

#[test]
fn innovator_opponent_turn_trigger_belongs_to_unit_controller() {
    let mut g = position(0);
    let id = ready_copy(&mut g, 1);
    exhaust(&mut g, 1, id);
    mode(&mut g, 1);
    reach_payment(&mut g);
    g.apply(pay_action(&g, true)).unwrap();
    settle(&mut g);
    assert_eq!(g.players[1].hand, vec![71]);
    assert!(g.players[0].hand.is_empty());
    assert_eq!(g.players[0].power_pool[4], 4);
    assert_eq!(g.players[1].power_pool[4], 3);
}

#[test]
fn innovator_simultaneous_copies_keep_independent_modes_and_payment_identities() {
    let mut g = position(0);
    let first = ready_copy(&mut g, 0);
    let second = ready_copy(&mut g, 0);
    g.apply(Action::MoveGroup {
        unit_ids: vec![first, second],
        lane: Some(0),
    })
    .unwrap();
    let Some(Decision::OrderTriggers { ref options, .. }) = g.decision else {
        panic!("No ordering choice");
    };
    let trigger_id = options[1].id;
    g.apply(Action::OrderTrigger { trigger_id }).unwrap();
    mode(&mut g, 1);
    assert!(matches!(g.decision, Some(Decision::TriggerMode { .. })));
    mode(&mut g, 0);
    assert_eq!(g.chain.len(), 2);
    assert_eq!(g.players[0].power_pool[4], 4);
    reach_payment(&mut g);
    let payment = pay_action(&g, true);
    g.apply(payment.clone()).unwrap();
    reject(&mut g, payment);
    settle(&mut g);
    assert_eq!(g.players[0].hand, vec![71]);
    assert_eq!(g.players[0].gear.len(), 1);
    assert_eq!(g.players[0].power_pool[4], 3);
}

#[test]
fn innovator_spell_counter_cannot_target_its_triggered_ability() {
    let mut g = position(0);
    let id = ready_copy(&mut g, 0);
    exhaust(&mut g, 0, id);
    mode(&mut g, 1);
    let chain_id = g.chain.last().unwrap().id;
    g.apply(Action::Pass).unwrap();
    g.players[1].hand = vec![6];
    reject(
        &mut g,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Spell { chain_id }],
        },
    );
    settle(&mut g);
    assert_eq!(g.players[0].hand, vec![71]);
    assert_eq!(g.players[0].power_pool[4], 3);
}

#[test]
fn innovator_ai_chooses_legal_modes_and_can_finish_both_effects() {
    for owner in 0..2 {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            for payable in [false, true] {
                let mut g = position(owner);
                g.difficulty = difficulty;
                let id = ready_copy(&mut g, owner);
                if !payable {
                    g.players[owner].power_pool = [0; 6];
                }
                exhaust(&mut g, owner, id);
                assert!(g.step_ai().unwrap());
                for _ in 0..12 {
                    if g.decision.is_some() {
                        assert!(g.step_ai().unwrap());
                    } else if !g.chain.is_empty() {
                        g.apply(Action::Pass).unwrap();
                    } else {
                        break;
                    }
                }
                assert!(g.decision.is_none() && g.chain.is_empty());
                assert_eq!(
                    g.players[owner].gear.len() + g.players[owner].hand.len(),
                    1,
                    "{difficulty:?}, owner {owner}, payable {payable}"
                );
                if !payable {
                    assert_eq!(g.players[owner].gear.len(), 1);
                }
            }
        }
    }
}

// Core383.3.b and Riot's Oct26 Radiance FAQ: choose-one precedes this cost.
// The FAQ names Riven; Innovator has the same relevant instruction structure.
#[test]
fn innovator_later_modal_cost_waits_for_resolution() {
    for owner in 0..2 {
        let mut g = position(owner);
        let id = ready_copy(&mut g, owner);
        exhaust(&mut g, owner, id);
        mode(&mut g, 1);
        assert!(
            g.decision.is_none(),
            "Draw mode must reach the response window before its later cost: {:?}",
            g.decision
        );
        assert_eq!(g.chain.len(), 1);
        assert_eq!(g.players[owner].power_pool[4], 4);
        assert!(g.players[owner].hand.is_empty());
        g.apply(Action::Pass).unwrap();
        g.apply(Action::Pass).unwrap();
        assert!(matches!(g.decision, Some(Decision::TriggerPayment { .. })));
        assert_eq!(g.players[owner].power_pool[4], 4);
        assert_eq!(
            g.chain.len(),
            1,
            "The resolving trigger stays on chain during its cost decision"
        );
        g.apply(pay_action(&g, true)).unwrap();
        assert_eq!(g.players[owner].power_pool[4], 3);
        assert_eq!(g.players[owner].hand, vec![71]);
        assert!(g.chain.is_empty());
        assert!(g.decision.is_none());
    }
}

#[test]
fn innovator_resolution_payment_obeys_exact_sources_without_reopening_responses() {
    for owner in 0..2 {
        let mut g = position(owner);
        let id = ready_copy(&mut g, owner);
        exhaust(&mut g, owner, id);
        mode(&mut g, 1);
        reach_payment(&mut g);
        let action = pay_action(&g, true);
        let before = format!("{g:?}");
        let quote = g.quote_power_payment(&action).unwrap();
        assert_eq!(format!("{g:?}"), before);
        assert_eq!(quote.suggested.colored[4], 1);
        let mut wrong = quote.suggested.clone();
        wrong.colored[4] = 0;
        wrong.colored[1] = 1;
        reject(
            &mut g,
            Action::WithPowerPayment {
                action: Box::new(action.clone()),
                payment: wrong,
            },
        );
        reject(&mut g, Action::Pass);
        let mut excess = quote.suggested.clone();
        excess.colored[4] = 2;
        reject(
            &mut g,
            Action::WithPowerPayment {
                action: Box::new(action.clone()),
                payment: excess,
            },
        );
        g.apply(Action::WithPowerPayment {
            action: Box::new(action.clone()),
            payment: quote.suggested,
        })
        .unwrap();
        assert_eq!(g.players[owner].hand, vec![71]);
        assert_eq!(g.players[owner].power_pool[4], 3);
        reject(&mut g, action);
        assert!(g.resolution_progress().is_none());
    }
}

#[test]
fn innovator_instruction_plan_refusal_keeps_add_while_undo_restores_it() {
    use crate::payment_plan::PaymentRefusalKind;
    for owner in 0..2 {
        for undo in [false, true] {
            let mut g = position(owner);
            let id = ready_copy(&mut g, owner);
            let gold = g.gear_gold(owner, false);
            g.players[owner].power_pool = [0; 6];
            exhaust(&mut g, owner, id);
            mode(&mut g, 1);
            reach_payment(&mut g);
            let before = format!("{g:?}");
            let parent = g.chain.last().unwrap().id;
            g.apply(Action::BeginPayment {
                action: Box::new(pay_action(&g, true)),
            })
            .unwrap();
            assert_eq!(
                g.payment_plan_view(owner).unwrap().refusal_kind,
                Some(PaymentRefusalKind::Instruction)
            );
            assert_eq!(g.resolution_progress().unwrap().chain_id, parent);
            g.apply(Action::ActivateGear { gear_id: gold }).unwrap();
            assert!(g.players[owner].gear.is_empty());
            assert_eq!(g.players[owner].any_power, 1);
            if undo {
                g.apply(Action::Choose { accept: false }).unwrap();
                assert_eq!(format!("{g:?}"), before);
                g.apply(pay_action(&g, false)).unwrap();
                assert_eq!(g.players[owner].gear.len(), 1);
                assert_eq!(g.players[owner].any_power, 0);
            } else {
                g.apply(Action::RefusePayment).unwrap();
                assert!(g.players[owner].gear.is_empty());
                assert_eq!(g.players[owner].any_power, 1);
            }
            assert!(g.players[owner].hand.is_empty());
            assert!(g.chain.is_empty() && g.decision.is_none());
            assert!(g.resolution_progress().is_none());
        }
    }
}

#[test]
fn innovator_nested_conversion_must_finish_before_skip_and_can_fund_the_draw() {
    for owner in 0..2 {
        for staged in [false, true] {
            for pay in [false, true] {
                let mut g = position(owner);
                let id = ready_copy(&mut g, owner);
                g.gear_play(owner, 244, None).unwrap();
                let gear_id = g.players[owner].gear.last().unwrap().id;
                g.players[owner].power_pool = [0; 6];
                exhaust(&mut g, owner, id);
                mode(&mut g, 1);
                reach_payment(&mut g);
                let action = pay_action(&g, true);
                if staged {
                    g.apply(Action::BeginPayment {
                        action: Box::new(action.clone()),
                    })
                    .unwrap();
                }
                g.apply(Action::ActivateAbility {
                    source: AbilitySource::Gear {
                        player: owner,
                        gear_id,
                    },
                    ability_index: 0,
                    targets: vec![],
                })
                .unwrap();
                assert!(matches!(
                    g.decision,
                    Some(Decision::ResourceConversion { .. })
                ));
                assert!(g.resolution_progress().is_some());
                reject(&mut g, Action::RefusePayment);
                let Action::ResolveTriggerPayment { choice_id, .. } = action else {
                    unreachable!()
                };
                reject(
                    &mut g,
                    Action::ResolveTriggerPayment {
                        choice_id,
                        pay: false,
                    },
                );
                g.apply(Action::ResolveConversion { amount: 2 }).unwrap();
                assert_eq!(g.players[owner].energy, 18);
                assert_eq!(g.players[owner].any_power, 2);
                if staged {
                    g.apply(if pay {
                        Action::CompletePayment
                    } else {
                        Action::RefusePayment
                    })
                    .unwrap();
                } else {
                    g.apply(pay_action(&g, pay)).unwrap();
                }
                assert_eq!(g.players[owner].energy, 18);
                assert_eq!(g.players[owner].any_power, if pay { 1 } else { 2 });
                assert_eq!(g.players[owner].hand.len(), usize::from(pay));
                assert!(!g.players[owner].gear[0].ready);
                assert!(g.chain.is_empty() && g.decision.is_none());
            }
        }
    }
}

#[test]
fn innovator_response_resource_actions_change_what_can_be_paid_on_resolution() {
    for owner in 0..2 {
        for generate in [false, true] {
            let mut g = position(owner);
            let id = ready_copy(&mut g, owner);
            g.players[owner].power_pool = [0; 6];
            if !generate {
                g.players[owner].power_pool[4] = 1;
            }
            let converter = if generate { 244 } else { 245 };
            g.gear_play(owner, converter, None).unwrap();
            let gear_id = g.players[owner].gear.last().unwrap().id;
            exhaust(&mut g, owner, id);
            mode(&mut g, 1);
            assert!(g.decision.is_none());
            assert_eq!(g.priority(), owner);
            g.apply(Action::ActivateAbility {
                source: AbilitySource::Gear {
                    player: owner,
                    gear_id,
                },
                ability_index: 0,
                targets: vec![],
            })
            .unwrap();
            g.apply(Action::ResolveConversion { amount: 1 }).unwrap();
            reach_payment(&mut g);
            assert_eq!(g.players[owner].power_pool[4], 0);
            if generate {
                g.apply(pay_action(&g, true)).unwrap();
                assert_eq!(g.players[owner].hand, vec![71]);
            } else {
                let action = pay_action(&g, true);
                reject(&mut g, action);
                g.apply(pay_action(&g, false)).unwrap();
                assert!(g.players[owner].hand.is_empty());
            }
            assert!(g.chain.is_empty());
        }
    }
}

#[test]
fn innovator_skip_releases_resource_observers_only_after_the_parent_finishes() {
    for staged in [false, true] {
        let mut g = position(0);
        let id = ready_copy(&mut g, 0);
        let observer = g.create_unit(0, 215, false, 0);
        let gold = g.gear_gold(0, false);
        exhaust(&mut g, 0, id);
        mode(&mut g, 1);
        reach_payment(&mut g);
        let parent = g.chain[0].id;
        if staged {
            g.apply(Action::BeginPayment {
                action: Box::new(pay_action(&g, true)),
            })
            .unwrap();
        }
        g.apply(Action::ActivateGear { gear_id: gold }).unwrap();
        assert_eq!(g.chain.len(), 1);
        assert_eq!(g.chain[0].id, parent);
        assert!(!g.pending_triggers.is_empty());
        assert_eq!(
            g.unit_ref(Target::Unit {
                player: 0,
                unit_id: observer
            })
            .unwrap()
            .temporary_buff,
            0
        );
        if staged {
            g.apply(Action::RefusePayment).unwrap();
        } else {
            g.apply(pay_action(&g, false)).unwrap();
        }
        assert!(g.chain.iter().all(|item| item.id != parent));
        settle(&mut g);
        assert!(
            g.unit_might(
                g.unit_ref(Target::Unit {
                    player: 0,
                    unit_id: observer
                })
                .unwrap()
            )
            .unwrap()
                > u64::from(g.catalog[215].might)
        );
        assert!(g.players[0].hand.is_empty());
        assert_eq!(g.players[0].any_power, 1);
    }
}

#[test]
fn innovator_counter_engine_removes_the_trigger_without_charging_its_later_cost() {
    // Engine primitive coverage: the admitted Not So Fast cannot legally choose
    // this untargeted trigger. Do not pretend it supplies a legal card response.
    for owner in 0..2 {
        let mut g = position(owner);
        let id = ready_copy(&mut g, owner);
        exhaust(&mut g, owner, id);
        mode(&mut g, 1);
        let chain_id = g.chain.last().unwrap().id;
        assert!(!g.counter_target_valid(1 - owner, 12, chain_id));
        g.counter_spell(chain_id, false);
        g.after_resolution().unwrap();
        assert!(g.chain.is_empty() && g.decision.is_none());
        assert_eq!(g.players[owner].power_pool[4], 4);
        assert!(g.players[owner].hand.is_empty());
    }
}
