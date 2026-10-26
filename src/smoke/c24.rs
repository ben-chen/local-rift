//! Real input/actions for independent gear, modal plays and trigger finalization.
use super::*;
thread_local! { static TRIGGER_CHOICE: std::cell::Cell<u32> = const { std::cell::Cell::new(0) }; }

fn reset(app: &mut App, card: usize) {
    expansion_fixture(app, card);
    app.game.players[0].units.clear();
    app.game.players[1].units.clear();
    app.game.players[0].gear.clear();
    app.game.players[1].gear.clear();
    app.game.battlefield_owner = [None; 2];
    app.field_gear_filter = None;
    app.inspect = None;
    app.inspect_scroll = 0;
    app.field_inspect = None;
    app.lesson = None;
    app.confirm_restart = false;
    app.rune_menu = None;
    app.chain_cursor = 0;
    app.prefs.coach = true;
    app.prefs.advanced = false;
    app.companion.set_level(AdviceLevel::Beginner);
}

fn place(app: &mut App, card: usize, lane: usize) -> u32 {
    app.game.players[0].hand = vec![card];
    app.game.players[0].energy_pool = 30;
    app.game.players[0].energy = 30;
    app.game.players[0].power_pool = [30; 6];
    app.game
        .apply(Action::PlayCard {
            hand_index: 0,
            lane: Some(lane),
            target: None,
        })
        .unwrap();
    app.game.players[0].gear.last().unwrap().id
}

fn gear_rect(lane: usize, owner: usize) -> Rect {
    super::super::board_layout::gear_rect(lane, owner)
}

fn dense(app: &mut App) {
    reset(app, 401);
    for _ in 0..6 {
        place(app, 401, 0);
    }
    for owner in 0..2 {
        app.game.players[owner].units = (0..7)
            .map(|index| public_unit(24000 + owner as u32 * 100 + index, 24, Some(0)))
            .collect();
    }
    app.game.players[1].gear = (0..5)
        .map(|index| GearState {
            id: 24200 + index,
            card_id: 404,
            lane: Some(0),
            attached_to: None,
            ready: index != 0,
            temporary: false,
            token: false,
        })
        .collect();
    app.game.players[0].gear.push(GearState {
        id: 24300,
        card_id: 226,
        lane: Some(0),
        attached_to: Some(24000),
        ready: true,
        temporary: false,
        token: false,
    });
    app.game.battlefield_owner = [None; 2];
    app.game.players[0].hand = vec![410];
    app.selection = None;
    app.tray = super::super::Tray::Board;
}

pub(super) fn run_frame(app: &mut App, frame: u32) -> bool {
    match frame {
        0 => {
            reset(app, 401);
            click(app, 315.0, 760.0);
        }
        1 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            assert!(
                app.drop_action(
                    &super::super::DragSource::Hand {
                        index: 0,
                        card_id: 401
                    },
                    super::super::DropZone::Base
                )
                .is_err()
            );
            click_field(app, 0);
        }
        2 => {
            assert_eq!(app.game.players[0].gear.len(), 1);
            assert_eq!(app.game.players[0].gear[0].lane, Some(0));
            assert_eq!(app.game.battlefield_owner, [None; 2]);
            capture(app, "RIFT_SMOKE_C24_GEAR_NEUTRAL_SCREENSHOT");
            reset(app, 402);
            app.game.players[1].units = vec![public_unit(24001, 24, Some(1))];
            app.game.battlefield_owner[1] = Some(1);
            click(app, 315.0, 760.0);
            app.ui.pointer_override = Some((true, false));
        }
        3 | 4 => {
            let rect = unit_rect(app, 24001);
            app.ui.input_override = Some((rect.center(), false, false));
            app.ui.pointer_override = Some((frame == 3, frame == 4));
        }
        5 => {
            assert_eq!(app.game.players[0].gear[0].lane, Some(1));
            assert_eq!(app.game.players[0].gear[0].attached_to, None);
            assert_eq!(app.game.battlefield_owner[1], Some(1));
            capture(app, "RIFT_SMOKE_C24_ENEMY_FIELD_DROP_SCREENSHOT");
            dense(app);
        }
        6 => {
            assert_eq!(super::super::scene_gear(&app.game).len(), 11);
            capture(app, "RIFT_SMOKE_C24_DENSE_GEAR_SCREENSHOT");
            let r = gear_rect(0, 0);
            click(app, r.x + r.w - 23.0, r.y + 14.0);
        }
        7 => {
            assert!(app.tray == super::super::Tray::Loadout);
            assert_eq!(app.field_gear_filter, Some(0));
            capture(app, "RIFT_SMOKE_C24_FILTERED_LOADOUT_SCREENSHOT");
            app.ui.input_override = Some((vec2(615.0, 745.0), false, true));
        }
        8 => {
            assert_eq!(app.inspect, Some(401));
            assert!(app.hover_object.is_some());
            capture(app, "RIFT_SMOKE_C24_GEAR_INSPECTOR_SCREENSHOT");
            click(app, 960.0, 704.0);
        }
        9 => {
            assert!(app.inspect.is_none());
            reset(app, 401);
            place(app, 401, 0);
            app.game.players[0].hand = vec![410];
            app.game.players[0].deck.extend([401, 3, 401, 4, 401]);
            click(app, 315.0, 760.0);
        }
        10 => {
            capture(app, "RIFT_SMOKE_C24_COMMAND_MODES_SCREENSHOT");
            assert!(
                app.play_draft
                    .as_ref()
                    .is_some_and(|d| d.open && d.choices.mode.is_none()),
                "mode draft: selected_hand={} card={:?} draft={} phase={:?} decision={:?} toast={}",
                app.selection == Some(Selection::Hand(0)),
                app.selected_card_id(),
                app.play_draft.is_some(),
                app.game.phase,
                app.game.decision,
                app.toast
            );
            save_state(app);
            click(app, 795.0, 770.0);
        }
        11 => {
            assert_same_state(app);
            assert_eq!(app.play_draft.as_ref().unwrap().choices.mode, Some(0));
            let r = gear_rect(0, 0);
            click(app, r.x + 55.0, r.y + 35.0);
        }
        12 => {
            let id = app.game.players[0].gear[0].id;
            assert_eq!(
                app.targets,
                vec![Target::Gear {
                    player: 0,
                    gear_id: id
                }]
            );
            assert!(app.targeting_battlefield());
            click_field(app, 1);
        }
        13 => {
            assert_eq!(app.targets.len(), 2);
            assert_eq!(app.targets[1], Target::Battlefield { lane: 1 });
            capture(app, "RIFT_SMOKE_C24_COMMAND_MOVE_TARGETS_SCREENSHOT");
            click(app, 1005.0, 770.0);
        }
        14 => {
            assert_same_state(app);
            assert!(app.targets.is_empty());
            assert_eq!(app.play_draft.as_ref().unwrap().choices.mode, Some(1));
            capture(app, "RIFT_SMOKE_C24_COMMAND_DRAW_MODE_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        15 => {
            assert!(!app.game.chain.is_empty());
            finish_chain_until_choice(app);
        }
        16 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::DrawLook {
                    kind: local_rift::game::DrawLookKind::CommandAttack,
                    ..
                })
            ));
            capture(app, "RIFT_SMOKE_C24_COMMAND_PRIVATE_LOOK_SCREENSHOT");
            click(app, 315.0, 760.0);
        }
        17 => {
            click(app, 1285.0, 510.0);
        }
        18 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::RevealedCard { card_id: 401, .. })
            ));
            capture(app, "RIFT_SMOKE_C24_COMMAND_REVEAL_SCREENSHOT");
            app.game.apply(Action::Choose { accept: true }).unwrap();
            reset(app, 403);
            place(app, 403, 0);
        }
        19 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerPayment { card_id: 403, .. })
            ));
            assert!(app.game.chain.is_empty());
            capture(app, "RIFT_SMOKE_C24_TRIGGER_PAYMENT_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        20 => {
            assert!(app.game.decision.is_none());
            assert!(app.game.chain.is_empty());
            assert!(app.game.players[0].units.is_empty());
            reset(app, 403);
            place(app, 403, 0);
            click(app, 1285.0, 510.0);
        }
        21 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.chain.len(), 1);
            assert!(app.game.players[0].units.is_empty());
            capture(app, "RIFT_SMOKE_C24_PAID_TRIGGER_RESPONSE_SCREENSHOT");
            finish_chain_until_choice(app);
        }
        22 => {
            assert!(app.game.players[0].units.iter().any(|unit| unit.token));
            capture(app, "RIFT_SMOKE_C24_TRIGGER_TOKEN_SCREENSHOT");
            reset(app, 403);
            place(app, 403, 0);
            app.game.players[0].runes = [0; 6];
            app.game.players[0].exhausted_runes = [0; 6];
            app.game.players[0].energy = 0;
            app.game.players[0].energy_pool = 0;
            app.game.players[0].xp = 6;
            app.game.players[0].gear.push(GearState {
                id: 24400,
                card_id: 347,
                lane: None,
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            });
            let Some(Decision::TriggerPayment { choice_id, .. }) = app.game.decision else {
                panic!("Poster payment")
            };
            TRIGGER_CHOICE.with(|saved| saved.set(choice_id));
        }
        23 => {
            click(app, 1285.0, 510.0);
        }
        24 => {
            assert!(matches!(app.game.decision, Some(Decision::Payment { .. })));
            capture(app, "RIFT_SMOKE_C24_TRIGGER_FIXED_PAYMENT_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        25 => {
            let Some(Decision::TriggerPayment { choice_id, .. }) = app.game.decision else {
                panic!("cancel restores trigger")
            };
            TRIGGER_CHOICE.with(|saved| assert_eq!(saved.get(), choice_id));
            assert!(
                app.game.players[0]
                    .gear
                    .iter()
                    .find(|g| g.id == 24400)
                    .unwrap()
                    .ready
            );
            click(app, 1285.0, 510.0);
        }
        26 => {
            assert!(matches!(app.game.decision, Some(Decision::Payment { .. })));
            click(app, 795.0, 716.0);
        }
        27 => {
            assert!(matches!(
                app.selection,
                Some(Selection::Ability(
                    local_rift::activated::AbilitySource::Gear { gear_id: 24400, .. },
                    None
                ))
            ));
            click(app, 1285.0, 480.0);
        }
        28 => {
            assert!(matches!(
                app.selection,
                Some(Selection::Ability(_, Some(1)))
            ));
            click(app, 1285.0, 510.0);
        }
        29 => {
            assert!(
                !app.game.players[0]
                    .gear
                    .iter()
                    .find(|g| g.id == 24400)
                    .unwrap()
                    .ready
            );
            assert_eq!(app.game.players[0].energy, 1);
            assert!(matches!(app.game.decision, Some(Decision::Payment { .. })));
            assert!(
                app.selection.is_none(),
                "Add returns to the pending trigger payment panel"
            );
            capture(app, "RIFT_SMOKE_C24_TRIGGER_ADD_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        30 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.chain.len(), 1);
            finish_chain_until_choice(app);
            assert!(app.game.players[0].units.iter().any(|unit| unit.token));
            reset(app, 406);
            app.game.players[1].units = vec![public_unit(24401, 24, Some(0))];
            app.game.players[1].units[0].ready = false;
            place(app, 406, 0);
        }
        31 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerTargets { card_id: 406, .. })
            ));
            capture(app, "RIFT_SMOKE_C24_PORTAL_TARGET_SCREENSHOT");
            click_unit(app, 24401);
        }
        32 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: 24401
                }]
            );
            click(app, 1285.0, 510.0);
        }
        33 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerPayment { card_id: 406, .. })
            ));
            assert!(!app.game.players[1].units[0].ready);
            app.prefs.exact_payment = true;
            capture(app, "RIFT_SMOKE_C24_PORTAL_PAYMENT_SCREENSHOT");
            save_state(app);
            click(app, 1285.0, 510.0);
        }
        34 => {
            assert!(app.payment_draft.is_some());
            assert_same_state(app);
            capture(app, "RIFT_SMOKE_C24_PORTAL_EXACT_PAYMENT_SCREENSHOT");
            click(app, 480.0, 709.0);
        }
        35 => {
            assert!(app.payment_draft.is_none());
            assert_same_state(app);
            click(app, 1285.0, 510.0);
        }
        36 => {
            assert!(app.payment_draft.is_some());
            click(app, 936.0, 709.0);
        }
        37 => {
            assert!(app.payment_draft.is_none());
            assert!(app.game.decision.is_none());
            assert!(!app.game.players[1].units[0].ready);
            assert_eq!(app.game.chain.len(), 1);
            capture(app, "RIFT_SMOKE_C24_PORTAL_PAID_RESPONSE_SCREENSHOT");
            finish_chain_until_choice(app);
        }
        38 => {
            assert!(app.game.players[1].units[0].ready);
            reset(app, 401);
            place(app, 401, 0);
            app.game.players[1].units = vec![public_unit(24402, 24, Some(0))];
            app.game.battlefield_owner[0] = Some(1);
            app.game.apply(Action::EndTurn).unwrap();
        }
        39 => {
            assert!(!app.game.players[0].gear.is_empty());
            assert!(!app.game.chain.is_empty());
            capture(app, "RIFT_SMOKE_C24_OPPONENT_HOLD_SCREENSHOT");
            finish_chain_until_choice(app);
        }
        40 => {
            assert!(app.game.players[0].gear.is_empty());
            assert_eq!(app.game.players[1].score, 1);
            capture(app, "RIFT_SMOKE_C24_HOLD_GEAR_GONE_SCREENSHOT");
            reset(app, 401);
            place(app, 401, 0);
            place(app, 401, 0);
            app.game.players[0].units = vec![public_unit(24410, 24, Some(0))];
            app.game.players[0].gear[0].attached_to = Some(24410);
            app.game.battlefield_owner[0] = Some(0);
            app.game.players[0].hand = vec![410];
            click(app, 315.0, 760.0);
        }
        41 => {
            app.sync_play_draft().unwrap();
            click(app, 795.0, 770.0);
        }
        42 => {
            click(app, 460.0, 807.0);
        }
        43 => {
            assert!(app.tray == super::super::Tray::Loadout);
            assert!(app.play_draft.as_ref().is_some_and(|d| !d.open));
            capture(app, "RIFT_SMOKE_C24_ATTACHED_COMMAND_TARGET_SCREENSHOT");
            click(app, 615.0, 760.0);
        }
        44 => {
            let id = app.game.players[0].gear[0].id;
            assert_eq!(
                app.targets,
                vec![Target::Gear {
                    player: 0,
                    gear_id: id
                }]
            );
            click_field(app, 1);
        }
        45 => {
            assert_eq!(app.targets.len(), 2);
            click(app, 1285.0, 510.0);
        }
        46 => {
            assert!(!app.game.chain.is_empty());
            finish_chain_until_choice(app);
        }
        47 => {
            let attached = &app.game.players[0].gear[0];
            assert_eq!(attached.attached_to, Some(24410));
            assert_eq!(app.game.gear_location(0, attached.id), Some(0));
            assert!(
                super::super::live_gear_description(&app.game, 0, attached.id)
                    .unwrap()
                    .contains("does not move while attached")
            );
            capture(app, "RIFT_SMOKE_C24_ATTACHED_COMMAND_NOOP_SCREENSHOT");
            reset(app, 401);
            place(app, 401, 0);
            app.game.players[0].units = vec![public_unit(24411, 409, Some(0))];
            app.game.battlefield_owner[0] = Some(0);
            assert_eq!(
                app.game.unit_might(&app.game.players[0].units[0]).unwrap(),
                u64::from(app.game.catalog[409].might) + 1
            );
            app.game.players[0].hand = vec![410];
            click(app, 315.0, 760.0);
        }
        48 => {
            app.sync_play_draft().unwrap();
            click(app, 795.0, 770.0);
        }
        49 => {
            let r = gear_rect(0, 0);
            click(app, r.x + 55.0, r.y + 35.0);
        }
        50 => {
            assert_eq!(app.targets.len(), 1);
            click_field(app, 1);
        }
        51 => {
            assert_eq!(app.targets.len(), 2);
            click(app, 1285.0, 510.0);
        }
        52 => {
            assert!(!app.game.chain.is_empty());
            finish_chain_until_choice(app);
        }
        53 => {
            assert_eq!(app.game.players[0].gear[0].lane, Some(1));
            assert_eq!(
                app.game.unit_might(&app.game.players[0].units[0]).unwrap(),
                u64::from(app.game.catalog[409].might)
            );
            capture(app, "RIFT_SMOKE_C24_MOVED_GEAR_LOCAL_EFFECT_SCREENSHOT");
            reset(app, 401);
            save_state(app);
            click(app, 315.0, 760.0);
            app.ui.pointer_override = Some((true, false));
        }
        54 | 55 => {
            app.ui.input_override = Some((vec2(685.0, 530.0), false, false));
            app.ui.pointer_override = Some((frame == 54, frame == 55));
        }
        56 => {
            assert_same_state(app);
            assert!(app.drag.is_none());
            reset(app, 403);
            place(app, 403, 0);
            app.game.players[0].runes = [0; 6];
            app.game.players[0].exhausted_runes = [0; 6];
            app.game.players[0].energy = 0;
            app.game.players[0].energy_pool = 0;
            app.game.players[0].xp = 6;
            app.game.players[0].gear.push(GearState {
                id: 24400,
                card_id: 347,
                lane: None,
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            });
            let Some(Decision::TriggerPayment { choice_id, .. }) = app.game.decision else {
                panic!("Poster choice")
            };
            TRIGGER_CHOICE.with(|saved| saved.set(choice_id));
        }
        57 => {
            click(app, 1285.0, 510.0);
        }
        58 => {
            assert!(matches!(app.game.decision, Some(Decision::Payment { .. })));
            click(app, 795.0, 716.0);
        }
        59 => {
            click(app, 1285.0, 480.0);
        }
        60 => {
            click(app, 1285.0, 510.0);
        }
        61 => {
            assert_eq!(app.game.players[0].energy, 1);
            assert!(app.game.can_cancel_payment().unwrap());
            assert!(
                !app.game.players[0]
                    .gear
                    .iter()
                    .find(|g| g.id == 24400)
                    .unwrap()
                    .ready
            );
            capture(app, "RIFT_SMOKE_C24_FUNDED_OPTIONAL_PAYMENT_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        62 => {
            let Some(Decision::TriggerPayment { choice_id, .. }) = app.game.decision else {
                panic!("Funded undo restores optional choice")
            };
            TRIGGER_CHOICE.with(|saved| assert_eq!(saved.get(), choice_id));
            assert_eq!(app.game.players[0].energy, 0);
            assert!(
                app.game.players[0]
                    .gear
                    .iter()
                    .find(|g| g.id == 24400)
                    .unwrap()
                    .ready
            );
            assert!(app.game.chain.is_empty());
            capture(app, "RIFT_SMOKE_C24_OPTIONAL_PAYMENT_UNDONE_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        63 => {
            assert!(app.game.decision.is_none() && app.game.chain.is_empty());
            reset(app, 401);
            place(app, 401, 0);
            let r = gear_rect(0, 0);
            app.ui.input_override = Some((vec2(r.x + 55.0, r.y + 35.0), false, true));
        }
        64 => {
            assert_eq!(app.inspect, Some(401));
            let text = app.game.catalog[401].official_text;
            assert_eq!(
                app.ui
                    .text_lines(text, 394.0, 13.0)
                    .join(" ")
                    .split_whitespace()
                    .collect::<Vec<_>>(),
                text.split_whitespace().collect::<Vec<_>>()
            );
            app.ui.input_override = Some((vec2(950.0, 530.0), false, false));
            app.ui.wheel_override = Some(-100.0);
        }
        65 => {
            assert!(app.inspect_scroll > 0);
            app.ui.wheel_override = Some(0.0);
            capture(app, "RIFT_SMOKE_C24_GEAR_FULL_RULES_SCREENSHOT");
            click(app, 960.0, 704.0);
        }
        66 => {
            assert!(app.inspect.is_none());
            app.ui.input_override = None;
            app.ui.pointer_override = None;
            app.ui.wheel_override = None;
            println!(
                "PASS: 1132 native UI frames including 67 Deploy gear, modal spell, and trigger-payment controls; the focused C24 option runs only the appended 67 frames."
            );
            return true;
        }
        _ => panic!("C24 smoke exceeded its 67-frame script"),
    }
    false
}
