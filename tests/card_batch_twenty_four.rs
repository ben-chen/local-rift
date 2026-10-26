//! Deploy source metadata and deck legality; behavioral checks live in the engine suite.
use local_rift::{
    cards::{self, CardKind, Domain},
    deck::{CustomDeck, eligible_cards, eligible_champions},
    legend_catalog::legends,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn proof() -> Value {
    serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c24/admissions.json"
    ))
    .unwrap()
}

#[test]
fn eight_gear_one_unit_one_spell_preserve_exact_reviewed_facts() {
    let cards = cards::cards();
    let proof = proof();
    let expected = [
        ("rad-002-167", Domain::Fury, 1, 1, 0, CardKind::Gear),
        ("rad-024-167", Domain::Calm, 2, 0, 0, CardKind::Gear),
        ("rad-116-167", Domain::Order, 1, 0, 0, CardKind::Gear),
        ("rad-044-167", Domain::Calm, 4, 2, 0, CardKind::Gear),
        ("rad-073-167", Domain::Body, 1, 0, 0, CardKind::Gear),
        ("rad-087-167", Domain::Body, 2, 0, 0, CardKind::Gear),
        ("rad-052-167", Domain::Mind, 1, 0, 0, CardKind::Gear),
        ("rad-106-167", Domain::Chaos, 3, 1, 0, CardKind::Gear),
        ("rad-072-167", Domain::Body, 2, 0, 2, CardKind::Unit),
        ("rad-033-167", Domain::Calm, 1, 1, 0, CardKind::Spell),
    ];
    for (offset, (official_id, domain, energy, power, might, kind)) in
        expected.into_iter().enumerate()
    {
        let card = cards[401 + offset];
        let record = &proof["records"][offset];
        assert_eq!((card.id, card.official_id), (401 + offset, official_id));
        assert_eq!((card.kind, card.domain), (kind, domain));
        assert_eq!(card.domains, &[domain]);
        assert_eq!(
            (card.cost, card.power_cost, card.might),
            (energy, power, might)
        );
        assert_eq!(record["runtime_id"], card.id);
        assert_eq!(record["effective"]["rules_text"], card.official_text);
        assert_eq!(record["effective"]["tags"], json!(card.tags));
        assert_eq!(record["effective"]["super_types"], json!([]));
        assert_eq!(record["effective"]["equipment_text"], "");
        assert_eq!(record["runtime_image"]["url"], card.art_url);
        assert_eq!(record["errata"], json!([]));
        if offset < 8 {
            assert!(card.official_text.starts_with("[Deploy]"));
            assert!(card.tags.is_empty());
        }
    }
    // Observed words in metadata are not evidence of an intrinsic keyword.
    assert!(!cards[409].official_text.starts_with("[Deploy]"));
    assert!(cards[409].tags.contains(&"Bandle City"));
    assert!(cards[409].tags.contains(&"Yordle"));
    assert!(!cards[410].keywords.contains(&"Action"));
    assert!(!cards[410].keywords.contains(&"Reaction"));
}

#[test]
fn complete_single_printings_keep_raw_domain_errors_and_nullable_costs() {
    let proof = proof();
    for record in proof["records"].as_array().unwrap() {
        assert_eq!(record["printing_cohort"], json!([record["official_id"]]));
        assert_eq!(record["alternate_printings"], json!([]));
    }
    for (offset, old, new) in [
        (1, "chaos", "calm"),
        (4, "calm", "body"),
        (7, "body", "chaos"),
    ] {
        let record = &proof["records"][offset];
        assert_eq!(record["raw_printing"]["domains"], json!([old]));
        assert_eq!(record["effective"]["domains"], json!([new]));
        assert!(
            record["existing_field_resolution_ids"]
                .as_array()
                .unwrap()
                .len()
                >= 2
        );
    }
    for offset in [1, 2, 4, 5, 6, 8] {
        assert!(proof["records"][offset]["raw_printing"]["power"].is_null());
    }
}

#[test]
fn all_ten_have_exact_legend_homes_without_changing_presets_or_champions() {
    let homes: [&[usize]; 10] = [
        &[3, 4, 8],
        &[0, 1],
        &[2, 4, 5, 6, 7],
        &[0, 1],
        &[1, 7, 8],
        &[1, 7, 8],
        &[3, 5, 6, 9],
        &[0, 2, 9],
        &[1, 7, 8],
        &[0, 1],
    ];
    let mut covered = BTreeSet::new();
    for legend in legends() {
        let original = CustomDeck::from_legend(legend.id).unwrap();
        assert_eq!(original.card_count(), 40);
        assert!(original.cards.iter().all(|(id, _)| *id < 401));
        let mut expanded = original.clone();
        for id in 401..411 {
            let legal = homes[id - 401].contains(&legend.id);
            assert_eq!(eligible_cards(legend.id).contains(&id), legal);
            assert!(!eligible_champions(legend.id).contains(&id));
            let mut one = original.clone();
            one.cards.push((id, 1));
            assert_eq!(
                one.validate().is_ok(),
                legal,
                "card {id}, legend {}",
                legend.id
            );
            if legal {
                expanded.cards.push((id, 1));
                covered.insert(id);
                one.chosen_champion = id;
                assert!(one.validate().is_err());
            }
        }
        expanded.validate().unwrap();
        assert_eq!(
            CustomDeck::from_json(&expanded.to_json()).unwrap(),
            expanded
        );
        assert_eq!(expanded.chosen_champion, original.chosen_champion);
    }
    assert_eq!(covered, (401..411).collect());
    assert!(
        cards::presets()
            .iter()
            .all(|d| d.cards.iter().all(|(id, _)| *id < 401))
    );
}

#[test]
fn all_ten_runtime_faces_are_bounded_proportional_original_art() {
    for bytes in [
        include_bytes!("../assets/art/card_401.png").as_slice(),
        include_bytes!("../assets/art/card_402.png").as_slice(),
        include_bytes!("../assets/art/card_403.png").as_slice(),
        include_bytes!("../assets/art/card_404.png").as_slice(),
        include_bytes!("../assets/art/card_405.png").as_slice(),
        include_bytes!("../assets/art/card_406.png").as_slice(),
        include_bytes!("../assets/art/card_407.png").as_slice(),
        include_bytes!("../assets/art/card_408.png").as_slice(),
        include_bytes!("../assets/art/card_409.png").as_slice(),
        include_bytes!("../assets/art/card_410.png").as_slice(),
    ] {
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(bytes.len() < 300_000);
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        assert_eq!((width, height), (280, 391));
    }
}
