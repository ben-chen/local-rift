//! Actual played best-of-three, using complete source registrations and sideboard pools.
use local_rift::{
    deck::CustomDeck,
    game::Difficulty,
    match_ai::{MatchPolicy, Scouting, recommend},
    match_series::{MatchSeries, Registration, SeriesPhase},
};

fn exercise(level: usize, difficulty: Difficulty) {
    let mut games = 0;
    let mut seen = std::collections::BTreeSet::new();
    for rival in [0, 2, 4] {
        for irelia_seat in 0..2 {
            let presets = if irelia_seat == 0 {
                [5, rival]
            } else {
                [rival, 5]
            };
            let regs = presets.map(|p| Registration::from_preset(p).unwrap());
            let mut series = MatchSeries::new(regs.clone(), 1 - irelia_seat).unwrap();
            let mut scouts: [Scouting; 2] = Default::default();
            let seed = 1134000 + (level * 100 + rival * 10 + irelia_seat) as u64;
            while series.phase() != SeriesPhase::Complete {
                assert!(
                    series.game_number() <= 8,
                    "series must finish: {difficulty:?}/{presets:?}"
                );
                if series.phase() == SeriesPhase::Sideboarding {
                    for player in 0..2 {
                        let view = series.observe(player).unwrap();
                        let mut deck = recommend(
                            &view,
                            player,
                            &scouts[player],
                            difficulty,
                            MatchPolicy::Adaptive,
                        )
                        .unwrap()
                        .deck;
                        // Explicit human-style swaps exercise each newly completed sideboard
                        // even if its heuristic would keep it out in this particular matchup.
                        let swap = match presets[player] {
                            2 => Some((44, 434)),
                            4 => Some((32, 433)),
                            _ => None,
                        };
                        if let Some((out, inside)) = swap
                            && !deck.cards.iter().any(|&(id, n)| id == inside && n > 0)
                        {
                            let row = deck.cards.iter_mut().find(|(id, _)| *id == out).unwrap();
                            row.1 -= 1;
                            deck.cards.retain(|(_, n)| *n > 0);
                            deck.cards.push((inside, 1));
                        }
                        regs[player]
                            .configure(deck.cards.clone(), deck.chosen_champion)
                            .unwrap();
                        series
                            .submit_sideboard(player, deck.cards, deck.chosen_champion)
                            .unwrap();
                    }
                }
                for player in 0..2 {
                    let view = series.observe(player).unwrap();
                    let fields = view["available_battlefields"].as_array().unwrap();
                    let id = fields[(rival + level + player) % fields.len()]
                        .as_u64()
                        .unwrap() as usize;
                    series.choose_battlefield(player, id).unwrap();
                }
                let chooser = series.summary()["designated_player"].as_u64().unwrap() as usize;
                series.choose_play_order(chooser, chooser).unwrap();
                let expected: [CustomDeck; 2] = std::array::from_fn(|p| {
                    CustomDeck::from_json(&series.observe(p).unwrap()["deck"]).unwrap()
                });
                let mut g = series
                    .new_game(seed + series.game_number(), difficulty)
                    .unwrap();
                g.start_match().unwrap();
                for step in 0..6000 {
                    for item in &g.chain {
                        if [433, 434].contains(&item.card_id) {
                            seen.insert(item.card_id);
                        }
                    }
                    for (p, scout) in scouts.iter_mut().enumerate() {
                        scout.observe_game(&g, p).unwrap();
                    }
                    if g.winner.is_some() {
                        break;
                    }
                    assert!(g.step_ai().unwrap_or_else(|e| panic!(
                        "{difficulty:?}/{presets:?} game {} step {step}: {e}",
                        series.game_number()
                    )));
                }
                assert!(g.winner.is_some(), "{difficulty:?}/{presets:?}");
                for (player, deck) in expected.iter().enumerate() {
                    // Newly added spells must remain one physical copy across every zone.
                    for id in [433, 434] {
                        let p = &g.players[player];
                        let actual = p
                            .hand
                            .iter()
                            .chain(&p.deck)
                            .chain(&p.discard)
                            .chain(&p.banish)
                            .filter(|&&x| x == id)
                            .count()
                            + g.chain
                                .iter()
                                .filter(|c| {
                                    c.player == player && c.card_id == id && c.ability.is_none()
                                })
                                .count();
                        let count = deck
                            .cards
                            .iter()
                            .find(|(x, _)| *x == id)
                            .map_or(0, |(_, n)| usize::from(*n));
                        assert_eq!(actual, count);
                    }
                }
                series.finish_completed_game(&g).unwrap();
                games += 1;
            }
            assert!(series.summary()["winner"].is_number());
        }
    }
    println!(
        "Registered matches: {difficulty:?}, 6 completed matches, {games} played games, new spells seen {seen:?}"
    );
}
#[test]
fn easy_registered_matches() {
    exercise(0, Difficulty::Easy);
}
#[test]
fn tactical_registered_matches() {
    exercise(1, Difficulty::Tactical);
}
#[test]
fn strategic_registered_matches() {
    exercise(2, Difficulty::Strategic);
}
