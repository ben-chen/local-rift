//! Complete legal constructed games exercise the Empower/Disempower family and conservation.
mod support;
use support::conserved;

use local_rift::deck::{CustomDeck, eligible_cards};
use local_rift::game::{ChainAbility, Difficulty, Game, Phase};
use local_rift::legend_catalog::legends;
use std::collections::BTreeSet;

fn expanded(legend: usize) -> CustomDeck {
    let mut deck = CustomDeck::from_legend(legend).unwrap();
    deck.name = format!("Empower verification {legend}");
    for id in eligible_cards(legend)
        .into_iter()
        .filter(|id| (246..254).contains(id))
    {
        if let Some((_, copies)) = deck.cards.iter_mut().find(|(existing, _)| *existing == id) {
            *copies = 3;
        } else {
            deck.cards.push((id, 3));
        }
    }
    deck.validate().unwrap();
    deck
}

#[test]
fn empowered_cards_finish_all_legend_pairings_levels_and_starting_seats() {
    let mut seen_cards = BTreeSet::new();
    let mut seen_abilities = BTreeSet::new();
    let mut matches = 0;
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for starting in 0..2 {
            for first in 0..legends().len() {
                for second in 0..legends().len() {
                    let decks = [expanded(first), expanded(second)];
                    // Preserve the original eight-legend seeds as the matrix grows.
                    let seed = 38000 + (level * 1000 + starting * 100 + first * 8 + second) as u64;
                    let mut game = Game::new_with_custom(
                        [0, 3],
                        [Some(decks[0].clone()), Some(decks[1].clone())],
                        seed,
                        difficulty,
                        false,
                    )
                    .unwrap();
                    game.choose_starting_player(starting).unwrap();
                    game.start_match().unwrap();
                    for step in 0..6000 {
                        conserved(&game, &decks);
                        for player in &game.players {
                            seen_cards.extend(
                                player.units.iter().filter(|u| !u.token).map(|u| u.card_id),
                            );
                            seen_cards
                                .extend(player.gear.iter().filter(|g| !g.token).map(|g| g.card_id));
                        }
                        for item in &game.chain {
                            if let Some(ChainAbility::Activated {
                                card_id,
                                ability_index,
                                ..
                            }) = item.ability
                                && (246..254).contains(&card_id)
                            {
                                seen_abilities.insert((card_id, ability_index));
                            }
                        }
                        if game.winner.is_some() {
                            break;
                        }
                        assert!(game.step_ai().unwrap_or_else(|error| panic!(
                            "legends{first}/{second} {difficulty:?} starter{starting} seed{seed} step{step}: {error}; recent {:?}",
                            game.log.iter().rev().take(8).collect::<Vec<_>>()
                        )), "AI stopped in an unfinished match");
                    }
                    assert_eq!(
                        game.phase,
                        Phase::Finished,
                        "legends{first}/{second} {difficulty:?} starter{starting} seed{seed} stalled"
                    );
                    let winner = game.winner.expect("finished game needs a winner");
                    assert!(game.players[winner].score >= 8);
                    conserved(&game, &decks);
                    matches += 1;
                }
            }
        }
    }
    assert_eq!(matches, legends().len() * legends().len() * 6);
    assert!(
        (246..254).all(|id| seen_cards.contains(&id)),
        "not all new cards entered play: {seen_cards:?}"
    );
    assert!(
        seen_abilities.len() >= 10,
        "too few distinct new abilities exercised: {seen_abilities:?}"
    );
    println!(
        "PASS: {matches} complete Empower/Disempower games; respondable abilities observed {seen_abilities:?}"
    );
}
