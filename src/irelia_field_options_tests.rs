//! Printed rules for OGN176, VEN115 and RAD046, exercised through normal actions.
use super::*;
use crate::activated::AbilitySource;
use test_support::{
    decline_legend_action, position, ready_unit, reject_unchanged, settle_unopposed_chain,
};

fn play(lane: Option<usize>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane,
        targets: vec![],
    }
}

fn targets(g: &mut Game, card: CardId) {
    for _ in 0..60 {
        if matches!(g.decision, Some(Decision::TriggerTargets { card_id, .. }) if card_id == card) {
            return;
        }
        let action = if let Some(action) = decline_legend_action(g) {
            action
        } else if let Some(Decision::OrderTriggers { options, .. }) = &g.decision {
            Action::OrderTrigger {
                trigger_id: options[0].id,
            }
        } else {
            assert!(
                g.decision.is_none() && !g.chain.is_empty(),
                "expected {card} choice, got {:?}",
                g.decision
            );
            Action::Pass
        };
        g.apply(action).unwrap();
    }
    panic!("trigger choice did not appear");
}

#[test]
fn open_field_permission_means_no_units_and_no_controller_in_both_seats() {
    for actor in 0..2 {
        for card in [470, 471, 473] {
            for holder in [None, Some(actor), Some(1 - actor)] {
                for occupant in [None, Some(actor), Some(1 - actor)] {
                    let mut g = position(actor, 0, 470001);
                    g.players[actor].hand = vec![card];
                    g.battlefield_owner[0] = holder;
                    if let Some(player) = occupant {
                        let id = ready_unit(&mut g, player, 193);
                        g.unit_mut(Target::Unit {
                            player,
                            unit_id: id,
                        })
                        .unwrap()
                        .lane = Some(0);
                    }
                    let allowed = holder == Some(actor) || (holder.is_none() && occupant.is_none());
                    if allowed {
                        g.apply(play(Some(0))).unwrap();
                    } else {
                        reject_unchanged(&mut g, play(Some(0)));
                    }
                }
            }
            let mut g = position(actor, 0, 470002);
            g.players[actor].hand = vec![card];
            g.apply(play(None)).unwrap(); // Ordinary base permission is retained.
        }
    }
}

#[test]
fn open_field_permission_is_discoverable_but_does_not_override_prohibitions_or_timing() {
    for actor in 0..2 {
        for card in [470, 471, 473] {
            let mut g = position(actor, 0, 470003);
            g.players[actor].hand = vec![card];
            assert!(
                g.canonical_play_candidates(128)
                    .unwrap()
                    .iter()
                    .any(|action| {
                        matches!(action, Action::WithPlayChoices { action, .. }
                    if matches!(**action, Action::PlayCard { lane: Some(0), .. }))
                    }),
                "AI did not discover {card} at an open field"
            );
            g.battlefields[0] = crate::battlefields::ROCKFALL_PATH_ID;
            reject_unchanged(&mut g, play(Some(0)));
            g.battlefields[0] = 3;
            let warden = ready_unit(&mut g, 1 - actor, 464);
            g.players[1 - actor].units.last_mut().unwrap().lane = Some(1);
            reject_unchanged(&mut g, play(Some(0)));
            g.players[1 - actor].units.retain(|unit| unit.id != warden);
            g.players[actor].hand.insert(0, 8); // A normal spell creates a response window.
            let id = ready_unit(&mut g, actor, 193);
            g.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![Target::Unit {
                    player: actor,
                    unit_id: id,
                }],
            })
            .unwrap();
            assert_eq!(g.priority(), actor);
            reject_unchanged(&mut g, play(Some(1)));
        }
    }
}

#[test]
fn gear_does_not_occupy_an_open_field_and_normal_units_get_no_extra_permission() {
    let mut g = position(0, 0, 470004);
    g.gear_play_at(1, 401, Some(0)).unwrap();
    g.players[0].hand = vec![193];
    reject_unchanged(&mut g, play(Some(0)));
    g.players[0].hand = vec![470];
    g.apply(play(Some(0))).unwrap();
    settle_unopposed_chain(&mut g);
    let unit = g.players[0]
        .units
        .iter()
        .find(|u| u.card_id == 470)
        .unwrap();
    assert_eq!((unit.lane, unit.ready), (Some(0), false));
    assert_eq!(g.players[0].energy, 17);
    assert_eq!(g.players[0].power_pool, [5; 6]);
}

#[test]
fn ocean_drake_can_decline_and_returns_only_non_dragons_to_their_original_owner() {
    for actor in 0..2 {
        for controller in 0..2 {
            for transferred in [false, true] {
                let mut g = position(actor, 0, 471001);
                let owner = if transferred {
                    1 - controller
                } else {
                    controller
                };
                let id = ready_unit(&mut g, owner, 193);
                let target = if transferred {
                    test_support::transfer_control_for_fixture(&mut g, id)
                } else {
                    Target::Unit {
                        player: owner,
                        unit_id: id,
                    }
                };
                let dragon = ready_unit(&mut g, controller, 471);
                g.players[actor].hand = vec![471];
                g.apply(play(None)).unwrap();
                targets(&mut g, 471);
                assert!(g.trigger_target_candidates().unwrap().contains(&target));
                reject_unchanged(
                    &mut g,
                    Action::ChooseTriggerTargets {
                        targets: vec![Target::Unit {
                            player: controller,
                            unit_id: dragon,
                        }],
                    },
                );
                let mut declined = g.clone();
                declined
                    .apply(Action::ChooseTriggerTargets { targets: vec![] })
                    .unwrap();
                settle_unopposed_chain(&mut declined);
                assert!(declined.unit_ref(target).is_some());
                g.apply(Action::ChooseTriggerTargets {
                    targets: vec![target],
                })
                .unwrap();
                settle_unopposed_chain(&mut g);
                assert!(g.unit_ref(target).is_none());
                assert_eq!(g.players[owner].hand, vec![193]);
                assert!(!g.players[1 - owner].hand.contains(&193));
                assert_eq!(g.players[actor].energy, 12);
                assert_eq!(g.players[actor].power_pool[domain_index(Domain::Chaos)], 3);
            }
        }
    }
}

fn rostrum(actor: usize) -> (Game, u32, Target) {
    let mut g = position(actor, 0, 472001);
    let unit_id = ready_unit(&mut g, actor, 193);
    g.players[actor].hand = vec![472];
    g.players[actor].rune_deck = vec![Domain::Chaos];
    g.apply(play(None)).unwrap();
    settle_unopposed_chain(&mut g);
    let gear_id = g.players[actor].gear[0].id;
    assert!(g.players[actor].gear[0].ready);
    (
        g,
        gear_id,
        Target::Unit {
            player: actor,
            unit_id,
        },
    )
}

fn activate(actor: usize, gear_id: u32) -> Action {
    Action::ActivateAbility {
        source: AbilitySource::Gear {
            player: actor,
            gear_id,
        },
        ability_index: 0,
        targets: vec![],
    }
}

#[test]
fn rostrum_exhaust_cost_triggers_separately_then_draws_and_channels_exhausted() {
    for actor in 0..2 {
        let (mut g, gear_id, target) = rostrum(actor);
        let enemy = ready_unit(&mut g, 1 - actor, 193);
        let energy = g.players[actor].energy;
        g.apply(activate(actor, gear_id)).unwrap();
        assert!(!g.players[actor].gear[0].ready);
        assert!(g.players[actor].hand.is_empty());
        assert_eq!(g.players[actor].total_runes(), 0);
        targets(&mut g, 472);
        reject_unchanged(&mut g, Action::ChooseTriggerTargets { targets: vec![] });
        reject_unchanged(
            &mut g,
            Action::ChooseTriggerTargets {
                targets: vec![Target::Unit {
                    player: 1 - actor,
                    unit_id: enemy,
                }],
            },
        );
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target],
        })
        .unwrap();
        assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), 2);
        settle_unopposed_chain(&mut g);
        assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), 5);
        assert_eq!(g.players[actor].hand, vec![71]);
        assert_eq!(g.players[actor].total_runes(), 1);
        assert_eq!(g.players[actor].ready_runes(Domain::Chaos), 0);
        assert!(g.players[actor].rune_deck.is_empty());
        assert_eq!(g.players[actor].energy, energy);
        reject_unchanged(&mut g, activate(actor, gear_id));
        g.apply(Action::EndTurn).unwrap();
        assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), 2);
    }
}

#[test]
fn rostrum_target_removed_in_response_does_not_cancel_its_draw_and_channel() {
    for actor in 0..2 {
        let (mut g, gear_id, target) = rostrum(actor);
        g.unit_mut(target).unwrap().lane = Some(0);
        g.battlefield_owner[0] = Some(actor);
        g.players[1 - actor].hand = vec![11];
        g.apply(activate(actor, gear_id)).unwrap();
        targets(&mut g, 472);
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target],
        })
        .unwrap();
        while let Some(action) = decline_legend_action(&g) {
            g.apply(action).unwrap();
        }
        assert!(g.decision.is_none(), "unexpected {:?}", g.decision);
        if g.priority() != 1 - actor {
            g.apply(Action::Pass).unwrap();
        }
        assert_eq!(g.priority(), 1 - actor);
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        })
        .unwrap();
        settle_unopposed_chain(&mut g);
        assert!(g.unit_ref(target).is_none());
        assert!(g.players[actor].hand.contains(&193));
        assert!(g.players[actor].hand.contains(&71));
        assert_eq!(g.players[actor].total_runes(), 1);
    }
}

#[test]
fn rostrum_observes_each_ready_to_exhausted_transition_including_other_object_costs() {
    let (mut g, gear_id, _) = rostrum(0);
    assert!(g.pending_triggers.is_empty());
    g.exhaust_gear(0, gear_id);
    assert_eq!(g.pending_triggers.len(), 1);
    g.exhaust_gear(0, gear_id);
    assert_eq!(g.pending_triggers.len(), 1);
    g.ready_object_by(0, Target::Gear { player: 0, gear_id });
    g.pay_object_costs(0, &[play::ObjectCost::ExhaustGear { player: 0, gear_id }])
        .unwrap();
    assert_eq!(g.pending_triggers.len(), 2);
    assert!(g.players[0].hand.is_empty());
    assert_eq!(g.players[0].total_runes(), 0);
}

#[test]
fn rostrum_without_friendly_units_or_remaining_runes_still_draws() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 472002);
        g.players[actor].hand = vec![472];
        g.players[actor].rune_deck.clear();
        g.apply(play(None)).unwrap();
        settle_unopposed_chain(&mut g);
        let gear_id = g.players[actor].gear[0].id;
        g.apply(activate(actor, gear_id)).unwrap();
        settle_unopposed_chain(&mut g);
        assert_eq!(g.players[actor].hand, vec![71]);
        assert_eq!(g.players[actor].total_runes(), 0);
    }
}

#[test]
fn every_ai_level_can_play_rostrum_and_choose_its_separate_trigger() {
    for difficulty in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        for actor in 0..2 {
            let mut g = position(actor, 0, 472003);
            g.difficulty = difficulty;
            let target = Target::Unit {
                player: actor,
                unit_id: ready_unit(&mut g, actor, 193),
            };
            g.players[actor].hand = vec![472];
            g.players[actor].rune_deck = vec![Domain::Chaos];
            g.unit_mut(target).unwrap().ready = false;
            assert!(g.step_ai().unwrap());
            assert!(
                !g.players[actor].gear.is_empty(),
                "{difficulty:?} seat{actor}: {:?}",
                g.log
            );
            assert_eq!(g.players[actor].gear[0].card_id, 472);
            for _ in 0..40 {
                if g.players[actor].hand.contains(&71) {
                    break;
                }
                assert!(g.step_ai().unwrap());
            }
            assert!(g.players[actor].hand.contains(&71));
            assert_eq!(g.players[actor].total_runes(), 1);
            assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), 5);
        }
    }
}
