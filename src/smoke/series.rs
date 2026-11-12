//! Native clicks through a full played match, plus explicit concession checks.
use super::*;
use crate::series_ui::SideboardDraft;
use local_rift::match_series::SeriesPhase;
use std::cell::Cell;
thread_local! {
    static STAGE: Cell<u32> = const { Cell::new(0) };
    static GAMES: Cell<u32> = const { Cell::new(0) };
}
fn advance() {
    STAGE.set(STAGE.get() + 1);
}
fn snapshot(app: &mut App, name: &str) {
    capture(app, &format!("RIFT_SERIES_{name}"));
}
fn card_control(app: &mut App, id: usize, pile: usize, choose: bool) -> bool {
    let n = app.native_match.as_ref().unwrap();
    let d = n.draft.as_ref().unwrap();
    let list = if pile == 0 { &d.deck.cards } else { &d.side };
    let index = list.iter().position(|&(card, _)| card == id).unwrap();
    let x = 260.0 + pile as f32 * 577.0;
    let wanted = index / 10;
    let page = n.pages[pile];
    if page != wanted {
        click(app, x + if page < wanted { 495.0 } else { 417.0 }, 676.0);
        return false;
    }
    click(
        app,
        x + if choose { 390.0 } else { 500.0 },
        244.0 + (index % 10) as f32 * 42.0,
    );
    true
}
fn play_to_end(app: &mut App) {
    app.ui.input_override = None;
    app.ai_worker.invalidate();
    for _ in 0..6000 {
        if app.game.winner.is_some() {
            break;
        }
        assert!(app.game.step_ai().unwrap());
        app.native_match
            .as_mut()
            .unwrap()
            .observe_game(&app.game)
            .unwrap();
    }
    assert!(app.game.winner.is_some());
    GAMES.set(GAMES.get() + 1);
    // A winning action can share coordinates with the old standalone result's
    // Play again button. It must not reset the registered match during draw.
    click(app, 530.0, 480.0);
}
pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    assert!(frame < 180, "native match UI did not finish");
    let stage = STAGE.get();
    println!("Series UI frame {frame} stage {stage}");
    app.ui.input_override = None;
    match stage {
        0 => {
            app.screen = Screen::Practice;
            app.in_match = false;
            app.native_match = None;
            app.series_lobby = false;
            app.prefs.use_custom = [false; 2];
            app.prefs.difficulty = match std::env::var("RIFT_SERIES_LEVEL").as_deref() {
                Ok("easy") => Difficulty::Easy,
                Ok("strategic") => Difficulty::Strategic,
                _ => Difficulty::Tactical,
            };
            app.player_deck = 0;
            app.opponent_deck = 1;
            click(app, 1110.0, 46.0);
            advance();
        }
        1 => {
            assert!(app.series_lobby);
            snapshot(app, "LOBBY_INCOMPLETE");
            click(app, 520.0, 477.0);
            advance();
        }
        2 => {
            assert_eq!(app.player_deck, 5);
            click(app, 1100.0, 477.0);
            advance();
        }
        3 => {
            assert_eq!(app.opponent_deck, 5);
            app.series_chooser = 0;
            snapshot(app, "LOBBY");
            click(app, 1200.0, 800.0);
            advance();
        }
        4 => {
            assert!(app.in_match && app.native_match.is_some());
            app.begin_native_match(920001, 0).unwrap();
            let view = app.native_match.as_ref().unwrap().state.observe(0).unwrap();
            assert!(view["players"][1]["selected_battlefield"].is_null());
            snapshot(app, "FIELDS");
            click(app, 350.0, 598.0);
            advance();
        }
        5 => {
            assert_eq!(app.field_inspect, Some(13));
            snapshot(app, "FIELD_RULES");
            click(app, 690.0, 502.0);
            advance();
        }
        6 => {
            assert!(app.field_inspect.is_none());
            click(app, 525.0, 598.0);
            advance();
        }
        7 => {
            assert_eq!(app.native_match.as_ref().unwrap().field_choice, Some(13));
            click(app, 1200.0, 814.0);
            advance();
        }
        8 => {
            assert_eq!(
                app.native_match.as_ref().unwrap().state.phase(),
                SeriesPhase::PlayOrder
            );
            snapshot(app, "REVEAL");
            click(app, 1100.0, 752.0);
            advance();
        }
        9 => {
            assert_eq!(app.game.phase, Phase::Mulligan);
            assert_eq!(app.game.first_player, 1);
            assert_eq!(app.game.mulligans_used, [false, true]);
            snapshot(app, "MULLIGAN");
            click(app, 315.0, 777.0);
            advance();
        }
        10 => {
            assert!(app.mulligan.contains(&0));
            click(app, 1283.0, 514.0);
            advance();
        }
        11 => {
            assert_eq!(app.game.phase, Phase::Playing);
            assert_eq!(app.game.mulligans_used, [true; 2]);
            snapshot(app, "BOARD");
            play_to_end(app);
            advance();
        }
        12 => {
            assert!(app.native_match.as_ref().unwrap().result_pending);
            snapshot(app, "RESULT");
            click(app, 1200.0, 817.0);
            advance();
        }
        13 => {
            assert_eq!(
                app.native_match.as_ref().unwrap().state.phase(),
                SeriesPhase::Sideboarding
            );
            snapshot(app, "SIDEBOARD");
            click(app, 151.0, 600.0);
            STAGE.set(49);
        }
        14 => {
            assert!(!app.series_coach_tip().unwrap().starts_with("Consider "));
            assert_eq!(draft(app).deck.card_count(), 39);
            snapshot(app, "INVALID");
            click(app, 1200.0, 817.0);
            advance();
        }
        15 => {
            assert_eq!(
                app.native_match.as_ref().unwrap().state.phase(),
                SeriesPhase::Sideboarding
            );
            assert_eq!(draft(app).deck.card_count(), 39);
            click(app, 390.0, 815.0);
            advance();
        }
        16 => {
            assert_eq!(draft(app).deck.card_count(), 40);
            if card_control(app, 432, 0, false) {
                advance();
            }
        }
        17 => {
            assert_eq!(draft(app).deck.card_count(), 39);
            if card_control(app, 424, 1, false) {
                advance();
            }
        }
        18 => {
            assert_eq!(draft(app).deck.card_count(), 40);
            if card_control(app, 0, 0, false) {
                advance();
            }
        }
        19 => {
            if card_control(app, 0, 1, false) {
                advance();
            }
        }
        20 => {
            assert_eq!(draft(app).deck.card_count(), 40);
            assert_eq!(draft(app).deck.chosen_champion, 0);
            if card_control(app, 0, 0, true) {
                advance();
            }
        }
        21 => {
            assert_eq!(draft(app).deck.chosen_champion, 0);
            snapshot(app, "CHAMPION");
            let n = app.native_match.as_ref().unwrap();
            let i = draft(app)
                .deck
                .cards
                .iter()
                .position(|&(id, _)| id == 0)
                .unwrap()
                % 10;
            assert_eq!(
                n.pages[0],
                draft(app)
                    .deck
                    .cards
                    .iter()
                    .position(|&(id, _)| id == 0)
                    .unwrap()
                    / 10
            );
            click(app, 390.0, 244.0 + i as f32 * 42.0);
            advance();
        }
        22 => {
            assert_eq!(app.inspect, Some(0));
            snapshot(app, "CARD_RULES");
            click(app, 940.0, 700.0);
            advance();
        }
        23 => {
            assert!(app.inspect.is_none());
            click(app, 1200.0, 817.0);
            advance();
        }
        24 => {
            assert_eq!(
                app.native_match.as_ref().unwrap().state.phase(),
                SeriesPhase::Battlefields
            );
            snapshot(app, "USED_FIELD");
            click(app, 525.0, 598.0);
            advance();
        }
        25 => {
            assert!(app.native_match.as_ref().unwrap().field_choice.is_none());
            click(app, 907.0, 598.0);
            advance();
        }
        26 => {
            assert_eq!(app.native_match.as_ref().unwrap().field_choice, Some(0));
            click(app, 1200.0, 814.0);
            advance();
        }
        27 => {
            assert_eq!(
                app.native_match.as_ref().unwrap().state.phase(),
                SeriesPhase::PlayOrder
            );
            let v = app.native_match.as_ref().unwrap().state.observe(0).unwrap();
            assert_eq!(v["players"][0]["chosen_champion"], 0);
            snapshot(app, "SECOND_REVEAL");
            click(
                app,
                if v["designated_player"] == 0 {
                    530.0
                } else {
                    1100.0
                },
                752.0,
            );
            advance();
        }
        28 => {
            assert_eq!(app.game.phase, Phase::Mulligan);
            assert_eq!(app.game.players[0].champion, Some(0));
            assert!(app.selection.is_none() && app.targets.is_empty());
            click(app, 1283.0, 514.0);
            advance();
        }
        29 => {
            assert_eq!(app.game.phase, Phase::Playing);
            play_to_end(app);
            advance();
        }
        30 => {
            let n = app.native_match.as_ref().unwrap();
            assert!(n.result_pending);
            if n.state.phase() == SeriesPhase::Complete {
                STAGE.set(37);
            } else {
                click(app, 1200.0, 817.0);
                advance();
            }
        }
        31 => {
            assert_eq!(
                app.native_match.as_ref().unwrap().state.phase(),
                SeriesPhase::Sideboarding
            );
            click(app, 1200.0, 817.0);
            advance();
        }
        32 => {
            assert_eq!(
                app.native_match.as_ref().unwrap().state.phase(),
                SeriesPhase::Battlefields
            );
            click(app, 1289.0, 598.0);
            advance();
        }
        33 => {
            assert_eq!(app.native_match.as_ref().unwrap().field_choice, Some(14));
            click(app, 1200.0, 814.0);
            advance();
        }
        34 => {
            let v = app.native_match.as_ref().unwrap().state.observe(0).unwrap();
            click(
                app,
                if v["designated_player"] == 0 {
                    530.0
                } else {
                    1100.0
                },
                752.0,
            );
            advance();
        }
        35 => {
            assert_eq!(app.game.phase, Phase::Mulligan);
            click(app, 1283.0, 514.0);
            advance();
        }
        36 => {
            play_to_end(app);
            advance();
        }
        37 => {
            let v = app.native_match.as_ref().unwrap().state.summary();
            assert_eq!(v["complete"], true);
            assert_eq!(v["history"].as_array().unwrap().len(), GAMES.get() as usize);
            println!(
                "Native UI completed {} actual games to a match winner.",
                GAMES.get()
            );
            snapshot(app, "COMPLETE");
            click(app, 1200.0, 817.0);
            advance();
        }
        38 => {
            assert!(app.series_lobby && app.native_match.is_none() && !app.in_match);
            click(app, 100.0, 220.0);
            STAGE.set(50);
        }
        39 => {
            let v = app.native_match.as_ref().unwrap().state.observe(0).unwrap();
            assert_eq!(v["deck"]["chosen_champion"], 0);
            assert_eq!(v["available_battlefields"].as_array().unwrap().len(), 3);
            click(app, 525.0, 598.0);
            advance();
        }
        40 => {
            click(app, 1200.0, 814.0);
            advance();
        }
        41 => {
            let v = app.native_match.as_ref().unwrap().state.observe(0).unwrap();
            click(
                app,
                if v["designated_player"] == 0 {
                    530.0
                } else {
                    1100.0
                },
                752.0,
            );
            advance();
        }
        42 => {
            assert_eq!(app.game.phase, Phase::Mulligan);
            click(app, 920.0, 50.0);
            advance();
        }
        43 => {
            assert!(app.series_concede);
            snapshot(app, "CONCEDE");
            click(app, 610.0, 506.0);
            advance();
        }
        44 => {
            assert!(!app.series_concede);
            assert_eq!(
                app.native_match.as_ref().unwrap().state.summary()["wins"],
                serde_json::json!([0, 0])
            );
            click(app, 920.0, 50.0);
            advance();
        }
        45 => {
            click(app, 870.0, 506.0);
            advance();
        }
        46 => {
            assert!(app.native_match.as_ref().unwrap().result_pending);
            assert_eq!(
                app.native_match.as_ref().unwrap().state.summary()["wins"],
                serde_json::json!([0, 1])
            );
            click(app, 1320.0, 46.0);
            advance();
        }
        47 => {
            assert!(app.confirm_restart);
            click(app, 850.0, 500.0);
            advance();
        }
        48 => {
            assert!(!app.in_match && app.native_match.is_none() && !app.series_lobby);
            return true;
        }
        49 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            snapshot(app, "SIDEBOARD_ADVANCED");
            app.prefs.coach = false;
            click(app, 775.0, 815.0);
            STAGE.set(54);
        }
        54 => {
            assert!(app.prefs.coach, "an explicit review wakes Mote");
            let tip = app.series_coach_tip().unwrap();
            assert!(tip.starts_with("Consider ") || tip.starts_with("No clear swap"));
            assert_eq!(draft(app).deck.card_count(), 40);
            snapshot(app, "SIDEBOARD_HINT");
            if card_control(app, 0, 0, false) {
                STAGE.set(14);
            }
        }
        50 => {
            assert!(app.screen == Screen::Decks);
            click(app, 420.0, 657.0);
            advance();
        }
        51 => {
            assert!(app.screen == Screen::DeckEditor);
            assert_eq!(app.deck_editor.as_ref().unwrap().scroll, 0);
            app.hover_card = None;
            app.ui.input_override = Some((vec2(1390.0, 740.0), false, false));
            app.ui.wheel_override = Some(-1.0);
            advance();
        }
        52 => {
            assert_eq!(
                app.deck_editor.as_ref().unwrap().scroll,
                1,
                "match lobby must not pause the deck editor's input handling"
            );
            app.ui.wheel_override = None;
            snapshot(app, "LOBBY_EDITOR");
            click(app, 100.0, 170.0);
            advance();
        }
        53 => {
            assert!(app.screen == Screen::Practice && app.series_lobby);
            click(app, 1200.0, 800.0);
            STAGE.set(39);
        }
        _ => unreachable!(),
    }
    false
}
fn draft(app: &App) -> &SideboardDraft {
    app.native_match.as_ref().unwrap().draft.as_ref().unwrap()
}
