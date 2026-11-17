//! Actual inspection, deployment, public trash selection and private Vision controls.
use super::*;

pub(super) fn setup(app: &mut App, card: usize) {
    expansion_fixture(app, card);
    app.game = Game::new_for_agents(0, 0, 475475, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![3, 3];
    app.game.battlefield_owner = vec![None, Some(1)];
    for player in &mut app.game.players {
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.hand.clear();
        player.discard.clear();
        player.champion = None;
        player.deck = vec![96, 71, 8];
        player.set_rune_setup([0, 3, 0, 0, 3, 0], [0; 6]).unwrap();
        player.energy_pool = 6;
        player.energy = 12;
        player.power_pool = [5; 6];
        player.any_power = 0;
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
    }
    app.game.players[0].hand = vec![card];
    app.game.players[0].discard = if card == 474 {
        vec![8, 96, 71]
    } else {
        vec![96, 8, 9]
    };
    app.player_deck = 0;
    app.opponent_deck = 0;
    app.fields = [3, 3];
    app.clear_game_controls();
    app.inspect = None;
    app.field_inspect = None;
    app.lesson = None;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.companion.set_level(AdviceLevel::Beginner);
    app.ai_worker.invalidate();
}

fn advance(app: &mut App) {
    for _ in 0..80 {
        if matches!(
            app.game.decision,
            Some(Decision::TrashTarget { .. } | Decision::Predict { .. })
        ) {
            return;
        }
        if app.game.decision.is_none() && app.game.chain.is_empty() && app.game.showdown.is_none() {
            return;
        }
        let action = match &app.game.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::Legend { .. }) => Action::Choose { accept: false },
            None => Action::Pass,
            other => panic!("unexpected recovery choice: {other:?}"),
        };
        app.game.apply(action).unwrap();
    }
    panic!("recovery scenario did not settle");
}

pub(super) fn advice(app: &mut App, text: &str) {
    app.companion.observe(&app.game);
    for _ in 0..100 {
        if app.companion.message().contains(text) {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("missing {text} advice");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100., -100.), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.);
    if frame < 16 {
        let source = if frame < 8 { 474 } else { 475 };
        let chosen = if source == 474 { 96 } else { 8 };
        let captures = if source == 474 {
            [
                "RIFT_RECOVERY_ATTENDANT_INSPECT",
                "RIFT_RECOVERY_ATTENDANT_BEGINNER",
                "RIFT_RECOVERY_ATTENDANT_ADVANCED",
                "RIFT_RECOVERY_UNIT_CHOICE",
                "RIFT_RECOVERY_UNIT_CHAIN",
                "RIFT_RECOVERY_UNIT_RETURNED",
            ]
        } else {
            [
                "RIFT_RECOVERY_ANNIE_INSPECT",
                "RIFT_RECOVERY_ANNIE_BEGINNER",
                "RIFT_RECOVERY_ANNIE_ADVANCED",
                "RIFT_RECOVERY_SPELL_CHOICE",
                "RIFT_RECOVERY_SPELL_CHAIN",
                "RIFT_RECOVERY_SPELL_RETURNED",
            ]
        };
        match frame % 8 {
            0 => setup(app, source),
            1 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
            2 => {
                assert_eq!(app.inspect, Some(source));
                capture(app, captures[0]);
                app.inspect = None;
                click(app, 315., 760.);
            }
            3 => {
                advice(app, "Cemetery Attendant");
                capture(app, captures[1]);
                click(app, 150., 600.);
            }
            4 => {
                assert_eq!(app.companion.level(), AdviceLevel::Advanced);
                advice(app, "Recovery adds");
                capture(app, captures[2]);
                click(app, 1280., 510.);
            }
            5 => {
                advance(app);
                assert!(
                    matches!(app.game.decision, Some(Decision::TrashTarget { source_card_id, .. }) if source_card_id == source)
                );
                assert!(app.game.players[0].hand.is_empty());
                capture(app, captures[3]);
                click(app, 315., 760.); // First eligible tile, original trash index 1.
            }
            6 => {
                assert!(matches!(app.game.chain.last().unwrap().ability,
                    Some(local_rift::game::ChainAbility::ReturnChosen { trash_index: 1, card_id }) if card_id == chosen));
                assert!(app.game.players[0].hand.is_empty());
                capture(app, captures[4]);
                advance(app);
            }
            7 => {
                assert_eq!(app.game.players[0].hand, vec![chosen]);
                assert_eq!(app.game.players[0].units.len(), 1);
                capture(app, captures[5]);
            }
            _ => unreachable!(),
        }
        return false;
    }
    match frame {
        16 => setup(app, 473),
        17 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        18 => {
            assert_eq!(app.inspect, Some(473));
            capture(app, "RIFT_RECOVERY_SCOUT_INSPECT");
            app.inspect = None;
            click(app, 315., 760.);
        }
        19 => {
            advice(app, "Sai Scout");
            capture(app, "RIFT_RECOVERY_SCOUT_BEGINNER");
            click(app, 150., 600.);
        }
        20 => {
            advice(app, "Sai Scout");
            capture(app, "RIFT_RECOVERY_SCOUT_ADVANCED");
            click(app, 1220., 548.);
        }
        21 => {
            advance(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::Predict {
                    player: 0,
                    card_id: 8
                })
            ));
            assert_eq!(app.game.players[0].units[0].lane, Some(0));
            capture(app, "RIFT_RECOVERY_VISION_KEEP_CHOICE");
            click(app, 1280., 548.);
        }
        22 => {
            advance(app);
            assert_eq!(app.game.players[0].deck, vec![96, 71, 8]);
            capture(app, "RIFT_RECOVERY_VISION_KEPT");
        }
        23 => setup(app, 473),
        24 => click(app, 315., 760.),
        25 => click(app, 1220., 548.),
        26 => {
            advance(app);
            assert!(matches!(app.game.decision, Some(Decision::Predict { .. })));
            capture(app, "RIFT_RECOVERY_VISION_RECYCLE_CHOICE");
            click(app, 1280., 510.);
        }
        27 => {
            advance(app);
            assert_eq!(app.game.players[0].deck, vec![8, 96, 71]);
            capture(app, "RIFT_RECOVERY_VISION_RECYCLED");
        }
        28 => {
            setup(app, 474);
            app.game.players[0].discard = vec![8];
        }
        29 => click(app, 315., 760.),
        30 => click(app, 1280., 510.),
        31 => {
            advance(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TrashTarget { .. })
            ));
            capture(app, "RIFT_RECOVERY_NO_ELIGIBLE_TARGET");
            click(app, 1280., 548.);
        }
        32 => {
            advance(app);
            assert!(app.game.decision.is_none() && app.game.players[0].hand.is_empty());
            assert_eq!(app.game.players[0].discard, vec![8]);
        }
        33 => return true,
        _ => panic!("recovery smoke exceeded its frames"),
    }
    false
}
