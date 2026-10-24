//! Source facts and construction coverage; these checks are not interaction proof.
use local_rift::battlefields::{BATTLEFIELDS, default_battlefield};
use local_rift::cards::{self, CardKind};
use local_rift::deck::{CustomDeck, eligible_cards};
use local_rift::legend_catalog::legends;
use serde_json::{Value, json};
use std::collections::BTreeSet;

const MAIN_IDS: [&str; 18] = [
    "ogn-021-298",
    "ogn-032-298",
    "sfd-078-221",
    "ogn-031-298",
    "ven-044-166",
    "ogs-001-024",
    "ven-010-166",
    "unl-052-219",
    "sfd-004-221",
    "ogn-129-298",
    "ogn-011-298",
    "sfd-029-221",
    "unl-146-219",
    "ven-055-166",
    "ven-045-166",
    "ven-098-166",
    "sfd-146-221",
    "sfd-141-221",
];
const FIELD_IDS: [&str; 7] = [
    "unl-216-219",
    "rad-162-167",
    "sfd-213-221",
    "rad-163-167",
    "ven-161-166",
    "ven-163-166",
    "unl-219-219",
];
fn inventory() -> Value {
    serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap()
}
fn art(name: &str) {
    let path = format!("{}/assets/art/{name}.png", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(bytes.len() < 300_000);
}

#[test]
fn eighteen_cards_preserve_exact_clean_printing_facts_and_offline_art() {
    let catalog = cards::cards();
    let data = inventory();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    assert!(catalog.len() >= 272);
    for (offset, official_id) in MAIN_IDS.iter().enumerate() {
        let card = catalog[254 + offset];
        let printing = data["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["official_id"] == *official_id)
            .unwrap();
        let identity = data["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["identity"] == *official_id)
            .unwrap();
        assert_eq!((card.id, card.official_id), (254 + offset, *official_id));
        assert_eq!(identity["standard_legal"], true);
        assert!(
            identity["conflicting_fields"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(printing["type_labels_conflict"], false);
        assert_eq!(identity["implementation"]["runtime_id"], card.id);
        assert_eq!(printing["name"], card.name);
        assert_eq!(printing["types"], json!([card.kind.name().to_lowercase()]));
        for (key, actual) in [
            ("energy", card.cost),
            ("power", card.power_cost),
            ("might", card.might),
        ] {
            assert_eq!(printing[key].as_u64().unwrap_or(0), u64::from(actual));
        }
        assert_eq!(
            printing["domains"],
            json!(
                card.domains
                    .iter()
                    .map(|d| d.name().to_lowercase())
                    .collect::<Vec<_>>()
            )
        );
        assert_eq!(printing["tags"], json!(card.tags));
        assert_eq!(identity["keywords_observed"], json!(card.keywords));
        assert_eq!(printing["equipment_text"], "");
        assert_eq!(
            manifest["cards"][card.id]["official_text"],
            card.official_text
        );
        assert_eq!(
            manifest["cards"][card.id]["official_text_html"],
            printing["rules_html"]
        );
        assert_eq!(manifest["cards"][card.id]["art_url"], card.art_url);
        assert_eq!(
            card.art_url.split('?').next(),
            printing["image_url"].as_str().unwrap().split('?').next()
        );
        if card.id != 258 {
            assert_eq!(printing["rules_text"], card.official_text);
        }
        if !matches!(card.id, 255 | 258) {
            assert!(identity["errata"].as_array().unwrap().is_empty());
        }
        art(&format!("card_{}", card.id));
    }
    assert_eq!(catalog[259].kind, CardKind::Unit);
    assert!(catalog[259].tags.contains(&"Annie"));
    assert_eq!(
        (
            catalog[265].cost,
            catalog[265].power_cost,
            catalog[265].might
        ),
        (3, 0, 3)
    );
}

#[test]
fn errata_changes_text_without_overwriting_original_gallery_evidence() {
    let cards = cards::cards();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    assert_eq!(
        cards[258].official_text,
        "When you play your first card each turn, if I'm at a battlefield, the next card you play this turn costs [2 Energy][Any Power][Any Power] less."
    );
    assert!(
        manifest["cards"][258]["official_text_html"]
            .as_str()
            .unwrap()
            .contains("your next card costs")
    );
    assert!(
        manifest["cards"][258]["errata_source"]
            .as_str()
            .unwrap()
            .ends_with("vendetta-errata-updates/")
    );
    assert!(
        manifest["cards"][255]["errata_source"]
            .as_str()
            .unwrap()
            .ends_with("riftbound-origins-card-errata/")
    );
    assert!(
        cards[255]
            .official_text
            .contains("next spell you play this turn")
    );
}

#[test]
fn eleven_fields_keep_source_identity_and_original_preset_defaults() {
    let data = inventory();
    let manifest: Value =
        serde_json::from_str(include_str!("../assets/data/battlefields.json")).unwrap();
    assert!(BATTLEFIELDS.len() >= 11);
    for (offset, official_id) in FIELD_IDS.iter().enumerate() {
        let field = &BATTLEFIELDS[4 + offset];
        let printing = data["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["official_id"] == *official_id)
            .unwrap();
        let identity = data["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["identity"] == *official_id)
            .unwrap();
        assert_eq!((field.id, field.source_id), (4 + offset, *official_id));
        assert_eq!(printing["types"], json!(["battlefield"]));
        assert_eq!(printing["domains"], json!(["colorless"]));
        assert_eq!(printing["name"], field.name);
        assert_eq!(printing["rules_text"], field.official_text);
        assert_eq!(
            manifest["battlefields"][field.id]["official_html"],
            printing["rules_html"]
        );
        assert_eq!(identity["standard_legal"], true);
        assert!(
            identity["conflicting_fields"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(identity["errata"].as_array().unwrap().is_empty());
        assert_eq!(identity["implementation"]["runtime_id"], field.id);
    }
    for id in 0..11 {
        art(&format!("battlefield_{id}"));
    }
    for (index, preset) in cards::presets().iter().enumerate() {
        assert_eq!(default_battlefield(index), index);
        assert_eq!(CustomDeck::from_preset(index).unwrap().cards, preset.cards);
    }
    for id in 4..7 {
        assert_eq!(default_battlefield(id), 3);
    }
}

#[test]
fn all_new_cards_have_real_legal_homes_and_all_fields_are_selectable() {
    let mut covered = BTreeSet::new();
    for legend in legends() {
        let eligible = eligible_cards(legend.id);
        let mut custom = CustomDeck::from_legend(legend.id).unwrap();
        for id in 254..272 {
            if eligible.contains(&id) {
                covered.insert(id);
                custom.cards.push((id, 1));
            }
        }
        for field in 0..11 {
            custom.battlefield = field;
            custom.validate().unwrap();
            assert_eq!(CustomDeck::from_json(&custom.to_json()).unwrap(), custom);
        }
    }
    assert_eq!(covered, (254..272).collect());
    assert_eq!(cards::presets().len(), 4);
    assert_eq!(legends().len(), 7);
}
