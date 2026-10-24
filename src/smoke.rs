//! Deterministic native UI checks. Invoke before `App::draw`, starting at frame 0.
//!
//! This deliberately clicks the actual controls. A small public-state fixture
//! supplies two ready units so the test does not depend on a particular AI plan.
use super::{AdviceLevel, App, Difficulty, Game, Phase, SceneState, SceneUnit, Screen, Selection};
use local_rift::{
    cards::{CardKind, Domain},
    game::{Action, Decision, Target, Unit, domain_index},
    gear::GearState,
};
use macroquad::prelude::*;

fn click(app: &mut App, x: f32, y: f32) {
    app.ui.input_override = Some((vec2(x, y), true, false));
}

fn hand_card(app: &mut App, name: &str) {
    let id = app
        .game
        .catalog
        .iter()
        .position(|card| card.name == name)
        .unwrap();
    let player = &mut app.game.players[0];
    if let Some(index) = player.hand.iter().position(|card| *card == id) {
        player.hand.swap(0, index);
    } else {
        let index = player.deck.iter().position(|card| *card == id).unwrap();
        std::mem::swap(&mut player.hand[0], &mut player.deck[index]);
    }
}

fn fixture_unit(app: &mut App, owner: usize, id: u32, card_id: usize) {
    let state = &mut app.game.players[owner];
    if let Some(index) = state.deck.iter().position(|card| *card == card_id) {
        state.deck.remove(index);
    } else {
        let index = state.hand.iter().position(|card| *card == card_id).unwrap();
        state.hand.remove(index);
    }
    state.units.push(Unit {
        id,
        card_id,
        lane: None,
        ready: false,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    });
}

fn fixture_resources(app: &mut App, domain: Domain) {
    app.game.players[0].runes = [0; 6];
    app.game.players[0].runes[domain_index(domain)] = 6;
    app.game.players[0].exhausted_runes = [0; 6];
    app.game.players[0].energy = 6;
    app.game.players[0].energy_pool = 0;
    app.game.players[0].rune_deck.truncate(6);
}

fn click_unit(app: &mut App, selected: u32) {
    let units = app
        .game
        .players
        .iter()
        .enumerate()
        .flat_map(|(owner, player)| player.units.iter().map(move |unit| (owner, unit)))
        .map(|(owner, unit)| SceneUnit {
            id: unit.id,
            owner,
            location: unit.lane.map_or(0, |lane| lane + 1),
            ready: unit.ready,
            might: app.game.unit_might(unit),
            accent: app.game.presets[if owner == 0 {
                app.player_deck
            } else {
                app.opponent_deck
            }]
            .accent,
        })
        .collect();
    let state = SceneState {
        units,
        selected: None,
        owners: app.game.battlefield_owner,
        time: 0.0,
        orbit: app.orbit,
    };
    let hits = super::scene::draw_scene(
        app.ui.physical(Rect::new(232.0, 105.0, 910.0, 536.0)),
        &state,
    );
    let hit = hits.units.iter().find(|(id, _)| *id == selected).unwrap().1;
    let center = (vec2(hit.x + hit.w / 2.0, hit.y + hit.h / 2.0) - app.ui.offset) / app.ui.scale;
    click(app, center.x, center.y);
}

/// Returns true after all assertions pass. Use an isolated XDG_DATA_HOME because
/// the settings controls intentionally exercise the normal persistence path.
pub fn run_frame(app: &mut App, frame: u32) -> bool {
    if std::env::var_os("RIFT_SMOKE_TRACE").is_some() {
        eprintln!("UI frame {frame}");
    }
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => {
            app.screen = Screen::Practice;
            app.in_match = false;
            app.inspect = None;
            app.lesson = None;
            app.confirm_restart = false;
            app.prefs.coach = true;
            app.prefs.advanced = false;
            app.prefs.tactical = false;
            app.companion.set_level(AdviceLevel::Beginner);
            app.ui.resize();
            app.starting_player = 0;
            click(app, 1285.0, 548.0);
        }
        1 => {
            assert_eq!(
                app.starting_player, 1,
                "setup cycles starting-player choice"
            );
            app.starting_player = 0;
            click(app, 100.0, 219.0);
        }
        2 => {
            assert!(app.screen == Screen::Decks, "deck navigation");
            click(app, 677.0, 240.0);
        }
        3 => {
            assert_eq!(app.browse_deck, 1);
            click(app, 340.0, 748.0);
        }
        4 => {
            assert_eq!(app.player_deck, 1);
            click(app, 974.0, 240.0);
        }
        5 => {
            assert_eq!(app.browse_deck, 2);
            click(app, 507.0, 748.0);
        }
        6 => {
            assert_eq!(app.opponent_deck, 2);
            click(app, 425.0, 802.0);
        }
        7 => {
            assert!(app.screen == Screen::Practice && !app.in_match);
            click(app, 1330.0, 46.0);
        }
        8 => {
            assert!(
                app.in_match && app.game.phase == Phase::Mulligan,
                "start opens mulligan"
            );
            app.game = Game::new(1, 2, 42, Difficulty::Easy);
            app.mulligan.clear();
            click(app, 315.0, 760.0);
        }
        9 => {
            assert!(app.mulligan.contains(&0), "first mulligan selection");
            click(app, 465.0, 760.0);
        }
        10 => {
            assert_eq!(app.mulligan.len(), 2);
            click(app, 1283.0, 514.0);
        }
        11 => {
            assert!(
                app.game.phase == Phase::Playing && app.game.active == 0,
                "mulligan starts match"
            );
            assert_eq!(app.game.players[0].hand.len(), 5);
            hand_card(app, "Legion Marauder");
            click(app, 315.0, 760.0);
        }
        12 => {
            assert!(app.selection == Some(Selection::Hand(0)), "hand selection");
            click(app, 1285.0, 510.0);
        }
        13 => {
            assert_eq!(app.game.players[0].units.len(), 1, "deploy button");
            let unit = &app.game.players[0].units[0];
            assert!(
                !unit.ready && unit.lane.is_none(),
                "unit enters exhausted at base"
            );
            click(app, 1285.0, 595.0);
        }
        14 => {
            assert_eq!(app.game.active, 1, "end turn button");
            app.game.run_ai_turn().expect("AI completes fixture turn");
            assert_eq!(app.game.active, 0);
            let mut second = app.game.players[0].units[0].clone();
            let index = app.game.players[0]
                .deck
                .iter()
                .position(|id| {
                    app.game.catalog[*id].kind == CardKind::Unit && !matches!(*id, 24 | 45)
                })
                .unwrap();
            second.card_id = app.game.players[0].deck.remove(index);
            second.id = 40_001;
            second.ready = true;
            app.game.players[0].units.push(second);
            for player in &mut app.game.players {
                for unit in &mut player.units {
                    unit.lane = None;
                    unit.ready = true;
                    unit.damage = 0;
                }
            }
            app.game.battlefield_owner = [None; 2];
            app.selection = None;
            app.group.clear();
            click_unit(app, 40_001);
        }
        15 => {
            assert!(
                matches!(app.selection, Some(Selection::Unit(_))),
                "3D unit selection"
            );
            click(app, 1285.0, 546.0);
        }
        16 => {
            assert!(app.group.len() >= 2, "group ready units button");
            click(app, 1285.0, 420.0);
        }
        17 => {
            assert!(app.game.lane_count(0, 0) >= 2, "group moved together");
            assert!(app.game.showdown.is_some(), "empty field opens showdown");
            click(app, 1285.0, 595.0);
        }
        18 => {
            assert_eq!(app.game.priority(), 1, "pass button transfers priority");
            app.game
                .apply(Action::Pass)
                .expect("opponent passes fixture showdown");
            assert_eq!(app.game.battlefield_owner[0], Some(0));
            assert!(
                app.game.players[0].score > 0,
                "conquest scores after showdown"
            );
            app.ui.input_override = Some((vec2(315.0, 760.0), false, true));
        }
        19 => {
            assert!(app.inspect.is_some(), "right-click inspector");
            click(app, 960.0, 704.0);
        }
        20 => {
            assert!(app.inspect.is_none(), "inspector close");
            click(app, 151.0, 599.0);
        }
        21 => {
            assert!(app.companion.level() == AdviceLevel::Advanced);
            click(app, 63.0, 599.0);
        }
        22 => {
            assert!(app.companion.level() == AdviceLevel::Beginner);
            click(app, 100.0, 323.0);
        }
        23 => {
            assert!(app.screen == Screen::Settings);
            click(app, 1320.0, 171.0);
        }
        24 => {
            assert!(!app.prefs.coach, "coach toggle off");
            click(app, 1320.0, 171.0);
        }
        25 => {
            assert!(app.prefs.coach, "coach toggle on");
            click(app, 100.0, 167.0);
        }
        26 => {
            assert!(app.screen == Screen::Practice);
            click(app, 1330.0, 46.0);
        }
        27 => {
            assert!(app.confirm_restart, "new match confirmation");
            click(app, 854.0, 501.0);
        }
        28 => {
            assert!(
                !app.in_match && !app.confirm_restart,
                "choose matchup clears match"
            );
            click(app, 100.0, 219.0);
        }
        29 => {
            assert!(app.screen == Screen::Decks);
            click(app, 382.0, 240.0);
        }
        30 => {
            assert_eq!(app.browse_deck, 0);
            click(app, 340.0, 748.0);
        }
        31 => {
            assert_eq!(app.player_deck, 0, "deck selection works after a match");
            app.screen = Screen::Practice;
            app.in_match = true;
            app.player_deck = 3;
            app.opponent_deck = 1;
            app.game = Game::new(3, 1, 73, Difficulty::Easy);
            app.game.start_match().unwrap();
            app.selection = None;
            app.targets.clear();
            app.choices.clear();
            app.tray = super::Tray::Hand;
            app.hand_scroll = 0;
            // A fixture gear exercises the native resource-ability control.
            app.game.players[0].deck.pop();
            app.game.players[0].gear.push(GearState {
                id: 50_001,
                card_id: 42,
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            });
        }
        32 => click(app, 400.0, 663.0),
        33 => {
            assert!(app.tray == super::Tray::Loadout);
            click(app, 315.0, 760.0);
        }
        34 => {
            assert!(app.selection == Some(Selection::Legend));
            click(app, 1285.0, 548.0);
        }
        35 => {
            assert!(
                !app.game.legend[0].ready && app.game.legend[0].spell_power == 1,
                "native legend activation"
            );
            assert!(
                app.game.chain.is_empty(),
                "resource legend preserves open state"
            );
            click(app, 615.0, 760.0);
        }
        36 => {
            assert!(app.selection == Some(Selection::Gear(50_001)));
            click(app, 1285.0, 514.0);
        }
        37 => {
            assert!(!app.game.players[0].gear[0].ready, "native gear activation");
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Chaos)],
                1
            );
            let card = app.game.players[0].hand.pop().unwrap();
            app.game.players[0].discard.push(card);
            click(app, 505.0, 663.0);
        }
        38 => {
            assert!(app.tray == super::Tray::Trash);
            click(app, 315.0, 760.0);
        }
        39 => {
            assert!(
                app.selection == Some(Selection::Trash(0)),
                "trash selection"
            );
            app.ui.input_override = Some((vec2(315.0, 760.0), false, true));
        }
        40 => {
            assert!(app.inspect.is_some(), "trash inspector");
            click(app, 960.0, 704.0);
        }
        41 => {
            assert!(app.inspect.is_none());
            let top = *app.game.players[0].deck.last().unwrap();
            app.deck_scroll = top;
            app.game.decision = Some(Decision::Predict {
                player: 0,
                card_id: top,
            });
            click(app, 1285.0, 506.0);
        }
        42 => {
            assert!(app.game.decision.is_none(), "predict choice resolves");
            assert_eq!(
                app.game.players[0].deck[0], app.deck_scroll,
                "predict recycles selected card"
            );
            app.deck_scroll = app.game.players[0].hand.len();
            app.game.decision = Some(Decision::Discard {
                player: 0,
                count: 1,
                draw_after: 2,
            });
            click(app, 315.0, 760.0);
        }
        43 => {
            assert_eq!(app.choices.len(), 1, "native discard selection");
            click(app, 1285.0, 508.0);
        }
        44 => {
            assert!(app.game.decision.is_none(), "discard confirmation");
            assert_eq!(app.game.players[0].hand.len(), app.deck_scroll + 1);
            let cards = app.game.players[0]
                .deck
                .iter()
                .rev()
                .take(3)
                .copied()
                .collect::<Vec<_>>();
            app.deck_scroll = cards[1];
            app.game.decision = Some(Decision::TopThree { player: 0, cards });
            click(app, 465.0, 760.0);
        }
        45 => {
            assert!(app.game.decision.is_none(), "top-card choice resolves");
            assert_eq!(*app.game.players[0].hand.last().unwrap(), app.deck_scroll);
            let hand = std::mem::take(&mut app.game.players[0].hand);
            app.game.players[0].discard.extend(hand);
            app.game.decision = Some(Decision::Discard {
                player: 0,
                count: 1,
                draw_after: 2,
            });
            click(app, 1285.0, 508.0);
        }
        46 => {
            assert!(
                app.game.decision.is_none(),
                "empty-hand discard must continue"
            );
            assert_eq!(app.game.players[0].hand.len(), 2);
            let hand = std::mem::take(&mut app.game.players[1].hand);
            for id in hand {
                if app.game.catalog[id].kind == CardKind::Unit {
                    app.game.players[1].hand.push(id);
                } else {
                    app.game.players[1].deck.push(id);
                }
            }
            app.game.decision = Some(Decision::Sabotage {
                player: 0,
                opponent: 1,
            });
            click(app, 1285.0, 546.0);
        }
        47 => {
            assert!(
                app.game.decision.is_none(),
                "Sabotage with no non-unit must continue"
            );
            let trash = std::mem::take(&mut app.game.players[0].discard);
            for id in trash {
                if app.game.catalog[id].kind == CardKind::Unit {
                    app.game.players[0].discard.push(id);
                } else {
                    app.game.players[0].deck.push(id);
                }
            }
            app.game.decision = Some(Decision::GrantFlow { player: 0 });
            click(app, 1285.0, 546.0);
        }
        48 => {
            assert!(
                app.game.decision.is_none(),
                "Flow with no spell must continue"
            );
            app.game = Game::new(3, 1, 99, Difficulty::Easy);
            app.game.start_match().unwrap();
            app.game.players[0].score = 7;
            let deck = std::mem::take(&mut app.game.players[1].deck);
            let trash = std::mem::take(&mut app.game.players[1].discard);
            app.game.players[1].banish.extend(deck);
            app.game.players[1].banish.extend(trash);
            app.selection = None;
            app.choices.clear();
            app.tray = super::Tray::Hand;
            app.recorded = false;
            app.deck_scroll = app.prefs.wins as usize;
            click(app, 1285.0, 595.0);
        }
        49 => {
            assert_eq!(app.game.winner, Some(0), "burnout can earn the final point");
            assert_eq!(app.game.phase, Phase::Finished);
            assert_eq!(app.game.players[0].score, 8);
            assert!(app.recorded, "finished match is recorded once");
            assert_eq!(app.prefs.wins as usize, app.deck_scroll + 1);
            click(app, 525.0, 476.0);
        }
        50 => {
            assert_eq!(
                app.game.phase,
                Phase::Mulligan,
                "victory Play again opens a fresh match"
            );
            assert!(app.game.winner.is_none() && !app.recorded);
            app.player_deck = 1;
            app.opponent_deck = 3;
            app.game = Game::new(1, 3, 123, Difficulty::Easy);
            app.game.start_match().unwrap();
            fixture_resources(app, Domain::Body);
            hand_card(app, "First Mate");
            fixture_unit(app, 0, 60_001, 25);
            app.hand_scroll = 0;
            click(app, 315.0, 760.0);
        }
        51 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            click(app, 1285.0, 510.0);
        }
        52 => {
            assert!(
                matches!(
                    app.game.decision,
                    Some(Decision::OnPlayTarget { card_id: 23, .. })
                ),
                "on-play target chosen after entry"
            );
            click_unit(app, 60_001);
        }
        53 => {
            assert!(
                app.game.chain.iter().any(|item| item.card_id == 23
                    && item.targets
                        == vec![Target::Unit {
                            player: 0,
                            unit_id: 60_001
                        }]),
                "chosen on-play target reaches chain"
            );
            click(app, 1285.0, 595.0);
        }
        54 => {
            assert_eq!(app.game.priority(), 1);
            app.game.apply(Action::Pass).unwrap();
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .find(|unit| unit.id == 60_001)
                    .unwrap()
                    .ready,
                "First Mate readies chosen unit after responses"
            );
            app.player_deck = 3;
            app.opponent_deck = 1;
            app.game = Game::new(3, 1, 124, Difficulty::Easy);
            app.game.start_match().unwrap();
            fixture_resources(app, Domain::Mind);
            hand_card(app, "Bellows Breath");
            fixture_unit(app, 1, 60_002, 25);
            fixture_unit(app, 1, 60_003, 25);
            app.selection = None;
            app.targets.clear();
            app.repeat_targets.clear();
            app.editing_repeat = false;
            app.hand_scroll = 0;
            click(app, 315.0, 760.0);
        }
        55 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            click_unit(app, 60_002);
        }
        56 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 60_002
                }]
            );
            click(app, 1230.0, 468.0);
        }
        57 => {
            assert!(app.options.repeat);
            click(app, 1337.0, 468.0);
        }
        58 => {
            assert!(app.editing_repeat);
            click_unit(app, 60_003);
        }
        59 => {
            assert_eq!(
                app.repeat_targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 60_003
                }]
            );
            click(app, 1285.0, 510.0);
        }
        60 => {
            let spell = app.game.chain.last().expect("repeat spell on chain");
            assert_eq!(spell.card_id, 50);
            assert!(spell.options.repeat);
            assert_eq!(
                spell.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 60_002
                }]
            );
            assert_eq!(
                spell.repeat_targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 60_003
                }]
            );
            click(app, 1285.0, 595.0);
        }
        61 => {
            assert_eq!(app.game.priority(), 1);
            app.game.apply(Action::Pass).unwrap();
            assert!(app.game.chain.is_empty());
            assert!(
                app.game.players[1]
                    .units
                    .iter()
                    .all(|unit| unit.damage == 1),
                "separate Repeat targets receive separate packets"
            );
            app.player_deck = 1;
            app.opponent_deck = 1;
            app.game = Game::new(1, 1, 125, Difficulty::Easy);
            app.game.start_match().unwrap();
            fixture_unit(app, 0, 60_004, 25);
            fixture_unit(app, 1, 60_005, 25);
            fixture_unit(app, 1, 60_006, 25);
            app.game.players[0].units[0].ready = true;
            for unit in &mut app.game.players[1].units {
                unit.lane = Some(0);
            }
            app.game.battlefield_owner[0] = Some(1);
            app.selection = None;
            app.targets.clear();
            app.damage_order.clear();
            app.game
                .apply(Action::Move {
                    unit_id: 60_004,
                    lane: Some(0),
                })
                .unwrap();
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            assert!(matches!(
                app.game.decision,
                Some(Decision::CombatDamage {
                    player: 0,
                    total: 2,
                    ..
                })
            ));
            click(app, 465.0, 760.0);
        }
        62 => {
            assert_eq!(
                app.damage_order.first(),
                Some(&60_006),
                "native combat target order"
            );
            click(app, 1285.0, 510.0);
        }
        63 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::CombatDamage { player: 1, .. })
            ));
            assert!(
                app.game
                    .players
                    .iter()
                    .flat_map(|state| &state.units)
                    .all(|unit| unit.damage == 0),
                "damage waits for both assignments"
            );
            app.game.step_ai().unwrap();
            assert!(
                app.game.players[0].units.is_empty(),
                "combat deals captured enemy damage simultaneously"
            );
            assert!(
                app.game.players[1]
                    .units
                    .iter()
                    .any(|unit| unit.id == 60_005)
            );
            assert!(
                !app.game.players[1]
                    .units
                    .iter()
                    .any(|unit| unit.id == 60_006),
                "chosen first target receives lethal damage"
            );
            click(app, 1285.0, 663.0);
        }
        64 => {
            assert!(app.show_journal, "match journal opens");
            assert_eq!(app.journal_page, 0);
            click(app, 550.0, 732.0);
        }
        65 => {
            assert_eq!(app.journal_page, 1, "journal can show older events");
            click(app, 1040.0, 732.0);
        }
        66 => {
            assert!(!app.show_journal, "journal closes back to board");
            app.player_deck = 3;
            app.opponent_deck = 1;
            app.game = Game::new(3, 1, 126, Difficulty::Easy);
            app.game.choose_starting_player(1).unwrap();
            app.mulligan.clear();
            app.selection = None;
            app.targets.clear();
            app.choices.clear();
            app.tray = super::Tray::Hand;
            app.hand_scroll = 0;
            click(app, 1283.0, 514.0);
        }
        67 => {
            assert_eq!(app.game.phase, Phase::Playing);
            assert_eq!(app.game.first_player, 1);
            assert_eq!(app.game.active, 1, "AI opens when human chooses second");
            assert_eq!(app.game.players[1].total_runes(), 2);
            assert_eq!(app.game.players[0].total_runes(), 0);
            assert_eq!(app.game.players[0].hand.len(), 4);
            app.game.apply(Action::EndTurn).unwrap();
            assert_eq!(app.game.active, 0);
            assert_eq!(
                app.game.players[0].total_runes(),
                3,
                "human receives second-player opening rune"
            );
            assert_eq!(app.game.players[0].hand.len(), 5);
        }
        68 => {
            app.game = Game::new(1, 2, 42, Difficulty::Easy);
            app.game.start_match().unwrap();
            app.player_deck = 1;
            app.opponent_deck = 2;
            app.selection = None;
            app.group.clear();
            app.targets.clear();
            app.hover_card = None;
            app.hover_rect = None;
            app.drag = None;
            app.rune_menu = None;
            app.tray = super::Tray::Hand;
            app.hand_scroll = 0;
            hand_card(app, "Legion Marauder");
            fixture_resources(app, Domain::Body);
            click(app, 315.0, 760.0);
            app.ui.pointer_override = Some((true, false));
        }
        69 | 70 => {
            assert!(app.drag.is_some(), "hand press arms a drag");
            let r = app
                .drop_zones
                .iter()
                .find(|(zone, _)| matches!(zone, super::DropZone::Base))
                .unwrap()
                .1;
            app.ui.input_override = Some((vec2(r.x + r.w / 2.0, r.y + r.h / 2.0), false, false));
            app.ui.pointer_override = Some((frame == 69, frame == 70));
        }
        71 => {
            assert_eq!(
                app.game.players[0].units.len(),
                1,
                "drag hand card onto base plays it"
            );
            assert_eq!(app.game.players[0].hand.len(), 4);
            assert_eq!(app.game.players[0].energy, 4);
            click(app, 315.0, 760.0);
            app.ui.pointer_override = Some((true, false));
        }
        72 | 73 => {
            app.ui.input_override = Some((vec2(220.0, 850.0), false, false));
            app.ui.pointer_override = Some((frame == 72, frame == 73));
        }
        74 => {
            assert!(app.drag.is_none(), "invalid drop cancels");
            assert_eq!(
                app.game.players[0].hand.len(),
                4,
                "invalid drop preserves hand"
            );
            assert_eq!(
                app.game.players[0].energy, 4,
                "invalid drop preserves resources"
            );
            app.game.players[0].units[0].ready = true;
            click_unit(app, app.game.players[0].units[0].id);
            app.ui.pointer_override = Some((true, false));
        }
        75 | 76 => {
            let r = app
                .drop_zones
                .iter()
                .find(|(zone, _)| matches!(zone, super::DropZone::Battlefield(0)))
                .unwrap()
                .1;
            app.ui.input_override = Some((vec2(r.x + r.w * 0.8, r.y + r.h * 0.8), false, false));
            app.ui.pointer_override = Some((frame == 75, frame == 76));
        }
        77 => {
            assert_eq!(
                app.game.players[0].units[0].lane,
                Some(0),
                "unit drag moves to battlefield"
            );
            while app.game.in_window() || app.game.decision.is_some() {
                let action = app
                    .game
                    .legal_actions()
                    .into_iter()
                    .find(|a| matches!(a, Action::Pass))
                    .unwrap_or_else(|| app.game.legal_actions().remove(0));
                app.game.apply(action).unwrap();
            }
            let card_id = *app.game.players[0]
                .deck
                .iter()
                .find(|id| app.game.catalog[**id].kind == CardKind::Unit)
                .unwrap();
            fixture_unit(app, 0, 50_002, card_id);
            for unit in &mut app.game.players[0].units {
                unit.lane = None;
                unit.ready = true;
            }
            app.group = app.game.players[0]
                .units
                .iter()
                .map(|unit| unit.id)
                .collect();
            app.selection = Some(Selection::Unit(app.group[0]));
            click_unit(app, app.group[0]);
            app.ui.pointer_override = Some((true, false));
        }
        78 | 79 => {
            let r = app
                .drop_zones
                .iter()
                .find(|(zone, _)| matches!(zone, super::DropZone::Battlefield(1)))
                .unwrap()
                .1;
            app.ui.input_override = Some((vec2(r.x + r.w * 0.8, r.y + r.h * 0.8), false, false));
            app.ui.pointer_override = Some((frame == 78, frame == 79));
        }
        80 => {
            assert_eq!(
                app.game.lane_count(0, 1),
                2,
                "drag preserves selected group"
            );
            app.selection = None;
            app.hover_card = app.game.players[0].hand.first().copied();
            app.hover_since = -1.0;
            app.ui.input_override = Some((vec2(315.0, 760.0), false, false));
        }
        81 => {
            assert!(app.hover_rect.is_some(), "hover exposes full-text preview");
            app.inspect = Some(13);
            app.hover_card = None;
            app.hover_rect = None;
        }
        82 => {
            assert_eq!(app.inspect_last, Some(13), "full inspector opens");
            let text = app.game.catalog[13].official_text;
            let lines = app.ui.text_lines(text, 382.0, 13.0);
            assert_eq!(
                lines.join(" ").split_whitespace().collect::<Vec<_>>(),
                text.split_whitespace().collect::<Vec<_>>(),
                "all rule words remain reachable in order"
            );
            let mut scroll = 0;
            app.ui.wheel_override = Some(-20.0);
            let max = app.ui.scroll_text(
                &text.repeat(5),
                Rect::new(764.0, 427.0, 394.0, 218.0),
                13.0,
                &mut scroll,
            );
            assert!(
                max > 0 && scroll > 0,
                "long card text can scroll to later lines"
            );
            click(app, 960.0, 704.0);
        }
        83 => {
            assert!(app.inspect.is_none());
            app.game = Game::new(3, 1, 42, Difficulty::Easy);
            app.game.start_match().unwrap();
            app.player_deck = 3;
            app.opponent_deck = 1;
            app.selection = Some(Selection::Hand(0));
            app.targets.clear();
            app.group.clear();
            app.drag = None;
            app.hover_card = None;
            app.hover_rect = None;
            fixture_resources(app, Domain::Fury);
            click(app, 430.0, 605.0);
        }
        84 => {
            assert!(app.rune_menu.is_some(), "rune opens resource choice");
            click(app, app.rune_menu_at.x + 123.0, app.rune_menu_at.y + 78.0);
        }
        85 => {
            assert_eq!(
                app.game.players[0].energy_pool, 1,
                "manual rune stores energy"
            );
            assert_eq!(
                app.game.players[0].energy, 6,
                "exhausting into pool preserves available energy"
            );
            assert!(
                app.selection == Some(Selection::Hand(0)),
                "resource preparation preserves card selection"
            );
            click(app, 430.0, 605.0);
        }
        86 => {
            click(app, app.rune_menu_at.x + 123.0, app.rune_menu_at.y + 120.0);
        }
        87 => {
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Fury)],
                1,
                "recycle adds matching power"
            );
            assert_eq!(
                app.game.players[0].energy, 6,
                "recycle prefers exhausted rune"
            );
            assert!(app.selection == Some(Selection::Hand(0)));
            click(app, 715.0, 663.0);
        }
        88 => {
            assert!(
                app.tray == super::Tray::Board,
                "Board tray gives every unit an accessible card"
            );
        }
        89 => {
            app.selection = None;
            app.hover_card = None;
            app.hover_rect = None;
            fixture_unit(app, 0, 70_001, 54);
            app.game.players[0].gear.extend([
                GearState {
                    id: 70_010,
                    card_id: 3,
                    attached_to: Some(70_001),
                    ready: true,
                    temporary: false,
                    token: false,
                },
                GearState {
                    id: 70_011,
                    card_id: 21,
                    attached_to: Some(70_001),
                    ready: true,
                    temporary: false,
                    token: false,
                },
            ]);
        }
        90 | 91 => {
            let r = app
                .drop_zones
                .iter()
                .find(|(zone, _)| matches!(zone, super::DropZone::Unit(0, 70_001)))
                .unwrap()
                .1;
            app.hover_card = Some(3);
            app.hover_since = -1.0;
            app.ui.input_override =
                Some((vec2(r.x + r.w / 2.0, r.y + r.h - 28.0), frame == 91, false));
        }
        92 => {
            assert_eq!(
                app.inspect,
                Some(3),
                "attached gear at base can be inspected"
            );
            click(app, 960.0, 704.0);
        }
        93 => {
            assert!(app.inspect.is_none());
            app.hover_card = None;
            app.hover_rect = None;
            app.game.players[0]
                .units
                .iter_mut()
                .find(|unit| unit.id == 70_001)
                .unwrap()
                .lane = Some(0);
        }
        94 => {
            let r = app
                .drop_zones
                .iter()
                .find(|(zone, _)| matches!(zone, super::DropZone::Unit(0, 70_001)))
                .unwrap()
                .1;
            app.hover_card = Some(21);
            app.hover_since = -1.0;
            app.ui.input_override = Some((vec2(r.x + r.w / 2.0, r.y + r.h - 42.0), false, false));
        }
        95 => {
            assert_eq!(
                app.hover_card,
                Some(21),
                "second attached gear remains independently readable at field"
            );
            assert_eq!(app.game.gear_location(0, 70_011), Some(0));
            click(app, 663.0, 277.0);
        }
        96 => {
            assert_eq!(
                app.field_inspect,
                Some(3),
                "battlefield has full effect inspector"
            );
            click(app, 694.0, 502.0);
        }
        97 => {
            assert!(app.field_inspect.is_none());
            app.tray = super::Tray::Trash;
            app.gear_owner = 0;
            app.selection = None;
            app.hover_card = None;
            app.hover_rect = None;
            app.game.players[1].discard = vec![1];
            click(app, 856.0, 663.0);
        }
        98 => {
            assert_eq!(app.gear_owner, 1, "opponent public trash is browseable");
            click(app, 315.0, 760.0);
        }
        99 => {
            assert_eq!(app.inspect, Some(1));
            assert!(
                app.selection.is_none(),
                "opponent trash cannot be selected to play"
            );
            click(app, 960.0, 704.0);
        }
        100 => {
            app.game = Game::new(3, 1, 42, Difficulty::Easy);
            app.game.start_match().unwrap();
            app.player_deck = 3;
            app.opponent_deck = 1;
            app.selection = None;
            app.targets.clear();
            app.group.clear();
            app.hover_card = None;
            app.hover_rect = None;
            app.tray = super::Tray::Hand;
            app.hand_scroll = 0;
            app.gear_owner = 0;
            fixture_resources(app, Domain::Fury);
            hand_card(app, "Falling Star");
            fixture_unit(app, 1, 70_020, 25);
            fixture_unit(app, 1, 70_021, 25);
            for unit in &mut app.game.players[1].units {
                unit.lane = Some(0);
            }
            click(app, 315.0, 760.0);
            app.ui.pointer_override = Some((true, false));
        }
        101 | 102 => {
            let r = app
                .drop_zones
                .iter()
                .find(|(zone, _)| matches!(zone, super::DropZone::Unit(1, 70_020)))
                .unwrap()
                .1;
            app.ui.input_override = Some((vec2(r.x + r.w / 2.0, r.y + r.h / 2.0), false, false));
            app.ui.pointer_override = Some((frame == 101, frame == 102));
        }
        103 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 70_020
                }],
                "multi-target spell drag stages first choice"
            );
            assert_eq!(
                app.game.players[0].energy, 6,
                "partial target selection spends no resources"
            );
            assert_eq!(app.game.players[0].hand.len(), 5);
            click_unit(app, 70_021);
        }
        104 => {
            assert_eq!(app.targets.len(), 2);
            click(app, 1285.0, 510.0);
        }
        105 => {
            assert!(
                app.game
                    .chain
                    .iter()
                    .any(|item| app.game.catalog[item.card_id].name == "Falling Star"
                        && item.targets.len() == 2),
                "dragged spell casts only after all targets confirmed"
            );
            app.ui.input_override = None;
            app.ui.pointer_override = None;
            app.ui.wheel_override = None;
            println!(
                "PASS: 106 native UI frames covering click flows, card/unit/group/spell drag, invalid drops, full text, attached gear at base/field, battlefield inspection, symmetric runes and read-only opponent trash."
            );
            return true;
        }
        _ => panic!("UI smoke runner exceeded its 106-frame script"),
    }
    false
}
