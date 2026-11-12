//! Reckless Innovator: actual exhaustion, frozen modes and pre-response costs.
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
            if chosen == 1 {
                assert_eq!(g.players[owner].power_pool[4], 4);
                assert!(g.chain.is_empty());
                g.apply(pay_action(&g, true)).unwrap();
                assert_eq!(g.players[owner].power_pool[4], 3);
                assert!(g.players[owner].hand.is_empty());
            }
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
    g.apply(pay_action(&g, false)).unwrap();
    settle(&mut g);
    assert!(g.players[0].gear.is_empty());
    assert!(g.players[0].hand.is_empty());
}

#[test]
fn innovator_payment_undo_keeps_mode_and_refusal_restores_generated_power_without_gold() {
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
        assert_eq!(g.players[owner].runes[4], 1);
        assert_eq!(g.players[owner].power_pool[4], 0);
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
    let payment = pay_action(&g, true);
    g.apply(payment.clone()).unwrap();
    assert!(matches!(g.decision, Some(Decision::TriggerMode { .. })));
    reject(&mut g, payment);
    mode(&mut g, 0);
    assert_eq!(g.chain.len(), 2);
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
    g.apply(pay_action(&g, true)).unwrap();
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
                if matches!(g.decision, Some(Decision::TriggerPayment { .. })) {
                    assert!(g.step_ai().unwrap());
                }
                settle(&mut g);
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
