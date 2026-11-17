//! Resolving physical references keep identity, then recheck current allegiance.
//! Control changes are authored response-state boundaries. These tests do not
//! exercise a control-taking card or the eventual transfer operation.
use super::*;
use crate::game::test_support::{
    position, reject_unchanged, settle_unopposed_chain as finish_chain,
    transfer_control_for_fixture as transfer,
};

#[test]
fn finalized_spells_and_play_triggers_recheck_current_target_allegiance() {
    for actor in 0..2 {
        for card in [19, 55, 144] {
            let mut g = position(actor, 0, 468201);
            let old = if card == 19 { actor } else { 1 - actor };
            let id = g.create_unit(old, 69, false, 0);
            let target = Target::Unit {
                player: old,
                unit_id: id,
            };
            g.unit_mut(target).unwrap().lane = Some(0);
            g.battlefield_owner[0] = Some(old);
            g.players[actor].hand = vec![card];
            g.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: if card == 144 { vec![] } else { vec![target] },
            })
            .unwrap();
            if card == 144 {
                assert!(matches!(
                    g.decision,
                    Some(Decision::OnPlayTarget { card_id: 144, .. })
                ));
                g.apply(Action::ChooseTarget {
                    target: Some(target),
                })
                .unwrap();
            }
            let current = transfer(&mut g, id);
            let mut clone = g.clone();
            finish_chain(&mut g);
            finish_chain(&mut clone);
            assert_eq!(format!("{g:?}"), format!("{clone:?}"));
            let unit = g.unit_ref(current).unwrap();
            // Ray chooses any unit; Rex's enemy-only and Shroud's friendly-only
            // targets become invalid. Void Gate adds one to Ray's three damage.
            assert_eq!(unit.damage, if card == 55 { 4 } else { 0 }, "card {card}");
            assert_eq!(unit.temporary_buff, 0);
            assert!(!unit.protected);
            assert_eq!(g.board_owner(current), Some(old));
        }
    }
}

#[test]
fn new_submissions_reject_wrong_controller_and_departed_targets_do_not_rebind() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 468202);
        let old = 1 - actor;
        let id = g.create_unit(old, 69, false, 0);
        let target = Target::Unit {
            player: old,
            unit_id: id,
        };
        g.unit_mut(target).unwrap().lane = Some(0);
        g.battlefield_owner[0] = Some(old);
        g.players[actor].hand = vec![55];
        reject_unchanged(
            &mut g,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![Target::Unit {
                    player: actor,
                    unit_id: id,
                }],
            },
        );
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        })
        .unwrap();
        assert_eq!(g.return_unit_to_hand(target), Some(old));
        let new_id = g.create_unit(old, 69, false, 0);
        let new_target = Target::Unit {
            player: old,
            unit_id: new_id,
        };
        g.unit_mut(new_target).unwrap().lane = Some(0);
        assert_ne!(new_id, id);
        finish_chain(&mut g);
        assert_eq!(g.unit_ref(new_target).unwrap().damage, 0);
    }
}

#[test]
fn pending_temporary_follows_the_physical_object_but_not_a_replayed_copy() {
    use crate::empower::BoardObject;
    for owner in 0..2 {
        for replace in [false, true] {
            let mut g = position(owner, 0, 468203);
            let id = g.create_unit(owner, 69, false, 0);
            let target = Target::Unit {
                player: owner,
                unit_id: id,
            };
            g.grant_temporary(target);
            g.temporary_beginning(owner);
            g.after_resolution().unwrap();
            assert!(g.chain.iter().any(|i| matches!(i.ability, Some(ChainAbility::Temporary { source: BoardObject::Unit { unit_id, .. }, .. }) if unit_id == id)));
            if replace {
                g.return_unit_to_hand(target);
                let fresh = g.create_unit(owner, 69, false, 0);
                finish_chain(&mut g);
                assert!(g.unit_controller(fresh).is_some());
                assert!(g.players[owner].discard.is_empty());
            } else {
                transfer(&mut g, id);
                finish_chain(&mut g);
                assert_eq!(g.unit_controller(id), None);
                assert_eq!(g.players[owner].discard, [69]);
                assert!(g.players[1 - owner].discard.is_empty());
            }
        }
    }
}

#[test]
fn pending_irelia_self_bonus_survives_source_control_change() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 468204);
        let id = g.create_unit(actor, 0, false, 0);
        let target = Target::Unit {
            player: actor,
            unit_id: id,
        };
        g.players[actor].hand = vec![19];
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        })
        .unwrap();
        let current = transfer(&mut g, id);
        finish_chain(&mut g);
        // Irelia's triggered self bonus survives; Twilight Shroud's formerly
        // friendly target no longer qualifies, so it gives no Might/protection.
        assert_eq!(g.unit_ref(current).unwrap().temporary_buff, 1);
        assert!(!g.unit_ref(current).unwrap().protected);
    }
}

#[test]
fn finalized_activations_follow_the_source_without_transferring_the_ability() {
    for actor in 0..2 {
        for card in [345, 267, 238] {
            let mut g = position(actor, 0, 468205);
            let id = g.create_unit(actor, card, false, 0);
            let source = AbilitySource::Unit {
                player: actor,
                unit_id: id,
            };
            let original = Target::Unit {
                player: actor,
                unit_id: id,
            };
            g.unit_mut(original).unwrap().lane = (card == 238).then_some(0);
            g.unit_mut(original).unwrap().ready = true;
            g.players[actor].xp = 10;
            if card == 267 {
                g.legend[actor] = crate::legends::LegendState::from_legend(7).unwrap();
            }
            let opposing_xp = g.players[1 - actor].xp;
            let victim = g.create_unit(1 - actor, 69, false, 0);
            let target = Target::Unit {
                player: 1 - actor,
                unit_id: victim,
            };
            g.unit_mut(target).unwrap().lane = Some(1);
            g.unit_mut(target).unwrap().temporary_buff = 30;
            let targets = if card == 238 { vec![target] } else { vec![] };
            reject_unchanged(
                &mut g,
                Action::ActivateAbility {
                    source: AbilitySource::Unit {
                        player: 1 - actor,
                        unit_id: id,
                    },
                    ability_index: 0,
                    targets: targets.clone(),
                },
            );
            g.apply(Action::ActivateAbility {
                source,
                ability_index: 0,
                targets,
            })
            .unwrap();
            assert!(g.chain.iter().any(|i| i.player == actor && matches!(i.ability, Some(ChainAbility::Activated { source: saved, .. }) if saved == source)));
            let current = transfer(&mut g, id);
            // The ability reads current source Might, not a copied activation value.
            g.unit_mut(current).unwrap().temporary_buff = 2;
            let might = g.unit_might(g.unit_ref(current).unwrap()).unwrap();
            finish_chain(&mut g);
            match card {
                345 => assert!(g.buffed_units.contains(&id)),
                267 => {
                    assert!(g.unit_empowered(id));
                    assert!(g.legend[actor].empowered, "the original actor Empowers");
                    assert!(!g.legend[1 - actor].empowered);
                }
                238 => assert_eq!(g.unit_ref(target).unwrap().damage, might + 1),
                _ => unreachable!(),
            }
            assert_eq!(g.board_owner(current), Some(actor));
            assert_eq!(g.players[1 - actor].xp, opposing_xp, "card {card}");
        }
    }
}

#[test]
fn irelias_referential_ready_tracks_control_but_never_a_new_copy() {
    for actor in 0..2 {
        for transfer_before_cost in [false, true] {
            for departed in [false, true] {
                let mut g = position(actor, 0, 468206);
                let id = g.create_unit(actor, 71, false, 0);
                let original = Target::Unit {
                    player: actor,
                    unit_id: id,
                };
                g.legend[actor].ready = true;
                g.players[actor].hand = vec![19];
                g.apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![original],
                })
                .unwrap();
                assert!(matches!(g.decision, Some(Decision::Legend { .. })));
                let mut current = original;
                if transfer_before_cost {
                    current = transfer(&mut g, id);
                }
                if departed && transfer_before_cost {
                    g.return_unit_to_hand(current);
                }
                g.apply(Action::Choose { accept: true }).unwrap();
                assert!(!g.legend[actor].ready);
                let paid = g.players[actor].power_pool;
                if !transfer_before_cost {
                    current = transfer(&mut g, id);
                    if departed {
                        g.return_unit_to_hand(current);
                    }
                }
                let fresh = departed.then(|| g.create_unit(actor, 71, false, 0));
                finish_chain(&mut g);
                assert_eq!(g.players[actor].power_pool, paid);
                if let Some(fresh) = fresh {
                    assert_ne!(fresh, id);
                    assert!(
                        !g.unit_ref(Target::Unit {
                            player: actor,
                            unit_id: fresh
                        })
                        .unwrap()
                        .ready
                    );
                } else {
                    assert!(g.unit_ref(current).unwrap().ready);
                    assert!(!g.unit_ref(current).unwrap().protected);
                }
            }
        }
    }
}

#[test]
fn targeted_legend_abilities_recheck_protection_after_control_changes() {
    for actor in 0..2 {
        for legend in [2, 7] {
            for protected in [false, true] {
                let mut g = position(actor, 0, 468207);
                g.legend[actor] = crate::legends::LegendState::from_legend(legend).unwrap();
                g.legend[actor].empowered = true;
                let id = g.create_unit(actor, 71, false, 0);
                let target = Target::Unit {
                    player: actor,
                    unit_id: id,
                };
                g.apply(Action::ActivateLegend {
                    target: Some(target),
                })
                .unwrap();
                let current = transfer(&mut g, id);
                g.unit_mut(current).unwrap().protected = protected;
                finish_chain(&mut g);
                let unit = g.unit_ref(current).unwrap();
                if legend == 7 {
                    assert_eq!(unit.ready, !protected);
                } else {
                    assert_eq!(
                        g.unit_keywords(unit).unwrap().assault,
                        if protected { 0 } else { 2 }
                    );
                }
            }
        }
    }
}

#[test]
fn legacy_empower_and_delayed_disempower_keep_physical_identity() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 468208);
        let id = g.create_unit(actor, 24, false, 0);
        g.legend[actor] = crate::legends::LegendState::from_legend(7).unwrap();
        g.apply(Action::EmpowerUnit {
            unit_id: id,
            pay_power: false,
        })
        .unwrap();
        let current = transfer(&mut g, id);
        finish_chain(&mut g);
        assert!(g.unit_empowered(id));
        assert!(g.legend[actor].empowered);
        // Authored delayed-effect setup; EndTurn queues it through the normal
        // end-of-turn dispatcher, then the same physical unit changes back.
        g.temporary_empower.push((actor, 1 - actor, Some(id)));
        g.apply(Action::EndTurn).unwrap();
        assert!(g.chain.iter().any(|i| matches!(i.ability, Some(ChainAbility::Disempower { object_id: Some(object), .. }) if object == id)));
        assert_eq!(g.board_owner(current), Some(actor));
        transfer(&mut g, id);
        finish_chain(&mut g);
        assert!(!g.unit_empowered(id));
        assert!(g.legend[actor].empowered);
    }
}

#[test]
fn vex_pending_stun_follows_the_played_unit() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 468209);
        let vex = g.create_unit(1 - actor, 20, false, 0);
        g.unit_mut(Target::Unit {
            player: 1 - actor,
            unit_id: vex,
        })
        .unwrap()
        .lane = Some(0);
        g.battlefield_owner[0] = Some(1 - actor);
        g.players[actor].hand = vec![71];
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        })
        .unwrap();
        let id = g.players[actor].units[0].id;
        assert!(g.chain.iter().any(|i| matches!(i.ability, Some(ChainAbility::Triggered(TriggeredAbility::Stun { unit_id, .. })) if unit_id == id)));
        let current = transfer(&mut g, id);
        finish_chain(&mut g);
        assert!(g.unit_ref(current).unwrap().stunned);
        assert!(g.movement_locks.contains(&(actor, id)));
        assert!(g.movement_prevented(actor, 1 - actor, id));
        assert!(!g.movement_prevented(1 - actor, 1 - actor, id));
    }
}

#[test]
fn wild_claws_linked_empower_keeps_its_original_actor_after_control_changes() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 468210);
        g.legend[actor] = crate::legends::LegendState::from_legend(7).unwrap();
        g.players[actor].hand = vec![388];
        g.players[actor].deck = vec![71];
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        })
        .unwrap();
        for _ in 0..30 {
            if g.decision.is_some() {
                break;
            }
            g.apply(Action::Pass).unwrap();
        }
        let Some(Decision::DeckLookPlay { choice_id, .. }) = g.decision else {
            panic!("missing deck choice");
        };
        g.apply(Action::ResolveDeckLook {
            choice_id,
            index: Some(0),
        })
        .unwrap();
        g.apply(Action::FinalizePendingPlay {
            lane: None,
            targets: vec![],
            repeat_targets: vec![],
            repeat: false,
            accelerate: false,
            bonus: false,
        })
        .unwrap();
        let Some(Decision::EmpowerPlayed {
            target: Some(Target::Unit { unit_id, .. }),
            ..
        }) = g.decision
        else {
            panic!("missing linked Empower");
        };
        g.apply(Action::Choose { accept: true }).unwrap();
        transfer(&mut g, unit_id);
        finish_chain(&mut g);
        assert!(g.unit_empowered(unit_id));
        assert!(g.legend[actor].empowered);
        assert!(!g.legend[1 - actor].empowered);
    }
}
