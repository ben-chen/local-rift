//! Full games with authored legal substitutions, never edits to sourced presets.
use local_rift::{
    deck::CustomDeck,
    game::{Difficulty, Game, Target},
};
use std::collections::BTreeSet;
mod support;

fn control_deck() -> CustomDeck {
    let mut deck = CustomDeck::from_preset(0).unwrap();
    deck.name = "Irelia / control integration".into();
    for (old, new) in [(5, 468), (6, 469), (17, 449)] {
        let row = deck.cards.iter_mut().find(|row| row.0 == old).unwrap();
        assert_eq!(row.1, 3);
        row.0 = new;
    }
    deck
}

#[test]
fn control_cards_finish_games_and_conserve_original_ownership_at_every_level() {
    let mut plays = [[[0; 2]; 2]; 3];
    let mut transfers = [[0; 2]; 3];
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for seat in 0..2 {
            for starter in 0..2 {
                for sample in 0..2 {
                    let mut custom = [None, None];
                    custom[seat] = Some(control_deck());
                    let presets = [0, 0];
                    let specs = std::array::from_fn(|p| {
                        custom[p]
                            .clone()
                            .unwrap_or_else(|| CustomDeck::from_preset(presets[p]).unwrap())
                    });
                    let seed = 469200
                        + level as u64 * 100
                        + seat as u64 * 20
                        + starter as u64 * 4
                        + sample;
                    let mut g =
                        Game::new_with_custom(presets, custom, seed, difficulty, false).unwrap();
                    g.choose_starting_player(starter).unwrap();
                    g.start_match().unwrap();
                    let mut played = BTreeSet::new();
                    let mut taken = BTreeSet::new();
                    for step in 0..4000 {
                        support::conserved(&g, &specs);
                        for item in &g.chain {
                            if item.player == seat
                                && item.ability.is_none()
                                && [468, 469].contains(&item.card_id)
                                && played.insert(item.id)
                            {
                                plays[level][seat][item.card_id - 468] += 1;
                            }
                        }
                        for unit in &g.players[seat].units {
                            if g.board_owner(Target::Unit {
                                player: seat,
                                unit_id: unit.id,
                            }) == Some(1 - seat)
                                && taken.insert(unit.id)
                            {
                                transfers[level][seat] += 1;
                            }
                        }
                        if g.winner.is_some() {
                            break;
                        }
                        assert!(g.step_ai().unwrap_or_else(|e| panic!(
                            "{difficulty:?} seat{seat} start{starter} seed{seed} step{step}: {e}"
                        )));
                        g.validate_numeric_state().unwrap();
                    }
                    assert!(
                        g.winner.is_some(),
                        "{difficulty:?} seat{seat} start{starter} seed{seed}"
                    );
                }
            }
        }
    }
    eprintln!("Control plays by level/seat/card: {plays:?}; distinct taken units: {transfers:?}");
    for (level, counts) in plays.iter().enumerate() {
        for card in 0..2 {
            assert!(
                counts[0][card] + counts[1][card] > 0,
                "level{level} card{card}"
            );
        }
    }
    for seat in 0..2 {
        for card in 0..2 {
            assert!(
                plays.iter().map(|level| level[seat][card]).sum::<usize>() > 0,
                "seat{seat} card{card}"
            );
        }
        assert!(transfers.iter().map(|level| level[seat]).sum::<usize>() > 0);
    }
}
