//! Clocktower's three triggers and CR356.1.b/358.5/420 limited plays.
use super::*;

fn position() -> Game {
    let mut g = Game::new(3, 3, 18018, Difficulty::Tactical);
    g.start_match().unwrap();
    g.battlefields = [7, 9];
    g.battlefield_owner = [None; 2];
    for p in &mut g.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.runes = [0; 6];
        p.exhausted_runes = [0; 6];
        p.energy_pool = 100;
        p.energy = 100;
        p.power_pool = [20; 6];
        p.any_power = 0;
    }
    for l in &mut g.legend {
        l.ready = false;
    }
    g
}
fn unit(g: &mut Game, player: usize, card: usize, lane: Option<usize>) -> Target {
    let unit_id = g.create_unit(player, card, false, 0);
    let t = Target::Unit { player, unit_id };
    g.unit_mut(t).unwrap().lane = lane;
    t
}
fn resolve(g: &mut Game) {
    for _ in 0..40 {
        if g.decision.is_some() || g.chain.is_empty() {
            return;
        }
        g.apply(Action::Pass).unwrap();
    }
    panic!("Chain did not finish");
}
fn death(g: &mut Game, player: usize, card: usize) {
    g.players[player].deck = vec![71, 72, card];
    let t = unit(g, player, 384, Some(0));
    g.battlefield_owner[0] = Some(player);
    g.kill_unit(t).unwrap();
    g.after_resolution().unwrap();
    assert!(g.players[player].banish.is_empty());
    assert!(
        !g.chain.is_empty(),
        "Deathknell can be responded to before banishing"
    );
    resolve(g);
    assert!(
        matches!(g.decision,Some(Decision::BanishPlay{card_id,player:p,..}) if card_id==card && p==player)
    );
}
fn pending(g: &mut Game, player: usize, card: usize) {
    death(g, player, card);
    g.apply(Action::Choose { accept: true }).unwrap();
    assert!(matches!(
        g.decision,
        Some(Decision::PendingPlay {
            kind: InstructedPlayKind::ClocktowerGuardian,
            ..
        })
    ));
    assert!(g.players[player].banish.is_empty());
    assert_eq!(g.pending_cards()[0].play_origin(), PlayOrigin::Banish);
}
fn finalize() -> Action {
    Action::FinalizePendingPlay {
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate: false,
        bonus: false,
    }
}
fn no_resources(g: &mut Game, player: usize) {
    let p = &mut g.players[player];
    p.energy = 0;
    p.energy_pool = 0;
    p.power_pool = [0; 6];
    p.any_power = 0;
}
#[test]
fn c18_play_predicts_three_with_explicit_recycle_and_top_order() {
    let mut g = position();
    g.players[0].deck = vec![69, 70, 71, 72];
    g.players[0].hand = vec![384];
    g.apply(Action::PlayCard {
        hand_index: 0,
        lane: None,
        target: None,
    })
    .unwrap();
    assert_eq!(g.players[0].units[0].card_id, 384);
    assert!(g.decision.is_none());
    resolve(&mut g);
    let Some(Decision::PredictCards {
        choice_id,
        ref cards,
        ..
    }) = g.decision
    else {
        panic!("{:?}", g.decision)
    };
    assert_eq!(cards, &vec![72, 71, 70]);
    assert_eq!(g.players[0].deck, vec![69, 70, 71, 72]);
    let before = format!("{g:?}");
    assert!(
        g.apply(Action::ResolvePredict {
            choice_id,
            recycle: vec![1],
            top_order: vec![0, 0]
        })
        .is_err()
    );
    assert_eq!(format!("{g:?}"), before);
    g.apply(Action::ResolvePredict {
        choice_id,
        recycle: vec![1],
        top_order: vec![2, 0],
    })
    .unwrap();
    assert_eq!(g.players[0].deck, vec![71, 69, 72, 70]);
}
#[test]
fn c18_hold_predicts_before_channel_and_draw_without_a_second_play_trigger() {
    let mut g = position();
    unit(&mut g, 1, 384, Some(0));
    g.battlefield_owner[0] = Some(1);
    g.players[1].deck = vec![69, 70, 71, 72];
    let rune_deck = g.players[1].rune_deck.clone();
    g.apply(Action::EndTurn).unwrap();
    resolve(&mut g);
    assert_eq!(g.players[1].score, 1);
    assert!(g.players[1].hand.is_empty());
    assert_eq!(g.players[1].rune_deck, rune_deck);
    let Some(Decision::PredictCards {
        choice_id,
        ref cards,
        ..
    }) = g.decision
    else {
        panic!("{:?}", g.decision)
    };
    assert_eq!(cards, &vec![72, 71, 70]);
    g.apply(Action::ResolvePredict {
        choice_id,
        recycle: vec![],
        top_order: vec![2, 1, 0],
    })
    .unwrap();
    assert_eq!(g.players[1].hand, vec![70]);
    assert!(g.players[1].rune_deck.len() < rune_deck.len());
    assert!(g.decision.is_none() && g.chain.is_empty());
}
#[test]
fn c18_banish_is_mandatory_but_play_is_optional_for_either_owner() {
    for p in 0..2 {
        let mut g = position();
        death(&mut g, p, 70);
        assert_eq!(g.players[p].banish, vec![70]);
        assert_eq!(g.players[p].deck, vec![71, 72]);
        assert_eq!(g.players[p].discard, vec![384]);
        g.apply(Action::Choose { accept: false }).unwrap();
        assert_eq!(g.players[p].banish, vec![70]);
        assert!(g.pending_cards().is_empty());
        assert_eq!(g.battlefield_owner[0], None);
    }
}
#[test]
fn c18_pending_child_retains_control_and_enters_the_vacated_battlefield() {
    for p in 0..2 {
        let mut g = position();
        pending(&mut g, p, 70);
        no_resources(&mut g, p);
        assert_eq!(g.battlefield_owner[0], Some(p));
        let mut a = finalize();
        if let Action::FinalizePendingPlay { lane, .. } = &mut a {
            *lane = Some(0);
        }
        g.apply(a).unwrap();
        assert_eq!(g.players[p].units[0].lane, Some(0));
        assert_eq!(g.battlefield_owner[0], Some(p));
    }
}
#[test]
fn c18_any_main_kind_can_be_played_and_spells_do_not_inherit_fizz_recycle() {
    for card in [70, 10, 133] {
        let mut g = position();
        pending(&mut g, 0, card);
        no_resources(&mut g, 0);
        assert_eq!(g.cost_plan(&finalize()).unwrap().energy, 0);
        g.apply(finalize()).unwrap();
        if card == 133 {
            assert!(!g.chain.last().unwrap().recycle_after);
            resolve(&mut g);
            assert!(g.players[0].discard.contains(&133));
            assert!(!g.players[0].deck.contains(&133));
        } else if card == 10 {
            assert_eq!(g.players[0].gear[0].card_id, 10);
        } else {
            assert_eq!(g.players[0].units[0].card_id, 70);
        }
    }
}
#[test]
fn c18_power_is_not_waived_and_failed_finalization_restores_only_the_child() {
    let mut g = position();
    pending(&mut g, 0, 145);
    no_resources(&mut g, 0);
    let before = format!("{g:?}");
    let plan = g.cost_plan(&finalize()).unwrap();
    assert_eq!(plan.energy, 0);
    assert_eq!(plan.power.iter().map(|p| p.amount).sum::<u16>(), 1);
    assert!(g.apply(finalize()).is_err());
    assert_eq!(format!("{g:?}"), before);
    assert!(g.can_decline_trash_play().unwrap());
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(g.players[0].banish, vec![145]);
    assert_eq!(g.players[0].discard, vec![384]);
    assert_eq!(g.players[0].deck, vec![71, 72]);
}
#[test]
fn c18_accelerate_keeps_energy_and_power_in_addition_to_printed_power() {
    let mut g = position();
    pending(&mut g, 0, 145);
    no_resources(&mut g, 0);
    let mut a = finalize();
    if let Action::FinalizePendingPlay { accelerate, .. } = &mut a {
        *accelerate = true;
    }
    let q = g.cost_plan(&a).unwrap();
    assert_eq!(q.energy, 1);
    assert_eq!(q.power.iter().map(|p| p.amount).sum::<u16>(), 2);
    g.players[0].energy = 1;
    g.players[0].energy_pool = 1;
    g.players[0].power_pool[domain_index(Domain::Mind)] = 2;
    g.apply(a).unwrap();
    assert!(g.players[0].units[0].ready);
    assert_eq!(g.players[0].energy, 0);
    assert_eq!(g.players[0].power_pool, [0; 6]);
}
#[test]
fn c18_repeat_is_paid_and_both_executions_finish_before_normal_spell_trash() {
    let mut g = position();
    pending(&mut g, 0, 133);
    no_resources(&mut g, 0);
    let mut a = finalize();
    if let Action::FinalizePendingPlay { repeat, .. } = &mut a {
        *repeat = true;
    }
    assert_eq!(g.cost_plan(&a).unwrap().energy, 2);
    assert!(g.apply(a.clone()).is_err());
    g.players[0].energy = 2;
    g.players[0].energy_pool = 2;
    g.apply(a).unwrap();
    resolve(&mut g);
    assert_eq!(g.players[0].hand, vec![72, 71]);
    assert_eq!(g.players[0].discard, vec![384, 133]);
    assert!(g.players[0].deck.is_empty());
}
#[test]
fn c18_empty_deck_neither_banishes_nor_burns_out_or_asks_for_a_play() {
    let mut g = position();
    let t = unit(&mut g, 0, 384, None);
    g.players[0].deck.clear();
    g.kill_unit(t).unwrap();
    g.after_resolution().unwrap();
    resolve(&mut g);
    assert!(g.decision.is_none());
    assert!(g.players[0].banish.is_empty());
    assert_eq!(g.players[1].score, 0);
    assert!(g.winner.is_none());
}
#[test]
fn c18_silence_does_not_prevent_banishment_but_prevents_the_optional_play() {
    let mut g = position();
    g.silenced[1] = true;
    death(&mut g, 1, 70);
    let before = format!("{g:?}");
    assert!(g.apply(Action::Choose { accept: true }).is_err());
    assert_eq!(format!("{g:?}"), before);
    assert_eq!(
        g.legal_actions().unwrap(),
        vec![Action::Choose { accept: false }]
    );
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(g.players[1].banish, vec![70]);
}
#[test]
fn c18_declined_or_failed_play_cannot_pick_a_different_identical_banish_copy() {
    let mut g = position();
    g.players[0].banish = vec![145, 71];
    g.players[0].deck = vec![72, 145];
    g.clocktower_deathknell(0);
    g.after_resolution().unwrap();
    g.apply(Action::Choose { accept: true }).unwrap();
    assert_eq!(g.players[0].banish, vec![145, 71]);
    no_resources(&mut g, 0);
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(g.players[0].banish, vec![145, 71, 145]);
    assert_eq!(g.players[0].deck, vec![72]);
}
#[test]
fn c18_stale_banish_selection_rejects_atomically_instead_of_substituting_a_card() {
    let mut g = position();
    death(&mut g, 0, 70);
    g.players[0].banish[0] = 71;
    let before = format!("{g:?}");
    assert!(g.apply(Action::Choose { accept: true }).is_err());
    assert_eq!(format!("{g:?}"), before);
}
#[test]
fn c18_bounce_and_direct_banish_do_not_trigger_deathknell() {
    for banish in [false, true] {
        let mut g = position();
        let t = unit(&mut g, 0, 384, None);
        let deck = g.players[0].deck.clone();
        if banish {
            g.banish_unit_directly(t);
        } else {
            g.bounce(t);
        }
        g.after_resolution().unwrap();
        assert_eq!(g.players[0].deck, deck);
        assert!(g.decision.is_none() && g.chain.is_empty());
    }
}
