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
    static ADD_CHAIN_SNAPSHOT: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    static SAVED_DECK_SNAPSHOT: std::cell::RefCell<Option<Vec<u8>>> = const { std::cell::RefCell::new(None) };
    static COMBAT_SNAPSHOT: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    static PREDICT_CHOICE: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static C20_SCORES: std::cell::Cell<[u8; 2]> = const { std::cell::Cell::new([0; 2]) };
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
            might: app
                .game
                .unit_might(unit)
                .expect("valid numeric smoke fixture"),
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

fn click_field(app: &mut App, lane: usize) {
    let r = app
        .drop_zones
        .iter()
        .find_map(|(zone, r)| {
            matches!(zone, super::DropZone::Battlefield(index) if *index == lane).then_some(*r)
        })
        .expect("battlefield surface");
    click(app, r.x + r.w * 0.8, r.y + r.h * 0.72);
}

fn capture_unit(app: &mut App, id: u32, variable: &str) {
    app.companion.observe(&app.game);
    let card = app.game.players[0]
        .units
        .iter()
        .find(|unit| unit.id == id)
        .unwrap()
        .card_id;
    app.hover_card = Some(card);
    app.hover_token = app.game.players[0]
        .units
        .iter()
        .find(|unit| unit.id == id && unit.token)
        .map(|unit| super::TokenPreview {
            name: app.game.unit_name(unit),
            might: app.game.unit_might(unit).unwrap(),
            gear: false,
            rules: super::token_rules(unit.card_id),
        });
    app.hover_object = Some((0, id));
    app.hover_since = get_time() - 1.0;
    if let Ok(path) = std::env::var(variable) {
        let rect = unit_rect(app, id);
        app.ui.input_override = Some((rect.center(), false, false));
        app.draw();
        get_screen_data().export_png(&path);
    }
}

fn capture_gear(app: &mut App, id: u32, variable: &str) {
    app.companion.observe(&app.game);
    let gear = app.game.players[0]
        .gear
        .iter()
        .find(|gear| gear.id == id)
        .unwrap();
    app.hover_card = Some(gear.card_id);
    app.hover_object = Some((0, id));
    app.hover_token = None;
    app.hover_rect = Some(Rect::new(776.0, 176.0, 350.0, 458.0));
    app.hover_since = get_time() - 1.0;
    app.hover_scroll = 0;
    if let Ok(path) = std::env::var(variable) {
        app.ui.input_override = Some((vec2(800.0, 450.0), false, false));
        app.draw();
        get_screen_data().export_png(&path);
    }
    app.hover_card = None;
    app.hover_object = None;
    app.hover_rect = None;
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
    app.banish_view = false;
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
    app.hover_object = None;
    app.hover_token = None;
    app.hover_object_candidate = None;
    app.toast_until = 0.0;
    app.payment_draft = None;
    app.play_draft = None;
    app.cost_editor = None;
    app.show_play_effects = false;
    app.suspended_payments.clear();
    app.resource_drafts.clear();
    app.ability_costs.clear();
    app.conversion_key = None;
    app.conversion_editor = None;
    app.payment_origin = None;
    app.prefs.exact_payment = false;
    app.drag = None;
    app.damage_amounts = None;
    app.damage_editor = None;
}
fn played_gear_fixture(app: &mut App, card_id: usize) {
    expansion_fixture(app, card_id);
    app.game
        .apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: None,
        })
        .unwrap();
    app.tray = super::Tray::Loadout;
}
fn save_state(app: &App) {
    COMBAT_SNAPSHOT.with(|value| *value.borrow_mut() = Some(format!("{:?}", app.game)));
}
fn assert_same_state(app: &App) {
    COMBAT_SNAPSHOT.with(|value| {
        assert_eq!(
            value.borrow().as_ref().unwrap(),
            &format!("{:?}", app.game),
            "UI draft or invalid choice must not mutate the game"
        )
    });
}
fn fixture_counter_tax(app: &mut App, unit_id: u32) {
    app.game.players[0].hand = vec![7];
    app.game
        .apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit { player: 0, unit_id }],
        })
        .unwrap();
    let chain_id = app.game.chain.last().unwrap().id;
    app.game.apply(Action::Pass).unwrap();
    app.game.players[1].hand = vec![37];
    app.game.players[1].runes = [0; 6];
    app.game.players[1].runes[domain_index(Domain::Chaos)] = 6;
    app.game.players[1].exhausted_runes = [0; 6];
    app.game.players[1].energy = 6;
    app.game.players[1].energy_pool = 0;
    app.game
        .apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Spell { chain_id }],
        })
        .unwrap();
    app.game.apply(Action::Pass).unwrap();
    app.game.apply(Action::Pass).unwrap();
    assert!(matches!(
        app.game.decision,
        Some(Decision::CounterTax {
            player: 0,
            cost: 2,
            ..
        })
    ));
}
fn anomaly_payment_fixture(app: &mut App, energy: u16, power: u16) {
    played_gear_fixture(app, 245);
    app.game.players[0].gear.push(GearState {
        id: 9902,
        card_id: 245,
        ready: true,
        attached_to: None,
        temporary: false,
        token: false,
    });
    app.game.players[0].units = vec![public_unit(9901, 239, Some(0))];
    app.game.battlefield_owner[0] = Some(0);
    app.game.players[0].runes = [0; 6];
    app.game.players[0].exhausted_runes = [0; 6];
    app.game.players[0].rune_deck = vec![Domain::Mind; 12];
    app.game.players[0].energy_pool = energy;
    app.game.players[0].energy = energy;
    app.game.players[0].power_pool = [0; 6];
    app.game.players[0].power_pool[domain_index(Domain::Mind)] = power;
    app.prefs.exact_payment = true;
    app.tray = super::Tray::Hand;
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

fn c17_fixture(app: &mut App, spell: usize) {
    expansion_fixture(app, 380);
    let legend = (0..local_rift::legend_catalog::legends().len())
        .find(|&id| local_rift::deck::eligible_cards(id).contains(&spell))
        .unwrap();
    let mut deck = local_rift::deck::CustomDeck::from_legend(legend).unwrap();
    deck.cards.push((spell, 1));
    app.game =
        Game::new_with_custom([0, 3], [Some(deck), None], 171700, Difficulty::Easy, false).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = [7, 9];
    app.game.battlefield_owner = [None; 2];
    app.fields = app.game.battlefields;
    for p in &mut app.game.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.discard.clear();
        p.banish.clear();
        p.runes = [0; 6];
        p.exhausted_runes = [0; 6];
        p.energy = 30;
        p.energy_pool = 30;
        p.power_pool = [5; 6];
    }
    app.game.players[0].hand = vec![spell];
    app.game.players[0].units = vec![public_unit(17001, 71, None)];
    app.game.players[1].units = vec![public_unit(17002, 70, Some(1))];
    app.game.legend[0].ready = false;
}
fn c17_pending(app: &mut App, spell: usize, lane: Option<usize>) {
    c17_fixture(app, spell);
    app.game.players[0].units[0].lane = lane;
    app.game
        .apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: 0,
                unit_id: 17001,
            }],
        })
        .unwrap();
    finish_chain_until_choice(app);
    assert!(matches!(
        app.game.decision,
        Some(Decision::PendingPlay { .. })
    ));
}

fn c18_offer(app: &mut App, card: usize) {
    c17_fixture(app, 384);
    app.game.players[0].units[0].card_id = 384;
    app.game.players[0].units[0].damage = 8;
    app.game.players[0].deck = vec![70, 71, card];
    app.game.players[0].hand = vec![133];
    app.game
        .apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        })
        .unwrap();
    finish_chain_until_choice(app);
    assert!(matches!(app.game.decision,Some(Decision::BanishPlay{card_id,..}) if card_id==card));
}

fn c19_look(app: &mut App, spell: usize, top_first: &[usize]) {
    c17_fixture(app, spell);
    app.deck_look_draft = None;
    app.game.players[0].deck = top_first.iter().rev().copied().collect();
    app.game
        .apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        })
        .unwrap();
    finish_chain_until_choice(app);
    assert!(matches!(
        app.game.decision,
        Some(Decision::DeckLookPlay { player: 0, .. })
    ));
}
fn c19_hook(app: &mut App) {
    c17_fixture(app, 385);
    app.deck_look_draft = None;
    app.game.players[0].hand.clear();
    app.game.players[0].units[0].card_id = 273;
    app.game.players[0].units[0].buff = 3;
    app.game.players[0].gear.push(GearState {
        id: 19001,
        card_id: 385,
        ready: true,
        temporary: false,
        token: false,
        attached_to: None,
    });
    app.game.players[0].deck = vec![96, 145, 96, 10, 133];
    app.game
        .apply(Action::ActivateAbility {
            source: local_rift::activated::AbilitySource::Gear {
                player: 0,
                gear_id: 19001,
            },
            ability_index: 0,
            targets: vec![Target::Unit {
                player: 0,
                unit_id: 17001,
            }],
        })
        .unwrap();
    finish_chain_until_choice(app);
    assert!(matches!(
        app.game.decision,
        Some(Decision::DeckLookPlay {
            player: 0,
            killed_might: Some(3),
            ..
        })
    ));
}

fn c20_look(app: &mut App, source: usize, top_first: &[usize], player: usize, repeat: bool) {
    c17_fixture(app, source);
    // The existing enemy at field 1 already controls it; do not stage an
    // unrelated conquest when the tested spell finishes resolving.
    app.game.battlefield_owner[1] = Some(1);
    app.draw_look_draft = None;
    if player == 1 {
        app.game.players[0].hand.clear();
        app.game.apply(Action::EndTurn).unwrap();
        finish_chain_until_choice(app);
        app.game.players[1].hand = vec![source];
        app.game.players[1].power_pool = [5; 6];
        app.game.players[1].energy = 30;
        app.game.players[1].energy_pool =
            30 - u16::from(app.game.players[1].runes.iter().sum::<u8>());
    }
    app.game.players[player].deck = top_first.iter().rev().copied().collect();
    app.game
        .apply(Action::PlayCardOptions {
            hand_index: 0,
            lane: None,
            targets: vec![],
            repeat_targets: vec![],
            repeat,
            accelerate: false,
            bonus: false,
        })
        .unwrap();
    finish_chain_until_choice(app);
    assert!(
        matches!(app.game.decision, Some(Decision::DrawLook { player: owner, .. }) if owner == player)
    );
}

fn c20_move(app: &mut App, source: usize, top: Option<usize>) {
    c17_fixture(app, source);
    app.game.players[0].hand.clear();
    app.game.players[0].units[0].card_id = source;
    app.game.players[0]
        .units
        .push(public_unit(20002, 96, Some(0)));
    app.game.battlefield_owner[0] = Some(0);
    app.game.players[0].deck = top.map_or_else(Vec::new, |card| vec![69, card]);
    app.game
        .apply(Action::Move {
            unit_id: 17001,
            lane: Some(0),
        })
        .unwrap();
    finish_chain_until_choice(app);
}

fn c21_fixture(app: &mut App, source: usize, top_first: &[usize]) {
    c17_fixture(app, source);
    app.game.players[1].units.clear();
    app.game.battlefield_owner = [None; 2];
    app.game.players[0].deck = top_first.iter().rev().copied().collect();
    app.game.players[0].xp = 0;
    app.predict_draft = None;
    app.draw_look_draft = None;
    app.inspect = None;
}

fn c21_cast_to_choice(app: &mut App, source: usize, top_first: &[usize]) {
    c21_fixture(app, source, top_first);
    app.game
        .apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: None,
        })
        .unwrap();
    finish_chain_until_choice(app);
}

fn c22_fixture(app: &mut App, card_id: usize) {
    c21_fixture(app, card_id, &[69, 71, 96]);
    app.game.players[0].units.clear();
    app.game.battlefields = [2, 2];
    app.fields = app.game.battlefields;
    app.selection = None;
    app.tray = super::Tray::Hand;
    app.inspect = None;
    app.hover_object = None;
    app.hover_token = None;
    app.hand_scroll = 0;
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
            app.hover_object = Some((0, app.game.players[0].units[0].id));
            app.ui.input_override = Some((vec2(315.0, 760.0), false, true));
        }
        19 => {
            assert!(app.inspect.is_some(), "right-click inspector");
            assert!(
                app.hover_object.is_none(),
                "hand inspection is printed-only"
            );
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
            app.hover_object = Some((0, 50_001));
            app.ui.input_override = Some((vec2(315.0, 760.0), false, true));
        }
        40 => {
            assert!(app.inspect.is_some(), "trash inspector");
            assert!(
                app.hover_object.is_none(),
                "trash inspection is printed-only"
            );
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
            app.hover_object = Some((1, 60_005));
            app.ui.input_override = Some((vec2(465.0, 760.0), false, true));
            app.draw();
            assert_eq!(app.hover_object, Some((1, 60_006)));
            assert_eq!(app.inspect, Some(25));
            assert_eq!(app.damage_order.first(), Some(&60_006));
            click(app, 960.0, 704.0);
            app.draw();
            assert!(app.inspect.is_none());
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
                    .expect("valid numeric smoke fixture")
                    .into_iter()
                    .find(|a| matches!(a, Action::Pass))
                    .unwrap_or_else(|| {
                        app.game
                            .legal_actions()
                            .expect("valid numeric smoke fixture")
                            .remove(0)
                    });
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
            app.damage_editor = Some((80_002, "18446744073709551616".into()));
            assert!(
                app.commit_damage_editor().is_err(),
                "out-of-range input is rejected without wrapping"
            );
            assert_eq!(
                app.damage_assignment(0, 3)
                    .expect("valid numeric smoke fixture"),
                vec![(80_002, 2), (80_003, 1)]
            );
            app.damage_editor = Some((80_002, "18446744073709551615".into()));
            app.commit_damage_editor().unwrap();
            assert_eq!(
                app.damage_assignment(0, 3)
                    .expect("valid numeric smoke fixture")[0]
                    .1,
                u64::MAX,
                "typed amounts are not silently clamped"
            );
            assert!(app.damage_validation(0, 3).is_err());
            app.set_damage_amount(80_002, 2);
            click(app, 266.0, 845.0); // Reduce first target below lethal.
        }
        108 => {
            assert_eq!(
                app.damage_assignment(0, 3)
                    .expect("valid numeric smoke fixture"),
                vec![(80_002, 1), (80_003, 1)]
            );
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
            assert_eq!(
                app.damage_assignment(0, 3)
                    .expect("valid numeric smoke fixture"),
                vec![(80_002, 1), (80_003, 2)]
            );
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
            assert_eq!(
                app.damage_assignment(0, 3)
                    .expect("valid numeric smoke fixture"),
                vec![(80_003, 2), (80_002, 1)]
            );
            assert!(app.damage_validation(0, 3).is_ok());
            assert_ne!(
                app.damage_assignment(0, 3)
                    .expect("valid numeric smoke fixture"),
                app.game
                    .automatic_damage_assignment(0, 0, 3)
                    .expect("valid numeric smoke fixture")
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
            assert!(app.game.players[0].units.iter().all(|unit| {
                app.game
                    .unit_might(unit)
                    .expect("valid numeric smoke fixture")
                    == 7
            }));
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
                12 - u16::from(app.game.catalog[98].cost) - 1
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
            app.hover_object = Some((0, unit_id));
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
            assert_eq!(app.hover_object, Some((0, unit_id)));
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
                app.game
                    .unit_might(unit)
                    .expect("valid numeric smoke fixture"),
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
            app.hover_object = None;
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
            app.hover_object = Some((0, 100));
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
            app.hover_object = None;
            let r = unit_rect(app, 100);
            app.ui.input_override = Some((vec2(r.x + r.w / 2.0, r.y + r.h - 9.0), false, false));
        }
        234 => {
            assert_eq!(
                app.hover_card,
                Some(121),
                "attached gear has its own readable hover"
            );
            assert_eq!(app.hover_object, Some((0, app.game.players[0].gear[0].id)));
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
            assert!(app.game.players[0].units.iter().all(|unit| {
                app.game
                    .unit_might(unit)
                    .expect("valid numeric smoke fixture")
                    == 4
            }));
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
                app.game
                    .unit_keywords(&app.game.players[0].units[0])
                    .expect("valid numeric smoke fixture")
                    .shield,
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
            assert!(
                !app.game
                    .can_decline_trash_play()
                    .expect("valid numeric smoke fixture")
            );
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
                app.game
                    .unit_might(
                        app.game.players[0]
                            .units
                            .iter()
                            .find(|unit| unit.id == 100)
                            .unwrap()
                    )
                    .expect("valid numeric smoke fixture"),
                4
            );
            assert_eq!(
                app.game
                    .unit_might(
                        app.game.players[0]
                            .units
                            .iter()
                            .find(|unit| unit.id == 101)
                            .unwrap()
                    )
                    .expect("valid numeric smoke fixture"),
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
            app.hover_object = Some((0, 900));
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
        }
        338 => {
            expansion_fixture(app, 230);
            app.game.players[1].units.push(public_unit(9000, 239, None));
            click(app, 315.0, 760.0);
        }
        339 => {
            click(app, 1285.0, 510.0);
        }
        340 => {
            assert!(
                app.game.players[0]
                    .gear
                    .iter()
                    .any(|gear| gear.card_id == 230 && gear.ready)
            );
            click(app, 400.0, 663.0);
        }
        341 => {
            click(app, 615.0, 760.0);
        }
        342 => {
            assert!(matches!(app.selection, Some(Selection::Gear(_))));
            click(app, 1285.0, 417.0);
        }
        343 => {
            assert!(matches!(
                app.selection,
                Some(Selection::Ability(_, Some(0)))
            ));
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
            click_unit(app, 9000);
        }
        344 => {
            assert_eq!(
                app.ability_targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 9000
                }],
                "Heart may choose an enemy"
            );
            capture(app, "RIFT_SMOKE_ABILITY_TARGET_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        345 => {
            assert!(matches!(app.selection, Some(Selection::Ability(_, None))));
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(format!("{:?}", app.game).as_str()),
                    "cancel does not exhaust or spend"
                )
            });
            click(app, 1285.0, 417.0);
        }
        346 => {
            assert!(app.ability_targets.is_empty());
            click_unit(app, 9000);
        }
        347 => {
            click(app, 1285.0, 510.0);
        }
        348 => {
            assert!(app.game.chain.iter().any(|item| matches!(
                item.ability,
                Some(local_rift::game::ChainAbility::Activated {
                    card_id: 230,
                    ability_index: 0,
                    ..
                })
            )));
            assert!(
                !app.game.players[0]
                    .gear
                    .iter()
                    .find(|gear| gear.card_id == 230)
                    .unwrap()
                    .ready
            );
            finish_chain_until_choice(app);
            assert_eq!(
                app.game.players[1]
                    .units
                    .iter()
                    .find(|unit| unit.id == 9000)
                    .unwrap()
                    .temporary_buff,
                3
            );
            click(app, 700.0, 663.0);
        }
        349 => {
            assert!(app.tray == super::Tray::Board);
            click(app, 345.0, 715.0);
        }
        350 => {
            assert!(matches!(
                app.selection,
                Some(Selection::Ability(
                    super::AbilitySource::Unit {
                        player: 1,
                        unit_id: 9000
                    },
                    None
                ))
            ));
            click(app, 1285.0, 417.0);
        }
        351 => {
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1285.0, 510.0);
        }
        352 => {
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(format!("{:?}", app.game).as_str()),
                    "opponent abilities are read only"
                )
            });
            expansion_fixture(app, 233);
            click(app, 315.0, 760.0);
        }
        353 => {
            click(app, 1285.0, 510.0);
        }
        354 => {
            let unit = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 233)
                .unwrap();
            assert!(unit.ready, "Arena Kingpin enters ready");
            let id = unit.id;
            click_unit(app, id);
        }
        355 => {
            click(app, 1340.0, 547.0);
        }
        356 => {
            assert!(matches!(app.selection, Some(Selection::Ability(_, None))));
            click(app, 1285.0, 417.0);
        }
        357 => {
            let id = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 233)
                .unwrap()
                .id;
            click_unit(app, id);
        }
        358 => {
            click(app, 1285.0, 510.0);
        }
        359 => {
            finish_chain_until_choice(app);
            let unit = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 233)
                .unwrap();
            assert!(!unit.ready);
            assert_eq!(unit.temporary_buff, 3, "source may target itself");
            expansion_fixture(app, 239);
            click(app, 315.0, 760.0);
        }
        360 => {
            click(app, 1285.0, 510.0);
        }
        361 => {
            let id = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 239)
                .unwrap()
                .id;
            click_unit(app, id);
        }
        362 => {
            click(app, 1340.0, 547.0);
        }
        363 => {
            assert!(matches!(app.selection, Some(Selection::Ability(_, None))));
            capture(app, "RIFT_SMOKE_RENATA_SCREENSHOT");
            click(app, 1285.0, 417.0);
        }
        364 => {
            assert!(matches!(
                app.selection,
                Some(Selection::Ability(_, Some(0)))
            ));
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1285.0, 510.0);
        }
        365 => {
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(format!("{:?}", app.game).as_str()),
                    "base-only Renata cannot pay for a battlefield ability"
                )
            });
            let unit = app.game.players[0]
                .units
                .iter_mut()
                .find(|unit| unit.card_id == 239)
                .unwrap();
            unit.lane = Some(0);
            unit.ready = true;
            app.game.battlefield_owner[0] = Some(0);
            app.game.players[0].power_pool[domain_index(Domain::Mind)] = 2;
            app.prefs.exact_payment = true;
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1285.0, 510.0);
        }
        366 => {
            assert!(app.payment_draft.is_some());
            assert_eq!(
                app.payment_draft.as_ref().unwrap().chosen.colored[domain_index(Domain::Mind)],
                1
            );
            capture(app, "RIFT_SMOKE_ABILITY_PAYMENT_SCREENSHOT");
            click(app, 480.0, 709.0);
        }
        367 => {
            assert!(app.payment_draft.is_none());
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(format!("{:?}", app.game).as_str()),
                    "ability payment cancellation is atomic"
                )
            });
            click(app, 1285.0, 510.0);
        }
        368 => {
            click(app, 936.0, 709.0);
        }
        369 => {
            assert!(app.payment_draft.is_none());
            assert_eq!(app.game.players[0].energy, 6);
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Mind)],
                1
            );
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .find(|unit| unit.card_id == 239)
                    .unwrap()
                    .ready,
                "draw ability does not exhaust"
            );
            let before = app.game.players[0].hand.len();
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].hand.len(), before + 1);
            app.prefs.exact_payment = false;
            let id = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 239)
                .unwrap()
                .id;
            click_unit(app, id);
        }
        370 => {
            click(app, 1340.0, 547.0);
        }
        371 => {
            click(app, 1285.0, 481.0);
        }
        372 => {
            assert!(matches!(
                app.selection,
                Some(Selection::Ability(_, Some(1)))
            ));
            click(app, 1285.0, 510.0);
        }
        373 => {
            assert_eq!(app.game.players[0].score, 0, "scoring waits for responses");
            assert!(app.game.chain.iter().any(|item| matches!(
                item.ability,
                Some(local_rift::game::ChainAbility::Activated {
                    card_id: 239,
                    ability_index: 1,
                    ..
                })
            )));
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].score, 1);
            assert!(
                !app.game.players[0]
                    .units
                    .iter()
                    .find(|unit| unit.card_id == 239)
                    .unwrap()
                    .ready
            );
            expansion_fixture(app, 236);
            click(app, 315.0, 760.0);
        }
        374 => {
            click(app, 1285.0, 510.0);
        }
        375 => {
            let source = app.game.players[0]
                .units
                .iter_mut()
                .find(|unit| unit.card_id == 236)
                .unwrap();
            source.ready = true;
            let id = source.id;
            fixture_resources(app, Domain::Body);
            app.game.players[0].exhausted_runes[domain_index(Domain::Body)] = 3;
            app.game.players[0].energy = 3;
            app.game.players[0].hand = vec![7];
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![Target::Unit {
                        player: 0,
                        unit_id: id,
                    }],
                })
                .unwrap();
            let chain_id = app.game.chain.last().unwrap().id;
            app.game.apply(Action::Pass).unwrap();
            app.game.players[1].hand = vec![37];
            app.game.players[1].runes = [0; 6];
            app.game.players[1].runes[domain_index(Domain::Chaos)] = 6;
            app.game.players[1].exhausted_runes = [0; 6];
            app.game.players[1].energy = 6;
            app.game.players[1].energy_pool = 0;
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![Target::Spell { chain_id }],
                })
                .unwrap();
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            assert!(matches!(
                app.game.decision,
                Some(Decision::CounterTax {
                    player: 0,
                    cost: 2,
                    ..
                })
            ));
            assert_eq!(app.game.players[0].energy, 1);
            click(app, 700.0, 663.0);
        }
        376 => {
            assert!(app.tray == super::Tray::Board);
            click(app, 345.0, 715.0);
        }
        377 => {
            assert!(matches!(app.selection, Some(Selection::Ability(_, None))));
            click(app, 1285.0, 417.0);
        }
        378 => {
            assert!(matches!(
                app.selection,
                Some(Selection::Ability(_, Some(0)))
            ));
            ADD_CHAIN_SNAPSHOT.with(|ids| {
                *ids.borrow_mut() = app.game.chain.iter().map(|item| item.id).collect()
            });
            capture(app, "RIFT_SMOKE_DRAGONSOUL_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        379 => {
            assert_eq!(app.game.players[0].energy, 2);
            assert_eq!(app.game.players[0].energy_pool, 1);
            assert_eq!(app.game.priority(), 0);
            ADD_CHAIN_SNAPSHOT.with(|ids| {
                assert_eq!(
                    *ids.borrow(),
                    app.game
                        .chain
                        .iter()
                        .map(|item| item.id)
                        .collect::<Vec<_>>(),
                    "Add has no response item"
                )
            });
            assert!(matches!(
                app.game.decision,
                Some(Decision::CounterTax { cost: 2, .. })
            ));
            assert!(
                !app.game.players[0]
                    .units
                    .iter()
                    .find(|unit| unit.card_id == 236)
                    .unwrap()
                    .ready
            );
            click(app, 1285.0, 506.0);
        }
        380 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[0].energy, 0);
            finish_chain_until_choice(app);
            assert_eq!(
                app.game.players[0]
                    .units
                    .iter()
                    .find(|unit| unit.card_id == 236)
                    .unwrap()
                    .temporary_buff,
                2,
                "tax payment preserves the spell"
            );
        }
        381 => {
            app.game = Game::new(3, 3, 934, Difficulty::Easy);
            app.game.start_match().unwrap();
            app.game.players[0].units = vec![public_unit(9001, 69, None)];
            app.game.players[1].units = vec![
                public_unit(9002, 238, Some(0)),
                public_unit(9003, 25, Some(0)),
                public_unit(9004, 25, Some(0)),
            ];
            app.game.players[1].gear.push(GearState {
                id: 9010,
                card_id: 121,
                ready: true,
                attached_to: Some(9002),
                temporary: false,
                token: false,
            });
            app.game.battlefield_owner[0] = Some(1);
            app.selection = None;
            app.targets.clear();
            app.ability_targets.clear();
            app.damage_order.clear();
            app.damage_amounts = None;
            app.damage_editor = None;
            app.hover_card = None;
            app.hover_rect = None;
            app.hand_scroll = 0;
            app.toast_until = 0.0;
            app.game
                .apply(Action::Move {
                    unit_id: 9001,
                    lane: Some(0),
                })
                .unwrap();
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            assert!(matches!(
                app.game.decision,
                Some(Decision::CombatDamage {
                    player: 0,
                    total: 8,
                    ..
                })
            ));
            assert!(
                app.damage_validation(0, 8).is_ok(),
                "automatic Tank choice remains legal"
            );
            click(app, 865.0, 663.0);
        }
        382 => {
            assert_eq!(
                app.damage_assignment(0, 8)
                    .expect("valid numeric smoke fixture"),
                vec![(9002, 4), (9003, 2), (9004, 2)]
            );
            click(app, 315.0, 750.0);
        }
        383 => {
            assert_eq!(
                app.damage_assignment(0, 8)
                    .expect("valid numeric smoke fixture"),
                vec![(9003, 2), (9004, 2), (9002, 4)]
            );
            assert!(
                app.damage_validation(0, 8).is_ok(),
                "Caitlyn plus Doran Shield may be assigned last"
            );
            capture(app, "RIFT_SMOKE_TANK_BACKLINE_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        384 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::CombatDamage { player: 1, .. })
            ));
            let Decision::CombatDamage {
                player,
                lane,
                total,
            } = app.game.decision.clone().unwrap()
            else {
                unreachable!()
            };
            app.game
                .apply(Action::AssignDamage {
                    assignments: app
                        .game
                        .automatic_damage_assignment(player, lane, total)
                        .expect("valid numeric smoke fixture"),
                })
                .unwrap();
            assert!(app.game.players[1].units.is_empty());
        }
        385 => {
            played_gear_fixture(app, 240);
            let trash = app.game.players[0].deck.drain(..3).collect::<Vec<_>>();
            app.game.players[0].discard = trash;
            click(app, 615.0, 760.0);
        }
        386 => {
            click(app, 1285.0, 417.0);
        }
        387 => {
            assert!(app.tray == super::Tray::Trash);
            assert!(app.ability_costs.is_empty());
            click(app, 315.0, 760.0);
        }
        388 => {
            assert_eq!(app.ability_costs, vec![0]);
            click(app, 465.0, 760.0);
        }
        389 => {
            assert_eq!(app.ability_costs, vec![0, 1]);
            click(app, 615.0, 760.0);
        }
        390 => {
            assert_eq!(app.ability_costs, vec![0, 1, 2]);
            assert!(
                app.ability_targets.is_empty(),
                "recycled costs are not targets"
            );
            COMBAT_SNAPSHOT.with(|value| *value.borrow_mut() = Some(format!("{:?}", app.game)));
            capture(app, "RIFT_SMOKE_TRASH_COST_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        391 => {
            COMBAT_SNAPSHOT.with(|value| {
                assert_eq!(value.borrow().as_ref().unwrap(), &format!("{:?}", app.game))
            });
            click(app, 1285.0, 417.0);
        }
        392 => {
            assert!(app.ability_costs.is_empty());
            click(app, 315.0, 760.0);
        }
        393 => {
            click(app, 465.0, 760.0);
        }
        394 => {
            click(app, 615.0, 760.0);
        }
        395 => {
            click(app, 1285.0, 510.0);
        }
        396 => {
            assert!(app.game.players[0].discard.is_empty());
            assert!(!app.game.players[0].gear[0].ready);
            assert!(app.game.chain.last().unwrap().targets.is_empty());
            let before = app.game.players[0].hand.len();
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].hand.len(), before + 1);
            played_gear_fixture(app, 241);
            let spell = app.game.players[0]
                .deck
                .iter()
                .position(|id| app.game.catalog[*id].kind == CardKind::Spell)
                .unwrap();
            let spell = app.game.players[0].deck.remove(spell);
            let unit = app.game.players[0]
                .deck
                .iter()
                .position(|id| app.game.catalog[*id].kind == CardKind::Unit)
                .unwrap();
            let unit = app.game.players[0].deck.remove(unit);
            app.game.players[0].discard = vec![spell, unit];
            click(app, 615.0, 760.0);
        }
        397 => {
            click(app, 1285.0, 417.0);
        }
        398 => {
            COMBAT_SNAPSHOT.with(|value| *value.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 315.0, 760.0);
        }
        399 => {
            assert!(
                app.ability_costs.is_empty(),
                "Rig cannot recycle a spell for its unit cost"
            );
            COMBAT_SNAPSHOT.with(|value| {
                assert_eq!(value.borrow().as_ref().unwrap(), &format!("{:?}", app.game))
            });
            click(app, 465.0, 760.0);
        }
        400 => {
            assert_eq!(app.ability_costs, vec![1]);
            app.prefs.exact_payment = true;
            click(app, 1285.0, 510.0);
        }
        401 => {
            assert!(app.payment_draft.is_some());
            click(app, 480.0, 709.0);
        }
        402 => {
            assert!(app.payment_draft.is_none());
            assert_eq!(app.ability_costs, vec![1]);
            COMBAT_SNAPSHOT.with(|value| {
                assert_eq!(value.borrow().as_ref().unwrap(), &format!("{:?}", app.game))
            });
            click(app, 1285.0, 510.0);
        }
        403 => {
            click(app, 936.0, 709.0);
        }
        404 => {
            assert_eq!(app.game.players[0].discard.len(), 1);
            assert_eq!(
                app.game.catalog[app.game.players[0].discard[0]].kind,
                CardKind::Spell
            );
            assert!(!app.game.players[0].gear[0].ready);
            finish_chain_until_choice(app);
            let mech = app.game.players[0]
                .units
                .iter()
                .find(|u| u.token && u.card_id == 241)
                .unwrap();
            assert_eq!(
                app.game
                    .unit_might(mech)
                    .expect("valid numeric smoke fixture"),
                3
            );
            assert_eq!(mech.lane, None);
            assert!(!mech.ready);
            played_gear_fixture(app, 242);
            finish_chain_until_choice(app);
            assert!(
                matches!(app.game.decision, Some(Decision::Predict { player: 0, .. })),
                "Shells Vision responds before predicting"
            );
            click(app, 1285.0, 547.0);
        }
        405 => {
            assert!(app.game.decision.is_none());
            app.game.players[1].units = vec![public_unit(9901, 69, None)];
            app.tray = super::Tray::Loadout;
            click(app, 615.0, 760.0);
        }
        406 => {
            click(app, 1285.0, 417.0);
        }
        407 => {
            let r = unit_rect(app, 9901);
            click(app, r.center().x, r.center().y);
        }
        408 => {
            click(app, 1285.0, 510.0);
        }
        409 => {
            assert!(
                app.game.players[0].gear.is_empty(),
                "Shells dies as a cost before responses"
            );
            assert!(app.game.players[0].discard.contains(&242));
            assert!(!app.game.chain.is_empty());
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[1].units[0].temporary_buff, 2);
            played_gear_fixture(app, 243);
            assert!(
                !app.game.players[0].gear[0].ready,
                "Formula enters exhausted"
            );
            app.game.players[0].gear[0].ready = true;
            app.game.players[1].units = vec![public_unit(9901, 69, None)];
            app.game.players[1].gear = vec![GearState {
                id: 9902,
                card_id: 121,
                ready: true,
                attached_to: Some(9901),
                temporary: false,
                token: false,
            }];
            click(app, 615.0, 760.0);
        }
        410 => {
            click(app, 1285.0, 417.0);
        }
        411 => {
            let id = app.game.players[0].gear[0].id;
            assert!(!app.ability_target_candidate(Target::Gear {
                player: 0,
                gear_id: id
            }));
            COMBAT_SNAPSHOT.with(|value| *value.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 615.0, 760.0);
        }
        412 => {
            assert!(app.ability_targets.is_empty());
            COMBAT_SNAPSHOT.with(|value| {
                assert_eq!(value.borrow().as_ref().unwrap(), &format!("{:?}", app.game))
            });
            app.toast_until = 0.0;
            click(app, 850.0, 663.0);
        }
        413 => {
            assert_eq!(app.gear_owner, 1);
            click(app, 615.0, 760.0);
        }
        414 => {
            assert_eq!(
                app.ability_targets,
                vec![Target::Gear {
                    player: 1,
                    gear_id: 9902
                }]
            );
            capture(app, "RIFT_SMOKE_FORMULA_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        415 => {
            assert!(!app.game.players[0].gear[0].ready);
            finish_chain_until_choice(app);
            assert!(app.game.unit_empowered(9902));
            assert!(
                !app.game.unit_empowered(9901),
                "the gear's bearer is not empowered"
            );
            assert_eq!(app.game.players[1].units[0].temporary_buff, 0);
            played_gear_fixture(app, 244);
            app.game.players[0].units = vec![public_unit(9901, 236, None)];
            fixture_resources(app, Domain::Body);
            app.game.players[0].exhausted_runes[domain_index(Domain::Body)] = 2;
            app.game.players[0].energy = 4;
            fixture_counter_tax(app, 9901);
            assert_eq!(app.game.players[0].energy, 2);
            app.tray = super::Tray::Loadout;
            click(app, 615.0, 760.0);
        }
        416 => {
            click(app, 1285.0, 417.0);
        }
        417 => {
            click(app, 1285.0, 510.0);
        }
        418 => {
            app.draw();
            assert!(matches!(
                app.game.decision,
                Some(Decision::ResourceConversion { card_id: 244, .. })
            ));
            assert!(!app.game.players[0].gear[0].ready);
            assert_eq!(app.game.conversion_available(), Some(2));
            app.conversion_amount = 3;
            COMBAT_SNAPSHOT.with(|value| *value.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1285.0, 510.0);
        }
        419 => {
            COMBAT_SNAPSHOT.with(|value| {
                assert_eq!(value.borrow().as_ref().unwrap(), &format!("{:?}", app.game))
            });
            app.toast_until = 0.0;
            app.conversion_amount = 1;
            capture(app, "RIFT_SMOKE_CONVERSION_SCREENSHOT");
            click(app, 700.0, 663.0);
        }
        420 => {
            click(app, 345.0, 715.0);
        }
        421 => {
            click(app, 1285.0, 417.0);
        }
        422 => {
            click(app, 1285.0, 510.0);
        }
        423 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::ResourceConversion { card_id: 244, .. })
            ));
            assert_eq!(
                app.conversion_amount, 1,
                "nested Add restores the chosen outer amount"
            );
            assert_eq!(app.game.conversion_available(), Some(3));
            assert!(!app.game.players[0].units[0].ready);
            click(app, 1285.0, 510.0);
        }
        424 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::CounterTax {
                    player: 0,
                    cost: 2,
                    ..
                })
            ));
            assert_eq!(app.game.players[0].energy, 2);
            assert_eq!(app.game.players[0].any_power, 1);
            click(app, 1285.0, 506.0);
        }
        425 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[0].energy, 0);
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].units[0].temporary_buff, 2);
            assert!(app.suspended_payments.is_empty());
            assert!(app.resource_drafts.is_empty());
        }
        426 => {
            anomaly_payment_fixture(app, 2, 3);
            let r = unit_rect(app, 9901);
            click(app, r.center().x, r.center().y);
        }
        427 => {
            click(app, 1340.0, 547.0);
        }
        428 => {
            click(app, 1285.0, 417.0);
        }
        429 => {
            click(app, 1285.0, 510.0);
        }
        430 => {
            assert!(app.payment_draft.is_some());
            assert!(app.game.payment_plan_view(0).is_none());
            click(app, 885.0, 595.0);
        }
        431 => {
            assert!(app.payment_draft.is_none());
            let plan = app.game.payment_plan_view(0).unwrap();
            assert_eq!(plan.energy, 1);
            assert_eq!(plan.power[0].amount, 1);
            assert!(matches!(
                app.game.decision,
                Some(Decision::Payment { player: 0 })
            ));
            click(app, 615.0, 760.0);
        }
        432 => {
            click(app, 1285.0, 417.0);
        }
        433 => {
            click(app, 1285.0, 510.0);
        }
        434 => {
            app.draw();
            assert!(matches!(
                app.game.decision,
                Some(Decision::ResourceConversion { card_id: 245, .. })
            ));
            assert_eq!(app.game.conversion_available(), Some(3));
            let before = format!("{:?}", app.game);
            app.conversion_editor = Some("-1".into());
            assert!(app.commit_conversion_editor().is_err());
            app.conversion_editor = Some("18446744073709551616".into());
            assert!(app.commit_conversion_editor().is_err());
            assert_eq!(
                format!("{:?}", app.game),
                before,
                "malformed conversion input never pays"
            );
            app.conversion_editor = Some("1".into());
            app.commit_conversion_editor().unwrap();
            click(app, 1285.0, 510.0);
        }
        435 => {
            assert_eq!(
                app.payment_draft.as_ref().unwrap().chosen.colored[domain_index(Domain::Mind)],
                1
            );
            capture(app, "RIFT_SMOKE_ANOMALY_PAYMENT_SCREENSHOT");
            click(app, 885.0, 595.0);
        }
        436 => {
            assert_eq!(app.suspended_payments.len(), 2);
            assert!(app.game.payment_plan_view(0).is_some());
            click(app, 765.0, 760.0);
        }
        437 => {
            click(app, 1285.0, 417.0);
        }
        438 => {
            click(app, 1285.0, 510.0);
        }
        439 => {
            app.draw();
            assert!(matches!(
                app.game.decision,
                Some(Decision::ResourceConversion {
                    source: local_rift::activated::AbilitySource::Gear { gear_id: 9902, .. },
                    ..
                })
            ));
            assert_eq!(app.conversion_amount, 0);
            assert!(app.game.players[0].gear.iter().all(|gear| !gear.ready));
            click(app, 1285.0, 548.0);
        }
        440 => {
            assert!(
                matches!(app.game.decision,Some(Decision::ResourceConversion{source:local_rift::activated::AbilitySource::Gear{gear_id,..},..}) if gear_id!=9902)
            );
            assert_eq!(app.conversion_amount, 1);
            assert_eq!(app.game.players[0].energy, 2);
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Mind)],
                3
            );
            click(app, 896.0, 106.0);
        }
        441 => {
            assert!(matches!(
                app.payment_draft.as_ref().unwrap().action,
                Action::ResolveConversion { amount: 1 }
            ));
            click(app, 936.0, 709.0);
        }
        442 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::Payment { player: 0 })
            ));
            assert_eq!(app.game.players[0].energy, 3);
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Mind)],
                2
            );
            assert_eq!(app.suspended_payments.len(), 1);
            click(app, 896.0, 106.0);
        }
        443 => {
            let draft = app.payment_draft.as_ref().unwrap();
            assert!(matches!(draft.action, Action::CompletePayment));
            assert_eq!(
                draft.chosen.colored[domain_index(Domain::Mind)],
                1,
                "original source choice survives nested Add"
            );
            capture(app, "RIFT_SMOKE_FROZEN_PAYMENT_SCREENSHOT");
            click(app, 480.0, 709.0);
        }
        444 => {
            assert!(app.payment_draft.is_none());
            assert!(matches!(
                app.game.decision,
                Some(Decision::Payment { player: 0 })
            ));
            assert!(
                !app.game
                    .can_cancel_payment()
                    .expect("valid numeric smoke fixture"),
                "a legal begun payment must complete"
            );
            click(app, 1285.0, 510.0);
        }
        445 => {
            assert!(app.payment_draft.is_some());
            click(app, 936.0, 709.0);
        }
        446 => {
            assert!(app.game.payment_plan_view(0).is_none());
            assert!(app.suspended_payments.is_empty());
            assert!(app.payment_origin.is_none());
            assert_eq!(app.game.players[0].energy, 2);
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Mind)],
                1
            );
            assert!(app.game.players[0].units[0].ready);
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].hand.len(), 1);
            anomaly_payment_fixture(app, 0, 0);
            let r = unit_rect(app, 9901);
            click(app, r.center().x, r.center().y);
        }
        447 => {
            click(app, 1340.0, 547.0);
        }
        448 => {
            click(app, 1285.0, 417.0);
        }
        449 => {
            COMBAT_SNAPSHOT.with(|value| *value.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1285.0, 510.0);
        }
        450 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::Payment { player: 0 })
            ));
            assert!(
                app.game
                    .can_cancel_payment()
                    .expect("valid numeric smoke fixture")
            );
            click(app, 615.0, 760.0);
        }
        451 => {
            click(app, 1285.0, 417.0);
        }
        452 => {
            click(app, 1285.0, 510.0);
        }
        453 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::ResourceConversion { card_id: 245, .. })
            ));
            assert!(!app.game.players[0].gear[0].ready);
            assert_eq!(app.game.conversion_available(), Some(0));
            click(app, 1285.0, 548.0);
        }
        454 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::Payment { player: 0 })
            ));
            assert!(
                app.game
                    .can_cancel_payment()
                    .expect("valid numeric smoke fixture")
            );
            assert!(!app.game.players[0].gear[0].ready);
            click(app, 1285.0, 548.0);
        }
        455 => {
            assert!(app.game.payment_plan_view(0).is_none());
            assert!(
                app.game.players[0].gear[0].ready,
                "rollback undoes in-payment Add costs"
            );
            COMBAT_SNAPSHOT.with(|value| {
                assert_eq!(value.borrow().as_ref().unwrap(), &format!("{:?}", app.game))
            });
            played_gear_fixture(app, 246);
            finish_chain_until_choice(app);
            app.prefs.exact_payment = true;
        }
        456 => {
            assert_eq!(
                app.game.players[0].hand.len(),
                1,
                "Snax's play trigger draws first"
            );
            click(app, 615.0, 760.0);
        }
        457 => {
            click(app, 1285.0, 417.0);
        }
        458 => {
            save_state(app);
            click(app, 1285.0, 548.0);
        }
        459 => {
            assert_same_state(app);
            click(app, 1285.0, 417.0);
        }
        460 => {
            click(app, 1285.0, 510.0);
        }
        461 => {
            assert!(app.payment_draft.is_some());
            click(app, 480.0, 709.0);
        }
        462 => {
            assert_same_state(app);
            click(app, 1285.0, 510.0);
        }
        463 => {
            click(app, 936.0, 709.0);
        }
        464 => {
            assert!(
                app.game.players[0].gear.is_empty(),
                "Snax is killed as a cost"
            );
            assert_eq!(
                app.game.players[0].hand.len(),
                1,
                "activation has not resolved yet"
            );
            assert!(app.game.players[0].discard.contains(&246));
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].hand.len(), 2);
            played_gear_fixture(app, 250);
            click(app, 615.0, 760.0);
        }
        465 => {
            assert_eq!(
                app.source_abilities(local_rift::activated::AbilitySource::Gear {
                    player: 0,
                    gear_id: app.game.players[0].gear[0].id
                })
                .len(),
                2
            );
            click(app, 1285.0, 481.0);
        }
        466 => {
            save_state(app);
            click(app, 1285.0, 510.0);
        }
        467 => {
            assert_same_state(app);
            assert!(app.game.chain.is_empty());
            click(app, 1285.0, 548.0);
        }
        468 => {
            click(app, 1285.0, 417.0);
        }
        469 => {
            assert!(app.ability_targets.is_empty());
            click(app, 1285.0, 510.0);
        }
        470 => {
            let id = app.game.players[0].gear[0].id;
            assert!(!app.game.players[0].gear[0].ready);
            assert!(
                !app.game.unit_empowered(id),
                "Empower waits for its response window"
            );
            assert!(app.game.chain.last().unwrap().targets.is_empty());
            finish_chain_until_choice(app);
            assert!(app.game.unit_empowered(id));
            app.game.players[0].gear[0].ready = true;
            click(app, 615.0, 760.0);
        }
        471 => {
            click(app, 1285.0, 417.0);
        }
        472 => {
            save_state(app);
            click(app, 1285.0, 510.0);
        }
        473 => {
            assert_same_state(app);
            click(app, 1285.0, 548.0);
        }
        474 => {
            click(app, 1285.0, 481.0);
        }
        475 => {
            capture(app, "RIFT_SMOKE_EMPOWER_TOME_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        476 => {
            assert!(
                !app.game.unit_empowered(app.game.players[0].gear[0].id),
                "Disempower is paid before responses"
            );
            assert!(!app.game.players[0].gear[0].ready);
            assert!(app.game.players[0].hand.is_empty());
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].hand.len(), 1);
            played_gear_fixture(app, 251);
            click(app, 615.0, 760.0);
        }
        477 => {
            click(app, 1285.0, 417.0);
        }
        478 => {
            click(app, 1285.0, 510.0);
        }
        479 => {
            finish_chain_until_choice(app);
            assert!(app.game.unit_empowered(app.game.players[0].gear[0].id));
            app.game.players[0].gear[0].ready = true;
            click(app, 615.0, 760.0);
        }
        480 => {
            click(app, 1285.0, 481.0);
        }
        481 => {
            click(app, 1285.0, 510.0);
        }
        482 => {
            assert!(!app.game.unit_empowered(app.game.players[0].gear[0].id));
            assert!(!app.game.players[0].gear[0].ready);
            finish_chain_until_choice(app);
            let mech = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.token)
                .unwrap();
            assert_eq!(app.game.unit_name(mech), "Mech token");
            assert_eq!(
                app.game
                    .unit_might(mech)
                    .expect("valid numeric smoke fixture"),
                3
            );
            assert_eq!(mech.lane, None);
            played_gear_fixture(app, 253);
            app.game.players[0].units = vec![public_unit(9901, 18, None)];
            click(app, 615.0, 760.0);
        }
        483 => {
            click(app, 1285.0, 417.0);
        }
        484 => {
            click(app, 1285.0, 510.0);
        }
        485 => {
            assert!(
                app.game.players[0].gear[0].ready,
                "Tools' Empower has no Exhaust cost"
            );
            assert_eq!(app.game.players[0].energy, 6);
            finish_chain_until_choice(app);
            assert!(app.game.unit_empowered(app.game.players[0].gear[0].id));
            click(app, 615.0, 760.0);
        }
        486 => {
            capture(app, "RIFT_SMOKE_EMPOWER_TOOLS_SCREENSHOT");
            click(app, 1285.0, 481.0);
        }
        487 => {
            click_unit(app, 9901);
        }
        488 => {
            assert_eq!(
                app.ability_targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: 9901
                }]
            );
            click(app, 1285.0, 510.0);
        }
        489 => {
            assert!(!app.game.players[0].gear[0].ready);
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].units[0].temporary_buff, 4);
            played_gear_fixture(app, 252);
            assert!(!app.game.players[0].gear[0].ready, "Egg enters exhausted");
            app.game.players[0].gear[0].ready = true;
            click(app, 615.0, 760.0);
        }
        490 => {
            click(app, 1285.0, 481.0);
        }
        491 => {
            click(app, 1285.0, 510.0);
        }
        492 => {
            assert_eq!(app.game.players[0].energy, 10);
            assert!(!app.game.players[0].gear[0].ready);
            assert!(app.game.chain.is_empty(), "Egg Add has no response window");
            played_gear_fixture(app, 252);
            app.game.players[0].gear[0].ready = true;
            click(app, 615.0, 760.0);
        }
        493 => {
            click(app, 1285.0, 417.0);
        }
        494 => {
            click(app, 1285.0, 510.0);
        }
        495 => {
            assert_eq!(app.game.players[0].energy, 8);
            finish_chain_until_choice(app);
            assert!(app.game.unit_empowered(app.game.players[0].gear[0].id));
            app.game.players[0].gear[0].ready = true;
            app.game.players[0].units = vec![public_unit(9901, 18, None)];
            fixture_counter_tax(app, 9901);
            app.game.players[0].exhausted_runes = app.game.players[0].runes;
            app.game.players[0].energy = 0;
            app.game.players[0].energy_pool = 0;
            app.selection = None;
            app.tray = super::Tray::Loadout;
            click(app, 615.0, 760.0);
        }
        496 => {
            click(app, 1285.0, 417.0);
        }
        497 => {
            save_state(app);
            click(app, 1285.0, 510.0);
        }
        498 => {
            assert_same_state(app);
            click(app, 1285.0, 548.0);
        }
        499 => {
            click(app, 1285.0, 481.0);
        }
        500 => {
            capture(app, "RIFT_SMOKE_EMPOWER_EGG_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        501 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::CounterTax { cost: 2, .. })
            ));
            assert_eq!(app.game.players[0].energy, 2);
            assert!(!app.game.players[0].gear[0].ready);
            assert!(
                app.game.unit_empowered(app.game.players[0].gear[0].id),
                "Add keeps Empowered"
            );
            click(app, 1285.0, 510.0);
        }
        502 => {
            assert_eq!(app.game.players[0].energy, 0);
            finish_chain_until_choice(app);
            assert!(app.game.decision.is_none());
            played_gear_fixture(app, 247);
            let mut moving = public_unit(9901, 18, Some(0));
            moving.ready = false;
            app.game.players[0].units = vec![moving, public_unit(9902, 18, None)];
            app.game.players[1].units = vec![public_unit(9903, 18, Some(1))];
            app.game.battlefield_owner = [Some(0), Some(1)];
            click(app, 615.0, 760.0);
        }
        503 => {
            click(app, 1285.0, 417.0);
        }
        504 => {
            save_state(app);
            click_unit(app, 9902);
        }
        505 => {
            assert_same_state(app);
            assert!(app.ability_targets.is_empty());
            click_unit(app, 9903);
        }
        506 => {
            assert_same_state(app);
            assert!(app.ability_targets.is_empty());
            click_unit(app, 9901);
        }
        507 => {
            assert_eq!(
                app.ability_targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: 9901
                }]
            );
            click(app, 1285.0, 510.0);
        }
        508 => {
            assert!(!app.game.players[0].gear[0].ready);
            assert_eq!(app.game.players[0].energy, 9);
            finish_chain_until_choice(app);
            let moved = &app.game.players[0].units[0];
            assert_eq!(moved.lane, None);
            assert!(!moved.ready, "Syren moves, without readying");
            expansion_fixture(app, 248);
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
            app.game.players[0].units[0].ready = true;
            app.game.players[1].units = vec![public_unit(9901, 18, None)];
            let id = app.game.players[0].units[0].id;
            click_unit(app, id);
        }
        509 => {
            click(app, 1340.0, 547.0);
        }
        510 => {
            click(app, 1285.0, 417.0);
        }
        511 => {
            click_unit(app, 9901);
        }
        512 => {
            assert_eq!(
                app.ability_targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 9901
                }]
            );
            click(app, 1285.0, 510.0);
        }
        513 => {
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[1].units[0].buff, 1);
            assert_eq!(app.game.players[1].units[0].temporary_buff, 0);
            app.game.players[0].units[0].ready = true;
            let id = app.game.players[0].units[0].id;
            click_unit(app, id);
        }
        514 => {
            click(app, 1340.0, 547.0);
        }
        515 => {
            click(app, 1285.0, 417.0);
        }
        516 => {
            click_unit(app, 9901);
        }
        517 => {
            click(app, 1285.0, 510.0);
        }
        518 => {
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[1].units[0].buff, 1);
            assert_eq!(app.game.players[1].units[0].temporary_buff, 2);
            expansion_fixture(app, 249);
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
            app.game.players[0].units[0].ready = true;
            let id = app.game.players[0].units[0].id;
            click_unit(app, id);
        }
        519 => {
            click(app, 1340.0, 547.0);
        }
        520 => {
            click(app, 1285.0, 417.0);
        }
        521 => {
            save_state(app);
            click(app, 1285.0, 510.0);
        }
        522 => {
            assert_same_state(app);
            app.game.players[0].units[0].lane = Some(0);
            app.game.battlefield_owner[0] = Some(0);
            click(app, 1285.0, 510.0);
        }
        523 => {
            assert!(!app.game.players[0].units[0].ready);
            assert!(app.game.players[0].units.iter().all(|unit| !unit.token));
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TokenPlacement {
                    card_id: 249,
                    remaining: 2,
                    ..
                })
            ));
            assert!(app.game.chain.is_empty());
            capture(app, "RIFT_SMOKE_EMPOWER_BIRDS_SCREENSHOT");
            click(app, 1285.0, 469.0);
        }
        524 => {
            let birds: Vec<_> = app.game.players[0]
                .units
                .iter()
                .filter(|unit| unit.token)
                .collect();
            assert_eq!(birds.len(), 1);
            assert_eq!(birds[0].lane, None);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TokenPlacement { remaining: 1, .. })
            ));
            click(app, 1285.0, 504.0);
        }
        525 => {
            let birds: Vec<_> = app.game.players[0]
                .units
                .iter()
                .filter(|unit| unit.token)
                .collect();
            assert_eq!(birds.len(), 2);
            assert_eq!(birds[1].lane, Some(0));
            for bird in birds {
                assert_eq!(app.game.unit_name(bird), "Bird token");
                assert_eq!(
                    app.game
                        .unit_might(bird)
                        .expect("valid numeric smoke fixture"),
                    1
                );
                assert_eq!(
                    app.game
                        .unit_keywords(bird)
                        .expect("valid numeric smoke fixture")
                        .deflect,
                    1
                );
            }
            played_gear_fixture(app, 250);
            app.game
                .empowered_units
                .push(app.game.players[0].gear[0].id);
            app.game.players[0].gear[0].ready = false;
            app.hover_candidate = Some(250);
            app.hover_card = Some(250);
            app.hover_object = Some((0, app.game.players[0].gear[0].id));
            app.hover_since = -1.0;
            app.hover_rect = Some(Rect::new(776.0, 176.0, 350.0, 458.0));
            app.ui.input_override = Some((vec2(800.0, 420.0), false, false));
        }
        526 => {
            assert_eq!(app.hover_object, Some((0, app.game.players[0].gear[0].id)));
            let text =
                super::live_gear_description(&app.game, 0, app.game.players[0].gear[0].id).unwrap();
            assert!(
                text.contains("Exhausted")
                    && text.contains("EMPOWERED")
                    && text.contains("Disempower source")
            );
            app.ui.input_override = Some((vec2(800.0, 420.0), false, false));
            if let Ok(path) = std::env::var("RIFT_SMOKE_EMPOWER_HOVER_SCREENSHOT") {
                app.draw();
                get_screen_data().export_png(&path);
            }
            app.ui.input_override = None;
            app.ui.pointer_override = None;
            app.ui.wheel_override = None;
        }
        527 => {
            app.in_match = false;
            app.screen = Screen::Practice;
            app.hover_card = None;
            app.hover_rect = None;
            app.field_inspect = None;
            app.fields = [0, 1];
            click(app, 450.0, 876.0);
        }
        528 => {
            assert!(app.field_picker.is_some());
            click(app, 480.0, 616.0);
        }
        529 => {
            assert_eq!(app.field_picker.as_ref().unwrap().selected, 10);
            assert_eq!(app.fields, [0, 1], "preview does not change setup");
            capture(app, "RIFT_SMOKE_C4_FIELDS_SCREENSHOT");
            click(app, 480.0, 715.0);
        }
        530 => {
            assert!(app.field_picker.is_none());
            assert_eq!(app.fields, [0, 1], "cancel preserves setup");
            click(app, 450.0, 876.0);
        }
        531 => {
            click(app, 480.0, 400.0);
        }
        532 => {
            assert_eq!(app.field_picker.as_ref().unwrap().selected, 4);
            click(app, 870.0, 715.0);
        }
        533 => {
            assert_eq!(app.fields, [4, 1]);
            assert!(app.field_picker.is_none());
            expansion_fixture(app, 260);
            app.game.players[0].any_power = 4;
            app.game.players[0].gear = vec![
                GearState {
                    id: 9901,
                    card_id: 256,
                    attached_to: None,
                    ready: true,
                    temporary: false,
                    token: false,
                },
                GearState {
                    id: 9902,
                    card_id: 256,
                    attached_to: None,
                    ready: true,
                    temporary: false,
                    token: false,
                },
            ];
            for id in [9901, 9902] {
                app.game
                    .apply(Action::ActivateAbility {
                        source: local_rift::activated::AbilitySource::Gear {
                            player: 0,
                            gear_id: id,
                        },
                        ability_index: 0,
                        targets: vec![],
                    })
                    .unwrap();
                finish_chain_until_choice(app);
                assert!(app.game.decision.is_none());
            }
            assert_eq!(app.game.play_grants.len(), 2);
            app.game.players[1].units = vec![
                public_unit(9911, 25, Some(0)),
                public_unit(9912, 25, Some(0)),
                public_unit(9913, 25, Some(1)),
            ];
            app.game.battlefield_owner = [Some(1), Some(1)];
            save_state(app);
            click(app, 315.0, 760.0);
        }
        534 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            click_unit(app, 9911);
        }
        535 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 9911
                }]
            );
            click(app, 1285.0, 468.0);
        }
        536 => {
            assert!(app.play_draft.as_ref().unwrap().open);
            click(app, 450.0, 739.0);
        }
        537 => {
            assert_eq!(app.play_draft.as_ref().unwrap().choices.repeats.len(), 1);
            click_unit(app, 9912);
        }
        538 => {
            assert_eq!(
                app.play_draft.as_ref().unwrap().choices.repeats[0]
                    .choices
                    .targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 9912
                }]
            );
            click(app, 450.0, 789.0);
        }
        539 => {
            assert_eq!(app.play_draft.as_ref().unwrap().choices.repeats.len(), 2);
            click_unit(app, 9913);
        }
        540 => {
            assert_eq!(
                app.play_draft.as_ref().unwrap().choices.repeats[1]
                    .choices
                    .targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 9913
                }]
            );
            assert_same_state(app);
            capture(app, "RIFT_SMOKE_C4_REPEAT_SCREENSHOT");
            click(app, 744.0, 731.0);
        }
        541 => {
            assert_eq!(app.play_draft.as_ref().unwrap().execution, 0);
            assert_eq!(
                app.current_targets(),
                &[Target::Unit {
                    player: 1,
                    unit_id: 9911
                }]
            );
            click(app, 1280.0, 509.0);
        }
        542 => {
            let spell = app
                .game
                .chain
                .iter()
                .find(|item| item.card_id == 260 && item.ability.is_none())
                .unwrap();
            assert_eq!(spell.executions.len(), 3);
            for (index, id) in [9911, 9912, 9913].into_iter().enumerate() {
                assert_eq!(
                    spell.executions[index].targets,
                    vec![Target::Unit {
                        player: 1,
                        unit_id: id
                    }]
                );
            }
            assert!(
                app.game.play_grants.is_empty(),
                "next-spell grants consumed by one play"
            );
            assert_eq!(
                app.game.cards_played_this_turn(0),
                1,
                "Repeat is not an extra card play"
            );
            finish_chain_until_choice(app);
            assert!(app.game.decision.is_none());
            expansion_fixture(app, 81);
            app.game.players[0].units = vec![public_unit(9921, 271, None)];
            app.game.players[0].any_power = 2;
            app.prefs.exact_payment = true;
            save_state(app);
            click(app, 315.0, 760.0);
        }
        543 => {
            click_unit(app, 9921);
        }
        544 => {
            assert!(app.play_draft.is_some());
            click(app, 1280.0, 509.0);
        }
        545 => {
            assert!(app.cost_editor.is_some());
            assert_same_state(app);
            capture(app, "RIFT_SMOKE_C4_COST_SCREENSHOT");
            click(app, 410.0, 752.0);
        }
        546 => {
            assert!(app.cost_editor.is_none());
            assert_same_state(app);
            click(app, 1280.0, 509.0);
        }
        547 => {
            assert!(app.cost_editor.is_some());
            click(app, 808.0, 332.0);
        }
        548 => {
            assert_same_state(app);
            click(app, 925.0, 752.0);
        }
        549 => {
            assert!(app.cost_editor.is_none());
            assert!(
                app.payment_draft.is_none(),
                "Power reduction eliminates the power payment"
            );
            assert_eq!(
                app.game.players[0].energy, 10,
                "chosen Power reduction retains the full two-energy cost"
            );
            assert_eq!(
                app.game.players[0].any_power, 2,
                "chosen reduction spends no generated Power"
            );
            assert_eq!(app.game.chain.last().unwrap().card_id, 81);
            expansion_fixture(app, 87);
            app.game.players[0].units = vec![public_unit(9931, 265, None)];
            app.game.players[0].any_power = 4;
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
            finish_chain_until_choice(app);
            assert!(
                matches!(
                    app.game.decision,
                    Some(Decision::TokenPlay {
                        remaining: 4,
                        fixed_lane: None,
                        ..
                    })
                ),
                "Expected token optional-cost choice, got {:?}",
                app.game.decision
            );
            save_state(app);
            click(app, 1284.0, 458.0);
        }
        550 => {
            assert!(app.options.accelerate);
            assert_same_state(app);
            capture(app, "RIFT_SMOKE_C4_TOKEN_SCREENSHOT");
            click(app, 1284.0, 492.0);
        }
        551 => {
            assert_eq!(
                app.game.players[0]
                    .units
                    .iter()
                    .filter(|unit| unit.token && unit.ready)
                    .count(),
                1
            );
            assert!(matches!(
                app.game.decision,
                Some(Decision::TokenPlay { remaining: 3, .. })
            ));
            assert!(!app.options.accelerate);
            click(app, 1284.0, 492.0);
        }
        552 => {
            assert_eq!(
                app.game.players[0]
                    .units
                    .iter()
                    .filter(|unit| unit.token && !unit.ready)
                    .count(),
                1
            );
            click(app, 1284.0, 492.0);
        }
        553 => {
            click(app, 1284.0, 492.0);
        }
        554 => {
            assert!(app.game.decision.is_none());
            assert_eq!(
                app.game.players[0]
                    .units
                    .iter()
                    .filter(|unit| unit.token)
                    .count(),
                4
            );
            assert_eq!(app.game.players[0].any_power, 3);
            app.game.play_grants.push(local_rift::play::PlayGrant {
                id: 9999,
                player: 0,
                source: local_rift::game::ChainSource::Card { card_id: 261 },
                expires_turn: app.game.turn,
                kind: local_rift::play::PlayGrantKind::ReadyBuffNextUnit,
            });
            click(app, 1110.0, 50.0);
        }
        555 => {
            assert!(app.show_play_effects);
            capture(app, "RIFT_SMOKE_C4_EFFECTS_SCREENSHOT");
            click(app, 700.0, 720.0);
        }
        556 => {
            assert!(!app.show_play_effects);
            expansion_fixture(app, 50);
            app.game.players[0].units =
                vec![public_unit(9941, 267, None), public_unit(9942, 271, None)];
            app.game.empowered_units.push(9941);
            save_state(app);
            click(app, 315.0, 760.0);
        }
        557 => {
            click_unit(app, 9942);
        }
        558 => {
            click(app, 1284.0, 468.0);
        }
        559 => {
            assert!(app.play_draft.as_ref().unwrap().open);
            click(app, 450.0, 739.0);
        }
        560 => {
            assert_eq!(app.play_draft.as_ref().unwrap().execution, 1);
            click(app, 793.0, 814.0);
        }
        561 => {
            assert_eq!(
                app.play_draft.as_ref().unwrap().choices.repeats[0]
                    .choices
                    .targets,
                app.targets
            );
            click(app, 1044.0, 701.0);
        }
        562 => {
            assert!(app.cost_editor.is_some());
            let plan = app
                .game
                .cost_plan(&app.canonical_action(app.selected_play_action().unwrap()))
                .unwrap();
            assert_eq!(
                plan.energy, 0,
                "Researchers floor applies before later Irelia reduction"
            );
            click(app, 1065.0, 387.0);
        }
        563 => {
            click(app, 940.0, 440.0);
        }
        564 => {
            click(app, 1036.0, 422.0);
        }
        565 => {
            click(app, 955.0, 680.0);
        }
        566 => {
            capture(app, "RIFT_SMOKE_C4_ALLOCATION_SCREENSHOT");
            click(app, 925.0, 752.0);
        }
        567 => {
            assert!(app.cost_editor.is_none());
            let draft = app.play_draft.as_ref().unwrap();
            let plan = app
                .game
                .cost_plan(&app.canonical_action(app.selected_play_action().unwrap()))
                .unwrap();
            assert_eq!(
                plan.energy, 1,
                "Irelia first leaves Researchers minimum at one"
            );
            let modifier = plan
                .modifiers
                .iter()
                .find(|modifier| {
                    modifier.source == local_rift::game::ChainSource::Card { card_id: 267 }
                })
                .unwrap();
            let selected = draft
                .choices
                .cost_order
                .iter()
                .find(|selected| selected.modifier_id == modifier.id)
                .unwrap();
            assert_eq!(selected.power_allocations.len(), 1);
            assert_eq!(
                selected.power_allocations[0].component_id, plan.components[1].id,
                "Power reduction explicitly assigned to Repeat component"
            );
            assert_same_state(app);
            click(app, 1284.0, 509.0);
        }
        568 => {
            assert_eq!(app.game.players[0].energy, 11);
            assert_eq!(app.game.chain.last().unwrap().executions.len(), 2);
            expansion_fixture(app, 283);
            app.game.players[0].deck.extend([18, 20]);
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
            finish_chain_until_choice(app);
            let id = app.game.players[0].units[0].id;
            click_unit(app, id);
        }
        569 => {
            click(app, 1340.0, 547.0);
        }
        570 => {
            click(app, 1285.0, 417.0);
        }
        571 => {
            assert!(app.ability_targets.is_empty());
            click(app, 1285.0, 510.0);
        }
        572 => {
            assert!(!app.game.unit_empowered(app.game.players[0].units[0].id));
            finish_chain_until_choice(app);
            let Some(Decision::PredictCards {
                player: 0,
                choice_id,
                ref cards,
            }) = app.game.decision
            else {
                panic!("Apprentice must Predict after Empower");
            };
            assert_eq!(cards, &vec![20, 18]);
            PREDICT_CHOICE.with(|id| id.set(choice_id));
            assert!(app.game.unit_empowered(app.game.players[0].units[0].id));
            save_state(app);
            app.sync_predict_draft();
            capture(app, "RIFT_SMOKE_C5_PREDICT_SCREENSHOT");
            click(app, 935.0, 826.0);
        }
        573 => {
            assert_same_state(app);
            assert_eq!(app.predict_draft.as_ref().unwrap().top_order, vec![1, 0]);
            click(app, 365.0, 858.0);
        }
        574 => {
            assert_same_state(app);
            assert_eq!(app.predict_draft.as_ref().unwrap().recycle, vec![0]);
            assert_eq!(app.predict_draft.as_ref().unwrap().top_order, vec![1]);
            click(app, 1285.0, 548.0);
        }
        575 => {
            assert_same_state(app);
            assert!(app.predict_draft.as_ref().unwrap().recycle.is_empty());
            assert_eq!(app.predict_draft.as_ref().unwrap().top_order, vec![0, 1]);
            click(app, 935.0, 826.0);
        }
        576 => {
            click(app, 1285.0, 510.0);
        }
        577 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[0].deck.last(), Some(&18));
            assert_eq!(app.game.players[0].deck.iter().rev().nth(1), Some(&20));
            finish_chain_until_choice(app);
            app.game.empowered_units.clear();
            app.game.players[0].deck.extend([18, 18]);
            let id = app.game.players[0].units[0].id;
            app.game
                .apply(Action::ActivateAbility {
                    source: local_rift::activated::AbilitySource::Unit {
                        player: 0,
                        unit_id: id,
                    },
                    ability_index: 0,
                    targets: vec![],
                })
                .unwrap();
            finish_chain_until_choice(app);
            app.sync_predict_draft();
            let draft = app.predict_draft.as_ref().unwrap();
            PREDICT_CHOICE.with(|id| assert_ne!(draft.choice_id, id.get()));
            assert_eq!(draft.top_order, vec![0, 1]);
            save_state(app);
            let old_id = PREDICT_CHOICE.with(|id| id.get());
            assert!(
                app.game
                    .apply(Action::ResolvePredict {
                        choice_id: old_id,
                        recycle: vec![],
                        top_order: vec![0, 1]
                    })
                    .is_err()
            );
            assert_same_state(app);
            let id = app.predict_draft.as_ref().unwrap().choice_id;
            assert!(
                app.game
                    .apply(Action::ResolvePredict {
                        choice_id: id,
                        recycle: vec![0],
                        top_order: vec![0]
                    })
                    .is_err()
            );
            assert_same_state(app);
            click(app, 365.0, 858.0);
        }
        578 => {
            click(app, 605.0, 858.0);
        }
        579 => {
            assert!(app.predict_draft.as_ref().unwrap().top_order.is_empty());
            assert_eq!(app.predict_draft.as_ref().unwrap().recycle, vec![0, 1]);
            assert_same_state(app);
            click(app, 1285.0, 510.0);
        }
        580 => {
            assert!(app.game.decision.is_none());
            finish_chain_until_choice(app);
            assert_eq!(&app.game.players[0].deck[..2], &[18, 18]);
            let id = app.game.players[0].units[0].id;
            app.hover_card = Some(283);
            app.hover_object = Some((0, id));
            app.hover_since = get_time() - 1.0;
            if let Ok(path) = std::env::var("RIFT_SMOKE_C5_EMPOWER_SCREENSHOT") {
                let r = unit_rect(app, id);
                app.ui.input_override = Some((r.center(), false, false));
                app.draw();
                get_screen_data().export_png(&path);
            }
            expansion_fixture(app, 278);
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
            finish_chain_until_choice(app);
            fixture_resources(app, Domain::Calm);
            let id = app.game.players[0].units[0].id;
            click_unit(app, id);
        }
        581 => {
            click(app, 1340.0, 547.0);
        }
        582 => {
            click(app, 1285.0, 417.0);
        }
        583 => {
            let Some(Selection::Ability(source, Some(index))) = app.selection else {
                panic!("Empower cost control");
            };
            let plan = app
                .game
                .cost_plan(&Action::ActivateAbility {
                    source,
                    ability_index: index,
                    targets: vec![],
                })
                .unwrap();
            assert_eq!(plan.energy, 6);
            save_state(app);
            capture(app, "RIFT_SMOKE_C5_COST_SCREENSHOT");
            click(app, 1354.0, 548.0);
        }
        584 => {
            assert!(app.cost_editor.is_some());
            capture(app, "RIFT_SMOKE_C5_COST_DETAIL_SCREENSHOT");
            click(app, 430.0, 752.0);
        }
        585 => {
            assert!(app.cost_editor.is_none());
            assert_same_state(app);
            let domain = Domain::Calm;
            app.game
                .apply(Action::RecycleRune {
                    domain,
                    exhausted: false,
                })
                .unwrap();
            let Some(Selection::Ability(source, Some(index))) = app.selection else {
                panic!("source retained");
            };
            let plan = app
                .game
                .cost_plan(&Action::ActivateAbility {
                    source,
                    ability_index: index,
                    targets: vec![],
                })
                .unwrap();
            assert_eq!(plan.energy, 7);
            save_state(app);
            click(app, 1285.0, 510.0);
        }
        586 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::Payment { player: 0 })
            ));
            assert_eq!(app.game.payment_plan_view(0).unwrap().energy, 7);
            click(app, 1285.0, 548.0);
        }
        587 => {
            assert_same_state(app);
            app.game.players[0].energy_pool = 7;
            app.game.players[0].energy = 12;
            click(app, 1285.0, 510.0);
        }
        588 => {
            assert_eq!(app.game.players[0].energy, 5);
            finish_chain_until_choice(app);
            assert!(app.game.unit_empowered(app.game.players[0].units[0].id));
            app.in_match = false;
            app.legend_picker = Some(0);
            click(app, 447.0, 625.0);
        }
        589 => {
            assert_eq!(app.legend_picker, Some(7));
            capture(app, "RIFT_SMOKE_C5_LEGEND_PICKER_SCREENSHOT");
            click(app, 840.0, 700.0);
        }
        590 => {
            assert_eq!(app.deck_editor.as_ref().unwrap().deck.legend, 7);
            assert_eq!(app.deck_editor.as_ref().unwrap().deck.card_count(), 40);
            click(app, 328.0, 771.0);
        }
        591 => {
            assert_eq!(app.custom_decks[0].as_ref().unwrap().legend, 7);
            assert!(app.player_deck < 4);
            app.reset();
            assert_eq!(app.game.legend_id(0), 7);
            app.game.start_match().unwrap();
            app.game.players[0].units = vec![public_unit(9901, 275, None)];
            app.game.players[0].units[0].ready = false;
            fixture_resources(app, Domain::Body);
            app.game
                .apply(Action::ActivateAbility {
                    source: local_rift::activated::AbilitySource::Unit {
                        player: 0,
                        unit_id: 9901,
                    },
                    ability_index: 0,
                    targets: vec![],
                })
                .unwrap();
            finish_chain_until_choice(app);
            assert!(
                app.game.legend[0].empowered,
                "Matriarch observes another object's Empower"
            );
            app.screen = Screen::Practice;
            app.tray = super::Tray::Loadout;
            click(app, 315.0, 760.0);
        }
        592 => {
            assert!(matches!(app.selection, Some(Selection::Legend)));
            assert!(app.targeting_selection());
            save_state(app);
            click(app, 1285.0, 548.0);
        }
        593 => {
            assert_same_state(app);
            app.game.players[1].units.push(public_unit(9902, 279, None));
            app.game.players[1].units.last_mut().unwrap().ready = false;
            app.game.empowered_units.push(9902);
            save_state(app);
            assert!(app.ability_target_candidate(Target::Unit {
                player: 1,
                unit_id: 9902
            }));
            click_unit(app, 9902);
        }
        594 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 9902
                }]
            );
            let action = Action::ActivateLegend {
                target: app.targets.first().copied(),
            };
            let plan = app.game.cost_plan(&action).unwrap();
            assert_eq!(
                plan.power.iter().map(|cost| cost.amount).sum::<u16>(),
                2,
                "Any power plus enemy Deflect"
            );
            assert_same_state(app);
            capture(app, "RIFT_SMOKE_C5_LEGEND_ENEMY_SCREENSHOT");
            click_unit(app, 9902);
        }
        595 => {
            assert!(app.targets.is_empty());
            assert_same_state(app);
            click_unit(app, 9901);
        }
        596 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: 9901
                }]
            );
            capture(app, "RIFT_SMOKE_C5_LEGEND_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        597 => {
            assert!(!app.game.legend[0].ready);
            assert!(!app.game.legend[0].empowered);
            assert!(
                !app.game.players[0].units[0].ready,
                "ready effect waits for responses"
            );
            finish_chain_until_choice(app);
            assert!(app.game.players[0].units[0].ready);
            app.reset();
            assert_eq!(app.game.legend_id(0), 7);
            assert_eq!(app.game.players[0].champion, Some(289));
        }
        598 => {
            expansion_fixture(app, 299);
            app.game.battlefields = [0, 0];
            app.game.players[0].units.clear();
            app.game.players[1].units.clear();
            save_state(app);
            click(app, 315.0, 760.0);
        }
        599 => {
            assert!(app.targeting_battlefield());
            assert_eq!(
                app.game
                    .card_target_candidates(0, 299, None, &[])
                    .expect("valid numeric smoke fixture"),
                vec![
                    Target::Battlefield { lane: 0 },
                    Target::Battlefield { lane: 1 }
                ]
            );
            let mut automatic = app.game.clone();
            automatic
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
            assert!(matches!(
                automatic.chain.last().unwrap().targets.as_slice(),
                [Target::Battlefield { .. }]
            ));
            assert!(automatic.chain.last().unwrap().lane.is_none());
            let mut poor = app.game.clone();
            poor.players[0].runes = [0; 6];
            poor.players[0].energy = 0;
            assert_eq!(
                poor.card_target_candidates(0, 299, None, &[])
                    .expect("valid numeric smoke fixture")
                    .len(),
                2,
                "target choice is independent of affordability"
            );
            click(app, 600.0, 540.0);
        }
        600 => {
            assert!(
                app.targets.is_empty() && app.destination.is_none(),
                "base is not a battlefield target"
            );
            assert_same_state(app);
            click_field(app, 0);
        }
        601 => {
            assert_eq!(app.targets, vec![Target::Battlefield { lane: 0 }]);
            assert!(app.destination.is_none());
            assert!(app.target_hint(299).contains("Field 1"));
            click_field(app, 1);
        }
        602 => {
            assert_eq!(app.targets, vec![Target::Battlefield { lane: 1 }]);
            assert!(app.target_hint(299).contains("Field 2"));
            assert_same_state(app);
            capture(app, "RIFT_SMOKE_C8_FIELD_SCREENSHOT");
            click_field(app, 1);
        }
        603 => {
            assert!(app.targets.is_empty());
            assert!(
                app.game
                    .apply(Action::PlayCardWithTargets {
                        hand_index: 0,
                        lane: None,
                        targets: vec![]
                    })
                    .is_err()
            );
            assert_same_state(app);
            click_field(app, 1);
        }
        604 => {
            click(app, 1090.0, 274.0);
        }
        605 => {
            assert_eq!(app.field_inspect, Some(0));
            assert_eq!(app.targets, vec![Target::Battlefield { lane: 1 }]);
            click(app, 690.0, 503.0);
        }
        606 => {
            assert!(app.field_inspect.is_none());
            assert_same_state(app);
            app.prefs.exact_payment = true;
            click(app, 1285.0, 508.0);
        }
        607 => {
            assert!(app.payment_draft.is_some());
            assert_same_state(app);
            click(app, 470.0, 709.0);
        }
        608 => {
            assert!(app.payment_draft.is_none());
            assert_eq!(app.targets, vec![Target::Battlefield { lane: 1 }]);
            assert_same_state(app);
            click(app, 1285.0, 508.0);
        }
        609 => {
            assert!(app.payment_draft.is_some());
            click(app, 935.0, 709.0);
        }
        610 => {
            let spell = app
                .game
                .chain
                .iter()
                .find(|item| item.card_id == 299 && item.ability.is_none())
                .unwrap();
            assert_eq!(spell.targets, vec![Target::Battlefield { lane: 1 }]);
            assert!(spell.lane.is_none());
            finish_chain_until_choice(app);
            assert!(app.game.players[0].discard.contains(&299));
            expansion_fixture(app, 299);
            app.game.battlefields = [0, 0];
            app.game.players[0].any_power = 4;
            app.game.players[0].gear.push(GearState {
                id: 9950,
                card_id: 256,
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            });
            app.game
                .apply(Action::ActivateAbility {
                    source: local_rift::activated::AbilitySource::Gear {
                        player: 0,
                        gear_id: 9950,
                    },
                    ability_index: 0,
                    targets: vec![],
                })
                .unwrap();
            finish_chain_until_choice(app);
            assert_eq!(app.game.play_grants.len(), 1);
            app.game.players[1].units = vec![
                public_unit(9951, 110, Some(0)),
                public_unit(9952, 110, Some(1)),
            ];
            app.game.battlefield_owner = [Some(1), Some(1)];
            save_state(app);
            click(app, 315.0, 760.0);
        }
        611 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            click_field(app, 0);
        }
        612 => {
            assert!(app.play_draft.is_some());
            assert_eq!(app.targets, vec![Target::Battlefield { lane: 0 }]);
            click(app, 1285.0, 468.0);
        }
        613 => {
            assert!(app.play_draft.as_ref().unwrap().open);
            click(app, 450.0, 739.0);
        }
        614 => {
            assert_eq!(app.play_draft.as_ref().unwrap().choices.repeats.len(), 1);
            click_unit(app, 9952);
        }
        615 => {
            assert_eq!(
                app.current_targets(),
                &[Target::Battlefield { lane: 1 }],
                "clicking an occupant chooses its field, not that unit"
            );
            assert_eq!(app.targets, vec![Target::Battlefield { lane: 0 }]);
            assert_same_state(app);
            capture(app, "RIFT_SMOKE_C8_REPEAT_SCREENSHOT");
            click(app, 1035.0, 701.0);
        }
        616 => {
            assert!(app.cost_editor.is_some());
            click(app, 420.0, 752.0);
        }
        617 => {
            assert!(app.cost_editor.is_none());
            assert_eq!(app.current_targets(), &[Target::Battlefield { lane: 1 }]);
            assert_same_state(app);
            click(app, 744.0, 731.0);
        }
        618 => {
            assert_eq!(app.current_targets(), &[Target::Battlefield { lane: 0 }]);
            click(app, 845.0, 731.0);
        }
        619 => {
            assert_eq!(app.current_targets(), &[Target::Battlefield { lane: 1 }]);
            app.prefs.exact_payment = true;
            click(app, 1285.0, 508.0);
        }
        620 => {
            assert!(app.payment_draft.is_some());
            assert_same_state(app);
            click(app, 470.0, 709.0);
        }
        621 => {
            assert!(app.payment_draft.is_none());
            assert_eq!(app.current_targets(), &[Target::Battlefield { lane: 1 }]);
            assert_same_state(app);
            click(app, 1285.0, 508.0);
        }
        622 => {
            assert!(app.payment_draft.is_some());
            click(app, 935.0, 709.0);
        }
        623 => {
            let spell = app
                .game
                .chain
                .iter()
                .find(|item| item.card_id == 299 && item.ability.is_none())
                .unwrap();
            assert_eq!(spell.executions.len(), 2);
            assert_eq!(
                spell.executions[0].targets,
                vec![Target::Battlefield { lane: 0 }]
            );
            assert_eq!(
                spell.executions[1].targets,
                vec![Target::Battlefield { lane: 1 }]
            );
            assert!(spell.lane.is_none());
            assert_eq!(super::chain_field_targets(spell), vec![0, 1]);
            capture(app, "RIFT_SMOKE_C8_CHAIN_SCREENSHOT");
            finish_chain_until_choice(app);
            assert!(
                app.game.players[1].units.is_empty(),
                "independent executions reach both fields"
            );
            expansion_fixture(app, 299);
            save_state(app);
            click(app, 315.0, 760.0);
            app.ui.pointer_override = Some((true, false));
        }
        624 | 625 => {
            assert!(app.drag.is_some());
            let rect = app
                .drop_zones
                .iter()
                .find_map(|(zone, rect)| {
                    matches!(zone, super::DropZone::Battlefield(0)).then_some(*rect)
                })
                .unwrap();
            app.ui.input_override = Some((
                vec2(rect.x + rect.w * 0.8, rect.y + rect.h * 0.72),
                false,
                false,
            ));
            app.ui.pointer_override = Some((frame == 624, frame == 625));
        }
        626 => {
            assert!(app.drag.is_none());
            assert_eq!(app.targets, vec![Target::Battlefield { lane: 0 }]);
            assert!(app.destination.is_none());
            assert_same_state(app);
            click(app, 315.0, 760.0);
            app.ui.pointer_override = Some((true, false));
        }
        627 | 628 => {
            assert!(app.drag.is_some());
            app.ui.input_override = Some((vec2(600.0, 540.0), false, false));
            app.ui.pointer_override = Some((frame == 627, frame == 628));
        }
        629 => {
            assert!(app.drag.is_none());
            assert_same_state(app);
            expansion_fixture(app, 299);
            app.game.players[0].units = vec![
                public_unit(9961, 304, None),
                public_unit(9962, 306, None),
                public_unit(9963, 307, None),
            ];
            app.game.players[1].units = vec![public_unit(9964, 110, Some(0))];
            app.game.battlefield_owner[0] = Some(1);
            assert!(
                super::live_unit_description(&app.game, 0, 9961)
                    .unwrap()
                    .contains("INACTIVE · Crimson Pigeons")
            );
            app.game
                .apply(Action::MoveGroup {
                    unit_ids: vec![9961, 9962, 9963],
                    lane: Some(0),
                })
                .unwrap();
            assert!(app.game.in_combat(0));
        }
        630 => {
            let text = super::live_unit_description(&app.game, 0, 9961).unwrap();
            assert!(text.contains("6 current might"));
            assert!(text.contains("ACTIVE · Crimson Pigeons: +2 might"));
            assert!(text.contains("ACTIVE · Captain Farron: Assault 1"));
            assert!(text.contains("ACTIVE · Taric, Protector: Shield 1"));
            assert!(text.contains("source's location"));
            capture_unit(app, 9961, "RIFT_SMOKE_C8_PASSIVE_SCREENSHOT");
        }
        631 => {
            expansion_fixture(app, 299);
            app.game.players[0].units = vec![public_unit(9971, 305, None)];
            app.game.players[1].units = vec![public_unit(9972, 110, Some(1))];
            app.game.battlefield_owner[1] = Some(1);
            app.game
                .apply(Action::Move {
                    unit_id: 9971,
                    lane: Some(1),
                })
                .unwrap();
            assert!(app.game.in_combat(1));
            let text = super::live_unit_description(&app.game, 0, 9971).unwrap();
            assert!(
                text.contains("4 current might")
                    && text.contains("ACTIVE · Wielder of Water: +2 might")
            );
            assert!(text.contains("while attacking or defending alone"));
            capture_unit(app, 9971, "RIFT_SMOKE_C8_ALONE_SCREENSHOT");
        }
        632 => {
            played_gear_fixture(app, 328);
            app.game.players[0].units = vec![public_unit(9980, 22, None)];
            app.game.players[0].hand = vec![328];
            app.game.players[0].any_power = 4;
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
            assert_eq!(app.game.players[0].gear.len(), 2);
            app.selection = Some(Selection::Gear(app.game.players[0].gear[0].id));
            click_unit(app, 9980);
        }
        633 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: 9980
                }]
            );
            click(app, 1285.0, 522.0);
        }
        634 => {
            assert!(
                app.game.players[0].gear[0].attached_to.is_none(),
                "Equip awaits responses"
            );
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].gear[0].attached_to, Some(9980));
            app.selection = Some(Selection::Gear(app.game.players[0].gear[1].id));
            click_unit(app, 9980);
        }
        635 => click(app, 1285.0, 522.0),
        636 => {
            finish_chain_until_choice(app);
            assert!(
                app.game.players[0]
                    .gear
                    .iter()
                    .all(|gear| gear.attached_to == Some(9980))
            );
            assert_eq!(
                app.game
                    .unit_keywords(&app.game.players[0].units[0])
                    .expect("valid numeric smoke fixture")
                    .hunt,
                4
            );
            let text = super::live_unit_description(&app.game, 0, 9980).unwrap();
            assert!(text.contains("one Hunt ability gains 4 XP"));
            capture_unit(app, 9980, "RIFT_SMOKE_C9_HUNT_SCREENSHOT");
        }
        637 => {
            app.hover_card = None;
            app.hover_rect = None;
            let r = unit_rect(app, 9980);
            click(app, r.x + r.w - 10.0, r.y + r.h - 9.0);
        }
        638 => {
            assert_eq!(app.equipment_inspect, Some((0, 9980)));
            capture(app, "RIFT_SMOKE_C9_GEAR_SCREENSHOT");
            click(app, 700.0, 713.0);
        }
        639 => {
            assert!(app.equipment_inspect.is_none());
            app.game.battlefields = [3, 3];
            app.game
                .apply(Action::Move {
                    unit_id: 9980,
                    lane: Some(0),
                })
                .unwrap();
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            let hunts: Vec<_> = app
                .game
                .chain
                .iter()
                .filter(|item| {
                    matches!(
                        item.ability,
                        Some(local_rift::game::ChainAbility::Hunt { .. })
                    )
                })
                .collect();
            assert_eq!(hunts.len(), 1);
            assert!(app.game.chain_name(hunts[0]).contains("Hunt 4"));
            capture(app, "RIFT_SMOKE_C9_HUNT_CHAIN_SCREENSHOT");
        }
        640 => {
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].xp, 4);
            expansion_fixture(app, 330);
            app.game.players[0].units = vec![public_unit(9981, 110, None)];
            app.game.players[0].gear = vec![GearState {
                id: 9982,
                card_id: 330,
                ready: true,
                attached_to: Some(9981),
                temporary: false,
                token: false,
            }];
            app.game.players[0].xp = 2;
            assert!(
                super::live_unit_description(&app.game, 0, 9981)
                    .unwrap()
                    .contains("INACTIVE · Soul Sword")
            );
            app.game.players[0].xp = 3;
            assert_eq!(
                app.game
                    .unit_might(&app.game.players[0].units[0])
                    .expect("valid numeric smoke fixture"),
                4
            );
            capture_gear(app, 9982, "RIFT_SMOKE_C9_LEVEL_SCREENSHOT");
        }
        641 => {
            expansion_fixture(app, 325);
            app.game.players[0].units = vec![public_unit(9983, 325, None)];
            let calm = domain_index(Domain::Calm);
            app.game.players[0].runes = [0; 6];
            app.game.players[0].runes[calm] = 8;
            app.game.players[0].exhausted_runes = [0; 6];
            app.game.players[0].exhausted_runes[calm] = 1;
            app.game.players[0].energy = 7;
            app.game.players[0].rune_deck = vec![Domain::Calm; 4];
            assert_eq!(
                app.game
                    .unit_might(&app.game.players[0].units[0])
                    .expect("valid numeric smoke fixture"),
                8
            );
            capture_unit(app, 9983, "RIFT_SMOKE_C9_RUNES_SCREENSHOT");
        }
        642 => {
            app.hover_card = None;
            app.hover_rect = None;
            click(app, 430.0, 605.0);
        }
        643 => {
            assert_eq!(app.rune_menu, Some(Domain::Calm));
            click(app, app.rune_menu_at.x + 123.0, app.rune_menu_at.y + 120.0);
        }
        644 => {
            assert!(app.rune_menu.is_none());
            assert_eq!(app.game.players[0].runes.iter().sum::<u8>(), 7);
            assert_eq!(
                app.game
                    .unit_might(&app.game.players[0].units[0])
                    .expect("valid numeric smoke fixture"),
                4
            );
            let text = super::live_unit_description(&app.game, 0, 9983).unwrap();
            assert!(text.contains("INACTIVE · Master Yi") && text.contains("currently 7;"));
            capture_unit(app, 9983, "RIFT_SMOKE_C9_RECYCLED_SCREENSHOT");
        }
        645 => {
            expansion_fixture(app, 319);
            app.game.players[0].units = vec![public_unit(9984, 265, None)];
            app.game.players[0].any_power = 4;
            click(app, 315.0, 760.0);
        }
        646 => click(app, 1285.0, 510.0),
        647 => {
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TokenPlay {
                    card_id: 319,
                    fixed_lane: Some(None),
                    ..
                })
            ));
            assert!(
                app.game
                    .pending_tokens()
                    .iter()
                    .find(|token| token.card_id == 319)
                    .unwrap()
                    .entry_ready
            );
            assert!(!app.options.accelerate);
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .find(|unit| unit.card_id == 319 && !unit.token)
                    .unwrap()
                    .ready
            );
            save_state(app);
            capture(app, "RIFT_SMOKE_C9_READY_TOKEN_SCREENSHOT");
            click(app, 1285.0, 521.0);
        }
        648 => {
            assert_same_state(app);
            click(app, 1285.0, 492.0);
        }
        649 => {
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .any(|unit| unit.card_id == 319 && unit.token && unit.ready)
            );
            assert!(app.game.pending_tokens().is_empty());
            expansion_fixture(app, 318);
            app.game.players[0].units = vec![public_unit(9985, 265, None)];
            app.game.players[0].any_power = 4;
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
                Some(Decision::TokenPlay { card_id: 318, .. })
            ));
            assert!(!app.game.pending_tokens()[0].entry_ready);
        }
        650 => click(app, 1285.0, 492.0),
        651 => {
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .any(|unit| unit.card_id == 318 && unit.token && !unit.ready)
            );
            expansion_fixture(app, 331);
            app.game.players[0].units = vec![public_unit(9986, 110, None)];
            click(app, 315.0, 760.0);
        }
        652 => click(app, 1285.0, 510.0),
        653 => {
            let gear_id = app.game.players[0]
                .gear
                .iter()
                .find(|gear| gear.card_id == 331 && gear.temporary)
                .unwrap()
                .id;
            assert_eq!(
                app.game
                    .unit_keywords(&app.game.players[0].units[0])
                    .expect("valid numeric smoke fixture")
                    .deflect,
                1
            );
            capture_gear(app, gear_id, "RIFT_SMOKE_C9_TEMPORARY_SCREENSHOT");
            app.game.apply(Action::EndTurn).unwrap();
            app.game.run_ai_turn().unwrap();
        }
        654 => {
            assert_eq!(app.game.active, 0);
            assert!(app.game.chain.iter().any(|item| item.card_id == 331));
            assert!(
                app.game.players[0]
                    .gear
                    .iter()
                    .any(|gear| gear.card_id == 331),
                "Temporary remains until its trigger resolves"
            );
            finish_chain_until_choice(app);
            assert!(
                !app.game.players[0]
                    .gear
                    .iter()
                    .any(|gear| gear.card_id == 331)
            );
            assert_eq!(
                app.game
                    .unit_keywords(&app.game.players[0].units[0])
                    .expect("valid numeric smoke fixture")
                    .deflect,
                0
            );
            expansion_fixture(app, 308);
            app.game.players[0].units = vec![
                public_unit(9987, 110, None),
                public_unit(9988, 110, Some(0)),
            ];
            app.game.players[1].units = vec![
                public_unit(9989, 110, None),
                public_unit(9990, 110, Some(1)),
            ];
            app.game.players[0].gear = vec![
                GearState {
                    id: 9991,
                    card_id: 10,
                    ready: true,
                    attached_to: Some(9987),
                    temporary: false,
                    token: false,
                },
                GearState {
                    id: 9992,
                    card_id: 21,
                    ready: true,
                    attached_to: None,
                    temporary: false,
                    token: false,
                },
            ];
            app.game.players[0].any_power = 3;
            click(app, 315.0, 760.0);
        }
        655 => {
            assert!(super::card_brief(&app.game, 308).contains("both players, everywhere"));
            assert_eq!(local_rift::rules::play_target_spec(308).maximum, 0);
            click(app, 1285.0, 510.0);
        }
        656 => {
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::DeathReplacement { player: 0, .. })
            ));
            assert_eq!(
                app.game
                    .chain
                    .iter()
                    .filter(|item| item.card_id == 308 && item.ability.is_none())
                    .count(),
                1
            );
            save_state(app);
            assert!(
                app.game
                    .apply(Action::ChooseDeathReplacement {
                        unit_id: 999999,
                        gear_id: None
                    })
                    .is_err()
            );
            assert_same_state(app);
            capture(app, "RIFT_SMOKE_C9_RUINATION_SCREENSHOT");
            let options = match &app.game.decision {
                Some(Decision::DeathReplacement { options, .. }) => options,
                _ => unreachable!(),
            };
            let index = options
                .iter()
                .position(|option| option.gear_id == Some(9991))
                .unwrap();
            click(app, 315.0 + 148.0 * index as f32, 760.0);
        }
        657 => {
            if matches!(app.game.decision, Some(Decision::DeathReplacement { .. })) {
                click(app, 315.0, 760.0);
            } else {
                finish_chain_until_choice(app);
            }
        }
        658 => {
            finish_chain_until_choice(app);
            assert!(app.game.decision.is_none());
            assert!(
                app.game.players[1].units.is_empty(),
                "global kill reaches both enemy locations"
            );
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .any(|unit| unit.id == 9987 && unit.lane.is_none()),
                "chosen Guardian saves and recalls its original bearer"
            );
            assert_eq!(
                app.game.players[0]
                    .discard
                    .iter()
                    .filter(|&&id| id == 308)
                    .count(),
                1
            );
            expansion_fixture(app, 310);
            let body = domain_index(Domain::Body);
            app.game.players[0].runes[body] = 11;
            app.game.players[0].rune_deck = vec![Domain::Body];
            app.game.players[0].energy = 11;
            click(app, 315.0, 760.0);
        }
        659 => click(app, 1285.0, 510.0),
        660 => {
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].runes.iter().sum::<u8>(), 12);
            assert_eq!(
                app.game.players[0].exhausted_runes.iter().sum::<u8>(),
                5,
                "pay four energy, then channel the last rune exhausted"
            );
            assert_eq!(
                app.game.players[0].hand.len(),
                1,
                "channeling fewer than two also draws exactly once"
            );
            expansion_fixture(app, 27);
            app.game.players[0].units = vec![
                public_unit(9993, 322, None),
                public_unit(9994, 321, None),
                public_unit(9995, 323, None),
            ];
            click(app, 315.0, 760.0);
        }
        661 => click(app, 1285.0, 510.0),
        662 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::OnPlayTarget { card_id: 27, .. })
            ));
            click_unit(app, 9993);
        }
        663 => {
            finish_chain_until_choice(app);
            assert!(app.game.buffed_units.contains(&9993));
            let bully = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.id == 9993)
                .unwrap();
            assert!(
                app.game
                    .unit_keywords(bully)
                    .expect("valid numeric smoke fixture")
                    .ganking
            );
            let text = super::live_unit_description(&app.game, 0, 9993).unwrap();
            assert!(
                text.contains("BUFFED ·") && text.contains("ACTIVE · Bilgewater Bully: Ganking")
            );
            let hound = super::live_unit_description(&app.game, 0, 9995).unwrap();
            assert!(
                hound.contains("3 current might")
                    && hound.contains("another friendly unit is at its location")
            );
            capture_unit(app, 9993, "RIFT_SMOKE_C9_BUFFED_SCREENSHOT");
        }
        664 => {
            expansion_fixture(app, 66);
            app.game.players[0].units = vec![public_unit(9997, 69, None)];
            app.game.players[0].gear = vec![GearState {
                id: 9996,
                card_id: 3,
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            }];
            app.game.players[0].any_power = 2;
            click(app, 315.0, 760.0);
        }
        665 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            save_state(app);
            click(app, 400.0, 663.0);
        }
        666 => {
            assert_same_state(app);
            assert!(
                app.selection == Some(Selection::Hand(0)),
                "Loadout preserves a gear-targeting card draft"
            );
            assert!(app.targets.is_empty());
            click(app, 615.0, 760.0);
        }
        667 => {
            assert_same_state(app);
            assert_eq!(
                app.targets,
                vec![Target::Gear {
                    player: 0,
                    gear_id: 9996
                }]
            );
            click(app, 1285.0, 510.0);
        }
        668 => {
            finish_chain_until_choice(app);
            let status = app.game.gear_temporary_status(0, 9996).unwrap();
            assert!(status.granted && status.active && !status.intrinsic);
            click(app, 615.0, 760.0);
        }
        669 => click_unit(app, 9997),
        670 => {
            assert!(app.selection == Some(Selection::Gear(9996)));
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: 9997
                }]
            );
            click(app, 1285.0, 522.0);
        }
        671 => {
            assert!(
                app.game.players[0].gear[0].attached_to.is_none(),
                "Equip still offers responses"
            );
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].gear[0].attached_to, Some(9997));
            let status = app.game.gear_temporary_status(0, 9996).unwrap();
            assert!(status.granted && status.active && !status.intrinsic);
            let text = super::live_gear_description(&app.game, 0, 9996).unwrap();
            assert!(text.contains("Granted Temporary remains active even while attached"));
            capture_gear(app, 9996, "RIFT_SMOKE_C9_GRANTED_TEMPORARY_SCREENSHOT");
        }
        672 => click(app, 1285.0, 595.0),
        673 => {
            assert_eq!(app.game.active, 1);
            app.game.run_ai_turn().unwrap();
        }
        674 => {
            assert_eq!(app.game.active, 0);
            assert!(app.game.chain.iter().any(|item| item.card_id == 3));
            assert_eq!(
                app.game.players[0].gear[0].attached_to,
                Some(9997),
                "granted expiration waits on the chain"
            );
            capture(app, "RIFT_SMOKE_C9_GRANTED_EXPIRY_SCREENSHOT");
            finish_chain_until_choice(app);
            assert!(app.game.players[0].gear.iter().all(|gear| gear.id != 9996));
            assert!(
                app.game.players[0].units.iter().any(|unit| unit.id == 9997),
                "only the gear expires, not its bearer"
            );
            assert!(app.game.players[0].discard.contains(&3));
        }
        675 => {
            expansion_fixture(app, 334);
            app.game.players[0].xp = 2;
            click(app, 315.0, 760.0);
        }
        676 => {
            assert_eq!(
                super::entry_ready_hint(&app.game, 0, 334).unwrap(),
                "Level 3 entry: exhausted · XP 2"
            );
            capture(app, "RIFT_SMOKE_C10_ENTRY_LOW_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        677 => {
            finish_chain_until_choice(app);
            assert!(
                !app.game.players[0]
                    .units
                    .iter()
                    .find(|unit| unit.card_id == 334)
                    .unwrap()
                    .ready
            );
            expansion_fixture(app, 334);
            app.game.players[0].xp = 3;
            click(app, 315.0, 760.0);
        }
        678 => {
            assert_eq!(
                super::entry_ready_hint(&app.game, 0, 334).unwrap(),
                "Level 3 entry: ready · XP 3"
            );
            capture(app, "RIFT_SMOKE_C10_ENTRY_HIGH_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        679 => {
            finish_chain_until_choice(app);
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .find(|unit| unit.card_id == 334)
                    .unwrap()
                    .ready
            );
            // This controlled balance fixture is not an available XP-spending action.
            app.game.players[0].xp = 2;
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .find(|unit| unit.card_id == 334)
                    .unwrap()
                    .ready,
                "losing the entry threshold does not exhaust an entered unit"
            );
        }
        680 => {
            expansion_fixture(app, 342);
            app.game.battlefields = [3, 3];
            app.game.players[0].xp = 255;
            app.game.players[0].units = vec![
                public_unit(10010, 342, Some(0)),
                public_unit(10011, 343, None),
            ];
            app.game.battlefield_owner = [Some(0), None];
            assert!(
                !app.game.gained_xp_this_turn(0),
                "setting the public fixture balance is not an XP gain"
            );
            assert!(
                !app.game
                    .unit_keywords(&app.game.players[0].units[0])
                    .expect("valid numeric smoke fixture")
                    .ganking
            );
            click_unit(app, 10010);
        }
        681 => {
            save_state(app);
            click(app, 1285.0, 462.0);
        }
        682 => {
            assert_same_state(app);
            assert_eq!(app.game.players[0].units[0].lane, Some(0));
            click_unit(app, 10011);
        }
        683 => click(app, 1285.0, 462.0),
        684 => {
            finish_chain_until_choice(app);
            assert_eq!(
                app.game.players[0].xp, 257,
                "actual Root move resolves a gain beyond 255"
            );
            assert!(app.game.gained_xp_this_turn(0));
            while app.game.in_window() {
                app.game.apply(Action::Pass).unwrap();
            }
            let text = super::live_unit_description(&app.game, 0, 10010).unwrap();
            assert!(
                text.contains("gained XP this turn: yes (current XP 257")
                    && text.contains("ACTIVE · Wily Newtfish: Ganking")
            );
            capture_unit(app, 10010, "RIFT_SMOKE_C10_HISTORY_SCREENSHOT");
        }
        685 => {
            app.hover_card = None;
            app.hover_rect = None;
            click_unit(app, 10010);
        }
        686 => click(app, 1285.0, 462.0),
        687 => {
            assert_eq!(
                app.game.players[0]
                    .units
                    .iter()
                    .find(|unit| unit.id == 10010)
                    .unwrap()
                    .lane,
                Some(1),
                "history-granted Ganking enables the actual move button"
            );
            while app.game.in_window() {
                app.game.apply(Action::Pass).unwrap();
            }
            app.game.apply(Action::EndTurn).unwrap();
            assert!(
                !app.game.gained_xp_this_turn(0),
                "history resets at the next turn"
            );
            assert!(
                !app.game
                    .unit_keywords(
                        app.game.players[0]
                            .units
                            .iter()
                            .find(|unit| unit.id == 10010)
                            .unwrap()
                    )
                    .expect("valid numeric smoke fixture")
                    .ganking
            );
            capture_unit(app, 10010, "RIFT_SMOKE_C10_HISTORY_RESET_SCREENSHOT");
        }
        688 => {
            expansion_fixture(app, 336);
            app.game.players[0].xp = 10;
            click(app, 315.0, 760.0);
        }
        689 => {
            assert_eq!(
                app.game
                    .cost_plan(&app.canonical_action(app.selected_play_action().unwrap()))
                    .unwrap()
                    .energy,
                3
            );
            save_state(app);
            click(app, 1285.0, 468.0);
        }
        690 => {
            assert!(
                app.play_draft.as_ref().is_some_and(|draft| draft.open),
                "the actual Level reduction opens canonical cost choices"
            );
            click(app, 900.0, 851.0);
        }
        691 => {
            assert!(app.cost_editor.is_some());
            capture(app, "RIFT_SMOKE_C10_COST_SCREENSHOT");
            click(app, 420.0, 752.0);
        }
        692 => {
            assert!(app.cost_editor.is_none());
            assert_same_state(app);
            assert!(app.selection == Some(Selection::Hand(0)));
            // Controlled threshold fixture: the first slice has no XP spending action.
            app.game.players[0].xp = 11;
            assert_eq!(
                app.game
                    .cost_plan(&app.canonical_action(app.selected_play_action().unwrap()))
                    .unwrap()
                    .energy,
                1,
                "Concentrate uses only the highest reached discount"
            );
            click(app, 1285.0, 510.0);
        }
        693 => {
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].energy, 11);
            assert_eq!(app.game.players[0].hand.len(), 2);
            expansion_fixture(app, 335);
            app.game.players[0].xp = 6;
            app.game.players[1].units = vec![public_unit(10012, 110, None)];
            click(app, 315.0, 760.0);
        }
        694 => click_unit(app, 10012),
        695 => click(app, 1285.0, 510.0),
        696 => {
            finish_chain_until_choice(app);
            assert_eq!(
                app.game.players[1].units[0].temporary_buff, 3,
                "Combat Experience may target an enemy and uses +3 instead"
            );
            expansion_fixture(app, 344);
            app.game.battlefields = [3, 3];
            app.game.players[0].xp = 255;
            app.game.players[1].xp = u64::MAX;
            app.game.players[0].units = vec![public_unit(10013, 344, Some(0))];
            app.game.battlefield_owner = [Some(0), None];
            click_unit(app, 10013);
        }
        697 => click(app, 1285.0, 462.0),
        698 => {
            assert_eq!(
                app.game.players[0].units[0].lane,
                Some(1),
                "intrinsic Nilah Ganking enables the actual move button"
            );
            finish_chain_until_choice(app);
            while app.game.in_window() {
                app.game.apply(Action::Pass).unwrap();
            }
            assert_eq!(app.game.players[0].xp, 256);
            assert_eq!(super::xp_status(&app.game, 0), "Your XP 256 · + this turn");
            assert_eq!(
                super::xp_status(&app.game, 1),
                "Opponent XP 18446744073709551615"
            );
            capture(app, "RIFT_SMOKE_C10_WIDE_XP_SCREENSHOT");
            expansion_fixture(app, 341);
            app.game.players[0].xp = 6;
            click(app, 315.0, 760.0);
        }
        699 => click(app, 1285.0, 510.0),
        700 => {
            finish_chain_until_choice(app);
            assert_eq!(
                app.game.players[0].hand.len(),
                1,
                "actual Level6 Wuju play draws exactly one"
            );
            expansion_fixture(app, 340);
            app.game.battlefields = [3, 3];
            app.game.players[0].xp = 1;
            app.game.players[0].units = vec![public_unit(10014, 343, None)];
            click_unit(app, 10014);
        }
        701 => click(app, 1285.0, 420.0),
        702 => {
            finish_chain_until_choice(app);
            while app.game.in_window() {
                app.game.apply(Action::Pass).unwrap();
            }
            assert_eq!(
                app.game.players[0].xp, 3,
                "Root reaches the entry threshold through its actual move trigger"
            );
            click(app, 315.0, 760.0);
        }
        703 => {
            assert!(
                super::entry_ready_hint(&app.game, 0, 340)
                    .unwrap()
                    .contains("ready · XP 3")
            );
            click(app, 1285.0, 510.0);
        }
        704 => {
            finish_chain_until_choice(app);
            let unit = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 340)
                .unwrap();
            assert!(unit.ready);
            assert_eq!(
                app.game
                    .unit_might(unit)
                    .expect("valid numeric smoke fixture"),
                4
            );
            capture_unit(app, unit.id, "RIFT_SMOKE_C10_READY_ENTRY_SCREENSHOT");
        }
        705 => {
            expansion_fixture(app, 338);
            app.game.battlefields = [3, 3];
            app.game.players[0].xp = 3;
            app.game.players[0].units = vec![public_unit(10015, 338, Some(0))];
            app.game.battlefield_owner = [Some(0), None];
            click_unit(app, 10015);
        }
        706 => click(app, 1285.0, 462.0),
        707 => {
            assert_eq!(
                app.game.players[0].units[0].lane,
                Some(1),
                "Level-granted Gustwalker Ganking enables the actual move button"
            );
            while app.game.in_window() {
                app.game.apply(Action::Pass).unwrap();
            }
            assert_eq!(
                app.game.players[0].xp, 5,
                "actual Hunt2 conquest resolves after the move"
            );
            capture_unit(app, 10015, "RIFT_SMOKE_C10_GANKING_SCREENSHOT");
        }
        708 => {
            expansion_fixture(app, 345);
            app.game.battlefields = [3, 3];
            app.game.players[0].xp = 1;
            let mut crowd = public_unit(10020, 345, None);
            crowd.ready = false;
            app.game.players[0].units = vec![
                crowd,
                public_unit(10021, 343, None),
                public_unit(10023, 342, None),
            ];
            app.game
                .apply(Action::Move {
                    unit_id: 10021,
                    lane: Some(0),
                })
                .unwrap();
            finish_chain_until_choice(app);
            while app.game.in_window() {
                app.game.apply(Action::Pass).unwrap();
            }
            assert_eq!(app.game.players[0].xp, 3);
            assert!(app.game.gained_xp_this_turn(0));
            let mut damaged = public_unit(10022, 339, None);
            damaged.damage = 3;
            app.game.players[0].units.push(damaged);
            assert_eq!(
                app.game
                    .unit_might(app.game.players[0].units.last().unwrap())
                    .expect("valid numeric smoke fixture"),
                4
            );
            app.game.players[0].gear = vec![GearState {
                id: 10025,
                card_id: 347,
                ready: true,
                attached_to: None,
                temporary: false,
                token: false,
            }];
            click_unit(app, 10020);
        }
        709 => click(app, 1340.0, 547.0),
        710 => click(app, 1285.0, 417.0),
        711 => {
            assert!(matches!(
                app.selection,
                Some(Selection::Ability(_, Some(0)))
            ));
            save_state(app);
            capture(app, "RIFT_SMOKE_C11_XP_COST_SCREENSHOT");
            click(app, 1285.0, 547.0);
        }
        712 => {
            assert_same_state(app);
            click(app, 1285.0, 417.0);
        }
        713 => click(app, 1285.0, 510.0),
        714 => {
            assert_eq!(app.game.players[0].xp, 1, "XP is paid before responses");
            assert_eq!(
                app.game.players[0]
                    .units
                    .iter()
                    .find(|unit| unit.id == 10020)
                    .unwrap()
                    .buff,
                0
            );
            assert!(
                !app.game.players[0]
                    .units
                    .iter()
                    .any(|unit| unit.id == 10022),
                "lost Level Might kills a lethally damaged Mosstomper before Buff resolves"
            );
            assert!(app.game.players[0].discard.contains(&339));
            assert!(app.game.gained_xp_this_turn(0));
            assert!(
                app.game
                    .unit_keywords(
                        app.game.players[0]
                            .units
                            .iter()
                            .find(|unit| unit.id == 10023)
                            .unwrap()
                    )
                    .expect("valid numeric smoke fixture")
                    .ganking,
                "spending XP does not erase Wily's real gain history"
            );
            capture(app, "RIFT_SMOKE_C11_XP_SPENT_SCREENSHOT");
            finish_chain_until_choice(app);
            assert_eq!(
                app.game.players[0]
                    .units
                    .iter()
                    .find(|unit| unit.id == 10020)
                    .unwrap()
                    .buff,
                1
            );
            click_unit(app, 10020);
        }
        715 => click(app, 1340.0, 547.0),
        716 => click(app, 1285.0, 417.0),
        717 => {
            save_state(app);
            click(app, 1285.0, 510.0);
        }
        718 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::Payment { player: 0 })
            ));
            assert_eq!(app.game.payment_plan_view(0).unwrap().xp, 2);
            assert_eq!(
                app.game.players[0].xp, 1,
                "recording an obligation does not spend unavailable XP"
            );
            assert!(
                app.game
                    .can_cancel_payment()
                    .expect("valid numeric smoke fixture")
            );
            capture(app, "RIFT_SMOKE_C11_LOCKED_XP_SCREENSHOT");
            click(app, 400.0, 662.0);
        }
        719 => click(app, 615.0, 760.0),
        720 => click(app, 1285.0, 417.0),
        721 => click(app, 1285.0, 510.0),
        722 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::Payment { player: 0 })
            ));
            assert_eq!(app.game.players[0].any_power, 1);
            assert!(!app.game.players[0].gear[0].ready);
            assert_eq!(app.game.players[0].xp, 1);
            assert_eq!(
                app.game.payment_plan_view(0).unwrap().xp,
                2,
                "Add power cannot replace the frozen XP obligation"
            );
            app.prefs.exact_payment = true;
            click(app, 1285.0, 547.0);
        }
        723 => {
            assert_same_state(app);
            assert!(app.game.payment_plan_view(0).is_none());
            expansion_fixture(app, 346);
            app.game.players[0].xp = 2;
            let mut unit = public_unit(10026, 346, None);
            unit.ready = false;
            app.game.players[0].units = vec![unit];
            click_unit(app, 10026);
        }
        724 => click(app, 1340.0, 547.0),
        725 => click(app, 1285.0, 417.0),
        726 => click(app, 1285.0, 510.0),
        727 => {
            assert_eq!(app.game.players[0].xp, 0);
            assert_eq!(app.game.players[0].units[0].buff, 0);
            assert!(
                !app.game.players[0].units[0].ready,
                "XP self-Buff does not require or grant readiness"
            );
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].units[0].buff, 1);
            capture_unit(app, 10026, "RIFT_SMOKE_C11_PROTECTOR_SCREENSHOT");
        }
        728 => {
            expansion_fixture(app, 347);
            app.game.players[0].xp = 5;
            click(app, 315.0, 760.0);
        }
        729 => click(app, 1285.0, 510.0),
        730 => {
            assert_eq!(app.game.players[0].gear[0].card_id, 347);
            assert!(
                !app.game.players[0].gear[0].ready,
                "Honeyfruit's real play enters exhausted"
            );
            app.game.apply(Action::EndTurn).unwrap();
            app.game.run_ai_turn().unwrap();
            finish_chain_until_choice(app);
            assert_eq!(app.game.active, 0);
            assert!(
                app.game.players[0].gear[0].ready,
                "Honeyfruit readies at its next actual awakening"
            );
            app.tray = super::Tray::Loadout;
            click(app, 615.0, 760.0);
        }
        731 => {
            let source = local_rift::activated::AbilitySource::Gear {
                player: 0,
                gear_id: app.game.players[0].gear[0].id,
            };
            assert_eq!(app.source_abilities(source), vec![(347, 0)]);
            capture(app, "RIFT_SMOKE_C11_HONEY_LOW_SCREENSHOT");
            click(app, 1285.0, 417.0);
        }
        732 => {
            assert_eq!(app.game.players[0].any_power, 0);
            ADD_CHAIN_SNAPSHOT.with(|value| {
                *value.borrow_mut() = app.game.chain.iter().map(|item| item.id).collect()
            });
            click(app, 1285.0, 510.0);
        }
        733 => {
            assert_eq!(app.game.players[0].any_power, 1);
            assert_eq!(
                app.game.players[0].energy_pool, 0,
                "base Honeyfruit output adds no Energy"
            );
            assert_eq!(app.game.players[0].xp, 5);
            assert!(!app.game.players[0].gear[0].ready);
            ADD_CHAIN_SNAPSHOT.with(|value| {
                assert_eq!(
                    *value.borrow(),
                    app.game
                        .chain
                        .iter()
                        .map(|item| item.id)
                        .collect::<Vec<_>>()
                )
            });
            app.game.apply(Action::EndTurn).unwrap();
            app.game.run_ai_turn().unwrap();
            finish_chain_until_choice(app);
            app.game.players[0].xp = 6;
            app.selection = None;
            click(app, 615.0, 760.0);
        }
        734 => {
            let source = local_rift::activated::AbilitySource::Gear {
                player: 0,
                gear_id: app.game.players[0].gear[0].id,
            };
            assert_eq!(app.source_abilities(source), vec![(347, 0), (347, 1)]);
            capture(app, "RIFT_SMOKE_C11_HONEY_CHOICES_SCREENSHOT");
            click(app, 1285.0, 481.0);
        }
        735 => {
            assert!(matches!(
                app.selection,
                Some(Selection::Ability(_, Some(1)))
            ));
            capture(app, "RIFT_SMOKE_C11_HONEY_OUTPUT_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        736 => {
            assert_eq!(app.game.players[0].energy_pool, 1);
            assert_eq!(app.game.players[0].any_power, 1);
            assert_eq!(
                app.game.players[0].xp, 6,
                "Level checks XP without spending it"
            );
            assert!(!app.game.players[0].gear[0].ready);
            expansion_fixture(app, 348);
            app.game.players[0].xp = 0;
            app.game.players[0].units = vec![public_unit(10024, 110, None)];
            click(app, 315.0, 760.0);
        }
        737 => click(app, 1285.0, 510.0),
        738 => {
            assert_eq!(app.game.players[0].gear[0].card_id, 348);
            assert_eq!(
                app.game.players[0].xp, 0,
                "Heirloom's play XP waits for its real trigger"
            );
            assert!(!app.game.chain.is_empty());
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].xp, 1);
            assert!(app.game.gained_xp_this_turn(0));
            app.tray = super::Tray::Loadout;
            click(app, 615.0, 760.0);
        }
        739 => click_unit(app, 10024),
        740 => {
            let gear_id = app.game.players[0].gear[0].id;
            let plan = app
                .game
                .cost_plan(&Action::Equip {
                    gear_id,
                    target: Target::Unit {
                        player: 0,
                        unit_id: 10024,
                    },
                })
                .unwrap();
            assert_eq!(plan.xp, 1);
            assert_eq!(plan.energy, 0);
            assert!(plan.power.is_empty());
            capture(app, "RIFT_SMOKE_C11_HEIRLOOM_COST_SCREENSHOT");
            click(app, 1285.0, 522.0);
        }
        741 => {
            assert_eq!(app.game.players[0].xp, 0);
            assert!(
                app.game.players[0].gear[0].attached_to.is_none(),
                "XP Equip opens responses before attachment"
            );
            assert!(app.game.gained_xp_this_turn(0));
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].gear[0].attached_to, Some(10024));
            assert_eq!(
                app.game
                    .unit_might(&app.game.players[0].units[0])
                    .expect("valid numeric smoke fixture"),
                4
            );
            assert_eq!(
                app.game.players[0].xp, 0,
                "attachment does not repeat the own-play XP trigger"
            );
            capture_gear(
                app,
                app.game.players[0].gear[0].id,
                "RIFT_SMOKE_C11_HEIRLOOM_SCREENSHOT",
            );
        }
        742 => {
            played_gear_fixture(app, 347);
            app.game.players[0].gear[0].ready = true;
            app.game.players[0].xp = 6;
            app.game.players[0].units = vec![public_unit(10027, 110, None)];
            fixture_counter_tax(app, 10027);
            app.game.players[0].runes = [0; 6];
            app.game.players[0].exhausted_runes = [0; 6];
            app.game.players[0].energy_pool = 1;
            app.game.players[0].energy = 1;
            ADD_CHAIN_SNAPSHOT.with(|value| {
                *value.borrow_mut() = app.game.chain.iter().map(|item| item.id).collect()
            });
            click(app, 615.0, 760.0);
        }
        743 => click(app, 1285.0, 481.0),
        744 => click(app, 1285.0, 510.0),
        745 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::CounterTax {
                    player: 0,
                    cost: 2,
                    ..
                })
            ));
            assert_eq!(app.game.players[0].energy, 2);
            assert_eq!(app.game.players[0].any_power, 1);
            assert_eq!(app.game.players[0].xp, 6);
            assert!(!app.game.players[0].gear[0].ready);
            ADD_CHAIN_SNAPSHOT.with(|value| {
                assert_eq!(
                    *value.borrow(),
                    app.game
                        .chain
                        .iter()
                        .map(|item| item.id)
                        .collect::<Vec<_>>(),
                    "Reaction Add preserves the outer payment and every physical chain item"
                )
            });
            capture(app, "RIFT_SMOKE_C11_HONEY_PAYMENT_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        746 => {
            assert_eq!(app.game.players[0].energy, 0);
            assert_eq!(
                app.game.players[0].any_power, 1,
                "tax pays Energy, not the accompanying Any power"
            );
            finish_chain_until_choice(app);
            assert_eq!(
                app.game.players[0].units[0].temporary_buff, 2,
                "paid tax lets Discipline resolve its printed +2 Might"
            );
        }
        747 => {
            expansion_fixture(app, 226);
            app.game = Game::new(3, 3, 1818, Difficulty::Easy);
            app.game.start_match().unwrap();
            app.game.battlefields = [3, 3];
            app.fields = [3, 3];
            app.player_deck = 3;
            app.opponent_deck = 3;
            let exact = 9_007_199_254_740_993;
            let mut attacker = public_unit(10031, 69, None);
            attacker.buff = exact - 8;
            let mut first = public_unit(10032, 69, Some(0));
            first.buff = exact + 100 - 8;
            first.damage = 100;
            let mut second = public_unit(10033, 69, Some(0));
            second.buff = exact + 5 - 8;
            second.damage = 5;
            app.game.players[0].units = vec![attacker];
            app.game.players[1].units = vec![first, second];
            app.game.battlefield_owner = [Some(1), None];
            app.damage_order.clear();
            app.game
                .apply(Action::Move {
                    unit_id: 10031,
                    lane: Some(0),
                })
                .unwrap();
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            assert!(matches!(
                app.game.decision,
                Some(Decision::CombatDamage {
                    player: 0,
                    total: 9_007_199_254_740_993,
                    ..
                })
            ));
            save_state(app);
            click(app, 865.0, 663.0);
        }
        748 => {
            assert_eq!(
                app.damage_amounts,
                Some(vec![(10032, 9_007_199_254_740_993)])
            );
            click(app, 315.0, 845.0);
        }
        749 => {
            assert_eq!(app.damage_editor, Some((10032, String::new())));
            app.damage_editor.as_mut().unwrap().1 = "18446744073709551616".into();
            assert!(app.commit_damage_editor().is_err());
            assert!(app.damage_editor.is_some(), "invalid input stays editable");
            assert_eq!(
                app.damage_amounts,
                Some(vec![(10032, 9_007_199_254_740_993)])
            );
            assert_same_state(app);
            app.damage_editor.as_mut().unwrap().1 = u64::MAX.to_string();
            capture(app, "RIFT_SMOKE_NUMERIC_INPUT_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        750 => {
            assert_same_state(app);
            assert_eq!(app.damage_amounts, Some(vec![(10032, u64::MAX)]));
            assert!(app.damage_validation(0, 9_007_199_254_740_993).is_err());
            click(app, 465.0, 845.0);
        }
        751 => {
            assert_eq!(app.damage_editor, Some((10033, String::new())));
            app.damage_editor.as_mut().unwrap().1 = u64::MAX.to_string();
            app.commit_damage_editor().unwrap();
            capture(app, "RIFT_SMOKE_NUMERIC_INVALID_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        752 => {
            assert_same_state(app);
            let sum: u128 = app
                .damage_amounts
                .as_ref()
                .unwrap()
                .iter()
                .map(|(_, n)| u128::from(*n))
                .sum();
            assert_eq!(
                sum,
                u128::from(u64::MAX) * 2,
                "invalid aggregate is displayed without wrapping"
            );
            click(app, 315.0, 845.0);
        }
        753 => {
            app.damage_editor.as_mut().unwrap().1 = "0".into();
            app.commit_damage_editor().unwrap();
            click(app, 465.0, 845.0);
        }
        754 => {
            app.damage_editor.as_mut().unwrap().1 = "9007199254740993".into();
            capture(app, "RIFT_SMOKE_NUMERIC_EXACT_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        755 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::CombatDamage { player: 1, .. })
            ));
            assert_eq!(app.game.players[1].units[0].damage, 100);
            assert_eq!(
                app.game.players[1].units[1].damage, 5,
                "both assignments precede damage"
            );
            let Decision::CombatDamage {
                player,
                lane,
                total,
            } = app.game.decision.clone().unwrap()
            else {
                unreachable!()
            };
            let assignments = app
                .game
                .automatic_damage_assignment(player, lane, total)
                .unwrap();
            app.game
                .apply(Action::AssignDamage { assignments })
                .unwrap();
            assert!(
                app.game.players[1]
                    .units
                    .iter()
                    .any(|unit| unit.id == 10032)
            );
            assert!(
                !app.game.players[1]
                    .units
                    .iter()
                    .any(|unit| unit.id == 10033),
                "exact alternate allocation defeats the chosen unit"
            );
            assert!(app.game.players[0].units.is_empty());
            expansion_fixture(app, 226);
            let mut unit = public_unit(10034, 69, Some(0));
            unit.buff = 9_007_199_254_740_993 - 8;
            unit.damage = 9_007_199_254_740_992;
            app.game.players[0].units = vec![unit];
            app.game.battlefield_owner[0] = Some(0);
            capture_unit(app, 10034, "RIFT_SMOKE_NUMERIC_MIGHT_SCREENSHOT");
        }
        756 => {
            app.game.players[0].units[0].buff = 0;
            app.game.players[0].units[0].damage = 0;
            app.game.players[0].units[0].temporary_buff = -11;
            let text = super::live_unit_description(&app.game, 0, 10034).unwrap();
            assert!(text.contains("Calculated might: -3"));
            capture_unit(app, 10034, "RIFT_SMOKE_NUMERIC_NEGATIVE_SCREENSHOT");
        }
        757 => {
            app.hover_card = None;
            app.hover_rect = None;
            app.game.players[0].units[0].temporary_buff = 0;
            app.game.players[0].units[0].buff = u64::MAX;
            assert!(
                app.game
                    .validate_numeric_state()
                    .unwrap_err()
                    .starts_with("engine_capacity:")
            );
            save_state(app);
            app.update();
            assert_same_state(app);
            app.companion.observe(&app.game);
            assert!(app.companion.message().contains("numeric limit"));
            capture(app, "RIFT_SMOKE_NUMERIC_ERROR_SCREENSHOT");
            assert_same_state(app);
        }
        758 => {
            expansion_fixture(app, 226);
            let mut unit = public_unit(10035, 69, Some(0));
            unit.buff = 9_007_199_254_740_993 - 8;
            app.game.players[0].units = vec![unit];
            app.game.players[1].units = vec![public_unit(10036, 69, Some(1))];
            app.game.battlefield_owner = [Some(0), Some(1)];
            click(app, 315.0, 760.0);
        }
        759 => click_unit(app, 10036),
        760 => {
            let plan = app
                .game
                .cost_plan(&app.canonical_action(app.selected_play_action().unwrap()))
                .unwrap();
            assert_eq!(plan.energy, 0);
            assert!(
                plan.modifiers
                    .iter()
                    .any(|modifier| modifier.energy == 9_007_199_254_740_993)
            );
            save_state(app);
            click(app, 1285.0, 468.0);
        }
        761 => {
            assert!(app.play_draft.as_ref().is_some_and(|draft| draft.open));
            click(app, 900.0, 851.0);
        }
        762 => {
            assert!(app.cost_editor.is_some());
            capture(app, "RIFT_SMOKE_NUMERIC_COST_SCREENSHOT");
            click(app, 420.0, 752.0);
        }
        763 => {
            assert!(app.cost_editor.is_none());
            assert_same_state(app);
            assert!(matches!(app.selection, Some(Selection::Hand(0))));
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 10036
                }]
            );
        }
        764 => {
            expansion_fixture(app, 349);
            app.game.players[0].units = vec![
                public_unit(11000, 71, None),
                public_unit(11001, 71, Some(0)),
            ];
            app.game.players[0].gear = vec![GearState {
                id: 11002,
                card_id: 21,
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            }];
            app.game.battlefield_owner[0] = Some(0);
            click(app, 315.0, 760.0);
        }
        765 => {
            assert!(
                !app.card_target_candidate(Target::Unit {
                    player: 0,
                    unit_id: 11000
                })
                .unwrap()
            );
            assert!(
                app.card_target_candidate(Target::Unit {
                    player: 0,
                    unit_id: 11001
                })
                .unwrap()
            );
            assert!(
                app.card_target_candidate(Target::Gear {
                    player: 0,
                    gear_id: 11002
                })
                .unwrap()
            );
            click(app, 400.0, 661.0);
        }
        766 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            assert!(matches!(app.tray, super::Tray::Loadout));
            capture(app, "RIFT_SMOKE_C12_MIXED_TARGET_SCREENSHOT");
            click(app, 615.0, 760.0);
        }
        767 => {
            assert_eq!(
                app.targets,
                vec![Target::Gear {
                    player: 0,
                    gear_id: 11002
                }]
            );
            click(app, 1285.0, 510.0);
        }
        768 => {
            finish_chain_until_choice(app);
            assert!(app.game.gear_temporary_status(0, 11002).unwrap().granted);
            capture_gear(app, 11002, "RIFT_SMOKE_C12_GRANTED_SCREENSHOT");
        }
        769 => {
            expansion_fixture(app, 354);
            app.game.players[0].hand.clear();
            app.game.players[0]
                .hidden
                .push(local_rift::hidden::HiddenCard {
                    id: 11003,
                    card_id: 354,
                    lane: 0,
                    hidden_turn: 0,
                    revealed: false,
                });
            app.game.players[0].units = vec![
                public_unit(11004, 69, Some(0)),
                public_unit(11005, 69, None),
                public_unit(11006, 69, Some(1)),
            ];
            app.game.battlefield_owner = [Some(0), Some(0)];
            click(app, 610.0, 661.0);
        }
        770 => click(app, 315.0, 760.0),
        771 => {
            assert!(app.selection == Some(Selection::Hidden(11003)));
            assert!(
                !app.card_target_candidate(Target::Unit {
                    player: 0,
                    unit_id: 11005
                })
                .unwrap()
            );
            click_unit(app, 11004);
        }
        772 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: 11004
                }]
            );
            assert!(
                app.card_target_candidate(Target::Unit {
                    player: 0,
                    unit_id: 11005
                })
                .unwrap()
            );
            capture(app, "RIFT_SMOKE_C12_HIDDEN_SLOT_SCREENSHOT");
            click_unit(app, 11005);
        }
        773 => {
            assert_eq!(app.targets.len(), 2);
            capture(app, "RIFT_SMOKE_C12_HIDDEN_PAIR_SCREENSHOT");
            click_unit(app, 11004);
        }
        774 => {
            assert!(
                app.targets.is_empty(),
                "Editing an early slot also clears dependent later slots"
            );
            click_unit(app, 11004);
        }
        775 => click_unit(app, 11005),
        776 => click(app, 1285.0, 534.0),
        777 => {
            finish_chain_until_choice(app);
            assert!(app.game.players[0].discard.contains(&354));
            assert_eq!(
                app.game.players[0].hand.len(),
                1,
                "Independent draw without a Temporary pair"
            );
        }
        778 => {
            expansion_fixture(app, 355);
            click(app, 315.0, 760.0);
        }
        779 => click(app, 1285.0, 510.0),
        780 => {
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TokenPlacement { .. } | Decision::TokenPlay { .. })
            ));
            app.sync_token_draft();
            let token = app.game.current_pending_token().unwrap();
            assert_eq!(app.token_draft_id, Some(token.id));
            assert!(token.entry_ready);
            capture(app, "RIFT_SMOKE_C12_SPRITE_PLACEMENT_SCREENSHOT");
            click(app, 1285.0, 470.0);
        }
        781 => {
            assert_eq!(
                app.game.players[0].units.iter().filter(|u| u.token).count(),
                1
            );
            let next = app.game.current_pending_token().unwrap().id;
            assert_ne!(app.token_draft_id, Some(next));
            app.options.accelerate = true;
            app.sync_token_draft();
            assert_eq!(app.token_draft_id, Some(next));
            assert!(
                !app.options.accelerate,
                "A second child gets a fresh play draft"
            );
            click(app, 1285.0, 470.0);
        }
        782 => {
            finish_chain_until_choice(app);
            assert!(app.game.current_pending_token().is_none());
            assert_eq!(
                app.game.players[0].units.iter().filter(|u| u.token).count(),
                2
            );
            let id = app.game.players[0].units[0].id;
            capture_unit(app, id, "RIFT_SMOKE_C12_SPRITE_PREVIEW_SCREENSHOT");
        }
        783 => {
            expansion_fixture(app, 352);
            let mut sprite = public_unit(11008, 355, Some(0));
            sprite.token = true;
            sprite.buff = 3;
            app.game.players[0].units = vec![public_unit(11007, 352, Some(0)), sprite];
            app.game.battlefield_owner[0] = Some(0);
            let text = super::live_unit_description(&app.game, 0, 11007).unwrap();
            assert!(text.contains("+1 might · +1 for each friendly Temporary"));
            capture_unit(app, 11007, "RIFT_SMOKE_C12_PETAL_SCREENSHOT");
        }
        784 => {
            expansion_fixture(app, 360);
            let mut sprite = public_unit(11010, 355, None);
            sprite.token = true;
            sprite.buff = 3;
            app.game.players[0].units = vec![public_unit(11009, 358, None), sprite];
            app.game.players[0].gear = vec![GearState {
                id: 11011,
                card_id: 360,
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            }];
            app.game.apply(Action::EndTurn).unwrap();
            app.game.apply(Action::EndTurn).unwrap();
            assert!(matches!(
                app.game.decision,
                Some(Decision::OrderTriggers { .. })
            ));
            capture(app, "RIFT_SMOKE_C12_BEGINNING_SCREENSHOT");
        }
        785 => {
            for _ in 0..160 {
                if app.game.decision.is_none() && app.game.chain.is_empty() {
                    break;
                }
                app.game.step_ai().unwrap();
            }
            assert!(app.game.players[0].gear.is_empty());
            assert!(!app.game.players[0].units.iter().any(|u| u.id == 11010));
            assert_eq!(
                app.game.players[0].units.iter().filter(|u| u.token).count(),
                2
            );
            capture(app, "RIFT_SMOKE_C12_SURVIVORS_SCREENSHOT");
        }
        786 => {
            expansion_fixture(app, 366);
            app.game.players[1].units = vec![public_unit(12000, 2, Some(0))];
            app.game.battlefield_owner[0] = Some(1);
            app.game.players[1].gear = vec![GearState {
                id: 12001,
                card_id: 3,
                attached_to: Some(12000),
                ready: true,
                temporary: false,
                token: false,
            }];
            app.game.players[0].gear = vec![GearState {
                id: 12002,
                card_id: 21,
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            }];
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                })
                .unwrap();
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerTargets { card_id: 366, .. })
            ));
        }
        787 => {
            assert!(app.tray == super::Tray::Loadout);
            assert_eq!(app.gear_owner, 1);
            capture(app, "RIFT_SMOKE_C13_GEAR_CHOICE_SCREENSHOT");
            click(app, 615.0, 760.0);
        }
        788 => {
            assert_eq!(
                app.targets,
                vec![Target::Gear {
                    player: 1,
                    gear_id: 12001
                }]
            );
            click(app, 855.0, 663.0);
        }
        789 => {
            assert_eq!(app.gear_owner, 0);
            assert_eq!(
                app.targets,
                vec![Target::Gear {
                    player: 1,
                    gear_id: 12001
                }]
            );
            capture(app, "RIFT_SMOKE_C13_RETAINED_SCREENSHOT");
            click(app, 855.0, 663.0);
        }
        790 => {
            assert_eq!(app.gear_owner, 1);
            capture(app, "RIFT_SMOKE_C13_SELECTED_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        791 => {
            assert!(!matches!(
                app.game.decision,
                Some(Decision::TriggerTargets { .. })
            ));
            finish_chain_until_choice(app);
            assert!(app.game.players[1].gear.is_empty());
            assert_eq!(app.game.players[1].units[0].id, 12000);
            assert_eq!(app.game.players[0].gear.len(), 1);
        }
        792 => {
            expansion_fixture(app, 367);
            app.game.players[1].gear = vec![GearState {
                id: 12003,
                card_id: 3,
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            }];
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                })
                .unwrap();
            finish_chain_until_choice(app);
        }
        793 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerTargets { card_id: 367, .. })
            ));
            capture(app, "RIFT_SMOKE_C13_ENERGY_LIMIT_SCREENSHOT");
            click(app, 1285.0, 547.0);
        }
        794 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[1].gear.len(), 1);
            capture(app, "RIFT_SMOKE_C13_DECLINED_SCREENSHOT");
        }
        795 => {
            // Public score fixtures exercise both tracks without changing scoring rules.
            expansion_fixture(app, 367);
            app.game.players[0].score = 0;
            app.game.players[1].score = 0;
            capture(app, "RIFT_SMOKE_VICTORY_TRACK_START_SCREENSHOT");
        }
        796 => {
            app.game.players[0].score = 4;
            app.game.players[1].score = 7;
            capture(app, "RIFT_SMOKE_VICTORY_TRACK_RACE_SCREENSHOT");
        }
        797 => {
            app.game.players[0].score = 8;
            app.game.players[1].score = 3;
            capture(app, "RIFT_SMOKE_VICTORY_TRACK_YOU_SCREENSHOT");
        }
        798 => {
            app.game.players[0].score = 2;
            app.game.players[1].score = 8;
            capture(app, "RIFT_SMOKE_VICTORY_TRACK_AI_SCREENSHOT");
        }
        799 => {
            expansion_fixture(app, 74);
            app.game.players[0].units = vec![public_unit(13000, 374, Some(1))];
            app.game.players[1].units = vec![public_unit(13001, 375, Some(1))];
            app.game.players[1].gear = vec![GearState {
                id: 13002,
                card_id: 3,
                attached_to: Some(13001),
                ready: true,
                temporary: false,
                token: false,
            }];
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![Target::Unit {
                        player: 0,
                        unit_id: 13000,
                    }],
                })
                .unwrap();
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerTargets { card_id: 374, .. })
            ));
        }
        800 => {
            assert_eq!(
                app.game.trigger_target_location(),
                Some(local_rift::game::board_targets::BoardLocation::Battlefield { lane: 1 })
            );
            capture(app, "RIFT_SMOKE_C14_DEATH_LOCATION_SCREENSHOT");
            click(app, 710.0, 663.0); // Board
        }
        801 => {
            click(app, 315.0, 760.0);
        }
        802 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 13001
                }]
            );
            capture(app, "RIFT_SMOKE_C14_UNIT_CHOICE_SCREENSHOT");
            click(app, 400.0, 663.0); // Loadout
        }
        803 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 13001
                }]
            );
            click(app, 855.0, 663.0); // Opponent
        }
        804 => {
            assert_eq!(app.gear_owner, 1);
            capture(app, "RIFT_SMOKE_C14_RETAINED_CHOICE_SCREENSHOT");
            click(app, 710.0, 663.0);
        }
        805 => {
            click(app, 315.0, 760.0);
        } // Undo unit choice.
        806 => {
            assert!(app.targets.is_empty());
            click(app, 400.0, 663.0);
        }
        807 => {
            click(app, 615.0, 760.0);
        }
        808 => {
            assert_eq!(
                app.targets,
                vec![Target::Gear {
                    player: 1,
                    gear_id: 13002
                }]
            );
            capture(app, "RIFT_SMOKE_C14_GEAR_CHOICE_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        809 => {
            finish_chain_until_choice(app);
            assert!(app.game.players[1].gear.is_empty());
            assert!(app.game.players[1].units.iter().any(|u| u.id == 13001));
            capture(app, "RIFT_SMOKE_C14_RESOLVED_SCREENSHOT");
        }
        810 => {
            expansion_fixture(app, 378);
            app.game.players[1].units = vec![public_unit(14000, 374, Some(0))];
            app.game.players[1].gear = vec![GearState {
                id: 14001,
                card_id: 10,
                attached_to: Some(14000),
                ready: false,
                temporary: false,
                token: false,
            }];
            let current = app.game.unit_might(&app.game.players[1].units[0]).unwrap();
            app.game.players[1].units[0].temporary_buff = 3 - i64::try_from(current).unwrap();
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![Target::Unit {
                        player: 1,
                        unit_id: 14000,
                    }],
                })
                .unwrap();
            finish_chain_until_choice(app);
            assert!(app.game.players[1].banish.contains(&374));
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[1].gear[0].attached_to, None);
            assert!(!app.game.players[1].gear[0].ready);
            click(app, 500.0, 663.0); // Trash browser.
        }
        811 => {
            click(app, 855.0, 663.0);
        }
        812 => {
            click(app, 930.0, 663.0);
        }
        813 => {
            assert!(app.banish_view && app.gear_owner == 1);
            assert!(app.selection.is_none());
            capture(app, "RIFT_SMOKE_C15_BANISH_BROWSER_SCREENSHOT");
            click(app, 315.0, 760.0);
        }
        814 => {
            assert_eq!(app.inspect, Some(374));
            assert!(app.selection.is_none());
            capture(app, "RIFT_SMOKE_C15_BANISH_INSPECT_SCREENSHOT");
            click(app, 960.0, 704.0);
        }
        815 => {
            assert!(app.inspect.is_none());
            click(app, 855.0, 663.0);
        }
        816 => {
            assert!(app.banish_view && app.gear_owner == 0);
            capture(app, "RIFT_SMOKE_C15_EMPTY_BANISH_SCREENSHOT");
            click(app, 930.0, 663.0);
        }
        817 => {
            assert!(!app.banish_view);
            capture(app, "RIFT_SMOKE_C15_TRASH_RESTORED_SCREENSHOT");
            expansion_fixture(app, 376);
            app.game.players[0].units = vec![public_unit(14010, 25, Some(0))];
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: Some(0),
                    targets: vec![],
                })
                .unwrap();
            finish_chain_until_choice(app);
        }
        818 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerTargets { card_id: 376, .. })
            ));
            capture(app, "RIFT_SMOKE_C15_APOTHECARY_CHOICE_SCREENSHOT");
            click(app, 710.0, 663.0); // Board.
        }
        819 => {
            click(app, 465.0, 760.0);
        } // Apothecary is second own unit.
        820 => {
            assert_eq!(app.targets.len(), 1);
            let Target::Unit { unit_id, .. } = app.targets[0] else {
                panic!()
            };
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .any(|u| u.id == unit_id && u.card_id == 376)
            );
            capture(app, "RIFT_SMOKE_C15_APOTHECARY_SELF_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        821 => {
            finish_chain_until_choice(app);
            assert!(app.game.players[0].hand.contains(&376));
            assert_eq!(app.game.players[0].units.len(), 1);
            expansion_fixture(app, 377);
            app.game.players[0].units = vec![public_unit(14020, 25, Some(0))];
            // Replace one ready rune with a rune in the deck, retaining the total.
            let d = domain_index(Domain::Mind);
            app.game.players[0].runes[d] -= 1;
            app.game.players[0].energy -= 1;
            app.game.players[0].rune_deck = vec![Domain::Mind];
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![Target::Unit {
                        player: 0,
                        unit_id: 14020,
                    }],
                })
                .unwrap();
            finish_chain_until_choice(app);
        }
        822 => {
            let d = domain_index(Domain::Mind);
            assert!(app.game.players[0].hand.contains(&25));
            assert_eq!(app.game.players[0].runes[d], 12);
            assert_eq!(app.game.players[0].exhausted_runes[d], 2); // Pay one, Channel one exhausted.
            capture(app, "RIFT_SMOKE_C15_RETREAT_RUNES_SCREENSHOT");
        }
        823 => {
            app.in_match = false;
            app.screen = Screen::Decks;
            app.legend_picker = Some(0);
            app.selection = None;
            app.inspect = None;
            click(app, 502.0, 661.0);
        }
        824 => {
            assert_eq!(app.legend_picker, Some(8));
            capture(app, "RIFT_SMOKE_C16_RENGAR_PICKER_SCREENSHOT");
            click(app, 447.0, 307.0);
        }
        825 => {
            assert_eq!(app.legend_picker, Some(9));
            capture(app, "RIFT_SMOKE_C16_EZREAL_PICKER_SCREENSHOT");
            click(app, 390.0, 661.0);
        }
        826 => {
            assert_eq!(app.legend_picker, Some(0));
            click(app, 502.0, 661.0);
        }
        827 => {
            assert_eq!(app.legend_picker, Some(8));
            click(app, 840.0, 700.0);
        }
        828 => {
            let deck = &app.deck_editor.as_ref().unwrap().deck;
            assert_eq!(
                (deck.legend, deck.chosen_champion, deck.card_count()),
                (8, 30, 40)
            );
            assert!(deck.validate().is_ok());
            click(app, 328.0, 771.0);
        }
        829 => {
            assert_eq!(app.custom_decks[0].as_ref().unwrap().legend, 8);
            app.reset();
            app.game.start_match().unwrap();
            app.in_match = true;
            app.screen = Screen::Practice;
            app.tray = super::Tray::Board;
            for p in &mut app.game.players {
                p.units.clear();
                p.gear.clear();
                p.hand.clear();
            }
            fixture_resources(app, Domain::Fury);
            app.game.players[0].hand = vec![72];
            app.game.players[1].units = vec![public_unit(15001, 72, None)];
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                })
                .unwrap();
            assert!(matches!(
                app.game.decision,
                Some(Decision::LegendTargets { player: 0 })
            ));
        }
        830 => {
            capture(app, "RIFT_SMOKE_C16_RENGAR_CHOICE_SCREENSHOT");
            click_unit(app, 15001);
        }
        831 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 15001
                }]
            );
            capture(app, "RIFT_SMOKE_C16_RENGAR_TARGET_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        832 => {
            assert!(app.game.decision.is_none());
            assert_eq!(
                app.game.chain_source(&app.game.chain[0]),
                local_rift::game::ChainSource::Legend {
                    player: 0,
                    legend_id: 8
                }
            );
            assert_eq!(app.game.players[1].units[0].temporary_buff, 0);
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[1].units[0].temporary_buff, 1);
            capture(app, "RIFT_SMOKE_C16_RENGAR_MIGHT_SCREENSHOT");
        }
        833 => {
            app.custom_decks[0] = Some(local_rift::deck::CustomDeck::from_legend(9).unwrap());
            app.reset();
            app.game.start_match().unwrap();
            app.in_match = true;
            app.screen = Screen::Practice;
            app.tray = super::Tray::Loadout;
            app.gear_owner = 0;
            for p in &mut app.game.players {
                p.units.clear();
                p.gear.clear();
                p.hand.clear();
            }
            fixture_resources(app, Domain::Mind);
            app.game.players[1].units = vec![public_unit(15002, 72, None)];
            app.game.players[0].hand = vec![63, 63];
            click(app, 315.0, 760.0);
        }
        834 => {
            assert!(matches!(app.selection, Some(Selection::Legend)));
            assert_eq!(app.game.ezreal_progress(0), 0);
            capture(app, "RIFT_SMOKE_C16_EZREAL_ZERO_SCREENSHOT");
            save_state(app);
            click(app, 1285.0, 548.0);
        }
        835 => {
            assert_same_state(app);
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![Target::Unit {
                        player: 1,
                        unit_id: 15002,
                    }],
                })
                .unwrap();
            finish_chain_until_choice(app);
            assert_eq!(app.game.ezreal_progress(0), 1);
            capture(app, "RIFT_SMOKE_C16_EZREAL_ONE_SCREENSHOT");
            save_state(app);
            click(app, 1285.0, 548.0);
        }
        836 => {
            assert_same_state(app);
            assert_eq!(app.game.players[0].hand[0], 63);
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![Target::Unit {
                        player: 1,
                        unit_id: 15002,
                    }],
                })
                .unwrap();
            finish_chain_until_choice(app);
        }
        837 => {
            assert_eq!(app.game.ezreal_progress(0), 2);
            capture(app, "RIFT_SMOKE_C16_EZREAL_READY_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        838 => {
            assert!(!app.game.legend[0].ready);
            assert_eq!(
                app.game.chain_source(&app.game.chain[0]),
                local_rift::game::ChainSource::Legend {
                    player: 0,
                    legend_id: 9
                }
            );
            capture(app, "RIFT_SMOKE_C16_EZREAL_CHAIN_SCREENSHOT");
            let count = app.game.players[0].hand.len();
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].hand.len(), count + 1);
        }
        839 => {
            assert!(app.game.chain.is_empty());
            assert!(!app.game.legend[0].ready);
        }
        840 => {
            c17_fixture(app, 379);
            app.game.players[1].units[0].card_id = 72;
            click(app, 315.0, 760.0);
        }
        841 => {
            click_unit(app, 17002);
        }
        842 => {
            capture(app, "RIFT_SMOKE_C17_BURN_TARGET_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        843 => {
            finish_chain_until_choice(app);
            assert!(app.game.players[1].units.is_empty());
            assert_eq!(app.game.players[1].banish, vec![72]);
        }
        844 => {
            c17_pending(app, 380, Some(0));
        }
        845 => {
            assert_eq!(app.destination, None);
            capture(app, "RIFT_SMOKE_C17_PORTAL_PENDING_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        846 => {
            assert!(app.game.pending_cards().is_empty());
            assert_eq!(app.game.players[0].units[0].lane, None);
            assert_ne!(app.game.players[0].units[0].id, 17001);
        }
        847 => {
            c17_pending(app, 381, Some(1));
        }
        848 => {
            assert_eq!(app.destination, Some(1));
            capture(app, "RIFT_SMOKE_C17_TEMPORAL_PENDING_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        849 => {
            assert!(app.game.pending_cards().is_empty());
            assert_eq!(app.game.players[0].units[0].lane, Some(1));
        }
        850 => {
            c17_pending(app, 382, None);
        }
        851 => {
            assert_eq!(app.destination, Some(0));
            capture(app, "RIFT_SMOKE_C17_THRILL_PENDING_SCREENSHOT");
            click(app, 1338.0, 546.0);
        }
        852 => {
            assert!(app.game.pending_cards().is_empty());
            assert_eq!(app.game.players[0].units[0].lane, Some(1));
            assert!(matches!(
                app.game.decision,
                Some(Decision::LegendTargets { .. })
            ));
            capture(app, "RIFT_SMOKE_C17_RENGAR_AFTER_REPLAY_SCREENSHOT");
        }
        853 => {
            c17_fixture(app, 383);
            click(app, 315.0, 760.0);
        }
        854 => {
            click_unit(app, 17001);
        }
        855 => {
            click_unit(app, 17002);
        }
        856 => {
            assert_eq!(app.targets.len(), 2);
            capture(app, "RIFT_SMOKE_C17_ARCANE_TARGETS_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        857 => {
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::PendingPlay { .. })
            ));
        }
        858 => {
            assert_eq!(app.game.players[0].banish, vec![383]);
            assert_eq!(app.game.players[1].units[0].damage, 3);
            capture(app, "RIFT_SMOKE_C17_ARCANE_PENDING_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        859 => {
            assert!(app.game.pending_cards().is_empty());
            assert_eq!(app.game.players[0].units[0].lane, None);
            assert_ne!(app.game.players[0].units[0].id, 17001);
            capture(app, "RIFT_SMOKE_C17_ARCANE_RETURN_SCREENSHOT");
        }
        860 => {
            c17_fixture(app, 379);
            app.game.players[0].hand = vec![70];
            app.game.players[0].energy = 0;
            app.game.players[0].energy_pool = 0;
            app.game.players[0].power_pool = [0; 6];
            app.game.players[0].any_power = 0;
            click(app, 315.0, 760.0);
        }
        861 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            capture(app, "RIFT_SMOKE_C17_UNAFFORDABLE_SCREENSHOT");
            COMBAT_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1338.0, 546.0);
        }
        862 => {
            COMBAT_SNAPSHOT.with(|snapshot| {
                assert_eq!(
                    snapshot.borrow().as_deref(),
                    Some(format!("{:?}", app.game).as_str()),
                    "Unavailable destination must not mutate the game"
                )
            });
        }
        863 => {
            c17_fixture(app, 384);
            app.game.players[0].deck = vec![70, 71, 72];
            click(app, 315.0, 760.0);
        }
        864 => {
            click(app, 1285.0, 510.0);
        }
        865 => {
            finish_chain_until_choice(app);
            assert!(
                matches!(app.game.decision,Some(Decision::PredictCards{ref cards,..}) if cards.len()==3)
            );
            app.sync_predict_draft();
            capture(app, "RIFT_SMOKE_C18_PREDICT_SCREENSHOT");
        }
        866 => {
            click(app, 905.0, 822.0);
        }
        867 => {
            assert_eq!(app.predict_draft.as_ref().unwrap().top_order, vec![0, 2, 1]);
            assert_eq!(app.game.players[0].deck, vec![70, 71, 72]);
            click(app, 905.0, 822.0);
        }
        868 => {
            assert_eq!(app.predict_draft.as_ref().unwrap().top_order, vec![2, 0, 1]);
            assert_eq!(app.game.players[0].deck, vec![70, 71, 72]);
            capture(app, "RIFT_SMOKE_C18_PREDICT_ORDER_SCREENSHOT");
        }
        869 => {
            click(app, 1285.0, 510.0);
        }
        870 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[0].deck, vec![71, 72, 70]);
            c18_offer(app, 70);
        }
        871 => {
            capture(app, "RIFT_SMOKE_C18_BANISH_OFFER_SCREENSHOT");
            app.ui.input_override = Some((vec2(315.0, 760.0), false, true));
        }
        872 => {
            assert_eq!(app.inspect, Some(70));
            capture(app, "RIFT_SMOKE_C18_BANISH_HOVER_SCREENSHOT");
            app.inspect = None;
            click(app, 1285.0, 506.0);
        }
        873 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::PendingPlay { card_id: 70, .. })
            ));
            capture(app, "RIFT_SMOKE_C18_PENDING_UNIT_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        874 => {
            assert!(app.game.players[0].units.iter().any(|u| u.card_id == 70));
            assert!(app.game.pending_cards().is_empty());
            c18_offer(app, 145);
        }
        875 => {
            capture(app, "RIFT_SMOKE_C18_DECLINE_SCREENSHOT");
            click(app, 1285.0, 546.0);
        }
        876 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[0].banish, vec![145]);
            c18_offer(app, 133);
        }
        877 => {
            click(app, 1285.0, 506.0);
        }
        878 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::PendingPlay { card_id: 133, .. })
            ));
            capture(app, "RIFT_SMOKE_C18_PENDING_SPELL_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        879 => {
            assert!(app.game.pending_cards().is_empty());
            assert!(
                app.game
                    .chain
                    .iter()
                    .filter(|i| i.card_id == 133 && i.ability.is_none())
                    .all(|i| !i.recycle_after)
            );
            finish_chain_until_choice(app);
            assert_eq!(
                app.game.players[0]
                    .discard
                    .iter()
                    .filter(|&&id| id == 133)
                    .count(),
                2
            );
            capture(app, "RIFT_SMOKE_C18_SPELL_RESOLVED_SCREENSHOT");
        }
        880 => {
            c19_hook(app);
        }
        881 => {
            assert_eq!(app.deck_look_draft.as_ref().unwrap().selection, None);
            assert!(
                matches!(&app.game.decision,Some(Decision::DeckLookPlay{cards,eligible,..}) if cards==&vec![133,10,96,145,96] && eligible==&vec![2,3,4])
            );
            save_state(app);
            capture(app, "RIFT_SMOKE_C19_HOOK_LOOK_SCREENSHOT");
            click(app, 320.0, 760.0);
        }
        882 => {
            assert_same_state(app);
            assert_eq!(app.deck_look_draft.as_ref().unwrap().selection, None);
            app.ui.input_override = Some((vec2(670.0, 760.0), false, true));
        }
        883 => {
            assert_eq!(app.inspect, Some(96));
            capture(app, "RIFT_SMOKE_C19_LOOK_INSPECT_SCREENSHOT");
            click(app, 960.0, 704.0);
        }
        884 => {
            assert!(app.inspect.is_none());
            click(app, 1029.0, 760.0);
        }
        885 => {
            assert_eq!(
                app.deck_look_draft.as_ref().unwrap().selection,
                Some(Some(4))
            );
            assert_same_state(app);
            capture(app, "RIFT_SMOKE_C19_DUPLICATE_CHOICE_SCREENSHOT");
            click(app, 1285.0, 578.0);
        }
        886 => {
            assert_eq!(app.deck_look_draft.as_ref().unwrap().selection, None);
            assert_same_state(app);
            click(app, 670.0, 760.0);
        }
        887 => {
            assert_eq!(
                app.deck_look_draft.as_ref().unwrap().selection,
                Some(Some(2))
            );
            click(app, 1285.0, 510.0);
        }
        888 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::PendingPlay { card_id: 96, .. })
            ));
            assert_eq!(app.game.players[0].deck.len(), 4);
            capture(app, "RIFT_SMOKE_C19_HOOK_PENDING_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        889 => {
            assert!(app.game.players[0].units.iter().any(|u| u.card_id == 96));
            assert!(app.game.pending_cards().is_empty());
        }
        890 => {
            c19_look(app, 387, &[10, 133, 70, 145, 70]);
        }
        891 => {
            save_state(app);
            click(app, 1029.0, 760.0);
        }
        892 => {
            assert_same_state(app);
            assert_eq!(
                app.deck_look_draft.as_ref().unwrap().selection,
                Some(Some(4))
            );
            capture(app, "RIFT_SMOKE_C19_REINFORCE_LOOK_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        893 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::PendingPlay { card_id: 70, .. })
            ));
            let action = Action::FinalizePendingPlay {
                lane: None,
                targets: vec![],
                repeat_targets: vec![],
                repeat: false,
                accelerate: false,
                bonus: false,
            };
            assert_eq!(app.game.cost_plan(&action).unwrap().energy, 4);
            capture(app, "RIFT_SMOKE_C19_REDUCED_COST_SCREENSHOT");
            save_state(app);
        }
        894 => {
            click(app, 1285.0, 510.0);
        }
        895 => {
            assert!(app.game.players[0].units.iter().any(|u| u.card_id == 70));
            assert!(app.game.pending_cards().is_empty());
        }
        896 => {
            c19_look(app, 388, &[133, 250, 70, 250, 96]);
        }
        897 => {
            click(app, 496.0, 760.0);
        }
        898 => {
            assert_eq!(
                app.deck_look_draft.as_ref().unwrap().selection,
                Some(Some(1))
            );
            capture(app, "RIFT_SMOKE_C19_WILD_GEAR_CHOICE_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        899 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::PendingPlay { card_id: 250, .. })
            ));
            capture(app, "RIFT_SMOKE_C19_WILD_PENDING_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        900 => {
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::EmpowerPlayed {
                    target: Some(Target::Gear { player: 0, .. }),
                    ..
                })
            ));
            let gear = app.game.players[0]
                .gear
                .iter()
                .find(|g| g.card_id == 250)
                .unwrap();
            assert!(
                !app.game
                    .object_empowered(local_rift::empower::BoardObject::Gear {
                        player: 0,
                        gear_id: gear.id
                    })
            );
            capture(app, "RIFT_SMOKE_C19_EMPOWER_OFFER_SCREENSHOT");
            click(app, 1285.0, 578.0);
        }
        901 => {
            assert_eq!(app.inspect, Some(250));
            capture(app, "RIFT_SMOKE_C19_EMPOWER_INSPECT_SCREENSHOT");
            click(app, 960.0, 704.0);
        }
        902 => {
            assert!(app.inspect.is_none());
            click(app, 1285.0, 510.0);
        }
        903 => {
            assert!(app.game.chain.iter().any(|i| matches!(
                i.ability,
                Some(local_rift::game::ChainAbility::EmpowerLinked { .. })
            )));
            let gear = app.game.players[0]
                .gear
                .iter()
                .find(|g| g.card_id == 250)
                .unwrap();
            assert!(
                !app.game
                    .object_empowered(local_rift::empower::BoardObject::Gear {
                        player: 0,
                        gear_id: gear.id
                    })
            );
            finish_chain_until_choice(app);
            let gear = app.game.players[0]
                .gear
                .iter()
                .find(|g| g.card_id == 250)
                .unwrap();
            assert!(
                app.game
                    .object_empowered(local_rift::empower::BoardObject::Gear {
                        player: 0,
                        gear_id: gear.id
                    })
            );
            capture(app, "RIFT_SMOKE_C19_EMPOWERED_GEAR_SCREENSHOT");
        }
        904 => {
            c19_look(app, 388, &[70, 250, 133]);
        }
        905 => {
            save_state(app);
            click(app, 1285.0, 547.0);
        }
        906 => {
            assert_same_state(app);
            assert_eq!(app.deck_look_draft.as_ref().unwrap().selection, Some(None));
            capture(app, "RIFT_SMOKE_C19_RECYCLE_ALL_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        907 => {
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::EmpowerPlayed { target: None, .. })
            ));
            assert_eq!(app.game.players[0].deck.len(), 3);
            save_state(app);
            click(app, 1285.0, 510.0);
        }
        908 => {
            assert_same_state(app);
            capture(app, "RIFT_SMOKE_C19_NO_PLAYED_OBJECT_SCREENSHOT");
            click(app, 1285.0, 547.0);
        }
        909 => {
            assert!(app.game.decision.is_none());
            assert!(app.game.players[0].gear.is_empty());
        }
        910 => {
            c17_fixture(app, 386);
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                })
                .unwrap();
            finish_chain_until_choice(app);
            app.game.players[0].deck = vec![96, 133, 133, 133, 133, 133, 133];
            app.game.apply(Action::EndTurn).unwrap();
            finish_chain_until_choice(app);
            assert!(
                matches!(&app.game.decision,Some(Decision::RevealedDeck{cards,..}) if cards.len()==7)
            );
        }
        911 => {
            save_state(app);
            capture(app, "RIFT_SMOKE_C19_AURORA_REVEAL_SCREENSHOT");
            click(app, 320.0, 760.0);
        }
        912 => {
            assert_same_state(app);
            assert!(app.deck_look_draft.is_none());
            click(app, 1104.0, 661.0);
        }
        913 => {
            assert_eq!(app.hand_scroll, 2);
            assert_same_state(app);
            capture(app, "RIFT_SMOKE_C19_AURORA_LAST_CARDS_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        914 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::PendingPlay { card_id: 96, .. })
            ));
            capture(app, "RIFT_SMOKE_C19_AURORA_PENDING_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        915 => {
            assert!(app.game.players[0].units.iter().any(|u| u.card_id == 96));
            assert!(app.game.pending_cards().is_empty());
        }
        916 => {
            c17_fixture(app, 387);
            app.game.players[0].hand.clear();
            app.game.apply(Action::EndTurn).unwrap();
            finish_chain_until_choice(app);
            assert_eq!(app.game.priority(), 1);
            app.game.players[1].hand = vec![387];
            app.game.players[1].deck = vec![70, 250, 133, 96, 145];
            app.game.players[1].energy = 30;
            app.game.players[1].energy_pool =
                30 - app.game.players[1].runes.iter().sum::<u8>() as u16;
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                })
                .unwrap();
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::DeckLookPlay { player: 1, .. })
            ));
        }
        917 => {
            assert!(app.deck_look_draft.is_none());
            assert!(
                !app.deck_look_tray(),
                "Opponent private look must not produce a card strip"
            );
            capture(app, "RIFT_SMOKE_C19_OPPONENT_PRIVATE_SCREENSHOT");
        }
        918 => {
            c17_fixture(app, 386);
            app.game.players[0].hand.clear();
            app.game.apply(Action::EndTurn).unwrap();
            finish_chain_until_choice(app);
            app.game.players[1].hand = vec![386];
            app.game.players[1].power_pool = [5; 6];
            app.game.players[1].energy = 30;
            app.game.players[1].energy_pool =
                30 - u16::from(app.game.players[1].runes.iter().sum::<u8>());
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                })
                .unwrap();
            finish_chain_until_choice(app);
            app.game.players[1].deck = vec![96, 133, 133];
            app.game.apply(Action::EndTurn).unwrap();
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::RevealedDeck { player: 1, .. })
            ));
        }
        919 => {
            save_state(app);
            app.next_ai = 0.0;
            app.update();
            assert_same_state(app);
            app.next_ai = f64::INFINITY;
            capture(app, "RIFT_SMOKE_C19_OPPONENT_PUBLIC_SCREENSHOT");
            app.ui.input_override = Some((vec2(670.0, 760.0), false, true));
        }
        920 => {
            assert_eq!(app.inspect, Some(96));
            capture(app, "RIFT_SMOKE_C19_OPPONENT_REVEAL_INSPECT_SCREENSHOT");
            click(app, 960.0, 704.0);
        }
        921 => {
            assert!(app.inspect.is_none());
            assert_same_state(app);
            click(app, 1285.0, 510.0);
        }
        922 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::PendingPlay {
                    player: 1,
                    card_id: 96,
                    ..
                })
            ));
        }
        923 => {}
        924 => {
            c20_look(app, 389, &[394, 3, 394], 0, false);
            let live = app.game.players[0]
                .units
                .iter_mut()
                .find(|unit| unit.id == 17001)
                .unwrap();
            live.card_id = 394;
            live.temporary_buff = 2;
        }
        925 => {
            assert!(
                matches!(&app.game.decision,Some(Decision::DrawLook{cards,eligible,..}) if cards == &[394,3,394] && eligible == &[0,2])
            );
            assert_eq!(app.draw_look_draft.as_ref().unwrap().selection, None);
            save_state(app);
            capture(app, "RIFT_SMOKE_C20_DOUBLE_LOOK_SCREENSHOT");
            click(app, 501.0, 760.0);
        }
        926 => {
            assert_same_state(app);
            assert_eq!(app.draw_look_draft.as_ref().unwrap().selection, None);
            // Actually hover a different physical copy before inspecting this looked copy.
            app.ui.input_override = Some((unit_rect(app, 17001).center(), false, false));
            app.draw();
            assert_eq!(app.hover_object, Some((0, 17001)));
            assert!(
                super::live_object_description(&app.game, 0, 17001, 394)
                    .unwrap()
                    .contains("6 current might")
            );
            app.ui.input_override = Some((vec2(677.0, 760.0), false, true));
        }
        927 => {
            assert_eq!(app.inspect, Some(394));
            assert!(
                app.hover_object.is_none(),
                "nonlive copy must not inherit the board copy's current stats"
            );
            assert_eq!(app.game.catalog[394].might, 4);
            capture(app, "RIFT_SMOKE_C20_LOOK_INSPECT_SCREENSHOT");
            click(app, 960.0, 704.0);
        }
        928 => {
            assert!(app.inspect.is_none());
            assert_same_state(app);
            click(app, 677.0, 760.0);
        }
        929 => {
            assert_eq!(
                app.draw_look_draft.as_ref().unwrap().selection,
                Some(Some(2))
            );
            assert_same_state(app);
            capture(app, "RIFT_SMOKE_C20_DUPLICATE_CHOSEN_SCREENSHOT");
            click(app, 1285.0, 578.0);
        }
        930 => {
            assert_eq!(app.draw_look_draft.as_ref().unwrap().selection, None);
            assert_same_state(app);
            click(app, 677.0, 760.0);
        }
        931 => {
            assert_same_state(app);
            click(app, 1285.0, 510.0);
        }
        932 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::RevealedCard {
                    player: 0,
                    card_id: 394,
                    outcome: local_rift::game::RevealOutcome::Draw,
                    ..
                })
            ));
            assert_eq!(app.game.players[0].hand, vec![394]);
            assert_eq!(app.game.players[0].deck.len(), 2);
            save_state(app);
            capture(app, "RIFT_SMOKE_C20_DRAW_RESULT_SCREENSHOT");
            app.ui.input_override = Some((unit_rect(app, 17001).center(), false, false));
            app.draw();
            assert_eq!(app.hover_object, Some((0, 17001)));
            click(app, 1285.0, 548.0);
        }
        933 => {
            // The public-result frame has rendered and synchronized away the private draft.
            assert!(app.draw_look_draft.is_none());
            assert_eq!(app.inspect, Some(394));
            assert!(
                app.hover_object.is_none(),
                "nonlive copy must not inherit the board copy's current stats"
            );
            assert_eq!(app.game.catalog[394].might, 4);
            assert_same_state(app);
            capture(app, "RIFT_SMOKE_C20_DRAW_INSPECT_SCREENSHOT");
            click(app, 960.0, 704.0);
        }
        934 => {
            assert!(app.inspect.is_none());
            assert_same_state(app);
            click(app, 1285.0, 510.0);
        }
        935 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[0].hand, vec![394]);
            assert!(app.game.players[0].discard.contains(&389));
        }
        936 => c20_look(app, 390, &[26, 67, 17, 117], 0, false),
        937 => {
            assert!(
                matches!(&app.game.decision,Some(Decision::DrawLook{eligible,..}) if eligible==&[0])
            );
            capture(app, "RIFT_SMOKE_C20_FATE_LOOK_SCREENSHOT");
            click(app, 325.0, 760.0);
        }
        938 => {
            assert_eq!(
                app.draw_look_draft.as_ref().unwrap().selection,
                Some(Some(0))
            );
            click(app, 1285.0, 510.0);
        }
        939 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::RevealedCard { card_id: 26, .. })
            ));
            assert_eq!(app.game.players[0].hand, vec![26]);
            capture(app, "RIFT_SMOKE_C20_FATE_DRAW_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        940 => {
            assert!(app.game.decision.is_none());
            c20_look(app, 391, &[230, 119, 117, 16], 0, false);
        }
        941 => {
            assert!(
                matches!(&app.game.decision,Some(Decision::DrawLook{eligible,..}) if eligible==&[0])
            );
            save_state(app);
            capture(app, "RIFT_SMOKE_C20_CAPTAIN_LIMITS_SCREENSHOT");
            click(app, 501.0, 760.0);
        }
        942 => {
            assert_same_state(app);
            assert_eq!(app.draw_look_draft.as_ref().unwrap().selection, None);
            click(app, 325.0, 760.0);
        }
        943 => {
            assert_same_state(app);
            assert_eq!(
                app.draw_look_draft.as_ref().unwrap().selection,
                Some(Some(0))
            );
            click(app, 1285.0, 510.0);
        }
        944 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::RevealedCard { card_id: 230, .. })
            ));
            assert_eq!(app.game.players[0].hand, vec![230]);
            capture(app, "RIFT_SMOKE_C20_CAPTAIN_GEAR_DRAW_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        945 => {
            assert!(app.game.decision.is_none());
            c20_look(app, 389, &[96, 3, 96], 0, false);
        }
        946 => {
            save_state(app);
            click(app, 1285.0, 548.0);
        }
        947 => {
            assert_same_state(app);
            assert_eq!(app.draw_look_draft.as_ref().unwrap().selection, Some(None));
            capture(app, "RIFT_SMOKE_C20_DECLINE_SCREENSHOT");
            click(app, 1285.0, 578.0);
        }
        948 => {
            assert_same_state(app);
            assert_eq!(app.draw_look_draft.as_ref().unwrap().selection, None);
            click(app, 1285.0, 548.0);
        }
        949 => {
            assert_same_state(app);
            click(app, 1285.0, 510.0);
        }
        950 => {
            assert!(app.game.decision.is_none());
            assert!(app.game.players[0].hand.is_empty());
            assert_eq!(app.game.players[0].deck.len(), 3);
            assert!(app.game.players[0].discard.contains(&389));
        }
        951 => c20_move(app, 392, Some(3)),
        952 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::RevealedCard {
                    card_id: 3,
                    outcome: local_rift::game::RevealOutcome::Draw,
                    ..
                })
            ));
            assert_eq!(app.game.players[0].hand, vec![3]);
            capture(app, "RIFT_SMOKE_C20_SMITH_DRAW_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        953 => {
            assert!(app.game.decision.is_none());
            c20_move(app, 392, Some(70));
        }
        954 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::RevealedCard {
                    card_id: 70,
                    outcome: local_rift::game::RevealOutcome::Recycle,
                    ..
                })
            ));
            assert_eq!(app.game.players[0].deck, vec![70, 69]);
            assert!(app.game.players[0].hand.is_empty());
            capture(app, "RIFT_SMOKE_C20_SMITH_RECYCLE_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        955 => {
            assert!(app.game.decision.is_none());
            c20_move(app, 393, Some(71));
        }
        956 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::RevealedCard {
                    card_id: 71,
                    outcome: local_rift::game::RevealOutcome::Draw,
                    ..
                })
            ));
            capture(app, "RIFT_SMOKE_C20_WITNESS_DRAW_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        957 => {
            assert!(app.game.decision.is_none());
            c20_move(app, 393, Some(92));
        }
        958 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::RevealedCard {
                    card_id: 92,
                    outcome: local_rift::game::RevealOutcome::Recycle,
                    ..
                })
            ));
            capture(app, "RIFT_SMOKE_C20_WITNESS_RECYCLE_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        959 => {
            assert!(app.game.decision.is_none());
            c20_move(app, 394, Some(96));
        }
        960 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::RevealedCard {
                    card_id: 96,
                    outcome: local_rift::game::RevealOutcome::Draw,
                    ..
                })
            ));
            capture(app, "RIFT_SMOKE_C20_PAKAA_DRAW_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        961 => {
            assert!(app.game.decision.is_none());
            c20_move(app, 394, Some(7));
        }
        962 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::RevealedCard {
                    card_id: 7,
                    outcome: local_rift::game::RevealOutcome::Trash,
                    ..
                })
            ));
            assert!(app.game.players[0].discard.contains(&7));
            assert_eq!(
                app.game.players[0]
                    .units
                    .iter()
                    .find(|u| u.id == 17001)
                    .unwrap()
                    .temporary_buff,
                2
            );
            capture(app, "RIFT_SMOKE_C20_PAKAA_TRASH_SCREENSHOT");
            save_state(app);
            click(app, 1285.0, 548.0);
        }
        963 => {
            assert_eq!(app.inspect, Some(7));
            assert_same_state(app);
            click(app, 960.0, 704.0);
        }
        964 => {
            assert_same_state(app);
            click(app, 1285.0, 510.0);
        }
        965 => {
            assert!(app.game.decision.is_none());
            // A direct click must replace stale hover identity, even when both
            // physical units share the same printed card and have different stats.
            app.game.players[0]
                .units
                .iter_mut()
                .find(|unit| unit.id == 20002)
                .unwrap()
                .card_id = 394;
            app.hover_object = Some((0, 20002));
            app.hover_card = Some(394);
            assert!(
                super::live_object_description(&app.game, 0, 20002, 394)
                    .unwrap()
                    .contains("4 current might")
            );
            save_state(app);
            let previous_tray = app.tray;
            app.tray = super::Tray::Board;
            app.ui.input_override = Some((vec2(315.0, 760.0), false, true));
            app.draw();
            assert_eq!(app.inspect, Some(394));
            assert_eq!(
                app.hover_object,
                Some((0, 17001)),
                "Board tray latches the exact copy"
            );
            assert_same_state(app);
            click(app, 960.0, 704.0);
            app.draw();
            assert!(app.inspect.is_none());
            app.tray = previous_tray;
            app.hover_object = Some((0, 20002));
            let rect = unit_rect(app, 17001);
            app.ui.input_override = Some((rect.center(), false, true));
        }
        966 => {
            assert_eq!(app.inspect, Some(394));
            assert_eq!(app.hover_object, Some((0, 17001)));
            assert_eq!(app.game.catalog[394].might, 4);
            assert!(
                super::live_object_description(&app.game, 0, 17001, 394)
                    .unwrap()
                    .contains("6 current might")
            );
            assert_same_state(app);
            capture(app, "RIFT_SMOKE_C20_PAKAA_LIVE_SCREENSHOT");
            click(app, 960.0, 704.0);
        }
        967 => {
            assert!(app.inspect.is_none());
            c20_look(app, 389, &[], 0, false);
        }
        968 => {
            assert!(
                matches!(&app.game.decision,Some(Decision::DrawLook{cards,eligible,..}) if cards.is_empty()&&eligible.is_empty())
            );
            C20_SCORES
                .with(|scores| scores.set([app.game.players[0].score, app.game.players[1].score]));
            capture(app, "RIFT_SMOKE_C20_EMPTY_LOOK_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        969 => click(app, 1285.0, 510.0),
        970 => {
            assert!(app.game.decision.is_none());
            assert!(app.game.players[0].hand.is_empty());
            assert!(app.game.players[0].deck.is_empty());
            C20_SCORES.with(|scores| {
                assert_eq!(
                    [app.game.players[0].score, app.game.players[1].score],
                    scores.get(),
                    "an empty look does not cause Burn Out"
                )
            });
        }
        971 => c20_look(app, 389, &[96, 3, 96], 1, false),
        972 => {
            assert!(app.draw_look_draft.is_none());
            assert!(!app.deck_look_tray());
            capture(app, "RIFT_SMOKE_C20_OPPONENT_PRIVATE_SCREENSHOT");
            let Some(Decision::DrawLook { choice_id, .. }) = app.game.decision else {
                panic!("private look")
            };
            app.game
                .apply(Action::ResolveDrawLook {
                    choice_id,
                    index: Some(0),
                })
                .unwrap();
        }
        973 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::RevealedCard {
                    player: 1,
                    card_id: 96,
                    ..
                })
            ));
            save_state(app);
            app.next_ai = 0.0;
            app.update();
            assert_same_state(app);
            app.next_ai = f64::INFINITY;
            capture(app, "RIFT_SMOKE_C20_OPPONENT_PUBLIC_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        974 => {
            assert_eq!(app.inspect, Some(96));
            capture(app, "RIFT_SMOKE_C20_OPPONENT_INSPECT_SCREENSHOT");
            click(app, 960.0, 704.0);
        }
        975 => {
            assert_same_state(app);
            click(app, 1285.0, 510.0);
        }
        976 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[1].hand, vec![96]);
        }
        977 => c20_look(app, 389, &[96, 96, 96], 0, true),
        978 => {
            let Some(Decision::DrawLook { choice_id, .. }) = app.game.decision else {
                panic!("first Repeat look")
            };
            PREDICT_CHOICE.with(|slot| slot.set(choice_id));
            click(app, 325.0, 760.0);
        }
        979 => click(app, 1285.0, 510.0),
        980 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::RevealedCard { .. })
            ));
            click(app, 1285.0, 510.0);
        }
        981 => {
            let Some(Decision::DrawLook { choice_id, .. }) = app.game.decision else {
                panic!("second Repeat look")
            };
            PREDICT_CHOICE.with(|slot| assert_ne!(slot.get(), choice_id));
            // Render the newly reached private choice before checking its reset draft.
            app.draw();
            assert_eq!(app.draw_look_draft.as_ref().unwrap().choice_id, choice_id);
            assert_eq!(app.draw_look_draft.as_ref().unwrap().selection, None);
            capture(app, "RIFT_SMOKE_C20_REPEAT_NEXT_LOOK_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        982 => click(app, 1285.0, 510.0),
        983 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[0].hand, vec![96]);
            assert_eq!(
                app.game.players[0]
                    .discard
                    .iter()
                    .filter(|&&id| id == 389)
                    .count(),
                1
            );
        }
        984 => {
            expansion_fixture(app, 367);
            app.game.players[0].score = 4;
            app.game.players[1].score = 7;
            save_state(app);
            capture(app, "RIFT_SMOKE_C20_SCORE_RAILS_SCREENSHOT");
            click(app, 229.0, 400.0);
        }
        985 => {
            assert_same_state(app);
            assert!(
                app.selection.is_none(),
                "score rails do not select board objects"
            );
            click(app, 1143.0, 230.0);
        }
        986 => {
            assert_same_state(app);
            assert!(
                app.selection.is_none(),
                "opponent score rail has no input overlap"
            );
        }
        987 => {
            assert_same_state(app);
        }
        988 => {
            c21_fixture(app, 396, &[96, 3, 70, 69, 71]);
            click(app, 315.0, 760.0);
        }
        989 => click(app, 1285.0, 510.0),
        990 => {
            finish_chain_until_choice(app);
            assert!(
                matches!(&app.game.decision, Some(Decision::PredictCards { cards, .. }) if cards == &[96, 3, 70, 69, 71])
            );
            let tail = app.game.predict_continuation().unwrap();
            assert_eq!((tail.remaining_draw, tail.remaining_xp), (2, 0));
            assert!(app.game.players[0].hand.is_empty());
            assert!(app.game.chain.iter().any(|item| item.card_id == 396));
            assert!(!app.game.players[0].discard.contains(&396));
            app.sync_predict_draft();
            save_state(app);
            capture(app, "RIFT_SMOKE_C21_PREDICT_FIVE_SCREENSHOT");
            click(app, 514.0, 858.0);
        }
        991 => {
            assert_same_state(app);
            assert_eq!(app.predict_draft.as_ref().unwrap().recycle, vec![1]);
            click(app, 868.0, 858.0);
        }
        992 => {
            assert_eq!(app.predict_draft.as_ref().unwrap().recycle, vec![1, 3]);
            click(app, 1077.0, 822.0);
        }
        993 => click(app, 1077.0, 822.0),
        994 => click(app, 723.0, 822.0),
        995 => {
            assert_same_state(app);
            assert_eq!(app.predict_draft.as_ref().unwrap().top_order, vec![4, 2, 0]);
            capture(app, "RIFT_SMOKE_C21_PREDICT_ORDER_SCREENSHOT");
            app.hover_object = Some((0, 17001));
            app.ui.input_override = Some((vec2(1044.0, 750.0), false, true));
        }
        996 => {
            assert_eq!(app.inspect, Some(71));
            assert!(
                app.hover_object.is_none(),
                "Predict inspects a printed occurrence, not an identical live unit"
            );
            assert_same_state(app);
            click(app, 960.0, 704.0);
        }
        997 => click(app, 1285.0, 510.0),
        998 => {
            assert_eq!(app.game.players[0].hand, vec![71, 70]);
            assert!(app.game.predict_continuation().is_none());
            assert_eq!(
                app.game.players[0]
                    .discard
                    .iter()
                    .filter(|&&card| card == 396)
                    .count(),
                1
            );
            capture(app, "RIFT_SMOKE_C21_CLAIRVOYANCE_DRAWN_SCREENSHOT");
            c21_cast_to_choice(app, 395, &[96, 3, 133, 3]);
            assert!(
                matches!(&app.game.decision, Some(Decision::DrawLook { kind: local_rift::game::DrawLookKind::Ornn, eligible, .. }) if eligible == &[1, 3])
            );
            app.sync_draw_look_draft();
            save_state(app);
            click(app, 325.0, 760.0);
        }
        999 => {
            assert_same_state(app);
            assert_eq!(app.draw_look_draft.as_ref().unwrap().selection, None);
            capture(app, "RIFT_SMOKE_C21_ORNN_LOOK_SCREENSHOT");
            click(app, 853.0, 760.0);
        }
        1000 => {
            assert_eq!(
                app.draw_look_draft.as_ref().unwrap().selection,
                Some(Some(3))
            );
            assert_same_state(app);
            click(app, 1285.0, 579.0);
        }
        1001 => {
            assert_eq!(app.draw_look_draft.as_ref().unwrap().selection, None);
            assert_same_state(app);
            click(app, 501.0, 760.0);
        }
        1002 => click(app, 1285.0, 510.0),
        1003 => {
            assert_eq!(app.game.players[0].hand, vec![3]);
            assert!(matches!(
                app.game.decision,
                Some(Decision::RevealedCard {
                    source_card_id: 395,
                    card_id: 3,
                    ..
                })
            ));
            capture(app, "RIFT_SMOKE_C21_ORNN_DRAW_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        1004 => {
            // A controlled initial battlefield position exercises the actual Hold event.
            c21_fixture(app, 395, &[96, 3, 133, 3, 70]);
            app.game.players[0].hand.clear();
            app.game.players[0].units = vec![public_unit(21001, 395, Some(0))];
            app.game.battlefield_owner[0] = Some(0);
            app.game.apply(Action::EndTurn).unwrap();
            finish_chain_until_choice(app);
            app.game.apply(Action::EndTurn).unwrap();
            finish_chain_until_choice(app);
            assert!(matches!(
                &app.game.decision,
                Some(Decision::DrawLook {
                    source_card_id: 395,
                    ..
                })
            ));
            capture(app, "RIFT_SMOKE_C21_ORNN_HOLD_SCREENSHOT");
            click(app, 1285.0, 547.0);
        }
        1005 => click(app, 1285.0, 510.0),
        1006 => {
            assert!(!matches!(
                app.game.decision,
                Some(Decision::DrawLook { .. })
            ));
            c21_fixture(app, 397, &[70, 96, 69]);
            click(app, 315.0, 760.0);
        }
        1007 => click(app, 1285.0, 510.0),
        1008 => {
            finish_chain_until_choice(app);
            assert_eq!(app.game.players[0].gear[0].card_id, 397);
            assert!(!app.game.players[0].gear[0].ready);
            app.tray = super::Tray::Loadout;
            click(app, 615.0, 760.0);
        }
        1009 => click(app, 1285.0, 417.0),
        1010 => {
            save_state(app);
            capture(app, "RIFT_SMOKE_C21_BLOOM_EXHAUSTED_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        1011 => {
            assert_same_state(app);
            app.game.apply(Action::EndTurn).unwrap();
            finish_chain_until_choice(app);
            app.game.apply(Action::EndTurn).unwrap();
            finish_chain_until_choice(app);
            assert!(app.game.players[0].gear[0].ready);
            app.game.players[0].hand.clear();
            app.game.players[0].deck = vec![96, 70];
            app.game.players[0].xp = 9;
            app.game.players[0]
                .units
                .push(public_unit(21002, 215, None));
            app.selection = None;
            click(app, 615.0, 760.0);
        }
        1012 => click(app, 1285.0, 417.0),
        1013 => {
            save_state(app);
            capture(app, "RIFT_SMOKE_C21_BLOOM_COST_SCREENSHOT");
            click(app, 1285.0, 547.0);
        }
        1014 => {
            assert_same_state(app);
            click(app, 1285.0, 417.0);
        }
        1015 => click(app, 1285.0, 510.0),
        1016 => {
            assert!(app.game.players[0].gear.is_empty());
            assert!(app.game.players[0].discard.contains(&397));
            assert_eq!(app.game.players[0].xp, 9);
            assert!(app.game.players[0].hand.is_empty());
            assert!(app.game.chain.iter().any(|item| item.card_id == 397));
            capture(app, "RIFT_SMOKE_C21_BLOOM_PAID_SCREENSHOT");
            finish_chain_until_choice(app);
        }
        1017 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::PredictCards { .. })
            ));
            let tail = app.game.predict_continuation().unwrap();
            assert_eq!((tail.remaining_draw, tail.remaining_xp), (1, 1));
            assert!(app.game.chain.iter().any(|item| item.card_id == 397));
            assert_eq!(
                app.game.players[0]
                    .units
                    .iter()
                    .find(|unit| unit.id == 21002)
                    .unwrap()
                    .temporary_buff,
                0
            );
            assert_eq!(app.game.players[0].xp, 9);
            assert!(app.game.players[0].hand.is_empty());
            app.sync_predict_draft();
            save_state(app);
            capture(app, "RIFT_SMOKE_C21_BLOOM_PREDICT_SCREENSHOT");
            click(app, 937.0, 827.0);
        }
        1018 => {
            assert_same_state(app);
            assert_eq!(app.predict_draft.as_ref().unwrap().top_order, vec![1, 0]);
            capture(app, "RIFT_SMOKE_C21_BLOOM_ORDER_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        1019 => {
            assert_eq!(app.game.players[0].hand, vec![96]);
            assert_eq!(app.game.players[0].xp, 10);
            assert!(app.game.predict_continuation().is_none());
            assert!(!app.game.chain.iter().any(|item| item.card_id == 397));
            assert!(app.game.chain.iter().any(|item| item.card_id == 215));
            assert_eq!(
                app.game.players[0]
                    .units
                    .iter()
                    .find(|unit| unit.id == 21002)
                    .unwrap()
                    .temporary_buff,
                0
            );
            finish_chain_until_choice(app);
            assert_eq!(
                app.game.players[0]
                    .units
                    .iter()
                    .find(|unit| unit.id == 21002)
                    .unwrap()
                    .temporary_buff,
                1
            );
            capture(app, "RIFT_SMOKE_C21_BLOOM_COMPLETED_SCREENSHOT");
        }
        1020 => {
            c21_fixture(app, 397, &[]);
            app.game.players[0].hand.clear();
            app.game.players[0].gear.push(GearState {
                id: 21003,
                card_id: 397,
                ready: true,
                attached_to: None,
                temporary: false,
                token: false,
            });
            app.tray = super::Tray::Loadout;
            click(app, 615.0, 760.0);
        }
        1021 => click(app, 1285.0, 417.0),
        1022 => click(app, 1285.0, 510.0),
        1023 => {
            finish_chain_until_choice(app);
            assert!(app.game.decision.is_none());
            assert!(app.game.predict_continuation().is_none());
            assert_eq!(
                app.game.players[0].hand,
                vec![397],
                "empty Predict still continues into Draw/Burnout of the sacrificed gear"
            );
            assert_eq!(app.game.players[0].xp, 1);
            assert_eq!(app.game.players[1].score, 1);
            capture(app, "RIFT_SMOKE_C21_EMPTY_PREDICT_TAIL_SCREENSHOT");
        }
        1024 => {
            c21_fixture(app, 396, &[]);
            app.game.players[0].hand.clear();
            app.game.apply(Action::EndTurn).unwrap();
            app.game.players[1].hand = vec![396];
            app.game.players[1].deck = vec![96, 3, 70, 71, 69];
            app.game.players[1].energy = 30;
            app.game.players[1].energy_pool =
                30 - u16::from(app.game.players[1].runes.iter().sum::<u8>());
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
            finish_chain_until_choice(app);
            app.sync_predict_draft();
            assert!(matches!(
                app.game.decision,
                Some(Decision::PredictCards { player: 1, .. })
            ));
            assert!(app.predict_draft.is_none());
            assert_eq!(
                super::opponent_private_choice_copy(app.game.decision.as_ref()),
                Some((
                    "Your opponent is choosing privately.",
                    "The current effect will finish before responses resume."
                )),
                "a mid-effect private choice must not teach that responses are currently available"
            );
            capture(app, "RIFT_SMOKE_C21_OPPONENT_PRIVATE_SCREENSHOT");
        }
        1025 => {
            app.game.step_ai().unwrap();
            assert!(app.game.predict_continuation().is_none());
            assert_eq!(app.game.players[1].hand.len(), 2);
            c21_cast_to_choice(app, 396, &[96, 3, 70, 71]);
            assert!(
                matches!(&app.game.decision, Some(Decision::PredictCards { cards, .. }) if cards.len() == 4)
            );
            app.sync_predict_draft();
            capture(app, "RIFT_SMOKE_C21_PREDICT_FOUR_SCREENSHOT");
            click(app, 1023.0, 858.0);
        }
        1026 => {
            assert_eq!(app.predict_draft.as_ref().unwrap().recycle, vec![3]);
            click(app, 1285.0, 510.0);
        }
        1027 => {
            assert_eq!(app.game.players[0].hand, vec![96, 3]);
            assert!(app.game.predict_continuation().is_none());
        }
        1028 => {
            c22_fixture(app, 399);
            click(app, 315.0, 760.0);
        }
        1029 => click(app, 1285.0, 510.0),
        1030 => {
            let unit = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 399)
                .unwrap();
            assert_eq!(app.game.unit_keywords(unit).unwrap().vision, 1);
            assert_eq!(app.game.chain.len(), 1);
            assert!(
                app.game
                    .chain_name(&app.game.chain[0])
                    .ends_with(" · Vision")
            );
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::Predict {
                    player: 0,
                    card_id: 69
                })
            ));
            click(app, 1285.0, 546.0);
        }
        1031 => {
            assert!(app.game.chain.is_empty());
            assert_eq!(app.game.players[0].deck, vec![96, 71, 69]);
            assert!(app.game.players[0].hand.is_empty());
            let id = app.game.players[0].units[0].id;
            app.ui.input_override = Some((unit_rect(app, id).center(), false, true));
        }
        1032 => {
            let id = app.game.players[0].units[0].id;
            assert_eq!(app.hover_object, Some((0, id)));
            let text = super::live_unit_description(&app.game, 0, id).unwrap();
            assert!(text.contains("VISION · 1 active instance."));
            assert!(text.contains("granted by"));
            assert!(!text.contains("Vision printed"));
            capture(app, "RIFT_SMOKE_C22_FORECASTER_SCREENSHOT");
            click(app, 960.0, 704.0);
        }
        1033 => {
            c22_fixture(app, 400);
            app.game.players[0]
                .units
                .push(public_unit(22001, 398, None));
            click(app, 315.0, 760.0);
        }
        1034 => click(app, 1285.0, 510.0),
        1035 => {
            assert!(
                matches!(&app.game.decision, Some(Decision::OrderTriggers { options, .. }) if options.len() == 2)
            );
            capture(app, "RIFT_SMOKE_C22_TRIGGER_ORDER_SCREENSHOT");
            click(app, 315.0, 760.0);
        }
        1036 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.chain.len(), 2);
            assert_ne!(app.game.chain[0].id, app.game.chain[1].id);
            assert!(
                app.game
                    .chain
                    .iter()
                    .all(|item| app.game.chain_name(item) == "Jeweled Colossus · Vision")
            );
            capture(app, "RIFT_SMOKE_C22_STACKED_CHAIN_SCREENSHOT");
            finish_chain_until_choice(app);
        }
        1037 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::Predict {
                    player: 0,
                    card_id: 69
                })
            ));
            click(app, 1285.0, 546.0);
        }
        1038 => {
            finish_chain_until_choice(app);
            assert!(
                matches!(
                    app.game.decision,
                    Some(Decision::Predict {
                        player: 0,
                        card_id: 69
                    })
                ),
                "keeping lets the next independent Vision see the same card"
            );
            assert_eq!(app.game.players[0].deck.len(), 3);
            assert!(app.game.players[0].hand.is_empty());
            assert_eq!(
                super::own_predict_choice_copy(app.game.decision.as_ref())
                    .unwrap()
                    .0,
                "Finish your Predict choice."
            );
            capture(app, "RIFT_SMOKE_C22_KEEP_AGAIN_SCREENSHOT");
            click(app, 1285.0, 506.0);
        }
        1039 => {
            assert!(app.game.chain.is_empty());
            assert_eq!(app.game.players[0].deck, vec![69, 96, 71]);
            assert!(app.game.players[0].hand.is_empty());
            let id = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 400)
                .unwrap()
                .id;
            app.ui.input_override = Some((unit_rect(app, id).center(), false, true));
        }
        1040 => {
            let id = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 400)
                .unwrap()
                .id;
            assert_eq!(app.hover_object, Some((0, id)));
            let text = super::live_unit_description(&app.game, 0, id).unwrap();
            assert!(text.contains("VISION · 2 active instances"));
            assert!(text.contains("Vision printed"));
            assert!(text.contains("granted by"));
            assert!(text.contains("Shield 1"));
            capture(app, "RIFT_SMOKE_C22_COLOSSUS_SCREENSHOT");
            click(app, 960.0, 704.0);
        }
        1041 => {
            c22_fixture(app, 138);
            app.game.players[0].units =
                vec![public_unit(22001, 398, None), public_unit(22002, 399, None)];
            click(app, 315.0, 760.0);
        }
        1042 => click(app, 1285.0, 510.0),
        1043 => {
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TokenPlacement { .. })
            ));
            capture(app, "RIFT_SMOKE_C22_TOKEN_PLACEMENT_SCREENSHOT");
            click(app, 1285.0, 469.0);
        }
        1044 => {
            assert!(
                matches!(&app.game.decision, Some(Decision::OrderTriggers { options, .. }) if options.len() == 2)
            );
            let token = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.token)
                .unwrap();
            assert_eq!(app.game.unit_keywords(token).unwrap().vision, 2);
            click(app, 315.0, 760.0);
        }
        1045 => {
            assert_eq!(app.game.chain.len(), 2);
            let token = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.token)
                .unwrap();
            for item in &app.game.chain {
                assert_eq!(app.game.chain_name(item), "Mech token · Vision");
                assert_eq!(
                    app.game.chain_source(item),
                    local_rift::game::ChainSource::Unit {
                        player: 0,
                        unit_id: token.id
                    }
                );
            }
            capture(app, "RIFT_SMOKE_C22_TOKEN_CHAIN_SCREENSHOT");
            finish_chain_until_choice(app);
        }
        1046 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::Predict { card_id: 69, .. })
            ));
            click(app, 1285.0, 546.0);
        }
        1047 => {
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::Predict { card_id: 69, .. })
            ));
            click(app, 1285.0, 546.0);
        }
        1048 => {
            assert!(app.game.chain.is_empty());
            assert!(app.game.players[0].hand.is_empty());
            let id = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.token)
                .unwrap()
                .id;
            app.ui.input_override = Some((unit_rect(app, id).center(), false, false));
        }
        1049 => {
            let id = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.token)
                .unwrap()
                .id;
            assert_eq!(app.hover_object, Some((0, id)));
            let text = super::live_unit_description(&app.game, 0, id).unwrap();
            assert!(text.contains("VISION · 2 active instances"));
            assert!(!text.contains("Vision printed"));
            assert!(!text.contains("Iterative Design"));
            capture_unit(app, id, "RIFT_SMOKE_C22_TOKEN_PREVIEW_SCREENSHOT");
            click(app, 960.0, 704.0);
        }
        1050 => {
            c22_fixture(app, 400);
            app.game.players[0]
                .units
                .push(public_unit(22001, 398, None));
            app.game.players[1].units.push(public_unit(22002, 71, None));
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
            while let Some(Decision::OrderTriggers { options, .. }) = &app.game.decision {
                app.game
                    .apply(Action::OrderTrigger {
                        trigger_id: options[0].id,
                    })
                    .unwrap();
            }
            app.game.players[0].hand = vec![16];
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![
                        Target::Unit {
                            player: 0,
                            unit_id: 22001,
                        },
                        Target::Unit {
                            player: 1,
                            unit_id: 22002,
                        },
                    ],
                })
                .unwrap();
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            assert!(
                !app.game.players[0]
                    .units
                    .iter()
                    .any(|unit| unit.id == 22001)
            );
            assert_eq!(
                app.game.chain.len(),
                2,
                "both captured Vision instances survive granting source departure"
            );
        }
        1051 => {
            capture(app, "RIFT_SMOKE_C22_DEPARTED_SOURCE_CHAIN_SCREENSHOT");
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::Predict { card_id: 69, .. })
            ));
            click(app, 1285.0, 546.0);
        }
        1052 => {
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::Predict { card_id: 69, .. })
            ));
            click(app, 1285.0, 546.0);
        }
        1053 => {
            assert!(app.game.chain.is_empty());
            assert_eq!(app.game.players[0].deck, vec![96, 71, 69]);
            let id = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 400)
                .unwrap()
                .id;
            app.ui.input_override = Some((unit_rect(app, id).center(), false, true));
        }
        1054 => {
            let id = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == 400)
                .unwrap()
                .id;
            let text = super::live_unit_description(&app.game, 0, id).unwrap();
            assert!(text.contains("VISION · 1 active instance."));
            assert!(!text.contains("granted by"));
            capture(app, "RIFT_SMOKE_C22_DEPARTED_SOURCE_PREVIEW_SCREENSHOT");
            click(app, 960.0, 704.0);
        }
        1055 => {
            assert!(app.inspect.is_none());
            app.ui.input_override = None;
            app.ui.pointer_override = None;
            app.ui.wheel_override = None;
            println!(
                "PASS: 1056 native UI frames including Vision self-grants, independent Predict keep/recycle controls, printed and granted live provenance, actual Mech token identity, captured triggers after source departure, and all previous 1028 controls."
            );
            return true;
        }
        _ => panic!("UI smoke runner exceeded its 1056-frame script"),
    }
    false
}
