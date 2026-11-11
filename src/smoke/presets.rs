//! Preset pagination, both-seat selection and an editable exact copy.
use super::*;

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    println!("Preset UI stage {frame}");
    match frame {
        0 => {
            app.screen = Screen::Decks;
            app.in_match = false;
            app.browse_deck = 0;
            app.deck_scroll = 0;
            app.ui.input_override = None;
            assert_eq!(app.game.presets.len(), 5);
        }
        1 => {
            capture(app, "RIFT_SMOKE_PRESET_FIRST_PAGE_SCREENSHOT");
            click(app, 1375.0, 90.0);
        }
        2 => {
            assert_eq!(app.browse_deck, 4);
            assert_eq!(app.game.presets[4].legend_id, 8);
            capture(app, "RIFT_SMOKE_PRESET_RENGAR_SCREENSHOT");
            click(app, 1375.0, 90.0); // Next is disabled on the final page.
        }
        3 => {
            assert_eq!(app.browse_deck, 4);
            click(app, 1300.0, 90.0);
        }
        4 => {
            assert_eq!(app.browse_deck, 0);
            click(app, 1375.0, 90.0);
        }
        5 => {
            assert_eq!(app.browse_deck, 4);
            click(app, 340.0, 748.0);
        }
        6 => {
            assert_eq!(app.player_deck, 4);
            assert_eq!(app.fields[0], 1);
            assert!(!app.prefs.use_custom[0]);
            click(app, 500.0, 748.0);
        }
        7 => {
            assert_eq!(app.opponent_deck, 4);
            assert_eq!(app.fields[1], 1);
            assert!(!app.prefs.use_custom[1]);
            click(app, 420.0, 800.0);
        }
        8 => {
            assert!(matches!(app.screen, Screen::Practice));
            assert_eq!(app.setup_legend(0).id, 8);
            assert_eq!(app.setup_legend(1).id, 8);
            capture(app, "RIFT_SMOKE_PRESET_MATCHUP_SCREENSHOT");
            click(app, 520.0, 813.0);
        }
        9 => {
            assert!(matches!(app.screen, Screen::Decks));
            assert_eq!(app.browse_deck, 4);
            click(app, 420.0, 657.0);
        }
        10 => {
            assert!(matches!(app.screen, Screen::DeckEditor));
            let editor = app.deck_editor.as_ref().unwrap();
            assert_eq!(editor.deck.legend, 8);
            assert_eq!(editor.deck.cards, app.game.presets[4].cards);
            assert_eq!(editor.deck.chosen_champion, 30);
            assert_eq!(editor.deck.battlefield, 1);
            capture(app, "RIFT_SMOKE_PRESET_EDITOR_SCREENSHOT");
            click(app, 395.0, 815.0);
        }
        11 => {
            assert!(matches!(app.screen, Screen::Decks));
            app.in_match = true;
            app.player_deck = 0;
            app.opponent_deck = 1;
            click(app, 340.0, 748.0);
        }
        12 => {
            assert_eq!(app.player_deck, 0, "in-match selection must stay disabled");
            click(app, 500.0, 748.0);
        }
        13 => {
            assert_eq!(
                app.opponent_deck, 1,
                "in-match selection must stay disabled"
            );
            app.in_match = false;
            click(app, 340.0, 748.0);
        }
        14 => {
            assert_eq!(app.player_deck, 4);
            click(app, 500.0, 748.0);
        }
        15 => {
            assert_eq!(app.opponent_deck, 4);
            click(app, 420.0, 800.0);
        }
        16 => {
            assert!(matches!(app.screen, Screen::Practice));
            app.starting_player = 0;
            click(app, 1330.0, 46.0);
        }
        17 => {
            assert!(app.in_match && app.game.phase == Phase::Mulligan);
            assert_eq!(app.game.battlefields, [1, 1]);
            for seat in 0..2 {
                assert_eq!(app.game.legend_id(seat), 8);
                assert_eq!(app.game.players[seat].champion, Some(30));
                assert_eq!(app.game.players[seat].hand.len(), 4);
                assert!(!app.prefs.use_custom[seat]);
            }
            app.game.validate_numeric_state().unwrap();
            capture(app, "RIFT_SMOKE_PRESET_MULLIGAN_SCREENSHOT");
            click(app, 1283.0, 514.0);
        }
        18 => {
            assert_eq!(app.game.phase, Phase::Playing);
            assert_eq!(app.game.active, 0);
            assert_eq!(app.game.players[0].hand.len(), 5);
            assert_eq!(app.game.players[0].rune_objects().len(), 2);
            assert_eq!(app.game.battlefields, [1, 1]);
            app.game.validate_numeric_state().unwrap();
            capture(app, "RIFT_SMOKE_PRESET_FIRST_TURN_SCREENSHOT");
            app.ui.input_override = None;
            println!(
                "PASS: 19 preset UI stages; pagination, both seats, editable exact copy, mulligan and first turn."
            );
            return true;
        }
        _ => panic!("preset smoke exceeded 19 stages"),
    }
    false
}
