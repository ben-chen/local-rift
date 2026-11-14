//! Independently expected interactions: Core 383.3.b/d, 355.10.c and 416.1.
//! Other engines suggest cases; printed rules determine these expectations.
use super::*;
use test_support::{ready_unit, reject_unchanged};

const JAE: CardId = 450;
const WHEEL: CardId = 454;
const ORDERS: [[CardId; 3]; 6] = [
    [0, JAE, WHEEL],
    [0, WHEEL, JAE],
    [JAE, 0, WHEEL],
    [JAE, WHEEL, 0],
    [WHEEL, 0, JAE],
    [WHEEL, JAE, 0],
];

fn position(actor: usize, ready: bool) -> (Game, Target) {
    let mut game = test_support::position(actor, 0, 454200);
    let unit_id = ready_unit(&mut game, actor, JAE);
    let target = Target::Unit {
        player: actor,
        unit_id,
    };
    game.unit_mut(target).unwrap().ready = ready;
    game.legend[actor].ready = true;
    game.players[actor].hand = vec![WHEEL, 8];
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    })
    .unwrap();
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![target],
    })
    .unwrap();
    (game, target)
}

/// Select each named source through the public ordering action. The last
/// remaining occurrence may finalize automatically without an ordering prompt.
fn finalize(game: &mut Game, order: [CardId; 3], legend: bool, wheel: bool) {
    for card in order {
        if let Some(Decision::OrderTriggers { options, .. }) = &game.decision {
            let trigger_id = options
                .iter()
                .find(|o| o.card_id == card)
                .unwrap_or_else(|| panic!("missing source {card}: {options:?}"))
                .id;
            game.apply(Action::OrderTrigger { trigger_id }).unwrap();
        }
        match card {
            0 => {
                assert!(matches!(game.decision, Some(Decision::Legend { .. })));
                game.apply(Action::Choose { accept: legend }).unwrap();
            }
            WHEEL => {
                let Some(Decision::TriggerPayment {
                    choice_id,
                    card_id: WHEEL,
                    resolving_chain_id: None,
                    ..
                }) = game.decision
                else {
                    panic!("missing pre-response Wheel cost: {:?}", game.decision);
                };
                game.apply(Action::ResolveTriggerPayment {
                    choice_id,
                    pay: wheel,
                })
                .unwrap();
            }
            JAE => assert!(game.chain.iter().any(|i| i.card_id == JAE)),
            _ => unreachable!(),
        }
    }
    assert!(game.decision.is_none());
    assert!(game.pending_triggers.is_empty());
}

fn settle(game: &mut Game) {
    for _ in 0..20 {
        assert!(
            game.decision.is_none(),
            "unexpected new choice: {:?}",
            game.decision
        );
        if game.chain.is_empty() {
            assert!(game.pending_triggers.is_empty());
            return;
        }
        game.apply(Action::Pass).unwrap();
    }
    panic!("trigger chain failed to settle");
}

#[test]
fn irelia_trigger_audit_all_orders_cost_choices_and_parent_counters() {
    let mut scenarios = 0;
    for actor in 0..2 {
        for ready in [false, true] {
            for order in ORDERS {
                for legend in [false, true] {
                    for wheel in [false, true] {
                        for counter in [false, true] {
                            let (mut game, target) = position(actor, ready);
                            let energy = game.players[actor].energy;
                            let power = game.available_power(actor, None);
                            reject_unchanged(&mut game, Action::Pass);
                            finalize(&mut game, order, legend, wheel);
                            let expected: Vec<_> = std::iter::once(8)
                                .chain(order.into_iter().filter(|&c| {
                                    c == JAE || (c == 0 && legend) || (c == WHEEL && wheel)
                                }))
                                .collect();
                            assert_eq!(
                                game.chain.iter().map(|i| i.card_id).collect::<Vec<_>>(),
                                expected
                            );
                            assert!(
                                game.chain[1..].iter().all(|i| i.targets.is_empty()),
                                "referenced units and costs are not new choices"
                            );
                            assert_eq!(
                                game.unit_ref(target).unwrap().ready,
                                ready,
                                "ready effect waits for resolution"
                            );
                            assert_eq!(game.legend[actor].ready, !legend);
                            assert_eq!(game.players[actor].gear[0].ready, !wheel);
                            assert_eq!(game.players[actor].energy, energy - u16::from(wheel));
                            assert_eq!(
                                game.available_power(actor, None),
                                power - u16::from(legend)
                            );
                            assert!(
                                game.players[actor].hand.is_empty(),
                                "neither draw happens during payment"
                            );
                            if counter {
                                let parent = game.chain[0].id;
                                game.apply(Action::Pass).unwrap();
                                assert_eq!(game.priority(), 1 - actor);
                                game.players[1 - actor].hand = vec![6];
                                game.apply(Action::PlayCardWithTargets {
                                    hand_index: 0,
                                    lane: None,
                                    targets: vec![Target::Spell { chain_id: parent }],
                                })
                                .unwrap();
                            }
                            settle(&mut game);
                            assert_eq!(game.players[actor].hand, vec![71; 1 + usize::from(wheel)]);
                            assert_eq!(game.unit_ref(target).unwrap().ready, ready || legend);
                            assert!(game.players[1 - actor].hand.is_empty());
                            assert_eq!(
                                game.players[actor].energy,
                                energy - u16::from(wheel),
                                "no refund when parent is countered"
                            );
                            assert_eq!(
                                game.available_power(actor, None),
                                power - u16::from(legend)
                            );
                            scenarios += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(scenarios, 192);
}

#[test]
fn irelia_trigger_audit_initially_ready_unit_can_be_exhausted_in_response() {
    for actor in 0..2 {
        let (mut game, target) = position(actor, true);
        finalize(&mut game, [0, JAE, WHEEL], true, true);
        let Target::Unit { unit_id, .. } = target else {
            unreachable!()
        };
        assert_eq!(game.priority(), actor);
        game.players[actor].hand = vec![452]; // Meditation: exhaust as a cost, then draw two.
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
        assert!(!game.unit_ref(target).unwrap().ready);
        assert_eq!(game.chain.len(), 5);
        assert!(
            game.decision.is_none(),
            "paying an exhaust cost does not choose Jae again"
        );
        settle(&mut game);
        assert!(
            game.unit_ref(target).unwrap().ready,
            "Irelia readies the same physical unit after the response"
        );
        assert!(!game.legend[actor].ready);
        assert!(!game.players[actor].gear[0].ready);
        assert_eq!(game.players[actor].hand, vec![71; 4]);
    }
}
