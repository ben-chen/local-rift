//! Third expansion: source facts, deck eligibility and conserved custom matches.
use local_rift::cards::{self, CardKind};
use local_rift::deck::{CustomDeck, eligible_cards, eligible_champions};
use local_rift::game::{Difficulty, Game};
use local_rift::rules::{self, OfficialEffect};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
const IDS: [&str; 49] = [
    "sfd-006-221",
    "ogs-007-024",
    "sfd-096-221",
    "ogs-009-024",
    "unl-036-219",
    "ogn-135-298",
    "ogn-241-298",
    "ogn-052-298",
    "rad-088-167",
    "ogn-054-298",
    "unl-099-219",
    "ogs-016-024",
    "rad-077-167",
    "ogs-005-024",
    "sfd-161-221",
    "sfd-095-221",
    "sfd-056-221",
    "sfd-064-221",
    "sfd-033-221",
    "sfd-102-221",
    "sfd-009-221",
    "rad-071-167",
    "ogn-081-298",
    "ogn-120-298",
    "ogn-040-298",
    "ogn-163-298",
    "ogn-245-298",
    "ogn-098-298",
    "sfd-012-221",
    "ogn-083-298",
    "unl-061-219",
    "ven-049-166",
    "sfd-034-221",
    "sfd-066-221",
    "sfd-151-221",
    "ven-051-166",
    "ven-048-166",
    "unl-123-219",
    "ven-026-166",
    "ogn-082-298",
    "rad-008-167",
    "ogn-092-298",
    "rad-058-167",
    "ogn-097-298",
    "ogn-211-298",
    "unl-033-219",
    "sfd-155-221",
    "rad-119-167",
    "sfd-174-221",
];

fn expanded(legend: usize) -> CustomDeck {
    let mut deck = CustomDeck::from_preset(legend).unwrap();
    for id in eligible_cards(legend)
        .into_iter()
        .filter(|id| (103..152).contains(id))
    {
        deck.cards.push((id, 3));
    }
    deck.name = format!("Third expansion {legend}");
    deck.validate().unwrap();
    deck
}

#[test]
fn new_card_facts_match_both_official_panels_and_stable_identity() {
    let cards = cards::cards();
    let inventory: Value =
        serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    assert!(cards.len() >= 152);
    for (offset, official_id) in IDS.into_iter().enumerate() {
        let card = cards[103 + offset];
        assert_eq!(card.id, 103 + offset);
        assert_eq!(card.official_id, official_id);
        let p = inventory["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["official_id"] == official_id)
            .unwrap();
        let identity = inventory["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["identity"] == official_id)
            .unwrap();
        assert_eq!(identity["standard_legal"], true);
        assert!(
            identity["conflicting_fields"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(identity["errata"].as_array().unwrap().is_empty());
        assert_eq!(p["name"], card.name);
        assert_eq!(p["types"][0], card.kind.name().to_lowercase());
        assert_eq!(p["energy"].as_u64().unwrap_or(0), u64::from(card.cost));
        assert_eq!(p["power"].as_u64().unwrap_or(0), u64::from(card.power_cost));
        assert_eq!(p["might"].as_u64().unwrap_or(0), u64::from(card.might));
        let mut text = p["rules_text"].as_str().unwrap().to_owned();
        let panel = p["equipment_text"].as_str().unwrap();
        if !panel.is_empty() {
            text += &format!(
                "\nAttached unit (+{} Might): {panel}",
                p["might_bonus"].as_u64().unwrap_or(0)
            );
        }
        assert_eq!(card.official_text, text);
        assert_eq!(manifest["cards"][card.id]["official_text"], text);
        assert_ne!(rules::effect(card.id), OfficialEffect::Unsupported);
        let bytes = std::fs::read(format!(
            "{}/assets/art/card_{}.png",
            env!("CARGO_MANIFEST_DIR"),
            card.id
        ))
        .unwrap();
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(bytes.len() < 300_000);
    }
    assert_eq!(rules::effect(cards.len()), OfficialEffect::Unsupported);
}

#[test]
fn all_new_cards_have_legal_deck_paths_and_yi_has_an_alternative_chosen_champion() {
    let catalog = cards::cards();
    let mut offered = BTreeSet::new();
    for legend in 0..4 {
        let preset = &cards::presets()[legend];
        let eligible = eligible_cards(legend);
        for card in &catalog[103..152] {
            let compatible = card.domains.iter().all(|d| preset.domains.contains(d));
            assert_eq!(eligible.contains(&card.id), compatible);
            if compatible {
                offered.insert(card.id);
            }
        }
        expanded(legend).validate().unwrap();
        assert!(preset.cards.iter().all(|(id, _)| *id <= 68));
    }
    assert_eq!(offered, (103..152).collect());
    assert!(eligible_champions(1).contains(&106));
    let mut yi = expanded(1);
    yi.chosen_champion = 106;
    yi.validate().unwrap();
    assert!(!eligible_champions(1).contains(&104));
    assert!(!eligible_champions(2).contains(&109));
}

fn count(ids: impl IntoIterator<Item = usize>) -> BTreeMap<usize, usize> {
    let mut out = BTreeMap::new();
    for id in ids {
        *out.entry(id).or_default() += 1;
    }
    out
}
fn conserved(game: &Game, specs: &[CustomDeck; 2]) {
    let pending = game.pending_cards();
    for (owner, spec) in specs.iter().enumerate() {
        let p = &game.players[owner];
        let actual = count(
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
                    pending
                        .iter()
                        .filter(|c| c.player == owner)
                        .map(|c| c.card_id),
                ),
        );
        let expected = count(
            spec.cards
                .iter()
                .flat_map(|&(id, n)| std::iter::repeat_n(id, usize::from(n))),
        );
        assert_eq!(actual, expected, "Seat {owner}, turn{}", game.turn);
        assert_eq!(usize::from(p.total_runes()) + p.rune_deck.len(), 12);
        assert_eq!(
            p.energy,
            p.energy_pool
                + p.runes
                    .iter()
                    .zip(p.exhausted_runes)
                    .map(|(&all, spent)| all - spent)
                    .sum::<u8>()
        );
    }
}

#[test]
fn forty_nine_card_expansion_plays_full_custom_matches_without_losing_cards() {
    let mut observed = BTreeSet::new();
    for first in 0..4 {
        for second in 0..4 {
            let specs = [expanded(first), expanded(second)];
            let mut game = Game::new_with_custom(
                [first, second],
                [Some(specs[0].clone()), Some(specs[1].clone())],
                4900 + (first * 4 + second) as u64,
                Difficulty::Tactical,
                false,
            )
            .unwrap();
            game.choose_starting_player((first + second) % 2).unwrap();
            game.start_match().unwrap();
            for step in 0..6000 {
                conserved(&game, &specs);
                observed.extend(
                    game.chain
                        .iter()
                        .filter(|c| c.ability.is_none() && (103..152).contains(&c.card_id))
                        .map(|c| c.card_id),
                );
                observed.extend(
                    game.players
                        .iter()
                        .flat_map(|p| &p.units)
                        .filter(|u| !u.token && (103..152).contains(&u.card_id))
                        .map(|u| u.card_id),
                );
                observed.extend(
                    game.players
                        .iter()
                        .flat_map(|p| &p.gear)
                        .filter(|g| !g.token && (103..152).contains(&g.card_id))
                        .map(|g| g.card_id),
                );
                if game.winner.is_some() {
                    break;
                }
                assert!(
                    game.step_ai()
                        .unwrap_or_else(|error| panic!("{first}/{second} step{step}: {error}"))
                );
            }
            assert!(
                game.winner.is_some(),
                "{first}/{second}, turn{}, decision{:?}",
                game.turn,
                game.decision
            );
            conserved(&game, &specs);
        }
    }
    let catalog = cards::cards();
    for kind in [CardKind::Unit, CardKind::Spell, CardKind::Gear] {
        assert!(observed.iter().any(|&id| catalog[id].kind == kind));
    }
}
