//! Shared native trash and Repeat controls, both advice levels, complete faces.
use super::*;

fn settle(app: &mut App) {
    for _ in 0..30 {
        if app.game.chain.is_empty() {
            return;
        }
        assert!(app.game.decision.is_none());
        app.game.apply(Action::Pass).unwrap();
    }
    panic!("return-option chain did not settle");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100., -100.), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.);
    match frame {
        0 => {
            recovery::setup(app, 482);
            app.game.players[0].discard = vec![3, 71];
        }
        1 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        2 => {
            assert_eq!(app.inspect, Some(482));
            capture(app, "RIFT_RETURN_MORBID_INSPECT");
            app.inspect = None;
            click(app, 315., 760.);
        }
        3 => {
            recovery::advice(app, "Morbid Return chooses");
            capture(app, "RIFT_RETURN_MORBID_BEGINNER");
            app.companion.set_level(AdviceLevel::Advanced);
        }
        4 => {
            recovery::advice(app, "Morbid Return recovers");
            capture(app, "RIFT_RETURN_MORBID_ADVANCED");
            click(app, 500., 665.);
        }
        5 => {
            assert!(matches!(app.tray, crate::Tray::Trash));
            assert!(
                !app.card_target_candidate(app.game.trash_target(0, 0).unwrap())
                    .unwrap()
            );
            capture(app, "RIFT_RETURN_MORBID_TARGETS");
            click(app, 465., 760.);
        }
        6 => {
            assert_eq!(app.current_targets().len(), 1);
            click(app, 1280., 510.);
        }
        7 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 482);
            capture(app, "RIFT_RETURN_MORBID_CHAIN");
            settle(app);
        }
        8 => {
            assert_eq!(app.game.players[0].hand, vec![71]);
            assert_eq!(app.game.players[0].discard, vec![3, 482]);
            capture(app, "RIFT_RETURN_MORBID_RECOVERED");
        }
        9 => {
            recovery::setup(app, 483);
            app.game.players[0].discard.clear();
            app.game.players[0].units = vec![public_unit(48302, 71, Some(0))];
            app.game.players[1].units = vec![
                public_unit(48301, 71, Some(0)),
                public_unit(48303, 71, None),
            ];
            app.game.battlefield_owner[0] = Some(0);
            app.game.showdown = Some(local_rift::game::Showdown {
                lane: 0,
                attacker: 1,
                focus: 0,
                passes: 0,
                combat: true,
            });
        }
        10 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        11 => {
            assert_eq!(app.inspect, Some(483));
            capture(app, "RIFT_RETURN_DREAD_INSPECT");
            app.inspect = None;
            click(app, 315., 760.);
        }
        12 => {
            recovery::advice(app, "Existential Dread chooses");
            capture(app, "RIFT_RETURN_DREAD_BEGINNER");
            click_unit(app, 48302);
        }
        13 => {
            assert!(app.targets.is_empty(), "a defender cannot be chosen");
            click_unit(app, 48303);
        }
        14 => {
            assert!(app.targets.is_empty(), "an enemy in base is not attacking");
            click_unit(app, 48301);
        }
        15 => {
            assert_eq!(app.targets.len(), 1);
            capture(app, "RIFT_RETURN_DREAD_TARGET");
            click(app, 1220., 468.);
        }
        16 => {
            assert!(app.options.repeat);
            click(app, 1340., 468.);
        }
        17 => {
            assert!(app.editing_repeat);
            click_unit(app, 48301);
        }
        18 => {
            assert_eq!(app.repeat_targets.len(), 1);
            capture(app, "RIFT_RETURN_DREAD_REPEAT");
            app.companion.set_level(AdviceLevel::Advanced);
        }
        19 => {
            recovery::advice(app, "Repeat can stun then return");
            capture(app, "RIFT_RETURN_DREAD_ADVANCED");
            click(app, 1280., 510.);
        }
        20 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 483);
            capture(app, "RIFT_RETURN_DREAD_CHAIN");
            settle(app);
        }
        21 => {
            assert!(!app.game.players[1].units.iter().any(|u| u.id == 48301));
            assert!(app.game.players[1].units.iter().any(|u| u.id == 48303));
            assert_eq!(app.game.players[1].hand, vec![71]);
            capture(app, "RIFT_RETURN_DREAD_RETURNED");
        }
        22 => {
            println!("return-options smoke passed");
            return true;
        }
        _ => {}
    }
    false
}
