use super::*;
use crate::game::test_support::{decline_legend_action, reject_unchanged};

const ENTOURAGE: CardId = 455;

fn position(actor: usize) -> Game {
    let mut game = test_support::position(actor, 0, 455001);
    game.players[actor].hand = vec![ENTOURAGE];
    game
}

fn play(game: &mut Game) {
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    })
    .unwrap();
    assert!(matches!(
        game.decision,
        Some(Decision::TriggerTargets {
            card_id: ENTOURAGE,
            ..
        })
    ));
    assert!(
        game.chain.is_empty(),
        "the unit enters before its play ability finalizes"
    );
}

fn target(game: &Game, owner: usize) -> Target {
    Target::Legend {
        player: owner,
        legend_id: game.legend[owner].kind.id(),
    }
}

fn finalize(game: &mut Game, owner: usize) {
    game.apply(Action::ChooseTriggerTargets {
        targets: vec![target(game, owner)],
    })
    .unwrap();
    assert!(game.decision.is_none());
    assert_eq!(
        game.chain.len(),
        1,
        "choosing a legend triggers neither Irelia nor Jae/Wheel"
    );
}

fn resolve_to_choice(game: &mut Game) -> u32 {
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    let Some(Decision::LegendReadiness { choice_id, .. }) = game.decision else {
        panic!("missing resolution choice: {:?}", game.decision)
    };
    assert_eq!(game.resolution_progress().unwrap().chain_id, choice_id);
    choice_id
}

#[test]
fn entourage_target_and_readiness_are_separate_for_both_seats_and_states() {
    for actor in 0..2 {
        for owner in 0..2 {
            for before in [false, true] {
                for ready in [false, true] {
                    let mut game = position(actor);
                    game.legend[owner].ready = before;
                    play(&mut game);
                    assert_eq!(
                        game.trigger_target_candidates().unwrap(),
                        vec![target(&game, 0), target(&game, 1)]
                    );
                    reject_unchanged(&mut game, Action::ChooseTriggerTargets { targets: vec![] });
                    reject_unchanged(&mut game, Action::Pass);
                    finalize(&mut game, owner);
                    assert_eq!(game.legend[owner].ready, before);
                    let choice_id = resolve_to_choice(&mut game);
                    assert_eq!(game.legend[owner].ready, before);
                    assert_eq!(game.legal_actions().unwrap().len(), 2);
                    game.apply(Action::ResolveReadiness { choice_id, ready })
                        .unwrap();
                    assert_eq!(game.legend[owner].ready, ready);
                    assert!(game.chain.is_empty());
                    assert!(game.decision.is_none());
                    assert!(game.resolution_progress().is_none());
                }
            }
        }
    }
}

#[test]
fn entourage_invalid_targets_and_resolution_actions_are_atomic() {
    let mut game = position(0);
    play(&mut game);
    for bad in [
        Target::Legend {
            player: 2,
            legend_id: 0,
        },
        Target::Legend {
            player: 0,
            legend_id: 3,
        },
        Target::Unit {
            player: 0,
            unit_id: game.players[0].units[0].id,
        },
    ] {
        reject_unchanged(
            &mut game,
            Action::ChooseTriggerTargets { targets: vec![bad] },
        );
    }
    reject_unchanged(
        &mut game,
        Action::ResolveReadiness {
            choice_id: 0,
            ready: true,
        },
    );
    finalize(&mut game, 0);
    let id = game.chain[0].id;
    reject_unchanged(
        &mut game,
        Action::ResolveReadiness {
            choice_id: id,
            ready: true,
        },
    );
    let choice_id = resolve_to_choice(&mut game);
    game.players[0].hand = vec![7];
    for action in [
        Action::Pass,
        Action::EndTurn,
        Action::Choose { accept: false },
        Action::ResolveReadiness {
            choice_id: choice_id + 1,
            ready: true,
        },
        Action::ChooseTriggerTargets {
            targets: vec![target(&game, 1)],
        },
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        },
        Action::ActivateLegend { target: None },
    ] {
        reject_unchanged(&mut game, action);
    }
    game.apply(Action::ResolveReadiness {
        choice_id,
        ready: true,
    })
    .unwrap();
    reject_unchanged(
        &mut game,
        Action::ResolveReadiness {
            choice_id,
            ready: false,
        },
    );
}

#[test]
fn entourage_opponent_can_spend_a_reaction_activation_before_resolution() {
    for actor in 0..2 {
        let owner = 1 - actor;
        let mut game = position(actor);
        game.legend[owner] = LegendState::from_legend(3).unwrap(); // Kai'Sa: Reaction Add spell power.
        play(&mut game);
        finalize(&mut game, owner);
        game.apply(Action::Pass).unwrap();
        game.apply(Action::ActivateLegend { target: None }).unwrap();
        assert!(!game.legend[owner].ready);
        assert_eq!(game.legend[owner].spell_power, 1);
        // Add is immediate and the responding player retains priority.
        assert_eq!(game.priority(), owner);
        game.apply(Action::Pass).unwrap();
        let Some(Decision::LegendReadiness { choice_id, .. }) = game.decision else {
            panic!("missing choice")
        };
        game.apply(Action::ResolveReadiness {
            choice_id,
            ready: false,
        })
        .unwrap();
        assert_eq!(
            game.legend[owner].spell_power, 1,
            "exhausting does not undo the paid activation"
        );
        assert!(!game.legend[owner].ready);
    }
}

#[test]
fn entourage_finalized_trigger_survives_its_units_retreat() {
    for actor in 0..2 {
        let mut game = position(actor);
        play(&mut game);
        let unit_id = game.players[actor].units[0].id;
        finalize(&mut game, actor);
        game.players[actor].hand = vec![377];
        game.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: actor,
                unit_id,
            }],
        })
        .unwrap();
        while let Some(action) = decline_legend_action(&game) {
            game.apply(action).unwrap();
        }
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert!(game.players[actor].units.is_empty());
        assert!(game.players[actor].hand.contains(&ENTOURAGE));
        let choice_id = resolve_to_choice(&mut game);
        game.apply(Action::ResolveReadiness {
            choice_id,
            ready: true,
        })
        .unwrap();
        assert!(game.legend[actor].ready);
    }
}

#[test]
fn entourage_ai_targets_useful_legends_and_resolves_without_private_information() {
    for actor in 0..2 {
        for own_ready in [false, true] {
            for difficulty in [
                Difficulty::Easy,
                Difficulty::Tactical,
                Difficulty::Strategic,
            ] {
                let mut game = position(actor);
                game.difficulty = difficulty;
                game.legend[actor].ready = own_ready;
                game.legend[1 - actor].ready = true;
                play(&mut game);
                let action = game.choose_ai_action().unwrap();
                assert!(game.clone().apply(action.clone()).is_ok());
                if difficulty != Difficulty::Easy {
                    let owner = if own_ready { 1 - actor } else { actor };
                    assert_eq!(
                        action,
                        Action::ChooseTriggerTargets {
                            targets: vec![target(&game, owner)]
                        }
                    );
                }
                game.apply(action).unwrap();
                let choice_id = resolve_to_choice(&mut game);
                let action = game.choose_ai_action().unwrap();
                assert!(
                    matches!(action, Action::ResolveReadiness { choice_id: id, .. } if id == choice_id)
                );
                let mut hidden = game.clone();
                hidden.players[1 - actor].hand = vec![8, 7];
                hidden.players[1 - actor].deck.reverse();
                if difficulty != Difficulty::Easy {
                    assert_eq!(hidden.choose_ai_action().unwrap(), action);
                }
                game.apply(action).unwrap();
                assert!(game.decision.is_none());
            }
        }
    }
}
