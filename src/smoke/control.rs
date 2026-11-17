//! Real native controls for optional XP, conditional targets and changed control.
use super::*;

const LARGE: u32 = 46901;
const SMALL: u32 = 46902;
const GEAR: u32 = 46903;

fn target(unit_id: u32) -> Target {
    Target::Unit { player: 1, unit_id }
}

fn setup(app: &mut App, card: usize) {
    expansion_fixture(app, card);
    app.game = Game::new_for_agents(0, 0, 469469, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![3, 3];
    app.game.battlefield_owner = vec![Some(1), None];
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
        p.xp = 5;
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
    }
    app.game.players[0].hand = vec![card];
    let mut large = public_unit(LARGE, 71, Some(0));
    large.damage = 1;
    app.game.players[1].units = vec![large, public_unit(SMALL, 193, Some(0))];
    app.game.players[1].gear = vec![GearState {
        id: GEAR,
        card_id: 203,
        lane: None,
        attached_to: Some(LARGE),
        ready: true,
        temporary: false,
        token: false,
    }];
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
            other => panic!("unexpected control choice: {other:?}"),
        };
        app.game.apply(action).unwrap();
    }
    panic!("control chain did not settle");
}

fn assert_taken(app: &App, id: u32, ready: bool) {
    let unit = app.game.players[0]
        .units
        .iter()
        .find(|u| u.id == id)
        .expect("unit taken");
    assert_eq!((unit.lane, unit.ready), (None, ready));
    assert_eq!(
        app.game.board_owner(Target::Unit {
            player: 0,
            unit_id: id
        }),
        Some(1)
    );
    assert!(!app.game.unit_moved_this_turn(0, id));
    assert_eq!(app.game.players[0].score, 0);
    if id == LARGE {
        assert_eq!(unit.damage, 1);
        assert_eq!(app.game.gear_controller(GEAR), Some(1));
        assert_eq!(app.game.players[1].gear[0].attached_to, Some(LARGE));
        let text = super::super::live_unit_description(&app.game, 0, id).unwrap();
        assert!(
            text.contains("opponent still owns")
                && text.contains("Equipment controlled by your opponent")
        );
        assert_eq!(app.game.players[0].units.len(), 1); // no Move token
    }
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100., -100.), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.);
    match frame {
        0 => setup(app, 469),
        1 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        2 => {
            assert_eq!(app.inspect, Some(469));
            capture(app, "RIFT_CONTROL_CONSCRIPTION_INSPECT");
            app.inspect = None;
        }
        3 => click(app, 315., 760.),
        4 => {
            assert!(matches!(app.selection, Some(Selection::Hand(0))));
            advice(app, "Conscription");
            // Draft synchronization happens at the beginning of the next draw.
            capture(app, "RIFT_CONTROL_BEGINNER");
            assert!(
                app.play_draft
                    .as_ref()
                    .is_some_and(|d| d.open && !d.choices.bonus)
            );
            click_unit(app, LARGE);
        }
        5 => {
            assert!(app.targets.is_empty());
            click(app, 150., 600.);
        }
        6 => {
            advice(app, "XP");
            capture(app, "RIFT_CONTROL_ADVANCED");
            click(app, 790., 850.);
        }
        7 => {
            assert!(app.play_draft.as_ref().unwrap().choices.bonus);
            assert_eq!(app.game.players[0].xp, 5);
            let action = app.canonical_action(app.selected_play_action().unwrap());
            let plan = app.game.cost_plan(&action).unwrap();
            assert_eq!(
                super::super::play_ui::resource_total_label(&plan),
                "5 energy · 2 power · 5 XP"
            );
            capture(app, "RIFT_CONTROL_XP_CHOICE");
            click_unit(app, LARGE);
        }
        8 => {
            assert_eq!(app.targets, vec![target(LARGE)]);
            click(app, 790., 850.);
        }
        9 => {
            assert!(!app.play_draft.as_ref().unwrap().choices.bonus && app.targets.is_empty());
            capture(app, "RIFT_CONTROL_XP_REMOVED");
            click(app, 790., 850.);
        }
        10 => click_unit(app, LARGE),
        11 => {
            assert_eq!(app.targets, vec![target(LARGE)]);
            capture(app, "RIFT_CONTROL_PAID_TARGET");
            click(app, 1040., 700.);
        }
        12 => {
            assert!(app.cost_editor.is_some());
            capture(app, "RIFT_CONTROL_XP_COST_REVIEW");
            click(app, 900., 750.);
        }
        13 => {
            assert!(app.cost_editor.is_none());
            assert!(app.play_draft.as_ref().unwrap().choices.bonus);
            click(app, 1280., 510.);
        }
        14 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 469);
            assert_eq!(app.game.players[0].xp, 0);
            assert_eq!(app.game.unit_controller(LARGE), Some(1));
            capture(app, "RIFT_CONTROL_RESPONSE");
            settle(app);
        }
        15 => {
            assert_taken(app, LARGE, false);
            capture_unit(app, LARGE, "RIFT_CONTROL_CONSCRIPTION_TAKEN");
        }
        16 => setup(app, 468),
        17 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        18 => {
            assert_eq!(app.inspect, Some(468));
            capture(app, "RIFT_CONTROL_POSSESSION_INSPECT");
            app.inspect = None;
            advice(app, "Possession");
            capture(app, "RIFT_CONTROL_POSSESSION_BEGINNER");
            click(app, 150., 600.);
        }
        19 => {
            advice(app, "Possession");
            capture(app, "RIFT_CONTROL_POSSESSION_ADVANCED");
            click(app, 315., 760.);
        }
        20 => click_unit(app, LARGE),
        21 => {
            assert_eq!(app.targets, vec![target(LARGE)]);
            click(app, 1280., 510.);
        }
        22 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 468);
            settle(app);
        }
        23 => {
            assert_taken(app, LARGE, true);
            assert_eq!(app.game.players[0].xp, 5);
            capture_unit(app, LARGE, "RIFT_CONTROL_POSSESSION_TAKEN");
        }
        24 => {
            setup(app, 469);
            app.game.players[0].xp = 4;
        }
        25 => click(app, 315., 760.),
        26 => click_unit(app, SMALL),
        27 => {
            assert_eq!(app.targets, vec![target(SMALL)]);
            click(app, 1280., 510.);
        }
        28 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 469);
            settle(app);
        }
        29 => {
            assert_taken(app, SMALL, false);
            assert_eq!(app.game.players[0].xp, 4);
            capture_unit(app, SMALL, "RIFT_CONTROL_UNPAID_TAKEN");
        }
        30 => return true,
        _ => panic!("control smoke exceeded script"),
    }
    false
}
