use super::*;
use crate::game::test_support::{decline_legend_action, position};
use test_support::{apply, views};

#[test]
fn optional_trash_api_reports_kind_union_and_distinguishes_return_from_instructed_play() {
    for actor in 0..2 {
        for (source, chosen, outcome, kinds) in [
            (476, 472, "return_to_hand", json!(["unit", "gear"])),
            (477, 470, "play", json!(["unit"])),
        ] {
            for skip in [false, true] {
                let mut s = Session {
                    game: position(actor, 0, 477100),
                    revision: 0,
                    series: None,
                    format_date: None,
                };
                s.game.players[actor].discard = vec![8, chosen];
                s.game.decision = Some(Decision::TrashTarget {
                    player: actor,
                    source_card_id: source,
                    gear_id: None,
                });
                let observation = views(&s);
                assert_eq!(observation[actor]["decision"]["optional"], true);
                assert_eq!(observation[actor]["decision"]["card_kinds"], kinds);
                assert_eq!(observation[actor]["decision"]["outcome"], outcome);
                assert_eq!(
                    observation[actor]["decision"]["card_kind"],
                    if source == 476 {
                        Value::Null
                    } else {
                        json!("unit")
                    }
                );
                let result = s.handle(json!({"command":"apply", "revision":s.revision(),
                    "player":1-actor,"action":{"type":"select_cards","indices":[1]}}));
                assert_eq!(result["ok"], false);
                assert_eq!(views(&s), observation);
                apply(
                    &mut s,
                    Action::SelectCards {
                        indices: if skip { vec![] } else { vec![1] },
                    },
                );
                if skip {
                    assert_eq!(s.game.players[actor].discard, vec![8, chosen]);
                    assert!(s.game.chain.is_empty());
                    continue;
                }
                assert_eq!(s.game.players[actor].discard, vec![8, chosen]);
                for _ in 0..60 {
                    if matches!(s.game.decision, Some(Decision::PendingPlay { .. }))
                        || (s.game.decision.is_none() && s.game.chain.is_empty())
                    {
                        break;
                    }
                    advance(&mut s);
                }
                if source == 477 {
                    let observation = views(&s);
                    assert_eq!(observation[actor]["decision"]["kind"], "soulgorger");
                    assert_eq!(
                        observation[actor]["decision"]["origin"],
                        json!({"type":"trash"})
                    );
                    assert_eq!(
                        observation[actor]["decision"]["instruction"],
                        json!({"base_cost_waiver":"energy", "destination":{"mode":"normal"}})
                    );
                    apply(
                        &mut s,
                        Action::FinalizePendingPlay {
                            lane: None,
                            targets: vec![],
                            repeat_targets: vec![],
                            repeat: false,
                            accelerate: false,
                            bonus: false,
                        },
                    );
                    assert_eq!(s.game.players[actor].units[0].card_id, chosen);
                } else {
                    let observation = views(&s);
                    assert_eq!(
                        observation[actor]["players"][actor]["hand"],
                        json!([chosen])
                    );
                    assert!(observation[1 - actor]["players"][actor]["hand"].is_null());
                }
            }
        }
    }
}

fn advance(s: &mut Session) {
    let action = if let Some(action) = decline_legend_action(&s.game) {
        action
    } else if let Some(Decision::OrderTriggers { options, .. }) = &s.game.decision {
        Action::OrderTrigger {
            trigger_id: options[0].id,
        }
    } else {
        assert!(s.game.decision.is_none() && !s.game.chain.is_empty());
        Action::Pass
    };
    apply(s, action);
}

#[test]
fn recovery_api_explains_choices_preserves_atomicity_and_filters_hands() {
    for actor in 0..2 {
        for (source, chosen, wrong, kind) in [(474, 96, 8, "unit"), (475, 8, 96, "spell")] {
            let mut s = Session {
                game: position(actor, 0, 475100),
                revision: 0,
                series: None,
                format_date: None,
            };
            s.game.players[actor].hand = vec![source];
            s.game.players[actor].discard = vec![wrong, chosen, chosen];
            apply(
                &mut s,
                Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                },
            );
            for _ in 0..60 {
                if matches!(s.game.decision, Some(Decision::TrashTarget { .. })) {
                    break;
                }
                advance(&mut s);
            }
            let before = views(&s);
            assert_eq!(before[actor]["decision"]["outcome"], "return_to_hand");
            assert_eq!(before[actor]["decision"]["card_kind"], kind);
            let revision = s.revision();
            for player in [actor, 1 - actor] {
                let result = s.handle(
                    json!({"command":"apply", "revision":revision, "player":player,
                    "action":{"type":"select_cards", "indices":[0]}}),
                );
                assert_eq!(result["ok"], false, "{result}");
                assert_eq!(s.revision(), revision);
                assert_eq!(views(&s), before);
            }
            let result = s.handle(
                json!({"command":"apply", "revision":revision, "player":actor,
                "action":{"type":"select_cards", "indices":[2]}}),
            );
            assert_eq!(result["ok"], true, "{result}");
            let ability = ability_to_json(s.game.chain.last().unwrap().ability.as_ref().unwrap());
            assert_eq!(
                ability,
                json!({"type":"return_chosen", "trash_index":2,"card_id":chosen})
            );
            for _ in 0..60 {
                if s.game.decision.is_none() && s.game.chain.is_empty() {
                    break;
                }
                advance(&mut s);
            }
            assert_eq!(s.game.players[actor].hand, vec![chosen]);
            let after = views(&s);
            assert_eq!(after[actor]["players"][actor]["hand"], json!([chosen]));
            assert!(after[1 - actor]["players"][actor]["hand"].is_null());
            assert_eq!(after[1 - actor]["players"][actor]["hand_count"], 1);
        }
    }
}

#[test]
fn scout_api_vision_reveals_top_card_only_to_its_controller() {
    for actor in 0..2 {
        let mut s = Session {
            game: position(actor, 0, 473100),
            revision: 0,
            series: None,
            format_date: None,
        };
        s.game.players[actor].hand = vec![473];
        s.game.players[actor].deck = vec![96, 71, 8];
        apply(
            &mut s,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: Some(0),
                targets: vec![],
            },
        );
        for _ in 0..60 {
            if matches!(s.game.decision, Some(Decision::Predict { .. })) {
                break;
            }
            advance(&mut s);
        }
        let view = views(&s);
        assert_eq!(view[actor]["decision"]["card_id"], 8);
        assert!(view[1 - actor]["decision"]["card_id"].is_null());
        apply(&mut s, Action::Choose { accept: true });
        assert_eq!(s.game.players[actor].deck, vec![8, 96, 71]);
    }
}

#[test]
fn trash_spell_api_exposes_public_candidates_and_preserves_exact_reference_tokens() {
    for actor in 0..2 {
        for source in [478, 479] {
            let mut s = Session {
                game: position(actor, 0, 479100),
                revision: 0,
                series: None,
                format_date: None,
            };
            s.game.players[actor].hand = vec![source];
            for state in &mut s.game.players {
                state.discard = vec![8, 470];
            }
            let empty = Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            };
            let before = views(&s);
            let view = s.target_candidates(actor, s.revision(), empty, 0).unwrap();
            assert_eq!(view.spec.kind, Some(crate::rules::PlayTargetKind::Trash));
            assert_eq!(view.spec.required, if source == 478 { 1 } else { 0 });
            assert_eq!(view.candidates.len(), if source == 478 { 1 } else { 2 });
            assert_eq!(
                views(&s),
                before,
                "preview must not change either seat's observation"
            );
            for target in &view.candidates {
                let encoded = target_to_json(target);
                assert_eq!(encoded["type"], "trash");
                assert_eq!(parse_target(&encoded).unwrap(), *target);
                assert_eq!(encoded["reference"]["index"], 1);
                let mut malformed = encoded.clone();
                malformed["reference"]["unknown"] = json!(true);
                assert!(parse_target(&malformed).is_err());
            }
            let targets = view.candidates.clone();
            let action = Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: targets.clone(),
            };
            let wrong=s.handle(json!({"command":"apply","revision":s.revision(),"player":1-actor,"action":action_to_json(&action)}));
            assert_eq!(wrong["ok"], false);
            assert_eq!(views(&s), before);
            apply(&mut s, action);
            let observations = views(&s);
            for observation in observations {
                assert_eq!(
                    observation["chain"][0]["targets"],
                    json!(targets.iter().map(target_to_json).collect::<Vec<_>>())
                );
            }
            for _ in 0..50 {
                if s.game.chain.is_empty() && s.game.decision.is_none()
                    || matches!(s.game.decision, Some(Decision::PendingPlay { .. }))
                {
                    break;
                }
                advance(&mut s);
            }
            if source == 478 {
                let observation = s.observe(actor).unwrap();
                assert_eq!(observation["decision"]["kind"], "the_harrowing");
                assert_eq!(
                    observation["decision"]["instruction"]["base_cost_waiver"],
                    "energy"
                );
            } else {
                for owner in 0..2 {
                    assert_eq!(
                        s.observe(owner).unwrap()["players"][owner]["hand"],
                        json!([470])
                    );
                    assert!(s.observe(1 - owner).unwrap()["players"][owner]["hand"].is_null());
                }
            }
        }
    }
}
