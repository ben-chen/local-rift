//! Eight source-reviewed activations; metadata does not prove runtime conformance.
use local_rift::cards::{self, CardKind, Domain};
use local_rift::deck::{CustomDeck, eligible_cards};
use local_rift::legend_catalog::legends;
use local_rift::rules::{self, Mechanic, OfficialEffect, StaticAbility, TokenPlacement};
use local_rift::trigger_rules::{self, Effect, Event, TargetRule};
use serde_json::Value;
use std::collections::BTreeSet;

const IDS: [&str; 8] = [
    "sfd-046-221",
    "ogn-184-298",
    "rad-027-167",
    "unl-160-219",
    "ven-054-166",
    "ven-087-166",
    "ven-075-166",
    "ven-077-166",
];

#[test]
fn eight_source_facts_and_art_preserve_clean_legal_printings() {
    let catalog = cards::cards();
    let inventory: Value =
        serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    assert!(catalog.len() >= 254);
    for (offset, official_id) in IDS.iter().enumerate() {
        let card = catalog[246 + offset];
        assert_eq!((card.id, card.official_id), (246 + offset, *official_id));
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
        assert_eq!(printing["type_labels_conflict"], false);
        assert_eq!(identity["implementation"]["runtime_id"], card.id);
        assert_eq!(printing["name"], card.name);
        let kind = if matches!(card.id, 248 | 249) {
            CardKind::Unit
        } else {
            CardKind::Gear
        };
        assert_eq!(card.kind, kind);
        assert_eq!(
            printing["types"],
            serde_json::json!([kind.name().to_lowercase()])
        );
        assert!(printing["super_types"].as_array().unwrap().is_empty());
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
        assert_eq!(printing["equipment_text"], "");
        assert_eq!(
            manifest["cards"][card.id]["official_text"],
            card.official_text
        );
        assert_eq!(
            manifest["cards"][card.id]["official_text_html"],
            printing["rules_html"]
        );
        if !matches!(card.id, 247 | 249) {
            assert_eq!(printing["rules_text"], card.official_text);
        }
        if card.id != 247 {
            assert!(identity["errata"].as_array().unwrap().is_empty());
        }
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
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n") && bytes.len() < 300_000);
    }
    assert_eq!(catalog[248].domain, Domain::Calm);
    assert_eq!((catalog[249].cost, catalog[249].might), (5, 5));
    assert_eq!(catalog[249].tags, &["Poro", "Freljord"]);
    assert_eq!((catalog[251].cost, catalog[251].power_cost), (4, 1));
}

#[test]
fn syren_errata_and_ultrasoft_core_correction_keep_original_source_text() {
    let catalog = cards::cards();
    let inventory: Value =
        serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    let syren = inventory["cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["identity"] == IDS[1])
        .unwrap();
    assert_eq!(
        syren["errata"][0]["text"],
        "[1], [E]: Move a friendly unit at a battlefield to its base."
    );
    assert_eq!(
        catalog[247].official_text,
        "[1 Energy], [Exhaust]: Move a friendly unit at a battlefield to its base."
    );
    assert_eq!(
        manifest["cards"][247]["errata_source"],
        syren["errata"][0]["source_url"]
    );
    assert!(
        manifest["cards"][247]["official_text_html"]
            .as_str()
            .unwrap()
            .contains("your base")
    );
    let correction = &manifest["cards"][249]["text_correction"];
    assert_eq!(correction["kind"], "official_image_and_core");
    assert_eq!(correction["core_rule"], "377.2.b");
    assert!(
        correction["raw_gallery_text"]
            .as_str()
            .unwrap()
            .contains("[1 Energy] Might Bird")
    );
    assert!(
        manifest["cards"][249]["official_text_html"]
            .as_str()
            .unwrap()
            .contains(":rb_energy_1: :rb_might: Bird")
    );
    assert!(
        catalog[249]
            .official_text
            .contains("Play two 1 Might Bird unit tokens with [Deflect]")
    );
    assert!(!catalog[249].official_text.contains("Energy"));
}

#[test]
fn only_reviewed_entry_trigger_and_token_metadata_apply_when_played() {
    for id in 246..254 {
        assert_eq!(rules::effect(id), OfficialEffect::None);
        assert_eq!(rules::target_bounds(id), (0, 0));
        assert!(!rules::on_play_requires_target(id));
        assert!(rules::triggers(id).is_empty());
        assert!(rules::requirements(id).contains(&Mechanic::ActivatedAbilities));
        assert!(rules::equipment(id).is_none());
        assert!(rules::resource_gear(id).is_none());
        if id != 246 {
            assert!(trigger_rules::trigger_rules(id).is_empty());
        }
    }
    let draw = trigger_rules::trigger_rules(246);
    assert_eq!(draw.len(), 1);
    assert_eq!(draw[0].event, Event::SelfPlayed);
    assert_eq!(draw[0].targets, TargetRule::NONE);
    assert!(draw[0].conditions.is_empty() && !draw[0].optional);
    assert_eq!(draw[0].effects, &[Effect::Draw(1)]);
    assert_eq!(
        rules::static_abilities(252),
        &[StaticAbility::EntersExhausted]
    );
    assert!(
        rules::static_abilities(249).is_empty(),
        "Deflect belongs to the Birds, not Ultrasoft Poro"
    );
    for id in 250..254 {
        assert!(rules::requirements(id).contains(&Mechanic::Empower));
    }
    assert!(rules::requirements(252).contains(&Mechanic::ResourceAbilities));
    let bird = rules::token_spec(249).unwrap();
    assert_eq!(
        (bird.name, bird.count, bird.might, bird.tag, bird.deflect),
        ("Bird", 2, 1, Some("Bird"), true)
    );
    assert!(!bird.is_gear);
    assert_eq!(rules::token_placement(249), TokenPlacement::Default);
    let mech = rules::token_spec(251).unwrap();
    assert_eq!(
        (mech.name, mech.count, mech.might, mech.tag),
        ("Mech", 1, 3, Some("Mech"))
    );
    assert!(!mech.is_gear && !mech.deflect);
    assert_eq!(rules::token_placement(251), TokenPlacement::Base);
    assert_eq!(rules::effect(catalog_len()), OfficialEffect::Unsupported);
}
fn catalog_len() -> usize {
    cards::cards().len()
}

#[test]
fn eight_cards_have_existing_legend_homes_without_changing_recipes() {
    let offered: BTreeSet<_> = legends()
        .iter()
        .flat_map(|l| eligible_cards(l.id))
        .filter(|id| (246..254).contains(id))
        .collect();
    assert_eq!(offered, (246..254).collect());
    for id in [251, 252, 253] {
        let homes: Vec<_> = legends()
            .iter()
            .filter(|l| eligible_cards(l.id).contains(&id))
            .map(|l| l.id)
            .collect();
        let expected: Vec<_> = legends()
            .iter()
            .filter(|legend| legend.domains.contains(&cards::Domain::Body))
            .map(|legend| legend.id)
            .collect();
        assert_eq!(homes, expected);
    }
    assert!(
        cards::presets()
            .iter()
            .flat_map(|p| &p.cards)
            .all(|(id, _)| *id < 69)
    );
    // The original C3 checkpoint promised to preserve these seven recipes.
    for legend in &legends()[..7] {
        let deck = CustomDeck::from_legend(legend.id).unwrap();
        assert!(deck.cards.iter().all(|(id, _)| *id < 246));
        assert_eq!(deck.card_count(), 40);
    }
}
