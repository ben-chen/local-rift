//! Separate legal-40-card match matrix; all earlier matrices/seeds stay unchanged.
mod support;
use support::conserved;

use local_rift::cards::CardId;
use local_rift::deck::{CustomDeck, eligible_cards};
use local_rift::game::{ChainAbility, Difficulty, Game, Phase};
use std::collections::BTreeSet;

fn combat_deck(legend: usize, card: CardId) -> CustomDeck {
    let mut deck = CustomDeck::from_legend(legend).unwrap();
    assert!(eligible_cards(legend).contains(&card));
    deck.name = format!("Combat card verification {legend}/{card}");
    if matches!((legend, card), (8, 411) | (9, 412)) {
        deck.chosen_champion = card;
    }
    if let Some((_, count)) = deck.cards.iter_mut().find(|(id, _)| *id == card) {
        *count = 3;
    } else {
        deck.cards.push((card, 3));
    }
    let mut excess = deck
        .cards
        .iter()
        .map(|(_, n)| usize::from(*n))
        .sum::<usize>()
        - 40;
    for (id, count) in &mut deck.cards {
        if *id == card {
            continue;
        }
        let min = u8::from(*id == deck.chosen_champion);
        let remove = usize::from(*count - min).min(excess);
        *count -= u8::try_from(remove).unwrap();
        excess -= remove;
    }
    assert_eq!(excess, 0);
    deck.cards.retain(|(_, count)| *count > 0);
    deck.validate().unwrap();
    deck
}

#[test]
fn combat_cards_complete_exact_forty_card_games_in_all_seven_legend_homes() {
    let mut seen: [BTreeSet<CardId>; 2] = std::array::from_fn(|_| BTreeSet::new());
    let mut triggers = [0usize; 2];
    let mut activations = [0usize; 2];
    let mut rengar_attack_entries = [0usize; 2];
    let mut matches = 0;
    let mut actions = 0;
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for starting in 0..2 {
            for (home_index, (card, legend)) in [
                (411, 3),
                (411, 4),
                (411, 8),
                (412, 3),
                (412, 5),
                (412, 6),
                (412, 9),
            ]
            .into_iter()
            .enumerate()
            {
                for seat in 0..2 {
                    let home = combat_deck(legend, card);
                    let opponent =
                        CustomDeck::from_legend(if card == 411 { 7 } else { 4 }).unwrap();
                    let decks = if seat == 0 {
                        [home, opponent]
                    } else {
                        [opponent, home]
                    };
                    let seed =
                        426000 + (level * 10000 + starting * 1000 + home_index * 10 + seat) as u64;
                    let mut game = Game::new_with_custom(
                        [0, 3],
                        [Some(decks[0].clone()), Some(decks[1].clone())],
                        seed,
                        difficulty,
                        false,
                    )
                    .unwrap();
                    game.choose_starting_player(starting).unwrap();
                    game.start_match().unwrap();
                    let mut seen_chain = BTreeSet::new();
                    for step in 0..6000 {
                        conserved(&game, &decks);
                        for (owner, p) in game.players.iter().enumerate() {
                            seen[owner].extend(
                                p.units
                                    .iter()
                                    .filter(|u| !u.token && (411..=412).contains(&u.card_id))
                                    .map(|u| u.card_id),
                            );
                            for u in &p.units {
                                let c = game.unit_combat_contribution(u).unwrap();
                                if u.card_id == 412 && !u.token {
                                    assert_eq!(c.damage, 0);
                                    assert!(c.prevents_combat_damage);
                                }
                            }
                        }
                        for item in &game.chain {
                            if !seen_chain.insert(item.id) {
                                continue;
                            }
                            if item.card_id != 412 {
                                continue;
                            }
                            match item.ability {
                                Some(ChainAbility::CardTrigger { .. }) => {
                                    triggers[item.player] += 1
                                }
                                Some(ChainAbility::Activated { .. }) => {
                                    activations[item.player] += 1
                                }
                                _ => {}
                            }
                        }
                        if game.winner.is_some() {
                            break;
                        }
                        let prior_units: BTreeSet<_> = game
                            .players
                            .iter()
                            .flat_map(|p| p.units.iter().map(|u| u.id))
                            .collect();
                        let attacking_field = game
                            .showdown
                            .as_ref()
                            .filter(|s| s.combat)
                            .map(|s| (s.attacker, s.lane));
                        assert!(game.step_ai().unwrap_or_else(|error|panic!("card{card}/legend{legend} seat{seat} {difficulty:?} starter{starting} seed{seed} step{step}: {error}; decision {:?}; log {:?}",game.decision,game.log.iter().rev().take(8).collect::<Vec<_>>())),"AI stopped before winner");
                        if let Some((owner, lane)) = attacking_field {
                            rengar_attack_entries[owner] += game.players[owner]
                                .units
                                .iter()
                                .filter(|u| {
                                    !u.token
                                        && u.card_id == 411
                                        && u.lane == Some(lane)
                                        && !prior_units.contains(&u.id)
                                })
                                .count();
                        }
                        actions += 1;
                    }
                    assert_eq!(
                        game.phase,
                        Phase::Finished,
                        "card{card}/legend{legend} seat{seat} {difficulty:?} starter{starting} seed{seed} stalled"
                    );
                    assert!(game.players[game.winner.unwrap()].score >= 8);
                    conserved(&game, &decks);
                    matches += 1;
                }
            }
        }
    }
    assert_eq!(matches, 84);
    for owner in 0..2 {
        assert_eq!(seen[owner], [411, 412].into_iter().collect());
        assert!(triggers[owner] > 0);
    }
    println!(
        "PASS: {matches} legal40 games / {actions} actions; played {seen:?}; Ezreal triggers {triggers:?}; self-Move activations {activations:?}; Rengar attacking-field plays {rengar_attack_entries:?}"
    );
}
