//! Controlled C17 positions exercise linked instructions, physical identity and
//! CR354.3/358.5/359.3.e.15 ordering through the public action dispatcher.
use super::*;
use crate::game::play::{
    ExecutionChoices, PlayChoices, PlayGrantKind, PlayOrigin, RepeatSelection,
};

fn pending(g: &mut Game) {
    for _ in 0..80 {
        if matches!(g.decision, Some(Decision::PendingPlay { .. })) {
            return;
        }
        match &g.decision {
            None if !g.chain.is_empty() => g.apply(Action::Pass).unwrap(),
            Some(Decision::OrderTriggers { options, .. }) => g
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            other => panic!("Expected instructed play, found {other:?}; {:?}", g.chain),
        }
    }
    panic!("Pending play did not arrive");
}
fn finalize(lane: Option<usize>) -> Action {
    Action::FinalizePendingPlay {
        lane,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate: false,
        bonus: false,
    }
}
fn start_replay(g: &mut Game, spell: usize, target: Target) {
    let action = hand_action(g, spell, vec![target]);
    g.apply(action).unwrap();
    pending(g);
}
fn zero_resources(g: &mut Game, player: usize) {
    let p = &mut g.players[player];
    p.energy = 0;
    p.energy_pool = 0;
    p.power_pool = [0; 6];
    p.any_power = 0;
    p.runes = [0; 6];
    p.exhausted_runes = [0; 6];
    p.gear.clear();
}

#[test]
fn c17_burn_away_banishes_unit_or_gear_without_kill_and_burns_two() {
    for gear_target in [false, true] {
        let mut g = position();
        g.players[0].deck = vec![70, 71, 72];
        let target = if gear_target {
            gear(&mut g, 1, 10, false, None)
        } else {
            unit(&mut g, 1, 374, false, Some(0))
        };
        cast(&mut g, 379, vec![target]);
        assert_eq!(
            g.players[1].banish,
            vec![if gear_target { 10 } else { 374 }]
        );
        assert_eq!(g.players[0].deck, vec![70]);
        assert_eq!(g.players[0].discard, vec![72, 71, 379]);
        assert!(g.players[1].discard.is_empty() && g.pending_triggers.is_empty());
    }
}
#[test]
fn c17_burn_away_might_rechecked_but_burn_is_independent() {
    let mut g = position();
    let target = unit(&mut g, 1, 72, false, None);
    g.players[0].deck = vec![70, 71, 72];
    let action = hand_action(&mut g, 379, vec![target]);
    g.apply(action).unwrap();
    g.modify_might(target, 1, 0).unwrap();
    drive(&mut g, false, false);
    assert!(g.unit_ref(target).is_some());
    assert!(g.players[1].banish.is_empty());
    assert_eq!(g.players[0].discard, vec![72, 71, 379]);
    let invalid = hand_action(&mut g, 379, vec![target]);
    let before = format!("{g:?}");
    assert!(g.apply(invalid).is_err());
    assert_eq!(format!("{g:?}"), before);
}
#[test]
fn c17_banish_tokens_vanish_and_never_replay_creator() {
    for spell in [379, 380, 381, 382] {
        let mut g = position();
        let target = unit(&mut g, 0, 54, true, None);
        cast(&mut g, spell, vec![target]);
        assert!(g.players[0].units.is_empty() && g.pending_cards().is_empty());
        assert!(g.players[0].banish.is_empty());
        assert_eq!(g.players[0].discard.last(), Some(&spell));
        assert!(!g.players[0].discard.contains(&54));
    }
    let mut g = position();
    let target = gear(&mut g, 1, 13, true, None);
    cast(&mut g, 379, vec![target]);
    assert!(g.players[1].gear.is_empty() && g.players[1].banish.is_empty());
}
#[test]
fn c17_portal_fresh_object_base_cost_waiver_and_equipment_detachment() {
    let mut g = position();
    let target = unit(&mut g, 0, 70, false, Some(0));
    let item = gear(&mut g, 0, 10, false, Some(uid(target)));
    g.players[0].gear[0].ready = false;
    let old = g.unit_mut(target).unwrap();
    old.damage = 3;
    old.buff = 2;
    old.stunned = true;
    old.ready = true;
    old.temporary_buff = 4;
    old.protected = true;
    g.empowered_units.push(uid(target));
    g.granted_temporary_units.push(uid(target));
    start_replay(&mut g, 380, target);
    assert!(g.unit_ref(target).is_none());
    assert_eq!(g.players[0].gear[0].attached_to, None);
    assert!(!g.players[0].gear[0].ready);
    assert_eq!(g.players[0].discard, vec![380]);
    assert_eq!(g.pending_cards()[0].play_origin(), PlayOrigin::Banish);
    let quote = g.cost_plan(&finalize(None)).unwrap();
    assert_eq!(quote.energy, 0);
    assert!(quote.power.is_empty());
    let before = format!("{g:?}");
    assert!(g.apply(finalize(Some(0))).is_err());
    assert_eq!(format!("{g:?}"), before);
    g.players[0].energy = 0;
    g.players[0].energy_pool = 0;
    g.players[0].power_pool = [0; 6];
    g.apply(finalize(None)).unwrap();
    let fresh = &g.players[0].units[0];
    assert_ne!(fresh.id, uid(target));
    assert_eq!(fresh.lane, None);
    assert_eq!((fresh.damage, fresh.buff, fresh.temporary_buff), (0, 0, 0));
    assert!(!fresh.ready && !fresh.stunned && !fresh.protected);
    assert!(
        !g.empowered_units.contains(&uid(target))
            && !g.granted_temporary_units.contains(&uid(target))
    );
    assert!(g.board_target_facts(item).is_some());
    assert!(g.players[0].banish.is_empty());
}
#[test]
fn c17_temporal_owner_and_exact_location_override_ordinary_deployment() {
    for owner in 0..2 {
        for location in [None, Some(0), Some(1)] {
            let mut g = position();
            let target = unit(&mut g, owner, 71, false, location);
            g.battlefield_owner = [Some(1 - owner); 2];
            start_replay(&mut g, 381, target);
            assert_eq!(g.priority(), owner);
            assert_eq!(
                g.pending_cards()[0].instruction.destination,
                PlayDestination::Exact(location)
            );
            let wrong = if location.is_none() { Some(0) } else { None };
            let before = format!("{g:?}");
            assert!(g.apply(finalize(wrong)).is_err());
            assert_eq!(format!("{g:?}"), before);
            zero_resources(&mut g, owner);
            g.apply(finalize(location)).unwrap();
            assert_eq!(g.players[owner].units[0].lane, location);
        }
    }
}
#[test]
fn c17_temporal_captures_resolution_location_and_hidden_rechecks_locality() {
    let mut g = position();
    let target = unit(&mut g, 1, 71, false, Some(0));
    let action = hand_action(&mut g, 381, vec![target]);
    g.apply(action).unwrap();
    g.unit_mut(target).unwrap().lane = Some(1);
    pending(&mut g);
    assert_eq!(
        g.pending_cards()[0].instruction.destination,
        PlayDestination::Exact(Some(1))
    );
    let mut g = position();
    let target = unit(&mut g, 1, 71, false, Some(0));
    unit(&mut g, 0, 71, false, Some(0));
    g.battlefield_owner[0] = Some(0);
    let hidden_id = g.next_unit_id;
    g.next_unit_id += 1;
    g.players[0].hidden.push(crate::hidden::HiddenCard {
        id: hidden_id,
        card_id: 381,
        lane: 0,
        hidden_turn: 0,
        revealed: false,
    });
    g.apply(Action::PlayHidden {
        hidden_id,
        targets: vec![target],
    })
    .unwrap();
    g.unit_mut(target).unwrap().lane = Some(1);
    drive(&mut g, false, false);
    assert!(g.unit_ref(target).is_some() && g.pending_cards().is_empty());
    assert_eq!(g.players[0].discard, vec![381]);
}
#[test]
fn c17_thrill_requires_any_battlefield_even_enemy_held_and_cannot_decline() {
    for lane in 0..2 {
        let mut g = position();
        let target = unit(&mut g, 0, 71, false, None);
        g.battlefield_owner = [Some(1); 2];
        start_replay(&mut g, 382, target);
        assert!(!g.can_decline_trash_play().unwrap());
        assert!(g.apply(Action::Choose { accept: false }).is_err());
        assert!(g.apply(finalize(None)).is_err());
        zero_resources(&mut g, 0);
        g.apply(finalize(Some(lane))).unwrap();
        assert_eq!(g.players[0].units[0].lane, Some(lane));
    }
}
#[test]
fn c17_arcane_parent_damage_and_self_banish_precede_child_choices() {
    let mut g = position();
    let friendly = unit(&mut g, 0, 70, false, None);
    let enemy = unit(&mut g, 1, 70, false, Some(0));
    let action = hand_action(&mut g, 383, vec![friendly, enemy]);
    g.apply(action).unwrap();
    pending(&mut g);
    assert_eq!(g.unit_ref(enemy).unwrap().damage, 3);
    assert_eq!(g.players[0].banish, vec![383]);
    assert!(g.players[0].discard.is_empty() && g.unit_ref(friendly).is_none());
    assert!(g.chain.iter().all(|i| i.card_id != 383));
    // Ordinary placement still applies to Arcane Shift.
    assert!(g.apply(finalize(Some(1))).is_err());
    zero_resources(&mut g, 0);
    g.apply(finalize(None)).unwrap();
    assert_eq!(g.players[0].units[0].lane, None);
    assert_eq!(g.players[0].banish, vec![383]);
}
#[test]
fn c17_arcane_mistargets_are_independent_and_preserve_target_slots() {
    for invalid_friend in [false, true] {
        let mut g = position();
        let friendly = unit(&mut g, 0, 71, false, None);
        let enemy = unit(&mut g, 1, 70, false, Some(0));
        let action = hand_action(&mut g, 383, vec![friendly, enemy]);
        g.apply(action).unwrap();
        if invalid_friend {
            g.return_unit_to_hand(friendly);
        } else {
            g.unit_mut(enemy).unwrap().lane = None;
        }
        if invalid_friend {
            drive(&mut g, false, false);
        } else {
            pending(&mut g);
        }
        assert_eq!(
            g.unit_ref(enemy).unwrap().damage,
            if invalid_friend { 3 } else { 0 }
        );
        assert_eq!(g.pending_cards().len(), usize::from(!invalid_friend));
        assert_eq!(g.players[0].banish, vec![383]);
    }
}
#[test]
fn c17_arcane_leaves_chain_before_granted_repeat_even_with_new_targets() {
    let mut g = position();
    let f1 = unit(&mut g, 0, 71, false, None);
    let f2 = unit(&mut g, 0, 72, false, None);
    let e1 = unit(&mut g, 1, 70, false, Some(0));
    let e2 = unit(&mut g, 1, 70, false, Some(1));
    g.grant_play(
        0,
        ChainSource::Card { card_id: 265 },
        PlayGrantKind::RepeatNextSpell,
    );
    let base = hand_action(&mut g, 383, vec![f1, e1]);
    let option = g.play_choices(&base).unwrap().repeats[0].id;
    g.apply(Action::WithPlayChoices {
        action: Box::new(base),
        choices: PlayChoices {
            repeats: vec![RepeatSelection {
                option_id: option,
                choices: ExecutionChoices {
                    targets: vec![f2, e2],
                    mode: None,
                },
            }],
            ..Default::default()
        },
    })
    .unwrap();
    pending(&mut g);
    assert_eq!(g.pending_cards().len(), 1);
    assert!(g.unit_ref(f2).is_some());
    assert_eq!(g.unit_ref(e1).unwrap().damage, 3);
    assert_eq!(g.unit_ref(e2).unwrap().damage, 0);
    assert_eq!(g.players[0].banish, vec![383]);
    assert!(g.resolution_progress().is_none());
}
#[test]
fn c17_failed_child_returns_only_itself_to_banish_and_preserves_parent_effects() {
    let mut g = position();
    let friendly = unit(&mut g, 0, 71, false, None);
    let enemy = unit(&mut g, 1, 70, false, Some(0));
    let action = hand_action(&mut g, 383, vec![friendly, enemy]);
    g.apply(action).unwrap();
    pending(&mut g);
    g.silenced[0] = true;
    assert!(g.can_decline_trash_play().unwrap());
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(g.players[0].banish, vec![71, 383]);
    assert!(g.players[0].discard.is_empty() && g.pending_cards().is_empty());
    assert_eq!(g.unit_ref(enemy).unwrap().damage, 3);
}
#[test]
fn c17_silenced_owner_stays_banished_and_parent_still_resolves() {
    let mut g = position();
    let target = unit(&mut g, 1, 71, false, None);
    g.silenced[1] = true;
    cast(&mut g, 381, vec![target]);
    assert_eq!(g.players[1].banish, vec![71]);
    assert_eq!(g.players[0].discard, vec![381]);
}
#[test]
fn c17_base_waiver_keeps_optional_costs_and_quotes_do_not_mutate() {
    let mut g = position();
    let target = unit(&mut g, 0, 145, false, None);
    start_replay(&mut g, 380, target);
    zero_resources(&mut g, 0);
    let mut action = finalize(None);
    if let Action::FinalizePendingPlay { accelerate, .. } = &mut action {
        *accelerate = true;
    }
    let before = format!("{g:?}");
    let plan = g.cost_plan(&action).unwrap();
    assert_eq!(plan.energy, 1);
    assert!(!plan.power.is_empty());
    assert_eq!(format!("{g:?}"), before);
    assert!(g.apply(action.clone()).is_err());
    assert_eq!(format!("{g:?}"), before);
    g.players[0].energy = 1;
    g.players[0].energy_pool = 1;
    g.players[0].power_pool[domain_index(Domain::Mind)] = 1;
    g.apply(action).unwrap();
    assert!(g.players[0].units[0].ready);
    assert_eq!(g.players[0].energy, 0);
}

#[test]
fn c17_linked_replay_does_not_follow_a_different_copy_of_the_same_card() {
    let mut g = position();
    let original = unit(&mut g, 0, 71, false, None);
    let action = hand_action(&mut g, 380, vec![original]);
    g.apply(action).unwrap();
    g.return_unit_to_hand(original);
    g.players[0].hand.clear();
    let replacement = unit(&mut g, 0, 71, false, None);
    drive(&mut g, false, false);
    assert!(g.unit_ref(replacement).is_some());
    assert!(g.pending_cards().is_empty());
}
#[test]
fn c17_countered_parent_neither_banishes_nor_plays_its_unit() {
    let mut g = position();
    let friendly = unit(&mut g, 0, 71, false, None);
    let enemy = unit(&mut g, 1, 70, false, Some(0));
    let action = hand_action(&mut g, 383, vec![friendly, enemy]);
    g.apply(action).unwrap();
    let parent = g.chain[0].id;
    let counter = hand_action(&mut g, 86, vec![Target::Spell { chain_id: parent }]);
    g.apply(counter).unwrap();
    drive(&mut g, false, false);
    assert!(g.unit_ref(friendly).is_some());
    assert_eq!(g.unit_ref(enemy).unwrap().damage, 0);
    assert!(g.players[0].banish.is_empty() && g.pending_cards().is_empty());
    assert!(g.players[0].discard.contains(&383));
}
#[test]
fn c17_rengar_waits_until_replay_finishes_and_can_target_the_fresh_unit() {
    let mut g = position();
    g.legend[0] = crate::legends::LegendState::from_legend(8).unwrap();
    let original = unit(&mut g, 0, 71, false, None);
    start_replay(&mut g, 382, original);
    assert!(!g.pending_triggers.iter().any(|t| matches!(
        t.effect,
        PendingTriggerEffect::Choice(Decision::LegendTargets { .. })
    )));
    g.apply(finalize(Some(0))).unwrap();
    assert!(matches!(
        g.decision,
        Some(Decision::LegendTargets { player: 0 })
    ));
    let fresh = Target::Unit {
        player: 0,
        unit_id: g.players[0].units[0].id,
    };
    assert_ne!(fresh, original);
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![fresh],
    })
    .unwrap();
    drive(&mut g, false, false);
    assert_eq!(g.unit_ref(fresh).unwrap().temporary_buff, 1);
}
#[test]
fn c17_ezreal_counts_repeat_choices_even_when_arcane_ends_before_repeating() {
    let mut g = position();
    g.legend[0] = crate::legends::LegendState::from_legend(9).unwrap();
    let friendly = unit(&mut g, 0, 71, false, None);
    let enemy = unit(&mut g, 1, 70, false, Some(0));
    g.grant_play(
        0,
        ChainSource::Card { card_id: 265 },
        PlayGrantKind::RepeatNextSpell,
    );
    let base = hand_action(&mut g, 383, vec![friendly, enemy]);
    let choice = g.play_choices(&base).unwrap().repeats[0].id;
    let action = Action::WithPlayChoices {
        action: Box::new(base),
        choices: PlayChoices {
            repeats: vec![RepeatSelection {
                option_id: choice,
                choices: ExecutionChoices {
                    targets: vec![friendly, enemy],
                    mode: None,
                },
            }],
            ..Default::default()
        },
    };
    assert_eq!(g.ezreal_progress(0), 0);
    let before = format!("{g:?}");
    g.cost_plan(&action).unwrap();
    assert_eq!(format!("{g:?}"), before);
    g.apply(action).unwrap();
    assert_eq!(g.ezreal_progress(0), 2);
    pending(&mut g);
    assert_eq!(g.unit_ref(enemy).unwrap().damage, 3);
}
#[test]
fn c17_arcane_damage_keeps_spell_bonuses_and_rechecks_enemy_field() {
    let mut g = position();
    g.battlefields[0] = 3;
    let friendly = unit(&mut g, 0, 71, false, None);
    let enemy = unit(&mut g, 1, 70, false, Some(0));
    g.grant_play(
        0,
        ChainSource::Card { card_id: 253 },
        PlayGrantKind::BonusNextSpell(2),
    );
    let action = hand_action(&mut g, 383, vec![friendly, enemy]);
    g.apply(action).unwrap();
    pending(&mut g);
    assert_eq!(g.unit_ref(enemy).unwrap().damage, 6);
}
#[test]
fn c17_arcane_via_fizz_banishes_itself_instead_of_recycling_or_relocating_another_copy() {
    let mut g = position();
    let friendly = unit(&mut g, 0, 71, false, None);
    let enemy = unit(&mut g, 1, 70, false, Some(0));
    g.players[0].discard = vec![383, 383];
    g.queue_card_play(0, 0, InstructedPlayKind::Fizz);
    g.after_resolution().unwrap();
    let mut action = finalize(None);
    if let Action::FinalizePendingPlay { targets, .. } = &mut action {
        *targets = vec![friendly, enemy];
    }
    g.apply(action).unwrap();
    pending(&mut g);
    assert_eq!(g.players[0].banish, vec![383]);
    assert_eq!(g.players[0].discard, vec![383]);
    assert!(g.players[0].deck.iter().all(|&id| id != 383));
}
