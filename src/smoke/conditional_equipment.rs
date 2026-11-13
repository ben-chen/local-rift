//! Actual Equip controls, live bonus explanations and complete equipment faces.
use super::*;
const BEARER: u32 = 44401;
const PARTNER: u32 = 44402;
const THIRD: u32 = 44403;
const GEAR: u32 = 44400;

fn setup(app: &mut App, card: usize) {
    expansion_fixture(app, card);
    app.game = Game::new_for_agents(3, 3, 444445, Difficulty::Easy);
    app.game.start_match().unwrap();
    app.game.battlefields = vec![9, 9];
    app.game.battlefield_owner = vec![Some(0), None];
    for p in &mut app.game.players {
        p.units.clear();
        p.gear.clear();
        p.hand.clear();
        p.hidden.clear();
        p.discard.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy = 20;
        p.energy_pool = 20;
        p.power_pool = [10; 6];
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
    }
    let lane = if card == 444 { None } else { Some(0) };
    app.game.players[0].units = vec![
        public_unit(BEARER, 69, lane),
        public_unit(PARTNER, 69, lane),
    ];
    app.game.players[0].gear.push(GearState {
        id: GEAR,
        card_id: card,
        lane: None,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    app.fields = [9, 9];
    app.ai_worker.invalidate();
    app.selection = None;
    app.targets.clear();
    app.inspect = None;
    app.field_inspect = None;
    app.lesson = None;
    app.chain_cursor = 0;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.tray = super::super::Tray::Loadout;
    app.gear_owner = 0;
    app.companion.set_level(AdviceLevel::Beginner);
}
fn tip(app: &mut App, card: usize, level: AdviceLevel) {
    app.companion.set_level(level);
    app.companion.observe(&app.game);
    for _ in 0..80 {
        if app
            .companion
            .message()
            .contains(app.game.catalog[card].name)
        {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("Equipment advice unavailable");
}
fn might(app: &App) -> u64 {
    app.game.unit_might(&app.game.players[0].units[0]).unwrap()
}
pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    if frame == 28 {
        return true;
    }
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    let card = if frame < 14 { 444 } else { 445 };
    match frame % 14 {
        0 => setup(app, card),
        1 => {
            tip(app, card, AdviceLevel::Beginner);
            click(app, 615.0, 760.0);
        }
        2 => {
            assert!(app.selection == Some(Selection::Gear(GEAR)));
            click_unit(app, BEARER);
        }
        3 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: BEARER
                }]
            );
            capture(
                app,
                if card == 444 {
                    "RIFT_BRUTALIZER_EQUIP"
                } else {
                    "RIFT_HAND_HAMMER_EQUIP"
                },
            );
            click(app, 1285.0, 522.0);
        }
        4 => {
            assert_eq!(might(app), 8);
            assert!(
                !app.game
                    .gear_attachment_status(0, GEAR)
                    .unwrap()
                    .attached_this_turn
            );
            assert_eq!(app.game.chain.len(), 1);
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        5 => {
            assert_eq!(might(app), 11);
            assert!(
                app.game
                    .gear_attachment_status(0, GEAR)
                    .unwrap()
                    .attached_this_turn
            );
            capture_unit(
                app,
                BEARER,
                if card == 444 {
                    "RIFT_BRUTALIZER_ACTIVE"
                } else {
                    "RIFT_HAND_HAMMER_ACTIVE"
                },
            );
        }
        6 => {
            app.inspect = Some(card);
            capture(
                app,
                if card == 444 {
                    "RIFT_BRUTALIZER_INSPECT"
                } else {
                    "RIFT_HAND_HAMMER_INSPECT"
                },
            );
            app.inspect = None;
        }
        7 => {
            tip(app, card, AdviceLevel::Advanced);
            capture(
                app,
                if card == 444 {
                    "RIFT_BRUTALIZER_ADVICE"
                } else {
                    "RIFT_HAND_HAMMER_ADVICE"
                },
            );
        }
        8 => {
            if card == 444 {
                app.game.apply(Action::EndTurn).unwrap();
            } else {
                app.game.players[0].units.push(public_unit(THIRD, 69, None));
                app.game
                    .apply(Action::Move {
                        unit_id: THIRD,
                        lane: Some(0),
                    })
                    .unwrap();
            }
        }
        9 => {
            assert_eq!(might(app), 9);
            if card == 444 {
                tip(app, card, AdviceLevel::Advanced);
                assert!(app.companion.message().contains("expired"));
            }
            capture_unit(
                app,
                BEARER,
                if card == 444 {
                    "RIFT_BRUTALIZER_EXPIRED"
                } else {
                    "RIFT_HAND_HAMMER_THIRD_ALLY"
                },
            );
        }
        10 if card == 445 => {
            app.game
                .apply(Action::Move {
                    unit_id: PARTNER,
                    lane: None,
                })
                .unwrap();
        }
        11 if card == 445 => {
            assert_eq!(might(app), 11);
            capture_unit(app, BEARER, "RIFT_HAND_HAMMER_RESTORED");
        }
        10..=13 => {}
        _ => unreachable!(),
    }
    false
}
