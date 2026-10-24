//! Repeatable offline smoke checks for every preset and AI difficulty.
use local_rift::cards::presets;
use local_rift::game::{Difficulty, Game, Phase, VICTORY_SCORE};
use std::collections::HashSet;
use std::time::Instant;

#[derive(Clone, Copy)]
struct Options {
    seeds: u64,
    first_seed: u64,
    max_steps: usize,
}

fn options() -> Result<Option<Options>, String> {
    let mut options = Options {
        seeds: 32,
        first_seed: 1,
        max_steps: 5000,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            println!(
                "Usage: cargo run --bin simulate -- [--seeds 32] [--seed 1] [--max-steps 5000]"
            );
            println!(
                "Runs every deck pairing with both AI levels; exits unsuccessfully on an invariant failure, illegal AI action, or stalled match."
            );
            return Ok(None);
        }
        let value = args
            .next()
            .ok_or_else(|| format!("Missing value for {arg}"))?;
        match arg.as_str() {
            "--seeds" => {
                options.seeds = value
                    .parse()
                    .map_err(|_| "--seeds must be a positive integer")?
            }
            "--seed" => {
                options.first_seed = value
                    .parse()
                    .map_err(|_| "--seed must be an unsigned integer")?
            }
            "--max-steps" => {
                options.max_steps = value
                    .parse()
                    .map_err(|_| "--max-steps must be a positive integer")?
            }
            _ => return Err(format!("Unknown option: {arg}. Use --help for usage.")),
        }
    }
    if options.seeds == 0 || options.max_steps == 0 {
        return Err("--seeds and --max-steps must be greater than zero.".into());
    }
    options
        .first_seed
        .checked_add(options.seeds - 1)
        .ok_or("The requested seed range exceeds u64.")?;
    Ok(Some(options))
}

fn check_invariants(game: &Game) -> Result<(), String> {
    let mut ids = HashSet::new();
    let mut hidden_lanes = HashSet::new();
    for (owner, player) in game.players.iter().enumerate() {
        if usize::from(player.total_runes()) + player.rune_deck.len() != 12 {
            return Err(format!("player {owner}: rune conservation failed"));
        }
        let mut available = 0;
        for (&runes, &exhausted) in player.runes.iter().zip(&player.exhausted_runes) {
            if exhausted > runes {
                return Err(format!(
                    "player {owner}: more exhausted runes than channelled runes"
                ));
            }
            available += runes - exhausted;
        }
        available += player.energy_pool;
        if player.energy != available {
            return Err(format!(
                "player {owner}: energy {} differs from {available} available from ready runes and stored energy",
                player.energy
            ));
        }
        let cards_in_chain = game
            .chain
            .iter()
            .filter(|item| item.player == owner && item.ability.is_none())
            .count();
        let cards_total = player.deck.len()
            + player.hand.len()
            + player.discard.len()
            + player.banish.len()
            + player.hidden.len()
            + usize::from(player.champion.is_some())
            + player.units.iter().filter(|unit| !unit.token).count()
            + player.gear.iter().filter(|gear| !gear.token).count()
            + cards_in_chain;
        if cards_total != 40 {
            return Err(format!(
                "player {owner}: card conservation failed ({cards_total} cards including chain)"
            ));
        }
        for unit in &player.units {
            if !ids.insert(unit.id) {
                return Err(format!("duplicate unit identifier {}", unit.id));
            }
            if unit.lane.is_some_and(|lane| lane >= 2) {
                return Err(format!("unit {} is in an unknown battlefield", unit.id));
            }
            // Lethal units stay visible while their owner chooses an applicable
            // death replacement. Cleanup resumes as soon as that choice ends.
            if unit.damage > 0
                && unit.damage >= game.unit_might(unit)
                && !matches!(
                    game.decision,
                    Some(local_rift::game::Decision::DeathReplacement { .. })
                )
            {
                return Err(format!(
                    "defeated unit {} ({}) was left on the board: damage {}, might {}, lane {:?}; recent actions: {}",
                    unit.id,
                    game.unit_name(unit),
                    unit.damage,
                    game.unit_might(unit),
                    unit.lane,
                    game.log
                        .iter()
                        .rev()
                        .take(10)
                        .rev()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(" | ")
                ));
            }
        }
        for gear in &player.gear {
            if !ids.insert(gear.id) {
                return Err(format!("duplicate gear identifier {}", gear.id));
            }
            if gear
                .attached_to
                .is_some_and(|id| !player.units.iter().any(|unit| unit.id == id))
            {
                return Err(format!(
                    "gear {} remains attached to a missing unit",
                    gear.id
                ));
            }
        }
        for hidden in &player.hidden {
            if !ids.insert(hidden.id) {
                return Err(format!("duplicate hidden-card identifier {}", hidden.id));
            }
            if hidden.lane >= 2 || !hidden_lanes.insert(hidden.lane) {
                return Err("invalid or multiply occupied facedown battlefield slot".into());
            }
        }
    }
    if let Some(winner) = game.winner
        && (game.phase != Phase::Finished || game.players[winner].score < VICTORY_SCORE)
    {
        return Err("winner and finished match state are inconsistent".into());
    }
    Ok(())
}

fn run(options: Options) -> Result<(), String> {
    let count = presets().len();
    let start = Instant::now();
    let mut games = 0;
    let mut max_steps = 0;
    let mut max_turns = 0;
    let mut total_steps = 0;
    for difficulty in [Difficulty::Easy, Difficulty::Tactical] {
        let mut difficulty_games = 0;
        let mut difficulty_wins = [0usize; 2];
        for first in 0..count {
            for second in 0..count {
                for offset in 0..options.seeds {
                    let seed = options.first_seed + offset;
                    let starting_player = (offset % 2) as usize;
                    let context = format!(
                        "{difficulty:?}, deck {first} vs {second}, seed {seed}, player {starting_player} starts"
                    );
                    let mut game = Game::new(first, second, seed, difficulty);
                    game.choose_starting_player(starting_player)
                        .map_err(|error| format!("{context}: {error}"))?;
                    game.start_match()
                        .map_err(|error| format!("{context}: {error}"))?;
                    check_invariants(&game).map_err(|error| format!("{context}: {error}"))?;
                    let mut steps = 0;
                    while game.winner.is_none() && steps < options.max_steps {
                        let progressed = game.step_ai().map_err(|error| {
                            format!("{context}, turn {}, step {steps}: {error}", game.turn)
                        })?;
                        if !progressed {
                            return Err(format!("{context}: AI stopped before the match finished"));
                        }
                        steps += 1;
                        check_invariants(&game).map_err(|error| {
                            format!("{context}, turn {}, step {steps}: {error}", game.turn)
                        })?;
                    }
                    let winner = game.winner.ok_or_else(|| {
                        format!("{context}: no winner after {steps} actions (turn {}); possible stalled match", game.turn)
                    })?;
                    difficulty_wins[usize::from(winner != starting_player)] += 1;
                    games += 1;
                    difficulty_games += 1;
                    total_steps += steps;
                    max_steps = max_steps.max(steps);
                    max_turns = max_turns.max(game.turn);
                }
            }
        }
        println!(
            "{difficulty:?}: {difficulty_games} completed; first player {} wins, second player {} wins",
            difficulty_wins[0], difficulty_wins[1]
        );
    }
    println!(
        "PASS: {games} complete matches across {} pairings and 2 AI levels, alternating starting player by seed",
        count * count
    );
    println!(
        "Maximum: {max_steps} actions, {max_turns} turns; average {:.1} actions; elapsed {:.2}s",
        total_steps as f64 / games as f64,
        start.elapsed().as_secs_f64()
    );
    println!(
        "All matches checked card/rune conservation, energy accounting, unique valid units, and winner consistency after every action."
    );
    Ok(())
}

fn main() {
    let outcome = options().and_then(|options| match options {
        Some(options) => run(options),
        None => Ok(()),
    });
    if let Err(error) = outcome {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}
