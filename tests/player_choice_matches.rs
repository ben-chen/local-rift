//! Legal authored Irelia lists; exact tournament presets are never modified.
use local_rift::{
    deck::CustomDeck,
    game::{Decision, Difficulty, Game},
};
use std::collections::BTreeSet;
mod support;

fn deck() -> CustomDeck {
    let mut deck = CustomDeck::from_preset(0).unwrap();
    deck.name = "Irelia / each-player choices integration".into();
    for (old, new) in [(5, 480), (7, 481)] {
        let row = deck.cards.iter_mut().find(|row| row.0 == old).unwrap();
        assert_eq!(row.1, 3);
        row.0 = new;
    }
    deck
}

#[test]
fn each_player_spells_finish_48_games_and_preserve_physical_cards() {
    let mut casts = [[[0usize; 2]; 2]; 3];
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
                let seed = 480200 + level as u64 * 100 + starter as u64 * 10 + sample;
                let mut game =
                    Game::new_with_custom([0, 0], specs.clone().map(Some), seed, difficulty, false)
                        .unwrap();
                game.choose_starting_player(starter).unwrap();
                game.start_match().unwrap();
                let mut seen = BTreeSet::new();
                let mut decisions = BTreeSet::new();
                for step in 0..4000 {
                    support::conserved(&game, &specs);
                    for item in &game.chain {
                        if (480..=481).contains(&item.card_id)
                            && item.ability.is_none()
                            && seen.insert(item.id)
                        {
                            casts[level][item.player][item.card_id - 480] += 1;
                            assert!(item.targets.is_empty());
                        }
                    }
                    if let Some(Decision::PlayerObject {
                        player,
                        choice_id,
                        card_id,
                        ..
                    }) = game.decision
                        && decisions.insert(choice_id)
                    {
                        choices[level][player][card_id - 480] += 1;
                    }
                    if game.winner.is_some() {
                        break;
                    }
                    assert!(game.step_ai().unwrap_or_else(|error| panic!(
                        "{difficulty:?} start{starter} seed{seed} step{step}: {error}"
                    )));
                    game.validate_numeric_state().unwrap();
                }
                support::conserved(&game, &specs);
                assert!(
                    game.winner.is_some(),
                    "unfinished {difficulty:?} start{starter} seed{seed}"
                );
            }
        }
    }
    eprintln!("casts={casts:?}; player choices={choices:?}");
    for level in 0..3 {
        for seat in 0..2 {
            for card in 0..2 {
                assert!(
                    casts[level][seat][card] > 0,
                    "no cast: level{level} seat{seat} card{}",
                    480 + card
                );
                assert!(
                    choices[level][seat][card] > 0,
                    "no choice: level{level} seat{seat} card{}",
                    480 + card
                );
            }
        }
    }
}
