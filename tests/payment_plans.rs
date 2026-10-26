//! Frozen-cost finalization and rollback through the public action interface.
use local_rift::game::{Action, Decision, Difficulty, Game, Target, Unit};
use local_rift::gear::GearState;
use local_rift::payment::PowerPayment;

fn position(card: usize, energy: u16) -> Game {
    let mut game = Game::new_for_agents(3, 3, 54676, Difficulty::Tactical);
    game.start_match().unwrap();
    game.legend[0].ready = false;
    game.legend[1].ready = false;
    game.battlefields = [0, 0];
    game.battlefield_owner = [None; 2];
    for state in &mut game.players {
        state.hand.clear();
        state.units.clear();
        state.gear.clear();
        state.hidden.clear();
        state.discard.clear();
        state.champion = None;
        state.runes = [0; 6];
        state.exhausted_runes = [0; 6];
        state.energy_pool = 0;
        state.energy = 0;
        state.power_pool = [0; 6];
        state.any_power = 0;
    }
    game.players[0].hand.push(card);
    game.players[0].energy_pool = energy;
    game.players[0].energy = energy;
    game.players[1].units.push(Unit {
        id: 700,
        card_id: 70,
        lane: Some(0),
        ready: true,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    });
    game.battlefield_owner[0] = Some(1);
    game
}
fn gold(game: &mut Game, empowered: bool) {
    game.players[0].gear.push(GearState {
        id: 900,
        card_id: 13,
        lane: None,
        attached_to: None,
        ready: true,
        temporary: false,
        token: true,
    });
    if empowered {
        game.empowered_units.push(900);
    }
}
fn target() -> Target {
    Target::Unit {
        player: 1,
        unit_id: 700,
    }
}
fn play(targets: Vec<Target>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets,
    }
}
fn begin(game: &mut Game, action: Action) {
    game.apply(Action::BeginPayment {
        action: Box::new(action),
    })
    .unwrap();
}
fn complete_with_any(game: &mut Game, amount: u16) -> Result<(), String> {
    game.apply(Action::WithPowerPayment {
        action: Box::new(Action::CompletePayment),
        payment: PowerPayment {
            any: amount,
            ..PowerPayment::default()
        },
    })
}

#[test]
fn spending_last_empowered_gold_during_payment_keeps_the_locked_discount() {
    let mut game = position(227, 1); // Shock Blast: 3E1P, discounted by2 when Empowered.
    gold(&mut game, true);
    let mut before_finalization = game.clone();
    before_finalization
        .apply(Action::ActivateGear { gear_id: 900 })
        .unwrap();
    assert!(
        before_finalization.apply(play(vec![target()])).is_err(),
        "using Gold before cost determination correctly loses the discount"
    );
    begin(&mut game, play(vec![target()]));
    let plan = game.payment_plan_view(0).unwrap();
    assert_eq!(plan.energy, 1);
    assert_eq!(plan.power.iter().map(|c| c.amount).sum::<u16>(), 1);
    assert!(plan.action.is_some());
    assert_eq!(plan.pending_card, Some(227));
    assert_eq!(game.payment_plan_view(1).unwrap().pending_card, Some(227));
    assert!(
        game.players[0].hand.is_empty(),
        "card leaves hand before payment"
    );
    assert!(game.payment_plan_view(1).unwrap().action.is_none());
    assert!(matches!(
        game.decision,
        Some(Decision::Payment { player: 0 })
    ));
    game.apply(Action::ActivateGear { gear_id: 900 }).unwrap();
    assert_eq!(game.priority(), 0);
    assert!(game.players[0].gear.is_empty());
    assert_eq!(game.payment_plan_view(0).unwrap().energy, 1);
    let quote = game.quote_power_payment(&Action::CompletePayment).unwrap();
    assert_eq!(quote.suggested.any, 1);
    complete_with_any(&mut game, 1).unwrap();
    assert_eq!(game.players[0].energy, 0);
    assert_eq!(game.players[0].any_power, 0);
    assert!(game.players[0].hand.is_empty());
    assert_eq!(game.chain.last().unwrap().card_id, 227);
    assert!(game.payment_plan_view(0).is_none());
    assert_eq!(game.priority(), 0);
}

#[test]
fn consumed_chosen_gold_cannot_be_retargeted_and_failed_payment_rolls_back() {
    let mut game = position(41, 2); // Salvage may choose up to one gear.
    gold(&mut game, false);
    let before = format!("{game:?}");
    begin(
        &mut game,
        play(vec![Target::Gear {
            player: 0,
            gear_id: 900,
        }]),
    );
    game.apply(Action::ActivateGear { gear_id: 900 }).unwrap();
    let pending = format!("{game:?}");
    assert!(game.apply(Action::CompletePayment).is_err());
    assert_eq!(format!("{game:?}"), pending, "failed completion is atomic");
    assert!(game.can_cancel_payment().unwrap());
    game.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(
        format!("{game:?}"),
        before,
        "358.5 restores Gold, resources, RNG, queued events, and original intent state"
    );
}

#[test]
fn failed_attempt_restores_original_mandatory_choice_and_in_payment_sources() {
    let mut game = position(227, 0);
    game.decision = Some(Decision::CounterTax {
        player: 0,
        chain_id: 1234,
        cost: 2,
    });
    game.players[0].gear.push(GearState {
        id: 901,
        card_id: 130,
        lane: None,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    let before = format!("{game:?}");
    begin(&mut game, Action::Choose { accept: true });
    game.apply(Action::ActivateGear { gear_id: 901 }).unwrap();
    assert_eq!(game.players[0].energy, 1);
    assert!(game.apply(Action::CompletePayment).is_err());
    assert!(game.can_cancel_payment().unwrap());
    game.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(format!("{game:?}"), before);
    assert!(matches!(game.decision, Some(Decision::CounterTax { .. })));
}

#[test]
fn malformed_begin_and_exact_budget_errors_preserve_the_whole_transaction() {
    let mut game = position(227, 1);
    gold(&mut game, true);
    let before = format!("{game:?}");
    assert!(
        game.apply(Action::BeginPayment {
            action: Box::new(play(vec![]))
        })
        .is_err()
    );
    assert_eq!(format!("{game:?}"), before);
    begin(&mut game, play(vec![target()]));
    let during = format!("{game:?}");
    assert!(
        game.apply(Action::BeginPayment {
            action: Box::new(play(vec![target()]))
        })
        .is_err()
    );
    assert!(
        game.apply(Action::SetOptionalDiscount { power: 0 })
            .is_err()
    );
    assert_eq!(format!("{game:?}"), during);
    game.apply(Action::ActivateGear { gear_id: 900 }).unwrap();
    game.players[0].any_power = 2;
    let funded = format!("{game:?}");
    assert!(complete_with_any(&mut game, 2).is_err());
    assert_eq!(format!("{game:?}"), funded);
    assert!(
        !game.can_cancel_payment().unwrap(),
        "a fully payable selected action must finish"
    );
    complete_with_any(&mut game, 1).unwrap();
    assert_eq!(game.players[0].any_power, 1);
}

#[test]
fn convenience_play_selects_its_targets_once_before_resource_actions() {
    let mut game = position(227, 1);
    gold(&mut game, true);
    begin(
        &mut game,
        Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: None,
        },
    );
    assert_eq!(
        game.payment_plan_view(0).unwrap().action,
        Some(play(vec![target()]))
    );
    game.apply(Action::ActivateGear { gear_id: 900 }).unwrap();
    game.apply(Action::CompletePayment).unwrap();
    assert_eq!(game.chain.last().unwrap().targets, vec![target()]);
}

fn physical_cards(game: &Game, player: usize) -> std::collections::BTreeMap<usize, usize> {
    let state = &game.players[player];
    let mut cards = Vec::new();
    cards.extend(state.deck.iter().copied());
    cards.extend(state.hand.iter().copied());
    cards.extend(state.discard.iter().copied());
    cards.extend(state.banish.iter().copied());
    cards.extend(state.champion);
    cards.extend(state.units.iter().filter(|u| !u.token).map(|u| u.card_id));
    cards.extend(state.gear.iter().filter(|g| !g.token).map(|g| g.card_id));
    cards.extend(state.hidden.iter().map(|c| c.card_id));
    cards.extend(
        game.chain
            .iter()
            .filter(|c| c.player == player && c.ability.is_none())
            .map(|c| c.card_id),
    );
    cards.extend(
        game.pending_cards()
            .iter()
            .filter(|c| c.player == player)
            .map(|c| c.card_id),
    );
    if let Some(plan) = game.payment_plan_view(player)
        && plan.player == player
    {
        cards.extend(plan.pending_card);
    }
    let mut counts = std::collections::BTreeMap::new();
    for id in cards {
        *counts.entry(id).or_insert(0) += 1;
    }
    counts
}

#[test]
fn hand_champion_hidden_and_trash_payments_have_one_public_pending_card() {
    let mut hand = position(227, 3);
    hand.players[0].any_power = 1;
    let before = physical_cards(&hand, 0);
    begin(&mut hand, play(vec![target()]));
    assert!(hand.players[0].hand.is_empty());
    assert_eq!(physical_cards(&hand, 0), before);
    hand.apply(Action::CompletePayment).unwrap();
    assert_eq!(physical_cards(&hand, 0), before);

    let mut champion = position(227, 12);
    champion.players[0].champion = Some(49);
    champion.players[0].any_power = 4;
    let before = physical_cards(&champion, 0);
    begin(
        &mut champion,
        Action::PlayChampion {
            lane: None,
            target: None,
        },
    );
    assert_eq!(champion.players[0].champion, None);
    assert_eq!(
        champion.payment_plan_view(1).unwrap().pending_card,
        Some(49)
    );
    assert_eq!(physical_cards(&champion, 0), before);
    champion.apply(Action::CompletePayment).unwrap();
    assert_eq!(physical_cards(&champion, 0), before);

    let mut hidden = position(227, 0);
    hidden.battlefield_owner[1] = Some(0);
    hidden.players[0]
        .hidden
        .push(local_rift::hidden::HiddenCard {
            id: 1234,
            card_id: 21,
            lane: 1,
            hidden_turn: 0,
            revealed: false,
        });
    let before = physical_cards(&hidden, 0);
    begin(
        &mut hidden,
        Action::PlayHidden {
            hidden_id: 1234,
            targets: vec![],
        },
    );
    assert!(hidden.players[0].hidden.is_empty());
    assert_eq!(hidden.payment_plan_view(1).unwrap().pending_card, Some(21));
    assert_eq!(physical_cards(&hidden, 0), before);
    hidden.apply(Action::CompletePayment).unwrap();
    assert_eq!(physical_cards(&hidden, 0), before);

    let mut trash = position(227, 2);
    trash.players[0].discard = vec![69, 134, 70];
    let before = physical_cards(&trash, 0);
    begin(
        &mut trash,
        Action::PlayFromTrashOptions {
            trash_index: 1,
            lane: None,
            targets: vec![],
            repeat_targets: vec![],
            repeat: false,
            accelerate: false,
            bonus: false,
            flow: Some(local_rift::game::FlowSource::Printed),
        },
    );
    assert_eq!(trash.players[0].discard, vec![69, 70]);
    assert_eq!(trash.payment_plan_view(1).unwrap().pending_card, Some(134));
    assert_eq!(physical_cards(&trash, 0), before);
    trash.apply(Action::CompletePayment).unwrap();
    assert_eq!(physical_cards(&trash, 0), before);
    assert!(trash.chain.last().unwrap().banish_after);
}

#[test]
fn trash_variable_discount_is_fixed_after_the_card_leaves_its_source() {
    let mut game = position(227, 0);
    game.players[0].discard = vec![69, 40, 70];
    game.decision = Some(Decision::LastRites {
        player: 0,
        gear_id: 901,
    });
    let before = format!("{game:?}");
    begin(
        &mut game,
        Action::PlayFromTrashWithTargets {
            trash_index: 1,
            lane: None,
            targets: vec![],
        },
    );
    assert_eq!(game.players[0].discard, vec![69, 70]);
    assert_eq!(
        game.payment_plan_view(0).unwrap().energy,
        u32::from(game.catalog[40].cost - 2),
        "Rhasa does not discount itself in trash"
    );
    assert!(game.can_cancel_payment().unwrap());
    game.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(format!("{game:?}"), before);
}

#[test]
fn irrelevant_decision_cannot_manufacture_a_resource_payment_window() {
    let mut game = position(227, 0);
    game.decision = Some(Decision::Predict {
        player: 0,
        card_id: 69,
    });
    let before = format!("{game:?}");
    assert!(
        game.apply(Action::BeginPayment {
            action: Box::new(Action::Choose { accept: true })
        })
        .is_err()
    );
    assert_eq!(format!("{game:?}"), before);
}

#[test]
fn rollback_can_be_quoted_or_wrapped_with_exactly_zero_power() {
    let mut game = position(227, 0);
    gold(&mut game, true);
    let original = format!("{game:?}");
    begin(&mut game, play(vec![target()]));
    game.apply(Action::ActivateGear { gear_id: 900 }).unwrap();
    let rollback = Action::Choose { accept: false };
    let quote = game.quote_power_payment(&rollback).unwrap();
    assert_eq!(quote.suggested.total(), 0);
    assert!(quote.costs.is_empty());
    let pending = format!("{game:?}");
    assert!(
        game.apply(Action::WithPowerPayment {
            action: Box::new(rollback.clone()),
            payment: PowerPayment {
                any: 1,
                ..PowerPayment::default()
            },
        })
        .is_err()
    );
    assert_eq!(format!("{game:?}"), pending);
    game.apply(Action::WithPowerPayment {
        action: Box::new(rollback),
        payment: PowerPayment::default(),
    })
    .unwrap();
    assert_eq!(format!("{game:?}"), original);
}
