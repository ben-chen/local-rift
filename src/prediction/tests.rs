//! Exact source sequences with controlled initial positions and public actions.
use super::*;
use crate::activated::AbilitySource;
use crate::gear::GearState;

fn position() -> Game {
    let mut g = Game::new_for_agents(3, 3, 21031, Difficulty::Tactical);
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
        p.xp = 0;
    }
    for l in &mut g.legend {
        l.ready = false;
    }
    g.trigger_state.draws = [0; 2];
    g
}
fn unit(g: &mut Game, owner: usize, card: CardId, lane: Option<usize>) -> Target {
    let unit_id = g.create_unit(owner, card, false, 0);
    let target = Target::Unit {
        player: owner,
        unit_id,
    };
    g.unit_mut(target).unwrap().lane = lane;
    if let Some(lane) = lane {
        g.battlefield_owner[lane] = Some(owner);
    }
    target
}
fn gear(g: &mut Game, card: CardId) -> u32 {
    let id = g.next_unit_id;
    g.next_unit_id += 1;
    g.players[0].gear.push(GearState {
        id,
        card_id: card,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    id
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
    for _ in 0..100 {
        if g.decision.is_some() || g.chain.is_empty() {
            return;
        }
        g.apply(Action::Pass).unwrap();
    }
    panic!("unsettled chain");
}
fn predict(g: &Game) -> (u32, Vec<CardId>) {
    let Some(Decision::PredictCards {
        choice_id, cards, ..
    }) = &g.decision
    else {
        panic!("expected Predict: {:?}", g.decision)
    };
    (*choice_id, cards.clone())
}
fn keep(g: &mut Game, order: Vec<usize>) {
    let choice_id = predict(g).0;
    g.apply(Action::ResolvePredict {
        choice_id,
        recycle: vec![],
        top_order: order,
    })
    .unwrap();
}
fn bloom_action(id: u32) -> Action {
    Action::ActivateAbility {
        source: AbilitySource::Gear {
            player: 0,
            gear_id: id,
        },
        ability_index: 0,
        targets: vec![],
    }
}
fn capacity(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    let error = g.apply(action).unwrap_err();
    assert!(error.starts_with("engine_capacity:"), "{error}");
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn ornn_on_play_draws_one_exact_duplicate_gear_and_recycles_other_occurrences() {
    let mut g = position();
    g.players[0].deck = vec![71, 3, 96, 3, 7];
    cast(&mut g, 395, vec![]);
    assert!(g.chain.iter().any(|i| i.card_id == 395));
    settle(&mut g);
    let Some(Decision::DrawLook {
        choice_id,
        ref cards,
        ref eligible,
        kind,
        ..
    }) = g.decision
    else {
        panic!("no look")
    };
    assert_eq!(cards, &[7, 3, 96, 3]);
    assert_eq!(eligible, &[1, 3]);
    assert_eq!(kind, DrawLookKind::Ornn);
    g.apply(Action::ResolveDrawLook {
        choice_id,
        index: Some(3),
    })
    .unwrap();
    assert_eq!(g.players[0].hand, [3]);
    assert_eq!(g.trigger_state.draws[0], 1);
    assert_eq!(g.players[0].deck.iter().filter(|&&id| id == 3).count(), 1);
    assert_eq!(g.players[0].deck.len(), 4);
    assert!(matches!(
        g.decision,
        Some(Decision::RevealedCard {
            card_id: 3,
            outcome: RevealOutcome::Draw,
            ..
        })
    ));
}

#[test]
fn ornn_trigger_is_mandatory_but_empty_short_and_no_gear_choices_can_decline() {
    for top in [vec![], vec![96], vec![3], vec![7, 96]] {
        let mut g = position();
        g.players[0].deck = top.clone();
        cast(&mut g, 395, vec![]);
        settle(&mut g);
        let Some(Decision::DrawLook {
            choice_id,
            ref cards,
            ..
        }) = g.decision
        else {
            panic!("mandatory look missing")
        };
        assert_eq!(cards.len(), top.len());
        g.apply(Action::ResolveDrawLook {
            choice_id,
            index: None,
        })
        .unwrap();
        assert_eq!(g.trigger_state.draws[0], 0);
        assert_eq!(g.players[0].deck.len(), top.len());
        assert_eq!(g.players[1].score, 0);
    }
}

#[test]
fn ornn_hold_looks_before_channel_and_turn_draw() {
    let mut g = position();
    unit(&mut g, 0, 395, Some(0));
    g.apply(Action::EndTurn).unwrap();
    assert_eq!(g.active, 1);
    g.players[0].deck = vec![96, 3, 7, 117];
    g.apply(Action::EndTurn).unwrap();
    settle(&mut g);
    assert_eq!(g.active, 0);
    assert_eq!(g.players[0].score, 1);
    assert_eq!(g.trigger_state.draws[0], 0);
    let Some(Decision::DrawLook {
        source_card_id: 395,
        choice_id,
        ..
    }) = g.decision
    else {
        panic!("no Hold look")
    };
    g.apply(Action::ResolveDrawLook {
        choice_id,
        index: None,
    })
    .unwrap();
    assert_eq!(g.trigger_state.draws[0], 1);
}

#[test]
fn ornn_resolves_after_a_real_source_return_response() {
    let mut g = position();
    let enemy = unit(&mut g, 1, 96, None);
    g.players[0].deck = vec![3, 96];
    cast(&mut g, 395, vec![]);
    let own = Target::Unit {
        player: 0,
        unit_id: g.players[0]
            .units
            .iter()
            .find(|u| u.card_id == 395)
            .unwrap()
            .id,
    };
    cast(&mut g, 16, vec![own, enemy]);
    settle(&mut g);
    assert!(g.unit_ref(own).is_none());
    assert!(g.players[0].hand.contains(&395));
    assert!(matches!(
        g.decision,
        Some(Decision::DrawLook {
            kind: DrawLookKind::Ornn,
            ..
        })
    ));
}

#[test]
fn clairvoyance_draws_after_arbitrary_predict_partition_and_permutation() {
    let mut g = position();
    g.players[0].deck = vec![71, 7, 96, 3, 145, 117];
    cast(&mut g, 396, vec![]);
    settle(&mut g);
    let (choice_id, cards) = predict(&g);
    assert_eq!(cards, [117, 145, 3, 96, 7]);
    assert_eq!(g.trigger_state.draws[0], 0);
    assert!(g.players[0].hand.is_empty());
    let tail = g.predict_continuation().unwrap();
    assert_eq!(
        (tail.source_card_id, tail.remaining_draw, tail.remaining_xp),
        (396, 2, 0)
    );
    assert!(
        g.chain
            .iter()
            .any(|i| i.id == tail.chain_id && i.ability.is_none())
    );
    assert!(!g.players[0].discard.contains(&396));
    let before = format!("{g:?}");
    assert!(
        g.apply(Action::ResolvePredict {
            choice_id,
            recycle: vec![1],
            top_order: vec![0, 2, 3]
        })
        .is_err()
    );
    assert_eq!(format!("{g:?}"), before);
    g.apply(Action::ResolvePredict {
        choice_id,
        recycle: vec![1, 4],
        top_order: vec![2, 0, 3],
    })
    .unwrap();
    assert_eq!(g.players[0].hand, [3, 117]);
    assert_eq!(g.players[0].deck.last(), Some(&96));
    assert!(g.players[0].discard.contains(&396));
    assert!(g.predict_continuation().is_none());
    assert_eq!(g.trigger_state.completed_cards[0], 1);
}

#[test]
fn clairvoyance_recycle_all_still_draws_existing_recycled_cards_without_burnout() {
    let mut g = position();
    g.players[0].deck = vec![3, 96];
    cast(&mut g, 396, vec![]);
    settle(&mut g);
    let choice_id = predict(&g).0;
    g.apply(Action::ResolvePredict {
        choice_id,
        recycle: vec![0, 1],
        top_order: vec![],
    })
    .unwrap();
    let mut drawn = g.players[0].hand.clone();
    drawn.sort_unstable();
    assert_eq!(drawn, [3, 96]);
    assert_eq!(g.players[1].score, 0);
}

#[test]
fn empty_clairvoyance_predict_continues_to_burnout_without_recycling_its_parent() {
    let mut g = position();
    g.players[0].deck.clear();
    g.players[0].discard = vec![96, 145];
    cast(&mut g, 396, vec![]);
    settle(&mut g);
    assert_eq!(g.players[0].hand.len(), 2);
    assert!(!g.players[0].hand.contains(&396));
    assert_eq!(g.players[1].score, 1);
    assert!(g.players[0].discard.contains(&396));
    assert!(g.decision.is_none());
    assert!(g.predict_continuation().is_none());
}

#[test]
fn repeated_clairvoyance_finishes_each_draw_tail_before_fresh_predict_and_completion() {
    let mut g = position();
    for _ in 0..2 {
        let id = gear(&mut g, 256);
        g.apply(bloom_action(id)).unwrap();
        settle(&mut g);
    }
    unit(&mut g, 0, 96, None);
    gear(&mut g, 195);
    g.players[0].deck = vec![3, 7, 96, 145, 117, 71, 70];
    g.players[0].hand = vec![396];
    let action = Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    };
    let repeats = g.play_choices(&action).unwrap().repeats;
    assert_eq!(repeats.len(), 2);
    g.apply(Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats: repeats
                .into_iter()
                .map(|r| RepeatSelection {
                    option_id: r.id,
                    choices: ExecutionChoices::default(),
                })
                .collect(),
            ..Default::default()
        },
    })
    .unwrap();
    settle(&mut g);
    let mut prior_id = None;
    for execution in 0..3 {
        let (id, cards) = predict(&g);
        assert_ne!(Some(id), prior_id);
        prior_id = Some(id);
        assert_eq!(g.resolution_progress().unwrap().execution, execution);
        assert_eq!(g.trigger_state.draws[0], (execution * 2) as u64);
        assert!(!g.players[0].discard.contains(&396));
        assert!(!g.chain.iter().any(|i| i.card_id == 195));
        assert_eq!(
            cards,
            g.players[0]
                .deck
                .iter()
                .rev()
                .take(5)
                .copied()
                .collect::<Vec<_>>()
        );
        keep(&mut g, (0..cards.len()).collect());
    }
    assert_eq!(g.trigger_state.draws[0], 6);
    assert_eq!(
        g.players[0].discard.iter().filter(|&&id| id == 396).count(),
        1
    );
    assert_eq!(g.trigger_state.completed_cards[0], 1);
    assert!(
        matches!(
            g.decision,
            Some(Decision::TriggerTargets { card_id: 195, .. })
        ) || g.chain.iter().any(|i| i.card_id == 195)
    );
}

#[test]
fn clairvoyance_counter_before_resolution_never_starts_predict_or_draw() {
    let mut g = position();
    cast(&mut g, 396, vec![]);
    let chain_id = g.chain.last().unwrap().id;
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 86, vec![Target::Spell { chain_id }]);
    settle(&mut g);
    assert!(g.predict_continuation().is_none());
    assert_eq!(g.trigger_state.draws[0], 0);
    assert!(g.players[0].discard.contains(&396));
}

#[test]
fn bloom_enters_exhausted_then_real_turn_readiness_allows_the_paid_sacrifice() {
    let mut g = position();
    cast(&mut g, 397, vec![]);
    let id = g.players[0]
        .gear
        .iter()
        .find(|x| x.card_id == 397)
        .unwrap()
        .id;
    assert!(!g.players[0].gear.iter().find(|x| x.id == id).unwrap().ready);
    assert!(g.apply(bloom_action(id)).is_err());
    g.apply(Action::EndTurn).unwrap();
    g.apply(Action::EndTurn).unwrap();
    let energy = g.players[0].energy;
    // The fixture's stored pools clear at turn transition; supply one actual ready rune.
    if energy == 0 {
        g.players[0].runes[0] = 1;
        g.players[0].energy = 1;
    }
    g.apply(bloom_action(id)).unwrap();
    assert!(g.players[0].gear.iter().all(|gear| gear.id != id));
    assert!(g.players[0].discard.contains(&397));
    assert!(g.decision.is_none());
    assert!(g.chain.iter().any(|i| matches!(
        i.ability,
        Some(ChainAbility::Activated { card_id: 397, .. })
    )));
}

#[test]
fn bloom_direct_exact_and_staged_costs_finish_draw_xp_then_gear_use() {
    for mode in 0..3 {
        let mut g = position();
        let prize = unit(&mut g, 0, 215, None);
        let id = gear(&mut g, 397);
        g.players[0].deck = vec![96, 3];
        let action = bloom_action(id);
        match mode {
            0 => g.apply(action).unwrap(),
            1 => {
                let payment = g.quote_power_payment(&action).unwrap().suggested;
                g.apply(Action::WithPowerPayment {
                    action: Box::new(action),
                    payment,
                })
                .unwrap();
            }
            _ => {
                g.apply(Action::BeginPayment {
                    action: Box::new(action),
                })
                .unwrap();
                g.apply(Action::CompletePayment).unwrap();
            }
        }
        assert_eq!(g.players[0].energy, 99);
        assert!(g.players[0].gear.iter().all(|gear| gear.id != id));
        assert_eq!(
            g.players[0].discard.iter().filter(|&&c| c == 397).count(),
            1
        );
        settle(&mut g);
        let tail = g.predict_continuation().unwrap();
        assert_eq!(
            (tail.source_card_id, tail.remaining_draw, tail.remaining_xp),
            (397, 1, 1)
        );
        assert_eq!(g.resolution_progress().unwrap().chain_id, tail.chain_id);
        assert!(g.chain.iter().any(|i| i.id == tail.chain_id));
        assert_eq!(g.players[0].xp, 0);
        assert_eq!(g.trigger_state.draws[0], 0);
        assert!(!g.chain.iter().any(|i| i.card_id == 215));
        keep(&mut g, vec![1, 0]);
        assert_eq!(g.players[0].hand, [96]);
        assert_eq!(g.players[0].xp, 1);
        assert!(g.predict_continuation().is_none());
        assert!(!g.chain.iter().any(|i| i.id == tail.chain_id));
        assert!(g.chain.iter().any(|i| i.card_id == 215));
        settle(&mut g);
        assert_eq!(g.unit_ref(prize).unwrap().temporary_buff, 1);
    }
}

#[test]
fn bloom_empty_predict_can_burn_out_and_draw_the_already_sacrificed_source() {
    let mut g = position();
    g.players[0].deck.clear();
    let id = gear(&mut g, 397);
    g.apply(bloom_action(id)).unwrap();
    settle(&mut g);
    assert_eq!(g.players[0].hand, [397]);
    assert_eq!(g.players[0].xp, 1);
    assert_eq!(g.players[1].score, 1);
    assert!(g.predict_continuation().is_none());
}

#[test]
fn deferred_draw_capacity_restores_predict_partition_rng_parent_and_tail() {
    let mut g = position();
    g.players[0].deck = vec![3, 7, 96, 145, 117];
    g.trigger_state.draws[0] = u64::MAX - 1;
    cast(&mut g, 396, vec![]);
    settle(&mut g);
    let choice_id = predict(&g).0;
    capacity(
        &mut g,
        Action::ResolvePredict {
            choice_id,
            recycle: vec![0, 2],
            top_order: vec![4, 1, 3],
        },
    );
    assert_eq!(g.trigger_state.draws[0], u64::MAX - 1);
}

#[test]
fn bloom_xp_capacity_undoes_its_draw_but_not_prior_paid_costs() {
    let mut g = position();
    let id = gear(&mut g, 397);
    g.players[0].xp = u64::MAX;
    g.players[0].deck = vec![3, 96];
    g.apply(bloom_action(id)).unwrap();
    settle(&mut g);
    let choice_id = predict(&g).0;
    capacity(
        &mut g,
        Action::ResolvePredict {
            choice_id,
            recycle: vec![],
            top_order: vec![1, 0],
        },
    );
    assert_eq!(g.trigger_state.draws[0], 0);
    assert!(g.players[0].hand.is_empty());
    assert_eq!(g.players[0].energy, 99);
    assert!(g.players[0].discard.contains(&397));
}
