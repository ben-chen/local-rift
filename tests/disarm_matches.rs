//! Authored legal practice lists, not new tournament presets.
//! Irelia is Calm/Chaos: only Siren fits her identity. The Mind cards run in
//! Kai'Sa and Ezreal homes, with both seats/starters and every AI level.
mod support;

use local_rift::cards::CardId;
use local_rift::deck::{CustomDeck, eligible_cards};
use local_rift::game::{ChainAbility, Difficulty, Game, KeywordTrigger, Phase};
use std::collections::BTreeSet;

fn deck(legend: usize) -> CustomDeck {
    let mut deck = CustomDeck::from_legend(legend).unwrap();
    let eligible = eligible_cards(legend);
    let package: Vec<CardId> = (458..=460).filter(|id| eligible.contains(id)).collect();
    if legend == 0 {
        assert_eq!(package, [460]);
    }
    for &id in &package {
        if let Some((_, count)) = deck.cards.iter_mut().find(|(existing, _)| *existing == id) {
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
        if package.contains(id) {
            continue;
        }
        let minimum = u8::from(*id == deck.chosen_champion);
        let remove = usize::from(*count - minimum).min(excess);
        *count -= u8::try_from(remove).unwrap();
        excess -= remove;
    }
    assert_eq!(excess, 0);
    deck.cards.retain(|(_, count)| *count > 0);
    deck.name = format!("Disarm verification {legend}");
    deck.validate().unwrap();
    deck
}

#[test]
fn disarm_finishes_legal_irelia_matchups_with_both_seats_starters_and_all_levels() {
    let mut seen: [BTreeSet<CardId>; 2] = std::array::from_fn(|_| BTreeSet::new());
    let mut triggers = [0; 2];
    let mut completed = 0;
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for home in [3, 9] {
            for seat in 0..2 {
                for first in 0..2 {
                    let mut legends = [0, home];
                    if seat == 1 {
                        legends.swap(0, 1);
                    }
                    let decks = legends.map(deck);
                    let seed = 458000 + (1000 * level + 100 * home + 10 * seat + first) as u64;
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
                    for _ in 0..6000 {
                        support::conserved(&g, &decks);
                        for (owner, cards) in seen.iter_mut().enumerate() {
                            cards.extend(
                                g.players[owner]
                                    .units
                                    .iter()
                                    .filter(|u| !u.token && (458..=460).contains(&u.card_id))
                                    .map(|u| u.card_id),
                            );
                        }
                        for item in &g.chain {
                            if matches!(
                                item.ability,
                                Some(ChainAbility::CardTrigger {
                                    context: local_rift::game::TriggerContext {
                                        keyword: Some(KeywordTrigger::Disarm { .. }),
                                        ..
                                    },
                                    ..
                                })
                            ) && chain_ids.insert(item.id)
                            {
                                triggers[item.player] += 1;
                            }
                        }
                        if g.winner.is_some() {
                            break;
                        }
                        assert!(
                            g.step_ai().unwrap(),
                            "AI stopped before completion: seed {seed}"
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
    for cards in &seen {
        assert_eq!(cards, &BTreeSet::from([458, 459, 460]));
    }
    assert!(triggers.iter().all(|count| *count > 0));
    println!(
        "PASS: {completed} complete legal games, new units per seat {seen:?}, distinct finalized Disarm triggers {triggers:?}"
    );
}
