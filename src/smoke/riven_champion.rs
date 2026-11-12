//! Native Riven modes, legend selection and entry in controlled positions.
//! The authored training list is not the sourced Riven tournament registration.
use super::*;
const RIVEN: u32 = 43901;
const ENEMY: u32 = 43902;

fn setup(app: &mut App) {
    expansion_fixture(app, 439);
    app.game = Game::new_for_agents(3, 3, 439439, Difficulty::Easy);
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
        p.power_pool = [0; 6];
        p.any_power = 0;
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
        legend.spell_power = 0;
    }
    app.game.players[0]
        .units
        .push(public_unit(RIVEN, 439, None));
    app.game.players[0]
        .units
        .push(public_unit(43903, 71, Some(0)));
    app.game.players[1].units.push(public_unit(ENEMY, 71, None));
    app.fields = std::array::from_fn(|i| app.game.battlefields[i]);
    app.ai_worker.invalidate();
    app.inspect = None;
    app.field_inspect = None;
    app.lesson = None;
    app.chain_cursor = 0;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.equipment_cost = None;
    app.companion.set_level(AdviceLevel::Beginner);
    app.game
        .apply(Action::Move {
            unit_id: RIVEN,
            lane: Some(0),
        })
        .unwrap();
}

fn exile_setup(app: &mut App) {
    expansion_fixture(app, 439);
    app.game = Game::new_with_custom(
        [3, 3],
        [
            Some(local_rift::deck::CustomDeck::from_legend(10).unwrap()),
            None,
        ],
        149149,
        Difficulty::Easy,
        false,
    )
    .unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![9, 9];
    app.game.battlefield_owner = vec![Some(0), None];
    for p in &mut app.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy = 20;
        p.energy_pool = 20;
        p.power_pool = [0; 6];
    }
    app.game.players[0]
        .units
        .push(public_unit(14901, 71, Some(0)));
    app.game.players[0].hand = vec![439];
    app.fields = std::array::from_fn(|i| app.game.battlefields[i]);
    app.ai_worker.invalidate();
    app.inspect = None;
    app.lesson = None;
    app.legend_picker = None;
    app.deck_editor = None;
    app.prefs.exact_payment = false;
    app.companion.set_level(AdviceLevel::Beginner);
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app),
        1 => {
            capture(app, "RIFT_CHAMPION_MODES");
            click(app, 1280.0, 468.0);
        }
        2 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[0].units[0].buff, 0);
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        3 => {
            assert_eq!(app.game.players[0].units[0].buff, 1);
            capture_unit(app, RIVEN, "RIFT_CHAMPION_BUFF_HISTORY");
            app.game.players[0].units[0].ready = true;
            app.game
                .apply(Action::Move {
                    unit_id: RIVEN,
                    lane: None,
                })
                .unwrap();
        }
        4 => {
            assert!(!app.game.trigger_mode_available(0).unwrap());
            capture(app, "RIFT_CHAMPION_USED_MODE");
            click(app, 1280.0, 468.0);
        }
        5 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerMode { .. })
            ));
            click(app, 1280.0, 564.0);
        }
        6 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[0].units[0].buff, 1);
            assert!(!app.game.players[0].units[0].ready);
            capture(app, "RIFT_CHAMPION_READY_PENDING");
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        7 => {
            assert!(app.game.players[0].units[0].ready);
            assert_eq!(app.game.players[0].units[0].buff, 0);
            capture_unit(app, RIVEN, "RIFT_CHAMPION_READY_HISTORY");
            // A fresh Buff fixture lets the last mode demonstrate its linked Stun.
            app.game.players[0].units[0].buff = 1;
            app.game.buffed_units.push(RIVEN);
            app.game
                .apply(Action::Move {
                    unit_id: RIVEN,
                    lane: Some(0),
                })
                .unwrap();
            app.companion.set_level(AdviceLevel::Advanced);
        }
        8 => {
            capture(app, "RIFT_CHAMPION_LAST_MODE");
            click(app, 1280.0, 516.0);
        }
        9 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerTargets { card_id: 439, .. })
            ));
            click_unit(app, ENEMY);
        }
        10 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: ENEMY
                }]
            );
            capture(app, "RIFT_CHAMPION_STUN_TARGET");
            click(app, 1280.0, 510.0);
        }
        11 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[0].units[0].buff, 1);
            assert!(!app.game.players[1].units[0].stunned);
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        12 => {
            assert_eq!(app.game.players[0].units[0].buff, 0);
            assert!(app.game.players[1].units[0].stunned);
            capture_unit(app, ENEMY, "RIFT_CHAMPION_STUN_RESOLVED");
        }
        13 => app.inspect = Some(439),
        14 => {
            capture(app, "RIFT_CHAMPION_RULES");
        }
        15 => {
            app.inspect = None;
            app.legend_picker = Some(0);
            click(app, 500.0, 661.0);
        }
        16 => {
            assert_eq!(app.legend_picker, Some(8));
            click(app, 450.0, 361.0);
        }
        17 => {
            assert_eq!(app.legend_picker, Some(10));
            assert!(app.legend_art[10].is_some());
            capture(app, "RIFT_CHAMPION_EXILE_PICKER");
            click(app, 840.0, 700.0);
        }
        18 => {
            assert_eq!(app.deck_editor.as_ref().unwrap().deck.legend, 10);
            app.deck_editor.as_ref().unwrap().deck.validate().unwrap();
            capture(app, "RIFT_CHAMPION_EXILE_TRAINING");
            exile_setup(app);
            click(app, 315.0, 760.0);
        }
        19 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            app.destination = Some(0);
            let preview = app
                .game
                .play_choices(&app.selected_play_action().unwrap())
                .unwrap();
            assert!(preview.legend_entry_ready.is_some());
            capture(app, "RIFT_CHAMPION_EXILE_READY_PREVIEW");
            click(app, 1280.0, 510.0);
        }
        20 => {
            let riven = app.game.players[0]
                .units
                .iter()
                .find(|u| u.card_id == 439)
                .unwrap();
            assert!(riven.ready);
            assert_eq!(riven.lane, Some(0));
            assert_eq!(app.game.players[0].energy, 16);
            capture_unit(app, riven.id, "RIFT_CHAMPION_EXILE_READY_ENTRY");
            exile_setup(app);
            click(app, 315.0, 760.0);
        }
        21 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            assert_eq!(app.destination, None);
            click(app, 1280.0, 510.0);
        }
        22 => {
            let riven = app.game.players[0]
                .units
                .iter()
                .find(|u| u.card_id == 439)
                .unwrap();
            assert!(!riven.ready);
            assert_eq!(riven.lane, None);
            capture_unit(app, riven.id, "RIFT_CHAMPION_EXILE_BASE_ENTRY");
            app.lesson = Some(10);
        }
        23 => {
            capture(app, "RIFT_CHAMPION_EXILE_LESSON");
        }
        24 => {
            exile_setup(app);
            app.game.battlefields[0] = 7;
            app.game.players[0].hand = vec![440];
            app.game.players[0].power_pool = [4; 6];
            click(app, 315.0, 760.0);
        }
        25 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            app.destination = Some(0);
            let preview = app
                .game
                .play_choices(&app.selected_play_action().unwrap())
                .unwrap();
            assert_eq!(preview.baron_pit_entry, Some(2));
            assert!(preview.legend_entry_ready.is_some());
            capture(app, "RIFT_CHAMPION_BARON_PREVIEW");
            click(app, 1280.0, 510.0);
        }
        26 => {
            let baron = app.game.players[0]
                .units
                .iter()
                .find(|u| u.card_id == 440)
                .unwrap();
            assert!(baron.ready);
            assert_eq!(baron.lane, Some(2));
            assert_eq!(app.game.players[0].energy, 11);
            assert_eq!(app.game.battlefields, vec![7, 9, 1000]);
            capture_unit(app, baron.id, "RIFT_CHAMPION_BARON_ENTRY");
            for _ in 0..12 {
                if !app.game.in_window() {
                    break;
                }
                assert!(app.game.decision.is_none(), "{:?}", app.game.decision);
                app.game.apply(Action::Pass).unwrap();
            }
            assert!(!app.game.in_window());
        }
        27 => {
            capture(app, "RIFT_CHAMPION_BARON_BOARD");
            click_unit(app, 14901);
        }
        28 => {
            assert!(app.selection == Some(Selection::Unit(14901)));
            capture(app, "RIFT_CHAMPION_BARON_MOVE_CHOICES");
            click(app, 1228.0, 462.0);
        }
        29 => {
            let moved = app.game.players[0]
                .units
                .iter()
                .find(|u| u.id == 14901)
                .unwrap();
            assert_eq!(moved.lane, Some(2));
            assert!(!moved.ready);
            assert_eq!(app.game.unit_might(moved).unwrap(), 7);
            capture_unit(app, 14901, "RIFT_CHAMPION_BARON_MOVED");
            app.selection = None;
            let field = crate::board_layout::fields(3)[2];
            click(app, field.x + field.w - 39.0, field.y + 16.0);
        }
        30 => {
            assert_eq!(app.field_inspect, Some(1000));
            capture(app, "RIFT_CHAMPION_BARON_PIT_RULES");
            app.field_inspect = None;
            app.game.rematch(147149);
            assert_eq!(app.game.battlefields, vec![7, 9]);
            app.game.start_match().unwrap();
            app.selection = None;
        }
        31 => {
            capture(app, "RIFT_CHAMPION_BARON_REMATCH");
        }
        32 => return true,
        _ => {}
    }
    false
}
