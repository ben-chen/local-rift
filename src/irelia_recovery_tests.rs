//! Reviewed recovery and Vision behavior; authored positions, ordinary choices.
use super::*;
use test_support::{decline_legend_action, position, reject_unchanged, settle_unopposed_chain};

fn play(lane: Option<usize>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane,
        targets: vec![],
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

fn recovery(actor: usize, source: CardId, trash: Vec<CardId>) -> Game {
    let mut g = position(actor, 0, 475001);
    g.players[actor].hand = vec![source];
    g.players[actor].discard = trash;
    g.battlefield_owner[0] = Some(actor);
    g.apply(play(Some(0))).unwrap();
    for _ in 0..60 {
        if matches!(g.decision, Some(Decision::TrashTarget { source_card_id, .. }) if source_card_id == source)
        {
            return g;
        }
        advance(&mut g);
    }
    panic!("recovery trigger was not offered");
}

#[test]
fn recovery_chooses_one_matching_card_before_responses_and_neither_draws_nor_plays_it() {
    for actor in 0..2 {
        for (source, chosen, wrong) in [(474, 96, 8), (475, 8, 96)] {
            let mut g = recovery(actor, source, vec![wrong, chosen, chosen]);
            let draws = g.trigger_state.draws;
            let resources = (g.players[actor].energy, g.players[actor].power_pool);
            assert!(
                !g.chain
                    .iter()
                    .any(|i| matches!(i.ability, Some(ChainAbility::ReturnChosen { .. })))
            );
            for indices in [vec![], vec![0], vec![1, 2], vec![1, 1], vec![3]] {
                reject_unchanged(&mut g, Action::SelectCards { indices });
            }
            assert_eq!(g.legal_actions().unwrap().len(), 2);
            g.apply(Action::SelectCards { indices: vec![2] }).unwrap();
            assert!(g.players[actor].hand.is_empty());
            assert!(matches!(g.chain.last().unwrap().ability,
                Some(ChainAbility::ReturnChosen { trash_index: 2, card_id }) if card_id == chosen));
            settle_unopposed_chain(&mut g);
            assert_eq!(g.players[actor].hand, vec![chosen]);
            assert_eq!(g.players[actor].discard, vec![wrong, chosen]);
            assert_eq!(
                g.players[actor].units.len(),
                1,
                "returning a card is not playing it"
            );
            assert_eq!(g.trigger_state.draws, draws, "returning is not drawing");
            assert_eq!(
                (g.players[actor].energy, g.players[actor].power_pool),
                resources
            );
        }
    }
}

#[test]
fn recovery_with_no_matching_target_does_not_put_an_invalid_ability_on_chain() {
    for actor in 0..2 {
        for (source, wrong) in [(474, 8), (475, 96)] {
            for trash in [vec![], vec![wrong]] {
                let mut g = recovery(actor, source, trash.clone());
                assert_eq!(
                    g.legal_actions().unwrap(),
                    vec![Action::SelectCards { indices: vec![] }]
                );
                g.apply(Action::SelectCards { indices: vec![] }).unwrap();
                assert!(
                    !g.chain
                        .iter()
                        .any(|i| matches!(i.ability, Some(ChainAbility::ReturnChosen { .. })))
                );
                settle_unopposed_chain(&mut g);
                assert!(g.players[actor].hand.is_empty());
                assert_eq!(g.players[actor].discard, trash);
            }
        }
    }
}

#[test]
fn recovery_survives_an_opposing_response_that_returns_its_source() {
    for actor in 0..2 {
        for (source, chosen) in [(474, 96), (475, 8)] {
            let mut g = recovery(actor, source, vec![chosen]);
            let source_id = g.players[actor].units[0].id;
            g.players[1 - actor].hand = vec![11]; // Gust is a real opposing Reaction.
            g.apply(Action::SelectCards { indices: vec![0] }).unwrap();
            if g.priority() == actor {
                g.apply(Action::Pass).unwrap();
            }
            assert_eq!(g.priority(), 1 - actor);
            g.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![Target::Unit {
                    player: actor,
                    unit_id: source_id,
                }],
            })
            .unwrap();
            settle_unopposed_chain(&mut g);
            assert!(g.players[actor].units.is_empty());
            assert_eq!(g.players[actor].hand, vec![source, chosen]);
            assert!(g.players[actor].discard.is_empty());
        }
    }
}

#[test]
fn recovery_tracks_selected_copy_across_trash_removal_recycle_and_reentry() {
    // Exercise the central zone-movement boundary after a normally finalized
    // choice. Direct helper calls isolate physical identity from card timing.
    for actor in 0..2 {
        for (source, chosen) in [(474, 96), (475, 8)] {
            for remove in [0, 1, 2] {
                let mut g = recovery(actor, source, vec![chosen, chosen, chosen]);
                g.apply(Action::SelectCards { indices: vec![1] }).unwrap();
                g.recycle_cards(actor, &[remove]);
                // An identical newly arrived occurrence cannot replace a target.
                g.players[actor].discard.push(chosen);
                settle_unopposed_chain(&mut g);
                assert_eq!(g.players[actor].hand.len(), usize::from(remove != 1));
                assert_eq!(
                    g.players[actor].discard.len(),
                    if remove == 1 { 3 } else { 2 }
                );
            }
        }
    }
}

#[test]
fn recovery_reference_is_restored_only_by_cancelling_the_same_pending_trash_card() {
    for actor in 0..2 {
        for (source, chosen) in [(474, 96), (475, 8)] {
            for removed in [0, 1] {
                let mut g = recovery(actor, source, vec![chosen, chosen, chosen]);
                g.apply(Action::SelectCards { indices: vec![1] }).unwrap();
                let before = g.chain.clone();
                let pending = g.payment_detach_trash(actor, removed).unwrap();
                assert_eq!(
                    g.chain
                        .last()
                        .unwrap()
                        .ability
                        .as_ref()
                        .unwrap()
                        .trash_index(),
                    Some(if removed == 1 { usize::MAX } else { 0 })
                );
                g.restore_failed_card(pending);
                assert_eq!(g.chain, before);
                settle_unopposed_chain(&mut g);
                assert_eq!(g.players[actor].hand, vec![chosen]);
            }
            let mut g = recovery(actor, source, vec![chosen, chosen]);
            g.apply(Action::SelectCards { indices: vec![0] }).unwrap();
            let pending = g.payment_detach_trash(actor, 0).unwrap();
            g.players[actor].hand.push(pending.card_id); // committed departure
            settle_unopposed_chain(&mut g);
            assert_eq!(g.players[actor].hand, vec![chosen]);
            assert_eq!(g.players[actor].discard, vec![chosen]);
        }
    }
}

#[test]
fn burnout_invalidates_recovery_even_when_an_identical_card_returns_to_trash() {
    for actor in 0..2 {
        let mut g = recovery(actor, 474, vec![96, 96]);
        g.apply(Action::SelectCards { indices: vec![0] }).unwrap();
        g.players[actor].deck.clear();
        g.move_top_cards(actor, 1, true).unwrap();
        g.players[actor].discard.push(96);
        let hand = g.players[actor].hand.clone();
        settle_unopposed_chain(&mut g);
        assert_eq!(g.players[actor].hand, hand);
        assert_eq!(g.players[actor].discard, vec![96]);
    }
}

#[test]
fn sai_scout_vision_keeps_or_recycles_without_drawing() {
    for actor in 0..2 {
        for recycle in [false, true] {
            let mut g = position(actor, 0, 473001);
            g.players[actor].hand = vec![473];
            g.players[actor].deck = vec![96, 71, 8];
            let draws = g.trigger_state.draws;
            g.apply(play(Some(0))).unwrap();
            for _ in 0..40 {
                if matches!(g.decision, Some(Decision::Predict { .. })) {
                    break;
                }
                advance(&mut g);
            }
            assert!(
                matches!(g.decision, Some(Decision::Predict { player, card_id: 8 }) if player == actor)
            );
            g.apply(Action::Choose { accept: recycle }).unwrap();
            settle_unopposed_chain(&mut g);
            assert_eq!(
                g.players[actor].deck,
                if recycle {
                    vec![8, 96, 71]
                } else {
                    vec![96, 71, 8]
                }
            );
            assert!(g.players[actor].hand.is_empty());
            assert_eq!(g.trigger_state.draws, draws);
            let scout = &g.players[actor].units[0];
            assert_eq!(
                (scout.card_id, scout.lane, scout.ready),
                (473, Some(0), false)
            );
        }
    }
}

#[test]
fn every_ai_level_resolves_mandatory_recovery_in_both_seats() {
    for difficulty in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        for actor in 0..2 {
            for (source, chosen, wrong) in [(474, 96, 8), (475, 8, 96)] {
                let mut g = recovery(actor, source, vec![wrong, chosen]);
                g.difficulty = difficulty;
                assert!(g.step_ai().unwrap());
                assert!(matches!(g.chain.last().unwrap().ability,
                    Some(ChainAbility::ReturnChosen { card_id, .. }) if card_id == chosen));
                settle_unopposed_chain(&mut g);
                assert_eq!(g.players[actor].hand, vec![chosen]);
            }
        }
    }
}
