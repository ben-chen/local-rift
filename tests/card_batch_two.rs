//! Second expansion: source-backed facts and real custom-deck integration.
//! Focused operation/timing tests live with the engine and spell-batch tests.
use local_rift::cards::{self, CardKind};
use local_rift::deck::{CustomDeck, eligible_cards, eligible_champions};
use local_rift::game::{Difficulty, Game};
use local_rift::rules::{self, OfficialEffect};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const OFFICIAL_IDS: [&str; 22] = [
    "ogn-093-298",
    "ogn-128-298",
    "unl-110-219",
    "unl-125-219",
    "ogn-206-298",
    "ogn-064-298",
    "ogs-015-024",
    "ogn-210-298",
    "sfd-156-221",
    "ogn-215-298",
    "ven-118-166",
    "sfd-037-221",
    "ogn-013-298",
    "ven-123-166",
    "ogn-001-298",
    "ogn-010-298",
    "unl-006-219",
    "unl-024-219",
    "ogn-016-298",
    "ogn-020-298",
    "ogn-217-298",
    "ogn-218-298",
];

fn expanded(legend: usize) -> CustomDeck {
    let mut deck = CustomDeck::from_preset(legend).unwrap();
    for id in eligible_cards(legend).into_iter().filter(|id| *id >= 81) {
        deck.cards.push((id, 3));
    }
    deck.name = format!("Second expansion {legend}");
    deck.validate().unwrap();
    deck
}

#[test]
fn append_only_cards_match_official_source_and_manifest() {
    let catalog = cards::cards();
    let inventory: Value =
        serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    assert!(catalog.len() >= 103);
    assert_eq!(catalog[80].official_id, "ogn-172-298");
    for (offset, official_id) in OFFICIAL_IDS.iter().enumerate() {
        let card = catalog[81 + offset];
        assert_eq!(card.id, 81 + offset);
        assert_eq!(card.official_id, *official_id);
        let printing = inventory["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["official_id"] == *official_id)
            .unwrap();
        let identity = inventory["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["identity"] == *official_id)
            .unwrap();
        assert_eq!(identity["standard_legal"], true);
        assert!(identity["errata"].as_array().unwrap().is_empty());
        assert!(
            identity["conflicting_fields"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(printing["name"], card.name);
        assert_eq!(printing["rules_text"], card.official_text);
        assert_eq!(printing["types"][0], card.kind.name().to_lowercase());
        assert_eq!(
            printing["energy"].as_u64().unwrap_or(0),
            u64::from(card.cost)
        );
        assert_eq!(
            printing["power"].as_u64().unwrap_or(0),
            u64::from(card.power_cost)
        );
        assert_eq!(
            printing["might"].as_u64().unwrap_or(0),
            u64::from(card.might)
        );
        assert_eq!(manifest["cards"][card.id]["official_id"], card.official_id);
        assert_eq!(
            manifest["cards"][card.id]["official_text"],
            card.official_text
        );
        assert_ne!(rules::effect(card.id), OfficialEffect::Unsupported);
    }
    assert_eq!(rules::effect(catalog.len()), OfficialEffect::Unsupported);
}

#[test]
fn custom_builder_exposes_only_compatible_new_cards_and_keeps_source_presets() {
    let catalog = cards::cards();
    let mut offered = BTreeSet::new();
    for (legend, preset) in cards::presets().iter().enumerate() {
        assert_eq!(
            preset
                .cards
                .iter()
                .map(|(_, n)| usize::from(*n))
                .sum::<usize>(),
            40
        );
        assert!(preset.cards.iter().all(|(id, _)| *id <= 68));
        let eligible = eligible_cards(legend);
        for card in &catalog[81..103] {
            let compatible = card
                .domains
                .iter()
                .all(|domain| preset.domains.contains(domain));
            assert_eq!(
                eligible.contains(&card.id),
                compatible,
                "{} / {}",
                preset.name,
                card.name
            );
            if compatible {
                offered.insert(card.id);
            }
        }
        expanded(legend).validate().unwrap();
    }
    assert_eq!(offered, (81..103).collect());
}

#[test]
fn rengar_is_a_valid_main_card_but_not_a_kaisa_chosen_champion() {
    let mut deck = expanded(3);
    assert_eq!(deck.chosen_champion, 49);
    assert!(deck.cards.contains(&(98, 3)));
    assert!(!eligible_champions(3).contains(&98));
    deck.chosen_champion = 98;
    assert!(deck.validate().is_err());
}

fn counts(ids: impl IntoIterator<Item = usize>) -> BTreeMap<usize, usize> {
    let mut counts = BTreeMap::new();
    for id in ids {
        *counts.entry(id).or_default() += 1;
    }
    counts
}

fn conservation(game: &Game, specs: &[CustomDeck; 2]) {
    for (owner, spec) in specs.iter().enumerate() {
        let p = &game.players[owner];
        let actual = counts(
            p.deck
                .iter()
                .chain(&p.hand)
                .chain(&p.discard)
                .chain(&p.banish)
                .copied()
                .chain(p.champion)
                .chain(p.units.iter().filter(|u| !u.token).map(|u| u.card_id))
                .chain(p.gear.iter().filter(|g| !g.token).map(|g| g.card_id))
                .chain(p.hidden.iter().map(|h| h.card_id))
                .chain(
                    game.chain
                        .iter()
                        .filter(|c| c.player == owner && c.ability.is_none())
                        .map(|c| c.card_id),
                )
                .chain(
                    game.pending_cards()
                        .iter()
                        .filter(|c| c.player == owner)
                        .map(|c| c.card_id),
                ),
        );
        let expected = counts(
            spec.cards
                .iter()
                .flat_map(|&(id, n)| std::iter::repeat_n(id, usize::from(n))),
        );
        assert_eq!(
            actual, expected,
            "Card conservation at turn {} seat {owner}",
            game.turn
        );
        assert_eq!(usize::from(p.total_runes()) + p.rune_deck.len(), 12);
        assert!(
            p.runes
                .iter()
                .zip(p.exhausted_runes)
                .all(|(&total, spent)| spent <= total)
        );
        assert_eq!(
            p.energy,
            p.energy_pool
                + p.runes
                    .iter()
                    .zip(p.exhausted_runes)
                    .map(|(&total, spent)| total - spent)
                    .sum::<u8>()
        );
    }
}

#[test]
fn new_cards_play_through_all_custom_pairings_with_conservation() {
    let mut observed = BTreeSet::new();
    for first in 0..4 {
        for second in 0..4 {
            for (difficulty, starting) in [(Difficulty::Easy, 0), (Difficulty::Tactical, 1)] {
                let specs = [expanded(first), expanded(second)];
                let seed = 2200 + (first * 4 + second) as u64;
                let mut game = Game::new_with_custom(
                    [first, second],
                    [Some(specs[0].clone()), Some(specs[1].clone())],
                    seed,
                    difficulty,
                    false,
                )
                .unwrap();
                game.choose_starting_player(starting).unwrap();
                game.start_match().unwrap();
                for step in 0..4000 {
                    conservation(&game, &specs);
                    observed.extend(
                        game.players
                            .iter()
                            .flat_map(|p| p.units.iter())
                            .filter(|u| !u.token && u.card_id >= 81)
                            .map(|u| u.card_id),
                    );
                    observed.extend(
                        game.chain
                            .iter()
                            .filter(|c| c.ability.is_none() && c.card_id >= 81)
                            .map(|c| c.card_id),
                    );
                    if game.winner.is_some() {
                        break;
                    }
                    assert!(game.step_ai().unwrap_or_else(|error| panic!(
                        "{first}/{second} {difficulty:?} seed{seed} step{step}: {error}"
                    )));
                }
                assert!(
                    game.winner.is_some(),
                    "{first}/{second} {difficulty:?} failed to finish, turn{} decision{:?}",
                    game.turn,
                    game.decision
                );
                conservation(&game, &specs);
            }
        }
    }
    let catalog = cards::cards();
    assert!(
        observed
            .iter()
            .any(|&id| catalog[id].kind == CardKind::Unit)
    );
    assert!(
        observed
            .iter()
            .any(|&id| catalog[id].kind == CardKind::Spell)
    );
}
