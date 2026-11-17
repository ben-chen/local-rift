//! Authored positions; every subsequent decision uses the public action path.
use super::*;
use crate::game::test_support::{
    position, ready_unit, reject_unchanged, settle_unopposed_chain, transfer_control_for_fixture,
};
use crate::gear::GearState;

fn gear(g: &mut Game, owner: usize, id: u32, card_id: CardId, attached_to: Option<u32>) -> Target {
    g.players[owner].gear.push(GearState {
        id,
        card_id,
        attached_to,
        lane: None,
        ready: true,
        temporary: false,
        token: false,
    });
    Target::Gear {
        player: owner,
        gear_id: id,
    }
}
fn cast() -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    }
}
fn reach_choice(g: &mut Game) {
    for _ in 0..20 {
        if matches!(g.decision, Some(Decision::PlayerObject { .. })) {
            return;
        }
        assert!(
            g.decision.is_none() && !g.chain.is_empty(),
            "{:?}",
            g.decision
        );
        g.apply(Action::Pass).unwrap();
    }
    panic!("no resolution choice");
}
fn choose(g: &mut Game, object: Option<Target>) -> u32 {
    let Some(Decision::PlayerObject { choice_id, .. }) = g.decision else {
        panic!("{:?}", g.decision)
    };
    g.apply(Action::ChoosePlayerObject { choice_id, object })
        .unwrap();
    choice_id
}

#[test]
fn each_player_kills_only_own_gear_and_cannot_respond_between_choices() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 48001);
        g.players[actor].hand = vec![480];
        let bearer = ready_unit(&mut g, actor, 88);
        let mine = gear(&mut g, actor, 50001, 3, Some(bearer));
        let theirs = gear(&mut g, 1 - actor, 50002, 3, None);
        g.apply(cast()).unwrap();
        assert!(g.chain.last().unwrap().targets.is_empty());
        reach_choice(&mut g);
        assert_eq!(g.priority(), actor);
        let Some(Decision::PlayerObject { choice_id, .. }) = g.decision else {
            unreachable!()
        };
        for action in [
            Action::Pass,
            Action::Choose { accept: false },
            Action::ChoosePlayerObject {
                choice_id,
                object: None,
            },
            Action::ChoosePlayerObject {
                choice_id,
                object: Some(theirs),
            },
        ] {
            reject_unchanged(&mut g, action);
        }
        let old = choose(&mut g, Some(mine));
        assert!(g.players[actor].gear.is_empty());
        assert!(g.players[actor].units.iter().any(|u| u.id == bearer));
        assert_eq!(g.priority(), 1 - actor);
        assert_eq!(g.chain.len(), 1);
        assert_eq!(g.player_object_candidates(), vec![theirs]);
        reject_unchanged(
            &mut g,
            Action::ChoosePlayerObject {
                choice_id: old,
                object: Some(theirs),
            },
        );
        choose(&mut g, Some(theirs));
        settle_unopposed_chain(&mut g);
        assert!(g.players[1 - actor].gear.is_empty());
        assert!(g.players[actor].discard.contains(&480));
    }
}

#[test]
fn acceptable_losses_starts_with_turn_player_even_when_opponent_casts_it() {
    let mut g = position(0, 0, 48002);
    let attacker = ready_unit(&mut g, 0, 88);
    let defender = ready_unit(&mut g, 1, 88);
    g.players[1]
        .units
        .iter_mut()
        .find(|u| u.id == defender)
        .unwrap()
        .lane = Some(0);
    g.battlefield_owner[0] = Some(1);
    let first = gear(&mut g, 0, 50011, 3, None);
    let second = gear(&mut g, 1, 50012, 3, None);
    g.apply(Action::Move {
        unit_id: attacker,
        lane: Some(0),
    })
    .unwrap();
    if g.priority() == 0 {
        g.apply(Action::Pass).unwrap();
    }
    assert_eq!(g.priority(), 1);
    assert_eq!(g.active, 0);
    g.players[1].hand = vec![480];
    g.apply(cast()).unwrap();
    reach_choice(&mut g);
    assert_eq!(g.priority(), 0);
    choose(&mut g, Some(first));
    assert_eq!(g.priority(), 1);
    choose(&mut g, Some(second));
}

#[test]
fn whirlwind_choices_are_not_targets_and_later_candidates_follow_the_first_return() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 48101);
        g.players[actor].hand = vec![481];
        let a = ready_unit(&mut g, actor, 0);
        let b = ready_unit(&mut g, 1 - actor, 0);
        let ours = Target::Unit {
            player: actor,
            unit_id: a,
        };
        let theirs = Target::Unit {
            player: 1 - actor,
            unit_id: b,
        };
        for seat in 0..2 {
            g.legend[seat].ready = true;
            for u in &mut g.players[seat].units {
                u.protected = true;
            }
        }
        reject_unchanged(
            &mut g,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![ours],
            },
        );
        g.apply(cast()).unwrap();
        let power = g.players[1 - actor].power_pool;
        reach_choice(&mut g);
        assert_eq!(g.priority(), 1 - actor);
        assert_eq!(g.player_object_candidates().len(), 2);
        choose(&mut g, Some(ours));
        assert_eq!(
            g.players[1 - actor].power_pool,
            power,
            "Deflect must not be charged"
        );
        assert_eq!(g.priority(), actor);
        assert_eq!(g.player_object_candidates(), vec![theirs]);
        assert!(g.legend.iter().all(|legend| legend.ready));
        choose(&mut g, None);
        settle_unopposed_chain(&mut g);
        assert!(g.players[actor].hand.contains(&0));
        assert_eq!(g.players[1 - actor].units[0].temporary_buff, 0);
        assert!(g.legend.iter().all(|legend| legend.ready));
    }
}

#[test]
fn empty_instructions_complete_and_a_player_without_gear_is_skipped() {
    for actor in 0..2 {
        for card in [480, 481] {
            let mut g = position(actor, 0, 48003);
            g.players[actor].hand = vec![card];
            g.apply(cast()).unwrap();
            settle_unopposed_chain(&mut g);
            assert!(g.players[actor].discard.contains(&card));
        }
        let mut g = position(actor, 0, 48004);
        g.players[actor].hand = vec![480];
        let object = gear(&mut g, 1 - actor, 50021, 3, None);
        g.apply(cast()).unwrap();
        reach_choice(&mut g);
        assert_eq!(g.priority(), 1 - actor);
        choose(&mut g, Some(object));
        settle_unopposed_chain(&mut g);
    }
}

#[test]
fn a_real_counter_prevents_every_resolution_choice() {
    for actor in 0..2 {
        for card in [480, 481] {
            let mut g = position(actor, 0, 48005);
            g.players[actor].hand = vec![card];
            g.players[1 - actor].hand = vec![12, 6];
            ready_unit(&mut g, actor, 88);
            gear(&mut g, actor, 50031, 3, None);
            g.apply(cast()).unwrap();
            let chain_id = g.chain.last().unwrap().id;
            if g.priority() == actor {
                g.apply(Action::Pass).unwrap();
            }
            // Not So Fast requires the spell to target a friendly object.
            // These spells target nothing; Defy can counter their printed costs.
            reject_unchanged(
                &mut g,
                Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![Target::Spell { chain_id }],
                },
            );
            g.apply(Action::PlayCardWithTargets {
                hand_index: 1,
                lane: None,
                targets: vec![Target::Spell { chain_id }],
            })
            .unwrap();
            settle_unopposed_chain(&mut g);
            assert_eq!(g.players[actor].units.len(), 1);
            assert_eq!(g.players[actor].gear.len(), 1);
        }
    }
}

#[test]
fn gear_deathknell_waits_until_both_players_finish_the_parent() {
    let mut g = position(0, 0, 48006);
    g.players[0].hand = vec![480];
    let fountain = gear(&mut g, 0, 50041, 360, None);
    let other = gear(&mut g, 1, 50042, 3, None);
    g.apply(cast()).unwrap();
    reach_choice(&mut g);
    choose(&mut g, Some(fountain));
    assert!(g.players[0].units.is_empty());
    assert_eq!(g.chain.len(), 1);
    assert_eq!(g.chain[0].card_id, 480);
    assert!(matches!(
        g.decision,
        Some(Decision::PlayerObject { player: 1, .. })
    ));
    choose(&mut g, Some(other));
    assert!(!g.chain.iter().any(|item| item.card_id == 480));
    for _ in 0..30 {
        if g.decision.is_none() && g.chain.is_empty() {
            break;
        }
        let action = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::TokenPlay { .. }) => Action::FinalizeTokenPlay {
                lane: None,
                accelerate: false,
            },
            None => Action::Pass,
            other => panic!("{other:?}"),
        };
        g.apply(action).unwrap();
    }
    assert_eq!(g.players[0].units.len(), 1);
    assert!(g.players[0].units[0].token);
}

#[test]
fn returning_a_borrowed_unit_uses_its_owner_and_detaches_equipment() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 48102);
        g.players[actor].hand = vec![481];
        let unit = ready_unit(&mut g, actor, 88);
        let borrowed = transfer_control_for_fixture(&mut g, unit);
        gear(&mut g, 1 - actor, 50051, 3, Some(unit));
        g.apply(cast()).unwrap();
        reach_choice(&mut g);
        choose(&mut g, Some(borrowed));
        settle_unopposed_chain(&mut g);
        assert!(g.players[actor].hand.contains(&88));
        assert!(!g.players[1 - actor].hand.contains(&88));
        assert_eq!(g.players[1 - actor].gear.len(), 1);
        assert_eq!(g.players[1 - actor].gear[0].attached_to, None);
    }
}

#[test]
fn each_repeat_has_fresh_per_player_choices_and_identifiers() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 48103);
        g.players[actor].hand = vec![481];
        ready_unit(&mut g, actor, 88);
        g.grant_play(
            actor,
            ChainSource::Card { card_id: 57 },
            PlayGrantKind::RepeatNextSpell,
        );
        let option_id = g.play_choices(&cast()).unwrap().repeats[0].id;
        g.apply(Action::WithPlayChoices {
            action: Box::new(cast()),
            choices: PlayChoices {
                repeats: vec![crate::play::RepeatSelection {
                    option_id,
                    choices: ExecutionChoices::default(),
                }],
                ..Default::default()
            },
        })
        .unwrap();
        reach_choice(&mut g);
        let mut ids = Vec::new();
        for expected in [1 - actor, actor, 1 - actor, actor] {
            assert_eq!(g.priority(), expected);
            ids.push(choose(&mut g, None));
        }
        settle_unopposed_chain(&mut g);
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 4);
        assert_eq!(
            g.players[actor]
                .discard
                .iter()
                .filter(|&&id| id == 481)
                .count(),
            1
        );
    }
}

#[test]
fn every_ai_level_can_finish_each_players_independent_choice() {
    for actor in 0..2 {
        for level in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            for card in [480, 481] {
                let mut g = position(actor, 0, 48104);
                g.difficulty = level;
                g.players[actor].hand = vec![card];
                for owner in 0..2 {
                    ready_unit(&mut g, owner, 88);
                    gear(&mut g, owner, 50100 + owner as u32, 3, None);
                }
                g.apply(cast()).unwrap();
                reach_choice(&mut g);
                for _ in 0..30 {
                    if g.decision.is_none() && g.chain.is_empty() {
                        break;
                    }
                    if g.decision.is_some() {
                        assert!(g.step_ai().unwrap());
                    } else {
                        g.apply(Action::Pass).unwrap();
                    }
                }
                assert!(
                    g.decision.is_none() && g.chain.is_empty(),
                    "{actor}/{level:?}/{card}"
                );
                assert!(g.players[actor].discard.contains(&card));
            }
        }
    }
}

#[test]
fn all_ai_levels_cast_a_favorable_each_player_spell_and_finish_its_choices() {
    for actor in 0..2 {
        for level in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            for card in [480, 481] {
                let mut g = position(actor, 0, 48105);
                g.difficulty = level;
                g.players[actor].hand = vec![card];
                if card == 480 {
                    gear(&mut g, 1 - actor, 50200, 3, None);
                } else {
                    ready_unit(&mut g, 1 - actor, 88);
                }
                assert!(g.step_ai().unwrap());
                assert_eq!(
                    g.chain.last().map(|item| item.card_id),
                    Some(card),
                    "{actor}/{level:?}/{card}"
                );
                for _ in 0..30 {
                    if g.decision.is_none() && g.chain.is_empty() {
                        break;
                    }
                    assert!(g.step_ai().unwrap());
                }
                assert!(g.decision.is_none() && g.chain.is_empty());
                assert!(g.players[actor].discard.contains(&card));
                if card == 480 {
                    assert!(g.players[1 - actor].gear.is_empty());
                    assert!(g.players[1 - actor].discard.contains(&3));
                } else {
                    assert!(g.players[1 - actor].units.is_empty());
                    assert!(g.players[1 - actor].hand.contains(&88));
                }
            }
        }
    }
}

#[test]
fn each_player_instructions_include_tokens_without_creating_physical_cards() {
    let mut g = position(0, 0, 48106);
    g.players[0].hand = vec![480];
    let token_gear = gear(&mut g, 0, 50201, 3, None);
    g.players[0].gear[0].token = true;
    g.apply(cast()).unwrap();
    reach_choice(&mut g);
    assert_eq!(g.player_object_candidates(), vec![token_gear]);
    choose(&mut g, Some(token_gear));
    settle_unopposed_chain(&mut g);
    assert!(g.players[0].gear.is_empty());
    assert_eq!(g.players[0].discard, vec![480]);

    let mut g = position(0, 0, 48107);
    g.players[0].hand = vec![481];
    let token_id = ready_unit(&mut g, 1, 88);
    g.players[1].units[0].token = true;
    gear(&mut g, 1, 50202, 3, Some(token_id));
    let token_unit = Target::Unit {
        player: 1,
        unit_id: token_id,
    };
    g.apply(cast()).unwrap();
    reach_choice(&mut g);
    assert_eq!(g.player_object_candidates(), vec![token_unit]);
    choose(&mut g, None);
    choose(&mut g, Some(token_unit));
    settle_unopposed_chain(&mut g);
    assert!(g.players[1].units.is_empty());
    assert!(g.players[1].hand.is_empty() && g.players[1].discard.is_empty());
    assert_eq!(g.players[1].gear[0].attached_to, None);
}
