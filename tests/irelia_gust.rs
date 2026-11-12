//! Exact 40-card authored Irelia practice variant, leaving sourced recipes intact.
use local_rift::game::{Difficulty, Game};

#[test]
fn gust_irelia_completes_all_presets_seats_starters_and_levels() {
    let mut paid = 0;
    let mut granted = 0;
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for opponent in 0..local_rift::cards::presets().len() {
            for seat in 0..2 {
                for starter in 0..2 {
                    let mut deck = local_rift::deck::CustomDeck::from_preset(0).unwrap();
                    deck.name = "Irelia Gust Monk practice verification".into();
                    let slot = deck.cards.iter_mut().find(|entry| entry.0 == 17).unwrap();
                    assert_eq!(slot.1, 3);
                    slot.0 = 431;
                    assert_eq!(deck.card_count(), 40);
                    deck.validate().unwrap();
                    let mut presets = [0, opponent];
                    let mut custom = [Some(deck), None];
                    if seat == 1 {
                        presets.swap(0, 1);
                        custom.swap(0, 1);
                    }
                    let seed = 431_000 + (level * 100 + opponent * 4 + seat * 2 + starter) as u64;
                    let mut game =
                        Game::new_with_custom(presets, custom, seed, difficulty, false).unwrap();
                    game.choose_starting_player(starter).unwrap();
                    game.start_match().unwrap();
                    for _ in 0..2400 {
                        if game.winner.is_some() {
                            break;
                        }
                        if let Some(local_rift::game::Decision::TriggerPayment {
                            card_id: 431,
                            ..
                        }) = game.decision
                        {
                            paid += 1;
                        }
                        if !game.legend[seat].assault_grants.is_empty() {
                            granted += 1;
                        }
                        assert!(game.step_ai().unwrap_or_else(|e| panic!(
                            "{difficulty:?}/{opponent}/{seat}/{starter}: {e}"
                        )));
                    }
                    assert!(
                        game.winner.is_some(),
                        "{difficulty:?}/{opponent}/{seat}/{starter}"
                    );
                    let player = &game.players[seat];
                    let copies = player
                        .deck
                        .iter()
                        .chain(&player.hand)
                        .chain(&player.discard)
                        .chain(&player.banish)
                        .filter(|&&id| id == 431)
                        .count()
                        + player
                            .units
                            .iter()
                            .filter(|unit| !unit.token && unit.card_id == 431)
                            .count()
                        + game
                            .chain
                            .iter()
                            .filter(|item| {
                                item.player == seat && item.card_id == 431 && item.ability.is_none()
                            })
                            .count();
                    assert_eq!(copies, 3);
                }
            }
        }
    }
    assert!(
        paid > 0 && granted > 0,
        "Gust Monk must reach payment and a temporary Assault grant: {paid}/{granted}"
    );
}
