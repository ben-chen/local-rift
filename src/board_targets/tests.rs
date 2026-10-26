//! Controlled board fixtures cover source liveness, actual target types and
//! parent/child ordering. They do not claim to be ordinary starting positions.
use super::*;
use crate::gear::GearState;

fn position() -> Game {
    let mut g = Game::new(3, 3, 13013, Difficulty::Tactical);
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
fn unit(g: &mut Game, player: usize, card: usize, token: bool, lane: Option<usize>) -> Target {
    let id = g.create_unit(player, card, token, if token { 3 } else { 0 });
    let target = Target::Unit {
        player,
        unit_id: id,
    };
    g.unit_mut(target).unwrap().lane = lane;
    target
}
fn uid(target: Target) -> u32 {
    let Target::Unit { unit_id, .. } = target else {
        panic!("unit fixture")
    };
    unit_id
}
fn gear(g: &mut Game, player: usize, card: usize, token: bool, attached_to: Option<u32>) -> Target {
    let gear_id = g.next_unit_id;
    g.next_unit_id += 1;
    g.players[player].gear.push(GearState {
        id: gear_id,
        card_id: card,
        token,
        lane: None,
        attached_to,
        ready: true,
        temporary: false,
    });
    Target::Gear { player, gear_id }
}
fn hand_action(g: &mut Game, card: usize, targets: Vec<Target>) -> Action {
    let player = g.priority();
    let hand_index = g.players[player].hand.len();
    g.players[player].hand.push(card);
    Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    }
}
fn drive(g: &mut Game, stop_at_targets: bool, stop_at_token: bool) {
    for _ in 0..300 {
        let action = match &g.decision {
            Some(Decision::TriggerTargets { .. }) if stop_at_targets => return,
            Some(Decision::TokenPlay { .. } | Decision::TokenPlacement { .. }) if stop_at_token => {
                return;
            }
            Some(Decision::TokenPlay { .. } | Decision::TokenPlacement { .. }) => {
                Action::PlaceToken {
                    lane: g.current_pending_token().unwrap().location.flatten(),
                }
            }
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::Legend { .. }) => Action::Choose { accept: false },
            Some(other) => panic!("Unexpected decision: {other:?}"),
            None if !g.chain.is_empty() => Action::Pass,
            None => return,
        };
        g.apply(action).unwrap();
    }
    panic!("Fixture failed to settle");
}
// A real Firelights' Hideout grant creates an optional child choice, so tests
// can observe the completed parent before a fixed-base token enters.
fn offer_accelerate(g: &mut Game, player: usize) {
    g.grant_play(
        player,
        ChainSource::Battlefield {
            lane: 0,
            battlefield_id: 5,
        },
        PlayGrantKind::AccelerateNextUnit,
    );
}
fn cast(g: &mut Game, card: usize, targets: Vec<Target>) {
    let action = hand_action(g, card, targets);
    g.apply(action).unwrap();
    drive(g, false, false);
}
fn conquer(g: &mut Game, source: Target) {
    let Target::Unit { player, unit_id } = source else {
        panic!()
    };
    // Conquer is emitted after control has changed. Preserve that event context.
    let lane = g.unit_ref(source).unwrap().lane.unwrap();
    g.battlefield_owner[lane] = Some(player);
    g.emit_unit_event(Occurrence::Conquer, player, unit_id);
    g.after_resolution().unwrap();
    drive(g, true, false);
    assert!(matches!(g.decision, Some(Decision::TriggerTargets { .. })));
}

#[test]
fn tokens_use_their_own_zero_cost_domains_and_tags() {
    let mut g = position();
    let gold = gear(&mut g, 1, 13, true, None);
    assert!(g.catalog[13].domains.contains(&Domain::Chaos));
    let facts = g.board_target_facts(gold).unwrap();
    assert_eq!(facts.printed_energy, 0);
    assert_eq!(facts.domains, &[]);
    assert!(!g.board_target_has_tag(gold, "Pyke"));
    let mech = unit(&mut g, 0, 54, true, None);
    assert_eq!(g.board_target_facts(mech).unwrap().domains, &[]);
    assert_eq!(g.board_target_facts(mech).unwrap().printed_energy, 0);
    assert!(g.board_target_has_tag(mech, "Mech"));
    assert!(!g.board_target_has_tag(mech, "Ferrous Forerunner"));
}

#[test]
fn bases_missing_objects_and_attached_gear_have_distinct_live_locations() {
    let mut g = position();
    let own = gear(&mut g, 0, 3, false, None);
    let enemy = gear(&mut g, 1, 3, false, None);
    assert_eq!(
        g.board_target_facts(own).unwrap().location,
        BoardLocation::Base { player: 0 }
    );
    assert_eq!(
        g.board_target_facts(enemy).unwrap().location,
        BoardLocation::Base { player: 1 }
    );
    let bearer = unit(&mut g, 1, 2, false, Some(0));
    let attached = gear(&mut g, 1, 3, false, Some(uid(bearer)));
    assert_eq!(
        g.board_target_facts(attached).unwrap().location,
        BoardLocation::Battlefield { lane: 0 }
    );
    assert!(!g.can_choose_unit(0, bearer));
    assert!(
        g.can_choose_board_target(0, attached),
        "Bearer's protection does not protect equipment"
    );
    g.unit_mut(bearer).unwrap().lane = Some(1);
    assert_eq!(g.target_location(attached), Some((Some(1), 0)));
    g.bounce(bearer);
    assert_eq!(g.target_location(attached), Some((Some(1), 0)));
    g.cleanup_gear_locations();
    assert_eq!(g.target_location(attached), Some((None, 1)));
    g.kill_gear_target(attached);
    assert_eq!(g.target_location(attached), None);
    assert!(!g.can_choose_board_target(0, attached));
}

#[test]
fn detonate_draws_for_the_actual_controller_and_tokens_do_not_enter_trash() {
    for owner in [0, 1] {
        let mut g = position();
        let gold = gear(&mut g, owner, 13, true, None);
        cast(&mut g, 362, vec![gold]);
        assert_eq!(g.players[owner].hand.len(), 2);
        assert_eq!(g.players[1 - owner].hand.len(), 0);
        assert!(!g.players[owner].discard.contains(&13));
        assert!(g.board_target_facts(gold).is_none());
    }
}

#[test]
fn detonate_invalidated_by_response_does_not_draw_for_old_controller() {
    let mut g = position();
    let target = gear(&mut g, 1, 3, false, None);
    let action = hand_action(&mut g, 362, vec![target]);
    g.apply(action).unwrap();
    let old = g.clone();
    assert_eq!(g.chain.last().unwrap().targets, vec![target]);
    g.kill_gear_target(target);
    drive(&mut g, false, false);
    assert!(g.players[1].hand.is_empty());
    assert_eq!(g.players[1].discard, vec![3]);
    assert!(old.board_target_facts(target).is_some());
}

#[test]
fn detonate_repeat_rechecks_exact_target_and_draws_only_once() {
    let mut g = position();
    let target = gear(&mut g, 1, 3, false, None);
    g.grant_play(
        0,
        ChainSource::Card { card_id: 255 },
        PlayGrantKind::RepeatNextSpell,
    );
    let action = hand_action(&mut g, 362, vec![target]);
    let repeat = g.play_choices(&action).unwrap().repeats[0].id;
    g.apply(Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats: vec![RepeatSelection {
                option_id: repeat,
                choices: ExecutionChoices {
                    targets: vec![target],
                    mode: None,
                },
            }],
            ..Default::default()
        },
    })
    .unwrap();
    drive(&mut g, false, false);
    assert_eq!(g.players[1].hand.len(), 2);
    assert_eq!(g.players[1].discard, vec![3]);
}

#[test]
fn decree_uses_actual_enemy_domains_and_includes_units_in_base() {
    let mut g = position();
    let enemy = unit(&mut g, 1, 13, false, None);
    let friendly = unit(&mut g, 0, 13, false, None);
    let wrong = unit(&mut g, 1, 0, false, None);
    let chaos_gear = gear(&mut g, 1, 3, false, Some(uid(wrong)));
    let gold = gear(&mut g, 1, 13, true, None);
    let candidates = g.card_target_candidates(0, 365, None, &[]).unwrap();
    assert!(candidates.contains(&enemy) && candidates.contains(&chaos_gear));
    assert!(
        !candidates.contains(&friendly)
            && !candidates.contains(&wrong)
            && !candidates.contains(&gold)
    );
    let bad = hand_action(&mut g, 365, vec![gold]);
    let before = format!("{g:?}");
    assert!(g.apply(bad).is_err());
    assert_eq!(format!("{g:?}"), before);
    // Clear the rejected draft card, then exercise a real base-unit Kill.
    g.players[0].hand.clear();
    cast(&mut g, 365, vec![enemy]);
    assert!(g.unit_ref(enemy).is_none());
    assert_eq!(g.players[1].discard, vec![13]);
    cast(&mut g, 365, vec![chaos_gear]);
    assert!(g.board_target_facts(chaos_gear).is_none());
    assert!(
        g.unit_ref(wrong).is_some(),
        "Destroying the Chaos gear preserves its non-Chaos bearer"
    );
}

#[test]
fn thermo_fixes_all_gear_deaths_before_cleanup_or_fountain_children() {
    let mut g = position();
    for player in 0..2 {
        offer_accelerate(&mut g, player);
    }
    let bearer = unit(&mut g, 0, 0, false, Some(0));
    gear(&mut g, 0, 3, false, Some(uid(bearer)));
    gear(&mut g, 0, 10, false, Some(uid(bearer)));
    g.unit_mut(bearer).unwrap().damage = 5;
    assert!(g.unit_might(g.unit_ref(bearer).unwrap()).unwrap() > 5);
    gear(&mut g, 0, 360, false, None);
    gear(&mut g, 1, 360, false, None);
    gear(&mut g, 1, 13, true, None);
    let action = hand_action(&mut g, 363, vec![]);
    g.apply(action).unwrap();
    drive(&mut g, false, true);
    assert!(g.players.iter().all(|p| p.gear.is_empty()));
    assert!(
        g.unit_ref(bearer).is_none(),
        "Killed Angel cannot replace later unit cleanup"
    );
    assert!(
        g.players.iter().all(|p| p.units.is_empty()),
        "Children wait until parent completes"
    );
    drive(&mut g, false, false);
    for player in 0..2 {
        assert_eq!(g.players[player].units.len(), 1);
        assert!(g.players[player].units[0].token);
        assert!(g.players[player].units[0].ready);
        assert!(g.players[player].discard.contains(&360));
        assert!(!g.players[player].discard.contains(&13));
    }
}

#[test]
fn rake_can_choose_both_players_attached_gear_or_decline() {
    for decline in [true, false] {
        let mut g = position();
        let bearer = unit(&mut g, 1, 2, false, Some(0));
        let enemy = gear(&mut g, 1, 3, false, Some(uid(bearer)));
        let friendly = gear(&mut g, 0, 21, false, None);
        let action = hand_action(&mut g, 366, vec![]);
        g.apply(action).unwrap();
        drive(&mut g, true, false);
        assert_eq!(g.trigger_target_bounds(), Some((1, 0)));
        assert!(g.valid_trigger_target(enemy, &[]).unwrap());
        assert!(g.valid_trigger_target(friendly, &[]).unwrap());
        assert!(!g.valid_trigger_target(bearer, &[]).unwrap());
        assert!(
            g.trigger_target_candidates()
                .unwrap()
                .iter()
                .all(|t| matches!(t, Target::Gear { .. }))
        );
        g.apply(Action::ChooseTriggerTargets {
            targets: if decline { vec![] } else { vec![enemy] },
        })
        .unwrap();
        drive(&mut g, false, false);
        assert_eq!(g.board_target_facts(enemy).is_some(), decline);
        assert!(g.unit_ref(bearer).is_some());
    }
}

#[test]
fn goatherd_checks_printed_energy_not_discounted_cost_or_factory_cost() {
    let mut g = position();
    let low = gear(&mut g, 1, 3, false, None);
    let boundary_id = g
        .catalog
        .iter()
        .find(|c| c.kind == CardKind::Gear && c.cost == 4)
        .unwrap()
        .id;
    let boundary = gear(&mut g, 1, boundary_id, false, None);
    let high_id = g
        .catalog
        .iter()
        .find(|c| c.kind == CardKind::Gear && c.cost > 4)
        .unwrap()
        .id;
    let high = gear(&mut g, 1, high_id, false, None);
    let gold = gear(&mut g, 1, 13, true, None);
    let action = hand_action(&mut g, 367, vec![]);
    g.apply(action).unwrap();
    drive(&mut g, true, false);
    assert!(g.valid_trigger_target(low, &[]).unwrap());
    assert!(g.valid_trigger_target(boundary, &[]).unwrap());
    assert!(g.valid_trigger_target(gold, &[]).unwrap());
    assert!(!g.valid_trigger_target(high, &[]).unwrap());
    let source = g.players[0]
        .units
        .iter()
        .find(|u| u.card_id == 367)
        .unwrap();
    assert_eq!(g.unit_keywords(source).unwrap().shield, 2);
}

#[test]
fn demolitionist_uses_live_might_and_missing_source_is_not_zero() {
    let mut g = position();
    let source = unit(&mut g, 0, 368, false, Some(0));
    let cost3 = gear(&mut g, 1, 3, false, None);
    let gold = gear(&mut g, 1, 13, true, None);
    conquer(&mut g, source);
    assert!(!g.valid_trigger_target(cost3, &[]).unwrap());
    g.modify_might(source, 2, 0).unwrap();
    assert!(g.valid_trigger_target(cost3, &[]).unwrap());
    g.modify_might(source, -8, 0).unwrap();
    assert!(g.actual_unit_might(g.unit_ref(source).unwrap()).unwrap() < 0);
    assert!(
        g.valid_trigger_target(gold, &[]).unwrap(),
        "Referenced negative Might is zero"
    );
    g.bounce(source);
    assert!(
        !g.valid_trigger_target(gold, &[]).unwrap(),
        "Absent source has null Might"
    );
}

#[test]
fn demolitionist_revalidates_source_might_after_target_finalization() {
    let mut g = position();
    let source = unit(&mut g, 0, 368, false, Some(0));
    g.modify_might(source, 2, 0).unwrap();
    let target = gear(&mut g, 1, 3, false, None);
    conquer(&mut g, source);
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![target],
    })
    .unwrap();
    g.modify_might(source, -2, 0).unwrap();
    drive(&mut g, false, false);
    assert!(g.board_target_facts(target).is_some());
}

#[test]
fn adaptatron_buffs_only_after_actual_kill_and_never_a_returned_new_source() {
    for mode in 0..3 {
        let mut g = position();
        let source = unit(&mut g, 0, 369, false, Some(0));
        let target = gear(&mut g, 1, 3, false, None);
        conquer(&mut g, source);
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target],
        })
        .unwrap();
        if mode == 1 {
            g.kill_gear_target(target);
        }
        let replacement = if mode == 2 {
            g.bounce(source);
            Some(unit(&mut g, 0, 369, false, Some(0)))
        } else {
            None
        };
        drive(&mut g, false, false);
        assert_eq!(g.buffed_units.contains(&uid(source)), mode == 0);
        if let Some(new) = replacement {
            assert!(!g.buffed_units.contains(&uid(new)));
        }
    }
}

#[test]
fn dropboarder_condition_is_at_play_event_and_ready_is_a_real_event() {
    let mut g = position();
    unit(&mut g, 0, 196, false, None); // Ready observer: +1 Might to the readied unit.
    let first = gear(&mut g, 0, 3, false, None);
    gear(&mut g, 0, 13, true, None);
    let action = hand_action(&mut g, 370, vec![]);
    g.apply(action).unwrap();
    // Stop before its finalized triggered ability resolves.
    assert!(!g.chain.is_empty() || !g.pending_triggers.is_empty());
    g.kill_gear_target(first);
    drive(&mut g, false, false);
    assert!(
        g.players[0]
            .units
            .iter()
            .find(|u| u.card_id == 370)
            .unwrap()
            .ready
    );
    let readied = g.players[0]
        .units
        .iter()
        .find(|u| u.card_id == 370)
        .unwrap();
    assert_eq!(g.unit_might(readied).unwrap(), 5);
    let mut no_trigger = position();
    gear(&mut no_trigger, 0, 13, true, None);
    cast(&mut no_trigger, 370, vec![]);
    gear(&mut no_trigger, 0, 3, false, None);
    assert!(
        !no_trigger.players[0]
            .units
            .iter()
            .find(|u| u.card_id == 370)
            .unwrap()
            .ready
    );
}

#[test]
fn production_surge_discount_uses_real_mech_tag_and_draw_precedes_pending_token() {
    let mut g = position();
    offer_accelerate(&mut g, 0);
    let action = hand_action(&mut g, 371, vec![]);
    assert_eq!(g.cost_plan(&action).unwrap().energy, 4);
    let mech = unit(&mut g, 0, 54, true, None);
    assert_eq!(g.cost_plan(&action).unwrap().energy, 2);
    g.bounce(mech);
    assert_eq!(g.cost_plan(&action).unwrap().energy, 4);
    g.apply(action).unwrap();
    drive(&mut g, false, true);
    assert_eq!(g.players[0].hand.len(), 1);
    assert!(g.players[0].units.is_empty());
    let pending = g.current_pending_token().unwrap();
    assert_eq!(pending.location, Some(None));
    assert_eq!(pending.might, 3);
    assert!(!pending.entry_ready);
    drive(&mut g, false, false);
    let mech = &g.players[0].units[0];
    assert!(mech.token && !mech.ready && mech.lane.is_none());
    assert_eq!(g.unit_might(mech).unwrap(), 3);
    assert!(g.board_target_has_tag(
        Target::Unit {
            player: 0,
            unit_id: mech.id
        },
        "Mech"
    ));
}

#[test]
fn brittle_steel_flow_pays_its_printed_cost_and_banishes_after_resolution() {
    let mut g = position();
    let target = gear(&mut g, 1, 3, false, None);
    g.players[0].discard.push(364);
    let action = Action::PlayFromTrashOptions {
        trash_index: 0,
        lane: None,
        targets: vec![target],
        repeat_targets: vec![],
        repeat: false,
        accelerate: false,
        bonus: false,
        flow: Some(FlowSource::Printed),
    };
    assert_eq!(g.cost_plan(&action).unwrap().energy, 4);
    assert_eq!(g.flow_costs(0, 0).len(), 1);
    g.apply(action).unwrap();
    drive(&mut g, false, false);
    assert!(g.board_target_facts(target).is_none());
    assert_eq!(g.players[0].banish, vec![364]);
    assert!(g.players[0].discard.is_empty());
}

#[test]
fn decree_uses_real_unit_death_replacement_without_destroying_the_bearer() {
    let mut g = position();
    let bearer = unit(&mut g, 1, 13, false, Some(0));
    gear(&mut g, 1, 10, false, Some(uid(bearer)));
    cast(&mut g, 365, vec![bearer]);
    assert!(g.unit_ref(bearer).is_some());
    assert_eq!(g.unit_ref(bearer).unwrap().lane, None);
    assert!(g.players[1].gear.is_empty());
    assert!(g.players[1].discard.contains(&10));
    assert!(!g.players[1].discard.contains(&13));
}

#[test]
fn production_surge_freezes_mech_reduction_when_payment_begins() {
    let mut g = position();
    let mech = unit(&mut g, 0, 54, true, None);
    let action = hand_action(&mut g, 371, vec![]);
    g.apply(Action::BeginPayment {
        action: Box::new(action),
    })
    .unwrap();
    assert_eq!(g.payment_plan_view(0).unwrap().energy, 2);
    g.bounce(mech);
    assert_eq!(g.payment_plan_view(0).unwrap().energy, 2);
    let before = g.players[0].energy_pool;
    g.apply(Action::CompletePayment).unwrap();
    assert_eq!(g.players[0].energy_pool, before - 2);
    drive(&mut g, false, false);
    assert_eq!(g.players[0].hand.len(), 1);
    assert_eq!(g.players[0].units.len(), 1);
}

#[test]
fn repeated_production_surge_draws_twice_before_distinct_pending_children_enter() {
    let mut g = position();
    offer_accelerate(&mut g, 0);
    g.grant_play(
        0,
        ChainSource::Card { card_id: 255 },
        PlayGrantKind::RepeatNextSpell,
    );
    let action = hand_action(&mut g, 371, vec![]);
    let repeat = g.play_choices(&action).unwrap().repeats[0].id;
    g.apply(Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats: vec![RepeatSelection {
                option_id: repeat,
                choices: ExecutionChoices::default(),
            }],
            ..Default::default()
        },
    })
    .unwrap();
    drive(&mut g, false, true);
    assert_eq!(g.players[0].hand.len(), 2);
    assert!(g.players[0].units.is_empty());
    drive(&mut g, false, false);
    assert_eq!(g.players[0].units.len(), 2);
    assert_ne!(g.players[0].units[0].id, g.players[0].units[1].id);
    assert!(
        g.players[0]
            .units
            .iter()
            .all(|u| u.token && u.lane.is_none() && !u.ready)
    );
}
