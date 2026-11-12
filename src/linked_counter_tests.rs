//! Physical target relationships, Repulse's dependent slots, and Not So Fast.
use super::*;
const REPULSE: usize = 442;
fn position(actor: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 442442, Difficulty::Tactical);
    g.choose_starting_player(actor).unwrap();
    g.start_match().unwrap();
    g.battlefields = vec![9, 9];
    g.battlefield_owner = vec![None; 2];
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
        p.energy = 100;
        p.energy_pool = 100;
        p.power_pool = [20; 6];
        p.any_power = 0;
    }
    for legend in &mut g.legend {
        legend.ready = false;
    }
    g
}
fn unit(g: &mut Game, owner: usize, card: CardId, lane: Option<usize>) -> Target {
    let id = g.create_unit(owner, card, false, 0);
    let u = g.players[owner].units.last_mut().unwrap();
    u.lane = lane;
    u.ready = true;
    Target::Unit {
        player: owner,
        unit_id: id,
    }
}
fn cast(g: &mut Game, card: CardId, targets: Vec<Target>) -> u32 {
    let p = g.priority();
    let hand_index = g.players[p].hand.len();
    g.players[p].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
    g.chain
        .iter()
        .rev()
        .find(|item| item.card_id == card && item.ability.is_none())
        .unwrap()
        .id
}
fn focus(g: &mut Game, p: usize) {
    if g.priority() != p {
        g.apply(Action::Pass).unwrap();
    }
    assert_eq!(g.priority(), p);
}
fn pair(g: &mut Game) {
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn settle(g: &mut Game) {
    for _ in 0..100 {
        let action = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            None if !g.chain.is_empty() => Action::Pass,
            None => return,
            other => panic!("Unexpected choice {other:?}"),
        };
        g.apply(action).unwrap();
    }
    panic!("Chain did not settle");
}
fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action).is_err());
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn repulse_counts_distinct_friendly_units_not_repeated_target_references() {
    for defender in 0..2 {
        let mut g = position(1 - defender);
        let target = unit(&mut g, defender, 69, Some(0));
        let chain_id = cast(&mut g, 53, vec![target, target]);
        focus(&mut g, defender);
        assert_eq!(g.repulse_candidates(defender, &[]), vec![target]);
        assert_eq!(
            g.repulse_candidates(defender, &[target]),
            vec![Target::Spell { chain_id }]
        );
        cast(&mut g, REPULSE, vec![target, Target::Spell { chain_id }]);
        settle(&mut g);
        assert_eq!(g.unit_ref(target).unwrap().damage, 0);
        assert!(g.players[1 - defender].discard.contains(&53));
        assert_eq!(g.players[defender].energy, 99);
        assert_eq!(
            g.players[defender].power_pool[domain_index(Domain::Body)],
            19
        );
    }
}

#[test]
fn repulse_cannot_ignore_a_target_that_flash_moved_to_base() {
    for defender in 0..2 {
        let mut g = position(1 - defender);
        let a = unit(&mut g, defender, 69, Some(0));
        let b = unit(&mut g, defender, 69, Some(1));
        let chain_id = cast(&mut g, 53, vec![a, b]);
        focus(&mut g, defender);
        assert!(g.repulse_candidates(defender, &[]).is_empty());
        cast(&mut g, 9, vec![b]);
        pair(&mut g);
        focus(&mut g, defender);
        assert_eq!(g.unit_ref(b).unwrap().lane, None);
        assert_eq!(g.live_chosen_board_targets(&g.chain[0]), vec![a, b]);
        g.players[defender].hand = vec![REPULSE];
        reject(
            &mut g,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![a, Target::Spell { chain_id }],
            },
        );
        assert!(g.repulse_candidates(defender, &[]).is_empty());
        assert!(g.counter_target_valid(defender, 12, chain_id));
    }
}

#[test]
fn repulse_becomes_legal_after_the_other_target_leaves_and_does_not_retarget_its_replay() {
    for defender in 0..2 {
        let mut g = position(1 - defender);
        let a = unit(&mut g, defender, 69, Some(0));
        let b = unit(&mut g, defender, 73, Some(1));
        let chain_id = cast(&mut g, 53, vec![a, b]);
        focus(&mut g, defender);
        cast(&mut g, 377, vec![b]);
        pair(&mut g);
        focus(&mut g, defender);
        assert!(g.unit_ref(b).is_none());
        // A new physical copy at the old location is not the departing target.
        let replayed = unit(&mut g, defender, 73, Some(1));
        assert_ne!(replayed, b);
        assert_eq!(g.repulse_candidates(defender, &[]), vec![a]);
        cast(&mut g, REPULSE, vec![a, Target::Spell { chain_id }]);
        settle(&mut g);
        assert_eq!(g.unit_ref(a).unwrap().damage, 0);
        assert_eq!(g.unit_ref(replayed).unwrap().damage, 0);
    }
}

#[test]
fn repulse_rechecks_its_own_battlefield_target_after_a_response() {
    for defender in 0..2 {
        let mut g = position(1 - defender);
        let target = unit(&mut g, defender, 69, Some(0));
        let chain_id = cast(&mut g, 53, vec![target, target]);
        focus(&mut g, defender);
        cast(&mut g, REPULSE, vec![target, Target::Spell { chain_id }]);
        cast(&mut g, 9, vec![target]);
        settle(&mut g);
        assert_eq!(g.unit_ref(target).unwrap().lane, None);
        assert_eq!(g.unit_ref(target).unwrap().damage, 6);
        assert!(g.players[defender].discard.contains(&REPULSE));
    }
}

#[test]
fn repulse_counters_activated_abilities_and_still_triggers_irelia_when_chosen() {
    for defender in 0..2 {
        let mut g = position(1 - defender);
        let target = unit(&mut g, defender, 0, Some(0));
        g.players[1 - defender].gear.push(GearState {
            id: 9000,
            card_id: 232,
            lane: None,
            attached_to: None,
            ready: true,
            temporary: false,
            token: false,
        });
        g.apply(Action::ActivateAbility {
            source: AbilitySource::Gear {
                player: 1 - defender,
                gear_id: 9000,
            },
            ability_index: 0,
            targets: vec![target],
        })
        .unwrap();
        let chain_id = g.chain.last().unwrap().id;
        focus(&mut g, defender);
        cast(&mut g, REPULSE, vec![target, Target::Spell { chain_id }]);
        settle(&mut g);
        assert_eq!(g.unit_ref(target).unwrap().damage, 0);
        assert_eq!(g.unit_ref(target).unwrap().temporary_buff, 1);
        assert!(!g.players[1 - defender].gear[0].ready);
        assert!(!g.players[1 - defender].discard.contains(&232));
    }
}

#[test]
fn repulse_wrong_order_missing_link_and_failed_payment_are_atomic() {
    let mut g = position(1);
    let a = unit(&mut g, 0, 69, Some(0));
    let b = unit(&mut g, 0, 69, Some(1));
    let chain_id = cast(&mut g, 53, vec![a, a]);
    focus(&mut g, 0);
    g.players[0].hand = vec![REPULSE];
    for targets in [
        vec![],
        vec![a],
        vec![Target::Spell { chain_id }, a],
        vec![b, Target::Spell { chain_id }],
        vec![a, Target::Spell { chain_id: 999999 }],
    ] {
        reject(
            &mut g,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets,
            },
        );
    }
    g.players[0].power_pool = [0; 6];
    reject(
        &mut g,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![a, Target::Spell { chain_id }],
        },
    );
}

#[test]
fn linked_counter_relationships_include_repeat_targets_and_all_current_controllers() {
    for defender in 0..2 {
        let mut g = position(1 - defender);
        let own = unit(&mut g, 1 - defender, 69, Some(0));
        let enemy = unit(&mut g, defender, 69, Some(0));
        g.players[1 - defender].hand = vec![50];
        g.apply(Action::PlayCardOptions {
            hand_index: 0,
            lane: None,
            targets: vec![own],
            repeat_targets: vec![enemy],
            repeat: true,
            accelerate: false,
            bonus: false,
        })
        .unwrap();
        let chain_id = g.chain[0].id;
        focus(&mut g, defender);
        assert!(g.counter_target_valid(defender, 12, chain_id));
        assert_eq!(g.repulse_candidates(defender, &[]), vec![enemy]);
        // Current controller matters to 'friendly', without breaking identity.
        let moved = g.players[1 - defender].units.remove(0);
        g.players[defender].units.push(moved);
        assert!(g.repulse_candidates(defender, &[]).is_empty());
        assert!(g.counter_target_valid(defender, 12, chain_id));
    }
}

#[test]
fn linked_not_so_fast_loses_a_departed_relationship_even_if_a_new_copy_enters() {
    for defender in 0..2 {
        let mut g = position(1 - defender);
        let target = unit(&mut g, defender, 69, Some(0));
        let chain_id = cast(&mut g, 53, vec![target, target]);
        focus(&mut g, defender);
        cast(&mut g, 377, vec![target]);
        pair(&mut g);
        focus(&mut g, defender);
        let new = unit(&mut g, defender, 69, Some(0));
        assert_ne!(new, target);
        assert!(!g.counter_target_valid(defender, 12, chain_id));
        assert!(g.repulse_candidates(defender, &[]).is_empty());
    }
}

#[test]
fn repulse_target_preview_changes_kind_without_spending_or_mutating() {
    let mut g = position(1);
    let target = unit(&mut g, 0, 69, Some(0));
    let chain_id = cast(&mut g, 53, vec![target, target]);
    focus(&mut g, 0);
    g.players[0].hand = vec![REPULSE];
    let before = format!("{g:?}");
    for (targets, kind, candidates) in [
        (vec![], rules::PlayTargetKind::Unit, vec![target]),
        (
            vec![target],
            rules::PlayTargetKind::Spell,
            vec![Target::Spell { chain_id }],
        ),
    ] {
        let view = g
            .play_target_view(
                &Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets,
                },
                0,
            )
            .unwrap();
        assert_eq!(view.next_slot.unwrap().kind, kind);
        assert_eq!(view.candidates, candidates);
        assert_eq!(view.spec.required, 2);
    }
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn repulse_ai_uses_both_linked_choices_to_save_a_unit_at_every_level_and_seat() {
    for defender in 0..2 {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let mut g = position(1 - defender);
            g.difficulty = difficulty;
            let target = unit(&mut g, defender, 73, Some(0));
            let chain_id = cast(&mut g, 67, vec![target]);
            focus(&mut g, defender);
            g.players[defender].hand = vec![REPULSE];
            g.step_ai().unwrap();
            let counter = g.chain.last().unwrap();
            assert_eq!(counter.card_id, REPULSE, "{difficulty:?}/{defender}");
            assert_eq!(counter.targets, vec![target, Target::Spell { chain_id }]);
            settle(&mut g);
            assert_eq!(g.unit_ref(target).unwrap().damage, 0);
        }
    }
}
