//! CR831 summed attack trigger, 359.3.e/f live targeting and 383 ordering.
use super::*;
use test_support::{position, ready_unit, reject_unchanged};

fn unit(g: &mut Game, owner: usize, card: CardId, lane: Option<usize>) -> Target {
    let id = ready_unit(g, owner, card);
    let target = Target::Unit {
        player: owner,
        unit_id: id,
    };
    g.unit_mut(target).unwrap().lane = lane;
    target
}

fn begin(actor: usize, card: CardId) -> (Game, Target, Target, Target) {
    let mut g = position(actor, 0, 458001);
    let attacker = unit(&mut g, actor, card, None);
    let grant = unit(&mut g, actor, 459, Some(1));
    let defender = unit(&mut g, 1 - actor, 110, Some(0));
    g.battlefield_owner = vec![Some(1 - actor), Some(actor)];
    let Target::Unit { unit_id, .. } = attacker else {
        unreachable!()
    };
    g.apply(Action::Move {
        unit_id,
        lane: Some(0),
    })
    .unwrap();
    assert!(
        matches!(
            g.decision,
            Some(Decision::TriggerTargets {
                context: TriggerContext {
                    keyword: Some(KeywordTrigger::Disarm { .. }),
                    ..
                },
                ..
            })
        ),
        "{:?}",
        g.decision
    );
    (g, attacker, grant, defender)
}

fn settle(g: &mut Game) {
    for _ in 0..30 {
        if let Some(action) = test_support::decline_legend_action(g) {
            g.apply(action).unwrap();
            continue;
        }
        if g.chain.is_empty() {
            return;
        }
        assert!(g.decision.is_none(), "{:?}", g.decision);
        g.apply(Action::Pass).unwrap();
    }
    panic!("Disarm chain did not settle");
}

fn cast(g: &mut Game, card: CardId, target: Target) {
    let player = g.priority();
    let hand_index = g.players[player].hand.len();
    g.players[player].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets: vec![target],
    })
    .unwrap();
}

#[test]
fn disarm_grants_sum_on_friendly_mechs_including_self_and_tokens() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 458002);
        let a = unit(&mut g, actor, 459, None);
        let b = unit(&mut g, actor, 459, Some(1));
        let peddler = unit(&mut g, actor, 458, None);
        let ordinary = unit(&mut g, actor, 71, None);
        let enemy = unit(&mut g, 1 - actor, 459, None);
        let token_id = g.create_unit(actor, 54, true, 3);
        let token = Target::Unit {
            player: actor,
            unit_id: token_id,
        };
        for (target, expected) in [
            (a, 2),
            (b, 2),
            (token, 2),
            (peddler, 1),
            (ordinary, 0),
            (enemy, 1),
        ] {
            assert_eq!(
                g.unit_keywords(g.unit_ref(target).unwrap()).unwrap().disarm,
                expected
            );
        }
    }
}

#[test]
fn disarm_one_trigger_with_summed_value_and_live_source_relative_targets() {
    for actor in 0..2 {
        for card in [458, 459, 460] {
            let (mut g, attacker, _, defender) = begin(actor, card);
            for targets in [
                vec![],
                vec![attacker],
                vec![Target::Unit {
                    player: 1 - actor,
                    unit_id: u32::MAX,
                }],
            ] {
                reject_unchanged(&mut g, Action::ChooseTriggerTargets { targets });
            }
            g.apply(Action::ChooseTriggerTargets {
                targets: vec![defender],
            })
            .unwrap();
            assert_eq!(g.chain.len(), 1, "one summed trigger, not one per grant");
            let expected = if card == 459 { 2 } else { 1 };
            assert!(
                g.chain_name(&g.chain[0])
                    .contains(&format!("Disarm {expected}"))
            );
            assert_eq!(
                g.unit_ref(defender).unwrap().temporary_buff,
                0,
                "responses precede reduction"
            );
            let mut copied = g.clone();
            settle(&mut g);
            settle(&mut copied);
            assert_eq!(format!("{g:?}"), format!("{copied:?}"));
            assert_eq!(g.unit_ref(defender).unwrap().temporary_buff, -expected);
        }
    }
}

#[test]
fn disarm_grant_loss_does_not_change_trigger_but_source_or_target_departure_mistargets() {
    for actor in 0..2 {
        for removed in [0, 1, 2] {
            let (mut g, attacker, grant, defender) = begin(actor, 459);
            g.apply(Action::ChooseTriggerTargets {
                targets: vec![defender],
            })
            .unwrap();
            cast(&mut g, 11, [grant, attacker, defender][removed]);
            settle(&mut g);
            if removed == 2 {
                assert!(g.unit_ref(defender).is_none());
            } else {
                assert_eq!(
                    g.unit_ref(defender).unwrap().temporary_buff,
                    if removed == 0 { -2 } else { 0 }
                );
            }
            if removed == 0 {
                assert_eq!(
                    g.unit_keywords(g.unit_ref(attacker).unwrap())
                        .unwrap()
                        .disarm,
                    1
                );
            }
        }
    }
}

#[test]
fn disarm_moving_source_to_base_mistargets_and_counter_prevents_reduction() {
    for actor in 0..2 {
        for counter in [false, true] {
            let (mut g, attacker, _, defender) = begin(actor, 458);
            g.apply(Action::ChooseTriggerTargets {
                targets: vec![defender],
            })
            .unwrap();
            if counter {
                let chain_id = g.chain[0].id;
                g.apply(Action::Pass).unwrap();
                let hand_index = g.players[1 - actor].hand.len();
                g.players[1 - actor].hand.push(442); // Repulse can counter abilities.
                g.apply(Action::PlayCardWithTargets {
                    hand_index,
                    lane: None,
                    targets: vec![defender, Target::Spell { chain_id }],
                })
                .unwrap();
            } else {
                cast(&mut g, 9, attacker);
            }
            settle(&mut g);
            assert_eq!(g.unit_ref(defender).unwrap().temporary_buff, 0);
        }
    }
}

#[test]
fn disarm_mech_token_has_its_own_named_trigger_at_every_ai_level() {
    for actor in 0..2 {
        for level in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let mut g = position(actor, 0, 458003);
            g.difficulty = level;
            unit(&mut g, actor, 459, None);
            let defender = unit(&mut g, 1 - actor, 110, Some(0));
            g.battlefield_owner[0] = Some(1 - actor);
            let unit_id = g.create_unit(actor, 54, true, 3);
            g.players[actor].units.last_mut().unwrap().ready = true;
            g.apply(Action::Move {
                unit_id,
                lane: Some(0),
            })
            .unwrap();
            let action = g.choose_ai_action().unwrap();
            assert_eq!(
                action,
                Action::ChooseTriggerTargets {
                    targets: vec![defender]
                }
            );
            g.apply(action).unwrap();
            assert_eq!(g.chain_name(&g.chain[0]), "Mech · Disarm 1");
            assert_eq!(
                g.chain_source(&g.chain[0]),
                ChainSource::Unit {
                    player: actor,
                    unit_id
                }
            );
            assert!(
                matches!(g.chain[0].ability, Some(ChainAbility::CardTrigger {
                context: TriggerContext { source: TriggerSource { unit_id: Some(id), .. }, .. }, ..
            }) if id == unit_id)
            );
            settle(&mut g);
            assert_eq!(g.unit_ref(defender).unwrap().temporary_buff, -1);
        }
    }
}

#[test]
fn disarm_obeys_deflect_and_never_reduces_an_untargetable_defender() {
    for actor in 0..2 {
        for protected in [false, true] {
            let mut g = position(actor, 0, 458004);
            let attacker = unit(&mut g, actor, 458, None);
            let defender = unit(&mut g, 1 - actor, 0, Some(0));
            g.unit_mut(defender).unwrap().protected = protected;
            g.battlefield_owner[0] = Some(1 - actor);
            let Target::Unit { unit_id, .. } = attacker else {
                unreachable!()
            };
            g.apply(Action::Move {
                unit_id,
                lane: Some(0),
            })
            .unwrap();
            if protected {
                assert!(g.decision.is_none());
                assert!(g.chain.is_empty());
            } else {
                let before = g.players[actor]
                    .power_pool
                    .iter()
                    .map(|n| u64::from(*n))
                    .sum::<u64>();
                g.apply(Action::ChooseTriggerTargets {
                    targets: vec![defender],
                })
                .unwrap();
                let after = g.players[actor]
                    .power_pool
                    .iter()
                    .map(|n| u64::from(*n))
                    .sum::<u64>();
                assert_eq!(after, before - 1);
                settle(&mut g);
            }
            assert_eq!(
                g.unit_ref(defender).unwrap().temporary_buff,
                if protected { 0 } else { -1 }
            );
        }
    }
}

#[test]
fn disarm_can_reduce_might_below_zero_and_expires_at_turn_end() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 458005);
        let attacker = unit(&mut g, actor, 459, None);
        for _ in 0..3 {
            unit(&mut g, actor, 459, None);
        }
        let defender = unit(&mut g, 1 - actor, 110, Some(0));
        g.battlefield_owner[0] = Some(1 - actor);
        let Target::Unit { unit_id, .. } = attacker else {
            unreachable!()
        };
        g.apply(Action::Move {
            unit_id,
            lane: Some(0),
        })
        .unwrap();
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![defender],
        })
        .unwrap();
        settle(&mut g);
        let stats = g.try_unit_stats(g.unit_ref(defender).unwrap()).unwrap();
        assert_eq!((stats.signed_might, stats.might), (-1, 0));
        assert_eq!(g.unit_ref(defender).unwrap().temporary_buff, -4);
        // Move the attacker out through an actual reaction to leave the
        // reduced defender alive, then finish the turn normally.
        cast(&mut g, 9, attacker);
        settle(&mut g);
        // The response chain ending does not itself close the showdown.
        g.apply(Action::Pass).unwrap();
        g.apply(Action::Pass).unwrap();
        assert!(!g.in_window());
        g.apply(Action::EndTurn).unwrap();
        assert_eq!(g.unit_ref(defender).unwrap().temporary_buff, 0);
        assert_eq!(g.unit_might(g.unit_ref(defender).unwrap()).unwrap(), 2);
    }
}
