use super::*;
use crate::game::Unit;
use crate::gear::GearState;
use crate::play::{ExecutionChoices, PlayChoices, RepeatSelection};

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
        player.runes = [0; 6];
        player.exhausted_runes = [0; 6];
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
                        targets: second,
                        mode: None,
                    },
                },
                RepeatSelection {
                    option_id: 8_000_000_002,
                    choices: ExecutionChoices {
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
