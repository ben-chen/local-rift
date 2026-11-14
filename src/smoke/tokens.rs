//! Public token fixtures, real hover/right-click/close controls, both owners.
use super::*;
const SOURCES: [usize; 6] = [448, 355, 34, 48, 54, 87];
const HOVERS: [&str; 6] = [
    "RIFT_TOKEN_SOLDIER_HOVER",
    "RIFT_TOKEN_SPRITE_HOVER",
    "RIFT_TOKEN_BIRD_HOVER",
    "RIFT_TOKEN_TENTACLE_HOVER",
    "RIFT_TOKEN_MECH_HOVER",
    "RIFT_TOKEN_RECRUIT_HOVER",
];
const PINS: [&str; 6] = [
    "RIFT_TOKEN_SOLDIER_PIN",
    "RIFT_TOKEN_SPRITE_PIN",
    "RIFT_TOKEN_BIRD_PIN",
    "RIFT_TOKEN_TENTACLE_PIN",
    "RIFT_TOKEN_MECH_PIN",
    "RIFT_TOKEN_RECRUIT_PIN",
];

fn setup(app: &mut App) {
    expansion_fixture(app, 448);
    app.game = Game::new_for_agents(3, 3, 778811, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![9, 9];
    app.game.battlefield_owner = vec![None; 2];
    for player in &mut app.game.players {
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.hand.clear();
        player.champion = None;
    }
    app.game.players[0].hand = vec![448];
    for (index, source) in SOURCES.into_iter().enumerate() {
        let mut unit = public_unit(
            9000 + index as u32,
            source,
            match index % 3 {
                0 => None,
                n => Some(n - 1),
            },
        );
        unit.token = true;
        unit.buff = local_rift::rules::token_spec(source).unwrap().might.into();
        unit.ready = index != 0;
        if index == 0 {
            unit.temporary_buff = 2;
            unit.damage = 1;
        }
        app.game.players[index / 3].units.push(unit);
    }
    app.game.players[0].gear = vec![
        GearState {
            id: 9700,
            card_id: 13,
            lane: None,
            attached_to: None,
            ready: true,
            temporary: false,
            token: true,
        },
        GearState {
            id: 9701,
            card_id: 444,
            lane: None,
            attached_to: Some(9000),
            ready: true,
            temporary: false,
            token: false,
        },
    ];
    app.player_deck = 3;
    app.opponent_deck = 3;
    app.fields = [9, 9];
    app.ai_worker.invalidate();
    app.selection = None;
    app.targets.clear();
    app.inspect = None;
    app.pinned_token = None;
    app.field_inspect = None;
    app.lesson = None;
    app.show_play_effects = false;
    app.tray = super::super::Tray::Board;
    app.hand_scroll = 0;
    app.chain_cursor = 0;
}

fn right_unit(app: &mut App, id: u32) {
    let r = unit_rect(app, id);
    app.ui.input_override = Some((r.center(), false, true));
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app),
        1 => capture(app, "RIFT_TOKEN_BOARD"),
        2..=19 => {
            let index = ((frame - 2) / 3) as usize;
            let id = 9000 + index as u32;
            match (frame - 2) % 3 {
                0 => {
                    capture_unit(app, id, HOVERS[index]);
                    assert!(app.hover_token.is_some());
                    right_unit(app, id);
                }
                1 => {
                    let pinned = app
                        .pinned_token
                        .as_ref()
                        .expect("Right-click opens token inspector");
                    assert_eq!(pinned.object_id, id);
                    assert_eq!(pinned.owner, index / 3);
                    assert_eq!(app.inspect, Some(SOURCES[index]));
                    capture(app, PINS[index]);
                    click(app, 960.0, 700.0);
                }
                _ => {
                    assert!(app.inspect.is_none());
                    assert!(app.pinned_token.is_none());
                }
            }
        }
        20 => click(app, 430.0, 660.0),
        21 => {
            assert!(matches!(app.tray, super::super::Tray::Loadout));
            capture(app, "RIFT_TOKEN_GOLD_TRAY");
            app.ui.input_override = Some((vec2(615.0, 748.0), false, true));
        }
        22 => {
            assert!(app.pinned_token.as_ref().unwrap().preview.gear);
            capture(app, "RIFT_TOKEN_GOLD_PIN");
            click(app, 960.0, 700.0);
        }
        23 => click(app, 285.0, 660.0),
        24 => app.ui.input_override = Some((vec2(315.0, 760.0), false, true)),
        25 => {
            assert_eq!(app.inspect, Some(448));
            assert!(app.pinned_token.is_none());
            capture(app, "RIFT_TOKEN_CREATOR_CARD");
            click(app, 960.0, 700.0);
        }
        26 => right_unit(app, 9000),
        27 => {
            assert_eq!(app.pinned_token.as_ref().unwrap().object_id, 9000);
            // Controlled departure tests stale inspection, not a claimed legal action.
            app.game.players[0].units.retain(|u| u.id != 9000);
            app.game.players[0]
                .gear
                .retain(|g| g.attached_to != Some(9000));
        }
        28 => {
            capture(app, "RIFT_TOKEN_DEPARTED");
            click(app, 960.0, 700.0);
        }
        29 => {
            assert!(app.inspect.is_none());
            setup(app);
        }
        30 => app.ui.input_override = Some((vec2(465.0, 748.0), false, true)),
        31 => {
            assert_eq!(app.pinned_token.as_ref().unwrap().object_id, 9001);
            capture(app, "RIFT_TOKEN_TRAY_PIN");
            click(app, 960.0, 700.0);
        }
        32 => {
            assert!(app.inspect.is_none());
            // Isolate one movable token and its actual attached equipment.
            app.game.players[0].units.retain(|unit| unit.id == 9000);
            app.game.players[1].units.clear();
            let unit = &mut app.game.players[0].units[0];
            unit.ready = true;
            unit.damage = 0;
            app.selection = None;
            app.group.clear();
            app.drag = None;
            app.hover_card = None;
            app.hover_rect = None;
        }
        33 => {
            click_unit(app, 9000);
            app.ui.pointer_override = Some((true, false));
        }
        34..=36 => {
            assert!(app.drag.is_some(), "Token press arms a drag");
            let rect = app
                .drop_zones
                .iter()
                .find(|(zone, _)| matches!(zone, crate::DropZone::Battlefield(0)))
                .unwrap()
                .1;
            app.ui.input_override = Some((
                vec2(rect.x + rect.w * 0.8, rect.y + rect.h * 0.8),
                false,
                false,
            ));
            app.ui.pointer_override = Some((frame != 36, frame == 36));
            if frame == 35 {
                app.draw();
                assert!(app.drag.as_ref().unwrap().active);
                if let Ok(path) = std::env::var("RIFT_TOKEN_DRAG") {
                    get_screen_data().export_png(&path);
                }
            }
        }
        37 => {
            assert!(app.drag.is_none());
            let unit = &app.game.players[0].units[0];
            assert_eq!(unit.id, 9000);
            assert!(unit.token);
            assert_eq!(unit.lane, Some(0));
            assert!(!unit.ready);
            assert_eq!(app.game.gear_location(0, 9701), Some(0));
            capture(app, "RIFT_TOKEN_MOVED");
        }
        38 => {
            return true;
        }
        _ => panic!("Token UI smoke exceeded its defined frames"),
    }
    false
}
