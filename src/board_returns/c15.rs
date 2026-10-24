//! Linked return instructions and direct Banish are distinct from Kill/Recall.
use super::*;

fn runes(g: &mut Game, seat: usize, count: usize) {
    g.players[seat].rune_deck = vec![Domain::Mind; count];
}
fn might(g: &mut Game, target: Target, value: i128) {
    let old = i128::from(g.unit_might(g.unit_ref(target).unwrap()).unwrap());
    g.modify_might(target, value - old, 0).unwrap();
}

#[test]
fn retreat_returns_physical_card_channels_exhausted_and_detaches_gear() {
    let mut g = position();
    let target = unit(&mut g, 0, 374, false, Some(0));
    gear(&mut g, 0, 10, false, Some(uid(target)));
    g.players[0].gear[0].ready = false;
    runes(&mut g, 0, 2);
    let energy = g.players[0].energy;
    cast(&mut g, 377, vec![target]);
    assert_eq!(g.players[0].hand, vec![374]);
    assert_eq!(g.players[0].runes[domain_index(Domain::Mind)], 1);
    assert_eq!(g.players[0].exhausted_runes[domain_index(Domain::Mind)], 1);
    assert_eq!(g.players[0].rune_deck.len(), 1);
    assert_eq!(g.players[0].energy, energy - 1); // Channel is exhausted.
    assert_eq!(g.players[0].gear[0].attached_to, None);
    assert!(!g.players[0].gear[0].ready);
    assert!(g.pending_triggers.is_empty() && g.decision.is_none());
}

#[test]
fn retreat_token_still_supplies_its_owner_to_channel() {
    let mut g = position();
    let target = unit(&mut g, 0, 54, true, None);
    runes(&mut g, 0, 1);
    cast(&mut g, 377, vec![target]);
    assert!(g.players[0].hand.is_empty() && g.players[0].banish.is_empty());
    assert!(g.unit_ref(target).is_none());
    assert_eq!(g.players[0].runes[domain_index(Domain::Mind)], 1);
    assert_eq!(g.players[0].discard, vec![377]);
}

#[test]
fn retreat_empty_rune_supply_still_returns_the_unit() {
    let mut g = position();
    let target = unit(&mut g, 0, 25, false, None);
    runes(&mut g, 0, 0);
    cast(&mut g, 377, vec![target]);
    assert_eq!(g.players[0].hand, vec![25]);
    assert_eq!(g.players[0].runes, [0; 6]);
}

#[test]
fn retreat_rejects_enemy_and_gear_before_spending() {
    let mut g = position();
    let enemy = unit(&mut g, 1, 25, false, None);
    let item = gear(&mut g, 0, 3, false, None);
    for target in [enemy, item] {
        let action = hand_action(&mut g, 377, vec![target]);
        let before = format!("{g:?}");
        assert!(g.apply(action).is_err());
        assert_eq!(format!("{g:?}"), before);
    }
}

#[test]
fn retreat_linked_channel_is_ignored_if_an_earlier_reaction_returned_target() {
    let mut g = position();
    let target = unit(&mut g, 0, 25, false, None);
    runes(&mut g, 0, 3);
    let action = hand_action(&mut g, 377, vec![target]);
    g.apply(action).unwrap();
    assert_eq!(g.priority(), 0);
    let action = hand_action(&mut g, 377, vec![target]);
    g.apply(action).unwrap();
    drive(&mut g, false, false);
    assert_eq!(g.players[0].hand, vec![25]);
    assert_eq!(g.players[0].rune_deck.len(), 2);
    assert_eq!(g.players[0].runes[domain_index(Domain::Mind)], 1);
    assert_eq!(g.players[0].discard, vec![377, 377]);
}

#[test]
fn retreat_tracks_identity_not_a_replayed_card_and_accepts_a_board_move() {
    for replay in [false, true] {
        let mut g = position();
        let target = unit(&mut g, 0, 25, false, Some(0));
        runes(&mut g, 0, 1);
        let action = hand_action(&mut g, 377, vec![target]);
        g.apply(action).unwrap();
        if replay {
            g.return_unit_to_hand(target);
            g.players[0].hand.clear();
            unit(&mut g, 0, 25, false, None);
        } else {
            g.unit_mut(target).unwrap().lane = None;
        }
        drive(&mut g, false, false);
        assert_eq!(
            g.players[0].runes[domain_index(Domain::Mind)],
            u8::from(!replay)
        );
        assert_eq!(g.players[0].units.len(), usize::from(replay));
    }
}

#[test]
fn direct_banish_bypasses_guardian_angel_and_never_emits_deathknell() {
    let mut g = position();
    let target = unit(&mut g, 1, 374, false, Some(0));
    gear(&mut g, 1, 10, false, Some(uid(target)));
    g.players[1].gear[0].ready = false;
    might(&mut g, target, 3);
    g.empowered_units.push(uid(target));
    g.granted_temporary_units.push(uid(target));
    cast(&mut g, 378, vec![target]);
    assert!(g.unit_ref(target).is_none());
    assert_eq!(g.players[1].banish, vec![374]);
    assert!(g.players[1].discard.is_empty() && g.players[1].hand.is_empty());
    assert_eq!(g.players[1].gear[0].card_id, 10);
    assert_eq!(g.players[1].gear[0].attached_to, None);
    assert!(!g.players[1].gear[0].ready);
    assert!(!g.empowered_units.contains(&uid(target)));
    assert!(!g.granted_temporary_units.contains(&uid(target)));
    assert!(g.pending_triggers.is_empty() && g.chain.is_empty() && g.decision.is_none());
}

#[test]
fn wind_and_ghosts_branch_uses_resolution_might_not_choice_might() {
    for (selected, resolved) in [(3, 4), (4, 3), (1, -3), (6, 10)] {
        let mut g = position();
        let target = unit(&mut g, 1, 25, false, Some(0));
        might(&mut g, target, selected);
        let action = hand_action(&mut g, 378, vec![target]);
        g.apply(action).unwrap();
        might(&mut g, target, resolved);
        drive(&mut g, false, false);
        assert!(g.unit_ref(target).is_none());
        assert_eq!(
            g.players[1].banish,
            if resolved <= 3 { vec![25] } else { vec![] }
        );
        assert_eq!(
            g.players[1].hand,
            if resolved > 3 { vec![25] } else { vec![] }
        );
        assert!(g.players[1].discard.is_empty());
    }
}

#[test]
fn wind_and_ghosts_tokens_vanish_in_both_branches() {
    for value in [2, 5] {
        let mut g = position();
        let target = unit(&mut g, 1, 54, true, Some(0));
        might(&mut g, target, value);
        cast(&mut g, 378, vec![target]);
        assert!(g.players[1].units.is_empty());
        assert!(
            g.players[1].hand.is_empty()
                && g.players[1].discard.is_empty()
                && g.players[1].banish.is_empty()
        );
    }
}

#[test]
fn wind_and_ghosts_rechecks_field_and_physical_identity() {
    for change in 0..3 {
        let mut g = position();
        let target = unit(&mut g, 1, 25, false, Some(0));
        might(&mut g, target, 2);
        let action = hand_action(&mut g, 378, vec![target]);
        g.apply(action).unwrap();
        match change {
            0 => g.unit_mut(target).unwrap().lane = None,
            1 => g.unit_mut(target).unwrap().lane = Some(1),
            _ => {
                g.return_unit_to_hand(target);
                g.players[1].hand.clear();
                unit(&mut g, 1, 25, false, Some(0));
            }
        }
        drive(&mut g, false, false);
        assert_eq!(g.players[1].banish.len(), usize::from(change == 1));
        assert_eq!(g.players[1].units.len(), usize::from(change != 1));
    }
}

#[test]
fn wind_and_ghosts_target_restrictions_are_atomic_and_do_not_restrict_might() {
    let mut g = position();
    let base = unit(&mut g, 1, 25, false, None);
    let protected = unit(&mut g, 1, 31, false, Some(0));
    let item = gear(&mut g, 1, 3, false, None);
    for target in [base, protected, item] {
        let action = hand_action(&mut g, 378, vec![target]);
        let before = format!("{g:?}");
        assert!(g.apply(action).is_err());
        assert_eq!(format!("{g:?}"), before);
    }
    g.unit_mut(base).unwrap().lane = Some(0);
    might(&mut g, base, 100);
    assert!(g.valid_card_target(0, 378, None, base, &[]).unwrap());
}

#[test]
fn apothecary_base_play_offers_only_friendly_battlefield_units_and_can_decline() {
    let mut g = position();
    let own_base = unit(&mut g, 0, 25, false, None);
    let friendly = unit(&mut g, 0, 25, false, Some(1));
    let enemy = unit(&mut g, 1, 25, false, Some(1));
    let action = hand_action(&mut g, 376, vec![]);
    g.apply(action).unwrap();
    drive(&mut g, true, false);
    assert_eq!(g.trigger_target_bounds(), Some((1, 0)));
    assert!(g.valid_trigger_target(friendly, &[]).unwrap());
    for target in [own_base, enemy] {
        assert!(!g.valid_trigger_target(target, &[]).unwrap());
    }
    assert_eq!(
        g.choose_ai_action().unwrap(),
        Action::ChooseTriggerTargets { targets: vec![] }
    );
    choose(&mut g, None);
    assert!(g.players[0].units.iter().any(|u| u.card_id == 376));
}

#[test]
fn apothecary_ambushes_on_chain_and_can_return_itself() {
    let mut g = position();
    let ally = unit(&mut g, 0, 25, false, Some(0));
    let action = hand_action(&mut g, 377, vec![ally]);
    g.apply(action).unwrap();
    let hand_index = g.players[0].hand.len();
    g.players[0].hand.push(376);
    // No battlefield controller is needed: the friendly unit permits Ambush.
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: Some(0),
        targets: vec![],
    })
    .unwrap();
    drive(&mut g, true, false);
    let apothecary = g.players[0]
        .units
        .iter()
        .find(|u| u.card_id == 376)
        .unwrap();
    let target = Target::Unit {
        player: 0,
        unit_id: apothecary.id,
    };
    assert!(g.valid_trigger_target(target, &[]).unwrap());
    choose(&mut g, Some(target));
    assert!(g.players[0].hand.contains(&376));
    assert!(g.players[0].hand.contains(&25));
}

#[test]
fn apothecary_return_choice_rechecks_field_and_does_not_need_live_source() {
    for move_to_base in [false, true] {
        let mut g = position();
        let ally = unit(&mut g, 0, 25, false, Some(0));
        let action = hand_action(&mut g, 376, vec![]);
        g.apply(action).unwrap();
        drive(&mut g, true, false);
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![ally],
        })
        .unwrap();
        let source = g.players[0]
            .units
            .iter()
            .find(|u| u.card_id == 376)
            .unwrap()
            .id;
        g.banish_unit_directly(Target::Unit {
            player: 0,
            unit_id: source,
        });
        g.unit_mut(ally).unwrap().lane = if move_to_base { None } else { Some(1) };
        drive(&mut g, false, false);
        assert_eq!(g.unit_ref(ally).is_some(), move_to_base);
        assert_eq!(g.players[0].hand.contains(&25), !move_to_base);
    }
}

#[test]
fn direct_departures_safely_reject_absent_objects_and_preserve_owner_for_both_seats() {
    let mut g = position();
    for seat in 0..2 {
        let target = unit(&mut g, seat, 25, false, None);
        assert_eq!(g.return_unit_to_hand(target), Some(seat));
        assert_eq!(g.banish_unit_directly(target), None);
        assert_eq!(g.players[seat].hand, vec![25]);
    }
    assert_eq!(
        g.return_unit_to_hand(Target::Unit {
            player: 99,
            unit_id: 0
        }),
        None
    );
}
