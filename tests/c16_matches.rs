//! Complete games exercise both new legends through normal AI actions.
mod support;
use support::conserved;

use local_rift::deck::CustomDeck;
use local_rift::game::{ChainAbility, Difficulty, Game, Phase};
use local_rift::legend_catalog::legends;
use local_rift::legends::LegendAbility;
use std::collections::BTreeSet;

#[test]
fn rengar_and_ezreal_finish_against_every_legend_from_both_seats_at_all_levels() {
    let mut matches = 0;
    let mut abilities = BTreeSet::new();
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for new_legend in [8, 9] {
            for opponent in 0..legends().len() {
                for seat in 0..2 {
                    for starting in 0..2 {
                        let mut ids = [opponent; 2];
                        ids[seat] = new_legend;
                        let decks = ids.map(|id| CustomDeck::from_legend(id).unwrap());
                        let seed = 261000
                            + (level * 10000
                                + new_legend * 1000
                                + opponent * 100
                                + seat * 10
                                + starting) as u64;
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
                            for item in &game.chain {
                                if item.player != seat {
                                    continue;
                                }
                                let exercised = matches!(
                                    item.ability,
                                    Some(ChainAbility::Legend(
                                        LegendAbility::RengarMight { .. }
                                            | LegendAbility::EzrealDraw
                                    ))
                                );
                                if exercised {
                                    abilities.insert((level, new_legend, seat));
                                }
                            }
                            if game.winner.is_some() {
                                break;
                            }
                            assert!(game.step_ai().unwrap_or_else(|error| panic!(
                                "legends{ids:?} {difficulty:?} starter{starting} seed{seed} step{step}: {error}; recent {:?}",
                                game.log.iter().rev().take(8).collect::<Vec<_>>()
                            )), "AI stopped in an unfinished match");
                        }
                        assert_eq!(
                            game.phase,
                            Phase::Finished,
                            "legends{ids:?} {difficulty:?} starter{starting} seed{seed} stalled"
                        );
                        assert!(game.players[game.winner.unwrap()].score >= 8);
                        conserved(&game, &decks);
                        matches += 1;
                    }
                }
            }
        }
    }
    assert_eq!(matches, legends().len() * 24);
    assert_eq!(
        abilities.len(),
        12,
        "each legend's ability must be exercised at every difficulty and seat: {abilities:?}"
    );
    println!(
        "PASS: {matches} complete Rengar/Ezreal games; both abilities exercised at all levels and seats"
    );
}
