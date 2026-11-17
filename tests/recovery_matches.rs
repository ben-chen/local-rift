//! Unassisted games with legal authored Irelia variants; sourced presets stay intact.
use local_rift::{
    deck::CustomDeck,
    game::{ChainAbility, Difficulty, Game},
};
use std::collections::BTreeSet;
mod support;

fn deck() -> CustomDeck {
    let mut deck = CustomDeck::from_preset(0).unwrap();
    deck.name = "Irelia / recovery integration".into();
    for (old, new) in [(5, 473), (6, 474), (17, 475)] {
        let row = deck.cards.iter_mut().find(|row| row.0 == old).unwrap();
        assert_eq!(row.1, 3);
        row.0 = new;
    }
    deck
}

#[test]
fn recovery_options_finish_forty_eight_games_and_conserve_every_card() {
    let mut entries = [[[0usize; 3]; 2]; 3];
    let mut choices = [[[0usize; 2]; 2]; 3];
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for starter in 0..2 {
            for sample in 0..8 {
                let specs = [deck(), deck()];
                let seed = 475200 + level as u64 * 100 + starter as u64 * 10 + sample;
                let mut g =
                    Game::new_with_custom([0, 0], specs.clone().map(Some), seed, difficulty, false)
                        .unwrap();
                g.choose_starting_player(starter).unwrap();
                g.start_match().unwrap();
                let mut seen = BTreeSet::new();
                let mut used = BTreeSet::new();
                for step in 0..4000 {
                    support::conserved(&g, &specs);
                    for (seat, state) in g.players.iter().enumerate() {
                        for unit in &state.units {
                            if !unit.token
                                && (473..=475).contains(&unit.card_id)
                                && seen.insert(unit.id)
                            {
                                entries[level][seat][unit.card_id - 473] += 1;
                            }
                        }
                    }
                    for item in &g.chain {
                        if matches!(item.ability, Some(ChainAbility::ReturnChosen { .. }))
                            && used.insert(item.id)
                        {
                            choices[level][item.player][item.card_id - 474] += 1;
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
        "Recovery entries by difficulty/seat/card: {entries:?}; finalized recovery targets: {choices:?}"
    );
    for (level, seats) in entries.iter().enumerate() {
        for (seat, cards) in seats.iter().enumerate() {
            for (card, count) in cards.iter().enumerate() {
                assert!(
                    *count > 0,
                    "no {} entry at level{level} seat{seat}",
                    card + 473
                );
            }
        }
    }
    for source in 0..2 {
        assert!(
            choices
                .iter()
                .flat_map(|seats| seats.iter())
                .map(|counts| counts[source])
                .sum::<usize>()
                > 0,
            "no finalized recovery target for {}",
            source + 474
        );
    }
}
