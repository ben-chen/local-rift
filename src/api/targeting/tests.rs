use super::*;
use crate::game::Unit;
use crate::gear::GearState;
use crate::play::{ExecutionChoices, PlayChoices, RepeatSelection};

#[test]
fn backoff_api_preserves_hidden_privacy_origin_and_the_hand_only_draw() {
    for from_hidden in [false, true] {
        let mut session = position();
        session.game.battlefields = [2, 2];
        session.game.battlefield_owner[0] = Some(0);
        session.game.players[0].units = vec![unit(500, 73, Some(0), false)];
        session.game.players[0].hand = vec![428];
        session.game.players[0].deck = vec![71];
        let target = Target::Unit {
            player: 0,
            unit_id: 500,
        };
        let action = if from_hidden {
            apply(
                &mut session,
                Action::Hide {
                    hand_index: 0,
                    lane: 0,
                },
            );
            let owner = session.observe(0).unwrap();
            let opponent = session.observe(1).unwrap();
            assert_eq!(owner["players"][0]["hidden"][0]["card_id"], 428);
            assert!(opponent["players"][0]["hidden"][0]["card_id"].is_null());
            session.game.players[0].hidden[0].hidden_turn = session.game.turn - 1;
            Action::PlayHidden {
                hidden_id: session.game.players[0].hidden[0].id,
                targets: vec![target],
            }
        } else {
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![target],
            }
        };
        let before = session.observe(0).unwrap();
        for command in ["play_choices", "cost_plan", "quote_payment", "validate"] {
            let response = session.handle(json!({"command":command,"player":0,"revision":session.revision(),"action":action_to_json(&action)}));
            assert_eq!(response["ok"], true, "{response}");
            assert_eq!(session.observe(0).unwrap(), before);
        }
        apply(&mut session, action);
        for viewer in 0..2 {
            let view = session.observe(viewer).unwrap();
            let spell = view["chain"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["card_id"] == 428)
                .unwrap();
            assert_eq!(
                spell["play_origin"]["type"],
                if from_hidden { "hidden" } else { "hand" }
            );
        }
        resolve_chain(&mut session);
        assert!(session.game.players[0].units[0].stunned);
        assert!(session.game.players[0].units[0].ready);
        assert_eq!(
            session.game.players[0].hand,
            if from_hidden { vec![] } else { vec![71] }
        );
        assert_eq!(
            session.observe(1).unwrap()["players"][0]["units"][0]["stunned"],
            true
        );
    }
}

#[test]
fn move_destination_wire_is_explicit_strict_and_preserves_legacy_omission() {
    use crate::play::MoveDestination;
    for destination in [
        None,
        Some(MoveDestination::Base),
        Some(MoveDestination::Battlefield { lane: 1 }),
    ] {
        let action = Action::WithPlayChoices {
            action: Box::new(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: Some(0),
                targets: vec![],
            }),
            choices: PlayChoices {
                repeats: vec![RepeatSelection {
                    option_id: 1,
                    choices: ExecutionChoices {
                        destination,
                        ..Default::default()
                    },
                }],
                ..Default::default()
            },
        };
        let wire = action_to_json(&action);
        assert_eq!(parse_action(&wire).unwrap(), action);
        assert_eq!(
            wire["choices"]["repeats"][0]["choices"]
                .get("destination")
                .is_some(),
            destination.is_some()
        );
        for invalid in [
            json!(null),
            json!("base"),
            json!({}),
            json!({"type":"unit"}),
            json!({"type":"base","lane":0}),
            json!({"type":"battlefield"}),
            json!({"type":"battlefield","lane":-1}),
            json!({"type":"battlefield","lane":2}),
            json!({"type":"battlefield","lane":0.5}),
            json!({"type":"battlefield","lane":"1"}),
        ] {
            let mut bad = wire.clone();
            bad["choices"]["repeats"][0]["choices"]["destination"] = invalid;
            assert!(parse_action(&bad).is_err(), "{bad}");
        }
    }
}

#[test]
fn siren_api_exposes_public_history_and_independent_destinations_without_mutating_previews() {
    use crate::play::{MoveDestination, PlayGrant, PlayGrantKind};
    let mut session = position();
    session.game.players[1].units = vec![unit(500, 73, None, false), unit(501, 73, Some(1), false)];
    let target = Target::Unit {
        player: 1,
        unit_id: 500,
    };
    session.game.move_effect(0, target, Some(0), false).unwrap();
    session.game.players[0].hand = vec![427];
    session.game.play_grants.push(PlayGrant {
        id: 9900,
        player: 0,
        expires_turn: session.game.turn,
        source: ChainSource::Card { card_id: 427 },
        kind: PlayGrantKind::RepeatNextSpell,
    });
    let original = Action::PlayCardWithTargets {
        hand_index: 0,
        lane: Some(1),
        targets: vec![target],
    };
    let option = session.game.play_choices(&original).unwrap().repeats[0].id;
    let action = Action::WithPlayChoices {
        action: Box::new(original),
        choices: PlayChoices {
            repeats: vec![RepeatSelection {
                option_id: option,
                choices: ExecutionChoices {
                    destination: Some(MoveDestination::Base),
                    targets: vec![target],
                    ..Default::default()
                },
            }],
            ..Default::default()
        },
    };
    for viewer in 0..2 {
        let view = session.observe(viewer).unwrap();
        assert_eq!(view["players"][1]["units"][0]["moved_this_turn"], true);
        assert_eq!(view["players"][1]["units"][1]["moved_this_turn"], false);
    }
    assert_eq!(
        play_targets_to_json(427)["condition"]["type"],
        "moved_this_turn"
    );
    let before = session.observe(0).unwrap();
    for command in [
        "target_candidates",
        "play_choices",
        "cost_plan",
        "quote_payment",
        "validate",
    ] {
        let response = session.handle(
            json!({"command":command,"player":0,"revision":session.revision(),
            "action":action_to_json(&action),"execution":1}),
        );
        // The target query accepts execution; other commands reject unknown fields.
        if command == "target_candidates" {
            assert_eq!(response["ok"], true, "{response}");
            assert_eq!(
                response["result"]["destination"],
                json!({"type":"base"}),
                "{response}"
            );
        } else {
            let response = session.handle(
                json!({"command":command,"player":0,"revision":session.revision(),
                "action":action_to_json(&action)}),
            );
            assert_eq!(response["ok"], true, "{response}");
        }
        assert_eq!(session.observe(0).unwrap(), before);
    }
    apply(&mut session, action);
    for viewer in 0..2 {
        let view = session.observe(viewer).unwrap();
        let spell = view["chain"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["card_id"] == 427)
            .unwrap();
        assert_eq!(
            spell["executions"][0]["destination"],
            json!({"type":"battlefield","lane":1})
        );
        assert_eq!(
            spell["executions"][1]["destination"],
            json!({"type":"base"})
        );
    }
    resolve_chain(&mut session);
    assert_eq!(session.game.players[1].units[0].lane, None);
}

fn position() -> Session {
    let mut session = Session::new(ResetOptions {
        decks: [3, 3],
        ..ResetOptions::default()
    })
    .unwrap();
    session.start(0).unwrap();
    session.game.battlefields = [3, 3];
    session.game.battlefield_owner = [None; 2];
    for player in &mut session.game.players {
        player.hand.clear();
        player.units.clear();
        player.gear.clear();
        player.discard.clear();
        player.champion = None;
        player.set_rune_setup([0; 6], [0; 6]).unwrap();
        player.energy_pool = 50;
        player.energy = 50;
        player.power_pool = [20; 6];
    }
    session
}

fn unit(id: u32, card_id: usize, lane: Option<usize>, token: bool) -> Unit {
    Unit {
        id,
        card_id,
        lane,
        ready: true,
        damage: 0,
        buff: if token { 10 } else { 0 },
        temporary_buff: 0,
        token,
        protected: false,
        stunned: false,
        banish_on_death: false,
    }
}

fn apply(session: &mut Session, action: Action) {
    let result = session.handle(json!({"command":"apply","revision":session.revision(),
        "player":session.game.priority(),"action":action_to_json(&action)}));
    assert_eq!(result["ok"], true, "{result}");
}

fn settle(session: &mut Session) {
    for _ in 0..30 {
        assert!(
            session.game.decision.is_none(),
            "{:?}",
            session.game.decision
        );
        if !session.game.in_window() {
            return;
        }
        apply(session, Action::Pass);
    }
    panic!("Response window did not settle");
}

fn resolve_chain(session: &mut Session) {
    for _ in 0..30 {
        if session.game.chain.is_empty() {
            return;
        }
        assert!(
            session.game.decision.is_none(),
            "{:?}",
            session.game.decision
        );
        apply(session, Action::Pass);
    }
    panic!("Chain did not finish");
}

fn portal(session: &mut Session, id: u32) {
    session.game.players[0].gear.push(GearState {
        id,
        card_id: 256,
        attached_to: None,
        lane: None,
        ready: true,
        temporary: false,
        token: false,
    });
    apply(
        session,
        Action::ActivateAbility {
            source: AbilitySource::Gear {
                player: 0,
                gear_id: id,
            },
            ability_index: 0,
            targets: vec![],
        },
    );
    settle(session);
}

fn firestorm(lane: usize) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Battlefield { lane }],
    }
}

#[test]
fn battlefield_target_wire_requires_a_physical_slot_and_no_extra_identity() {
    for lane in 0..2 {
        let target = Target::Battlefield { lane };
        let wire = json!({"type":"battlefield","lane":lane});
        assert_eq!(target_to_json(&target), wire);
        assert_eq!(parse_target(&wire).unwrap(), target);
    }
    for invalid in [
        json!({"type":"battlefield"}),
        json!({"type":"battlefield","lane":null}),
        json!({"type":"battlefield","lane":-1}),
        json!({"type":"battlefield","lane":0.5}),
        json!({"type":"battlefield","lane":"1"}),
        json!({"type":"battlefield","lane":true}),
        json!({"type":"battlefield","lane":2}),
        json!({"type":"battlefield","lane":u64::MAX}),
        json!({"type":"battlefield","lane":0,"player":0}),
        json!({"type":"battlefield","lane":0,"battlefield_id":3}),
        json!({"type":"battlefield","lane":0,"unit_id":12}),
    ] {
        assert!(parse_target(&invalid).is_err(), "{invalid}");
    }
    for old in [
        json!({"type":"unit","player":1,"unit_id":7}),
        json!({"type":"gear","player":0,"gear_id":12}),
        json!({"type":"spell","chain_id":5}),
    ] {
        assert_eq!(target_to_json(&parse_target(&old).unwrap()), old);
    }
}

#[test]
fn battlefield_wire_preserves_independent_choices_across_play_origins_and_payments() {
    let first = vec![Target::Battlefield { lane: 0 }];
    let second = vec![Target::Battlefield { lane: 1 }];
    let origins = [
        firestorm(0),
        Action::PlayCardOptions {
            hand_index: 0,
            lane: None,
            targets: first.clone(),
            repeat_targets: second.clone(),
            repeat: true,
            accelerate: false,
            bonus: false,
        },
        Action::PlayFromTrashWithTargets {
            trash_index: 2,
            lane: None,
            targets: first.clone(),
        },
        Action::PlayFromTrashOptions {
            trash_index: 2,
            lane: None,
            targets: first.clone(),
            repeat_targets: second.clone(),
            repeat: true,
            accelerate: false,
            bonus: false,
            flow: Some(FlowSource::Granted),
        },
        Action::FinalizePendingPlay {
            lane: None,
            targets: first.clone(),
            repeat_targets: second.clone(),
            repeat: true,
            accelerate: false,
            bonus: false,
        },
    ];
    for action in origins {
        assert_eq!(parse_action(&action_to_json(&action)).unwrap(), action);
    }
    let selected = Action::WithPlayChoices {
        action: Box::new(firestorm(0)),
        choices: PlayChoices {
            repeats: vec![
                RepeatSelection {
                    option_id: 8_000_000_001,
                    choices: ExecutionChoices {
                        destination: None,
                        targets: second,
                        mode: None,
                    },
                },
                RepeatSelection {
                    option_id: 8_000_000_002,
                    choices: ExecutionChoices {
                        destination: None,
                        targets: first,
                        mode: None,
                    },
                },
            ],
            ..Default::default()
        },
    };
    for action in [
        selected.clone(),
        Action::BeginPayment {
            action: Box::new(selected.clone()),
        },
        Action::WithPowerPayment {
            action: Box::new(selected),
            payment: PowerPayment::default(),
        },
    ] {
        assert_eq!(parse_action(&action_to_json(&action)).unwrap(), action);
    }
}

#[test]
fn catalog_distinguishes_field_area_and_delayed_target_choices() {
    let value = catalog();
    for (id, kind, maximum, required) in [
        (6, Some("spell"), 1, 1),
        (41, Some("gear"), 1, 0),
        (290, Some("unit"), 1, 1),
        (295, Some("unit"), 1, 1),
        (297, None, 0, 0),
        (298, None, 0, 0),
        (299, Some("battlefield"), 1, 1),
        (23, None, 0, 0),
        (58, None, 0, 0),
        (303, None, 0, 0),
    ] {
        assert_eq!(
            value["cards"][id]["play_targets"],
            json!({"kind":kind,"maximum":maximum,"required":required}),
            "card {id}"
        );
    }
    for (id, condition, recipients) in [
        (304, "attacking_with_another", "self_unit"),
        (305, "combat_alone", "self_unit"),
        (306, "always", "other_friendly_units_here"),
        (307, "always", "other_friendly_units_here"),
    ] {
        assert_eq!(
            value["cards"][id]["passive_rules"][0]["condition"],
            condition
        );
        assert_eq!(
            value["cards"][id]["passive_rules"][0]["recipients"],
            recipients
        );
    }
}

#[test]
fn invalid_field_target_actions_and_nested_choices_preserve_both_player_views() {
    let mut session = position();
    session.game.players[0].hand = vec![299];
    session.game.players[1]
        .units
        .push(unit(700, 25, Some(0), true));
    let good = action_to_json(&firestorm(0));
    let before = [session.observe(0).unwrap(), session.observe(1).unwrap()];
    let revision = session.revision();
    let mut bad = Vec::new();
    for targets in [
        json!([]),
        json!([{"type":"unit","player":1,"unit_id":700}]),
        json!([{"type":"battlefield","lane":0},{"type":"battlefield","lane":1}]),
        json!([{"type":"battlefield","lane":2}]),
        json!([{"type":"battlefield","lane":0,"player":1}]),
    ] {
        let mut action = good.clone();
        action["targets"] = targets;
        bad.push(action);
    }
    let mut action = good.clone();
    action["lane"] = json!(0);
    bad.push(action);
    bad.push(
        json!({"type":"with_play_choices","action":good,"choices":{"repeats":[
            {"option_id":"8000000001","choices":{"targets":[{"type":"battlefield","lane":-1}]}}
        ]}}),
    );
    for action in bad {
        for command in ["validate", "apply"] {
            let response = session.handle(json!({"command":command,"player":0,
                "revision":revision,"action":action}));
            assert_eq!(response["ok"], false, "{action}: {response}");
            assert_eq!(session.revision(), revision);
            assert_eq!(
                [session.observe(0).unwrap(), session.observe(1).unwrap()],
                before
            );
        }
    }
    session.game.players[1].units.clear();
    for lane in 0..2 {
        let mut empty = session.clone();
        apply(&mut empty, firestorm(lane));
        settle(&mut empty);
        assert_eq!(empty.game.players[0].discard, vec![299]);
    }
}

#[test]
fn actual_portal_repeats_keep_each_field_through_exact_and_staged_payments() {
    for staged in [false, true] {
        let mut session = position();
        portal(&mut session, 801);
        portal(&mut session, 802);
        session.game.players[0].hand = vec![299];
        session.game.players[0]
            .units
            .push(unit(900, 25, Some(0), true));
        session.game.players[1].units = vec![
            unit(901, 25, Some(0), true),
            unit(902, 25, Some(1), true),
            unit(903, 25, None, true),
        ];
        let draft = firestorm(0);
        let options = session
            .play_choices(0, session.revision(), draft.clone())
            .unwrap();
        assert_eq!(options.repeats.len(), 2);
        let selected = Action::WithPlayChoices {
            action: Box::new(draft),
            choices: PlayChoices {
                repeats: options
                    .repeats
                    .iter()
                    .zip([1, 0])
                    .map(|(option, lane)| RepeatSelection {
                        option_id: option.id,
                        choices: ExecutionChoices {
                            destination: None,
                            targets: vec![Target::Battlefield { lane }],
                            mode: None,
                        },
                    })
                    .collect(),
                ..Default::default()
            },
        };
        let quote = session
            .quote_payment(0, session.revision(), selected.clone())
            .unwrap();
        assert_eq!(quote.suggested.total(), 3);
        if staged {
            apply(
                &mut session,
                Action::BeginPayment {
                    action: Box::new(selected.clone()),
                },
            );
            assert_eq!(
                session.observe(0).unwrap()["payment_plan"]["action"],
                action_to_json(&selected)
            );
        }
        apply(
            &mut session,
            Action::WithPowerPayment {
                action: Box::new(if staged {
                    Action::CompletePayment
                } else {
                    selected
                }),
                payment: quote.suggested,
            },
        );
        for viewer in 0..2 {
            let observed = session.observe(viewer).unwrap();
            let item = observed["chain"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["card_id"] == 299)
                .unwrap();
            let executions = item["executions"].as_array().unwrap();
            assert_eq!(executions.len(), 3);
            for (execution, lane) in executions.iter().zip([0, 1, 0]) {
                assert_eq!(
                    execution["targets"],
                    json!([{"type":"battlefield","lane":lane}])
                );
            }
        }
        resolve_chain(&mut session);
        assert_eq!(session.game.players[0].units[0].damage, 0);
        assert_eq!(
            session.game.players[1]
                .units
                .iter()
                .map(|u| u.damage)
                .collect::<Vec<_>>(),
            vec![8, 4, 0] // Each packet includes this slot's Void Gate bonus.
        );
        assert_eq!(
            session.game.players[0]
                .discard
                .iter()
                .filter(|&&c| c == 299)
                .count(),
            1
        );
    }
}

#[test]
fn kennen_granted_flow_preserves_field_selection_and_banishes_one_parent() {
    let mut session = position();
    session.game.players[0].discard = vec![299];
    session.game.players[0]
        .units
        .push(unit(910, 33, None, false));
    apply(
        &mut session,
        Action::Move {
            unit_id: 910,
            lane: Some(0),
        },
    );
    apply(&mut session, Action::Pass);
    apply(&mut session, Action::Pass);
    assert!(matches!(
        session.game.decision,
        Some(Decision::GrantFlow { player: 0 })
    ));
    apply(&mut session, Action::SelectCards { indices: vec![0] });
    settle(&mut session);
    assert_eq!(session.game.granted_flow_indices(0), &[0]);
    session.game.players[1]
        .units
        .push(unit(911, 25, Some(1), true));
    let action = Action::PlayFromTrashOptions {
        trash_index: 0,
        lane: None,
        targets: vec![Target::Battlefield { lane: 1 }],
        repeat_targets: vec![],
        repeat: false,
        accelerate: false,
        bonus: false,
        flow: Some(FlowSource::Granted),
    };
    apply(&mut session, action);
    let observed = session.observe(1).unwrap();
    let item = observed["chain"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["card_id"] == 299)
        .unwrap();
    assert_eq!(item["targets"], json!([{"type":"battlefield","lane":1}]));
    settle(&mut session);
    assert_eq!(session.game.players[1].units[0].damage, 4); // Void Gate.
    assert!(!session.game.players[0].discard.contains(&299));
    assert_eq!(
        session.game.players[0]
            .banish
            .iter()
            .filter(|&&c| c == 299)
            .count(),
        1
    );
}

#[test]
fn cost_previews_reject_supplied_wrong_kind_or_extra_fields_in_every_execution() {
    let mut session = position();
    portal(&mut session, 990);
    session.game.players[0].hand = vec![299];
    session.game.players[1]
        .units
        .push(unit(991, 25, Some(0), true));
    let option = session
        .play_choices(0, session.revision(), firestorm(0))
        .unwrap()
        .repeats[0]
        .id;
    let before = [session.observe(0).unwrap(), session.observe(1).unwrap()];
    let invalid_lists = [
        json!([{"type":"unit","player":1,"unit_id":991}]),
        json!([{"type":"battlefield","lane":0},{"type":"battlefield","lane":1}]),
    ];
    for targets in invalid_lists {
        for repeated in [false, true] {
            let mut action = action_to_json(&firestorm(0));
            if repeated {
                action = json!({"type":"with_play_choices","action":action,"choices":{"repeats":[
                    {"option_id":option.to_string(),"choices":{"targets":targets}}
                ]}});
            } else {
                action["targets"] = targets.clone();
            }
            for command in ["play_choices", "cost_plan", "quote_payment"] {
                let response = session
                    .handle(json!({"command":command,"revision":session.revision(),
                    "player":0,"action":action}));
                assert_eq!(response["ok"], false, "{command}: {action}: {response}");
                assert_eq!(
                    [session.observe(0).unwrap(), session.observe(1).unwrap()],
                    before
                );
            }
        }
    }
    // A partial draft can ask what its optional costs are; it is not a legal final play.
    let draft = json!({"type":"play_card_targets","hand_index":0,"targets":[]});
    for command in ["play_choices", "cost_plan"] {
        let response = session.handle(json!({"command":command,"revision":session.revision(),
            "player":0,"action":draft}));
        assert_eq!(response["ok"], true, "{response}");
    }
    let response = session.handle(json!({"command":"validate","revision":session.revision(),
        "player":0,"action":draft}));
    assert_eq!(response["ok"], false);
    assert_eq!(
        [session.observe(0).unwrap(), session.observe(1).unwrap()],
        before
    );
}

#[test]
fn decree_preview_wire_and_apply_share_the_live_fury_restriction() {
    let mut session = position();
    session.game.players[0].hand = vec![421];
    session.game.players[0]
        .units
        .push(unit(500, 73, Some(0), false));
    session.game.players[0]
        .units
        .push(unit(501, 73, None, false));
    session.game.players[1]
        .units
        .push(unit(502, 183, Some(0), false));
    session.game.showdown = Some(crate::game::Showdown {
        lane: 0,
        attacker: 0,
        focus: 0,
        passes: 0,
        combat: true,
    });
    let action = Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    };
    let before = format!("{:?}", session.game);
    let view = session
        .target_candidates(0, session.revision(), action, 0)
        .unwrap();
    assert_eq!(
        view.candidates,
        vec![Target::Unit {
            player: 0,
            unit_id: 500
        }]
    );
    assert_eq!(format!("{:?}", session.game), before);
    let revision = session.revision();
    let bad = session.handle(json!({"command":"apply", "player":0, "revision":revision,
        "action":{"type":"play_card_targets","hand_index":0,"lane":null,
            "targets":[{"type":"unit","player":0,"unit_id":501}]}}));
    assert_eq!(bad["ok"], false, "{bad}");
    assert_eq!(bad["error"]["code"], "illegal_action", "{bad}");
    assert_eq!(session.revision(), revision);
    assert_eq!(format!("{:?}", session.game), before);
    apply(
        &mut session,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: 0,
                unit_id: 500,
            }],
        },
    );
    resolve_chain(&mut session);
    assert_eq!(session.game.players[0].units[0].temporary_buff, 4);
}

#[test]
fn hush_api_filters_printed_power_and_rejects_invalid_choices_atomically() {
    let mut session = position();
    session.game.players[0].hand = vec![53, 63];
    session.game.players[1].hand = vec![423];
    session.game.players[1]
        .units
        .push(unit(510, 73, None, false));
    let target = Target::Unit {
        player: 1,
        unit_id: 510,
    };
    apply(
        &mut session,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target, target],
        },
    );
    let eligible = Target::Spell {
        chain_id: session.game.chain[0].id,
    };
    apply(
        &mut session,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        },
    );
    let ineligible = Target::Spell {
        chain_id: session.game.chain[1].id,
    };
    apply(&mut session, Action::Pass);
    let revision = session.revision();
    let before = format!("{:?}", session.game);
    let view = session
        .target_candidates(
            1,
            revision,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            },
            0,
        )
        .unwrap();
    assert_eq!(view.candidates, vec![eligible]);
    assert_eq!(
        play_targets_to_json(423)["condition"],
        json!({"type":"spell_printed_power_at_least", "minimum":2})
    );
    let bad = session.handle(json!({"command":"apply", "player":1, "revision":revision,
    "action":action_to_json(&Action::PlayCardWithTargets {
        hand_index: 0, lane: None, targets: vec![ineligible],
    })}));
    assert_eq!(bad["ok"], false, "{bad}");
    assert_eq!(bad["error"]["code"], "illegal_action", "{bad}");
    assert_eq!(session.revision(), revision);
    assert_eq!(format!("{:?}", session.game), before);
    apply(
        &mut session,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![eligible],
        },
    );
    resolve_chain(&mut session);
    assert!(session.game.players[0].discard.contains(&53));
    assert!(session.game.players[1].discard.contains(&423));
}

#[test]
fn prefect_api_exposes_public_object_cost_and_exact_event_gear() {
    let mut session = position();
    session.game.players[0].hand = vec![235];
    session.game.players[1]
        .units
        .push(unit(520, 424, None, false));
    apply(
        &mut session,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        },
    );
    let gear = session.game.players[0].gear[0].id;
    let before = format!("{:?}", session.game);
    for viewer in 0..2 {
        let observed = session.observe(viewer).unwrap();
        let choice = &observed["decision"];
        assert_eq!(choice["type"], "trigger_object_cost");
        assert_eq!(choice["cost"]["type"], "banish_source");
        assert_eq!(
            choice["cost"]["source"],
            json!({"type":"unit","player":1,"unit_id":520})
        );
        assert_eq!(
            choice["context"]["subject"],
            json!({"type":"gear","player":0,"gear_id":gear})
        );
        assert_eq!(choice["can_accept"], true);
        assert_eq!(choice["paid_before_responses"], true);
    }
    assert_eq!(format!("{:?}", session.game), before);
    let mut missing = session.clone();
    missing.game.players[1].units.clear();
    assert_eq!(missing.observe(0).unwrap()["decision"]["can_accept"], false);
    apply(&mut session, Action::Choose { accept: true });
    assert_eq!(session.game.players[1].banish, vec![424]);
    assert_eq!(session.game.players[0].gear.len(), 1);
    resolve_chain(&mut session);
    assert_eq!(session.game.players[0].banish, vec![235]);
}

#[test]
fn hunker_api_filters_friendly_targets_reports_public_lock_and_rolls_back_bad_moves() {
    let mut session = position();
    session.game.players[0].hand = vec![425];
    session.game.players[0].units = vec![unit(500, 73, None, false)];
    session.game.players[1].units = vec![unit(501, 73, None, false)];
    let revision = session.revision();
    let before = format!("{:?}", session.game);
    let draft = Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    };
    let target = Target::Unit {
        player: 0,
        unit_id: 500,
    };
    assert_eq!(
        session
            .target_candidates(0, revision, draft, 0)
            .unwrap()
            .candidates,
        vec![target]
    );
    assert_eq!(session.revision(), revision);
    assert_eq!(format!("{:?}", session.game), before);
    apply(
        &mut session,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        },
    );
    resolve_chain(&mut session);
    for viewer in 0..2 {
        let observation = session.observe(viewer).unwrap();
        let friendly = &observation["players"][0]["units"][0];
        assert_eq!(friendly["ready"], true);
        assert_eq!(friendly["movement_locked"], true);
        assert_eq!(
            friendly["movement_restriction"],
            json!({"scope":"any_player","expires":"end_of_turn"})
        );
        assert!(
            observation["players"][1]["units"][0]
                .get("movement_restriction")
                .is_none()
        );
    }
    let revision = session.revision();
    let before = format!("{:?}", session.game);
    let move_action = Action::Move {
        unit_id: 500,
        lane: Some(0),
    };
    for command in ["validate", "apply"] {
        let response=session.handle(json!({"command":command,"player":0,"revision":revision,"action":action_to_json(&move_action)}));
        assert_eq!(response["ok"], false, "{response}");
        assert_eq!(response["error"]["code"], "illegal_action", "{response}");
        assert_eq!(session.revision(), revision);
        assert_eq!(format!("{:?}", session.game), before);
    }
    apply(&mut session, Action::EndTurn);
    assert!(
        session.observe(1).unwrap()["players"][0]["units"][0]
            .get("movement_restriction")
            .is_none()
    );
}

#[test]
fn show_off_protocol_exposes_only_selected_reveal_and_pins_it_through_payment() {
    use crate::show_off::{ShowOffObject, ShowOffSelection};
    let mut s = position();
    s.game.players[0].hand = vec![426, 58, 10, 73];
    s.game.players[0].units = vec![unit(500, 73, None, false)];
    s.game.players[0]
        .set_rune_setup([0, 3, 0, 0, 0, 0], [0, 3, 0, 0, 0, 0])
        .unwrap();
    let rune = s.game.players[0].rune_objects()[0].id;
    let wire = json!({"type":"with_play_choices","action":{"type":"play_card_targets","hand_index":0,
        "lane":null,"targets":[{"type":"unit","player":0,"unit_id":500},{"type":"rune","player":0,"rune_id":rune}]},
        "choices":{"show_off":[{"instance":0,"object":{"type":"hand","hand_index":1}}]}});
    let action = parse_action(&wire).unwrap();
    if let Action::WithPlayChoices { choices, .. } = &action {
        assert_eq!(
            choices.show_off,
            vec![ShowOffSelection {
                instance: 0,
                object: ShowOffObject::Hand { hand_index: 1 }
            }]
        );
    } else {
        panic!("missing explicit choices");
    }
    assert_eq!(parse_action(&action_to_json(&action)).unwrap(), action);
    let before = format!("{:?}", s.game);
    for command in [
        "validate",
        "play_choices",
        "cost_plan",
        "quote_payment",
        "target_candidates",
    ] {
        let mut request =
            json!({"command":command,"player":0,"revision":s.revision(),"action":wire});
        if command == "target_candidates" {
            request["execution"] = json!(0);
        }
        let response = s.handle(request);
        assert_eq!(response["ok"], true, "{command}: {response}");
        assert_eq!(format!("{:?}", s.game), before);
    }
    let opponent = s
        .handle(json!({"command":"play_choices","player":1,"revision":s.revision(),"action":wire}));
    assert_eq!(opponent["ok"], false);
    apply(
        &mut s,
        Action::BeginPayment {
            action: Box::new(action),
        },
    );
    for viewer in 0..2 {
        let obs = s.observe(viewer).unwrap();
        assert_eq!(
            obs["payment_plan"]["show_off"],
            json!([{"instance":0,"source":{"type":"hand"},"card_id":58,"token":false}])
        );
        if viewer == 1 {
            assert!(obs["payment_plan"]["action"].is_null());
            assert!(obs["players"][0]["hand"].is_null());
        }
    }
    apply(&mut s, Action::CompletePayment);
    for viewer in 0..2 {
        let obs = s.observe(viewer).unwrap();
        assert_eq!(obs["chain"][0]["show_off"][0]["card_id"], 58);
        assert!(
            obs["chain"][0]["show_off"][0]["source"]
                .get("hand_index")
                .is_none()
        );
    }
    resolve_chain(&mut s);
    assert!(s.game.players[0].rune_objects()[0].ready);
}

#[test]
fn malformed_show_off_wire_cannot_mutate_the_session() {
    let mut s = position();
    s.game.players[0].hand = vec![426, 58];
    s.game.players[0].units = vec![unit(500, 73, None, false)];
    for object in [
        json!({"type":"hand","hand_index":-1}),
        json!({"type":"gear","gear_id":4294967296u64}),
        json!({"type":"hand","hand_index":1,"player":1}),
        json!({"type":"unit","unit_id":500}),
    ] {
        let before = format!("{:?}", s.game);
        let rev = s.revision();
        let response=s.handle(json!({"command":"apply","player":0,"revision":rev,"action":{
            "type":"with_play_choices","action":{"type":"play_card_targets","hand_index":0,"targets":[{"type":"unit","player":0,"unit_id":500}]},
            "choices":{"show_off":[{"instance":0,"object":object}]}}}));
        assert_eq!(response["ok"], false);
        assert_eq!(s.revision(), rev);
        assert_eq!(format!("{:?}", s.game), before);
    }
}
