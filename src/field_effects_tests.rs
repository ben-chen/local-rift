//! Printed battlefield instructions, exercised through ordinary player actions.
use super::*;
use crate::battlefields::CANDLELIT_SANCTUM_ID;
use test_support::{position, ready_unit, reject_unchanged};

fn board(actor: usize, field: usize) -> Game {
    let mut g = position(actor, 0, 182019);
    g.battlefields = vec![field, 3];
    g
}

fn settle(g: &mut Game) {
    for _ in 0..80 {
        if matches!(g.decision, Some(Decision::PredictCards { .. })) {
            return;
        }
        if g.decision.is_none() && g.chain.is_empty() && g.showdown.is_none() {
            return;
        }
        let action = test_support::decline_legend_action(g).unwrap_or_else(|| {
            if let Some(Decision::OrderTriggers { options, .. }) = &g.decision {
                Action::OrderTrigger {
                    trigger_id: options[0].id,
                }
            } else {
                Action::Pass
            }
        });
        g.apply(action).unwrap();
    }
    panic!("battlefield did not settle: {:?}", g.decision);
}

fn conquer(g: &mut Game, actor: usize, card: CardId) -> u32 {
    let id = ready_unit(g, actor, card);
    g.apply(Action::Move {
        unit_id: id,
        lane: Some(0),
    })
    .unwrap();
    settle(g);
    id
}

fn look(g: &Game) -> (u32, Vec<CardId>) {
    let Some(Decision::PredictCards {
        choice_id,
        cards,
        source,
        ..
    }) = &g.decision
    else {
        panic!("expected Sanctum choice: {:?}", g.decision);
    };
    assert_eq!(
        *source,
        Some(ChainSource::Battlefield {
            lane: 0,
            battlefield_id: CANDLELIT_SANCTUM_ID
        })
    );
    (*choice_id, cards.clone())
}

#[test]
fn monument_control_does_not_score_or_trigger_before_each_players_third_turn() {
    for actor in 0..2 {
        let mut g = board(actor, FORGOTTEN_MONUMENT_ID);
        // This unit normally draws on Conquer. Neither that nor the legend's
        // Conquer trigger may fire when the Scoring process is prohibited.
        conquer(&mut g, actor, 49);
        assert_eq!(g.battlefield_owner[0], Some(actor));
        assert_eq!(g.players[actor].score, 0);
        assert!(g.players[actor].hand.is_empty());
        assert!(!g.player_scored_this_turn[actor][0]);
        assert_eq!(g.player_turns_started(actor), 1);
        for turn in 2..=3 {
            g.apply(Action::EndTurn).unwrap();
            settle(&mut g);
            g.apply(Action::EndTurn).unwrap();
            settle(&mut g);
            assert_eq!(g.active, actor);
            assert_eq!(g.player_turns_started(actor), turn);
            assert_eq!(g.players[actor].score, if turn == 3 { 1 } else { 0 });
            // Exactly the ordinary start-of-turn draws; no Conquer draw.
            assert_eq!(g.players[actor].hand.len(), (turn - 1) as usize);
        }
    }
}

#[test]
fn monument_uses_individual_turns_including_extra_turns() {
    for actor in 0..2 {
        let mut g = board(actor, FORGOTTEN_MONUMENT_ID);
        conquer(&mut g, actor, 71);
        g.extra_turns[actor] = 2;
        for expected in 2..=3 {
            g.apply(Action::EndTurn).unwrap();
            settle(&mut g);
            assert_eq!(g.active, actor);
            assert_eq!(g.player_turns_started(actor), expected);
            assert_eq!(g.battlefield_score_unlock(actor, 0), Some(3 - expected));
            assert_eq!(g.battlefield_score_unlock(1 - actor, 0), Some(3));
            assert_eq!(g.players[actor].score, if expected == 3 { 1 } else { 0 });
        }
    }
}

#[test]
fn monument_blocks_final_point_replacement_but_unlocked_scoring_still_draws_and_triggers() {
    for turn in [2, 3] {
        let mut g = board(0, FORGOTTEN_MONUMENT_ID);
        g.turns_taken = [turn, 3];
        g.players[0].score = 7;
        conquer(&mut g, 0, 49);
        assert_eq!(g.players[0].score, 7);
        assert_eq!(g.players[0].hand.len(), if turn == 3 { 2 } else { 0 });
        assert_eq!(g.player_scored_this_turn[0][0], turn == 3);
        assert!(g.winner.is_none());
    }
}

#[test]
fn back_alley_group_move_includes_tokens_responds_before_buff_and_expires() {
    for actor in 0..2 {
        let mut g = board(actor, BACK_ALLEY_BAR_ID);
        let first = ready_unit(&mut g, actor, 71);
        let second = g.create_unit(actor, 49, true, 1);
        for u in &mut g.players[actor].units {
            u.lane = Some(0);
            u.ready = true;
        }
        g.battlefield_owner[0] = Some(actor);
        g.apply(Action::MoveGroup {
            unit_ids: vec![first, second],
            lane: None,
        })
        .unwrap();
        assert!(g.players[actor].units.iter().all(|u| u.temporary_buff == 0));
        assert!(g.decision.is_some() || !g.chain.is_empty());
        settle(&mut g);
        assert!(g.players[actor].units.iter().all(|u| u.temporary_buff == 1));
        g.apply(Action::EndTurn).unwrap();
        settle(&mut g);
        assert!(g.players[actor].units.iter().all(|u| u.temporary_buff == 0));
    }
}

#[test]
fn back_alley_tracks_the_same_object_across_control_change_without_targeting() {
    let mut g = board(0, BACK_ALLEY_BAR_ID);
    let id = ready_unit(&mut g, 0, 71);
    g.players[0].units[0].lane = Some(0);
    g.players[0].units[0].protected = true;
    g.battlefield_owner[0] = Some(0);
    g.apply(Action::Move {
        unit_id: id,
        lane: None,
    })
    .unwrap();
    let ability = g
        .chain
        .iter()
        .find(|item| {
            matches!(
                item.ability,
                Some(ChainAbility::Battlefield(
                    BattlefieldAbility::BackAlleyMight { .. }
                ))
            )
        })
        .unwrap();
    assert_eq!(ability.player, 0);
    assert!(ability.targets.is_empty());
    assert_eq!(
        g.chain_source(ability),
        ChainSource::Battlefield {
            lane: 0,
            battlefield_id: BACK_ALLEY_BAR_ID
        }
    );
    // A control change during the response window retains the physical object.
    let unit = g.players[0].units.remove(0);
    g.players[1].units.push(unit);
    settle(&mut g);
    assert_eq!(g.players[1].units[0].temporary_buff, 1);
}

#[test]
fn back_alley_does_not_buff_a_returned_new_object_or_trigger_on_zone_departure() {
    let mut g = board(0, BACK_ALLEY_BAR_ID);
    let id = ready_unit(&mut g, 0, 71);
    g.players[0].units[0].lane = Some(0);
    g.battlefield_owner[0] = Some(0);
    g.apply(Action::Move {
        unit_id: id,
        lane: None,
    })
    .unwrap();
    g.return_unit_to_hand(Target::Unit {
        player: 0,
        unit_id: id,
    });
    let replacement = ready_unit(&mut g, 0, 71);
    assert_ne!(id, replacement);
    settle(&mut g);
    assert_eq!(g.players[0].units[0].temporary_buff, 0);
    g.players[0].units[0].lane = Some(0);
    g.battlefield_owner[0] = Some(0);
    g.return_unit_to_hand(Target::Unit {
        player: 0,
        unit_id: replacement,
    });
    g.after_resolution().unwrap();
    assert!(g.chain.is_empty());
    assert!(g.pending_triggers.is_empty());
}

#[test]
fn back_alley_effect_moves_use_old_control_and_noop_moves_do_not_trigger() {
    for owner in [None, Some(0), Some(1)] {
        let mut g = board(0, BACK_ALLEY_BAR_ID);
        let id = ready_unit(&mut g, 1, 71);
        g.players[1].units[0].lane = Some(0);
        g.battlefield_owner[0] = owner;
        let target = Target::Unit {
            player: 1,
            unit_id: id,
        };
        g.move_effect(0, target, Some(0), false).unwrap();
        assert!(g.pending_triggers.is_empty());
        g.move_effect(0, target, None, false).unwrap();
        g.after_resolution().unwrap();
        let ability = g
            .chain
            .iter()
            .find(|item| {
                matches!(
                    item.ability,
                    Some(ChainAbility::Battlefield(
                        BattlefieldAbility::BackAlleyMight { .. }
                    ))
                )
            })
            .unwrap();
        assert_eq!(ability.player, owner.unwrap_or(0));
        settle(&mut g);
        assert_eq!(g.players[1].units[0].temporary_buff, 1);
    }
}

#[test]
fn back_alley_repeated_moves_accumulate_but_corrective_recall_does_not_trigger() {
    let mut g = board(0, BACK_ALLEY_BAR_ID);
    let id = ready_unit(&mut g, 0, 71);
    let target = Target::Unit {
        player: 0,
        unit_id: id,
    };
    g.players[0].units[0].lane = Some(0);
    g.battlefield_owner[0] = Some(0);
    // CR457-458: relocating to Base by Recall is not a Move.
    g.recall_unit(0, id, false, false);
    g.after_resolution().unwrap();
    assert!(g.chain.is_empty());
    assert!(g.pending_triggers.is_empty());
    assert_eq!(g.players[0].units[0].temporary_buff, 0);
    for expected in 1..=2 {
        g.move_effect(0, target, Some(0), false).unwrap();
        g.after_resolution().unwrap();
        settle(&mut g);
        g.move_effect(0, target, None, false).unwrap();
        g.after_resolution().unwrap();
        settle(&mut g);
        assert_eq!(g.players[0].units[0].temporary_buff, expected);
    }
}

#[test]
fn sanctum_can_keep_reorder_recycle_one_or_both_and_rejects_invalid_partitions() {
    for actor in 0..2 {
        for (recycle, keep, expected) in [
            (vec![], vec![0, 1], vec![71, 70, 69, 68]),
            (vec![], vec![1, 0], vec![71, 70, 68, 69]),
            (vec![0], vec![1], vec![68, 71, 70, 69]),
            (vec![1], vec![0], vec![69, 71, 70, 68]),
        ] {
            let mut g = board(actor, CANDLELIT_SANCTUM_ID);
            g.players[actor].deck = vec![71, 70, 69, 68];
            conquer(&mut g, actor, 71);
            let (id, cards) = look(&g);
            assert_eq!(cards, vec![68, 69]);
            assert!(g.players[actor].hand.is_empty());
            reject_unchanged(&mut g, Action::Pass);
            for (choice_id, recycle, top_order) in [
                (id + 1, vec![], vec![0, 1]),
                (id, vec![0], vec![0]),
                (id, vec![], vec![0]),
                (id, vec![], vec![0, 2]),
            ] {
                reject_unchanged(
                    &mut g,
                    Action::ResolvePredict {
                        choice_id,
                        recycle,
                        top_order,
                    },
                );
            }
            g.apply(Action::ResolvePredict {
                choice_id: id,
                recycle,
                top_order: keep,
            })
            .unwrap();
            settle(&mut g);
            assert_eq!(g.players[actor].deck, expected);
            assert!(g.predict_continuation().is_none());
        }
        let mut g = board(actor, CANDLELIT_SANCTUM_ID);
        g.players[actor].deck = vec![71, 70, 69, 68];
        conquer(&mut g, actor, 71);
        let (choice_id, _) = look(&g);
        g.apply(Action::ResolvePredict {
            choice_id,
            recycle: vec![0, 1],
            top_order: vec![],
        })
        .unwrap();
        assert_eq!(&g.players[actor].deck[2..], &[71, 70]);
        let mut bottom = g.players[actor].deck[..2].to_vec();
        bottom.sort();
        assert_eq!(bottom, vec![68, 69]);
    }
}

#[test]
fn sanctum_empty_short_and_duplicate_decks_do_not_draw_or_burn_out() {
    for deck in [vec![], vec![71], vec![71, 71]] {
        let mut g = board(0, CANDLELIT_SANCTUM_ID);
        g.players[0].deck = deck.clone();
        conquer(&mut g, 0, 71);
        if !deck.is_empty() {
            let (choice_id, cards) = look(&g);
            assert_eq!(cards.len(), deck.len());
            g.apply(Action::ResolvePredict {
                choice_id,
                recycle: vec![0],
                top_order: (1..deck.len()).collect(),
            })
            .unwrap();
        }
        assert_eq!(g.players[0].deck, deck);
        assert_eq!(g.players[1].score, 0);
        assert!(g.players[0].hand.is_empty());
    }
}

#[test]
fn sanctum_conquer_opens_responses_before_looking_and_hold_does_not_look() {
    let mut g = board(0, CANDLELIT_SANCTUM_ID);
    let id = ready_unit(&mut g, 0, 71);
    g.apply(Action::Move {
        unit_id: id,
        lane: Some(0),
    })
    .unwrap();
    for _ in 0..40 {
        if g.chain.iter().any(|i| {
            matches!(
                i.ability,
                Some(ChainAbility::Battlefield(
                    BattlefieldAbility::SanctumArrange { .. }
                ))
            )
        }) {
            break;
        }
        g.apply(test_support::decline_legend_action(&g).unwrap_or(Action::Pass))
            .unwrap();
    }
    assert!(!g.chain.is_empty());
    assert!(!matches!(g.decision, Some(Decision::PredictCards { .. })));
    settle(&mut g);
    let (choice_id, _) = look(&g);
    g.apply(Action::ResolvePredict {
        choice_id,
        recycle: vec![],
        top_order: vec![0, 1],
    })
    .unwrap();
    settle(&mut g);
    g.apply(Action::EndTurn).unwrap();
    settle(&mut g);
    g.apply(Action::EndTurn).unwrap();
    settle(&mut g);
    assert_eq!(g.players[0].score, 2);
    assert!(g.decision.is_none());
    assert_eq!(g.players[0].hand.len(), 1);
}
