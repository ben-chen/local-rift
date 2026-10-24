//! Deterministic native UI checks. Invoke before `App::draw`, starting at frame 0.
//!
//! This deliberately clicks the actual controls. A small public-state fixture
//! supplies two ready units so the test does not depend on a particular AI plan.
use super::{AdviceLevel, App, Difficulty, Game, Phase, SceneState, SceneUnit, Screen, Selection};
use local_rift::{
    cards::{CardKind, Domain},
    game::{Action, Decision, FlowSource, Target, Unit, domain_index},
    gear::GearState,
};
use macroquad::prelude::*;

thread_local! {
    static SAVED_DECK_SNAPSHOT: std::cell::RefCell<Option<Vec<u8>>> = const { std::cell::RefCell::new(None) };
    static COMBAT_SNAPSHOT: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    static DIFFICULTY_SNAPSHOT: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

fn click(app: &mut App, x: f32, y: f32) {
    app.ui.input_override = Some((vec2(x, y), true, false));
}

fn capture(app: &mut App, variable: &str) {
    if let Ok(path) = std::env::var(variable) {
        app.companion.observe(&app.game);
        let input = app.ui.input_override;
        app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
        app.draw();
        get_screen_data().export_png(&path);
        app.ui.input_override = input;
    }
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

fn unit_rect(app: &mut App, selected: u32) -> Rect {
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
            accent: app.game.legend_def(owner).accent,
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
    app.ui.logical(hit)
}
fn click_unit(app: &mut App, selected: u32) {
    let r = unit_rect(app, selected);
    click(app, r.x + r.w / 2.0, r.y + r.h / 2.0);
}

fn flow_fixture(app: &mut App, granted: bool) {
    app.game = Game::new(2, 1, 42, Difficulty::Easy);
    app.game.start_match().unwrap();
    app.player_deck = 2;
    app.opponent_deck = 1;
    fixture_resources(app, Domain::Chaos);
    hand_card(app, "Lightning Rush");
    let id = app.game.players[0].hand.remove(0);
    app.game.players[0].discard.push(id);
    app.game.players[0].any_power = 1;
    if granted {
        app.game.decision = Some(Decision::GrantFlow { player: 0 });
        app.game
            .apply(Action::SelectCards { indices: vec![0] })
            .unwrap();
        for _ in 0..20 {
            if app.game.chain.is_empty() {
                break;
            }
            app.game.apply(Action::Pass).unwrap();
        }
    }
    assert_eq!(app.game.flow_costs(0, 0).len(), if granted { 2 } else { 1 });
    app.selection = None;
    app.targets.clear();
    app.repeat_targets.clear();
    app.options = Default::default();
    app.flow_source = None;
    app.destination = None;
    app.hover_card = None;
    app.hover_rect = None;
    app.toast_until = 0.0;
    app.tray = super::Tray::Trash;
    app.gear_owner = 0;
    app.hand_scroll = 0;
    app.prefs.exact_payment = true;
}

fn staged_battlefield_fixture(app: &mut App) {
    app.game = Game::new(0, 1, 729, Difficulty::Tactical);
    app.game.start_match().unwrap();
    for player in &mut app.game.players {
        player.hand.clear();
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.exhausted_runes = [0; 6];
        player.energy_pool = 0;
        player.energy = 12;
    }
    app.game.players[0].runes = [0, 6, 0, 0, 6, 0];
    app.game.players[1].runes = [0, 5, 0, 7, 0, 0];
    app.game.players[0].hand = vec![7, 9];
    app.game.players[1].hand = vec![30, 30];
    app.game.players[0].units = (0..2)
        .map(|lane| Unit {
            id: 100 + lane as u32,
            card_id: 18,
            lane: Some(lane),
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        })
        .collect();
    app.game.battlefield_owner = [Some(0), Some(0)];
    app.game
        .apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: 0,
                unit_id: 100,
            }],
        })
        .unwrap();
    app.game.apply(Action::Choose { accept: false }).unwrap();
    for lane in 0..2 {
        app.game.apply(Action::Pass).unwrap();
        app.game
            .apply(Action::PlayCard {
                hand_index: 0,
                lane: Some(lane),
                target: None,
            })
            .unwrap();
    }
    for _ in 0..30 {
        if app.game.chain.is_empty() {
            break;
        }
        app.game.apply(Action::Pass).unwrap();
    }
    app.player_deck = 0;
    app.opponent_deck = 1;
    app.selection = None;
    app.targets.clear();
    app.hover_card = None;
    app.hover_rect = None;
    app.tray = super::Tray::Hand;
    app.prefs.exact_payment = false;
}

fn expansion_fixture(app: &mut App, card_id: usize) {
    use local_rift::deck::{CustomDeck, eligible_cards};
    let legend = [3, 1, 2, 0]
        .into_iter()
        .find(|legend| eligible_cards(*legend).contains(&card_id))
        .unwrap();
    let mut deck = CustomDeck::from_preset(legend).unwrap();
    if !deck.cards.iter().any(|(id, _)| *id == card_id) {
        deck.cards.push((card_id, 1));
    }
    app.game = Game::new_with_custom(
        [legend, 3],
        [Some(deck), None],
        1717,
        Difficulty::Easy,
        true,
    )
    .unwrap();
    app.game.start_match().unwrap();
    app.player_deck = legend;
    app.opponent_deck = 3;
    app.fields = app.game.battlefields;
    app.screen = Screen::Practice;
    app.in_match = true;
    let name = app.game.catalog[card_id].name;
    hand_card(app, name);
    let extra = app.game.players[0].hand.drain(1..).collect::<Vec<_>>();
    app.game.players[0].deck.extend(extra);
    app.game.players[0].runes = [0; 6];
    app.game.players[0].runes[domain_index(app.game.catalog[card_id].domain)] = 12;
    app.game.players[0].exhausted_runes = [0; 6];
    app.game.players[0].energy = 12;
    app.game.players[0].energy_pool = 0;
    app.game.players[0].rune_deck.clear();
    app.selection = None;
    app.targets.clear();
    app.repeat_targets.clear();
    app.group.clear();
    app.choices.clear();
    app.options = Default::default();
    app.flow_source = None;
    app.destination = None;
    app.tray = super::Tray::Hand;
    app.gear_owner = 0;
    app.hand_scroll = 0;
    app.hover_card = None;
    app.hover_rect = None;
    app.hover_unit = None;
    app.hover_token = None;
    app.hover_unit_candidate = None;
    app.toast_until = 0.0;
    app.payment_draft = None;
    app.prefs.exact_payment = false;
    app.drag = None;
    app.damage_amounts = None;
    app.damage_editor = None;
}
fn public_unit(id: u32, card_id: usize, lane: Option<usize>) -> Unit {
    Unit {
        id,
        card_id,
        lane,
        ready: true,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    }
}
fn finish_chain_until_choice(app: &mut App) {
    for _ in 0..60 {
        if app.game.decision.is_some() || app.game.chain.is_empty() {
            return;
        }
        app.game.apply(Action::Pass).unwrap();
    }
    panic!("fixture chain failed to settle");
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
            app.prefs.difficulty = Difficulty::Easy;
            app.prefs.exact_payment = false;
            app.prefs.use_custom = [false; 2];
            app.payment_draft = None;
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
            assert!(
                app.game.players[0].score >= 8,
                "burnout reaches the winning threshold"
            );
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
                Some((vec2(r.x + r.w / 2.0, r.y + r.h - 9.0), frame == 91, false));
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
            app.ui.input_override = Some((vec2(r.x + r.w - 10.0, r.y + r.h - 9.0), false, false));
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
        }
        106 => {
            app.player_deck = 1;
            app.opponent_deck = 1;
            app.game = Game::new(1, 1, 125, Difficulty::Easy);
            app.game.start_match().unwrap();
            fixture_unit(app, 0, 80_001, 23);
            fixture_unit(app, 1, 80_002, 25);
            fixture_unit(app, 1, 80_003, 25);
            app.game.players[0].units[0].ready = true;
            for unit in &mut app.game.players[1].units {
                unit.lane = Some(0);
            }
            app.game.battlefield_owner[0] = Some(1);
            app.selection = None;
            app.targets.clear();
            app.damage_order.clear();
            app.damage_amounts = None;
            app.damage_editor = None;
            app.hover_card = None;
            app.hover_rect = None;
            app.hand_scroll = 0;
            app.game
                .apply(Action::Move {
                    unit_id: 80_001,
                    lane: Some(0),
                })
                .unwrap();
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            assert!(matches!(
                app.game.decision,
                Some(Decision::CombatDamage {
                    player: 0,
                    total: 3,
                    ..
                })
            ));
            click(app, 865.0, 663.0);
        }
        107 => {
            assert_eq!(app.damage_amounts, Some(vec![(80_002, 2), (80_003, 1)]));
            app.damage_editor = Some((80_002, "65536".into()));
            assert!(
                app.commit_damage_editor().is_err(),
                "out-of-range input is rejected without wrapping"
            );
            assert_eq!(app.damage_assignment(0, 3), vec![(80_002, 2), (80_003, 1)]);
            app.damage_editor = Some((80_002, "65535".into()));
            app.commit_damage_editor().unwrap();
            assert_eq!(
                app.damage_assignment(0, 3)[0].1,
                65535,
                "typed amounts are not silently clamped"
            );
            assert!(app.damage_validation(0, 3).is_err());
            app.set_damage_amount(80_002, 2);
            click(app, 266.0, 845.0); // Reduce first target below lethal.
        }
        108 => {
            assert_eq!(app.damage_assignment(0, 3), vec![(80_002, 1), (80_003, 1)]);
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1285.0, 510.0); // Incomplete allocation must not commit.
        }
        109 => {
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_ref().unwrap(),
                    &format!("{:?}", app.game),
                    "invalid native damage confirmation is atomic"
                )
            });
            assert!(app.toast.contains("Assign all 3 damage exactly"));
            click(app, 514.0, 845.0); // 1 + 2 is complete, but wrong lethal order.
        }
        110 => {
            assert_eq!(app.damage_assignment(0, 3), vec![(80_002, 1), (80_003, 2)]);
            assert!(app.damage_validation(0, 3).unwrap_err().contains("lethal"));
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1285.0, 510.0);
        }
        111 => {
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_ref().unwrap(),
                    &format!("{:?}", app.game),
                    "lethal-order rejection preserves all game state"
                )
            });
            assert!(app.toast.contains("lethal"));
            click(app, 315.0, 750.0); // Move first target last, retaining explicit amounts.
        }
        112 => {
            assert_eq!(app.damage_assignment(0, 3), vec![(80_003, 2), (80_002, 1)]);
            assert!(app.damage_validation(0, 3).is_ok());
            assert_ne!(
                app.damage_assignment(0, 3),
                app.game.automatic_damage_assignment(0, 0, 3)
            );
            if let Ok(path) = std::env::var("RIFT_SMOKE_COMBAT_SCREENSHOT") {
                app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
                app.draw();
                get_screen_data().export_png(&path);
            }
            click(app, 1285.0, 510.0);
        }
        113 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::CombatDamage { player: 1, .. })
            ));
            assert!(
                app.game
                    .players
                    .iter()
                    .flat_map(|p| &p.units)
                    .all(|u| u.damage == 0),
                "explicit amounts still wait for simultaneous combat damage"
            );
            app.game.step_ai().unwrap();
            assert!(app.game.players[1].units.iter().any(|u| u.id == 80_002));
            assert!(
                !app.game.players[1].units.iter().any(|u| u.id == 80_003),
                "alternate allocation defeats the chosen enemy"
            );
        }
        114 => {
            app.game = Game::new(3, 1, 42, Difficulty::Easy);
            app.game.start_match().unwrap();
            app.player_deck = 3;
            app.opponent_deck = 1;
            fixture_resources(app, Domain::Fury);
            hand_card(app, "Falling Star");
            fixture_unit(app, 1, 90_001, 25);
            fixture_unit(app, 1, 90_002, 25);
            for unit in &mut app.game.players[1].units {
                unit.lane = Some(0);
            }
            app.game.players[0].power_pool[domain_index(Domain::Fury)] = 2;
            app.game.players[0].any_power = 2;
            app.game.legend[0].spell_power = 2;
            app.selection = Some(Selection::Hand(0));
            app.targets = vec![
                Target::Unit {
                    player: 1,
                    unit_id: 90_001,
                },
                Target::Unit {
                    player: 1,
                    unit_id: 90_002,
                },
            ];
            app.destination = None;
            app.options = Default::default();
            app.repeat_targets.clear();
            app.hover_card = None;
            app.hover_rect = None;
            app.tray = super::Tray::Hand;
            app.hand_scroll = 0;
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1285.0, 633.0);
        }
        115 => {
            assert!(
                app.prefs.exact_payment,
                "native payment toggle enables source choices"
            );
            click(app, 1285.0, 510.0);
        }
        116 => {
            let draft = app.payment_draft.as_ref().unwrap_or_else(|| {
                panic!("payable action opens source editor; UI said: {}", app.toast)
            });
            assert_eq!(draft.chosen.spell, 2);
            assert_eq!(draft.quote.available.any, 2);
            assert_eq!(draft.quote.available.recycle[0], 6);
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_ref().unwrap(),
                    &format!("{:?}", app.game),
                    "opening payment editor does not pay or play"
                )
            });
            click(app, 480.0, 709.0);
        }
        117 => {
            assert!(app.payment_draft.is_none());
            assert!(matches!(app.selection, Some(Selection::Hand(0))));
            assert_eq!(app.targets.len(), 2);
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_ref().unwrap(),
                    &format!("{:?}", app.game),
                    "cancelling payment preserves every game field"
                )
            });
            click(app, 1285.0, 510.0);
        }
        118 | 119 => {
            assert!(app.payment_draft.is_some());
            click(app, 567.0, 575.0);
        }
        120 => {
            assert_eq!(app.payment_draft.as_ref().unwrap().chosen.total(), 0);
            click(app, 936.0, 709.0);
        }
        121 => {
            assert!(
                app.payment_draft.is_some(),
                "invalid payment keeps draft open"
            );
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_ref().unwrap(),
                    &format!("{:?}", app.game),
                    "invalid payment confirmation preserves every game field"
                )
            });
            click(app, 661.0, 539.0);
        }
        122 => {
            click(app, 661.0, 539.0);
        }
        123 => {
            let draft = app.payment_draft.as_ref().unwrap();
            assert_eq!(draft.chosen.any, 2);
            assert_eq!(draft.chosen.spell, 0);
            assert!(app.game.clone().apply(App::payment_action(draft)).is_ok());
            if let Ok(path) = std::env::var("RIFT_SMOKE_PAYMENT_SCREENSHOT") {
                app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
                app.draw();
                get_screen_data().export_png(&path);
            }
            click(app, 936.0, 709.0);
        }
        124 => {
            assert!(app.payment_draft.is_none());
            assert!(app.game.chain.iter().any(|item| item.card_id == 53));
            assert_eq!(app.game.players[0].any_power, 0, "chosen source was spent");
            assert_eq!(
                app.game.legend[0].spell_power, 2,
                "unchosen spell power remains"
            );
            assert_eq!(
                app.game.players[0].power_pool[0], 2,
                "unchosen colored power remains"
            );
            assert_eq!(
                app.game.players[0].total_runes(),
                6,
                "unchosen runes were not recycled"
            );
            assert_eq!(
                app.game.players[0].energy, 4,
                "normal energy cost still paid"
            );
            let unchanged = format!("{:?}", app.game);
            app.action(Action::PlayCard {
                hand_index: usize::MAX,
                lane: None,
                target: None,
            });
            assert!(
                app.payment_draft.is_none(),
                "an invalid action does not open payment editor"
            );
            assert_eq!(format!("{:?}", app.game), unchanged);
            app.action(Action::Pass);
            assert!(
                app.payment_draft.is_none(),
                "zero-power action needs no source editor"
            );
            app.prefs.exact_payment = false;
            app.prefs.save();
        }
        125 => {
            app.game = Game::new(3, 1, 42, Difficulty::Easy);
            app.game.start_match().unwrap();
            fixture_resources(app, Domain::Fury);
            app.game.players[0].exhausted_runes[0] = 2;
            app.game.players[0].energy = 4;
            app.selection = Some(Selection::Hand(0));
            app.targets.clear();
            app.toast_until = 0.0;
            app.hover_card = None;
            app.hover_rect = None;
            click(app, 430.0, 605.0);
        }
        126 => {
            assert!(app.rune_menu.is_some());
            click(app, app.rune_menu_at.x + 123.0, app.rune_menu_at.y + 162.0);
        }
        127 => {
            assert_eq!(app.game.players[0].runes[0], 5);
            assert_eq!(
                app.game.players[0].exhausted_runes[0], 2,
                "ready-rune choice preserves exhausted runes"
            );
            assert_eq!(
                app.game.players[0].energy, 3,
                "direct ready recycling forgoes its energy"
            );
            assert_eq!(app.game.players[0].energy_pool, 0);
            assert_eq!(app.game.players[0].power_pool[0], 1);
            assert!(
                matches!(app.selection, Some(Selection::Hand(0))),
                "rune source preparation preserves card selection"
            );
            click(app, 430.0, 605.0);
        }
        128 => {
            assert!(app.rune_menu.is_some());
            if let Ok(path) = std::env::var("RIFT_SMOKE_RUNE_SCREENSHOT") {
                app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
                app.draw();
                get_screen_data().export_png(&path);
            }
            click(app, app.rune_menu_at.x + 123.0, app.rune_menu_at.y + 120.0);
        }
        129 => {
            assert_eq!(app.game.players[0].runes[0], 4);
            assert_eq!(
                app.game.players[0].exhausted_runes[0], 1,
                "exhausted-rune choice preserves ready runes"
            );
            assert_eq!(app.game.players[0].energy, 3);
            assert_eq!(app.game.players[0].power_pool[0], 2);
            assert!(matches!(app.selection, Some(Selection::Hand(0))));
        }
        130 => {
            flow_fixture(app, true);
            click(app, 315.0, 760.0);
        }
        131 => {
            assert!(matches!(app.selection, Some(Selection::Trash(0))));
            assert_eq!(
                app.flow_source, None,
                "two Flow costs require an explicit choice"
            );
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1285.0, 510.0);
        }
        132 => {
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_ref().unwrap(),
                    &format!("{:?}", app.game),
                    "missing Flow choice cannot cast"
                )
            });
            click(app, 1229.0, 440.0);
        }
        133 => {
            assert_eq!(app.flow_source, Some(FlowSource::Printed));
            if let Ok(path) = std::env::var("RIFT_SMOKE_FLOW_SCREENSHOT") {
                app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
                app.draw();
                get_screen_data().export_png(&path);
            }
            click(app, 1285.0, 510.0);
        }
        134 => {
            let draft = app
                .payment_draft
                .as_ref()
                .expect("printed Flow has one power payment");
            assert_eq!(draft.quote.suggested.total(), 1);
            assert!(matches!(
                draft.action,
                Action::PlayFromTrashOptions {
                    flow: Some(FlowSource::Printed),
                    ..
                }
            ));
            click(app, 480.0, 709.0);
        }
        135 => {
            assert!(app.payment_draft.is_none());
            assert_eq!(
                app.flow_source,
                Some(FlowSource::Printed),
                "cancel preserves the chosen Flow instance"
            );
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_ref().unwrap(),
                    &format!("{:?}", app.game),
                    "Flow payment cancel preserves all game state"
                )
            });
            click(app, 1285.0, 510.0);
        }
        136 => {
            click(app, 936.0, 709.0);
        }
        137 => {
            assert_eq!(
                app.game.players[0].energy, 4,
                "printed Lightning Rush Flow costs2 energy"
            );
            assert_eq!(
                app.game.players[0].any_power, 0,
                "printed Flow uses its Any power"
            );
            assert!(
                app.game
                    .chain
                    .iter()
                    .any(|item| item.card_id == 39 && item.banish_after)
            );
            flow_fixture(app, true);
            click(app, 315.0, 760.0);
        }
        138 => {
            click(app, 1339.0, 440.0);
        }
        139 => {
            assert_eq!(app.flow_source, Some(FlowSource::Granted));
            click(app, 1285.0, 510.0);
        }
        140 => {
            assert!(
                app.payment_draft.is_none(),
                "granted Lightning Rush Flow has no power cost"
            );
            assert_eq!(
                app.game.players[0].energy, 5,
                "granted Flow uses the printed1 energy cost"
            );
            assert_eq!(
                app.game.players[0].any_power, 1,
                "granted Flow retains unused power"
            );
            assert!(
                app.game
                    .chain
                    .iter()
                    .any(|item| item.card_id == 39 && item.banish_after)
            );
            flow_fixture(app, false);
            let before = format!("{:?}", app.game);
            app.action(Action::PlayFromTrashOptions {
                trash_index: 0,
                flow: Some(FlowSource::Granted),
                lane: None,
                targets: vec![],
                repeat_targets: vec![],
                repeat: false,
                accelerate: false,
                bonus: false,
            });
            assert_eq!(
                before,
                format!("{:?}", app.game),
                "unavailable Flow never silently falls back"
            );
            assert!(app.payment_draft.is_none());
            staged_battlefield_fixture(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::BattlefieldOrder {
                    player: 0,
                    combat: true,
                    ..
                })
            ));
            let before = format!("{:?}", app.game);
            app.action(Action::ChooseBattlefield { lane: 2 });
            assert_eq!(
                before,
                format!("{:?}", app.game),
                "invalid field selection is atomic"
            );
            app.toast_until = 0.0;
        }
        141 => {
            if let Ok(path) = std::env::var("RIFT_SMOKE_FIELDS_SCREENSHOT") {
                app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
                app.draw();
                get_screen_data().export_png(&path);
            }
            click(app, 1285.0, 532.0);
        }
        142 => {
            let window = app
                .game
                .showdown
                .as_ref()
                .expect("native choice opens selected field");
            assert_eq!(window.lane, 1, "player chooses the non-default battlefield");
            assert_eq!(window.attacker, 1);
            assert_eq!(window.focus, 1, "actual contender gets focus");
        }
        143 => {
            app.screen = Screen::Decks;
            app.in_match = false;
            app.browse_deck = 0;
            app.custom_decks = [None, None];
            app.prefs.use_custom = [false; 2];
            app.hover_card = None;
            app.hover_rect = None;
            app.toast_until = 0.0;
            click(app, 425.0, 657.0);
        }
        144 => {
            assert!(app.screen == Screen::DeckEditor);
            assert_eq!(app.deck_editor.as_ref().unwrap().deck.card_count(), 40);
            click(app, 1323.0, 152.0);
        }
        145 => {
            let name = app.game.catalog[72].name.to_owned();
            app.deck_editor.as_mut().unwrap().query = name;
            app.deck_editor.as_mut().unwrap().scroll = 0;
            let editor = app.deck_editor.as_ref().unwrap();
            assert!(editor.filter == super::deck_editor::DeckFilter::Additional);
            let row = editor
                .visible_cards()
                .iter()
                .position(|id| *id == 72)
                .unwrap();
            click(app, 1365.0, 236.0 + row as f32 * 63.0);
        }
        146 => {
            let editor = app.deck_editor.as_ref().unwrap();
            assert_eq!(editor.count(72), 1);
            assert_eq!(
                editor.deck.card_count(),
                41,
                "constructed minimum permits more than40"
            );
            let row = editor
                .visible_cards()
                .iter()
                .position(|id| *id == 72)
                .unwrap();
            click(app, 1274.0, 236.0 + row as f32 * 63.0);
        }
        147 => {
            let editor = app.deck_editor.as_ref().unwrap();
            assert_eq!(editor.count(72), 0);
            assert_eq!(editor.deck.card_count(), 40);
            let row = editor
                .visible_cards()
                .iter()
                .position(|id| *id == 72)
                .unwrap();
            click(app, 1365.0, 236.0 + row as f32 * 63.0);
        }
        148 => {
            assert_eq!(app.deck_editor.as_ref().unwrap().count(72), 1);
            SAVED_DECK_SNAPSHOT.with(|saved| {
                *saved.borrow_mut() = std::fs::read(super::deck_editor::saved_path(0)).ok()
            });
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 416.0, 504.0);
        }
        149 => {
            let editor = app.deck_editor.as_ref().unwrap();
            assert_eq!(editor.deck.runes.iter().sum::<u8>(), 11);
            assert!(editor.deck.validate().is_err());
            click(app, 328.0, 771.0);
        }
        150 => {
            assert!(
                app.custom_decks[0].is_none(),
                "invalid deck cannot be saved by button"
            );
            let invalid = app.deck_editor.as_ref().unwrap().deck.clone();
            assert!(
                app.save_editor_deck(0, &invalid).is_err(),
                "save also validates independently of disabled button"
            );
            SAVED_DECK_SNAPSHOT.with(|saved| {
                assert_eq!(
                    *saved.borrow(),
                    std::fs::read(super::deck_editor::saved_path(0)).ok(),
                    "invalid save preserves disk"
                )
            });
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(format!("{:?}", app.game).as_str()),
                    "editing never mutates match"
                )
            });
            click(app, 505.0, 504.0);
        }
        151 => {
            assert!(app.deck_editor.as_ref().unwrap().deck.validate().is_ok());
            if let Ok(path) = std::env::var("RIFT_SMOKE_EDITOR_SCREENSHOT") {
                app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
                app.draw();
                get_screen_data().export_png(&path);
            }
            click(app, 328.0, 771.0);
        }
        152 => {
            let saved = app.custom_decks[0].as_ref().expect("You slot saved");
            assert_eq!(saved.card_count(), 41);
            assert_eq!(saved.cards.iter().find(|(id, _)| *id == 72).unwrap().1, 1);
            assert_eq!(
                super::deck_editor::load_saved_decks()[0].as_ref(),
                Some(saved)
            );
            assert!(app.prefs.use_custom[0]);
            click(app, 462.0, 771.0);
        }
        153 => {
            assert!(app.prefs.use_custom[1]);
            assert_eq!(super::deck_editor::load_saved_decks(), app.custom_decks);
            let row = app
                .deck_editor
                .as_ref()
                .unwrap()
                .visible_cards()
                .iter()
                .position(|id| *id == 72)
                .unwrap();
            click(app, 1365.0, 236.0 + row as f32 * 63.0);
        }
        154 => {
            assert_eq!(app.deck_editor.as_ref().unwrap().count(72), 2);
            assert_eq!(
                app.custom_decks[0]
                    .as_ref()
                    .unwrap()
                    .cards
                    .iter()
                    .find(|(id, _)| *id == 72)
                    .unwrap()
                    .1,
                1
            );
            click(app, 395.0, 816.0);
        }
        155 => {
            assert!(app.screen == Screen::Decks);
            assert_eq!(
                super::deck_editor::load_saved_decks(),
                app.custom_decks,
                "leaving unsaved draft preserves slots"
            );
            click(app, 342.0, 696.0);
        }
        156 => {
            assert_eq!(
                app.deck_editor.as_ref().unwrap().count(72),
                1,
                "reopen restores saved deck"
            );
            click(app, 395.0, 816.0);
        }
        157 => {
            click(app, 341.0, 748.0);
        }
        158 => {
            assert!(
                !app.prefs.use_custom[0],
                "selecting a preset deactivates custom deck"
            );
            assert!(
                app.custom_decks[0].is_some(),
                "preset selection preserves saved file"
            );
            click(app, 342.0, 696.0);
        }
        159 => {
            click(app, 328.0, 771.0);
        }
        160 => {
            assert!(app.prefs.use_custom[0]);
            assert_eq!(
                super::Preferences::load().use_custom,
                [true, true],
                "custom slot selection survives restart"
            );
            click(app, 395.0, 816.0);
        }
        161 => {
            click(app, 425.0, 802.0);
        }
        162 => {
            assert!(app.screen == Screen::Practice && !app.in_match);
            app.starting_player = 0;
            click(app, 1330.0, 46.0);
        }
        163 => {
            assert_eq!(app.game.phase, Phase::Mulligan);
            for owner in 0..2 {
                assert_eq!(
                    app.game.custom_deck(owner),
                    app.custom_decks[owner].as_ref()
                );
                let player = &app.game.players[owner];
                assert_eq!(
                    player.hand.len() + player.deck.len() + usize::from(player.champion.is_some()),
                    41
                );
                assert_eq!(
                    player
                        .hand
                        .iter()
                        .chain(&player.deck)
                        .filter(|id| **id == 72)
                        .count(),
                    1,
                    "new card is in actual playable deck"
                );
            }
            click(app, 1283.0, 514.0);
        }
        164 => {
            assert_eq!(app.game.phase, Phase::Playing);
            fixture_resources(app, Domain::Chaos);
            hand_card(app, "Shipyard Skulker");
            click(app, 315.0, 760.0);
        }
        165 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            click(app, 1285.0, 510.0);
        }
        166 => {
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .any(|unit| unit.card_id == 72),
                "additional card actually deploys through native controls"
            );
            assert_eq!(app.game.catalog[72].name, "Shipyard Skulker");
        }
        167 => {
            expansion_fixture(app, 85);
            app.game.players[0].units =
                vec![public_unit(100, 71, None), public_unit(101, 71, None)];
            app.game.players[1].units = vec![public_unit(200, 69, None)];
            click(app, 315.0, 760.0);
        }
        168 => {
            assert!(app.target_hint(85).contains("friendly"));
            click_unit(app, 200);
        }
        169 => {
            assert!(app.targets.is_empty(), "enemy is not a Back to Back target");
            click_unit(app, 100);
        }
        170 => {
            assert_eq!(app.targets.len(), 1);
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1285.0, 510.0);
        }
        171 => {
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(format!("{:?}", app.game).as_str()),
                    "partial required target selection is atomic"
                )
            });
            click_unit(app, 101);
        }
        172 => {
            assert_eq!(app.targets.len(), 2);
            click(app, 1285.0, 510.0);
        }
        173 => {
            assert!(
                app.game
                    .chain
                    .iter()
                    .any(|item| item.card_id == 85 && item.targets.len() == 2)
            );
            finish_chain_until_choice(app);
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .all(|unit| app.game.unit_might(unit) == 7)
            );
        }
        174 => {
            expansion_fixture(app, 83);
            app.game.players[0].units =
                vec![public_unit(100, 70, None), public_unit(101, 70, None)];
            click(app, 315.0, 760.0);
        }
        175 => {
            click_unit(app, 100);
        }
        176 => {
            click_unit(app, 101);
        }
        177 => {
            assert_eq!(app.targets.len(), 2, "Clash can choose two friendly units");
            click(app, 1285.0, 510.0);
        }
        178 => {
            assert!(app.game.chain.iter().any(|item| {
                item.card_id == 83
                    && item
                        .targets
                        .iter()
                        .all(|target| matches!(target, Target::Unit { player: 0, .. }))
            }));
            finish_chain_until_choice(app);
            assert!(
                app.game.players[0].units.is_empty(),
                "both chosen units deal their actual might"
            );
        }
        179 => {
            expansion_fixture(app, 87);
            app.game.players[0].units =
                vec![public_unit(100, 72, Some(0)), public_unit(101, 73, Some(1))];
            app.game.battlefield_owner = [Some(0), Some(0)];
            click(app, 315.0, 760.0);
        }
        180 => {
            click(app, 1285.0, 510.0);
        }
        181 => {
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TokenPlacement {
                    remaining: 4,
                    card_id: 87,
                    ..
                })
            ));
            assert!(app.game.players[0].discard.contains(&87));
            assert!(!app.game.chain.iter().any(|item| item.card_id == 87));
            assert!(!app.game.players[0].units.iter().any(|unit| unit.token));
        }
        182 => {
            click(app, 1285.0, 470.0);
        }
        183 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TokenPlacement { remaining: 3, .. })
            ));
            assert_eq!(
                app.game.players[0]
                    .units
                    .iter()
                    .filter(|unit| unit.token)
                    .count(),
                1,
                "the first token enters before the next placement choice"
            );
            click(app, 1285.0, 505.0);
        }
        184 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TokenPlacement { remaining: 2, .. })
            ));
            assert_eq!(
                app.game.players[0]
                    .units
                    .iter()
                    .filter(|unit| unit.token)
                    .count(),
                2
            );
            click(app, 1285.0, 540.0);
        }
        185 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TokenPlacement { remaining: 1, .. })
            ));
            assert_eq!(
                app.game.players[0]
                    .units
                    .iter()
                    .filter(|unit| unit.token)
                    .count(),
                3
            );
            click(app, 1285.0, 470.0);
        }
        186 => {
            let recruits = app.game.players[0]
                .units
                .iter()
                .filter(|unit| unit.token)
                .collect::<Vec<_>>();
            assert_eq!(recruits.len(), 4);
            assert!(
                recruits
                    .iter()
                    .all(|unit| app.game.unit_name(unit) == "Recruit token")
            );
            assert_eq!(
                recruits.iter().filter(|unit| unit.lane.is_none()).count(),
                2
            );
            assert_eq!(
                recruits.iter().filter(|unit| unit.lane == Some(0)).count(),
                1
            );
            assert_eq!(
                recruits.iter().filter(|unit| unit.lane == Some(1)).count(),
                1
            );
            finish_chain_until_choice(app);
            assert!(app.game.decision.is_none());
        }
        187 => {
            expansion_fixture(app, 84);
            let extra = app.game.players[0].deck.pop().unwrap();
            app.game.players[0].hand.push(extra);
            click(app, 315.0, 760.0);
        }
        188 => {
            click(app, 1285.0, 510.0);
        }
        189 => {
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::Discard {
                    count: 1,
                    draw_after: 2,
                    ..
                })
            ));
            assert!(
                app.game.chain.iter().any(|item| item.card_id == 84),
                "resolving Loot keeps physical spell on chain"
            );
            click(app, 315.0, 760.0);
        }
        190 => {
            click(app, 1285.0, 510.0);
        }
        191 => {
            assert_eq!(app.game.players[0].hand.len(), 2);
            assert!(app.game.decision.is_none());
            finish_chain_until_choice(app);
            assert!(app.game.players[0].discard.contains(&84));
        }
        192 => {
            expansion_fixture(app, 98);
            click(app, 315.0, 760.0);
        }
        193 => {
            click(app, 1229.0, 468.0);
        }
        194 => {
            assert!(app.options.accelerate, "new metadata exposes Accelerate");
            click(app, 1285.0, 510.0);
        }
        195 => {
            let unit = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 98)
                .unwrap();
            assert!(unit.ready);
            assert_eq!(
                app.game.players[0].energy,
                12 - app.game.catalog[98].cost - 1
            );
        }
        196 => {
            let unit_id = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 98)
                .unwrap()
                .id;
            click_unit(app, unit_id);
            let position = app.ui.mouse();
            app.ui.input_override = Some((position, false, false));
            app.hover_card = Some(98);
            app.hover_unit = Some((0, unit_id));
            app.hover_since = -1.0;
            app.hover_rect = None;
        }
        197 => {
            let unit_id = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 98)
                .unwrap()
                .id;
            assert_eq!(app.hover_unit, Some((0, unit_id)));
            let panel = app.hover_rect.expect("live keyword hover appears");
            let text = super::live_unit_description(&app.game, 0, unit_id).unwrap();
            assert!(
                text.contains("Assault 2")
                    && text.contains("Deflect 1")
                    && text.contains("Ganking")
                    && text.contains("Accelerate, when played")
            );
            app.ui.input_override = Some((vec2(panel.x + 40.0, panel.y + 240.0), false, false));
            if let Ok(path) = std::env::var("RIFT_SMOKE_KEYWORDS_SCREENSHOT") {
                app.draw();
                get_screen_data().export_png(&path);
            }
            app.ui.wheel_override = Some(-12.0);
        }
        198 => {
            assert!(
                app.hover_scroll > 0,
                "full keyword and printed text remain scrollable"
            );
            expansion_fixture(app, 96);
            let id = app.game.players[0].hand.remove(0);
            app.game.players[0].discard.push(id);
            app.game.players[0].units = vec![public_unit(300, 45, None)];
            app.game.players[0].power_pool[domain_index(Domain::Chaos)] = 1;
            app.game
                .apply(Action::EmpowerUnit {
                    unit_id: 300,
                    pay_power: false,
                })
                .unwrap();
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::ReviveUnit { player: 0 })
            ));
            click(app, 315.0, 760.0);
        }
        199 => {
            assert!(app.selection == Some(Selection::Trash(0)));
            click(app, 1229.0, 468.0);
        }
        200 => {
            assert!(
                !app.options.accelerate,
                "additional cost is not chosen before revival resolves"
            );
            click(app, 1285.0, 510.0);
        }
        201 => {
            assert!(app.game.chain.iter().any(|item| matches!(
                item.ability,
                Some(local_rift::game::ChainAbility::ReviveChosen { .. })
            )));
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::PendingPlay {
                    card_id: 96,
                    kind: local_rift::game::InstructedPlayKind::Matriarch,
                    ..
                })
            ));
            assert!(!app.game.players[0].discard.contains(&96));
            assert_eq!(app.game.pending_cards().len(), 1);
        }
        202 => {
            assert!(
                matches!(app.selection, Some(Selection::Pending(_))),
                "chosen revival is selected automatically"
            );
            click(app, 1229.0, 468.0);
        }
        203 => {
            assert!(app.options.accelerate);
            click(app, 1285.0, 510.0);
        }
        204 => {
            let unit = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 96)
                .unwrap();
            assert!(unit.ready && unit.lane.is_none());
            assert_eq!(
                app.game.players[0].energy, 9,
                "Matriarch2 + Accelerate1; base unit cost waived"
            );
            assert!(app.game.decision.is_none());
        }
        205 => {
            expansion_fixture(app, 50);
            app.game.players[1].units = (0..4).map(|i| public_unit(300 + i, 69, None)).collect();
            click(app, 315.0, 760.0);
        }
        206 => {
            click_unit(app, 300);
        }
        207 => {
            click_unit(app, 301);
        }
        208 => {
            click_unit(app, 302);
        }
        209 => {
            click_unit(app, 303);
        }
        210 => {
            assert_eq!(
                app.targets.len(),
                3,
                "shared metadata rejects a fourth Bellows target"
            );
            click(app, 1285.0, 510.0);
        }
        211 => {
            assert!(
                app.game
                    .chain
                    .iter()
                    .any(|item| item.card_id == 50 && item.targets.len() == 3)
            );
        }
        212 => {
            expansion_fixture(app, 99);
            app.game.players[0].hand.push(69);
            app.game
                .apply(Action::PlayCard {
                    hand_index: 1,
                    lane: None,
                    target: None,
                })
                .unwrap();
            finish_chain_until_choice(app);
            assert_eq!(app.game.cards_played_this_turn(0), 1);
            click(app, 315.0, 760.0);
        }
        213 => {
            click(app, 1285.0, 510.0);
        }
        214 => {
            assert!(
                matches!(
                    app.game.decision,
                    Some(Decision::OnPlayTarget { card_id: 99, .. })
                ),
                "Legion target is chosen after unit enters"
            );
            let unit_id = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 69)
                .unwrap()
                .id;
            click_unit(app, unit_id);
        }
        215 => {
            finish_chain_until_choice(app);
            let unit = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 69)
                .unwrap();
            assert_eq!(
                app.game.unit_might(unit),
                10,
                "new Legion target resolves through native choice"
            );
        }
        216 => {
            app.screen = Screen::Practice;
            app.in_match = false;
            app.selection = None;
            app.hover_card = None;
            app.hover_rect = None;
            app.hover_unit = None;
            app.hover_token = None;
            app.toast_until = 0.0;
            app.prefs.use_custom = [false; 2];
            app.prefs.difficulty = Difficulty::Easy;
            app.player_deck = 3;
            app.opponent_deck = 1;
            app.fields = [
                local_rift::battlefields::default_battlefield(3),
                local_rift::battlefields::default_battlefield(1),
            ];
            app.starting_player = 0;
            click(app, 1357.0, 312.0);
        }
        217 => {
            assert_eq!(app.prefs.difficulty, Difficulty::Strategic);
            assert_eq!(super::Preferences::load().difficulty, Difficulty::Strategic);
            assert_eq!(
                app.game.difficulty,
                Difficulty::Easy,
                "setup choice does not mutate the previous game"
            );
            if let Ok(path) = std::env::var("RIFT_SMOKE_DIFFICULTY_SCREENSHOT") {
                app.draw();
                get_screen_data().export_png(&path);
            }
            click(app, 1284.0, 312.0);
        }
        218 => {
            assert_eq!(app.prefs.difficulty, Difficulty::Tactical);
            click(app, 1211.0, 312.0);
        }
        219 => {
            assert_eq!(app.prefs.difficulty, Difficulty::Easy);
            click(app, 1357.0, 312.0);
        }
        220 => {
            assert_eq!(app.prefs.difficulty, Difficulty::Strategic);
            click(app, 1330.0, 46.0);
        }
        221 => {
            assert_eq!(app.game.difficulty, Difficulty::Strategic);
            assert_eq!(app.game.phase, Phase::Mulligan);
            click(app, 1283.0, 514.0);
        }
        222 => {
            assert_eq!(app.game.phase, Phase::Playing);
            click(app, 100.0, 323.0);
        }
        223 => {
            assert!(app.screen == Screen::Settings);
            DIFFICULTY_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1097.0, 383.0);
        }
        224 => {
            assert_eq!(app.prefs.difficulty, Difficulty::Easy);
            assert_eq!(super::Preferences::load().difficulty, Difficulty::Easy);
            assert_eq!(app.game.difficulty, Difficulty::Strategic);
            DIFFICULTY_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(format!("{:?}", app.game).as_str()),
                    "changing next-match settings must preserve the entire active game"
                )
            });
            click(app, 1209.0, 383.0);
        }
        225 => {
            assert_eq!(app.prefs.difficulty, Difficulty::Tactical);
            assert_eq!(app.game.difficulty, Difficulty::Strategic);
            click(app, 1321.0, 383.0);
        }
        226 => {
            assert_eq!(app.prefs.difficulty, Difficulty::Strategic);
            assert_eq!(super::Preferences::load().difficulty, Difficulty::Strategic);
            click(app, 1097.0, 383.0);
        }
        227 => {
            assert_eq!(app.prefs.difficulty, Difficulty::Easy);
            assert_eq!(app.game.difficulty, Difficulty::Strategic);
            click(app, 100.0, 167.0);
        }
        228 => {
            assert!(app.screen == Screen::Practice);
            assert_eq!(app.game.difficulty, Difficulty::Strategic);
            click(app, 1330.0, 46.0);
        }
        229 => {
            assert!(app.confirm_restart);
            click(app, 854.0, 501.0);
        }
        230 => {
            assert!(!app.in_match && !app.confirm_restart);
            click(app, 1330.0, 46.0);
        }
        231 => {
            assert_eq!(app.game.difficulty, Difficulty::Easy);
            assert_eq!(app.game.phase, Phase::Mulligan);
        }
        232 => {
            expansion_fixture(app, 110);
            app.game.players[0].units = vec![
                public_unit(100, 110, Some(0)),
                public_unit(101, 110, Some(0)),
            ];
            app.game.players[1].units = vec![public_unit(200, 69, Some(0))];
            app.game.players[0].gear.push(GearState {
                id: 800,
                card_id: 121,
                ready: true,
                attached_to: Some(100),
                temporary: false,
                token: false,
            });
            app.game.battlefield_owner[0] = Some(0);
            app.game.showdown = Some(local_rift::game::Showdown {
                lane: 0,
                attacker: 1,
                focus: 0,
                passes: 0,
                combat: true,
            });
            let r = unit_rect(app, 100);
            app.ui.input_override = Some((vec2(r.x + r.w / 2.0, r.y + 10.0), false, false));
            app.hover_card = Some(110);
            app.hover_unit = Some((0, 100));
            app.hover_since = -1.0;
        }
        233 => {
            let text = super::live_unit_description(&app.game, 0, 100).unwrap();
            assert!(
                text.contains("4 current might")
                    && text.contains("Shield 1")
                    && text.contains("Tank:")
            );
            let panel = app.hover_rect.expect("Shield hover appears");
            app.ui.input_override = Some((vec2(panel.x + 40.0, panel.y + 240.0), false, false));
            if let Ok(path) = std::env::var("RIFT_SMOKE_SHIELD_SCREENSHOT") {
                app.draw();
                get_screen_data().export_png(&path);
            }
            app.hover_card = None;
            app.hover_rect = None;
            app.hover_unit = None;
            let r = unit_rect(app, 100);
            app.ui.input_override = Some((vec2(r.x + r.w / 2.0, r.y + r.h - 9.0), false, false));
        }
        234 => {
            assert_eq!(
                app.hover_card,
                Some(121),
                "attached gear has its own readable hover"
            );
            assert_eq!(app.hover_unit, None);
            let r = unit_rect(app, 100);
            click(app, r.x + r.w / 2.0, r.y + r.h - 9.0);
        }
        235 => {
            assert_eq!(
                app.inspect,
                Some(121),
                "gear chip opens full equipment rules"
            );
            click(app, 960.0, 704.0);
        }
        236 => {
            assert!(app.inspect.is_none());
            expansion_fixture(app, 135);
            app.game.players[0].units =
                vec![public_unit(100, 110, None), public_unit(101, 110, None)];
            click(app, 315.0, 760.0);
        }
        237 => {
            click(app, 1229.0, 468.0);
        }
        238 => {
            assert!(app.options.repeat, "new Repeat cards use generic metadata");
            click_unit(app, 100);
        }
        239 => {
            click(app, 1339.0, 468.0);
        }
        240 => {
            assert!(app.editing_repeat);
            click_unit(app, 101);
        }
        241 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: 100
                }]
            );
            assert_eq!(
                app.repeat_targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: 101
                }]
            );
            click(app, 1285.0, 510.0);
        }
        242 => {
            assert!(
                app.game
                    .chain
                    .iter()
                    .any(|item| item.card_id == 135 && item.options.repeat)
            );
            assert_eq!(app.game.players[0].energy, 8);
            finish_chain_until_choice(app);
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .all(|unit| app.game.unit_might(unit) == 4)
            );
        }
        243 => {
            expansion_fixture(app, 133);
            click(app, 315.0, 760.0);
        }
        244 => {
            click(app, 1229.0, 468.0);
        }
        245 => {
            assert!(app.options.repeat);
            click(app, 1339.0, 468.0);
        }
        246 => {
            assert!(
                !app.editing_repeat,
                "draw-only Repeat does not offer target editing"
            );
            click(app, 1285.0, 510.0);
        }
        247 => {
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].hand.len(), 2);
            assert_eq!(app.game.players[0].energy, 8);
        }
        248 => {
            expansion_fixture(app, 124);
            app.game.players[0].units = vec![public_unit(100, 69, None)];
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
            app.selection = Some(Selection::Gear(app.game.players[0].gear[0].id));
            app.tray = super::Tray::Loadout;
            click_unit(app, 100);
        }
        249 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: 100
                }]
            );
            click(app, 1285.0, 522.0);
        }
        250 => {
            assert_eq!(
                app.game.players[0].energy, 10,
                "play costs1 and Equip costs1 energy"
            );
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].gear[0].attached_to, Some(100));
            assert_eq!(
                app.game.unit_keywords(&app.game.players[0].units[0]).shield,
                2
            );
        }
        251 => {
            expansion_fixture(app, 130);
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
            app.selection = Some(Selection::Gear(app.game.players[0].gear[0].id));
            app.tray = super::Tray::Loadout;
            click(app, 1285.0, 513.0);
        }
        252 => {
            assert_eq!(app.game.players[0].energy, 10);
            assert_eq!(app.game.players[0].energy_pool, 1);
            assert!(!app.game.players[0].gear[0].ready);
            app.selection = Some(Selection::Gear(app.game.players[0].gear[0].id));
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1285.0, 522.0);
        }
        253 => {
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(format!("{:?}", app.game).as_str()),
                    "exhausted resource gear offers no attach action"
                )
            });
            expansion_fixture(app, 135);
            app.game.players[0].hand = vec![36];
            app.game.players[0].discard = vec![69, 135];
            app.game.players[0].power_pool[domain_index(Domain::Chaos)] = 1;
            app.game.players[0].units =
                vec![public_unit(100, 110, None), public_unit(101, 110, None)];
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::OptionalPlay { card_id: 36, .. })
            ));
            click(app, 1285.0, 506.0);
        }
        254 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TrashTarget {
                    source_card_id: 36,
                    ..
                })
            ));
            click(app, 315.0, 760.0);
        }
        255 => {
            assert!(app.game.chain.iter().any(|item| matches!(
                item.ability,
                Some(local_rift::game::ChainAbility::PlayChosen { card_id: 135, .. })
            )));
            assert!(app.game.players[0].discard.contains(&135));
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::PendingPlay {
                    card_id: 135,
                    kind: local_rift::game::InstructedPlayKind::Fizz,
                    ..
                })
            ));
            assert!(!app.game.players[0].discard.contains(&135));
        }
        256 => {
            assert!(matches!(app.selection, Some(Selection::Pending(_))));
            assert!(!app.game.can_decline_trash_play());
            click(app, 1229.0, 468.0);
        }
        257 => {
            click_unit(app, 100);
        }
        258 => {
            click(app, 1339.0, 468.0);
        }
        259 => {
            click_unit(app, 101);
        }
        260 => {
            assert!(app.options.repeat && app.targets.len() == 1 && app.repeat_targets.len() == 1);
            click(app, 1285.0, 510.0);
        }
        261 => {
            assert!(app.game.pending_cards().is_empty());
            assert!(
                app.game
                    .chain
                    .iter()
                    .any(|item| item.card_id == 135 && item.options.repeat && item.recycle_after)
            );
            assert_eq!(
                app.game.players[0].energy, 7,
                "Fizz pays3; chosen spell base waived, Repeat costs2"
            );
            finish_chain_until_choice(app);
            assert_eq!(
                app.game.unit_might(
                    app.game.players[0]
                        .units
                        .iter()
                        .find(|unit| unit.id == 100)
                        .unwrap()
                ),
                4
            );
            assert_eq!(
                app.game.unit_might(
                    app.game.players[0]
                        .units
                        .iter()
                        .find(|unit| unit.id == 101)
                        .unwrap()
                ),
                4
            );
        }
        262 => {
            expansion_fixture(app, 130);
            app.game.players[0].gear.push(GearState {
                id: 900,
                card_id: 18,
                ready: true,
                attached_to: None,
                temporary: false,
                token: true,
            });
            app.tray = super::Tray::Loadout;
            app.ui.input_override = Some((vec2(615.0, 760.0), false, false));
            app.hover_card = Some(18);
            app.hover_token = Some(super::TokenPreview {
                name: "Gold".into(),
                might: 0,
                gear: true,
                rules: super::GOLD_RULES.into(),
            });
            app.hover_since = -1.0;
        }
        263 => {
            let preview = app
                .hover_token
                .as_ref()
                .expect("Gold has its own rules preview");
            assert_eq!(preview.name, "Gold");
            assert!(
                preview.gear
                    && preview.rules.contains("kill this Gold")
                    && preview.rules.contains("1 Any power")
            );
            assert!(app.hover_rect.is_some());
            app.ui.input_override = Some((vec2(615.0, 760.0), false, true));
        }
        264 => {
            assert!(
                app.inspect.is_none(),
                "Gold never opens its source card's inspector"
            );
            app.hover_rect = None;
            click(app, 615.0, 760.0);
        }
        265 => {
            assert!(app.selection == Some(Selection::Gear(900)));
            click(app, 1285.0, 513.0);
        }
        266 => {
            assert!(app.game.players[0].gear.is_empty());
            assert_eq!(app.game.players[0].any_power, 1);
        }
        267 => {
            expansion_fixture(app, 188);
            app.game.players[0]
                .units
                .push(public_unit(100, 110, Some(0)));
            app.game.players[1]
                .units
                .push(public_unit(200, 110, Some(1)));
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerTargets { card_id: 188, .. })
            ));
            assert_eq!(app.game.trigger_target_bounds(), Some((1, 0)));
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
        }
        268 => {
            let source = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 188)
                .unwrap()
                .id;
            click_unit(app, source);
        }
        269 => {
            assert!(app.targets.is_empty(), "Windsinger cannot target itself");
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(format!("{:?}", app.game).as_str())
                )
            });
            click_unit(app, 200);
        }
        270 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 200
                }]
            );
            click_unit(app, 200);
        }
        271 => {
            assert!(
                app.targets.is_empty(),
                "clicking the target undoes the draft"
            );
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(format!("{:?}", app.game).as_str())
                )
            });
            click(app, 1285.0, 548.0);
        }
        272 => {
            assert!(app.game.decision.is_none());
            assert!(app.game.players[1].units.iter().any(|unit| unit.id == 200));
            assert!(!app.game.chain.iter().any(|item| item.card_id == 188));
            expansion_fixture(app, 188);
            app.game.players[1]
                .units
                .push(public_unit(200, 110, Some(1)));
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
        }
        273 => {
            click_unit(app, 200);
        }
        274 => {
            assert_eq!(app.targets.len(), 1);
            click(app, 1285.0, 510.0);
        }
        275 => {
            assert!(
                app.game
                    .chain
                    .iter()
                    .any(|item| item.card_id == 188 && item.targets.len() == 1)
            );
            finish_chain_until_choice(app);
            assert!(!app.game.players[1].units.iter().any(|unit| unit.id == 200));
            expansion_fixture(app, 190);
            app.game.players[0].units.push(public_unit(100, 110, None));
            app.game.players[1]
                .units
                .push(public_unit(200, 110, Some(1)));
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
            assert_eq!(app.game.trigger_target_bounds(), Some((2, 2)));
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
        }
        276 => {
            click_unit(app, 200);
        }
        277 => {
            assert!(
                app.targets.is_empty(),
                "Beast Below must choose its friendly slot first"
            );
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(format!("{:?}", app.game).as_str())
                )
            });
            click_unit(app, 100);
        }
        278 => {
            assert_eq!(app.targets.len(), 1);
            click(app, 1285.0, 510.0);
        }
        279 => {
            assert_eq!(
                app.targets.len(),
                1,
                "incomplete confirmation stays a draft"
            );
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(format!("{:?}", app.game).as_str())
                )
            });
            click_unit(app, 200);
        }
        280 => {
            assert_eq!(app.targets.len(), 2);
            capture(app, "RIFT_SMOKE_TRIGGER_SCREENSHOT");
            let before = format!("{:?}", app.game);
            assert!(
                app.game
                    .apply(Action::ChooseTriggerTargets {
                        targets: vec![Target::Unit {
                            player: 1,
                            unit_id: 200
                        }]
                    })
                    .is_err()
            );
            assert_eq!(
                format!("{:?}", app.game),
                before,
                "invalid target submission is atomic"
            );
            click(app, 1285.0, 548.0);
        }
        281 => {
            assert!(app.targets.is_empty());
            click_unit(app, 100);
        }
        282 => {
            click_unit(app, 200);
        }
        283 => {
            assert_eq!(app.targets.len(), 2);
            click(app, 1285.0, 510.0);
        }
        284 => {
            assert!(
                app.game
                    .chain
                    .iter()
                    .any(|item| item.card_id == 190 && item.targets.len() == 2)
            );
            finish_chain_until_choice(app);
            assert!(!app.game.players[0].units.iter().any(|unit| unit.id == 100));
            assert!(!app.game.players[1].units.iter().any(|unit| unit.id == 200));
            expansion_fixture(app, 110);
            app.game.players[0].units.push(public_unit(100, 110, None));
            app.game.players[0]
                .units
                .push(public_unit(101, 110, Some(0)));
            app.game.players[1]
                .units
                .push(public_unit(200, 69, Some(1)));
            for (owner, unit_id, gear_id, card_id) in [
                (0, 100, 800, 121),
                (0, 100, 801, 124),
                (0, 101, 802, 121),
                (1, 200, 803, 124),
            ] {
                app.game.players[owner].gear.push(GearState {
                    id: gear_id,
                    card_id,
                    ready: true,
                    attached_to: Some(unit_id),
                    temporary: false,
                    token: false,
                });
            }
            for owner in 0..2 {
                app.game.players[owner].runes = [0, 4, 0, 4, 0, 0];
                app.game.players[owner].exhausted_runes = [0, 2, 0, 2, 0, 0];
                app.game.players[owner].energy_pool = 2;
                app.game.players[owner].energy = 6;
                app.game.players[owner].power_pool = [1; 6];
                app.game.players[owner].any_power = 1;
                app.game.legend[owner].spell_power = 1;
            }
            app.hover_card = None;
            app.hover_rect = None;
            app.ui.input_override = Some((vec2(214.0, 100.0), false, false));
        }
        285 => {
            capture(app, "RIFT_SMOKE_MAT_SCREENSHOT");
            let r = unit_rect(app, 100);
            click(app, r.x + r.w - 10.0, r.y + r.h - 9.0);
        }
        286 => {
            assert_eq!(
                app.equipment_inspect,
                Some((0, 100)),
                "multiple attachments open a readable list"
            );
            capture(app, "RIFT_SMOKE_EQUIPMENT_SCREENSHOT");
            click(app, 967.0, 489.0);
        }
        287 => {
            assert_eq!(
                app.inspect,
                Some(124),
                "each attachment opens its full card text"
            );
            assert!(app.equipment_inspect.is_none());
            click(app, 960.0, 704.0);
        }
        288 => {
            assert!(app.inspect.is_none());
            click(app, 643.0, 273.0);
        }
        289 => {
            assert_eq!(app.field_inspect, Some(app.game.battlefields[0]));
            capture(app, "RIFT_SMOKE_FIELD_RULES_SCREENSHOT");
            click(app, 690.0, 502.0);
        }
        290 => {
            assert!(app.field_inspect.is_none());
        }
        291 => {
            app.in_match = false;
            app.screen = Screen::Decks;
            app.browse_deck = 0;
            app.selection = None;
            app.hover_card = None;
            app.hover_rect = None;
            click(app, 1300.0, 46.0);
        }
        292 => {
            assert!(app.legend_picker.is_some());
            click(app, 447.0, 466.0);
        }
        293 => {
            assert_eq!(app.legend_picker, Some(4));
            capture(app, "RIFT_SMOKE_LEGEND_PICKER_SCREENSHOT");
            click(app, 840.0, 700.0);
        }
        294 => {
            let editor = app.deck_editor.as_mut().unwrap();
            assert_eq!(editor.deck.legend, 4);
            assert_eq!(editor.deck.card_count(), 40);
            assert!(editor.deck.validate().is_ok());
            editor.deck.name = "Crimson practice".into();
            click(app, 328.0, 771.0);
        }
        295 => {
            assert_eq!(app.custom_decks[0].as_ref().unwrap().legend, 4);
            assert!(
                app.player_deck < 4,
                "custom legend never becomes a preset index"
            );
            assert_eq!(
                super::deck_editor::load_saved_decks()[0],
                app.custom_decks[0]
            );
            click(app, 509.0, 159.0);
        }
        296 => {
            assert_eq!(app.legend_picker, Some(4));
            click(app, 447.0, 519.0);
        }
        297 => {
            assert_eq!(app.legend_picker, Some(5));
            click(app, 840.0, 700.0);
        }
        298 => {
            assert_eq!(app.deck_editor.as_ref().unwrap().deck.legend, 5);
            assert!(app.deck_editor.as_ref().unwrap().deck.validate().is_ok());
            click(app, 462.0, 771.0);
        }
        299 => {
            assert_eq!(app.custom_decks[1].as_ref().unwrap().legend, 5);
            assert!(app.opponent_deck < 4);
            assert_eq!((app.setup_legend(0).id, app.setup_legend(1).id), (4, 5));
            click(app, 395.0, 816.0);
        }
        300 => {
            assert!(app.screen == Screen::Decks);
            click(app, 425.0, 800.0);
        }
        301 => {
            assert!(app.screen == Screen::Practice);
            app.toast_until = 0.0;
            capture(app, "RIFT_SMOKE_LEGEND_SETUP_SCREENSHOT");
            click(app, 1328.0, 46.0);
        }
        302 => {
            assert_eq!(app.game.phase, Phase::Mulligan);
            assert_eq!((app.game.legend_id(0), app.game.legend_id(1)), (4, 5));
            assert_eq!(app.game.players[0].champion, Some(183));
            assert_eq!(app.game.players[1].champion, Some(175));
            click(app, 1285.0, 510.0);
        }
        303 => {
            assert_eq!(app.game.phase, Phase::Playing);
            fixture_resources(app, Domain::Fury);
            hand_card(app, "Inferna");
            app.hand_scroll = 0;
            click(app, 315.0, 760.0);
        }
        304 => {
            assert!(matches!(app.selection, Some(Selection::Hand(0))));
            click(app, 1285.0, 510.0);
        }
        305 => {
            assert!(app.game.cards_played_this_turn(0) > 0);
            assert_eq!(app.game.players[0].energy, 4);
            click(app, 400.0, 663.0);
        }
        306 => {
            assert!(app.tray == super::Tray::Loadout);
            click(app, 315.0, 760.0);
        }
        307 => {
            assert_eq!(app.game.legend_id(0), 4);
            assert!(app.selection == Some(Selection::Legend));
            capture(app, "RIFT_SMOKE_DARIUS_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        308 => {
            assert_eq!(app.game.players[0].energy, 5);
            assert_eq!(app.game.players[0].energy_pool, 1);
            assert!(!app.game.legend[0].ready);
            assert!(app.game.chain.is_empty());
            assert_eq!(app.game.priority(), 0);
            click(app, 1328.0, 46.0);
        }
        309 => {
            assert!(app.confirm_restart);
            click(app, 854.0, 500.0);
        }
        310 => {
            assert!(!app.in_match);
            click(app, 1328.0, 46.0);
        }
        311 => {
            assert_eq!((app.game.legend_id(0), app.game.legend_id(1)), (4, 5));
            assert_eq!(app.game.custom_deck(0), app.custom_decks[0].as_ref());
            click(app, 1285.0, 510.0);
        }
        312 => {
            assert_eq!(app.game.phase, Phase::Playing);
            app.custom_decks.swap(0, 1);
            app.reset();
            app.game.start_match().unwrap();
            assert_eq!(app.game.legend_id(0), 5);
            app.tray = super::Tray::Loadout;
            click(app, 315.0, 760.0);
        }
        313 => {
            assert!(app.selection == Some(Selection::Legend));
            app.game.showdown = Some(local_rift::game::Showdown {
                lane: 0,
                attacker: 0,
                focus: 0,
                passes: 0,
                combat: false,
            });
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1285.0, 548.0);
        }
        314 => {
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(format!("{:?}", app.game).as_str()),
                    "Viktor cannot activate during showdown"
                )
            });
            app.game.showdown = None;
            app.toast_until = 0.0;
            capture(app, "RIFT_SMOKE_VIKTOR_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        315 => {
            assert_eq!(app.game.players[0].energy, 1);
            assert!(!app.game.legend[0].ready);
            assert!(app.game.chain.iter().any(|item| matches!(
                app.game.chain_source(item),
                local_rift::game::ChainSource::Legend { legend_id: 5, .. }
            )));
            finish_chain_until_choice(app);
        }
        316 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TokenPlacement { remaining: 1, .. })
            ));
            click(app, 1285.0, 470.0);
        }
        317 => {
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .any(|unit| unit.token && app.game.unit_name(unit) == "Recruit token")
            );
            app.in_match = false;
            app.screen = Screen::Decks;
            app.selection = None;
            app.hover_card = None;
            app.hover_rect = None;
            click(app, 1300.0, 46.0);
        }
        318 => {
            assert!(app.legend_picker.is_some());
            click(app, 447.0, 572.0);
        }
        319 => {
            assert_eq!(app.legend_picker, Some(6));
            click(app, 840.0, 700.0);
        }
        320 => {
            assert_eq!(app.deck_editor.as_ref().unwrap().deck.legend, 6);
            assert!(app.deck_editor.as_ref().unwrap().deck.validate().is_ok());
            click(app, 328.0, 771.0);
        }
        321 => {
            assert_eq!(app.custom_decks[0].as_ref().unwrap().legend, 6);
            click(app, 395.0, 816.0);
        }
        322 => {
            click(app, 425.0, 800.0);
        }
        323 => {
            click(app, 1328.0, 46.0);
        }
        324 => {
            assert_eq!(app.game.legend_id(0), 6);
            click(app, 1285.0, 510.0);
        }
        325 => {
            click(app, 400.0, 663.0);
        }
        326 => {
            click(app, 315.0, 760.0);
        }
        327 => {
            assert!(app.selection == Some(Selection::Legend));
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1285.0, 548.0);
        }
        328 => {
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(format!("{:?}", app.game).as_str()),
                    "Lux has no activated button"
                )
            });
            fixture_resources(app, Domain::Mind);
            hand_card(app, "Progress Day");
            app.deck_scroll = app.game.players[0].hand.len();
            app.hand_scroll = 0;
            click(app, 300.0, 663.0);
        }
        329 => {
            click(app, 315.0, 760.0);
        }
        330 => {
            click(app, 1285.0, 510.0);
        }
        331 => {
            assert!(
                app.game
                    .chain
                    .iter()
                    .any(|item| item.card_id == 77 && item.ability.is_none()),
                "Progress Day is on the chain first"
            );
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            assert!(app.game.chain.iter().any(|item| matches!(
                app.game.chain_source(item),
                local_rift::game::ChainSource::Legend { legend_id: 6, .. }
            )));
            app.chain_cursor = app
                .game
                .chain
                .iter()
                .rev()
                .position(|item| {
                    matches!(
                        app.game.chain_source(item),
                        local_rift::game::ChainSource::Legend { legend_id: 6, .. }
                    )
                })
                .unwrap();
            app.ui.input_override = Some((vec2(1285.0, 270.0), false, true));
        }
        332 => {
            assert_eq!(
                app.lesson,
                Some(6),
                "chain opens canonical legend instead of champion rules"
            );
            capture(app, "RIFT_SMOKE_LUX_SCREENSHOT");
            click(app, 760.0, 720.0);
        }
        333 => {
            assert!(app.lesson.is_none());
            finish_chain_until_choice(app);
            assert_eq!(
                app.game.players[0].hand.len(),
                app.deck_scroll + 4,
                "Progress Day draws4 and Lux adds1 after spending one card"
            );
            assert!(app.game.legend[0].ready);
            SAVED_DECK_SNAPSHOT.with(|saved| {
                *saved.borrow_mut() = std::fs::read(super::deck_editor::saved_path(0)).ok()
            });
            std::fs::write(super::deck_editor::saved_path(0), b"{broken deck").unwrap();
            let (decks, errors) = super::deck_editor::load_saved_decks_with_errors();
            assert!(decks[0].is_none() && errors.len() == 1);
            assert_eq!(
                std::fs::read(super::deck_editor::saved_path(0)).unwrap(),
                b"{broken deck"
            );
            app.deck_load_errors = errors;
        }
        334 => {
            assert!(!app.deck_load_errors.is_empty());
            capture(app, "RIFT_SMOKE_DECK_ERROR_SCREENSHOT");
            click(app, 720.0, 608.0);
        }
        335 => {
            assert!(app.deck_load_errors.is_empty());
            SAVED_DECK_SNAPSHOT.with(|saved| {
                std::fs::write(
                    super::deck_editor::saved_path(0),
                    saved.borrow().as_ref().unwrap(),
                )
                .unwrap()
            });
            assert_eq!(
                super::deck_editor::load_saved_decks()[0]
                    .as_ref()
                    .unwrap()
                    .legend,
                6
            );
            app.screen = Screen::DeckEditor;
            app.legend_picker = Some(6);
            COMBAT_SNAPSHOT.with(|snapshot| {
                *snapshot.borrow_mut() =
                    Some(app.deck_editor.as_ref().unwrap().deck.to_json().to_string())
            });
            click(app, 447.0, 254.0);
        }
        336 => {
            assert_eq!(app.legend_picker, Some(0));
            click(app, 447.0, 700.0);
        }
        337 => {
            assert!(app.legend_picker.is_none());
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(
                        app.deck_editor
                            .as_ref()
                            .unwrap()
                            .deck
                            .to_json()
                            .to_string()
                            .as_str()
                    ),
                    "canceling legend change preserves current edited list"
                )
            });
            app.ui.input_override = None;
            app.ui.pointer_override = None;
            app.ui.wheel_override = None;
            println!(
                "PASS: 338 native UI frames including seven-legend custom creation, saved identity and rematch, Darius energy, Viktor timing and Recruit, Lux passive chain, and visible saved-deck errors."
            );
            return true;
        }
        _ => panic!("UI smoke runner exceeded its 338-frame script"),
    }
    false
}
