//! Controlled positions use real public play/response actions. Fixture changes
//! model source departure where no admitted counter can legally choose a trigger.
use super::*;

fn riven_id(target: Target) -> u32 {
    let Target::Unit { unit_id, .. } = target else {
        panic!("unit fixture")
    };
    unit_id
}

fn riven_exhaust(g: &mut Game, source: Target) {
    let Target::Unit { player, unit_id } = source else {
        unreachable!()
    };
    // Repeated event tests use canonical game actions, not direct readiness writes.
    g.ready_unit_by(player, player, unit_id);
    g.exhaust_unit(player, unit_id);
    g.after_resolution().unwrap();
}

fn riven_mode(g: &mut Game, mode: usize) {
    let Some(Decision::TriggerMode {
        choice_id,
        card_id: 439,
        ..
    }) = g.decision
    else {
        panic!("Expected Riven mode: {:?}", g.decision);
    };
    g.apply(Action::ChooseTriggerMode { choice_id, mode })
        .unwrap();
}

#[test]
fn riven_champion_enters_exhausted_without_a_become_exhausted_trigger() {
    for actor in 0..2 {
        let mut g = position(actor);
        g.players[actor].hand.push(439);
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        })
        .unwrap();
        drive(&mut g);
        let source = &g.players[actor].units[0];
        assert_eq!(source.card_id, 439);
        assert!(!source.ready);
        assert_eq!(g.unit_might(source).unwrap(), 4);
        assert_eq!(g.players[actor].energy, 96);
        assert!(g.decision.is_none() && g.chain.is_empty());
        assert!(g.trigger_state.modes_used.is_empty());
        assert!(g.catalog[439].tags.contains(&"Noxus"));
    }
}

#[test]
fn riven_champion_separate_copies_have_separate_modes_but_control_changes_keep_history() {
    let mut g = position(0);
    let first = unit(&mut g, 0, 439, None);
    let second = unit(&mut g, 0, 439, None);
    for source in [first, second] {
        riven_exhaust(&mut g, source);
        assert!(g.trigger_mode_available(0).unwrap());
        riven_mode(&mut g, 0);
        drive(&mut g);
    }
    // Controlled transfer of the same physical object; this is not a replay.
    let index = g.players[0]
        .units
        .iter()
        .position(|u| u.id == riven_id(first))
        .unwrap();
    let transferred = g.players[0].units.remove(index);
    g.players[1].units.push(transferred);
    let source = Target::Unit {
        player: 1,
        unit_id: riven_id(first),
    };
    riven_exhaust(&mut g, source);
    assert!(!g.trigger_mode_available(0).unwrap());
    assert!(g.trigger_mode_available(2).unwrap());
    riven_mode(&mut g, 2);
    drive(&mut g);
    assert_eq!(g.unit_ref(source).unwrap().buff, 0);
    assert!(g.unit_ref(source).unwrap().ready);
    assert_eq!(g.unit_ref(second).unwrap().buff, 1);
}

#[test]
fn riven_champion_spending_buff_can_make_marked_damage_lethal() {
    let mut g = position(0);
    let source = unit(&mut g, 0, 439, None);
    g.buff_unit(source).unwrap();
    g.unit_mut(source).unwrap().damage = 4;
    riven_exhaust(&mut g, source);
    riven_mode(&mut g, 2);
    assert!(
        g.unit_ref(source).is_some(),
        "Buff is not spent at finalization"
    );
    drive(&mut g);
    assert!(g.unit_ref(source).is_none());
    assert!(g.players[0].discard.contains(&439));
    assert!(!g.buffed_units.contains(&riven_id(source)));
}

#[test]
fn riven_champion_modes_are_once_per_physical_copy_and_reset_on_the_next_turn() {
    for actor in 0..2 {
        let mut g = position(actor);
        let source = unit(&mut g, actor, 439, None);
        let enemy = unit(&mut g, 1 - actor, 71, None);
        g.apply(Action::Move {
            unit_id: riven_id(source),
            lane: Some(0),
        })
        .unwrap();
        riven_mode(&mut g, 0);
        assert_eq!(g.unit_ref(source).unwrap().buff, 0);
        drive(&mut g);
        assert_eq!(g.unit_ref(source).unwrap().buff, 1);
        riven_exhaust(&mut g, source);
        assert!(!g.trigger_mode_available(0).unwrap());
        let before = format!("{g:?}");
        let Some(Decision::TriggerMode { choice_id, .. }) = g.decision else {
            panic!()
        };
        assert!(
            g.apply(Action::ChooseTriggerMode { choice_id, mode: 0 })
                .is_err()
        );
        assert_eq!(format!("{g:?}"), before);
        riven_mode(&mut g, 2);
        assert_eq!(
            g.unit_ref(source).unwrap().buff,
            1,
            "cost waits for resolution"
        );
        drive(&mut g);
        assert!(g.unit_ref(source).unwrap().ready);
        assert_eq!(g.unit_ref(source).unwrap().buff, 0);
        assert!(!g.buffed_units.contains(&riven_id(source)));
        g.apply(Action::Move {
            unit_id: riven_id(source),
            lane: None,
        })
        .unwrap();
        riven_mode(&mut g, 1);
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![enemy],
        })
        .unwrap();
        assert_eq!(g.unit_ref(source).unwrap().buff, 0);
        // A Buff gained after finalization is the one spent on resolution.
        g.buff_unit(source).unwrap();
        drive(&mut g);
        assert!(g.unit_ref(enemy).unwrap().stunned);
        assert_eq!(g.unit_ref(source).unwrap().buff, 0);
        riven_exhaust(&mut g, source);
        assert!(
            g.decision.is_none() && g.chain.is_empty(),
            "all three modes exhausted"
        );
        g.apply(Action::EndTurn).unwrap();
        assert!(g.trigger_state.modes_used.is_empty());
        riven_exhaust(&mut g, source);
        assert!((0..3).all(|mode| g.trigger_mode_available(mode).unwrap()));
    }
}

#[test]
fn riven_champion_can_choose_a_later_buff_cost_without_currently_having_a_buff() {
    for mode in [1, 2] {
        let mut g = position(0);
        let source = unit(&mut g, 0, 439, None);
        let enemy = unit(&mut g, 1, 71, None);
        riven_exhaust(&mut g, source);
        assert!(g.trigger_mode_available(mode).unwrap());
        riven_mode(&mut g, mode);
        if mode == 1 {
            g.apply(Action::ChooseTriggerTargets {
                targets: vec![enemy],
            })
            .unwrap();
        }
        drive(&mut g);
        assert!(!g.unit_ref(source).unwrap().ready);
        assert!(!g.unit_ref(enemy).unwrap().stunned);
        assert!(
            g.trigger_state
                .modes_used
                .contains(&(riven_id(source), 439, 0, mode))
        );
    }
}

#[test]
fn riven_champion_cannot_choose_an_illegal_stun_to_avoid_a_legal_mode() {
    let mut g = position(0);
    let source = unit(&mut g, 0, 439, None);
    let enemy = unit(&mut g, 1, 2, None); // Akali cannot be chosen out of combat.
    riven_exhaust(&mut g, source);
    assert!(!g.trigger_mode_available(1).unwrap());
    let Some(Decision::TriggerMode { choice_id, .. }) = g.decision else {
        panic!()
    };
    let before = format!("{g:?}");
    for action in [
        Action::ChooseTriggerMode { choice_id, mode: 1 },
        Action::Choose { accept: false },
    ] {
        assert!(g.apply(action).is_err());
        assert_eq!(format!("{g:?}"), before);
    }
    riven_mode(&mut g, 0);
    drive(&mut g);
    riven_exhaust(&mut g, source);
    riven_mode(&mut g, 2);
    drive(&mut g);
    riven_exhaust(&mut g, source);
    assert!(g.decision.is_none() && g.chain.is_empty());
    assert!(!g.unit_ref(enemy).unwrap().stunned);
}

#[test]
fn riven_champion_actual_counter_preserves_buff_but_still_uses_stun_mode() {
    for actor in 0..2 {
        let mut g = position(actor);
        let source = unit(&mut g, actor, 439, None);
        let enemy = unit(&mut g, 1 - actor, 71, None);
        g.buff_unit(source).unwrap();
        riven_exhaust(&mut g, source);
        riven_mode(&mut g, 1);
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![enemy],
        })
        .unwrap();
        let trigger = g.chain.last().unwrap().id;
        g.apply(Action::Pass).unwrap();
        g.players[1 - actor].hand.push(12); // Not So Fast counters the enemy ability choosing its unit.
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Spell { chain_id: trigger }],
        })
        .unwrap();
        drive(&mut g);
        assert_eq!(g.unit_ref(source).unwrap().buff, 1);
        assert!(!g.unit_ref(enemy).unwrap().stunned);
        riven_exhaust(&mut g, source);
        assert!(!g.trigger_mode_available(1).unwrap());
        assert!(g.trigger_mode_available(0).unwrap());
    }
}

#[test]
fn riven_champion_departed_source_cannot_spend_the_new_copys_buff() {
    for mode in [0, 1, 2] {
        let mut g = position(0);
        let source = unit(&mut g, 0, 439, None);
        let enemy = unit(&mut g, 1, 71, None);
        g.buff_unit(source).unwrap();
        riven_exhaust(&mut g, source);
        riven_mode(&mut g, mode);
        if mode == 1 {
            g.apply(Action::ChooseTriggerTargets {
                targets: vec![enemy],
            })
            .unwrap();
        }
        // Controlled source departure; a replay has a new physical identity.
        g.return_unit_to_hand(source);
        let replacement = unit(&mut g, 0, 439, None);
        g.buff_unit(replacement).unwrap();
        g.unit_mut(replacement).unwrap().ready = false;
        drive(&mut g);
        assert_eq!(g.unit_ref(replacement).unwrap().buff, 1);
        assert!(!g.unit_ref(replacement).unwrap().ready);
        assert!(!g.unit_ref(enemy).unwrap().stunned);
        riven_exhaust(&mut g, replacement);
        assert!(g.trigger_mode_available(mode).unwrap());
    }
}

#[test]
fn riven_champion_invalid_stun_keeps_buff_and_ready_spends_only_the_real_counter() {
    let mut g = position(0);
    let source = unit(&mut g, 0, 439, None);
    let enemy = unit(&mut g, 1, 71, None);
    g.buff_unit(source).unwrap();
    g.unit_mut(source).unwrap().temporary_buff = 3;
    riven_exhaust(&mut g, source);
    riven_mode(&mut g, 1);
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![enemy],
    })
    .unwrap();
    g.return_unit_to_hand(enemy);
    drive(&mut g);
    assert_eq!(g.unit_ref(source).unwrap().buff, 1);
    assert_eq!(g.unit_might(g.unit_ref(source).unwrap()).unwrap(), 8);
    riven_exhaust(&mut g, source);
    riven_mode(&mut g, 2);
    drive(&mut g);
    assert!(g.unit_ref(source).unwrap().ready);
    assert_eq!(g.unit_ref(source).unwrap().buff, 0);
    assert_eq!(g.unit_ref(source).unwrap().temporary_buff, 3);
    assert_eq!(g.unit_might(g.unit_ref(source).unwrap()).unwrap(), 7);
    assert!(!g.buffed_units.contains(&riven_id(source)));
    g.buff_unit(source).unwrap();
    assert_eq!(
        g.unit_ref(source).unwrap().buff,
        1,
        "spending permits a later Buff"
    );
}

#[test]
fn riven_champion_group_move_places_exhaust_trigger_below_later_move_trigger() {
    let mut g = position(0);
    let source = unit(&mut g, 0, 439, None);
    let herder = unit(&mut g, 0, 17, None);
    g.apply(Action::MoveGroup {
        unit_ids: vec![riven_id(source), riven_id(herder)],
        lane: Some(0),
    })
    .unwrap();
    assert!(matches!(
        g.decision,
        Some(Decision::TriggerMode { card_id: 439, .. })
    ));
    riven_mode(&mut g, 0);
    assert_eq!(
        g.chain.iter().map(|item| item.card_id).collect::<Vec<_>>(),
        vec![439, 17]
    );
    assert_eq!(g.players[0].hand.len(), 0);
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
    assert_eq!(g.players[0].hand.len(), 1);
    assert_eq!(g.unit_ref(source).unwrap().buff, 0);
    drive(&mut g);
    assert_eq!(g.unit_ref(source).unwrap().buff, 1);
}

#[test]
fn riven_champion_blade_cost_trigger_resolves_before_attachment_and_ai_uses_available_modes() {
    for level in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        let mut g = position(0);
        g.difficulty = level;
        let source = unit(&mut g, 0, 439, None);
        let gear_id = blade(&mut g, 0);
        g.apply(Action::EquipExhaustUnit {
            gear_id,
            target: source,
            exhaust_unit_id: riven_id(source),
        })
        .unwrap();
        assert!(matches!(
            g.decision,
            Some(Decision::TriggerMode { card_id: 439, .. })
        ));
        assert_eq!(g.players[0].gear[0].attached_to, None);
        let action = g.choose_ai_action().unwrap();
        let Action::ChooseTriggerMode { mode, .. } = action else {
            panic!("{action:?}")
        };
        assert!(g.trigger_mode_available(mode).unwrap());
        g.apply(action).unwrap();
        drive(&mut g);
        assert_eq!(g.players[0].gear[0].attached_to, Some(riven_id(source)));
        riven_exhaust(&mut g, source);
        let action = g.choose_ai_action().unwrap();
        let Action::ChooseTriggerMode { mode: next, .. } = action else {
            panic!("{action:?}")
        };
        assert_ne!(mode, next);
        assert!(g.trigger_mode_available(next).unwrap());
        g.apply(action).unwrap();
        drive(&mut g);
    }
}

fn position(actor: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 436436, Difficulty::Tactical);
    g.start_match().unwrap();
    g.active = actor;
    g.priority_player = actor;
    g.battlefields = vec![9, 9];
    g.battlefield_owner = vec![Some(actor), None];
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

fn unit(g: &mut Game, player: usize, card: usize, lane: Option<usize>) -> Target {
    let id = g.create_unit(player, card, false, 0);
    let target = Target::Unit {
        player,
        unit_id: id,
    };
    let u = g.unit_mut(target).unwrap();
    u.lane = lane;
    u.ready = true;
    target
}

fn drive(g: &mut Game) {
    for _ in 0..150 {
        match g.decision.clone() {
            Some(Decision::TriggerTargets { .. }) => return,
            Some(Decision::OrderTriggers { options, .. }) => g
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            Some(Decision::Legend { .. }) => g.apply(Action::Choose { accept: false }).unwrap(),
            None if !g.chain.is_empty() => g.apply(Action::Pass).unwrap(),
            None => return,
            other => panic!("Unexpected decision: {other:?}"),
        }
    }
    panic!("Fixture failed to finish resolving");
}

fn play_blitz(g: &mut Game, actor: usize, lane: Option<usize>) -> Target {
    let hand_index = g.players[actor].hand.len();
    g.players[actor].hand.push(436);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane,
        targets: vec![],
    })
    .unwrap();
    drive(g);
    let unit_id = g.players[actor]
        .units
        .iter()
        .find(|u| u.card_id == 436)
        .unwrap()
        .id;
    Target::Unit {
        player: actor,
        unit_id,
    }
}

#[test]
fn blitzcrank_play_to_field_pulls_enemy_from_base_or_other_field_for_both_seats() {
    for actor in 0..2 {
        for origin in [None, Some(1)] {
            let mut g = position(actor);
            unit(&mut g, actor, 71, Some(0));
            let enemy = unit(&mut g, 1 - actor, 71, origin);
            let source = play_blitz(&mut g, actor, Some(0));
            assert_eq!(g.players[actor].energy, 95);
            assert_eq!(g.unit_might(g.unit_ref(source).unwrap()).unwrap(), 5);
            assert!(g.unit_keywords(g.unit_ref(source).unwrap()).unwrap().tank);
            assert!(!g.unit_ref(source).unwrap().ready);
            assert!(matches!(
                g.decision,
                Some(Decision::TriggerTargets { card_id: 436, .. })
            ));
            g.apply(Action::ChooseTriggerTargets {
                targets: vec![enemy],
            })
            .unwrap();
            assert_eq!(g.unit_ref(enemy).unwrap().lane, origin);
            assert!(
                g.chain
                    .iter()
                    .any(|c| c.card_id == 436 && c.ability.is_some())
            );
            drive(&mut g);
            assert_eq!(g.unit_ref(enemy).unwrap().lane, Some(0));
            assert!(g.unit_ref(enemy).unwrap().ready);
        }
    }
}

#[test]
fn blitzcrank_base_play_has_no_pull_and_battlefield_pull_can_be_declined() {
    for lane in [None, Some(0)] {
        let mut g = position(0);
        unit(&mut g, 0, 71, Some(0));
        let enemy = unit(&mut g, 1, 71, Some(1));
        play_blitz(&mut g, 0, lane);
        if lane.is_some() {
            assert!(matches!(
                g.decision,
                Some(Decision::TriggerTargets { card_id: 436, .. })
            ));
            g.apply(Action::ChooseTriggerTargets { targets: vec![] })
                .unwrap();
        }
        drive(&mut g);
        assert!(g.chain.is_empty() && g.decision.is_none());
        assert_eq!(g.unit_ref(enemy).unwrap().lane, Some(1));
    }
}

#[test]
fn blitzcrank_rejects_friendly_already_here_and_missing_targets_atomically() {
    let mut g = position(0);
    let friendly = unit(&mut g, 0, 71, Some(0));
    let here = unit(&mut g, 1, 71, Some(0));
    unit(&mut g, 1, 71, None);
    play_blitz(&mut g, 0, Some(0));
    for target in [
        friendly,
        here,
        Target::Unit {
            player: 1,
            unit_id: u32::MAX,
        },
    ] {
        let before = format!("{g:?}");
        assert!(
            g.apply(Action::ChooseTriggerTargets {
                targets: vec![target]
            })
            .is_err()
        );
        assert_eq!(format!("{g:?}"), before);
    }
}

#[test]
fn blitzcrank_pull_uses_live_here_and_never_a_replayed_copy() {
    for destination in [Some(1), None] {
        let mut g = position(0);
        unit(&mut g, 0, 71, Some(0));
        let enemy = unit(&mut g, 1, 71, None);
        let source = play_blitz(&mut g, 0, Some(0));
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![enemy],
        })
        .unwrap();
        // Controlled response outcome: the finalized source moves or leaves.
        if let Some(lane) = destination {
            g.unit_mut(source).unwrap().lane = Some(lane);
        } else {
            g.return_unit_to_hand(source);
            unit(&mut g, 0, 436, Some(1));
        }
        drive(&mut g);
        assert_eq!(g.unit_ref(enemy).unwrap().lane, destination);
    }
}

#[test]
fn blitzcrank_hold_scores_then_returns_only_the_physical_holder_without_death() {
    for actor in 0..2 {
        let mut g = position(1 - actor);
        g.battlefield_owner = vec![Some(actor), None];
        let source = unit(&mut g, actor, 436, Some(0));
        let other = unit(&mut g, actor, 436, None);
        let before = g.players[actor].score;
        g.apply(Action::EndTurn).unwrap();
        assert_eq!(g.players[actor].score, before + 1);
        assert!(g.unit_ref(source).is_some());
        drive(&mut g);
        assert!(g.unit_ref(source).is_none());
        assert!(g.unit_ref(other).is_some());
        assert!(g.players[actor].hand.contains(&436));
        assert!(!g.players[actor].discard.contains(&436));
        assert_eq!(g.players[actor].score, before + 1);
    }
}

#[test]
fn blitzcrank_hold_trigger_cannot_return_a_new_copy_after_source_leaves() {
    let mut g = position(1);
    g.battlefield_owner = vec![Some(0), None];
    let source = unit(&mut g, 0, 436, Some(0));
    g.apply(Action::EndTurn).unwrap();
    g.return_unit_to_hand(source);
    let replacement = unit(&mut g, 0, 436, Some(0));
    drive(&mut g);
    assert!(g.unit_ref(replacement).is_some());
    assert_eq!(g.players[0].hand.iter().filter(|&&id| id == 436).count(), 1);
}

#[test]
fn blitzcrank_pull_pays_deflect_before_responses_without_refunding_on_failure() {
    let mut g = position(0);
    unit(&mut g, 0, 71, Some(0));
    let enemy = unit(&mut g, 1, 273, None);
    let source = play_blitz(&mut g, 0, Some(0));
    g.players[0].power_pool = [0; 6];
    g.players[0].any_power = 0;
    assert!(!g.valid_trigger_target(enemy, &[]).unwrap());
    let before = format!("{g:?}");
    assert!(
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![enemy]
        })
        .is_err()
    );
    assert_eq!(format!("{g:?}"), before);
    g.players[0].any_power = 1;
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![enemy],
    })
    .unwrap();
    assert_eq!(g.players[0].any_power, 0);
    g.return_unit_to_hand(source);
    drive(&mut g);
    assert_eq!(g.unit_ref(enemy).unwrap().lane, None);
    assert_eq!(g.players[0].any_power, 0);
}

#[test]
fn blitzcrank_pull_respects_hunker_down_response_and_unchoosable_units() {
    let mut g = position(0);
    unit(&mut g, 0, 71, Some(0));
    let protected = unit(&mut g, 1, 31, None);
    let enemy = unit(&mut g, 1, 71, None);
    play_blitz(&mut g, 0, Some(0));
    assert!(!g.valid_trigger_target(protected, &[]).unwrap());
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![enemy],
    })
    .unwrap();
    if g.priority() != 1 {
        g.apply(Action::Pass).unwrap();
    }
    let hand_index = g.players[1].hand.len();
    g.players[1].hand.push(425);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets: vec![enemy],
    })
    .unwrap();
    drive(&mut g);
    assert_eq!(g.unit_ref(enemy).unwrap().lane, None);
    let Target::Unit { unit_id, .. } = enemy else {
        unreachable!()
    };
    assert!(g.movement_prevented(0, 1, unit_id));
}

#[test]
fn blitzcrank_hold_detaches_equipment_and_ignores_death_replacements() {
    let mut g = position(1);
    g.battlefield_owner = vec![Some(0), None];
    let source = unit(&mut g, 0, 436, Some(0));
    let Target::Unit { unit_id, .. } = source else {
        unreachable!()
    };
    let gear_id = g.next_unit_id;
    g.next_unit_id += 1;
    g.players[0].gear.push(GearState {
        id: gear_id,
        card_id: 13,
        token: false,
        lane: None,
        attached_to: Some(unit_id),
        ready: true,
        temporary: false,
    });
    g.apply(Action::EndTurn).unwrap();
    drive(&mut g);
    assert!(g.unit_ref(source).is_none());
    assert!(g.players[0].hand.contains(&436));
    assert!(!g.players[0].discard.contains(&436));
    let gear = g.players[0]
        .gear
        .iter()
        .find(|gear| gear.id == gear_id)
        .unwrap();
    assert_eq!(gear.attached_to, None);
    assert_eq!(gear.lane, None);
}

#[test]
fn bedeviling_requires_field_and_enemy_targets_before_the_optional_return() {
    let mut g = position(0);
    unit(&mut g, 0, 71, Some(0));
    let enemy = unit(&mut g, 1, 71, Some(0));
    unit(&mut g, 1, 71, Some(0));
    g.players[0].hand.push(437);
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: Some(0),
        targets: vec![],
    })
    .unwrap();
    drive(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::TriggerTargets { card_id: 437, .. })
    ));
    let before = format!("{g:?}");
    assert!(g.apply(Action::Choose { accept: false }).is_err());
    assert_eq!(format!("{g:?}"), before);
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![Target::Battlefield { lane: 0 }, enemy],
    })
    .unwrap();
    assert!(g.decision.is_none());
    assert!(g.unit_ref(enemy).is_some());
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
    assert!(g.decision.is_some());
    assert!(g.unit_ref(enemy).is_some());
    let chain_id = g.chain.last().unwrap().id;
    assert_eq!(g.resolution_progress().unwrap().chain_id, chain_id);
    g.apply(Action::Choose { accept: true }).unwrap();
    assert!(g.unit_ref(enemy).is_none());
    assert!(g.chain.is_empty() && g.decision.is_none());
}

fn play_stranger(g: &mut Game, actor: usize, hidden: bool) -> Target {
    if hidden {
        g.players[actor].hidden.push(crate::hidden::HiddenCard {
            id: 50000,
            card_id: 437,
            lane: 0,
            hidden_turn: 0,
            revealed: false,
        });
        g.apply(Action::PlayHidden {
            hidden_id: 50000,
            targets: vec![],
        })
        .unwrap();
    } else {
        let hand_index = g.players[actor].hand.len();
        g.players[actor].hand.push(437);
        g.apply(Action::PlayCardWithTargets {
            hand_index,
            lane: Some(0),
            targets: vec![],
        })
        .unwrap();
    }
    drive(g);
    let unit_id = g.players[actor]
        .units
        .iter()
        .find(|u| u.card_id == 437)
        .unwrap()
        .id;
    Target::Unit {
        player: actor,
        unit_id,
    }
}

fn choose_stranger(g: &mut Game, target: Target, lane: usize) {
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![Target::Battlefield { lane }, target],
    })
    .unwrap();
}

#[test]
fn bedeviling_token_return_value_is_independent_of_the_factory_card() {
    for actor in 0..2 {
        let mut g = position(actor);
        // These different printed cards create the same Bird token. Neither
        // factory's Energy is a characteristic of the resulting token.
        assert_ne!(g.catalog[34].cost, g.catalog[249].cost);
        let birds: Vec<_> = [34, 249]
            .into_iter()
            .map(|factory| {
                let id = g.create_unit(1 - actor, factory, true, 0);
                let target = Target::Unit {
                    player: 1 - actor,
                    unit_id: id,
                };
                g.unit_mut(target).unwrap().lane = Some(0);
                target
            })
            .collect();
        unit(&mut g, actor, 71, Some(0));
        play_stranger(&mut g, actor, false);
        let mut values = Vec::new();
        for bird in birds {
            let targets = vec![Target::Battlefield { lane: 0 }, bird];
            g.clone()
                .apply(Action::ChooseTriggerTargets {
                    targets: targets.clone(),
                })
                .unwrap();
            assert_eq!(g.board_target_facts(bird).unwrap().printed_energy, 0);
            values.push(g.trigger_return_target_score(actor, &targets).unwrap());
        }
        assert_eq!(values[0], values[1]);
    }
}

fn resolve_stranger(g: &mut Game) {
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}

#[test]
fn bedeviling_counts_live_units_after_responses_instead_of_on_trigger() {
    for actor in 0..2 {
        for increase in [false, true] {
            let mut g = position(actor);
            unit(&mut g, actor, 71, Some(0));
            let enemy = unit(&mut g, 1 - actor, 71, Some(0));
            let extra = (!increase).then(|| unit(&mut g, actor, 71, Some(0)));
            play_stranger(&mut g, actor, false);
            choose_stranger(&mut g, enemy, 0);
            // Controlled response outcomes cross the printed threshold in both directions.
            if let Some(extra) = extra {
                g.return_unit_to_hand(extra);
            } else {
                unit(&mut g, 1 - actor, 71, Some(0));
            }
            resolve_stranger(&mut g);
            assert_eq!(
                matches!(g.decision, Some(Decision::TriggerReturn { .. })),
                increase
            );
            if increase {
                g.apply(Action::Choose { accept: true }).unwrap();
            }
            assert_eq!(g.unit_ref(enemy).is_none(), increase);
        }
    }
}

#[test]
fn bedeviling_target_order_and_field_relation_are_mandatory_and_atomic() {
    let mut g = position(0);
    let friend = unit(&mut g, 0, 71, Some(0));
    let enemy = unit(&mut g, 1, 71, Some(0));
    let elsewhere = unit(&mut g, 1, 71, Some(1));
    play_stranger(&mut g, 0, false);
    let field = Target::Battlefield { lane: 0 };
    for targets in [
        vec![],
        vec![field],
        vec![enemy],
        vec![enemy, field],
        vec![field, friend],
        vec![field, elsewhere],
        vec![Target::Battlefield { lane: usize::MAX }, enemy],
    ] {
        let before = format!("{g:?}");
        assert!(g.apply(Action::ChooseTriggerTargets { targets }).is_err());
        assert_eq!(format!("{g:?}"), before);
    }
    choose_stranger(&mut g, enemy, 0);
    resolve_stranger(&mut g);
    assert!(g.decision.is_none()); // Only three units, including Stranger.
    assert!(g.unit_ref(enemy).is_some());
}

#[test]
fn bedeviling_late_decline_keeps_deflect_and_blocks_further_responses() {
    let mut g = position(0);
    unit(&mut g, 0, 71, Some(0));
    unit(&mut g, 0, 71, Some(0));
    let enemy = unit(&mut g, 1, 273, Some(0));
    play_stranger(&mut g, 0, false);
    g.players[0].power_pool = [0; 6];
    g.players[0].any_power = 1;
    choose_stranger(&mut g, enemy, 0);
    assert_eq!(g.players[0].any_power, 0);
    resolve_stranger(&mut g);
    assert!(matches!(g.decision, Some(Decision::TriggerReturn { .. })));
    let before = format!("{g:?}");
    assert!(g.apply(Action::Pass).is_err());
    assert_eq!(format!("{g:?}"), before);
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(g.players[0].any_power, 0);
    assert!(g.unit_ref(enemy).is_some());
    assert!(g.chain.is_empty());
}

#[test]
fn bedeviling_hidden_restricts_both_targets_and_mistargets_after_source_departure() {
    for hidden in [false, true] {
        let mut g = position(0);
        for _ in 0..3 {
            unit(&mut g, 0, 71, Some(0));
        }
        let enemy = unit(&mut g, 1, 71, Some(0));
        let elsewhere = unit(&mut g, 1, 71, Some(1));
        let source = play_stranger(&mut g, 0, hidden);
        assert_eq!(
            g.valid_trigger_target(Target::Battlefield { lane: 1 }, &[])
                .unwrap(),
            !hidden
        );
        assert!(
            !g.valid_trigger_target(elsewhere, &[Target::Battlefield { lane: 0 }])
                .unwrap()
        );
        choose_stranger(&mut g, enemy, 0);
        g.return_unit_to_hand(source);
        unit(&mut g, 0, 437, Some(0)); // A new same-name object supplies no old source information.
        resolve_stranger(&mut g);
        assert_eq!(
            matches!(g.decision, Some(Decision::TriggerReturn { .. })),
            !hidden
        );
        if !hidden {
            g.apply(Action::Choose { accept: true }).unwrap();
        }
        assert_eq!(g.unit_ref(enemy).is_some(), hidden);
    }
}

#[test]
fn bedeviling_rechecks_target_location_and_physical_identity_with_enough_units_remaining() {
    for remove in [false, true] {
        let mut g = position(0);
        for _ in 0..3 {
            unit(&mut g, 0, 71, Some(0));
        }
        let enemy = unit(&mut g, 1, 71, Some(0));
        play_stranger(&mut g, 0, false);
        choose_stranger(&mut g, enemy, 0);
        if remove {
            g.return_unit_to_hand(enemy);
        } else {
            g.unit_mut(enemy).unwrap().lane = Some(1);
        }
        let replacement = unit(&mut g, 1, 71, Some(0));
        resolve_stranger(&mut g);
        assert!(g.decision.is_none() && g.chain.is_empty());
        assert!(g.unit_ref(replacement).is_some());
    }
}

#[test]
fn bedeviling_no_eligible_target_drops_only_trigger_and_ai_finishes_both_choices() {
    let mut g = position(0);
    unit(&mut g, 0, 71, Some(0));
    unit(&mut g, 1, 71, Some(1));
    let source = play_stranger(&mut g, 0, true);
    assert!(g.unit_ref(source).is_some());
    assert!(g.chain.is_empty() && g.decision.is_none());
    for level in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        let mut g = position(0);
        g.difficulty = level;
        for _ in 0..2 {
            unit(&mut g, 0, 71, Some(0));
        }
        let enemy = unit(&mut g, 1, 71, Some(0));
        play_stranger(&mut g, 0, false);
        let a = g.choose_ai_action().unwrap();
        g.apply(a).unwrap();
        resolve_stranger(&mut g);
        let a = g.choose_ai_action().unwrap();
        assert_eq!(a, Action::Choose { accept: true });
        g.apply(a).unwrap();
        assert!(g.unit_ref(enemy).is_none());
    }
}

fn blade(g: &mut Game, actor: usize) -> u32 {
    g.gear_play_at(actor, 438, None).unwrap();
    g.players[actor].gear.last().unwrap().id
}

fn settle_blade(g: &mut Game) {
    for _ in 0..50 {
        if g.chain.is_empty() && g.decision.is_none() {
            return;
        }
        let action = match g.decision {
            Some(Decision::TriggerMode { choice_id, .. }) => {
                Action::ChooseTriggerMode { choice_id, mode: 0 }
            }
            Some(_) => g.choose_ai_action().unwrap(),
            None => Action::Pass,
        };
        g.apply(action).unwrap();
    }
    panic!("Blade interaction did not settle");
}

#[test]
fn blade_printed_play_cost_is_calm_or_chaos_but_equip_accepts_any_domain() {
    for actor in 0..2 {
        for domain in 0..6 {
            let mut g = position(actor);
            g.players[actor].hand = vec![438];
            g.players[actor].power_pool = [0; 6];
            g.players[actor].power_pool[domain] = 1;
            let action = Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            };
            let before = format!("{g:?}");
            let result = g.apply(action);
            if matches!(domain, 1 | 4) {
                result.unwrap();
                assert_eq!(g.players[actor].energy, 97);
                assert_eq!(g.players[actor].gear[0].card_id, 438);
            } else {
                assert!(result.is_err());
                assert_eq!(format!("{g:?}"), before);
            }
            let mut g = position(actor);
            let gear_id = blade(&mut g, actor);
            let target = unit(&mut g, actor, 71, None);
            g.players[actor].power_pool = [0; 6];
            g.players[actor].power_pool[domain] = 1;
            let cost = g.cost_plan(&Action::Equip { gear_id, target }).unwrap();
            assert_eq!(cost.power[0].domains.len(), 6);
            g.apply(Action::Equip { gear_id, target }).unwrap();
            assert_eq!(g.players[actor].power_pool[domain], 0);
            assert!(g.unit_ref(target).unwrap().ready);
            assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), 5);
            settle_blade(&mut g);
            assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), 8);
            assert!(
                g.unit_keywords(g.unit_ref(target).unwrap())
                    .unwrap()
                    .ganking
            );
        }
    }
}

#[test]
fn blade_exhaust_cost_and_bearer_are_independent_and_costs_precede_responses() {
    for actor in 0..2 {
        for same in [false, true] {
            let mut g = position(actor);
            let gear_id = blade(&mut g, actor);
            let target = unit(&mut g, actor, 71, Some(0));
            let cost_unit = if same {
                target
            } else {
                unit(&mut g, actor, 71, None)
            };
            let Target::Unit {
                unit_id: exhaust_unit_id,
                ..
            } = cost_unit
            else {
                unreachable!()
            };
            g.players[actor].power_pool = [0; 6];
            let action = Action::EquipExhaustUnit {
                gear_id,
                target,
                exhaust_unit_id,
            };
            let before = format!("{g:?}");
            let plan = g.cost_plan(&action).unwrap();
            assert!(plan.power.is_empty());
            assert_eq!(
                plan.object_costs,
                vec![crate::play::ObjectCost::ExhaustUnit {
                    player: actor,
                    unit_id: exhaust_unit_id
                }]
            );
            assert_eq!(g.quote_power_payment(&action).unwrap().suggested.total(), 0);
            assert_eq!(format!("{g:?}"), before);
            assert!(g.gear_actions(actor).contains(&action));
            g.apply(action).unwrap();
            assert!(!g.unit_ref(cost_unit).unwrap().ready);
            assert_eq!(g.unit_ref(target).unwrap().ready, !same);
            assert_eq!(g.players[actor].gear[0].attached_to, None);
            assert_eq!(g.chain.last().unwrap().targets, vec![target]);
            assert_eq!(g.players[actor].energy, 100);
            settle_blade(&mut g);
            assert_eq!(
                g.players[actor].gear[0].attached_to,
                match target {
                    Target::Unit { unit_id, .. } => Some(unit_id),
                    _ => None,
                }
            );
        }
    }
}

#[test]
fn blade_exhaust_rejects_enemy_spent_missing_cost_and_wrong_equipment_atomically() {
    let mut g = position(0);
    let gear_id = blade(&mut g, 0);
    let target = unit(&mut g, 0, 71, None);
    let enemy = unit(&mut g, 1, 71, None);
    let spent = unit(&mut g, 0, 71, None);
    g.unit_mut(spent).unwrap().ready = false;
    let id = |t| match t {
        Target::Unit { unit_id, .. } => unit_id,
        _ => unreachable!(),
    };
    for (cost, bearer, gear) in [
        (id(enemy), target, gear_id),
        (id(spent), target, gear_id),
        (u32::MAX, target, gear_id),
        (id(target), enemy, gear_id),
        (id(target), target, u32::MAX),
    ] {
        let before = format!("{g:?}");
        assert!(
            g.apply(Action::EquipExhaustUnit {
                gear_id: gear,
                target: bearer,
                exhaust_unit_id: cost
            })
            .is_err()
        );
        assert_eq!(format!("{g:?}"), before);
    }
    g.players[0].gear[0].card_id = 3;
    let before = format!("{g:?}");
    assert!(
        g.apply(Action::EquipExhaustUnit {
            gear_id,
            target,
            exhaust_unit_id: id(target)
        })
        .is_err()
    );
    assert_eq!(format!("{g:?}"), before);
    g.players[0].gear[0].card_id = 438;
    g.players[0].power_pool = [0; 6];
    g.legend[0].spell_power = 1;
    assert!(
        g.apply(Action::Equip { gear_id, target }).is_err(),
        "Spell-only power cannot pay Equip"
    );
}

#[test]
fn blade_exhaust_cost_triggers_innovator_above_equip_without_targeting_the_cost_unit() {
    let mut g = position(0);
    let gear_id = blade(&mut g, 0);
    let target = unit(&mut g, 0, 71, None);
    let innovator = unit(&mut g, 0, 429, None);
    let Target::Unit {
        unit_id: exhaust_unit_id,
        ..
    } = innovator
    else {
        unreachable!()
    };
    g.apply(Action::EquipExhaustUnit {
        gear_id,
        target,
        exhaust_unit_id,
    })
    .unwrap();
    assert!(matches!(
        g.decision,
        Some(Decision::TriggerMode { card_id: 429, .. })
    ));
    assert!(matches!(
        g.chain[0].ability,
        Some(ChainAbility::Equip { .. })
    ));
    assert_eq!(g.chain[0].targets, vec![target]);
    let Some(Decision::TriggerMode { choice_id, .. }) = g.decision else {
        unreachable!()
    };
    g.apply(Action::ChooseTriggerMode { choice_id, mode: 0 })
        .unwrap();
    assert_eq!(
        g.chain.iter().map(|item| item.card_id).collect::<Vec<_>>(),
        vec![438, 429]
    );
    settle_blade(&mut g);
    assert_eq!(
        g.players[0]
            .gear
            .iter()
            .filter(|gear| gear.is_gold())
            .count(),
        1
    );
    assert!(
        g.players[0]
            .gear
            .iter()
            .any(|gear| gear.id == gear_id && gear.attached_to.is_some())
    );
}

#[test]
fn blade_counter_or_departed_bearer_does_not_refund_the_exhausted_cost() {
    for counter in [false, true] {
        let mut g = position(0);
        let gear_id = blade(&mut g, 0);
        let target = unit(&mut g, 0, 71, Some(0));
        let cost = unit(&mut g, 0, 71, None);
        let Target::Unit {
            unit_id: exhaust_unit_id,
            ..
        } = cost
        else {
            unreachable!()
        };
        g.apply(Action::EquipExhaustUnit {
            gear_id,
            target,
            exhaust_unit_id,
        })
        .unwrap();
        if counter {
            let chain_id = g.chain.last().unwrap().id;
            // Controlled response outcome: no currently admitted counter can
            // legally choose this friendly-targeting equipment ability.
            g.chain.retain(|item| item.id != chain_id);
        } else {
            g.return_unit_to_hand(target);
        }
        settle_blade(&mut g);
        assert!(!g.unit_ref(cost).unwrap().ready);
        assert_eq!(g.players[0].gear[0].attached_to, None);
    }
}

#[test]
fn blade_departed_cost_unit_does_not_invalidate_attachment_and_attachment_goes_inactive() {
    let mut g = position(0);
    let gear_id = blade(&mut g, 0);
    let target = unit(&mut g, 0, 71, None);
    let cost = unit(&mut g, 0, 71, None);
    let Target::Unit {
        unit_id: exhaust_unit_id,
        ..
    } = cost
    else {
        unreachable!()
    };
    g.apply(Action::EquipExhaustUnit {
        gear_id,
        target,
        exhaust_unit_id,
    })
    .unwrap();
    g.return_unit_to_hand(cost);
    settle_blade(&mut g);
    assert!(g.players[0].gear[0].attached_to.is_some());
    let Target::Unit { unit_id, .. } = target else {
        unreachable!()
    };
    assert!(
        g.apply(Action::EquipExhaustUnit {
            gear_id,
            target,
            exhaust_unit_id: unit_id
        })
        .is_err()
    );
    g.return_unit_to_hand(target);
    assert_eq!(g.players[0].gear[0].attached_to, None);
    let new_target = unit(&mut g, 0, 71, None);
    let Target::Unit { unit_id, .. } = new_target else {
        unreachable!()
    };
    g.apply(Action::EquipExhaustUnit {
        gear_id,
        target: new_target,
        exhaust_unit_id: unit_id,
    })
    .unwrap();
    settle_blade(&mut g);
    assert_eq!(g.players[0].gear[0].attached_to, Some(unit_id));
}

#[test]
fn blade_staged_cost_preserves_ready_state_until_commit_and_cannot_cancel_legal_finalization() {
    for actor in 0..2 {
        let mut g = position(actor);
        let gear_id = blade(&mut g, actor);
        let target = unit(&mut g, actor, 71, None);
        let cost = unit(&mut g, actor, 429, None);
        let Target::Unit {
            unit_id: exhaust_unit_id,
            ..
        } = cost
        else {
            unreachable!()
        };
        let action = Action::EquipExhaustUnit {
            gear_id,
            target,
            exhaust_unit_id,
        };
        g.apply(Action::BeginPayment {
            action: Box::new(action.clone()),
        })
        .unwrap();
        assert!(g.unit_ref(cost).unwrap().ready);
        assert!(g.chain.is_empty());
        assert!(matches!(g.decision, Some(Decision::Payment { .. })));
        let pending = format!("{g:?}");
        assert!(g.apply(Action::Choose { accept: false }).is_err());
        assert_eq!(format!("{g:?}"), pending);
        g.apply(Action::CompletePayment).unwrap();
        assert!(!g.unit_ref(cost).unwrap().ready);
        assert!(matches!(
            g.decision,
            Some(Decision::TriggerMode { card_id: 429, .. })
        ));
        settle_blade(&mut g);
        assert!(
            g.players[actor]
                .gear
                .iter()
                .any(|gear| gear.id == gear_id && gear.attached_to.is_some())
        );
    }
}

#[test]
fn blade_piltovan_energy_discount_does_not_erase_power_or_exhaustion_cost() {
    for alternative in [false, true] {
        let mut g = position(0);
        g.battlefields = vec![8, 9];
        g.battlefield_owner = vec![Some(0), None];
        let gear_id = blade(&mut g, 0);
        let target = unit(&mut g, 0, 71, None);
        let Target::Unit { unit_id, .. } = target else {
            unreachable!()
        };
        g.players[0].power_pool = [0; 6];
        g.players[0].any_power = 1;
        let action = if alternative {
            Action::EquipExhaustUnit {
                gear_id,
                target,
                exhaust_unit_id: unit_id,
            }
        } else {
            Action::Equip { gear_id, target }
        };
        let costs = g.cost_plan(&action).unwrap();
        assert_eq!(costs.modifiers[0].power, 0);
        assert!(!costs.modifiers[0].choose_energy_or_power);
        assert_eq!(costs.power.len(), usize::from(!alternative));
        assert_eq!(costs.object_costs.len(), usize::from(alternative));
        g.apply(action).unwrap();
        assert_eq!(g.unit_ref(target).unwrap().ready, !alternative);
        assert_eq!(g.players[0].any_power, u16::from(alternative));
        assert_eq!(g.play_history.gear_activations[0], 1);
        settle_blade(&mut g);
        assert_eq!(g.play_history.gear_activations[0], 1);
    }
}

fn exile_position(actor: usize) -> Game {
    let mut g = position(actor);
    g.legend[actor] = crate::legends::LegendState::from_legend(10).unwrap();
    g.legend[actor].ready = false; // A passive does not exhaust or require ready.
    unit(&mut g, actor, 71, Some(0));
    g
}

#[test]
fn riven_exile_hand_and_champion_plays_enter_ready_only_at_controlled_fields() {
    for actor in 0..2 {
        for champion in [false, true] {
            for lane in [None, Some(0)] {
                let mut g = exile_position(actor);
                let action = if champion {
                    g.players[actor].champion = Some(439);
                    Action::PlayChampion { lane, target: None }
                } else {
                    g.players[actor].hand.push(439);
                    Action::PlayCardWithTargets {
                        hand_index: 0,
                        lane,
                        targets: vec![],
                    }
                };
                let before = format!("{g:?}");
                let preview = g.play_choices(&action).unwrap();
                assert_eq!(preview.legend_entry_ready.is_some(), lane.is_some());
                assert_eq!(format!("{g:?}"), before, "preview is read-only");
                g.apply(action).unwrap();
                drive(&mut g);
                let riven = g.players[actor]
                    .units
                    .iter()
                    .find(|u| u.card_id == 439)
                    .unwrap();
                assert_eq!(riven.ready, lane.is_some());
                assert_eq!(riven.lane, lane);
                assert_eq!(g.players[actor].energy, 96);
                assert!(!g.legend[actor].ready);
                assert!(
                    g.trigger_state.modes_used.is_empty(),
                    "entry is not an exhaust event"
                );
                assert!(g.chain.is_empty() && g.decision.is_none());
            }
        }
    }
}

#[test]
fn riven_exile_does_not_ready_the_opponents_play_or_gear() {
    for actor in 0..2 {
        let mut g = exile_position(actor);
        g.legend.swap(0, 1);
        g.players[actor].hand.push(439);
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: Some(0),
            targets: vec![],
        })
        .unwrap();
        assert!(!g.players[actor].units.last().unwrap().ready);
        let mut g = exile_position(actor);
        g.players[actor].hand.push(438);
        let action = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        };
        assert!(
            g.play_choices(&action)
                .unwrap()
                .legend_entry_ready
                .is_none()
        );
        g.apply(action).unwrap();
        assert!(g.players[actor].units[0].ready);
        assert!(g.players[actor].gear[0].attached_to.is_none());
    }
}

#[test]
fn riven_exile_token_plays_use_the_location_chosen_when_finalized() {
    for actor in 0..2 {
        for fixed in [false, true] {
            for lane in [None, Some(0)] {
                let mut g = exile_position(actor);
                g.queue_token_play(actor, 62, 1, if fixed { Some(lane) } else { None }, false);
                assert_eq!(
                    g.players[actor].units.len(),
                    1,
                    "pending token is not on board"
                );
                g.after_resolution().unwrap();
                if !fixed {
                    let action = Action::PlaceToken { lane };
                    assert_eq!(
                        g.play_choices(&action)
                            .unwrap()
                            .legend_entry_ready
                            .is_some(),
                        lane.is_some()
                    );
                    g.apply(action).unwrap();
                }
                drive(&mut g);
                let token = g.players[actor].units.iter().find(|u| u.token).unwrap();
                assert_eq!(token.ready, lane.is_some());
                assert_eq!(token.lane, lane);
                assert_eq!(g.players[actor].energy, 100, "no Accelerate payment");
            }
        }
    }
}

#[test]
fn riven_exile_fixed_token_checks_control_at_finalization_not_creation() {
    for actor in 0..2 {
        for controlled_on_entry in [false, true] {
            let mut g = exile_position(actor);
            g.battlefield_owner[0] = if controlled_on_entry {
                None
            } else {
                Some(actor)
            };
            g.queue_token_play(actor, 62, 1, Some(Some(0)), false);
            // Model an older pending effect changing control before this token
            // is finalized. This fixture is not a claim of Baron redirection.
            g.battlefield_owner[0] = if controlled_on_entry {
                Some(actor)
            } else {
                None
            };
            g.after_resolution().unwrap();
            let token = g.players[actor].units.iter().find(|u| u.token).unwrap();
            assert_eq!(token.ready, controlled_on_entry);
        }
    }
}

#[test]
fn riven_exile_hidden_and_instructed_trash_or_banish_plays_are_units_too() {
    for actor in 0..2 {
        let mut g = exile_position(actor);
        g.players[actor].hidden.push(crate::hidden::HiddenCard {
            id: 59000,
            card_id: 13,
            lane: 0,
            hidden_turn: 0,
            revealed: false,
        });
        g.apply(Action::PlayHidden {
            hidden_id: 59000,
            targets: vec![],
        })
        .unwrap();
        assert!(
            g.players[actor]
                .units
                .iter()
                .find(|u| u.card_id == 13)
                .unwrap()
                .ready
        );
        assert_eq!(g.players[actor].energy, 100);
        for banish in [false, true] {
            for lane in [None, Some(0)] {
                let mut g = exile_position(actor);
                if banish {
                    g.players[actor].banish.push(439);
                    g.queue_banish_play(actor, 0, InstructedPlayKind::ArcaneShift, None);
                } else {
                    g.players[actor].discard.push(439);
                    g.queue_card_play(actor, 0, InstructedPlayKind::BaitedHook);
                }
                g.after_resolution().unwrap();
                assert!(matches!(g.decision, Some(Decision::PendingPlay { .. })));
                g.apply(Action::FinalizePendingPlay {
                    lane,
                    targets: vec![],
                    repeat_targets: vec![],
                    repeat: false,
                    accelerate: false,
                    bonus: false,
                })
                .unwrap();
                drive(&mut g);
                assert_eq!(g.players[actor].units.last().unwrap().ready, lane.is_some());
                assert_eq!(g.players[actor].energy, 100);
            }
        }
    }
}

#[test]
fn riven_exile_does_not_grant_permission_to_play_to_uncontrolled_battlefields() {
    for actor in 0..2 {
        for owner in [None, Some(1 - actor)] {
            let mut g = exile_position(actor);
            g.battlefield_owner[1] = owner;
            g.players[actor].hand.push(439);
            let before = format!("{g:?}");
            assert!(
                g.apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: Some(1),
                    targets: vec![],
                })
                .is_err()
            );
            assert_eq!(format!("{g:?}"), before);
        }
    }
}

#[test]
fn riven_exile_suspended_payment_keeps_ready_entry_and_can_roll_back() {
    for actor in 0..2 {
        let mut g = exile_position(actor);
        g.players[actor].hand.push(439);
        let action = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: Some(0),
            targets: vec![],
        };
        g.apply(Action::BeginPayment {
            action: Box::new(action.clone()),
        })
        .unwrap();
        assert_eq!(g.players[actor].units.len(), 1);
        assert!(g.players[actor].hand.is_empty());
        g.apply(Action::Choose { accept: false }).unwrap();
        assert_eq!(g.players[actor].hand, vec![439]);
        assert_eq!(g.players[actor].energy, 100);
        g.apply(Action::BeginPayment {
            action: Box::new(action),
        })
        .unwrap();
        g.apply(Action::CompletePayment).unwrap();
        drive(&mut g);
        assert!(g.players[actor].units.last().unwrap().ready);
        assert_eq!(g.players[actor].energy, 96);
    }
}

#[test]
fn riven_exile_instructed_play_at_an_uncontrolled_field_still_enters_exhausted() {
    for actor in 0..2 {
        for owner in [None, Some(1 - actor)] {
            let mut g = exile_position(actor);
            g.battlefield_owner[1] = owner;
            if let Some(owner) = owner {
                unit(&mut g, owner, 71, Some(1));
            }
            g.players[actor].banish.push(439);
            g.queue_banish_play(actor, 0, InstructedPlayKind::ThrillOfTheHunt, None);
            g.after_resolution().unwrap();
            let action = Action::FinalizePendingPlay {
                lane: Some(1),
                targets: vec![],
                repeat_targets: vec![],
                repeat: false,
                accelerate: false,
                bonus: false,
            };
            assert!(
                g.play_choices(&action)
                    .unwrap()
                    .legend_entry_ready
                    .is_none()
            );
            g.apply(action).unwrap();
            assert!(!g.players[actor].units.last().unwrap().ready);
        }
    }
}
