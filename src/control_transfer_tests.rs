//! Control instructions keep physical identity. Cleanup waits until the whole
//! effect finishes; these operation tests do not claim control-card admission.
use super::*;
use test_support::{position, settle_unopposed_chain};

fn field_unit(g: &mut Game, player: usize, card: CardId) -> Target {
    let unit_id = g.create_unit(player, card, false, 0);
    let target = Target::Unit { player, unit_id };
    let unit = g.unit_mut(target).unwrap();
    unit.lane = Some(0);
    unit.ready = true;
    target
}

fn id(target: Target) -> u32 {
    let Target::Unit { unit_id, .. } = target else {
        unreachable!()
    };
    unit_id
}

#[test]
fn control_transfer_preserves_the_object_gear_owner_and_turn_state() {
    for owner in 0..2 {
        let controller = 1 - owner;
        let mut g = position(owner, 0, 468401);
        let source = field_unit(&mut g, owner, 71);
        let unit_id = id(source);
        {
            let u = g.unit_mut(source).unwrap();
            u.damage = 2;
            u.buff = 4;
            u.temporary_buff = 3;
            u.protected = true;
            u.stunned = true;
            u.banish_on_death = true;
        }
        g.empowered_units.push(unit_id);
        g.buffed_units.push(unit_id);
        g.granted_temporary_units.push(unit_id);
        g.immobile_units.push((owner, unit_id));
        g.movement_locks.push((owner, unit_id));
        g.moved_units.push(unit_id);
        g.gear_play_at(owner, 203, None).unwrap();
        let gear_id = g.players[owner].gear.last().unwrap().id;
        assert!(g.attach_equipment(owner, gear_id, unit_id));
        let before = format!("{:?}", g.unit_ref(source).unwrap());
        let history = (g.cards_played, g.trigger_state.clone(), g.next_unit_id);
        let target = g
            .take_control_of_battlefield_unit(controller, source)
            .unwrap();
        assert_eq!(format!("{:?}", g.unit_ref(target).unwrap()), before);
        assert_eq!(
            (g.cards_played, g.trigger_state.clone(), g.next_unit_id),
            history
        );
        assert!(g.chain.is_empty() && g.pending_triggers.is_empty());
        assert_eq!(g.board_owner(target), Some(owner));
        assert_eq!(g.gear_controller(gear_id), Some(owner));
        assert_eq!(g.players[owner].gear[0].attached_to, Some(unit_id));
        assert!(g.movement_prevented(controller, controller, unit_id));
        assert!(g.unit_moved_this_turn(controller, unit_id));
        assert!(g.empowered_units.contains(&unit_id) && g.buffed_units.contains(&unit_id));
        assert!(g.granted_temporary_units.contains(&unit_id));
        let restored = g.take_control_of_battlefield_unit(owner, target).unwrap();
        assert_eq!(restored, source);
        assert!(g.board_owner_overrides.is_empty());
        assert_eq!(format!("{:?}", g.unit_ref(source).unwrap()), before);
    }
}

#[test]
fn invalid_control_instructions_and_same_controller_are_no_ops() {
    let mut g = position(0, 0, 468402);
    let target = field_unit(&mut g, 0, 71);
    for (controller, target) in [
        (2, target),
        (
            1,
            Target::Unit {
                player: 1,
                unit_id: id(target),
            },
        ),
        (
            1,
            Target::Unit {
                player: 0,
                unit_id: 9999,
            },
        ),
        (1, Target::Battlefield { lane: 0 }),
    ] {
        let before = format!("{g:?}");
        assert_eq!(g.take_control_of_battlefield_unit(controller, target), None);
        assert_eq!(format!("{g:?}"), before);
    }
    let before = format!("{g:?}");
    assert_eq!(g.take_control_of_battlefield_unit(0, target), Some(target));
    assert_eq!(format!("{g:?}"), before);
    g.recall_unit(0, id(target), false, false);
    let before = format!("{g:?}");
    assert_eq!(g.take_control_of_battlefield_unit(1, target), None);
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn transfer_stages_control_but_recall_before_cleanup_prevents_a_new_showdown() {
    for actor in 0..2 {
        for recall in [false, true] {
            let mut g = position(actor, 0, 468403);
            let target = field_unit(&mut g, 1 - actor, 71);
            g.battlefield_owner[0] = Some(1 - actor);
            let transferred = g.take_control_of_battlefield_unit(actor, target).unwrap();
            assert_eq!(g.pending_entries, [(0, actor)]);
            assert!(g.showdown.is_none());
            assert_eq!(g.battlefield_owner[0], Some(1 - actor));
            assert_eq!(g.players[actor].score, 0);
            if recall {
                g.recall_unit(actor, id(transferred), false, false);
            }
            g.after_resolution().unwrap();
            assert_eq!(g.players[actor].score, 0);
            if recall {
                assert!(g.showdown.is_none() && g.pending_entries.is_empty());
                assert_eq!(g.battlefield_owner[0], None);
                assert!(g.unit_ref(transferred).unwrap().ready);
            } else {
                assert!(
                    g.showdown
                        .as_ref()
                        .is_some_and(|w| w.lane == 0 && !w.combat && w.attacker == actor)
                );
                g.apply(Action::Pass).unwrap();
                g.apply(Action::Pass).unwrap();
                assert_eq!(g.battlefield_owner[0], Some(actor));
                assert_eq!(g.players[actor].score, 1);
            }
        }
    }
}

#[test]
fn control_change_checks_attack_and_defense_once_each_per_combat() {
    for actor in 0..2 {
        for recall in [false, true] {
            let mut g = position(actor, 0, 468404);
            let source = field_unit(&mut g, actor, 414);
            let ally = field_unit(&mut g, actor, 71);
            let enemy = field_unit(&mut g, 1 - actor, 71);
            g.unit_mut(ally).unwrap().buff = 100;
            g.unit_mut(enemy).unwrap().buff = 100;
            g.unit_mut(source).unwrap().lane = None;
            g.battlefield_owner[0] = Some(1 - actor);
            g.apply(Action::Move {
                unit_id: id(source),
                lane: Some(0),
            })
            .unwrap();
            settle_unopposed_chain(&mut g);
            assert!(g.unit_ref(enemy).unwrap().damage > 0);
            assert_eq!(g.unit_ref(ally).unwrap().damage, 0);
            let transferred = g
                .take_control_of_battlefield_unit(1 - actor, source)
                .unwrap();
            assert!(
                g.chain.is_empty(),
                "cleanup must wait until all instructions finish"
            );
            if recall {
                g.recall_unit(1 - actor, id(source), false, false);
            }
            let mut clone = g.clone();
            g.after_resolution().unwrap();
            clone.after_resolution().unwrap();
            settle_unopposed_chain(&mut g);
            settle_unopposed_chain(&mut clone);
            assert_eq!(format!("{g:?}"), format!("{clone:?}"));
            let damage = g.unit_ref(ally).unwrap().damage;
            assert_eq!(
                damage > 0,
                !recall,
                "first defense is a separate trigger check"
            );
            if !recall {
                let original = g
                    .take_control_of_battlefield_unit(actor, transferred)
                    .unwrap();
                g.after_resolution().unwrap();
                assert!(g.chain.is_empty(), "a second attack does not retrigger");
                g.take_control_of_battlefield_unit(1 - actor, original)
                    .unwrap();
                g.after_resolution().unwrap();
                assert!(g.chain.is_empty(), "a second defense does not retrigger");
                assert_eq!(g.unit_ref(ally).unwrap().damage, damage);
            }
        }
    }
}

#[test]
fn control_change_recalculates_combat_might_before_lethal_cleanup() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 468405);
        let source = field_unit(&mut g, actor, 56); // Inferna: 1 Might, Assault 2.
        field_unit(&mut g, actor, 71);
        field_unit(&mut g, 1 - actor, 71);
        g.unit_mut(source).unwrap().lane = None;
        g.battlefield_owner[0] = Some(1 - actor);
        g.apply(Action::Move {
            unit_id: id(source),
            lane: Some(0),
        })
        .unwrap();
        settle_unopposed_chain(&mut g);
        g.unit_mut(source).unwrap().damage = 2;
        assert_eq!(g.unit_might(g.unit_ref(source).unwrap()).unwrap(), 3);
        let transferred = g
            .take_control_of_battlefield_unit(1 - actor, source)
            .unwrap();
        assert!(
            g.unit_ref(transferred).is_some(),
            "no mid-instruction cleanup"
        );
        g.after_resolution().unwrap();
        assert!(g.unit_ref(transferred).is_none());
        assert_eq!(g.players[actor].discard, [56]);
        assert!(g.players[1 - actor].discard.is_empty());
        assert!(g.board_owner_overrides.is_empty());
    }
}

#[test]
fn vex_restricts_the_original_player_even_after_transfer() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 468406);
        let source = field_unit(&mut g, actor, 71);
        let target = g
            .take_control_of_battlefield_unit(1 - actor, source)
            .unwrap();
        g.resolve_trigger(
            1 - actor,
            20,
            TriggeredAbility::Stun {
                player: actor,
                unit_id: id(source),
            },
        )
        .unwrap();
        assert!(g.unit_ref(target).unwrap().stunned);
        assert!(g.movement_prevented(actor, 1 - actor, id(source)));
        assert!(!g.movement_prevented(1 - actor, 1 - actor, id(source)));
        g.recall_unit(1 - actor, id(source), false, false);
        assert_eq!(g.unit_ref(target).unwrap().lane, None);
        assert!(g.unit_ref(target).unwrap().ready);
    }
}
