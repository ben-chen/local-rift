//! Native optional choices, unit-or-gear filtering and instructed-play cost review.
use super::*;
use local_rift::game::{ChainAbility, InstructedPlayKind};

fn setup(app: &mut App, source: usize) {
    recovery::setup(app, source);
    app.game.battlefield_owner[0] = Some(0);
    app.game.players[0].discard = if source == 476 {
        vec![8, 470, 472]
    } else {
        vec![8, 474]
    };
    if source == 477 {
        app.game.players[0].set_rune_setup([0; 6], [0; 6]).unwrap();
        app.game.players[0].energy_pool = 8;
        app.game.players[0].energy = 8;
        app.game.players[0].power_pool = [0; 6];
        app.game.players[0].power_pool[Domain::Chaos as usize] = 3;
    }
}

fn advance(app: &mut App) {
    for _ in 0..80 {
        if matches!(
            app.game.decision,
            Some(Decision::TrashTarget { .. } | Decision::PendingPlay { .. })
        ) {
            return;
        }
        if app.game.decision.is_none() && app.game.chain.is_empty() {
            return;
        }
        let action = match &app.game.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::Legend { .. }) => Action::Choose { accept: false },
            None => Action::Pass,
            other => panic!("unexpected trash-choice scenario: {other:?}"),
        };
        app.game.apply(action).unwrap();
    }
    panic!("trash-choice scenario did not settle");
}

fn hold(app: &mut App) {
    advance(app);
    app.game.apply(Action::EndTurn).unwrap();
    advance(app);
    app.game.apply(Action::EndTurn).unwrap();
    advance(app);
    assert!(matches!(
        app.game.decision,
        Some(Decision::TrashTarget {
            source_card_id: 476,
            ..
        })
    ));
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100., -100.), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.);
    match frame {
        0 => setup(app, 476),
        1 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        2 => {
            assert_eq!(app.inspect, Some(476));
            capture(app, "RIFT_TRASH_GUARDIAN_INSPECT");
            app.inspect = None;
            click(app, 315., 760.);
        }
        3 => {
            recovery::advice(app, "Guardian of the Passage");
            capture(app, "RIFT_TRASH_GUARDIAN_BEGINNER");
            click(app, 150., 600.);
        }
        4 => {
            recovery::advice(app, "Guardian rewards");
            capture(app, "RIFT_TRASH_GUARDIAN_ADVANCED");
            click(app, 1220., 548.);
        }
        5 => {
            assert_eq!(app.game.players[0].units[0].card_id, 476);
            assert_eq!(app.game.players[0].units[0].lane, Some(0));
            hold(app);
            capture(app, "RIFT_TRASH_GUARDIAN_CHOICES");
            click(app, 465., 760.); // Second eligible card is the gear at trash index 2.
        }
        6 => {
            assert!(matches!(
                app.game.chain.last().unwrap().ability,
                Some(ChainAbility::ReturnChosen {
                    trash_index: 2,
                    card_id: 472
                })
            ));
            capture(app, "RIFT_TRASH_GUARDIAN_CHAIN");
            advance(app);
        }
        7 => {
            assert_eq!(app.game.players[0].hand[0], 472);
            assert_eq!(app.game.players[0].units.len(), 1);
            capture(app, "RIFT_TRASH_GUARDIAN_RETURNED");
        }
        8 => {
            setup(app, 476);
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: Some(0),
                    targets: vec![],
                })
                .unwrap();
            hold(app);
        }
        9 => {
            capture(app, "RIFT_TRASH_GUARDIAN_SKIP");
            click(app, 1280., 548.);
        }
        10 => {
            advance(app);
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[0].discard, vec![8, 470, 472]);
            assert_eq!(app.game.players[0].hand.len(), 1); // Normal turn draw only.
        }
        11 => setup(app, 477),
        12 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        13 => {
            assert_eq!(app.inspect, Some(477));
            capture(app, "RIFT_TRASH_SOULGORGER_INSPECT");
            app.inspect = None;
            click(app, 315., 760.);
        }
        14 => {
            recovery::advice(app, "Soulgorger");
            capture(app, "RIFT_TRASH_SOULGORGER_BEGINNER");
            click(app, 150., 600.);
        }
        15 => {
            recovery::advice(app, "Reserve enough Power");
            capture(app, "RIFT_TRASH_SOULGORGER_ADVANCED");
            click(app, 1280., 510.);
        }
        16 => {
            advance(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TrashTarget {
                    source_card_id: 477,
                    ..
                })
            ));
            capture(app, "RIFT_TRASH_SOULGORGER_CHOICE");
            click(app, 315., 760.);
        }
        17 => {
            assert!(matches!(
                app.game.chain.last().unwrap().ability,
                Some(ChainAbility::PlayChosen {
                    trash_index: 1,
                    card_id: 474,
                    kind: InstructedPlayKind::Soulgorger
                })
            ));
            assert_eq!(app.game.players[0].discard, vec![8, 474]);
            capture(app, "RIFT_TRASH_SOULGORGER_CHAIN");
            advance(app);
        }
        18 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::PendingPlay {
                    kind: InstructedPlayKind::Soulgorger,
                    ..
                })
            ));
            assert_eq!(app.game.players[0].energy, 0);
            assert_eq!(app.game.players[0].power_pool[Domain::Chaos as usize], 1);
            capture(app, "RIFT_TRASH_SOULGORGER_COST");
            click(app, 1280., 510.);
        }
        19 => {
            advance(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TrashTarget {
                    source_card_id: 474,
                    ..
                })
            ));
            click(app, 1280., 548.);
        }
        20 => {
            advance(app);
            assert_eq!(app.game.players[0].units.len(), 2);
            assert_eq!(app.game.players[0].power_pool[Domain::Chaos as usize], 0);
            assert_eq!(app.game.players[0].energy, 0);
            assert_eq!(app.game.players[0].discard, vec![8]);
            capture(app, "RIFT_TRASH_SOULGORGER_PLAYED");
        }
        21 => {
            setup(app, 477);
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                })
                .unwrap();
            advance(app);
        }
        22 => {
            capture(app, "RIFT_TRASH_SOULGORGER_SKIP");
            click(app, 1280., 548.);
        }
        23 => {
            advance(app);
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[0].units.len(), 1);
            assert_eq!(app.game.players[0].discard, vec![8, 474]);
        }
        24 => {
            setup(app, 477);
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                })
                .unwrap();
            advance(app);
            app.game
                .apply(Action::SelectCards { indices: vec![1] })
                .unwrap();
            advance(app);
        }
        25 => click(app, 400., 665.),
        26 => {
            assert!(matches!(app.tray, crate::Tray::Loadout));
            assert!(matches!(
                app.game.decision,
                Some(Decision::PendingPlay { .. })
            ));
            assert!(app.game.resource_payment_pending());
            capture(app, "RIFT_TRASH_PENDING_LOADOUT");
            click(app, 295., 665.);
        }
        27 => {
            assert!(matches!(app.tray, crate::Tray::Hand));
            assert!(matches!(
                app.game.decision,
                Some(Decision::PendingPlay { .. })
            ));
            click(app, 1280., 510.);
        }
        28 => {
            advance(app);
            assert_eq!(app.game.players[0].units.len(), 2);
            click(app, 1280., 548.);
        }
        29 => {
            advance(app);
            assert!(app.game.decision.is_none());
        }
        30 => {
            recovery::setup(app, 479);
            app.game.players[0].discard = vec![8, 470];
            app.game.players[1].discard = vec![472, 470];
            click(app, 315., 760.);
        }
        31 => {
            recovery::advice(app, "Shadows of the Past can return");
            capture(app, "RIFT_TRASH_SHADOWS_SELECTED");
            click(app, 500., 665.);
        }
        32 => {
            assert!(matches!(app.selection, Some(crate::Selection::Hand(0))));
            assert!(matches!(app.tray, crate::Tray::Trash));
            assert!(
                !app.card_target_candidate(app.game.trash_target(0, 0).unwrap())
                    .unwrap()
            );
            capture(app, "RIFT_TRASH_SHADOWS_OWN");
            click(app, 465., 760.);
        }
        33 => {
            assert_eq!(app.current_targets().len(), 1);
            click(app, 820., 665.);
        }
        34 => {
            assert_eq!(app.gear_owner, 1);
            assert!(matches!(app.selection, Some(crate::Selection::Hand(0))));
            assert_eq!(app.current_targets().len(), 1);
            assert!(
                !app.card_target_candidate(app.game.trash_target(1, 0).unwrap())
                    .unwrap()
            );
            capture(app, "RIFT_TRASH_SHADOWS_OPPONENT");
            click(app, 465., 760.);
        }
        35 => {
            assert_eq!(app.current_targets().len(), 2);
            capture(app, "RIFT_TRASH_SHADOWS_TWO_TARGETS");
            click(app, 1280., 510.);
        }
        36 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 479);
            assert_eq!(app.game.chain.last().unwrap().targets.len(), 2);
            capture(app, "RIFT_TRASH_SHADOWS_CHAIN");
            advance(app);
            assert_eq!(app.game.players[0].hand, vec![470]);
            assert_eq!(app.game.players[1].hand, vec![470]);
        }
        37 => {
            recovery::setup(app, 478);
            app.game.players[0].discard = vec![8, 474];
            app.game.players[0].set_rune_setup([0; 6], [0; 6]).unwrap();
            app.game.players[0].energy_pool = 6;
            app.game.players[0].energy = 6;
            app.game.players[0].power_pool = [0; 6];
            app.game.players[0].power_pool[Domain::Chaos as usize] = 3;
            click(app, 315., 760.);
        }
        38 => {
            recovery::advice(app, "The Harrowing chooses");
            capture(app, "RIFT_TRASH_HARROWING_SELECTED");
            click(app, 500., 665.);
        }
        39 => {
            assert!(matches!(app.selection, Some(crate::Selection::Hand(0))));
            capture(app, "RIFT_TRASH_HARROWING_TARGETS");
            click(app, 465., 760.);
        }
        40 => {
            assert_eq!(app.current_targets().len(), 1);
            click(app, 1280., 510.);
        }
        41 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 478);
            assert_eq!(app.game.players[0].discard, vec![8, 474]);
            capture(app, "RIFT_TRASH_HARROWING_CHAIN");
            advance(app);
        }
        42 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::PendingPlay {
                    kind: InstructedPlayKind::TheHarrowing,
                    ..
                })
            ));
            assert_eq!(app.game.players[0].energy, 0);
            assert_eq!(app.game.players[0].power_pool[Domain::Chaos as usize], 1);
            capture(app, "RIFT_TRASH_HARROWING_COST");
            click(app, 1280., 510.);
        }
        43 => {
            advance(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TrashTarget {
                    source_card_id: 474,
                    ..
                })
            ));
            click(app, 1280., 548.);
        }
        44 => {
            advance(app);
            assert_eq!(app.game.players[0].units.len(), 1);
            assert_eq!(app.game.players[0].units[0].card_id, 474);
            assert_eq!(app.game.players[0].power_pool[Domain::Chaos as usize], 0);
            capture(app, "RIFT_TRASH_HARROWING_PLAYED");
        }
        45 => return true,
        _ => panic!("trash-choice smoke exceeded its frames"),
    }
    false
}
