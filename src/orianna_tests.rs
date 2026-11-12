//! Orianna uses physical gear locations without changing her reference Might.
use super::*;
use crate::gear::GearState;

fn position(actor: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 430430, Difficulty::Tactical);
    g.start_match().unwrap();
    g.active = actor;
    g.priority_player = actor;
    g.battlefields = [2, 2];
    g.battlefield_owner = [Some(1 - actor), None];
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
        p.energy_pool = 100;
        p.energy = 100;
        p.power_pool = [20; 6];
        p.any_power = 0;
    }
    for legend in &mut g.legend {
        legend.ready = false;
    }
    g
}
fn unit(g: &mut Game, owner: usize, card: usize, lane: Option<usize>) -> Target {
    let unit_id = g.create_unit(owner, card, false, 0);
    let unit = g.players[owner].units.last_mut().unwrap();
    unit.lane = lane;
    unit.ready = true;
    Target::Unit {
        player: owner,
        unit_id,
    }
}
fn id(t: Target) -> u32 {
    let Target::Unit { unit_id, .. } = t else {
        panic!()
    };
    unit_id
}
fn contribution(g: &Game, target: Target) -> crate::numeric::CombatContribution {
    g.unit_combat_contribution(g.unit_ref(target).unwrap())
        .unwrap()
}
fn gear(
    g: &mut Game,
    owner: usize,
    lane: Option<usize>,
    bearer: Option<u32>,
    token: bool,
    ready: bool,
) -> u32 {
    let id = g.next_unit_id;
    g.next_unit_id += 1;
    g.players[owner].gear.push(GearState {
        id,
        card_id: if bearer.is_some() { 3 } else { 402 },
        lane,
        attached_to: bearer,
        ready,
        temporary: false,
        token,
    });
    id
}
fn cast(g: &mut Game, card: usize, targets: Vec<Target>) {
    let player = g.priority();
    let hand_index = g.players[player].hand.len();
    g.players[player].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
}
fn settle(g: &mut Game) {
    for _ in 0..60 {
        match g.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => g
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            None if !g.chain.is_empty() => g.apply(Action::Pass).unwrap(),
            None => return,
            other => panic!("unexpected {other:?}"),
        }
    }
    panic!("chain did not settle")
}

#[test]
fn orianna_counts_only_friendly_gear_at_her_actual_location_in_both_seats() {
    for owner in 0..2 {
        for lane in [None, Some(0), Some(1)] {
            for ready in [false, true] {
                for token in [false, true] {
                    let mut g = position(owner);
                    let ori = unit(&mut g, owner, 430, lane);
                    let c = contribution(&g, ori);
                    assert_eq!((c.might, c.damage, c.prevents_combat_damage), (4, 0, true));
                    gear(&mut g, 1 - owner, lane, None, token, ready);
                    let elsewhere = if lane == Some(1) { Some(0) } else { Some(1) };
                    gear(&mut g, owner, elsewhere, None, token, ready);
                    assert_eq!(contribution(&g, ori).damage, 0);
                    gear(&mut g, owner, lane, None, token, ready);
                    let c = contribution(&g, ori);
                    assert_eq!((c.might, c.damage, c.prevents_combat_damage), (4, 4, false));
                    g.unit_mut(ori).unwrap().stunned = true;
                    let c = contribution(&g, ori);
                    assert_eq!(
                        (c.damage, c.stunned, c.prevents_combat_damage),
                        (0, true, false)
                    );
                    g.players[owner].gear.clear();
                    let c = contribution(&g, ori);
                    assert_eq!(
                        (c.damage, c.stunned, c.prevents_combat_damage),
                        (0, true, true)
                    );
                }
            }
        }
    }
}

#[test]
fn orianna_uses_attached_gear_location_even_when_its_printed_abilities_are_inactive() {
    for owner in 0..2 {
        for same_bearer in [false, true] {
            let mut g = position(owner);
            let ori = unit(&mut g, owner, 430, Some(0));
            let bearer = if same_bearer {
                ori
            } else {
                unit(&mut g, owner, 71, Some(0))
            };
            // The physical attachment follows its bearer, not its stale lane field.
            gear(&mut g, owner, Some(1), Some(id(bearer)), false, false);
            assert!(!contribution(&g, ori).prevents_combat_damage);
            g.recall_unit(owner, id(bearer), false, false);
            assert_eq!(g.gear_location(owner, g.players[owner].gear[0].id), None);
            assert_eq!(contribution(&g, ori).prevents_combat_damage, !same_bearer);
        }
    }
}

#[test]
fn orianna_equipping_and_moving_an_ally_changes_combat_permission_without_targeting_her() {
    for owner in 0..2 {
        let mut g = position(owner);
        g.battlefield_owner = [Some(owner), Some(owner)];
        let ori = unit(&mut g, owner, 430, Some(0));
        let ally = unit(&mut g, owner, 71, None);
        cast(&mut g, 3, vec![]);
        settle(&mut g);
        let boots = g.players[owner].gear[0].id;
        assert_eq!(contribution(&g, ori).damage, 0);
        g.apply(Action::Equip {
            gear_id: boots,
            target: ally,
        })
        .unwrap();
        settle(&mut g);
        assert_eq!(contribution(&g, ori).damage, 0);
        g.apply(Action::Move {
            unit_id: id(ally),
            lane: Some(0),
        })
        .unwrap();
        settle(&mut g);
        assert_eq!(contribution(&g, ori).damage, 4);
        // Ride the Wind moves the equipped ally away; Orianna loses permission.
        cast(&mut g, 14, vec![ally]);
        settle(&mut g);
        assert_eq!(g.unit_ref(ally).unwrap().lane, None);
        assert_eq!(contribution(&g, ori).damage, 0);
    }
}

#[test]
fn orianna_shield_and_lethal_threshold_apply_even_without_combat_damage() {
    for owner in 0..2 {
        for defending in [false, true] {
            let actor = if defending { 1 - owner } else { owner };
            let mut g = position(actor);
            let ori = unit(&mut g, owner, 430, defending.then_some(0));
            let enemy = unit(&mut g, 1 - owner, 70, (!defending).then_some(0));
            let attacker = if defending { enemy } else { ori };
            g.apply(Action::Move {
                unit_id: id(attacker),
                lane: Some(0),
            })
            .unwrap();
            settle(&mut g);
            let c = contribution(&g, ori);
            assert_eq!((c.might, c.damage), (if defending { 5 } else { 4 }, 0));
            g.apply(Action::Pass).unwrap();
            g.apply(Action::Pass).unwrap();
            let Some(Decision::CombatDamage { player, total, .. }) = g.decision else {
                panic!("{:?}", g.decision)
            };
            assert_eq!(player, 1 - owner);
            assert_eq!(total, 10);
            g.apply(Action::AssignDamage {
                assignments: vec![(id(ori), total)],
            })
            .unwrap();
            assert!(g.unit_ref(ori).is_none());
            assert_eq!(g.unit_ref(enemy).unwrap().damage, 0);
        }
    }
}

#[test]
fn orianna_last_gear_can_be_removed_in_response_before_combat_damage() {
    for owner in 0..2 {
        let mut g = position(owner);
        let ori = unit(&mut g, owner, 430, None);
        let enemy = unit(&mut g, 1 - owner, 70, Some(0));
        let boots = gear(&mut g, owner, None, Some(id(ori)), false, true);
        g.apply(Action::Move {
            unit_id: id(ori),
            lane: Some(0),
        })
        .unwrap();
        settle(&mut g);
        assert!(!contribution(&g, ori).prevents_combat_damage);
        g.apply(Action::Pass).unwrap(); // Give the defender focus.
        cast(
            &mut g,
            41,
            vec![Target::Gear {
                player: owner,
                gear_id: boots,
            }],
        ); // Salvage is an Action that can kill gear during a showdown.
        settle(&mut g);
        assert!(g.players[owner].gear.is_empty());
        assert_eq!(contribution(&g, ori).damage, 0);
        for _ in 0..4 {
            if g.decision.is_some() {
                break;
            }
            g.apply(Action::Pass).unwrap();
        }
        let Some(Decision::CombatDamage { player, total, .. }) = g.decision else {
            panic!("{:?}", g.decision)
        };
        assert_eq!(player, 1 - owner);
        g.apply(Action::AssignDamage {
            assignments: vec![(id(ori), total)],
        })
        .unwrap();
        assert_eq!(g.unit_ref(enemy).unwrap().damage, 0);
    }
}

#[test]
fn orianna_fight_damage_uses_might_and_gear_condition_does_not_leak_to_tokens() {
    for owner in 0..2 {
        let mut g = position(owner);
        let ori = unit(&mut g, owner, 430, None);
        let enemy = unit(&mut g, 1 - owner, 71, None);
        cast(&mut g, 82, vec![ori, enemy]);
        settle(&mut g);
        assert_eq!(g.unit_ref(enemy).unwrap().damage, 4);
        assert!(g.unit_ref(ori).is_none());
    }
    let mut g = position(0);
    let ori = unit(&mut g, 0, 430, None);
    g.unit_mut(ori).unwrap().token = true;
    g.unit_mut(ori).unwrap().buff = 7;
    let c = contribution(&g, ori);
    assert_eq!(c.damage, 7);
    assert!(!c.prevents_combat_damage);
    assert_eq!(
        g.unit_combat_gear_requirement(g.unit_ref(ori).unwrap()),
        None
    );
}
