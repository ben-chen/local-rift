//! Actual target, Predict, card inspection, hover and companion controls.
use super::*;
const UNIT: u32 = 45701;

fn setup(app: &mut App) {
    expansion_fixture(app, 457);
    app.game = Game::new_for_agents(3, 3, 457457, Difficulty::Easy);
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
        p.deck = vec![71, 8, 7];
        p.set_rune_setup([0, 6, 0, 0, 0, 0], [0; 6]).unwrap();
        p.rune_deck = vec![Domain::Calm; 6];
        p.energy_pool = 6;
        p.energy = 12;
        p.power_pool = [0; 6];
        p.any_power = 0;
    }
    app.game.players[0].hand = vec![457];
    app.game.players[0].units = vec![public_unit(UNIT, 110, None)];
    app.player_deck = 3;
    app.opponent_deck = 3;
    app.fields = [3, 3];
    app.clear_game_controls();
    app.lesson = None;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.companion.set_level(AdviceLevel::Beginner);
}

fn advice(app: &mut App) {
    app.companion.observe(&app.game);
    for _ in 0..100 {
        if app.companion.message().contains("Consult the Heavens") {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("missing Consult advice");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app),
        1 => app.ui.input_override = Some((vec2(315.0, 760.0), false, true)),
        2 => {
            assert_eq!(app.inspect, Some(457));
            capture(app, "RIFT_HEAVENS_INSPECT");
        }
        3 => {
            app.inspect = None;
            click(app, 315.0, 760.0);
        }
        4 => {
            assert!(matches!(app.selection, Some(Selection::Hand(0))));
            click_unit(app, UNIT);
        }
        5 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: UNIT
                }]
            );
            advice(app);
            capture(app, "RIFT_HEAVENS_TARGET");
            click(app, 1280.0, 510.0);
        }
        6 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 457);
            advice(app);
            capture(app, "RIFT_HEAVENS_BEGINNER");
            click(app, 150.0, 600.0);
        }
        7 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            advice(app);
            capture(app, "RIFT_HEAVENS_ADVANCED");
            click(app, 1285.0, 595.0);
        }
        8 => {
            assert_eq!(app.game.priority(), 1, "companion does not block passing");
            app.game.apply(Action::Pass).unwrap();
        }
        9 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::Predict {
                    player: 0,
                    card_id: 7
                })
            ));
            assert_eq!(app.game.turn_shield(UNIT).unwrap(), 2);
            capture(app, "RIFT_HEAVENS_PREDICT");
            click(app, 1285.0, 547.0);
        }
        10 => {
            assert!(app.game.decision.is_none());
            assert!(app.game.players[0].discard.contains(&457));
            app.ui.input_override = Some((unit_rect(app, UNIT).center(), false, false));
        }
        11 => {
            let text = super::super::live_unit_description(&app.game, 0, UNIT).unwrap();
            assert!(text.contains("Shield +2 this turn"));
            assert!(text.contains("Shield 3"));
            assert_eq!(app.hover_object, Some((0, UNIT)));
            capture_unit(app, UNIT, "RIFT_HEAVENS_SHIELD");
        }
        12 => return true,
        _ => panic!("Consult smoke exceeded its frames"),
    }
    false
}
