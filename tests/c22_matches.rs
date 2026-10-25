//! Bounded granted-Vision complete games. Preserve every prior family seed.
//! Four legal Mind homes, two fixed hostile anchors, both seats and starters,
//! and all three AI levels: 4 * 2 * 2 * 2 * 3 = 96 complete games.
use local_rift::cards::CardId;
use local_rift::deck::{CustomDeck, eligible_cards};
use local_rift::empower::BoardObject;
use local_rift::game::{ChainAbility, Decision, Difficulty, Game, Phase, domain_index};
use local_rift::vision::VisionOrigin;
use std::collections::{BTreeMap, BTreeSet};

const MIND_HOMES: [usize; 4] = [3, 5, 6, 9];
const VISION_PACKAGE: [CardId; 4] = [138, 398, 399, 400];

fn vision_deck(legend: usize) -> CustomDeck {
    let mut deck = CustomDeck::from_legend(legend).unwrap();
    if MIND_HOMES.contains(&legend) {
        deck.name = format!("Granted Vision verification {legend}");
        let eligible = eligible_cards(legend);
        for id in VISION_PACKAGE {
            assert!(
                eligible.contains(&id),
                "{id} is not legal for legend {legend}"
            );
            if let Some((_, copies)) = deck.cards.iter_mut().find(|(existing, _)| *existing == id) {
                *copies = 3;
            } else {
                deck.cards.push((id, 3));
            }
        }
        // Replace old recipe cards, preserving the chosen champion and all
        // twelve package copies. The test never relies on oversized decks.
        let mut excess = deck
            .cards
            .iter()
            .map(|(_, count)| usize::from(*count))
            .sum::<usize>()
            - 40;
        for (id, count) in &mut deck.cards {
            if VISION_PACKAGE.contains(id) {
                continue;
            }
            let minimum = u8::from(*id == deck.chosen_champion);
            let remove = usize::from(*count - minimum).min(excess);
            *count -= u8::try_from(remove).unwrap();
            excess -= remove;
        }
        assert_eq!(excess, 0);
        deck.cards.retain(|(_, count)| *count > 0);
    }
    assert_eq!(
        deck.cards
            .iter()
            .map(|(_, count)| usize::from(*count))
            .sum::<usize>(),
        40
    );
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
fn granted_vision_finishes_exact_forty_card_games_and_resolves_separately_in_both_seats() {
    let mut seen_cards: [BTreeSet<CardId>; 2] = std::array::from_fn(|_| BTreeSet::new());
    let mut token_grant_seats = BTreeSet::new();
    let mut token_grant_origins = BTreeSet::new();
    let mut separate_resolution_seats = BTreeSet::new();
    let mut resolved_counts = [0usize; 2];
    let mut token_resolved_counts = [0usize; 2];
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
            for home in MIND_HOMES {
                for anchor in [1, 7] {
                    for seat in 0..2 {
                        let mut ids = [home, anchor];
                        if seat == 1 {
                            ids.swap(0, 1);
                        }
                        let decks = ids.map(vision_deck);
                        let seed = 321000
                            + (level * 10000 + starting * 1000 + home * 40 + anchor * 2 + seat)
                                as u64;
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
                        // Chain IDs distinguish separate responses/resolutions;
                        // physical bearer IDs distinguish one entry from another.
                        let mut resolved_by_bearer: BTreeMap<(usize, u32), BTreeSet<u32>> =
                            BTreeMap::new();
                        for step in 0..6000 {
                            conserved(&game, &decks);
                            for (owner, player) in game.players.iter().enumerate() {
                                seen_cards[owner].extend(
                                    player
                                        .units
                                        .iter()
                                        .filter(|unit| {
                                            !unit.token && (398..401).contains(&unit.card_id)
                                        })
                                        .map(|unit| unit.card_id),
                                );
                            }
                            if game.winner.is_some() {
                                break;
                            }
                            // A Vision opening a private Predict is direct proof
                            // of that individual ability resolving. Inspect only
                            // public source facts before the action, never cards.
                            let resolving = game
                                .decision
                                .is_none()
                                .then(|| game.chain.last())
                                .flatten()
                                .and_then(|item| match item.ability.as_ref() {
                                    Some(ChainAbility::Vision {
                                        source: BoardObject::Unit { player, unit_id },
                                        origin,
                                        ..
                                    }) => {
                                        let token = game.players[*player]
                                            .units
                                            .iter()
                                            .find(|unit| unit.id == *unit_id)
                                            .is_some_and(|unit| unit.token);
                                        Some((item.id, *player, *unit_id, *origin, token))
                                    }
                                    _ => None,
                                });
                            assert!(game.step_ai().unwrap_or_else(|error| panic!(
                                "home{home}/anchor{anchor} seat{seat} {difficulty:?} starter{starting} seed{seed} step{step}: {error}; recent {:?}",
                                game.log.iter().rev().take(8).collect::<Vec<_>>()
                            )), "AI stopped in an unfinished match");
                            actions += 1;
                            if let Some((chain_id, owner, bearer, origin, token)) = resolving
                                && matches!(game.decision, Some(Decision::Predict { player, .. }) if player == owner)
                                && !game.chain.iter().any(|item| item.id == chain_id)
                            {
                                let completed =
                                    resolved_by_bearer.entry((owner, bearer)).or_default();
                                assert!(completed.insert(chain_id), "a Vision was counted twice");
                                resolved_counts[owner] += 1;
                                if completed.len() >= 2 {
                                    separate_resolution_seats.insert(owner);
                                }
                                if token && let VisionOrigin::Granted { card_id, .. } = origin {
                                    token_grant_seats.insert(owner);
                                    token_grant_origins.insert(card_id);
                                    token_resolved_counts[owner] += 1;
                                }
                            }
                        }
                        assert_eq!(
                            game.phase,
                            Phase::Finished,
                            "home{home}/anchor{anchor} seat{seat} {difficulty:?} starter{starting} seed{seed} stalled"
                        );
                        let winner = game.winner.expect("finished game needs a winner");
                        assert!(game.players[winner].score >= 8);
                        conserved(&game, &decks);
                        matches += 1;
                    }
                }
            }
        }
    }
    assert_eq!(matches, 96);
    for (seat, cards) in seen_cards.iter().enumerate() {
        assert_eq!(
            cards,
            &BTreeSet::from([398, 399, 400]),
            "new units not all played in seat {seat}"
        );
    }
    assert_eq!(
        token_grant_seats,
        BTreeSet::from([0, 1]),
        "granted token Vision did not resolve from both seats"
    );
    assert!(
        token_grant_origins.contains(&399),
        "no actual Mech received Forecaster's Vision grant"
    );
    assert_eq!(
        separate_resolution_seats,
        BTreeSet::from([0, 1]),
        "multiple Vision instances did not resolve separately from both seats"
    );
    println!(
        "PASS: {matches} exact-40-card games / {actions} actions; new cards per seat {seen_cards:?}; individual Vision resolutions {resolved_counts:?}; granted token resolutions {token_resolved_counts:?}; token grant sources {token_grant_origins:?}; separate-resolution seats {separate_resolution_seats:?}"
    );
}
