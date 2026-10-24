//! Complete legal constructed games exercise the object-cost/conversion family and conservation.
use local_rift::cards::CardId;
use local_rift::deck::{CustomDeck, eligible_cards};
use local_rift::game::{ChainAbility, Difficulty, Game, Phase, domain_index};
use local_rift::legend_catalog::legends;
use std::collections::{BTreeMap, BTreeSet};

fn expanded(legend: usize) -> CustomDeck {
    let mut deck = CustomDeck::from_legend(legend).unwrap();
    deck.name = format!("Conversion verification {legend}");
    for id in eligible_cards(legend)
        .into_iter()
        .filter(|id| (240..246).contains(id))
    {
        if let Some((_, copies)) = deck.cards.iter_mut().find(|(existing, _)| *existing == id) {
            *copies = 3;
        } else {
            deck.cards.push((id, 3));
        }
    }
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
fn object_cost_cards_finish_all_legend_pairings_levels_and_starting_seats() {
    let mut seen_cards = BTreeSet::new();
    let mut seen_abilities = BTreeSet::new();
    let mut matches = 0;
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for starting in 0..2 {
            for first in 0..legends().len() {
                for second in 0..legends().len() {
                    let decks = [expanded(first), expanded(second)];
                    // Preserve the original eight-legend seeds as the matrix grows.
                    let seed = 34000 + (level * 1000 + starting * 100 + first * 8 + second) as u64;
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
                    for step in 0..6000 {
                        conserved(&game, &decks);
                        for player in &game.players {
                            seen_cards.extend(
                                player.units.iter().filter(|u| !u.token).map(|u| u.card_id),
                            );
                            seen_cards
                                .extend(player.gear.iter().filter(|g| !g.token).map(|g| g.card_id));
                        }
                        for item in &game.chain {
                            if let Some(ChainAbility::Activated {
                                card_id,
                                ability_index,
                                ..
                            }) = item.ability
                                && (240..246).contains(&card_id)
                            {
                                seen_abilities.insert((card_id, ability_index));
                            }
                        }
                        if game.winner.is_some() {
                            break;
                        }
                        assert!(game.step_ai().unwrap_or_else(|error| panic!(
                            "legends{first}/{second} {difficulty:?} starter{starting} seed{seed} step{step}: {error}; recent {:?}",
                            game.log.iter().rev().take(8).collect::<Vec<_>>()
                        )), "AI stopped in an unfinished match");
                    }
                    assert_eq!(
                        game.phase,
                        Phase::Finished,
                        "legends{first}/{second} {difficulty:?} starter{starting} seed{seed} stalled"
                    );
                    let winner = game.winner.expect("finished game needs a winner");
                    assert!(game.players[winner].score >= 8);
                    conserved(&game, &decks);
                    matches += 1;
                }
            }
        }
    }
    assert_eq!(matches, legends().len() * legends().len() * 6);
    assert!(
        (240..246).all(|id| seen_cards.contains(&id)),
        "not all new cards entered play: {seen_cards:?}"
    );
    assert!(
        seen_abilities.len() >= 4,
        "too few distinct new abilities exercised: {seen_abilities:?}"
    );
    println!(
        "PASS: {matches} complete object-cost/conversion games; respondable abilities observed {seen_abilities:?}"
    );
}
