//! Authored 40-card practice substitutions, not a new tournament registration.
use local_rift::{
    cards::presets,
    deck::CustomDeck,
    game::{Difficulty, Game},
};

#[test]
fn unyielding_master_yi_finishes_irelia_and_damage_matchups_at_every_level() {
    let mut observed = [[false; 2]; 3];
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for opponent in [0, 5, 3] {
            // Xeno Irelia, Arito Irelia, historical Kai'Sa main deck.
            for seat in 0..2 {
                for starter in 0..2 {
                    for sample in 0..4 {
                        let mut deck = CustomDeck::from_preset(1).unwrap();
                        deck.name = "Master Yi / Unyielding Spirit practice verification".into();
                        let discipline = deck.cards.iter_mut().find(|(id, _)| *id == 7).unwrap();
                        assert_eq!(discipline.1, 3);
                        discipline.0 = 441; // Three-for-three authored substitution only.
                        deck.validate().unwrap();
                        assert_eq!(deck.card_count(), 40);
                        let mut ids = [1, opponent];
                        let mut custom = [Some(deck), None];
                        if seat == 1 {
                            ids.swap(0, 1);
                            custom.swap(0, 1);
                        }
                        let seed = 441000
                            + (sample * 4096 + level * 100 + opponent * 10 + seat * 2 + starter)
                                as u64;
                        let mut g =
                            Game::new_with_custom(ids, custom, seed, difficulty, false).unwrap();
                        g.choose_starting_player(starter).unwrap();
                        g.start_match().unwrap();
                        for step in 0..2400 {
                            if g.winner.is_some() {
                                break;
                            }
                            for item in &g.chain {
                                if item.card_id == 441 && item.ability.is_none() {
                                    observed[level][item.player] = true;
                                }
                            }
                            assert!(g.step_ai().unwrap_or_else(|e| panic!(
                            "{difficulty:?} vs{opponent} seat{seat} start{starter} step{step}: {e}"
                        )));
                            g.validate_numeric_state().unwrap();
                            let p = &g.players[seat];
                            let copies = p
                                .deck
                                .iter()
                                .chain(&p.hand)
                                .chain(&p.discard)
                                .chain(&p.banish)
                                .filter(|&&id| id == 441)
                                .count()
                                + g.chain
                                    .iter()
                                    .filter(|item| {
                                        item.player == seat
                                            && item.card_id == 441
                                            && item.ability.is_none()
                                    })
                                    .count();
                            assert_eq!(
                                copies, 3,
                                "A physical Unyielding Spirit copy disappeared or duplicated"
                            );
                        }
                        assert!(
                            g.winner.is_some(),
                            "{difficulty:?} vs{opponent} seat{seat} start{starter} sample{sample}"
                        );
                    }
                }
            }
        }
    }
    assert!(
        observed.iter().all(|seats| seats.iter().all(|seen| *seen)),
        "Card plays observed by level/seat: {observed:?}"
    );
    // Source recipes remain untouched by this experiment.
    assert!(presets()[1].cards.contains(&(7, 3)));
    assert!(!presets()[1].cards.iter().any(|&(id, _)| id == 441));
}
