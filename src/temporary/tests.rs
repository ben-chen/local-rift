//! Boundary fixtures exercise actual effects and public action resolution. They
//! intentionally control the board without claiming an ordinary opening hand.
use super::*;
use crate::deck::CustomDeck;
use crate::gear::GearState;

fn position(legend: usize) -> Game {
    let mut g = Game::new_with_custom(
        [0, 3],
        [Some(CustomDeck::from_legend(legend).unwrap()), None],
        12012,
        Difficulty::Tactical,
        false,
    )
    .unwrap();
    g.start_match().unwrap();
    g.battlefields = [7, 9];
    g.battlefield_owner = [None; 2];
    for p in &mut g.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.runes = [0; 6];
        p.exhausted_runes = [0; 6];
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
fn unit(g: &mut Game, card: usize, lane: Option<usize>) -> Target {
    let id = g.create_unit(0, card, false, 0);
    g.unit_mut(Target::Unit {
        player: 0,
        unit_id: id,
    })
    .unwrap()
    .lane = lane;
    Target::Unit {
        player: 0,
        unit_id: id,
    }
}
fn tid(t: Target) -> u32 {
    let Target::Unit { unit_id, .. } = t else {
        panic!()
    };
    unit_id
}
fn object(t: Target) -> BoardObject {
    let Target::Unit { player, unit_id } = t else {
        panic!()
    };
    BoardObject::Unit { player, unit_id }
}
fn gear(g: &mut Game, card: usize, attached_to: Option<u32>) -> u32 {
    let id = g.next_unit_id;
    g.next_unit_id += 1;
    g.players[0].gear.push(GearState {
        id,
        card_id: card,
        lane: None,
        attached_to,
        ready: true,
        temporary: false,
        token: false,
    });
    id
}
fn drive(g: &mut Game) {
    for _ in 0..200 {
        let action = match &g.decision {
            Some(Decision::TokenPlay { .. } | Decision::TokenPlacement { .. }) => {
                Action::PlaceToken {
                    lane: g.current_pending_token().unwrap().location.flatten(),
                }
            }
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::Legend { .. }) => Action::Choose { accept: false },
            Some(other) => panic!("Unexpected decision {other:?}"),
            None if !g.chain.is_empty() => Action::Pass,
            None => return,
        };
        g.apply(action).unwrap();
    }
    panic!("Unsettled temporary fixture");
}
fn cast(g: &mut Game, card: usize, targets: Vec<Target>) {
    let hand_index = g.players[g.priority()].hand.len();
    g.players[g.priority()].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
    drive(g);
}

#[test]
fn beginning_captures_one_batch_and_new_queen_fountain_sprites_survive() {
    let mut g = position(3);
    unit(&mut g, 358, None);
    let fountain = gear(&mut g, 360, None);
    let old = g.create_unit(0, 355, true, 3);
    g.temporary_beginning(0);
    assert_eq!(g.pending_triggers.len(), 3);
    let batches: std::collections::HashSet<_> =
        g.pending_triggers.iter().map(|t| t.batch).collect();
    assert_eq!(
        batches.len(),
        1,
        "All abilities observe the same Beginning event"
    );
    g.after_resolution().unwrap();
    drive(&mut g);
    assert!(!g.players[0].gear.iter().any(|gear| gear.id == fountain));
    assert!(!g.players[0].units.iter().any(|u| u.id == old));
    let sprites: Vec<_> = g.players[0].units.iter().filter(|u| u.token).collect();
    assert_eq!(
        sprites.len(),
        2,
        "Queen and actual Fountain death each create one"
    );
    assert!(
        sprites
            .iter()
            .all(|u| u.ready && g.unit_might(u).unwrap() == 3)
    );
    assert_eq!(g.players[0].discard, vec![360]);
    assert_eq!(
        g.trigger_state.completed_cards[0], 0,
        "Deathknell and token plays are not extra card plays"
    );
}

#[test]
fn temporary_is_a_physical_source_and_recall_keeps_its_grant() {
    let mut g = position(0);
    let t = unit(&mut g, 71, Some(0));
    g.grant_temporary(t);
    g.grant_temporary(t);
    g.temporary_beginning(0);
    assert_eq!(g.pending_triggers.len(), 1);
    g.after_resolution().unwrap();
    let item = g.chain.last().unwrap();
    assert_eq!(
        g.chain_source(item),
        ChainSource::Unit {
            player: 0,
            unit_id: tid(t)
        }
    );
    let id = tid(t);
    g.recall_unit(0, id, true, true);
    assert!(g.temporary_status(object(t)).unwrap().granted);
    assert_eq!(g.unit_ref(t).unwrap().lane, None);
    drive(&mut g);
    assert!(g.unit_ref(t).is_none());
    assert_eq!(g.temporary_status(object(t)), None);
}

#[test]
fn countering_a_temporary_expiry_preserves_the_characteristic_for_next_beginning() {
    let mut g = position(3);
    let target = unit(&mut g, 69, Some(0));
    g.grant_temporary(target);
    g.temporary_beginning(0);
    g.after_resolution().unwrap();
    let first_expiry = g.chain[0].id;
    // Not So Fast only counters an opponent's effect that chooses your unit
    // or gear. Temporary chooses nothing, so it is not a legal target. Test
    // the general counter operation directly until an eligible card exists.
    assert!(!g.counter_target_valid(0, 12, first_expiry));
    assert!(!g.counter_target_valid(1, 12, first_expiry));
    g.counter_spell(first_expiry, false);
    assert!(g.chain.is_empty());
    assert!(g.unit_ref(target).is_some());
    assert!(g.temporary_status(object(target)).unwrap().granted);
    g.temporary_beginning(0);
    g.after_resolution().unwrap();
    assert_ne!(g.chain[0].id, first_expiry);
    drive(&mut g);
    assert!(g.unit_ref(target).is_none());
}

#[test]
fn attached_intrinsic_gear_is_characteristic_but_only_granted_expiry_is_active() {
    let mut g = position(3);
    let bearer = unit(&mut g, 69, None);
    let id = gear(&mut g, 360, Some(tid(bearer)));
    let source = BoardObject::Gear {
        player: 0,
        gear_id: id,
    };
    assert_eq!(
        g.temporary_status(source),
        Some(TemporaryStatus {
            intrinsic: true,
            granted: false,
            active: false
        })
    );
    assert!(g.has_temporary(source));
    g.temporary_beginning(0);
    assert!(g.pending_triggers.is_empty());
    g.grant_temporary(Target::Gear {
        player: 0,
        gear_id: id,
    });
    g.temporary_beginning(0);
    g.after_resolution().unwrap();
    drive(&mut g);
    assert!(g.players[0].gear.is_empty());
    assert_eq!(
        g.players[0].units.len(),
        1,
        "Attached Rules-panel Deathknell was inactive at death"
    );
    assert!(g.unit_ref(bearer).is_some());
}

#[test]
fn smoke_moves_each_unlocked_target_using_the_original_pair_of_locations() {
    let mut g = position(3);
    let a = unit(&mut g, 69, Some(0));
    let b = unit(&mut g, 69, None);
    g.battlefield_owner[0] = Some(0);
    g.grant_temporary(a);
    g.movement_locks.push((0, tid(a)));
    let draws = g.trigger_state.draws[0];
    cast(&mut g, 354, vec![a, b]);
    assert_eq!(g.unit_ref(a).unwrap().lane, Some(0));
    assert_eq!(g.unit_ref(b).unwrap().lane, Some(0));
    assert_eq!(g.trigger_state.draws[0], draws + 1);
}

#[test]
fn smoke_does_not_invent_a_departed_targets_location_but_still_draws() {
    let mut g = position(3);
    let a = unit(&mut g, 69, Some(0));
    let b = unit(&mut g, 69, None);
    g.battlefield_owner[0] = Some(0);
    g.grant_temporary(a);
    g.players[0].hand.push(354);
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![a, b],
    })
    .unwrap();
    g.bounce(a);
    let draws = g.trigger_state.draws[0];
    drive(&mut g);
    assert_eq!(g.unit_ref(b).unwrap().lane, None);
    assert_eq!(g.trigger_state.draws[0], draws + 1);
    assert!(!g.granted_temporary_units.contains(&tid(a)));
}

#[test]
fn hidden_smoke_repeat_keeps_each_original_slot_after_the_first_target_leaves() {
    use crate::activated::AbilitySource;
    use crate::play::{ExecutionChoices, PlayChoices, RepeatSelection};
    let mut g = position(3);
    let portal = gear(&mut g, 256, None);
    g.apply(Action::ActivateAbility {
        source: AbilitySource::Gear {
            player: 0,
            gear_id: portal,
        },
        ability_index: 0,
        targets: vec![],
    })
    .unwrap();
    drive(&mut g);
    let a = unit(&mut g, 69, Some(0));
    let b = unit(&mut g, 69, None);
    let c = unit(&mut g, 69, Some(0));
    let d = unit(&mut g, 69, Some(1));
    g.grant_temporary(a);
    g.grant_temporary(c);
    g.battlefield_owner = [Some(0); 2];
    g.players[0].hidden.push(crate::hidden::HiddenCard {
        id: 9000,
        card_id: 354,
        lane: 0,
        hidden_turn: 0,
        revealed: false,
    });
    let original = Action::PlayHidden {
        hidden_id: 9000,
        targets: vec![a, b],
    };
    let options = g.play_choices(&original).unwrap();
    assert_eq!(options.repeats.len(), 1);
    let action = Action::WithPlayChoices {
        action: Box::new(original),
        choices: PlayChoices {
            repeats: vec![RepeatSelection {
                option_id: options.repeats[0].id,
                choices: ExecutionChoices {
                    targets: vec![c, d],
                    mode: None,
                },
            }],
            ..Default::default()
        },
    };
    let query = g.play_target_view(&action, 1).unwrap();
    assert_eq!(query.targets, vec![c, d]);
    g.apply(action).unwrap();
    g.bounce(a);
    let draws = g.trigger_state.draws[0];
    drive(&mut g);
    assert_eq!(g.unit_ref(b).unwrap().lane, None);
    assert_eq!(g.unit_ref(c).unwrap().lane, Some(1));
    assert_eq!(g.unit_ref(d).unwrap().lane, Some(0));
    assert_eq!(g.trigger_state.draws[0], draws + 2);
}

#[test]
fn tasty_death_channels_only_available_runes_exhausted_and_draws_independently() {
    for count in 0..=2 {
        let mut g = position(0);
        let t = unit(&mut g, 361, None);
        g.players[0].rune_deck = vec![Domain::Calm; count];
        let draws = g.trigger_state.draws[0];
        g.kill_unit(t).unwrap();
        g.after_resolution().unwrap();
        drive(&mut g);
        assert_eq!(g.players[0].runes[domain_index(Domain::Calm)], count as u8);
        assert_eq!(
            g.players[0].exhausted_runes[domain_index(Domain::Calm)],
            count as u8
        );
        assert_eq!(g.trigger_state.draws[0], draws + 1);
    }
}

#[test]
fn petal_counts_own_temporary_units_here_including_self_but_never_base() {
    let mut g = position(5);
    let petal = unit(&mut g, 352, Some(0));
    let other = unit(&mut g, 69, Some(0));
    let base = unit(&mut g, 69, None);
    g.grant_temporary(base);
    g.grant_temporary(other);
    assert_eq!(g.unit_might(g.unit_ref(petal).unwrap()).unwrap(), 3);
    g.grant_temporary(petal);
    assert_eq!(g.unit_might(g.unit_ref(petal).unwrap()).unwrap(), 4);
    g.recall_unit(0, tid(petal), false, false);
    assert_eq!(g.unit_might(g.unit_ref(petal).unwrap()).unwrap(), 2);
}

#[test]
fn petal_losing_a_temporary_neighbor_causes_a_separate_lethal_cleanup() {
    let mut g = position(3);
    let petal = unit(&mut g, 352, Some(0));
    let sprite_id = g.create_unit(0, 355, true, 3);
    let sprite = Target::Unit {
        player: 0,
        unit_id: sprite_id,
    };
    g.unit_mut(sprite).unwrap().lane = Some(0);
    g.unit_mut(petal).unwrap().damage = 2;
    assert_eq!(g.unit_might(g.unit_ref(petal).unwrap()).unwrap(), 3);
    g.after_resolution().unwrap();
    assert!(g.unit_ref(petal).is_some());
    g.kill_unit(sprite).unwrap();
    g.after_resolution().unwrap();
    assert!(g.unit_ref(sprite).is_none());
    assert!(g.unit_ref(petal).is_none());
    assert_eq!(g.players[0].discard, vec![352]);
}

#[test]
fn trevor_hold_uses_live_here_and_never_a_departed_sources_last_location() {
    for departed in [false, true] {
        let mut g = position(0);
        let trevor = unit(&mut g, 359, Some(0));
        g.battlefield_owner[0] = Some(0);
        g.score_triggers(0, 0, true).unwrap();
        g.after_resolution().unwrap();
        assert!(!g.chain.is_empty());
        if departed {
            g.bounce(trevor);
        } else {
            g.recall_unit(0, tid(trevor), false, true);
        }
        drive(&mut g);
        let sprites: Vec<_> = g.players[0].units.iter().filter(|u| u.token).collect();
        assert_eq!(sprites.len(), usize::from(!departed));
        assert!(sprites.iter().all(|u| u.ready && u.lane.is_none()));
    }
}

#[test]
fn last_stand_snapshots_shield_then_keeps_only_the_fixed_increase() {
    let mut g = position(0);
    let target = unit(&mut g, 107, Some(0));
    // Exact signed fixture: printed unit + this-turn modifier totals 3 before Shield 2.
    let printed = i64::from(g.catalog[107].might);
    g.unit_mut(target).unwrap().temporary_buff = 3 - printed;
    g.showdown = Some(Showdown {
        lane: 0,
        attacker: 1,
        focus: 0,
        passes: 0,
        combat: true,
    });
    assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), 5);
    cast(&mut g, 350, vec![target]);
    assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), 10);
    g.showdown = None;
    assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), 8);
}

#[test]
fn a_negative_last_stand_snapshot_adds_zero_and_still_grants_temporary() {
    let mut g = position(0);
    let target = unit(&mut g, 71, None);
    let printed = i64::from(g.catalog[71].might);
    g.unit_mut(target).unwrap().temporary_buff = -2 - printed;
    let before = g.unit_ref(target).unwrap().temporary_buff;
    g.resolve_operation(0, 350, None, vec![target], None)
        .unwrap();
    assert_eq!(g.unit_ref(target).unwrap().temporary_buff, before);
    assert!(g.temporary_status(object(target)).unwrap().granted);
}

#[test]
fn a_double_beyond_host_capacity_rolls_back_the_entire_resolution() {
    let mut g = position(0);
    let target = unit(&mut g, 71, None);
    let printed = i64::from(g.catalog[71].might);
    g.unit_mut(target).unwrap().temporary_buff = i64::MAX - printed;
    g.players[0].hand.push(350);
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![target],
    })
    .unwrap();
    if matches!(g.decision, Some(Decision::Legend { .. })) {
        g.apply(Action::Choose { accept: false }).unwrap();
    }
    g.apply(Action::Pass).unwrap();
    let before = format!("{g:?}");
    let error = g.apply(Action::Pass).unwrap_err();
    assert!(error.starts_with("engine_capacity:"), "{error}");
    assert_eq!(format!("{g:?}"), before);
    assert!(!g.has_temporary(object(target)));
}

#[test]
fn mother_needs_a_live_here_while_queen_keeps_its_fixed_base_instruction() {
    for (card, expected) in [(357, 0), (358, 1)] {
        let mut g = position(3);
        g.players[0].hand.push(card);
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        })
        .unwrap();
        let id = g.players[0]
            .units
            .iter()
            .find(|u| u.card_id == card)
            .unwrap()
            .id;
        g.bounce(Target::Unit {
            player: 0,
            unit_id: id,
        });
        drive(&mut g);
        assert_eq!(g.players[0].units.len(), expected, "creator {card}");
        assert!(
            g.players[0]
                .units
                .iter()
                .all(|u| u.token && u.ready && u.lane.is_none())
        );
    }
}

#[test]
fn hidden_call_keeps_its_fixed_origin_even_after_control_changes() {
    let mut g = position(3);
    g.battlefield_owner[0] = Some(0);
    g.players[0].hidden.push(crate::hidden::HiddenCard {
        id: 9000,
        card_id: 356,
        lane: 0,
        hidden_turn: 0,
        revealed: false,
    });
    g.apply(Action::PlayHidden {
        hidden_id: 9000,
        targets: vec![],
    })
    .unwrap();
    g.battlefield_owner[0] = Some(1);
    // A fixed effect instruction may play there without control; it cannot be
    // redirected to base. Normal staging still follows entry.
    drive(&mut g);
    let sprite = g.players[0].units.iter().find(|u| u.token).unwrap();
    assert_eq!(sprite.lane, Some(0));
    assert!(sprite.ready);
}

#[test]
fn hidden_sprite_keeps_identity_and_fixed_field_through_payment_and_conversion() {
    use crate::activated::AbilitySource;
    let mut g = position(3);
    unit(&mut g, 265, None); // Grants Accelerate to token plays.
    let henge = gear(&mut g, 244, None);
    g.battlefield_owner[0] = Some(0);
    g.players[0].hidden.push(crate::hidden::HiddenCard {
        id: 9000,
        card_id: 356,
        lane: 0,
        hidden_turn: 0,
        revealed: false,
    });
    g.apply(Action::PlayHidden {
        hidden_id: 9000,
        targets: vec![],
    })
    .unwrap();
    while g.decision.is_none() {
        g.apply(Action::Pass).unwrap();
    }
    assert!(matches!(g.decision, Some(Decision::TokenPlay { .. })));
    let child = g.current_pending_token().unwrap().id;
    let begin = Action::BeginPayment {
        action: Box::new(Action::FinalizeTokenPlay {
            lane: Some(0),
            accelerate: true,
        }),
    };
    g.apply(begin.clone()).unwrap();
    assert_eq!(g.current_pending_token().unwrap().id, child);
    g.apply(Action::ActivateAbility {
        source: AbilitySource::Gear {
            player: 0,
            gear_id: henge,
        },
        ability_index: 0,
        targets: vec![],
    })
    .unwrap();
    assert!(matches!(
        g.decision,
        Some(Decision::ResourceConversion { .. })
    ));
    assert_eq!(g.current_pending_token().unwrap().id, child);
    g.apply(Action::ResolveConversion { amount: 1 }).unwrap();
    assert!(matches!(g.decision, Some(Decision::Payment { .. })));
    // A mandatory token play with a payable cost cannot be cancelled.
    let before = format!("{g:?}");
    assert!(g.apply(Action::Choose { accept: false }).is_err());
    assert_eq!(format!("{g:?}"), before);
    assert_eq!(g.current_pending_token().unwrap().id, child);
    assert!(
        !g.players[0]
            .gear
            .iter()
            .find(|gear| gear.id == henge)
            .unwrap()
            .ready
    );
    g.battlefield_owner[0] = Some(1);
    assert_eq!(g.current_pending_token().unwrap().id, child);
    g.apply(Action::CompletePayment).unwrap();
    let sprite = g.players[0].units.iter().find(|unit| unit.token).unwrap();
    assert_eq!(sprite.lane, Some(0));
    assert!(sprite.ready);
    assert!(g.current_pending_token().is_none());
}

#[test]
fn momentary_grants_only_from_its_opponent_turn_play_trigger() {
    for opponent_turn in [false, true] {
        let mut g = position(0);
        g.battlefield_owner[0] = Some(0);
        if opponent_turn {
            g.active = 1;
            g.priority_player = 0;
        }
        g.players[0].hidden.push(crate::hidden::HiddenCard {
            id: 9000,
            card_id: 351,
            lane: 0,
            hidden_turn: 0,
            revealed: false,
        });
        g.apply(Action::PlayHidden {
            hidden_id: 9000,
            targets: vec![],
        })
        .unwrap();
        drive(&mut g);
        let companion = g.players[0]
            .units
            .iter()
            .find(|u| u.card_id == 351)
            .unwrap();
        let status = g
            .temporary_status(BoardObject::Unit {
                player: 0,
                unit_id: companion.id,
            })
            .unwrap();
        assert!(!status.intrinsic);
        assert_eq!(status.granted, opponent_turn);
    }
}
