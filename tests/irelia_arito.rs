use local_rift::{
    cards::presets,
    deck::CustomDeck,
    game::{Difficulty, Game},
};
use std::collections::BTreeMap;

#[test]
fn arito_is_the_exact_second_irelia_preset_with_a_separate_registration() {
    let all = presets();
    let p = &all[5];
    assert_eq!(p.legend_id, 0);
    assert_eq!(p.chosen_champion, 0);
    assert_eq!(
        p.cards.iter().map(|(_, n)| usize::from(*n)).sum::<usize>(),
        40
    );
    for entry in [(432, 2), (430, 3), (431, 1), (425, 1), (426, 1)] {
        assert!(p.cards.contains(&entry));
    }
    let r = p.registration.unwrap();
    assert_eq!(r.battlefields, &[13, 0, 14]);
    assert_eq!(
        r.sideboard,
        &[(424, 2), (369, 2), (20, 1), (421, 2), (11, 2), (1, 1)]
    );
    assert_eq!((r.pilot, r.date, r.placing), ("Arito", "2026-10-31", 2));
    let deck = CustomDeck::from_preset(5).unwrap();
    deck.validate().unwrap();
    assert_eq!(deck.legend, 0);
    assert_eq!(deck.cards, p.cards);
    assert_eq!(deck.battlefield, 13);
    assert_ne!(deck.cards, CustomDeck::from_legend(0).unwrap().cards);
    assert_eq!(CustomDeck::from_legend(0).unwrap().cards, all[0].cards);
    let catalog = local_rift::api::catalog();
    assert_eq!(catalog["presets"][5]["legend_id"], 0);
    assert_eq!(
        catalog["presets"][5]["registration"]["battlefields"],
        serde_json::json!([13, 0, 14])
    );
    assert_eq!(
        catalog["presets"][5]["registration"]["sideboard"],
        serde_json::json!(r.sideboard)
    );
    assert_eq!(
        catalog["legends"][0]["training_deck"],
        CustomDeck::from_legend(0).unwrap().to_json()
    );
}

#[test]
fn arito_setup_keeps_chosen_champion_and_sideboard_out_of_the_draw_deck() {
    for seat in 0..2 {
        for field in [13, 0, 14] {
            let decks = if seat == 0 { [5, 1] } else { [1, 5] };
            let mut game = Game::new_for_agents(decks[0], decks[1], 432005, Difficulty::Easy);
            game.choose_starting_player(seat).unwrap();
            game.choose_battlefield(seat, field).unwrap();
            assert_eq!(game.legend_id(seat), 0);
            assert_eq!(game.players[seat].champion, Some(0));
            let p = &game.players[seat];
            assert_eq!(p.hand.len() + p.deck.len(), 39);
            let mut cards = BTreeMap::new();
            for &c in p.hand.iter().chain(&p.deck) {
                *cards.entry(c).or_insert(0usize) += 1;
            }
            let mut expected: BTreeMap<_, _> = presets()[5]
                .cards
                .iter()
                .map(|&(c, n)| (c, usize::from(n)))
                .collect();
            expected.remove(&0);
            assert_eq!(cards, expected);
            assert!(!cards.contains_key(&424));
            assert!(!cards.contains_key(&369));
            game.start_match().unwrap();
            assert_eq!(game.battlefields[seat], field);
            assert_eq!(p_reset_runes(&game, seat), 12);
        }
    }
}
fn p_reset_runes(game: &Game, seat: usize) -> usize {
    game.players[seat].rune_deck.len() + usize::from(game.players[seat].total_runes())
}

#[test]
fn exact_arito_deck_completes_every_preset_level_seat_and_starter() {
    let mut games = 0;
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for opponent in 0..presets().len() {
            for seat in 0..2 {
                for starter in 0..2 {
                    let decks = if seat == 0 {
                        [5, opponent]
                    } else {
                        [opponent, 5]
                    };
                    let seed = 432000 + (level * 100 + opponent * 4 + seat * 2 + starter) as u64;
                    let mut game = Game::new_for_agents(decks[0], decks[1], seed, difficulty);
                    game.choose_starting_player(starter).unwrap();
                    game.choose_battlefield(
                        seat,
                        [13, 0, 14][(opponent + level + seat + starter) % 3],
                    )
                    .unwrap();
                    game.start_match().unwrap();
                    for _ in 0..2600 {
                        if game.winner.is_some() {
                            break;
                        }
                        assert!(game.step_ai().unwrap_or_else(|e| panic!(
                            "{difficulty:?}/{opponent}/{seat}/{starter}: {e}"
                        )));
                    }
                    assert!(
                        game.winner.is_some(),
                        "{difficulty:?}/{opponent}/{seat}/{starter}"
                    );
                    games += 1;
                }
            }
        }
    }
    assert_eq!(games, 72);
}
