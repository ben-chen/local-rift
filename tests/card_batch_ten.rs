//! C8 source and deck-construction boundaries; runtime interaction tests are separate.
use local_rift::cards::{self, CardKind, Domain};
use local_rift::deck::{CustomDeck, eligible_cards, eligible_champions};
use local_rift::legend_catalog::legends;
use serde_json::{Value, json};
use std::collections::BTreeSet;

const IDS: [&str; 18] = [
    "ogs-022-024",
    "rad-059-167",
    "ogn-050-298",
    "ogn-004-298",
    "ogn-229-298",
    "unl-159-219",
    "ogs-012-024",
    "ogn-133-298",
    "rad-079-167",
    "ogs-002-024",
    "ogn-216-298",
    "unl-152-219",
    "ogn-137-298",
    "unl-062-219",
    "unl-154-219",
    "ogn-055-298",
    "ogn-015-298",
    "ogn-074-298",
];
const STATS: [(u8, u8, u8); 18] = [
    (8, 0, 0),
    (9, 1, 0),
    (2, 1, 0),
    (1, 0, 0),
    (4, 2, 0),
    (2, 1, 0),
    (6, 1, 0),
    (1, 0, 0),
    (5, 2, 0),
    (6, 1, 0),
    (2, 0, 1),
    (3, 0, 2),
    (7, 0, 6),
    (4, 0, 4),
    (3, 0, 3),
    (3, 0, 2),
    (4, 1, 5),
    (4, 1, 4),
];
fn source() -> Value {
    serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap()
}

#[test]
fn eighteen_appended_cards_use_exact_reviewed_effective_facts_and_offline_faces() {
    let cards = cards::cards();
    let source = source();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    let proof: Value = serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c8/admissions.json"
    ))
    .unwrap();
    assert!(cards.len() >= 308);
    assert_eq!(proof["records"].as_array().unwrap().len(), 18);
    for (offset, official_id) in IDS.iter().enumerate() {
        let id = 290 + offset;
        let card = &cards[id];
        let printing = source["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["official_id"] == *official_id)
            .unwrap();
        let identity = source["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["identity"] == *official_id)
            .unwrap();
        let effective = &printing["effective"];
        assert_eq!((card.id, card.official_id), (id, *official_id));
        assert_eq!((card.cost, card.power_cost, card.might), STATS[offset]);
        assert_eq!(identity["official_ids"], json!([official_id]));
        assert_eq!(identity["standard_legal"], true);
        assert_eq!(identity["errata"], json!([]));
        assert_eq!(identity["implementation"]["runtime_id"], id);
        assert_eq!(
            identity["source_review"]["unresolved_gameplay_fields"],
            json!([])
        );
        assert_eq!(
            identity["source_review"]["unresolved_variant_fields"],
            json!([])
        );
        assert_eq!(effective["types"], json!([card.kind.name().to_lowercase()]));
        assert_eq!(
            effective["domains"],
            json!(
                card.domains
                    .iter()
                    .map(|d| d.name().to_lowercase())
                    .collect::<Vec<_>>()
            )
        );
        assert_eq!(effective["tags"], json!(card.tags));
        assert_eq!(effective["rules_text"], card.official_text);
        assert_eq!(
            identity["effective_keywords_observed"],
            json!(card.keywords)
        );
        assert_eq!(manifest["cards"][id]["power"], card.power_cost);
        assert_eq!(
            manifest["cards"][id]["official_text_html"],
            effective["rules_html"]
        );
        assert_eq!(manifest["cards"][id]["art_url"], card.art_url);
        assert_eq!(proof["records"][offset]["runtime_id"], id);
        assert_eq!(proof["records"][offset]["effective"], *effective);
        let art = std::fs::read(format!(
            "{}/assets/art/card_{id}.png",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        assert!(art.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(art.len() < 300_000);
    }
    assert!(cards[290..300].iter().all(|c| c.kind == CardKind::Spell));
    assert!(cards[300..308].iter().all(|c| c.kind == CardKind::Unit));
}

#[test]
fn supernova_uses_reviewed_mind_spell_without_rewriting_the_raw_source() {
    let source = source();
    let printing = source["printings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["official_id"] == "rad-059-167")
        .unwrap();
    assert_eq!(printing["types"], json!(["unit"]));
    assert_eq!(printing["domains"], json!(["order"]));
    assert_eq!(printing["effective"]["types"], json!(["spell"]));
    assert_eq!(printing["effective"]["domains"], json!(["mind"]));
    let card = cards::cards()[291];
    assert_eq!(card.kind, CardKind::Spell);
    assert_eq!(card.domains, &[Domain::Mind]);
    assert_eq!(card.official_text, "Deal 10 to a unit. Draw 2.");
    for legend in legends() {
        assert_eq!(
            eligible_cards(legend.id).contains(&291),
            [3, 5, 6].contains(&legend.id)
        );
    }
}

#[test]
fn lux_signature_and_taric_champion_identity_do_not_leak_into_other_decks() {
    for legend in legends() {
        assert_eq!(eligible_cards(legend.id).contains(&290), legend.id == 6);
        assert_eq!(
            eligible_cards(legend.id).contains(&307),
            [0, 1].contains(&legend.id)
        );
        assert!(!eligible_champions(legend.id).contains(&307));
    }
    let mut lux = CustomDeck::from_legend(6).unwrap();
    lux.cards.push((290, 3));
    lux.validate().unwrap();
    assert_eq!(CustomDeck::from_json(&lux.to_json()).unwrap(), lux);
    let mut viktor = CustomDeck::from_legend(5).unwrap();
    viktor.cards.push((290, 1));
    assert!(
        viktor.validate().is_err(),
        "matching domains do not permit another champion's signature"
    );
    for legend in [0, 1] {
        let mut deck = CustomDeck::from_legend(legend).unwrap();
        deck.cards.push((307, 3));
        deck.validate().unwrap();
        deck.chosen_champion = 307;
        assert!(
            deck.validate().is_err(),
            "Taric does not match the deck's legend champion"
        );
    }
}

#[test]
fn every_new_identity_has_a_legal_custom_deck_home_without_changing_training_recipes() {
    let cards = cards::cards();
    let mut covered = BTreeSet::new();
    for legend in legends() {
        let original = CustomDeck::from_legend(legend.id).unwrap();
        assert_eq!(original.card_count(), 40);
        assert!(original.cards.iter().all(|(id, _)| *id < 290));
        let mut deck = original.clone();
        for id in eligible_cards(legend.id)
            .into_iter()
            .filter(|id| (290..308).contains(id))
        {
            assert!(cards[id].domains.iter().all(|d| legend.domains.contains(d)));
            deck.cards.push((id, 1));
            covered.insert(id);
        }
        deck.validate().unwrap();
        assert_eq!(deck.chosen_champion, original.chosen_champion);
        assert_eq!(CustomDeck::from_json(&deck.to_json()).unwrap(), deck);
    }
    assert_eq!(covered, (290..308).collect());
}
