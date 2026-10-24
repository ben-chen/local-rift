//! Registry, saved-deck and bot-protocol boundaries for independently selected legends.
use local_rift::api::{ResetOptions, Session, catalog};
use local_rift::cards::presets;
use local_rift::deck::{CustomDeck, eligible_cards, eligible_champions};
use local_rift::game::{Action, Difficulty, Game, Phase};
use local_rift::legend_catalog::{legend, legends};
use serde_json::json;

#[test]
fn every_registered_legend_has_a_valid_editable_training_list() {
    for definition in legends() {
        let deck = CustomDeck::from_legend(definition.id).unwrap();
        deck.validate().unwrap();
        assert_eq!(deck.legend, definition.id);
        assert_eq!(deck.chosen_champion, definition.default_champion);
        assert_eq!(deck.card_count(), 40);
        assert_eq!(
            deck.runes.iter().map(|&n| usize::from(n)).sum::<usize>(),
            12
        );
        assert!(deck.name.ends_with(" training"));
        assert!(eligible_champions(definition.id).contains(&deck.chosen_champion));
        assert_eq!(CustomDeck::from_json(&deck.to_json()).unwrap(), deck);
    }
    assert!(CustomDeck::from_legend(usize::MAX).is_err());
    for (index, preset) in presets().iter().enumerate() {
        let copy = CustomDeck::from_preset(index).unwrap();
        assert_eq!(copy.legend, preset.legend_id);
        assert_eq!(copy.cards, preset.cards, "source presets must not change");
        assert_eq!(copy.chosen_champion, preset.chosen_champion);
    }
}

#[test]
fn official_legend_imports_normalize_to_legacy_stable_numeric_saves() {
    for definition in legends() {
        let deck = CustomDeck::from_legend(definition.id).unwrap();
        let mut imported = deck.to_json();
        imported["legend"] = json!(definition.official_id);
        let parsed = CustomDeck::from_json(&imported).unwrap();
        assert_eq!(parsed, deck);
        assert_eq!(parsed.to_json()["legend"], definition.id);
    }
    let mut bad = CustomDeck::from_legend(4).unwrap().to_json();
    for identity in [
        json!("unknown"),
        json!("4"),
        json!(-1),
        json!(4.5),
        json!(true),
    ] {
        bad["legend"] = identity;
        assert!(CustomDeck::from_json(&bad).is_err());
    }
}

#[test]
fn custom_legend_and_chosen_champion_are_independent_of_preset_selection() {
    let mut viktor = CustomDeck::from_legend(5).unwrap();
    assert!(eligible_champions(5).contains(&176));
    viktor.chosen_champion = 176; // Leader rather than default Innovator.
    viktor.battlefield = 3;
    viktor.validate().unwrap();
    let lux = CustomDeck::from_legend(6).unwrap();
    let mut game = Game::new_with_custom(
        [2, 0],
        [Some(viktor.clone()), Some(lux.clone())],
        811,
        Difficulty::Strategic,
        false,
    )
    .unwrap();
    assert_eq!([game.legend_id(0), game.legend_id(1)], [5, 6]);
    assert_eq!(game.legend_def(0).official_id, "ogn-265-298");
    assert_eq!(game.players[0].champion, Some(176));
    assert_eq!(game.players[1].champion, Some(182));
    assert_eq!(game.players[0].name, viktor.name);
    assert_eq!(game.players[1].name, lux.name);
    assert_eq!(game.battlefields, [3, lux.battlefield]);
    assert_eq!(game.mulligans_used, [false, false]);
    game.choose_starting_player(1).unwrap();
    game.start_match().unwrap();
    game.rematch(812);
    assert_eq!(game.phase, Phase::Mulligan);
    assert_eq!(game.first_player, 1);
    assert_eq!([game.legend_id(0), game.legend_id(1)], [5, 6]);
    assert_eq!(game.players[0].champion, Some(176));
    assert_eq!(game.mulligans_used, [false, false]);
    assert_eq!(game.custom_deck(0), Some(&viktor));
    assert_eq!(game.custom_deck(1), Some(&lux));
    assert_eq!(game.battlefields, [3, lux.battlefield]);
}

#[test]
fn new_legend_construction_rejects_wrong_tags_domains_and_runes() {
    let darius = CustomDeck::from_legend(4).unwrap();
    let mut wrong = darius.clone();
    wrong.chosen_champion = 49; // Kai'Sa shares Fury, not the Champion Tag.
    wrong.cards.push((49, 1));
    assert!(wrong.validate().unwrap_err().contains("champion"));
    wrong = darius.clone();
    wrong.cards.push((61, 1)); // Mind-only Singularity.
    assert!(!eligible_cards(4).contains(&61));
    assert!(wrong.validate().unwrap_err().contains("domains"));
    wrong = darius;
    wrong.runes[0] -= 1;
    wrong.runes[2] += 1;
    assert!(wrong.validate().unwrap_err().contains("runes"));
    let mut lux = CustomDeck::from_legend(6).unwrap();
    lux.chosen_champion = 175; // Same domains, different champion identity.
    lux.cards.push((175, 1));
    assert!(lux.validate().unwrap_err().contains("champion"));
}

#[test]
fn catalog_separates_legends_from_presets_and_exposes_training_templates() {
    let catalog = catalog();
    assert_eq!(catalog["presets"].as_array().unwrap().len(), 4);
    assert_eq!(
        catalog["legends"].as_array().unwrap().len(),
        legends().len()
    );
    for definition in legends() {
        let row = &catalog["legends"][definition.id];
        assert_eq!(row["official_id"], definition.official_id);
        assert_eq!(row["official_text"], definition.text);
        assert!(
            row["eligible_champions"]
                .as_array()
                .unwrap()
                .contains(&json!(definition.default_champion))
        );
        let deck = CustomDeck::from_json(&row["training_deck"]).unwrap();
        assert_eq!(deck.legend, definition.id);
    }
    for (id, row) in catalog["presets"].as_array().unwrap().iter().enumerate() {
        assert_eq!(row["legend_id"], id);
    }
}

#[test]
fn api_exposes_actual_legend_identity_without_leaking_opponent_custom_list() {
    let first = CustomDeck::from_legend(4).unwrap();
    let second = CustomDeck::from_legend(6).unwrap();
    let mut session = Session::new(ResetOptions {
        decks: [0, 2],
        custom_decks: [Some(first), Some(second)],
        ..ResetOptions::default()
    })
    .unwrap();
    for viewer in 0..2 {
        let observation = session.observe(viewer).unwrap();
        for (owner, id) in [(0, 4), (1, 6)] {
            let player = &observation["players"][owner];
            assert_eq!(player["legend_id"], id);
            assert_eq!(player["legend"]["id"], id);
            assert_eq!(
                player["legend"]["official_id"],
                legend(id).unwrap().official_id
            );
            assert!(player["preset"].is_null());
            assert_eq!(player["custom_deck"].is_null(), owner != viewer);
            assert_eq!(player["hand"].is_null(), owner != viewer);
        }
    }
    let before = session.observe(0).unwrap();
    let mut invalid = CustomDeck::from_legend(4).unwrap().to_json();
    invalid["legend"] = json!("ogn-265-298"); // Changing legend cannot legalize Fury cards.
    let response = session.handle(json!({"command":"reset","revision":0,
        "custom_decks":[invalid,null]}));
    assert_eq!(response["ok"], false);
    assert_eq!(session.revision(), 0);
    assert_eq!(session.observe(0).unwrap(), before);
}

#[test]
fn viktor_chain_uses_canonical_legend_source_and_defers_token_placement() {
    let mut session = Session::new(ResetOptions {
        custom_decks: [Some(CustomDeck::from_legend(5).unwrap()), None],
        ..ResetOptions::default()
    })
    .unwrap();
    session.start(0).unwrap();
    session
        .apply_action(
            0,
            session.revision(),
            Action::ActivateLegend { target: None },
        )
        .unwrap();
    let pending = session.observe(0).unwrap();
    assert_eq!(pending["chain"].as_array().unwrap().len(), 1);
    assert_eq!(
        pending["chain"][0]["source"],
        json!({"type":"legend","player":0,"legend_id":5})
    );
    assert_eq!(pending["chain"][0]["ability"]["type"], "viktor_recruit");
    assert!(
        pending["chain"][0]["name"]
            .as_str()
            .unwrap()
            .contains("Herald")
    );
    assert!(
        pending["pending_token_plays"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        pending["players"][0]["units"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for _ in 0..2 {
        let priority = session.observe(0).unwrap()["priority"].as_u64().unwrap() as usize;
        session
            .apply_action(priority, session.revision(), Action::Pass)
            .unwrap();
    }
    let resolved = session.observe(0).unwrap();
    assert!(resolved["chain"].as_array().unwrap().is_empty());
    assert_eq!(resolved["pending_token_plays"][0]["might"], 1);
    assert_eq!(
        resolved["pending_token_plays"][0]["destination"]["mode"],
        "choose"
    );
    assert!(
        resolved["players"][0]["units"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
