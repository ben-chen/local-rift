//! Authored expectations from CR420.4 and Riot's Vendetta FAQ, not another engine.
use super::*;
use crate::game::test_support::{position, reject_unchanged};

const SANDS: CardId = 456;
const DRAW: CardId = 133; // Downstage Dramatics: Reaction, Draw 1, optional Repeat.

fn cast(game: &mut Game, card: CardId, target: Option<u32>) -> u32 {
    let actor = game.priority();
    let hand_index = game.players[actor]
        .hand
        .iter()
        .position(|&id| id == card)
        .unwrap();
    game.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets: target
            .map(|chain_id| Target::Spell { chain_id })
            .into_iter()
            .collect(),
    })
    .unwrap();
    game.play_history.finalized_spells[actor]
        .last()
        .copied()
        .unwrap()
}

fn give_priority(game: &mut Game, actor: usize) {
    if game.priority() != actor {
        game.apply(Action::Pass).unwrap();
    }
    assert_eq!(game.priority(), actor);
}

fn resolve_top(game: &mut Game) {
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
}

#[test]
fn sands_can_target_first_or_second_spell_and_checks_condition_only_on_resolution() {
    for actor in 0..2 {
        for (count, target_index) in [(1, 0), (2, 0), (2, 1)] {
            let opponent = 1 - actor;
            let mut game = position(opponent, 0, 456001);
            game.players[opponent].hand = vec![DRAW; count];
            game.players[actor].hand = vec![SANDS];
            let ids: Vec<_> = (0..count).map(|_| cast(&mut game, DRAW, None)).collect();
            give_priority(&mut game, actor);
            let target = ids[target_index];
            assert_eq!(game.opponent_played_other_spell(actor, target), count == 2);
            let sand = cast(&mut game, SANDS, Some(target));
            assert_eq!(game.play_history.finalized_spells[actor], vec![sand]);
            assert_eq!(game.players[actor].energy, 19);
            assert_eq!(
                game.players[actor].power_pool[domain_index(Domain::Calm)],
                4
            );
            resolve_top(&mut game);
            assert_eq!(game.chain.iter().any(|item| item.id == target), count == 1);
            assert_eq!(game.players[opponent].discard.contains(&DRAW), count == 2);
            assert_eq!(game.play_history.finalized_spells[opponent], ids);
            assert!(game.players[actor].discard.contains(&SANDS));
        }
    }
}

#[test]
fn sands_counts_another_spell_even_after_it_was_countered() {
    for actor in 0..2 {
        let opponent = 1 - actor;
        let mut game = position(opponent, 0, 456002);
        game.players[opponent].hand = vec![DRAW, DRAW];
        game.players[actor].hand = vec![6, SANDS];
        let first = cast(&mut game, DRAW, None);
        let second = cast(&mut game, DRAW, None);
        give_priority(&mut game, actor);
        cast(&mut game, 6, Some(second));
        resolve_top(&mut game);
        assert_eq!(
            game.chain.iter().map(|i| i.id).collect::<Vec<_>>(),
            vec![first]
        );
        assert_eq!(
            game.players[opponent].hand.len(),
            0,
            "countered Draw did not resolve"
        );
        give_priority(&mut game, actor);
        cast(&mut game, SANDS, Some(first));
        resolve_top(&mut game);
        assert!(game.chain.is_empty());
        assert_eq!(game.players[opponent].discard, vec![DRAW, DRAW]);
        assert_eq!(
            game.play_history.finalized_spells[opponent],
            vec![first, second]
        );
    }
}

#[test]
fn sands_condition_can_become_true_after_it_is_played() {
    let mut game = position(1, 0, 456003);
    game.players[1].hand = vec![DRAW, DRAW];
    game.players[0].hand = vec![SANDS];
    let first = cast(&mut game, DRAW, None);
    give_priority(&mut game, 0);
    cast(&mut game, SANDS, Some(first));
    give_priority(&mut game, 1);
    let second = cast(&mut game, DRAW, None);
    resolve_top(&mut game);
    assert!(
        game.players[1].hand.contains(&71),
        "the responding Draw resolved"
    );
    assert!(game.opponent_played_other_spell(0, first));
    resolve_top(&mut game);
    assert!(game.chain.is_empty());
    assert_eq!(game.play_history.finalized_spells[1], vec![first, second]);
}

#[test]
fn sands_can_counter_own_spell_if_an_opponent_played_a_different_spell() {
    for actor in 0..2 {
        let opponent = 1 - actor;
        let mut game = position(actor, 0, 456004);
        game.players[actor].hand = vec![DRAW, SANDS];
        game.players[opponent].hand = vec![DRAW];
        let own = cast(&mut game, DRAW, None);
        give_priority(&mut game, opponent);
        let other = cast(&mut game, DRAW, None);
        give_priority(&mut game, actor);
        cast(&mut game, SANDS, Some(own));
        resolve_top(&mut game);
        assert_eq!(
            game.chain.iter().map(|i| i.id).collect::<Vec<_>>(),
            vec![other]
        );
        assert!(game.players[actor].discard.contains(&DRAW));
    }
}

#[test]
fn sands_repeat_resolves_twice_but_counts_as_one_spell_and_history_expires() {
    let mut game = position(0, 0, 456005);
    game.players[0].hand = vec![DRAW];
    game.apply(Action::PlayCardOptions {
        hand_index: 0,
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: true,
        accelerate: false,
        bonus: false,
    })
    .unwrap();
    let id = game.chain[0].id;
    assert_eq!(game.play_history.finalized_spells[0], vec![id]);
    assert!(!game.opponent_played_other_spell(1, id));
    resolve_top(&mut game);
    assert_eq!(game.players[0].hand, vec![71, 71]);
    assert_eq!(game.play_history.finalized_spells[0], vec![id]);
    game.apply(Action::EndTurn).unwrap();
    assert!(game.play_history.finalized_spells.iter().all(Vec::is_empty));
}

#[test]
fn sands_failed_and_staged_plays_do_not_enter_history() {
    let mut game = position(0, 0, 456006);
    game.players[0].hand = vec![SANDS, DRAW];
    reject_unchanged(
        &mut game,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Spell { chain_id: 9999 }],
        },
    );
    assert!(game.play_history.finalized_spells.iter().all(Vec::is_empty));
    game.apply(Action::BeginPayment {
        action: Box::new(Action::PlayCardWithTargets {
            hand_index: 1,
            lane: None,
            targets: vec![],
        }),
    })
    .unwrap();
    assert!(game.play_history.finalized_spells.iter().all(Vec::is_empty));
    assert!(game.chain.is_empty());
}

#[test]
fn sands_history_records_hidden_flow_and_replay_finalizations() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 456007);
        game.battlefield_owner[0] = Some(actor);
        let unit = crate::game::test_support::ready_unit(&mut game, actor, 71);
        game.players[actor]
            .units
            .iter_mut()
            .find(|u| u.id == unit)
            .unwrap()
            .lane = Some(0);
        // Authored prior-turn facedown position. Only the actual play counts.
        game.players[actor].hidden.push(HiddenCard {
            id: 900,
            card_id: 132,
            lane: 0,
            hidden_turn: 0,
            revealed: false,
        });
        assert!(game.play_history.finalized_spells[actor].is_empty());
        game.apply(Action::PlayHidden {
            hidden_id: 900,
            targets: vec![],
        })
        .unwrap();
        let hidden = game.chain[0].id;
        resolve_top(&mut game);
        assert_eq!(game.play_history.finalized_spells[actor], vec![hidden]);

        game.players[actor].hand.push(134); // Dredge Up; Flow from trash is a new play.
        let original = cast(&mut game, 134, None);
        resolve_top(&mut game);
        let trash_index = game.players[actor]
            .discard
            .iter()
            .position(|&id| id == 134)
            .unwrap();
        game.apply(Action::PlayFromTrashWithTargets {
            trash_index,
            lane: None,
            targets: vec![],
        })
        .unwrap();
        let flow = game.chain[0].id;
        assert_ne!(original, flow);
        resolve_top(&mut game);
        assert!(game.players[actor].banish.contains(&134));
        assert_eq!(
            game.play_history.finalized_spells[actor],
            vec![hidden, original, flow]
        );

        // A separate replay permission also finalizes, even though its payment differs.
        game.players[actor].discard.push(DRAW);
        game.decision = Some(Decision::ReplaySpell { player: actor });
        let trash_index = game.players[actor].discard.len() - 1;
        game.apply(Action::PlayFromTrashWithTargets {
            trash_index,
            lane: None,
            targets: vec![],
        })
        .unwrap();
        let replay = game.chain[0].id;
        assert_eq!(
            game.play_history.finalized_spells[actor],
            vec![hidden, original, flow, replay]
        );
        resolve_top(&mut game);
        assert_eq!(
            game.players[actor].deck[0], DRAW,
            "replay recycles the resolved card"
        );
    }
}

#[test]
fn sands_cannot_target_an_ability_and_units_do_not_count_as_spells() {
    let mut game = position(1, 0, 456008);
    game.players[1].hand = vec![455];
    game.players[0].hand = vec![SANDS];
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    })
    .unwrap();
    game.apply(Action::ChooseTriggerTargets {
        targets: vec![Target::Legend {
            player: 1,
            legend_id: game.legend[1].kind.id(),
        }],
    })
    .unwrap();
    let ability = game.chain[0].id;
    assert!(game.chain[0].ability.is_some());
    assert!(game.play_history.finalized_spells.iter().all(Vec::is_empty));
    give_priority(&mut game, 0);
    reject_unchanged(
        &mut game,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Spell { chain_id: ability }],
        },
    );
}

#[test]
fn sands_ai_preserves_resources_when_condition_is_false_without_hiding_legal_actions() {
    for actor in 0..2 {
        for count in [1, 2] {
            for difficulty in [Difficulty::Tactical, Difficulty::Strategic] {
                let opponent = 1 - actor;
                let mut game = position(opponent, 0, 456009);
                game.difficulty = difficulty;
                game.players[opponent].hand = vec![DRAW; count];
                game.players[actor].hand = vec![SANDS];
                let mut target = 0;
                for _ in 0..count {
                    target = cast(&mut game, DRAW, None);
                }
                give_priority(&mut game, actor);
                let action = Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![Target::Spell { chain_id: target }],
                };
                assert!(
                    game.clone().apply(action).is_ok(),
                    "condition is not a legality restriction"
                );
                let chosen = game.choose_ai_action().unwrap();
                let mut private_changed = game.clone();
                private_changed.players[opponent].hand = vec![7, 86];
                private_changed.players[opponent].deck.reverse();
                assert_eq!(private_changed.choose_ai_action().unwrap(), chosen);
                if count == 1 {
                    assert_eq!(chosen, Action::Pass);
                } else {
                    assert_eq!(game.ai_card_score(SANDS).unwrap(), 105);
                    // Spending a Calm rune to stop a draw is a strategic tradeoff;
                    // the Tactical heuristic takes it, search may preserve the rune.
                    if difficulty == Difficulty::Tactical {
                        assert_ne!(chosen, Action::Pass);
                    }
                }
                game.apply(chosen).unwrap();
            }
        }
    }
}
