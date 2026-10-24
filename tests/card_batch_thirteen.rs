//! C11 exact source facts and deck legality; XP payment behavior is tested separately.
use local_rift::cards::{self, CardKind, Domain};
use local_rift::deck::{CustomDeck, eligible_cards, eligible_champions};
use local_rift::legend_catalog::legends;
use serde_json::{Value, json};
use std::collections::BTreeSet;

const IDS: [&str; 4] = ["unl-102-219", "unl-162-219", "unl-049-219", "unl-158-219"];
const STATS: [(u8, u8, u8); 4] = [(3, 0, 3), (2, 0, 2), (2, 0, 0), (2, 0, 2)];

#[test]
fn four_source_identities_preserve_exact_rules_panels_and_printed_costs() {
    let cards = cards::cards();
    let catalog: Value =
        serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    let proof: Value = serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c11/admissions.json"
    ))
    .unwrap();
    assert!(cards.len() >= 349);
    assert_eq!(proof["records"].as_array().unwrap().len(), 4);
    for (offset, official_id) in IDS.iter().enumerate() {
        let id = 345 + offset;
        let card = cards[id];
        let printing = catalog["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["official_id"] == *official_id)
            .unwrap();
        let identity = catalog["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["identity"] == *official_id)
            .unwrap();
        let facts = &printing["effective"];
        assert_eq!((card.id, card.official_id), (id, *official_id));
        assert_eq!((card.cost, card.power_cost, card.might), STATS[offset]);
        assert_eq!(identity["official_ids"], json!([official_id]));
        assert_eq!(identity["standard_legal"], true);
        assert_eq!(identity["errata"], json!([]));
        assert_eq!(identity["implementation"]["runtime_id"], id);
        assert_eq!(
            identity["source_review"]["unresolved_gameplay_fields"],
            json!([])
        );
        assert_eq!(
            identity["source_review"]["unresolved_variant_fields"],
            json!([])
        );
        assert_eq!(printing["resolution_ids"], json!([]));
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
        assert_eq!(facts["super_types"], json!([]));
        assert_eq!(facts["rules_text"], card.official_text);
        assert_eq!(facts["equipment_text"], "");
        assert_eq!(facts["equipment_html"], "");
        assert_eq!(
            identity["effective_keywords_observed"],
            json!(card.keywords)
        );
        assert_eq!(
            manifest["cards"][id]["official_text_html"],
            facts["rules_html"]
        );
        assert_eq!(manifest["cards"][id]["power"], 0);
        assert_eq!(manifest["cards"][id]["art_url"], card.art_url);
        assert_eq!(proof["records"][offset]["runtime_id"], id);
        assert_eq!(proof["records"][offset]["effective"], *facts);
        let art = std::fs::read(format!(
            "{}/assets/art/card_{id}.png",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        assert!(art.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(art.len() < 300_000);
    }
    assert_eq!(
        cards[345..349]
            .iter()
            .filter(|c| c.kind == CardKind::Unit)
            .count(),
        2
    );
    assert_eq!(
        cards[345..349]
            .iter()
            .filter(|c| c.kind == CardKind::Gear)
            .count(),
        2
    );
}

#[test]
fn all_four_have_exact_domain_homes_without_new_chosen_champions() {
    let cards = cards::cards();
    let mut covered = BTreeSet::new();
    for legend in legends() {
        let original = CustomDeck::from_legend(legend.id).unwrap();
        assert_eq!(original.card_count(), 40);
        if legend.id < 8 {
            // This checkpoint preserves the original eight training recipes.
            assert!(original.cards.iter().all(|(id, _)| *id < 290));
        }
        let mut expanded = original.clone();
        for card in &cards[345..349] {
            let id = card.id;
            let compatible = card.domains.iter().all(|d| legend.domains.contains(d));
            assert_eq!(eligible_cards(legend.id).contains(&id), compatible);
            assert!(!eligible_champions(legend.id).contains(&id));
            let mut one = original.clone();
            one.cards.push((id, 1));
            assert_eq!(one.validate().is_ok(), compatible);
            one.chosen_champion = id;
            assert!(one.validate().is_err());
            if compatible {
                expanded.cards.push((id, 1));
                covered.insert(id);
            }
        }
        expanded.validate().unwrap();
        assert_eq!(expanded.chosen_champion, original.chosen_champion);
        assert_eq!(
            CustomDeck::from_json(&expanded.to_json()).unwrap(),
            expanded
        );
        // There is no Calm/Order legend: a Honeyfruit + Heirloom recipe would be illegal.
        assert!(
            !(legend.domains.contains(&Domain::Calm) && legend.domains.contains(&Domain::Order))
        );
    }
    assert_eq!(covered, (345..349).collect());
    assert!(
        cards::presets()
            .iter()
            .all(|d| d.cards.iter().all(|(id, _)| *id < 69))
    );
}

#[test]
fn heirloom_bonus_is_not_a_unit_stat_or_an_equipment_effect_trigger() {
    let cards = cards::cards();
    let proof: Value = serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c11/admissions.json"
    ))
    .unwrap();
    let source = &proof["records"][3];
    assert_eq!((cards[348].kind, cards[348].might), (CardKind::Gear, 2));
    assert_eq!(cards[348].tags, &["Equipment"]);
    assert_eq!(source["raw_printing"]["might"], 2);
    assert_eq!(source["raw_printing"]["might_bonus"], 2);
    assert!(source["visual_observation"]["printed_might"].is_null());
    assert_eq!(source["visual_observation"]["printed_might_bonus"], 2);
    assert_eq!(source["effective"]["equipment_text"], "");
    assert!(
        cards[348]
            .official_text
            .starts_with("When you play this, gain 1 XP.")
    );
}

#[test]
fn reviewed_costs_resource_outputs_and_rule_panel_have_explicit_descriptors() {
    use local_rift::activated::{AbilityTargets, AbilityTiming, ResourceOutput, activated_rules};
    use local_rift::rules::{self, StaticAbility};
    use local_rift::trigger_rules::{Amount, Effect, Event, TriggerTextArea, trigger_rules};
    for id in [345, 346] {
        let abilities = activated_rules(id);
        assert_eq!(abilities.len(), 1);
        let a = abilities[0];
        assert_eq!(a.xp_cost, 2);
        assert_eq!((a.energy, a.power), (0, 0));
        assert!(!a.exhaust);
        assert_eq!(a.timing, AbilityTiming::Main);
        assert_eq!(a.targets, AbilityTargets::NONE);
        assert!(!a.adds_resources());
        assert!(rules::static_abilities(id).contains(&StaticAbility::Hunt(1)));
    }
    let honey = activated_rules(347);
    assert_eq!(honey.len(), 2);
    for a in honey {
        assert_eq!(a.xp_cost, 0);
        assert_eq!((a.energy, a.power), (0, 0));
        assert!(a.exhaust && a.adds_resources());
        assert_eq!(a.timing, AbilityTiming::Reaction);
        assert_eq!(a.targets, AbilityTargets::NONE);
    }
    assert_eq!(honey[0].minimum_xp, None);
    assert_eq!(honey[1].minimum_xp, Some(6));
    assert_eq!(
        honey[0].fixed_resource_output(),
        Some(ResourceOutput {
            energy: 0,
            power: 1,
            domain: None
        })
    );
    assert_eq!(
        honey[1].fixed_resource_output(),
        Some(ResourceOutput {
            energy: 1,
            power: 1,
            domain: None
        })
    );
    assert!(rules::static_abilities(347).contains(&StaticAbility::EntersExhausted));
    let equip = rules::equipment(348).unwrap();
    assert_eq!(
        (
            equip.might,
            equip.equip_xp,
            equip.equip_energy,
            equip.equip_power
        ),
        (2, 1, 0, 0)
    );
    let heirloom = trigger_rules(348);
    assert_eq!(heirloom.len(), 1);
    assert_eq!(heirloom[0].text_area, TriggerTextArea::Rules);
    assert_eq!(heirloom[0].event, Event::SelfPlayed);
    assert_eq!(heirloom[0].effects, &[Effect::Xp(Amount::Fixed(1))]);
}
