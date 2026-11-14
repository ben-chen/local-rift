//! Live restriction explanations, rejected unit play, and permitted movement.
use super::*;

const WARDEN: u32 = 46401;
const MOVER: u32 = 46402;
const GEAR: u32 = 46404;

fn setup(app: &mut App) {
    expansion_fixture(app, 464);
    app.game = Game::new_for_agents(0, 0, 464464, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![3, 3];
    app.game.battlefield_owner = vec![Some(0), Some(1)];
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
        p.energy_pool = 0;
        p.energy = 6;
        p.power_pool = [0; 6];
        p.any_power = 0;
    }
    app.game.players[0].hand = vec![71];
    app.game.players[0].units = vec![
        public_unit(MOVER, 71, None),
        public_unit(46403, 71, Some(0)),
    ];
    app.game.players[1].units = vec![public_unit(WARDEN, 464, Some(1))];
    app.game.players[0].gear = vec![GearState {
        id: GEAR,
        card_id: 125, // Seal of Focus: a real Calm gear, exhausted in base.
        lane: None,
        attached_to: None,
        ready: false,
        temporary: false,
        token: false,
    }];
    app.player_deck = 0;
    app.opponent_deck = 0;
    app.fields = [3, 3];
    app.clear_game_controls();
    app.lesson = None;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.companion.set_level(AdviceLevel::Beginner);
    app.ai_worker.invalidate();
}

fn advice(app: &mut App) {
    app.companion.observe(&app.game);
    for _ in 0..100 {
        if app.companion.message().contains("Warden") {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("missing Warden advice");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app),
        1 => app.ui.input_override = Some((unit_rect(app, WARDEN).center(), false, false)),
        2 => {
            assert_eq!(app.hover_object, Some((1, WARDEN)));
            capture_unit(app, WARDEN, "RIFT_WARDEN_SOURCE");
        }
        3 => {
            app.ui.wheel_override = Some(-12.0);
            capture_at(app, "RIFT_WARDEN_RULES", vec2(400.0, 450.0));
            app.ui.wheel_override = Some(0.0);
        }
        // Leave the scrollable preview before hovering a unit beneath it.
        4 => {}
        5 => app.ui.input_override = Some((unit_rect(app, MOVER).center(), false, false)),
        6 => {
            assert_eq!(app.hover_object, Some((0, MOVER)));
            capture_unit(app, MOVER, "RIFT_WARDEN_UNIT");
        }
        7 => capture_gear(app, GEAR, "RIFT_WARDEN_GEAR"),
        8 => click(app, 315.0, 760.0),
        9 => {
            assert!(matches!(app.selection, Some(Selection::Hand(0))));
            click_field(app, 0);
        }
        10 => {
            assert_eq!(app.game.players[0].hand, vec![71]);
            assert_eq!(app.game.players[0].energy, 6);
            assert!(app.toast.contains("Warden"), "{}", app.toast);
            capture(app, "RIFT_WARDEN_BLOCKED");
            app.selection = None;
        }
        11 => {
            advice(app);
            capture(app, "RIFT_WARDEN_BEGINNER");
            click(app, 150.0, 600.0);
        }
        12 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            advice(app);
            capture(app, "RIFT_WARDEN_ADVANCED");
            click_unit(app, MOVER);
        }
        13 => click_field(app, 0),
        14 => {
            let unit = app.game.players[0]
                .units
                .iter()
                .find(|u| u.id == MOVER)
                .unwrap();
            assert_eq!(unit.lane, Some(0), "Warden permits ordinary movement");
            assert!(!unit.ready);
            capture_unit(app, MOVER, "RIFT_WARDEN_MOVED");
        }
        15 => return true,
        _ => panic!("Warden smoke exceeded its script"),
    }
    false
}
