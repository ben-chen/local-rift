//! Public-trash spell targets: physical occurrences, response windows, and costs.
use super::*;
use test_support::{decline_legend_action, position, reject_unchanged, settle_unopposed_chain};

fn play(targets: Vec<Target>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets,
    }
}
fn advance(g: &mut Game) {
    let action = if let Some(action) = decline_legend_action(g) {
        action
    } else if let Some(Decision::OrderTriggers { options, .. }) = &g.decision {
        Action::OrderTrigger {
            trigger_id: options[0].id,
        }
    } else {
        assert!(
            g.decision.is_none() && !g.chain.is_empty(),
            "{:?}",
            g.decision
        );
        Action::Pass
    };
    g.apply(action).unwrap();
}
fn pending(g: &mut Game) {
    for _ in 0..60 {
        if matches!(
            g.decision,
            Some(Decision::PendingPlay {
                kind: InstructedPlayKind::TheHarrowing,
                ..
            })
        ) {
            return;
        }
        advance(g);
    }
    panic!("Harrowing did not instruct the selected unit play");
}
fn finalize(accelerate: bool) -> Action {
    Action::FinalizePendingPlay {
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate,
        bonus: false,
    }
}

#[test]
fn trash_references_follow_exact_copies_through_removal_refusal_and_burnout() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 47801);
        g.players[actor].discard = vec![8, 470, 470];
        let first = g.trash_target(actor, 1).unwrap();
        let second = g.trash_target(actor, 2).unwrap();
        g.remove_from_trash(actor, 0);
        assert_eq!(g.trash_target_index(first), Some(0));
        assert_eq!(g.trash_target_index(second), Some(1));
        assert!(g.same_target(second, g.trash_target(actor, 1).unwrap()));
        let detached = g.payment_detach_trash(actor, 0).unwrap();
        assert_eq!(g.trash_target_index(first), None);
        assert_eq!(g.trash_target_index(second), Some(0));
        g.payment_restore_trash(detached);
        assert_eq!(g.trash_target_index(first), Some(0));
        assert_eq!(g.trash_target_index(second), Some(1));
        g.remove_from_trash(actor, 0);
        g.players[actor].discard.push(470);
        assert_eq!(
            g.trash_target_index(first),
            None,
            "an identical arrival is a new object"
        );
        assert_eq!(g.trash_target_index(second), Some(0));
        let detached = g.payment_detach_trash(actor, 0).unwrap();
        g.players[actor].deck.clear();
        g.draw(actor, 1).unwrap();
        assert_eq!(g.trash_target_index(second), None);
        g.payment_restore_trash(detached);
        assert_eq!(
            g.trash_target_index(second),
            Some(0),
            "refusal restores the card absent from burnout"
        );
        assert_eq!(g.trash_target_index(first), None);
    }
}

#[test]
fn shadows_accepts_zero_one_or_two_units_from_either_trash_without_drawing_or_playing() {
    for actor in 0..2 {
        for owners in [
            vec![],
            vec![actor],
            vec![1 - actor],
            vec![actor, actor],
            vec![actor, 1 - actor],
        ] {
            let mut g = position(actor, 0, 47901);
            g.players[actor].hand = vec![479];
            for p in &mut g.players {
                p.discard = vec![8, 470, 470];
            }
            let targets: Vec<_> = owners
                .iter()
                .enumerate()
                .map(|(slot, &owner)| g.trash_target(owner, slot + 1).unwrap())
                .collect();
            let before_draws = g.trigger_state.draws;
            g.apply(play(targets.clone())).unwrap();
            assert_eq!(
                g.chain.last().unwrap().targets,
                targets,
                "targets are finalized before responses"
            );
            assert!(g.players.iter().all(|p| p.hand.is_empty()));
            settle_unopposed_chain(&mut g);
            for (owner, state) in g.players.iter().enumerate() {
                assert_eq!(
                    state.hand.len(),
                    owners.iter().filter(|&&p| p == owner).count()
                );
            }
            assert_eq!(g.trigger_state.draws, before_draws);
            assert!(g.players.iter().all(|p| p.units.is_empty()));
            assert!(g.players[actor].discard.contains(&479));
        }
    }
}

#[test]
fn choosing_a_trash_card_does_not_trigger_irelias_board_unit_ability() {
    // CR 141.1.b and 355.9.a.1: a unit card in trash is not a board unit.
    // Keep the legend ready so an accidental trigger cannot be masked by cost.
    for actor in 0..2 {
        for spell in [478, 479] {
            let mut g = position(actor, 0, 47909);
            g.legend[actor].ready = true;
            g.players[actor].hand = vec![spell];
            g.players[actor].discard = vec![470];
            let target = g.trash_target(actor, 0).unwrap();
            g.apply(play(vec![target])).unwrap();
            assert!(g.pending_triggers.is_empty());
            assert!(g.decision.is_none());
            assert_eq!(g.chain.len(), 1);
            assert!(g.legend[actor].ready);
        }
        // Positive control: targeting an actual friendly board unit does trigger.
        let mut g = position(actor, 0, 47910);
        g.legend[actor].ready = true;
        let id = test_support::ready_unit(&mut g, actor, 470);
        g.players[actor].hand = vec![425];
        g.apply(play(vec![Target::Unit {
            player: actor,
            unit_id: id,
        }]))
        .unwrap();
        assert!(decline_legend_action(&g).is_some());
    }
}

#[test]
fn automatic_flash_keeps_both_targets_and_an_explicit_single_choice_stays_single() {
    // Regression found in the Spirit Wheel API series: legacy automatic play
    // retained only Flash's first target, also losing the second chosen event.
    for actor in 0..2 {
        for automatic in [false, true] {
            let mut g = position(actor, 0, 47911);
            let first = test_support::ready_unit(&mut g, actor, 470);
            test_support::ready_unit(&mut g, actor, 470);
            for unit in &mut g.players[actor].units {
                unit.lane = Some(0);
            }
            g.battlefield_owner[0] = Some(actor);
            g.players[actor].hand = vec![9];
            g.apply(Action::PlayCard {
                hand_index: 0,
                lane: None,
                target: (!automatic).then_some(Target::Unit {
                    player: actor,
                    unit_id: first,
                }),
            })
            .unwrap();
            let expected = if automatic { 2 } else { 1 };
            assert_eq!(g.chain.last().unwrap().targets.len(), expected);
            let trigger_choices = match &g.decision {
                Some(Decision::Legend { .. }) => 1,
                Some(Decision::OrderTriggers { options, .. }) => options.len(),
                other => panic!("missing Irelia choice: {other:?}"),
            };
            assert_eq!(trigger_choices, expected);
            settle_unopposed_chain(&mut g);
            assert_eq!(
                g.players[actor]
                    .units
                    .iter()
                    .filter(|u| u.lane.is_none())
                    .count(),
                expected
            );
        }
    }
}

#[test]
fn trash_target_validation_rejects_wrong_kinds_duplicates_aliases_and_future_references() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 47902);
        g.players[actor].hand = vec![479];
        g.players[actor].discard = vec![8, 470, 472];
        let spell = g.trash_target(actor, 0).unwrap();
        let unit = g.trash_target(actor, 1).unwrap();
        let gear = g.trash_target(actor, 2).unwrap();
        for targets in [
            vec![spell],
            vec![gear],
            vec![unit, unit],
            vec![unit, unit, unit],
        ] {
            reject_unchanged(&mut g, play(targets));
        }
        g.remove_from_trash(actor, 0);
        let alias = g.trash_target(actor, 0).unwrap();
        assert_ne!(unit, alias);
        assert!(g.same_target(unit, alias));
        reject_unchanged(&mut g, play(vec![unit, alias]));
        assert!(
            !g.valid_card_target(actor, 479, None, alias, &[unit])
                .unwrap()
        );
        let future = Target::Trash {
            player: actor,
            card_id: 470,
            reference: crate::trash::Reference {
                index: 0,
                revision: usize::MAX,
            },
        };
        reject_unchanged(&mut g, play(vec![future]));
        g.players[actor].hand = vec![478];
        let opposing = {
            g.players[1 - actor].discard.push(470);
            g.trash_target(1 - actor, 0).unwrap()
        };
        reject_unchanged(&mut g, play(vec![opposing]));
        reject_unchanged(&mut g, play(vec![]));
    }
}

#[test]
fn shadows_keeps_surviving_target_after_a_real_response_burns_out_other_trash() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 47903);
        g.players[actor].hand = vec![479];
        g.players[actor].discard = vec![470];
        g.players[1 - actor].hand = vec![133]; // Downstage Dramatics: Reaction, draw one.
        g.players[1 - actor].discard = vec![96];
        g.players[1 - actor].deck.clear();
        let own = g.trash_target(actor, 0).unwrap();
        let opposing = g.trash_target(1 - actor, 0).unwrap();
        g.apply(play(vec![own, opposing])).unwrap();
        while g.priority() != 1 - actor {
            advance(&mut g);
        }
        g.apply(play(vec![])).unwrap();
        assert_eq!(g.chain.len(), 2);
        settle_unopposed_chain(&mut g);
        assert_eq!(g.players[actor].hand, vec![470]);
        assert_eq!(g.players[1 - actor].hand, vec![96]);
        assert_eq!(g.trash_target_index(opposing), None);
        assert_eq!(
            g.players[actor].score, 1,
            "the response actually caused burnout"
        );
        assert_eq!(g.trigger_state.draws[1 - actor], 1);
    }
}

#[test]
fn shadows_repeat_preserves_each_execution_and_does_not_retarget_an_identical_arrival() {
    for actor in 0..2 {
        for same in [false, true] {
            let mut g = position(actor, 0, 47904);
            g.players[actor].hand = vec![479];
            g.players[actor].discard = vec![470, 470, 470];
            let a = g.trash_target(actor, 0).unwrap();
            let b = g.trash_target(actor, if same { 0 } else { 2 }).unwrap();
            g.grant_play(
                actor,
                ChainSource::Card { card_id: 57 },
                PlayGrantKind::RepeatNextSpell,
            );
            let action = play(vec![a]);
            let option_id = g.play_choices(&action).unwrap().repeats[0].id;
            g.apply(Action::WithPlayChoices {
                action: Box::new(action),
                choices: PlayChoices {
                    repeats: vec![crate::play::RepeatSelection {
                        option_id,
                        choices: ExecutionChoices {
                            targets: vec![b],
                            ..Default::default()
                        },
                    }],
                    ..Default::default()
                },
            })
            .unwrap();
            settle_unopposed_chain(&mut g);
            assert_eq!(g.players[actor].hand.len(), if same { 1 } else { 2 });
            assert_eq!(
                g.players[actor]
                    .discard
                    .iter()
                    .filter(|&&c| c == 470)
                    .count(),
                if same { 2 } else { 1 }
            );
            assert_eq!(
                g.cards_played_this_turn(actor),
                1,
                "Repeat isn't another card play"
            );
        }
    }
}

#[test]
fn harrowing_waits_for_responses_then_waives_only_base_energy_and_restores_refused_unit() {
    for actor in 0..2 {
        for extra_power in [0, 1] {
            let mut g = position(actor, 0, 47802);
            g.players[actor].hand = vec![478];
            g.players[actor].discard = vec![8, 474];
            g.players[actor].energy_pool = 6;
            g.players[actor].energy = 6;
            g.players[actor].power_pool = [0; 6];
            g.players[actor].power_pool[Domain::Chaos as usize] = 2 + extra_power;
            let target = g.trash_target(actor, 1).unwrap();
            g.apply(play(vec![target])).unwrap();
            assert!(g.pending_card.is_none());
            assert_eq!(g.players[actor].discard, vec![8, 474]);
            pending(&mut g);
            assert_eq!(
                g.players[actor].discard,
                vec![8, 478],
                "parent spell finishes before child finalization"
            );
            assert_eq!(g.players[actor].energy, 0);
            assert!(g.players[actor].units.is_empty());
            if extra_power == 0 {
                reject_unchanged(&mut g, finalize(false));
                g.apply(Action::Choose { accept: false }).unwrap();
                settle_unopposed_chain(&mut g);
                assert_eq!(g.players[actor].discard, vec![8, 474, 478]);
                assert_eq!(g.trash_target_index(target), Some(1));
            } else {
                g.apply(finalize(false)).unwrap();
                assert_eq!(g.players[actor].units[0].card_id, 474);
                assert_eq!(g.players[actor].power_pool[Domain::Chaos as usize], 0);
                for _ in 0..30 {
                    if matches!(
                        g.decision,
                        Some(Decision::TrashTarget {
                            source_card_id: 474,
                            ..
                        })
                    ) {
                        break;
                    }
                    advance(&mut g);
                }
                assert!(
                    matches!(
                        g.decision,
                        Some(Decision::TrashTarget {
                            source_card_id: 474,
                            ..
                        })
                    ),
                    "the recovered unit's actual on-play trigger must happen"
                );
                g.apply(Action::SelectCards { indices: vec![] }).unwrap();
                settle_unopposed_chain(&mut g);
            }
        }
    }
}

#[test]
fn harrowing_repeat_tracks_independent_copies_and_never_replays_a_departed_target() {
    for actor in 0..2 {
        for same in [false, true] {
            let mut g = position(actor, 0, 47803);
            g.players[actor].hand = vec![478];
            g.players[actor].discard = vec![470, 470];
            let a = g.trash_target(actor, 0).unwrap();
            let b = g.trash_target(actor, usize::from(!same)).unwrap();
            g.grant_play(
                actor,
                ChainSource::Card { card_id: 57 },
                PlayGrantKind::RepeatNextSpell,
            );
            let action = play(vec![a]);
            let option_id = g.play_choices(&action).unwrap().repeats[0].id;
            g.apply(Action::WithPlayChoices {
                action: Box::new(action),
                choices: PlayChoices {
                    repeats: vec![crate::play::RepeatSelection {
                        option_id,
                        choices: ExecutionChoices {
                            targets: vec![b],
                            ..Default::default()
                        },
                    }],
                    ..Default::default()
                },
            })
            .unwrap();
            let mut instructed = 0;
            for _ in 0..80 {
                if matches!(g.decision, Some(Decision::PendingPlay { .. })) {
                    instructed += 1;
                    g.apply(finalize(false)).unwrap();
                } else if g.decision.is_none() && g.chain.is_empty() {
                    break;
                } else {
                    advance(&mut g);
                }
            }
            assert!(g.chain.is_empty() && g.decision.is_none());
            assert_eq!(instructed, if same { 1 } else { 2 });
            assert_eq!(g.players[actor].units.len(), instructed);
            assert_eq!(
                g.players[actor]
                    .discard
                    .iter()
                    .filter(|&&c| c == 478)
                    .count(),
                1
            );
        }
    }
}

#[test]
fn harrowing_does_not_waive_accelerate_additional_energy() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 47804);
        g.players[actor].hand = vec![478];
        g.players[actor].discard = vec![96];
        g.players[actor].energy_pool = 6;
        g.players[actor].energy = 6;
        let target = g.trash_target(actor, 0).unwrap();
        g.apply(play(vec![target])).unwrap();
        pending(&mut g);
        reject_unchanged(&mut g, finalize(true));
        g.apply(finalize(false)).unwrap();
        settle_unopposed_chain(&mut g);
        assert_eq!(g.players[actor].energy, 0);
        assert_eq!(g.players[actor].units[0].card_id, 96);
        assert!(!g.players[actor].units[0].ready);
    }
}

#[test]
fn recovery_ai_scores_actual_targets_and_all_levels_use_a_helpful_two_card_return() {
    for actor in 0..2 {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let mut g = position(actor, 0, 47905);
            g.difficulty = difficulty;
            g.players[actor].hand = vec![479];
            g.players[actor].discard = vec![470, 470];
            g.players[1 - actor].discard = vec![470];
            let own = g.trash_target(actor, 0).unwrap();
            let other = g.trash_target(actor, 1).unwrap();
            let enemy = g.trash_target(1 - actor, 0).unwrap();
            assert_eq!(g.canonical_action_score(&play(vec![])).unwrap(), 0);
            assert!(g.ai_avoids_empty_removal(&play(vec![])).unwrap());
            assert!(g.canonical_action_score(&play(vec![enemy])).unwrap() < 0);
            assert!(
                g.canonical_action_score(&play(vec![own, other])).unwrap()
                    > g.canonical_action_score(&play(vec![own])).unwrap()
            );
            assert_eq!(g.prepare_targets(actor, 479, None, &[]).unwrap().len(), 2);
            let action = g.choose_ai_action().unwrap();
            g.apply(action.clone()).unwrap();
            let item = g
                .chain
                .iter()
                .find(|item| item.card_id == 479)
                .unwrap_or_else(|| {
                    panic!(
                        "{difficulty:?} seat{actor} skipped the useful recovery: {:?}",
                        g.log
                    )
                });
            assert_eq!(
                item.targets.len(),
                2,
                "{difficulty:?} seat{actor}: {action:?}"
            );
            assert!(
                item.targets
                    .iter()
                    .all(|t| matches!(t,Target::Trash {player,..} if *player==actor))
            );
        }
    }
}
