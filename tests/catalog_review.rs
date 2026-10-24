//! Consumer regressions for source reconciliation, without admitting new cards.
use local_rift::{
    cards,
    deck::{CustomDeck, eligible_cards, eligible_champions},
    legend_catalog,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn catalog() -> Value {
    let data: Value =
        serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap();
    assert_eq!(data["schema_version"], 2);
    data
}
fn resolutions() -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/data/catalog_resolutions.json"
        ))
        .expect("reviewed resolution manifest"),
    )
    .unwrap()
}

#[test]
fn reviewed_facts_are_effective_while_original_evidence_stays_intact() {
    let data = catalog();
    let manifest = resolutions();
    let printings: BTreeMap<_, _> = data["printings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|printing| (printing["official_id"].as_str().unwrap(), printing))
        .collect();
    assert_eq!(manifest["status"], "reviewed");
    assert_eq!(manifest["printing_projection_version"], 1);
    assert_eq!(manifest["resolutions"].as_array().unwrap().len(), 118);
    assert_eq!(manifest["raw_printings"].as_array().unwrap().len(), 74);
    for raw in manifest["raw_printings"].as_array().unwrap() {
        let id = raw["official_id"].as_str().unwrap();
        for (field, value) in raw.as_object().unwrap() {
            if field == "review_draft_sha256" {
                // Review-manifest metadata is not part of the original printing.
                continue;
            }
            assert_eq!(&printings[id][field], value, "raw {id}.{field} changed");
        }
    }
    for resolution in manifest["resolutions"].as_array().unwrap() {
        let id = resolution["official_id"].as_str().unwrap();
        let field = resolution["field"].as_str().unwrap();
        assert_eq!(
            printings[id][field], resolution["expected_raw_value"],
            "raw {id}.{field}"
        );
        assert_eq!(
            printings[id]["effective"][field], resolution["effective_value"],
            "effective {id}.{field}"
        );
        assert!(
            printings[id]["resolution_ids"]
                .as_array()
                .unwrap()
                .contains(&resolution["id"])
        );
        assert_eq!(resolution["enables_runtime_support"], false);
    }
    let historical: Vec<_> = manifest["historical_issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|issue| issue["raw_issue"].clone())
        .collect();
    assert_eq!(data["source_quality_issues"], json!(historical));
    assert_eq!(historical.len(), 43);
}

#[test]
fn mournful_spirit_keeps_raw_tags_and_exposes_the_complete_printed_set() {
    let data = catalog();
    let printing = data["printings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["official_id"] == "rad-126-167")
        .unwrap();
    assert_eq!(printing["tags"], json!(["Spirit"]));
    assert_eq!(
        printing["effective"]["tags"],
        json!(["Shadow Isles", "Spirit"])
    );
    assert_eq!(printing["resolution_ids"], json!(["rad-126-167:tags"]));
    let source = resolutions();
    let resolution = source["resolutions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "rad-126-167:tags")
        .unwrap();
    assert_eq!(resolution["enables_runtime_support"], false);
    assert!(
        cards::cards()[..372]
            .iter()
            .all(|c| c.official_id != "rad-126-167")
    );
}

#[test]
fn reviewed_dual_type_stays_unavailable_and_goatherd_uses_later_admission() {
    let data = catalog();
    let printing = |id: &str| {
        data["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["official_id"] == id)
            .unwrap()
    };
    let porobot = printing("ven-058-166");
    assert_eq!(porobot["types"], json!(["unit"]));
    assert_eq!(porobot["effective"]["types"], json!(["unit", "gear"]));
    assert_eq!(porobot["effective"]["might"], 2);
    assert_eq!(porobot["effective"]["equipment_text"], "");
    let goatherd = printing("rad-083-167");
    assert_eq!(goatherd["tags"], json!([]));
    assert_eq!(goatherd["effective"]["tags"], json!(["Mount Targon"]));
    for id in ["ven-058-166", "rad-083-167"] {
        let identity = data["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["identity"] == id)
            .unwrap();
        assert_eq!(identity["standard_legal"], true);
        assert_eq!(
            identity["implementation"]["playable_in_client"],
            id == "rad-083-167"
        );
        assert_eq!(
            cards::cards().iter().any(|card| card.official_id == id),
            id == "rad-083-167"
        );
    }
    assert_eq!(data["counts"]["standard_legal_by_kind"]["gear+unit"], 1);
    assert_eq!(data["counts"]["implemented_main_cards"], 379);
    assert_eq!(data["counts"]["implemented_legal_identities"], 406);
    assert_eq!(data["counts"]["unimplemented_legal_identities"], 673);
}

#[test]
fn corrected_domains_text_and_scalar_do_not_replace_their_raw_values() {
    let data = catalog();
    let printing = |id: &str| {
        data["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|printing| printing["official_id"] == id)
            .unwrap()
    };
    let anvil = printing("rad-012-167");
    assert_eq!(anvil["domains"], json!(["chaos"]));
    assert_eq!(anvil["effective"]["domains"], json!(["fury"]));
    assert!(
        anvil["rules_text"]
            .as_str()
            .unwrap()
            .contains("Chaos Power")
    );
    assert!(
        anvil["effective"]["rules_text"]
            .as_str()
            .unwrap()
            .contains("Fury Power")
    );
    let base = printing("rad-063-167");
    let alternate = printing("rad-sp4-005");
    assert_eq!(
        (base["might"].as_u64(), alternate["might"].as_u64()),
        (Some(5), Some(4))
    );
    assert_eq!(base["effective"]["might"], 5);
    assert_eq!(alternate["effective"]["might"], 5);
    let identity = data["cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|card| {
            card["official_ids"]
                .as_array()
                .unwrap()
                .contains(&json!("rad-sp4-005"))
        })
        .unwrap();
    assert_eq!(
        identity["conflicting_fields"],
        json!(["might"]),
        "raw cross-printing conflict history remains available"
    );
    assert!(
        identity["source_review"]["unresolved_gameplay_fields"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn equipment_might_and_attached_bonus_remain_distinct_source_facts() {
    let data = catalog();
    let shield = data["printings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|printing| printing["official_id"] == "rad-123-167")
        .unwrap();
    for facts in [shield, &shield["effective"]] {
        assert_eq!(facts["types"], json!(["gear"]));
        assert_eq!(facts["might"], 1);
        assert_eq!(facts["might_bonus"], 1);
    }
    let identity = data["cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|card| {
            card["official_ids"]
                .as_array()
                .unwrap()
                .contains(&json!("rad-123-167"))
        })
        .unwrap();
    assert_eq!(identity["kind"], "gear");
    assert_eq!(identity["implementation"]["playable_in_client"], true);
    assert_eq!(identity["implementation"]["runtime_id"], 327);
}

#[test]
fn source_review_never_admits_inventory_only_or_unresolved_identities() {
    let data = catalog();
    let auxiliary_printings: BTreeSet<_> = data["cards"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|card| card["auxiliary"] == true)
        .flat_map(|card| card["official_ids"].as_array().unwrap())
        .map(|id| id.as_str().unwrap())
        .collect();
    for card in data["cards"].as_array().unwrap() {
        let unresolved = card["source_review"]["unresolved_gameplay_fields"]
            .as_array()
            .unwrap();
        let unresolved_variants = card["source_review"]["unresolved_variant_fields"]
            .as_array()
            .unwrap();
        if !unresolved.is_empty()
            || !unresolved_variants.is_empty()
            || card["implementation"]["status"] == "not_implemented"
        {
            assert_eq!(
                card["implementation"]["playable_in_client"], false,
                "{} unexpectedly admitted",
                card["name"]
            );
        }
        assert!(card["effective_keywords_observed"].is_array());
    }
    for printing in data["printings"].as_array().unwrap() {
        let effective = printing["effective"]
            .as_object()
            .expect("every printing requires effective facts");
        if effective["domains"].as_array().unwrap().is_empty()
            && effective["types"].as_array().unwrap().iter().any(|kind| {
                matches!(
                    kind.as_str(),
                    Some("unit" | "spell" | "gear" | "legend" | "rune")
                )
            })
            && !auxiliary_printings.contains(printing["official_id"].as_str().unwrap())
        {
            assert!(
                printing["unresolved_gameplay_fields"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("domains")),
                "missing domains are not colorless: {}",
                printing["official_id"]
            );
        }
    }
    assert_eq!(data["counts"]["implemented_legal_identities"], 406);
    assert_eq!(data["counts"]["unimplemented_legal_identities"], 673);
}

#[test]
fn completed_variant_reviews_cover_every_member_without_compatibility_exceptions() {
    let data = catalog();
    let mut reviewed_identities = 0;
    let mut reviewed_printings = BTreeSet::new();
    let mut review_ids = BTreeSet::new();
    let mut unresolved_identities = 0;
    let mut unresolved_legal = 0;
    let mut unresolved_auxiliary = Vec::new();
    for card in data["cards"].as_array().unwrap() {
        let review = &card["source_review"];
        let unresolved = review["unresolved_variant_fields"].as_array().unwrap();
        assert!(unresolved.iter().all(Value::is_string));
        let reviewed = review["reviewed_variant_fields"].as_array().unwrap();
        assert!(reviewed.iter().all(Value::is_string));
        if let Some(id) = review["identity_review_id"].as_str() {
            assert!(!id.is_empty());
            assert!(review_ids.insert(id), "duplicate whole-identity review");
            assert!(
                unresolved.is_empty(),
                "{} is only partly reviewed",
                card["name"]
            );
            assert!(!reviewed.is_empty());
            assert_eq!(card["implementation"]["playable_in_client"], true);
            reviewed_identities += 1;
            for id in card["official_ids"].as_array().unwrap() {
                assert!(reviewed_printings.insert(id.as_str().unwrap()));
            }
        }
        if !unresolved.is_empty() {
            unresolved_identities += 1;
            unresolved_legal += usize::from(card["standard_legal"] == true);
            if card["auxiliary"] == true {
                unresolved_auxiliary.push(card["name"].as_str().unwrap());
            }
            assert_eq!(card["implementation"]["playable_in_client"], false);
        }
    }
    assert_eq!(reviewed_identities, 29);
    assert_eq!(reviewed_printings.len(), 90);
    assert_eq!((unresolved_identities, unresolved_legal), (95, 94));
    assert_eq!(unresolved_auxiliary, vec!["Gold"]);
    assert_eq!(data["counts"]["implemented_legal_identities"], 406);
}

#[test]
fn kennen_ionia_correction_reaches_runtime_and_public_catalog_without_rewriting_raw_tags() {
    let data = catalog();
    let printing = |id: &str| {
        data["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|printing| printing["official_id"] == id)
            .unwrap()
    };
    let base = printing("ven-113-166");
    let alternate = printing("ven-113a-166");
    assert_eq!(base["tags"], json!(["Yordle", "Kennen"]));
    assert_eq!(alternate["tags"], json!(["Yordle", "Ionia", "Kennen"]));
    let expected: BTreeSet<_> = ["Yordle", "Kennen", "Ionia"].into_iter().collect();
    for face in [base, alternate] {
        let effective: BTreeSet<_> = face["effective"]["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tag| tag.as_str().unwrap())
            .collect();
        assert_eq!(effective, expected);
    }
    // Keep the existing public tag order and append the evidenced missing tag.
    let runtime = cards::cards();
    assert_eq!(runtime[33].official_id, "ven-113-166");
    assert_eq!(runtime[33].tags, &["Yordle", "Kennen", "Ionia"]);
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    assert_eq!(manifest["cards"][33]["tags"], json!(runtime[33].tags));
    let public = local_rift::api::catalog();
    assert_eq!(public["cards"][33]["tags"], json!(runtime[33].tags));
    assert_eq!(public["cards"][33]["official_id"], "ven-113-166");
}

#[test]
fn rune_placeholder_corrections_keep_raw_text_and_do_not_create_a_keyword() {
    let data = catalog();
    for id in [
        "rad-r01a", "rad-r02a", "rad-r03a", "rad-r04a", "rad-r05a", "rad-r06a",
    ] {
        let printing = data["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|printing| printing["official_id"] == id)
            .unwrap();
        assert_eq!(printing["rules_text"], "[NO TEXT]");
        assert_eq!(printing["rules_html"], "<p>[NO TEXT]</p>");
        assert_eq!(printing["effective"]["rules_text"], "");
        assert_eq!(printing["effective"]["rules_html"], "");
        let identity = data["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|card| {
                card["official_ids"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(id))
            })
            .unwrap();
        assert_eq!(identity["keywords_observed"], json!(["NO TEXT"]));
        assert_eq!(identity["effective_keywords_observed"], json!([]));
        assert_eq!(
            identity["implementation"]["status"],
            "implemented_resource_model"
        );
    }
}

#[test]
fn variant_review_preserves_printed_reminders_separators_and_separate_current_errata() {
    let data = catalog();
    let printing = |id: &str| {
        data["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|printing| printing["official_id"] == id)
            .unwrap()
    };
    for (base_id, alternate_id) in [
        ("unl-113-219", "unl-113a-219"),
        ("sfd-036-221", "unl-221-219"),
        ("unl-120-219", "unl-120a-219"),
    ] {
        let base = printing(base_id);
        let alternate = printing(alternate_id);
        for face in [base, alternate] {
            assert_eq!(face["effective"]["rules_text"], face["rules_text"]);
            assert_eq!(face["effective"]["rules_html"], face["rules_html"]);
        }
        assert_ne!(
            base["effective"]["rules_text"],
            alternate["effective"]["rules_text"]
        );
    }
    assert!(
        printing("sfd-036-221")["effective"]["rules_text"]
            .as_str()
            .unwrap()
            .starts_with("[Deathknell] —")
    );
    assert!(
        printing("unl-221-219")["effective"]["rules_text"]
            .as_str()
            .unwrap()
            .starts_with("[Deathknell][>]")
    );
    let rengar = printing("unl-120-219");
    assert!(
        rengar["effective"]["rules_text"]
            .as_str()
            .unwrap()
            .contains("I can be played")
    );
    let identity = data["cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|card| card["identity"] == "unl-120-219")
        .unwrap();
    let runtime = cards::cards();
    assert_eq!(
        runtime[30].official_text,
        identity["errata"][0]["text"].as_str().unwrap()
    );
    assert!(runtime[30].official_text.contains("I can [Ambush]"));
}

#[test]
fn source_reconciliation_preserves_eligibility_presets_and_all_training_lists() {
    // Captured from the published C5 API before changing any source consumer.
    // Keep the original 290-card eligibility baseline fixed; later appended cards
    // are covered by their own admission tests instead of rewriting history.
    const BASELINE: &str = r###"{"legends":[{"id":0,"eligible_cards":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,25,33,35,36,37,38,40,42,43,44,45,46,47,48,71,72,80,84,86,92,107,110,112,116,119,121,125,135,140,141,142,148,154,158,159,162,163,164,169,171,173,184,185,186,187,188,189,190,192,193,197,200,201,208,210,211,213,220,230,238,246,247,248,258,261,266,268,269,270,271,273,274,275,278,287],"eligible_champions":[0,271],"training_deck":{"battlefield":0,"cards":[[0,1],[1,1],[2,1],[3,2],[4,2],[5,3],[6,3],[7,3],[8,2],[9,1],[10,1],[11,1],[12,1],[13,1],[14,2],[15,3],[16,2],[17,3],[18,3],[19,1],[20,1],[21,2]],"chosen_champion":0,"legend":0,"name":"Irelia training","runes":[0,6,0,0,6,0]}},{"id":1,"eligible_cards":[0,2,4,6,7,8,10,12,15,17,19,21,22,23,24,25,26,27,28,29,30,31,32,70,71,76,82,83,86,92,104,105,106,107,108,110,111,112,113,115,116,118,119,121,122,125,128,135,141,142,148,152,153,154,157,158,167,174,181,184,191,192,196,197,204,207,208,210,211,213,217,218,220,221,225,230,234,236,238,244,246,248,251,252,253,258,261,263,268,273,274,276,278,286,287],"eligible_champions":[22,106],"training_deck":{"battlefield":1,"cards":[[22,1],[2,1],[4,3],[6,3],[7,3],[8,2],[23,3],[24,1],[25,2],[26,1],[27,3],[28,3],[29,2],[30,3],[31,1],[32,3],[15,3],[21,2]],"chosen_champion":22,"legend":1,"name":"Master Yi training","runes":[0,5,0,7,0,0]}},{"id":2,"eligible_cards":[1,3,9,11,13,14,16,18,20,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,72,73,80,84,85,87,88,89,90,91,94,101,102,109,114,117,124,129,137,140,147,149,150,151,155,159,160,162,163,164,169,170,171,173,176,177,178,179,180,185,186,187,188,189,190,193,198,199,200,201,203,209,235,242,247,249,266,269,270,271,275,277,282,285,289],"eligible_champions":[33],"training_deck":{"battlefield":2,"cards":[[33,1],[34,1],[35,2],[36,1],[9,2],[11,2],[37,1],[38,2],[39,3],[40,3],[14,2],[41,1],[42,3],[43,3],[16,1],[44,2],[45,3],[18,1],[46,2],[47,3],[48,1]],"chosen_champion":33,"legend":2,"name":"Kennen training","runes":[0,0,0,0,9,3]}},{"id":3,"eligible_cards":[49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,64,65,66,67,68,69,74,75,77,78,79,81,93,95,96,97,98,99,100,103,120,123,126,127,130,131,132,133,134,136,138,139,143,144,145,146,156,161,165,166,168,172,175,182,183,194,195,202,205,206,212,214,215,216,219,222,223,224,226,227,228,229,231,232,233,237,239,240,241,243,245,250,254,255,256,257,259,260,262,264,265,267,272,279,280,281,283,284,288],"eligible_champions":[49],"training_deck":{"battlefield":3,"cards":[[49,1],[50,1],[51,1],[52,2],[53,3],[54,3],[55,3],[56,2],[57,2],[58,1],[59,2],[60,3],[61,2],[62,1],[63,3],[64,3],[65,2],[66,1],[67,1],[68,3]],"chosen_champion":49,"legend":3,"name":"Kai'Sa training","runes":[7,0,5,0,0,0]}},{"id":4,"eligible_cards":[34,41,49,51,53,54,55,56,58,59,62,67,73,74,85,87,88,89,90,91,93,94,95,96,97,98,99,100,101,102,103,109,114,117,123,124,127,129,131,137,143,147,149,150,151,155,160,166,170,172,176,177,178,179,180,183,198,199,203,205,206,209,216,222,223,224,226,228,232,233,235,237,241,242,249,254,255,257,259,260,262,264,265,272,277,280,281,282,285,288,289],"eligible_champions":[183],"training_deck":{"battlefield":0,"cards":[[56,3],[58,3],[59,3],[74,3],[85,3],[96,3],[99,3],[101,3],[102,3],[143,3],[179,3],[183,3],[203,2],[224,2]],"chosen_champion":183,"legend":4,"name":"Darius training","runes":[7,0,0,0,0,5]}},{"id":5,"eligible_cards":[34,41,50,52,57,60,61,63,64,65,66,68,69,73,75,77,78,79,81,85,87,88,89,90,91,94,101,102,109,114,117,120,124,126,129,130,132,133,134,136,137,138,139,144,145,146,147,149,150,151,155,156,160,161,165,168,170,175,176,177,178,179,180,182,194,195,198,199,202,203,209,212,214,215,219,227,229,231,235,239,240,242,243,245,249,250,256,267,277,279,282,283,284,285,289],"eligible_champions":[175,176],"training_deck":{"battlefield":0,"cards":[[52,3],[57,3],[63,3],[68,3],[85,3],[87,2],[120,2],[126,2],[129,2],[138,3],[147,3],[170,3],[175,3],[176,3],[177,2]],"chosen_champion":175,"legend":5,"name":"Viktor training","runes":[0,0,6,0,0,6]}},{"id":6,"eligible_cards":[34,41,50,52,57,60,61,63,64,65,66,68,69,73,75,77,78,79,81,85,87,88,89,90,91,94,101,102,109,114,117,120,124,126,129,130,132,133,134,136,137,138,139,144,145,146,147,149,150,151,155,156,160,161,165,168,170,175,176,177,178,179,180,182,194,195,198,199,202,203,209,212,214,215,219,227,229,231,235,239,240,242,243,245,249,250,256,267,277,279,282,283,284,285,289],"eligible_champions":[182],"training_deck":{"battlefield":0,"cards":[[52,3],[57,3],[60,3],[61,2],[63,3],[68,3],[75,3],[77,2],[79,2],[85,3],[87,2],[109,2],[126,2],[129,2],[139,2],[182,3]],"chosen_champion":182,"legend":6,"name":"Lux training","runes":[0,0,8,0,0,4]}},{"id":7,"eligible_cards":[22,23,24,26,27,28,29,30,31,32,34,41,70,73,76,82,83,85,87,88,89,90,91,94,101,102,104,105,106,108,109,111,113,114,115,117,118,122,124,128,129,137,147,149,150,151,152,153,155,157,160,167,170,174,176,177,178,179,180,181,191,196,198,199,203,204,207,209,217,218,221,225,234,235,236,242,244,249,251,252,253,263,276,277,282,285,286,289],"eligible_champions":[289],"training_deck":{"battlefield":9,"cards":[[24,3],[85,3],[128,2],[129,2],[221,3],[225,2],[236,3],[252,2],[253,2],[276,3],[277,3],[282,3],[285,3],[286,3],[289,3]],"chosen_champion":289,"legend":7,"name":"Ambessa training","runes":[0,0,0,5,0,7]}}],"presets":[{"id":0,"legend_id":0,"name":"Irelia / Blade Dance","cards":[[0,1],[1,1],[2,1],[3,2],[4,2],[5,3],[6,3],[7,3],[8,2],[9,1],[10,1],[11,1],[12,1],[13,1],[14,2],[15,3],[16,2],[17,3],[18,3],[19,1],[20,1],[21,2]],"chosen_champion":0,"domains":["Calm","Chaos"],"rune_counts":[6,6],"source_url":"https://playriftbound.com/en-us/news/organizedplay/los-angeles-top-decks/"},{"id":1,"legend_id":1,"name":"Master Yi / Wuju","cards":[[22,1],[2,1],[4,3],[6,3],[7,3],[8,2],[23,3],[24,1],[25,2],[26,1],[27,3],[28,3],[29,2],[30,3],[31,1],[32,3],[15,3],[21,2]],"chosen_champion":22,"domains":["Calm","Body"],"rune_counts":[5,7],"source_url":"https://playriftbound.com/en-us/news/organizedplay/los-angeles-top-decks/"},{"id":2,"legend_id":2,"name":"Kennen / Tempest","cards":[[33,1],[34,1],[35,2],[36,1],[9,2],[11,2],[37,1],[38,2],[39,3],[40,3],[14,2],[41,1],[42,3],[43,3],[16,1],[44,2],[45,3],[18,1],[46,2],[47,3],[48,1]],"chosen_champion":33,"domains":["Chaos","Order"],"rune_counts":[9,3],"source_url":"https://playriftbound.com/en-us/news/organizedplay/los-angeles-top-decks/"},{"id":3,"legend_id":3,"name":"Kai'Sa / Void Control","cards":[[49,1],[50,1],[51,1],[52,2],[53,3],[54,3],[55,3],[56,2],[57,2],[58,1],[59,2],[60,3],[61,2],[62,1],[63,3],[64,3],[65,2],[66,1],[67,1],[68,3]],"chosen_champion":49,"domains":["Fury","Mind"],"rune_counts":[7,5],"source_url":"https://playriftbound.com/en-us/news/organizedplay/vancouvers-top-decks/"}]}"###;
    let baseline: Value = serde_json::from_str(BASELINE).unwrap();
    assert!(legend_catalog::legends().len() >= baseline["legends"].as_array().unwrap().len());
    for expected in baseline["legends"].as_array().unwrap() {
        let id = expected["id"].as_u64().unwrap() as usize;
        assert_eq!(
            json!(
                eligible_cards(id)
                    .into_iter()
                    .filter(|card| *card < 290)
                    .collect::<Vec<_>>()
            ),
            expected["eligible_cards"],
            "legend {id} eligibility"
        );
        assert_eq!(
            json!(
                eligible_champions(id)
                    .into_iter()
                    .filter(|card| *card < 290)
                    .collect::<Vec<_>>()
            ),
            expected["eligible_champions"],
            "legend {id} champions"
        );
        let deck = CustomDeck::from_legend(id).unwrap();
        deck.validate().unwrap();
        assert_eq!(
            deck.to_json(),
            expected["training_deck"],
            "legend {id} training recipe"
        );
    }
    let presets = cards::presets();
    assert_eq!(presets.len(), 4);
    for expected in baseline["presets"].as_array().unwrap() {
        let id = expected["id"].as_u64().unwrap() as usize;
        let preset = &presets[id];
        assert_eq!(
            json!({"id":id,"legend_id":preset.legend_id,"name":preset.name,"cards":preset.cards,"chosen_champion":preset.chosen_champion,"domains":preset.domains.map(|domain|domain.name()),"rune_counts":preset.rune_counts,"source_url":preset.source_url}),
            *expected,
            "preset {id} source list"
        );
        CustomDeck::from_preset(id).unwrap().validate().unwrap();
    }
}
