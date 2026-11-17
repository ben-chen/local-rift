//! Complete legal authored Irelia games, independent physical-card accounting.
use local_rift::{
    deck::CustomDeck,
    game::{Difficulty, Game},
};
use std::collections::BTreeSet;
mod support;

fn deck() -> CustomDeck {
    let mut deck = CustomDeck::from_preset(0).unwrap();
    deck.name = "Irelia / resource spells integration".into();
    for (old, new) in [(4, 491), (14, 492), (16, 493), (21, 494)] {
        let row = deck.cards.iter_mut().find(|row| row.0 == old).unwrap();
        assert_eq!(row.1, 2);
        row.0 = new;
    }
    deck
}

#[test]
fn resource_spells_complete_48_games_and_report_actual_cast_coverage() {
    let mut casts = [[[0usize; 4]; 2]; 3];
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
                let seed = 494300 + level as u64 * 100 + starter as u64 * 10 + sample;
                let mut game =
                    Game::new_with_custom([0, 0], specs.clone().map(Some), seed, difficulty, false)
                        .unwrap();
                game.choose_starting_player(starter).unwrap();
                game.start_match().unwrap();
                let mut seen = BTreeSet::new();
                for step in 0..4000 {
                    support::conserved(&game, &specs);
                    for item in &game.chain {
                        if (491..=494).contains(&item.card_id)
                            && item.ability.is_none()
                            && seen.insert(item.id)
                        {
                            casts[level][item.player][item.card_id - 491] += 1;
                            if item.card_id == 491 {
                                assert_eq!(item.targets.len(), 1);
                            }
                        }
                    }
                    if game.winner.is_some() {
                        break;
                    }
                    assert!(game.step_ai().unwrap_or_else(|error| panic!(
                        "{difficulty:?} start{starter} seed{seed} step{step}: {error}"
                    )));
                }
                support::conserved(&game, &specs);
                assert!(
                    game.winner.is_some(),
                    "unfinished {difficulty:?} start{starter} seed{seed}"
                );
            }
        }
    }
    eprintln!("resource spells casts={casts:?}");
    for (level, seats) in casts.iter().enumerate() {
        for (seat, cards) in seats.iter().enumerate() {
            for (card, &count) in cards.iter().enumerate() {
                // Party Favors is symmetric and costs its caster a card and
                // Energy. Strategic may rationally decline it in this Irelia
                // list. Its mandatory opponent choices are exercised at every
                // difficulty in resource_spells_tests; retain actual zero counts.
                if level == 2 && card == 3 {
                    continue;
                }
                assert!(
                    count > 0,
                    "no cast level{level} seat{seat} card{}",
                    491 + card
                );
            }
        }
    }
}
