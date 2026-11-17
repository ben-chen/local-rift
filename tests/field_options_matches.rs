//! Authored Irelia variants exercise new options without changing sourced presets.
use local_rift::{
    battlefields::SANDSWEPT_TOMB_ID,
    deck::CustomDeck,
    game::{Action, ChainAbility, Difficulty, Game},
};
use std::collections::BTreeSet;
mod support;

fn deck() -> CustomDeck {
    let mut deck = CustomDeck::from_preset(0).unwrap();
    deck.name = "Irelia / field options integration".into();
    deck.battlefield = SANDSWEPT_TOMB_ID;
    for (old, new) in [(5, 470), (6, 471), (17, 472)] {
        let row = deck.cards.iter_mut().find(|row| row.0 == old).unwrap();
        assert_eq!(row.1, 3);
        row.0 = new;
    }
    deck
}

#[test]
fn field_options_finish_fifty_four_games_with_conservation_at_every_level() {
    let mut entered = [[[0usize; 3]; 2]; 3];
    let mut natural_entries = [[[0usize; 3]; 2]; 3];
    let mut activations = [[0usize; 2]; 3];
    let mut open_entries = [[0usize; 2]; 3];
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for starter in 0..2 {
            for sample in 0..9 {
                let specs = [deck(), deck()];
                let seed = 472200 + level as u64 * 100 + starter as u64 * 10 + sample;
                let mut g =
                    Game::new_with_custom([0, 0], specs.clone().map(Some), seed, difficulty, false)
                        .unwrap();
                g.choose_starting_player(starter).unwrap();
                g.start_match().unwrap();
                if sample == 8 {
                    // Six reachable, quiet openings complement 48 unassisted games.
                    // Both players legally end turns until the starting player can
                    // play Rostrum; AI handles its choices and the rest of the game.
                    // No hand, deck order, resource or board state is injected.
                    for _ in 0..80 {
                        support::conserved(&g, &specs);
                        if g.priority() == starter
                            && let Some(hand_index) =
                                g.players[starter].hand.iter().position(|id| *id == 472)
                        {
                            let action = Action::PlayCard {
                                hand_index,
                                lane: None,
                                target: None,
                            };
                            if g.clone().apply(action.clone()).is_ok() {
                                g.apply(action).unwrap();
                                break;
                            }
                        }
                        g.apply(Action::EndTurn).unwrap();
                    }
                    assert!(
                        g.players[starter]
                            .gear
                            .iter()
                            .any(|gear| gear.card_id == 472),
                        "quiet opening never reached Rostrum"
                    );
                }
                let mut seen = BTreeSet::new();
                let mut used = BTreeSet::new();
                for step in 0..4000 {
                    support::conserved(&g, &specs);
                    for (seat, state) in g.players.iter().enumerate() {
                        for u in &state.units {
                            if !u.token && [470, 471].contains(&u.card_id) && seen.insert(u.id) {
                                entered[level][seat][u.card_id - 470] += 1;
                                if sample < 8 {
                                    natural_entries[level][seat][u.card_id - 470] += 1;
                                }
                                if u.lane.is_some() {
                                    open_entries[level][seat] += 1;
                                }
                            }
                        }
                        for gear in &state.gear {
                            if !gear.token && gear.card_id == 472 && seen.insert(gear.id) {
                                entered[level][seat][2] += 1;
                                if sample < 8 {
                                    natural_entries[level][seat][2] += 1;
                                }
                            }
                        }
                    }
                    for item in &g.chain {
                        if matches!(
                            item.ability,
                            Some(ChainAbility::Activated { card_id: 472, .. })
                        ) && used.insert(item.id)
                        {
                            activations[level][item.player] += 1;
                        }
                    }
                    if g.winner.is_some() {
                        break;
                    }
                    assert!(g.step_ai().unwrap_or_else(|e| panic!(
                        "{difficulty:?} start{starter} seed{seed} step{step}: {e}"
                    )));
                    g.validate_numeric_state().unwrap();
                }
                assert!(
                    g.winner.is_some(),
                    "{difficulty:?} start{starter} seed{seed}"
                );
            }
        }
    }
    eprintln!(
        "New card entries by level/seat/card: {entered:?}; Rostrum activations: {activations:?}; field entries: {open_entries:?}"
    );
    eprintln!(
        "48 unassisted games, natural entries: {natural_entries:?}; 6 scripted legal openings then AI play."
    );
    for (level, seats) in entered.iter().enumerate() {
        for card in 0..3 {
            assert!(
                seats.iter().map(|counts| counts[card]).sum::<usize>() > 0,
                "level{level} card{card}"
            );
        }
        assert!(
            activations[level].iter().sum::<usize>() > 0,
            "Rostrum unused at level{level}"
        );
        assert!(
            open_entries[level].iter().sum::<usize>() > 0,
            "no field entry at level{level}"
        );
    }
}
