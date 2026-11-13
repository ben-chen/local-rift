//! Controlled public board; actual drag/drop, confirmation and inspector controls.
use super::*;
const VICTIM: u32 = 44601;
const BASE_UNIT: u32 = 44602;

fn setup(app: &mut App) {
    expansion_fixture(app, 446);
    app.game = Game::new_for_agents(2, 2, 446446, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![9, 9];
    app.game.battlefield_owner = vec![None; 2];
    for player in &mut app.game.players {
        player.units.clear();
        player.gear.clear();
        player.hand.clear();
        player.hidden.clear();
        player.discard.clear();
        player.champion = None;
        player.deck = vec![71; 20];
        player.set_rune_setup([0; 6], [0; 6]).unwrap();
        player.energy = 10;
        player.energy_pool = 10;
        player.power_pool = [10; 6];
    }
    app.game.players[0].hand = vec![446];
    app.game.players[1].units = vec![
        public_unit(VICTIM, 69, Some(0)),
        public_unit(BASE_UNIT, 69, None),
    ];
    app.player_deck = 2;
    app.opponent_deck = 2;
    app.fields = [9, 9];
    app.ai_worker.invalidate();
    app.selection = None;
    app.targets.clear();
    app.inspect = None;
    app.field_inspect = None;
    app.lesson = None;
    app.show_play_effects = false;
    app.chain_cursor = 0;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
}

fn advice(app: &mut App, level: AdviceLevel) {
    app.companion.set_level(level);
    app.companion.observe(&app.game);
    for _ in 0..80 {
        if app.companion.message().contains("Hidden Blade") {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("Hidden Blade advice must be available while held");
}

fn drag_to(app: &mut App, unit_id: u32, release: bool) {
    let rect = app
        .drop_zones
        .iter()
        .find(|(zone, _)| matches!(zone, crate::DropZone::Unit(1, id) if *id==unit_id))
        .unwrap()
        .1;
    app.ui.input_override = Some((
        vec2(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0),
        false,
        false,
    ));
    app.ui.pointer_override = Some((!release, release));
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app),
        1 => {
            advice(app, AdviceLevel::Beginner);
            capture(app, "RIFT_HIDDEN_BLADE_BEGINNER");
            click(app, 315.0, 760.0);
            app.ui.pointer_override = Some((true, false));
        }
        2 | 3 => drag_to(app, BASE_UNIT, frame == 3),
        4 => {
            assert!(app.game.chain.is_empty());
            assert_eq!(app.game.players[0].hand, vec![446]);
            assert_eq!(app.game.players[0].energy, 10);
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Order)],
                10
            );
            assert!(app.targets.is_empty());
            advice(app, AdviceLevel::Advanced);
            capture(app, "RIFT_HIDDEN_BLADE_REJECTED_DROP");
            click(app, 315.0, 760.0);
            app.ui.pointer_override = Some((true, false));
        }
        5 | 6 => drag_to(app, VICTIM, frame == 6),
        7 => {
            capture(app, "RIFT_HIDDEN_BLADE_TARGET");
            if app.game.chain.is_empty() {
                assert_eq!(
                    app.targets,
                    vec![Target::Unit {
                        player: 1,
                        unit_id: VICTIM
                    }]
                );
                click(app, 1280.0, 510.0);
            }
        }
        8 => {
            assert!(
                app.toast_until <= get_time(),
                "Successful drop kept a stale error"
            );
            assert_eq!(app.game.chain.last().unwrap().card_id, 446);
            assert_eq!(app.game.players[0].energy, 8);
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Order)],
                9
            );
            assert!(app.game.players[1].hand.is_empty());
            capture(app, "RIFT_HIDDEN_BLADE_RESPONSE");
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        9 => {
            assert!(app.game.chain.is_empty());
            assert!(app.game.players[1].units.iter().all(|u| u.id != VICTIM));
            assert!(app.game.players[1].units.iter().any(|u| u.id == BASE_UNIT));
            assert_eq!(app.game.players[1].hand, vec![71, 71]);
            capture(app, "RIFT_HIDDEN_BLADE_RESOLVED");
            app.inspect = Some(446);
        }
        10 => capture(app, "RIFT_HIDDEN_BLADE_INSPECT"),
        11 => return true,
        _ => panic!("Hidden Blade smoke exceeded its defined frames"),
    }
    false
}
