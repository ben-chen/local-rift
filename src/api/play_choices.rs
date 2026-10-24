//! Canonical play choices and cost-plan views for local automated opponents.
use super::*;
use crate::play::{
    CostPlanView, CostScope, CostSelection, CostValue, ExecutionChoices, PlayChoiceView,
    PlayChoices, PlayGrantKind, PlayOrigin, PowerReduction, RepeatSelection,
};

impl Session {
    /// Preview the acting player's optional costs without committing any choice.
    pub fn play_choices(
        &self,
        player: usize,
        revision: u64,
        action: Action,
    ) -> Result<PlayChoiceView, ApiError> {
        self.require_revision(revision)?;
        self.require_actor(player)?;
        let action = parse_action(&action_to_json(&action))?;
        match catch_unwind(AssertUnwindSafe(|| self.game.play_choices(&action))) {
            Ok(result) => result.map_err(ApiError::illegal),
            Err(_) => Err(ApiError::new(
                "engine_error",
                "Could not preview these play choices.",
            )),
        }
    }

    /// Calculate the same typed cost plan used by direct and staged payments.
    pub fn cost_plan(
        &self,
        player: usize,
        revision: u64,
        action: Action,
    ) -> Result<CostPlanView, ApiError> {
        self.require_revision(revision)?;
        self.require_actor(player)?;
        let action = parse_action(&action_to_json(&action))?;
        match catch_unwind(AssertUnwindSafe(|| self.game.cost_plan(&action))) {
            Ok(result) => result.map_err(ApiError::illegal),
            Err(_) => Err(ApiError::new(
                "engine_error",
                "Could not calculate this cost plan.",
            )),
        }
    }
}

pub(super) fn origin_to_json(origin: PlayOrigin) -> Value {
    match origin {
        PlayOrigin::Hand => json!({"type":"hand"}),
        PlayOrigin::Champion => json!({"type":"champion"}),
        PlayOrigin::Trash => json!({"type":"trash"}),
        PlayOrigin::Hidden { lane } => json!({"type":"hidden","lane":lane}),
        PlayOrigin::Token => json!({"type":"token"}),
    }
}

pub(super) fn execution_to_json(choices: &ExecutionChoices) -> Value {
    json!({"targets":choices.targets.iter().map(target_to_json).collect::<Vec<_>>(),"mode":choices.mode})
}

fn selection_to_json(selection: &CostSelection) -> Value {
    json!({"modifier_id":selection.modifier_id.to_string(),"use_power":selection.use_power,
        "power_allocations":selection.power_allocations.iter().map(|allocation|json!({
            "component_id":allocation.component_id.to_string(),"amount":allocation.amount})).collect::<Vec<_>>()})
}

pub(super) fn choices_to_json(choices: &PlayChoices) -> Value {
    json!({"repeats":choices.repeats.iter().map(|repeat|json!({"option_id":repeat.option_id.to_string(),
        "choices":execution_to_json(&repeat.choices)})).collect::<Vec<_>>(),
        "accelerate":choices.accelerate,"bonus":choices.bonus,
        "cost_order":choices.cost_order.iter().map(selection_to_json).collect::<Vec<_>>()})
}

fn cost_to_json(cost: &CostValue) -> Value {
    json!({"energy":cost.energy,"power":cost.power,"xp":cost.xp,"xp_exact":cost.xp.to_string(),
        "domains":cost.domains.iter().map(|d|d.name().to_lowercase()).collect::<Vec<_>>()})
}

pub(super) fn plan_to_json(plan: &CostPlanView) -> Value {
    json!({"components":plan.components.iter().map(|component|json!({"id":component.id.to_string(),
        "label":component.label,"cost":cost_to_json(&component.cost),"optional":component.optional})).collect::<Vec<_>>(),
        "modifiers":plan.modifiers.iter().map(|modifier|json!({"id":modifier.id.to_string(),"label":modifier.label,
            "source":chain_source_to_json(modifier.source),"energy":modifier.energy,"energy_exact":modifier.energy.to_string(),
            "power":modifier.power,"minimum_energy":modifier.minimum_energy,
            "choose_energy_or_power":modifier.choose_energy_or_power,"increase":modifier.increase,
            "scope":match &modifier.scope {CostScope::Total=>json!({"type":"total"}),
                CostScope::Component{component_id}=>json!({"type":"component","component_id":component_id.to_string()})}
        })).collect::<Vec<_>>(),
        "energy":plan.energy,"xp":plan.xp,"xp_exact":plan.xp.to_string(),"power":plan.power.iter().map(|cost|json!({"amount":cost.amount,
            "domains":cost.domains.iter().map(|d|d.name().to_lowercase()).collect::<Vec<_>>(),
            "spell_eligible":cost.spell_eligible})).collect::<Vec<_>>(),
        "applied_order":plan.applied_order.iter().map(selection_to_json).collect::<Vec<_>>()})
}

pub(super) fn view_to_json(view: &PlayChoiceView) -> Value {
    json!({"card_id":view.card_id,"origin":origin_to_json(view.origin),
        "repeats":view.repeats.iter().map(|option|json!({"id":option.id.to_string(),"label":option.label,
            "source":chain_source_to_json(option.source),"cost":cost_to_json(&option.cost)})).collect::<Vec<_>>(),
        "accelerate":view.accelerate.as_ref().map(|option|json!({"label":option.label,"cost":cost_to_json(&option.cost)})),
        "bonus":view.bonus.as_ref().map(|option|json!({"label":option.label,"cost":cost_to_json(&option.cost)})),
        "costs":plan_to_json(&view.costs)})
}

pub(super) fn grant_kind_to_json(kind: PlayGrantKind) -> Value {
    match kind {
        PlayGrantKind::ReadyNextUnit => json!({"type":"ready_next_unit"}),
        PlayGrantKind::AccelerateNextUnit => json!({"type":"accelerate_next_unit"}),
        PlayGrantKind::BonusNextSpell(amount) => json!({"type":"bonus_next_spell","amount":amount}),
        PlayGrantKind::RepeatNextSpell => json!({"type":"repeat_next_spell"}),
        PlayGrantKind::DiscountNextSpell { energy, power } => {
            json!({"type":"discount_next_spell","energy":energy,"power":power})
        }
        PlayGrantKind::DiscountNextCard { energy, power } => {
            json!({"type":"discount_next_card","energy":energy,"power":power})
        }
        PlayGrantKind::ReadyUnitsThisTurn => json!({"type":"ready_units_this_turn"}),
        PlayGrantKind::ReadyBuffNextUnit => json!({"type":"ready_buff_next_unit"}),
        PlayGrantKind::UnitSurchargeThisTurn(energy) => {
            json!({"type":"unit_surcharge_this_turn","energy":energy})
        }
    }
}

// IDs are strings on the wire so JavaScript clients never round a u64.
// Numeric IDs remain accepted for clients whose JSON number type is exact.
fn opaque_id(obj: &Map<String, Value>, key: &str) -> Result<u64, ApiError> {
    let value = required(obj, key)?;
    if let Some(id) = value.as_u64() {
        return Ok(id);
    }
    if let Some(text) = value.as_str()
        && !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && let Ok(id) = text.parse::<u64>()
    {
        return Ok(id);
    }
    Err(ApiError::invalid(format!(
        "{key} must be an unsigned 64-bit decimal ID."
    )))
}

fn optional_array<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<&'a [Value], ApiError> {
    if obj.contains_key(key) {
        Ok(array(obj, key)?.as_slice())
    } else {
        Ok(&[])
    }
}

fn parse_execution(value: &Value) -> Result<ExecutionChoices, ApiError> {
    let obj = object(value)?;
    keys(obj, &["targets", "mode"])?;
    let mode = if obj.get("mode").is_none_or(Value::is_null) {
        None
    } else {
        Some(index(obj, "mode")?)
    };
    Ok(ExecutionChoices {
        targets: targets(obj, "targets")?,
        mode,
    })
}

pub(super) fn parse_choices(value: &Value) -> Result<PlayChoices, ApiError> {
    let obj = object(value)?;
    keys(obj, &["repeats", "accelerate", "bonus", "cost_order"])?;
    let repeats = optional_array(obj, "repeats")?
        .iter()
        .map(|value| {
            let obj = object(value)?;
            keys(obj, &["option_id", "choices"])?;
            Ok(RepeatSelection {
                option_id: opaque_id(obj, "option_id")?,
                choices: parse_execution(required(obj, "choices")?)?,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    let cost_order = optional_array(obj, "cost_order")?
        .iter()
        .map(|value| {
            let obj = object(value)?;
            keys(obj, &["modifier_id", "use_power", "power_allocations"])?;
            let power_allocations = optional_array(obj, "power_allocations")?
                .iter()
                .map(|value| {
                    let obj = object(value)?;
                    keys(obj, &["component_id", "amount"])?;
                    Ok(PowerReduction {
                        component_id: opaque_id(obj, "component_id")?,
                        amount: u16::try_from(unsigned(obj, "amount")?)
                            .map_err(|_| ApiError::invalid("Power reduction exceeds 65535."))?,
                    })
                })
                .collect::<Result<Vec<_>, ApiError>>()?;
            Ok(CostSelection {
                modifier_id: opaque_id(obj, "modifier_id")?,
                use_power: flag(obj, "use_power")?,
                power_allocations,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Ok(PlayChoices {
        repeats,
        accelerate: flag(obj, "accelerate")?,
        bonus: flag(obj, "bonus")?,
        cost_order,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_action() -> Action {
        Action::WithPlayChoices {
            action: Box::new(Action::PlayCardWithTargets {
                hand_index: 2,
                lane: None,
                targets: vec![Target::Unit {
                    player: 1,
                    unit_id: 41,
                }],
            }),
            choices: PlayChoices {
                repeats: vec![
                    RepeatSelection {
                        option_id: 8_000_000_003,
                        choices: ExecutionChoices {
                            targets: vec![Target::Unit {
                                player: 0,
                                unit_id: 19,
                            }],
                            mode: None,
                        },
                    },
                    RepeatSelection {
                        option_id: 8_000_000_001,
                        choices: ExecutionChoices {
                            targets: vec![Target::Unit {
                                player: 1,
                                unit_id: 42,
                            }],
                            mode: None,
                        },
                    },
                ],
                accelerate: false,
                bonus: false,
                cost_order: vec![
                    CostSelection {
                        modifier_id: 6_000_000_009,
                        use_power: true,
                        power_allocations: vec![PowerReduction {
                            component_id: 5,
                            amount: 1,
                        }],
                    },
                    CostSelection {
                        modifier_id: 6_000_000_008,
                        use_power: false,
                        power_allocations: vec![],
                    },
                ],
            },
        }
    }

    #[test]
    fn canonical_wire_choices_preserve_repetition_order_targets_and_power_allocation() {
        let action = canonical_action();
        let wire = action_to_json(&action);
        assert_eq!(wire["choices"]["repeats"][0]["option_id"], "8000000003");
        assert_eq!(
            wire["choices"]["repeats"][1]["choices"]["targets"][0]["unit_id"],
            42
        );
        assert_eq!(
            wire["choices"]["cost_order"][0]["power_allocations"][0]["component_id"],
            "5"
        );
        assert_eq!(parse_action(&wire).unwrap(), action);
        for wrapped in [
            Action::BeginPayment {
                action: Box::new(action.clone()),
            },
            Action::WithPowerPayment {
                action: Box::new(action),
                payment: PowerPayment::default(),
            },
            Action::FinalizeTokenPlay {
                lane: Some(1),
                accelerate: true,
            },
        ] {
            assert_eq!(parse_action(&action_to_json(&wrapped)).unwrap(), wrapped);
        }
    }

    #[test]
    fn opaque_cost_ids_preserve_all_u64_bits_for_number_and_string_clients() {
        let mut action = canonical_action();
        let Action::WithPlayChoices { choices, .. } = &mut action else {
            unreachable!()
        };
        choices.repeats[0].option_id = u64::MAX;
        choices.cost_order[0].modifier_id = (1 << 63) + 1;
        choices.cost_order[0].power_allocations[0].component_id = (1 << 53) + 1;
        let wire = action_to_json(&action);
        assert_eq!(
            wire["choices"]["repeats"][0]["option_id"],
            "18446744073709551615"
        );
        assert_eq!(
            wire["choices"]["cost_order"][0]["modifier_id"],
            "9223372036854775809"
        );
        assert_eq!(
            wire["choices"]["cost_order"][0]["power_allocations"][0]["component_id"],
            "9007199254740993"
        );
        assert_eq!(parse_action(&wire).unwrap(), action);
        let mut numeric = wire.clone();
        numeric["choices"]["repeats"][0]["option_id"] = json!(u64::MAX);
        numeric["choices"]["cost_order"][0]["modifier_id"] = json!((1u64 << 63) + 1);
        numeric["choices"]["cost_order"][0]["power_allocations"][0]["component_id"] =
            json!((1u64 << 53) + 1);
        assert_eq!(parse_action(&numeric).unwrap(), action);
        for invalid in ["", "-1", "+1", "1.5", " 1", "18446744073709551616"] {
            let mut bad = wire.clone();
            bad["choices"]["cost_order"][0]["modifier_id"] = json!(invalid);
            assert!(parse_action(&bad).is_err(), "accepted invalid ID {invalid}");
        }
    }

    #[test]
    fn malformed_canonical_wire_choices_fail_before_mutating_the_session() {
        let valid = action_to_json(&canonical_action());
        let mut malformed = Vec::new();
        let mut value = valid.clone();
        value["choices"]["repeats"][0]["option_id"] = json!(-1);
        malformed.push(value);
        let mut value = valid.clone();
        value["choices"]["repeats"][0]["choices"]["unexpected"] = json!(true);
        malformed.push(value);
        let mut value = valid.clone();
        value["choices"]["cost_order"][0]["power_allocations"][0]["amount"] = json!(65536);
        malformed.push(value);
        let mut value = valid.clone();
        value["choices"]["accelerate"] = json!("false");
        malformed.push(value);
        let mut value = valid;
        value["choices"]["cost_order"][0]["power_allocations"] =
            json!(vec![json!({"component_id":0,"amount":0}); 1025]);
        malformed.push(value);
        malformed.push(json!({"type":"finalize_token_play","lane":2,"accelerate":false}));
        let mut nested = json!({"type":"end_turn"});
        for _ in 0..MAX_ACTION_DEPTH {
            nested = json!({"type":"with_play_choices","action":nested,"choices":{}});
        }
        malformed.push(nested);

        let mut session = Session::default();
        session.start(0).unwrap();
        let before = session.observe(0).unwrap();
        for action in malformed {
            assert!(
                parse_action(&action).is_err(),
                "accepted malformed choices: {action}"
            );
            let response = session.handle(json!({"command":"apply","player":0,
                "revision":session.revision(),"action":action}));
            assert_eq!(response["ok"], false);
            assert_eq!(response["error"]["code"], "invalid_request");
            assert_eq!(session.observe(0).unwrap(), before);
        }
    }

    #[test]
    fn choice_and_cost_previews_are_read_only_and_require_the_acting_seat_and_revision() {
        let mut session = Session::default();
        session.start(0).unwrap();
        session.game.players[0].hand = vec![25]; // Single ordinary unit, no chosen targets.
        let action = json!({"type":"play_card_targets","hand_index":0,"lane":null,"targets":[]});
        let before = session.observe(0).unwrap();
        for command in ["play_choices", "cost_plan"] {
            let response = session.handle(json!({"command":command,"player":0,
                "revision":session.revision(),"action":action}));
            assert_eq!(response["ok"], true, "{response}");
            assert_eq!(session.observe(0).unwrap(), before);
            let wrong_actor = session.handle(json!({"command":command,"player":1,
                "revision":session.revision(),"action":action}));
            assert_eq!(wrong_actor["error"]["code"], "wrong_player");
            let stale =
                session.handle(json!({"command":command,"player":0,"revision":0,"action":action}));
            assert_eq!(stale["error"]["code"], "stale_revision");
            assert_eq!(session.observe(0).unwrap(), before);
        }
    }

    fn repeated_spell_fixture(card_id: usize) -> Session {
        let mut session = Session::default();
        session.start(0).unwrap();
        for player in &mut session.game.players {
            player.hand.clear();
            player.champion = None;
            player.units.clear();
            player.gear.clear();
            player.discard.clear();
            player.banish.clear();
            player.energy_pool = 50;
            player.energy = 50;
            player.power_pool = [20; 6];
        }
        session.game.players[0].hand = vec![card_id];
        session.game.players[0].deck = vec![25, 27, 69, 25, 27, 69, 25, 27, 69];
        // Explicit starting position: two previously resolved Portal activations.
        for id in [901, 902] {
            session.game.play_grants.push(crate::play::PlayGrant {
                id,
                player: 0,
                source: crate::game::ChainSource::Card { card_id: 256 },
                expires_turn: session.game.turn,
                kind: PlayGrantKind::RepeatNextSpell,
            });
        }
        session
    }

    fn repeat_twice(session: &Session) -> Action {
        let action = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        };
        let view = session
            .play_choices(0, session.revision(), action.clone())
            .unwrap();
        assert_eq!(view.repeats.len(), 2);
        Action::WithPlayChoices {
            action: Box::new(action),
            choices: PlayChoices {
                repeats: view
                    .repeats
                    .iter()
                    .map(|option| RepeatSelection {
                        option_id: option.id,
                        choices: ExecutionChoices::default(),
                    })
                    .collect(),
                ..Default::default()
            },
        }
    }

    fn apply_wire(session: &mut Session, action: Action) {
        let response = session.handle(json!({"command":"apply", "player":session.game.priority(),
            "revision":session.revision(), "action":action_to_json(&action)}));
        assert_eq!(response["ok"], true, "{response}");
    }

    fn advance_to_choice(session: &mut Session) {
        for _ in 0..20 {
            if session.game.decision.is_some() || !session.game.in_window() {
                return;
            }
            apply_wire(session, Action::Pass);
        }
        panic!("Spell failed to reach a decision or finish");
    }

    #[test]
    fn repeated_look_resumes_each_private_choice_and_keeps_one_physical_spell() {
        let mut session = repeated_spell_fixture(39); // Lightning Rush.
        let action = repeat_twice(&session);
        apply_wire(&mut session, action);
        let chain_id = session.game.chain.last().unwrap().id;
        for execution in 0..3 {
            advance_to_choice(&mut session);
            let own = session.observe(0).unwrap();
            let other = session.observe(1).unwrap();
            assert_eq!(own["decision"]["type"], "top_three");
            assert_eq!(own["decision"]["cards"], json!([69, 27, 25]));
            assert!(other["decision"].get("cards").is_none());
            assert!(other["players"][0]["hand"].is_null());
            assert_eq!(
                own["resolution_progress"],
                json!({"chain_id":chain_id,
                "execution":execution,"executions":3})
            );
            assert_eq!(own["chain"].as_array().unwrap().len(), 1);
            assert_eq!(own["chain"][0]["executions"].as_array().unwrap().len(), 3);
            assert_eq!(own["players"][0]["cards_completed_this_turn"], 0);
            assert!(!session.game.players[0].discard.contains(&39));
            let player = &session.game.players[0];
            assert_eq!(
                player.hand.len() + player.deck.len() + player.discard.len() + 1,
                10
            );
            apply_wire(&mut session, Action::SelectCards { indices: vec![0] });
        }
        advance_to_choice(&mut session);
        let view = session.observe(0).unwrap();
        assert!(view["resolution_progress"].is_null());
        assert!(view["chain"].as_array().unwrap().is_empty());
        assert_eq!(view["players"][0]["hand"], json!([69, 69, 69]));
        assert_eq!(view["players"][0]["cards_played_this_turn"], 1);
        assert_eq!(view["players"][0]["cards_completed_this_turn"], 1);
        assert_eq!(session.game.players[0].discard.len(), 7);
        assert_eq!(
            session.game.players[0]
                .discard
                .iter()
                .filter(|&&id| id == 39)
                .count(),
            1
        );
    }

    #[test]
    fn repeated_loot_finishes_each_discard_and_draw_before_the_next_execution() {
        let mut session = repeated_spell_fixture(84); // Lunar Boon.
        session.game.players[0].hand.push(25);
        session.game.players[0].deck = vec![69, 69, 27, 27, 25, 25];
        let initial_draws = session.game.trigger_state.draws[0];
        let action = repeat_twice(&session);
        apply_wire(&mut session, action);
        for (execution, expected_hand) in [vec![25], vec![25, 25], vec![25, 27, 27]]
            .iter()
            .enumerate()
        {
            advance_to_choice(&mut session);
            let view = session.observe(0).unwrap();
            assert_eq!(view["decision"]["type"], "discard");
            assert_eq!(view["players"][0]["hand"], json!(expected_hand));
            assert_eq!(view["resolution_progress"]["execution"], execution);
            assert_eq!(view["players"][0]["cards_completed_this_turn"], 0);
            assert!(!session.game.players[0].discard.contains(&84));
            apply_wire(&mut session, Action::SelectCards { indices: vec![0] });
        }
        advance_to_choice(&mut session);
        let view = session.observe(0).unwrap();
        assert_eq!(view["players"][0]["hand"], json!([27, 27, 69, 69]));
        assert_eq!(
            view["players"][0]["cards_drawn_this_turn"],
            initial_draws + 6
        );
        assert_eq!(view["players"][0]["cards_completed_this_turn"], 1);
        assert!(view["chain"].as_array().unwrap().is_empty());
        assert_eq!(session.game.players[0].discard, vec![25, 25, 25, 84]);
    }

    #[test]
    fn impossible_repeat_choices_reject_atomically_in_preview_validation_and_payment() {
        let session = repeated_spell_fixture(39);
        let valid = repeat_twice(&session);
        let Action::WithPlayChoices { action, choices } = valid else {
            unreachable!()
        };
        let mut duplicate = choices.clone();
        duplicate.repeats[1].option_id = duplicate.repeats[0].option_id;
        let mut absent = choices.clone();
        absent.repeats[0].option_id = u64::MAX;
        let mut mode = choices;
        mode.repeats[0].choices.mode = Some(0);
        for choices in [duplicate, absent, mode] {
            for command in [
                "play_choices",
                "cost_plan",
                "quote_payment",
                "validate",
                "apply",
            ] {
                let mut trial = session.clone();
                let before = trial.observe(0).unwrap();
                let action = Action::WithPlayChoices {
                    action: action.clone(),
                    choices: choices.clone(),
                };
                let response = trial.handle(json!({"command":command,"player":0,
                    "revision":trial.revision(),"action":action_to_json(&action)}));
                assert_eq!(
                    response["ok"], false,
                    "{command} accepted {choices:?}: {response}"
                );
                assert_eq!(trial.observe(0).unwrap(), before);
            }
            let mut trial = session.clone();
            let before = trial.observe(0).unwrap();
            let wrapped = Action::BeginPayment {
                action: Box::new(Action::WithPlayChoices {
                    action: action.clone(),
                    choices,
                }),
            };
            let result = trial.apply_action(0, trial.revision(), wrapped);
            assert!(result.is_err());
            assert_eq!(trial.observe(0).unwrap(), before);
        }
    }

    #[test]
    fn canonical_choices_cannot_be_nested_or_silently_ignored_on_an_ability() {
        let mut session = repeated_spell_fixture(39);
        session.game.players[0].gear.push(crate::gear::GearState {
            id: 903,
            card_id: 254,
            attached_to: None,
            ready: true,
            temporary: false,
            token: false,
        });
        let ability = Action::ActivateAbility {
            source: crate::activated::AbilitySource::Gear {
                player: 0,
                gear_id: 903,
            },
            ability_index: 0,
            targets: vec![],
        };
        let mut actions = vec![Action::WithPlayChoices {
            action: Box::new(repeat_twice(&session)),
            choices: PlayChoices::default(),
        }];
        for choices in [
            PlayChoices {
                accelerate: true,
                ..Default::default()
            },
            PlayChoices {
                bonus: true,
                ..Default::default()
            },
            PlayChoices {
                repeats: vec![RepeatSelection {
                    option_id: 1,
                    choices: ExecutionChoices::default(),
                }],
                ..Default::default()
            },
        ] {
            actions.push(Action::WithPlayChoices {
                action: Box::new(ability.clone()),
                choices,
            });
        }
        for action in actions {
            for command in [
                "play_choices",
                "cost_plan",
                "quote_payment",
                "validate",
                "apply",
            ] {
                let mut trial = session.clone();
                let before = trial.observe(0).unwrap();
                let response = trial.handle(json!({"command":command,"player":0,
                    "revision":trial.revision(),"action":action_to_json(&action)}));
                assert_eq!(
                    response["ok"], false,
                    "{command} accepted {action:?}: {response}"
                );
                assert_eq!(trial.observe(0).unwrap(), before);
            }
        }
    }
}
