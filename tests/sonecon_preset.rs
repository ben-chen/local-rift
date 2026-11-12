use local_rift::{
    cards::presets,
    deck::CustomDeck,
    game::{Difficulty, Game},
    match_series::{MatchSeries, Registration, SeriesPhase},
};
use std::collections::BTreeMap;

#[test]
fn sonecon_registration_is_exact_and_uses_reviewed_starting_fields() {
    let p = presets().remove(6);
    assert_eq!((p.legend_id, p.chosen_champion), (10, 439));
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../assets/data/preset_admissions/riven_stuttgart_2026_11/preset.json"
    ))
    .unwrap();
    for (actual, key) in [
        (&p.cards, "cards"),
        (&p.registration.unwrap().sideboard.to_vec(), "sideboard"),
    ] {
        let expected: Vec<(usize, u8)> = proof[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                (
                    v["id"].as_u64().unwrap() as usize,
                    v["count"].as_u64().unwrap() as u8,
                )
            })
            .collect();
        assert_eq!(*actual, expected);
    }
    let reg = Registration::from_preset(6).unwrap();
    reg.validate().unwrap();
    assert_eq!(reg.battlefields, [11, 7, 12]);
    assert_eq!(reg.deck.battlefield, 11);
    assert_eq!(reg.deck.runes, [0, 6, 0, 0, 6, 0]);
    assert_eq!(p.registration.unwrap().pilot, "Sonecon");
    assert_eq!(p.registration.unwrap().placing, 2);
    assert_ne!(CustomDeck::from_legend(10).unwrap().cards, reg.deck.cards);
    let api = local_rift::api::catalog();
    assert_eq!(
        api["presets"][6]["registration"]["battlefields"],
        serde_json::json!([11, 7, 12])
    );
    assert_eq!(api["presets"][6]["legend_id"], 10);
    for seat in 0..2 {
        let decks = if seat == 0 { [6, 5] } else { [5, 6] };
        let game = Game::new_for_agents(decks[0], decks[1], 441006, Difficulty::Easy);
        let player = &game.players[seat];
        assert_eq!(player.champion, Some(439));
        assert_eq!(player.hand.len() + player.deck.len(), 39);
        let mut expected: BTreeMap<_, _> = p
            .cards
            .iter()
            .map(|&(id, n)| (id, usize::from(n)))
            .collect();
        expected.remove(&439);
        let mut actual = BTreeMap::new();
        for &id in player.hand.iter().chain(&player.deck) {
            *actual.entry(id).or_default() += 1usize;
        }
        assert_eq!(actual, expected);
        assert_eq!(game.battlefields[seat], 11);
    }
}

fn finish(game: &mut Game, label: &str) {
    game.start_match().unwrap();
    for _ in 0..3200 {
        game.validate_numeric_state().unwrap();
        for player in &game.players {
            assert_eq!(
                usize::from(player.total_runes()) + player.rune_deck.len(),
                12,
                "{label}"
            );
            for unit in &player.units {
                assert!(
                    unit.lane.is_none_or(|lane| lane < game.battlefield_count()),
                    "{label}"
                );
            }
        }
        if game.winner.is_some() {
            return;
        }
        assert!(game.step_ai().unwrap_or_else(|e| panic!("{label}: {e}")));
    }
    panic!("Unfinished game: {label}");
}

#[test]
fn sonecon_completes_all_presets_difficulties_seats_and_starting_players() {
    let mut completed = 0;
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for opponent in 0..presets().len() {
            for seat in 0..2 {
                for first in 0..2 {
                    let decks = if seat == 0 {
                        [6, opponent]
                    } else {
                        [opponent, 6]
                    };
                    let seed = 441000 + (level * 100 + opponent * 4 + seat * 2 + first) as u64;
                    let mut game = Game::new_for_agents(decks[0], decks[1], seed, difficulty);
                    game.choose_battlefield(
                        seat,
                        [11, 7, 12][(level + opponent + seat + first) % 3],
                    )
                    .unwrap();
                    game.choose_starting_player(first).unwrap();
                    finish(
                        &mut game,
                        &format!(
                            "preset {opponent}, {difficulty:?}, seat {seat}, first {first}, seed {seed}"
                        ),
                    );
                    completed += 1;
                }
            }
        }
    }
    assert_eq!(completed, presets().len() * 12);
    println!("Completed {completed} exact Sonecon games.");
}

#[test]
fn sonecon_completes_registered_matches_with_real_sideboard_changes() {
    let mut completed = 0;
    let mut games = 0;
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for seat in 0..2 {
            for first in 0..2 {
                let regs = if seat == 0 { [6, 5] } else { [5, 6] }
                    .map(|id| Registration::from_preset(id).unwrap());
                let mut series = MatchSeries::new(regs, first).unwrap();
                while series.phase() != SeriesPhase::Complete {
                    if series.phase() == SeriesPhase::Sideboarding {
                        for player in 0..2 {
                            let mut deck =
                                CustomDeck::from_json(&series.observe(player).unwrap()["deck"])
                                    .unwrap();
                            if player == seat && series.game_number() == 2 {
                                let baron =
                                    deck.cards.iter_mut().find(|(id, _)| *id == 440).unwrap();
                                baron.1 -= 1;
                                deck.cards.retain(|(_, n)| *n > 0);
                                deck.cards.push((435, 1));
                            }
                            series
                                .submit_sideboard(player, deck.cards, deck.chosen_champion)
                                .unwrap();
                        }
                    }
                    for player in 0..2 {
                        let view = series.observe(player).unwrap();
                        let field = view["available_battlefields"][0].as_u64().unwrap() as usize;
                        series.choose_battlefield(player, field).unwrap();
                    }
                    let chooser = series.summary()["designated_player"].as_u64().unwrap() as usize;
                    series.choose_play_order(chooser, first).unwrap();
                    let seed = 441600
                        + (level * 100 + seat * 10 + first * 4) as u64
                        + series.game_number();
                    let mut game = series.new_game(seed, difficulty).unwrap();
                    finish(
                        &mut game,
                        &format!("series {difficulty:?}, seat {seat}, first {first}, seed {seed}"),
                    );
                    series.finish_completed_game(&game).unwrap();
                    games += 1;
                }
                assert!(
                    series.summary()["wins"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|wins| wins == 2)
                );
                completed += 1;
            }
        }
    }
    assert_eq!(completed, 12);
    assert!((24..=36).contains(&games));
    println!("Completed {completed} registered Sonecon matches across {games} games.");
}
