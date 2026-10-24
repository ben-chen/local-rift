//! This validates catalog provenance/mapping, not executable support for new cards.
use local_rift::cards;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn snapshot() -> Value {
    let data: Value =
        serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap();
    assert_eq!(
        data["schema_version"], 2,
        "coverage requires effective catalog facts"
    );
    data
}

#[test]
fn all_gallery_printings_have_one_identity_and_complete_base_sets() {
    let data = snapshot();
    let printings = data["printings"].as_array().unwrap();
    let by_id: BTreeMap<_, _> = printings
        .iter()
        .map(|p| (p["official_id"].as_str().unwrap(), p))
        .collect();
    assert_eq!(by_id.len(), printings.len());
    let mut assigned = BTreeSet::new();
    for card in data["cards"].as_array().unwrap() {
        for id in card["official_ids"].as_array().unwrap() {
            let id = id.as_str().unwrap();
            assert!(by_id.contains_key(id), "Unknown printing {id}");
            assert!(
                assigned.insert(id),
                "Printing belongs to two identities: {id}"
            );
            assert_eq!(card["name"], by_id[id]["name"]);
        }
    }
    assert_eq!(assigned.len(), printings.len());
    for (set, coverage) in data["set_coverage"].as_object().unwrap() {
        let expected = coverage["numbered_base_slots"].as_u64().unwrap();
        let mut slots = BTreeSet::new();
        for printing in printings.iter().filter(|p| p["set"] == *set) {
            let code = printing["public_code"].as_str().unwrap();
            let Some((_, number)) = code.split_once('-') else {
                continue;
            };
            let Some((number, total)) = number.split_once('/') else {
                continue;
            };
            let (Ok(number), Ok(total)) = (number.parse::<u64>(), total.parse::<u64>()) else {
                continue;
            };
            if number <= total {
                assert_eq!(total, expected);
                assert!(slots.insert(number), "Duplicate {set} base slot {number}");
            }
        }
        assert_eq!(slots, (1..=expected).collect(), "Incomplete base set {set}");
    }
}

#[test]
fn every_runtime_card_maps_by_official_id_without_enabling_inventory_only_cards() {
    let data = snapshot();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    let runtime = cards::cards();
    let mapped: BTreeMap<_, _> = data["cards"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["implementation"]["status"] == "implemented_main_card")
        .map(|c| {
            (
                c["implementation"]["runtime_id"].as_u64().unwrap() as usize,
                c,
            )
        })
        .collect();
    assert_eq!(mapped.len(), runtime.len());
    for card in &runtime {
        let entry = mapped[&card.id];
        assert_eq!(entry["name"], card.name);
        assert_eq!(entry["implementation"]["official_id"], card.official_id);
        assert_eq!(
            entry["implementation"]["official_id"],
            manifest["cards"][card.id]["official_id"]
        );
        assert_eq!(entry["standard_legal"], true);
        assert_eq!(entry["implementation"]["playable_in_client"], true);
        let official = data["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["official_id"] == entry["implementation"]["official_id"])
            .unwrap();
        assert!(
            entry["source_review"]["unresolved_gameplay_fields"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            official["unresolved_gameplay_fields"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let official = official
            .get("effective")
            .filter(|value| value.is_object())
            .expect("complete effective source facts");
        // Absent resource icons remain null in the effective projection. Runtime
        // numeric costs use zero; raw preservation is tested separately.
        assert_eq!(
            u64::from(card.cost),
            official["energy"].as_u64().unwrap_or(0)
        );
        assert_eq!(
            u64::from(card.power_cost),
            official["power"].as_u64().unwrap_or(0)
        );
        match card.kind {
            cards::CardKind::Unit => assert_eq!(
                u64::from(card.might),
                official["might"].as_u64().expect("a unit has base Might"),
                "{} unit Might",
                card.name
            ),
            // CardDef stores an equipment bonus in this shared numeric field;
            // the catalog's non-unit `might` is not a unit base stat.
            cards::CardKind::Gear => assert_eq!(
                u64::from(card.might),
                official["might_bonus"].as_u64().unwrap_or(0),
                "{} equipment bonus",
                card.name
            ),
            cards::CardKind::Spell => assert_eq!(card.might, 0),
        }
        let domains: BTreeSet<_> = card
            .domains
            .iter()
            .map(|d| d.name().to_lowercase())
            .collect();
        let official_domains: BTreeSet<_> = official["domains"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(domains, official_domains);
        let runtime_tags: BTreeSet<_> = card.tags.iter().copied().collect();
        let effective_tags: BTreeSet<_> = official["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tag| tag.as_str().unwrap())
            .collect();
        assert_eq!(
            runtime_tags.len(),
            card.tags.len(),
            "duplicate runtime tags"
        );
        assert_eq!(runtime_tags, effective_tags, "{} source tags", card.name);
    }
    for card in data["cards"].as_array().unwrap() {
        if card["implementation"]["status"] == "not_implemented" {
            assert_eq!(card["implementation"]["playable_in_client"], false);
        }
    }
}

#[test]
fn legality_counts_exclude_bans_and_auxiliary_game_pieces() {
    let data = snapshot();
    let cards = data["cards"].as_array().unwrap();
    let legal = cards.iter().filter(|c| c["standard_legal"] == true).count();
    let banned = cards
        .iter()
        .filter(|c| c["exclusion"] == "banned_in_constructed")
        .count();
    let auxiliary = cards.iter().filter(|c| c["auxiliary"] == true).count();
    assert_eq!((legal, banned, auxiliary), (1079, 12, 12));
    assert_eq!(legal + banned + auxiliary, cards.len());
    assert_eq!(data["counts"]["standard_legal_identities"], legal);
    let unimplemented = cards
        .iter()
        .filter(|c| {
            c["standard_legal"] == true && c["implementation"]["playable_in_client"] == false
        })
        .count();
    assert_eq!(
        data["counts"]["unimplemented_legal_identities"],
        unimplemented
    );
}
