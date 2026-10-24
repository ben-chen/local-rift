//! C19 source metadata: exact original faces and separately authoritative errata.
use local_rift::{
    cards::{self, CardKind},
    deck::{CustomDeck, eligible_cards, eligible_champions},
    legend_catalog::legends,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[test]
fn four_card_definitions_match_reviewed_facts_and_separate_errata() {
    let catalog = cards::cards();
    let proof: Value = serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c19/admissions.json"
    ))
    .unwrap();
    let records = proof["records"].as_array().unwrap();
    assert_eq!(records.len(), 4);
    let expected = [
        ("ogn-242-298", "Baited Hook", CardKind::Gear, 3, 0),
        ("ogn-160-298", "Dazzling Aurora", CardKind::Gear, 9, 2),
        ("ogn-062-298", "Reinforce", CardKind::Spell, 5, 0),
        ("ven-089-166", "Wild Claw", CardKind::Spell, 7, 1),
    ];
    for (offset, record) in records.iter().enumerate() {
        let id = 385 + offset;
        let card = catalog[id];
        let facts = &record["effective"];
        let (official_id, name, kind, energy, power) = expected[offset];
        assert_eq!(card.id, id);
        assert_eq!(card.official_id, official_id);
        assert_eq!(card.name, name);
        assert_eq!(card.kind, kind);
        assert_eq!((card.cost, card.power_cost, card.might), (energy, power, 0));
        assert_eq!(record["runtime_id"], id);
        assert_eq!(record["official_id"], card.official_id);
        assert_eq!(record["name"], card.name);
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
        assert!(card.tags.is_empty());
        assert!(card.keywords.is_empty());
        assert_eq!(facts["super_types"], json!([]));
        assert_eq!(facts["equipment_text"], "");
        assert_eq!(record["runtime_rules_text"], card.official_text);
        if offset < 3 {
            assert_eq!(record["errata"][0]["text"], card.official_text);
            assert_ne!(
                record["visual_observation"]["printed_rules_icons_expanded"],
                card.official_text
            );
        } else {
            assert_eq!(record["errata"], json!([]));
            assert_eq!(facts["rules_text"], card.official_text);
        }
        assert_eq!(record["runtime_image"]["url"], card.art_url);
        assert!(record["runtime_image"]["bytes"].as_u64().unwrap() < 300_000);
    }
    assert!(
        !records[1]["effective"]["rules_text"]
            .as_str()
            .unwrap()
            .contains("banish")
    );
    assert!(catalog[386].official_text.contains("and banish it"));
}

#[test]
fn new_cards_fit_existing_domains_without_changing_recipes_or_champions() {
    let catalog = cards::cards();
    let mut covered = BTreeSet::new();
    for legend in legends() {
        let original = CustomDeck::from_legend(legend.id).unwrap();
        assert_eq!(original.card_count(), 40);
        assert!(original.cards.iter().all(|(id, _)| *id < 385));
        let mut expanded = original.clone();
        for card in &catalog[385..389] {
            let compatible = card.domains.iter().all(|d| legend.domains.contains(d));
            assert_eq!(eligible_cards(legend.id).contains(&card.id), compatible);
            assert!(!eligible_champions(legend.id).contains(&card.id));
            let mut one = original.clone();
            one.cards.push((card.id, 1));
            assert_eq!(one.validate().is_ok(), compatible);
            if compatible {
                expanded.cards.push((card.id, 1));
                covered.insert(card.id);
            }
        }
        expanded.validate().unwrap();
        assert_eq!(
            CustomDeck::from_json(&expanded.to_json()).unwrap(),
            expanded
        );
    }
    assert_eq!(covered, (385..389).collect());
    assert!(
        cards::presets()
            .iter()
            .all(|p| p.cards.iter().all(|(id, _)| *id < 69))
    );
}

#[test]
fn runtime_art_keeps_full_proportional_card_faces() {
    for bytes in [
        include_bytes!("../assets/art/card_385.png").as_slice(),
        include_bytes!("../assets/art/card_386.png").as_slice(),
        include_bytes!("../assets/art/card_387.png").as_slice(),
        include_bytes!("../assets/art/card_388.png").as_slice(),
    ] {
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(bytes.len() < 300_000);
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        assert!((f64::from(width) / f64::from(height) - 744.0 / 1039.0).abs() < 0.002);
    }
}
