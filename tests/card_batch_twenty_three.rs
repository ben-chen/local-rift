//! C22 exact source metadata; executable deck eligibility also needs the runtime patch.
use local_rift::{
    cards::{self, CardKind, Domain},
    deck::{CustomDeck, eligible_cards, eligible_champions},
    legend_catalog::legends,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn proof() -> Value {
    serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c22/admissions.json"
    ))
    .unwrap()
}

#[test]
fn three_mind_units_preserve_exact_stats_tags_and_distinct_vision_text() {
    let cards = cards::cards();
    let proof = proof();
    let expected = [
        ("ogn-100-298", "Gemcraft Seer", 3, 1, 3),
        ("sfd-065-221", "Forecaster", 2, 0, 2),
        ("ogn-086-298", "Jeweled Colossus", 5, 0, 5),
    ];
    for (offset, (official_id, name, energy, power, might)) in expected.into_iter().enumerate() {
        let card = cards[398 + offset];
        let record = &proof["records"][offset];
        assert_eq!(
            (card.id, card.official_id, card.name),
            (398 + offset, official_id, name)
        );
        assert_eq!(
            (card.kind, card.domain, card.domains),
            (CardKind::Unit, Domain::Mind, &[Domain::Mind][..])
        );
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
        assert_eq!(record["existing_field_resolution_ids"], json!([]));
    }
    assert_eq!(cards[398].tags, &["Mount Targon"]);
    assert_eq!(cards[399].tags, &["Mech", "Yordle", "Bandle City"]);
    assert_eq!(cards[400].tags, &["Mount Targon"]);
    assert!(cards[398].official_text.starts_with("[Vision]"));
    assert!(
        cards[398]
            .official_text
            .ends_with("Other friendly units have [Vision].")
    );
    assert!(
        cards[399]
            .official_text
            .starts_with("Your Mechs have [Vision].")
    );
    assert!(!cards[399].official_text.starts_with("[Vision]"));
    assert!(
        cards[400]
            .official_text
            .ends_with("[Shield] (+1 Might while I'm a defender.)")
    );
    // Observed keyword words are source metadata, not an inference of intrinsic abilities.
    assert_eq!(cards[399].keywords, &["Vision"]);
}

#[test]
fn complete_single_printings_keep_nullable_power_and_actual_face_tag_order() {
    let proof = proof();
    for (offset, id) in ["ogn-100-298", "sfd-065-221", "ogn-086-298"]
        .into_iter()
        .enumerate()
    {
        let record = &proof["records"][offset];
        assert_eq!(record["printing_cohort"], json!([id]));
        assert_eq!(record["alternate_printings"], json!([]));
        assert_eq!(
            record["visual_observation"]["printed_types"],
            json!(["unit"])
        );
        assert_eq!(
            record["visual_observation"]["printed_supertypes"],
            json!([])
        );
    }
    assert!(proof["records"][1]["raw_printing"]["power"].is_null());
    assert!(proof["records"][2]["raw_printing"]["power"].is_null());
    assert_eq!(
        proof["records"][1]["visual_observation"]["printed_tags"],
        json!(["Bandle City", "Yordle", "Mech"])
    );
    assert_eq!(
        proof["records"][1]["effective"]["tags"],
        json!(["Mech", "Yordle", "Bandle City"])
    );
}

#[test]
fn all_three_have_four_legal_main_deck_homes_and_no_chosen_champion_eligibility() {
    let homes = [3, 5, 6, 9];
    let mut covered = BTreeSet::new();
    for legend in legends() {
        let original = CustomDeck::from_legend(legend.id).unwrap();
        assert_eq!(original.card_count(), 40);
        assert!(original.cards.iter().all(|(id, _)| *id < 398));
        let mut expanded = original.clone();
        for id in 398..401 {
            let legal = homes.contains(&legend.id);
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
                let mut invalid_champion = one;
                invalid_champion.chosen_champion = id;
                assert!(invalid_champion.validate().is_err());
            }
        }
        expanded.validate().unwrap();
        assert_eq!(
            CustomDeck::from_json(&expanded.to_json()).unwrap(),
            expanded
        );
        assert_eq!(expanded.chosen_champion, original.chosen_champion);
    }
    assert_eq!(covered, (398..401).collect());
    assert!(
        cards::presets()
            .iter()
            .all(|d| d.cards.iter().all(|(id, _)| *id < 398))
    );
}

#[test]
fn offline_faces_are_complete_proportional_pngs_under_300kb() {
    for bytes in [
        include_bytes!("../assets/art/card_398.png").as_slice(),
        include_bytes!("../assets/art/card_399.png").as_slice(),
        include_bytes!("../assets/art/card_400.png").as_slice(),
    ] {
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(bytes.len() < 300_000);
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        assert!((f64::from(width) / f64::from(height) - 744.0 / 1039.0).abs() < 0.002);
    }
}
