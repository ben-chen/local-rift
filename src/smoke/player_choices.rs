//! Real card inspection, mandatory gear selection and optional unit return.
use super::*;

const OUR_UNIT: u32 = 48001;
const THEIR_UNIT: u32 = 48002;
const OUR_GEAR: u32 = 48003;
const THEIR_GEAR: u32 = 48004;

fn setup(app: &mut App, card: usize) {
    expansion_fixture(app, card);
    app.game = Game::new_for_agents(0, 3, 480481, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![3, 3];
    app.game.battlefield_owner = vec![None, None];
    for (seat, p) in app.game.players.iter_mut().enumerate() {
        p.units = vec![public_unit(
            if seat == 0 { OUR_UNIT } else { THEIR_UNIT },
            88,
            None,
        )];
        p.gear = vec![GearState {
            id: if seat == 0 { OUR_GEAR } else { THEIR_GEAR },
            card_id: 3,
            attached_to: Some(p.units[0].id),
            lane: None,
            ready: true,
            temporary: false,
            token: false,
        }];
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 12;
        p.energy = 12;
        p.power_pool = [5; 6];
        p.any_power = 0;
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
    }
    app.game.players[0].hand = vec![card];
    app.clear_game_controls();
    app.lesson = None;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.companion.set_level(AdviceLevel::Beginner);
    app.ai_worker.invalidate();
}

fn reach_choice(app: &mut App) {
    for _ in 0..20 {
        if matches!(app.game.decision, Some(Decision::PlayerObject { .. })) {
            app.clear_game_controls();
            return;
        }
        assert!(app.game.decision.is_none() && !app.game.chain.is_empty());
        app.game.apply(Action::Pass).unwrap();
    }
    panic!("each-player resolution did not start");
}

fn decline_opponent(app: &mut App) {
    let Some(Decision::PlayerObject {
        player: 1,
        choice_id,
        ..
    }) = app.game.decision
    else {
        panic!("expected opponent's first Whirlwind choice");
    };
    app.game
        .apply(Action::ChoosePlayerObject {
            choice_id,
            object: None,
        })
        .unwrap();
    app.clear_game_controls();
}

fn advanced_advice(app: &mut App, phrase: &str) {
    app.companion.set_level(AdviceLevel::Advanced);
    app.companion.observe(&app.game);
    for _ in 0..100 {
        if app.companion.message().contains(phrase) {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("missing each-player strategic advice: {phrase}");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100., -100.), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.);
    match frame {
        0 => setup(app, 480),
        1 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        2 => {
            assert_eq!(app.inspect, Some(480));
            capture(app, "RIFT_PLAYER_LOSSES_INSPECT");
            app.inspect = None;
            click(app, 315., 760.);
        }
        3 => {
            assert!(matches!(app.selection, Some(Selection::Hand(0))));
            click(app, 1280., 510.);
        }
        4 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 480);
            reach_choice(app);
            assert_eq!(app.game.priority(), 0);
            capture(app, "RIFT_PLAYER_LOSSES_CHOICE");
            click(app, 1280., 507.); // Mandatory confirmation cannot skip a choice.
        }
        5 => {
            assert_eq!(app.game.players[0].gear.len(), 1);
            assert_eq!(app.game.priority(), 0);
            assert!(app.targets.is_empty());
            click(app, 315., 760.);
        }
        6 => {
            assert_eq!(
                app.targets,
                vec![Target::Gear {
                    player: 0,
                    gear_id: OUR_GEAR
                }]
            );
            capture(app, "RIFT_PLAYER_LOSSES_SELECTED");
            advanced_advice(app, "Keep the gear");
            capture(app, "RIFT_PLAYER_LOSSES_ADVANCED");
            click(app, 1280., 507.);
        }
        7 => {
            assert!(app.game.players[0].gear.is_empty());
            assert_eq!(app.game.players[0].units[0].id, OUR_UNIT);
            assert_eq!(app.game.priority(), 1);
            capture(app, "RIFT_PLAYER_LOSSES_OPPONENT");
            assert!(app.game.step_ai().unwrap());
        }
        8 => {
            assert!(app.game.decision.is_none() && app.game.chain.is_empty());
            assert!(app.game.players[1].gear.is_empty());
            assert_eq!(app.game.players[1].units[0].id, THEIR_UNIT);
            capture(app, "RIFT_PLAYER_LOSSES_FINISHED");
        }
        9 => setup(app, 481),
        10 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        11 => {
            assert_eq!(app.inspect, Some(481));
            capture(app, "RIFT_PLAYER_WHIRLWIND_INSPECT");
            app.inspect = None;
            click(app, 315., 760.);
        }
        12 => click(app, 1280., 510.),
        13 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 481);
            reach_choice(app);
            assert_eq!(app.game.priority(), 1);
            capture(app, "RIFT_PLAYER_WHIRLWIND_OPPONENT");
            decline_opponent(app);
        }
        14 => {
            assert_eq!(app.game.priority(), 0);
            assert!(app.targets.is_empty());
            capture(app, "RIFT_PLAYER_WHIRLWIND_CHOICE");
            click_unit(app, THEIR_UNIT);
        }
        15 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: THEIR_UNIT
                }]
            );
            capture(app, "RIFT_PLAYER_WHIRLWIND_SELECTED");
            advanced_advice(app, "Reassess");
            capture(app, "RIFT_PLAYER_WHIRLWIND_ADVANCED");
            app.ui.input_override = Some((vec2(465., 760.), false, true));
        }
        16 => {
            assert_eq!(app.inspect, Some(88));
            assert_eq!(app.hover_object, Some((1, THEIR_UNIT)));
            capture(app, "RIFT_PLAYER_OBJECT_INSPECT");
            app.inspect = None;
            click(app, 1280., 507.);
        }
        17 => {
            assert!(app.game.decision.is_none() && app.game.chain.is_empty());
            assert!(app.game.players[1].units.is_empty());
            assert_eq!(app.game.players[1].hand, vec![88]);
            assert_eq!(app.game.players[1].gear[0].attached_to, None);
            capture(app, "RIFT_PLAYER_WHIRLWIND_FINISHED");
        }
        18 => {
            setup(app, 481);
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                })
                .unwrap();
            reach_choice(app);
            decline_opponent(app);
        }
        19 => {
            capture(app, "RIFT_PLAYER_WHIRLWIND_DECLINE");
            click(app, 1280., 545.);
        }
        20 => {
            assert!(app.game.decision.is_none() && app.game.chain.is_empty());
            assert_eq!(app.game.players[0].units.len(), 1);
            assert_eq!(app.game.players[1].units.len(), 1);
            assert!(app.game.players[0].discard.contains(&481));
            capture(app, "RIFT_PLAYER_WHIRLWIND_DECLINED");
        }
        21 => return true,
        _ => panic!("each-player smoke exceeded script"),
    }
    false
}
