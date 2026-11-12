//! Back Off: actual play origin, independent draw, Hidden and Stun timing.
use super::*;
use crate::play::{
    ExecutionChoices, MoveDestination, PlayChoices, PlayGrant, PlayGrantKind, RepeatSelection,
};

fn position(owner: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 428100, Difficulty::Tactical);
    g.choose_starting_player(owner).unwrap();
    g.start_match().unwrap();
    g.battlefields = vec![2, 2];
    g.battlefield_owner = vec![None; 2];
    for p in &mut g.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.discard.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 50;
        p.energy = 50;
        p.power_pool = [20; 6];
    }
    g
}
fn unit(g: &mut Game, player: usize, id: u32, lane: Option<usize>) -> Target {
    g.players[player].units.push(Unit {
        id,
        card_id: 73,
        lane,
        ready: true,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    });
    Target::Unit {
        player,
        unit_id: id,
    }
}
fn settle(g: &mut Game) {
    if g.chain.is_empty() {
        g.after_resolution().unwrap();
    }
    for _ in 0..80 {
        let action = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            None if g.in_window() => Action::Pass,
            None => return,
            other => panic!("Unexpected decision {other:?}"),
        };
        g.apply(action).unwrap();
    }
    panic!("Chain did not settle");
}
fn cast(g: &mut Game, id: usize, lane: Option<usize>, targets: Vec<Target>) {
    let actor = g.priority();
    let hand_index = g.players[actor].hand.len();
    g.players[actor].hand.push(id);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane,
        targets,
    })
    .unwrap();
}
fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(
        g.apply(action.clone()).is_err(),
        "Accepted {action:?}, priority {}",
        g.priority()
    );
    assert_eq!(format!("{g:?}"), before);
}
fn draft(g: &mut Game, id: usize, target: Target, lane: Option<usize>) -> Action {
    g.players[g.priority()].hand = vec![id];
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane,
        targets: vec![target],
    }
}
fn repeat(
    g: &mut Game,
    action: Action,
    target: Target,
    destination: Option<MoveDestination>,
) -> Action {
    g.play_grants.push(PlayGrant {
        id: 8800,
        player: g.priority(),
        source: ChainSource::Card { card_id: 428 },
        expires_turn: g.turn,
        kind: PlayGrantKind::RepeatNextSpell,
    });
    let option_id = g.play_choices(&action).unwrap().repeats[0].id;
    Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats: vec![RepeatSelection {
                option_id,
                choices: ExecutionChoices {
                    targets: vec![target],
                    destination,
                    ..Default::default()
                },
            }],
            ..Default::default()
        },
    }
}

#[test]
fn backoff_from_hand_stuns_either_player_without_exhausting_and_draws_once() {
    for actor in 0..2 {
        for owner in 0..2 {
            for stunned in [false, true] {
                let mut g = position(actor);
                let target = unit(&mut g, owner, 500, None);
                g.unit_mut(target).unwrap().stunned = stunned;
                g.players[actor].deck = vec![71];
                let action = draft(&mut g, 428, target, None);
                let before = g.players[actor].energy;
                g.apply(action).unwrap();
                assert_eq!(g.players[actor].energy, before - 3);
                assert_eq!(g.chain.last().unwrap().play_origin, Some(PlayOrigin::Hand));
                assert!(g.players[actor].hand.is_empty());
                settle(&mut g);
                let u = g.unit_ref(target).unwrap();
                assert!(u.stunned && u.ready);
                assert_eq!(u.damage, 0);
                assert_eq!(g.players[actor].hand, vec![71]);
                assert_eq!(g.players[actor].discard, vec![428]);
            }
        }
    }
}

#[test]
fn backoff_draw_survives_a_departed_or_protected_unit_and_does_not_follow_a_new_copy() {
    for response in 0..3 {
        let mut g = position(0);
        let target = unit(&mut g, 1, 500, None);
        g.players[0].deck = vec![71];
        cast(&mut g, 428, None, vec![target]);
        match response {
            0 => {
                g.bounce(target);
            }
            1 => g.unit_mut(target).unwrap().protected = true,
            _ => {
                g.bounce(target);
                unit(&mut g, 1, 501, None);
            }
        }
        settle(&mut g);
        assert_eq!(g.players[0].hand, vec![71]);
        assert!(g.players[1].units.iter().all(|u| !u.stunned));
    }
}

#[test]
fn backoff_counter_prevents_both_stun_and_draw() {
    let mut g = position(0);
    let target = unit(&mut g, 1, 500, None);
    g.players[0].deck = vec![71];
    cast(&mut g, 428, None, vec![target]);
    let chain_id = g.chain.last().unwrap().id;
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 6, None, vec![Target::Spell { chain_id }]);
    settle(&mut g);
    assert!(!g.unit_ref(target).unwrap().stunned);
    assert_eq!(g.players[0].deck, vec![71]);
    assert!(g.players[0].hand.is_empty());
}

#[test]
fn backoff_independent_draw_finishes_first_burnout_and_repeat_respects_empty_deck_retry_victory() {
    for repeated in [false, true] {
        let mut g = position(0);
        let target = unit(&mut g, 1, 500, None);
        g.players[0].deck.clear();
        g.players[0].discard = vec![71];
        g.players[1].score = 7;
        let base = draft(&mut g, 428, target, None);
        let action = if repeated {
            repeat(&mut g, base, target, None)
        } else {
            base
        };
        g.apply(action).unwrap();
        g.apply(Action::Pass).unwrap();
        g.apply(Action::Pass).unwrap();
        assert!(g.unit_ref(target).unwrap().stunned);
        assert_eq!(g.players[0].hand, vec![71]);
        // The first draw burns out once and succeeds. Repeat starts a new draw
        // with empty deck and trash: its first burnout reaches 9, and the retry
        // reaches 10 and wins immediately (432.3.c.1), before ordinary cleanup.
        assert_eq!(g.players[1].score, if repeated { 10 } else { 8 });
        assert_eq!(g.winner, Some(1));
    }
}

#[test]
fn backoff_hand_is_action_timing_and_repeat_draws_for_each_execution() {
    let mut g = position(0);
    let target = unit(&mut g, 1, 500, None);
    g.players[0].deck = vec![70, 71];
    let base = draft(&mut g, 428, target, None);
    let action = repeat(&mut g, base, target, None);
    g.apply(action).unwrap();
    assert_eq!(g.chain.last().unwrap().executions.len(), 2);
    settle(&mut g);
    assert_eq!(g.players[0].hand, vec![71, 70]);
    assert_eq!(g.players[0].discard, vec![428]);
    cast(&mut g, 7, None, vec![target]);
    let action = draft(&mut g, 428, target, None);
    reject(&mut g, action);
}

fn hidden(g: &mut Game, actor: usize, target: Target) -> u32 {
    g.battlefield_owner[0] = Some(actor);
    g.unit_mut(target).unwrap().lane = Some(0);
    g.players[actor].hand = vec![428];
    g.apply(Action::Hide {
        hand_index: 0,
        lane: 0,
    })
    .unwrap();
    g.players[actor].hidden[0].id
}

#[test]
fn backoff_hidden_does_not_play_or_draw_when_hidden_then_stuns_without_drawing() {
    for actor in 0..2 {
        let mut g = position(actor);
        let target = unit(&mut g, actor, 500, Some(0));
        g.players[actor].deck = vec![71, 70];
        let played = g.cards_played_this_turn(actor);
        let hidden_id = hidden(&mut g, actor, target);
        assert!(g.chain.is_empty());
        assert_eq!(g.cards_played_this_turn(actor), played);
        assert_eq!(g.hidden_visible_card(actor, actor, hidden_id), Some(428));
        assert_eq!(g.hidden_visible_card(1 - actor, actor, hidden_id), None);
        reject(
            &mut g,
            Action::PlayHidden {
                hidden_id,
                targets: vec![target],
            },
        );
        g.apply(Action::EndTurn).unwrap();
        settle(&mut g);
        assert_eq!(g.active, 1 - actor);
        g.players[1 - actor].energy_pool = 20;
        g.players[1 - actor].energy = 20;
        cast(&mut g, 7, None, vec![target]);
        g.apply(Action::Pass).unwrap();
        assert_eq!(g.priority(), actor);
        let hand = g.players[actor].hand.clone();
        let deck = g.players[actor].deck.clone();
        let energy = g.players[actor].energy;
        g.apply(Action::PlayHidden {
            hidden_id,
            targets: vec![target],
        })
        .unwrap();
        let item = g.chain.last().unwrap();
        assert_eq!(item.play_origin, Some(PlayOrigin::Hidden { lane: 0 }));
        assert!(item.from_hidden);
        assert_eq!(g.players[actor].energy, energy);
        settle(&mut g);
        assert!(g.unit_ref(target).unwrap().stunned);
        assert_eq!(g.players[actor].hand, hand);
        assert_eq!(g.players[actor].deck, deck);
    }
}

#[test]
fn backoff_hidden_targets_stay_at_the_original_field_and_moved_targets_are_not_stunned() {
    let mut g = position(0);
    let target = unit(&mut g, 0, 500, Some(0));
    let outside = unit(&mut g, 1, 501, None);
    let hidden_id = hidden(&mut g, 0, target);
    g.players[0].hidden[0].hidden_turn = g.turn - 1;
    reject(
        &mut g,
        Action::PlayHidden {
            hidden_id,
            targets: vec![outside],
        },
    );
    let action = Action::PlayHidden {
        hidden_id,
        targets: vec![target],
    };
    assert_eq!(
        g.play_target_view(
            &Action::PlayHidden {
                hidden_id,
                targets: vec![]
            },
            0
        )
        .unwrap()
        .candidates,
        vec![target]
    );
    g.apply(action).unwrap();
    g.move_effect(0, target, None, false).unwrap();
    settle(&mut g);
    assert!(!g.unit_ref(target).unwrap().stunned);
    assert!(g.players[0].hand.is_empty());
}

#[test]
fn backoff_hidden_repeat_has_independent_local_targets_but_no_draws() {
    let mut g = position(0);
    let a = unit(&mut g, 0, 500, Some(0));
    let b = unit(&mut g, 0, 501, Some(0));
    let hidden_id = hidden(&mut g, 0, a);
    g.players[0].hidden[0].hidden_turn = g.turn - 1;
    g.players[0].deck = vec![71, 70];
    let action = repeat(
        &mut g,
        Action::PlayHidden {
            hidden_id,
            targets: vec![a],
        },
        b,
        None,
    );
    assert_eq!(g.cost_plan(&action).unwrap().energy, 3); // Hidden waives only the base cost.
    g.apply(action).unwrap();
    settle(&mut g);
    assert!(g.unit_ref(a).unwrap().stunned && g.unit_ref(b).unwrap().stunned);
    assert_eq!(g.players[0].deck, vec![71, 70]);
    assert!(g.players[0].hand.is_empty());
}

#[test]
fn backoff_hidden_still_pays_deflect_and_failed_payment_is_atomic() {
    let mut g = position(0);
    let guard = unit(&mut g, 0, 500, Some(0));
    let enemy = unit(&mut g, 1, 501, None);
    g.unit_mut(enemy).unwrap().card_id = 92;
    let hidden_id = hidden(&mut g, 0, guard);
    g.players[0].hidden[0].hidden_turn = g.turn - 1;
    g.move_effect(1, enemy, Some(0), false).unwrap();
    g.after_resolution().unwrap();
    assert_eq!(g.priority(), 1);
    g.apply(Action::Pass).unwrap();
    assert_eq!(g.priority(), 0);
    g.players[0].power_pool = [0; 6];
    g.players[0].any_power = 0;
    let action = Action::PlayHidden {
        hidden_id,
        targets: vec![enemy],
    };
    reject(&mut g, action.clone());
    g.players[0].any_power = 1;
    g.apply(action).unwrap();
    assert_eq!(g.players[0].any_power, 0);
    // Resolve only the spell, before the subsequent combat needs damage choices.
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
    assert!(g.unit_ref(enemy).unwrap().stunned);
}

#[test]
fn backoff_flow_from_trash_is_not_a_hand_play_even_when_it_is_not_hidden() {
    for repeated in [false, true] {
        let mut g = position(0);
        let target = unit(&mut g, 1, 500, None);
        g.players[0].discard = vec![428];
        g.players[0].deck = vec![71, 70];
        g.temporary_flow[0] = vec![0];
        let base = Action::PlayFromTrashWithTargets {
            trash_index: 0,
            lane: None,
            targets: vec![target],
        };
        let action = if repeated {
            repeat(&mut g, base, target, None)
        } else {
            base
        };
        g.apply(action).unwrap();
        assert_eq!(g.chain.last().unwrap().play_origin, Some(PlayOrigin::Trash));
        assert!(!g.chain.last().unwrap().from_hidden);
        settle(&mut g);
        assert!(g.unit_ref(target).unwrap().stunned);
        assert_eq!(g.players[0].deck, vec![71, 70]);
        assert!(g.players[0].hand.is_empty());
        assert_eq!(g.players[0].banish, vec![428]);
    }
}

#[test]
fn backoff_instructed_banish_play_keeps_its_origin_and_has_no_draw() {
    let mut g = position(0);
    let target = unit(&mut g, 1, 500, None);
    g.players[0].banish = vec![428];
    g.players[0].deck = vec![71];
    g.queue_banish_play(0, 0, InstructedPlayKind::ClocktowerGuardian, None);
    g.after_resolution().unwrap();
    let action = Action::FinalizePendingPlay {
        lane: None,
        targets: vec![target],
        repeat_targets: vec![],
        repeat: false,
        accelerate: false,
        bonus: false,
    };
    g.apply(action).unwrap();
    assert_eq!(
        g.chain.last().unwrap().play_origin,
        Some(PlayOrigin::Banish)
    );
    settle(&mut g);
    assert!(g.unit_ref(target).unwrap().stunned);
    assert_eq!(g.players[0].deck, vec![71]);
    assert!(g.players[0].hand.is_empty());
}

#[test]
fn backoff_staged_payment_and_refusal_preserve_origin_targets_and_deck() {
    let mut g = position(0);
    let target = unit(&mut g, 1, 500, None);
    let action = draft(&mut g, 428, target, None);
    let before = format!("{g:?}");
    g.apply(Action::BeginPayment {
        action: Box::new(action.clone()),
    })
    .unwrap();
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(format!("{g:?}"), before);
    g.players[0].deck = vec![71];
    g.apply(Action::BeginPayment {
        action: Box::new(action),
    })
    .unwrap();
    g.apply(Action::CompletePayment).unwrap();
    assert_eq!(g.chain.last().unwrap().play_origin, Some(PlayOrigin::Hand));
    settle(&mut g);
    assert_eq!(g.players[0].hand, vec![71]);
}

#[test]
fn backoff_friendly_irelia_choice_triggers_before_stun_and_stun_expires_at_turn_end() {
    let mut g = position(0);
    let target = unit(&mut g, 0, 500, None);
    g.unit_mut(target).unwrap().card_id = 0;
    cast(&mut g, 428, None, vec![target]);
    assert_eq!(g.chain.len(), 2);
    settle(&mut g);
    assert_eq!(g.unit_ref(target).unwrap().temporary_buff, 1);
    assert!(g.unit_ref(target).unwrap().stunned);
    g.apply(Action::EndTurn).unwrap();
    settle(&mut g);
    assert!(!g.unit_ref(target).unwrap().stunned);
}
