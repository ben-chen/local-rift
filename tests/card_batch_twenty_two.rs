//! C21 complete original printing cohorts and legal main-deck admission.
use local_rift::{
    cards::{self, CardKind, Domain},
    deck::{CustomDeck, eligible_cards, eligible_champions},
    legend_catalog::legends,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn proof() -> Value {
    serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c21/admissions.json"
    ))
    .unwrap()
}

#[test]
fn three_definitions_match_exact_facts_without_invented_tags_or_stats() {
    let cards = cards::cards();
    let proof = proof();
    let records = proof["records"].as_array().unwrap();
    let expected = [
        (
            "sfd-058-221",
            "Ornn, Blacksmith",
            CardKind::Unit,
            Domain::Calm,
            5,
            1,
            5,
        ),
        (
            "ven-056-166",
            "Clairvoyance",
            CardKind::Spell,
            Domain::Mind,
            7,
            0,
            0,
        ),
        (
            "unl-136-219",
            "Scryer's Bloom",
            CardKind::Gear,
            Domain::Chaos,
            1,
            0,
            0,
        ),
    ];
    assert_eq!(records.len(), expected.len());
    for (offset, record) in records.iter().enumerate() {
        let card = cards[395 + offset];
        let (id, name, kind, domain, energy, power, might) = expected[offset];
        assert_eq!(
            (card.id, card.official_id, card.name),
            (395 + offset, id, name)
        );
        assert_eq!(
            (card.kind, card.domain, card.domains),
            (kind, domain, &[domain][..])
        );
        assert_eq!(
            (card.cost, card.power_cost, card.might),
            (energy, power, might)
        );
        assert_eq!(record["runtime_id"], card.id);
        assert_eq!(record["effective"]["rules_text"], card.official_text);
        assert_eq!(record["effective"]["tags"], json!(card.tags));
        assert_eq!(record["effective"]["equipment_text"], "");
        assert_eq!(record["runtime_image"]["url"], card.art_url);
        assert_eq!(record["errata"], json!([]));
        assert_eq!(record["existing_field_resolution_ids"], json!([]));
    }
    assert_eq!(cards[395].tags, &["Ornn", "Freljord"]);
    assert_eq!(cards[396].keywords, &["Reaction"]);
    assert!(cards[397].tags.is_empty());
    assert!(
        cards[397]
            .official_text
            .starts_with("This enters exhausted.\nKill this, [1 Energy], [Exhaust]:")
    );
}

#[test]
fn both_ornn_printings_preserve_champion_supertype_and_matching_rules() {
    let proof = proof();
    let base = &proof["records"][0];
    let alternate = &base["alternate_printings"][0];
    assert_eq!(
        base["printing_cohort"],
        json!(["sfd-058-221", "sfd-058a-221"])
    );
    assert_eq!(base["effective"], alternate["effective"]);
    assert_eq!(base["effective"]["super_types"], json!(["champion"]));
    assert_eq!(
        alternate["visual_observation"]["printed_tags"],
        json!(["Ornn", "Freljord"])
    );
    assert_ne!(
        base["original_image"]["sha256"],
        alternate["original_image"]["sha256"]
    );
    assert_eq!(
        proof["records"][1]["printing_cohort"],
        json!(["ven-056-166"])
    );
    assert_eq!(
        proof["records"][2]["printing_cohort"],
        json!(["unl-136-219"])
    );
    assert!(proof["records"][1]["raw_printing"]["power"].is_null());
    assert!(proof["records"][2]["raw_printing"]["might"].is_null());
}

#[test]
fn all_three_fit_existing_main_decks_without_changing_champions_or_authored_lists() {
    let homes = [vec![0, 1], vec![3, 5, 6, 9], vec![0, 2, 9]];
    let mut covered = BTreeSet::new();
    for legend in legends() {
        let original = CustomDeck::from_legend(legend.id).unwrap();
        assert_eq!(original.card_count(), 40);
        assert!(original.cards.iter().all(|(id, _)| *id < 395));
        let mut expanded = original.clone();
        for (offset, allowed) in homes.iter().enumerate() {
            let id = 395 + offset;
            let legal = allowed.contains(&legend.id);
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
            }
        }
        expanded.validate().unwrap();
        assert_eq!(
            CustomDeck::from_json(&expanded.to_json()).unwrap(),
            expanded
        );
        assert_eq!(expanded.chosen_champion, original.chosen_champion);
        if homes[0].contains(&legend.id) {
            let mut invalid_champion = expanded;
            invalid_champion.chosen_champion = 395;
            assert!(
                invalid_champion.validate().is_err(),
                "Ornn does not match {}",
                legend.champion
            );
        }
    }
    assert_eq!(covered, (395..398).collect());
    assert!(
        cards::presets()
            .iter()
            .all(|d| d.cards.iter().all(|(id, _)| *id < 395))
    );
}

#[test]
fn full_face_offline_thumbnails_remain_proportional_and_bounded() {
    for bytes in [
        include_bytes!("../assets/art/card_395.png").as_slice(),
        include_bytes!("../assets/art/card_396.png").as_slice(),
        include_bytes!("../assets/art/card_397.png").as_slice(),
    ] {
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(bytes.len() < 300_000);
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        assert!((f64::from(width) / f64::from(height) - 744.0 / 1039.0).abs() < 0.002);
    }
}
