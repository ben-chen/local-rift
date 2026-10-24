//! Fourth expansion: source facts, deck eligibility and conserved custom matches.
use local_rift::cards::{self, CardKind};
use local_rift::deck::{CustomDeck, eligible_cards};
use local_rift::game::{Difficulty, Game};
use local_rift::rules::{self, OfficialEffect};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
const IDS: [&str; 78] = [
    "unl-092-219",
    "unl-100-219",
    "unl-034-219",
    "unl-157-219",
    "ogn-091-298",
    "ven-071-166",
    "sfd-047-221",
    "unl-129-219",
    "sfd-152-221",
    "sfd-069-221",
    "sfd-130-221",
    "sfd-121-221",
    "ogn-167-298",
    "sfd-062-221",
    "ven-020-166",
    "ogn-130-298",
    "ogn-119-298",
    "sfd-137-221",
    "ven-121-166",
    "ogn-178-298",
    "ogn-003-298",
    "unl-149-219",
    "ogn-139-298",
    "ogn-117-298",
    "ogn-246-298",
    "ogn-239-298",
    "sfd-157-221",
    "ogn-222-298",
    "sfd-179-221",
    "sfd-100-221",
    "ogs-006-024",
    "ogn-027-298",
    "rad-030-167",
    "ogn-197-298",
    "rad-099-167",
    "unl-130-219",
    "sfd-138-221",
    "ogn-188-298",
    "sfd-132-221",
    "ogn-149-298",
    "ogn-051-298",
    "ogn-171-298",
    "rad-053-167",
    "unl-074-219",
    "ogn-143-298",
    "ogn-060-298",
    "ogn-228-298",
    "sfd-172-221",
    "sfd-124-221",
    "sfd-134-221",
    "sfd-086-221",
    "sfd-153-221",
    "sfd-108-221",
    "ven-011-166",
    "sfd-016-221",
    "sfd-115-221",
    "ogn-066-298",
    "ven-138-166",
    "ven-042-166",
    "unl-043-219",
    "unl-068-219",
    "rad-036-167",
    "ogn-118-298",
    "sfd-075-221",
    "unl-009-219",
    "sfd-114-221",
    "ogs-008-024",
    "sfd-070-221",
    "sfd-031-221",
    "ven-072-166",
    "unl-014-219",
    "sfd-017-221",
    "unl-015-219",
    "sfd-106-221",
    "ogn-014-298",
    "ven-059-166",
    "sfd-001-221",
    "rad-048-167",
];

fn expanded(legend: usize) -> CustomDeck {
    let mut deck = CustomDeck::from_preset(legend).unwrap();
    for id in eligible_cards(legend)
        .into_iter()
        .filter(|id| (152..230).contains(id))
    {
        deck.cards.push((id, 3));
    }
    deck.name = format!("Fourth expansion {legend}");
    deck.validate().unwrap();
    deck
}

#[test]
fn new_card_facts_match_both_official_panels_and_stable_identity() {
    let cards = cards::cards();
    let inventory: Value =
        serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    assert!(cards.len() >= 230);
    for (offset, official_id) in IDS.into_iter().enumerate() {
        let card = cards[152 + offset];
        assert_eq!(card.id, 152 + offset);
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
    // Tags affect gameplay predicates, so compare reviewed effective facts.
    // Raw feed preservation and Kennen's missing Ionia tag are pinned separately
    // in catalog_review; tag order is not a gameplay distinction.
    for card in &cards {
        let printing = inventory["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["official_id"] == card.official_id)
            .unwrap();
        let tags: BTreeSet<_> = printing["effective"]["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tag| tag.as_str().unwrap())
            .collect();
        let runtime_tags: BTreeSet<_> = card.tags.iter().copied().collect();
        assert_eq!(
            runtime_tags.len(),
            card.tags.len(),
            "duplicate runtime tags"
        );
        assert_eq!(runtime_tags, tags, "{}", card.name);
    }
}

#[test]
fn all_new_cards_have_legal_deck_paths_without_altering_sourced_presets() {
    let catalog = cards::cards();
    let mut offered = BTreeSet::new();
    for legend in 0..4 {
        let preset = &cards::presets()[legend];
        let eligible = eligible_cards(legend);
        for card in &catalog[152..230] {
            let compatible = card.domains.iter().all(|d| preset.domains.contains(d));
            assert_eq!(eligible.contains(&card.id), compatible);
            if compatible {
                offered.insert(card.id);
            }
        }
        expanded(legend).validate().unwrap();
        assert!(preset.cards.iter().all(|(id, _)| *id <= 68));
    }
    assert_eq!(offered, (152..230).collect());
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
                .chain(game.payment_plan_view(owner).and_then(|plan| {
                    (plan.player == owner)
                        .then_some(plan.pending_card)
                        .flatten()
                }))
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
                    .map(|(&all, spent)| u16::from(all - spent))
                    .sum::<u16>()
        );
    }
}

#[test]
fn seventy_eight_card_expansion_plays_full_custom_matches_without_losing_cards() {
    let mut observed = BTreeSet::new();
    for first in 0..4 {
        for second in 0..4 {
            let specs = [expanded(first), expanded(second)];
            let mut game = Game::new_with_custom(
                [first, second],
                [Some(specs[0].clone()), Some(specs[1].clone())],
                7800 + (first * 4 + second) as u64,
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
                        .filter(|c| c.ability.is_none() && (152..230).contains(&c.card_id))
                        .map(|c| c.card_id),
                );
                observed.extend(
                    game.players
                        .iter()
                        .flat_map(|p| &p.units)
                        .filter(|u| !u.token && (152..230).contains(&u.card_id))
                        .map(|u| u.card_id),
                );
                observed.extend(
                    game.players
                        .iter()
                        .flat_map(|p| &p.gear)
                        .filter(|g| !g.token && (152..230).contains(&g.card_id))
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
