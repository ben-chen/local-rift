//! Separate legal-40-card match matrix; all earlier matrices/seeds stay unchanged.
use local_rift::cards::CardId;
use local_rift::deck::{CustomDeck, eligible_cards};
use local_rift::game::{ChainAbility, Difficulty, Game, Phase, domain_index};
use std::collections::{BTreeMap, BTreeSet};

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

fn multiset(cards: impl IntoIterator<Item = CardId>) -> BTreeMap<CardId, usize> {
    let mut out = BTreeMap::new();
    for card in cards {
        *out.entry(card).or_default() += 1;
    }
    out
}

fn conserved(game: &Game, decks: &[CustomDeck; 2]) {
    let mut objects = BTreeSet::new();
    for (owner, spec) in decks.iter().enumerate() {
        let player = &game.players[owner];
        let expected = multiset(
            spec.cards
                .iter()
                .flat_map(|&(id, copies)| std::iter::repeat_n(id, usize::from(copies))),
        );
        let actual = multiset(
            player
                .deck
                .iter()
                .chain(&player.hand)
                .chain(&player.discard)
                .chain(&player.banish)
                .copied()
                .chain(player.champion)
                .chain(game.payment_plan_view(owner).and_then(|plan| {
                    (plan.player == owner)
                        .then_some(plan.pending_card)
                        .flatten()
                }))
                .chain(
                    player
                        .units
                        .iter()
                        .filter(|unit| !unit.token)
                        .map(|unit| unit.card_id),
                )
                .chain(
                    player
                        .gear
                        .iter()
                        .filter(|gear| !gear.token)
                        .map(|gear| gear.card_id),
                )
                .chain(player.hidden.iter().map(|card| card.card_id))
                .chain(
                    game.pending_cards()
                        .iter()
                        .filter(|card| card.player == owner)
                        .map(|card| card.card_id),
                )
                .chain(
                    game.chain
                        .iter()
                        .filter(|item| item.player == owner && item.ability.is_none())
                        .map(|item| item.card_id),
                ),
        );
        assert_eq!(
            actual, expected,
            "physical cards changed: turn {} seat {owner}",
            game.turn
        );
        let mut rune_totals = player.runes.map(u16::from);
        for &domain in &player.rune_deck {
            rune_totals[domain_index(domain)] += 1;
        }
        assert_eq!(rune_totals, spec.runes.map(u16::from));
        assert!(
            player
                .runes
                .iter()
                .zip(player.exhausted_runes)
                .all(|(&all, used)| used <= all)
        );
        let ready_energy: u16 = player
            .runes
            .iter()
            .zip(player.exhausted_runes)
            .map(|(&all, used)| u16::from(all - used))
            .sum();
        assert_eq!(player.energy, ready_energy + player.energy_pool);
        for unit in &player.units {
            assert!(objects.insert(unit.id), "duplicate object ID {}", unit.id);
            assert!(unit.lane.is_none_or(|lane| lane < 2));
        }
        for gear in &player.gear {
            assert!(objects.insert(gear.id), "duplicate object ID {}", gear.id);
            assert!(
                gear.attached_to
                    .is_none_or(|id| player.units.iter().any(|unit| unit.id == id))
            );
        }
    }
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
