//! Authored legal Kennen substitution; the sourced tournament recipe is unchanged.
use local_rift::{
    cards::presets,
    deck::CustomDeck,
    game::{Difficulty, Game},
};

#[test]
fn deathgrip_completes_all_presets_both_seats_starters_and_difficulties() {
    let mut played = [[0usize; 2]; 3];
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for opponent in 0..presets().len() {
            for (seat, plays) in played[level].iter_mut().enumerate() {
                for starter in 0..2 {
                    let mut deck = CustomDeck::from_preset(2).unwrap();
                    deck.name = "Kennen / Deathgrip verification".into();
                    let row = deck.cards.iter_mut().find(|(id, _)| *id == 39).unwrap();
                    assert_eq!(row.1, 3);
                    row.0 = 447;
                    deck.validate().unwrap();
                    assert_eq!(deck.card_count(), 40);
                    let mut lists = [2, opponent];
                    let mut custom = [Some(deck), None];
                    if seat == 1 {
                        lists.swap(0, 1);
                        custom.swap(0, 1);
                    }
                    let seed = 447000 + (level * 100 + opponent * 4 + seat * 2 + starter) as u64;
                    let mut game =
                        Game::new_with_custom(lists, custom, seed, difficulty, false).unwrap();
                    game.choose_starting_player(starter).unwrap();
                    game.start_match().unwrap();
                    let mut seen = std::collections::BTreeSet::new();
                    for step in 0..2400 {
                        if game.winner.is_some() {
                            break;
                        }
                        assert!(game.step_ai().unwrap_or_else(|error| panic!("{difficulty:?} vs{opponent} seat{seat} starter{starter} step{step}: {error}")));
                        game.validate_numeric_state().unwrap();
                        for item in game
                            .chain
                            .iter()
                            .filter(|i| i.player == seat && i.card_id == 447 && i.ability.is_none())
                        {
                            if seen.insert(item.id) {
                                *plays += 1;
                            }
                        }
                        let state = &game.players[seat];
                        let copies = state
                            .deck
                            .iter()
                            .chain(&state.hand)
                            .chain(&state.discard)
                            .chain(&state.banish)
                            .filter(|&&id| id == 447)
                            .count()
                            + game
                                .pending_cards()
                                .iter()
                                .filter(|card| card.player == seat && card.card_id == 447)
                                .count()
                            + usize::from(game.payment_plan_view(seat).is_some_and(|plan| {
                                plan.player == seat && plan.pending_card == Some(447)
                            }))
                            + state.hidden.iter().filter(|h| h.card_id == 447).count()
                            + game
                                .chain
                                .iter()
                                .filter(|i| {
                                    i.player == seat && i.card_id == 447 && i.ability.is_none()
                                })
                                .count();
                        assert_eq!(
                            copies, 3,
                            "Deathgrip copies at {difficulty:?} vs{opponent} seat{seat} starter{starter} seed{seed} step{step}; decision={:?}",
                            game.decision
                        );
                    }
                    assert!(
                        game.winner.is_some(),
                        "{difficulty:?}/{opponent}/{seat}/{starter}"
                    );
                }
            }
        }
    }
    eprintln!("Deathgrip actual plays by difficulty/seat: {played:?}");
    assert!(played.iter().flatten().all(|&count| count > 0));
    assert!(presets()[2].cards.contains(&(39, 3)));
    assert!(!presets()[2].cards.iter().any(|&(id, _)| id == 447));
}
