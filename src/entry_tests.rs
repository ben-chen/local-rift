//! Expectations from the reviewed printed faces and CR356, 370, 416 and 420.4.
use super::*;
use crate::game::test_support::{decline_legend_action, position, ready_unit, reject_unchanged};

const GUARDIAN: CardId = 466;
const BOAR: CardId = 467;
const DRAW: CardId = 133;

fn hand() -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    }
}

fn chosen(bonus: bool, entry_ready: bool) -> Action {
    Action::WithPlayChoices {
        action: Box::new(hand()),
        choices: PlayChoices {
            bonus,
            entry_ready,
            ..Default::default()
        },
    }
}

/// Only responses and explicitly declined legend triggers may be settled here.
/// Any other choice is unexpected and must be supplied by the scenario itself.
fn settle(g: &mut Game) {
    for _ in 0..80 {
        if g.decision.is_none() && g.chain.is_empty() && g.showdown.is_none() {
            return;
        }
        let action = if let Some(action) = decline_legend_action(g) {
            action
        } else if let Some(Decision::OrderTriggers { options, .. }) = &g.decision {
            Action::OrderTrigger {
                trigger_id: options[0].id,
            }
        } else {
            assert!(g.decision.is_none(), "unexpected choice: {:?}", g.decision);
            Action::Pass
        };
        g.apply(action).unwrap();
    }
    panic!("resolution failed to finish");
}

fn play_spell(g: &mut Game, actor: usize, countered: bool) {
    g.players[actor].hand.insert(0, DRAW);
    g.apply(hand()).unwrap();
    if countered {
        let chain_id = g.chain.last().unwrap().id;
        g.players[1 - actor].hand = vec![6];
        if g.priority() == actor {
            g.apply(Action::Pass).unwrap();
        }
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Spell { chain_id }],
        })
        .unwrap();
    }
    settle(g);
    assert_eq!(g.priority(), actor);
}

#[test]
fn guardian_requires_own_finalized_spell_and_rejects_unavailable_choice_atomically() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 466001);
        g.players[actor].hand = vec![GUARDIAN];
        assert!(g.play_choices(&hand()).unwrap().bonus.is_none());
        g.play_history.finalized_spells[1 - actor].push(900);
        assert!(
            g.play_choices(&hand()).unwrap().bonus.is_none(),
            "an opponent's spell does not qualify"
        );
        reject_unchanged(&mut g, chosen(true, false));
        reject_unchanged(&mut g, chosen(false, true));
        g.apply(hand()).unwrap();
        assert!(!g.players[actor].units.last().unwrap().ready);
    }
}

#[test]
fn guardian_additional_chaos_cost_is_optional_and_countered_spells_still_qualify() {
    for actor in 0..2 {
        for countered in [false, true] {
            for accept in [false, true] {
                let mut g = position(actor, 0, 466002);
                g.players[actor].hand = vec![GUARDIAN];
                play_spell(&mut g, actor, countered);
                assert!(g.played_spell_this_turn(actor));
                let view = g.play_choices(&hand()).unwrap();
                let cost = &view.bonus.unwrap().cost;
                assert_eq!(
                    (cost.energy, cost.power, &cost.domains),
                    (0, 1, &vec![Domain::Chaos])
                );
                assert!(view.entry_ready.is_none());
                let energy = g.players[actor].energy;
                let chaos = g.players[actor].power_pool[domain_index(Domain::Chaos)];
                g.apply(chosen(accept, false)).unwrap();
                let unit = g.players[actor].units.last().unwrap();
                assert_eq!(unit.card_id, GUARDIAN);
                assert_eq!(unit.ready, accept);
                assert!(!g.movement_locked(actor, unit.id));
                assert_eq!(g.players[actor].energy, energy - 4);
                assert_eq!(
                    g.players[actor].power_pool[domain_index(Domain::Chaos)],
                    chaos - u16::from(accept)
                );
            }
        }
    }
}

#[test]
fn guardian_zero_cost_additional_payment_retains_ready_entry() {
    let mut g = position(0, 0, 466003);
    g.players[0].hand = vec![GUARDIAN];
    play_spell(&mut g, 0, false);
    ready_unit(&mut g, 0, 35); // Ezreal, Prodigy discounts optional costs.
    let action = chosen(true, false);
    let plan = g.cost_plan(&action).unwrap();
    let discount = plan
        .modifiers
        .iter()
        .find(|m| m.choose_energy_or_power)
        .unwrap()
        .id;
    let action = Action::WithPlayChoices {
        action: Box::new(hand()),
        choices: PlayChoices {
            bonus: true,
            cost_order: vec![CostSelection {
                modifier_id: discount,
                use_power: true,
                power_allocations: vec![],
            }],
            ..Default::default()
        },
    };
    let chaos = g.players[0].power_pool[domain_index(Domain::Chaos)];
    g.apply(action).unwrap();
    assert!(g.players[0].units.last().unwrap().ready);
    assert_eq!(g.players[0].power_pool[domain_index(Domain::Chaos)], chaos);
}

#[test]
fn boar_entry_choice_is_free_and_only_acceptance_prevents_movement() {
    for actor in 0..2 {
        for accept in [false, true] {
            let mut g = position(actor, 0, 467001);
            g.players[actor].hand = vec![BOAR];
            let ordinary = g.cost_plan(&hand()).unwrap();
            assert_eq!(g.cost_plan(&chosen(false, true)).unwrap(), ordinary);
            assert!(g.play_choices(&hand()).unwrap().entry_ready.is_some());
            assert!(g.play_choices(&hand()).unwrap().bonus.is_none());
            reject_unchanged(&mut g, chosen(true, false));
            g.apply(chosen(false, accept)).unwrap();
            let unit = g.players[actor].units.last().unwrap();
            let id = unit.id;
            assert_eq!(unit.ready, accept);
            assert_eq!(g.players[actor].energy, 18);
            assert_eq!(g.movement_locked(actor, id), accept);
            if accept {
                reject_unchanged(
                    &mut g,
                    Action::Move {
                        unit_id: id,
                        lane: Some(0),
                    },
                );
                for effect_actor in 0..2 {
                    g.move_effect(
                        effect_actor,
                        Target::Unit {
                            player: actor,
                            unit_id: id,
                        },
                        Some(0),
                        false,
                    )
                    .unwrap();
                    assert_eq!(g.players[actor].units.last().unwrap().lane, None);
                }
                g.apply(Action::EndTurn).unwrap();
                assert!(
                    !g.movement_locked(actor, id),
                    "restriction expires at turn end"
                );
            }
        }
    }
}

#[test]
fn declining_boar_does_not_restrict_a_different_ready_entry_grant() {
    for accept in [false, true] {
        let mut g = position(0, 0, 467002);
        g.players[0].hand = vec![BOAR];
        g.play_grants.push(PlayGrant {
            id: 999,
            player: 0,
            source: ChainSource::Card { card_id: 427 },
            expires_turn: g.turn,
            kind: PlayGrantKind::ReadyNextUnit,
        });
        g.apply(chosen(false, accept)).unwrap();
        let unit = g.players[0].units.last().unwrap();
        assert!(unit.ready);
        assert_eq!(g.movement_locked(0, unit.id), accept);
    }
}

#[test]
fn boar_choice_is_available_to_automated_players() {
    let mut g = position(0, 0, 467003);
    g.players[0].hand = vec![BOAR];
    assert!(g.legal_actions().unwrap().iter().any(|a| matches!(a,
        Action::WithPlayChoices { choices, .. } if choices.entry_ready)));
}

#[test]
fn optional_entry_ready_is_initial_state_even_under_warden() {
    for card in [GUARDIAN, BOAR] {
        let mut g = position(0, 0, 467004);
        g.players[0].hand = vec![card];
        if card == GUARDIAN {
            play_spell(&mut g, 0, true);
        }
        ready_unit(&mut g, 1, 464);
        g.players[1].units.last_mut().unwrap().lane = Some(1);
        g.apply(chosen(card == GUARDIAN, card == BOAR)).unwrap();
        let id = g.players[0].units.last().unwrap().id;
        assert!(g.players[0].units.last().unwrap().ready);
        g.players[0].units.last_mut().unwrap().ready = false;
        g.ready_unit_by(0, 0, id);
        assert!(
            !g.players[0].units.last().unwrap().ready,
            "the later Ready action is blocked"
        );
    }
}

#[test]
fn boar_restriction_follows_object_and_not_a_returned_replayed_card() {
    let mut g = position(0, 0, 467005);
    g.players[0].hand = vec![BOAR];
    g.apply(chosen(false, true)).unwrap();
    let id = g.players[0].units.last().unwrap().id;
    // Synthetic controller transfer isolates physical identity; no admitted
    // card currently transfers control, so this is an internal invariant.
    let unit = g.players[0].units.pop().unwrap();
    g.players[1].units.push(unit);
    assert!(g.movement_prevented(0, 1, id));
    g.move_effect(
        0,
        Target::Unit {
            player: 1,
            unit_id: id,
        },
        Some(0),
        false,
    )
    .unwrap();
    assert_eq!(g.players[1].units[0].lane, None);
    let unit = g.players[1].units.pop().unwrap();
    g.players[0].units.push(unit);
    g.return_unit_to_hand(Target::Unit {
        player: 0,
        unit_id: id,
    });
    g.apply(hand()).unwrap();
    let replay = g.players[0].units.last().unwrap();
    assert_ne!(replay.id, id);
    assert!(!g.movement_locked(0, replay.id));
    assert!(!replay.ready);
}

#[test]
fn instructed_boar_replay_offers_a_fresh_choice_to_humans_and_bots() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 467006);
        let old_id = ready_unit(&mut g, actor, BOAR);
        g.players[actor].hand = vec![381]; // Temporal Breach.
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: actor,
                unit_id: old_id,
            }],
        })
        .unwrap();
        for _ in 0..24 {
            if matches!(g.decision, Some(Decision::PendingPlay { .. })) {
                break;
            }
            g.apply(decline_legend_action(&g).unwrap_or(Action::Pass))
                .unwrap();
        }
        assert!(matches!(g.decision, Some(Decision::PendingPlay { .. })));
        let action = g
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|a| {
                matches!(a,
            Action::WithPlayChoices { choices, .. } if choices.entry_ready)
            })
            .expect("pending entry choice");
        let before = g.players[actor].energy;
        g.apply(action).unwrap();
        settle(&mut g);
        let new = g.players[actor].units.last().unwrap();
        assert_ne!(new.id, old_id);
        assert!(new.ready && g.movement_locked(actor, new.id));
        assert_eq!(
            g.players[actor].energy, before,
            "instructed base cost ignored, entry choice free"
        );
    }
}

#[test]
fn guardian_unfunded_optional_cost_rolls_back_and_spell_history_expires() {
    let mut g = position(0, 0, 466004);
    g.players[0].hand = vec![GUARDIAN];
    play_spell(&mut g, 0, true);
    g.players[0].power_pool = [0; 6];
    reject_unchanged(&mut g, chosen(true, false));
    assert!(g.played_spell_this_turn(0));
    g.apply(Action::EndTurn).unwrap();
    assert!(!g.played_spell_this_turn(0));
    settle(&mut g);
    g.apply(Action::EndTurn).unwrap();
    settle(&mut g);
    assert_eq!(g.priority(), 0);
    assert!(g.play_choices(&hand()).unwrap().bonus.is_none());
}

#[test]
fn staged_payment_keeps_entry_choices_and_refusal_does_not_leak_them() {
    for actor in 0..2 {
        for card in [GUARDIAN, BOAR] {
            for refuse in [false, true] {
                let mut g = position(actor, 0, 467007);
                g.players[actor].hand = vec![card];
                if card == GUARDIAN {
                    play_spell(&mut g, actor, true);
                }
                let hand_before = g.players[actor].hand.clone();
                let energy = g.players[actor].energy;
                let power = g.players[actor].power_pool;
                g.apply(Action::BeginPayment {
                    action: Box::new(chosen(card == GUARDIAN, card == BOAR)),
                })
                .unwrap();
                assert!(
                    g.players[actor].units.is_empty(),
                    "entry waits for finalization"
                );
                if refuse {
                    g.apply(Action::RefusePayment).unwrap();
                    assert_eq!(g.players[actor].hand, hand_before);
                    assert_eq!(g.players[actor].energy, energy);
                    assert_eq!(g.players[actor].power_pool, power);
                    g.apply(hand()).unwrap();
                    let unit = g.players[actor].units.last().unwrap();
                    assert!(!unit.ready && !g.movement_locked(actor, unit.id));
                } else {
                    g.apply(Action::CompletePayment).unwrap();
                    let unit = g.players[actor].units.last().unwrap();
                    assert!(unit.ready);
                    assert_eq!(g.movement_locked(actor, unit.id), card == BOAR);
                }
            }
        }
    }
}
