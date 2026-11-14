//! Click through dated setup, full-registration rejection and historical practice.
use super::*;
use crate::format_ui::FormatChoice;
use local_rift::deck::CustomDeck;

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    match frame {
        0 => {
            app.in_match = false;
            app.native_match = None;
            app.series_lobby = false;
            app.screen = Screen::Practice;
            app.select_preset(0, 0);
            app.select_preset(1, 5);
            app.prefs.format = FormatChoice::On("2026-11-22".into());
        }
        1 => click(app, 1280.0, 457.0),
        2 => {
            assert!(app.format_picker.is_some());
            assert!(!app.setup_format_check(0, false).unwrap().is_empty());
            assert!(!app.setup_format_check(1, false).unwrap().is_empty());
            capture(app, "RIFT_FORMAT_UPCOMING");
            click(app, 982.0, 261.0);
        }
        3 => {
            for seat in 0..2 {
                assert!(
                    app.setup_format_check(seat, false)
                        .unwrap_err()
                        .contains("Stellacorn")
                );
            }
            capture(app, "RIFT_FORMAT_BANNED");
            click(app, 720.0, 716.0);
        }
        4 => click(app, 1320.0, 46.0),
        5 => {
            assert!(!app.in_match);
            assert!(app.format_picker.is_some());
            click(app, 713.0, 261.0);
        }
        6 => {
            assert_eq!(app.prefs.format, FormatChoice::Snapshot);
            assert!(app.setup_format_check(0, false).is_ok());
            capture(app, "RIFT_FORMAT_HISTORICAL");
            click(app, 720.0, 716.0);
        }
        7 => click(app, 1320.0, 46.0),
        8 => {
            assert!(app.in_match);
            assert_eq!(app.game.phase, Phase::Mulligan);
            assert_eq!(app.game.format_date(), None);
            let hand = app.game.players[0].hand.clone();
            let log = app.game.log.clone();
            app.prefs.format = FormatChoice::On("2026-11-23".into());
            app.reset();
            assert!(app.in_match && app.format_picker.is_some());
            assert_eq!(app.game.players[0].hand, hand);
            assert_eq!(app.game.log, log);
            assert_eq!(app.game.format_date(), None);
            assert!(app.begin_native_match(1123, 0).is_err());
            assert_eq!(app.game.players[0].hand, hand);
            assert!(app.native_match.is_none());
            click(app, 720.0, 716.0);
        }
        9 => {
            app.in_match = false;
            app.series_lobby = true;
            app.select_preset(0, 4);
            app.select_preset(1, 4);
        }
        10 => click(app, 425.0, 748.0),
        11 => {
            let error = app.setup_format_check(0, true).unwrap_err();
            assert!(error.contains("Brynhir") && error.contains("battlefield"));
            capture(app, "RIFT_FORMAT_REGISTRATION");
            click(app, 713.0, 261.0);
        }
        12 => click(app, 720.0, 716.0),
        13 => click(app, 1210.0, 803.0),
        14 => {
            let native = app
                .native_match
                .as_ref()
                .expect("historical registration accepted");
            assert_eq!(native.format_date, None);
            let summary = native.state.summary();
            app.prefs.format = FormatChoice::On("2026-11-23".into());
            assert!(app.begin_native_match(1124, 0).is_err());
            assert_eq!(app.native_match.as_ref().unwrap().state.summary(), summary);
            // Rengar's main deck remains legal with a different selected field.
            // The historical registration above retains its banned sideboard.
            app.native_match = None;
            app.in_match = false;
            app.series_lobby = false;
            app.fields = [0, 0];
            for seat in 0..2 {
                let mut deck = CustomDeck::from_preset(4).unwrap();
                deck.battlefield = 0;
                app.custom_decks[seat] = Some(deck);
                app.prefs.use_custom[seat] = true;
                assert!(app.setup_format_check(seat, false).is_ok());
            }
        }
        15 => click(app, 1320.0, 46.0),
        16 => {
            assert!(app.in_match);
            assert!(app.toast.is_empty());
            assert_eq!(app.game.format_date(), Some("2026-11-23"));
            capture(app, "RIFT_FORMAT_DATED_GAME");
            app.game.spell_ability_damage_prevented = true;
        }
        17 => {
            // Capture this state on a new frame, as normal play renders it.
            // A second draw/readback in frame 16 clipped the heading in PNGs.
            capture(app, "RIFT_FORMAT_TURN_EFFECT");
            app.game.spell_ability_damage_prevented = false;
            app.prefs.format = FormatChoice::Today;
            app.reset();
            assert_eq!(
                app.game.format_date(),
                Some(chrono::Local::now().date_naive().to_string().as_str())
            );
        }
        18 => {
            capture(app, "RIFT_FORMAT_TODAY");
            println!(
                "Format UI: dated and historical starts, both seats, full registrations and rejected-reset preservation passed."
            );
            return true;
        }
        _ => {}
    }
    false
}
