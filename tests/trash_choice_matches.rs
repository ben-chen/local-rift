//! Unassisted games with legal authored Irelia variants; sourced presets stay intact.
use local_rift::{
    deck::CustomDeck,
    game::{ChainAbility, Difficulty, Game},
};
use std::collections::BTreeSet;
mod support;

fn deck(family: usize) -> CustomDeck {
    let mut deck = CustomDeck::from_preset(0).unwrap();
    deck.name = "Irelia / trash recovery integration".into();
    // Preserve coverage of each recovery family, then exercise their combination.
    // The combined list can win before some optional recovery opportunities occur.
    // A fourth legal variant uses Seal of Discord to test generated-Power funding.
    let replacements: &[(usize, usize)] = match family {
        0 => &[(5, 476), (6, 477)],
        1 => &[(7, 478), (8, 479)],
        2 => &[(5, 476), (6, 477), (7, 478), (8, 479)],
        _ => &[(5, 476), (6, 477), (7, 478), (8, 479), (15, 42)],
    };
    for &(old, new) in replacements {
        let row = deck.cards.iter_mut().find(|row| row.0 == old).unwrap();
        assert_eq!(row.1, if old == 8 { 2 } else { 3 });
        row.0 = new;
    }
    deck
}

#[test]
fn trash_recovery_families_finish_384_games_and_conserve_every_card() {
    let mut entries = [[[0usize; 4]; 2]; 3];
    let mut choices = [[[0usize; 4]; 2]; 3];
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for starter in 0..2 {
            for family in 0..4 {
                for sample in 0..16 {
                    let specs = [deck(family), deck(family)];
                    let seed = 477200 + level as u64 * 100 + starter as u64 * 10 + sample;
                    let mut g = Game::new_with_custom(
                        [0, 0],
                        specs.clone().map(Some),
                        seed,
                        difficulty,
                        false,
                    )
                    .unwrap();
                    g.choose_starting_player(starter).unwrap();
                    g.start_match().unwrap();
                    let mut seen = BTreeSet::new();
                    let mut used = BTreeSet::new();
                    let mut spells = BTreeSet::new();
                    for step in 0..4000 {
                        support::conserved(&g, &specs);
                        for (seat, state) in g.players.iter().enumerate() {
                            for unit in &state.units {
                                if !unit.token
                                    && (476..=477).contains(&unit.card_id)
                                    && seen.insert(unit.id)
                                {
                                    entries[level][seat][unit.card_id - 476] += 1;
                                }
                            }
                        }
                        for item in &g.chain {
                            if (478..=479).contains(&item.card_id)
                                && item.ability.is_none()
                                && spells.insert(item.id)
                            {
                                entries[level][item.player][item.card_id - 476] += 1;
                                if !item.targets.is_empty() {
                                    choices[level][item.player][item.card_id - 476] += 1;
                                }
                            }
                            if (476..=477).contains(&item.card_id)
                                && matches!(
                                    item.ability,
                                    Some(
                                        ChainAbility::ReturnChosen { .. }
                                            | ChainAbility::PlayChosen { .. }
                                    )
                                )
                                && used.insert(item.id)
                            {
                                choices[level][item.player][item.card_id - 476] += 1;
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
    }
    eprintln!(
        "Trash unit entries/spell plays by difficulty/seat/card: {entries:?}; finalized trash targets: {choices:?}"
    );
    for (level, seats) in entries.iter().enumerate() {
        for (seat, cards) in seats.iter().enumerate() {
            for (card, count) in cards.iter().enumerate() {
                assert!(
                    *count > 0,
                    "no {} entry at level{level} seat{seat}",
                    card + 476
                );
                assert!(
                    choices[level][seat][card] > 0,
                    "no finalized target for {} at level{level} seat{seat}",
                    card + 476
                );
            }
        }
    }
}
