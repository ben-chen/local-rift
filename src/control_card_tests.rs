//! Printed Possession/Conscription instructions, exercised through player actions.
use super::*;
use test_support::{position, reject_unchanged, settle_unopposed_chain};

const POSSESSION: CardId = 468;
const CONSCRIPTION: CardId = 469;

fn field(g: &mut Game, owner: usize, card: CardId) -> Target {
    let unit_id = g.create_unit(owner, card, false, 0);
    let target = Target::Unit {
        player: owner,
        unit_id,
    };
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

fn play(target: Target, bonus: bool) -> Action {
    Action::WithPlayChoices {
        action: Box::new(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        }),
        choices: PlayChoices {
            bonus,
            ..Default::default()
        },
    }
}

fn response(g: &mut Game, responder: usize, card: CardId, target: Target) {
    if g.priority() != responder {
        g.apply(Action::Pass).unwrap();
    }
    g.players[responder].hand = vec![card];
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![target],
    })
    .unwrap();
}

#[test]
fn control_cards_preserve_physical_identity_and_recall_without_moving() {
    for actor in 0..2 {
        for card in [POSSESSION, CONSCRIPTION] {
            for ready in [false, true] {
                let mut g = position(actor, 0, 469001);
                g.players[actor].hand = vec![card];
                g.players[actor].xp = 9;
                let source = field(&mut g, 1 - actor, 71);
                let unit_id = id(source);
                {
                    let u = g.unit_mut(source).unwrap();
                    u.ready = ready;
                    u.damage = 2;
                    u.buff = 1;
                    u.stunned = true;
                }
                g.gear_play_at(1 - actor, 203, None).unwrap();
                let gear_id = g.players[1 - actor].gear.last().unwrap().id;
                assert!(g.attach_equipment(1 - actor, gear_id, unit_id));
                g.movement_locks.push((1 - actor, unit_id));
                g.battlefield_owner[0] = Some(1 - actor);
                let before_count = g.next_unit_id;
                g.apply(play(source, card == CONSCRIPTION)).unwrap();
                assert_eq!(g.unit_controller(unit_id), Some(1 - actor)); // responses first
                settle_unopposed_chain(&mut g);
                let target = Target::Unit {
                    player: actor,
                    unit_id,
                };
                let unit = g.unit_ref(target).unwrap();
                assert_eq!(
                    (unit.lane, unit.damage, unit.buff, unit.stunned),
                    (None, 2, 1, true)
                );
                assert_eq!(unit.ready, card == POSSESSION && ready);
                assert_eq!(g.next_unit_id, before_count); // no gear Move token
                assert!(!g.unit_moved_this_turn(actor, unit_id));
                assert_eq!(g.board_owner(target), Some(1 - actor));
                assert_eq!(g.gear_controller(gear_id), Some(1 - actor));
                assert_eq!(g.players[1 - actor].gear[0].attached_to, Some(unit_id));
                assert_eq!(
                    g.players[actor].xp,
                    if card == CONSCRIPTION { 4 } else { 9 }
                );
                assert!(g.pending_entries.is_empty() && g.showdown.is_none());
                assert_eq!(g.players[actor].score, 0);
                assert!(g.players[actor].discard.contains(&card));
            }
        }
    }
}

#[test]
fn conscription_preview_payment_and_finalization_agree_on_might_and_xp() {
    let mut g = position(0, 0, 469002);
    g.players[0].hand = vec![CONSCRIPTION];
    g.players[0].xp = 5;
    let small = field(&mut g, 1, 193);
    let large = field(&mut g, 1, 71);
    let ally = field(&mut g, 0, 193);
    let base = field(&mut g, 1, 193);
    g.unit_mut(base).unwrap().lane = None;
    let empty = |bonus| Action::WithPlayChoices {
        action: Box::new(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        }),
        choices: PlayChoices {
            bonus,
            ..Default::default()
        },
    };
    assert_eq!(
        g.play_target_view(&empty(false), 0).unwrap().candidates,
        vec![small]
    );
    assert_eq!(
        g.play_target_view(&empty(true), 0).unwrap().candidates,
        vec![small, large]
    );
    assert_eq!(g.play_choices(&empty(true)).unwrap().costs.xp, 5);
    for target in [large, ally, base] {
        reject_unchanged(&mut g, play(target, false));
    }
    for target in [ally, base] {
        reject_unchanged(&mut g, play(target, true));
    }
    g.players[0].xp = 4;
    reject_unchanged(&mut g, play(large, true));
    g.apply(play(small, false)).unwrap();
    settle_unopposed_chain(&mut g);
    assert_eq!(g.unit_controller(id(small)), Some(0));
    assert_eq!(g.players[0].xp, 4);
}

#[test]
fn conscription_rechecks_live_might_but_keeps_declared_additional_cost() {
    for paid in [false, true] {
        let mut g = position(0, 0, 469003);
        g.players[0].hand = vec![CONSCRIPTION];
        g.players[0].xp = 5;
        let target = field(&mut g, 1, 193);
        g.apply(play(target, paid)).unwrap();
        response(&mut g, 1, 7, target); // Discipline raises 2 Might to 4.
        settle_unopposed_chain(&mut g);
        assert_eq!(
            g.unit_controller(id(target)),
            Some(if paid { 0 } else { 1 })
        );
        assert_eq!(g.players[0].xp, if paid { 0 } else { 5 });
        assert!(g.players[0].discard.contains(&CONSCRIPTION));
    }
}

#[test]
fn countered_control_spells_leave_units_unchanged_and_costs_paid() {
    for card in [POSSESSION, CONSCRIPTION] {
        let mut g = position(0, 0, 469004);
        g.players[0].hand = vec![card];
        g.players[0].xp = 5;
        let target = field(&mut g, 1, 71);
        let before = format!("{:?}", g.unit_ref(target));
        g.apply(play(target, card == CONSCRIPTION)).unwrap();
        let chain_id = g.chain.last().unwrap().id;
        response(&mut g, 1, 86, Target::Spell { chain_id });
        settle_unopposed_chain(&mut g);
        assert_eq!(format!("{:?}", g.unit_ref(target)), before);
        assert_eq!(g.players[0].xp, if card == CONSCRIPTION { 0 } else { 5 });
    }
}

#[test]
fn conscription_repeat_pays_xp_once_and_uses_independent_targets() {
    let mut g = position(0, 0, 469005);
    g.players[0].hand = vec![CONSCRIPTION];
    g.players[0].xp = 5;
    let first = field(&mut g, 1, 71);
    let second = field(&mut g, 1, 71);
    g.grant_play(
        0,
        ChainSource::Card { card_id: 133 },
        PlayGrantKind::RepeatNextSpell,
    );
    let mut action = play(first, true);
    let option_id = g.play_choices(&action).unwrap().repeats[0].id;
    if let Action::WithPlayChoices { choices, .. } = &mut action {
        choices.repeats.push(play::RepeatSelection {
            option_id,
            choices: play::ExecutionChoices {
                targets: vec![second],
                ..Default::default()
            },
        });
    }
    assert_eq!(g.play_choices(&action).unwrap().costs.xp, 5);
    g.apply(action).unwrap();
    settle_unopposed_chain(&mut g);
    assert_eq!(g.players[0].xp, 0);
    for target in [first, second] {
        assert_eq!(g.unit_controller(id(target)), Some(0));
    }
}

#[test]
fn bot_can_choose_paid_conscription_when_only_large_targets_exist() {
    let mut g = position(0, 0, 469006);
    g.players[0].hand = vec![CONSCRIPTION];
    g.players[0].xp = 5;
    let target = field(&mut g, 1, 71);
    let actions = g.canonical_play_candidates(32).unwrap();
    assert!(!actions.is_empty());
    g.apply(actions[0].clone()).unwrap();
    settle_unopposed_chain(&mut g);
    assert_eq!(g.unit_controller(id(target)), Some(0));
    assert_eq!(g.players[0].xp, 0);
}

#[test]
fn control_ai_values_the_chosen_unit_and_conserves_unneeded_xp() {
    let mut g = position(0, 0, 469009);
    g.players[0].hand = vec![CONSCRIPTION];
    g.players[0].xp = 5;
    let small = field(&mut g, 1, 193);
    let large = field(&mut g, 1, 71);
    assert!(
        g.canonical_action_score(&play(small, false)).unwrap()
            > g.canonical_action_score(&play(small, true)).unwrap()
    );
    assert!(
        g.canonical_action_score(&play(large, true)).unwrap()
            > g.canonical_action_score(&play(small, false)).unwrap()
    );
}

#[test]
fn control_target_that_leaves_before_resolution_is_not_replaced_by_another_copy() {
    for card in [POSSESSION, CONSCRIPTION] {
        let mut g = position(0, 0, 469007);
        g.players[0].hand = vec![card];
        let target = field(&mut g, 1, 193);
        let other = field(&mut g, 1, 193);
        g.apply(play(target, false)).unwrap();
        response(&mut g, 1, 11, target); // Gust returns the selected physical card.
        settle_unopposed_chain(&mut g);
        assert_eq!(g.unit_controller(id(target)), None);
        assert_eq!(g.unit_controller(id(other)), Some(1));
        assert!(g.players[1].hand.contains(&193));
        assert!(g.players[0].units.is_empty());
    }
}

#[test]
fn conscription_frozen_payment_and_exact_power_pay_xp_once() {
    for staged in [false, true] {
        let mut g = position(0, 0, 469008);
        g.players[0].hand = vec![CONSCRIPTION];
        g.players[0].xp = 5;
        let target = field(&mut g, 1, 71);
        let action = play(target, true);
        if staged {
            g.apply(Action::BeginPayment {
                action: Box::new(action.clone()),
            })
            .unwrap();
            let plan = g.payment_plan_view(0).unwrap();
            assert_eq!(plan.xp, 5);
            assert_eq!(g.players[0].xp, 5);
            g.apply(Action::Choose { accept: false }).unwrap();
            assert_eq!(g.players[0].xp, 5);
            assert_eq!(g.players[0].hand, vec![CONSCRIPTION]);
            assert_eq!(g.players[0].energy_pool, 20);
            assert!(g.chain.is_empty());
            g.apply(Action::BeginPayment {
                action: Box::new(action),
            })
            .unwrap();
            g.apply(Action::CompletePayment).unwrap();
        } else {
            let before = format!("{g:?}");
            let payment = g.quote_power_payment(&action).unwrap().suggested;
            assert_eq!(format!("{g:?}"), before);
            g.apply(Action::WithPowerPayment {
                action: Box::new(action),
                payment,
            })
            .unwrap();
        }
        assert_eq!(g.players[0].xp, 0);
        settle_unopposed_chain(&mut g);
        assert_eq!(g.unit_controller(id(target)), Some(0));
        assert_eq!(g.players[0].xp, 0);
    }
}

#[test]
fn possession_has_action_timing_but_conscription_requires_an_open_turn() {
    for active in 0..2 {
        let mut g = position(active, 0, 469010);
        field(&mut g, 1 - active, 193);
        let moving = test_support::ready_unit(&mut g, active, 71);
        g.apply(Action::Move {
            unit_id: moving,
            lane: Some(0),
        })
        .unwrap();
        assert!(g.showdown.is_some() && g.chain.is_empty());
        let actor = g.priority();
        let target = Target::Unit {
            player: 1 - actor,
            unit_id: g.players[1 - actor].units[0].id,
        };
        let mut normal = g.clone();
        normal.players[actor].hand = vec![CONSCRIPTION];
        normal.players[actor].xp = 5;
        reject_unchanged(&mut normal, play(target, true));
        g.players[actor].hand = vec![POSSESSION];
        g.apply(play(target, false)).unwrap();
        assert_eq!(g.chain.last().unwrap().card_id, POSSESSION);
    }
}

#[test]
fn neither_control_spell_is_a_reaction_to_a_pending_spell() {
    let mut g = position(0, 0, 469011);
    for owner in 0..2 {
        field(&mut g, owner, 193);
    }
    g.players[0].hand = vec![133];
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    })
    .unwrap();
    let actor = g.priority();
    let target = Target::Unit {
        player: 1 - actor,
        unit_id: g.players[1 - actor].units[0].id,
    };
    for card in [POSSESSION, CONSCRIPTION] {
        let mut candidate = g.clone();
        candidate.players[actor].hand = vec![card];
        reject_unchanged(&mut candidate, play(target, false));
    }
}
