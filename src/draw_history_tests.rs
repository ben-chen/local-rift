//! Controlled history-boundary fixtures; all game transitions use real actions.
use super::*;
use crate::gear::GearState;

fn position() -> Game {
    let mut g = Game::new_for_agents(3, 3, 21021, Difficulty::Tactical);
    g.start_match().unwrap();
    g.battlefields = [2, 2];
    g.battlefield_owner = [None; 2];
    for p in &mut g.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.deck = vec![96; 8];
        p.runes = [0; 6];
        p.exhausted_runes = [0; 6];
        p.energy_pool = 100;
        p.energy = 100;
        p.power_pool = [20; 6];
        p.any_power = 0;
    }
    for legend in &mut g.legend {
        legend.ready = false;
    }
    g.trigger_state.draws = [0; 2];
    g
}
fn unit(g: &mut Game, card: CardId, lane: Option<usize>) -> Target {
    let unit_id = g.create_unit(0, card, false, 0);
    let t = Target::Unit { player: 0, unit_id };
    g.unit_mut(t).unwrap().lane = lane;
    g.unit_mut(t).unwrap().ready = true;
    if let Some(lane) = lane {
        g.battlefield_owner[lane] = Some(0);
    }
    t
}
fn cast(g: &mut Game, card: CardId, targets: Vec<Target>) {
    let p = g.priority();
    let hand_index = g.players[p].hand.len();
    g.players[p].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
}
fn settle(g: &mut Game) {
    for _ in 0..80 {
        if g.decision.is_some() || g.chain.is_empty() {
            return;
        }
        g.apply(Action::Pass).unwrap();
    }
    panic!("unsettled chain");
}
fn reject_capacity(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    let err = g.apply(action).unwrap_err();
    assert!(err.starts_with("engine_capacity:"), "{err}");
    assert_eq!(format!("{g:?}"), before);
}
fn overflowing_pass(g: &mut Game) {
    g.apply(Action::Pass).unwrap();
    reject_capacity(g, Action::Pass);
}

#[test]
fn old_width_crossing_keeps_exact_count_and_does_not_retrigger_second_draw() {
    for initial in [1, u64::from(u16::MAX)] {
        let mut g = position();
        unit(&mut g, 96, None);
        g.players[0].gear.push(GearState {
            id: 9000,
            card_id: 195,
            lane: None,
            attached_to: None,
            ready: true,
            temporary: false,
            token: false,
        });
        g.trigger_state.draws[0] = initial;
        cast(&mut g, 77, vec![]);
        settle(&mut g);
        assert_eq!(g.trigger_state.draws[0], initial + 4);
        assert_eq!(g.players[0].hand.len(), 4);
        let jewel = g.chain.iter().filter(|i| i.card_id == 195).count()
            + usize::from(matches!(
                g.decision,
                Some(Decision::TriggerTargets { card_id: 195, .. })
            ));
        assert_eq!(jewel, usize::from(initial == 1));
    }
}

#[test]
fn later_draw_failure_rolls_back_the_entire_resolving_pass() {
    let mut g = position();
    g.trigger_state.draws[0] = u64::MAX - 1;
    cast(&mut g, 77, vec![]);
    overflowing_pass(&mut g);
    assert_eq!(g.trigger_state.draws[0], u64::MAX - 1);
    assert!(g.chain.iter().any(|i| i.card_id == 77));
    assert!(g.players[0].hand.is_empty());
}

#[test]
fn might_before_failed_draw_is_also_rolled_back() {
    let mut g = position();
    let target = unit(&mut g, 96, None);
    g.trigger_state.draws[0] = u64::MAX;
    cast(&mut g, 7, vec![target]);
    overflowing_pass(&mut g);
    assert_eq!(g.unit_ref(target).unwrap().temporary_buff, 0);
}

#[test]
fn exact_duplicate_occurrence_failure_preserves_private_choice_and_decline() {
    for card in [39, 389] {
        let mut g = position();
        g.players[0].deck = vec![71, 96, 96];
        g.trigger_state.draws[0] = u64::MAX;
        cast(&mut g, card, vec![]);
        settle(&mut g);
        let (select, decline) = match &g.decision {
            Some(Decision::TopThree { cards, .. }) => {
                assert_eq!(cards, &[96, 96, 71]);
                (
                    Action::SelectCards { indices: vec![1] },
                    Action::SelectCards { indices: vec![] },
                )
            }
            Some(Decision::DrawLook {
                choice_id, cards, ..
            }) => {
                assert_eq!(cards, &[96, 96, 71]);
                (
                    Action::ResolveDrawLook {
                        choice_id: *choice_id,
                        index: Some(1),
                    },
                    Action::ResolveDrawLook {
                        choice_id: *choice_id,
                        index: None,
                    },
                )
            }
            other => panic!("unexpected choice {other:?}"),
        };
        reject_capacity(&mut g, select);
        assert!(
            !g.legal_actions().unwrap().is_empty(),
            "decline is executable"
        );
        g.apply(decline).unwrap();
        assert_eq!(g.trigger_state.draws[0], u64::MAX);
        assert!(g.players[0].hand.is_empty());
        if card == 389 {
            assert_eq!(g.players[0].deck.len(), 3);
        } else {
            assert!(g.players[0].discard.contains(&96));
        }
    }
}

#[test]
fn exact_draw_crosses_old_width_without_drawing_a_different_occurrence() {
    for card in [39, 389] {
        let mut g = position();
        g.players[0].deck = vec![71, 96, 145];
        g.trigger_state.draws[0] = u64::from(u16::MAX);
        cast(&mut g, card, vec![]);
        settle(&mut g);
        let action = match g.decision {
            Some(Decision::TopThree { .. }) => Action::SelectCards { indices: vec![1] },
            Some(Decision::DrawLook { choice_id, .. }) => Action::ResolveDrawLook {
                choice_id,
                index: Some(1),
            },
            _ => panic!("missing look"),
        };
        g.apply(action).unwrap();
        assert_eq!(g.players[0].hand, [96]);
        assert_eq!(g.trigger_state.draws[0], 65_536);
    }
}

#[test]
fn second_players_draw_failure_restores_both_players_and_observers() {
    let mut g = position();
    g.trigger_state.draws = [u64::MAX - 1, u64::MAX];
    cast(&mut g, 186, vec![]);
    assert!(
        g.chain
            .iter()
            .any(|i| i.card_id == 186 && i.ability.is_some())
    );
    overflowing_pass(&mut g);
    assert!(g.players.iter().all(|p| p.hand.is_empty()));
}

#[test]
fn burnout_before_failed_draw_restores_shuffle_score_trash_and_flow() {
    let mut g = position();
    g.players[0].deck.clear();
    g.players[0].discard = vec![39, 96, 145];
    g.temporary_flow[0] = vec![0];
    g.trigger_state.draws[0] = u64::MAX;
    cast(&mut g, 77, vec![]);
    overflowing_pass(&mut g);
    assert_eq!(g.players[0].discard, [39, 96, 145]);
    assert_eq!(g.temporary_flow[0], [0]);
}

#[test]
fn terminal_empty_burnout_does_not_preflight_unfulfilled_draws() {
    for available in [0, 1] {
        let mut g = position();
        g.players[0].deck = vec![96; available];
        g.trigger_state.draws[0] = u64::MAX - available as u64;
        g.players[1].score = VICTORY_SCORE - 1;
        cast(&mut g, 77, vec![]);
        settle(&mut g);
        assert_eq!(g.winner, Some(1));
        assert_eq!(g.trigger_state.draws[0], u64::MAX);
        assert_eq!(g.players[0].hand.len(), available);
    }
}

#[test]
fn move_reveal_draw_hit_errors_but_recycle_miss_remains_legal() {
    for top in [3, 96] {
        let mut g = position();
        unit(&mut g, 96, Some(0));
        let Target::Unit { unit_id, .. } = unit(&mut g, 392, None) else {
            unreachable!()
        };
        g.players[0].deck = vec![top];
        g.trigger_state.draws[0] = u64::MAX;
        g.apply(Action::Move {
            unit_id,
            lane: Some(0),
        })
        .unwrap();
        if top == 3 {
            overflowing_pass(&mut g);
        } else {
            settle(&mut g);
            assert!(matches!(
                g.decision,
                Some(Decision::RevealedCard {
                    outcome: RevealOutcome::Recycle,
                    ..
                })
            ));
            assert_eq!(g.trigger_state.draws[0], u64::MAX);
        }
    }
}

#[test]
fn final_conquer_draw_failure_restores_control_and_scoring_flags() {
    let mut g = position();
    g.players[0].score = VICTORY_SCORE - 1;
    g.trigger_state.draws[0] = u64::MAX;
    let Target::Unit { unit_id, .. } = unit(&mut g, 96, None) else {
        unreachable!()
    };
    g.apply(Action::Move {
        unit_id,
        lane: Some(0),
    })
    .unwrap();
    overflowing_pass(&mut g);
    assert_eq!(g.battlefield_owner[0], None);
    assert!(!g.player_scored_this_turn[0][0]);
    assert!(g.showdown.is_some());
}

#[test]
fn turn_reset_and_non_draw_operations_accept_a_maximum_history() {
    let mut g = position();
    g.trigger_state.draws = [u64::MAX; 2];
    g.draw(0, 0).unwrap();
    g.burn(0, 1).unwrap();
    assert_eq!(g.trigger_state.draws, [u64::MAX; 2]);
    g.apply(Action::EndTurn).unwrap();
    assert_eq!(g.active, 1);
    assert_eq!(g.trigger_state.draws, [0, 1]);
}

#[test]
fn later_repeat_failure_undoes_the_first_execution_of_that_pass() {
    let mut g = position();
    g.trigger_state.draws[0] = u64::MAX - 1;
    g.players[0].hand = vec![133];
    g.apply(Action::PlayCardOptions {
        hand_index: 0,
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: true,
        accelerate: false,
        bonus: false,
    })
    .unwrap();
    assert!(g.chain.iter().any(|i| i.card_id == 133 && i.options.repeat));
    overflowing_pass(&mut g);
    assert!(g.players[0].hand.is_empty());
    assert_eq!(g.trigger_state.draws[0], u64::MAX - 1);
}

#[test]
fn failed_resolving_pass_preserves_costs_committed_by_prior_payment_command() {
    let mut g = position();
    g.trigger_state.draws[0] = u64::MAX - 1;
    g.players[0].hand = vec![77];
    g.apply(Action::BeginPayment {
        action: Box::new(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        }),
    })
    .unwrap();
    g.apply(Action::CompletePayment).unwrap();
    let paid_energy = g.players[0].energy;
    assert!(paid_energy < 100);
    overflowing_pass(&mut g);
    assert_eq!(g.players[0].energy, paid_energy);
    assert!(g.players[0].hand.is_empty());
    assert!(g.chain.iter().any(|i| i.card_id == 77));
}

#[test]
fn draw_phase_failure_restores_the_completed_beginning_expiry_command() {
    let mut g = position();
    g.players[1].gear.push(GearState {
        id: 9000,
        card_id: 331,
        lane: None,
        attached_to: None,
        ready: true,
        temporary: true,
        token: false,
    });
    g.apply(Action::EndTurn).unwrap();
    assert_eq!(g.active, 1);
    assert!(
        g.chain
            .iter()
            .any(|i| matches!(i.ability, Some(ChainAbility::Temporary { .. })))
    );
    // Boundary is seeded AFTER the real turn reset, in its Beginning window.
    g.trigger_state.draws[1] = u64::MAX;
    overflowing_pass(&mut g);
    assert!(g.players[1].gear.iter().any(|gear| gear.id == 9000));
    assert!(g.pending_start.is_some());
    assert!(!g.players[1].discard.contains(&331));
}
