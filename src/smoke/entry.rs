//! Exercise actual entry controls, costs, inspection and unobtrusive coaching.
use super::*;

fn settle(app: &mut App) {
    for _ in 0..80 {
        if app.game.decision.is_none() && app.game.chain.is_empty() {
            return;
        }
        let action = match &app.game.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::Legend { .. }) => Action::Choose { accept: false },
            None => Action::Pass,
            other => panic!("unexpected fixture choice: {other:?}"),
        };
        app.game.apply(action).unwrap();
    }
    panic!("entry fixture did not settle");
}

fn setup(app: &mut App, card: usize, spell: bool) {
    expansion_fixture(app, card);
    app.game = Game::new_for_agents(0, 0, 467467, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
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
        p.set_rune_setup([0, 3, 0, 0, 3, 0], [0; 6]).unwrap();
        p.rune_deck = vec![Domain::Calm; 6];
        p.energy_pool = 6;
        p.energy = 12;
        p.power_pool = [5; 6];
        p.any_power = 0;
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
    }
    app.game.players[0].hand = vec![card];
    if spell {
        app.game.players[0].hand.insert(0, 133);
        app.game
            .apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            })
            .unwrap();
        settle(app);
    }
    app.player_deck = 0;
    app.opponent_deck = 0;
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
        0 => setup(app, 467, false),
        1 => {
            advice(app, "Boar");
            capture(app, "RIFT_ENTRY_BOAR_BEGINNER");
            click(app, 150.0, 600.0);
        }
        2 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            advice(app, "Boar");
            capture(app, "RIFT_ENTRY_BOAR_ADVANCED");
            click(app, 315.0, 760.0);
        }
        3 => {
            capture(app, "RIFT_ENTRY_BOAR_CHOICE");
            assert!(
                app.play_draft
                    .as_ref()
                    .is_some_and(|d| d.open && !d.choices.entry_ready)
            );
            click(app, 790.0, 850.0);
        }
        4 => {
            assert!(app.play_draft.as_ref().unwrap().choices.entry_ready);
            click(app, 1280.0, 510.0);
        }
        5 => {
            let unit = app.game.players[0].units.last().expect("Boar played");
            assert!(unit.ready && app.game.movement_locked(0, unit.id));
            assert_eq!(app.game.players[0].energy, 10);
            capture_unit(app, unit.id, "RIFT_ENTRY_BOAR_RESTRICTED");
        }
        6 => setup(app, 466, false),
        7 => click(app, 315.0, 760.0),
        8 => {
            let action = app.selected_play_action().unwrap();
            assert!(app.game.play_choices(&action).unwrap().bonus.is_none());
            capture(app, "RIFT_ENTRY_GUARDIAN_UNAVAILABLE");
            setup(app, 466, true);
        }
        9 => {
            advice(app, "Guardian");
            capture(app, "RIFT_ENTRY_GUARDIAN_BEGINNER");
            click(app, 150.0, 600.0);
        }
        10 => {
            advice(app, "Guardian");
            capture(app, "RIFT_ENTRY_GUARDIAN_ADVANCED");
            click(app, 315.0, 760.0);
        }
        11 => {
            capture(app, "RIFT_ENTRY_GUARDIAN_CHOICE");
            assert!(
                app.play_draft
                    .as_ref()
                    .is_some_and(|d| d.open && !d.choices.bonus)
            );
            click(app, 790.0, 850.0);
        }
        12 => {
            assert!(app.play_draft.as_ref().unwrap().choices.bonus);
            click(app, 1280.0, 510.0);
        }
        13 => {
            let unit = app.game.players[0].units.last().expect("Guardian played");
            assert_eq!(unit.card_id, 466);
            assert!(unit.ready && !app.game.movement_locked(0, unit.id));
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Chaos)],
                4
            );
            capture_unit(app, unit.id, "RIFT_ENTRY_GUARDIAN_READY");
        }
        14 => return true,
        _ => panic!("entry smoke exceeded its script"),
    }
    false
}
