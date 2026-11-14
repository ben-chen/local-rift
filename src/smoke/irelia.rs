//! Irelia's optional object cost through real native accept/decline controls.
use super::*;
use crate::Tray;
use local_rift::game::ChainAbility;
const PREFECT: u32 = 42400;
#[path = "irelia_responses.rs"]
mod responses;
fn setup(app: &mut App) {
    expansion_fixture(app, 70);
    app.game = Game::new_for_agents(3, 3, 424424, Difficulty::Easy);
    app.game.choose_starting_player(1).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![2, 2];
    app.game.battlefield_owner = vec![None; 2];
    for p in &mut app.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 20;
        p.energy = 20;
        p.power_pool = [10; 6];
    }
    app.game.players[0]
        .units
        .push(public_unit(PREFECT, 424, None));
    app.game.players[1].hand = vec![235];
    app.game
        .apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        })
        .unwrap();
    assert!(matches!(
        app.game.decision,
        Some(Decision::TriggerObjectCost { player: 0, .. })
    ));
    app.fields = std::array::from_fn(|i| app.game.battlefields[i]);
    app.ai_worker.invalidate();
    app.inspect = None;
    app.field_inspect = None;
    app.lesson = None;
    app.chain_cursor = 0;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.companion.set_level(AdviceLevel::Beginner);
}

fn backoff_setup(app: &mut App) {
    expansion_fixture(app, 428);
    app.game = Game::new_for_agents(3, 3, 428428, Difficulty::Easy);
    app.game.start_match().unwrap();
    app.game.battlefields = vec![2, 2];
    app.game.battlefield_owner = vec![Some(0), None];
    for p in &mut app.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 20;
        p.energy = 20;
        p.power_pool = [10; 6];
    }
    app.game.players[0].units = vec![public_unit(42800, 73, Some(0))];
    app.game.players[1].units = vec![public_unit(42801, 73, None)];
    app.game.players[0].hand = vec![428];
    app.game.players[0].deck = vec![71, 70];
    app.fields = std::array::from_fn(|i| app.game.battlefields[i]);
    app.selection = None;
    app.targets.clear();
    app.play_draft = None;
    app.destination = None;
    app.companion.set_level(AdviceLevel::Beginner);
}
fn innovator_setup(app: &mut App) {
    backoff_setup(app);
    app.game.players[0].hand.clear();
    app.game.players[0].units = vec![public_unit(42900, 429, None)];
    app.game.players[1].units.clear();
    app.game
        .apply(Action::Move {
            unit_id: 42900,
            lane: Some(0),
        })
        .unwrap();
    assert!(matches!(
        app.game.decision,
        Some(Decision::TriggerMode { card_id: 429, .. })
    ));
    app.selection = None;
    app.tray = Tray::Board;
    app.inspect = None;
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app),
        1 => {
            // Allow the first render to populate the font atlas before capture.
            capture(app, "RIFT_IRELIA_COST");
            click(app, 1280.0, 505.0);
        }
        2 => {
            assert!(app.game.players[0].units.is_empty());
            assert_eq!(app.game.players[0].banish, vec![424]);
            assert_eq!(app.game.players[1].gear.len(), 1);
            assert_eq!(app.game.chain.len(), 1);
            capture(app, "RIFT_IRELIA_PAID");
        }
        3 => {
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            assert_eq!(app.game.players[1].banish, vec![235]);
            capture(app, "RIFT_IRELIA_RESOLVED");
        }
        4 => {
            setup(app);
            app.companion.set_level(AdviceLevel::Advanced);
            capture(app, "RIFT_IRELIA_ADVANCED");
        }
        5 => click(app, 1280.0, 545.0),
        6 => {
            assert!(app.game.decision.is_none());
            assert!(app.game.chain.is_empty());
            assert_eq!(app.game.players[0].units.len(), 1);
            assert_eq!(app.game.players[1].gear.len(), 1);
            capture(app, "RIFT_IRELIA_DECLINED");
        }
        7 => {
            setup(app);
            app.game.players[0].units.clear();
            capture(app, "RIFT_IRELIA_UNAVAILABLE");
        }
        8 => click(app, 1280.0, 505.0),
        9 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerObjectCost { .. })
            ));
            click(app, 1280.0, 545.0);
        }
        10 => {
            assert!(app.game.decision.is_none());
            setup(app);
            app.inspect = Some(424);
            capture(app, "RIFT_IRELIA_PREFECT");
        }
        11 => click(app, 940.0, 700.0),
        12 => {
            assert!(app.inspect.is_none());
            expansion_fixture(app, 425);
            app.game = Game::new_for_agents(3, 3, 425425, Difficulty::Easy);
            app.game.start_match().unwrap();
            app.game.battlefields = vec![2, 2];
            app.game.battlefield_owner = vec![None; 2];
            for p in &mut app.game.players {
                p.units.clear();
                p.gear.clear();
                p.hidden.clear();
                p.hand.clear();
                p.champion = None;
                p.energy_pool = 20;
                p.energy = 20;
                p.power_pool = [10; 6];
                p.set_rune_setup([0; 6], [0; 6]).unwrap();
            }
            let mut unit = public_unit(42500, 73, None);
            unit.ready = false;
            app.game.players[0].units = vec![unit];
            app.game.players[0].hand = vec![425];
            app.game.players[0].deck = vec![71; 10];
            app.fields = std::array::from_fn(|i| app.game.battlefields[i]);
            app.ai_worker.invalidate();
            app.prefs.exact_payment = false;
            app.companion.set_level(AdviceLevel::Beginner);
        }
        13 => {
            capture(app, "RIFT_IRELIA_HUNKER_HAND");
            click(app, 315.0, 760.0);
        }
        14 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            click_unit(app, 42500);
        }
        15 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: 42500
                }]
            );
            click(app, 1285.0, 510.0);
        }
        16 => {
            assert!(app.game.chain.iter().any(|item| item.card_id == 425));
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            assert!(app.game.players[0].units[0].ready);
            assert!(app.game.movement_prevented(1, 0, 42500));
            assert_eq!(app.game.players[0].hand, vec![71]);
            assert!(!app.game.action_hint().contains("ready unit at base"));
            capture(app, "RIFT_IRELIA_HUNKER_LOCKED");
            click_unit(app, 42500);
        }
        17 => {
            assert!(app.selection == Some(Selection::Unit(42500)));
            // The selected-unit movement button is disabled, not a failed action.
            app.toast_until = 0.0;
            click(app, 1285.0, 420.0);
        }
        18 => {
            assert_eq!(app.game.players[0].units[0].lane, None);
            assert_eq!(app.toast_until, 0.0);
            assert!(
                super::super::live_unit_description(&app.game, 0, 42500)
                    .unwrap()
                    .contains("either player's spells")
            );
            capture_unit(app, 42500, "RIFT_IRELIA_HUNKER_HOVER");
        }
        19 => {
            app.inspect = Some(425);
            app.companion.set_level(AdviceLevel::Advanced);
            capture(app, "RIFT_IRELIA_HUNKER_INSPECT");
            click(app, 940.0, 700.0);
        }
        20 => {
            assert!(app.inspect.is_none());
        }
        21 => {
            expansion_fixture(app, 426);
            app.game = Game::new_for_agents(3, 3, 426426, Difficulty::Easy);
            app.game.start_match().unwrap();
            app.game.battlefields = vec![2, 2];
            app.game.battlefield_owner = vec![None; 2];
            for p in &mut app.game.players {
                p.units.clear();
                p.gear.clear();
                p.hidden.clear();
                p.hand.clear();
                p.champion = None;
                p.set_rune_setup([0; 6], [0; 6]).unwrap();
                p.energy_pool = 0;
                p.energy = 0;
                p.power_pool = [0; 6];
            }
            app.game.players[0].units = vec![public_unit(42600, 73, None)];
            app.game.players[0].hand = vec![426, 58];
            app.game.players[0]
                .set_rune_setup([0, 2, 0, 0, 0, 0], [0; 6])
                .unwrap();
            app.game.players[0].energy = 2;
            app.fields = std::array::from_fn(|i| app.game.battlefields[i]);
            app.ai_worker.invalidate();
            app.prefs.exact_payment = false;
            app.companion.set_level(AdviceLevel::Beginner);
        }
        22 => {
            capture(app, "RIFT_IRELIA_PREP_HAND");
            click(app, 315.0, 760.0);
        }
        23 => {}
        24 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            assert!(app.play_draft.as_ref().unwrap().open);
            capture(app, "RIFT_IRELIA_PREP_SHOW");
            click(app, 450.0, 770.0);
        }
        25 => {
            assert_eq!(app.play_draft.as_ref().unwrap().choices.show_off.len(), 1);
            click_unit(app, 42600);
        }
        26 => {
            assert_eq!(app.targets.len(), 1);
            click(app, 430.0, 606.0);
        }
        27 => {
            assert_eq!(app.targets.len(), 2);
            click(app, 463.0, 606.0);
        }
        28 => {
            assert_eq!(app.targets.len(), 3);
            assert!(app.rune_menu.is_none());
            capture(app, "RIFT_IRELIA_PREP_TARGETS");
            click(app, 1285.0, 510.0);
        }
        29 => {
            let spell = app.game.chain.iter().find(|c| c.card_id == 426).unwrap();
            assert_eq!(spell.show_off.len(), 1);
            assert_eq!(spell.targets.len(), 3);
            assert!(app.game.players[0].rune_objects().iter().all(|r| !r.ready));
            capture(app, "RIFT_IRELIA_PREP_CHAIN");
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        30 => {
            assert_eq!(app.game.players[0].units[0].temporary_buff, 2);
            assert!(app.game.players[0].rune_objects().iter().all(|r| r.ready));
            capture(app, "RIFT_IRELIA_PREP_READY");
            app.inspect = Some(426);
            app.companion.set_level(AdviceLevel::Advanced);
        }
        31 => {
            capture(app, "RIFT_IRELIA_PREP_INSPECT");
            click(app, 940.0, 700.0);
        }
        32 => {
            assert!(app.inspect.is_none());
            expansion_fixture(app, 427);
            app.game = Game::new_for_agents(3, 3, 427427, Difficulty::Easy);
            app.game.start_match().unwrap();
            app.game.battlefields = vec![2, 2];
            app.game.battlefield_owner = vec![None; 2];
            for p in &mut app.game.players {
                p.units.clear();
                p.gear.clear();
                p.hidden.clear();
                p.hand.clear();
                p.champion = None;
                p.set_rune_setup([0; 6], [0; 6]).unwrap();
                p.energy_pool = 20;
                p.energy = 20;
                p.power_pool = [10; 6];
            }
            app.game.players[1].units =
                vec![public_unit(42700, 73, None), public_unit(42701, 73, None)];
            app.game.players[0].hand = vec![4];
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: Some(0),
                    targets: vec![Target::Unit {
                        player: 1,
                        unit_id: 42700,
                    }],
                })
                .unwrap();
            for _ in 0..12 {
                if !app.game.in_window() {
                    break;
                }
                app.game.apply(Action::Pass).unwrap();
            }
            assert_eq!(app.game.priority(), 0);
            assert!(app.game.unit_moved_this_turn(1, 42700));
            app.game.players[0].hand = vec![427];
            app.game.play_grants.push(local_rift::play::PlayGrant {
                id: 42799,
                player: 0,
                source: local_rift::game::ChainSource::Card { card_id: 427 },
                expires_turn: app.game.turn,
                kind: local_rift::play::PlayGrantKind::RepeatNextSpell,
            });
            app.fields = std::array::from_fn(|i| app.game.battlefields[i]);
            app.selection = None;
            app.play_draft = None;
            app.targets.clear();
            app.destination = None;
            app.companion.set_level(AdviceLevel::Beginner);
        }
        33 => click(app, 310.0, 760.0),
        34 => {}
        35 => {
            assert!(app.play_draft.as_ref().unwrap().open);
            capture(app, "RIFT_IRELIA_SIREN_CHOICES");
            click_unit(app, 42701);
        }
        36 => {
            assert!(
                app.targets.is_empty(),
                "An unmoved enemy must not be selectable"
            );
            click_unit(app, 42700);
        }
        37 => {
            assert_eq!(app.targets.len(), 1);
            click(app, 400.0, 741.0);
        }
        38 => {
            assert_eq!(app.play_draft.as_ref().unwrap().execution, 1);
            click(app, 1040.0, 768.0);
        }
        39 => {
            assert_eq!(app.destination, None);
            assert_eq!(
                app.play_draft.as_ref().unwrap().choices.repeats[0]
                    .choices
                    .destination,
                Some(local_rift::play::MoveDestination::Battlefield { lane: 1 })
            );
            click_unit(app, 42700);
        }
        40 => {
            assert_eq!(
                app.play_draft.as_ref().unwrap().choices.repeats[0]
                    .choices
                    .targets
                    .len(),
                1
            );
            capture(app, "RIFT_IRELIA_SIREN_REPEAT");
            click(app, 790.0, 848.0);
        }
        41 => {
            assert!(app.cost_editor.is_none());
            assert_eq!(
                app.play_draft.as_ref().unwrap().choices.repeats[0]
                    .choices
                    .destination,
                Some(local_rift::play::MoveDestination::Base)
            );
            click(app, 1040.0, 768.0);
        }
        42 => click_unit(app, 42700),
        43 => click(app, 1285.0, 510.0),
        44 => {
            let spell = app.game.chain.iter().find(|c| c.card_id == 427).unwrap();
            assert_eq!(
                spell.executions[0].destination,
                Some(local_rift::play::MoveDestination::Base)
            );
            assert_eq!(
                spell.executions[1].destination,
                Some(local_rift::play::MoveDestination::Battlefield { lane: 1 })
            );
            capture(app, "RIFT_IRELIA_SIREN_CHAIN");
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        45 => {
            assert_eq!(
                app.game.players[1]
                    .units
                    .iter()
                    .find(|u| u.id == 42700)
                    .unwrap()
                    .lane,
                Some(1)
            );
            assert_eq!(
                app.game.players[1]
                    .units
                    .iter()
                    .find(|u| u.id == 42701)
                    .unwrap()
                    .lane,
                None
            );
            capture(app, "RIFT_IRELIA_SIREN_MOVED");
        }
        46 => {
            assert!(
                super::super::live_unit_description(&app.game, 1, 42700)
                    .unwrap()
                    .contains("Moved this turn")
            );
            capture_unit(app, 42700, "RIFT_IRELIA_SIREN_HOVER");
            app.inspect = Some(427);
            app.companion.set_level(AdviceLevel::Advanced);
        }
        47 => {
            capture(app, "RIFT_IRELIA_SIREN_INSPECT");
            click(app, 940.0, 700.0);
        }
        48 => {
            assert!(app.inspect.is_none());
            backoff_setup(app);
        }
        49 => click(app, 310.0, 760.0),
        50 => {
            assert!(matches!(app.selection, Some(Selection::Hand(0))));
            capture(app, "RIFT_IRELIA_BACKOFF_HAND");
            click_unit(app, 42801);
        }
        51 => click(app, 1285.0, 510.0),
        52 => {
            let spell = app.game.chain.iter().find(|s| s.card_id == 428).unwrap();
            assert_eq!(spell.play_origin, Some(local_rift::play::PlayOrigin::Hand));
            assert!(app.game.players[0].hand.is_empty());
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        53 => {
            assert_eq!(app.game.players[0].hand, vec![70]);
            assert!(app.game.players[1].units[0].stunned && app.game.players[1].units[0].ready);
            capture(app, "RIFT_IRELIA_BACKOFF_DRAW");
            app.inspect = Some(428);
        }
        54 => {
            capture(app, "RIFT_IRELIA_BACKOFF_INSPECT");
            click(app, 940.0, 700.0);
        }
        55 => {
            assert!(app.inspect.is_none());
            backoff_setup(app);
        }
        56 => click(app, 310.0, 760.0),
        57 => click(app, 1225.0, 467.0),
        58 => {
            assert!(app.game.players[0].hand.is_empty());
            assert_eq!(app.game.players[0].hidden.len(), 1);
            let id = app.game.players[0].hidden[0].id;
            assert_eq!(app.game.hidden_visible_card(1, 0, id), None);
            assert!(app.game.chain.is_empty());
            capture(app, "RIFT_IRELIA_BACKOFF_HIDDEN");
            app.game.apply(Action::EndTurn).unwrap();
            for _ in 0..20 {
                if !app.game.in_window() {
                    break;
                }
                app.game.apply(Action::Pass).unwrap();
            }
            assert_eq!(app.game.active, 1);
            app.game
                .apply(Action::Move {
                    unit_id: 42801,
                    lane: Some(0),
                })
                .unwrap();
            app.game.apply(Action::Pass).unwrap();
            assert_eq!(app.game.priority(), 0);
            app.selection = None;
            app.targets.clear();
            app.companion.set_level(AdviceLevel::Advanced);
            click(app, 610.0, 661.0);
        }
        59 => click(app, 315.0, 760.0),
        60 => {
            assert!(matches!(app.selection, Some(Selection::Hidden(_))));
            click_unit(app, 42801);
        }
        61 => {
            capture(app, "RIFT_IRELIA_BACKOFF_REVEAL");
            click(app, 1285.0, 535.0);
        }
        62 => {
            let spell = app.game.chain.iter().find(|s| s.card_id == 428).unwrap();
            assert_eq!(
                spell.play_origin,
                Some(local_rift::play::PlayOrigin::Hidden { lane: 0 })
            );
            assert!(spell.from_hidden);
            assert_eq!(app.game.players[0].energy, 0);
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        63 => {
            assert!(app.game.players[1].units[0].stunned);
            assert!(app.game.players[0].hand.is_empty());
            assert_eq!(app.game.players[0].deck, vec![71, 70]);
            capture(app, "RIFT_IRELIA_BACKOFF_NO_DRAW");
        }
        64 => innovator_setup(app),
        65 => {
            capture(app, "RIFT_IRELIA_INNOVATOR_MODES");
            click(app, 1285.0, 468.0);
        }
        66 => {
            let item = app.game.chain.last().unwrap();
            assert!(
                matches!(item.ability, Some(ChainAbility::CardTrigger { context, .. }) if context.mode == Some(0))
            );
            assert!(app.game.players[0].gear.is_empty());
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        67 => {
            assert_eq!(app.game.players[0].gear.len(), 1);
            assert!(app.game.players[0].gear[0].is_gold() && !app.game.players[0].gear[0].ready);
            capture(app, "RIFT_IRELIA_INNOVATOR_GOLD");
            innovator_setup(app);
            app.companion.set_level(AdviceLevel::Advanced);
        }
        68 => {
            capture(app, "RIFT_IRELIA_INNOVATOR_ADVANCED");
            click(app, 1285.0, 516.0);
        }
        69 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.chain.len(), 1);
            assert_eq!(app.game.players[0].power_pool[4], 10);
            capture(app, "RIFT_IRELIA_INNOVATOR_RESPONSES");
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        70 => {
            assert!(matches!(app.game.decision, Some(Decision::TriggerPayment {
                context, resolving_chain_id: Some(_), ..
            }) if context.mode == Some(1)));
            assert_eq!(app.game.players[0].power_pool[4], 10);
            assert_eq!(app.game.chain.len(), 1);
            capture(app, "RIFT_IRELIA_INNOVATOR_PAYMENT");
            click(app, 1285.0, 510.0);
        }
        71 => {
            assert!(app.game.decision.is_none());
            assert!(app.game.chain.is_empty());
            assert_eq!(app.game.players[0].power_pool[4], 9);
            assert_eq!(app.game.players[0].hand, vec![70]);
            capture(app, "RIFT_IRELIA_INNOVATOR_PAID");
        }
        72 => {
            assert_eq!(app.game.players[0].hand, vec![70]);
            assert!(app.game.players[0].gear.is_empty());
            capture(app, "RIFT_IRELIA_INNOVATOR_DRAW");
            app.inspect = Some(429);
        }
        73 => {
            capture(app, "RIFT_IRELIA_INNOVATOR_INSPECT");
            click(app, 940.0, 700.0);
        }
        74 => {
            assert!(app.inspect.is_none());
            innovator_setup(app);
        }
        75 => click(app, 1285.0, 516.0),
        76 => {
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            click(app, 1285.0, 548.0);
        }
        77 => {
            assert!(app.game.decision.is_none() && app.game.chain.is_empty());
            assert!(app.game.players[0].gear.is_empty() && app.game.players[0].hand.is_empty());
            assert_eq!(app.game.players[0].power_pool[4], 10);
            capture(app, "RIFT_IRELIA_INNOVATOR_DECLINED");
        }
        78 => {
            backoff_setup(app);
            app.game.players[0].hand.clear();
            app.game.players[0].units = vec![
                public_unit(43000, 430, Some(0)),
                public_unit(43001, 71, Some(0)),
            ];
            app.game.players[1].units.clear();
        }
        79 => {
            let unit = &app.game.players[0].units[0];
            assert_eq!(app.game.unit_combat_contribution(unit).unwrap().damage, 0);
            capture_unit(app, 43000, "RIFT_IRELIA_ORIANNA_NO_GEAR");
            let rect = unit_rect(app, 43000);
            app.ui.input_override = Some((rect.center(), false, true));
        }
        80 => {
            assert_eq!(app.inspect, Some(430));
            assert!(
                crate::live_unit_description(&app.game, 0, 43000)
                    .unwrap()
                    .contains("NEEDS FRIENDLY GEAR HERE")
            );
            capture(app, "RIFT_IRELIA_ORIANNA_INSPECT");
            click(app, 940.0, 700.0);
        }
        81 => {
            assert!(app.inspect.is_none());
            app.game.players[0].gear.push(local_rift::gear::GearState {
                id: 43002,
                card_id: 3,
                lane: None,
                attached_to: Some(43001),
                ready: false,
                temporary: false,
                token: false,
            });
            app.companion.set_level(AdviceLevel::Advanced);
        }
        82 => {
            assert_eq!(
                app.game
                    .unit_combat_contribution(&app.game.players[0].units[0])
                    .unwrap()
                    .damage,
                4
            );
            capture_unit(app, 43000, "RIFT_IRELIA_ORIANNA_WITH_GEAR");
            let rect = unit_rect(app, 43000);
            app.ui.input_override = Some((rect.center(), false, true));
        }
        83 => {
            assert_eq!(app.inspect, Some(430));
            let text = crate::live_unit_description(&app.game, 0, 43000).unwrap();
            assert!(text.contains("FRIENDLY GEAR HERE · Combat requirement met"));
            assert!(text.contains("Combat contribution: 4 damage"));
            capture(app, "RIFT_IRELIA_ORIANNA_ENABLED_INSPECT");
        }
        84 => {
            backoff_setup(app);
            app.inspect = None;
            app.game.players[0].hand = vec![431];
            app.game.players[0].discard = vec![71, 7];
            app.game.players[1].discard = vec![6, 71];
            app.hand_scroll = 0;
        }
        85 => {
            capture(app, "RIFT_IRELIA_GUST_HAND");
            click(app, 315.0, 760.0);
        }
        86 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            click(app, 1285.0, 467.0);
        }
        87 => {
            assert!(app.play_draft.as_ref().unwrap().open);
            click(app, 790.0, 849.0);
        }
        88 => {
            assert!(app.play_draft.as_ref().unwrap().choices.bonus);
            capture(app, "RIFT_IRELIA_GUST_EXTRA");
            click(app, 1285.0, 510.0);
        }
        89 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerTargets { card_id: 431, .. })
            ));
            click_unit(app, 42800);
        }
        90 => {
            assert_eq!(app.targets.len(), 1);
            click(app, 1285.0, 510.0);
        }
        91 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerPayment { card_id: 431, .. })
            ));
            assert_eq!(app.game.players[0].energy, 17);
            capture(app, "RIFT_IRELIA_GUST_TRASH");
            app.ui.input_override = Some((vec2(315.0, 750.0), false, true));
        }
        92 => {
            assert_eq!(app.inspect, Some(71));
            capture(app, "RIFT_IRELIA_GUST_COST_INSPECT");
            click(app, 940.0, 700.0);
        }
        93 => {
            assert!(app.inspect.is_none());
            click(app, 615.0, 858.0);
        }
        94 => {
            assert_eq!(app.game.players[1].banish, vec![6]);
            assert_eq!(app.game.players[1].discard, vec![71]);
            assert!(app.game.chain.iter().any(|item| item.card_id == 431));
            assert_eq!(
                app.game
                    .unit_keywords(&app.game.players[0].units[0])
                    .unwrap()
                    .assault,
                0
            );
            capture(app, "RIFT_IRELIA_GUST_PAID");
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        95 => {
            assert_eq!(
                app.game
                    .unit_keywords(&app.game.players[0].units[0])
                    .unwrap()
                    .assault,
                2
            );
            app.companion.set_level(AdviceLevel::Advanced);
            capture_unit(app, 42800, "RIFT_IRELIA_GUST_ASSAULT");
        }
        96 => {
            backoff_setup(app);
            app.game.players[0].hand = vec![431];
            app.game
                .apply(Action::WithPlayChoices {
                    action: Box::new(Action::PlayCardWithTargets {
                        hand_index: 0,
                        lane: None,
                        targets: vec![],
                    }),
                    choices: local_rift::play::PlayChoices {
                        bonus: true,
                        ..Default::default()
                    },
                })
                .unwrap();
            app.game
                .apply(Action::ChooseTriggerTargets {
                    targets: vec![Target::Unit {
                        player: 0,
                        unit_id: 42800,
                    }],
                })
                .unwrap();
            app.targets.clear();
            app.selection = None;
            app.play_draft = None;
            app.hand_scroll = 0;
        }
        97 => {
            capture(app, "RIFT_IRELIA_GUST_EMPTY");
            click(app, 1285.0, 545.0);
        }
        98 => {
            assert!(app.game.decision.is_none());
            assert!(app.game.chain.is_empty());
            assert!(app.game.players[0].units.iter().any(|u| u.card_id == 431));
            capture(app, "RIFT_IRELIA_GUST_DECLINED");
        }
        99 => {
            backoff_setup(app);
            app.inspect = None;
            app.game.players[0].hand = vec![432];
            app.game.players[0].units[0].card_id = 430;
            app.tray = crate::Tray::Hand;
            app.hand_scroll = 0;
        }
        100 => {
            capture(app, "RIFT_IRELIA_EDGE_HAND");
            click(app, 310.0, 760.0);
        }
        101 => {
            assert!(matches!(app.selection, Some(Selection::Hand(0))));
            click(app, 1225.0, 467.0);
        }
        102 => {
            assert_eq!(app.game.players[0].hidden.len(), 1);
            assert!(app.game.players[0].gear.is_empty());
            capture(app, "RIFT_IRELIA_EDGE_HIDDEN");
            app.game.apply(Action::EndTurn).unwrap();
            for _ in 0..20 {
                if !app.game.in_window() {
                    break;
                }
                app.game.apply(Action::Pass).unwrap();
            }
            app.game
                .apply(Action::Move {
                    unit_id: 42801,
                    lane: Some(0),
                })
                .unwrap();
            app.game.apply(Action::Pass).unwrap();
            assert_eq!(app.game.priority(), 0);
            app.selection = None;
            app.targets.clear();
            click(app, 610.0, 661.0);
        }
        103 => click(app, 315.0, 760.0),
        104 => {
            assert!(matches!(app.selection, Some(Selection::Hidden(_))));
            capture(app, "RIFT_IRELIA_EDGE_REVEAL");
            click(app, 1285.0, 535.0);
        }
        105 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerTargets { card_id: 432, .. })
            ));
            assert_eq!(app.game.players[0].gear[0].lane, None);
            assert_eq!(app.game.players[0].gear[0].attached_to, None);
            capture(app, "RIFT_IRELIA_EDGE_TARGET");
            click_unit(app, 42800);
        }
        106 => {
            assert_eq!(app.targets.len(), 1);
            click(app, 1285.0, 510.0);
        }
        107 => {
            assert!(app.game.decision.is_none());
            assert!(app.game.chain.iter().any(|item| item.card_id == 432));
            capture(app, "RIFT_IRELIA_EDGE_CHAIN");
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        108 => {
            assert_eq!(app.game.players[0].gear[0].attached_to, Some(42800));
            assert_eq!(app.game.gear_might_bonus(42800).unwrap(), 2);
            assert!(
                app.game
                    .unit_combat_contribution(&app.game.players[0].units[0])
                    .unwrap()
                    .damage
                    > 0
            );
            app.companion.set_level(AdviceLevel::Advanced);
            capture_unit(app, 42800, "RIFT_IRELIA_EDGE_EQUIPPED");
            let rect = unit_rect(app, 42800);
            app.ui.input_override = Some((rect.center(), false, true));
        }
        109 => {
            assert_eq!(app.inspect, Some(430));
            capture(app, "RIFT_IRELIA_EDGE_BEARER_INSPECT");
            click(app, 940.0, 700.0);
        }
        110 => {
            app.inspect_printed_card(432);
            capture(app, "RIFT_IRELIA_EDGE_INSPECT");
        }
        111 => {
            innovator_setup(app);
            let Some(Decision::TriggerMode { choice_id, .. }) = app.game.decision else {
                panic!()
            };
            app.game
                .apply(Action::ChooseTriggerMode { choice_id, mode: 1 })
                .unwrap();
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            app.game.players[0].power_pool = [0; 6];
            app.game.players[0].gear.push(GearState {
                id: 42901,
                card_id: 13,
                lane: None,
                attached_to: None,
                ready: true,
                temporary: false,
                token: true,
            });
            let Some(Decision::TriggerPayment { choice_id, .. }) = app.game.decision else {
                panic!()
            };
            assert!(app.begin_ui_payment(Action::ResolveTriggerPayment {
                choice_id,
                pay: true
            }));
            app.game
                .apply(Action::ActivateGear { gear_id: 42901 })
                .unwrap();
        }
        112 => {
            assert_eq!(
                app.game.payment_plan_view(0).unwrap().refusal_kind,
                Some(local_rift::payment_plan::PaymentRefusalKind::Instruction)
            );
            assert_eq!(app.game.players[0].any_power, 1);
            capture(app, "RIFT_IRELIA_INNOVATOR_UNDO_OR_SKIP");
            click(app, 1285.0, 548.0);
        }
        113 => {
            assert_eq!(app.game.players[0].gear.len(), 1);
            assert_eq!(app.game.players[0].any_power, 0);
            assert_eq!(app.game.chain.len(), 1);
            capture(app, "RIFT_IRELIA_INNOVATOR_UNDONE");
            let Some(Decision::TriggerPayment { choice_id, .. }) = app.game.decision else {
                panic!()
            };
            assert!(app.begin_ui_payment(Action::ResolveTriggerPayment {
                choice_id,
                pay: true
            }));
            app.game
                .apply(Action::ActivateGear { gear_id: 42901 })
                .unwrap();
        }
        114 => {
            capture(app, "RIFT_IRELIA_INNOVATOR_SKIP_EFFECT");
            click(app, 1285.0, 578.0);
        }
        115 => {
            assert!(app.game.chain.is_empty() && app.game.decision.is_none());
            assert!(app.game.players[0].gear.is_empty());
            assert!(app.game.players[0].hand.is_empty());
            assert_eq!(app.game.players[0].any_power, 1);
            capture(app, "RIFT_IRELIA_INNOVATOR_SKIPPED_KEEP_ADD");
        }
        116.. => return responses::run_frame(app, frame - 116),
    }
    false
}
