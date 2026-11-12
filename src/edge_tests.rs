//! Edge of Night's erratum keeps its Hidden origin after gear cleanup recalls it.
use super::*;

fn position(owner: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 432432, Difficulty::Tactical);
    g.choose_starting_player(owner).unwrap();
    g.start_match().unwrap();
    g.battlefields = [2, 2];
    g.battlefield_owner = [Some(owner), Some(owner)];
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
    for legend in &mut g.legend {
        legend.ready = false;
    }
    g
}
fn unit(g: &mut Game, owner: usize, card: usize, lane: Option<usize>) -> Target {
    let unit_id = g.create_unit(owner, card, false, 0);
    g.players[owner]
        .units
        .iter_mut()
        .find(|u| u.id == unit_id)
        .unwrap()
        .lane = lane;
    Target::Unit {
        player: owner,
        unit_id,
    }
}
fn id(target: Target) -> u32 {
    let Target::Unit { unit_id, .. } = target else {
        panic!()
    };
    unit_id
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
            other => panic!("Unexpected {other:?}"),
        }
    }
    panic!("chain did not settle")
}
fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action).is_err());
    assert_eq!(format!("{g:?}"), before);
}
fn hide(g: &mut Game, lane: usize) -> u32 {
    let p = g.priority();
    g.players[p].hand = vec![432];
    g.apply(Action::Hide {
        hand_index: 0,
        lane,
    })
    .unwrap();
    g.players[p].hidden.last().unwrap().id
}
fn reveal(g: &mut Game, lane: usize) -> u32 {
    let player = g.priority();
    let hidden_id = hide(g, lane);
    g.turn += 1;
    g.apply(Action::PlayHidden {
        hidden_id,
        targets: vec![],
    })
    .unwrap_or_else(|error| {
        panic!(
            "reveal owner={player} lane={lane} priority={} decision={:?} chain={:?}: {error}",
            g.priority(),
            g.decision,
            g.chain
        )
    });
    let gear = g.players[player].gear.last().unwrap();
    assert_eq!(gear.card_id, 432);
    assert_eq!(gear.lane, None);
    assert_eq!(gear.attached_to, None);
    gear.id
}
fn choose(g: &mut Game, target: Target) {
    assert!(matches!(
        g.decision,
        Some(Decision::TriggerTargets { card_id: 432, .. })
    ));
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![target],
    })
    .unwrap();
}

#[test]
fn edge_hand_play_and_ordinary_equip_have_separate_costs_without_auto_attach() {
    for owner in 0..2 {
        let mut g = position(owner);
        let target = unit(&mut g, owner, 71, Some(1));
        g.players[owner].hand = vec![432];
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        })
        .unwrap();
        assert_eq!(g.players[owner].energy, 17);
        assert!(g.decision.is_none() && g.chain.is_empty());
        let gear_id = g.players[owner].gear[0].id;
        assert_eq!(g.players[owner].gear[0].lane, None);
        g.apply(Action::Equip { gear_id, target }).unwrap();
        assert_eq!(g.players[owner].power_pool[domain_index(Domain::Chaos)], 9);
        assert_eq!(g.gear_might_bonus(id(target)).unwrap(), 0);
        settle(&mut g);
        assert_eq!(g.gear_might_bonus(id(target)).unwrap(), 2);
        assert_eq!(g.gear_location(owner, gear_id), Some(1));
        reject(&mut g, Action::Equip { gear_id, target });
    }
}

#[test]
fn edge_hidden_is_private_costs_one_power_and_waits_until_a_later_turn() {
    for owner in 0..2 {
        let mut g = position(owner);
        let target = unit(&mut g, owner, 71, Some(0));
        let before = g.players[owner].power_pool.iter().sum::<u16>();
        let hidden_id = hide(&mut g, 0);
        assert_eq!(g.players[owner].power_pool.iter().sum::<u16>(), before - 1);
        assert_eq!(g.players[owner].energy, 20);
        assert!(g.players[owner].gear.is_empty());
        assert_eq!(g.hidden_visible_card(owner, owner, hidden_id), Some(432));
        assert_eq!(g.hidden_visible_card(1 - owner, owner, hidden_id), None);
        reject(
            &mut g,
            Action::PlayHidden {
                hidden_id,
                targets: vec![],
            },
        );
        g.turn += 1;
        g.players[owner].energy = 0;
        g.players[owner].energy_pool = 0;
        g.players[owner].power_pool = [0; 6];
        reject(
            &mut g,
            Action::PlayHidden {
                hidden_id,
                targets: vec![target],
            },
        );
        g.apply(Action::PlayHidden {
            hidden_id,
            targets: vec![],
        })
        .unwrap();
        choose(&mut g, target);
        assert_eq!(g.players[owner].energy, 0);
        settle(&mut g);
        assert_eq!(g.gear_might_bonus(id(target)).unwrap(), 2);
    }
}

#[test]
fn edge_recalled_source_still_targets_only_a_friendly_unit_at_the_hidden_origin() {
    for owner in 0..2 {
        for lane in 0..2 {
            let mut g = position(owner);
            let local = unit(&mut g, owner, 71, Some(lane));
            let elsewhere = unit(&mut g, owner, 71, Some(1 - lane));
            let base = unit(&mut g, owner, 71, None);
            let enemy = unit(&mut g, 1 - owner, 71, None);
            let gear_id = reveal(&mut g, lane);
            g.players[1 - owner]
                .units
                .iter_mut()
                .find(|u| u.id == id(enemy))
                .unwrap()
                .lane = Some(lane);
            for target in [elsewhere, base, enemy] {
                reject(
                    &mut g,
                    Action::ChooseTriggerTargets {
                        targets: vec![target],
                    },
                );
            }
            choose(&mut g, local);
            assert_eq!(g.chain.last().unwrap().targets, vec![local]);
            assert_eq!(g.gear_might_bonus(id(local)).unwrap(), 0);
            settle(&mut g);
            assert_eq!(g.gear_might_bonus(id(local)).unwrap(), 2);
            assert_eq!(g.gear_location(owner, gear_id), Some(lane));
        }
    }
}

#[test]
fn edge_unfinalizable_play_trigger_does_not_attach_elsewhere() {
    let mut g = position(0);
    let local = unit(&mut g, 0, 71, Some(0));
    unit(&mut g, 0, 71, None);
    reveal(&mut g, 0);
    g.return_board_targets(&[local]);
    // Exercise the filter used when a pending trigger begins finalization.
    g.discard_unfinalizable_trigger().unwrap();
    assert!(g.decision.is_none() && g.chain.is_empty());
    assert_eq!(g.players[0].gear[0].attached_to, None);
    assert_eq!(g.players[0].gear[0].lane, None);
}

#[test]
fn edge_source_departure_target_departure_and_target_movement_prevent_attachment() {
    for owner in 0..2 {
        for scenario in 0..3 {
            let mut g = position(owner);
            let target = unit(&mut g, owner, 71, Some(0));
            let gear_id = reveal(&mut g, 0);
            choose(&mut g, target);
            match scenario {
                0 => {
                    g.return_board_targets(&[Target::Gear {
                        player: owner,
                        gear_id,
                    }]);
                }
                1 => {
                    g.return_board_targets(&[target]);
                }
                _ => {
                    g.players[owner]
                        .units
                        .iter_mut()
                        .find(|u| u.id == id(target))
                        .unwrap()
                        .lane = Some(1);
                }
            }
            settle(&mut g);
            assert!(
                g.players[owner]
                    .gear
                    .iter()
                    .all(|gear| gear.attached_to.is_none())
            );
        }
    }
}

#[test]
fn edge_opponent_not_so_fast_cannot_counter_an_attachment_choosing_only_our_unit() {
    let mut g = position(0);
    let target = unit(&mut g, 0, 71, Some(0));
    let gear_id = reveal(&mut g, 0);
    choose(&mut g, target);
    let chain_id = g.chain.last().unwrap().id;
    g.apply(Action::Pass).unwrap();
    g.players[1].hand = vec![12];
    reject(
        &mut g,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Spell { chain_id }],
        },
    );
    settle(&mut g);
    assert_eq!(g.players[0].gear[0].id, gear_id);
    assert_eq!(g.players[0].gear[0].attached_to, Some(id(target)));
    assert_eq!(g.gear_might_bonus(id(target)).unwrap(), 2);
}

#[test]
fn edge_enables_nearby_orianna_only_after_attachment_and_detaches_on_bearer_departure() {
    let mut g = position(0);
    let orianna = unit(&mut g, 0, 430, Some(0));
    let bearer = unit(&mut g, 0, 71, Some(0));
    let gear_id = reveal(&mut g, 0);
    assert_eq!(
        g.unit_combat_contribution(g.unit_ref(orianna).unwrap())
            .unwrap()
            .damage,
        0
    );
    choose(&mut g, bearer);
    settle(&mut g);
    assert_eq!(
        g.unit_combat_contribution(g.unit_ref(orianna).unwrap())
            .unwrap()
            .damage,
        4
    );
    g.return_board_targets(&[bearer]);
    g.after_resolution().unwrap();
    assert_eq!(g.gear_location(0, gear_id), None);
    assert_eq!(
        g.unit_combat_contribution(g.unit_ref(orianna).unwrap())
            .unwrap()
            .damage,
        0
    );
}

#[test]
fn edge_chooses_irelia_then_ready_and_might_triggers_resolve_before_attachment() {
    let mut g = position(0);
    g.legend[0] = crate::legends::LegendState::from_legend(0).unwrap();
    let irelia = unit(&mut g, 0, 0, Some(0));
    reveal(&mut g, 0);
    choose(&mut g, irelia);
    let mut offered = false;
    for _ in 0..80 {
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
    assert!(g.unit_ref(irelia).unwrap().ready);
    assert_eq!(g.unit_ref(irelia).unwrap().temporary_buff, 2);
    assert_eq!(g.gear_might_bonus(id(irelia)).unwrap(), 2);
}
