//! Public-action coverage for the reviewed event-trigger family (152–215).
//! Small fixtures isolate actual card interactions; they are not deck lists.
use local_rift::game::{Action, Decision, Difficulty, Game, Target, Unit};
use local_rift::gear::GearState;
use local_rift::hidden::HiddenCard;

fn game() -> Game {
    let mut g = Game::new(3, 3, 382, Difficulty::Tactical);
    g.start_match().unwrap();
    g.battlefields = [1, 1];
    for p in &mut g.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.discard.clear();
        p.runes = [6; 6];
        p.exhausted_runes = [0; 6];
        p.energy_pool = 0;
        p.energy = 36;
        p.deck = vec![69; 30];
    }
    g
}
fn unit(g: &mut Game, p: usize, id: u32, card: usize, lane: Option<usize>, ready: bool) -> Target {
    g.players[p].units.push(Unit {
        id,
        card_id: card,
        lane,
        ready,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    });
    Target::Unit {
        player: p,
        unit_id: id,
    }
}
fn gear(g: &mut Game, p: usize, id: u32, card: usize, bearer: Option<u32>) {
    g.players[p].gear.push(GearState {
        id,
        card_id: card,
        attached_to: bearer,
        ready: true,
        temporary: false,
        token: false,
    });
}
fn u(g: &Game, t: Target) -> &Unit {
    let Target::Unit { player, unit_id } = t else {
        panic!()
    };
    g.players[player]
        .units
        .iter()
        .find(|u| u.id == unit_id)
        .unwrap()
}
fn play(g: &mut Game, card: usize) {
    let p = g.priority();
    g.players[p].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index: g.players[p].hand.len() - 1,
        lane: None,
        targets: vec![],
    })
    .unwrap();
}
fn cast(g: &mut Game, card: usize, targets: Vec<Target>) {
    let p = g.priority();
    g.players[p].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index: g.players[p].hand.len() - 1,
        lane: None,
        targets,
    })
    .unwrap();
}
fn settle(g: &mut Game) {
    for _ in 0..100 {
        let a = match g.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::TriggerTargets { .. }) => g
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|a| matches!(a,Action::ChooseTriggerTargets{targets}if !targets.is_empty()))
                .unwrap_or(Action::ChooseTriggerTargets { targets: vec![] }),
            Some(Decision::Discard { player, count, .. }) => Action::SelectCards {
                indices: (0..usize::min(count as usize, g.players[player].hand.len())).collect(),
            },
            Some(Decision::TokenPlacement { .. }) => Action::PlaceToken { lane: None },
            Some(
                Decision::Predict { .. } | Decision::Legend { .. } | Decision::Battlefield { .. },
            ) => Action::Choose { accept: false },
            Some(other) => panic!("unhandled {other:?}"),
            None if !g.chain.is_empty() => Action::Pass,
            None => return,
        };
        g.apply(a).unwrap();
    }
    panic!("trigger sequence did not settle")
}
fn conquer(g: &mut Game, id: u32) {
    g.apply(Action::Move {
        unit_id: id,
        lane: Some(0),
    })
    .unwrap();
    settle(g);
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
    settle(g);
}
fn gold(g: &Game, p: usize) -> usize {
    g.players[p].gear.iter().filter(|x| x.token).count()
}

#[test]
fn on_play_xp_and_self_might_are_real_queued_effects() {
    for (card, xp) in [(152, 1), (154, 2), (155, 2)] {
        let mut g = game();
        unit(&mut g, 0, 100, 72, None, true);
        play(&mut g, card);
        assert_eq!(g.players[0].xp, 0);
        settle(&mut g);
        assert_eq!(g.players[0].xp, xp, "card{card}");
    }
    let mut g = game();
    play(&mut g, 185);
    settle(&mut g);
    assert_eq!(g.players[0].units[0].temporary_buff, 3);
}
#[test]
fn played_discard_and_opponent_draw_respect_actual_effects() {
    let mut g = game();
    g.players[0].hand = vec![69];
    play(&mut g, 172);
    settle(&mut g);
    assert!(g.players[0].hand.is_empty());
    assert_eq!(g.players[0].discard, vec![69]);
    let mut g = game();
    play(&mut g, 186);
    settle(&mut g);
    assert_eq!([g.players[0].hand.len(), g.players[1].hand.len()], [1, 1]);
    let mut g = game();
    play(&mut g, 193);
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
    assert!(matches!(g.decision, Some(Decision::Predict { .. })));
}
#[test]
fn gear_play_and_ready_observers_preserve_actor_and_subject() {
    let mut g = game();
    let pit = unit(&mut g, 0, 100, 156, None, false);
    gear(&mut g, 0, 102, 196, None);
    play(&mut g, 42);
    settle(&mut g);
    assert!(u(&g, pit).ready);
    assert_eq!(u(&g, pit).temporary_buff, 1);
    let mut g = game();
    let cat = unit(&mut g, 1, 100, 157, None, false);
    gear(&mut g, 1, 101, 196, None);
    play(&mut g, 23);
    g.apply(Action::ChooseTarget { target: Some(cat) }).unwrap();
    settle(&mut g);
    assert!(u(&g, cat).ready);
    assert_eq!(
        u(&g, cat).temporary_buff,
        2,
        "enemy Ready triggers becomes-ready, but not opponent's when-you-ready gear"
    );
}
#[test]
fn buff_is_a_transition_and_simian_ready_is_respondable() {
    let mut g = game();
    let simian = unit(&mut g, 0, 100, 158, None, false);
    play(&mut g, 27);
    g.apply(Action::ChooseTarget {
        target: Some(simian),
    })
    .unwrap();
    settle(&mut g);
    assert!(u(&g, simian).ready);
    assert_eq!(u(&g, simian).buff, 1);
    g.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
    settle(&mut g);
    play(&mut g, 27);
    g.apply(Action::ChooseTarget {
        target: Some(simian),
    })
    .unwrap();
    settle(&mut g);
    assert!(
        !u(&g, simian).ready,
        "an existing buff does not become a new buff"
    );
}
#[test]
fn friendly_unit_observers_and_second_completed_card_are_distinct() {
    let mut g = game();
    let leader = unit(&mut g, 0, 100, 170, None, true);
    let cithria = unit(&mut g, 0, 101, 174, None, true);
    let darius = unit(&mut g, 0, 102, 183, None, false);
    play(&mut g, 69);
    settle(&mut g);
    assert_eq!(u(&g, leader).temporary_buff, 2);
    assert_eq!(u(&g, cithria).buff, 1);
    assert!(!u(&g, darius).ready);
    play(&mut g, 70);
    settle(&mut g);
    assert_eq!(u(&g, leader).temporary_buff, 4);
    assert_eq!(u(&g, cithria).buff, 1);
    assert!(u(&g, darius).ready);
    assert_eq!(u(&g, darius).temporary_buff, 2);
}
#[test]
fn spell_observers_use_printed_cost_and_successful_completion() {
    let mut g = game();
    let diana = unit(&mut g, 0, 100, 173, None, true);
    let lux = unit(&mut g, 0, 101, 182, None, true);
    unit(&mut g, 0, 102, 181, None, true);
    let guardian = unit(&mut g, 0, 103, 184, None, false);
    cast(&mut g, 77, vec![]);
    settle(&mut g);
    assert_eq!(u(&g, diana).temporary_buff, 2);
    assert_eq!(u(&g, lux).temporary_buff, 3);
    assert_eq!(g.players[0].hand.len(), 4);
    assert!(!u(&g, guardian).ready);
    play(&mut g, 69);
    settle(&mut g);
    assert!(u(&g, guardian).ready);
    cast(&mut g, 61, vec![]);
    settle(&mut g);
    assert_eq!(
        g.players[0].hand.len(),
        5,
        "Explorer sees Singularity printed3power"
    );
}
#[test]
fn hidden_and_opponent_turn_card_observers_trigger_after_play() {
    let mut g = game();
    unit(&mut g, 0, 100, 163, Some(0), true);
    let monk = unit(&mut g, 0, 101, 164, None, true);
    g.battlefield_owner[0] = Some(0);
    g.players[0].hidden.push(HiddenCard {
        id: 500,
        card_id: 9,
        lane: 0,
        hidden_turn: 0,
        revealed: false,
    });
    g.apply(Action::PlayHidden {
        hidden_id: 500,
        targets: vec![],
    })
    .unwrap();
    settle(&mut g);
    assert_eq!(gold(&g, 0), 1);
    assert_eq!(u(&g, monk).temporary_buff, 2);
    let mut g = game();
    unit(&mut g, 1, 100, 175, None, true);
    let t = unit(&mut g, 0, 101, 72, None, true);
    cast(&mut g, 7, vec![t]);
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 7, vec![t]);
    settle(&mut g);
    assert_eq!(g.players[1].units.iter().filter(|u| u.token).count(), 1);
}
#[test]
fn moving_source_and_attached_gear_generate_live_here_tokens() {
    for (card, count) in [(179, 1), (180, 3)] {
        let mut g = game();
        unit(&mut g, 0, 100, card, None, true);
        g.apply(Action::Move {
            unit_id: 100,
            lane: Some(0),
        })
        .unwrap();
        settle(&mut g);
        let tokens: Vec<_> = g.players[0].units.iter().filter(|u| u.token).collect();
        assert_eq!(tokens.len(), count);
        assert!(tokens.iter().all(|u| u.lane == Some(0)));
    }
    let mut g = game();
    unit(&mut g, 0, 100, 162, None, true);
    g.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    settle(&mut g);
    assert_eq!(gold(&g, 0), 1);
    let mut g = game();
    let squad = unit(&mut g, 0, 100, 169, Some(0), true);
    g.battlefield_owner[0] = Some(0);
    g.apply(Action::Move {
        unit_id: 100,
        lane: None,
    })
    .unwrap();
    settle(&mut g);
    assert_eq!(u(&g, squad).temporary_buff, 2);
    let mut g = game();
    let bearer = unit(&mut g, 0, 100, 72, None, true);
    gear(&mut g, 0, 101, 203, Some(100));
    gear(&mut g, 0, 102, 205, Some(100));
    g.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    settle(&mut g);
    assert_eq!(u(&g, bearer).temporary_buff, 2);
    assert!(
        g.players[0]
            .units
            .iter()
            .any(|u| u.token && u.lane == Some(0))
    );
}
#[test]
fn exhaustion_predict_waits_for_a_chain_response() {
    let mut g = game();
    unit(&mut g, 0, 100, 194, None, true);
    g.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    assert!(g.decision.is_none());
    assert!(!g.chain.is_empty());
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
    assert!(matches!(g.decision, Some(Decision::Predict { .. })));
}
#[test]
fn source_attack_and_defense_target_before_responses() {
    for (card, damage, negative) in [(167, 1, 0), (168, 0, -2)] {
        for source_player in [0, 1] {
            let mut g = game();
            let a = unit(
                &mut g,
                0,
                100,
                if source_player == 0 { card } else { 69 },
                None,
                true,
            );
            let b = unit(
                &mut g,
                1,
                101,
                if source_player == 1 { card } else { 69 },
                Some(0),
                true,
            );
            g.battlefield_owner[0] = Some(1);
            g.apply(Action::Move {
                unit_id: 100,
                lane: Some(0),
            })
            .unwrap();
            if card == 167 && source_player == 1 {
                assert!(g.chain.is_empty());
                continue;
            }
            assert!(matches!(g.decision, Some(Decision::TriggerTargets { .. })));
            g.apply(Action::ChooseTriggerTargets {
                targets: vec![if source_player == 0 { b } else { a }],
            })
            .unwrap();
            settle(&mut g);
            let t = if source_player == 0 { b } else { a };
            assert_eq!(u(&g, t).damage, damage);
            assert_eq!(u(&g, t).temporary_buff, negative);
        }
    }
    let mut g = game();
    unit(&mut g, 0, 100, 166, None, true);
    let friend = unit(&mut g, 0, 101, 72, None, false);
    unit(&mut g, 1, 102, 72, Some(0), true);
    g.battlefield_owner[0] = Some(1);
    g.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![friend],
    })
    .unwrap();
    settle(&mut g);
    assert!(u(&g, friend).ready);
}
#[test]
fn attached_combat_and_standalone_alone_observers() {
    let mut g = game();
    let bearer = unit(&mut g, 0, 100, 72, None, true);
    let enemy = unit(&mut g, 1, 101, 72, Some(0), true);
    gear(&mut g, 0, 102, 197, None);
    gear(&mut g, 0, 103, 206, Some(100));
    g.battlefield_owner[0] = Some(1);
    g.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    settle(&mut g);
    assert_eq!(u(&g, bearer).temporary_buff, 1);
    assert_eq!(u(&g, enemy).damage, 2);
}
#[test]
fn on_play_targets_enforce_mech_another_and_optional_decline() {
    let mut g = game();
    let mech = unit(&mut g, 0, 100, 54, None, false);
    let other = unit(&mut g, 0, 101, 72, None, false);
    play(&mut g, 165);
    assert!(!g.valid_trigger_target(other, &[]).unwrap());
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![mech],
    })
    .unwrap();
    settle(&mut g);
    assert!(u(&g, mech).ready);
    let mut g = game();
    unit(&mut g, 1, 100, 72, Some(0), true);
    g.battlefield_owner[0] = Some(1);
    play(&mut g, 188);
    assert_eq!(g.trigger_target_bounds(), Some((1, 0)));
    g.apply(Action::ChooseTriggerTargets { targets: vec![] })
        .unwrap();
    assert!(g.players[1].units.iter().any(|u| u.id == 100));
    assert!(g.chain.is_empty());
    let mut g = game();
    let enemy = unit(&mut g, 1, 100, 72, Some(0), true);
    g.battlefield_owner[0] = Some(1);
    play(&mut g, 189);
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![enemy],
    })
    .unwrap();
    settle(&mut g);
    assert_eq!(g.players[1].hand, vec![72]);
}
#[test]
fn two_target_bounce_is_chosen_after_entry_and_revalidated_independently() {
    let mut g = game();
    let friend = unit(&mut g, 0, 100, 72, None, true);
    let enemy = unit(&mut g, 1, 101, 70, None, true);
    play(&mut g, 190);
    assert_eq!(g.trigger_target_bounds(), Some((2, 2)));
    let before = g.players.clone();
    assert!(
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![friend]
        })
        .is_err()
    );
    assert_eq!(g.players, before);
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![friend, enemy],
    })
    .unwrap();
    settle(&mut g);
    assert_eq!(g.players[0].hand, vec![72]);
    assert_eq!(g.players[1].hand, vec![70]);
    assert_eq!(g.players[0].units.len(), 1);
}
#[test]
fn implicit_source_fights_and_stun_does_not_invent_vex_movement_lock() {
    let mut g = game();
    let enemy = unit(&mut g, 1, 100, 72, Some(0), true);
    g.battlefield_owner[0] = Some(1);
    play(&mut g, 191);
    let source = g.players[0].units[0].id;
    let might = g.unit_might(&g.players[0].units[0]).unwrap();
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![enemy],
    })
    .unwrap();
    settle(&mut g);
    assert!(g.players[1].units.is_empty() || u(&g, enemy).damage == might);
    assert!(
        g.players[0]
            .units
            .iter()
            .find(|u| u.id == source)
            .is_some_and(|u| u.damage > 0)
    );
    let mut g = game();
    let friend = unit(&mut g, 0, 100, 72, None, true);
    play(&mut g, 192);
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![friend],
    })
    .unwrap();
    settle(&mut g);
    assert!(u(&g, friend).stunned);
    g.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
}
#[test]
fn opponent_bird_and_here_sand_soldier_preserve_owner_and_characteristics() {
    let mut g = game();
    play(&mut g, 187);
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
    assert!(matches!(
        g.decision,
        Some(Decision::TokenPlacement { player: 1, .. })
    ));
    g.apply(Action::PlaceToken { lane: None }).unwrap();
    let bird = g.players[1].units.iter().find(|u| u.token).unwrap();
    assert_eq!(g.unit_might(bird).unwrap(), 1);
    assert_eq!(g.unit_keywords(bird).unwrap().deflect, 1);
    let mut g = game();
    play(&mut g, 178);
    settle(&mut g);
    let soldier = g.players[0].units.iter().find(|u| u.token).unwrap();
    assert_eq!(g.unit_might(soldier).unwrap(), 2);
    assert_eq!(soldier.lane, None);
}
#[test]
fn second_draw_counts_individual_cards_and_resets_next_turn() {
    let mut g = game();
    gear(&mut g, 0, 100, 195, None);
    let friend = unit(&mut g, 0, 101, 72, None, true);
    cast(&mut g, 78, vec![]);
    settle(&mut g);
    assert_eq!(g.players[0].hand.len(), 3);
    assert_eq!(u(&g, friend).temporary_buff, 2);
    cast(&mut g, 78, vec![]);
    settle(&mut g);
    assert_eq!(u(&g, friend).temporary_buff, 2);
    g.apply(Action::EndTurn).unwrap();
    assert_eq!(g.trigger_state.draws[0], 0);
}
#[test]
fn gear_resource_and_equip_activations_trigger_prize_once_each() {
    let mut g = game();
    let prize = unit(&mut g, 0, 100, 215, None, true);
    gear(&mut g, 0, 101, 42, None);
    gear(&mut g, 0, 102, 3, None);
    g.apply(Action::ActivateGear { gear_id: 101 }).unwrap();
    settle(&mut g);
    assert_eq!(u(&g, prize).temporary_buff, 1);
    g.apply(Action::Equip {
        gear_id: 102,
        target: prize,
    })
    .unwrap();
    settle(&mut g);
    assert_eq!(u(&g, prize).temporary_buff, 2);
}
#[test]
fn hold_effects_cover_xp_gold_points_and_trigger_time_conditions() {
    for (card, xp, coins, points, draw) in [
        (153, 3, 0, 0, 0),
        (154, 1, 0, 0, 0),
        (160, 0, 2, 0, 0),
        (208, 0, 0, 1, 0),
        (209, 0, 0, 1, 0),
        (210, 0, 0, 0, 1),
    ] {
        let mut g = game();
        unit(&mut g, 1, 100, card, Some(0), true);
        if card == 209 || card == 210 {
            unit(&mut g, 1, 101, 72, Some(0), true);
        }
        g.battlefield_owner[0] = Some(1);
        g.apply(Action::EndTurn).unwrap();
        settle(&mut g);
        assert_eq!(g.players[1].xp, xp, "{card}");
        assert_eq!(gold(&g, 1), coins, "{card}");
        assert_eq!(g.players[1].score, 1 + points, "{card}");
        assert_eq!(g.players[1].hand.len(), 1 + draw, "{card}");
    }
    for (card, coins, points) in [(202, 2, 0), (207, 0, 1)] {
        let mut g = game();
        unit(&mut g, 1, 100, 72, Some(0), true);
        gear(&mut g, 1, 101, card, Some(100));
        g.battlefield_owner[0] = Some(1);
        g.apply(Action::EndTurn).unwrap();
        settle(&mut g);
        assert_eq!(gold(&g, 1), coins);
        assert_eq!(g.players[1].score, 1 + points);
    }
    let mut g = game();
    let source = unit(&mut g, 1, 100, 211, Some(0), true);
    let friend = unit(&mut g, 1, 101, 72, Some(0), true);
    g.battlefield_owner[0] = Some(1);
    g.apply(Action::EndTurn).unwrap();
    settle(&mut g);
    assert_eq!(u(&g, source).buff, 1);
    assert_eq!(u(&g, friend).buff, 1);
}
#[test]
fn conquest_effects_cover_gold_loot_and_bearer_buff() {
    let mut g = game();
    unit(&mut g, 0, 100, 161, None, true);
    conquer(&mut g, 100);
    assert_eq!(gold(&g, 0), 1);
    for (card, coins, buff) in [(200, 0, 0), (201, 1, 0), (204, 0, 1)] {
        let mut g = game();
        let bearer = unit(&mut g, 0, 100, 72, None, true);
        gear(&mut g, 0, 101, card, Some(100));
        g.players[0].hand = vec![70];
        conquer(&mut g, 100);
        assert_eq!(gold(&g, 0), coins, "{card}");
        assert_eq!(u(&g, bearer).buff, buff, "{card}");
        if card == 200 {
            assert_eq!(g.players[0].discard, vec![70]);
            assert_eq!(g.players[0].hand, vec![69]);
        }
    }
}
fn kill(g: &mut Game, targets: Vec<Target>) {
    cast(g, 61, targets);
    settle(g);
}
#[test]
fn surviving_death_observers_and_deathknell_loot_are_distinct() {
    for (card, xp, token_count, might, draw) in [
        (159, 1, 0, 0, 0),
        (176, 0, 1, 0, 0),
        (212, 0, 0, 2, 0),
        (214, 0, 0, 0, 1),
    ] {
        let mut g = game();
        let observer = unit(&mut g, 0, 100, card, None, true);
        let victim = unit(&mut g, 0, 101, 72, None, true);
        kill(&mut g, vec![victim]);
        assert_eq!(g.players[0].xp, xp, "{card}");
        assert_eq!(
            g.players[0].units.iter().filter(|u| u.token).count(),
            token_count,
            "{card}"
        );
        assert_eq!(u(&g, observer).temporary_buff, might, "{card}");
        assert_eq!(g.players[0].hand.len(), draw, "{card}");
    }
    let mut g = game();
    let victim = unit(&mut g, 0, 100, 171, None, true);
    g.players[0].hand = vec![70, 71];
    kill(&mut g, vec![victim]);
    assert_eq!(g.players[0].hand, vec![69, 69]);
    assert!(g.players[0].discard.contains(&70) && g.players[0].discard.contains(&71));
    let mut g = game();
    let victim = unit(&mut g, 0, 100, 177, None, true);
    kill(&mut g, vec![victim]);
    assert_eq!(g.players[0].units.iter().filter(|u| u.token).count(), 3);
}
#[test]
fn captured_buff_and_stun_facts_survive_the_death_event() {
    let mut g = game();
    gear(&mut g, 0, 100, 198, None);
    let victim = unit(&mut g, 0, 101, 72, None, true);
    let friend = unit(&mut g, 0, 102, 70, None, true);
    g.buffed_units.push(101);
    g.players[0].units[0].buff = 1;
    kill(&mut g, vec![victim]);
    assert_eq!(u(&g, friend).buff, 1);
    let mut g = game();
    unit(&mut g, 0, 100, 213, Some(0), true);
    let enemy = unit(&mut g, 1, 101, 72, Some(0), true);
    g.players[1].units[0].stunned = true;
    g.battlefield_owner[0] = Some(0);
    kill(&mut g, vec![enemy]);
    assert_eq!(g.players[0].hand.len(), 1);
    let mut g = game();
    let victim = unit(&mut g, 0, 100, 72, None, true);
    gear(&mut g, 0, 101, 199, Some(100));
    kill(&mut g, vec![victim]);
    assert_eq!(g.players[0].hand.len(), 1);
    assert_eq!(g.players[0].gear[0].attached_to, None);
}
#[test]
fn simultaneous_death_observers_must_survive_and_first_time_is_per_source() {
    let mut g = game();
    unit(&mut g, 0, 100, 214, None, true);
    let a = unit(&mut g, 0, 101, 72, None, true);
    let b = unit(&mut g, 0, 102, 73, None, true);
    kill(&mut g, vec![a, b]);
    assert_eq!(g.players[0].hand.len(), 1);
    let c = unit(&mut g, 0, 103, 72, None, true);
    kill(&mut g, vec![c]);
    assert_eq!(g.players[0].hand.len(), 1);
    let mut g = game();
    let watcher = unit(&mut g, 0, 100, 214, None, true);
    let victim = unit(&mut g, 0, 101, 72, None, true);
    kill(&mut g, vec![watcher, victim]);
    assert!(g.players[0].hand.is_empty());
}

#[test]
fn hunt_conquest_and_defending_equipment_use_their_named_events() {
    for (card, xp) in [(153, 3), (154, 1)] {
        let mut g = game();
        unit(&mut g, 0, 100, card, None, true);
        conquer(&mut g, 100);
        assert_eq!(g.players[0].xp, xp);
    }
    let mut g = game();
    let attacker = unit(&mut g, 0, 100, 70, None, true);
    let defender = unit(&mut g, 1, 101, 70, Some(0), true);
    gear(&mut g, 1, 102, 197, None);
    gear(&mut g, 1, 103, 206, Some(101));
    g.battlefield_owner[0] = Some(1);
    g.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    settle(&mut g);
    assert_eq!(u(&g, defender).temporary_buff, 1);
    assert_eq!(u(&g, attacker).damage, 2);
}
#[test]
fn hidden_optional_target_rechecks_source_existence_before_bouncing() {
    let mut g = game();
    let friend = unit(&mut g, 0, 100, 72, Some(0), true);
    let outside = unit(&mut g, 0, 101, 72, Some(1), true);
    g.battlefield_owner = [Some(0), Some(0)];
    g.players[0].hidden.push(HiddenCard {
        id: 500,
        card_id: 188,
        lane: 0,
        hidden_turn: 0,
        revealed: false,
    });
    g.apply(Action::PlayHidden {
        hidden_id: 500,
        targets: vec![],
    })
    .unwrap();
    assert!(!g.valid_trigger_target(outside, &[]).unwrap());
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![friend],
    })
    .unwrap();
    let source = g.players[0]
        .units
        .iter()
        .find(|u| u.card_id == 188)
        .unwrap()
        .id;
    cast(
        &mut g,
        11,
        vec![Target::Unit {
            player: 0,
            unit_id: source,
        }],
    );
    settle(&mut g);
    assert!(g.players[0].units.iter().any(|u| u.id == 100));
    assert_eq!(g.players[0].hand, vec![188]);
    let mut g = game();
    let friend = unit(&mut g, 0, 100, 72, Some(0), true);
    g.battlefield_owner[0] = Some(0);
    play(&mut g, 188);
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![friend],
    })
    .unwrap();
    settle(&mut g);
    assert_eq!(g.players[0].hand, vec![72]);
}
#[test]
fn here_uses_current_source_location_and_has_no_location_after_source_leaves() {
    for remove in [false, true] {
        let mut g = game();
        let other = unit(&mut g, 1, 100, 72, None, true);
        play(&mut g, 178);
        let source = Target::Unit {
            player: 0,
            unit_id: g.players[0].units[0].id,
        };
        if remove {
            cast(&mut g, 16, vec![source, other]);
        } else {
            g.players[0].units[0].lane = Some(0);
            g.battlefield_owner[0] = Some(0);
            cast(&mut g, 9, vec![source]);
        }
        settle(&mut g);
        let tokens: Vec<_> = g.players[0].units.iter().filter(|u| u.token).collect();
        assert_eq!(tokens.len(), usize::from(!remove));
        assert!(tokens.iter().all(|u| u.lane.is_none()));
    }
    let mut g = game();
    let bearer = unit(&mut g, 0, 100, 72, None, true);
    let enemy = unit(&mut g, 1, 101, 72, None, true);
    gear(&mut g, 0, 102, 203, Some(100));
    g.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    cast(&mut g, 16, vec![bearer, enemy]);
    settle(&mut g);
    assert!(g.players[0].units.iter().all(|u| !u.token));
    assert_eq!(g.players[0].gear[0].attached_to, None);
}
#[test]
fn shen_condition_is_not_rechecked_after_triggering() {
    let mut g = game();
    unit(&mut g, 1, 100, 210, Some(0), true);
    let companion = unit(&mut g, 1, 101, 72, Some(0), true);
    let enemy = unit(&mut g, 0, 102, 72, None, true);
    g.battlefield_owner[0] = Some(1);
    g.apply(Action::EndTurn).unwrap();
    cast(&mut g, 16, vec![companion, enemy]);
    settle(&mut g);
    assert_eq!(
        g.players[1].hand.len(),
        3,
        "returned companion, triggered draw, then turn draw"
    );
}

#[test]
fn burnout_during_burn_does_not_count_as_drawing_cards() {
    let mut g = game();
    gear(&mut g, 0, 100, 195, None);
    unit(&mut g, 0, 101, 72, None, true);
    g.players[0].deck.clear();
    g.players[0].discard = vec![69, 70];
    let before = g.trigger_state.draws[0];
    play(&mut g, 33);
    settle(&mut g);
    assert_eq!(g.trigger_state.draws[0], before);
    assert_eq!(g.players[0].units[0].temporary_buff, 0);
    assert_eq!(g.players[1].score, 1);
    assert!(g.players[0].hand.is_empty());
    assert_eq!(g.players[0].discard.len(), 2);
}
#[test]
fn countered_cards_do_not_advance_completed_play_ordinals() {
    let mut g = game();
    let darius = unit(&mut g, 0, 100, 183, None, false);
    let friend = unit(&mut g, 0, 101, 72, None, true);
    cast(&mut g, 7, vec![friend]);
    let target = Target::Spell {
        chain_id: g.chain.last().unwrap().id,
    };
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 6, vec![target]);
    settle(&mut g);
    assert_eq!(g.trigger_state.completed_cards[0], 0);
    play(&mut g, 69);
    settle(&mut g);
    assert!(!u(&g, darius).ready);
    play(&mut g, 70);
    settle(&mut g);
    assert!(u(&g, darius).ready);
}
#[test]
fn trigger_deflect_cost_can_be_generated_and_failed_choices_are_atomic() {
    let mut g = game();
    let enemy = unit(&mut g, 1, 100, 0, Some(0), true);
    g.battlefield_owner[0] = Some(1);
    g.players[0].runes = [0; 6];
    g.players[0].energy_pool = 4;
    g.players[0].energy = 4;
    g.players[0].power_pool[4] = 2;
    gear(&mut g, 0, 101, 42, None);
    play(&mut g, 189);
    assert!(g.resource_payment_pending());
    let before = g.players.clone();
    assert!(
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![enemy]
        })
        .is_err()
    );
    assert_eq!(g.players, before);
    g.apply(Action::ActivateGear { gear_id: 101 }).unwrap();
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![enemy],
    })
    .unwrap();
    settle(&mut g);
    assert_eq!(g.players[1].hand, vec![0]);
}

#[test]
fn move_created_recruit_enters_before_attack_designations_and_alone_checks() {
    let mut g = game();
    let drummer = unit(&mut g, 0, 100, 179, None, true);
    unit(&mut g, 1, 101, 70, Some(0), true);
    gear(&mut g, 0, 102, 197, None);
    g.battlefield_owner[0] = Some(1);
    g.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    assert!(
        g.showdown.is_none(),
        "Move trigger chain precedes staged combat"
    );
    settle(&mut g);
    assert!(g.showdown.as_ref().is_some_and(|window| window.combat));
    assert_eq!(g.lane_count(0, 0), 2);
    assert_eq!(
        u(&g, drummer).temporary_buff,
        0,
        "neither attacker is alone after the Move token enters"
    );
    assert!(
        g.players[0]
            .units
            .iter()
            .filter(|u| u.token)
            .all(|u| u.temporary_buff == 0)
    );
}
