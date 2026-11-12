//! Gust Monk's extra play cost is separate from its pre-response banish cost.
use super::*;

fn position(owner: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 431431, Difficulty::Tactical);
    g.choose_starting_player(owner).unwrap();
    g.start_match().unwrap();
    g.battlefields = [2, 2];
    g.battlefield_owner = [Some(owner), None];
    for p in &mut g.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 20;
        p.energy = 20;
        p.power_pool = [10; 6];
        p.any_power = 0;
    }
    for l in &mut g.legend {
        l.ready = false;
    }
    g
}
fn unit(g: &mut Game, owner: usize, card: usize) -> Target {
    let unit_id = g.create_unit(owner, card, false, 0);
    Target::Unit {
        player: owner,
        unit_id,
    }
}
fn play(g: &mut Game, bonus: bool) -> Target {
    let p = g.priority();
    g.players[p].hand.push(431);
    g.apply(Action::WithPlayChoices {
        action: Box::new(Action::PlayCardWithTargets {
            hand_index: g.players[p].hand.len() - 1,
            lane: None,
            targets: vec![],
        }),
        choices: PlayChoices {
            bonus,
            ..Default::default()
        },
    })
    .unwrap();
    let u = g.players[p]
        .units
        .iter()
        .rev()
        .find(|u| u.card_id == 431)
        .unwrap();
    Target::Unit {
        player: p,
        unit_id: u.id,
    }
}
fn choose(g: &mut Game, target: Target) -> u32 {
    assert!(matches!(
        g.decision,
        Some(Decision::TriggerTargets { card_id: 431, .. })
    ));
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![target],
    })
    .unwrap();
    let Some(Decision::TriggerPayment { choice_id, .. }) = g.decision else {
        panic!("{:?}", g.decision)
    };
    choice_id
}
fn pay(g: &mut Game, choice_id: u32, owner: usize, index: usize) {
    g.apply(Action::ResolveTriggerTrashPayment {
        choice_id,
        trash_player: owner,
        trash_index: index,
    })
    .unwrap();
}
fn settle(g: &mut Game) {
    for _ in 0..80 {
        match g.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => g
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            None if !g.chain.is_empty() => g.apply(Action::Pass).unwrap(),
            None => return,
            x => panic!("Unexpected {x:?}"),
        }
    }
    panic!("chain did not settle")
}
fn reject(g: &mut Game, a: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(a).is_err());
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn gust_optional_energy_is_paid_on_play_and_does_not_itself_grant_assault() {
    for owner in 0..2 {
        for bonus in [false, true] {
            let mut g = position(owner);
            let monk = play(&mut g, bonus);
            assert_eq!(g.players[owner].energy, 18 - u16::from(bonus));
            assert_eq!(
                g.unit_keywords(g.unit_ref(monk).unwrap()).unwrap().assault,
                0
            );
            if bonus {
                let choice = choose(&mut g, monk);
                assert!(
                    g.legal_actions()
                        .unwrap()
                        .iter()
                        .all(|a| !matches!(a, Action::ResolveTriggerTrashPayment { .. }))
                );
                g.apply(Action::ResolveTriggerPayment {
                    choice_id: choice,
                    pay: false,
                })
                .unwrap();
            }
            assert!(g.decision.is_none());
            assert!(g.chain.is_empty());
            assert!(g.unit_ref(monk).is_some());
        }
    }
}

#[test]
fn gust_cost_can_banish_either_trash_without_targeting_it_and_assault_is_delayed() {
    for owner in 0..2 {
        for trash_owner in 0..2 {
            for recipient in 0..2 {
                let mut g = position(owner);
                let target = unit(&mut g, recipient, 71);
                g.players[trash_owner].discard = vec![71, 7, 71];
                let monk = play(&mut g, true);
                let choice = choose(&mut g, target);
                reject(
                    &mut g,
                    Action::ResolveTriggerPayment {
                        choice_id: choice,
                        pay: true,
                    },
                );
                reject(
                    &mut g,
                    Action::ResolveTriggerTrashPayment {
                        choice_id: choice + 1,
                        trash_player: trash_owner,
                        trash_index: 1,
                    },
                );
                reject(
                    &mut g,
                    Action::ResolveTriggerTrashPayment {
                        choice_id: choice,
                        trash_player: 2,
                        trash_index: 1,
                    },
                );
                reject(
                    &mut g,
                    Action::ResolveTriggerTrashPayment {
                        choice_id: choice,
                        trash_player: trash_owner,
                        trash_index: 99,
                    },
                );
                pay(&mut g, choice, trash_owner, 1);
                assert_eq!(g.players[trash_owner].discard, vec![71, 71]);
                assert_eq!(g.players[trash_owner].banish, vec![7]);
                let item = g.chain.last().unwrap();
                assert_eq!(item.targets, vec![target]);
                assert_eq!(
                    g.unit_keywords(g.unit_ref(target).unwrap())
                        .unwrap()
                        .assault,
                    0
                );
                g.kill_units(&[monk]).unwrap();
                settle(&mut g);
                assert_eq!(
                    g.unit_keywords(g.unit_ref(target).unwrap())
                        .unwrap()
                        .assault,
                    2
                );
                assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), 5);
                g.unit_mut(target).unwrap().lane = Some(0);
                g.showdown = Some(Showdown {
                    combat: true,
                    lane: 0,
                    attacker: recipient,
                    focus: recipient,
                    passes: 0,
                });
                assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), 7);
                g.showdown.as_mut().unwrap().attacker = 1 - recipient;
                assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), 5);
                g.legend_end_turn();
                assert_eq!(
                    g.unit_keywords(g.unit_ref(target).unwrap())
                        .unwrap()
                        .assault,
                    0
                );
            }
        }
    }
}

#[test]
fn gust_last_target_leaving_and_counter_do_not_refund_the_banish_cost() {
    for counter in [false, true] {
        let mut g = position(0);
        let target = unit(&mut g, 1, 71);
        g.players[0].discard = vec![7];
        play(&mut g, true);
        let choice = choose(&mut g, target);
        pay(&mut g, choice, 0, 0);
        if counter {
            let chain_id = g.chain.last().unwrap().id;
            g.apply(Action::Pass).unwrap();
            assert_eq!(g.priority(), 1);
            g.players[1].hand = vec![12];
            g.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![Target::Spell { chain_id }],
            })
            .unwrap();
        } else {
            g.return_board_targets(&[target]);
        }
        settle(&mut g);
        assert_eq!(g.players[0].banish, vec![7]);
        assert!(g.legend.iter().all(|l| l.assault_grants.is_empty()));
    }
}

#[test]
fn gust_can_target_itself_and_multiple_grants_stack_only_until_cleanup() {
    let mut g = position(0);
    g.players[1].discard = vec![71, 71];
    let monk = play(&mut g, true);
    let choice = choose(&mut g, monk);
    pay(&mut g, choice, 1, 0);
    settle(&mut g);
    play(&mut g, true);
    let choice = choose(&mut g, monk);
    pay(&mut g, choice, 1, 0);
    settle(&mut g);
    assert_eq!(
        g.unit_keywords(g.unit_ref(monk).unwrap()).unwrap().assault,
        4
    );
    assert_eq!(g.unit_might(g.unit_ref(monk).unwrap()).unwrap(), 2);
    g.legend_end_turn();
    assert_eq!(
        g.unit_keywords(g.unit_ref(monk).unwrap()).unwrap().assault,
        0
    );
}

#[test]
fn gust_deflect_is_an_ability_cost_and_payment_rejection_is_atomic() {
    let mut g = position(0);
    let target = unit(&mut g, 1, 287);
    g.players[1].discard = vec![71];
    play(&mut g, true);
    let choice = choose(&mut g, target);
    g.players[0].power_pool = [0; 6];
    g.legend[0].spell_power = 10;
    let a = Action::ResolveTriggerTrashPayment {
        choice_id: choice,
        trash_player: 1,
        trash_index: 0,
    };
    reject(&mut g, a.clone());
    assert_eq!(g.players[1].discard, vec![71]);
    g.players[0].any_power = 2;
    g.apply(a).unwrap();
    assert_eq!(g.players[0].any_power, 0);
    assert_eq!(g.legend[0].spell_power, 10);
    settle(&mut g);
    assert_eq!(
        g.unit_keywords(g.unit_ref(target).unwrap())
            .unwrap()
            .assault,
        2
    );
}

#[test]
fn gust_frozen_payment_tracks_physical_trash_copy_and_undo_restores_costs() {
    for remove_selected in [false, true] {
        let mut g = position(0);
        let monk = play(&mut g, true);
        g.players[0].discard = vec![71, 7, 71];
        let choice = choose(&mut g, monk);
        let a = Action::ResolveTriggerTrashPayment {
            choice_id: choice,
            trash_player: 0,
            trash_index: 2,
        };
        g.apply(Action::BeginPayment {
            action: Box::new(a),
        })
        .unwrap();
        assert!(g.can_refuse_payment().unwrap());
        assert!(g.can_cancel_payment().unwrap());
        g.recycle_cards(0, &[if remove_selected { 2 } else { 0 }]);
        if remove_selected {
            reject(&mut g, Action::CompletePayment);
            g.apply(Action::Choose { accept: false }).unwrap();
            assert_eq!(g.players[0].discard, vec![71, 7, 71]);
            assert!(g.players[0].banish.is_empty());
            g.apply(Action::ResolveTriggerPayment {
                choice_id: choice,
                pay: false,
            })
            .unwrap();
            assert!(g.chain.is_empty());
            assert!(g.unit_ref(monk).is_some());
        } else {
            g.apply(Action::CompletePayment).unwrap();
            assert_eq!(g.players[0].discard, vec![7]);
            assert_eq!(g.players[0].banish, vec![71]);
            settle(&mut g);
        }
    }
}

#[test]
fn gust_payment_burnout_cannot_substitute_a_new_copy_at_the_old_trash_index() {
    let mut g = position(0);
    let monk = play(&mut g, true);
    g.players[0].discard = vec![71];
    let choice = choose(&mut g, monk);
    g.apply(Action::BeginPayment {
        action: Box::new(Action::ResolveTriggerTrashPayment {
            choice_id: choice,
            trash_player: 0,
            trash_index: 0,
        }),
    })
    .unwrap();
    g.players[0].deck.clear();
    g.draw(0, 1).unwrap();
    g.players[0].discard.push(71);
    reject(&mut g, Action::CompletePayment);
    g.apply(Action::RefusePayment).unwrap();
    assert_eq!(g.players[0].discard, vec![71]);
    assert_eq!(g.players[1].score, 0);
    assert!(g.chain.is_empty());
}

#[test]
fn gust_declining_after_add_restores_resources_but_keeps_the_paid_unit_play() {
    for staged in [false, true] {
        let mut g = position(0);
        let sage = unit(&mut g, 0, 236);
        g.unit_mut(sage).unwrap().ready = true;
        g.players[0].discard = vec![71];
        let monk = play(&mut g, true);
        let choice = choose(&mut g, monk);
        if staged {
            g.apply(Action::BeginPayment {
                action: Box::new(Action::ResolveTriggerTrashPayment {
                    choice_id: choice,
                    trash_player: 0,
                    trash_index: 0,
                }),
            })
            .unwrap();
        }
        let Target::Unit { unit_id, .. } = sage else {
            panic!()
        };
        g.apply(Action::ActivateAbility {
            source: crate::activated::AbilitySource::Unit { player: 0, unit_id },
            ability_index: 0,
            targets: vec![],
        })
        .unwrap();
        assert_eq!(g.players[0].energy, 18);
        assert!(!g.unit_ref(sage).unwrap().ready);
        g.apply(if staged {
            Action::RefusePayment
        } else {
            Action::ResolveTriggerPayment {
                choice_id: choice,
                pay: false,
            }
        })
        .unwrap();
        assert_eq!(g.players[0].energy, 17);
        assert!(g.unit_ref(sage).unwrap().ready);
        assert!(g.unit_ref(monk).is_some());
        assert_eq!(g.players[0].discard, vec![71]);
        assert!(g.chain.is_empty());
    }
}

#[test]
fn gust_choose_irelia_readies_her_after_banish_payment_before_assault_resolves() {
    let mut g = position(0);
    g.legend[0] = crate::legends::LegendState::from_legend(0).unwrap();
    let irelia = unit(&mut g, 0, 0);
    assert!(!g.unit_ref(irelia).unwrap().ready);
    g.players[1].discard = vec![71];
    play(&mut g, true);
    let choice = choose(&mut g, irelia);
    assert!(!g.unit_ref(irelia).unwrap().ready);
    assert!(g.chain.is_empty());
    pay(&mut g, choice, 1, 0);
    assert_eq!(g.players[1].banish, vec![71]);
    let mut offered = false;
    for _ in 0..60 {
        match g.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => g
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            Some(Decision::Legend { .. }) => {
                offered = true;
                g.apply(Action::Choose { accept: true }).unwrap();
            }
            None if !g.chain.is_empty() => g.apply(Action::Pass).unwrap(),
            None => break,
            other => panic!("{other:?}"),
        }
    }
    assert!(offered);
    assert_eq!(g.unit_ref(irelia).unwrap().temporary_buff, 2);
    assert!(g.unit_ref(irelia).unwrap().ready);
    assert_eq!(
        g.unit_keywords(g.unit_ref(irelia).unwrap())
            .unwrap()
            .assault,
        2
    );
}

#[test]
fn gust_hunters_circle_discount_changes_total_cost_without_erasing_paid_bonus() {
    let mut g = position(0);
    g.battlefields[0] = 7;
    g.players[0].hand = vec![431];
    g.apply(Action::WithPlayChoices {
        action: Box::new(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: Some(0),
            targets: vec![],
        }),
        choices: PlayChoices {
            bonus: true,
            ..Default::default()
        },
    })
    .unwrap();
    assert_eq!(g.players[0].energy, 18);
    assert!(matches!(
        g.decision,
        Some(Decision::TriggerTargets { card_id: 431, .. })
    ));
}
