//! CR814.2's printed Block example, CR815 Tank, CR811 Hidden and live Here.
use super::*;
use test_support::{position, ready_unit, reject_unchanged};

const BLOCK: CardId = 461;
const YUUMI: CardId = 462;

fn unit(g: &mut Game, owner: usize, card: CardId, lane: Option<usize>) -> Target {
    let unit_id = ready_unit(g, owner, card);
    let target = Target::Unit {
        player: owner,
        unit_id,
    };
    g.unit_mut(target).unwrap().lane = lane;
    target
}
fn id(target: Target) -> u32 {
    let Target::Unit { unit_id, .. } = target else {
        panic!("unit required")
    };
    unit_id
}
fn cast(g: &mut Game, card: CardId, target: Target) {
    while let Some(action) = test_support::decline_legend_action(g) {
        g.apply(action).unwrap();
    }
    let actor = g.priority();
    let hand_index = g.players[actor].hand.len();
    g.players[actor].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets: vec![target],
    })
    .unwrap();
}
fn settle(g: &mut Game) {
    for _ in 0..40 {
        if let Some(action) = test_support::decline_legend_action(g) {
            g.apply(action).unwrap();
        } else if g.chain.is_empty() {
            return;
        } else {
            assert!(g.decision.is_none(), "unexpected choice: {:?}", g.decision);
            g.apply(Action::Pass).unwrap();
        }
    }
    panic!("chain failed to settle");
}

#[test]
fn turn_keywords_block_matches_official_poro_example_for_both_owners_and_stacks() {
    for actor in 0..2 {
        for owner in 0..2 {
            let mut g = position(actor, 3, 461001);
            let poro = unit(&mut g, owner, 110, None);
            let original_might = g.unit_might(g.unit_ref(poro).unwrap()).unwrap();
            let energy = g.players[actor].energy;
            cast(&mut g, BLOCK, poro);
            assert_eq!(g.players[actor].energy, energy - 2);
            assert!(!g.turn_tank(id(poro)), "keywords wait for resolution");
            settle(&mut g);
            assert_eq!(
                g.unit_keywords(g.unit_ref(poro).unwrap()).unwrap().shield,
                4
            );
            assert!(g.unit_keywords(g.unit_ref(poro).unwrap()).unwrap().tank);
            assert_eq!(
                g.unit_might(g.unit_ref(poro).unwrap()).unwrap(),
                original_might,
                "Shield grants Might only to a defender"
            );
            cast(&mut g, BLOCK, poro);
            settle(&mut g);
            assert_eq!(
                g.unit_keywords(g.unit_ref(poro).unwrap()).unwrap().shield,
                7
            );
            assert!(g.turn_tank(id(poro)));
            g.apply(Action::EndTurn).unwrap();
            assert_eq!(
                g.unit_keywords(g.unit_ref(poro).unwrap()).unwrap().shield,
                1
            );
            assert!(!g.turn_tank(id(poro)));
            assert!(g.turn_keyword_grants.is_empty());
        }
    }
}

#[test]
fn turn_keywords_block_has_required_unit_target_action_timing_and_can_be_countered() {
    for actor in 0..2 {
        let mut g = position(actor, 3, 461002);
        let poro = unit(&mut g, actor, 110, None);
        g.players[actor].hand = vec![BLOCK];
        for targets in [
            vec![],
            vec![Target::Legend {
                player: actor,
                legend_id: 3,
            }],
            vec![Target::Unit {
                player: actor,
                unit_id: u32::MAX,
            }],
            vec![poro, poro],
        ] {
            reject_unchanged(
                &mut g,
                Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets,
                },
            );
        }
        cast(&mut g, 7, poro); // A Reaction creates a closed chain.
        reject_unchanged(
            &mut g,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![poro],
            },
        );
        settle(&mut g);
        cast(&mut g, BLOCK, poro);
        let chain_id = g.chain.last().unwrap().id;
        g.apply(Action::Pass).unwrap();
        cast(&mut g, 6, Target::Spell { chain_id });
        settle(&mut g);
        assert_eq!(g.turn_shield(id(poro)).unwrap(), 0);
        assert!(!g.turn_tank(id(poro)));
    }
}

#[test]
fn turn_keywords_block_hidden_reacts_only_at_its_field_and_ignores_base_cost() {
    for actor in 0..2 {
        let mut g = position(actor, 3, 461003);
        let here = unit(&mut g, actor, 110, Some(0));
        let elsewhere = unit(&mut g, actor, 110, Some(1));
        g.battlefield_owner[0] = Some(actor);
        let hidden_id = 90000;
        g.players[actor].hidden.push(HiddenCard {
            id: hidden_id,
            card_id: BLOCK,
            lane: 0,
            hidden_turn: g.turn.saturating_sub(1),
            revealed: false,
        });
        cast(&mut g, 7, here);
        let energy = g.players[actor].energy;
        reject_unchanged(
            &mut g,
            Action::PlayHidden {
                hidden_id,
                targets: vec![elsewhere],
            },
        );
        g.apply(Action::PlayHidden {
            hidden_id,
            targets: vec![here],
        })
        .unwrap();
        assert_eq!(g.players[actor].energy, energy);
        assert!(g.chain.last().unwrap().from_hidden);
        let mut copied = g.clone();
        settle(&mut g);
        settle(&mut copied);
        assert_eq!(format!("{g:?}"), format!("{copied:?}"));
        assert_eq!(g.turn_shield(id(here)).unwrap(), 3);
        assert!(g.turn_tank(id(here)));
        assert_eq!(g.turn_shield(id(elsewhere)).unwrap(), 0);
    }
}

#[test]
fn turn_keywords_block_live_defender_might_and_tank_change_damage_assignment() {
    for actor in 0..2 {
        let mut g = position(actor, 3, 461004);
        let attacker = unit(&mut g, actor, 71, None);
        g.unit_mut(attacker).unwrap().buff = 20;
        let tank = unit(&mut g, 1 - actor, 110, Some(0));
        let other = unit(&mut g, 1 - actor, 71, Some(0));
        let printed = g.unit_might(g.unit_ref(tank).unwrap()).unwrap();
        cast(&mut g, BLOCK, tank);
        settle(&mut g);
        g.battlefield_owner[0] = Some(1 - actor);
        g.apply(Action::Move {
            unit_id: id(attacker),
            lane: Some(0),
        })
        .unwrap();
        settle(&mut g);
        assert_eq!(
            g.unit_might(g.unit_ref(tank).unwrap()).unwrap(),
            printed + 4
        );
        g.apply(Action::Pass).unwrap();
        g.apply(Action::Pass).unwrap();
        let Some(Decision::CombatDamage { player, total, .. }) = g.decision else {
            panic!("{:?}", g.decision)
        };
        assert_eq!(player, actor);
        reject_unchanged(
            &mut g,
            Action::AssignDamage {
                assignments: vec![(id(other), total)],
            },
        );
        let lethal = g.unit_might(g.unit_ref(tank).unwrap()).unwrap();
        g.apply(Action::AssignDamage {
            assignments: vec![(id(tank), lethal), (id(other), total - lethal)],
        })
        .unwrap();
    }
}

fn yuumi_position(actor: usize, defending: bool) -> (Game, Target, Target, Target) {
    let mut g = position(actor, 0, 462001);
    let owner = if defending { 1 - actor } else { actor };
    let yuumi = unit(&mut g, owner, YUUMI, if defending { Some(0) } else { None });
    let ally = unit(&mut g, owner, 110, Some(0));
    let enemy = unit(
        &mut g,
        1 - owner,
        71,
        if defending { None } else { Some(0) },
    );
    g.battlefield_owner[0] = Some(1 - actor);
    let mover = if defending { enemy } else { yuumi };
    g.apply(Action::Move {
        unit_id: id(mover),
        lane: Some(0),
    })
    .unwrap();
    assert!(
        matches!(
            g.decision,
            Some(Decision::TriggerTargets { card_id: YUUMI, .. })
        ),
        "{:?}",
        g.decision
    );
    (g, yuumi, ally, enemy)
}

#[test]
fn turn_keywords_yuumi_attack_and_defend_choose_only_other_friendly_here_and_clone_choices() {
    for actor in 0..2 {
        for defending in [false, true] {
            let (mut g, yuumi, ally, enemy) = yuumi_position(actor, defending);
            for targets in [vec![], vec![yuumi], vec![enemy]] {
                reject_unchanged(&mut g, Action::ChooseTriggerTargets { targets });
            }
            let mut copied = g.clone();
            for state in [&mut g, &mut copied] {
                state
                    .apply(Action::ChooseTriggerTargets {
                        targets: vec![ally],
                    })
                    .unwrap();
                assert!(!state.turn_tank(id(ally)));
                settle(state);
                assert_eq!(state.unit_ref(ally).unwrap().temporary_buff, 3);
                assert!(
                    state
                        .unit_keywords(state.unit_ref(ally).unwrap())
                        .unwrap()
                        .tank
                );
                assert!(
                    !state
                        .unit_keywords(state.unit_ref(yuumi).unwrap())
                        .unwrap()
                        .tank,
                    "Yuumi grants Tank; she does not have printed Tank"
                );
            }
            assert_eq!(format!("{g:?}"), format!("{copied:?}"));
        }
    }
}

#[test]
fn turn_keywords_yuumi_departure_before_resolution_mistargets_both_instructions() {
    for actor in 0..2 {
        for remove_source in [false, true] {
            let (mut g, yuumi, ally, _) = yuumi_position(actor, false);
            g.apply(Action::ChooseTriggerTargets {
                targets: vec![ally],
            })
            .unwrap();
            cast(&mut g, 11, if remove_source { yuumi } else { ally });
            settle(&mut g);
            assert!(!g.turn_tank(id(ally)));
            if remove_source {
                assert_eq!(g.unit_ref(ally).unwrap().temporary_buff, 0);
            } else {
                assert!(g.unit_ref(ally).is_none());
            }
        }
    }
}

#[test]
fn turn_keywords_leave_with_the_physical_unit_and_preserve_printed_tank_at_expiry() {
    for actor in 0..2 {
        let mut g = position(actor, 3, 461005);
        let poro = unit(&mut g, actor, 110, Some(0));
        let printed_tank = unit(&mut g, actor, 57, None);
        cast(&mut g, BLOCK, poro);
        settle(&mut g);
        cast(&mut g, 11, poro);
        settle(&mut g);
        assert!(g.unit_ref(poro).is_none());
        assert_eq!(g.turn_shield(id(poro)).unwrap(), 0);
        assert!(!g.turn_tank(id(poro)));
        assert!(g.players[actor].hand.contains(&110));
        cast(&mut g, BLOCK, printed_tank);
        settle(&mut g);
        g.apply(Action::EndTurn).unwrap();
        assert!(!g.turn_tank(id(printed_tank)));
        assert!(
            g.unit_keywords(g.unit_ref(printed_tank).unwrap())
                .unwrap()
                .tank
        );
    }
}

#[test]
fn turn_keywords_yuumi_required_target_is_playable_by_every_ai_level_and_seat() {
    for actor in 0..2 {
        for defending in [false, true] {
            for difficulty in [
                Difficulty::Easy,
                Difficulty::Tactical,
                Difficulty::Strategic,
            ] {
                let (mut g, _, ally, _) = yuumi_position(actor, defending);
                g.difficulty = difficulty;
                let action = g.choose_ai_action().unwrap();
                assert_eq!(
                    action,
                    Action::ChooseTriggerTargets {
                        targets: vec![ally]
                    }
                );
                g.apply(action).unwrap();
                settle(&mut g);
                assert!(g.turn_tank(id(ally)));
                assert_eq!(g.unit_ref(ally).unwrap().temporary_buff, 3);
            }
        }
    }
}

#[test]
fn turn_keywords_block_on_backline_permits_either_combat_damage_priority() {
    for actor in 0..2 {
        let mut g = position(actor, 3, 461006);
        let attacker = unit(&mut g, actor, 71, None);
        g.unit_mut(attacker).unwrap().buff = 20;
        let dual = unit(&mut g, 1 - actor, 13, Some(0));
        let ordinary = unit(&mut g, 1 - actor, 71, Some(0));
        cast(&mut g, BLOCK, dual);
        settle(&mut g);
        g.battlefield_owner[0] = Some(1 - actor);
        g.apply(Action::Move {
            unit_id: id(attacker),
            lane: Some(0),
        })
        .unwrap();
        settle(&mut g);
        g.apply(Action::Pass).unwrap();
        g.apply(Action::Pass).unwrap();
        let Some(Decision::CombatDamage { total, .. }) = g.decision else {
            panic!("{:?}", g.decision)
        };
        for order in [[dual, ordinary], [ordinary, dual]] {
            let mut copy = g.clone();
            let lethal = g.unit_might(g.unit_ref(order[0]).unwrap()).unwrap();
            copy.apply(Action::AssignDamage {
                assignments: vec![(id(order[0]), lethal), (id(order[1]), total - lethal)],
            })
            .unwrap();
        }
    }
}
