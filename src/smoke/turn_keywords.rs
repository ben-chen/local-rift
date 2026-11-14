//! Card inspection, real targeting, drag-to-attack and unobtrusive advice.
use super::*;
const ALLY: u32 = 46101;
const YUUMI: u32 = 46201;

fn setup(app: &mut App, yuumi: bool) {
    expansion_fixture(app, if yuumi { 462 } else { 461 });
    app.game = Game::new_for_agents(3, 3, 461462, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![3, 3];
    app.game.battlefield_owner = vec![Some(1), Some(0)];
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
        p.power_pool = [0; 6];
        p.any_power = 0;
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
    }
    app.game.players[0].hand = vec![if yuumi { 462 } else { 461 }];
    app.game.players[0].units = vec![public_unit(ALLY, 110, yuumi.then_some(0))];
    if yuumi {
        app.game.players[0]
            .units
            .push(public_unit(YUUMI, 462, None));
        app.game.players[1]
            .units
            .push(public_unit(46202, 71, Some(0)));
    }
    app.player_deck = 3;
    app.opponent_deck = 3;
    app.fields = [3, 3];
    app.clear_game_controls();
    app.lesson = None;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.companion.set_level(AdviceLevel::Beginner);
    app.ai_worker.invalidate();
}

fn advice(app: &mut App, name: &str) {
    app.companion.observe(&app.game);
    for _ in 0..100 {
        if app.companion.message().contains(name) {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("missing {name} advice");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app, false),
        1 => app.ui.input_override = Some((vec2(315.0, 760.0), false, true)),
        2 => {
            assert_eq!(app.inspect, Some(461));
            capture(app, "RIFT_BLOCK_INSPECT");
            app.inspect = None;
        }
        3 => click(app, 315.0, 760.0),
        4 => {
            assert!(matches!(app.selection, Some(Selection::Hand(0))));
            click_unit(app, ALLY);
        }
        5 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: ALLY
                }]
            );
            advice(app, "Block");
            capture(app, "RIFT_BLOCK_TARGET");
            click(app, 1280.0, 510.0);
        }
        6 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 461);
            advice(app, "Block");
            capture(app, "RIFT_BLOCK_BEGINNER");
            click(app, 150.0, 600.0);
        }
        7 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            advice(app, "Block");
            capture(app, "RIFT_BLOCK_ADVANCED");
            click(app, 1285.0, 595.0);
        }
        8 => {
            assert_eq!(app.game.priority(), 1, "Mote does not block passing");
            app.game.apply(Action::Pass).unwrap();
        }
        9 => {
            assert_eq!(app.game.turn_shield(ALLY).unwrap(), 3);
            assert!(app.game.turn_tank(ALLY));
            app.ui.input_override = Some((unit_rect(app, ALLY).center(), false, false));
        }
        10 => {
            assert_eq!(app.hover_object, Some((0, ALLY)));
            let text = super::super::live_unit_description(&app.game, 0, ALLY).unwrap();
            assert!(
                text.contains("Shield +3 this turn") && text.contains("Tank granted this turn")
            );
            capture_unit(app, ALLY, "RIFT_BLOCK_GRANTED");
        }
        11 => setup(app, true),
        12 => app.ui.input_override = Some((vec2(315.0, 760.0), false, true)),
        13 => {
            assert_eq!(app.inspect, Some(462));
            capture(app, "RIFT_YUUMI_INSPECT");
            app.inspect = None;
        }
        14 => {
            click_unit(app, YUUMI);
            app.ui.pointer_override = Some((true, false));
        }
        15 | 16 => {
            click_field(app, 0);
            let point = app.ui.input_override.unwrap().0;
            app.ui.input_override = Some((point, false, false));
            app.ui.pointer_override = Some((frame == 15, frame == 16));
        }
        17 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerTargets { card_id: 462, .. })
            ));
            advice(app, "Yuumi");
            capture(app, "RIFT_YUUMI_BEGINNER");
            click(app, 150.0, 600.0);
        }
        18 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            advice(app, "Yuumi");
            capture(app, "RIFT_YUUMI_ADVANCED");
            click_unit(app, ALLY);
        }
        19 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: ALLY
                }]
            );
            click(app, 1280.0, 510.0);
        }
        20 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 462);
            click(app, 1285.0, 595.0);
        }
        21 => {
            assert_eq!(app.game.priority(), 1);
            app.game.apply(Action::Pass).unwrap();
        }
        22 => {
            assert!(app.game.turn_tank(ALLY));
            let text = super::super::live_unit_description(&app.game, 0, ALLY).unwrap();
            assert!(text.contains("Might +3 this turn") && text.contains("Tank granted this turn"));
            app.ui.input_override = Some((unit_rect(app, ALLY).center(), false, false));
        }
        23 => {
            assert_eq!(app.hover_object, Some((0, ALLY)));
            capture_unit(app, ALLY, "RIFT_YUUMI_GRANTED");
        }
        24 => return true,
        _ => panic!("turn keyword smoke exceeded its frames"),
    }
    false
}
