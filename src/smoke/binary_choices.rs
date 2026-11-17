//! Authored Irelia positions; real native buttons answer the resulting choices.
use super::*;
use local_rift::legends::LegendTrigger;

fn setup(app: &mut App, payable: bool) {
    recovery::setup(app, 485);
    app.game.legend[0].ready = true;
    let player = &mut app.game.players[0];
    player.set_rune_setup([0; 6], [0; 6]).unwrap();
    player.power_pool = [0; 6];
    player.any_power = u16::from(payable);
    player.energy_pool = 2;
    player.energy = 2;
    player.units = vec![public_unit(48601, 71, None)];
    player.units[0].ready = false;
    app.game
        .apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: 0,
                unit_id: 48601,
            }],
        })
        .unwrap();
    assert!(matches!(app.game.decision, Some(Decision::Legend { .. })));
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100., -100.), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.);
    match frame {
        0 => setup(app, false),
        1 => {
            assert!(!app.game.binary_choices().unwrap().unwrap().options[0].enabled());
            capture(app, "RIFT_BINARY_UNAVAILABLE");
            click(app, 1280., 505.);
        }
        2 => {
            assert!(
                matches!(app.game.decision, Some(Decision::Legend { .. })),
                "disabled payment must not act"
            );
            assert!(app.game.legend[0].ready);
            assert_eq!(app.game.chain.len(), 1);
            click(app, 1280., 546.);
        }
        3 => {
            assert!(app.game.decision.is_none(), "decline is always accessible");
            assert!(app.game.legend[0].ready);
            assert_eq!(app.game.players[0].any_power, 0);
            setup(app, true);
        }
        4 => {
            capture(app, "RIFT_BINARY_PAYABLE");
            click(app, 1280., 505.);
        }
        5 => {
            assert!(app.game.decision.is_none());
            assert!(!app.game.legend[0].ready);
            assert_eq!(app.game.players[0].any_power, 0);
            assert!(!app.game.players[0].units[0].ready);
            assert_eq!(
                app.game.chain.len(),
                2,
                "opponent can respond to the ready ability"
            );
            capture(app, "RIFT_BINARY_RESPONSE");
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        6 => {
            assert!(app.game.players[0].units[0].ready);
            setup(app, false);
            app.game.legend[0].ready = false;
            app.game.players[0].energy_pool = 0;
            app.game.players[0].energy = 0;
            // A separate authored conquest trigger tests its Energy answer.
            app.game.decision = Some(Decision::Legend {
                player: 0,
                trigger: LegendTrigger::IreliaConquer,
            });
        }
        7 => {
            capture(app, "RIFT_BINARY_CONQUER");
            click(app, 1280., 505.);
        }
        8 => {
            assert!(matches!(app.game.decision, Some(Decision::Legend { .. })));
            assert!(!app.game.legend[0].ready);
            click(app, 1280., 546.);
        }
        9 => {
            assert!(app.game.decision.is_none());
            assert!(!app.game.legend[0].ready);
            return true;
        }
        _ => {}
    }
    false
}
