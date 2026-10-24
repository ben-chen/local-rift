//! Complete C18 facts are pinned to independently reviewed official faces.
use local_rift::{
    cards::{self, CardKind},
    deck::{CustomDeck, eligible_cards, eligible_champions},
    legend_catalog::legends,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[test]
fn clocktower_runtime_definitions_match_reviewed_effective_source_facts() {
    let cards = cards::cards();
    let proof: Value = serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c18/admissions.json"
    ))
    .unwrap();
    let records = proof["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    let mut kinds = [0; 3];
    for (offset, r) in records.iter().enumerate() {
        let id = 384 + offset;
        let c = cards[id];
        let facts = &r["effective"];
        assert_eq!(r["runtime_id"], id);
        assert_eq!(r["official_id"], c.official_id);
        assert_eq!(r["name"], c.name);
        assert_eq!(facts["types"], json!([c.kind.name().to_lowercase()]));
        assert_eq!(
            facts["domains"],
            json!(
                c.domains
                    .iter()
                    .map(|d| d.name().to_lowercase())
                    .collect::<Vec<_>>()
            )
        );
        assert_eq!(facts["tags"], json!(c.tags));
        assert_eq!(facts["rules_text"], c.official_text);
        assert_eq!(u64::from(c.cost), facts["energy"].as_u64().unwrap_or(0));
        assert_eq!(
            u64::from(c.power_cost),
            facts["power"].as_u64().unwrap_or(0)
        );
        assert_eq!(u64::from(c.might), facts["might"].as_u64().unwrap_or(0));
        assert_eq!(facts["super_types"], json!([]));
        assert_eq!(facts["equipment_text"], "");
        kinds[match c.kind {
            CardKind::Unit => 0,
            CardKind::Spell => 1,
            CardKind::Gear => 2,
        }] += 1;
        assert_ne!(
            local_rift::rules::effect(id),
            local_rift::rules::OfficialEffect::Unsupported
        );
    }
    assert_eq!(kinds, [1, 0, 0]);
}

#[test]
fn every_card_has_legal_homes_and_never_changes_authored_recipes_or_champions() {
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
        for card in &cards[384..385] {
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
    assert_eq!(covered, (384..385).collect());
    assert!(
        cards::presets()
            .iter()
            .all(|p| p.cards.iter().all(|(id, _)| *id < 69))
    );
}
