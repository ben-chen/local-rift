//! Authored Alpha package for complete games; no tournament recipe is altered.
use local_rift::{
    cards::presets,
    deck::CustomDeck,
    game::{Decision, Difficulty, Game},
};

#[test]
fn alpha_package_finishes_both_irelia_matchups_at_every_level_and_seat() {
    let mut played = [[0usize; 2]; 3];
    let mut splits = [[0usize; 2]; 3];
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for opponent in [0, 5] {
            for seat in 0..2 {
                for starter in 0..2 {
                    for sample in 0..12 {
                        let mut deck = CustomDeck::from_preset(1).unwrap();
                        deck.name = "Master Yi / Alpha allocation verification".into();
                        let row = deck.cards.iter_mut().find(|(id, _)| *id == 7).unwrap();
                        assert_eq!(row.1, 3);
                        row.0 = 443;
                        deck.validate().unwrap();
                        assert_eq!(deck.card_count(), 40);
                        let mut presets = [1, opponent];
                        let mut custom = [Some(deck), None];
                        if seat == 1 {
                            presets.swap(0, 1);
                            custom.swap(0, 1);
                        }
                        let seed = 443100
                            + (sample * 4096 + level * 100 + opponent * 10 + seat * 2 + starter)
                                as u64;
                        let mut g = Game::new_with_custom(presets, custom, seed, difficulty, false)
                            .unwrap();
                        g.choose_starting_player(starter).unwrap();
                        g.start_match().unwrap();
                        let mut seen_spells = std::collections::HashSet::new();
                        for step in 0..2400 {
                            if g.winner.is_some() {
                                break;
                            }
                            for item in &g.chain {
                                if item.card_id == 443
                                    && item.ability.is_none()
                                    && seen_spells.insert(item.id)
                                {
                                    played[level][seat] += 1;
                                }
                            }
                            if matches!(g.decision,Some(Decision::SplitDamage{player,..}) if player==seat)
                            {
                                splits[level][seat] += 1;
                            }
                            assert!(g.step_ai().unwrap_or_else(|e|panic!(
                                "{difficulty:?} vs{opponent} seat{seat} start{starter} sample{sample} step{step}: {e}"
                            )));
                            g.validate_numeric_state().unwrap();
                            let p = &g.players[seat];
                            let copies = p
                                .deck
                                .iter()
                                .chain(&p.hand)
                                .chain(&p.discard)
                                .chain(&p.banish)
                                .filter(|&&card| card == 443)
                                .count()
                                + g.chain
                                    .iter()
                                    .filter(|item| {
                                        item.player == seat
                                            && item.card_id == 443
                                            && item.ability.is_none()
                                    })
                                    .count();
                            assert_eq!(
                                copies, 3,
                                "An Alpha Strike physical copy disappeared or duplicated"
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
    eprintln!("Alpha casts by level/seat: {played:?}; allocation choices: {splits:?}");
    assert!(
        played.iter().flatten().all(|&count| count > 0),
        "Every level and seat must exercise Alpha"
    );
    assert!(
        splits.iter().flatten().all(|&count| count > 0),
        "Every level and seat must resolve an allocation"
    );
    assert!(presets()[1].cards.contains(&(7, 3)));
    assert!(!presets()[1].cards.iter().any(|&(id, _)| id == 443));
}
