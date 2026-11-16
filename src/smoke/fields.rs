//! Board explanations and real movement / private ordering controls.
use super::*;
use local_rift::battlefields::{BACK_ALLEY_BAR_ID, CANDLELIT_SANCTUM_ID, FORGOTTEN_MONUMENT_ID};

fn setup(app: &mut App, actor: usize, fields: [usize; 2], lane: Option<usize>) {
    expansion_fixture(app, 71);
    app.game = Game::new_for_agents(0, 0, 182019, Difficulty::Easy);
    app.game.choose_starting_player(actor).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = fields.to_vec();
    app.game.battlefield_owner = vec![None; 2];
    for player in &mut app.game.players {
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.hand.clear();
        player.champion = None;
        player.deck = vec![71, 70, 69, 68];
    }
    app.game.players[actor].units = vec![public_unit(182001, 71, lane)];
    if let Some(lane) = lane {
        app.game.battlefield_owner[lane] = Some(actor);
    }
    app.player_deck = 0;
    app.opponent_deck = 0;
    app.fields = fields;
    app.clear_game_controls();
    app.lesson = None;
    app.field_inspect = None;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.companion.set_level(AdviceLevel::Beginner);
    app.ai_worker.invalidate();
}

fn settle(app: &mut App) {
    for _ in 0..60 {
        if matches!(app.game.decision, Some(Decision::PredictCards { .. })) {
            return;
        }
        if app.game.chain.is_empty() && app.game.decision.is_none() && app.game.showdown.is_none() {
            return;
        }
        let action = match &app.game.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::Legend { .. }) => Action::Choose { accept: false },
            _ => Action::Pass,
        };
        app.game.apply(action).unwrap();
    }
    panic!("field UI fixture did not settle");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100., -100.), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.);
    match frame {
        0 => setup(app, 0, [FORGOTTEN_MONUMENT_ID, BACK_ALLEY_BAR_ID], Some(1)),
        1 => capture(app, "RIFT_FIELDS_MAT"),
        2 => app.field_inspect = Some(FORGOTTEN_MONUMENT_ID),
        3 => {
            capture(app, "RIFT_FIELDS_MONUMENT");
            click(app, 650., 505.);
        }
        4 => app.field_inspect = Some(BACK_ALLEY_BAR_ID),
        5 => {
            capture(app, "RIFT_FIELDS_BAR");
            click(app, 650., 505.);
        }
        6 => click_unit(app, 182001),
        7 => click(app, 1260., 503.), // Standard move back to Base.
        8 => {
            assert_eq!(app.game.players[0].units[0].lane, None);
            assert_eq!(app.game.players[0].units[0].temporary_buff, 0);
            capture(app, "RIFT_FIELDS_RESPONSE");
            settle(app);
        }
        9 => {
            assert_eq!(app.game.players[0].units[0].temporary_buff, 1);
            assert_eq!(app.game.players[0].score, 0);
            capture_unit(app, 182001, "RIFT_FIELDS_BONUS");
        }
        10 => setup(app, 0, [CANDLELIT_SANCTUM_ID, 3], None),
        11 => click_unit(app, 182001),
        12 => click_field(app, 0),
        13 => {
            settle(app);
            app.sync_predict_draft();
            assert!(matches!(
                app.game.decision,
                Some(Decision::PredictCards {
                    source: Some(_),
                    player: 0,
                    ..
                })
            ));
            assert_eq!(
                super::super::own_predict_choice_copy(app.game.decision.as_ref())
                    .unwrap()
                    .0,
                "Finish your deck look."
            );
            capture(app, "RIFT_FIELDS_LOOK");
        }
        14 => click(app, 920., 825.),
        15 => {
            assert_eq!(app.predict_draft.as_ref().unwrap().top_order, vec![1, 0]);
            capture(app, "RIFT_FIELDS_ORDER");
            click(app, 360., 858.);
        }
        16 => {
            assert_eq!(app.predict_draft.as_ref().unwrap().recycle, vec![0]);
            capture(app, "RIFT_FIELDS_RECYCLE");
            click(app, 1260., 510.);
        }
        17 => {
            assert_eq!(app.game.players[0].deck, vec![68, 71, 70, 69]);
            assert!(app.game.decision.is_none());
        }
        18 => app.field_inspect = Some(CANDLELIT_SANCTUM_ID),
        19 => {
            capture(app, "RIFT_FIELDS_SANCTUM");
            click(app, 650., 505.);
        }
        20 => {
            setup(app, 1, [CANDLELIT_SANCTUM_ID, 3], None);
            app.game
                .apply(Action::Move {
                    unit_id: 182001,
                    lane: Some(0),
                })
                .unwrap();
            settle(app);
            app.sync_predict_draft();
        }
        21 => {
            assert!(app.predict_draft.is_none());
            assert!(
                super::super::opponent_private_choice_copy(app.game.decision.as_ref()).is_some()
            );
            capture(app, "RIFT_FIELDS_PRIVATE");
        }
        22 => return true,
        _ => panic!("field smoke exceeded its script"),
    }
    false
}
