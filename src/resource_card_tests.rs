//! CR355.10.c, 356.2/4.f, 357.2 and 820: costs are not targets and Repeat
//! re-executes instructions, not payment. Keeper uses its official errata.
use super::*;
use test_support::{ready_unit, reject_unchanged};

const KEEPER: CardId = 451;
const MEDITATION: CardId = 452;
const LIFE: CardId = 453;

fn position(actor: usize) -> Game {
    test_support::position(actor, 0, 451001)
}

fn hand_action() -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    }
}

fn paid_action(card: CardId, unit: Option<u32>) -> Action {
    Action::WithPlayChoices {
        action: Box::new(hand_action()),
        choices: PlayChoices {
            bonus: card != LIFE,
            exhaust_unit: unit,
            ..Default::default()
        },
    }
}

fn settle(game: &mut Game) {
    for _ in 0..40 {
        match &game.decision {
            Some(Decision::OrderTriggers { options, .. }) => game
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            None if !game.chain.is_empty() => game.apply(Action::Pass).unwrap(),
            None => return,
            other => panic!("unexpected decision: {other:?}"),
        }
    }
    panic!("chain did not finish");
}

#[test]
fn irelia_resources_keeper_optional_payment_has_a_separate_draw_trigger() {
    for actor in 0..2 {
        for paid in [false, true] {
            let mut game = position(actor);
            game.players[actor].hand = vec![KEEPER];
            let energy = game.players[actor].energy;
            let calm = game.players[actor].power_pool[domain_index(Domain::Calm)];
            game.apply(if paid {
                paid_action(KEEPER, None)
            } else {
                hand_action()
            })
            .unwrap();
            assert_eq!(game.players[actor].units.last().unwrap().card_id, KEEPER);
            assert_eq!(game.players[actor].energy, energy - 2);
            assert_eq!(
                game.players[actor].power_pool[domain_index(Domain::Calm)],
                calm - u16::from(paid)
            );
            assert!(
                game.players[actor].hand.is_empty(),
                "draw must wait for responses"
            );
            assert_eq!(game.chain.len(), usize::from(paid));
            if paid {
                assert!(game.chain[0].ability.is_some());
            }
            settle(&mut game);
            assert_eq!(game.players[actor].hand, vec![71; usize::from(paid)]);
        }
    }
}

#[test]
fn irelia_resources_keeper_reduced_optional_cost_is_still_paid() {
    let mut game = position(0);
    game.players[0].hand = vec![KEEPER];
    game.players[0].power_pool = [0; 6];
    game.grant_play(
        0,
        ChainSource::Card { card_id: 258 },
        PlayGrantKind::DiscountNextCard {
            energy: 0,
            power: 1,
        },
    );
    let action = paid_action(KEEPER, None);
    assert!(
        game.play_choices(&action)
            .unwrap()
            .costs
            .power
            .iter()
            .all(|cost| cost.amount == 0)
    );
    game.apply(action).unwrap();
    assert_eq!(game.chain.len(), 1);
    settle(&mut game);
    assert_eq!(game.players[0].hand, vec![71]);
}

#[test]
fn irelia_resources_keeper_unpayable_bonus_rolls_back() {
    for actor in 0..2 {
        let mut game = position(actor);
        game.players[actor].hand = vec![KEEPER];
        game.players[actor].power_pool = [0; 6];
        reject_unchanged(&mut game, paid_action(KEEPER, None));
        game.apply(hand_action()).unwrap();
        assert!(game.chain.is_empty());
    }
}

#[test]
fn irelia_resources_draw_and_channel_follow_response_window_for_both_seats() {
    for actor in 0..2 {
        for (card, paid, draws, channels, energy) in [
            (MEDITATION, false, 1, 0, 2),
            (MEDITATION, true, 2, 0, 2),
            (LIFE, true, 1, 1, 1),
        ] {
            let mut game = position(actor);
            let unit = ready_unit(&mut game, actor, 71);
            game.players[actor].hand = vec![card];
            game.players[actor].rune_deck = vec![Domain::Calm; 5];
            let action = if paid {
                paid_action(card, Some(unit))
            } else {
                hand_action()
            };
            let before = format!("{game:?}");
            let view = game.play_choices(&action).unwrap();
            assert_eq!(view.unit_cost.unwrap().candidates, vec![unit]);
            assert_eq!(view.costs.object_costs.len(), usize::from(paid));
            assert_eq!(format!("{game:?}"), before, "preview must be read-only");
            game.apply(action).unwrap();
            assert_eq!(game.players[actor].energy, 20 - energy);
            assert_eq!(game.players[actor].units[0].ready, !paid);
            assert!(game.players[actor].hand.is_empty());
            assert_eq!(game.players[actor].total_runes(), 0);
            assert!(
                game.chain[0].targets.is_empty(),
                "exhaustion cost is not a target"
            );
            settle(&mut game);
            assert_eq!(game.players[actor].hand, vec![71; draws]);
            assert_eq!(game.players[actor].total_runes(), channels);
            assert_eq!(game.players[actor].ready_runes(Domain::Calm), 0);
            assert_eq!(game.players[actor].rune_deck.len(), 5 - channels as usize);
            assert_eq!(game.players[actor].discard, vec![card]);
        }
    }
}

#[test]
fn irelia_resources_invalid_or_missing_unit_costs_are_atomic() {
    for actor in 0..2 {
        for card in [MEDITATION, LIFE] {
            let mut game = position(actor);
            game.players[actor].hand = vec![card];
            let own = ready_unit(&mut game, actor, 71);
            let enemy = ready_unit(&mut game, 1 - actor, 71);
            for unit in [None, Some(enemy), Some(u32::MAX)] {
                reject_unchanged(&mut game, paid_action(card, unit));
            }
            game.players[actor].units[0].ready = false;
            reject_unchanged(&mut game, paid_action(card, Some(own)));
            game.players[actor].units[0].ready = true;
            game.players[actor].energy_pool = 0;
            game.players[actor].energy = 0;
            reject_unchanged(&mut game, paid_action(card, Some(own)));
        }
    }
    let mut game = position(0);
    let unit = ready_unit(&mut game, 0, 71);
    game.players[0].hand = vec![LIFE];
    reject_unchanged(&mut game, hand_action());
    for card in [MEDITATION, 45] {
        game.players[0].hand = vec![card];
        reject_unchanged(
            &mut game,
            Action::WithPlayChoices {
                action: Box::new(hand_action()),
                choices: PlayChoices {
                    exhaust_unit: Some(unit),
                    ..Default::default()
                },
            },
        );
    }
}

#[test]
fn irelia_resources_unit_cost_never_triggers_irelia_or_jae() {
    for actor in 0..2 {
        for card in [MEDITATION, LIFE] {
            let mut game = position(actor);
            game.legend[actor].ready = true;
            let jae = ready_unit(&mut game, actor, 450);
            game.players[actor].hand = vec![card];
            game.apply(paid_action(card, Some(jae))).unwrap();
            assert!(game.decision.is_none());
            assert!(game.legend[actor].ready);
            assert_eq!(game.chain.len(), 1);
            assert_eq!(game.chain[0].card_id, card);
            settle(&mut game);
            assert!(!game.players[actor].units[0].ready);
            assert_eq!(
                game.players[actor].hand.len(),
                if card == MEDITATION { 2 } else { 1 }
            );
        }
    }
}

#[test]
fn irelia_resources_exhaust_cost_does_trigger_innovator_before_draw() {
    for actor in 0..2 {
        let mut game = position(actor);
        let innovator = ready_unit(&mut game, actor, 429);
        game.players[actor].hand = vec![MEDITATION];
        game.apply(paid_action(MEDITATION, Some(innovator)))
            .unwrap();
        assert!(!game.players[actor].units[0].ready);
        let Some(Decision::TriggerMode {
            choice_id, player, ..
        }) = game.decision
        else {
            panic!("exhaustion did not trigger Innovator")
        };
        assert_eq!(player, actor);
        assert!(game.players[actor].hand.is_empty());
        game.apply(Action::ChooseTriggerMode { choice_id, mode: 0 })
            .unwrap();
        settle(&mut game);
        assert_eq!(game.players[actor].gear.len(), 1);
        assert_eq!(game.players[actor].hand, vec![71, 71]);
    }
}

#[test]
fn irelia_resources_repeat_pays_unit_cost_once_and_preserves_paid_choice() {
    for actor in 0..2 {
        for card in [MEDITATION, LIFE] {
            let mut game = position(actor);
            let syndra = ready_unit(&mut game, actor, 266); // Syndra grants Repeat at her showdown.
            let mover = ready_unit(&mut game, actor, 71);
            let enemy = ready_unit(&mut game, 1 - actor, 71);
            game.unit_mut(Target::Unit {
                player: 1 - actor,
                unit_id: enemy,
            })
            .unwrap()
            .lane = Some(0);
            game.battlefield_owner[0] = Some(1 - actor);
            game.apply(Action::MoveGroup {
                unit_ids: vec![mover, syndra],
                lane: Some(0),
            })
            .unwrap();
            let cost_unit = ready_unit(&mut game, actor, 71);
            game.players[actor].hand = vec![card];
            game.players[actor].rune_deck = vec![Domain::Calm; 5];
            let option_id = game.play_choices(&hand_action()).unwrap().repeats[0].id;
            let action = Action::WithPlayChoices {
                action: Box::new(hand_action()),
                choices: PlayChoices {
                    bonus: card == MEDITATION,
                    exhaust_unit: Some(cost_unit),
                    repeats: vec![crate::play::RepeatSelection {
                        option_id,
                        choices: ExecutionChoices::default(),
                    }],
                    ..Default::default()
                },
            };
            game.apply(action).unwrap();
            assert_eq!(game.chain[0].executions.len(), 2);
            settle(&mut game);
            assert_eq!(
                game.players[actor].hand,
                vec![71; if card == MEDITATION { 4 } else { 2 }]
            );
            assert_eq!(
                game.players[actor].total_runes(),
                if card == LIFE { 2 } else { 0 }
            );
            assert_eq!(game.players[actor].discard, vec![card]);
            assert!(
                !game
                    .unit_ref(Target::Unit {
                        player: actor,
                        unit_id: cost_unit
                    })
                    .unwrap()
                    .ready
            );
        }
    }
}

#[test]
fn irelia_resources_countering_does_not_refund_paid_unit_or_resources() {
    for actor in 0..2 {
        for card in [MEDITATION, LIFE] {
            let mut game = position(actor);
            let unit = ready_unit(&mut game, actor, 71);
            game.players[actor].hand = vec![card];
            game.apply(paid_action(card, Some(unit))).unwrap();
            let chain_id = game.chain[0].id;
            let energy = game.players[actor].energy;
            if game.priority() == actor {
                game.apply(Action::Pass).unwrap();
            }
            game.players[1 - actor].hand = vec![6];
            game.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![Target::Spell { chain_id }],
            })
            .unwrap();
            settle(&mut game);
            assert!(game.players[actor].hand.is_empty());
            assert_eq!(game.players[actor].total_runes(), 0);
            assert_eq!(game.players[actor].energy, energy);
            assert!(!game.players[actor].units[0].ready);
            assert_eq!(game.players[actor].discard, vec![card]);
        }
    }
}

#[test]
fn irelia_resources_staged_payment_preserves_cost_selection_and_decline() {
    for card in [MEDITATION, LIFE] {
        for complete in [false, true] {
            let mut game = position(0);
            let unit = ready_unit(&mut game, 0, 71);
            game.players[0].hand = vec![card];
            game.apply(Action::BeginPayment {
                action: Box::new(paid_action(card, Some(unit))),
            })
            .unwrap();
            assert!(game.players[0].units[0].ready);
            assert!(game.players[0].hand.is_empty());
            if complete {
                game.apply(Action::CompletePayment).unwrap();
                assert!(!game.players[0].units[0].ready);
                settle(&mut game);
                assert_eq!(
                    game.players[0].hand.len(),
                    if card == MEDITATION { 2 } else { 1 }
                );
            } else {
                game.apply(Action::RefusePayment).unwrap();
                assert!(game.players[0].units[0].ready);
                assert_eq!(game.players[0].hand, vec![card]);
                assert_eq!(game.players[0].energy, 20);
                assert!(game.chain.is_empty());
            }
        }
    }
}

#[test]
fn irelia_resources_ai_can_pay_required_cost_from_either_seat() {
    for actor in 0..2 {
        for level in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let mut game = position(actor);
            game.difficulty = level;
            let unit = ready_unit(&mut game, actor, 71);
            game.players[actor].hand = vec![LIFE];
            let legal = game.legal_actions().unwrap();
            assert!(legal.iter().any(|action| matches!(action, Action::WithPlayChoices { choices, .. } if choices.exhaust_unit == Some(unit))), "required-cost play missing at {level:?}: {legal:?}");
            let action = game.choose_ai_action().unwrap();
            game.apply(action).unwrap();
        }
    }
}

#[test]
fn irelia_resources_resource_ability_cannot_spend_the_selected_unit_twice() {
    for actor in 0..2 {
        for card in [MEDITATION, LIFE] {
            let mut game = position(actor);
            let unit = ready_unit(&mut game, actor, 236); // Dragonsoul Sage: exhaust, Add 1 Energy.
            game.players[actor].hand = vec![card];
            game.apply(Action::BeginPayment {
                action: Box::new(paid_action(card, Some(unit))),
            })
            .unwrap();
            game.apply(Action::ActivateAbility {
                source: crate::activated::AbilitySource::Unit {
                    player: actor,
                    unit_id: unit,
                },
                ability_index: 0,
                targets: vec![],
            })
            .unwrap();
            assert!(!game.players[actor].units[0].ready);
            assert_eq!(game.players[actor].energy, 21);
            reject_unchanged(&mut game, Action::CompletePayment);
            game.apply(Action::Choose { accept: false }).unwrap();
            assert!(game.players[actor].units[0].ready);
            assert_eq!(game.players[actor].energy, 20);
            assert_eq!(game.players[actor].hand, vec![card]);
        }
    }
}

#[test]
fn irelia_resources_unit_tokens_can_pay_the_exhaustion_cost() {
    for card in [MEDITATION, LIFE] {
        let mut game = position(0);
        let token = game.create_unit(0, 448, true, 2); // Sand Soldier created by Guards!
        game.players[0].units[0].ready = true;
        game.players[0].hand = vec![card];
        assert_eq!(
            game.play_choices(&hand_action())
                .unwrap()
                .unit_cost
                .unwrap()
                .candidates,
            vec![token]
        );
        game.apply(paid_action(card, Some(token))).unwrap();
        settle(&mut game);
        assert!(game.players[0].units[0].token);
        assert!(!game.players[0].units[0].ready);
        assert_eq!(
            game.players[0].hand.len(),
            if card == MEDITATION { 2 } else { 1 }
        );
    }
}
