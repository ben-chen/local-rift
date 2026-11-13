//! Controlled numeric-boundary positions; actual resource activations follow.
//! These assert host-capacity errors, not printed resource limits in Riftbound.
use local_rift::game::{Action, Difficulty, Game};
use local_rift::gear::GearState;
use local_rift::rules;

fn position(player: usize, card_id: usize, gold: bool, balance: u16) -> Game {
    let mut game = Game::new_for_agents(3, 3, 65535, Difficulty::Easy);
    game.choose_starting_player(player).unwrap();
    game.start_match().unwrap();
    for state in &mut game.players {
        state.units.clear();
        state.gear.clear();
        state.set_rune_setup([0; 6], [0; 6]).unwrap();
        state.energy = 0;
        state.energy_pool = 0;
        state.any_power = 0;
        state.power_pool = [0; 6];
    }
    let state = &mut game.players[player];
    if gold {
        state.any_power = balance;
    } else {
        let output = rules::resource_gear(card_id).unwrap();
        if output.energy > 0 {
            state.energy = balance;
            state.energy_pool = balance;
        } else {
            state.power_pool[local_rift::game::domain_index(output.domain.unwrap())] = balance;
        }
    }
    state.gear.push(GearState {
        id: 65000,
        card_id,
        lane: None,
        attached_to: None,
        ready: true,
        temporary: false,
        token: gold,
    });
    game.validate_numeric_state().unwrap();
    game
}

#[test]
fn gold_and_every_resource_gear_reject_overflow_without_consuming_the_source() {
    let mut failed = Vec::new();
    for player in 0..2 {
        for (card, gold) in [
            (0, true),
            (42, false),
            (125, false),
            (126, false),
            (127, false),
            (128, false),
            (129, false),
            (130, false),
        ] {
            let mut game = position(player, card, gold, u16::MAX);
            let before = format!("{game:?}");
            match game.apply(Action::ActivateGear { gear_id: 65000 }) {
                Err(error) if error.starts_with("engine_capacity:") => {
                    assert_eq!(format!("{game:?}"), before);
                }
                result => failed.push(format!(
                    "player{player}, card{card}, gold={gold}: {result:?}"
                )),
            }
        }
    }
    assert!(
        failed.is_empty(),
        "Overflow was clipped or misclassified: {failed:?}"
    );
}

#[test]
fn adding_the_last_representable_resource_is_exact_and_still_spends_the_source() {
    for player in 0..2 {
        for (card, gold) in [
            (0, true),
            (42, false),
            (125, false),
            (126, false),
            (127, false),
            (128, false),
            (129, false),
            (130, false),
        ] {
            let mut game = position(player, card, gold, u16::MAX - 1);
            game.apply(Action::ActivateGear { gear_id: 65000 }).unwrap();
            let state = &game.players[player];
            if gold {
                assert_eq!(state.any_power, u16::MAX);
                assert!(state.gear.is_empty());
            } else {
                assert!(!state.gear[0].ready);
                let output = rules::resource_gear(card).unwrap();
                if output.energy > 0 {
                    assert_eq!(state.energy, u16::MAX);
                    assert_eq!(state.energy_pool, u16::MAX);
                } else {
                    assert_eq!(
                        state.power_pool[local_rift::game::domain_index(output.domain.unwrap())],
                        u16::MAX
                    );
                }
            }
        }
    }
}

#[test]
fn available_energy_and_activation_history_are_checked_before_exhaust_or_kill() {
    for player in 0..2 {
        let mut game = position(player, 130, false, u16::MAX - 1);
        game.players[player]
            .set_rune_setup([1, 0, 0, 0, 0, 0], [0; 6])
            .unwrap();
        game.players[player].energy_pool = u16::MAX - 1;
        game.players[player].energy = u16::MAX;
        game.validate_numeric_state().unwrap();
        let before = format!("{game:?}");
        let error = game
            .apply(Action::ActivateGear { gear_id: 65000 })
            .unwrap_err();
        assert!(error.starts_with("engine_capacity:"), "{error}");
        assert_eq!(format!("{game:?}"), before);

        let mut game = position(player, 0, true, 0);
        game.play_history.gear_activations[player] = u16::MAX;
        let before = format!("{game:?}");
        let error = game
            .apply(Action::ActivateGear { gear_id: 65000 })
            .unwrap_err();
        assert!(error.starts_with("engine_capacity:"), "{error}");
        assert_eq!(format!("{game:?}"), before);
    }
}
