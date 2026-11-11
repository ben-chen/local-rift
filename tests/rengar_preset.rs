use local_rift::{
    battlefields::default_battlefield,
    cards::{cards, presets},
    deck::CustomDeck,
    game::{Difficulty, Game},
};
use serde_json::Value;

#[test]
fn rengar_winning_preset_has_exact_source_cards_and_distinct_legend_id() {
    let evidence: Value = serde_json::from_str(include_str!(
        "../assets/data/preset_admissions/rengar_2026_09/preset.json"
    ))
    .unwrap();
    let presets = presets();
    let preset = &presets[4];
    assert_eq!(preset.legend_id, 8);
    assert_eq!(preset.chosen_champion, 30);
    assert_eq!(preset.rune_counts, [4, 8]);
    assert_eq!(
        preset
            .cards
            .iter()
            .map(|(_, n)| usize::from(*n))
            .sum::<usize>(),
        40
    );
    let catalog = cards();
    let rows = evidence["cards"].as_array().unwrap();
    assert_eq!(preset.cards.len(), rows.len());
    for ((id, count), row) in preset.cards.iter().zip(rows) {
        assert_eq!(*id, row["id"].as_u64().unwrap() as usize);
        assert_eq!(u64::from(*count), row["count"].as_u64().unwrap());
        assert_eq!(catalog[*id].name, row["name"]);
    }
    assert_eq!(preset.name, evidence["name"]);
    assert_eq!(preset.learning_notes, evidence["learning_notes"]);
    let custom = CustomDeck::from_preset(4).unwrap();
    assert_eq!(custom.legend, 8);
    assert_eq!(custom.battlefield, 1);
    assert_eq!(default_battlefield(4), 1);
}

#[test]
fn rengar_preset_requires_all_its_card_rules() {
    // This is a runtime admission gate, separate from source-list validation.
    // It must pass on the combined engine before this preset can be published.
    CustomDeck::from_preset(4).unwrap().validate().unwrap();
}

#[test]
fn adding_a_competitive_preset_keeps_all_ten_training_recipes_unchanged() {
    let baseline: Value = serde_json::from_str(include_str!(
        "../assets/data/preset_admissions/rengar_2026_09/predecessor_training_decks.json"
    ))
    .unwrap();
    for (id, old) in baseline["training_decks"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let training = CustomDeck::from_legend(id).unwrap();
        assert_eq!(
            training.to_json(),
            *old,
            "legend {id} training recipe changed"
        );
    }
}

#[test]
fn rengar_preset_opens_from_either_seat_with_champion_removed_once() {
    for difficulty in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        for seat in 0..2 {
            let mut indices = [0, 0];
            indices[seat] = 4;
            let game = Game::new_for_agents(indices[0], indices[1], 280028, difficulty);
            let player = &game.players[seat];
            assert_eq!(game.legend_id(seat), 8);
            assert_eq!(player.champion, Some(30));
            assert_eq!(player.hand.len() + player.deck.len(), 39);
            assert!(!player.hand.contains(&30));
            assert!(!player.deck.contains(&30));
            assert_eq!(game.battlefields[seat], 1);
            assert_eq!(
                player.rune_deck.len() + usize::from(player.total_runes()),
                12
            );
        }
    }
}

#[test]
fn api_keeps_the_authored_rengar_recipe_distinct_from_the_tournament_preset() {
    let catalog = local_rift::api::catalog();
    let training = CustomDeck::from_legend(8).unwrap();
    let tournament = CustomDeck::from_preset(4).unwrap();
    assert_ne!(training.cards, tournament.cards);
    assert_eq!(catalog["legends"][8]["training_deck"], training.to_json());
    assert_eq!(
        catalog["legends"][8]["training_note"],
        "Authored practice list; not a tournament or meta recommendation."
    );
    for preset in presets().iter().take(4) {
        assert_eq!(
            catalog["legends"][preset.legend_id]["training_note"],
            "Editable copy of the sourced preset; the original preset remains unchanged."
        );
    }
}
