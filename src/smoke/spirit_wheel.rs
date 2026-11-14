//! Inspect, play, pay and decline Spirit Wheel with actual desktop controls.
use super::*;

const UNIT: u32 = 454001;

fn setup(app: &mut App) {
    expansion_fixture(app, 454);
    app.game = Game::new_for_agents(0, 1, 454454, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![3, 3];
    app.game.battlefield_owner = vec![None; 2];
    for player in &mut app.game.players {
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.hand.clear();
        player.discard.clear();
        player.champion = None;
        player.deck = vec![71; 20];
        player.set_rune_setup([0, 3, 0, 0, 3, 0], [0; 6]).unwrap();
        player.rune_deck = vec![Domain::Calm; 6];
        player.energy = 12;
        player.energy_pool = 6;
        player.power_pool = [0; 6];
        player.any_power = 0;
    }
    fixture_unit(app, 0, UNIT, 71);
    app.game.players[0].units[0].ready = true;
    app.game.players[0].hand = vec![454, 8];
    app.player_deck = 0;
    app.opponent_deck = 1;
    app.fields = [3, 3];
    app.inspect = None;
    app.field_inspect = None;
    app.lesson = None;
    app.rune_menu = None;
    app.chain_cursor = 0;
    app.prefs.coach = true;
    app.ai_worker.invalidate();
}

fn decline_legend(app: &mut App) {
    let Some(Decision::OrderTriggers { ref options, .. }) = app.game.decision else {
        panic!("Wheel and Irelia must both trigger");
    };
    let id = options.iter().find(|o| o.card_id == 0).unwrap().id;
    app.game
        .apply(Action::OrderTrigger { trigger_id: id })
        .unwrap();
    app.game.apply(Action::Choose { accept: false }).unwrap();
    assert!(matches!(
        app.game.decision,
        Some(Decision::TriggerPayment { card_id: 454, .. })
    ));
}

fn advice(app: &mut App, level: AdviceLevel) {
    app.companion.set_level(level);
    app.companion.observe(&app.game);
    for _ in 0..80 {
        if app.companion.message().contains("Wheel") {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("missing Spirit Wheel advice");
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
            assert_eq!(app.inspect, Some(454));
            capture(app, "RIFT_WHEEL_INSPECT");
        }
        3 => {
            app.inspect = None;
            click(app, 315.0, 760.0);
        }
        4 => click(app, 1285.0, 510.0),
        5 => {
            assert_eq!(app.game.players[0].gear[0].card_id, 454);
            assert!(app.game.players[0].gear[0].ready);
            assert_eq!(app.game.players[0].hand, vec![8]);
            click(app, 315.0, 760.0);
        }
        6 => click_unit(app, UNIT),
        7 => click(app, 1285.0, 510.0),
        8 => {
            decline_legend(app);
            click(app, 400.0, 663.0);
        }
        9 => {
            assert_eq!(app.game.players[0].energy, 9);
            assert!(app.game.players[0].gear[0].ready);
            assert_eq!(app.game.chain.len(), 1);
            advice(app, AdviceLevel::Beginner);
            capture(app, "RIFT_WHEEL_COST_BEGINNER");
            click(app, 150.0, 600.0);
        }
        10 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            advice(app, AdviceLevel::Advanced);
            capture(app, "RIFT_WHEEL_COST_ADVANCED");
        }
        11 => click(app, 1285.0, 510.0),
        12 => {
            assert_eq!(app.game.players[0].energy, 8);
            assert!(!app.game.players[0].gear[0].ready);
            assert!(app.game.players[0].hand.is_empty());
            assert_eq!(app.game.chain.len(), 2);
            assert_eq!(app.game.chain[1].card_id, 454);
            capture(app, "RIFT_WHEEL_PAID");
            click(app, 1285.0, 595.0);
        }
        13 => {
            assert_eq!(app.game.priority(), 1, "Mote cannot intercept passing");
            assert!(app.game.players[0].hand.is_empty());
            app.game.apply(Action::Pass).unwrap();
            click(app, 295.0, 663.0);
        }
        14 => {
            assert_eq!(app.game.players[0].hand, vec![71]);
            assert_eq!(app.game.chain.len(), 1);
            assert_eq!(app.game.chain[0].card_id, 8);
            capture(app, "RIFT_WHEEL_DRAW");
        }
        15 => {
            setup(app);
            for targets in [
                vec![],
                vec![Target::Unit {
                    player: 0,
                    unit_id: UNIT,
                }],
            ] {
                app.game
                    .apply(Action::PlayCardWithTargets {
                        hand_index: 0,
                        lane: None,
                        targets,
                    })
                    .unwrap();
            }
            decline_legend(app);
            click(app, 400.0, 663.0);
        }
        16 => click(app, 1285.0, 548.0),
        17 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[0].energy, 9);
            assert!(app.game.players[0].gear[0].ready);
            assert_eq!(app.game.chain.len(), 1);
            assert!(app.game.players[0].hand.is_empty());
            capture(app, "RIFT_WHEEL_DECLINED");
        }
        18 => return true,
        _ => panic!("Wheel scenario exceeded its frames"),
    }
    false
}
