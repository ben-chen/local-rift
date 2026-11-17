//! Native choices distinguish any attacker from enemy attacker and count tags.
use super::*;

fn settle(app: &mut App) {
    for _ in 0..40 {
        if app.game.chain.is_empty() && app.game.decision.is_none() {
            return;
        }
        let action = match &app.game.decision {
            Some(Decision::Legend { .. }) => Action::Choose { accept: false },
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            None => Action::Pass,
            other => panic!("unexpected choice: {other:?}"),
        };
        app.game.apply(action).unwrap();
    }
    panic!("tag/stun chain did not settle");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100., -100.), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.);
    match frame {
        0 => {
            recovery::setup(app, 484);
            app.game.players[0].units = vec![
                public_unit(48401, 71, Some(0)),
                public_unit(48403, 71, None),
            ];
            app.game.players[1].units = vec![public_unit(48402, 71, Some(0))];
            app.game.battlefield_owner[0] = Some(1);
            app.game.showdown = Some(local_rift::game::Showdown {
                lane: 0,
                attacker: 0,
                focus: 0,
                passes: 0,
                combat: true,
            });
        }
        1 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        2 => {
            assert_eq!(app.inspect, Some(484));
            capture(app, "RIFT_TAG_THWONK_INSPECT");
            app.inspect = None;
            click(app, 315., 760.);
        }
        3 => {
            recovery::advice(app, "Thwonk! stuns");
            capture(app, "RIFT_TAG_THWONK_BEGINNER");
            click_unit(app, 48402);
        }
        4 => {
            assert!(app.targets.is_empty(), "defender is not an attacker");
            click_unit(app, 48403);
        }
        5 => {
            assert!(app.targets.is_empty(), "base unit is not an attacker");
            click_unit(app, 48401);
        }
        6 => {
            assert_eq!(app.targets.len(), 1, "friendly attacker is legal");
            capture(app, "RIFT_TAG_THWONK_FRIENDLY");
            click(app, 1220., 468.);
        }
        7 => {
            assert!(app.options.repeat);
            click(app, 1340., 468.);
        }
        8 => {
            assert!(app.editing_repeat);
            click_unit(app, 48401);
        }
        9 => {
            assert_eq!(app.repeat_targets.len(), 1);
            capture(app, "RIFT_TAG_THWONK_REPEAT");
            app.companion.set_level(AdviceLevel::Advanced);
        }
        10 => {
            recovery::advice(app, "Use Thwonk! before");
            capture(app, "RIFT_TAG_THWONK_ADVANCED");
            click(app, 1280., 510.);
        }
        11 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 484);
            settle(app);
        }
        12 => {
            assert!(app.game.players[0].units[0].stunned);
            assert!(!app.game.players[1].units[0].stunned);
            capture(app, "RIFT_TAG_THWONK_STUNNED");
        }
        13 => {
            recovery::setup(app, 485);
            app.game.players[0].units = [258, 107, 338, 88]
                .into_iter()
                .enumerate()
                .map(|(i, card)| public_unit(48501 + i as u32, card, None))
                .collect();
            app.game.players[1].units = vec![public_unit(48510, 71, Some(0))];
        }
        14 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        15 => {
            assert_eq!(app.inspect, Some(485));
            capture(app, "RIFT_TAG_FRIENDSHIP_INSPECT");
            app.inspect = None;
            click(app, 315., 760.);
        }
        16 => {
            recovery::advice(app, "Friendship counts");
            capture(app, "RIFT_TAG_FRIENDSHIP_BEGINNER");
            click_unit(app, 48510);
        }
        17 => {
            assert_eq!(app.targets.len(), 1);
            app.companion.set_level(AdviceLevel::Advanced);
        }
        18 => {
            recovery::advice(app, "Friendship can turn");
            capture(app, "RIFT_TAG_FRIENDSHIP_ADVANCED");
            click(app, 1280., 510.);
        }
        19 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 485);
            settle(app);
        }
        20 => {
            assert_eq!(app.game.players[1].units[0].temporary_buff, 4);
            capture(app, "RIFT_TAG_FRIENDSHIP_RESOLVED");
        }
        21 => {
            println!("tag-stun smoke passed");
            return true;
        }
        _ => {}
    }
    false
}
