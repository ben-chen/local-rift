//! Actual destination, target and trash controls for a complete Hand → Flow cycle.
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
            other => panic!("unexpected Twilight choice: {other:?}"),
        };
        app.game.apply(action).unwrap();
    }
    panic!("Twilight chain did not settle");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100., -100.), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.);
    match frame {
        0 => {
            recovery::setup(app, 486);
            app.game.players[0].discard.clear();
            let mut ally = public_unit(48601, 13, None);
            ally.ready = false;
            ally.damage = 1;
            app.game.players[0].units = vec![ally];
            let mut enemy = public_unit(48602, 13, Some(0));
            enemy.ready = false;
            app.game.players[1].units = vec![enemy, public_unit(48603, 71, Some(0))];
            app.game.battlefield_owner = vec![Some(1), None];
        }
        1 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        2 => {
            assert_eq!(app.inspect, Some(486));
            capture(app, "RIFT_TWILIGHT_INSPECT");
            app.inspect = None;
            click(app, 315., 760.);
        }
        4 => {
            assert!(app.play_draft.as_ref().unwrap().open);
            recovery::advice(app, "Twilight Step moves");
            capture(app, "RIFT_TWILIGHT_BEGINNER");
            click(app, 1040., 768.);
        }
        5 => {
            assert_eq!(app.destination, Some(1));
            click_unit(app, 48603);
        }
        6 => {
            assert!(app.targets.is_empty(), "5 Might cannot be chosen");
            click_unit(app, 48601);
        }
        7 => {
            assert_eq!(app.targets.len(), 1, "friendly 3-Might unit is legal");
            app.companion.set_level(AdviceLevel::Advanced);
            recovery::advice(app, "Use Twilight Step");
            capture(app, "RIFT_TWILIGHT_ADVANCED");
            click(app, 1285., 510.);
        }
        8 => {
            let spell = app
                .game
                .chain
                .iter()
                .find(|item| item.card_id == 486)
                .unwrap();
            assert_eq!(spell.targets.len(), 1);
            assert!(!spell.banish_after);
            assert_eq!(app.game.players[0].energy, 10);
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Chaos)],
                4
            );
            settle(app);
        }
        9 => {
            let ally = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.id == 48601)
                .unwrap();
            assert_eq!(ally.lane, Some(1));
            assert!(!ally.ready, "Move does not ready the unit");
            assert_eq!(ally.damage, 1);
            assert_eq!(app.game.players[0].discard, [486]);
            capture(app, "RIFT_TWILIGHT_MOVED");
            click(app, 500., 663.);
        }
        10 => {
            assert!(app.tray == super::super::Tray::Trash);
            click(app, 315., 760.);
        }
        12 => {
            assert!(matches!(app.selection, Some(Selection::Trash(0))));
            assert_eq!(app.flow_source, Some(FlowSource::Printed));
            assert_eq!(app.destination, None);
            click_unit(app, 48602);
        }
        13 => {
            assert_eq!(app.targets.len(), 1, "enemy 3-Might unit is legal");
            assert!(app.game.showdown.is_some());
            capture(app, "RIFT_TWILIGHT_TIMING");
            click(app, 1285., 510.);
        }
        14 => {
            assert!(
                app.game.chain.is_empty(),
                "Flow retains ordinary spell timing"
            );
            assert_eq!(app.game.players[0].energy, 10);
            assert_eq!(app.game.players[0].discard, [486]);
            click(app, 1285., 590.);
        }
        15 => {
            assert_eq!(app.game.showdown.as_ref().unwrap().focus, 1);
            app.game.apply(Action::Pass).unwrap();
            settle(app);
            assert!(app.game.showdown.is_none());
            assert_eq!(app.game.battlefield_owner[1], Some(0));
            click(app, 500., 663.);
        }
        16 => click(app, 315., 760.),
        18 => {
            assert!(matches!(app.selection, Some(Selection::Trash(0))));
            assert_eq!(app.flow_source, Some(FlowSource::Printed));
            click_unit(app, 48602);
        }
        19 => {
            assert_eq!(app.targets.len(), 1);
            capture(app, "RIFT_TWILIGHT_FLOW");
            click(app, 1285., 510.);
        }
        20 => {
            let spell = app
                .game
                .chain
                .iter()
                .find(|item| item.card_id == 486)
                .unwrap();
            assert!(spell.banish_after);
            assert_eq!(app.game.players[0].energy, 6);
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Chaos)],
                3
            );
            settle(app);
        }
        21 => {
            let enemy = app.game.players[1]
                .units
                .iter()
                .find(|unit| unit.id == 48602)
                .unwrap();
            assert_eq!(enemy.lane, None);
            assert!(!enemy.ready);
            assert!(app.game.players[0].discard.is_empty());
            assert!(app.game.players[0].banish.contains(&486));
            capture(app, "RIFT_TWILIGHT_BANISHED");
        }
        22 => {
            println!("twilight smoke passed");
            return true;
        }
        _ => {}
    }
    false
}
