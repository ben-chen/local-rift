use super::*;
use test_support::{position, ready_unit, reject_unchanged, settle_unopposed_chain};

fn play(targets: Vec<Target>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets,
    }
}

fn ki(target: Target, cost: Option<u32>) -> Action {
    Action::WithPlayChoices {
        action: Box::new(play(vec![target])),
        choices: PlayChoices {
            exhaust_unit: cost,
            ..Default::default()
        },
    }
}

fn party_choice(game: &mut Game) {
    for _ in 0..20 {
        if matches!(game.decision, Some(Decision::SharedResources { .. })) {
            return;
        }
        game.apply(Action::Pass).unwrap();
    }
    panic!("Party Favors never offered the opponent its choice");
}

#[test]
fn ki_burst_cost_is_mandatory_and_separate_from_its_any_unit_target() {
    for actor in 0..2 {
        for owner in 0..2 {
            for lane in [None, Some(0)] {
                let mut game = position(actor, 0, 491101);
                let cost = ready_unit(&mut game, actor, 71);
                let unit_id = ready_unit(&mut game, owner, 71);
                let target = Target::Unit {
                    player: owner,
                    unit_id,
                };
                game.unit_mut(target).unwrap().lane = lane;
                game.players[actor].hand = vec![491];
                reject_unchanged(&mut game, ki(target, None));
                reject_unchanged(&mut game, ki(target, Some(u32::MAX)));
                game.players[actor].units[0].ready = false;
                reject_unchanged(&mut game, ki(target, Some(cost)));
                game.players[actor].units[0].ready = true;
                let before = format!("{game:?}");
                let view = game.play_choices(&ki(target, Some(cost))).unwrap();
                assert!(!view.unit_cost.as_ref().unwrap().optional);
                assert_eq!(format!("{game:?}"), before);
                let power = game.players[actor].power_pool;
                game.apply(ki(target, Some(cost))).unwrap();
                assert_eq!(game.players[actor].energy, 19);
                assert_eq!(game.players[actor].power_pool, power);
                assert!(!game.players[actor].units[0].ready);
                assert!(
                    !game.unit_ref(target).unwrap().stunned,
                    "wait for responses"
                );
                settle_unopposed_chain(&mut game);
                assert!(game.unit_ref(target).unwrap().stunned);
                assert!(
                    game.unit_ref(target).unwrap().ready,
                    "Stun does not exhaust"
                );
            }
        }
    }
}

#[test]
fn ki_burst_can_stun_its_cost_unit_and_ai_can_find_the_required_cost() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 491102);
        let unit_id = ready_unit(&mut game, actor, 71);
        game.players[actor].hand = vec![491];
        let target = Target::Unit {
            player: actor,
            unit_id,
        };
        assert!(
            game.legal_actions()
                .unwrap()
                .iter()
                .any(|action| matches!(action,
            Action::WithPlayChoices { choices, .. } if choices.exhaust_unit == Some(unit_id)))
        );
        game.apply(ki(target, Some(unit_id))).unwrap();
        settle_unopposed_chain(&mut game);
        let unit = game.unit_ref(target).unwrap();
        assert!(!unit.ready && unit.stunned);
    }
}

#[test]
fn ki_burst_counter_does_not_refund_exhaustion_or_energy() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 491103);
        let cost = ready_unit(&mut game, actor, 71);
        let enemy = ready_unit(&mut game, 1 - actor, 71);
        let target = Target::Unit {
            player: 1 - actor,
            unit_id: enemy,
        };
        game.players[actor].hand = vec![491];
        game.players[1 - actor].hand = vec![12];
        game.apply(ki(target, Some(cost))).unwrap();
        let chain_id = game.chain[0].id;
        if game.priority() != 1 - actor {
            game.apply(Action::Pass).unwrap();
        }
        game.apply(play(vec![Target::Spell { chain_id }])).unwrap();
        settle_unopposed_chain(&mut game);
        assert_eq!(game.players[actor].energy, 19);
        assert!(!game.players[actor].units[0].ready);
        assert!(!game.unit_ref(target).unwrap().stunned);
        assert!(game.players[actor].discard.contains(&491));
    }
}

#[test]
fn waters_wage_hand_plays_one_exhausted_gold_after_resolution() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 492101);
        game.players[actor].hand = vec![492];
        game.apply(play(vec![])).unwrap();
        assert_eq!(game.players[actor].energy, 19);
        assert!(game.players[actor].gear.is_empty());
        settle_unopposed_chain(&mut game);
        let gold = &game.players[actor].gear;
        assert_eq!(gold.len(), 1);
        assert!(gold[0].token && !gold[0].ready && gold[0].attached_to.is_none());
    }
}

#[test]
fn waters_wage_hidden_own_turn_is_three_ready_gold_opponent_turn_is_one_exhausted() {
    for actor in 0..2 {
        for own_turn in [false, true] {
            let mut game = position(actor, 0, 492102);
            let holder = ready_unit(&mut game, actor, 71);
            game.unit_mut(Target::Unit {
                player: actor,
                unit_id: holder,
            })
            .unwrap()
            .lane = Some(0);
            game.battlefield_owner[0] = Some(actor);
            game.players[actor].hand = vec![492];
            game.apply(Action::Hide {
                hand_index: 0,
                lane: 0,
            })
            .unwrap();
            let hidden_id = game.players[actor].hidden[0].id;
            reject_unchanged(
                &mut game,
                Action::PlayHidden {
                    hidden_id,
                    targets: vec![],
                },
            );
            let turn = game.turn;
            while game.turn < turn + if own_turn { 2 } else { 1 } {
                let action = if game.in_window() {
                    Action::Pass
                } else {
                    Action::EndTurn
                };
                game.apply(action).unwrap();
            }
            assert_eq!(game.active == actor, own_turn);
            // Open a genuine response window when it is the other player's turn.
            if !own_turn {
                game.players[1 - actor].hand = vec![15];
                game.apply(play(vec![])).unwrap();
                if game.priority() != actor {
                    game.apply(Action::Pass).unwrap();
                }
            }
            let energy = game.players[actor].energy;
            game.apply(Action::PlayHidden {
                hidden_id,
                targets: vec![],
            })
            .unwrap();
            assert_eq!(game.players[actor].energy, energy);
            assert!(game.players[actor].gear.is_empty());
            settle_unopposed_chain(&mut game);
            assert_eq!(game.players[actor].gear.len(), if own_turn { 3 } else { 1 });
            assert!(
                game.players[actor]
                    .gear
                    .iter()
                    .all(|g| g.token && g.ready == own_turn)
            );
            if own_turn {
                let gold: Vec<_> = game.players[actor].gear.iter().map(|g| g.id).collect();
                let trash = game.players[actor].discard.clone();
                for gear_id in gold {
                    game.apply(Action::ActivateGear { gear_id }).unwrap();
                }
                assert_eq!(game.players[actor].any_power, 3);
                assert!(game.players[actor].gear.is_empty());
                assert_eq!(
                    game.players[actor].discard, trash,
                    "tokens do not become trash cards"
                );
            }
        }
    }
}

#[test]
fn invert_timelines_discards_both_hands_then_draws_four_with_burnout() {
    for actor in 0..2 {
        for empty_deck in [false, true] {
            let mut game = position(actor, 0, 493101);
            game.players[actor].hand = vec![493, 1, 2, 3, 4];
            game.players[1 - actor].hand = vec![5, 6, 7, 8];
            for state in &mut game.players {
                state.deck = if empty_deck { vec![] } else { vec![71; 8] };
            }
            let power = game.players[actor].power_pool[domain_index(Domain::Chaos)];
            game.apply(play(vec![])).unwrap();
            assert_eq!(game.players[actor].energy, 17);
            assert_eq!(
                game.players[actor].power_pool[domain_index(Domain::Chaos)],
                power - 1
            );
            assert_eq!(game.players[1 - actor].hand.len(), 4);
            settle_unopposed_chain(&mut game);
            for state in &game.players {
                assert_eq!(state.hand.len(), 4);
                assert_eq!(state.score, u8::from(empty_deck));
                assert!(
                    !state.hand.contains(&493),
                    "the resolving spell cannot recycle"
                );
            }
            if empty_deck {
                let mut ours = game.players[actor].hand.clone();
                ours.sort();
                let mut theirs = game.players[1 - actor].hand.clone();
                theirs.sort();
                assert_eq!(ours, vec![1, 2, 3, 4]);
                assert_eq!(theirs, vec![5, 6, 7, 8]);
                assert_eq!(game.players[actor].discard, vec![493]);
            } else {
                assert_eq!(game.players[actor].discard, vec![1, 2, 3, 4, 493]);
                assert_eq!(game.players[1 - actor].discard, vec![5, 6, 7, 8]);
            }
        }
    }
}

#[test]
fn party_favors_asks_the_opponent_and_both_choices_apply_to_both_players() {
    for actor in 0..2 {
        for cards in [false, true] {
            let mut game = position(actor, 0, 494101);
            for state in &mut game.players {
                state.rune_deck = vec![Domain::Calm, Domain::Chaos];
            }
            game.players[actor].hand = vec![494];
            game.apply(play(vec![])).unwrap();
            party_choice(&mut game);
            assert!(matches!(game.decision, Some(Decision::SharedResources {
                player, controller, card_id: 494
            }) if player == 1-actor && controller == actor));
            let before = format!("{game:?}");
            let choices = game.binary_choices().unwrap().unwrap();
            assert_eq!(choices.player, 1 - actor);
            assert!(choices.options.iter().all(|option| option.enabled()));
            assert_eq!(format!("{game:?}"), before);
            reject_unchanged(&mut game, Action::Pass);
            let mut replay = game.clone();
            game.apply(Action::Choose { accept: cards }).unwrap();
            replay.apply(Action::Choose { accept: cards }).unwrap();
            assert_eq!(format!("{game:?}"), format!("{replay:?}"));
            settle_unopposed_chain(&mut game);
            for (owner, state) in game.players.iter().enumerate() {
                assert_eq!(state.hand.len(), usize::from(cards));
                assert_eq!(state.total_runes(), u8::from(!cards));
                assert_eq!(state.ready_runes(Domain::Chaos), 0);
                assert_eq!(state.energy, if owner == actor { 17 } else { 20 });
            }
        }
    }
}

#[test]
fn party_favors_empty_rune_deck_does_not_forbid_runes_or_create_energy() {
    let mut game = position(0, 0, 494102);
    game.players[0].rune_deck.clear();
    game.players[1].rune_deck = vec![Domain::Calm];
    game.players[0].hand = vec![494];
    game.apply(play(vec![])).unwrap();
    party_choice(&mut game);
    assert!(game.binary_choices().unwrap().unwrap().options[1].enabled());
    game.apply(Action::Choose { accept: false }).unwrap();
    settle_unopposed_chain(&mut game);
    assert_eq!(game.players[0].total_runes(), 0);
    assert_eq!(game.players[1].total_runes(), 1);
    assert_eq!(game.players[1].energy, 20);
}

#[test]
fn party_favors_draws_in_turn_order_and_repeated_empty_burnout_stops_the_spell() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 494104);
        game.players[actor].hand = vec![494];
        game.players[actor].deck.clear();
        game.players[1 - actor].score = 7;
        game.apply(play(vec![])).unwrap();
        party_choice(&mut game);
        assert!(game.binary_choices().unwrap().unwrap().options[0].enabled());
        game.apply(Action::Choose { accept: true }).unwrap();
        assert_eq!(game.winner, Some(1 - actor));
        assert!(
            game.players[1 - actor].hand.is_empty(),
            "first player's repeated Burn Out ends the game immediately"
        );
    }
}

#[test]
fn party_favors_ai_choice_does_not_depend_on_hidden_card_identities() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 494103);
        game.players[actor].hand = vec![494, 1, 2];
        game.apply(play(vec![])).unwrap();
        party_choice(&mut game);
        let original = game.shared_resources_prefers_cards(1 - actor);
        game.players[actor].hand = vec![3, 4];
        game.players[actor].deck.fill(70);
        game.players[1 - actor].deck.fill(69);
        assert_eq!(game.shared_resources_prefers_cards(1 - actor), original);
    }
}

#[test]
fn party_favors_every_ai_level_resolves_opponent_choices_and_avoids_lethal_draw() {
    for difficulty in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        for actor in 0..2 {
            for lethal in [false, true] {
                let mut game = position(actor, 0, 494105);
                game.difficulty = difficulty;
                game.players[actor].hand = vec![494];
                if lethal {
                    game.players[actor].score = 7;
                    game.players[1 - actor].deck.clear();
                    game.players[1 - actor].discard.clear();
                }
                for state in &mut game.players {
                    state.rune_deck = vec![Domain::Calm];
                }
                game.apply(play(vec![])).unwrap();
                party_choice(&mut game);
                let mut changed = game.clone();
                changed.players[actor].deck.fill(69);
                assert!(game.step_ai().unwrap());
                assert!(changed.step_ai().unwrap());
                assert_eq!(
                    game.players[0].total_runes(),
                    changed.players[0].total_runes()
                );
                assert_eq!(
                    game.players[1].total_runes(),
                    changed.players[1].total_runes()
                );
                assert!(!matches!(
                    game.decision,
                    Some(Decision::SharedResources { .. })
                ));
                if lethal {
                    assert!(game.winner.is_none(), "Runes avoids a known immediate loss");
                    assert_eq!(game.players[1 - actor].total_runes(), 1);
                } else {
                    assert!(
                        game.players.iter().all(|p| p.total_runes() == 1)
                            || game.players.iter().all(|p| p.hand.len() == 1)
                    );
                }
            }
        }
    }
}

#[test]
fn party_favors_repeat_offers_a_fresh_choice_after_the_first_reward() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 494106);
        game.players[actor].hand = vec![494];
        game.grant_play(
            actor,
            ChainSource::Card { card_id: 57 },
            PlayGrantKind::RepeatNextSpell,
        );
        let option_id = game.play_choices(&play(vec![])).unwrap().repeats[0].id;
        game.apply(Action::WithPlayChoices {
            action: Box::new(play(vec![])),
            choices: PlayChoices {
                repeats: vec![crate::play::RepeatSelection {
                    option_id,
                    choices: ExecutionChoices::default(),
                }],
                ..Default::default()
            },
        })
        .unwrap();
        assert_eq!(game.players[actor].energy, 14);
        party_choice(&mut game);
        game.apply(Action::Choose { accept: true }).unwrap();
        assert!(game.players.iter().all(|p| p.hand.len() == 1));
        assert!(
            matches!(game.decision, Some(Decision::SharedResources { player, .. }) if player == 1-actor)
        );
        game.apply(Action::Choose { accept: false }).unwrap();
        settle_unopposed_chain(&mut game);
        assert!(
            game.players
                .iter()
                .all(|p| p.hand.len() == 1 && p.total_runes() == 1)
        );
        assert_eq!(
            game.players[actor]
                .discard
                .iter()
                .filter(|&&id| id == 494)
                .count(),
            1
        );
    }
}
