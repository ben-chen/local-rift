//! Rules inspection, rejected deployment, movement and the last setup choice.
use super::*;
use local_rift::battlefields::ROCKFALL_PATH_ID;
const MOVER: u32 = 17001;

fn setup(app: &mut App) {
    expansion_fixture(app, 71);
    app.game = Game::new_for_agents(0, 1, 17017, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![ROCKFALL_PATH_ID, 3];
    app.game.battlefield_owner = vec![Some(0), None];
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
        p.energy = 12;
        p.energy_pool = 6;
        p.power_pool = [0; 6];
        p.any_power = 0;
    }
    app.game.players[0].units = vec![
        public_unit(MOVER, 71, None),
        public_unit(17002, 71, Some(0)),
    ];
    app.game.players[0].hand = vec![71];
    app.clear_game_controls();
    app.lesson = None;
    app.fields = [ROCKFALL_PATH_ID, 3];
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
}

fn advice(app: &mut App, level: AdviceLevel) {
    app.companion.set_level(level);
    app.companion.observe(&app.game);
    for _ in 0..100 {
        if app.companion.message().contains("Rockfall Path") {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("missing Rockfall advice");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app),
        1 => click(app, 640.0, 275.0),
        2 => {
            assert_eq!(app.field_inspect, Some(ROCKFALL_PATH_ID));
            capture(app, "RIFT_ROCKFALL_RULES");
            click(app, 650.0, 502.0);
        }
        3 => {
            assert!(app.field_inspect.is_none());
            click(app, 315.0, 760.0);
        }
        4 => {
            assert!(matches!(app.selection, Some(Selection::Hand(0))));
            click(app, 650.0, 430.0);
        }
        5 => {
            assert_eq!(app.game.players[0].hand, vec![71]);
            assert_eq!(app.game.players[0].energy, 12);
            assert!(app.toast.contains("Rockfall Path"));
            capture(app, "RIFT_ROCKFALL_BLOCKED");
            app.selection = None;
            click_unit(app, MOVER);
        }
        6 => {
            assert!(matches!(app.selection, Some(Selection::Unit(MOVER))));
            click(app, 650.0, 430.0);
        }
        7 => {
            let unit = app.game.players[0]
                .units
                .iter()
                .find(|u| u.id == MOVER)
                .unwrap();
            assert_eq!(unit.lane, Some(0));
            assert!(!unit.ready);
            advice(app, AdviceLevel::Beginner);
            capture(app, "RIFT_ROCKFALL_BEGINNER");
            click(app, 150.0, 600.0);
        }
        8 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            advice(app, AdviceLevel::Advanced);
            capture(app, "RIFT_ROCKFALL_ADVANCED");
        }
        9 => {
            app.in_match = false;
            app.screen = Screen::Practice;
            app.fields = [0, 1];
            click(app, 450.0, 876.0);
        }
        10 => {
            assert!(app.field_picker.is_some());
            for _ in 0..7 {
                app.ui.input_override = Some((vec2(480.0, 400.0), false, false));
                app.ui.wheel_override = Some(-1.0);
                app.draw();
            }
            app.ui.wheel_override = Some(0.0);
            assert_eq!(app.field_picker.as_ref().unwrap().list_scroll, 7);
        }
        11 => click(app, 480.0, 616.0),
        12 => {
            assert_eq!(
                app.field_picker.as_ref().unwrap().selected,
                ROCKFALL_PATH_ID
            );
            capture(app, "RIFT_ROCKFALL_PICKER");
            click(app, 870.0, 715.0);
        }
        13 => {
            assert_eq!(app.fields, [ROCKFALL_PATH_ID, 1]);
            assert!(app.field_picker.is_none());
            println!(
                "PASS: Rockfall rules, rejected play, movement, companion and field selection."
            );
            return true;
        }
        _ => panic!("Rockfall smoke exceeded its script"),
    }
    false
}
