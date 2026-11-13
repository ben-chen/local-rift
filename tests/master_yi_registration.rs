//! Exact registration and public-evidence sideboarding for Master Yi match practice.
use local_rift::{
    cards::{cards, presets},
    deck::CustomDeck,
    game::{Difficulty, Game, Unit},
    match_ai::{MatchPolicy, Scouting, recommend},
    match_series::{MatchSeries, Registration, SeriesPhase},
};

fn sideboarding() -> MatchSeries {
    let mut s = MatchSeries::new(
        [
            Registration::from_preset(1).unwrap(),
            Registration::from_preset(5).unwrap(),
        ],
        0,
    )
    .unwrap();
    s.choose_battlefield(0, 11).unwrap();
    s.choose_battlefield(1, 13).unwrap();
    s.choose_play_order(0, 0).unwrap();
    s.concede_game(1).unwrap();
    s
}

fn revealed(name: &str) -> Scouting {
    let mut g = Game::new_for_agents(1, 5, 443910, Difficulty::Tactical);
    g.players[1].champion = None;
    g.players[1].units.clear();
    g.players[1].gear.clear();
    g.players[1].discard = vec![cards().iter().find(|c| c.name == name).unwrap().id];
    let mut scout = Scouting::default();
    scout.observe_game(&g, 0).unwrap();
    scout
}

fn small_board() -> Scouting {
    let mut g = Game::new_for_agents(1, 5, 443911, Difficulty::Tactical);
    g.players[1].champion = None;
    g.players[1].units = (0..2)
        .map(|i| Unit {
            id: 9000 + i,
            card_id: 25,
            lane: Some(0),
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        })
        .collect();
    let mut scout = Scouting::default();
    scout.observe_game(&g, 0).unwrap();
    scout
}

#[test]
fn master_yi_registration_preserves_the_main_and_restricts_swaps_to_its_pool() {
    let r = Registration::from_preset(1).unwrap();
    assert_eq!(r.deck.cards, CustomDeck::from_preset(1).unwrap().cards);
    assert_eq!(r.battlefields, [11, 1, 12]);
    assert_eq!(
        r.sideboard,
        [
            (443, 1),
            (421, 1),
            (433, 2),
            (366, 3),
            (442, 1),
            (31, 1),
            (441, 1)
        ]
    );
    assert_eq!(r.deck.card_count(), 40);
    let mut changed = r.deck.cards.clone();
    changed.retain(|&(id, _)| id != 7);
    changed.extend([(441, 1), (442, 1), (443, 1)]);
    let (deck, side) = r.configure(changed, 22).unwrap();
    assert_eq!(deck.card_count(), 40);
    assert!(side.contains(&(7, 3)));
    assert_eq!(side.iter().map(|(_, n)| usize::from(*n)).sum::<usize>(), 10);
    let mut illegal = deck.cards.clone();
    illegal.iter_mut().find(|(id, _)| *id == 443).unwrap().0 = 444;
    assert!(r.configure(illegal, 22).is_err());
    assert!(presets()[1].cards.contains(&(7, 3)));
    assert!(!presets()[1].cards.iter().any(|&(id, _)| id == 443));
}

#[test]
fn master_yi_planner_uses_revealed_damage_units_and_gear_for_specific_answers() {
    let s = sideboarding();
    for (incoming, scout) in [
        (443, small_board()),
        (441, revealed("Shock Blast")),
        (442, revealed("Charm")),
        (366, revealed("Boots of Swiftness")),
    ] {
        let view = s.observe(0).unwrap();
        let plan = recommend(
            &view,
            0,
            &scout,
            Difficulty::Strategic,
            MatchPolicy::Adaptive,
        )
        .unwrap();
        assert!(
            plan.swaps.iter().any(|swap| swap.inside == incoming),
            "Missing {incoming}: {plan:?}"
        );
        Registration::from_preset(1)
            .unwrap()
            .configure(plan.deck.cards, 22)
            .unwrap();
        let easy = recommend(&view, 0, &scout, Difficulty::Easy, MatchPolicy::Adaptive).unwrap();
        assert!(easy.swaps.is_empty());
        assert_eq!(
            s.observe(0).unwrap(),
            view,
            "Recommendations never submit a choice"
        );
    }
}

#[test]
fn master_yi_planner_does_not_mistake_unit_damage_or_untargeted_damage_for_spell_targets() {
    let view = sideboarding().observe(0).unwrap();
    for name in ["Alpha Strike", "Challenge", "Thermo Beam"] {
        let plan = recommend(
            &view,
            0,
            &revealed(name),
            Difficulty::Strategic,
            MatchPolicy::Adaptive,
        )
        .unwrap();
        assert!(
            plan.swaps.iter().all(|swap| swap.inside != 441),
            "This card does not deal spell damage: {name}"
        );
    }
    let plan = recommend(
        &view,
        0,
        &revealed("Firestorm"),
        Difficulty::Strategic,
        MatchPolicy::Adaptive,
    )
    .unwrap();
    assert!(
        plan.swaps.iter().all(|swap| swap.inside != 442),
        "Area damage does not choose its units"
    );
    let plan = recommend(
        &view,
        0,
        &Scouting::default(),
        Difficulty::Strategic,
        MatchPolicy::Adaptive,
    )
    .unwrap();
    assert!(
        plan.swaps.is_empty(),
        "Unknown opposing cards are not evidence"
    );
}

#[test]
fn master_yi_registered_matches_finish_against_both_irelia_presets_in_both_seats() {
    let mut games = 0;
    let mut matches = 0;
    let mut plays = [0usize; 3];
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for opponent in [0, 5] {
            for yi in 0..2 {
                for first in 0..2 {
                    let ids = if yi == 0 {
                        [1, opponent]
                    } else {
                        [opponent, 1]
                    };
                    let regs = ids.map(|p| Registration::from_preset(p).unwrap());
                    let mut s = MatchSeries::new(regs.clone(), first).unwrap();
                    let mut scouts: [Scouting; 2] = Default::default();
                    while s.phase() != SeriesPhase::Complete {
                        assert!(s.game_number() <= 6);
                        if s.phase() == SeriesPhase::Sideboarding {
                            for p in 0..2 {
                                let mut deck = recommend(
                                    &s.observe(p).unwrap(),
                                    p,
                                    &scouts[p],
                                    difficulty,
                                    MatchPolicy::Adaptive,
                                )
                                .unwrap()
                                .deck;
                                if p == yi {
                                    // Explicit legal manual swap exercises all three new sideboard cards.
                                    deck = regs[p].deck.clone();
                                    deck.cards.retain(|&(id, _)| id != 7);
                                    deck.cards.extend([(441, 1), (442, 1), (443, 1)]);
                                }
                                s.submit_sideboard(p, deck.cards, deck.chosen_champion)
                                    .unwrap();
                            }
                        }
                        for p in 0..2 {
                            let field = s.observe(p).unwrap()["available_battlefields"][0]
                                .as_u64()
                                .unwrap() as usize;
                            s.choose_battlefield(p, field).unwrap();
                        }
                        let chooser = s.summary()["designated_player"].as_u64().unwrap() as usize;
                        s.choose_play_order(chooser, chooser).unwrap();
                        let game_number = s.game_number();
                        let mut g = s
                            .new_game(
                                443920
                                    + (level * 100 + opponent * 10 + yi * 2 + first) as u64
                                    + game_number,
                                difficulty,
                            )
                            .unwrap();
                        g.start_match().unwrap();
                        let mut seen = std::collections::HashSet::new();
                        for step in 0..6000 {
                            for item in &g.chain {
                                if item.player == yi
                                    && (441..=443).contains(&item.card_id)
                                    && item.ability.is_none()
                                    && seen.insert(item.id)
                                {
                                    plays[item.card_id - 441] += 1;
                                }
                            }
                            for (p, scout) in scouts.iter_mut().enumerate() {
                                scout.observe_game(&g, p).unwrap();
                            }
                            if g.winner.is_some() {
                                break;
                            }
                            assert!(g.step_ai().unwrap_or_else(|e| panic!(
                                "{ids:?}/{difficulty:?} game {game_number} step {step}: {e}"
                            )));
                            g.validate_numeric_state().unwrap();
                            for id in 441..=443 {
                                let p = &g.players[yi];
                                let n = p
                                    .deck
                                    .iter()
                                    .chain(&p.hand)
                                    .chain(&p.discard)
                                    .chain(&p.banish)
                                    .filter(|&&c| c == id)
                                    .count()
                                    + g.chain
                                        .iter()
                                        .filter(|c| {
                                            c.player == yi && c.card_id == id && c.ability.is_none()
                                        })
                                        .count();
                                assert_eq!(n, usize::from(game_number > 1));
                            }
                        }
                        assert!(g.winner.is_some());
                        s.finish_completed_game(&g).unwrap();
                        games += 1;
                    }
                    assert!(s.summary()["winner"].is_number());
                    matches += 1;
                }
            }
        }
    }
    assert_eq!(matches, 24);
    assert!(
        plays.iter().all(|&n| n > 0),
        "All three sideboard additions must be used: {plays:?}"
    );
    println!(
        "Master Yi registration: {matches} complete matches / {games} games; Spirit, Repulse, Alpha plays {plays:?}"
    );
}
