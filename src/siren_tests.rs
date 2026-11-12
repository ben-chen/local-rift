//! CR355.4, 448, 457–458 and 820.2: physical Move history and finalization choices.
use super::*;
use crate::play::{
    ExecutionChoices, MoveDestination, PlayChoices, PlayGrant, PlayGrantKind, RepeatSelection,
};

fn position(owner: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 427100, Difficulty::Tactical);
    g.choose_starting_player(owner).unwrap();
    g.start_match().unwrap();
    g.battlefields = vec![2, 2];
    g.battlefield_owner = vec![None; 2];
    for p in &mut g.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.discard.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 50;
        p.energy = 50;
        p.power_pool = [20; 6];
    }
    g
}
fn unit(g: &mut Game, player: usize, id: u32, lane: Option<usize>) -> Target {
    g.players[player].units.push(Unit {
        id,
        card_id: 73,
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
    Target::Unit {
        player,
        unit_id: id,
    }
}
fn settle(g: &mut Game) {
    if g.chain.is_empty() {
        g.after_resolution().unwrap();
    }
    for _ in 0..80 {
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
    panic!("Chain did not settle");
}
fn cast(g: &mut Game, id: usize, lane: Option<usize>, targets: Vec<Target>) {
    let actor = g.priority();
    let hand_index = g.players[actor].hand.len();
    g.players[actor].hand.push(id);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane,
        targets,
    })
    .unwrap();
}
fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(
        g.apply(action.clone()).is_err(),
        "Accepted {action:?}, priority {}",
        g.priority()
    );
    assert_eq!(format!("{g:?}"), before);
}
fn draft(g: &mut Game, id: usize, target: Target, lane: Option<usize>) -> Action {
    g.players[g.priority()].hand = vec![id];
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane,
        targets: vec![target],
    }
}
fn repeat(
    g: &mut Game,
    action: Action,
    target: Target,
    destination: Option<MoveDestination>,
) -> Action {
    g.play_grants.push(PlayGrant {
        id: 8800,
        player: g.priority(),
        source: ChainSource::Card { card_id: 427 },
        expires_turn: g.turn,
        kind: PlayGrantKind::RepeatNextSpell,
    });
    let option_id = g.play_choices(&action).unwrap().repeats[0].id;
    Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats: vec![RepeatSelection {
                option_id,
                choices: ExecutionChoices {
                    targets: vec![target],
                    destination,
                    ..Default::default()
                },
            }],
            ..Default::default()
        },
    }
}

#[test]
fn siren_requires_an_enemy_physical_unit_that_actually_moved_for_both_seats() {
    for actor in 0..2 {
        let mut g = position(actor);
        let enemy = unit(&mut g, 1 - actor, 500, None);
        let friend = unit(&mut g, actor, 501, None);
        let idle = unit(&mut g, 1 - actor, 502, Some(0));
        assert!(
            g.card_target_candidates(actor, 427, Some(1), &[])
                .unwrap()
                .is_empty()
        );
        cast(&mut g, 4, Some(0), vec![enemy]);
        settle(&mut g);
        assert!(g.unit_moved_this_turn(1 - actor, 500));
        g.move_effect(actor, friend, Some(1), false).unwrap();
        settle(&mut g);
        assert_eq!(
            g.card_target_candidates(actor, 427, None, &[]).unwrap(),
            vec![enemy]
        );
        for target in [friend, idle] {
            let action = draft(&mut g, 427, target, None);
            reject(&mut g, action);
        }
        let action = draft(&mut g, 427, enemy, None);
        let before = g.players[actor].energy;
        g.apply(action).unwrap();
        assert_eq!(g.players[actor].energy, before - 2);
        settle(&mut g);
        assert_eq!(g.unit_ref(enemy).unwrap().lane, None);
        assert!(g.unit_moved_this_turn(1 - actor, 500));
        assert_eq!(
            g.players[actor]
                .discard
                .iter()
                .filter(|&&id| id == 427)
                .count(),
            1
        );
    }
}

#[test]
fn basic_group_and_forced_moves_record_only_the_units_that_moved() {
    for actor in 0..2 {
        let mut g = position(actor);
        unit(&mut g, actor, 500, None);
        unit(&mut g, actor, 501, None);
        unit(&mut g, actor, 502, None);
        g.apply(Action::MoveGroup {
            unit_ids: vec![500, 501],
            lane: Some(0),
        })
        .unwrap();
        settle(&mut g);
        assert!(g.unit_moved_this_turn(actor, 500));
        assert!(g.unit_moved_this_turn(actor, 501));
        assert!(!g.unit_moved_this_turn(actor, 502));
        g.apply(Action::Move {
            unit_id: 502,
            lane: Some(1),
        })
        .unwrap();
        assert!(g.unit_moved_this_turn(actor, 502));
    }
}

#[test]
fn entry_recall_prevented_and_noop_moves_do_not_create_history() {
    let mut g = position(0);
    g.players[0].hand = vec![73];
    g.apply(Action::PlayCard {
        hand_index: 0,
        lane: None,
        target: None,
    })
    .unwrap();
    let id = g.players[0].units[0].id;
    let a = Target::Unit {
        player: 0,
        unit_id: id,
    };
    assert!(!g.unit_moved_this_turn(0, id));
    g.move_effect(0, a, None, false).unwrap();
    assert!(!g.unit_moved_this_turn(0, id));
    let b = unit(&mut g, 0, 500, Some(0));
    g.recall_unit(0, 500, false, false);
    assert!(!g.unit_moved_this_turn(0, 500));
    g.immobile_units.push((0, 500));
    g.move_effect(1, b, Some(0), false).unwrap();
    assert!(!g.unit_moved_this_turn(0, 500));
    g.immobile_units.clear();
    g.move_effect(1, b, Some(0), false).unwrap();
    g.recall_unit(0, 500, false, false);
    assert!(g.unit_moved_this_turn(0, 500));
}

#[test]
fn movement_history_expires_at_turn_end_and_does_not_transfer_to_a_replayed_card() {
    let mut g = position(0);
    let a = unit(&mut g, 0, 500, None);
    g.move_effect(0, a, Some(0), false).unwrap();
    cast(&mut g, 377, None, vec![a]);
    settle(&mut g);
    assert!(!g.unit_moved_this_turn(0, 500));
    let index = g.players[0].hand.iter().position(|&id| id == 73).unwrap();
    g.apply(Action::PlayCard {
        hand_index: index,
        lane: None,
        target: None,
    })
    .unwrap();
    let id = g.players[0].units[0].id;
    assert_ne!(id, 500);
    assert!(!g.unit_moved_this_turn(0, id));
    g.move_effect(
        0,
        Target::Unit {
            player: 0,
            unit_id: id,
        },
        Some(0),
        false,
    )
    .unwrap();
    g.apply(Action::EndTurn).unwrap();
    settle(&mut g);
    assert!(!g.unit_moved_this_turn(0, id));
    assert!(g.moved_units.is_empty());
}

#[test]
fn history_belongs_to_the_unit_not_its_controller() {
    let mut g = position(0);
    let a = unit(&mut g, 0, 500, None);
    g.move_effect(0, a, Some(0), false).unwrap();
    let changed = g.players[0].units.remove(0);
    g.players[1].units.push(changed);
    assert!(!g.unit_moved_this_turn(0, 500));
    assert!(g.unit_moved_this_turn(1, 500));
    assert_eq!(
        g.card_target_candidates(0, 427, None, &[]).unwrap(),
        vec![Target::Unit {
            player: 1,
            unit_id: 500
        }]
    );
}

#[test]
fn swaps_record_each_successful_half_and_vex_only_blocks_its_controllers_move() {
    for smoke in [false, true] {
        for blocked in [false, true] {
            let mut g = position(0);
            let a = unit(&mut g, 0, 500, None);
            let b = unit(&mut g, 0, 501, Some(0));
            if blocked {
                g.immobile_units.push((0, 500));
            }
            if smoke {
                g.grant_temporary(a);
                g.exchange_temporary_locations(0, a, b).unwrap();
            } else {
                g.resolve_swap(0, 500, b).unwrap();
            }
            assert_eq!(g.unit_moved_this_turn(0, 500), !blocked);
            assert!(g.unit_moved_this_turn(0, 501));
        }
    }
    let mut g = position(0);
    let a = unit(&mut g, 0, 500, None);
    g.movement_locks.push((0, 500));
    g.move_effect(0, a, Some(0), false).unwrap();
    assert!(!g.unit_moved_this_turn(0, 500));
    g.move_effect(1, a, Some(0), false).unwrap();
    assert!(g.unit_moved_this_turn(0, 500));
}

#[test]
fn siren_pays_deflect_and_does_not_trigger_an_enemys_when_you_choose_me() {
    for card in [0, 92] {
        let mut g = position(0);
        let a = unit(&mut g, 1, 500, None);
        g.players[1].units[0].card_id = card;
        g.move_effect(0, a, Some(0), false).unwrap();
        settle(&mut g);
        let action = draft(&mut g, 427, a, None);
        let power: u16 = g
            .cost_plan(&action)
            .unwrap()
            .power
            .iter()
            .map(|c| c.amount)
            .sum();
        assert_eq!(power, 1); // Both units have printed Deflect.
        g.apply(action).unwrap();
        assert_eq!(g.chain.len(), 1);
        settle(&mut g);
        assert_eq!(g.unit_ref(a).unwrap().lane, None);
    }
}

#[test]
fn independent_repeat_destinations_are_frozen_and_resolve_in_order() {
    for card in [4, 14, 427] {
        for actor in 0..2 {
            let mut g = position(actor);
            let owner = if card == 14 { actor } else { 1 - actor };
            let a = unit(&mut g, owner, 500, None);
            g.move_effect(actor, a, Some(0), false).unwrap();
            g.move_effect(actor, a, None, false).unwrap();
            let base = draft(&mut g, card, a, Some(0));
            let action = repeat(
                &mut g,
                base,
                a,
                Some(MoveDestination::Battlefield { lane: 1 }),
            );
            let before = format!("{g:?}");
            assert_eq!(
                g.play_target_view(&action, 0).unwrap().destination,
                Some(MoveDestination::Battlefield { lane: 0 })
            );
            assert_eq!(
                g.play_target_view(&action, 1).unwrap().destination,
                Some(MoveDestination::Battlefield { lane: 1 })
            );
            g.cost_plan(&action).unwrap();
            assert_eq!(format!("{g:?}"), before);
            g.apply(action).unwrap();
            let spell = g.chain.last().unwrap();
            assert_eq!(
                spell.executions[0].destination,
                Some(MoveDestination::Battlefield { lane: 0 })
            );
            assert_eq!(
                spell.executions[1].destination,
                Some(MoveDestination::Battlefield { lane: 1 })
            );
            settle(&mut g);
            assert_eq!(g.unit_ref(a).unwrap().lane, Some(1));
            assert_eq!(
                g.players[actor]
                    .discard
                    .iter()
                    .filter(|&&id| id == card)
                    .count(),
                1
            );
        }
    }
}

#[test]
fn repeat_base_is_explicit_and_omission_inherits_original_destination() {
    for destination in [None, Some(MoveDestination::Base)] {
        let mut g = position(0);
        let a = unit(&mut g, 1, 500, None);
        g.move_effect(0, a, Some(0), false).unwrap();
        let base = draft(&mut g, 427, a, Some(1));
        let action = repeat(&mut g, base, a, destination);
        g.apply(action).unwrap();
        assert_eq!(
            g.chain.last().unwrap().executions[1].destination,
            Some(destination.unwrap_or(MoveDestination::Battlefield { lane: 1 }))
        );
        settle(&mut g);
        assert_eq!(
            g.unit_ref(a).unwrap().lane,
            if destination.is_some() { None } else { Some(1) }
        );
    }
}

#[test]
fn each_repeat_destination_must_be_legal_before_any_execution() {
    for destination in [
        MoveDestination::Base,
        MoveDestination::Battlefield { lane: 2 },
    ] {
        let mut g = position(0);
        let a = unit(&mut g, 1, 500, None);
        g.move_effect(0, a, Some(0), false).unwrap();
        g.move_effect(0, a, None, false).unwrap();
        let base = draft(&mut g, 427, a, Some(0));
        let action = repeat(&mut g, base, a, Some(destination));
        assert!(g.play_choices(&action).is_err());
        reject(&mut g, action.clone());
        reject(
            &mut g,
            Action::BeginPayment {
                action: Box::new(action),
            },
        );
    }
    let mut g = position(0);
    let a = unit(&mut g, 0, 500, None);
    let base = draft(&mut g, 425, a, None);
    let action = repeat(
        &mut g,
        base,
        a,
        Some(MoveDestination::Battlefield { lane: 1 }),
    );
    assert!(g.play_choices(&action).is_err());
    reject(&mut g, action);
}

#[test]
fn repeated_target_previews_use_their_own_destination() {
    let mut g = position(0);
    let a = unit(&mut g, 1, 500, None);
    let b = unit(&mut g, 1, 501, None);
    g.move_effect(0, a, Some(0), false).unwrap();
    g.move_effect(0, b, Some(1), false).unwrap();
    let base = draft(&mut g, 427, a, None);
    let mut action = repeat(
        &mut g,
        base,
        a,
        Some(MoveDestination::Battlefield { lane: 1 }),
    );
    if let Action::WithPlayChoices { choices, .. } = &mut action {
        choices.repeats[0].choices.targets.clear();
    }
    let view = g.play_target_view(&action, 1).unwrap();
    assert_eq!(view.candidates, vec![a]);
    assert!(!view.candidates.contains(&b));
}

#[test]
fn hunker_response_prevents_siren_without_erasing_prior_movement() {
    for actor in 0..2 {
        let mut g = position(actor);
        let a = unit(&mut g, 1 - actor, 500, None);
        g.move_effect(actor, a, Some(0), false).unwrap();
        cast(&mut g, 427, None, vec![a]);
        g.apply(Action::Pass).unwrap();
        cast(&mut g, 425, None, vec![a]);
        settle(&mut g);
        assert_eq!(g.unit_ref(a).unwrap().lane, Some(0));
        assert!(g.unit_moved_this_turn(1 - actor, 500));
        assert!(
            g.card_target_candidates(actor, 427, None, &[])
                .unwrap()
                .contains(&a)
        );
    }
}

#[test]
fn siren_rechecks_protection_location_allegiance_and_physical_existence() {
    for response in 0..4 {
        let mut g = position(0);
        let a = unit(&mut g, 1, 500, None);
        g.move_effect(0, a, Some(0), false).unwrap();
        cast(&mut g, 427, None, vec![a]);
        match response {
            0 => g.players[1].units[0].protected = true,
            1 => {
                g.move_effect(1, a, None, false).unwrap();
            }
            2 => {
                let changed = g.players[1].units.remove(0);
                g.players[0].units.push(changed);
            }
            _ => {
                g.bounce(a);
                unit(&mut g, 1, 501, Some(0));
            }
        }
        settle(&mut g);
        match response {
            0 => assert_eq!(g.unit_ref(a).unwrap().lane, Some(0)),
            1 => assert_eq!(g.unit_ref(a).unwrap().lane, None),
            2 => assert_eq!(g.players[0].units[0].lane, Some(0)),
            _ => assert_eq!(g.players[1].units[0].lane, Some(0)),
        }
        assert!(g.players[0].discard.contains(&427));
    }
}

#[test]
fn staged_payment_preserves_repeat_destinations_and_refusal_restores_the_game() {
    let mut g = position(0);
    let a = unit(&mut g, 1, 500, None);
    g.move_effect(0, a, Some(0), false).unwrap();
    settle(&mut g);
    let base = draft(&mut g, 427, a, Some(1));
    let action = repeat(&mut g, base, a, Some(MoveDestination::Base));
    let before = format!("{g:?}");
    g.apply(Action::BeginPayment {
        action: Box::new(action.clone()),
    })
    .unwrap();
    assert_eq!(
        g.play_target_view(&Action::CompletePayment, 1)
            .unwrap()
            .destination,
        Some(MoveDestination::Base)
    );
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(format!("{g:?}"), before);
    g.apply(Action::BeginPayment {
        action: Box::new(action),
    })
    .unwrap();
    g.apply(Action::CompletePayment).unwrap();
    settle(&mut g);
    assert_eq!(g.unit_ref(a).unwrap().lane, None);
}

#[test]
fn ai_candidates_include_field_to_field_and_independent_repeat_moves() {
    let mut g = position(0);
    let a = unit(&mut g, 1, 500, None);
    g.move_effect(0, a, Some(0), false).unwrap();
    let base = draft(&mut g, 427, a, None);
    let _ = repeat(
        &mut g,
        base,
        a,
        Some(MoveDestination::Battlefield { lane: 1 }),
    );
    let actions = g.canonical_play_candidates(100).unwrap();
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::WithPlayChoices { action, .. }
        if matches!(action.as_ref(), Action::PlayCard { lane: Some(1), .. })))
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::WithPlayChoices { action, choices }
        if matches!(action.as_ref(), Action::PlayCard { lane: Some(1), .. })
        && choices.repeats.iter().any(|r| r.choices.destination == Some(MoveDestination::Base))))
    );
    for action in actions {
        g.clone().apply(action).unwrap();
    }
}
