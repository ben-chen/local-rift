//! Constructed requirements from CR 103, exercised through the public core API.
use local_rift::cards;
use local_rift::deck::{CustomDeck, eligible_cards, eligible_champions};
use local_rift::game::{Difficulty, Game, Phase};
use local_rift::legends::LegendKind;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn add(deck: &mut CustomDeck, card: usize, count: u8) {
    if let Some((_, copies)) = deck.cards.iter_mut().find(|(id, _)| *id == card) {
        *copies += count;
    } else {
        deck.cards.push((card, count));
    }
}

fn expanded(legend: usize) -> CustomDeck {
    let mut deck = CustomDeck::from_preset(legend).unwrap();
    let additions: &[usize] = match legend {
        0 => &[71, 72, 80],
        1 => &[70, 71, 76],
        2 => &[72, 73, 80],
        3 => &[69, 74, 75, 77, 78, 79],
        _ => unreachable!(),
    };
    for &id in additions {
        add(&mut deck, id, 3);
    }
    deck.name = format!("Expansion validation {legend}");
    deck.validate().unwrap();
    deck
}

fn counts(ids: impl IntoIterator<Item = usize>) -> BTreeMap<usize, usize> {
    let mut result = BTreeMap::new();
    for id in ids {
        *result.entry(id).or_default() += 1;
    }
    result
}

fn expected(deck: &CustomDeck) -> BTreeMap<usize, usize> {
    counts(
        deck.cards
            .iter()
            .flat_map(|&(id, n)| std::iter::repeat_n(id, usize::from(n))),
    )
}

fn all_cards(game: &Game, owner: usize) -> BTreeMap<usize, usize> {
    let p = &game.players[owner];
    counts(
        p.deck
            .iter()
            .chain(&p.hand)
            .chain(&p.discard)
            .chain(&p.banish)
            .copied()
            .chain(p.champion)
            .chain(p.units.iter().filter(|u| !u.token).map(|u| u.card_id))
            .chain(p.gear.iter().filter(|g| !g.token).map(|g| g.card_id))
            .chain(p.hidden.iter().map(|h| h.card_id))
            .chain(
                game.chain
                    .iter()
                    .filter(|c| c.player == owner && c.ability.is_none())
                    .map(|c| c.card_id),
            ),
    )
}

fn invariant(game: &Game, decks: &[CustomDeck; 2]) {
    for (owner, spec) in decks.iter().enumerate() {
        assert_eq!(
            all_cards(game, owner),
            expected(spec),
            "main-card multiset changed for seat {owner}, turn {}",
            game.turn
        );
        let p = &game.players[owner];
        assert_eq!(usize::from(p.total_runes()) + p.rune_deck.len(), 12);
        assert!(
            p.runes
                .iter()
                .zip(p.exhausted_runes)
                .all(|(&all, spent)| spent <= all)
        );
        assert_eq!(
            p.energy,
            p.energy_pool
                + p.runes
                    .iter()
                    .zip(p.exhausted_runes)
                    .map(|(&all, spent)| all - spent)
                    .sum::<u8>()
        );
    }
}

#[test]
fn all_presets_convert_and_every_colorless_field_is_legal_for_every_legend() {
    let presets = cards::presets();
    for (index, original) in presets.iter().enumerate() {
        let mut custom = CustomDeck::from_preset(index).unwrap();
        assert_eq!(custom.cards, original.cards);
        assert_eq!(custom.chosen_champion, original.chosen_champion);
        assert_eq!(custom.card_count(), 40);
        assert_eq!(
            custom.runes.iter().map(|&n| usize::from(n)).sum::<usize>(),
            12
        );
        for field in 0..4 {
            custom.battlefield = field;
            custom.validate().unwrap();
        }
    }
}

#[test]
fn main_deck_minimum_includes_champion_and_has_no_forty_card_maximum() {
    let mut deck = CustomDeck::from_preset(3).unwrap();
    deck.validate().unwrap();
    let row = deck.cards.iter_mut().find(|(_, n)| *n > 1).unwrap();
    let reduced_id = row.0;
    row.1 -= 1;
    assert_eq!(deck.card_count(), 39);
    assert!(deck.validate().unwrap_err().contains("at least 40"));
    add(&mut deck, reduced_id, 1);
    add(&mut deck, 69, 1);
    assert_eq!(deck.card_count(), 41);
    deck.validate().unwrap();
    add(&mut deck, 69, 2);
    assert_eq!(deck.card_count(), 43);
    deck.validate().unwrap();
}

#[test]
fn all_printed_domains_must_fit_and_a_shared_single_domain_is_not_enough() {
    let mut irelia = CustomDeck::from_preset(0).unwrap();
    assert!(eligible_cards(0).contains(&5)); // Calm + Chaos signature.
    assert!(!eligible_cards(0).contains(&39)); // Order + Chaos: Chaos alone is insufficient.
    add(&mut irelia, 39, 1);
    assert!(irelia.validate().is_err());
    let mut yi = CustomDeck::from_preset(1).unwrap();
    add(&mut yi, 74, 1); // Fury-only Incinerate.
    assert!(yi.validate().is_err());
    let full: Value =
        serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap();
    let manifest: Value = serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    let catalog = cards::cards();
    for legend in 0..4 {
        let legend_id = manifest["decks"][legend]["legend_id"].as_str().unwrap();
        let legend = full["printings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["official_id"] == legend_id)
            .unwrap();
        let allowed = legend["domains"].as_array().unwrap();
        for id in eligible_cards(
            manifest["decks"]
                .as_array()
                .unwrap()
                .iter()
                .position(|d| d["legend_id"] == legend_id)
                .unwrap(),
        ) {
            for domain in catalog[id].domains {
                assert!(
                    allowed
                        .iter()
                        .any(|d| d.as_str() == Some(&domain.name().to_lowercase()))
                );
            }
        }
    }
}

#[test]
fn only_actual_champion_tags_qualify_not_yordle_or_ordinary_unit_tags() {
    assert_eq!(eligible_champions(0), vec![0]);
    assert_eq!(eligible_champions(1), vec![22]);
    assert_eq!(eligible_champions(2), vec![33]);
    assert_eq!(eligible_champions(3), vec![49]);
    for wrong in [20, 36] {
        // Vex and Fizz share Kennen's Yordle tag, not his champion tag.
        let mut kennen = CustomDeck::from_preset(2).unwrap();
        if !kennen.cards.iter().any(|(id, _)| *id == wrong) {
            add(&mut kennen, wrong, 1);
        }
        kennen.chosen_champion = wrong;
        assert!(kennen.validate().unwrap_err().contains("champion"));
    }
    let mut kaisa = CustomDeck::from_preset(3).unwrap();
    assert!(kaisa.cards.iter().any(|(id, _)| *id == 64));
    kaisa.chosen_champion = 64; // Thousand-Tailed Watcher is not a Champion Unit.
    assert!(kaisa.validate().is_err());
    let mut irelia = CustomDeck::from_preset(0).unwrap();
    irelia.chosen_champion = 5; // A Signature spell is never a Chosen Champion Unit.
    assert!(irelia.validate().is_err());
}

#[test]
fn chosen_champion_must_be_present_and_its_copies_count_toward_three() {
    let mut deck = CustomDeck::from_preset(3).unwrap();
    deck.cards.retain(|(id, _)| *id != 49);
    add(&mut deck, 69, 3);
    assert!(deck.card_count() >= 40);
    assert!(
        deck.validate()
            .unwrap_err()
            .contains("Include your chosen champion")
    );
    add(&mut deck, 49, 4);
    assert!(deck.validate().unwrap_err().contains("Too many copies"));
    deck.cards.iter_mut().find(|(id, _)| *id == 49).unwrap().1 = 3;
    deck.validate().unwrap();
}

#[test]
fn unavailable_cards_zero_counts_duplicate_rows_and_excess_copies_are_rejected() {
    for row in [(81, 1), (usize::MAX, 1), (69, 0), (69, 4)] {
        let mut deck = CustomDeck::from_preset(3).unwrap();
        deck.cards.push(row);
        assert!(deck.validate().is_err(), "row {row:?}");
    }
    let mut deck = CustomDeck::from_preset(3).unwrap();
    deck.cards.push(deck.cards[0]);
    assert!(deck.validate().unwrap_err().contains("duplicate"));
}

#[test]
fn signatures_are_limited_and_only_allowed_for_their_champion_identity() {
    for (legend, signature) in [(0, 5), (2, 39)] {
        let mut deck = CustomDeck::from_preset(legend).unwrap();
        assert_eq!(
            deck.cards
                .iter()
                .find(|(id, _)| *id == signature)
                .unwrap()
                .1,
            3
        );
        deck.validate().unwrap();
        add(&mut deck, signature, 1);
        assert!(deck.validate().is_err());
        for other in 0..4 {
            assert_eq!(eligible_cards(other).contains(&signature), other == legend);
        }
    }
    // Only one Signature identity per supported legend currently exists.
    // Cross-name aggregate limits remain enforced by the production validator;
    // a real second supported Signature is needed to exercise that branch.
}

#[test]
fn rune_decks_have_exactly_twelve_within_legend_domains_and_allow_uneven_splits() {
    let mut deck = CustomDeck::from_preset(3).unwrap();
    for runes in [
        [12, 0, 0, 0, 0, 0],
        [0, 0, 12, 0, 0, 0],
        [1, 0, 11, 0, 0, 0],
    ] {
        deck.runes = runes;
        deck.validate().unwrap();
    }
    for runes in [
        [11, 0, 0, 0, 0, 0],
        [13, 0, 0, 0, 0, 0],
        [0, 12, 0, 0, 0, 0],
        [255; 6],
    ] {
        deck.runes = runes;
        assert!(deck.validate().is_err());
    }
}

#[test]
fn malformed_deck_json_never_reaches_game_construction() {
    let valid = CustomDeck::from_preset(3).unwrap();
    assert_eq!(CustomDeck::from_json(&valid.to_json()).unwrap(), valid);
    let json = valid.to_json();
    let mut cases = vec![Value::Null, json!([]), json!("deck")];
    for (field, bad) in [
        ("name", json!("")),
        ("name", json!("bad\nname")),
        ("name", json!("x".repeat(101))),
        ("legend", json!(4)),
        ("chosen_champion", json!(-1)),
        ("battlefield", json!(4)),
        ("cards", json!([[49]])),
        ("cards", json!([[49, 3, 1]])),
        ("cards", json!([[49, 256]])),
        ("cards", json!([[49, -1]])),
        ("cards", json!([[49.0, 1]])),
        ("cards", json!([[u64::MAX, 1]])),
        ("cards", json!(vec![vec![49, 1]; 82])),
        ("runes", json!([6, 6])),
        ("runes", json!([6, 0, 6, 0, 0, -1])),
        ("runes", json!([256, 0, 0, 0, 0, 0])),
    ] {
        let mut value = json.clone();
        value[field] = bad;
        cases.push(value);
    }
    let mut missing = json.clone();
    missing.as_object_mut().unwrap().remove("cards");
    cases.push(missing);
    let mut extra = json;
    extra["ignore_rules"] = json!(true);
    cases.push(extra);
    for bad in cases {
        assert!(
            CustomDeck::from_json(&bad).is_err(),
            "Accepted malformed deck {bad}"
        );
    }
}

#[test]
fn custom_setup_preserves_legend_and_counts_and_rematch_keeps_specs() {
    let specs = [expanded(3), expanded(1)];
    let mut game = Game::new_with_custom(
        [0, 0],
        [Some(specs[0].clone()), Some(specs[1].clone())],
        739,
        Difficulty::Tactical,
        false,
    )
    .unwrap();
    assert_eq!(game.legend[0].kind, LegendKind::KaiSa);
    assert_eq!(game.legend[0].kind.champion_id(), 49);
    assert_eq!(game.players[0].champion, Some(49));
    assert_eq!(game.mulligans_used, [false; 2]);
    assert_eq!(game.players[0].deck.len(), specs[0].card_count() - 5);
    invariant(&game, &specs);
    game.choose_starting_player(1).unwrap();
    game.choose_battlefield(0, 0).unwrap();
    game.choose_battlefield(1, 3).unwrap();
    game.mulligan_player(0, &[0, 2]).unwrap();
    game.mulligan_player(1, &[1]).unwrap();
    invariant(&game, &specs);
    game.start_match().unwrap();
    for _ in 0..20 {
        if game.winner.is_some() {
            break;
        }
        assert!(game.step_ai().unwrap());
    }
    invariant(&game, &specs);
    game.rematch(991);
    assert_eq!(game.phase, Phase::Mulligan);
    assert_eq!(game.first_player, 1);
    assert_eq!(game.battlefields, [0, 3]);
    assert_eq!(game.custom_deck(0), Some(&specs[0]));
    assert_eq!(game.custom_deck(1), Some(&specs[1]));
    invariant(&game, &specs);
}

#[test]
fn custom_seeded_setup_is_deterministic_and_rejects_invalid_specs() {
    let specs = [Some(expanded(0)), Some(expanded(3))];
    let left = Game::new_with_custom([0, 3], specs.clone(), 8181, Difficulty::Easy, false).unwrap();
    let right = Game::new_with_custom([0, 3], specs, 8181, Difficulty::Easy, false).unwrap();
    assert_eq!(left.players, right.players);
    assert_eq!(left.battlefields, right.battlefields);
    let mut invalid = expanded(3);
    invalid.runes = [0; 6];
    assert!(
        Game::new_with_custom([0, 3], [Some(invalid), None], 1, Difficulty::Easy, false).is_err()
    );
}

#[test]
fn custom_rematch_preserves_external_or_automatic_mulligan_mode() {
    let specs = [Some(expanded(0)), Some(expanded(3))];
    for automatic in [false, true] {
        let mut game =
            Game::new_with_custom([0, 3], specs.clone(), 31, Difficulty::Easy, automatic).unwrap();
        game.start_match().unwrap();
        game.rematch(812);
        let fresh =
            Game::new_with_custom([0, 3], specs.clone(), 812, Difficulty::Easy, automatic).unwrap();
        assert_eq!(
            game.players, fresh.players,
            "rematch must match fresh seeded setup"
        );
        assert_eq!(game.mulligans_used, [false, automatic]);
        assert_eq!(
            game.mulligan_player(1, &[0]).is_ok(),
            !automatic,
            "external agents retain their own opening-hand choice"
        );
    }
}

#[test]
fn native_preset_rematch_retains_automatic_opponent_mulligan() {
    let mut game = Game::new(0, 3, 31, Difficulty::Easy);
    game.start_match().unwrap();
    game.rematch(812);
    let fresh = Game::new(0, 3, 812, Difficulty::Easy);
    assert_eq!(game.players, fresh.players);
    assert_eq!(game.mulligans_used, [false, true]);
}

#[test]
fn expanded_custom_games_finish_with_card_and_rune_conservation_after_every_action() {
    let mut played = std::collections::BTreeSet::new();
    for first in 0..4 {
        for second in 0..4 {
            for (difficulty, starting) in [(Difficulty::Easy, 0), (Difficulty::Tactical, 1)] {
                let specs = [expanded(first), expanded(second)];
                let seed = 91 + (first * 4 + second) as u64;
                let mut game = Game::new_with_custom(
                    [first, second],
                    [Some(specs[0].clone()), Some(specs[1].clone())],
                    seed,
                    difficulty,
                    false,
                )
                .unwrap();
                game.choose_starting_player(starting).unwrap();
                game.start_match().unwrap();
                for step in 0..4000 {
                    invariant(&game, &specs);
                    for p in &game.players {
                        played.extend(
                            p.units
                                .iter()
                                .filter(|u| !u.token && u.card_id >= 69)
                                .map(|u| u.card_id),
                        );
                    }
                    played.extend(
                        game.chain
                            .iter()
                            .filter(|item| item.ability.is_none() && item.card_id >= 69)
                            .map(|item| item.card_id),
                    );
                    if game.winner.is_some() {
                        break;
                    }
                    let action = game.step_ai().unwrap_or_else(|error| {
                        panic!(
                            "custom {first}/{second} seed{seed} {difficulty:?} step{step}: {error}"
                        )
                    });
                    assert!(
                        action,
                        "AI stopped during unfinished custom game {first}/{second}"
                    );
                }
                assert!(
                    game.winner.is_some(),
                    "custom {first}/{second} {difficulty:?} stalled at turn{}",
                    game.turn
                );
                invariant(&game, &specs);
            }
        }
    }
    // AI choices are strategic, not a promise to play every card it draws.
    // Focused card_expansion tests exercise every new card's operation.
    assert!(
        played.iter().any(|&id| id < 74),
        "No new units were deployed"
    );
    assert!(played.iter().any(|&id| id >= 74), "No new spells were cast");
}
