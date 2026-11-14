//! Small, reproducible decision measurements; no claim of human-level strength.
//! Positions are authored midgames, not complete legal-deck replays. Each goal
//! has an explicit legal-action witness independent of the opponent policy.
use crate::companion::{AdviceLevel, Companion};
use local_rift::game::{Action, ChainAbility, Decision, Difficulty, Game, Target, Unit};
use local_rift::legends::{LegendAbility, LegendTrigger};
use serde_json::{Value, json};
use std::time::Instant;

const RUNNER: u32 = 62001;

#[path = "practice_responses.rs"]
mod responses;

#[derive(Clone, Copy, Debug)]
enum Start {
    Reset,
    UnitChoice,
    Open,
}

#[derive(Clone, Copy)]
struct Position {
    id: &'static str,
    start: Start,
    runner: usize,
    spell: usize,
    legend_ready: bool,
    unit_ready: bool,
    win: bool,
    // Established regressions are gates. New sequence measurements remain
    // visible in the report even when the weaker levels miss the solution.
    must_pass: bool,
}

const POSITIONS: &[Position] = &[
    Position {
        id: "skip-ready-legend",
        start: Start::Reset,
        runner: 13,
        spell: 7,
        legend_ready: true,
        unit_ready: false,
        win: false,
        must_pass: true,
    },
    Position {
        id: "skip-reset-without-followup",
        start: Start::Reset,
        runner: 13,
        spell: 7,
        legend_ready: false,
        unit_ready: false,
        win: false,
        must_pass: false,
    },
    Position {
        id: "skip-ready-pyke",
        start: Start::UnitChoice,
        runner: 13,
        spell: 7,
        legend_ready: true,
        unit_ready: true,
        win: false,
        must_pass: true,
    },
    Position {
        id: "skip-ready-fervent",
        start: Start::UnitChoice,
        runner: 0,
        spell: 8,
        legend_ready: true,
        unit_ready: true,
        win: false,
        must_pass: true,
    },
    Position {
        id: "ready-akali-to-win",
        start: Start::UnitChoice,
        runner: 2,
        spell: 8,
        legend_ready: true,
        unit_ready: false,
        win: true,
        must_pass: false,
    },
    Position {
        id: "ready-fervent-to-win",
        start: Start::UnitChoice,
        runner: 0,
        spell: 7,
        legend_ready: true,
        unit_ready: false,
        win: true,
        must_pass: false,
    },
    Position {
        id: "reset-discipline-pyke",
        start: Start::Reset,
        runner: 13,
        spell: 7,
        legend_ready: false,
        unit_ready: false,
        win: true,
        must_pass: false,
    },
    Position {
        id: "reset-discipline-fervent",
        start: Start::Reset,
        runner: 0,
        spell: 7,
        legend_ready: false,
        unit_ready: false,
        win: true,
        must_pass: false,
    },
    Position {
        id: "reset-en-garde-akali",
        start: Start::Reset,
        runner: 2,
        spell: 8,
        legend_ready: false,
        unit_ready: false,
        win: true,
        must_pass: false,
    },
    Position {
        id: "reset-en-garde-stellacorn",
        start: Start::Reset,
        runner: 17,
        spell: 8,
        legend_ready: false,
        unit_ready: false,
        win: true,
        must_pass: false,
    },
    Position {
        id: "move-ready-pyke-before-spending",
        start: Start::Open,
        runner: 13,
        spell: 7,
        legend_ready: true,
        unit_ready: true,
        win: true,
        must_pass: false,
    },
    Position {
        id: "move-ready-akali-before-spending",
        start: Start::Open,
        runner: 2,
        spell: 8,
        legend_ready: true,
        unit_ready: true,
        win: true,
        must_pass: false,
    },
];

fn unit(id: u32, card_id: usize, ready: bool) -> Unit {
    Unit {
        id,
        card_id,
        lane: None,
        ready,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    }
}

fn settle(game: &mut Game) {
    for _ in 0..32 {
        if game.winner.is_some() || (!game.in_window() && game.decision.is_none()) {
            return;
        }
        assert!(
            game.decision.is_none(),
            "unexpected witness decision: {:?}",
            game.decision
        );
        game.apply(Action::Pass).unwrap();
    }
    panic!("witness response window did not settle");
}

fn choose_runner(game: &mut Game, player: usize, spell: usize) {
    let hand_index = game.players[player]
        .hand
        .iter()
        .position(|&id| id == spell)
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
    for _ in 0..8 {
        if matches!(game.decision, Some(Decision::Legend {
            player: p, trigger: LegendTrigger::IreliaChoose { unit_id: RUNNER },
        }) if p == player)
        {
            return;
        }
        assert!(matches!(
            game.decision,
            Some(Decision::OrderTriggers { .. })
        ));
        // Ordering simultaneous triggers is not a policy decision under test.
        let first = game.legal_actions().unwrap().remove(0);
        game.apply(first).unwrap();
    }
    panic!("choosing the runner did not reach Irelia's optional ready");
}

fn setup(case: Position, player: usize, level: Difficulty) -> Game {
    let decks = if player == 0 { [5, 1] } else { [1, 5] };
    let mut game = Game::new_for_agents(decks[0], decks[1], 62001, level);
    game.choose_starting_player(player).unwrap();
    for seat in 0..2 {
        game.choose_battlefield(seat, 3).unwrap();
        game.mulligan_player(seat, &[]).unwrap();
    }
    game.start_match().unwrap();
    // Explicit bounded position: no claims of conservation from the opening.
    for state in &mut game.players {
        state.hand.clear();
        state.units.clear();
        state.gear.clear();
        state.hidden.clear();
        state.champion = None;
        state.set_rune_setup([0; 6], [0; 6]).unwrap();
        state.energy = 0;
        state.energy_pool = 0;
        state.power_pool = [0; 6];
        state.any_power = 0;
    }
    game.players[player]
        .set_rune_setup([0, 3, 0, 0, 3, 0], [0; 6])
        .unwrap();
    game.players[player].energy = 6;
    game.players[player].score = 6;
    game.players[player].hand = vec![case.spell];
    // Known-public hand size, private contents. The scripted opponent passes;
    // this benchmark does not assume a competent responding human opponent.
    game.players[1 - player].hand = vec![22, 23, 24];
    game.players[player].units = vec![
        unit(62000, 13, true),
        unit(RUNNER, case.runner, case.unit_ready),
    ];
    game.legend[player].ready = case.legend_ready;
    game.apply(Action::Move {
        unit_id: 62000,
        lane: Some(0),
    })
    .unwrap();
    for _ in 0..16 {
        if game.decision.is_some() {
            break;
        }
        game.apply(Action::Pass).unwrap();
    }
    assert_eq!(game.players[player].score, 7);
    assert!(matches!(game.decision, Some(Decision::Legend {
        player: p, trigger: LegendTrigger::IreliaConquer,
    }) if p == player));
    if case.id == "skip-reset-without-followup" {
        game.players[player].hand.clear();
    }
    if !matches!(case.start, Start::Reset) {
        game.apply(Action::Choose { accept: false }).unwrap();
        settle(&mut game);
    }
    if matches!(case.start, Start::UnitChoice) {
        choose_runner(&mut game, player, case.spell);
    }
    game.validate_numeric_state().unwrap();
    assert_eq!(game.priority(), player);
    game
}

fn witness(case: Position, player: usize, game: &Game) {
    let mut game = game.clone();
    if !case.win {
        game.apply(Action::Choose { accept: false }).unwrap();
        return;
    }
    if matches!(case.start, Start::Reset) {
        game.apply(Action::Choose { accept: true }).unwrap();
        settle(&mut game);
        choose_runner(&mut game, player, case.spell);
    }
    if !matches!(case.start, Start::Open) {
        game.apply(Action::Choose { accept: true }).unwrap();
        settle(&mut game);
    }
    game.apply(Action::Move {
        unit_id: RUNNER,
        lane: Some(1),
    })
    .unwrap();
    settle(&mut game);
    assert_eq!(
        game.winner,
        Some(player),
        "witness {} seat {player}",
        case.id
    );
}

fn different_secrets(game: &Game, player: usize) -> Game {
    let mut changed = game.clone();
    assert_eq!(changed.players[1 - player].hand.len(), 3);
    changed.players[1 - player].hand = vec![25, 26, 27];
    changed.players[1 - player].deck.reverse();
    changed.players[player].deck.reverse();
    changed
}

// Compare the first chosen step only: later draws legitimately change what
// the actor knows. Exclude deck/hand identities, but retain public progress.
fn public_step(game: &Game) -> Value {
    json!({
        "turn": game.turn, "active": game.active, "winner": game.winner,
        "priority": game.priority(), "legend": format!("{:?}", game.legend),
        "chain": format!("{:?}", game.chain), "decision": format!("{:?}", game.decision),
        "players": game.players.iter().map(|p| json!({
            "score": p.score, "energy": p.energy, "power": p.power_pool,
            "any_power": p.any_power, "runes": p.runes, "exhausted": p.exhausted_runes,
            "units": format!("{:?}", p.units), "gear": format!("{:?}", p.gear),
        })).collect::<Vec<_>>()
    })
}

fn coach(game: &Game) -> Value {
    let before = format!("{game:?}");
    let private = different_secrets(game, 0);
    let mut rows = Vec::new();
    for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
        let mut mote = Companion::new(level);
        let mut hidden = Companion::new(level);
        let mut messages = Vec::new();
        mote.observe(game);
        hidden.observe(&private);
        // Capture first advice and the next two available thoughts, as the UI
        // exposes them; repeated messages are expected for a shorter tip list.
        for _ in 0..3 {
            assert_eq!(
                mote.message(),
                hidden.message(),
                "private information affected advice"
            );
            assert!(mote.message().chars().count() <= 180);
            messages.push(mote.message().to_owned());
            mote.next_tip(game);
            hidden.next_tip(&private);
        }
        assert_eq!(format!("{game:?}"), before, "commentary changed game state");
        rows.push(json!({"level":level.label(), "messages":messages}));
    }
    json!(rows)
}

fn run(case: Position, player: usize, level: Difficulty) -> Value {
    let original = setup(case, player, level);
    witness(case, player, &original);
    let advice = (player == 0).then(|| coach(&original));
    let mut game = original.clone();
    let mut hidden = different_secrets(&original, player);
    let start = Instant::now();
    assert!(game.step_ai().unwrap());
    let mut times = vec![start.elapsed().as_secs_f64() * 1000.0];
    hidden.step_ai().unwrap();
    assert_eq!(
        public_step(&game),
        public_step(&hidden),
        "private information affected {} seat {player}",
        case.id
    );
    let first = public_step(&game);
    let mut steps = 1;
    let mut trace = vec![first.clone()];
    if case.win {
        for _ in 0..63 {
            if game.winner.is_some() || game.turn != original.turn {
                break;
            }
            if game.priority() == player {
                let start = Instant::now();
                assert!(game.step_ai().unwrap());
                times.push(start.elapsed().as_secs_f64() * 1000.0);
                steps += 1;
                trace.push(public_step(&game));
            } else {
                assert!(
                    game.decision.is_none(),
                    "scripted opponent needs a decision"
                );
                game.apply(Action::Pass).unwrap();
            }
        }
    }
    let success = if case.win {
        game.winner == Some(player)
            && game.turn == original.turn
            && (!matches!(case.start, Start::Open)
                || (game.players[player].hand == original.players[player].hand
                    && game.players[player].runes == original.players[player].runes
                    && game.players[player].energy == original.players[player].energy))
    } else {
        game.players[player].energy == original.players[player].energy
            && game.players[player].runes == original.players[player].runes
            && game.legend[player].ready == original.legend[player].ready
            && !game.chain.iter().any(|item| {
                matches!(
                    item.ability,
                    Some(ChainAbility::Legend(
                        LegendAbility::IreliaRefresh | LegendAbility::IreliaReady { .. }
                    ))
                )
            })
            && game.decision != original.decision
    };
    json!({
        "position":case.id, "seat":player, "difficulty":format!("{level:?}"),
        "objective":if matches!(case.start, Start::Open) { "win_without_spending" }
            else if case.win { "win_this_turn" } else { "decline_without_spending" },
        "must_pass":case.must_pass || level == Difficulty::Strategic
            || (case.win && !matches!(case.start, Start::Open)),
        "success":success, "ai_steps":steps,
        "step_ms":times, "first_step":first, "advice":advice,
        "failed_trace":if success { Value::Null } else { json!(trace) },
        "final_log":if success { Value::Null } else { json!(game.log) },
    })
}

#[test]
fn measured_irelia_decisions_and_companion() {
    let mut rows = Vec::new();
    for &case in POSITIONS {
        for player in 0..2 {
            for level in [
                Difficulty::Easy,
                Difficulty::Tactical,
                Difficulty::Strategic,
            ] {
                rows.push(run(case, player, level));
            }
        }
    }
    let summary: Vec<_> = ["Easy", "Tactical", "Strategic"].into_iter().map(|level| {
        let cases: Vec<_> = rows.iter().filter(|r| r["difficulty"] == level).collect();
        let mut times: Vec<_> = cases.iter().flat_map(|r| r["step_ms"].as_array().unwrap())
            .map(|v| v.as_f64().unwrap()).collect();
        times.sort_by(f64::total_cmp);
        json!({"difficulty":level, "cases":cases.len(),
            "successes":cases.iter().filter(|r| r["success"] == true).count(),
            "misses":cases.iter().filter(|r| r["success"] == false)
                .map(|r| json!({"position":r["position"],"seat":r["seat"]})).collect::<Vec<_>>(),
            "step_ms_p50":times[times.len()/2], "step_ms_p95":times[(times.len()-1)*95/100],
            "step_ms_max":times.last(),
        })
    }).collect();
    let report = json!({"schema_version":1, "positions":POSITIONS.len(),
        "scope":"Authored Irelia reset/ready and conquest positions. Scripted opponent passes. Development benchmark, not held-out strength or a complete-match test. Timing includes execution and is observational; no machine-dependent time threshold.",
        "rules":"October 9 Core 416.1.b-c (actual Ready), 473.1.b (final conquest); reviewed Irelia, Discipline and En Garde text in the bundled catalog.",
        "summary":summary, "records":rows});
    if let Some(path) = std::env::var_os("RIFT_IRELIA_BENCHMARK_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    println!("IRELIA_DECISIONS {}", report["summary"]);
    let failures: Vec<_> = report["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["must_pass"] == true && r["success"] == false)
        .collect();
    assert!(
        failures.is_empty(),
        "established regression failed: {failures:?}"
    );
}

#[test]
fn waiting_for_irelia_reset_does_not_ignore_an_enemy_response() {
    for player in 0..2 {
        for level in [Difficulty::Easy, Difficulty::Tactical] {
            let mut game = setup(POSITIONS[6], player, level);
            game.players[player].hand = vec![1]; // Abandon can answer their Gust.
            game.players[1 - player].hand = vec![11];
            game.players[1 - player].energy = 1;
            game.players[1 - player].energy_pool = 1;
            game.apply(Action::Choose { accept: true }).unwrap();
            if player == 0 {
                for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
                    let mut mote = Companion::new(level);
                    mote.observe(&game);
                    assert!(mote.message().contains("reset") && mote.message().contains("before"));
                    assert!(mote.message().chars().count() <= 180);
                }
            }
            let chain = game.chain.clone();
            game.step_ai().unwrap();
            assert_eq!(game.chain, chain, "wait for the lone friendly reset");
            assert_eq!(game.priority(), 1 - player);
            game.apply(Action::PlayCard {
                hand_index: 0,
                lane: None,
                target: Some(Target::Unit {
                    player,
                    unit_id: 62000,
                }),
            })
            .unwrap();
            assert_eq!(game.priority(), 1 - player, "the caster retains priority");
            game.apply(Action::Pass).unwrap();
            assert_eq!(game.priority(), player);
            if player == 0 {
                for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
                    let mut mote = Companion::new(level);
                    mote.observe(&game);
                    assert!(
                        !mote.message().contains("reset"),
                        "reassess the new enemy spell"
                    );
                }
            }
            game.step_ai().unwrap();
            assert!(
                game.chain.iter().any(|item| item.player == player
                    && item.card_id == 1
                    && item.ability.is_none()),
                "{player}/{level:?}: opponent response was ignored: {:?}",
                game.log
            );
        }
    }
}
