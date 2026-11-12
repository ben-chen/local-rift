//! Authored 40-card reaction package; the sourced tournament recipe is unchanged.
use local_rift::{
    cards::presets,
    deck::CustomDeck,
    game::{Difficulty, Game, Target},
};

#[test]
fn master_yi_reactions_finish_irelia_and_damage_matchups_at_every_level() {
    let mut played = [[[false; 2]; 3]; 2];
    let mut countered = [[false; 2]; 3];
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
                        deck.name = "Master Yi / reaction package verification".into();
                        for (old, new) in [(7, 442), (6, 441)] {
                            let row = deck.cards.iter_mut().find(|(id, _)| *id == old).unwrap();
                            assert_eq!(row.1, 3);
                            row.0 = new;
                        }
                        deck.validate().unwrap();
                        assert_eq!(deck.card_count(), 40);
                        let mut ids = [1, opponent];
                        let mut custom = [Some(deck), None];
                        if seat == 1 {
                            ids.swap(0, 1);
                            custom.swap(0, 1);
                        }
                        let seed = 442000
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
                                if item.ability.is_none() && matches!(item.card_id, 441 | 442) {
                                    played[item.card_id - 441][level][item.player] = true;
                                }
                            }
                            // A single AI step resolving Repulse removes both the counter
                            // and the exact enemy chain object. A cast alone is not success.
                            let pending_counter = g.chain.last().and_then(|item| {
                                if item.card_id != 442 || item.ability.is_some() {
                                    return None;
                                }
                                let Some(Target::Spell { chain_id }) = item.targets.get(1) else {
                                    return None;
                                };
                                if !g.chain.iter().any(|target| target.id == *chain_id) {
                                    return None;
                                }
                                Some((item.id, item.player, *chain_id))
                            });
                            assert!(g.step_ai().unwrap_or_else(|e| panic!(
                                "{difficulty:?} vs{opponent} seat{seat} start{starter} sample{sample} step{step}: {e}"
                            )));
                            if let Some((counter, player, target)) = pending_counter
                                && !g
                                    .chain
                                    .iter()
                                    .any(|item| item.id == counter || item.id == target)
                            {
                                countered[level][player] = true;
                            }
                            g.validate_numeric_state().unwrap();
                            let p = &g.players[seat];
                            for card in [441, 442] {
                                let copies = p
                                    .deck
                                    .iter()
                                    .chain(&p.hand)
                                    .chain(&p.discard)
                                    .chain(&p.banish)
                                    .filter(|&&id| id == card)
                                    .count()
                                    + g.chain
                                        .iter()
                                        .filter(|item| {
                                            item.player == seat
                                                && item.card_id == card
                                                && item.ability.is_none()
                                        })
                                        .count();
                                assert_eq!(
                                    copies, 3,
                                    "Physical card{card} disappeared or duplicated"
                                );
                            }
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
        played.iter().flatten().flatten().all(|&seen| seen),
        "Plays by card/level/seat: {played:?}"
    );
    assert!(
        countered.iter().flatten().all(|&seen| seen),
        "Successful Repulse by level/seat: {countered:?}"
    );
    for card in [6, 7] {
        assert!(presets()[1].cards.contains(&(card, 3)));
    }
    assert!(
        !presets()[1]
            .cards
            .iter()
            .any(|&(id, _)| matches!(id, 441 | 442))
    );
}
