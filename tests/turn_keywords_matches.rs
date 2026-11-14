//! Authored legal lists exercise both cards; these are not tournament presets.
mod support;

use local_rift::deck::CustomDeck;
use local_rift::game::{ChainAbility, Difficulty, Game, Phase};
use std::collections::BTreeSet;

fn deck(legend: usize) -> CustomDeck {
    let mut deck = CustomDeck::from_legend(legend).unwrap();
    if legend != 0 {
        return deck;
    }
    for id in [461, 462] {
        if let Some((_, count)) = deck.cards.iter_mut().find(|(card, _)| *card == id) {
            *count = 3;
        } else {
            deck.cards.push((id, 3));
        }
    }
    let mut excess = deck
        .cards
        .iter()
        .map(|(_, n)| usize::from(*n))
        .sum::<usize>()
        - 40;
    for (id, count) in &mut deck.cards {
        if [461, 462].contains(id) {
            continue;
        }
        let minimum = u8::from(*id == deck.chosen_champion);
        let remove = usize::from(*count - minimum).min(excess);
        *count -= u8::try_from(remove).unwrap();
        excess -= remove;
    }
    assert_eq!(excess, 0);
    deck.cards.retain(|(_, count)| *count > 0);
    deck.name = "Irelia / Block and Yuumi verification".into();
    deck.validate().unwrap();
    deck
}

#[test]
fn turn_keywords_complete_legal_games_both_seats_starters_and_all_ai_levels() {
    let mut block_casts = [0; 2];
    let mut yuumi_triggers = [0; 2];
    let mut shield_seen = [false; 2];
    let mut yuumi_seen = [false; 2];
    let mut completed = 0;
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for opponent in [1, 3] {
            for seat in 0..2 {
                for first in 0..2 {
                    let mut legends = [0, opponent];
                    if seat == 1 {
                        legends.swap(0, 1);
                    }
                    let decks = legends.map(deck);
                    let seed = 461000 + (1000 * level + 100 * opponent + 10 * seat + first) as u64;
                    let mut g = Game::new_with_custom(
                        [0, 3],
                        [Some(decks[0].clone()), Some(decks[1].clone())],
                        seed,
                        difficulty,
                        false,
                    )
                    .unwrap();
                    g.choose_starting_player(first).unwrap();
                    g.start_match().unwrap();
                    let mut chain_ids = BTreeSet::new();
                    for step in 0..6000 {
                        support::conserved(&g, &decks);
                        for owner in 0..2 {
                            for unit in &g.players[owner].units {
                                yuumi_seen[owner] |= !unit.token && unit.card_id == 462;
                                shield_seen[owner] |= g.turn_shield(unit.id).unwrap() >= 3;
                            }
                        }
                        for item in &g.chain {
                            if !chain_ids.insert(item.id) {
                                continue;
                            }
                            if item.card_id == 461 && item.ability.is_none() {
                                block_casts[item.player] += 1;
                            }
                            if matches!(item.ability, Some(ChainAbility::CardTrigger { .. }))
                                && item.card_id == 462
                            {
                                yuumi_triggers[item.player] += 1;
                            }
                        }
                        if g.winner.is_some() {
                            break;
                        }
                        assert!(
                            g.step_ai().unwrap_or_else(|error| panic!(
                                "seed {seed}, step {step}: {error}"
                            )),
                            "AI stopped: seed {seed}"
                        );
                    }
                    assert_eq!(g.phase, Phase::Finished, "seed {seed}, {difficulty:?}");
                    assert!(g.players[g.winner.unwrap()].score >= 8);
                    support::conserved(&g, &decks);
                    completed += 1;
                }
            }
        }
    }
    assert_eq!(completed, 24);
    assert!(
        block_casts.iter().all(|n| *n > 0),
        "Block casts: {block_casts:?}"
    );
    assert!(
        yuumi_triggers.iter().all(|n| *n > 0),
        "Yuumi triggers: {yuumi_triggers:?}"
    );
    assert_eq!(yuumi_seen, [true; 2]);
    assert_eq!(shield_seen, [true; 2]);
    println!(
        "PASS: {completed} legal complete games; Block casts {block_casts:?}; Yuumi triggers {yuumi_triggers:?}; Shield grants observed {shield_seen:?}"
    );
}
