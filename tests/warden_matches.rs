//! Authored legal Irelia lists; card exposure is separate from decision quality.
mod support;
use local_rift::{
    deck::CustomDeck,
    game::{Difficulty, Game, Phase},
};

fn deck(legend: usize) -> CustomDeck {
    let mut deck = CustomDeck::from_legend(legend).unwrap();
    if legend == 0 {
        deck.cards.push((464, 3));
        let mut excess = deck
            .cards
            .iter()
            .map(|(_, n)| usize::from(*n))
            .sum::<usize>()
            - 40;
        for (id, count) in &mut deck.cards {
            if *id == 464 {
                continue;
            }
            let remove = usize::from(*count - u8::from(*id == deck.chosen_champion)).min(excess);
            *count -= u8::try_from(remove).unwrap();
            excess -= remove;
        }
        assert_eq!(excess, 0);
        deck.cards.retain(|(_, count)| *count > 0);
        deck.name = "Irelia / Warden verification".into();
    }
    deck.validate().unwrap();
    deck
}

#[test]
fn warden_complete_legal_games_in_both_seats_at_every_level() {
    let mut completed = 0;
    let mut seen = [false; 2];
    let mut active = [false; 2];
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for opponent in [1, 3] {
            for seat in 0..2 {
                for first in 0..2 {
                    let mut legends = [0, opponent];
                    if seat == 1 {
                        legends.swap(0, 1);
                    }
                    let decks = legends.map(deck);
                    let seed = 464000 + (level * 1000 + opponent * 100 + seat * 10 + first) as u64;
                    let mut g = Game::new_with_custom(
                        [0, 3],
                        [Some(decks[0].clone()), Some(decks[1].clone())],
                        seed,
                        difficulty,
                        false,
                    )
                    .unwrap();
                    g.choose_starting_player(first).unwrap();
                    g.start_match().unwrap();
                    for step in 0..6000 {
                        support::conserved(&g, &decks);
                        for owner in 0..2 {
                            seen[owner] |= g.players[owner]
                                .units
                                .iter()
                                .any(|u| !u.token && u.card_id == 464);
                            active[owner] |= !g.restrictions_affecting(1 - owner).is_empty();
                        }
                        if g.winner.is_some() {
                            break;
                        }
                        assert!(
                            g.step_ai().unwrap_or_else(|error| panic!(
                                "seed {seed}, step {step}: {error}"
                            )),
                            "AI stopped at {seed}"
                        );
                    }
                    assert_eq!(g.phase, Phase::Finished, "seed {seed}");
                    assert!(g.players[g.winner.unwrap()].score >= 8);
                    support::conserved(&g, &decks);
                    completed += 1;
                }
            }
        }
    }
    assert_eq!(completed, 24);
    assert_eq!(seen, [true; 2]);
    assert_eq!(active, [true; 2]);
    println!("Warden completed {completed} games; played {seen:?}; restrictions active {active:?}");
}
