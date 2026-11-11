//! C27 printed facts and deck construction; engine interactions are tested separately.
use local_rift::{
    cards::{self, CardKind, Domain},
    deck::{CustomDeck, eligible_cards, eligible_champions},
    legend_catalog::legends,
};
use serde_json::{Value, json};

#[test]
fn four_units_keep_their_printed_costs_and_attributes_separate() {
    let cards = cards::cards();
    let proof: Value = serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c27/admissions.json"
    ))
    .unwrap();
    for (offset, (official, domain, energy, power, might)) in [
        ("ogn-076-298", Domain::Calm, 6, 2, 6),
        ("rad-085-167", Domain::Body, 5, 1, 4),
        ("sfd-028-221", Domain::Fury, 3, 0, 2),
        ("rad-078-167", Domain::Body, 6, 1, 4),
    ]
    .into_iter()
    .enumerate()
    {
        let card = cards[413 + offset];
        let record = &proof["records"][offset];
        assert_eq!((card.id, card.official_id), (413 + offset, official));
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
    assert!(cards[413].official_text.contains("equal to my Might"));
    assert!(cards[414].official_text.contains("all enemy units here"));
    assert!(
        cards[414]
            .official_text
            .contains("I don't deal combat damage.")
    );
    assert!(cards[415].official_text.contains("equal to my [Assault]"));
    assert!(proof["records"][2]["effective"]["power"].is_null());
    assert_eq!(cards[416].tags, &["Noxus"]);
    assert!(cards[416].official_text.contains("equal to my [Shield]"));
    assert_eq!(proof["records"][3]["raw_printing"]["tags"], json!([]));
    assert_eq!(proof["records"][3]["effective"]["super_types"], json!([]));
}

#[test]
fn exact_legal_forty_card_decks_accept_only_matching_domains_and_no_new_chosen_champions() {
    let homes: [&[usize]; 4] = [&[0, 1], &[1, 7, 8], &[3, 4, 8], &[1, 7, 8]];
    for legend in legends() {
        let original = CustomDeck::from_legend(legend.id).unwrap();
        assert_eq!(original.card_count(), 40);
        assert!(original.cards.iter().all(|(id, _)| *id < 413));
        for id in 413..417 {
            let legal = homes[id - 413].contains(&legend.id);
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
    assert!(
        cards::presets()
            .iter()
            .take(4)
            .all(|deck| deck.cards.iter().all(|(id, _)| *id < 413))
    );
}

#[test]
fn all_four_runtime_faces_are_full_proportioned_and_bounded() {
    for bytes in [
        include_bytes!("../assets/art/card_413.png").as_slice(),
        include_bytes!("../assets/art/card_414.png").as_slice(),
        include_bytes!("../assets/art/card_415.png").as_slice(),
        include_bytes!("../assets/art/card_416.png").as_slice(),
    ] {
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(bytes.len() < 300_000);
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        assert_eq!((width, height), (280, 391));
    }
}
