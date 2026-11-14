//! Authored legal lists exercise Flurry; these are not tournament presets.
mod support;

use local_rift::deck::CustomDeck;
use local_rift::game::{Difficulty, Game, Phase};
use std::collections::BTreeSet;

fn deck(legend: usize) -> CustomDeck {
    let mut deck = CustomDeck::from_legend(legend).unwrap();
    if legend != 0 {
        return deck;
    }
    if let Some((_, count)) = deck.cards.iter_mut().find(|(card, _)| *card == 463) {
        *count = 3;
    } else {
        deck.cards.push((463, 3));
    }
    let mut excess = deck
        .cards
        .iter()
        .map(|(_, n)| usize::from(*n))
        .sum::<usize>()
        - 40;
    for (id, count) in &mut deck.cards {
        if *id == 463 {
            continue;
        }
        let minimum = u8::from(*id == deck.chosen_champion);
        let remove = usize::from(*count - minimum).min(excess);
        *count -= u8::try_from(remove).unwrap();
        excess -= remove;
    }
    assert_eq!(excess, 0);
    deck.cards.retain(|(_, count)| *count > 0);
    deck.name = "Irelia / Flurry verification".into();
    deck.validate().unwrap();
    deck
}

#[test]
fn flurry_complete_legal_games_both_seats_starters_and_all_ai_levels() {
    let mut casts = [[0usize; 2]; 2];
    let mut birds_seen = [false; 2];
    let mut completed = 0;
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
                    let seed = 463000 + (1000 * level + 100 * opponent + 10 * seat + first) as u64;
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
                    let mut chain_ids = BTreeSet::new();
                    for step in 0..6000 {
                        support::conserved(&g, &decks);
                        for owner in 0..2 {
                            birds_seen[owner] |= g.players[owner]
                                .units
                                .iter()
                                .any(|u| u.token && u.card_id == 463);
                        }
                        for item in &g.chain {
                            if chain_ids.insert(item.id)
                                && item.card_id == 463
                                && item.ability.is_none()
                            {
                                for execution in &item.executions {
                                    casts[item.player][execution.mode.unwrap()] += 1;
                                }
                            }
                        }
                        if g.winner.is_some() {
                            break;
                        }
                        assert!(
                            g.step_ai().unwrap_or_else(|error| panic!(
                                "seed {seed}, step {step}: {error}"
                            )),
                            "AI stopped: seed {seed}"
                        );
                    }
                    assert_eq!(g.phase, Phase::Finished, "seed {seed}, {difficulty:?}");
                    assert!(g.players[g.winner.unwrap()].score >= 8);
                    support::conserved(&g, &decks);
                    completed += 1;
                }
            }
        }
    }
    assert_eq!(completed, 24);
    assert!(
        casts.iter().all(|modes| modes.iter().sum::<usize>() > 0),
        "Flurry casts {casts:?}"
    );
    assert_eq!(birds_seen, [true; 2]);
    println!(
        "PASS: {completed} legal complete games; mode executions [counter, Birds] per seat {casts:?}; Birds observed {birds_seen:?}"
    );
}
