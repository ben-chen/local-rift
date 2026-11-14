//! Targeted complete Return and death-location games against two anchor legends.
//! Preserve all prior match seeds; append this separate family matrix.
mod support;
use support::conserved;

use local_rift::deck::{CustomDeck, eligible_cards};
use local_rift::game::{Difficulty, Game, Phase};
use local_rift::legend_catalog::legends;
use std::collections::BTreeSet;

fn expanded(legend: usize) -> CustomDeck {
    let mut deck = CustomDeck::from_legend(legend).unwrap();
    deck.name = format!("Return and death-location verification {legend}");
    for id in eligible_cards(legend)
        .into_iter()
        .filter(|id| (372..376).contains(id))
    {
        if let Some((_, copies)) = deck.cards.iter_mut().find(|(existing, _)| *existing == id) {
            *copies = 3;
        } else {
            deck.cards.push((id, 3));
        }
    }
    for id in eligible_cards(legend)
        .into_iter()
        .filter(|id| local_rift::cards::cards()[*id].kind == local_rift::cards::CardKind::Gear)
        .take(3)
    {
        if !deck.cards.iter().any(|(existing, _)| *existing == id) {
            deck.cards.push((id, 3));
        }
    }
    deck.validate().unwrap();
    deck
}

#[test]
fn return_and_death_location_cards_finish_legal_homes_against_anchors_at_all_levels() {
    let mut seen_cards = BTreeSet::new();
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
                for second in [0, 3] {
                    let decks = [expanded(first), expanded(second)];
                    // Preserve the original eight-legend seeds as the matrix grows.
                    let seed = 181000 + (level * 1000 + starting * 100 + first * 8 + second) as u64;
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
                            if item.ability.is_none() {
                                seen_cards.insert(item.card_id);
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
    assert_eq!(matches, legends().len() * 2 * 6);
    assert!(
        (372..376).all(|id| seen_cards.contains(&id)),
        "not all new cards entered play: {seen_cards:?}"
    );
    println!("PASS: {matches} complete Return and death-location games; all four new cards played");
}
