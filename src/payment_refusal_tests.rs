//! Core445 positive finalization refusal, using actual card rules in controlled positions.
use super::*;
use crate::payment_plan::PaymentRefusalKind;

fn position(player: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 25025, Difficulty::Tactical);
    g.choose_starting_player(player).unwrap();
    g.start_match().unwrap();
    g.battlefields = vec![3, 3];
    g.battlefield_owner = vec![None; 2];
    for p in &mut g.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.deck = vec![71; 6];
        p.set_rune_setup([2; 6], [0; 6]).unwrap();
        p.energy_pool = 20;
        p.energy = 32;
        p.power_pool = [5; 6];
        p.any_power = 0;
    }
    for l in &mut g.legend {
        l.ready = false;
    }
    g
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
fn begin(g: &mut Game, action: Action) {
    g.apply(Action::BeginPayment {
        action: Box::new(action),
    })
    .unwrap();
}
fn add(g: &mut Game) {
    g.apply(Action::ActivateRune {
        domain: Domain::Mind,
        recycle: false,
    })
    .unwrap();
}
fn child_action(accelerate: bool) -> Action {
    Action::FinalizePendingPlay {
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate,
        bonus: false,
    }
}
fn cast(g: &mut Game, card: CardId, targets: Vec<Target>) {
    let p = g.priority();
    g.players[p].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index: g.players[p].hand.len() - 1,
        lane: None,
        targets,
    })
    .unwrap();
    settle(g);
}
fn clocktower_child(g: &mut Game, player: usize) {
    let id = g.create_unit(player, 384, false, 0);
    g.players[player].deck = vec![71, 109];
    g.players[player].banish = vec![109, 71, 109];
    cast(
        g,
        294,
        vec![Target::Unit {
            player,
            unit_id: id,
        }],
    );
    assert!(matches!(
        g.decision,
        Some(Decision::BanishPlay { card_id: 109, .. })
    ));
    g.apply(Action::Choose { accept: true }).unwrap();
    assert!(matches!(
        g.decision,
        Some(Decision::PendingPlay { card_id: 109, .. })
    ));
}

#[test]
fn c25_positive_funded_ordinary_undo_and_refusal_restore_exact_attempt() {
    for player in 0..2 {
        for card in [9, 10, 15] {
            for refuse in [false, true] {
                let mut g = position(player);
                g.players[player].hand = vec![71, card, 71];
                let before = format!("{g:?}");
                begin(
                    &mut g,
                    Action::PlayCardWithTargets {
                        hand_index: 1,
                        lane: None,
                        targets: vec![],
                    },
                );
                assert_eq!(
                    g.payment_plan_view(player).unwrap().refusal_kind,
                    Some(PaymentRefusalKind::Ordinary)
                );
                add(&mut g);
                assert!(g.can_cancel_payment().unwrap());
                assert!(g.can_refuse_payment().unwrap());
                g.apply(if refuse {
                    Action::RefusePayment
                } else {
                    Action::Choose { accept: false }
                })
                .unwrap();
                assert_eq!(
                    format!("{g:?}"),
                    before,
                    "seat{player} card{card} refuse{refuse}"
                );
            }
        }
    }
}

#[test]
fn c25_clocktower_refusal_preserves_parent_and_exact_banish_occurrence_after_direct_add() {
    for player in 0..2 {
        for direct in [false, true] {
            let mut g = position(player);
            clocktower_child(&mut g, player);
            let original = g.clone();
            let pending_id = g.pending_card.as_ref().unwrap().id;
            if direct {
                add(&mut g);
            }
            let before_plan = format!("{g:?}");
            begin(&mut g, child_action(false));
            add(&mut g);
            assert_eq!(
                g.payment_plan_view(player).unwrap().refusal_kind,
                Some(PaymentRefusalKind::PendingCard)
            );
            g.apply(Action::Choose { accept: false }).unwrap();
            assert_eq!(
                format!("{g:?}"),
                before_plan,
                "Undo returns the same pending choice"
            );
            begin(&mut g, child_action(false));
            g.apply(Action::RefusePayment).unwrap();
            assert_eq!(g.players[player].banish, vec![109, 71, 109, 109]);
            assert_eq!(g.players[player].discard, original.players[player].discard);
            assert!(g.players[player].discard.contains(&384));
            assert!(g.players[player].discard.contains(&294));
            assert_eq!(g.players[player].deck, original.players[player].deck);
            assert_eq!(g.players[player].energy, original.players[player].energy);
            assert_eq!(
                g.players[player].exhausted_runes,
                original.players[player].exhausted_runes
            );
            assert!(!g.players[player].units.iter().any(|u| u.card_id == 109));
            assert!(!g.pending_cards().iter().any(|p| p.id == pending_id));
            assert!(g.finalization_payment_origin.is_none());
            assert!(g.payment_plan.is_none());
        }
    }
}

#[test]
fn c25_zero_child_cannot_refuse_but_selected_positive_accelerate_can() {
    for player in 0..2 {
        for accelerate in [false, true] {
            let mut g = position(player);
            let id = g.create_unit(player, 145, false, 0);
            cast(
                &mut g,
                381,
                vec![Target::Unit {
                    player,
                    unit_id: id,
                }],
            );
            assert!(matches!(g.decision, Some(Decision::PendingPlay { .. })));
            begin(&mut g, child_action(accelerate));
            if accelerate {
                assert!(g.can_refuse_payment().unwrap());
                g.apply(Action::RefusePayment).unwrap();
                assert_eq!(g.players[player].banish, vec![145]);
                assert!(!g.players[player].units.iter().any(|u| u.card_id == 145));
            } else {
                assert!(!g.can_refuse_payment().unwrap());
                assert!(!g.can_cancel_payment().unwrap());
                let before = format!("{g:?}");
                assert!(g.apply(Action::RefusePayment).is_err());
                assert_eq!(format!("{g:?}"), before);
                g.apply(Action::CompletePayment).unwrap();
                assert!(
                    g.players[player]
                        .units
                        .iter()
                        .any(|u| u.card_id == 145 && u.lane.is_none())
                );
            }
            assert!(
                g.players[player].discard.contains(&381),
                "parent is never undone"
            );
        }
    }
}

#[test]
fn c25_paid_token_refusal_consumes_one_occurrence_and_resumes_siblings() {
    for player in 0..2 {
        let mut g = position(player);
        g.create_unit(player, 265, false, 0);
        cast(&mut g, 87, vec![]);
        assert!(matches!(
            g.decision,
            Some(Decision::TokenPlay { remaining: 4, .. })
        ));
        let first_id = g.pending_token.as_ref().unwrap().id;
        begin(
            &mut g,
            Action::FinalizeTokenPlay {
                lane: None,
                accelerate: true,
            },
        );
        assert_eq!(
            g.payment_plan_view(player).unwrap().refusal_kind,
            Some(PaymentRefusalKind::PendingToken)
        );
        g.apply(Action::RefusePayment).unwrap();
        assert!(matches!(
            g.decision,
            Some(Decision::TokenPlay { remaining: 3, .. })
        ));
        assert_ne!(g.pending_token.as_ref().unwrap().id, first_id);
        assert!(!g.players[player].units.iter().any(|u| u.token));
        assert!(g.players[player].discard.contains(&87));
        for _ in 0..3 {
            g.apply(Action::FinalizeTokenPlay {
                lane: None,
                accelerate: false,
            })
            .unwrap();
        }
        assert_eq!(
            g.players[player].units.iter().filter(|u| u.token).count(),
            3
        );
    }
}

#[test]
fn c25_irelia_and_temple_direct_add_is_undone_when_declining_finalization() {
    for player in 0..2 {
        for temple in [false, true] {
            for explicit in [false, true] {
                let mut g = position(player);
                g.battlefields = vec![0, 3];
                g.legend[player].kind = crate::legends::LegendKind::Irelia;
                g.decision = Some(if temple {
                    Decision::Battlefield { player, lane: 0 }
                } else {
                    Decision::Legend {
                        player,
                        trigger: LegendTrigger::IreliaConquer,
                    }
                });
                let before = g.players[player].clone();
                add(&mut g);
                if explicit {
                    begin(&mut g, Action::Choose { accept: true });
                    assert_eq!(
                        g.payment_plan_view(player).unwrap().refusal_kind,
                        Some(PaymentRefusalKind::Trigger)
                    );
                    g.apply(Action::RefusePayment).unwrap();
                } else {
                    g.apply(Action::Choose { accept: false }).unwrap();
                }
                assert_eq!(g.players[player], before);
                assert!(g.chain.is_empty());
                assert!(g.decision.is_none());
                assert!(g.finalization_payment_origin.is_none());
            }
        }
    }
}

#[test]
fn c25_paid_activation_refusal_restores_source_and_never_resolves() {
    for player in 0..2 {
        let mut g = position(player);
        let target = g.create_unit(player, 71, false, 0);
        g.gear_play(player, 385, None).unwrap();
        let gear_id = g.players[player].gear.last().unwrap().id;
        let before = format!("{g:?}");
        begin(
            &mut g,
            Action::ActivateAbility {
                source: AbilitySource::Gear { player, gear_id },
                ability_index: 0,
                targets: vec![Target::Unit {
                    player,
                    unit_id: target,
                }],
            },
        );
        add(&mut g);
        g.apply(Action::RefusePayment).unwrap();
        assert_eq!(format!("{g:?}"), before);
    }
}

#[test]
fn c25_xp_only_activation_and_resolving_counter_tax_are_not_resource_finalization_refusals() {
    let mut g = position(0);
    let unit_id = g.create_unit(0, 345, false, 0);
    g.players[0].xp = 5;
    begin(
        &mut g,
        Action::ActivateAbility {
            source: AbilitySource::Unit { player: 0, unit_id },
            ability_index: 0,
            targets: vec![],
        },
    );
    assert_eq!(g.payment_plan_view(0).unwrap().refusal_kind, None);
    assert!(!g.can_refuse_payment().unwrap());
    let before = format!("{g:?}");
    assert!(g.apply(Action::RefusePayment).is_err());
    assert_eq!(format!("{g:?}"), before);
    let mut g = position(0);
    g.decision = Some(Decision::CounterTax {
        player: 0,
        chain_id: 999,
        cost: 2,
    });
    begin(&mut g, Action::Choose { accept: true });
    assert_eq!(g.payment_plan_view(0).unwrap().refusal_kind, None);
    assert!(!g.can_refuse_payment().unwrap());
    let before = format!("{g:?}");
    assert!(g.apply(Action::RefusePayment).is_err());
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn c25_nested_add_must_finish_before_refusal_and_then_rolls_back() {
    for player in 0..2 {
        for direct in [false, true] {
            let mut g = position(player);
            clocktower_child(&mut g, player);
            g.gear_play(player, 244, None).unwrap();
            let gear_id = g.players[player].gear.last().unwrap().id;
            let before = g.players[player].clone();
            if !direct {
                begin(&mut g, child_action(false));
            }
            g.apply(Action::ActivateAbility {
                source: AbilitySource::Gear { player, gear_id },
                ability_index: 0,
                targets: vec![],
            })
            .unwrap();
            assert!(matches!(
                g.decision,
                Some(Decision::ResourceConversion { .. })
            ));
            let during = format!("{g:?}");
            assert!(!g.can_refuse_payment().unwrap());
            assert!(g.apply(Action::RefusePayment).is_err());
            assert_eq!(format!("{g:?}"), during);
            g.apply(Action::ResolveConversion { amount: 2 }).unwrap();
            if direct {
                begin(&mut g, child_action(false));
            }
            g.apply(Action::RefusePayment).unwrap();
            let mut expected = before;
            expected.banish.push(109);
            assert_eq!(g.players[player], expected);
            assert!(g.resource_frames.is_empty());
            assert!(g.finalization_payment_origin.is_none());
        }
    }
}

#[test]
fn c25_irelia_chosen_trigger_refusal_does_not_ready_unit_or_exhaust_legend() {
    for player in 0..2 {
        let mut g = position(player);
        let unit_id = g.create_unit(player, 71, false, 0);
        g.legend[player].ready = true;
        g.legend[player].kind = crate::legends::LegendKind::Irelia;
        g.decision = Some(Decision::Legend {
            player,
            trigger: LegendTrigger::IreliaChoose { unit_id },
        });
        let before = g.players[player].clone();
        add(&mut g);
        begin(&mut g, Action::Choose { accept: true });
        g.apply(Action::RefusePayment).unwrap();
        assert_eq!(g.players[player], before);
        assert!(g.legend[player].ready);
        assert!(g.chain.is_empty());
        assert!(g.decision.is_none());
    }
}

#[test]
fn c25_refused_token_returns_reserved_next_unit_grant() {
    for player in 0..2 {
        let mut g = position(player);
        let grant = g.grant_play(
            player,
            ChainSource::Battlefield {
                lane: 0,
                battlefield_id: 5,
            },
            PlayGrantKind::AccelerateNextUnit,
        );
        g.queue_token_play(player, 62, 1, Some(None), false);
        g.after_resolution().unwrap();
        assert!(matches!(g.decision, Some(Decision::TokenPlay { .. })));
        assert!(!g.play_grants.iter().any(|g| g.id == grant));
        begin(
            &mut g,
            Action::FinalizeTokenPlay {
                lane: None,
                accelerate: true,
            },
        );
        g.apply(Action::RefusePayment).unwrap();
        assert!(g.pending_token.is_none());
        assert!(g.play_grants.iter().any(|g| g.id == grant));
        assert!(!g.players[player].units.iter().any(|u| u.token));
    }
}

#[test]
fn c25_successful_finalization_clears_direct_origin_before_later_refusal() {
    for player in 0..2 {
        let mut g = position(player);
        g.legend[player].kind = crate::legends::LegendKind::Irelia;
        g.decision = Some(Decision::Legend {
            player,
            trigger: LegendTrigger::IreliaConquer,
        });
        add(&mut g);
        g.apply(Action::Choose { accept: true }).unwrap();
        assert!(g.finalization_payment_origin.is_none());
        settle(&mut g);
        let after = g.players[player].clone();
        g.decision = Some(Decision::Legend {
            player,
            trigger: LegendTrigger::IreliaConquer,
        });
        begin(&mut g, Action::Choose { accept: true });
        g.apply(Action::RefusePayment).unwrap();
        assert_eq!(
            g.players[player], after,
            "later refusal cannot undo completed earlier Add"
        );
    }
}

#[test]
fn c25_last_rites_restores_exact_trash_flow_indices_and_reserved_grant() {
    for player in 0..2 {
        let mut g = position(player);
        g.players[player].discard = vec![71, 109, 133, 109];
        g.temporary_flow[player] = vec![2];
        let unit_id = g.create_unit(player, 71, false, 0);
        g.unit_mut(Target::Unit { player, unit_id }).unwrap().ready = true;
        g.gear_play(player, 38, None).unwrap();
        let gear_id = g.players[player].gear.last().unwrap().id;
        g.gear_resolve_equip(player, gear_id, Target::Unit { player, unit_id });
        let grant = g.grant_play(
            player,
            ChainSource::Card { card_id: 172 },
            PlayGrantKind::ReadyNextUnit,
        );
        g.apply(Action::Move {
            unit_id,
            lane: Some(0),
        })
        .unwrap();
        g.apply(Action::Pass).unwrap();
        g.apply(Action::Pass).unwrap();
        assert!(matches!(g.decision, Some(Decision::OptionalPlay { .. })));
        g.apply(Action::Choose { accept: true }).unwrap();
        g.apply(Action::SelectCards { indices: vec![1] }).unwrap();
        settle(&mut g);
        assert!(matches!(
            g.decision,
            Some(Decision::PendingPlay {
                kind: InstructedPlayKind::LastRites,
                ..
            })
        ));
        assert_eq!(g.players[player].discard, vec![71, 133, 109]);
        assert_eq!(g.temporary_flow[player], vec![1]);
        // A separate already-finalized Flow target refers to the shifted spell
        // occurrence while Last Rites' selected physical unit is pending.
        g.queue_finalized_ability(
            player,
            33,
            vec![],
            ChainAbility::GrantFlowChosen {
                trash_index: 1,
                card_id: 133,
            },
        );
        let reference = g.chain.last().unwrap().id;
        let score = g.players[player].score;
        add(&mut g);
        begin(&mut g, child_action(false));
        g.apply(Action::RefusePayment).unwrap();
        assert_eq!(g.players[player].discard, vec![71, 109, 133, 109]);
        assert_eq!(g.temporary_flow[player], vec![2]);
        assert_eq!(g.players[player].score, score);
        assert_eq!(g.battlefield_owner[0], Some(player));
        assert!(g.play_grants.iter().any(|g| g.id == grant));
        assert!(matches!(
            g.chain.iter().find(|c| c.id == reference).unwrap().ability,
            Some(ChainAbility::GrantFlowChosen {
                trash_index: 2,
                card_id: 133
            })
        ));
        assert_eq!(
            g.players[player].units.len(),
            1,
            "the declined child did not enter"
        );
    }
}

#[test]
fn c25_targeted_deflect_trigger_refusal_keeps_the_completed_source_play() {
    for player in 0..2 {
        for direct in [false, true] {
            let mut g = position(player);
            let enemy = g.create_unit(1 - player, 92, false, 0);
            cast(&mut g, 145, vec![]);
            assert!(matches!(g.decision, Some(Decision::OnPlayTarget { .. })));
            let resources = g.players[player].clone();
            if direct {
                add(&mut g);
            }
            begin(
                &mut g,
                Action::ChooseTarget {
                    target: Some(Target::Unit {
                        player: 1 - player,
                        unit_id: enemy,
                    }),
                },
            );
            assert_eq!(
                g.payment_plan_view(player).unwrap().refusal_kind,
                Some(PaymentRefusalKind::Trigger)
            );
            g.apply(Action::RefusePayment).unwrap();
            assert_eq!(g.players[player], resources);
            assert!(g.players[player].units.iter().any(|u| u.card_id == 145));
            assert_eq!(
                g.unit_might(
                    g.unit_ref(Target::Unit {
                        player: 1 - player,
                        unit_id: enemy
                    })
                    .unwrap()
                )
                .unwrap(),
                4
            );
            assert!(g.chain.is_empty());
            assert!(g.decision.is_none());
        }
    }
}

#[test]
fn c25_portal_target_to_cost_transition_retains_its_first_add_origin() {
    for player in 0..2 {
        let mut g = position(player);
        let enemy = g.create_unit(1 - player, 92, false, 0);
        g.unit_mut(Target::Unit {
            player: 1 - player,
            unit_id: enemy,
        })
        .unwrap()
        .lane = Some(0);
        g.battlefield_owner[0] = Some(1 - player);
        g.players[player].hand = vec![406];
        g.apply(Action::PlayCard {
            hand_index: 0,
            lane: Some(0),
            target: None,
        })
        .unwrap();
        let before = g.players[player].clone();
        add(&mut g);
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![Target::Unit {
                player: 1 - player,
                unit_id: enemy,
            }],
        })
        .unwrap();
        let Some(Decision::TriggerPayment { choice_id, .. }) = g.decision else {
            panic!("{:?}", g.decision)
        };
        assert!(g.finalization_payment_origin.is_some());
        begin(
            &mut g,
            Action::ResolveTriggerPayment {
                choice_id,
                pay: true,
            },
        );
        add(&mut g);
        g.apply(Action::RefusePayment).unwrap();
        assert_eq!(g.players[player], before);
        assert!(
            !g.unit_ref(Target::Unit {
                player: 1 - player,
                unit_id: enemy
            })
            .unwrap()
            .ready
        );
        assert!(g.decision.is_none());
        assert!(g.chain.is_empty());
        assert!(g.trigger_payment_origin.is_none());
        assert!(g.finalization_payment_origin.is_none());
    }
}

#[test]
fn c25_identical_token_decisions_do_not_share_physical_payment_origins() {
    let mut g = position(0);
    g.create_unit(0, 265, false, 0);
    g.queue_token_play(0, 62, 1, Some(None), false);
    g.after_resolution().unwrap();
    add(&mut g);
    let decision = g.decision.clone();
    let first = g.pending_token.as_ref().unwrap().id;
    // Model a subsequent physical occurrence with the same public decision.
    // The private pending ID, not its card name/count/location, owns rollback.
    g.pending_token.as_mut().unwrap().id = first + 1;
    g.clear_completed_finalization_payment_origin();
    assert_eq!(g.decision, decision);
    assert!(g.finalization_payment_origin.is_none());
}

#[test]
fn c25_consecutive_identical_legacy_triggers_have_distinct_add_origins() {
    for player in 0..2 {
        let mut g = position(player);
        g.legend[player].kind = crate::legends::LegendKind::Irelia;
        let choice = Decision::Legend {
            player,
            trigger: LegendTrigger::IreliaConquer,
        };
        g.queue_trigger_choice(player, 0, choice.clone());
        g.queue_trigger_choice(player, 0, choice.clone());
        g.after_resolution().unwrap();
        let Some(Decision::OrderTriggers { ref options, .. }) = g.decision else {
            panic!("expected simultaneous trigger ordering")
        };
        let first = options[0].id;
        g.apply(Action::OrderTrigger { trigger_id: first }).unwrap();
        assert_eq!(g.decision, Some(choice.clone()));
        add(&mut g);
        g.apply(Action::Choose { accept: true }).unwrap();
        assert_eq!(g.decision, Some(choice));
        assert_eq!(g.chain.len(), 1, "the first trigger has finalized");
        let completed_first_resources = g.players[player].clone();
        let completed_first_chain = g.chain.clone();
        begin(&mut g, Action::Choose { accept: true });
        g.apply(Action::RefusePayment).unwrap();
        assert_eq!(g.players[player], completed_first_resources);
        assert_eq!(g.chain, completed_first_chain);
        assert!(g.finalization_payment_origin.is_none());
    }
}

fn quoted_add_checkpoint(base_cost: bool) {
    for player in 0..2 {
        let mut g = position(player);
        if base_cost {
            g.players[player].hand = vec![403];
            g.apply(Action::PlayCard {
                hand_index: 0,
                lane: Some(0),
                target: None,
            })
            .unwrap();
            assert!(matches!(g.decision, Some(Decision::TriggerPayment { .. })));
        } else {
            g.legend[player].kind = crate::legends::LegendKind::Irelia;
            g.decision = Some(Decision::Legend {
                player,
                trigger: LegendTrigger::IreliaConquer,
            });
        }
        let action = Action::ActivateRune {
            domain: Domain::Mind,
            recycle: false,
        };
        let payment = g.quote_power_payment(&action).unwrap().suggested;
        let mut exact = g.clone();
        g.apply(action.clone()).unwrap();
        exact
            .apply(Action::WithPowerPayment {
                action: Box::new(action),
                payment,
            })
            .unwrap();
        assert_eq!(
            format!("{g:?}"),
            format!("{exact:?}"),
            "a first-Add rollback origin cannot retain its command-local exact-payment ledger"
        );
    }
}

#[test]
fn c25_quoted_add_keeps_legacy_finalization_checkpoint_canonical() {
    quoted_add_checkpoint(false);
}

#[test]
fn c25_quoted_add_keeps_base_cost_trigger_checkpoint_canonical() {
    quoted_add_checkpoint(true);
}
