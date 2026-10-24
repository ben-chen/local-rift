//! C5 source identities and authored deck boundaries; not interaction conformance.
use local_rift::cards::{self, CardKind, Domain};
use local_rift::deck::{CustomDeck, eligible_cards, eligible_champions};
use local_rift::legend_catalog::{legend, legends};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const IDS: [&str; 18] = [
    "ven-014-166",
    "ven-043-166",
    "ven-030-166",
    "ven-093-166",
    "ven-070-166",
    "ven-122-166",
    "ven-032-166",
    "ven-050-166",
    "ven-001-166",
    "ven-018-166",
    "ven-130-166",
    "ven-047-166",
    "ven-057-166",
    "ven-128-166",
    "ven-078-166",
    "ven-046-166",
    "ven-021-166",
    "ven-136-166",
];

fn source() -> Value {
    serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap()
}

fn art(name: &str) {
    let data = std::fs::read(format!(
        "{}/assets/art/{name}.png",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    assert!(data.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(data.len() < 300_000);
}

#[test]
fn eighteen_appended_cards_preserve_clean_source_facts_and_art() {
    let cards = cards::cards();
    let source = source();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    assert!(cards.len() >= 290);
    for (offset, official_id) in IDS.iter().enumerate() {
        let card = &cards[272 + offset];
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
        assert_eq!((card.id, card.official_id), (272 + offset, *official_id));
        assert_eq!(identity["standard_legal"], true);
        assert_eq!(identity["conflicting_fields"], json!([]));
        assert_eq!(identity["errata"], json!([]));
        assert_eq!(identity["implementation"]["runtime_id"], card.id);
        assert_eq!(printing["type_labels_conflict"], false);
        assert_eq!(printing["name"], card.name);
        assert_eq!(printing["types"], json!([card.kind.name().to_lowercase()]));
        for (key, number) in [
            ("energy", card.cost),
            ("power", card.power_cost),
            ("might", card.might),
        ] {
            assert_eq!(printing[key].as_u64().unwrap_or(0), u64::from(number));
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
        assert_eq!(printing["rules_text"], card.official_text);
        assert_eq!(printing["equipment_text"], "");
        assert_eq!(identity["keywords_observed"], json!(card.keywords));
        assert_eq!(
            manifest["cards"][card.id]["official_text"],
            card.official_text
        );
        assert_eq!(
            manifest["cards"][card.id]["official_text_html"],
            printing["rules_html"]
        );
        assert_eq!(manifest["cards"][card.id]["art_url"], card.art_url);
        assert_eq!(manifest["cards"][card.id]["energy"], card.cost);
        assert_eq!(manifest["cards"][card.id]["power"], card.power_cost);
        assert_eq!(
            card.art_url.split('?').next(),
            printing["image_url"].as_str().unwrap().split('?').next()
        );
        assert!(
            !printing["super_types"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t == "signature")
        );
        art(&format!("card_{}", card.id));
    }
    assert_eq!(cards[281].kind, CardKind::Gear);
    assert_eq!(
        (272..290)
            .filter(|&id| cards[id].kind == CardKind::Unit)
            .count(),
        17
    );
    for id in [287, 288, 289] {
        let p = source["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["official_id"] == cards[id].official_id)
            .unwrap();
        assert!(
            p["super_types"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t == "champion")
        );
    }
    // The C6 review preserves raw issue history while resolving effective facts.
    assert_eq!(source["counts"]["source_quality_issues"], 37);
    assert_eq!(
        source["source_quality_issues"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|issue| issue["kind"] == "conflicting_type_labels")
            .count(),
        23
    );
    assert_eq!(source["counts"]["unresolved_type_conflict_identities"], 0);
}

#[test]
fn ambessa_has_her_actual_legend_identity_and_a_legal_authored_recipe() {
    let definition = legend(7).unwrap();
    assert_eq!(definition.official_id, "ven-153-166");
    assert_eq!(definition.name, "Matriarch of War");
    assert_eq!(definition.champion, "Ambessa");
    assert_eq!(definition.domains, [Domain::Body, Domain::Order]);
    assert_eq!(definition.default_champion, 289);
    assert_eq!(
        definition.text,
        "When you empower something else, empower me. (I become Empowered if I'm not already.)\nDisempower me, [Any Power], [Exhaust]: Ready a unit."
    );
    art("legend_7");
    let deck = CustomDeck::from_legend(7).unwrap();
    assert_eq!(deck.name, "Ambessa training");
    assert_eq!(deck.card_count(), 40);
    assert_eq!(deck.chosen_champion, 289);
    assert_eq!(deck.runes, [0, 0, 0, 5, 0, 7]);
    assert_eq!(deck.battlefield, 9);
    assert_eq!(
        deck.cards,
        vec![
            (24, 3),
            (85, 3),
            (128, 2),
            (129, 2),
            (221, 3),
            (225, 2),
            (236, 3),
            (252, 2),
            (253, 2),
            (276, 3),
            (277, 3),
            (282, 3),
            (285, 3),
            (286, 3),
            (289, 3)
        ]
    );
    assert_eq!(eligible_champions(7), vec![289]);
    assert_eq!(CustomDeck::from_json(&deck.to_json()).unwrap(), deck);
    let mut stable_id = deck.to_json();
    stable_id["legend"] = json!("ven-153-166");
    assert_eq!(CustomDeck::from_json(&stable_id).unwrap(), deck);
    assert_eq!(
        cards::presets().len(),
        4,
        "training is not another tournament preset"
    );
}

#[test]
fn new_legend_rejects_wrong_domains_champions_and_rune_identity() {
    let original = CustomDeck::from_legend(7).unwrap();
    for champion in [276, 287, 288, 183] {
        let mut bad = original.clone();
        bad.chosen_champion = champion;
        assert!(
            bad.validate().is_err(),
            "invalid chosen champion {champion}"
        );
    }
    let mut bad = original.clone();
    bad.cards.push((272, 1)); // Fury Shadow Fiend is outside Body/Order.
    assert!(bad.validate().is_err());
    let mut bad = original.clone();
    bad.cards.retain(|&(id, _)| id != 289);
    bad.cards.push((73, 3));
    assert_eq!(bad.card_count(), 40);
    assert!(
        bad.validate().is_err(),
        "chosen champion must be included in the deck"
    );
    let mut bad = original.clone();
    bad.runes = [1, 0, 0, 4, 0, 7];
    assert!(bad.validate().is_err());
    let mut bad = original;
    bad.cards.iter_mut().find(|(id, _)| *id == 289).unwrap().1 = 4;
    assert!(bad.validate().is_err());
}

#[test]
fn all_eighteen_new_cards_have_exact_domain_legal_homes() {
    let cards = cards::cards();
    let mut covered = BTreeSet::new();
    for definition in legends() {
        let offered = eligible_cards(definition.id);
        let mut deck = CustomDeck::from_legend(definition.id).unwrap();
        for card in &cards[272..290] {
            let expected = card
                .domains
                .iter()
                .all(|domain| definition.domains.contains(domain));
            assert_eq!(
                offered.contains(&card.id),
                expected,
                "{} / {}",
                definition.champion,
                card.name
            );
            if expected {
                covered.insert(card.id);
                if !deck.cards.iter().any(|&(id, _)| id == card.id) {
                    deck.cards.push((card.id, 1));
                }
            }
        }
        deck.validate().unwrap();
        assert_eq!(CustomDeck::from_json(&deck.to_json()).unwrap(), deck);
    }
    assert_eq!(covered, (272..290).collect());
    assert_eq!(
        eligible_cards(7)
            .into_iter()
            .filter(|id| (272..290).contains(id))
            .collect::<Vec<_>>(),
        vec![276, 277, 282, 285, 286, 289]
    );
}

#[test]
fn original_seven_training_lists_and_four_presets_remain_stable() {
    for (id, preset) in cards::presets().iter().enumerate() {
        assert_eq!(CustomDeck::from_legend(id).unwrap().cards, preset.cards);
        assert_eq!(
            CustomDeck::from_legend(id).unwrap().chosen_champion,
            preset.chosen_champion
        );
    }
    let recipes: [&[(usize, u8)]; 3] = [
        &[
            (56, 3),
            (59, 3),
            (96, 3),
            (99, 3),
            (101, 3),
            (102, 3),
            (143, 3),
            (179, 3),
            (183, 3),
            (74, 3),
            (85, 3),
            (224, 2),
            (58, 3),
            (203, 2),
        ],
        &[
            (175, 3),
            (176, 3),
            (68, 3),
            (57, 3),
            (147, 3),
            (170, 3),
            (177, 2),
            (63, 3),
            (52, 3),
            (85, 3),
            (87, 2),
            (138, 3),
            (126, 2),
            (129, 2),
            (120, 2),
        ],
        &[
            (182, 3),
            (68, 3),
            (57, 3),
            (60, 3),
            (109, 2),
            (139, 2),
            (75, 3),
            (77, 2),
            (79, 2),
            (61, 2),
            (87, 2),
            (52, 3),
            (63, 3),
            (85, 3),
            (126, 2),
            (129, 2),
        ],
    ];
    let runes = [[7, 0, 0, 0, 0, 5], [0, 0, 6, 0, 0, 6], [0, 0, 8, 0, 0, 4]];
    for (offset, old) in recipes.into_iter().enumerate() {
        let mut expected = old.to_vec();
        expected.sort_unstable();
        let deck = CustomDeck::from_legend(offset + 4).unwrap();
        assert_eq!(deck.cards, expected);
        assert_eq!(deck.runes, runes[offset]);
        assert_eq!(deck.battlefield, 0);
    }
}
