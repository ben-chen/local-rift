//! Spirit Wheel's initial instruction cost is paid at finalization (CR383.3.b).
use super::*;
use test_support::{decline_legend_action, ready_unit, reject_unchanged};

const WHEEL: CardId = 454;

fn position(actor: usize) -> (Game, u32, Target) {
    let mut game = test_support::position(actor, 0, 454001);
    let unit_id = ready_unit(&mut game, actor, 71);
    game.players[actor].hand = vec![WHEEL];
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    })
    .unwrap();
    let gear = game.players[actor].gear[0].id;
    assert!(game.players[actor].gear[0].ready);
    (
        game,
        gear,
        Target::Unit {
            player: actor,
            unit_id,
        },
    )
}

fn cast(game: &mut Game, card: CardId, targets: Vec<Target>) {
    let actor = game.priority();
    let hand_index = game.players[actor].hand.len();
    game.players[actor].hand.push(card);
    game.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
    while let Some(action) = decline_legend_action(game) {
        game.apply(action).unwrap();
    }
}

fn payment(game: &Game, pay: bool) -> Action {
    let Some(Decision::TriggerPayment {
        choice_id,
        card_id: WHEEL,
        resolving_chain_id: None,
        ..
    }) = game.decision
    else {
        panic!("expected pre-response Wheel payment: {:?}", game.decision);
    };
    Action::ResolveTriggerPayment { choice_id, pay }
}

fn settle(game: &mut Game) {
    for _ in 0..80 {
        match game.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => game
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            Some(Decision::TriggerPayment { .. }) => {
                let paid = payment(game, true);
                let action = if game.clone().apply(paid.clone()).is_ok() {
                    paid
                } else {
                    payment(game, false)
                };
                game.apply(action).unwrap();
            }
            Some(Decision::Legend { .. }) => game.apply(Action::Choose { accept: false }).unwrap(),
            None if !game.chain.is_empty() => game.apply(Action::Pass).unwrap(),
            None => return,
            other => panic!("unexpected decision: {other:?}"),
        }
    }
    panic!("Wheel chain failed to settle");
}

#[test]
fn irelia_wheel_pays_energy_and_source_before_responses_then_draws_separately() {
    for actor in 0..2 {
        let (mut game, gear_id, target) = position(actor);
        cast(&mut game, 8, vec![target]);
        let action = payment(&game, true);
        let energy = game.players[actor].energy;
        let before = format!("{game:?}");
        let plan = game.cost_plan(&action).unwrap();
        assert_eq!(plan.energy, 1);
        assert_eq!(
            plan.object_costs,
            vec![crate::play::ObjectCost::ExhaustGear {
                player: actor,
                gear_id
            }]
        );
        assert_eq!(format!("{game:?}"), before);
        reject_unchanged(&mut game, Action::Pass);
        game.apply(action).unwrap();
        assert_eq!(game.players[actor].energy, energy - 1);
        assert!(!game.players[actor].gear[0].ready);
        assert!(game.players[actor].hand.is_empty());
        assert_eq!(game.chain.len(), 2);
        assert!(game.chain[1].targets.is_empty());
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert_eq!(game.players[actor].hand, vec![71]);
        assert_eq!(
            game.chain.len(),
            1,
            "the choosing spell still awaits resolution"
        );
    }
}

#[test]
fn irelia_wheel_decline_and_unpayable_source_preserve_resources() {
    for fault in 0..4 {
        let (mut game, _, target) = position(0);
        cast(&mut game, 8, vec![target]);
        match fault {
            0 => {
                game.players[0].energy = 0;
                game.players[0].energy_pool = 0;
            }
            1 => game.players[0].gear[0].ready = false,
            2 => game.players[0].gear.clear(),
            3 => game.players[0].gear[0].id += 1000, // A new copy cannot pay the old copy's cost.
            _ => unreachable!(),
        }
        let paid = payment(&game, true);
        reject_unchanged(&mut game, paid);
        let energy = game.players[0].energy;
        game.apply(payment(&game, false)).unwrap();
        assert_eq!(game.players[0].energy, energy);
        assert_eq!(game.chain.len(), 1);
        settle(&mut game);
        assert!(game.players[0].hand.is_empty());
    }
}

#[test]
fn irelia_wheel_observes_friendly_ability_choices_but_not_enemy_units_or_unit_costs() {
    for actor in 0..2 {
        let (mut game, _, target) = position(actor);
        game.players[actor].gear.push(crate::gear::GearState {
            id: 454002,
            card_id: 230,
            lane: None,
            attached_to: None,
            ready: true,
            temporary: false,
            token: false,
        });
        game.apply(Action::ActivateAbility {
            source: crate::activated::AbilitySource::Gear {
                player: actor,
                gear_id: 454002,
            },
            ability_index: 0,
            targets: vec![target],
        })
        .unwrap();
        while let Some(action) = decline_legend_action(&game) {
            game.apply(action).unwrap();
        }
        assert!(matches!(
            game.decision,
            Some(Decision::TriggerPayment { card_id: WHEEL, .. })
        ));
        settle(&mut game);
        assert_eq!(game.players[actor].hand, vec![71]);

        let (mut game, _, _) = position(actor);
        let enemy = ready_unit(&mut game, 1 - actor, 71);
        cast(
            &mut game,
            7,
            vec![Target::Unit {
                player: 1 - actor,
                unit_id: enemy,
            }],
        );
        assert!(game.decision.is_none());
        assert_eq!(game.chain.len(), 1);

        let (mut game, _, target) = position(actor);
        let Target::Unit { unit_id, .. } = target else {
            unreachable!()
        };
        game.players[actor].hand = vec![452];
        game.apply(Action::WithPlayChoices {
            action: Box::new(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            }),
            choices: PlayChoices {
                bonus: true,
                exhaust_unit: Some(unit_id),
                ..Default::default()
            },
        })
        .unwrap();
        assert!(game.decision.is_none());
        assert!(game.players[actor].gear[0].ready);
        assert_eq!(game.chain.len(), 1);
    }
}

#[test]
fn irelia_wheel_repeated_choices_offer_each_copy_but_cannot_exhaust_twice() {
    let (mut game, _, target) = position(0);
    game.players[0].hand = vec![WHEEL];
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    })
    .unwrap();
    game.grant_play(
        0,
        ChainSource::Card { card_id: 256 },
        PlayGrantKind::RepeatNextSpell,
    );
    game.players[0].hand = vec![8];
    let base = Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![target],
    };
    let option_id = game.play_choices(&base).unwrap().repeats[0].id;
    game.apply(Action::WithPlayChoices {
        action: Box::new(base),
        choices: PlayChoices {
            repeats: vec![crate::play::RepeatSelection {
                option_id,
                choices: ExecutionChoices {
                    targets: vec![target],
                    ..Default::default()
                },
            }],
            ..Default::default()
        },
    })
    .unwrap();
    assert_eq!(
        game.pending_triggers.len(),
        6,
        "four Wheels and two Irelia occurrences"
    );
    assert_eq!(
        game.pending_triggers
            .iter()
            .filter(|t| t.option.card_id == WHEEL)
            .count(),
        4
    );
    settle(&mut game);
    assert_eq!(game.players[0].hand, vec![71, 71]);
    assert!(game.players[0].gear.iter().all(|g| !g.ready));
}

#[test]
fn irelia_wheel_paid_trigger_survives_source_departure_and_parent_counter() {
    for remove_source in [false, true] {
        let (mut game, _, target) = position(0);
        cast(&mut game, 8, vec![target]);
        let parent = game.chain[0].id;
        game.apply(payment(&game, true)).unwrap();
        if remove_source {
            // Authored post-finalization departure boundary, not a claim that
            // Factory Recall (Action speed) can respond to an existing chain.
            let gear = game.players[0].gear.remove(0);
            game.players[0].hand.push(gear.card_id);
        } else {
            game.apply(Action::Pass).unwrap();
            cast(&mut game, 6, vec![Target::Spell { chain_id: parent }]);
        }
        settle(&mut game);
        assert_eq!(
            game.players[0].hand.iter().filter(|&&id| id == 71).count(),
            1
        );
        if remove_source {
            assert!(game.players[0].gear.is_empty());
        } else {
            assert!(!game.players[0].gear[0].ready);
        }
    }
}

#[test]
fn irelia_wheel_failed_choosing_spell_rolls_back_all_triggers() {
    let (mut game, _, target) = position(0);
    game.players[0].hand = vec![8];
    game.players[0].energy = 0;
    game.players[0].energy_pool = 0;
    reject_unchanged(
        &mut game,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        },
    );
    assert!(game.pending_triggers.is_empty());
    assert!(game.decision.is_none());
}

#[test]
fn irelia_wheel_staged_payment_undo_and_refusal_preserve_source() {
    for complete in [false, true] {
        let (mut game, _, target) = position(0);
        cast(&mut game, 8, vec![target]);
        let energy = game.players[0].energy;
        game.apply(Action::BeginPayment {
            action: Box::new(payment(&game, true)),
        })
        .unwrap();
        assert!(game.players[0].gear[0].ready);
        game.apply(Action::Choose { accept: false }).unwrap();
        assert!(game.players[0].gear[0].ready);
        assert_eq!(game.players[0].energy, energy);
        game.apply(Action::BeginPayment {
            action: Box::new(payment(&game, true)),
        })
        .unwrap();
        game.apply(if complete {
            Action::CompletePayment
        } else {
            Action::RefusePayment
        })
        .unwrap();
        assert_eq!(game.players[0].gear[0].ready, !complete);
        assert_eq!(game.players[0].energy, energy - u16::from(complete));
        settle(&mut game);
        assert_eq!(game.players[0].hand.len(), usize::from(complete));
    }
}

#[test]
fn irelia_wheel_ai_payment_is_available_in_both_seats_at_every_level() {
    for actor in 0..2 {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let (mut game, _, target) = position(actor);
            game.difficulty = difficulty;
            cast(&mut game, 8, vec![target]);
            assert!(
                game.legal_actions()
                    .unwrap()
                    .contains(&payment(&game, true))
            );
            let action = game.choose_ai_action().unwrap();
            game.apply(action).unwrap();
            settle(&mut game);
            assert!(game.chain.is_empty());
        }
    }
}
