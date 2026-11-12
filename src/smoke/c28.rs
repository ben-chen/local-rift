//! Physical rune selection and battlefield choices through public actions and real controls.
use super::*;
use local_rift::game::field_triggers::BattlefieldTargetKind;
const UNIT: u32 = 28001;
const OTHER: u32 = 28002;
thread_local! {
    static RUNES: std::cell::Cell<[u32; 3]> = const { std::cell::Cell::new([0; 3]) };
    static CHOICE: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}
fn reset(app: &mut App, field: usize) {
    expansion_fixture(app, 70);
    app.game = Game::new_for_agents(3, 3, 280028, Difficulty::Easy);
    app.game.start_match().unwrap();
    app.game.battlefields = vec![field, 0];
    app.game.battlefield_owner = vec![None; 2];
    for p in &mut app.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.set_rune_setup([2, 0, 1, 0, 0, 0], [1, 0, 1, 0, 0, 0])
            .unwrap();
        p.rune_deck = vec![Domain::Fury; 9];
        p.energy_pool = 0;
        p.energy = 1;
        p.power_pool = [0; 6];
    }
    app.game.players[0].units = vec![public_unit(UNIT, 70, None), public_unit(OTHER, 70, None)];
    app.ai_worker.invalidate();
    app.fields = std::array::from_fn(|i| app.game.battlefields[i]);
    app.inspect = None;
    app.field_inspect = None;
    app.rune_menu = None;
    app.rune_menu_id = None;
    app.field_gear_filter = None;
    app.lesson = None;
    app.chain_cursor = 0;
    app.ui.pointer_override = None;
    app.ui.wheel_override = None;
    app.prefs.exact_payment = false;
    app.prefs.coach = true;
    app.companion.set_level(AdviceLevel::Beginner);
}
fn settle(app: &mut App) {
    for _ in 0..60 {
        if app.game.decision.is_some() || (app.game.chain.is_empty() && app.game.showdown.is_none())
        {
            return;
        }
        app.game.apply(Action::Pass).unwrap();
    }
    panic!("C28 public flow did not settle");
}
fn conquer(app: &mut App) {
    app.game
        .apply(Action::Move {
            unit_id: UNIT,
            lane: Some(0),
        })
        .unwrap();
    settle(app);
}
fn peak(app: &mut App) {
    reset(app, 14);
    conquer(app);
    assert_eq!(app.game.delayed_battlefield_triggers().len(), 1);
    app.game.apply(Action::EndTurn).unwrap();
    settle(app);
    assert!(matches!(
        app.game.decision,
        Some(Decision::BattlefieldTargets {
            player: 0,
            kind: BattlefieldTargetKind::TargonPeakReady,
            ..
        })
    ));
    let first = app.game.players[0].rune_objects()[0].id;
    let second = app.game.players[1].rune_objects()[2].id;
    let third = app.game.players[0].rune_objects()[2].id;
    RUNES.with(|r| r.set([first, second, third]));
}
fn click_rune(app: &mut App, owner: usize, id: u32) {
    let index = app
        .reserve(owner)
        .slots
        .iter()
        .position(|s| s.is_some_and(|s| s.rune_id == Some(id)))
        .unwrap();
    click(
        app,
        430.0 + index as f32 * 33.0,
        if owner == 0 { 606.0 } else { 145.0 },
    );
}
fn choice(app: &App) -> u32 {
    match app.game.decision {
        Some(
            Decision::BattlefieldPayment { choice_id, .. }
            | Decision::BattlefieldCost { choice_id, .. },
        ) => choice_id,
        _ => panic!("Dais choice expected"),
    }
}
fn prepare_dais(app: &mut App, add: bool) {
    reset(app, 11);
    app.game.players[0].set_rune_setup([0; 6], [0; 6]).unwrap();
    app.game.players[0].energy_pool = if add { 0 } else { 1 };
    app.game.players[0].energy = if add { 0 } else { 1 };
    if !add {
        app.game.players[0].units[1].card_id = 265;
    }
    if add {
        app.game.players[0].xp = 6;
        app.game.players[0].gear.push(GearState {
            id: 28003,
            card_id: 347,
            lane: None,
            attached_to: None,
            ready: true,
            temporary: false,
            token: false,
        });
    }
    conquer(app);
    CHOICE.with(|c| c.set(choice(app)));
}
pub(super) fn run_frame(app: &mut App, frame: u32) -> bool {
    match frame {
        0 => {
            peak(app);
            capture(app, "RIFT_SMOKE_C28_PEAK_CHOICE_SCREENSHOT");
            RUNES.with(|r| click_rune(app, 0, r.get()[0]));
        }
        1 => {
            assert_eq!(app.targets.len(), 1);
            RUNES.with(|r| click_rune(app, 1, r.get()[1]));
        }
        2 => {
            assert_eq!(app.targets.len(), 2);
            assert!(app.rune_menu.is_none());
            capture(app, "RIFT_SMOKE_C28_PEAK_BOTH_PLAYERS_SCREENSHOT");
            RUNES.with(|r| click_rune(app, 0, r.get()[2]));
        }
        3 => {
            assert_eq!(
                app.targets.len(),
                2,
                "third rune cannot replace a selected copy"
            );
            RUNES.with(|r| click_rune(app, 1, r.get()[1]));
        }
        4 => {
            assert_eq!(app.targets.len(), 1);
            RUNES.with(|r| click_rune(app, 1, r.get()[1]));
        }
        5 => {
            assert_eq!(app.targets.len(), 2);
            click(app, 1285.0, 510.0);
        }
        6 => {
            RUNES.with(|r| {
                assert_eq!(
                    app.game.chain.last().unwrap().targets,
                    vec![
                        Target::Rune {
                            player: 0,
                            rune_id: r.get()[0]
                        },
                        Target::Rune {
                            player: 1,
                            rune_id: r.get()[1]
                        }
                    ]
                )
            });
            capture(app, "RIFT_SMOKE_C28_PEAK_RESPONSES_SCREENSHOT");
            settle(app);
            RUNES.with(|r| {
                assert!(
                    app.game.players[0]
                        .rune_objects()
                        .iter()
                        .find(|v| v.id == r.get()[0])
                        .unwrap()
                        .ready
                );
                assert!(
                    !app.game.players[0]
                        .rune_objects()
                        .iter()
                        .find(|v| v.id == r.get()[2])
                        .unwrap()
                        .ready
                );
            });
            capture(app, "RIFT_SMOKE_C28_PEAK_RESOLVED_SCREENSHOT");
            peak(app);
            click(app, 1285.0, 548.0);
        }
        7 => {
            assert!(app.game.chain.last().unwrap().targets.is_empty());
            settle(app);
            prepare_dais(app, false);
            capture(app, "RIFT_SMOKE_C28_DAIS_RETURN_CHOICE_SCREENSHOT");
            click_unit(app, UNIT);
        }
        8 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::BattlefieldPayment { unit_id: UNIT, .. })
            ));
            CHOICE.with(|c| assert_eq!(choice(app), c.get()));
            assert!(
                app.targets.is_empty(),
                "return cost never enters target draft"
            );
            capture(app, "RIFT_SMOKE_C28_DAIS_COST_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        9 => {
            assert!(!app.game.players[0].units.iter().any(|u| u.id == UNIT));
            assert!(app.game.players[0].hand.contains(&70));
            assert_eq!(app.game.players[0].energy, 0);
            let item = app.game.chain.last().unwrap();
            assert!(item.targets.is_empty());
            assert!(matches!(
                app.game.chain_source(item),
                local_rift::game::ChainSource::Battlefield {
                    battlefield_id: 11,
                    ..
                }
            ));
            capture(app, "RIFT_SMOKE_C28_DAIS_PAID_RESPONSES_SCREENSHOT");
            settle(app);
        }
        10 => {
            let token = app.game.current_pending_token().unwrap();
            assert!(matches!(
                token.source,
                Some(local_rift::game::ChainSource::Battlefield {
                    battlefield_id: 11,
                    ..
                })
            ));
            capture(app, "RIFT_SMOKE_C28_DAIS_TOKEN_SCREENSHOT");
            click(app, 1285.0, 520.0);
        }
        11 => {
            settle(app);
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .any(|u| u.token && u.lane == Some(0))
            );
            capture(app, "RIFT_SMOKE_C28_DAIS_RESOLVED_SCREENSHOT");
            prepare_dais(app, true);
            click_unit(app, UNIT);
        }
        12 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::BattlefieldPayment { .. })
            ));
            click(app, 1285.0, 510.0);
        }
        13 => {
            assert!(matches!(app.game.decision, Some(Decision::Payment { .. })));
            assert!(
                app.payment_action_name(&Action::CompletePayment)
                    .contains("Emperor's Dais")
            );
            capture(app, "RIFT_SMOKE_C28_DAIS_FIXED_PAYMENT_SCREENSHOT");
            click(app, 645.0, 716.0);
        }
        14 => {
            click(app, 1285.0, 480.0);
        }
        15 => {
            click(app, 1285.0, 510.0);
        }
        16 => {
            assert_eq!(app.game.players[0].energy, 1);
            assert!(app.game.can_cancel_payment().unwrap());
            assert!(app.game.players[0].units.iter().any(|u| u.id == UNIT));
            capture(app, "RIFT_SMOKE_C28_DAIS_FUNDED_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        17 => {
            CHOICE.with(|c| assert_eq!(choice(app), c.get()));
            assert_eq!(app.game.players[0].energy, 0);
            assert!(app.game.players[0].gear[0].ready);
            assert!(app.game.players[0].units.iter().any(|u| u.id == UNIT));
            capture(app, "RIFT_SMOKE_C28_DAIS_UNDONE_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        18 => {
            assert!(app.game.decision.is_none() && app.game.chain.is_empty());
            assert!(app.game.players[0].units.iter().any(|u| u.id == UNIT));
            capture(app, "RIFT_SMOKE_C28_DAIS_DECLINED_SCREENSHOT");
            reset(app, 12);
            app.game.players[0].units[0].lane = Some(0);
            app.game.battlefield_owner[0] = Some(0);
            app.game.players[0].energy_pool = 10;
            app.game.players[0].energy = 11;
            app.game.players[0].hand = vec![70];
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: Some(0),
                    target: None,
                })
                .unwrap();
            settle(app);
            capture(app, "RIFT_SMOKE_C28_SPRING_CHOICE_SCREENSHOT");
            click_unit(app, UNIT);
        }
        19 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: UNIT
                }]
            );
            click(app, 1285.0, 510.0);
        }
        20 => {
            settle(app);
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .find(|u| u.id == UNIT)
                    .unwrap()
                    .lane
                    .is_none()
            );
            capture(app, "RIFT_SMOKE_C28_SPRING_RESOLVED_SCREENSHOT");
            reset(app, 13);
            app.game.players[0].units[0].lane = Some(0);
            app.game.battlefield_owner[0] = Some(0);
            app.game.players[0].energy_pool = 10;
            app.game.players[0].energy = 11;
            app.game.players[0].hand = vec![74];
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: Some(Target::Unit {
                        player: 0,
                        unit_id: UNIT,
                    }),
                })
                .unwrap();
            settle(app);
            capture(app, "RIFT_SMOKE_C28_HALL_CHOICE_SCREENSHOT");
            click_unit(app, UNIT);
        }
        21 => {
            assert_eq!(app.targets.len(), 1);
            click(app, 1285.0, 510.0);
        }
        22 => {
            settle(app);
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .find(|u| u.id == UNIT)
                    .unwrap()
                    .temporary_buff
                    > 0
            );
            capture(app, "RIFT_SMOKE_C28_HALL_RESOLVED_SCREENSHOT");
            reset(app, 14);
            conquer(app);
            app.field_inspect = Some(14);
        }
        23 => {
            capture(app, "RIFT_SMOKE_C28_PEAK_RULES_SCREENSHOT");
            app.field_inspect = Some(11);
        }
        24 => {
            capture(app, "RIFT_SMOKE_C28_DAIS_RULES_SCREENSHOT");
            app.field_inspect = None;
            reset(app, 0);
            let id = app.game.players[0].rune_objects()[0].id;
            RUNES.with(|r| r.set([id, 0, 0]));
            click_rune(app, 0, id);
        }
        25 => {
            RUNES.with(|r| assert_eq!(app.rune_menu_id, Some(r.get()[0])));
            capture(app, "RIFT_SMOKE_C28_EXACT_EXHAUSTED_RUNE_SCREENSHOT");
            let p = app.rune_menu_at;
            click(app, p.x + 100.0, p.y + 61.0 + 42.0 + 17.0);
        }
        26 => {
            RUNES.with(|r| {
                assert!(
                    app.game.players[0]
                        .rune_objects()
                        .iter()
                        .all(|v| v.id != r.get()[0])
                )
            });
            assert_eq!(app.game.players[0].power_pool[0], 1);
            assert_eq!(app.game.players[0].rune_objects().len(), 2);
            capture(app, "RIFT_SMOKE_C28_EXACT_RECYCLED_SCREENSHOT");
            let resources = (app.game.players[0].energy, app.game.players[0].power_pool);
            app.rune_menu = Some(Domain::Fury);
            RUNES.with(|r| app.rune_menu_id = Some(r.get()[0]));
            app.draw();
            assert!(app.rune_menu.is_none() && app.rune_menu_id.is_none());
            assert_eq!(
                resources,
                (app.game.players[0].energy, app.game.players[0].power_pool)
            );
        }
        27 => {
            reset(app, 0);
            app.game.players[0].hand = vec![417, 418, 419, 420];
            app.tray = super::super::Tray::Hand;
            app.ui.input_override = Some((vec2(315.0, 760.0), false, true));
        }
        28 | 30 | 32 | 34 => {
            let id = 417 + (frame - 28) as usize / 2;
            assert_eq!(app.inspect, Some(id));
            let variable = match id {
                417 => "RIFT_SMOKE_C28_FAEFOLK_RULES_SCREENSHOT",
                418 => "RIFT_SMOKE_C28_KINKOU_RULES_SCREENSHOT",
                419 => "RIFT_SMOKE_C28_NIDALEE_RULES_SCREENSHOT",
                _ => "RIFT_SMOKE_C28_PYKE_RULES_SCREENSHOT",
            };
            capture(app, variable);
            click(app, 950.0, 704.0);
        }
        29 | 31 | 33 => {
            assert!(app.inspect.is_none());
            let slot = (frame - 27) / 2;
            app.ui.input_override = Some((vec2(315.0 + slot as f32 * 150.0, 760.0), false, true));
        }
        35 => {
            assert!(app.inspect.is_none());
            app.in_match = false;
            app.screen = Screen::Practice;
            app.fields = [0, 1];
            click(app, 450.0, 876.0);
        }
        36 | 41 => {
            assert!(app.field_picker.is_some());
            for _ in 0..6 {
                app.ui.input_override = Some((vec2(480.0, 400.0), false, false));
                app.ui.wheel_override = Some(-1.0);
                app.draw();
            }
            app.ui.input_override = None;
            app.ui.wheel_override = None;
            assert_eq!(app.field_picker.as_ref().unwrap().list_scroll, 6);
        }
        37 | 42 => click(app, 480.0, 616.0),
        38 | 43 => {
            assert_eq!(app.field_picker.as_ref().unwrap().selected, 16);
            capture(
                app,
                if frame == 38 {
                    "RIFT_SMOKE_C28_LAST_FIELD_SETUP_SCREENSHOT"
                } else {
                    "RIFT_SMOKE_C28_LAST_FIELD_EDITOR_SCREENSHOT"
                },
            );
            click(app, 870.0, 715.0);
        }
        39 => {
            assert_eq!(app.fields, [16, 1]);
            assert!(app.field_picker.is_none());
            app.open_deck_editor(None);
            app.deck_editor.as_mut().unwrap().deck.battlefield = 0;
        }
        40 => click(app, 390.0, 630.0),
        44 => {
            assert_eq!(app.deck_editor.as_ref().unwrap().deck.battlefield, 16);
            assert!(app.field_picker.is_none());
            reset(app, 11);
            app.game.players[0].energy_pool = 1;
            app.game.players[0].energy = 2;
            app.game.players[0].units[0].card_id = 178;
            app.game.players[0].units[0].token = true;
            app.game.players[0].units[0].buff = 2;
            conquer(app);
            click_unit(app, UNIT);
        }
        45 => {
            assert_eq!(
                super::super::field_ui::payment_name(&app.game).as_deref(),
                Some("Emperor's Dais · return Sand Soldier token")
            );
            capture(app, "RIFT_SMOKE_C28_DAIS_TOKEN_COST_SCREENSHOT");
            println!(
                "PASS: 1261 native UI frames including 46 C28 controls and previous 1215; focused C28 runs only 46 frames."
            );
            return true;
        }
        _ => panic!("C28 smoke exceeded its 46-frame script"),
    }
    false
}
