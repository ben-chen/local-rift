//! Real mode controls, counter targets, token placement and both advice levels.
use super::*;

fn setup(app: &mut App, counter: bool) {
    expansion_fixture(app, 463);
    app.game = Game::new_for_agents(0, 3, 463463, Difficulty::Easy);
    app.game
        .choose_starting_player(usize::from(counter))
        .unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![3, 3];
    app.game.battlefield_owner = vec![Some(0), None];
    for p in &mut app.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.set_rune_setup([0, 6, 0, 0, 0, 0], [0; 6]).unwrap();
        p.rune_deck = vec![Domain::Calm; 6];
        p.energy_pool = 6;
        p.energy = 12;
        p.power_pool = [5; 6];
        p.any_power = 0;
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
    }
    app.game.players[0].hand = vec![463];
    app.game.players[0].units = vec![public_unit(46301, 110, Some(0))];
    if counter {
        app.game.players[1].hand = vec![78];
        app.game
            .apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            })
            .unwrap();
        app.game.apply(Action::Pass).unwrap();
    }
    app.player_deck = 0;
    app.opponent_deck = 3;
    app.fields = [3, 3];
    app.clear_game_controls();
    app.lesson = None;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.companion.set_level(AdviceLevel::Beginner);
    app.ai_worker.invalidate();
}

fn advice(app: &mut App, name: &str) {
    app.companion.observe(&app.game);
    for _ in 0..100 {
        if app.companion.message().contains(name) {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("missing {name} advice");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app, true),
        1 => app.ui.input_override = Some((vec2(315.0, 760.0), false, true)),
        2 => {
            assert_eq!(app.inspect, Some(463));
            capture(app, "RIFT_FLURRY_INSPECT");
            app.inspect = None;
        }
        3 => click(app, 315.0, 760.0),
        4 => {
            capture(app, "RIFT_FLURRY_MODES");
            assert!(
                app.play_draft
                    .as_ref()
                    .is_some_and(|d| d.open && d.choices.mode.is_none()),
                "selection={} card={:?} draft={:?} priority={} toast={} query={:?}",
                app.selection == Some(Selection::Hand(0)),
                app.selected_card_id(),
                app.play_draft.as_ref().map(|d| (d.open, d.choices.mode)),
                app.game.priority(),
                app.toast,
                app.selected_play_action()
                    .map(|a| app.game.play_choices(&a).map(|v| v.modes.len()))
            );
            click(app, 795.0, 770.0);
        }
        5 => {
            assert_eq!(app.play_draft.as_ref().unwrap().choices.mode, Some(0));
            click(app, 1300.0, 320.0);
        }
        6 => {
            assert_eq!(
                app.targets,
                vec![Target::Spell {
                    chain_id: app.game.chain[0].id
                }]
            );
            capture(app, "RIFT_FLURRY_COUNTER_TARGET");
            click(app, 1280.0, 510.0);
        }
        7 => {
            assert_eq!(app.game.chain.len(), 2);
            advice(app, "Flurry");
            capture(app, "RIFT_FLURRY_BEGINNER");
            click(app, 150.0, 600.0);
        }
        8 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            advice(app, "Flurry");
            capture(app, "RIFT_FLURRY_ADVANCED");
            click(app, 1285.0, 595.0);
        }
        9 => {
            assert_eq!(app.game.priority(), 1);
            app.game.apply(Action::Pass).unwrap();
        }
        10 => {
            assert!(app.game.chain.is_empty());
            assert!(app.game.players[1].discard.contains(&78));
            setup(app, false);
        }
        11 => click(app, 315.0, 760.0),
        12 => click(app, 1005.0, 770.0),
        13 => {
            assert_eq!(app.play_draft.as_ref().unwrap().choices.mode, Some(1));
            assert!(app.targets.is_empty());
            capture(app, "RIFT_FLURRY_BIRD_MODE");
            click(app, 1280.0, 510.0);
        }
        14 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 463);
            finish_chain_until_choice(app);
        }
        15 => {
            assert!(app.game.players[0].discard.contains(&463));
            capture(app, "RIFT_FLURRY_PLACEMENT");
            click(app, 1285.0, 470.0);
        }
        16 => {
            assert_eq!(bird_count(app), 1);
            click(app, 1285.0, 505.0);
        }
        17 => {
            assert_eq!(bird_count(app), 2);
            click(app, 1285.0, 470.0);
        }
        18 => {
            assert_eq!(bird_count(app), 3);
            click(app, 1285.0, 505.0);
        }
        19 => {
            assert_eq!(bird_count(app), 4);
            assert!(app.game.decision.is_none());
            let id = app.game.players[0]
                .units
                .iter()
                .find(|u| u.token)
                .unwrap()
                .id;
            app.ui.input_override = Some((unit_rect(app, id).center(), false, false));
        }
        20 => {
            let id = app.game.players[0]
                .units
                .iter()
                .find(|u| u.token)
                .unwrap()
                .id;
            assert_eq!(app.hover_object, Some((0, id)));
            capture_unit(app, id, "RIFT_FLURRY_BIRDS");
        }
        _ => {}
    }
    frame >= 21
}

fn bird_count(app: &App) -> usize {
    app.game.players[0]
        .units
        .iter()
        .filter(|u| u.token && u.card_id == 463)
        .count()
}
