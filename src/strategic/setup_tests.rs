//! Behavioral setup decisions, separate from complete-game coverage counts.
use super::*;

const WHEEL: CardId = 454;
const DISCIPLINE: CardId = 7;

fn position(actor: usize, energy: u16, hand: &[CardId]) -> Game {
    let mut game = test_support::position(actor, 0, 454200);
    game.difficulty = Difficulty::Strategic;
    game.players[actor].hand = hand.to_vec();
    game.players[actor].energy = energy;
    game.players[actor].energy_pool = energy;
    // Authored exhausted unit; all subsequent transitions use public actions.
    game.create_unit(actor, 71, false, 0);
    game
}

#[test]
fn draws_from_an_affordable_wheel_before_resolving_the_choosing_spell() {
    for actor in 0..2 {
        for energy in [5, 8] {
            let mut game = position(actor, energy, &[WHEEL, DISCIPLINE]);
            let action = game.choose_strategic_action().unwrap();
            game.apply(action).unwrap();
            assert_eq!(game.players[actor].gear.len(), 1);
            assert_eq!(game.players[actor].gear[0].card_id, WHEEL);
            assert_eq!(game.players[actor].hand, [DISCIPLINE]);
            let action = game.choose_strategic_action().unwrap();
            game.apply(action).unwrap();
            let deck = game.players[actor].deck.len();
            let turn = game.turn;
            for _ in 0..20 {
                if game.decision.is_none() && game.chain.is_empty() {
                    break;
                }
                let action = game.choose_strategic_action().unwrap();
                game.apply(action).unwrap();
                assert_eq!(game.turn, turn);
            }
            assert!(game.decision.is_none() && game.chain.is_empty());
            assert!(!game.players[actor].gear[0].ready);
            assert_eq!(game.players[actor].energy, energy - 5);
            assert_eq!(game.players[actor].deck.len(), deck - 2);
            assert_eq!(game.players[actor].hand.len(), 2);
        }
    }
}

#[test]
fn saves_the_setup_when_short_of_energy_or_a_useful_payoff() {
    for actor in 0..2 {
        // Three Energy cannot pay Wheel + Discipline + Wheel's draw cost.
        // En Garde replaces itself through Wheel but its temporary buff here
        // has no battle or scoring use, so spending it is not a payoff.
        for (energy, hand) in [(3, vec![WHEEL, DISCIPLINE]), (5, vec![WHEEL, 8])] {
            let game = position(actor, energy, &hand);
            assert_eq!(game.choose_strategic_action().unwrap(), Action::EndTurn);
        }
    }
}

#[test]
fn takes_a_winning_conquest_before_setting_up_draws() {
    for actor in 0..2 {
        let mut game = position(actor, 5, &[WHEEL, DISCIPLINE]);
        game.players[actor].score = 7;
        // The other battlefield has already scored this turn, so conquest of
        // the remaining field can legally supply the final victory point.
        game.player_scored_this_turn[actor][1] = true;
        game.scored_this_turn[1] = true;
        game.players[actor].units[0].ready = true;
        let action = game.choose_strategic_action().unwrap();
        assert!(matches!(
            action,
            Action::Move { .. } | Action::MoveGroup { .. }
        ));
        game.apply(action).unwrap();
        game.strategic_settle(game.turn).unwrap();
        assert_eq!(game.winner, Some(actor));
    }
}

#[test]
fn setup_decisions_do_not_use_secret_cards_or_future_draw_order() {
    for actor in 0..2 {
        let game = position(actor, 5, &[WHEEL, DISCIPLINE]);
        let mut other = game.clone();
        other.players[1 - actor].hand = vec![6, 6, 6];
        let mut game = game;
        game.players[1 - actor].hand = vec![8, 8, 8];
        other.players[0].deck.fill(6);
        other.players[1].deck.fill(8);
        other.rng = Rng::new(9999);
        assert_eq!(
            game.choose_strategic_action().unwrap(),
            other.choose_strategic_action().unwrap()
        );
    }
}

#[test]
fn equivalent_targetless_play_encodings_have_identical_engine_results() {
    for actor in 0..2 {
        for card in [WHEEL, 71, 450, 451] {
            let game = position(actor, 20, &[card]);
            let plain = Action::PlayCard {
                hand_index: 0,
                lane: None,
                target: None,
            };
            let explicit = Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            };
            let mut expected = game.clone();
            expected.apply(plain.clone()).unwrap();
            for action in [
                explicit.clone(),
                Action::WithPlayChoices {
                    action: Box::new(plain),
                    choices: PlayChoices::default(),
                },
                Action::WithPlayChoices {
                    action: Box::new(explicit),
                    choices: PlayChoices::default(),
                },
            ] {
                let mut actual = game.clone();
                actual.apply(action).unwrap();
                assert_eq!(
                    format!("{actual:?}"),
                    format!("{expected:?}"),
                    "card {card}, actor {actor}"
                );
            }
            let candidates = game.strategic_candidates(ROOT_LIMIT).unwrap();
            let mut seen = 0;
            for action in candidates {
                let mut after = game.clone();
                after.apply(action).unwrap();
                if format!("{after:?}") == format!("{expected:?}") {
                    seen += 1;
                }
            }
            assert_eq!(
                seen, 1,
                "equivalent plays must not crowd out different searches"
            );
        }
    }
}

#[test]
fn distinct_optional_costs_and_explicit_target_choices_remain_distinct() {
    let game = position(0, 20, &[451, 7]);
    let base = Action::PlayCard {
        hand_index: 0,
        lane: None,
        target: None,
    };
    let paid = Action::WithPlayChoices {
        action: Box::new(base.clone()),
        choices: PlayChoices {
            bonus: true,
            ..Default::default()
        },
    };
    for action in [&base, &paid] {
        game.clone().apply(action.clone()).unwrap();
    }
    assert_ne!(
        game.strategic_action_key(&base),
        game.strategic_action_key(&paid)
    );
    let automatic = Action::PlayCard {
        hand_index: 1,
        lane: None,
        target: None,
    };
    let empty = Action::PlayCardWithTargets {
        hand_index: 1,
        lane: None,
        targets: vec![],
    };
    assert_ne!(
        game.strategic_action_key(&automatic),
        game.strategic_action_key(&empty)
    );
    let mut game = game;
    game.players[0].hand[1] = 9; // Flash can choose zero or up to two units.
    game.players[0].units[0].lane = Some(0);
    assert_ne!(
        game.strategic_action_key(&automatic),
        game.strategic_action_key(&empty)
    );
    let unit = Target::Unit {
        player: 0,
        unit_id: game.players[0].units[0].id,
    };
    let chosen = Action::PlayCardWithTargets {
        hand_index: 1,
        lane: None,
        targets: vec![unit],
    };
    for action in [&empty, &chosen] {
        game.clone().apply(action.clone()).unwrap();
    }
    assert_ne!(
        game.strategic_action_key(&empty),
        game.strategic_action_key(&chosen)
    );
}

#[test]
fn targetless_catalog_plays_preserve_legality_and_full_state_across_encodings() {
    let mut checked = 0;
    for actor in 0..2 {
        let mut game = position(actor, 100, &[]);
        game.players[actor].power_pool = [50; 6];
        for card in 0..game.catalog.len() {
            if rules::target_bounds(card) != (0, 0) {
                continue;
            }
            game.players[actor].hand = vec![card];
            let plain = Action::PlayCard {
                hand_index: 0,
                lane: None,
                target: None,
            };
            let explicit = Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            };
            let mut expected = game.clone();
            let legal = expected.apply(plain.clone()).is_ok();
            for action in [
                explicit.clone(),
                Action::WithPlayChoices {
                    action: Box::new(plain),
                    choices: PlayChoices::default(),
                },
                Action::WithPlayChoices {
                    action: Box::new(explicit),
                    choices: PlayChoices::default(),
                },
            ] {
                let mut actual = game.clone();
                assert_eq!(
                    actual.apply(action).is_ok(),
                    legal,
                    "card {card}, actor {actor}"
                );
                if legal {
                    assert_eq!(
                        format!("{actual:?}"),
                        format!("{expected:?}"),
                        "card {card}, actor {actor}"
                    );
                }
            }
            if legal {
                checked += 1;
            }
        }
    }
    assert!(
        checked > 300,
        "expected broad catalog exercise, got {checked}"
    );
    eprintln!(
        "Equivalent targetless play checks: {checked} legal card/seat positions, three alternative encodings each"
    );
}
