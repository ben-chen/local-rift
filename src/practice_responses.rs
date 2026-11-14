//! Authored response positions. The opponent recasts returned Gust when legal;
//! it does not pass through a known available answer. These are development
//! measurements, not held-out strength ratings or full registered-deck replays.
use super::*;

const PARTNER: u32 = 62002;

#[derive(Clone, Copy, Debug)]
enum Case {
    BuffInsteadOfReturningAnswer,
    LetPartnerScore,
    AlreadyOutsideGustRange,
    CounterOnlyAnswer,
}

const CASES: [Case; 4] = [
    Case::BuffInsteadOfReturningAnswer,
    Case::LetPartnerScore,
    Case::AlreadyOutsideGustRange,
    Case::CounterOnlyAnswer,
];

fn position(case: Case, player: usize, level: Difficulty) -> Game {
    let mut game = setup(
        Position {
            id: "gust-response",
            start: Start::Open,
            runner: 13,
            spell: 8,
            legend_ready: false,
            unit_ready: true,
            win: true,
            must_pass: false,
        },
        player,
        level,
    );
    game.players[player].energy = if matches!(case, Case::AlreadyOutsideGustRange) {
        3
    } else {
        2
    };
    game.players[player].energy_pool = game.players[player].energy;
    game.players[player].hand = if matches!(case, Case::CounterOnlyAnswer) {
        vec![1]
    } else {
        vec![8, 1]
    };
    let opponent = 1 - player;
    game.players[opponent].hand = vec![11, 22, 23, 24];
    game.players[opponent].energy = if matches!(case, Case::BuffInsteadOfReturningAnswer) {
        2
    } else {
        1
    };
    game.players[opponent].energy_pool = game.players[opponent].energy;
    if matches!(case, Case::LetPartnerScore) {
        game.players[player].units.push(unit(PARTNER, 15, true));
        game.apply(Action::MoveGroup {
            unit_ids: vec![RUNNER, PARTNER],
            lane: Some(1),
        })
        .unwrap();
    } else {
        game.apply(Action::Move {
            unit_id: RUNNER,
            lane: Some(1),
        })
        .unwrap();
    }
    game.apply(Action::Pass).unwrap();
    assert_eq!(game.priority(), opponent);
    play_gust(&mut game, player);
    game.apply(Action::Pass).unwrap();
    assert_eq!(game.priority(), player);
    if matches!(case, Case::AlreadyOutsideGustRange) {
        play_buff(&mut game, player);
        while game.chain.last().is_some_and(|item| item.card_id != 11) {
            passive_step(&mut game);
        }
        assert_eq!(
            game.unit_might(
                game.players[player]
                    .units
                    .iter()
                    .find(|u| u.id == RUNNER)
                    .unwrap()
            )
            .unwrap(),
            5
        );
        if game.priority() != player {
            game.apply(Action::Pass).unwrap();
        }
    }
    game.validate_numeric_state().unwrap();
    assert_eq!(game.priority(), player);
    assert_eq!(game.chain.last().unwrap().card_id, 11);
    game
}

fn unspent_hand(case: Case, original: &Game, player: usize) -> Vec<usize> {
    let mut hand = original.players[player].hand.clone();
    if matches!(case, Case::LetPartnerScore) {
        hand.push(13); // Gust returns Pyke while Scuttle Crab wins the field.
    }
    hand
}

fn play_gust(game: &mut Game, player: usize) {
    let hand_index = game.players[1 - player]
        .hand
        .iter()
        .position(|&id| id == 11)
        .unwrap();
    game.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets: vec![Target::Unit {
            player,
            unit_id: RUNNER,
        }],
    })
    .unwrap();
}

fn play_buff(game: &mut Game, player: usize) {
    let hand_index = game.players[player]
        .hand
        .iter()
        .position(|&id| id == 8)
        .unwrap();
    game.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets: vec![Target::Unit {
            player,
            unit_id: RUNNER,
        }],
    })
    .unwrap();
}

fn passive_step(game: &mut Game) {
    let action = match &game.decision {
        Some(Decision::Legend { .. } | Decision::Predict { .. }) => {
            Action::Choose { accept: false }
        }
        Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
            trigger_id: options[0].id,
        },
        None => Action::Pass,
        other => panic!("unexpected response choice: {other:?}"),
    };
    game.apply(action).unwrap();
}

fn opponent_step(game: &mut Game, player: usize) {
    assert_eq!(game.priority(), 1 - player);
    // Only the opponent's own hand and publicly observed target are inspected.
    if game.decision.is_none()
        && let Some(hand_index) = game.players[1 - player]
            .hand
            .iter()
            .position(|&id| id == 11)
    {
        let action = Action::PlayCardWithTargets {
            hand_index,
            lane: None,
            targets: vec![Target::Unit {
                player,
                unit_id: RUNNER,
            }],
        };
        if game.clone().apply(action.clone()).is_ok() {
            game.apply(action).unwrap();
            return;
        }
    }
    passive_step(game);
}

fn witness(case: Case, player: usize, original: &Game) {
    let mut game = original.clone();
    match case {
        Case::BuffInsteadOfReturningAnswer => play_buff(&mut game, player),
        Case::CounterOnlyAnswer => {
            let chain_id = game.chain.last().unwrap().id;
            game.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![Target::Spell { chain_id }],
            })
            .unwrap();
        }
        Case::LetPartnerScore | Case::AlreadyOutsideGustRange => {}
    }
    for _ in 0..48 {
        if game.winner.is_some() {
            break;
        }
        if game.priority() == player {
            passive_step(&mut game);
        } else {
            opponent_step(&mut game, player);
        }
    }
    assert_eq!(game.winner, Some(player), "witness {case:?}/{player}");
    if matches!(case, Case::LetPartnerScore | Case::AlreadyOutsideGustRange) {
        assert_eq!(
            game.players[player].hand,
            unspent_hand(case, original, player)
        );
        assert_eq!(game.players[player].energy, original.players[player].energy);
    }
}

#[test]
fn repeated_gust_can_still_return_its_other_target_after_a_buff() {
    use local_rift::cards::Domain;
    use local_rift::game::domain_index;
    use local_rift::play::{ExecutionChoices, PlayChoices, RepeatSelection};

    let mut game = setup(
        Position {
            id: "repeated-gust",
            start: Start::Open,
            runner: 13,
            spell: 8,
            legend_ready: false,
            unit_ready: true,
            win: true,
            must_pass: false,
        },
        0,
        Difficulty::Easy,
    );
    game.players[0].units.push(unit(PARTNER, 15, true));
    game.players[0].hand = vec![8];
    game.players[1].hand = vec![11];
    for player in &mut game.players {
        player.energy = 2;
        player.energy_pool = 2;
    }
    // Syndra grants Repeat during this showdown, including on the opponent's
    // turn. Pay her real additional 2 Energy and 1 Chaos Power below.
    let mut syndra = unit(64001, 266, true);
    syndra.lane = Some(1);
    game.players[1].units.push(syndra);
    game.battlefield_owner[1] = Some(1);
    game.players[1].energy = 3;
    game.players[1].energy_pool = 3;
    game.players[1].power_pool[domain_index(Domain::Chaos)] = 1;
    game.apply(Action::MoveGroup {
        unit_ids: vec![RUNNER, PARTNER],
        lane: Some(1),
    })
    .unwrap();
    game.apply(Action::Pass).unwrap();
    let action = Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player: 0,
            unit_id: RUNNER,
        }],
    };
    let option_id = game.play_choices(&action).unwrap().repeats[0].id;
    game.apply(Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats: vec![RepeatSelection {
                option_id,
                choices: ExecutionChoices {
                    targets: vec![Target::Unit {
                        player: 0,
                        unit_id: PARTNER,
                    }],
                    ..Default::default()
                },
            }],
            ..Default::default()
        },
    })
    .unwrap();
    assert_eq!(game.players[1].energy, 0);
    assert_eq!(game.players[1].power_pool[domain_index(Domain::Chaos)], 0);
    game.apply(Action::Pass).unwrap();
    play_buff(&mut game, 0);
    while game.chain.last().is_some_and(|item| item.card_id != 11) {
        passive_step(&mut game);
    }
    if game.priority() != 0 {
        game.apply(Action::Pass).unwrap();
    }
    assert_eq!(game.chain.last().unwrap().executions.len(), 2);
    let mut mote = Companion::new(AdviceLevel::Advanced);
    mote.observe(&game);
    assert!(mote.message().contains("Repeat"), "{}", mote.message());
    assert!(mote.message().contains("another target"));
    assert!(mote.message().chars().count() <= 180);
    for _ in 0..8 {
        if game.chain.is_empty() {
            break;
        }
        passive_step(&mut game);
    }
    assert!(game.chain.is_empty());
    assert!(game.players[0].units.iter().any(|u| u.id == RUNNER));
    assert!(!game.players[0].units.iter().any(|u| u.id == PARTNER));
    assert!(game.players[0].hand.contains(&15));
}

#[test]
fn gust_coaching_respects_public_departure_replacements() {
    for banish in [true, false] {
        let mut game = position(Case::CounterOnlyAnswer, 0, Difficulty::Easy);
        // Controlled public chain metadata. Engine replacement regressions
        // separately exercise actual Flow and instructed-replay departures.
        let gust = game.chain.last_mut().unwrap();
        gust.banish_after = banish;
        gust.recycle_after = !banish;
        let before = format!("{game:?}");
        let mut mote = Companion::new(AdviceLevel::Advanced);
        mote.observe(&game);
        assert!(
            mote.message()
                .contains(if banish { "banishment" } else { "recycles" })
        );
        assert!(mote.message().contains("Abandon won't"));
        assert!(mote.message().chars().count() <= 180);
        assert_eq!(before, format!("{game:?}"));
    }
}

#[test]
fn returning_gust_allows_a_second_real_response() {
    for player in 0..2 {
        let mut game = position(Case::BuffInsteadOfReturningAnswer, player, Difficulty::Easy);
        let chain_id = game.chain.last().unwrap().id;
        let mut gust_ids = std::collections::BTreeSet::from([chain_id]);
        game.apply(Action::PlayCardWithTargets {
            hand_index: 1,
            lane: None,
            targets: vec![Target::Spell { chain_id }],
        })
        .unwrap();
        for _ in 0..48 {
            if game.winner.is_some() || (!game.in_window() && game.decision.is_none()) {
                break;
            }
            if game.priority() == player {
                passive_step(&mut game);
            } else {
                opponent_step(&mut game, player);
            }
            gust_ids.extend(
                game.chain
                    .iter()
                    .filter(|item| item.card_id == 11)
                    .map(|item| item.id),
            );
        }
        assert_eq!(
            gust_ids.len(),
            2,
            "the scripted opponent must actually recast Gust"
        );
        assert_eq!(game.players[1 - player].energy, 0);
        assert!(
            !game.players[player]
                .units
                .iter()
                .any(|unit| unit.id == RUNNER)
        );
        assert!(game.players[player].hand.contains(&13));
        assert_eq!(game.winner, None);
        assert!(!game.in_window());
    }
}

#[test]
fn measured_irelia_responses_against_recast_gust() {
    let mut records = Vec::new();
    for case in CASES {
        for player in 0..2 {
            for level in [
                Difficulty::Easy,
                Difficulty::Tactical,
                Difficulty::Strategic,
            ] {
                let original = position(case, player, level);
                witness(case, player, &original);
                let advice = (player == 0).then(|| coach(&original));
                if player == 0 {
                    let messages = advice.as_ref().unwrap().as_array().unwrap();
                    let beginner = messages[0]["messages"][0].as_str().unwrap();
                    let advanced = messages[1]["messages"][0].as_str().unwrap();
                    assert!(beginner.contains("Gust") && beginner.contains("3 Might"));
                    assert!(
                        advanced.contains(if matches!(case, Case::AlreadyOutsideGustRange) {
                            "Passing"
                        } else {
                            "Abandon"
                        })
                    );
                }
                let mut game = original.clone();
                let mut hidden = different_secrets(&original, player);
                game.step_ai().unwrap();
                hidden.step_ai().unwrap();
                assert_eq!(
                    public_step(&game),
                    public_step(&hidden),
                    "hidden information changed {case:?}"
                );
                let first = public_step(&game);
                let mut steps = 1;
                for _ in 0..63 {
                    if game.winner.is_some() || game.turn != original.turn {
                        break;
                    }
                    if game.priority() == player {
                        game.step_ai().unwrap();
                        steps += 1;
                    } else {
                        opponent_step(&mut game, player);
                    }
                }
                let keep_resources =
                    matches!(case, Case::LetPartnerScore | Case::AlreadyOutsideGustRange);
                let success = game.winner == Some(player)
                    && (!keep_resources
                        || (game.players[player].hand == unspent_hand(case, &original, player)
                            && game.players[player].energy == original.players[player].energy));
                let required = level == Difficulty::Strategic
                    || matches!(
                        case,
                        Case::BuffInsteadOfReturningAnswer | Case::CounterOnlyAnswer
                    );
                records.push(json!({"position":format!("{case:?}"),"seat":player,"difficulty":format!("{level:?}"),
                    "success":success,"must_pass":required,"ai_steps":steps,"first_step":first,"advice":advice,"final_log":game.log}));
            }
        }
    }
    let report = json!({"schema_version":1,"positions":CASES.len(),"scope":"Four authored Irelia response positions with a scripted legal Gust recast; independent legal winning witnesses, both seats, all levels. Development evidence, not held-out strength.","records":records});
    if let Some(path) = std::env::var_os("RIFT_IRELIA_RESPONSES_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    for level in ["Easy", "Tactical", "Strategic"] {
        let rows: Vec<_> = records
            .iter()
            .filter(|row| row["difficulty"] == level)
            .collect();
        println!(
            "{level}: {}/{} response outcomes",
            rows.iter().filter(|row| row["success"] == true).count(),
            rows.len()
        );
    }
    assert!(
        records
            .iter()
            .all(|row| row["must_pass"] != true || row["success"] == true)
    );
}
