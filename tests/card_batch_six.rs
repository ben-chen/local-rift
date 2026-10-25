//! C2 source and explicit metadata checks, independent of activation conformance.
use local_rift::cards::{self, CardKind, Domain};
use local_rift::deck::eligible_cards;
use local_rift::legend_catalog::legends;
use local_rift::rules::{self, Mechanic, OfficialEffect, StaticAbility, TokenPlacement};
use local_rift::trigger_rules;
use serde_json::Value;
use std::collections::BTreeSet;

const IDS: [&str; 6] = [
    "ogn-099-298",
    "sfd-019-221",
    "unl-161-219",
    "ven-062-166",
    "sfd-117-221",
    "sfd-083-221",
];

#[test]
fn six_gear_facts_and_art_match_clean_legal_official_printings() {
    let catalog = cards::cards();
    let inventory: Value =
        serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    assert!(catalog.len() >= 246);
    for (offset, official_id) in IDS.iter().enumerate() {
        let card = catalog[240 + offset];
        assert_eq!(card.id, 240 + offset);
        assert_eq!(card.official_id, *official_id);
        let printing = inventory["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["official_id"] == *official_id)
            .unwrap();
        let identity = inventory["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["identity"] == *official_id)
            .unwrap();
        assert_eq!(identity["standard_legal"], true);
        assert!(
            identity["conflicting_fields"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(identity["errata"].as_array().unwrap().is_empty());
        assert_eq!(identity["implementation"]["runtime_id"], card.id);
        assert_eq!(printing["name"], card.name);
        assert_eq!(printing["types"], serde_json::json!(["gear"]));
        assert_eq!(card.kind, CardKind::Gear);
        assert!(printing["super_types"].as_array().unwrap().is_empty());
        assert_eq!(
            printing["energy"].as_u64().unwrap_or(0),
            u64::from(card.cost)
        );
        assert_eq!(
            printing["power"].as_u64().unwrap_or(0),
            u64::from(card.power_cost)
        );
        assert_eq!(printing["might"], Value::Null);
        assert_eq!(card.might, 0);
        assert_eq!(printing["rules_text"], card.official_text);
        assert_eq!(printing["equipment_text"], "");
        assert_eq!(
            manifest["cards"][card.id]["official_text"],
            card.official_text
        );
        let domains: Vec<_> = card
            .domains
            .iter()
            .map(|d| d.name().to_lowercase())
            .collect();
        assert_eq!(printing["domains"], serde_json::json!(domains));
        assert_eq!(printing["tags"], serde_json::json!(card.tags));
        assert!(card.tags.is_empty());
        assert_eq!(
            identity["keywords_observed"],
            serde_json::json!(card.keywords)
        );
        assert_eq!(manifest["cards"][card.id]["art_url"], card.art_url);
        assert_eq!(
            card.art_url.split('?').next(),
            printing["image_url"].as_str().unwrap().split('?').next()
        );
        let bytes = std::fs::read(format!(
            "{}/assets/art/card_{}.png",
            env!("CARGO_MANIFEST_DIR"),
            card.id
        ))
        .unwrap();
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(bytes.len() < 300_000);
    }
    assert_eq!(catalog[241].domain, Domain::Fury);
    assert_eq!((catalog[244].cost, catalog[244].power_cost), (2, 1));
    assert_eq!((catalog[245].cost, catalog[245].power_cost), (3, 1));
}

#[test]
fn activation_and_vision_are_separate_from_playing_the_gear() {
    for id in 240..246 {
        assert_eq!(rules::effect(id), OfficialEffect::None);
        assert_eq!(rules::target_bounds(id), (0, 0));
        assert!(!rules::on_play_requires_target(id));
        assert!(rules::triggers(id).is_empty());
        assert!(rules::requirements(id).contains(&Mechanic::ActivatedAbilities));
        assert!(rules::equipment(id).is_none());
        // A converter is not a free fixed resource source.
        assert!(rules::resource_gear(id).is_none());
        if id != 242 {
            assert!(trigger_rules::trigger_rules(id).is_empty());
        }
    }
    // Intrinsic Vision now shares the actual entry-instance collector with
    // granted Vision. Retaining the former descriptor would trigger twice.
    for id in [193, 242] {
        assert_eq!(
            rules::static_abilities(id)
                .iter()
                .filter(|ability| **ability == StaticAbility::Vision)
                .count(),
            1
        );
        assert!(trigger_rules::trigger_rules(id).is_empty());
    }
    assert_eq!(
        rules::static_abilities(243),
        &[StaticAbility::EntersExhausted]
    );
    assert!(rules::requirements(242).contains(&Mechanic::Predict));
    assert!(rules::requirements(243).contains(&Mechanic::Empower));
    for id in [244, 245] {
        assert!(rules::requirements(id).contains(&Mechanic::ResourceAbilities));
    }
    let mech = rules::token_spec(241).unwrap();
    assert_eq!(
        (mech.name, mech.count, mech.might, mech.tag),
        ("Mech", 1, 3, Some("Mech"))
    );
    assert!(!mech.is_gear && !mech.deflect);
    assert_eq!(rules::token_placement(241), TokenPlacement::Base);
    assert_eq!(
        rules::effect(cards::cards().len()),
        OfficialEffect::Unsupported
    );
}

#[test]
fn all_six_have_existing_legend_homes_and_presets_stay_separate() {
    let offered: BTreeSet<_> = legends()
        .iter()
        .flat_map(|l| eligible_cards(l.id))
        .filter(|id| (240..246).contains(id))
        .collect();
    assert_eq!(offered, (240..246).collect());
    let body_homes: Vec<_> = legends()
        .iter()
        .filter(|l| eligible_cards(l.id).contains(&244))
        .map(|l| l.id)
        .collect();
    let expected: Vec<_> = legends()
        .iter()
        .filter(|legend| legend.domains.contains(&cards::Domain::Body))
        .map(|legend| legend.id)
        .collect();
    assert_eq!(body_homes, expected);
    let presets = cards::presets();
    assert_eq!(presets.len(), 4);
    assert!(
        presets
            .iter()
            .flat_map(|p| p.cards.iter())
            .all(|(id, _)| *id < 69)
    );
}
