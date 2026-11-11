//! C26 source facts and legal deck homes; engine behavior has separate tests.
use local_rift::{
    cards::{self, CardKind, Domain},
    deck::{CustomDeck, eligible_cards, eligible_champions},
    legend_catalog::legends,
};
use serde_json::{Value, json};

fn proof() -> Value {
    serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c26/admissions.json"
    ))
    .unwrap()
}

#[test]
fn both_champion_units_preserve_reviewed_printed_stats_and_clauses() {
    let cards = cards::cards();
    let proof = proof();
    for (offset, (official, domain, energy)) in [
        ("sfd-025-221", Domain::Fury, 3),
        ("sfd-082-221", Domain::Mind, 4),
    ]
    .into_iter()
    .enumerate()
    {
        let card = cards[411 + offset];
        let record = &proof["records"][offset];
        assert_eq!((card.id, card.official_id), (411 + offset, official));
        assert_eq!((card.kind, card.domain), (CardKind::Unit, domain));
        assert_eq!(card.domains, &[domain]);
        assert_eq!((card.cost, card.power_cost, card.might), (energy, 1, 3));
        assert_eq!(record["effective"]["super_types"], json!(["champion"]));
        assert_eq!(record["effective"]["rules_text"], card.official_text);
        assert_eq!(record["effective"]["tags"], json!(card.tags));
        assert_eq!(record["runtime_image"]["url"], card.art_url);
    }
    assert!(cards[411].official_text.starts_with("[Reaction]"));
    assert!(cards[411].official_text.contains("[Assault 2]"));
    assert!(cards[412].official_text.contains("[Mind Power]: [Action]"));
    assert!(!cards[412].official_text.starts_with("[Action]"));
    assert!(
        cards[412]
            .official_text
            .contains("I don't deal combat damage.")
    );
}

#[test]
fn main_deck_domains_and_chosen_champion_tags_have_distinct_eligibility() {
    let homes: [&[usize]; 2] = [&[3, 4, 8], &[3, 5, 6, 9]];
    for legend in legends() {
        let original = CustomDeck::from_legend(legend.id).unwrap();
        assert_eq!(original.card_count(), 40);
        assert!(original.cards.iter().all(|(id, _)| *id < 411));
        for id in 411..413 {
            let legal = homes[id - 411].contains(&legend.id);
            let chosen = legend.id == [8, 9][id - 411];
            assert_eq!(eligible_cards(legend.id).contains(&id), legal);
            assert_eq!(eligible_champions(legend.id).contains(&id), chosen);
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
                assert_eq!(deck.validate().is_ok(), chosen);
            }
        }
        assert_eq!(CustomDeck::from_legend(legend.id).unwrap(), original);
    }
    assert!(
        cards::presets()
            .iter()
            .take(4)
            .all(|deck| deck.cards.iter().all(|(id, _)| *id < 411))
    );
}

#[test]
fn all_four_printings_preserve_champion_identity_without_new_corrections() {
    let proof = proof();
    assert_eq!(
        proof["records"][0]["printing_cohort"],
        json!(["sfd-025-221"])
    );
    assert_eq!(
        proof["records"][1]["printing_cohort"],
        json!(["sfd-082-221", "sfd-082a-221", "rad-180-167"])
    );
    for record in proof["records"].as_array().unwrap() {
        assert_eq!(record["errata"], json!([]));
        for face in std::iter::once(record).chain(record["alternate_printings"].as_array().unwrap())
        {
            assert_eq!(face["effective"]["super_types"], json!(["champion"]));
            assert_eq!(face["effective"]["might"], 3);
            assert_eq!(face["existing_field_resolution_ids"], json!([]));
            assert_eq!(
                face["effective"]["rules_text"],
                record["effective"]["rules_text"]
            );
        }
    }
}

#[test]
fn both_runtime_faces_retain_full_proportions_at_bounded_size() {
    for bytes in [
        include_bytes!("../assets/art/card_411.png").as_slice(),
        include_bytes!("../assets/art/card_412.png").as_slice(),
    ] {
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(bytes.len() < 300_000);
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        assert_eq!((width, height), (280, 391));
    }
}
