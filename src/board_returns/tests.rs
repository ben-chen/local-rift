//! Controlled board fixtures cover source liveness, actual target types and
//! parent/child ordering. They do not claim to be ordinary starting positions.
use super::*;
use crate::game::board_targets::BoardLocation;
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

fn cast(g: &mut Game, card: usize, targets: Vec<Target>) {
    let action = hand_action(g, card, targets);
    g.apply(action).unwrap();
    drive(g, false, false);
}
fn death_choice(g: &mut Game, spirit: Target) {
    g.kill_unit(spirit).unwrap();
    g.after_resolution().unwrap();
    drive(g, true, false);
    assert!(matches!(
        g.decision,
        Some(Decision::TriggerTargets { card_id: 374, .. })
    ));
}
fn choose(g: &mut Game, target: Option<Target>) {
    g.apply(Action::ChooseTriggerTargets {
        targets: target.into_iter().collect(),
    })
    .unwrap();
    drive(g, false, false);
}

#[test]
fn factory_recall_returns_attached_gear_to_its_controller_without_killing_it() {
    for owner in [0, 1] {
        let mut g = position();
        let bearer = unit(&mut g, owner, 25, false, Some(0));
        let item = gear(&mut g, owner, 3, false, Some(uid(bearer)));
        let Target::Gear { gear_id, .. } = item else {
            unreachable!()
        };
        g.granted_temporary_gear.push(gear_id);
        g.empowered_units.push(gear_id);
        cast(&mut g, 372, vec![item]);
        assert_eq!(g.players[owner].hand, vec![3]);
        assert!(g.players[owner].gear.is_empty());
        assert!(g.unit_ref(bearer).is_some());
        assert!(!g.players[owner].discard.contains(&3));
        assert!(!g.granted_temporary_gear.contains(&gear_id));
        assert!(!g.empowered_units.contains(&gear_id));
        assert_eq!(g.players[owner].units.len(), 1);
        assert!(g.players[1 - owner].units.is_empty());
    }
}

#[test]
fn gear_tokens_cease_to_exist_and_absent_or_unit_targets_fail_atomically() {
    let mut g = position();
    let gold = gear(&mut g, 1, 13, true, None);
    cast(&mut g, 372, vec![gold]);
    assert!(g.players[1].hand.is_empty() && g.players[1].discard.is_empty());
    assert!(g.board_target_facts(gold).is_none());
    let target = unit(&mut g, 1, 25, false, None);
    for invalid in [
        target,
        gold,
        Target::Gear {
            player: 90,
            gear_id: 1,
        },
    ] {
        let action = hand_action(&mut g, 372, vec![invalid]);
        let before = format!("{g:?}");
        assert!(g.apply(action).is_err());
        assert_eq!(format!("{g:?}"), before);
    }
}

#[test]
fn bounced_unit_detaches_gear_without_returning_or_readying_it() {
    let mut g = position();
    let bearer = unit(&mut g, 1, 25, false, Some(0));
    gear(&mut g, 1, 3, false, Some(uid(bearer)));
    g.players[1].gear[0].ready = false;
    g.return_board_targets(&[bearer]);
    assert_eq!(g.players[1].hand, vec![25]);
    assert_eq!(g.players[1].gear[0].attached_to, None);
    assert!(!g.players[1].gear[0].ready);
    assert!(g.pending_triggers.is_empty());
}

#[test]
fn downwell_returns_one_fixed_set_without_deathknells_or_duplicate_gear() {
    let mut g = position();
    for player in 0..2 {
        let spirit = unit(&mut g, player, 374, false, Some(player));
        gear(&mut g, player, 360, false, None);
        gear(&mut g, player, 3, false, Some(uid(spirit)));
        gear(&mut g, player, 13, true, None);
        unit(&mut g, player, 54, true, None);
    }
    cast(&mut g, 373, vec![]);
    for p in &g.players {
        assert!(p.units.is_empty() && p.gear.is_empty());
        let mut cards = p.hand.clone();
        cards.sort();
        assert_eq!(cards, vec![3, 360, 374]);
        assert!(!p.discard.contains(&374) && !p.discard.contains(&360));
    }
    assert!(g.pending_triggers.is_empty() && g.decision.is_none());
}

#[test]
fn factory_resolution_rechecks_physical_identity_after_card_is_replayed() {
    let mut g = position();
    let old = gear(&mut g, 1, 3, false, None);
    let action = hand_action(&mut g, 372, vec![old]);
    g.apply(action).unwrap();
    g.return_board_targets(&[old, old]);
    g.players[1].hand.clear(); // Controlled replay of that physical card.
    let new = gear(&mut g, 1, 3, false, None);
    drive(&mut g, false, false);
    assert!(g.board_target_facts(new).is_some());
    assert!(g.players[1].hand.is_empty());
}

#[test]
fn angler_checks_current_might_at_resolution_and_keeps_gear_on_board() {
    let mut g = position();
    let low = unit(&mut g, 1, 374, false, Some(0));
    let high = unit(&mut g, 0, 375, false, Some(1));
    gear(&mut g, 1, 3, false, Some(uid(low)));
    // The equipment raises Might: force precise current values in the fixture.
    let amount = 2 - i128::from(g.unit_might(g.unit_ref(low).unwrap()).unwrap());
    g.modify_might(low, amount, 0).unwrap();
    g.modify_might(high, -20, 0).unwrap();
    let action = hand_action(&mut g, 375, vec![]);
    g.apply(action).unwrap();
    drive(&mut g, false, false);
    assert!(g.unit_ref(low).is_none() && g.unit_ref(high).is_none());
    assert_eq!(g.players[1].hand, vec![374]);
    assert!(g.players[0].hand.contains(&375));
    assert_eq!(g.players[0].units.len(), 1); // Newly played 5 Might Angler.
    assert_eq!(g.players[1].gear[0].attached_to, None);
    assert!(g.decision.is_none()); // Returned Spirit never dies.
}

#[test]
fn mournful_remembers_its_battlefield_and_excludes_both_bases() {
    let mut g = position();
    let spirit = unit(&mut g, 0, 374, false, Some(1));
    let enemy = unit(&mut g, 1, 25, false, Some(1));
    let enemy_gear = gear(&mut g, 1, 3, false, Some(uid(enemy)));
    let detached = gear(&mut g, 0, 3, false, Some(uid(spirit)));
    let own_base = unit(&mut g, 0, 25, false, None);
    let enemy_base = gear(&mut g, 1, 3, false, None);
    let elsewhere = unit(&mut g, 1, 25, false, Some(0));
    death_choice(&mut g, spirit);
    assert_eq!(g.trigger_target_bounds(), Some((1, 0)));
    assert_eq!(
        g.trigger_target_location(),
        Some(BoardLocation::Battlefield { lane: 1 })
    );
    for target in [enemy, enemy_gear] {
        assert!(g.valid_trigger_target(target, &[]).unwrap());
    }
    for target in [spirit, detached, own_base, enemy_base, elsewhere] {
        assert!(!g.valid_trigger_target(target, &[]).unwrap());
    }
    choose(&mut g, Some(enemy_gear));
    assert!(g.unit_ref(enemy).is_some());
    assert!(g.board_target_facts(enemy_gear).is_none());
    assert!(g.players[1].discard.contains(&3));
}

#[test]
fn mournful_base_is_owner_specific_and_can_decline() {
    for player in [0, 1] {
        let mut g = position();
        let spirit = unit(&mut g, player, 374, false, None);
        let own = gear(&mut g, player, 3, false, Some(uid(spirit)));
        let other = gear(&mut g, 1 - player, 3, false, None);
        death_choice(&mut g, spirit);
        assert_eq!(
            g.trigger_target_location(),
            Some(BoardLocation::Base { player })
        );
        assert!(g.valid_trigger_target(own, &[]).unwrap());
        assert!(!g.valid_trigger_target(other, &[]).unwrap());
        choose(&mut g, None);
        assert!(g.board_target_facts(own).is_some());
    }
}

#[test]
fn mournful_missing_snapshot_is_not_either_base_and_bad_target_is_atomic() {
    let mut g = position();
    let spirit = unit(&mut g, 0, 374, false, None);
    let target = gear(&mut g, 0, 3, false, None);
    death_choice(&mut g, spirit);
    let Some(Decision::TriggerTargets { context, .. }) = &mut g.decision else {
        panic!()
    };
    context.departure_location = None;
    assert_eq!(g.trigger_target_location(), None);
    assert!(!g.valid_trigger_target(target, &[]).unwrap());
    let before = format!("{g:?}");
    assert!(
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target]
        })
        .is_err()
    );
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn mournful_unit_kill_keeps_death_replacements() {
    let mut g = position();
    let spirit = unit(&mut g, 0, 374, false, Some(0));
    let enemy = unit(&mut g, 1, 13, false, Some(0));
    gear(&mut g, 1, 10, false, Some(uid(enemy)));
    death_choice(&mut g, spirit);
    choose(&mut g, Some(enemy));
    assert_eq!(g.unit_ref(enemy).unwrap().lane, None);
    assert!(g.players[1].discard.contains(&10));
    assert!(!g.players[1].discard.contains(&13));
}

#[test]
fn mournful_moved_target_mistargets_when_ability_resolves() {
    let mut g = position();
    let spirit = unit(&mut g, 0, 374, false, Some(0));
    let enemy = unit(&mut g, 1, 25, false, Some(0));
    death_choice(&mut g, spirit);
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![enemy],
    })
    .unwrap();
    g.unit_mut(enemy).unwrap().lane = Some(1);
    drive(&mut g, false, false);
    assert!(g.unit_ref(enemy).is_some());
}

#[test]
fn tactical_ai_declines_friendly_death_target_but_chooses_enemy() {
    let mut g = position();
    let spirit = unit(&mut g, 0, 374, false, Some(0));
    unit(&mut g, 0, 25, false, Some(0));
    death_choice(&mut g, spirit);
    assert_eq!(
        g.choose_ai_action().unwrap(),
        Action::ChooseTriggerTargets { targets: vec![] }
    );
    let enemy = gear(&mut g, 1, 3, false, None);
    // An attached opponent gear enters the remembered location.
    let bearer = unit(&mut g, 1, 375, false, Some(0));
    g.players[1].gear[0].attached_to = Some(uid(bearer));
    let Action::ChooseTriggerTargets { targets } = g.choose_ai_action().unwrap() else {
        panic!()
    };
    assert!(targets == vec![enemy] || targets == vec![bearer]);
}

#[test]
fn angler_freezes_recipients_before_aura_departures() {
    let mut g = position();
    let aura = unit(&mut g, 0, 320, false, None);
    let ally = unit(&mut g, 0, 25, false, None);
    g.modify_might(aura, -3, 0).unwrap();
    assert_eq!(g.unit_might(g.unit_ref(aura).unwrap()).unwrap(), 2);
    assert_eq!(g.unit_might(g.unit_ref(ally).unwrap()).unwrap(), 3);
    cast(&mut g, 375, vec![]);
    assert!(g.unit_ref(aura).is_none());
    assert_eq!(g.unit_might(g.unit_ref(ally).unwrap()).unwrap(), 2);
    assert!(
        g.unit_ref(ally).is_some(),
        "It did not qualify in the original set"
    );
}

#[test]
fn angler_threshold_is_evaluated_on_trigger_resolution_not_play() {
    let mut g = position();
    let low = unit(&mut g, 1, 374, false, Some(0));
    let high = unit(&mut g, 1, 375, false, Some(1));
    let action = hand_action(&mut g, 375, vec![]);
    g.apply(action).unwrap();
    for _ in 0..10 {
        if g.chain
            .iter()
            .any(|c| c.card_id == 375 && c.ability.is_some())
        {
            break;
        }
        g.apply(Action::Pass).unwrap();
    }
    assert!(
        g.chain
            .iter()
            .any(|c| c.card_id == 375 && c.ability.is_some())
    );
    g.modify_might(low, 4, 0).unwrap();
    g.modify_might(high, -4, 0).unwrap();
    drive(&mut g, false, false);
    assert!(g.unit_ref(low).is_some());
    assert!(g.unit_ref(high).is_none());
}

#[test]
fn live_here_source_absence_is_not_changed_to_departure_lookup() {
    let mut g = position();
    let target = unit(&mut g, 1, 25, false, Some(0));
    g.decision = Some(Decision::TriggerTargets {
        player: 0,
        card_id: 167,
        rule_index: 0,
        context: TriggerContext {
            source: TriggerSource {
                player: 0,
                unit_id: Some(90000),
                gear_id: None,
            },
            subject: None,
            movement: None,
            departure_location: Some(BoardLocation::Battlefield { lane: 0 }),
        },
    });
    assert_eq!(g.trigger_target_location(), None);
    assert!(!g.valid_trigger_target(target, &[]).unwrap());
}

#[test]
fn repeated_factory_recall_returns_a_physical_card_only_once() {
    let mut g = position();
    let target = gear(&mut g, 1, 3, false, None);
    g.grant_play(
        0,
        ChainSource::Card { card_id: 255 },
        PlayGrantKind::RepeatNextSpell,
    );
    let action = hand_action(&mut g, 372, vec![target]);
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
    assert_eq!(g.players[1].hand, vec![3]);
    assert!(g.players[1].discard.is_empty() && g.players[1].gear.is_empty());
}

#[test]
fn a_replaced_spirit_death_never_creates_its_deathknell() {
    let mut g = position();
    let spirit = unit(&mut g, 0, 374, false, Some(0));
    gear(&mut g, 0, 10, false, Some(uid(spirit)));
    unit(&mut g, 1, 25, false, Some(0));
    g.kill_unit(spirit).unwrap();
    g.after_resolution().unwrap();
    drive(&mut g, true, false);
    assert!(g.decision.is_none());
    assert_eq!(g.unit_ref(spirit).unwrap().lane, None);
}

#[path = "c15.rs"]
mod c15;

#[path = "c17.rs"]
mod c17;
