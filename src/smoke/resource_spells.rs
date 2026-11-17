//! Inspect full text, both Mote levels, and answer the opponent's real spell.
use super::*;

fn setup(app: &mut App, card: usize) {
    recovery::setup(app, card);
    app.game.players[0].discard.clear();
    app.game.players[0].units = vec![public_unit(49101, 71, None)];
    app.game.players[1].units = vec![public_unit(49102, 71, Some(0))];
    app.game.battlefield_owner = vec![Some(1), None];
    for state in &mut app.game.players {
        state.deck = vec![71; 12];
    }
}

fn settle(app: &mut App) {
    for _ in 0..80 {
        if matches!(app.game.decision, Some(Decision::SharedResources { .. }))
            || (app.game.chain.is_empty() && app.game.decision.is_none())
        {
            return;
        }
        let action = match &app.game.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::Legend { .. }) => Action::Choose { accept: false },
            None => Action::Pass,
            other => panic!("unexpected resource choice: {other:?}"),
        };
        app.game.apply(action).unwrap();
    }
    panic!("resource scenario never settled");
}

fn party(app: &mut App) {
    setup(app, 494);
    app.game.apply(Action::EndTurn).unwrap();
    for _ in 0..20 {
        if app.game.active == 1 {
            break;
        }
        app.game.apply(Action::Pass).unwrap();
    }
    assert_eq!(app.game.active, 1);
    app.game.players[0].hand.clear();
    app.game.players[1].hand = vec![494];
    for state in &mut app.game.players {
        state.rune_deck = vec![Domain::Calm; 4];
    }
    app.game
        .apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        })
        .unwrap();
    settle(app);
    assert!(matches!(
        app.game.decision,
        Some(Decision::SharedResources {
            player: 0,
            controller: 1,
            ..
        })
    ));
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100., -100.), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.);
    if frame < 24 {
        let card = 491 + (frame / 6) as usize;
        let labels = [
            ["RIFT_KI_INSPECT", "RIFT_KI_BEGINNER", "RIFT_KI_ADVANCED"],
            [
                "RIFT_WAGE_INSPECT",
                "RIFT_WAGE_BEGINNER",
                "RIFT_WAGE_ADVANCED",
            ],
            [
                "RIFT_INVERT_INSPECT",
                "RIFT_INVERT_BEGINNER",
                "RIFT_INVERT_ADVANCED",
            ],
            [
                "RIFT_PARTY_INSPECT",
                "RIFT_PARTY_BEGINNER",
                "RIFT_PARTY_ADVANCED",
            ],
        ][card - 491];
        let tips = [
            ["Ki Burst costs", "Ki Burst trades"],
            ["The Waters’ Wage plays", "Hide The Waters’ Wage"],
            ["Invert Timelines makes", "Invert Timelines refills"],
            ["Party Favors lets", "Your opponent controls Party"],
        ][card - 491];
        match frame % 6 {
            0 => setup(app, card),
            1 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
            2 => {
                assert_eq!(app.inspect, Some(card));
                capture(app, labels[0]);
                app.inspect = None;
            }
            3 => {
                recovery::advice(app, tips[0]);
                capture(app, labels[1]);
            }
            4 => {
                app.companion.set_level(AdviceLevel::Advanced);
                recovery::advice(app, tips[1]);
                capture(app, labels[2]);
            }
            _ => {}
        }
    } else {
        match frame {
            24 => party(app),
            25 => {
                recovery::advice(app, "Choose Cards");
                capture(app, "RIFT_PARTY_CHOICE");
                click(app, 1280., 505.);
            }
            26 => {
                settle(app);
                assert_eq!(app.game.players[0].hand.len(), 1);
                assert_eq!(app.game.players[1].hand.len(), 1);
                capture(app, "RIFT_PARTY_DRAW");
            }
            27 => party(app),
            28 => {
                app.companion.set_level(AdviceLevel::Advanced);
                recovery::advice(app, "Compare both hand sizes");
                capture(app, "RIFT_PARTY_STRATEGY");
                click(app, 1280., 546.);
            }
            29 => {
                settle(app);
                assert!(app.game.players[0].hand.is_empty());
                assert!(app.game.players[1].hand.is_empty());
                assert_eq!(app.game.players[0].total_runes(), 7);
                // The second player channels three runes on their first turn.
                assert_eq!(app.game.players[1].total_runes(), 10);
                capture(app, "RIFT_PARTY_RUNES");
            }
            30 => {
                setup(app, 491);
                click(app, 315., 760.);
            }
            32 => {
                click_unit(app, 49102);
            }
            33 => {
                click(app, 460., 768.);
            }
            34 => {
                assert_eq!(
                    app.play_draft.as_ref().unwrap().choices.exhaust_unit,
                    Some(49101)
                );
                capture(app, "RIFT_KI_COST");
                click(app, 1285., 510.);
            }
            35 => {
                assert!(!app.game.players[0].units[0].ready);
                settle(app);
                assert!(app.game.players[1].units[0].stunned);
                capture(app, "RIFT_KI_STUN");
            }
            _ => {}
        }
    }
    frame >= 38
}
