//! Independent cost expectations from Sandswept Tomb's printed restriction.
use super::*;
use crate::battlefields::SANDSWEPT_TOMB_ID;
use test_support::{position, ready_unit, settle_unopposed_chain};

fn unit(g: &mut Game, player: usize, lane: Option<usize>) -> Target {
    let unit_id = ready_unit(g, player, 193);
    let target = Target::Unit { player, unit_id };
    g.unit_mut(target).unwrap().lane = lane;
    target
}

fn play(targets: Vec<Target>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets,
    }
}

fn power(g: &Game, action: &Action) -> u32 {
    g.cost_plan(action)
        .unwrap()
        .power
        .iter()
        .map(|cost| u32::from(cost.amount))
        .sum()
}

#[test]
fn tomb_reduces_power_not_energy_regardless_of_battlefield_controller() {
    for actor in 0..2 {
        for holder in [None, Some(actor), Some(1 - actor)] {
            let mut g = position(actor, 0, 164001);
            g.battlefields[0] = SANDSWEPT_TOMB_ID;
            g.battlefield_owner[0] = holder;
            g.players[actor].hand = vec![28]; // Punch First: 1 Energy, 2 Body Power.
            // The action is affordable only after the target-aware reduction.
            g.players[actor].power_pool = [0, 0, 0, 1, 0, 0];
            let target = unit(&mut g, actor, Some(0));
            let action = play(vec![target]);
            let plan = g.cost_plan(&action).unwrap();
            assert_eq!((plan.energy, power(&g, &action)), (1, 1));
            let before = g.players[actor].power_pool;
            g.apply(action).unwrap();
            assert_eq!(g.players[actor].energy, 19);
            assert_eq!(g.players[actor].power_pool[3], before[3] - 1);
            settle_unopposed_chain(&mut g);
            assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), 7);
        }
    }
}

#[test]
fn enemy_base_and_untargeted_effects_do_not_qualify() {
    for actor in 0..2 {
        for (controller, lane) in [(actor, None), (1 - actor, Some(0)), (actor, Some(1))] {
            let mut g = position(actor, 0, 164002);
            g.battlefields[0] = SANDSWEPT_TOMB_ID;
            g.players[actor].hand = vec![28];
            let target = unit(&mut g, controller, lane);
            assert_eq!(power(&g, &play(vec![target])), 2);
            g.apply(play(vec![target])).unwrap();
            assert_eq!(g.players[actor].power_pool[3], 3);
        }
        let mut g = position(actor, 0, 164003);
        g.battlefields[0] = SANDSWEPT_TOMB_ID;
        unit(&mut g, actor, Some(0));
        g.players[actor].hand = vec![311]; // Affects friendly units without choosing them.
        assert_eq!(power(&g, &play(vec![])), 3);
    }
}

#[test]
fn repeated_choices_get_one_reduction_per_qualifying_tomb() {
    for actor in 0..2 {
        for second_lane in [0, 1] {
            let mut g = position(actor, 0, 164004);
            g.battlefields = vec![SANDSWEPT_TOMB_ID; 2];
            g.players[actor].hand = vec![28];
            let first = unit(&mut g, actor, Some(0));
            let second = unit(&mut g, actor, Some(second_lane));
            g.grant_play(
                actor,
                ChainSource::Card { card_id: 133 },
                PlayGrantKind::RepeatNextSpell,
            );
            let original = play(vec![first]);
            let option_id = g.play_choices(&original).unwrap().repeats[0].id;
            let action = Action::WithPlayChoices {
                action: Box::new(original),
                choices: PlayChoices {
                    repeats: vec![play::RepeatSelection {
                        option_id,
                        choices: play::ExecutionChoices {
                            targets: vec![second],
                            ..Default::default()
                        },
                    }],
                    ..Default::default()
                },
            };
            // Original and Repeat total 2 Energy + 4 Power. A field reduces once,
            // even when both executions choose different friendly units there.
            assert_eq!(g.cost_plan(&action).unwrap().energy, 2);
            assert_eq!(power(&g, &action), if second_lane == 0 { 3 } else { 2 });
            g.apply(action).unwrap();
            settle_unopposed_chain(&mut g);
            assert_eq!(g.unit_might(g.unit_ref(first).unwrap()).unwrap(), 7);
            assert_eq!(g.unit_might(g.unit_ref(second).unwrap()).unwrap(), 7);
        }
    }
}

#[test]
fn friendliness_uses_current_controller_and_paid_cost_survives_target_departure() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 164005);
        g.battlefields[0] = SANDSWEPT_TOMB_ID;
        let id = ready_unit(&mut g, 1 - actor, 193);
        let target = test_support::transfer_control_for_fixture(&mut g, id);
        g.unit_mut(target).unwrap().lane = Some(0);
        g.players[actor].hand = vec![28];
        assert_eq!(power(&g, &play(vec![target])), 1);
        g.apply(play(vec![target])).unwrap();
        while let Some(action) = test_support::decline_legend_action(&g) {
            g.apply(action).unwrap();
        }
        assert!(g.decision.is_none());
        if g.priority() != 1 - actor {
            g.apply(Action::Pass).unwrap();
        }
        g.players[1 - actor].hand = vec![11]; // Gust before the Might grant resolves.
        g.apply(play(vec![target])).unwrap();
        settle_unopposed_chain(&mut g);
        assert!(g.unit_ref(target).is_none());
        assert!(g.players[1 - actor].hand.contains(&193));
        assert_eq!(g.players[actor].power_pool[3], 4);
        assert_eq!(g.players[actor].energy, 19);
    }
}

#[test]
fn a_zero_power_spell_does_not_convert_the_reduction_into_energy() {
    let mut g = position(0, 0, 164006);
    g.battlefields[0] = SANDSWEPT_TOMB_ID;
    g.players[0].hand = vec![7]; // Discipline costs 2 Energy and no Power.
    let target = unit(&mut g, 0, Some(0));
    let action = play(vec![target]);
    assert_eq!(
        (g.cost_plan(&action).unwrap().energy, power(&g, &action)),
        (2, 0)
    );
    g.apply(action).unwrap();
    assert_eq!(g.players[0].energy, 18);
    assert_eq!(g.players[0].power_pool, [5; 6]);
}

#[test]
fn irelia_movement_spell_is_playable_and_discoverable_with_no_power() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 164007);
        g.battlefields[0] = SANDSWEPT_TOMB_ID;
        g.players[actor].hand = vec![14]; // Ride the Wind: 2 Energy + 1 Chaos Power.
        g.players[actor].power_pool = [0; 6];
        let target = unit(&mut g, actor, Some(0));
        let action = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: Some(1),
            targets: vec![target],
        };
        assert_eq!(
            (g.cost_plan(&action).unwrap().energy, power(&g, &action)),
            (2, 0)
        );
        assert!(
            g.canonical_play_candidates(128)
                .unwrap()
                .iter()
                .any(|candidate| {
                    let mut probe = g.clone();
                    probe.apply(candidate.clone()).is_ok()
                        && probe.chain.iter().any(|item| item.card_id == 14)
                })
        );
        g.apply(action).unwrap();
        settle_unopposed_chain(&mut g);
        let moved = g.unit_ref(target).unwrap();
        assert_eq!((moved.lane, moved.ready), (Some(1), true));
        assert_eq!(g.players[actor].power_pool, [0; 6]);
        assert_eq!(g.players[actor].energy, 18);
    }
}

#[test]
fn a_friendly_target_chosen_only_for_repeat_still_reduces_the_total() {
    let mut g = position(0, 0, 164008);
    g.battlefields[0] = SANDSWEPT_TOMB_ID;
    g.players[0].hand = vec![28];
    let enemy = unit(&mut g, 1, Some(0));
    let friendly = unit(&mut g, 0, Some(0));
    g.grant_play(
        0,
        ChainSource::Card { card_id: 133 },
        PlayGrantKind::RepeatNextSpell,
    );
    let original = play(vec![enemy]);
    let option_id = g.play_choices(&original).unwrap().repeats[0].id;
    let action = Action::WithPlayChoices {
        action: Box::new(original),
        choices: PlayChoices {
            repeats: vec![play::RepeatSelection {
                option_id,
                choices: play::ExecutionChoices {
                    targets: vec![friendly],
                    ..Default::default()
                },
            }],
            ..Default::default()
        },
    };
    assert_eq!(power(&g, &action), 3);
    g.apply(action).unwrap();
    settle_unopposed_chain(&mut g);
    for target in [enemy, friendly] {
        assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), 7);
    }
}
