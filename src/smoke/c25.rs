//! Payment refusal through real controls, including an unmodified public match.
use super::*;
use local_rift::payment_plan::PaymentRefusalKind;

thread_local! {
    static CHILD_ORIGIN: std::cell::RefCell<Option<(Vec<usize>, Vec<usize>)>> = const { std::cell::RefCell::new(None) };
    static NESTED_STATE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    static TOKEN_ID: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

fn child_action() -> Action {
    Action::FinalizePendingPlay {
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate: false,
        bonus: false,
    }
}

/// Replays the legal seed-1 Clocktower audit: no post-start state mutation.
fn clocktower_child() -> Game {
    let deck = local_rift::deck::CustomDeck {
        name: "C25 public payment audit".into(),
        legend: 6,
        chosen_champion: 182,
        cards: vec![
            (182, 1),
            (384, 3),
            (294, 3),
            (381, 3),
            (145, 3),
            (133, 3),
            (61, 3),
            (77, 3),
            (109, 3),
            (126, 3),
            (129, 3),
            (60, 3),
            (68, 3),
            (57, 3),
        ],
        runes: [0, 0, 6, 0, 0, 6],
        battlefield: 3,
    };
    let mut game = Game::new_with_custom(
        [0, 1],
        [Some(deck.clone()), Some(deck)],
        1,
        Difficulty::Tactical,
        false,
    )
    .unwrap();
    game.choose_starting_player(0).unwrap();
    game.choose_battlefield(0, 3).unwrap();
    game.choose_battlefield(1, 3).unwrap();
    game.start_match().unwrap();
    for _ in 0..30 {
        game.apply(Action::EndTurn).unwrap();
    }
    assert_eq!(game.players[0].hand[19], 384);
    game.apply(Action::PlayCardWithTargets {
        hand_index: 19,
        lane: None,
        targets: vec![],
    })
    .unwrap();
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    let Some(Decision::PredictCards {
        choice_id,
        ref cards,
        ..
    }) = game.decision
    else {
        panic!("Clocktower Predict")
    };
    game.apply(Action::ResolvePredict {
        choice_id,
        recycle: vec![],
        top_order: (0..cards.len()).collect(),
    })
    .unwrap();
    game.apply(Action::EndTurn).unwrap();
    game.apply(Action::EndTurn).unwrap();
    assert_eq!(game.players[0].hand[13], 294);
    game.apply(Action::PlayCardWithTargets {
        hand_index: 13,
        lane: None,
        targets: vec![Target::Unit {
            player: 0,
            unit_id: 1,
        }],
    })
    .unwrap();
    for _ in 0..4 {
        game.apply(Action::Pass).unwrap();
    }
    game.apply(Action::Choose { accept: true }).unwrap();
    assert!(matches!(
        game.decision,
        Some(Decision::PendingPlay { card_id: 109, .. })
    ));
    assert!(game.players[0].units.iter().all(|u| u.card_id != 384));
    assert!(game.players[0].discard.contains(&294));
    assert!(
        game.cost_plan(&child_action())
            .unwrap()
            .power
            .iter()
            .any(|cost| cost.amount > 0)
    );
    game
}

fn reset(app: &mut App, card: usize) {
    expansion_fixture(app, card);
    app.inspect = None;
    app.field_inspect = None;
    app.field_gear_filter = None;
    app.lesson = None;
    app.rune_menu = None;
    app.chain_cursor = 0;
    app.prefs.coach = true;
    app.prefs.advanced = false;
    app.companion.set_level(AdviceLevel::Beginner);
}

fn prepare_child(app: &mut App) {
    reset(app, 15);
    app.game = clocktower_child();
    app.fields = app.game.battlefields;
    CHILD_ORIGIN.with(|origin| {
        *origin.borrow_mut() = Some((
            app.game.players[0].banish.clone(),
            app.game.players[0].discard.clone(),
        ))
    });
}

fn assert_child_refused(app: &App) {
    assert!(app.game.pending_cards().is_empty());
    assert!(app.game.decision.is_none());
    assert!(app.game.chain.is_empty());
    assert!(
        app.game.players[0]
            .units
            .iter()
            .all(|u| u.card_id != 109 && u.card_id != 384)
    );
    CHILD_ORIGIN.with(|origin| {
        let original = origin.borrow();
        let (banish, trash) = original.as_ref().unwrap();
        let mut expected = banish.clone();
        expected.push(109);
        assert_eq!(app.game.players[0].banish, expected);
        assert_eq!(
            &app.game.players[0].discard, trash,
            "parent remains resolved"
        );
    });
    assert!(app.payment_draft.is_none());
    assert!(app.payment_origin.is_none());
    assert!(app.suspended_payments.is_empty());
    assert!(app.resource_drafts.is_empty());
    assert!(app.selection.is_none());
}

pub(super) fn run_frame(app: &mut App, frame: u32) -> bool {
    match frame {
        0 => {
            reset(app, 15);
            save_state(app);
            assert!(app.begin_ui_payment(Action::PlayCard {
                hand_index: 0,
                lane: None,
                target: None
            }));
            let domain = app.game.catalog[15].domain;
            app.game
                .apply(Action::ActivateRune {
                    domain,
                    recycle: false,
                })
                .unwrap();
        }
        1 => {
            assert_eq!(
                app.game.payment_plan_view(0).unwrap().refusal_kind,
                Some(PaymentRefusalKind::Ordinary)
            );
            assert!(app.game.can_cancel_payment().unwrap());
            assert!(app.game.can_refuse_payment().unwrap());
            assert!(app.game.clone().apply(Action::CompletePayment).is_ok());
            capture(app, "RIFT_SMOKE_C25_ORDINARY_FUNDED_UNDO_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        2 => {
            assert_same_state(app);
            assert!(app.payment_origin.is_none());
            prepare_child(app);
            save_state(app);
            assert!(app.begin_ui_payment(child_action()));
        }
        3 => {
            assert!(
                app.payment_action_name(&Action::CompletePayment)
                    .contains("Shen, Kinkou")
            );
            assert!(
                app.payment_action_name(&Action::CompletePayment)
                    .contains("banish")
            );
            assert_eq!(
                app.game.payment_plan_view(0).unwrap().refusal_kind,
                Some(PaymentRefusalKind::PendingCard)
            );
            assert!(app.game.clone().apply(Action::CompletePayment).is_ok());
            capture(app, "RIFT_SMOKE_C25_CHILD_PAYMENT_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        4 => {
            assert_same_state(app);
            app.prefs.exact_payment = true;
            app.action(child_action());
        }
        5 => {
            assert!(app.payment_draft.is_some());
            capture(app, "RIFT_SMOKE_C25_CHILD_EXACT_PAYMENT_SCREENSHOT");
            click(app, 875.0, 595.0);
        }
        6 => {
            assert_eq!(app.suspended_payments.len(), 1);
            assert!(app.payment_draft.is_none());
            assert!(app.game.can_refuse_payment().unwrap());
            capture(app, "RIFT_SMOKE_C25_CHILD_SUSPENDED_REFUSAL_SCREENSHOT");
            click(app, 710.0, 106.0);
        }
        7 => {
            assert_child_refused(app);
            capture(app, "RIFT_SMOKE_C25_CHILD_REFUSED_SCREENSHOT");
            prepare_child(app);
            assert!(app.begin_ui_payment(child_action()));
        }
        8 => {
            click(app, 1285.0, 578.0);
        }
        9 => {
            assert_child_refused(app);
            reset(app, 403);
            app.game
                .apply(Action::PlayCard {
                    hand_index: 0,
                    lane: Some(0),
                    target: None,
                })
                .unwrap();
            let Some(Decision::TriggerPayment { choice_id, .. }) = app.game.decision else {
                panic!("Poster payment")
            };
            save_state(app);
            assert!(app.begin_ui_payment(Action::ResolveTriggerPayment {
                choice_id,
                pay: true
            }));
        }
        10 => {
            assert_eq!(
                app.game.payment_plan_view(0).unwrap().refusal_kind,
                Some(PaymentRefusalKind::Trigger)
            );
            capture(app, "RIFT_SMOKE_C25_TRIGGER_UNDO_OR_DECLINE_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        11 => {
            assert_same_state(app);
            let Some(Decision::TriggerPayment { choice_id, .. }) = app.game.decision else {
                panic!("Same trigger restored")
            };
            assert!(app.begin_ui_payment(Action::ResolveTriggerPayment {
                choice_id,
                pay: true
            }));
        }
        12 => {
            click(app, 1285.0, 578.0);
        }
        13 => {
            assert!(app.game.decision.is_none());
            assert!(app.game.chain.is_empty());
            assert!(app.game.players[0].units.is_empty());
            assert!(app.game.players[0].gear.iter().any(|g| g.card_id == 403));
            reset(app, 15);
            app.game.players[0].gear.push(GearState {
                id: 25001,
                card_id: 244,
                lane: None,
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            });
            save_state(app);
            assert!(app.begin_ui_payment(Action::PlayCard {
                hand_index: 0,
                lane: None,
                target: None
            }));
            app.game
                .apply(Action::ActivateAbility {
                    source: local_rift::activated::AbilitySource::Gear {
                        player: 0,
                        gear_id: 25001,
                    },
                    ability_index: 0,
                    targets: vec![],
                })
                .unwrap();
        }
        14 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::ResourceConversion { .. })
            ));
            assert!(!app.game.can_refuse_payment().unwrap());
            assert!(!app.game.can_cancel_payment().unwrap());
            capture(app, "RIFT_SMOKE_C25_NESTED_CONVERSION_SCREENSHOT");
            NESTED_STATE.with(|state| *state.borrow_mut() = Some(format!("{:?}", app.game)));
            click(app, 1285.0, 578.0);
        }
        15 => {
            NESTED_STATE.with(|state| {
                assert_eq!(state.borrow().as_ref().unwrap(), &format!("{:?}", app.game))
            });
            click(app, 1285.0, 548.0);
        }
        16 => {
            assert!(matches!(app.game.decision, Some(Decision::Payment { .. })));
            assert!(app.game.can_refuse_payment().unwrap());
            capture(app, "RIFT_SMOKE_C25_CONVERSION_FINISHED_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        17 => {
            assert_same_state(app);
            app.inspect_printed_card(401);
        }
        18 => {
            capture(app, "RIFT_SMOKE_C25_GEAR_COST_INSPECTOR_SCREENSHOT");
            app.inspect_printed_card(9);
        }
        19 => {
            capture(app, "RIFT_SMOKE_C25_SPELL_COST_INSPECTOR_SCREENSHOT");
            app.inspect_printed_card(15);
        }
        20 => {
            capture(app, "RIFT_SMOKE_C25_UNIT_COST_INSPECTOR_SCREENSHOT");
            click(app, 960.0, 704.0);
        }
        21 => {
            assert!(app.inspect.is_none());
            reset(app, 87);
            app.game.players[0].units = vec![public_unit(25002, 265, None)];
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                })
                .unwrap();
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TokenPlay { remaining: 4, .. })
            ));
            TOKEN_ID.with(|id| id.set(app.game.current_pending_token().unwrap().id));
            save_state(app);
            assert!(app.begin_ui_payment(Action::FinalizeTokenPlay {
                lane: None,
                accelerate: true
            }));
        }
        22 => {
            assert_eq!(
                app.game.payment_plan_view(0).unwrap().refusal_kind,
                Some(PaymentRefusalKind::PendingToken)
            );
            capture(app, "RIFT_SMOKE_C25_TOKEN_UNDO_OR_DECLINE_SCREENSHOT");
            click(app, 1285.0, 548.0);
        }
        23 => {
            assert_same_state(app);
            TOKEN_ID.with(|id| assert_eq!(id.get(), app.game.current_pending_token().unwrap().id));
            assert!(app.begin_ui_payment(Action::FinalizeTokenPlay {
                lane: None,
                accelerate: true
            }));
        }
        24 => {
            click(app, 1285.0, 578.0);
        }
        25 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TokenPlay { remaining: 3, .. })
            ));
            TOKEN_ID.with(|id| assert_ne!(id.get(), app.game.current_pending_token().unwrap().id));
            assert!(app.game.players[0].discard.contains(&87));
            assert!(app.game.players[0].units.iter().all(|u| !u.token));
            for _ in 0..3 {
                app.game
                    .apply(Action::FinalizeTokenPlay {
                        lane: None,
                        accelerate: false,
                    })
                    .unwrap();
            }
            assert_eq!(
                app.game.players[0].units.iter().filter(|u| u.token).count(),
                3
            );
            reset(app, 381);
            app.game.players[0].units = vec![public_unit(25003, 145, None)];
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![Target::Unit {
                        player: 0,
                        unit_id: 25003,
                    }],
                })
                .unwrap();
            finish_chain_until_choice(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::PendingPlay { .. })
            ));
            assert!(app.begin_ui_payment(child_action()));
            save_state(app);
        }
        26 => {
            assert_eq!(app.game.payment_plan_view(0).unwrap().refusal_kind, None);
            assert!(!app.game.can_refuse_payment().unwrap());
            assert!(!app.game.can_cancel_payment().unwrap());
            capture(app, "RIFT_SMOKE_C25_ZERO_COST_MANDATORY_SCREENSHOT");
            click(app, 1285.0, 578.0);
        }
        27 => {
            assert_same_state(app);
            click(app, 1285.0, 510.0);
        }
        28 => {
            assert!(app.game.decision.is_none());
            assert!(app.game.pending_cards().is_empty());
            assert!(
                app.game.players[0]
                    .units
                    .iter()
                    .any(|u| u.card_id == 145 && u.lane.is_none())
            );
            assert!(app.game.players[0].discard.contains(&381));
            app.ui.input_override = None;
            println!(
                "PASS: 1161 native UI frames including 29 payment refusal controls and all previous 1132 controls; the focused C25 option runs only the appended 29 frames."
            );
            return true;
        }
        _ => panic!("C25 smoke exceeded its 29-frame script"),
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_clocktower_child_undo_and_refusal_preserve_the_resolved_parent() {
        let original = clocktower_child();
        let mut undone = original.clone();
        undone
            .apply(Action::BeginPayment {
                action: Box::new(child_action()),
            })
            .unwrap();
        undone.apply(Action::Choose { accept: false }).unwrap();
        assert_eq!(format!("{undone:?}"), format!("{original:?}"));
        let mut refused = original.clone();
        refused
            .apply(Action::BeginPayment {
                action: Box::new(child_action()),
            })
            .unwrap();
        refused.apply(Action::RefusePayment).unwrap();
        assert!(refused.pending_cards().is_empty());
        assert_eq!(refused.players[0].discard, original.players[0].discard);
        assert!(refused.players[0].banish.contains(&109));
        assert!(refused.players[0].units.is_empty());
    }
}
