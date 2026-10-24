//! C20 original-face facts, separately reviewed tags, and existing legal deck homes.
use local_rift::{
    cards::{self, CardKind},
    deck::{CustomDeck, eligible_cards, eligible_champions},
    legend_catalog::legends,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[test]
fn six_definitions_match_complete_reviewed_source_facts() {
    let cards = cards::cards();
    let proof: Value = serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c20/admissions.json"
    ))
    .unwrap();
    let records = proof["records"].as_array().unwrap();
    let expected = [
        ("unl-032-219", "Double Trouble", CardKind::Spell, 2, 0),
        ("unl-064-219", "Fate Weaver", CardKind::Unit, 5, 4),
        ("rad-129-167", "Trifarian Captain", CardKind::Unit, 2, 1),
        ("sfd-041-221", "Apprentice Smith", CardKind::Unit, 2, 2),
        ("rad-074-167", "Raving Witness", CardKind::Unit, 3, 3),
        ("ven-033-166", "Pakaa Protector", CardKind::Unit, 5, 4),
    ];
    assert_eq!(records.len(), expected.len());
    for (offset, record) in records.iter().enumerate() {
        let card = cards[389 + offset];
        let (official_id, name, kind, energy, might) = expected[offset];
        let e = &record["effective"];
        assert_eq!(
            (card.id, card.official_id, card.name, card.kind),
            (389 + offset, official_id, name, kind)
        );
        assert_eq!((card.cost, card.power_cost, card.might), (energy, 0, might));
        assert_eq!(record["runtime_id"], card.id);
        assert_eq!(record["official_id"], official_id);
        assert_eq!(e["types"], json!([card.kind.name().to_lowercase()]));
        assert_eq!(
            e["domains"],
            json!(
                card.domains
                    .iter()
                    .map(|d| d.name().to_lowercase())
                    .collect::<Vec<_>>()
            )
        );
        assert_eq!(e["tags"], json!(card.tags));
        assert_eq!(e["super_types"], json!([]));
        assert_eq!(e["equipment_text"], "");
        assert_eq!(record["errata"], json!([]));
        assert_eq!(e["rules_text"], card.official_text);
        assert_eq!(
            record["visual_observation"]["printed_rules_icons_expanded"],
            card.official_text
        );
        assert_eq!(record["runtime_image"]["url"], card.art_url);
    }
    assert_eq!(cards[389].keywords, &["Repeat"]);
    assert_eq!(cards[393].keywords, &["Mighty"]);
}

#[test]
fn source_tag_corrections_are_explicit_and_raw_feed_is_preserved() {
    let proof: Value = serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c20/admissions.json"
    ))
    .unwrap();
    let cards = cards::cards();
    for (offset, tags) in [(2, json!(["Noxus", "Trifarian"])), (4, json!(["Freljord"]))] {
        let record = &proof["records"][offset];
        assert_eq!(record["raw_printing"]["tags"], json!([]));
        assert_eq!(record["effective"]["tags"], tags);
        assert_eq!(json!(cards[389 + offset].tags), tags);
        assert_eq!(
            record["existing_field_resolution_ids"],
            json!([format!("{}:tags", cards[389 + offset].official_id)])
        );
    }
}

#[test]
fn six_cards_fit_existing_legends_without_changing_training_or_presets() {
    let cards = cards::cards();
    let mut covered = BTreeSet::new();
    for legend in legends() {
        let original = CustomDeck::from_legend(legend.id).unwrap();
        assert_eq!(original.card_count(), 40);
        assert!(original.cards.iter().all(|(id, _)| *id < 389));
        let mut expanded = original.clone();
        for card in &cards[389..395] {
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
    assert_eq!(covered, (389..395).collect());
    assert!(
        cards::presets()
            .iter()
            .all(|p| p.cards.iter().all(|(id, _)| *id < 69))
    );
}

#[test]
fn runtime_art_preserves_full_proportional_faces_below_three_hundred_kilobytes() {
    for bytes in [
        include_bytes!("../assets/art/card_389.png").as_slice(),
        include_bytes!("../assets/art/card_390.png").as_slice(),
        include_bytes!("../assets/art/card_391.png").as_slice(),
        include_bytes!("../assets/art/card_392.png").as_slice(),
        include_bytes!("../assets/art/card_393.png").as_slice(),
        include_bytes!("../assets/art/card_394.png").as_slice(),
    ] {
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(bytes.len() < 300_000);
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        assert!((f64::from(width) / f64::from(height) - 744.0 / 1039.0).abs() < 0.002);
    }
}
