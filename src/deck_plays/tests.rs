//! Independent C19 conformance: real parent actions; initial board/resources are fixtures.
use super::*;
use crate::activated::AbilitySource;
use crate::empower::BoardObject;
use crate::gear::GearState;

fn position() -> Game {
    let mut g = Game::new_for_agents(3, 3, 19019, Difficulty::Tactical);
    g.start_match().unwrap();
    g.battlefields = [2, 2];
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
fn unit(g: &mut Game, player: usize, card_id: usize, lane: Option<usize>) -> Target {
    let unit_id = g.create_unit(player, card_id, false, 0);
    let target = Target::Unit { player, unit_id };
    g.unit_mut(target).unwrap().lane = lane;
    if let Some(lane) = lane {
        g.battlefield_owner[lane] = Some(player);
    }
    target
}
fn gear(g: &mut Game, player: usize, card_id: usize, attached_to: Option<u32>) -> u32 {
    let id = g.next_unit_id;
    g.next_unit_id += 1;
    g.players[player].gear.push(GearState {
        id,
        card_id,
        attached_to,
        ready: true,
        temporary: false,
        token: false,
    });
    id
}
fn cast(g: &mut Game, card: usize, targets: Vec<Target>) {
    let p = g.priority();
    let hand_index = g.players[p].hand.len();
    g.players[p].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
}
fn resolve(g: &mut Game) {
    for _ in 0..80 {
        if g.decision.is_some() || g.chain.is_empty() {
            return;
        }
        g.apply(Action::Pass).unwrap();
    }
    panic!("unsettled chain");
}
fn hook(g: &mut Game, target: Target) {
    let id = gear(g, 0, 385, None);
    g.apply(Action::ActivateAbility {
        source: AbilitySource::Gear {
            player: 0,
            gear_id: id,
        },
        ability_index: 0,
        targets: vec![target],
    })
    .unwrap();
}
fn look(g: &Game) -> (u32, Vec<usize>, Vec<usize>) {
    let Some(Decision::DeckLookPlay {
        choice_id,
        cards,
        eligible,
        ..
    }) = &g.decision
    else {
        panic!("{:?}", g.decision)
    };
    (*choice_id, cards.clone(), eligible.clone())
}
fn select(g: &mut Game, index: Option<usize>) {
    let (choice_id, _, _) = look(g);
    g.apply(Action::ResolveDeckLook { choice_id, index })
        .unwrap();
}
fn finalize(accelerate: bool) -> Action {
    Action::FinalizePendingPlay {
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate,
        bonus: false,
    }
}
fn no_resources(g: &mut Game) {
    g.players[0].energy = 0;
    g.players[0].energy_pool = 0;
    g.players[0].power_pool = [0; 6];
    g.players[0].any_power = 0;
}
fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action).is_err());
    assert_eq!(format!("{g:?}"), before);
}
fn play_look(g: &mut Game, card: usize, deck: Vec<usize>) {
    g.players[0].deck = deck;
    cast(g, card, vec![]);
    resolve(g);
    assert!(matches!(g.decision, Some(Decision::DeckLookPlay { .. })));
}
fn repeat_parent(g: &mut Game, card: usize) {
    let portal = gear(g, 0, 256, None);
    g.apply(Action::ActivateAbility {
        source: AbilitySource::Gear {
            player: 0,
            gear_id: portal,
        },
        ability_index: 0,
        targets: vec![],
    })
    .unwrap();
    resolve(g);
    g.players[0].hand.push(card);
    let action = Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    };
    let option_id = g.play_choices(&action).unwrap().repeats[0].id;
    g.apply(Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats: vec![RepeatSelection {
                option_id,
                choices: ExecutionChoices::default(),
            }],
            ..Default::default()
        },
    })
    .unwrap();
}
#[test]
fn hook_uses_actual_killed_might_including_attached_gear_and_preserves_control() {
    let mut g = position();
    let target = unit(&mut g, 0, 273, Some(0));
    let Target::Unit { unit_id, .. } = target else {
        unreachable!()
    };
    let sword = gear(&mut g, 0, 117, Some(unit_id));
    g.players[0].deck = vec![70, 145, 96, 10, 133];
    hook(&mut g, target);
    resolve(&mut g);
    assert!(g.unit_ref(target).is_none());
    assert!(matches!(
        g.decision,
        Some(Decision::DeckLookPlay {
            killed_might: Some(3),
            ..
        })
    ));
    assert_eq!(look(&g).1, vec![133, 10, 96, 145, 70]);
    assert_eq!(look(&g).2, vec![2, 3]);
    assert_eq!(
        g.players[0]
            .gear
            .iter()
            .find(|x| x.id == sword)
            .unwrap()
            .attached_to,
        None
    );
    select(&mut g, Some(2));
    assert_eq!(g.battlefield_owner[0], Some(0));
    assert_eq!(g.pending_cards().len(), 1);
    no_resources(&mut g);
    let mut action = finalize(false);
    if let Action::FinalizePendingPlay { lane, .. } = &mut action {
        *lane = Some(0);
    }
    g.apply(action).unwrap();
    assert!(
        g.players[0]
            .units
            .iter()
            .any(|u| u.card_id == 96 && u.lane == Some(0))
    );
    assert_eq!(g.battlefield_owner[0], Some(0));
}
#[test]
fn hook_stale_target_still_looks_and_recycles_but_cannot_select() {
    let mut g = position();
    let target = unit(&mut g, 0, 70, None);
    let enemy = unit(&mut g, 1, 70, None);
    g.players[0].deck = vec![96, 96];
    hook(&mut g, target);
    if g.priority() == 0 {
        g.apply(Action::Pass).unwrap();
    }
    cast(&mut g, 16, vec![enemy, target]);
    resolve(&mut g);
    assert_eq!(look(&g).1, vec![96, 96]);
    assert!(look(&g).2.is_empty());
    assert!(matches!(
        g.decision,
        Some(Decision::DeckLookPlay {
            killed_might: None,
            ..
        })
    ));
    let choice_id = look(&g).0;
    reject(
        &mut g,
        Action::ResolveDeckLook {
            choice_id,
            index: Some(0),
        },
    );
    select(&mut g, None);
    assert_eq!(g.players[0].deck, vec![96, 96]);
    assert!(g.pending_cards().is_empty());
}
#[test]
fn hook_prevented_death_waits_for_replacement_choice_and_has_null_might() {
    let mut g = position();
    let target = unit(&mut g, 0, 70, Some(0));
    let Target::Unit { unit_id, .. } = target else {
        unreachable!()
    };
    gear(&mut g, 0, 10, Some(unit_id));
    let hourglass = gear(&mut g, 0, 21, None);
    g.players[0].deck = vec![96];
    hook(&mut g, target);
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::DeathReplacement { .. })
    ));
    assert_eq!(g.players[0].deck, vec![96]);
    g.apply(Action::ChooseDeathReplacement {
        unit_id,
        gear_id: Some(hourglass),
    })
    .unwrap();
    assert!(matches!(
        g.decision,
        Some(Decision::DeckLookPlay {
            killed_might: None,
            ..
        })
    ));
    assert!(look(&g).2.is_empty());
    assert_eq!(g.unit_ref(target).unwrap().lane, None);
    select(&mut g, None);
    assert!(g.pending_cards().is_empty());
}
#[test]
fn hook_cost_waiver_ignores_base_power_but_not_accelerate_addition() {
    let mut g = position();
    let target = unit(&mut g, 0, 70, None);
    g.players[0].deck = vec![145];
    hook(&mut g, target);
    resolve(&mut g);
    select(&mut g, Some(0));
    let normal = g.cost_plan(&finalize(false)).unwrap();
    assert_eq!(normal.energy, 0);
    assert_eq!(normal.power.iter().map(|p| p.amount).sum::<u16>(), 0);
    let accelerated = g.cost_plan(&finalize(true)).unwrap();
    assert_eq!(accelerated.energy, 1);
    assert_eq!(accelerated.power.iter().map(|p| p.amount).sum::<u16>(), 1);
    no_resources(&mut g);
    reject(&mut g, finalize(true));
    g.apply(finalize(false)).unwrap();
    assert!(g.players[0].units.iter().any(|u| u.card_id == 145));
}
#[test]
fn look_is_a_snapshot_rejects_wrong_choice_or_kind_and_handles_empty_without_burnout() {
    for card in [387, 388] {
        let mut g = position();
        play_look(&mut g, card, vec![133, 10, 96]);
        assert_eq!(g.players[0].deck, vec![133, 10, 96]);
        let (choice_id, cards, eligible) = look(&g);
        assert_eq!(cards, vec![96, 10, 133]);
        assert_eq!(eligible, if card == 387 { vec![0] } else { vec![0, 1] });
        reject(
            &mut g,
            Action::ResolveDeckLook {
                choice_id: choice_id + 99,
                index: None,
            },
        );
        reject(
            &mut g,
            Action::ResolveDeckLook {
                choice_id,
                index: Some(2),
            },
        );
        select(&mut g, None);
        assert!(g.pending_cards().is_empty());
        assert!(g.players[0].discard.contains(&card));
        assert_eq!(g.players[0].deck.len(), 3);
        let mut empty = position();
        empty.players[0].deck.clear();
        empty.players[0].discard = vec![70];
        cast(&mut empty, card, vec![]);
        resolve(&mut empty);
        if matches!(empty.decision, Some(Decision::DeckLookPlay { .. })) {
            select(&mut empty, None);
        }
        assert!(empty.players[0].deck.is_empty());
        assert!(empty.players[0].discard.contains(&70));
    }
}
#[test]
fn total_energy_discount_covers_accelerate_and_frozen_payment_charges_power_once() {
    for source in [387, 388] {
        for staged in [false, true] {
            let mut g = position();
            play_look(&mut g, source, vec![96]);
            select(&mut g, Some(0));
            no_resources(&mut g);
            g.players[0].power_pool[domain_index(Domain::Fury)] = 1;
            let action = finalize(true);
            let plan = g.cost_plan(&action).unwrap();
            assert_eq!(
                plan.energy, 0,
                "total discount includes Accelerate's Energy"
            );
            assert_eq!(plan.power.iter().map(|p| p.amount).sum::<u16>(), 1);
            assert!(
                plan.modifiers
                    .iter()
                    .any(|m| m.energy == 5 && m.scope == CostScope::Total)
            );
            if staged {
                g.apply(Action::BeginPayment {
                    action: Box::new(action),
                })
                .unwrap();
                g.apply(Action::CompletePayment).unwrap();
            } else {
                g.apply(action).unwrap();
            }
            assert_eq!(g.players[0].power_pool[domain_index(Domain::Fury)], 0);
            assert!(
                g.players[0]
                    .units
                    .iter()
                    .any(|u| u.card_id == 96 && u.ready)
            );
        }
    }
}
#[test]
fn failed_child_returns_its_exact_duplicate_to_banish_without_undoing_parent() {
    let mut g = position();
    g.players[0].banish = vec![70, 96, 70];
    play_look(&mut g, 387, vec![133, 70]);
    select(&mut g, Some(0));
    let pending_id = g.pending_cards()[0].id;
    assert_eq!(g.pending_cards()[0].card_id, 70);
    assert_eq!(g.players[0].banish, vec![70, 96, 70]);
    assert_eq!(g.players[0].deck, vec![133]);
    assert!(g.players[0].discard.contains(&387));
    no_resources(&mut g);
    assert!(g.can_decline_trash_play().unwrap());
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(g.players[0].banish, vec![70, 96, 70, 70]);
    assert_eq!(g.players[0].deck, vec![133]);
    assert!(!g.pending_cards().iter().any(|c| c.id == pending_id));
}
#[test]
fn repeated_parent_recycles_each_execution_before_children_finalize() {
    let mut g = position();
    g.players[0].deck = vec![96; 8];
    repeat_parent(&mut g, 387);
    resolve(&mut g);
    let first_choice = look(&g).0;
    select(&mut g, Some(0));
    assert!(g.players[0].units.is_empty());
    assert!(
        g.chain
            .iter()
            .any(|c| c.card_id == 387 && c.ability.is_none())
    );
    assert!(!g.players[0].discard.contains(&387));
    assert_eq!(g.players[0].deck.len(), 7);
    assert_ne!(look(&g).0, first_choice);
    assert_eq!(g.pending_cards().len(), 1);
    select(&mut g, Some(0));
    assert!(g.players[0].discard.contains(&387));
    assert_eq!(g.players[0].deck.len(), 6);
    assert_eq!(g.pending_cards().len(), 2);
    assert!(g.players[0].units.is_empty());
    let first = g.pending_cards()[0].id;
    g.apply(finalize(false)).unwrap();
    assert_eq!(g.players[0].units.len(), 1);
    assert!(g.pending_cards().iter().all(|c| c.id != first));
    g.apply(finalize(false)).unwrap();
    assert_eq!(g.players[0].units.len(), 2);
}
#[test]
fn aurora_reveals_first_unit_at_real_end_turn_and_plays_before_turn_handoff() {
    let mut g = position();
    gear(&mut g, 0, 386, None);
    g.players[0].deck = vec![70, 145, 133, 117];
    g.apply(Action::EndTurn).unwrap();
    assert!(!g.chain.is_empty(), "end trigger is respondable");
    resolve(&mut g);
    let Some(Decision::RevealedDeck {
        player,
        source_card_id,
        ref cards,
    }) = g.decision
    else {
        panic!("{:?}", g.decision)
    };
    assert_eq!((player, source_card_id), (0, 386));
    assert_eq!(cards, &vec![117, 133, 145]);
    assert_eq!(g.players[0].deck.len(), 3);
    reject(&mut g, Action::Choose { accept: false });
    g.apply(Action::Choose { accept: true }).unwrap();
    assert_eq!(g.pending_cards()[0].card_id, 145);
    no_resources(&mut g);
    assert_eq!(
        g.cost_plan(&finalize(false))
            .unwrap()
            .power
            .iter()
            .map(|p| p.amount)
            .sum::<u16>(),
        0
    );
    g.apply(finalize(false)).unwrap();
    assert!(g.players[0].units.iter().any(|u| u.card_id == 145));
}
#[test]
fn aurora_no_unit_or_empty_deck_recycles_without_drawing_or_burnout() {
    for deck in [vec![], vec![133, 117, 10]] {
        let mut g = position();
        gear(&mut g, 0, 386, None);
        g.players[0].deck = deck.clone();
        g.players[0].discard = vec![70];
        g.apply(Action::EndTurn).unwrap();
        resolve(&mut g);
        if let Some(Decision::RevealedDeck { ref cards, .. }) = g.decision {
            assert_eq!(cards.len(), deck.len());
            g.apply(Action::Choose { accept: true }).unwrap();
        }
        assert!(g.pending_cards().is_empty());
        assert_eq!(g.players[0].deck.len(), deck.len());
        assert_eq!(g.players[0].discard, vec![70]);
        assert!(g.players[0].hand.is_empty());
    }
}
#[test]
fn wild_claw_empower_is_separate_permanent_unit_or_gear_ability() {
    for card in [96, 117] {
        let mut g = position();
        play_look(&mut g, 388, vec![card]);
        select(&mut g, Some(0));
        assert!(matches!(g.decision, Some(Decision::PendingPlay { .. })));
        g.apply(finalize(false)).unwrap();
        let Some(Decision::EmpowerPlayed {
            target: Some(target),
            ..
        }) = g.decision
        else {
            panic!("{:?}", g.decision)
        };
        let object = match target {
            Target::Unit { player, unit_id } => BoardObject::Unit { player, unit_id },
            Target::Gear { player, gear_id } => BoardObject::Gear { player, gear_id },
            _ => panic!(),
        };
        assert!(!g.object_empowered(object));
        g.apply(Action::Choose { accept: true }).unwrap();
        assert!(!g.object_empowered(object));
        assert!(g.chain.iter().any(|c| matches!(c.ability, Some(ChainAbility::EmpowerLinked { target: Some(t) }) if t == target)));
        resolve(&mut g);
        assert!(g.object_empowered(object));
        g.apply(Action::EndTurn).unwrap();
        resolve(&mut g);
        assert!(g.object_empowered(object), "not a turn-scoped Empower");
    }
}
#[test]
fn wild_claw_link_does_not_follow_returned_unit_or_other_physical_copy() {
    let mut g = position();
    let old = unit(&mut g, 0, 96, None);
    let enemy = unit(&mut g, 1, 70, None);
    play_look(&mut g, 388, vec![96]);
    select(&mut g, Some(0));
    g.apply(finalize(false)).unwrap();
    let Some(Decision::EmpowerPlayed {
        target: Some(child),
        ..
    }) = g.decision
    else {
        panic!()
    };
    g.apply(Action::Choose { accept: true }).unwrap();
    // Pass priority if the finalized optional ability leaves it with its controller.
    if g.priority() == 0 {
        g.apply(Action::Pass).unwrap();
    }
    cast(&mut g, 16, vec![enemy, child]);
    resolve(&mut g);
    assert!(g.unit_ref(child).is_none());
    let Target::Unit { player, unit_id } = old else {
        unreachable!()
    };
    assert!(!g.object_empowered(BoardObject::Unit { player, unit_id }));
    assert!(g.players[0].hand.contains(&96));
}
#[test]
fn wild_claw_failed_child_or_declined_reflexive_never_empowers_another_copy() {
    for failed in [false, true] {
        let mut g = position();
        let old = unit(&mut g, 0, 70, None);
        play_look(&mut g, 388, vec![70]);
        select(&mut g, Some(0));
        if failed {
            no_resources(&mut g);
            g.apply(Action::Choose { accept: false }).unwrap();
        } else {
            g.apply(finalize(false)).unwrap();
        }
        if matches!(g.decision, Some(Decision::EmpowerPlayed { .. })) {
            g.apply(Action::Choose { accept: false }).unwrap();
        }
        let Target::Unit { player, unit_id } = old else {
            unreachable!()
        };
        assert!(!g.object_empowered(BoardObject::Unit { player, unit_id }));
        assert!(
            !g.players[0]
                .units
                .iter()
                .any(|u| g.object_empowered(BoardObject::Unit {
                    player: 0,
                    unit_id: u.id
                }))
        );
        if failed {
            assert_eq!(g.players[0].banish, vec![70]);
        }
    }
}
#[test]
fn silence_negates_instructed_play_but_not_selected_banish_or_recycling() {
    for card in [387, 388] {
        let mut g = position();
        play_look(&mut g, card, vec![133, 96]);
        // Boundary fixture: a play prohibition now applies to this already-resolving parent.
        g.silenced[0] = true;
        select(&mut g, Some(0));
        assert_eq!(g.players[0].banish, vec![96]);
        assert_eq!(g.players[0].deck, vec![133]);
        assert!(g.pending_cards().is_empty());
        assert!(g.players[0].units.is_empty());
        assert!(g.players[0].discard.contains(&card));
    }
}
#[test]
fn hook_does_not_cleanup_aura_loss_until_its_private_look_finishes() {
    let mut g = position();
    let garen = unit(&mut g, 0, 320, Some(0));
    let survivor = unit(&mut g, 0, 70, Some(0));
    g.unit_mut(survivor).unwrap().damage = 10;
    assert_eq!(g.unit_might(g.unit_ref(survivor).unwrap()).unwrap(), 11);
    g.players[0].deck = vec![96];
    hook(&mut g, garen);
    resolve(&mut g);
    assert!(matches!(g.decision, Some(Decision::DeckLookPlay { .. })));
    assert!(
        g.unit_ref(survivor).is_some(),
        "Core321: Hook is still resolving while its look choice is pending"
    );
    select(&mut g, None);
    assert!(g.unit_ref(survivor).is_none());
}
#[test]
fn hook_reads_changed_might_after_a_real_reaction_not_activation_time() {
    let mut g = position();
    let target = unit(&mut g, 0, 96, None);
    g.players[0].deck = vec![145, 133]; // Discipline draws133 before Hook looks.
    hook(&mut g, target);
    cast(&mut g, 7, vec![target]);
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::DeckLookPlay {
            killed_might: Some(4),
            ..
        })
    ));
    assert_eq!(look(&g).1, vec![145]);
    assert_eq!(look(&g).2, vec![0]);
    assert!(g.players[0].hand.contains(&133));
}
#[test]
fn hook_actual_zero_might_and_negative_reference_are_not_null_kills() {
    for modifier in [0, -4] {
        let mut g = position();
        let target = unit(&mut g, 0, 273, None);
        g.unit_mut(target).unwrap().temporary_buff = modifier;
        g.players[0].deck = vec![96, 273];
        hook(&mut g, target);
        resolve(&mut g);
        assert!(g.unit_ref(target).is_none());
        assert!(matches!(
            g.decision,
            Some(Decision::DeckLookPlay {
                killed_might: Some(0),
                ..
            })
        ));
        assert_eq!(look(&g).2, vec![0]);
    }
}
#[test]
fn hook_completion_observer_is_pending_after_its_child_not_finalized_before_it() {
    let mut g = position();
    unit(&mut g, 0, 215, None); // Prize of Progress observes a completed gear use.
    let target = unit(&mut g, 0, 70, None);
    g.players[0].deck = vec![96];
    hook(&mut g, target);
    resolve(&mut g);
    select(&mut g, Some(0));
    assert!(matches!(g.decision, Some(Decision::PendingPlay { .. })));
    assert!(
        !g.chain.iter().any(|c| c.card_id == 215),
        "Core377.2.a/337.1: child was appended during Hook, before completed-use observer"
    );
    g.apply(finalize(false)).unwrap();
    assert_eq!(g.chain.iter().filter(|c| c.card_id == 215).count(), 1);
    resolve(&mut g);
    assert_eq!(
        g.players[0]
            .units
            .iter()
            .find(|u| u.card_id == 215)
            .unwrap()
            .temporary_buff,
        1
    );
}
