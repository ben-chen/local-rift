//! Fifth expansion source facts and explicit metadata, not a conformance proof.
use local_rift::cards::{self, CardKind, Domain};
use local_rift::deck::eligible_cards;
use local_rift::legend_catalog::legends;
use local_rift::rules::{self, Mechanic, OfficialEffect, StaticAbility, TokenPlacement};
use serde_json::Value;
use std::collections::BTreeSet;

const IDS: [&str; 10] = [
    "sfd-052-221",
    "ogn-090-298",
    "ogn-017-298",
    "unl-001-219",
    "ogn-124-298",
    "sfd-168-221",
    "unl-093-219",
    "unl-026-219",
    "ogn-068-298",
    "sfd-088-221",
];

#[test]
fn activated_card_facts_and_art_match_the_clean_legal_official_printings() {
    let catalog = cards::cards();
    let inventory: Value =
        serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    assert!(catalog.len() >= 240);
    for (offset, official_id) in IDS.iter().enumerate() {
        let card = catalog[230 + offset];
        assert_eq!(card.id, 230 + offset);
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
        assert_eq!(printing["types"][0], card.kind.name().to_lowercase());
        assert_eq!(
            printing["energy"].as_u64().unwrap_or(0),
            u64::from(card.cost)
        );
        assert_eq!(
            printing["power"].as_u64().unwrap_or(0),
            u64::from(card.power_cost)
        );
        assert_eq!(
            printing["might"].as_u64().unwrap_or(0),
            u64::from(card.might)
        );
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
    assert_eq!(catalog[230].domain, Domain::Calm);
    assert_eq!(catalog[231].kind, CardKind::Gear);
    assert_eq!((catalog[235].cost, catalog[235].power_cost), (7, 1));
    assert_eq!(
        (
            catalog[238].cost,
            catalog[238].power_cost,
            catalog[238].might
        ),
        (3, 1, 3)
    );
}

#[test]
fn activated_permanents_do_not_invent_automatic_play_effects() {
    let catalog = cards::cards();
    for id in 230..240 {
        assert_eq!(rules::effect(id), OfficialEffect::None);
        assert_eq!(rules::target_bounds(id), (0, 0));
        assert!(!rules::on_play_requires_target(id));
        assert!(rules::triggers(id).is_empty());
        assert!(local_rift::trigger_rules::trigger_rules(id).is_empty());
        assert!(rules::requirements(id).contains(&Mechanic::ActivatedAbilities));
    }
    assert_eq!(
        rules::static_abilities(232),
        &[StaticAbility::EntersExhausted]
    );
    assert_eq!(rules::static_abilities(233), &[StaticAbility::EntersReady]);
    assert_eq!(rules::static_abilities(238), &[StaticAbility::Backline]);
    // Backline-equivalent wording is mapped explicitly, not invented in source text.
    assert!(!catalog[238].keywords.contains(&"Backline"));
    assert!(
        catalog[238]
            .official_text
            .starts_with("I must be assigned combat damage last.")
    );
    assert!(rules::requirements(236).contains(&Mechanic::ResourceAbilities));
    let token = rules::token_spec(235).unwrap();
    assert_eq!(
        (token.name, token.might, token.count, token.tag),
        ("Recruit", 1, 3, Some("Recruit"))
    );
    assert!(!token.is_gear && !token.deflect);
    assert_eq!(rules::token_placement(235), TokenPlacement::Default);
    assert_eq!(rules::effect(catalog.len()), OfficialEffect::Unsupported);
}

#[test]
fn all_ten_cards_have_a_legal_existing_legend_home_without_changing_presets() {
    let eligible: BTreeSet<_> = legends()
        .iter()
        .flat_map(|legend| eligible_cards(legend.id))
        .filter(|id| (230..240).contains(id))
        .collect();
    assert_eq!(eligible, (230..240).collect());
    let presets = cards::presets();
    assert_eq!(presets.len(), 4);
    assert!(
        presets
            .iter()
            .all(|p| p.cards.iter().all(|(id, _)| *id < 69))
    );
}
