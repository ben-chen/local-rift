//! Exercise Temptation's printed Repeat through the shared play-choice controls.
use super::*;
use local_rift::play::MoveDestination;

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100., -100.), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.);
    match frame {
        0 => {
            recovery::setup(app, 487);
            app.game.players[0].discard.clear();
            app.game.players[0].units = vec![public_unit(48700, 13, None)];
            let mut first = public_unit(48701, 13, Some(0));
            first.ready = false;
            first.damage = 1;
            let mut second = public_unit(48702, 71, Some(1));
            second.ready = false;
            app.game.players[1].units = vec![first, second];
            app.game.battlefield_owner = vec![Some(1), Some(1)];
        }
        1 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        2 => {
            assert_eq!(app.inspect, Some(487));
            capture(app, "RIFT_TEMPTATION_INSPECT");
            app.inspect = None;
            click(app, 315., 760.);
        }
        // A full draw frame opens the play-choice panel after hand selection.
        4 => {
            assert!(app.play_draft.as_ref().unwrap().open);
            recovery::advice(app, "Temptation moves");
            capture(app, "RIFT_TEMPTATION_BEGINNER");
            click_unit(app, 48701);
        }
        5 => {
            assert!(app.targets.is_empty(), "the enemy base has no unit anchor");
            capture(app, "RIFT_TEMPTATION_EMPTY_DESTINATION");
            click(app, 1040., 768.);
        }
        6 => {
            assert_eq!(app.destination, Some(1));
            click_unit(app, 48700);
        }
        7 => {
            assert!(app.targets.is_empty(), "Temptation targets an enemy");
            click_unit(app, 48701);
        }
        8 => {
            assert_eq!(app.targets.len(), 1);
            app.companion.set_level(AdviceLevel::Advanced);
            recovery::advice(app, "Use Temptation");
            capture(app, "RIFT_TEMPTATION_ADVANCED");
            click(app, 400., 741.);
        }
        9 => {
            let draft = app.play_draft.as_ref().unwrap();
            assert_eq!(draft.execution, 1);
            assert_eq!(draft.choices.repeats.len(), 1);
            click(app, 900., 768.);
        }
        10 => {
            assert_eq!(app.destination, Some(1), "original destination is retained");
            assert_eq!(
                app.play_draft.as_ref().unwrap().choices.repeats[0]
                    .choices
                    .destination,
                Some(MoveDestination::Battlefield { lane: 0 })
            );
            click_unit(app, 48702);
        }
        11 => {
            assert_eq!(
                app.play_draft.as_ref().unwrap().choices.repeats[0]
                    .choices
                    .targets
                    .len(),
                1
            );
            capture(app, "RIFT_TEMPTATION_REPEAT");
            click(app, 1285., 510.);
        }
        12 => {
            let spell = app
                .game
                .chain
                .iter()
                .find(|item| item.card_id == 487)
                .unwrap();
            assert_eq!(spell.executions.len(), 2);
            assert_eq!(
                spell.executions[0].destination,
                Some(MoveDestination::Battlefield { lane: 1 })
            );
            assert_eq!(
                spell.executions[1].destination,
                Some(MoveDestination::Battlefield { lane: 0 })
            );
            assert_eq!(app.game.players[0].energy, 8);
            assert_eq!(app.game.players[0].power_pool, [5; 6]);
            capture(app, "RIFT_TEMPTATION_CHAIN");
            for _ in 0..40 {
                if app.game.chain.is_empty() && app.game.decision.is_none() {
                    break;
                }
                let action = match &app.game.decision {
                    Some(Decision::Legend { .. }) => Action::Choose { accept: false },
                    Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                        trigger_id: options[0].id,
                    },
                    None => Action::Pass,
                    other => panic!("unexpected Temptation choice: {other:?}"),
                };
                app.game.apply(action).unwrap();
            }
            assert!(app.game.chain.is_empty());
            assert!(app.game.decision.is_none());
        }
        13 => {
            let units = &app.game.players[1].units;
            let first = units.iter().find(|unit| unit.id == 48701).unwrap();
            let second = units.iter().find(|unit| unit.id == 48702).unwrap();
            assert_eq!(first.lane, Some(1));
            assert_eq!(second.lane, None, "the now-empty destination causes Recall");
            assert!(!first.ready && !second.ready);
            assert_eq!(first.damage, 1);
            assert!(app.game.unit_moved_this_turn(1, 48701));
            assert!(
                !app.game.unit_moved_this_turn(1, 48702),
                "Recall is not a Move"
            );
            assert_eq!(app.game.players[0].discard, [487]);
            capture(app, "RIFT_TEMPTATION_RESOLVED");
        }
        14 => {
            println!("temptation smoke passed");
            return true;
        }
        _ => {}
    }
    false
}
