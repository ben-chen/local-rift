//! C9 source and deck-construction checks; runtime effect tests live separately.
use local_rift::cards::{self, CardKind, Domain};
use local_rift::deck::{CustomDeck, eligible_cards, eligible_champions};
use local_rift::legend_catalog::legends;
use serde_json::{Value, json};
use std::collections::BTreeSet;

const IDS: [&str; 26] = [
    "unl-180-219",
    "ogn-134-298",
    "ogn-138-298",
    "ogn-233-298",
    "rad-134-167",
    "ogn-123-298",
    "ogn-234-298",
    "sfd-158-221",
    "unl-067-219",
    "unl-027-219",
    "rad-049-167",
    "rad-009-167",
    "ogs-013-024",
    "ogn-065-298",
    "ogn-125-298",
    "sfd-159-221",
    "unl-077-219",
    "ogs-004-024",
    "unl-098-219",
    "rad-123-167",
    "unl-096-219",
    "sfd-118-221",
    "unl-039-219",
    "sfd-104-221",
    "sfd-038-221",
    "rad-003-167",
];
const STATS: [(u8, u8, u8); 26] = [
    (9, 3, 0),
    (2, 0, 0),
    (4, 0, 0),
    (6, 3, 0),
    (7, 1, 0),
    (7, 2, 0),
    (8, 2, 6),
    (5, 2, 6),
    (6, 1, 6),
    (8, 2, 8),
    (4, 0, 1),
    (6, 1, 3),
    (6, 1, 5),
    (4, 0, 4),
    (6, 0, 6),
    (2, 0, 2),
    (5, 0, 3),
    (5, 1, 4),
    (6, 0, 6),
    (2, 1, 1),
    (3, 0, 2),
    (3, 0, 2),
    (1, 0, 1),
    (2, 0, 0),
    (3, 0, 3),
    (3, 0, 3),
];
fn source() -> Value {
    serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap()
}

#[test]
fn appended_cards_match_exact_effective_facts_both_panels_and_offline_faces() {
    let cards = cards::cards();
    let source = source();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    let proof: Value = serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c9/admissions.json"
    ))
    .unwrap();
    assert!(cards.len() >= 334);
    assert_eq!(proof["records"].as_array().unwrap().len(), 26);
    for (offset, official_id) in IDS.iter().enumerate() {
        let id = 308 + offset;
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
            .find(|p| p["identity"] == *official_id)
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
        let mut text = effective["rules_text"].as_str().unwrap().to_owned();
        let panel = effective["equipment_text"].as_str().unwrap();
        if !panel.is_empty() {
            text += &format!(
                "\nAttached unit (+{} Might): {panel}",
                effective["might_bonus"].as_u64().unwrap_or(0)
            );
        }
        assert_eq!(text, card.official_text);
        assert_eq!(
            identity["effective_keywords_observed"],
            json!(card.keywords)
        );
        assert_eq!(
            manifest["cards"][id]["official_text_html"],
            effective["rules_html"]
        );
        assert_eq!(
            manifest["cards"][id]["equipment_effect_html"],
            effective["equipment_html"]
        );
        assert_eq!(manifest["cards"][id]["power"], card.power_cost);
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
    assert!(cards[308..314].iter().all(|c| c.kind == CardKind::Spell));
    assert!(cards[327..332].iter().all(|c| c.kind == CardKind::Gear));
}

#[test]
fn tracker_uses_reviewed_fury_and_bilgewater_without_rewriting_raw_facts() {
    let source = source();
    let printing = source["printings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["official_id"] == "rad-003-167")
        .unwrap();
    assert_eq!(printing["domains"], json!(["body"]));
    assert_eq!(printing["tags"], json!([]));
    assert_eq!(printing["effective"]["domains"], json!(["fury"]));
    assert_eq!(printing["effective"]["tags"], json!(["Bilgewater"]));
    let card = cards::cards()[333];
    assert_eq!(card.domains, &[Domain::Fury]);
    assert_eq!(card.tags, &["Bilgewater"]);
    assert_eq!(
        card.official_text,
        "When you play me, deal 1 to each of up to 2 units."
    );
    for legend in legends() {
        let legal = legend.domains.contains(&Domain::Fury);
        assert_eq!(eligible_cards(legend.id).contains(&333), legal);
        assert!(!eligible_champions(legend.id).contains(&333));
        let mut deck = CustomDeck::from_legend(legend.id).unwrap();
        deck.cards.push((333, 1));
        assert_eq!(deck.validate().is_ok(), legal, "legend {}", legend.id);
    }
    assert_eq!(source["counts"]["source_quality_issues"], 45);
    assert_eq!(source["counts"]["resolved_source_issue_groups"], 45);
}

#[test]
fn meditative_is_a_yi_champion_while_garen_is_only_an_order_main_card() {
    for legend in legends() {
        assert_eq!(eligible_champions(legend.id).contains(&325), legend.id == 1);
        assert!(!eligible_champions(legend.id).contains(&320));
        assert_eq!(
            eligible_cards(legend.id).contains(&320),
            legend.domains.contains(&Domain::Order)
        );
    }
    let mut yi = CustomDeck::from_legend(1).unwrap();
    yi.cards.push((325, 1));
    yi.chosen_champion = 325;
    yi.validate().unwrap();
    assert_eq!(CustomDeck::from_json(&yi.to_json()).unwrap(), yi);
    let mut darius = CustomDeck::from_legend(4).unwrap();
    darius.cards.push((320, 1));
    darius.validate().unwrap();
    darius.chosen_champion = 320;
    assert!(darius.validate().is_err());
}

#[test]
fn all_twenty_six_have_legal_homes_without_changing_existing_training_lists() {
    let mut covered = BTreeSet::new();
    for legend in legends() {
        let original = CustomDeck::from_legend(legend.id).unwrap();
        assert_eq!(original.card_count(), 40);
        if legend.id < 8 {
            // This checkpoint preserves the original eight training recipes.
            assert!(original.cards.iter().all(|(id, _)| *id < 290));
        }
        let mut expanded = original.clone();
        for id in eligible_cards(legend.id)
            .into_iter()
            .filter(|id| (308..334).contains(id))
        {
            expanded.cards.push((id, 1));
            covered.insert(id);
        }
        expanded.validate().unwrap();
        assert_eq!(expanded.chosen_champion, original.chosen_champion);
        assert_eq!(
            CustomDeck::from_json(&expanded.to_json()).unwrap(),
            expanded
        );
    }
    assert_eq!(covered, (308..334).collect());
    assert!(
        cards::presets()
            .iter()
            .all(|d| d.cards.iter().all(|(id, _)| *id < 69))
    );
}
