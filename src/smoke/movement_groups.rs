//! Shared target controls, a real response and the remaining-subset decision.
use super::*;

fn target(player: usize, unit_id: u32) -> Target {
    Target::Unit { player, unit_id }
}

fn settle_until_subset(app: &mut App) {
    for _ in 0..60 {
        if matches!(app.game.decision, Some(Decision::ResolutionSubset { .. }))
            || (app.game.decision.is_none() && app.game.chain.is_empty())
        {
            return;
        }
        let action = match &app.game.decision {
            Some(Decision::Legend { .. }) => Action::Choose { accept: false },
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            None => Action::Pass,
            other => panic!("unexpected movement choice: {other:?}"),
        };
        app.game.apply(action).unwrap();
    }
    panic!("movement scenario did not settle");
}

fn setup(app: &mut App, card: usize) {
    recovery::setup(app, card);
    app.game.players[0].discard.clear();
    let owner = usize::from(card != 488);
    let mut first = public_unit(48901, 13, Some(0));
    first.ready = false;
    first.damage = 1;
    let mut second = public_unit(48902, 71, Some(0));
    second.ready = false;
    app.game.players[owner].units = vec![first, second];
    app.game.battlefield_owner = vec![Some(owner), None];
    if card == 488 {
        app.game.players[0]
            .units
            .push(public_unit(48903, 13, Some(1)));
        app.game.battlefield_owner[1] = Some(0);
    } else if card == 489 {
        let mut zero = public_unit(48903, 88, Some(0));
        zero.temporary_buff = -2;
        app.game.players[1].units.push(zero);
        app.game.players[1].hand = vec![7];
    }
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100., -100.), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.);
    if frame < 14 {
        match frame {
            0 => setup(app, 488),
            1 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
            2 => {
                assert_eq!(app.inspect, Some(488));
                capture(app, "RIFT_EMPEROR_INSPECT");
                app.inspect = None;
                click(app, 315., 760.);
            }
            4 => {
                recovery::advice(app, "Emperor's Divide moves");
                capture(app, "RIFT_EMPEROR_BEGINNER");
                click_unit(app, 48901);
            }
            5 => {
                assert_eq!(app.targets, vec![target(0, 48901)]);
                click_unit(app, 48903);
            }
            6 => {
                assert_eq!(app.targets, vec![target(0, 48901)], "one battlefield only");
                click_unit(app, 48902);
            }
            7 => {
                assert_eq!(app.targets.len(), 2);
                app.companion.set_level(AdviceLevel::Advanced);
                recovery::advice(app, "Use Emperor's Divide");
                capture(app, "RIFT_EMPEROR_ADVANCED");
                click(app, 1285., 510.);
            }
            8 => {
                assert_eq!(app.game.chain.last().unwrap().card_id, 488);
                assert_eq!(app.game.players[0].energy, 10);
                assert_eq!(app.game.players[0].power_pool, [5; 6]);
                settle_until_subset(app);
            }
            9 => {
                assert!(app.game.chain.is_empty() && app.game.decision.is_none());
                for id in [48901, 48902] {
                    let unit = app.game.players[0]
                        .units
                        .iter()
                        .find(|u| u.id == id)
                        .unwrap();
                    assert_eq!(unit.lane, None);
                    assert!(!unit.ready);
                    assert!(app.game.unit_moved_this_turn(0, id));
                }
                assert_eq!(app.game.players[0].units[2].lane, Some(1));
                capture(app, "RIFT_EMPEROR_RETREATED");
            }
            _ => {}
        }
    } else if frame < 36 {
        match frame - 14 {
            0 => setup(app, 489),
            1 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
            2 => {
                assert_eq!(app.inspect, Some(489));
                capture(app, "RIFT_TENTACLES_INSPECT");
                app.inspect = None;
                click(app, 315., 760.);
            }
            4 => {
                assert!(app.play_draft.as_ref().unwrap().open);
                recovery::advice(app, "Tricksy Tentacles moves");
                capture(app, "RIFT_TENTACLES_BEGINNER");
                click_unit(app, 48901);
            }
            5 => click_unit(app, 48902),
            6 => click_unit(app, 48903),
            7 => {
                assert_eq!(app.targets.len(), 3);
                app.companion.set_level(AdviceLevel::Advanced);
                recovery::advice(app, "Use Tricksy Tentacles");
                capture(app, "RIFT_TENTACLES_ADVANCED");
                click(app, 1285., 510.);
            }
            8 => {
                assert_eq!(app.game.chain.last().unwrap().card_id, 489);
                assert_eq!(app.game.players[0].energy, 8);
                assert_eq!(app.game.players[0].power_pool[1], 4);
                if app.game.priority() == 0 {
                    app.game.apply(Action::Pass).unwrap();
                }
                app.game
                    .apply(Action::PlayCardWithTargets {
                        hand_index: 0,
                        lane: None,
                        targets: vec![target(1, 48901)],
                    })
                    .unwrap();
                settle_until_subset(app);
            }
            9 => {
                assert!(matches!(
                    app.game.decision,
                    Some(Decision::ResolutionSubset { maximum: 8, .. })
                ));
                assert!(app.targets.is_empty());
                capture(app, "RIFT_TENTACLES_RESPONSE");
                click_unit(app, 48902);
            }
            10 => {
                assert_eq!(app.targets, vec![target(1, 48902)]);
                assert!(app.game.validate_resolution_subset(&app.targets).is_err());
                click(app, 1280., 532.);
            }
            11 => {
                assert!(matches!(
                    app.game.decision,
                    Some(Decision::ResolutionSubset { .. })
                ));
                click_unit(app, 48903);
            }
            12 => {
                assert!(app.game.validate_resolution_subset(&app.targets).is_ok());
                capture(app, "RIFT_TENTACLES_SUBSET");
                click(app, 1280., 532.);
            }
            13 => {
                assert!(app.game.chain.is_empty() && app.game.decision.is_none());
                assert_eq!(app.game.players[1].units[0].lane, Some(0));
                assert_eq!(app.game.players[1].units[1].lane, None);
                assert_eq!(app.game.players[1].units[2].lane, None);
                assert!(!app.game.unit_moved_this_turn(1, 48901));
                assert!(app.game.unit_moved_this_turn(1, 48902));
                assert!(app.game.unit_moved_this_turn(1, 48903));
                capture(app, "RIFT_TENTACLES_RESOLVED");
            }
            _ => {}
        }
    } else {
        match frame - 36 {
            0 => setup(app, 490),
            1 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
            2 => {
                assert_eq!(app.inspect, Some(490));
                capture(app, "RIFT_ISOLATE_INSPECT");
                app.inspect = None;
                click(app, 315., 760.);
            }
            4 => {
                recovery::advice(app, "Isolate moves");
                capture(app, "RIFT_ISOLATE_BEGINNER");
                click_unit(app, 48901);
            }
            5 => {
                app.companion.set_level(AdviceLevel::Advanced);
                recovery::advice(app, "Isolate can break");
                capture(app, "RIFT_ISOLATE_ADVANCED");
                click(app, 1285., 510.);
            }
            6 => {
                assert_eq!(app.game.chain.last().unwrap().card_id, 490);
                assert_eq!(app.game.players[0].energy, 10);
                settle_until_subset(app);
            }
            7 => {
                assert!(app.game.chain.is_empty() && app.game.decision.is_none());
                assert_eq!(app.game.players[1].units[0].lane, None);
                assert!(!app.game.players[1].units[0].ready);
                assert_eq!(app.game.players[1].units[0].damage, 1);
                assert_eq!(app.game.players[1].units[1].lane, Some(0));
                assert_eq!(app.game.players[0].hand.len(), 1);
                assert_eq!(app.game.players[0].deck.len(), 2);
                capture(app, "RIFT_ISOLATE_DRAW");
            }
            8 => {
                println!("movement group smoke passed");
                return true;
            }
            _ => {}
        }
    }
    false
}
