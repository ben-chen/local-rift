//! Consult the Heavens: CR359.3.e, 437, 814 and 317.2.c.
use super::*;
use test_support::{position, ready_unit, reject_unchanged};

const HEAVENS: CardId = 457;

fn target(g: &mut Game, owner: usize, card: CardId, lane: Option<usize>) -> Target {
    let unit_id = ready_unit(g, owner, card);
    let target = Target::Unit {
        player: owner,
        unit_id,
    };
    g.unit_mut(target).unwrap().lane = lane;
    target
}

fn cast(g: &mut Game, card: CardId, target: Target) {
    let player = g.priority();
    let hand_index = g.players[player]
        .hand
        .iter()
        .position(|&id| id == card)
        .unwrap();
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets: vec![target],
    })
    .unwrap();
}

fn settle(g: &mut Game) {
    for _ in 0..30 {
        if g.decision.is_some() || g.chain.is_empty() {
            return;
        }
        g.apply(Action::Pass).unwrap();
    }
    panic!("unsettled chain");
}

fn shield(g: &Game, target: Target) -> u64 {
    g.unit_keywords(g.unit_ref(target).unwrap()).unwrap().shield
}

#[test]
fn heavens_any_unit_stacks_shield_without_unconditional_might_and_predict_is_private_choice() {
    for actor in 0..2 {
        for owner in 0..2 {
            for defending in [false, true] {
                let mut g = position(actor, 3, 457001);
                let unit = target(&mut g, owner, 110, Some(0)); // Poro has Shield 1.
                let base_might = g.unit_might(g.unit_ref(unit).unwrap()).unwrap();
                if defending {
                    target(&mut g, 1 - owner, 71, Some(0));
                    g.showdown = Some(Showdown {
                        lane: 0,
                        attacker: 1 - owner,
                        focus: actor,
                        passes: 0,
                        combat: true,
                    });
                }
                g.players[actor].hand = vec![HEAVENS];
                g.players[actor].deck = vec![71, 8, 7];
                cast(&mut g, HEAVENS, unit);
                assert_eq!(shield(&g, unit), 1, "grant waits for resolution");
                assert_eq!(g.players[actor].energy, 19);
                assert_eq!(g.players[actor].power_pool, [5; 6]);
                settle(&mut g);
                assert_eq!(shield(&g, unit), 3);
                assert_eq!(
                    g.unit_might(g.unit_ref(unit).unwrap()).unwrap(),
                    base_might + if defending { 3 } else { 0 }
                );
                assert!(
                    matches!(g.decision, Some(Decision::Predict {player,card_id:7}) if player==actor)
                );
                assert!(!g.players[actor].discard.contains(&HEAVENS));
                reject_unchanged(&mut g, Action::Pass);
                let mut clone = g.clone();
                g.apply(Action::Choose { accept: true }).unwrap();
                clone.apply(Action::Choose { accept: true }).unwrap();
                assert_eq!(
                    format!("{g:?}"),
                    format!("{clone:?}"),
                    "cloning preserves suspended resolution and RNG"
                );
                assert_eq!(g.players[actor].deck, [7, 71, 8]);
                assert!(g.players[actor].hand.is_empty(), "Predict does not draw");
                assert_eq!(g.players[actor].discard, [HEAVENS]);
            }
        }
    }
}

#[test]
fn heavens_expiration_removes_only_granted_shield_and_rejects_missing_or_wrong_targets() {
    for actor in 0..2 {
        let mut g = position(actor, 3, 457002);
        let unit = target(&mut g, actor, 110, None);
        g.players[actor].hand = vec![HEAVENS, HEAVENS];
        for targets in [
            vec![],
            vec![Target::Legend {
                player: actor,
                legend_id: 3,
            }],
            vec![Target::Unit {
                player: actor,
                unit_id: u32::MAX,
            }],
        ] {
            reject_unchanged(
                &mut g,
                Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets,
                },
            );
        }
        for _ in 0..2 {
            cast(&mut g, HEAVENS, unit);
            settle(&mut g);
            g.apply(Action::Choose { accept: false }).unwrap();
        }
        assert_eq!(shield(&g, unit), 5);
        g.apply(Action::EndTurn).unwrap();
        assert_eq!(shield(&g, unit), 1);
        assert!(g.turn_keyword_grants.is_empty());
    }
}

#[test]
fn heavens_lost_target_still_predicts_and_departure_clears_the_physical_grant() {
    for actor in 0..2 {
        for granted_before_return in [false, true] {
            let mut g = position(actor, 3, 457003);
            let unit = target(&mut g, actor, 110, Some(0));
            g.battlefield_owner[0] = Some(actor);
            g.players[actor].hand = vec![HEAVENS, 11];
            cast(&mut g, HEAVENS, unit);
            if granted_before_return {
                settle(&mut g);
                g.apply(Action::Choose { accept: false }).unwrap();
                assert_eq!(shield(&g, unit), 3);
            }
            cast(&mut g, 11, unit);
            settle(&mut g);
            assert!(g.unit_ref(unit).is_none());
            assert!(g.turn_keyword_grants.is_empty());
            if !granted_before_return {
                assert!(matches!(g.decision,Some(Decision::Predict{player,..}) if player==actor));
                g.apply(Action::Choose { accept: false }).unwrap();
            }
            assert!(g.players[actor].hand.contains(&110));
            assert!(g.players[actor].discard.contains(&HEAVENS));
        }
    }
}

#[test]
fn heavens_counter_prevents_both_instructions_and_empty_deck_does_not_burn_out() {
    for actor in 0..2 {
        for counter in [false, true] {
            let mut g = position(actor, 3, 457004);
            let unit = target(&mut g, actor, 110, None);
            g.players[actor].hand = vec![HEAVENS];
            g.players[actor].deck.clear();
            g.players[1 - actor].hand = vec![6];
            cast(&mut g, HEAVENS, unit);
            if counter {
                let id = g.chain[0].id;
                g.apply(Action::Pass).unwrap();
                cast(&mut g, 6, Target::Spell { chain_id: id });
            }
            settle(&mut g);
            assert!(g.decision.is_none());
            assert!(g.chain.is_empty());
            assert_eq!(shield(&g, unit), if counter { 1 } else { 3 });
            assert_eq!(
                g.players[1 - actor].score,
                0,
                "empty Predict never burns out"
            );
        }
    }
}

#[test]
fn heavens_syndra_repeat_gives_separate_shield_and_predict_instructions() {
    for actor in 0..2 {
        let mut g = position(actor, 3, 457005);
        target(&mut g, actor, 266, Some(0));
        target(&mut g, 1 - actor, 71, Some(0));
        let unit = target(&mut g, actor, 110, Some(0));
        g.showdown = Some(Showdown {
            lane: 0,
            attacker: 1 - actor,
            focus: actor,
            passes: 0,
            combat: true,
        });
        g.players[actor].hand = vec![HEAVENS];
        g.players[actor].deck = vec![71, 8, 7];
        let action = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![unit],
        };
        let options = g.play_choices(&action).unwrap().repeats;
        assert_eq!(options.len(), 1);
        g.apply(Action::WithPlayChoices {
            action: Box::new(action),
            choices: PlayChoices {
                repeats: vec![RepeatSelection {
                    option_id: options[0].id,
                    choices: ExecutionChoices {
                        targets: vec![unit],
                        ..Default::default()
                    },
                }],
                ..Default::default()
            },
        })
        .unwrap();
        settle(&mut g);
        assert_eq!(shield(&g, unit), 3);
        assert!(matches!(
            g.decision,
            Some(Decision::Predict { card_id: 7, .. })
        ));
        g.apply(Action::Choose { accept: true }).unwrap();
        assert_eq!(shield(&g, unit), 5);
        assert!(matches!(
            g.decision,
            Some(Decision::Predict { card_id: 8, .. })
        ));
        g.apply(Action::Choose { accept: false }).unwrap();
        assert_eq!(
            g.players[actor]
                .discard
                .iter()
                .filter(|&&id| id == HEAVENS)
                .count(),
            1
        );
        assert_eq!(g.players[actor].deck, [7, 71, 8]);
    }
}

#[test]
fn heavens_chooses_for_irelia_before_responses_but_predict_does_not_choose_again() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 457006);
        let unit = target(&mut g, actor, 110, None);
        g.unit_mut(unit).unwrap().ready = false;
        g.legend[actor].ready = true;
        g.players[actor].hand = vec![HEAVENS];
        cast(&mut g, HEAVENS, unit);
        assert!(matches!(g.decision, Some(Decision::Legend { .. })));
        g.apply(Action::Choose { accept: true }).unwrap();
        assert!(!g.legend[actor].ready);
        assert!(
            !g.unit_ref(unit).unwrap().ready,
            "ready effect waits for responses"
        );
        settle(&mut g);
        assert!(g.unit_ref(unit).unwrap().ready);
        assert_eq!(shield(&g, unit), 3);
        assert!(matches!(g.decision, Some(Decision::Predict { .. })));
        g.apply(Action::Choose { accept: false }).unwrap();
        assert!(g.decision.is_none() && g.chain.is_empty() && g.pending_triggers.is_empty());
    }
}

#[test]
fn heavens_ai_prefers_its_defender_and_ignores_opponents_private_cards() {
    for actor in 0..2 {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let mut g = position(actor, 3, 457007);
            g.difficulty = difficulty;
            let unit = target(&mut g, actor, 110, Some(0));
            target(&mut g, actor, 104, None);
            target(&mut g, 1 - actor, 71, Some(0));
            g.players[actor].hand = vec![HEAVENS];
            g.players[1 - actor].hand = vec![3, 7, 8];
            g.players[1 - actor].deck = vec![71, 7, 8];
            g.showdown = Some(Showdown {
                lane: 0,
                attacker: 1 - actor,
                focus: actor,
                passes: 0,
                combat: true,
            });
            assert_eq!(
                g.prepare_targets(actor, HEAVENS, None, &[]).unwrap(),
                [unit]
            );
            let action = g.choose_ai_action().unwrap();
            let mut hidden = g.clone();
            hidden.players[1 - actor].hand = vec![6, 457, 80];
            hidden.players[1 - actor].deck.reverse();
            assert_eq!(
                format!("{action:?}"),
                format!("{:?}", hidden.choose_ai_action().unwrap())
            );
            g.apply(action).unwrap();
            if difficulty == Difficulty::Tactical {
                assert!(
                    g.chain
                        .iter()
                        .any(|item| item.card_id == HEAVENS && item.targets == [unit])
                );
            }
        }
    }
}

#[test]
fn heavens_defensive_benchmark_matches_public_combat_outcomes() {
    for actor in 0..2 {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            for attack_might in [2, 3, 4, 5, 6] {
                let mut g = position(1 - actor, 0, 457008);
                g.difficulty = difficulty;
                let defender = target(&mut g, actor, 110, Some(0));
                let attacker = target(&mut g, 1 - actor, 71, None);
                g.unit_mut(attacker).unwrap().temporary_buff = attack_might - 5;
                g.battlefield_owner[0] = Some(actor);
                g.players[actor].hand = vec![HEAVENS];
                g.players[actor].deck = vec![71; 3];
                for p in &mut g.players {
                    p.energy_pool = 1;
                    p.energy = 1;
                    p.power_pool = [0; 6];
                }
                let Target::Unit { unit_id, .. } = attacker else {
                    unreachable!()
                };
                g.apply(Action::Move {
                    unit_id,
                    lane: Some(0),
                })
                .unwrap();
                g.apply(Action::Pass).unwrap();
                assert_eq!(g.priority(), actor);
                let chosen = g.choose_ai_action().unwrap();
                let run = |action: Action| {
                    let mut next = g.clone();
                    next.apply(action).unwrap();
                    for _ in 0..50 {
                        if !next.in_window() {
                            break;
                        }
                        let action = if next.decision.is_some() {
                            next.choose_ai_action().unwrap()
                        } else {
                            Action::Pass
                        };
                        next.apply(action).unwrap();
                    }
                    assert!(!next.in_window());
                    (
                        next.unit_ref(defender).is_some(),
                        next.unit_ref(attacker).is_some(),
                        next.battlefield_owner[0],
                    )
                };
                let pass = run(Action::Pass);
                let cast = run(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![defender],
                });
                let actual = run(chosen.clone());
                // Independent combat oracle: Poro defends at 3 Might, or 5 with
                // Consult. Damage is simultaneous; surviving units retain control.
                let expected_pass = match attack_might {
                    2 => (true, false, Some(actor)),
                    3 => (false, false, None),
                    _ => (false, true, Some(1 - actor)),
                };
                let expected_cast = match attack_might {
                    2..=4 => (true, false, Some(actor)),
                    5 => (false, false, None),
                    _ => (false, true, Some(1 - actor)),
                };
                assert_eq!(pass, expected_pass);
                assert_eq!(cast, expected_cast);
                let useful = matches!(attack_might, 3..=5);
                let decision_correct = (chosen != Action::Pass) == useful
                    && actual == if useful { expected_cast } else { expected_pass };
                if difficulty == Difficulty::Strategic {
                    assert!(
                        decision_correct,
                        "seat {actor}, attack {attack_might}: {chosen:?}"
                    );
                }
                println!(
                    "DEFENSE_BENCHMARK {}",
                    serde_json::json!({
                        "seat":actor,"difficulty":format!("{difficulty:?}"),"attack_might":attack_might,
                        "action":crate::api::action_to_json(&chosen),"useful_cast":useful,
                        "decision_correct":decision_correct,"pass":pass,"cast":cast,"actual":actual
                    })
                );
            }
        }
    }
}
