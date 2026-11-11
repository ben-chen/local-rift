//! C28 source facts and construction boundaries; no executable effect claim.
use local_rift::{
    battlefields::BATTLEFIELDS,
    cards::{self, CardKind, Domain},
    deck::{CustomDeck, eligible_cards, eligible_champions},
    legend_catalog::legends,
};
use serde_json::{Value, json};

fn proof() -> Value {
    serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c28/admissions.json"
    ))
    .unwrap()
}

#[test]
fn four_units_keep_exact_printed_stats_tags_text_and_optional_costs() {
    let catalog = cards::cards();
    let proof = proof();
    for (offset, (official, domain, energy, power, might)) in [
        ("unl-112-219", Domain::Body, 2, 0, 1),
        ("unl-097-219", Domain::Body, 3, 0, 3),
        ("unl-114-219", Domain::Body, 3, 1, 4),
        ("unl-028-219", Domain::Fury, 3, 0, 2),
    ]
    .into_iter()
    .enumerate()
    {
        let card = catalog[417 + offset];
        let record = &proof["records"][offset];
        assert_eq!((card.id, card.official_id), (417 + offset, official));
        assert_eq!((card.kind, card.domain), (CardKind::Unit, domain));
        assert_eq!(card.domains, &[domain]);
        assert_eq!(
            (card.cost, card.power_cost, card.might),
            (energy, power, might)
        );
        assert_eq!(record["effective"]["rules_text"], card.official_text);
        assert_eq!(record["effective"]["tags"], json!(card.tags));
        assert_eq!(record["runtime_image"]["url"], card.art_url);
    }
    assert!(catalog[417].official_text.contains("an enemy unit"));
    assert!(catalog[418].official_text.contains("your other units"));
    assert!(catalog[419].official_text.contains("[Ambush]"));
    assert!(catalog[420].official_text.contains("[Hidden]"));
    assert!(catalog[420].official_text.contains("[Ganking]"));
    assert!(
        catalog[420]
            .official_text
            .contains("[Fury Power] as an additional cost")
    );
    assert!(
        catalog[420]
            .official_text
            .contains("if you paid the additional cost")
    );
    for offset in [0, 1, 3] {
        assert!(proof["records"][offset]["effective"]["power"].is_null());
    }
}

#[test]
fn battlefield_registry_uses_errata_without_rewriting_preserved_faces() {
    let proof = proof();
    let fields: Value =
        serde_json::from_str(include_str!("../assets/data/battlefields.json")).unwrap();
    for (offset, official) in [
        "sfd-207-221",
        "unl-215-219",
        "unl-205-219",
        "ogn-289-298",
        "ven-165-166",
        "ogn-298-298",
    ]
    .into_iter()
    .enumerate()
    {
        let field = BATTLEFIELDS[11 + offset];
        let record = &proof["battlefield_records"][offset];
        let source = &fields["battlefields"][11 + offset];
        assert_eq!((field.id, field.source_id), (11 + offset, official));
        assert_eq!(record["runtime_rules_text"], field.official_text);
        assert_eq!(record["runtime_rules_source"], field.source_url);
        assert_eq!(record["effective"]["rules_text"], source["official_text"]);
        if offset == 0 || offset == 3 {
            assert_ne!(source["official_text"], source["corrected_text"]);
            assert_eq!(source["corrected_text"], field.official_text);
            assert_eq!(source["errata_url"], field.source_url);
        } else {
            assert_eq!(source["official_text"], field.official_text);
        }
    }
    assert!(
        BATTLEFIELDS[11]
            .official_text
            .contains("hand to play a 2 Might")
    );
    assert!(
        BATTLEFIELDS[14]
            .official_text
            .contains("ready up to 2 runes")
    );
    assert_eq!(
        BATTLEFIELDS[16].official_text,
        "When you conquer here, discard 1, then draw 1."
    );
}

#[test]
fn exact_legal_forty_card_decks_have_matching_domains_and_no_new_chosen_champion() {
    for legend in legends() {
        let original = CustomDeck::from_legend(legend.id).unwrap();
        assert_eq!(original.card_count(), 40);
        for id in 417..421 {
            let homes: &[usize] = if id == 420 { &[3, 4, 8] } else { &[1, 7, 8] };
            let legal = homes.contains(&legend.id);
            assert_eq!(eligible_cards(legend.id).contains(&id), legal);
            assert!(!eligible_champions(legend.id).contains(&id));
            let mut deck = original.clone();
            let entry = deck
                .cards
                .iter_mut()
                .find(|(card, count)| *card != original.chosen_champion && *count > 0)
                .unwrap();
            entry.1 -= 1;
            deck.cards.retain(|(_, count)| *count > 0);
            deck.cards.push((id, 1));
            assert_eq!(deck.card_count(), 40);
            assert_eq!(deck.validate().is_ok(), legal);
            if legal {
                assert_eq!(CustomDeck::from_json(&deck.to_json()).unwrap(), deck);
                deck.chosen_champion = id;
                assert!(deck.validate().is_err());
            }
        }
        assert_eq!(CustomDeck::from_legend(legend.id).unwrap(), original);
    }
    assert!(cards::presets().len() >= 4);
    assert!(
        cards::presets()
            .iter()
            .take(4)
            .all(|deck| deck.cards.iter().all(|(id, _)| *id < 417))
    );
}
