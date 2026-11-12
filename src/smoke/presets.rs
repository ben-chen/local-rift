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
            assert_eq!(app.game.presets.len(), 7);
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
        }
        19 => {
            app.in_match = false;
            app.screen = Screen::Decks;
            app.browse_deck = 4;
            app.deck_scroll = 0;
            click(app, 650.0, 250.0);
        }
        20 => {
            assert_eq!(app.browse_deck, 5);
            assert_eq!(app.game.presets[5].legend_id, 0);
            capture(app, "RIFT_SMOKE_ARITO_MAIN");
            click(app, 875.0, 440.0);
        }
        21 => {
            assert!(app.preset_tab == crate::preset_ui::PresetTab::Sideboard);
            capture(app, "RIFT_SMOKE_ARITO_SIDEBOARD");
            click(app, 800.0, 480.0);
        }
        22 => {
            assert_eq!(app.inspect, Some(424));
            capture(app, "RIFT_SMOKE_ARITO_SIDE_INSPECT");
            click(app, 940.0, 700.0);
        }
        23 => {
            assert!(app.inspect.is_none());
            click(app, 1080.0, 440.0);
        }
        24 => {
            assert!(app.preset_tab == crate::preset_ui::PresetTab::Battlefields);
            capture(app, "RIFT_SMOKE_ARITO_FIELDS");
            click(app, 1320.0, 485.0);
        }
        25 => {
            assert_eq!(app.field_inspect, Some(13));
            capture(app, "RIFT_SMOKE_ARITO_FIELD_INSPECT");
            click(app, 690.0, 502.0);
        }
        26 => {
            assert!(app.field_inspect.is_none());
            click(app, 340.0, 748.0);
        }
        27 => {
            assert_eq!(app.player_deck, 5);
            assert_eq!(app.fields[0], 13);
            click(app, 500.0, 748.0);
        }
        28 => {
            assert_eq!(app.opponent_deck, 5);
            assert_eq!(app.fields[1], 13);
            click(app, 1320.0, 632.0);
        }
        29 => {
            assert_eq!(app.fields[0], 0);
            click(app, 1320.0, 777.0);
        }
        30 => {
            assert_eq!(app.fields[1], 14);
            capture(app, "RIFT_SMOKE_ARITO_CHOSEN_FIELDS");
            click(app, 420.0, 800.0);
        }
        31 => {
            assert!(matches!(app.screen, Screen::Practice));
            assert_eq!(app.setup_legend(0).id, 0);
            assert_eq!(app.setup_legend(1).id, 0);
            capture(app, "RIFT_SMOKE_ARITO_MATCHUP");
            app.starting_player = 0;
            click(app, 1330.0, 46.0);
        }
        32 => {
            assert!(app.in_match && app.game.phase == Phase::Mulligan);
            assert_eq!([app.player_deck, app.opponent_deck], [5, 5]);
            assert_eq!(app.game.battlefields, [0, 14]);
            for seat in 0..2 {
                assert_eq!(app.game.legend_id(seat), 0);
                assert_eq!(app.game.players[seat].champion, Some(0));
                assert_eq!(app.game.players[seat].hand.len(), 4);
                assert_eq!(
                    app.game.players[seat]
                        .hand
                        .iter()
                        .chain(&app.game.players[seat].deck)
                        .filter(|&&c| c == 432)
                        .count(),
                    2
                );
            }
            capture(app, "RIFT_SMOKE_ARITO_MULLIGAN");
            click(app, 1283.0, 514.0);
        }
        33 => {
            assert_eq!(app.game.phase, Phase::Playing);
            assert_eq!(app.game.active, 0);
            assert_eq!(app.game.battlefields, [0, 14]);
            capture(app, "RIFT_SMOKE_ARITO_FIRST_TURN");
            app.in_match = false;
            app.screen = Screen::Decks;
            app.browse_deck = 5;
            app.preset_tab = crate::preset_ui::PresetTab::Main;
            click(app, 420.0, 657.0);
        }
        34 => {
            let editor = app.deck_editor.as_ref().unwrap();
            assert_eq!(editor.deck.cards, app.game.presets[5].cards);
            assert_eq!(editor.deck.legend, 0);
            assert_eq!(editor.deck.battlefield, 13);
            capture(app, "RIFT_SMOKE_ARITO_EDITOR");
            app.screen = Screen::Decks;
            app.browse_deck = 5;
            app.preset_tab = crate::preset_ui::PresetTab::Main;
            click(app, 970.0, 250.0);
        }
        35 => {
            assert_eq!(app.browse_deck, 6);
            assert_eq!(app.game.presets[6].legend_id, 10);
            capture(app, "RIFT_SMOKE_SONECON_MAIN");
            click(app, 340.0, 748.0);
        }
        36 => {
            assert_eq!(app.player_deck, 6);
            assert_eq!(app.fields[0], 11);
            click(app, 500.0, 748.0);
        }
        37 => {
            assert_eq!(app.opponent_deck, 6);
            assert_eq!(app.fields[1], 11);
            click(app, 875.0, 440.0);
        }
        38 => {
            assert!(app.preset_tab == crate::preset_ui::PresetTab::Sideboard);
            capture(app, "RIFT_SMOKE_SONECON_SIDEBOARD");
            click(app, 1080.0, 440.0);
        }
        39 => {
            assert!(app.preset_tab == crate::preset_ui::PresetTab::Battlefields);
            capture(app, "RIFT_SMOKE_SONECON_FIELDS");
            click(app, 420.0, 800.0);
        }
        40 => {
            assert!(matches!(app.screen, Screen::Practice));
            assert_eq!(app.setup_legend(0).id, 10);
            assert_eq!(app.setup_legend(1).id, 10);
            capture(app, "RIFT_SMOKE_SONECON_MATCHUP");
            app.starting_player = 0;
            click(app, 1330.0, 46.0);
        }
        41 => {
            assert!(app.in_match && app.game.phase == Phase::Mulligan);
            assert_eq!(app.game.battlefields, [11, 11]);
            for player in &app.game.players {
                assert_eq!(player.champion, Some(439));
                assert_eq!(player.hand.len() + player.deck.len(), 39);
                assert_eq!(
                    player
                        .hand
                        .iter()
                        .chain(&player.deck)
                        .filter(|&&id| id == 440)
                        .count(),
                    1
                );
                assert!(!player.hand.iter().chain(&player.deck).any(|&id| id == 435));
            }
            capture(app, "RIFT_SMOKE_SONECON_MULLIGAN");
            click(app, 1283.0, 514.0);
        }
        42 => {
            assert_eq!(app.game.phase, Phase::Playing);
            assert_eq!(app.game.active, 0);
            capture(app, "RIFT_SMOKE_SONECON_FIRST_TURN");
            app.in_match = false;
            app.screen = Screen::Practice;
            click(app, 1110.0, 46.0);
        }
        43 => {
            assert!(app.series_lobby);
            capture(app, "RIFT_SMOKE_SONECON_REGISTRATION");
            app.series_chooser = 0;
            click(app, 1200.0, 800.0);
        }
        44 => {
            assert!(app.in_match && app.native_match.is_some());
            let view = app.native_match.as_ref().unwrap().state.observe(0).unwrap();
            assert_eq!(
                view["registration"]["battlefields"],
                serde_json::json!([11, 7, 12])
            );
            assert_eq!(view["deck"]["legend"], 10);
            capture(app, "RIFT_SMOKE_SONECON_REGISTERED_FIELDS");
            println!(
                "PASS: 45 preset UI stages; Irelia, Rengar and Sonecon Riven, exact lists, sideboards, fields, mulligans, first turns and match registration."
            );
            return true;
        }
        _ => panic!("preset smoke exceeded 45 stages"),
    }
    false
}
