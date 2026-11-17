//! Optional recovery and instructed play through ordinary game actions.
use super::*;
use test_support::{decline_legend_action, position, reject_unchanged, settle_unopposed_chain};

fn advance(g: &mut Game) {
    let action = if let Some(action) = decline_legend_action(g) {
        action
    } else if let Some(Decision::OrderTriggers { options, .. }) = &g.decision {
        Action::OrderTrigger {
            trigger_id: options[0].id,
        }
    } else {
        assert!(
            g.decision.is_none() && !g.chain.is_empty(),
            "{:?}",
            g.decision
        );
        Action::Pass
    };
    g.apply(action).unwrap();
}

fn await_trash(g: &mut Game, source: CardId) {
    for _ in 0..60 {
        if matches!(g.decision, Some(Decision::TrashTarget { source_card_id, .. }) if source_card_id == source)
        {
            return;
        }
        advance(g);
    }
    panic!("no trash choice for {source}");
}

fn soul(actor: usize, chosen: CardId, power: u16) -> Game {
    let mut g = position(actor, 0, 477001);
    g.players[actor].hand = vec![477];
    g.players[actor].discard = vec![8, chosen];
    g.players[actor].energy_pool = 8;
    g.players[actor].energy = 8;
    g.players[actor].power_pool = [0; 6];
    g.players[actor].power_pool[Domain::Chaos as usize] = power;
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    })
    .unwrap();
    await_trash(&mut g, 477);
    g
}

fn finalize(accelerate: bool) -> Action {
    Action::FinalizePendingPlay {
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate,
        bonus: false,
    }
}

fn await_child(g: &mut Game) {
    for _ in 0..60 {
        if matches!(
            g.decision,
            Some(Decision::PendingPlay {
                kind: InstructedPlayKind::Soulgorger,
                ..
            })
        ) {
            return;
        }
        advance(g);
    }
    panic!("no instructed child play");
}

#[test]
fn guardian_holds_before_channel_draw_and_can_recover_unit_or_gear_or_decline() {
    for actor in 0..2 {
        for selected in [None, Some(1), Some(2)] {
            let mut g = position(actor, 0, 476001);
            g.players[actor].hand = vec![476];
            g.players[actor].discard = vec![8, 470, 472];
            g.battlefield_owner[0] = Some(actor);
            g.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: Some(0),
                targets: vec![],
            })
            .unwrap();
            settle_unopposed_chain(&mut g);
            assert!(g.decision.is_none(), "Guardian has no on-play ability");
            g.apply(Action::EndTurn).unwrap();
            settle_unopposed_chain(&mut g);
            g.apply(Action::EndTurn).unwrap();
            await_trash(&mut g, 476);
            assert_eq!(g.active, actor);
            assert!(matches!(g.pending_start, Some(StartStage::Channel)));
            let draws = g.trigger_state.draws;
            let deck = g.players[actor].deck.len();
            let trash = g.players[actor].discard.clone();
            let actions = g.legal_actions().unwrap();
            assert_eq!(actions.len(), 3);
            reject_unchanged(&mut g, Action::SelectCards { indices: vec![0] });
            reject_unchanged(
                &mut g,
                Action::SelectCards {
                    indices: vec![1, 2],
                },
            );
            g.apply(Action::SelectCards {
                indices: selected.into_iter().collect(),
            })
            .unwrap();
            if let Some(index) = selected {
                assert!(g.players[actor].hand.is_empty());
                assert!(
                    matches!(g.chain.last().unwrap().ability, Some(ChainAbility::ReturnChosen { trash_index, .. }) if trash_index == index)
                );
                assert_eq!(g.trigger_state.draws, draws);
                assert_eq!(g.players[actor].deck.len(), deck);
            }
            settle_unopposed_chain(&mut g);
            assert_eq!(g.players[actor].units.len(), 1);
            assert_eq!(
                g.players[actor].hand.len(),
                1 + usize::from(selected.is_some())
            );
            if let Some(index) = selected {
                assert_eq!(g.players[actor].hand[0], trash[index]);
            } else {
                assert_eq!(g.players[actor].discard, trash);
            }
        }
    }
}

#[test]
fn soulgorger_optional_target_is_public_before_child_moves_and_power_is_still_required() {
    for actor in 0..2 {
        for extra_power in [0, 1] {
            let mut g = soul(actor, 474, 2 + extra_power);
            reject_unchanged(&mut g, Action::SelectCards { indices: vec![0] });
            assert!(
                g.legal_actions()
                    .unwrap()
                    .contains(&Action::SelectCards { indices: vec![] })
            );
            g.apply(Action::SelectCards { indices: vec![1] }).unwrap();
            assert_eq!(g.players[actor].discard, vec![8, 474]);
            assert!(g.pending_card.is_none());
            await_child(&mut g);
            assert_eq!(g.players[actor].discard, vec![8]);
            assert_eq!(g.players[actor].units.len(), 1);
            assert_eq!(g.players[actor].energy, 0);
            if extra_power == 0 {
                reject_unchanged(&mut g, finalize(false));
                g.apply(Action::Choose { accept: false }).unwrap();
                settle_unopposed_chain(&mut g);
                assert_eq!(g.players[actor].discard, vec![8, 474]);
                assert_eq!(g.players[actor].units.len(), 1);
            } else {
                g.apply(finalize(false)).unwrap();
                assert_eq!(g.players[actor].units.len(), 2);
                assert_eq!(g.players[actor].power_pool[Domain::Chaos as usize], 0);
                await_trash(&mut g, 474);
                // Its own on-play ability is real and cannot recover the spell.
                g.apply(Action::SelectCards { indices: vec![] }).unwrap();
                settle_unopposed_chain(&mut g);
                assert_eq!(g.players[actor].discard, vec![8]);
            }
        }
    }
}

#[test]
fn soulgorger_waives_base_energy_but_not_accelerate_and_allows_declining_an_eligible_unit() {
    for actor in 0..2 {
        let mut skipped = soul(actor, 96, 2);
        skipped
            .apply(Action::SelectCards { indices: vec![] })
            .unwrap();
        settle_unopposed_chain(&mut skipped);
        assert_eq!(skipped.players[actor].discard, vec![8, 96]);
        assert_eq!(skipped.players[actor].units.len(), 1);

        let mut g = soul(actor, 96, 2);
        g.apply(Action::SelectCards { indices: vec![1] }).unwrap();
        await_child(&mut g);
        reject_unchanged(&mut g, finalize(true));
        g.apply(finalize(false)).unwrap();
        settle_unopposed_chain(&mut g);
        let unit = g.players[actor]
            .units
            .iter()
            .find(|u| u.card_id == 96)
            .unwrap();
        assert!(!unit.ready);
        assert_eq!(g.players[actor].energy, 0);
    }
}

#[test]
fn soulgorger_loses_departed_target_even_when_an_identical_card_arrives() {
    for actor in 0..2 {
        let mut g = soul(actor, 470, 2);
        g.apply(Action::SelectCards { indices: vec![1] }).unwrap();
        // Exercise the central zone boundary; the identical arrival is a new object.
        let card = g.remove_from_trash(actor, 1);
        g.players[actor].discard.push(card);
        settle_unopposed_chain(&mut g);
        assert_eq!(g.players[actor].discard, vec![8, 470]);
        assert_eq!(g.players[actor].units.len(), 1);
        assert!(g.pending_card.is_none());
    }
}
