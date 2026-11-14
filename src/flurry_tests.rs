//! Printed UNL-044: modes finalize before responses; instructed Birds use CR354/355.
use super::*;
use test_support::{position, ready_unit, reject_unchanged};

const FLURRY: CardId = 463;

fn action(mode: Option<usize>, targets: Vec<Target>) -> Action {
    Action::WithPlayChoices {
        action: Box::new(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets,
        }),
        choices: PlayChoices {
            mode,
            ..Default::default()
        },
    }
}

fn settle(g: &mut Game) {
    for _ in 0..40 {
        if g.decision.is_some() || g.chain.is_empty() {
            return;
        }
        g.apply(Action::Pass).unwrap();
    }
    panic!("chain did not settle");
}

fn priority(g: &mut Game, actor: usize) {
    for _ in 0..2 {
        if g.priority() == actor {
            return;
        }
        g.apply(Action::Pass).unwrap();
    }
    assert_eq!(g.priority(), actor);
}

fn spell_position(actor: usize, owner: usize) -> (Game, Target) {
    let mut g = position(owner, 0, 463001);
    g.players[owner].hand = vec![78]; // Draw three; no chosen targets or triggers.
    g.apply(Action::PlayCard {
        target: None,
        hand_index: 0,
        lane: None,
    })
    .unwrap();
    let target = Target::Spell {
        chain_id: g.chain.last().unwrap().id,
    };
    priority(&mut g, actor);
    g.players[actor].hand = vec![FLURRY];
    (g, target)
}

fn place_birds(g: &mut Game, actor: usize, lanes: &[Option<usize>]) {
    for &lane in lanes {
        settle(g);
        assert!(
            matches!(g.decision, Some(Decision::TokenPlacement { player, card_id: FLURRY, .. } | Decision::TokenPlay { player, card_id: FLURRY, .. }) if player == actor),
            "{:?}",
            g.decision
        );
        let play = Action::PlaceToken { lane };
        assert!(
            g.play_choices(&play).unwrap().modes.is_empty(),
            "a token does not inherit its creator's modes"
        );
        let before = g.players[actor].energy;
        let mut copy = g.clone();
        g.apply(play.clone()).unwrap();
        copy.apply(play).unwrap();
        assert_eq!(
            format!("{g:?}"),
            format!("{copy:?}"),
            "cloned token choices preserve continuation and random state"
        );
        assert_eq!(
            g.players[actor].energy, before,
            "Birds do not cost Flurry's four energy"
        );
    }
    settle(g);
}

#[test]
fn flurry_requires_a_mode_and_mode_specific_targets_before_payment() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 463002);
        g.players[actor].hand = vec![FLURRY];
        let unit = ready_unit(&mut g, actor, 110);
        let unit_target = Target::Unit {
            player: actor,
            unit_id: unit,
        };
        let view = g.play_choices(&action(None, vec![])).unwrap();
        assert_eq!(view.modes.len(), 2);
        for invalid in [
            action(None, vec![]),
            action(Some(2), vec![]),
            action(Some(0), vec![]),
            action(Some(0), vec![unit_target]),
            action(Some(1), vec![unit_target]),
            action(Some(0), vec![Target::Spell { chain_id: u32::MAX }]),
        ] {
            reject_unchanged(&mut g, invalid);
        }
        let candidates = g.canonical_play_candidates(64).unwrap();
        assert!(!candidates.is_empty());
        for candidate in candidates {
            g.clone().apply(candidate).unwrap();
        }
    }
}

#[test]
fn flurry_counters_either_players_spell_with_normal_cost_and_response_window() {
    for actor in 0..2 {
        for owner in 0..2 {
            let (mut g, target) = spell_position(actor, owner);
            let energy = g.players[actor].energy;
            let power = g.players[actor].power_pool;
            g.apply(action(Some(0), vec![target])).unwrap();
            assert_eq!(
                g.chain.len(),
                2,
                "countering waits for both players to pass"
            );
            assert_eq!(g.players[actor].energy, energy - 4);
            assert_eq!(
                g.players[actor].power_pool[Domain::Calm as usize],
                power[Domain::Calm as usize] - 2
            );
            assert_eq!(g.chain.last().unwrap().executions[0].mode, Some(0));
            settle(&mut g);
            assert!(g.chain.is_empty());
            assert!(g.players[owner].discard.contains(&78));
            assert!(g.players[actor].discard.contains(&FLURRY));
            assert!(g.players.iter().all(|p| p.units.is_empty()));
        }
    }
}

#[test]
fn flurry_birds_finalize_after_parent_choose_placements_and_keep_token_characteristics() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 463003);
        let holder = ready_unit(&mut g, actor, 110);
        g.unit_mut(Target::Unit {
            player: actor,
            unit_id: holder,
        })
        .unwrap()
        .lane = Some(0);
        g.battlefield_owner[0] = Some(actor);
        g.players[actor].hand = vec![FLURRY];
        g.apply(action(Some(1), vec![])).unwrap();
        assert_eq!(g.players[actor].units.len(), 1);
        settle(&mut g);
        assert!(
            g.chain.is_empty(),
            "the creating spell finishes before a Bird finalizes"
        );
        assert_eq!(g.players[actor].discard, [FLURRY]);
        reject_unchanged(&mut g, Action::PlaceToken { lane: Some(1) });
        place_birds(&mut g, actor, &[Some(0), None, Some(0), None]);
        let birds: Vec<_> = g.players[actor].units.iter().filter(|u| u.token).collect();
        assert_eq!(birds.len(), 4);
        assert_eq!(birds.iter().filter(|u| u.lane == Some(0)).count(), 2);
        for bird in birds {
            assert_eq!(g.unit_might(bird).unwrap(), 1);
            assert!(!bird.ready);
            assert_eq!(g.unit_name(bird), "Bird token");
            assert_eq!(g.unit_keywords(bird).unwrap().deflect, 1);
            assert!(g.board_target_has_tag(
                Target::Unit {
                    player: actor,
                    unit_id: bird.id
                },
                "Bird"
            ));
        }
    }
}

#[test]
fn flurry_repeat_selects_each_mode_before_responses_and_executes_both() {
    for actor in 0..2 {
        let (mut g, target) = spell_position(actor, 1 - actor);
        g.grant_play(
            actor,
            ChainSource::Card { card_id: 256 },
            PlayGrantKind::RepeatNextSpell,
        );
        let base = action(Some(0), vec![target]);
        let option_id = g.play_choices(&base).unwrap().repeats[0].id;
        let Action::WithPlayChoices {
            action: inner,
            mut choices,
        } = base
        else {
            unreachable!()
        };
        choices.repeats.push(crate::play::RepeatSelection {
            option_id,
            choices: ExecutionChoices {
                mode: Some(1),
                ..Default::default()
            },
        });
        g.apply(Action::WithPlayChoices {
            action: inner,
            choices,
        })
        .unwrap();
        assert_eq!(
            g.chain
                .last()
                .unwrap()
                .executions
                .iter()
                .map(|e| e.mode)
                .collect::<Vec<_>>(),
            [Some(0), Some(1)]
        );
        settle(&mut g);
        place_birds(&mut g, actor, &[None; 4]);
        assert!(g.players[1 - actor].discard.contains(&78));
        assert_eq!(
            g.players[actor]
                .discard
                .iter()
                .filter(|&&id| id == FLURRY)
                .count(),
            1
        );
        assert_eq!(g.players[actor].units.iter().filter(|u| u.token).count(), 4);
    }
}

#[test]
fn flurry_lost_counter_target_does_not_switch_modes() {
    for actor in 0..2 {
        let (mut g, target) = spell_position(actor, 1 - actor);
        g.apply(action(Some(0), vec![target])).unwrap();
        priority(&mut g, 1 - actor);
        let counter = g
            .catalog
            .iter()
            .find(|c| rules::effect(c.id) == OfficialEffect::Counter(CounterKind::Unconditional))
            .unwrap()
            .id;
        g.players[1 - actor].hand = vec![counter];
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        })
        .unwrap();
        settle(&mut g);
        assert!(g.chain.is_empty());
        assert!(g.decision.is_none());
        assert!(g.players.iter().all(|p| p.units.is_empty()));
        assert!(g.players[actor].discard.contains(&FLURRY));
    }
}

#[test]
fn flurry_payment_undo_restores_resources_and_completion_retains_the_selected_mode() {
    for actor in 0..2 {
        for mode in 0..2 {
            let (mut g, target) = spell_position(actor, 1 - actor);
            let mut runes = [0; 6];
            runes[Domain::Calm as usize] = 1;
            g.players[actor].set_rune_setup(runes, [0; 6]).unwrap();
            g.players[actor].energy += 1;
            let targets = if mode == 0 { vec![target] } else { vec![] };
            let selected = action(Some(mode), targets.clone());
            let before = format!("{g:?}");
            g.apply(Action::BeginPayment {
                action: Box::new(selected.clone()),
            })
            .unwrap();
            g.apply(Action::ActivateRune {
                domain: Domain::Calm,
                recycle: false,
            })
            .unwrap();
            g.apply(Action::Choose { accept: false }).unwrap();
            assert_eq!(
                format!("{g:?}"),
                before,
                "payment undo restores the whole attempt"
            );
            g.apply(Action::BeginPayment {
                action: Box::new(selected),
            })
            .unwrap();
            g.apply(Action::CompletePayment).unwrap();
            let item = g.chain.last().unwrap();
            assert_eq!(item.card_id, FLURRY);
            assert_eq!(item.executions[0].mode, Some(mode));
            assert_eq!(item.executions[0].targets, targets);
        }
    }
}

#[test]
fn flurry_instructed_banish_play_requires_a_mode_and_waives_only_the_printed_energy() {
    for actor in 0..2 {
        for mode in 0..2 {
            let (mut g, target) = spell_position(actor, 1 - actor);
            // Authored pending Clocktower instruction: both origins use ordinary finalization.
            g.players[actor].hand.clear();
            g.players[actor].banish = vec![FLURRY];
            g.queue_banish_play(actor, 0, InstructedPlayKind::ClocktowerGuardian, None);
            g.after_resolution().unwrap();
            assert!(matches!(
                g.decision,
                Some(Decision::PendingPlay {
                    card_id: FLURRY,
                    ..
                })
            ));
            let targets = if mode == 0 { vec![target] } else { vec![] };
            let finalize = Action::FinalizePendingPlay {
                lane: None,
                targets: targets.clone(),
                repeat_targets: vec![],
                repeat: false,
                accelerate: false,
                bonus: false,
            };
            reject_unchanged(&mut g, finalize.clone());
            let candidates = g.legal_actions().unwrap();
            assert!(!candidates.is_empty());
            for candidate in candidates {
                g.clone().apply(candidate).unwrap();
            }
            let energy = g.players[actor].energy;
            let power = g.players[actor].power_pool[Domain::Calm as usize];
            g.apply(Action::WithPlayChoices {
                action: Box::new(finalize),
                choices: PlayChoices {
                    mode: Some(mode),
                    ..Default::default()
                },
            })
            .unwrap();
            let item = g.chain.last().unwrap();
            assert_eq!(item.play_origin, Some(PlayOrigin::Banish));
            assert_eq!(item.executions[0].mode, Some(mode));
            assert_eq!(item.executions[0].targets, targets);
            assert_eq!(g.players[actor].energy, energy);
            assert_eq!(
                g.players[actor].power_pool[Domain::Calm as usize],
                power - 2
            );
            settle(&mut g);
            if mode == 1 {
                place_birds(&mut g, actor, &[None; 4]);
                assert_eq!(g.players[actor].units.iter().filter(|u| u.token).count(), 4);
            } else {
                assert!(g.chain.is_empty());
                assert!(g.players[actor].units.is_empty());
            }
        }
    }
}

#[test]
fn flurry_ai_preserves_its_board_by_countering_a_sweeper_in_both_seats() {
    for actor in 0..2 {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let mut g = position(1 - actor, 0, 463004);
            g.difficulty = difficulty;
            for _ in 0..2 {
                ready_unit(&mut g, actor, 71);
            }
            g.players[1 - actor].hand = vec![308];
            assert_eq!(rules::effect(308), OfficialEffect::KillAllUnits);
            g.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            })
            .unwrap();
            priority(&mut g, actor);
            g.players[actor].hand = vec![FLURRY];
            g.players[1 - actor].hand = vec![8, 8, 8];

            let mut pass = g.clone();
            pass.apply(Action::Pass).unwrap();
            settle(&mut pass);
            assert!(pass.players[actor].units.is_empty());
            let mut tokens = g.clone();
            tokens.apply(action(Some(1), vec![])).unwrap();
            settle(&mut tokens);
            place_birds(&mut tokens, actor, &[None; 4]);
            assert!(
                tokens.players[actor].units.is_empty(),
                "Birds also die to the waiting sweeper"
            );

            let before = format!("{g:?}");
            let chosen = g.choose_ai_action().unwrap();
            assert_eq!(format!("{g:?}"), before);
            let mut hidden = g.clone();
            hidden.players[1 - actor].hand = vec![1, 4, 5];
            hidden.players[1 - actor].deck.reverse();
            assert_eq!(chosen, hidden.choose_ai_action().unwrap());
            g.apply(chosen.clone()).unwrap();
            let counter = g.chain.last().unwrap();
            assert_eq!(counter.card_id, FLURRY, "{difficulty:?}: {chosen:?}");
            assert_eq!(
                counter.executions[0].mode,
                Some(0),
                "{difficulty:?}: {chosen:?}"
            );
            settle(&mut g);
            assert_eq!(g.players[actor].units.len(), 2);
        }
    }
}
