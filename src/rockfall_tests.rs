//! Rockfall's prohibition applies to plays, regardless of their permission or cost.
use super::*;
use crate::battlefields::ROCKFALL_PATH_ID;
use test_support::{position, ready_unit, reject_unchanged};

fn board(actor: usize) -> Game {
    let mut game = position(actor, 0, 17001);
    game.battlefields = vec![ROCKFALL_PATH_ID, 3];
    game
}

fn hand(lane: Option<usize>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane,
        targets: vec![],
    }
}

fn finish(lane: Option<usize>) -> Action {
    Action::FinalizePendingPlay {
        lane,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate: false,
        bonus: false,
    }
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if game.chain.is_empty() && game.showdown.is_none() && game.decision.is_none() {
            return;
        }
        let action = test_support::decline_legend_action(game).unwrap_or(Action::Pass);
        game.apply(action).unwrap();
    }
    panic!("resolution did not finish: {:?}", game.decision);
}

fn pending(game: &mut Game) {
    for _ in 0..24 {
        if matches!(game.decision, Some(Decision::PendingPlay { .. })) {
            return;
        }
        let action = test_support::decline_legend_action(game).unwrap_or(Action::Pass);
        game.apply(action).unwrap();
    }
    panic!("the instructed play did not become pending");
}

#[test]
fn rockfall_blocks_hand_champion_and_ambush_plays_without_spending_resources() {
    for actor in 0..2 {
        for owner in [None, Some(actor), Some(1 - actor)] {
            for card in [71, 30] {
                let mut game = board(actor);
                game.battlefield_owner[0] = owner;
                let id = ready_unit(&mut game, 1 - actor, 71);
                game.unit_mut(Target::Unit {
                    player: 1 - actor,
                    unit_id: id,
                })
                .unwrap()
                .lane = Some(0);
                game.players[actor].hand = vec![card];
                reject_unchanged(&mut game, hand(Some(0)));
                assert!(!game.legal_actions().unwrap().contains(&hand(Some(0))));
                game.players[actor].champion = Some(0);
                reject_unchanged(
                    &mut game,
                    Action::PlayChampion {
                        lane: Some(0),
                        target: None,
                    },
                );
            }
        }
        let mut game = board(actor);
        game.battlefield_owner[1] = Some(actor);
        let unit_id = ready_unit(&mut game, actor, 71);
        game.unit_mut(Target::Unit {
            player: actor,
            unit_id,
        })
        .unwrap()
        .lane = Some(1);
        game.players[actor].hand = vec![71, 71];
        game.apply(hand(None)).unwrap();
        game.apply(hand(Some(1))).unwrap();
        assert_eq!(game.players[actor].units.len(), 3);
    }
}

#[test]
fn rockfall_hiding_is_allowed_but_hidden_unit_play_is_not() {
    for actor in 0..2 {
        let mut game = board(actor);
        game.battlefield_owner[0] = Some(actor);
        let id = ready_unit(&mut game, actor, 71);
        game.unit_mut(Target::Unit {
            player: actor,
            unit_id: id,
        })
        .unwrap()
        .lane = Some(0);
        game.players[actor].hand = vec![108];
        game.apply(Action::Hide {
            hand_index: 0,
            lane: 0,
        })
        .unwrap();
        let hidden = &mut game.players[actor].hidden[0];
        hidden.hidden_turn = 0; // Authored position on a later turn.
        let hidden_id = hidden.id;
        let action = Action::PlayHidden {
            hidden_id,
            targets: vec![],
        };
        reject_unchanged(&mut game, action.clone());
        assert!(!game.legal_actions().unwrap().contains(&action));
        // A Hidden spell at that same location is still playable.
        game.players[actor].hidden[0].card_id = 381;
        game.apply(Action::PlayHidden {
            hidden_id,
            targets: vec![Target::Unit {
                player: actor,
                unit_id: id,
            }],
        })
        .unwrap();
        assert_eq!(game.chain.last().unwrap().card_id, 381);
    }
}

#[test]
fn rockfall_movement_conquest_and_deploy_gear_remain_legal() {
    for actor in 0..2 {
        let mut game = board(actor);
        game.players[actor].hand = vec![401];
        game.apply(hand(Some(0))).unwrap();
        assert_eq!(game.players[actor].gear[0].lane, Some(0));
        assert_eq!(
            game.battlefield_owner[0], None,
            "gear does not control a field"
        );
        let unit_id = ready_unit(&mut game, actor, 71);
        game.apply(Action::Move {
            unit_id,
            lane: Some(0),
        })
        .unwrap();
        settle(&mut game);
        assert_eq!(game.battlefield_owner[0], Some(actor));
        assert_eq!(game.players[actor].score, 1);
        assert_eq!(game.players[actor].units[0].lane, Some(0));
    }
}

#[test]
fn rockfall_token_choices_exclude_path_and_fixed_token_plays_cease() {
    for actor in 0..2 {
        let mut game = board(actor);
        game.battlefield_owner = vec![Some(actor); 2];
        game.queue_token_play(actor, 49, 2, None, false);
        game.after_resolution().unwrap();
        reject_unchanged(&mut game, Action::PlaceToken { lane: Some(0) });
        let actions = game.legal_actions().unwrap();
        assert!(!actions.contains(&Action::PlaceToken { lane: Some(0) }));
        assert!(actions.contains(&Action::PlaceToken { lane: None }));
        assert!(actions.contains(&Action::PlaceToken { lane: Some(1) }));
        game.apply(Action::PlaceToken { lane: Some(1) }).unwrap();
        assert_eq!(game.players[actor].units.len(), 1);
        let history = game.play_history.clone();
        game.queue_token_play(actor, 49, 2, Some(Some(0)), false);
        game.after_resolution().unwrap();
        assert!(game.pending_tokens().is_empty() && game.decision.is_none());
        assert_eq!(
            game.players[actor].units.len(),
            1,
            "no fallback token at base"
        );
        assert_eq!(game.play_history, history, "failed play must not finalize");
        game.queue_token_play(actor, 49, 2, Some(None), false);
        game.after_resolution().unwrap();
        assert_eq!(game.players[actor].units.len(), 2);
    }
}

#[test]
fn rockfall_temporal_breach_banishes_but_cannot_replay_or_relocate() {
    for actor in 0..2 {
        for owner in 0..2 {
            let mut game = board(actor);
            let unit_id = ready_unit(&mut game, owner, 71);
            game.unit_mut(Target::Unit {
                player: owner,
                unit_id,
            })
            .unwrap()
            .lane = Some(0);
            game.battlefield_owner[0] = Some(owner);
            game.players[actor].hand = vec![381];
            game.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![Target::Unit {
                    player: owner,
                    unit_id,
                }],
            })
            .unwrap();
            pending(&mut game);
            assert!(matches!(game.decision, Some(Decision::PendingPlay { .. })));
            reject_unchanged(&mut game, finish(Some(0)));
            reject_unchanged(&mut game, finish(None));
            assert_eq!(
                game.legal_actions().unwrap(),
                vec![Action::Choose { accept: false }]
            );
            game.apply(Action::Choose { accept: false }).unwrap();
            assert_eq!(game.players[owner].banish, vec![71]);
            assert!(game.players[owner].units.is_empty());
            assert_eq!(game.players[actor].discard, vec![381]);
        }
    }
}

#[test]
fn rockfall_any_battlefield_instruction_preserves_other_legal_destinations() {
    for actor in 0..2 {
        let mut game = board(actor);
        let unit_id = ready_unit(&mut game, actor, 71);
        game.players[actor].hand = vec![382];
        game.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: actor,
                unit_id,
            }],
        })
        .unwrap();
        pending(&mut game);
        reject_unchanged(&mut game, finish(Some(0)));
        reject_unchanged(&mut game, finish(None));
        reject_unchanged(&mut game, Action::Choose { accept: false });
        game.apply(finish(Some(1))).unwrap();
        assert_eq!(game.players[actor].units[0].lane, Some(1));
        assert!(game.players[actor].banish.is_empty());
    }
}
