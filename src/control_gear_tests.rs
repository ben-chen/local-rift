//! CR718/719: an equipment stays controlled by its original player while its
//! effect text belongs to the bearer. Initial foreign control is authored;
//! these fixtures do not admit Possession or Conscription by themselves.
use super::*;
use crate::game::board_targets::BoardLocation;
use crate::game::test_support::{decline_legend_action, position};

fn equipped(owner: usize, card: CardId) -> (Game, Target, u32) {
    let controller = 1 - owner;
    let mut g = position(controller, 0, 468101);
    let unit_id = g.create_unit(owner, 71, false, 0);
    g.gear_play_at(owner, card, None).unwrap();
    let gear_id = g.players[owner].gear.last().unwrap().id;
    assert!(g.attach_equipment(owner, gear_id, unit_id));
    let mut unit = g.players[owner].units.pop().unwrap();
    unit.ready = true;
    g.players[controller].units.push(unit);
    g.board_owner_overrides.insert(unit_id, owner);
    (
        g,
        Target::Unit {
            player: controller,
            unit_id,
        },
        gear_id,
    )
}

fn settle(g: &mut Game) {
    for _ in 0..100 {
        if g.chain.is_empty() && g.decision.is_none() && g.showdown.is_none() {
            return;
        }
        let action = if let Some(action) = decline_legend_action(g) {
            action
        } else if let Some(Decision::OrderTriggers { options, .. }) = &g.decision {
            Action::OrderTrigger {
                trigger_id: options[0].id,
            }
        } else {
            assert!(g.decision.is_none(), "unexpected choice {:?}", g.decision);
            Action::Pass
        };
        g.apply(action).unwrap();
    }
    panic!("fixture did not settle");
}

#[test]
fn borrowed_bearer_keeps_equipment_stats_and_the_opposing_base_is_explicit() {
    for owner in 0..2 {
        let (mut g, target, gear_id) = equipped(owner, 3);
        let Target::Unit { player, unit_id } = target else {
            unreachable!()
        };
        let gear_target = Target::Gear {
            player: owner,
            gear_id,
        };
        let facts = g.board_target_facts(gear_target).unwrap();
        assert_eq!(facts.controller, owner);
        assert_eq!(facts.location, BoardLocation::Base { player });
        assert_eq!(g.board_owner(gear_target), Some(owner));
        assert_eq!(
            g.attached_gear(unit_id)
                .map(|(p, gear)| (p, gear.id))
                .collect::<Vec<_>>(),
            [(owner, gear_id)]
        );
        assert_eq!(g.gear_might_bonus(unit_id).unwrap(), 2);
        assert!(
            g.unit_keywords(g.unit_ref(target).unwrap())
                .unwrap()
                .ganking
        );
        g.unit_mut(target).unwrap().lane = Some(1);
        assert_eq!(
            g.gear_board_location(owner, gear_id),
            Some(BoardLocation::Battlefield { lane: 1 })
        );
        g.recall_unit(player, unit_id, false, false);
        assert_eq!(
            g.gear_board_location(owner, gear_id),
            Some(BoardLocation::Base { player })
        );
        assert!(g.players[owner].gear[0].attached_to.is_some());
    }
}

#[test]
fn all_bearer_departures_detach_foreign_equipment_before_cleanup_recalls_it() {
    for owner in 0..2 {
        for lane in [None, Some(1)] {
            for departure in 0..3 {
                let (mut g, target, gear_id) = equipped(owner, 3);
                g.unit_mut(target).unwrap().lane = lane;
                g.players[owner].gear[0].ready = false;
                match departure {
                    0 => {
                        g.return_unit_to_hand(target);
                    }
                    1 => {
                        g.banish_unit_directly(target);
                    }
                    _ => g.kill_unit(target).unwrap(),
                }
                assert_eq!(g.players[owner].gear[0].attached_to, None);
                assert_eq!(
                    g.gear_board_location(owner, gear_id),
                    Some(BoardLocation::from_lane(1 - owner, lane))
                );
                g.cleanup_gear_locations();
                assert_eq!(
                    g.gear_board_location(owner, gear_id),
                    Some(BoardLocation::Base { player: owner })
                );
                assert!(!g.players[owner].gear[0].ready);
                assert!(g.detached_gear_bases.is_empty());
            }
        }
    }
}

#[test]
fn foreign_guardian_angel_protects_bearer_but_opposing_hourglass_does_not() {
    for owner in 0..2 {
        let (mut g, target, gear_id) = equipped(owner, 10);
        g.gear_play_at(owner, 21, None).unwrap();
        g.unit_mut(target).unwrap().lane = Some(0);
        g.unit_mut(target).unwrap().damage = 2;
        g.kill_unit(target).unwrap();
        let unit = g.unit_ref(target).expect("Guardian Angel keeps the bearer");
        assert_eq!((unit.lane, unit.damage, unit.ready), (None, 0, false));
        assert_eq!(g.players[owner].discard, [10]);
        assert_eq!(g.gear_controller(gear_id), None);
        assert_eq!(g.board_owner(target), Some(owner));
        g.kill_unit(target).unwrap();
        assert!(g.unit_ref(target).is_none());
        assert_eq!(g.players[owner].discard, [10, 71]);
        assert_eq!(g.players[owner].gear[0].card_id, 21);
    }
}

#[test]
fn granted_conditions_use_bearers_xp_and_friendly_units() {
    for owner in 0..2 {
        let controller = 1 - owner;
        let (mut g, target, _) = equipped(owner, 330);
        g.players[owner].xp = 20;
        g.players[controller].xp = 0;
        let printed = u64::from(g.catalog[71].might);
        assert_eq!(
            g.unit_might(g.unit_ref(target).unwrap()).unwrap(),
            printed + 1
        );
        g.players[owner].xp = 0;
        g.players[controller].xp = 3;
        assert_eq!(
            g.unit_might(g.unit_ref(target).unwrap()).unwrap(),
            printed + 2
        );

        let (mut g, target, _) = equipped(owner, 445);
        g.unit_mut(target).unwrap().lane = Some(0);
        let before = g.unit_might(g.unit_ref(target).unwrap()).unwrap();
        let other = g.create_unit(owner, 71, false, 0);
        g.unit_mut(Target::Unit {
            player: owner,
            unit_id: other,
        })
        .unwrap()
        .lane = Some(0);
        assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), before);
        let friendly = g.create_unit(controller, 71, false, 0);
        g.unit_mut(Target::Unit {
            player: controller,
            unit_id: friendly,
        })
        .unwrap()
        .lane = Some(0);
        assert_eq!(
            g.unit_might(g.unit_ref(target).unwrap()).unwrap(),
            before + 2
        );
    }
}

#[test]
fn granted_move_and_death_abilities_belong_to_bearers_controller() {
    for owner in 0..2 {
        let controller = 1 - owner;
        let (mut g, target, _) = equipped(owner, 205);
        let Target::Unit { unit_id, .. } = target else {
            unreachable!()
        };
        g.apply(Action::Move {
            unit_id,
            lane: Some(0),
        })
        .unwrap();
        settle(&mut g);
        assert_eq!(g.unit_ref(target).unwrap().temporary_buff, 2);

        let (mut g, target, _) = equipped(owner, 199);
        let decks = [g.players[0].deck.len(), g.players[1].deck.len()];
        g.kill_unit(target).unwrap();
        g.after_resolution().unwrap();
        settle(&mut g);
        assert_eq!(g.players[controller].deck.len(), decks[controller] - 1);
        assert_eq!(g.players[owner].deck.len(), decks[owner]);
        assert_eq!(g.players[owner].discard, [71]);
        assert_eq!(g.players[owner].gear[0].attached_to, None);
    }
}

#[test]
fn last_rites_is_offered_to_bearer_controller_and_dangling_gear_has_no_location() {
    for owner in 0..2 {
        let (mut g, target, gear_id) = equipped(owner, 38);
        let Target::Unit { player, unit_id } = target else {
            unreachable!()
        };
        g.gear_conquer_hold(player, unit_id);
        assert!(
            g.pending_triggers
                .iter()
                .any(|trigger| trigger.player == player && trigger.option.card_id == 38)
        );
        assert!(
            !g.pending_triggers
                .iter()
                .any(|trigger| trigger.player == owner && trigger.option.card_id == 38)
        );
        // An invalid external fixture must not silently put an attached gear in base.
        g.players[player].units.clear();
        assert_eq!(g.gear_board_location(owner, gear_id), None);
        assert!(
            g.board_target_facts(Target::Gear {
                player: owner,
                gear_id
            })
            .is_none()
        );
    }
}

#[test]
fn friendly_gear_here_checks_actual_base_and_can_be_attached_to_an_enemy() {
    for owner in 0..2 {
        let (mut g, target, _) = equipped(owner, 3);
        let guard_id = g.create_unit(owner, 430, false, 0);
        let guard = Target::Unit {
            player: owner,
            unit_id: guard_id,
        };
        assert_eq!(
            g.unit_combat_gear_requirement(g.unit_ref(guard).unwrap()),
            Some(false)
        );
        g.unit_mut(guard).unwrap().lane = Some(1);
        g.unit_mut(target).unwrap().lane = Some(1);
        assert_eq!(
            g.unit_combat_gear_requirement(g.unit_ref(guard).unwrap()),
            Some(true)
        );
    }
}

#[test]
fn explicit_reattachment_keeps_gear_control_and_changes_only_the_bearer() {
    for owner in 0..2 {
        let (mut g, target, gear_id) = equipped(owner, 3);
        let Target::Unit { unit_id, .. } = target else {
            unreachable!()
        };
        let next_bearer = g.create_unit(owner, 71, false, 0);
        // This is the Attach transition after an explicit permission such as
        // Weaponmaster. Ordinary Equip remains inactive while attached.
        assert!(g.attach_equipment(owner, gear_id, next_bearer));
        assert_eq!(g.gear_controller(gear_id), Some(owner));
        assert_eq!(g.gear_might_bonus(unit_id).unwrap(), 0);
        assert_eq!(g.gear_might_bonus(next_bearer).unwrap(), 2);
        assert_eq!(
            g.gear_board_location(owner, gear_id),
            Some(BoardLocation::Base { player: owner })
        );
        assert!(!g.attach_equipment(owner, gear_id, next_bearer));
    }
}

#[test]
fn pending_granted_ability_keeps_controller_and_finds_bearer_after_control_changes() {
    for owner in 0..2 {
        let (mut g, target, _) = equipped(owner, 205);
        let Target::Unit { player, unit_id } = target else {
            unreachable!()
        };
        g.apply(Action::Move {
            unit_id,
            lane: Some(0),
        })
        .unwrap();
        assert!(
            g.chain
                .iter()
                .any(|item| item.card_id == 205 && item.player == player)
        );
        let index = g.players[player]
            .units
            .iter()
            .position(|u| u.id == unit_id)
            .unwrap();
        let unit = g.players[player].units.remove(index);
        g.players[owner].units.push(unit);
        g.board_owner_overrides.remove(&unit_id);
        assert!(
            g.chain
                .iter()
                .any(|item| item.card_id == 205 && item.player == player)
        );
        settle(&mut g);
        assert_eq!(
            g.unit_ref(Target::Unit {
                player: owner,
                unit_id
            })
            .unwrap()
            .temporary_buff,
            2
        );
    }
}

#[test]
fn first_each_turn_is_not_reset_by_a_change_of_controller() {
    for owner in 0..2 {
        let mut g = position(owner, 0, 468102);
        let watcher = g.create_unit(owner, 214, false, 0);
        let victim = g.create_unit(owner, 71, false, 0);
        g.kill_unit(Target::Unit {
            player: owner,
            unit_id: victim,
        })
        .unwrap();
        g.after_resolution().unwrap();
        settle(&mut g);
        let index = g.players[owner]
            .units
            .iter()
            .position(|u| u.id == watcher)
            .unwrap();
        let unit = g.players[owner].units.remove(index);
        g.players[1 - owner].units.push(unit);
        g.board_owner_overrides.insert(watcher, owner);
        let decks = [g.players[0].deck.len(), g.players[1].deck.len()];
        let victim = g.create_unit(1 - owner, 71, false, 0);
        g.kill_unit(Target::Unit {
            player: 1 - owner,
            unit_id: victim,
        })
        .unwrap();
        g.after_resolution().unwrap();
        settle(&mut g);
        assert_eq!([g.players[0].deck.len(), g.players[1].deck.len()], decks);
    }
}
