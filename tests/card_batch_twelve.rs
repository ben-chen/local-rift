//! C10 first-slice source and construction checks; execution is tested separately.
use local_rift::cards::{self, CardKind, Domain};
use local_rift::deck::{CustomDeck, eligible_cards, eligible_champions};
use local_rift::legend_catalog::legends;
use serde_json::{Value, json};
use std::collections::BTreeSet;

const IDS: [&str; 11] = [
    "unl-151-219",
    "unl-031-219",
    "unl-091-219",
    "unl-094-219",
    "unl-075-219",
    "unl-047-219",
    "unl-016-219",
    "unl-040-219",
    "unl-108-219",
    "unl-127-219",
    "unl-115-219",
];
const STATS: [(u8, u8, u8); 11] = [
    (4, 1, 5),
    (1, 0, 0),
    (5, 0, 0),
    (2, 0, 2),
    (3, 1, 3),
    (3, 1, 3),
    (3, 1, 3),
    (2, 0, 2),
    (4, 0, 4),
    (2, 0, 1),
    (3, 1, 4),
];

#[test]
fn eleven_source_identities_match_exact_faces_and_runtime_metadata() {
    let cards = cards::cards();
    let catalog: Value =
        serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    let proof: Value = serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c10/admissions.json"
    ))
    .unwrap();
    assert!(cards.len() >= 345);
    assert_eq!(proof["records"].as_array().unwrap().len(), 11);
    for (offset, official_id) in IDS.iter().enumerate() {
        let id = 334 + offset;
        let card = cards[id];
        let printing = catalog["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["official_id"] == *official_id)
            .unwrap();
        let identity = catalog["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["identity"] == *official_id)
            .unwrap();
        let facts = &printing["effective"];
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
        assert_eq!(printing["resolution_ids"], json!([]));
        assert_eq!(facts["types"], json!([card.kind.name().to_lowercase()]));
        assert_eq!(
            facts["domains"],
            json!(
                card.domains
                    .iter()
                    .map(|d| d.name().to_lowercase())
                    .collect::<Vec<_>>()
            )
        );
        assert_eq!(facts["tags"], json!(card.tags));
        assert_eq!(facts["rules_text"], card.official_text);
        assert_eq!(facts["equipment_text"], "");
        assert_eq!(
            identity["effective_keywords_observed"],
            json!(card.keywords)
        );
        assert_eq!(
            manifest["cards"][id]["official_text_html"],
            facts["rules_html"]
        );
        assert_eq!(manifest["cards"][id]["power"], card.power_cost);
        assert_eq!(manifest["cards"][id]["art_url"], card.art_url);
        assert_eq!(proof["records"][offset]["runtime_id"], id);
        assert_eq!(proof["records"][offset]["effective"], *facts);
        let art = std::fs::read(format!(
            "{}/assets/art/card_{id}.png",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        assert!(art.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(art.len() < 300_000);
    }
    assert_eq!(
        cards[334..345]
            .iter()
            .filter(|c| c.kind == CardKind::Unit)
            .count(),
        9
    );
    assert_eq!(
        cards[334..345]
            .iter()
            .filter(|c| c.kind == CardKind::Spell)
            .count(),
        2
    );
}

#[test]
fn all_eleven_have_legal_homes_without_changing_recipes_or_published_lists() {
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
            .filter(|id| (334..345).contains(id))
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
    assert_eq!(covered, (334..345).collect());
    assert!(
        cards::presets()
            .iter()
            .all(|d| d.cards.iter().all(|(id, _)| *id < 69))
    );
}

#[test]
fn nilah_remains_main_deck_only_until_a_matching_legend_exists() {
    let card = cards::cards()[344];
    assert_eq!(card.tags, &["Nilah", "Kathkan", "Demon"]);
    assert_eq!(card.domains, &[Domain::Body]);
    for legend in legends() {
        let compatible = legend.domains.contains(&Domain::Body);
        assert_eq!(eligible_cards(legend.id).contains(&344), compatible);
        assert!(!eligible_champions(legend.id).contains(&344));
        let mut deck = CustomDeck::from_legend(legend.id).unwrap();
        deck.cards.push((344, 1));
        assert_eq!(deck.validate().is_ok(), compatible);
        deck.chosen_champion = 344;
        assert!(deck.validate().is_err());
    }
}
