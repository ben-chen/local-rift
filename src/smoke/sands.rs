//! Choose an older spell through the real chain controls; exercise a legal no-op.
use super::*;

fn setup(app: &mut App, spells: usize) {
    expansion_fixture(app, 456);
    app.game = Game::new_for_agents(0, 1, 456456, Difficulty::Easy);
    app.game.choose_starting_player(1).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![3, 3];
    app.game.battlefield_owner = vec![None; 2];
    for p in &mut app.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.set_rune_setup([0, 6, 0, 0, 0, 0], [0; 6]).unwrap();
        p.rune_deck = vec![Domain::Calm; 6];
        p.energy_pool = 6;
        p.energy = 12;
        p.power_pool = [5; 6];
        p.any_power = 0;
    }
    app.game.players[0].hand = vec![456];
    app.game.players[1].hand = vec![133; spells];
    for _ in 0..spells {
        app.game
            .apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            })
            .unwrap();
    }
    app.game.apply(Action::Pass).unwrap();
    app.player_deck = 0;
    app.opponent_deck = 1;
    app.fields = [3, 3];
    app.selection = None;
    app.targets.clear();
    app.choices.clear();
    app.inspect = None;
    app.field_inspect = None;
    app.lesson = None;
    app.rune_menu = None;
    app.chain_cursor = 0;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.ai_worker.invalidate();
    app.companion.set_level(AdviceLevel::Beginner);
}

fn tip(app: &mut App) {
    app.companion.observe(&app.game);
    for _ in 0..80 {
        if app.companion.message().contains("Sands") {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("missing Sands advice");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app, 2),
        1 => app.ui.input_override = Some((vec2(315.0, 760.0), false, true)),
        2 => {
            assert_eq!(app.inspect, Some(456));
            capture(app, "RIFT_SANDS_INSPECT");
        }
        3 => {
            app.inspect = None;
            click(app, 315.0, 760.0);
        }
        4 => click(app, 1225.0, 320.0),
        5 => {
            assert_eq!(app.chain_cursor, 1);
            tip(app);
            capture(app, "RIFT_SANDS_TARGET");
            click(app, 1300.0, 320.0);
        }
        6 => {
            assert_eq!(
                app.targets,
                vec![Target::Spell {
                    chain_id: app.game.chain[0].id
                }]
            );
            capture(app, "RIFT_SANDS_SELECTED");
            click(app, 1280.0, 510.0);
        }
        7 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 456);
            assert_eq!(app.game.play_history.finalized_spells[0].len(), 1);
            tip(app);
            capture(app, "RIFT_SANDS_BEGINNER");
            click(app, 150.0, 600.0);
        }
        8 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            tip(app);
            capture(app, "RIFT_SANDS_ADVANCED");
            click(app, 1285.0, 595.0);
        }
        9 => {
            assert_eq!(app.game.priority(), 1, "Mote cannot block passing");
            app.game.apply(Action::Pass).unwrap();
        }
        10 => {
            assert_eq!(app.game.chain.len(), 1);
            assert_eq!(
                app.game.chain[0].id,
                app.game.play_history.finalized_spells[1][1]
            );
            assert!(app.game.players[1].discard.contains(&133));
            capture(app, "RIFT_SANDS_COUNTERED");
        }
        11 => setup(app, 1),
        12 => click(app, 315.0, 760.0),
        13 => click(app, 1300.0, 320.0),
        14 => click(app, 1280.0, 510.0),
        15 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 456);
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        16 => {
            assert_eq!(app.game.chain.len(), 1);
            assert!(app.game.players[1].discard.is_empty());
            assert!(app.game.players[0].discard.contains(&456));
            capture(app, "RIFT_SANDS_NO_EFFECT");
        }
        17 => return true,
        _ => panic!("Sands smoke exceeded its defined frames"),
    }
    false
}
