use super::*;
use crate::game::Unit;
use crate::gear::GearState;

fn position() -> Session {
    let mut session = Session::new(ResetOptions {
        decks: [3, 3],
        ..ResetOptions::default()
    })
    .unwrap();
    session.start(0).unwrap();
    session.game.battlefields = [3, 3];
    for player in &mut session.game.players {
        player.units.clear();
        player.gear.clear();
        player.hand.clear();
        player.runes = [0; 6];
        player.exhausted_runes = [0; 6];
        player.energy_pool = 50;
        player.energy = 50;
        player.power_pool = [20; 6];
    }
    session
}

fn unit(id: u32, card_id: usize) -> Unit {
    Unit {
        id,
        card_id,
        lane: None,
        ready: true,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
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

fn mage_predict() -> Session {
    let mut session = position();
    session.game.players[0].units.push(unit(900, 283));
    session.game.players[0].deck = vec![0, 1, 8, 6];
    apply(
        &mut session,
        Action::ActivateAbility {
            source: AbilitySource::Unit {
                player: 0,
                unit_id: 900,
            },
            ability_index: 0,
            targets: vec![],
        },
    );
    for _ in 0..20 {
        if matches!(session.game.decision, Some(Decision::PredictCards { .. })) {
            return session;
        }
        assert!(
            session.game.decision.is_none(),
            "{:?}",
            session.game.decision
        );
        apply(&mut session, Action::Pass);
    }
    panic!("Apprentice's real Empower trigger did not reach Predict");
}

#[test]
fn predict_wire_preserves_occurrence_indices_and_rejects_malformed_input() {
    let action = json!({"type":"resolve_predict","choice_id":23,"recycle":[1],"top_order":[0]});
    assert_eq!(action_to_json(&parse_action(&action).unwrap()), action);
    for invalid in [
        json!({"type":"resolve_predict","recycle":[],"top_order":[0,1]}),
        json!({"type":"resolve_predict","choice_id":23,"recycle":[],"top_order":[-1]}),
        json!({"type":"resolve_predict","choice_id":23,"recycle":[],"top_order":[0.5]}),
        json!({"type":"resolve_predict","choice_id":23,"recycle":[],"top_order":[],"order":[0]}),
        json!({"type":"resolve_predict","choice_id":4294967296u64,"recycle":[],"top_order":[]}),
        json!({"type":"resolve_predict","choice_id":23,"recycle":vec![0;MAX_ACTION_ITEMS+1],"top_order":[]}),
    ] {
        assert!(parse_action(&invalid).is_err(), "{invalid}");
    }
}

#[test]
fn predict_view_hides_cards_and_order_from_the_other_seat() {
    let first = mage_predict();
    let mut second = first.clone();
    let choice_id = match first.game.decision.as_ref().unwrap() {
        Decision::PredictCards { choice_id, .. } => *choice_id,
        _ => unreachable!(),
    };
    second.game.players[0].deck = vec![4, 5, 9, 11];
    second.game.decision = Some(Decision::PredictCards {
        player: 0,
        choice_id,
        cards: vec![11, 9],
    });
    assert_eq!(first.observe(1).unwrap(), second.observe(1).unwrap());
    let own = first.observe(0).unwrap();
    assert_eq!(own["decision"]["cards"], json!([6, 8]));
    assert_eq!(own["decision"]["count"], 2);
    assert_eq!(
        own["players"][0]["deck_count"], 4,
        "looking does not move cards"
    );
    let enemy = first.observe(1).unwrap();
    assert_eq!(
        enemy["decision"],
        json!({"type":"predict_cards","player":0,"choice_id":choice_id,"count":2})
    );
    assert!(first.suggestions(1).unwrap().is_empty());
}

#[test]
fn predict_invalid_partitions_and_stale_choices_are_atomic_then_valid_order_draws_next() {
    let mut session = mage_predict();
    let choice_id = session.observe(0).unwrap()["decision"]["choice_id"]
        .as_u64()
        .unwrap() as u32;
    let before = [session.observe(0).unwrap(), session.observe(1).unwrap()];
    for (id, recycle, top_order) in [
        (choice_id, vec![0], vec![0, 1]),
        (choice_id, vec![], vec![0]),
        (choice_id, vec![2], vec![0, 1]),
        (choice_id, vec![1, 1], vec![0]),
        (choice_id + 1, vec![], vec![1, 0]),
    ] {
        let action = Action::ResolvePredict {
            choice_id: id,
            recycle,
            top_order,
        };
        assert!(session.apply_action(0, session.revision(), action).is_err());
        assert_eq!(
            [session.observe(0).unwrap(), session.observe(1).unwrap()],
            before
        );
    }
    let action = Action::ResolvePredict {
        choice_id,
        recycle: vec![],
        top_order: vec![1, 0],
    };
    session
        .validate_action(0, session.revision(), action.clone())
        .unwrap();
    assert_eq!(
        [session.observe(0).unwrap(), session.observe(1).unwrap()],
        before
    );
    apply(&mut session, action);
    assert_eq!(session.game.players[0].deck, vec![0, 1, 6, 8]);
    assert_eq!(
        session.revision(),
        before[0]["revision"].as_u64().unwrap() + 1
    );
}

#[test]
fn public_passive_sources_distinguish_copies_and_do_not_change_printed_might() {
    let mut session = position();
    session.game.players[0].units.push(unit(900, 274));
    for id in [901, 902] {
        session.game.players[0].gear.push(GearState {
            id,
            card_id: 281,
            attached_to: None,
            ready: true,
            temporary: false,
            token: false,
        });
    }
    session.game.empowered_units.push(902);
    let printed = session.game.catalog[274].might;
    for viewer in 0..2 {
        let view = session.observe(viewer).unwrap();
        let observed = &view["players"][0]["units"][0];
        assert_eq!(observed["might"], printed + 3);
        assert_eq!(observed["keywords"]["deflect"], 0);
        assert_eq!(observed["empowered"], false);
        let effects = observed["passive_contributions"].as_array().unwrap();
        for (gear_id, amount) in [(901, 1), (902, 2)] {
            assert!(effects.iter().any(|e| e["active"] == true
                && e["source"]["gear_id"] == gear_id
                && e["effect"]
                    == json!({"type":"might","amount":amount,"amount_exact":amount.to_string()})));
        }
        assert!(effects.iter().any(|e| e["active"] == false
            && e["source"]["unit_id"] == 900
            && e["effect"]["keyword"] == "deflect"));
    }
    assert_eq!(session.game.catalog[274].might, printed);
}

#[test]
fn catalog_describes_nominal_discounted_costs_and_targeted_legend_activation() {
    let catalog = catalog();
    for id in [278, 279] {
        let ability = &catalog["cards"][id]["activated_abilities"][0];
        assert_eq!(ability["energy"], 12);
        assert_eq!(
            ability["cost_modifiers"],
            json!([{"type":"energy_per_controlled_rune","amount":1}])
        );
    }
    assert_eq!(
        catalog["cards"][280]["activated_abilities"][0]["cost_modifiers"],
        json!([{"type":"energy_if_runes_at_most","maximum":4,"amount":3}])
    );
    let activation = &catalog["legends"][7]["activation"];
    assert_eq!(activation["timing"], "main");
    assert_eq!(activation["power"], 1);
    assert_eq!(activation["power_domain"], Value::Null);
    assert_eq!(activation["disempower"], true);
    assert_eq!(activation["exhaust"], true);
    assert_eq!(activation["targets"]["allegiance"], "any");
    assert_eq!(activation["targets"]["min"], 1);
    assert_eq!(activation["effect"], "ready");
}

#[test]
fn movement_event_context_preserves_base_and_battlefield_without_source_liveness() {
    let context = TriggerContext {
        departure_location: None,
        source: crate::game::TriggerSource {
            player: 0,
            unit_id: Some(900),
            gear_id: None,
        },
        subject: Some(Target::Unit {
            player: 0,
            unit_id: 900,
        }),
        movement: Some(crate::game::triggers::MoveFacts {
            origin: Some(1),
            destination: None,
        }),
    };
    let view = trigger_context_to_json(&context);
    assert_eq!(view["movement"], json!({"origin":1,"destination":null}));
    assert_eq!(view["source"]["unit_id"], 900);
    assert_eq!(
        ability_to_json(&ChainAbility::Legend(LegendAbility::MatriarchEmpower)),
        json!({"type":"matriarch_empower"})
    );
    assert_eq!(
        ability_to_json(&ChainAbility::Legend(LegendAbility::MatriarchReady {
            target: Target::Unit {
                player: 1,
                unit_id: 901
            }
        })),
        json!({"type":"matriarch_ready","target":{"type":"unit","player":1,"unit_id":901}})
    );
}

#[test]
fn matriarch_cost_plan_exact_payment_and_staged_completion_share_enemy_deflect_cost() {
    let mut session = position();
    session.game.legend[0] = crate::legends::LegendState::from_legend(7).unwrap();
    session.game.legend[0].empowered = true;
    let mut target = unit(900, 287);
    target.ready = false;
    session.game.players[1].units.push(target);
    let action = Action::ActivateLegend {
        target: Some(Target::Unit {
            player: 1,
            unit_id: 900,
        }),
    };
    let before = [session.observe(0).unwrap(), session.observe(1).unwrap()];
    let plan = session
        .cost_plan(0, session.revision(), action.clone())
        .unwrap();
    let quote = session
        .quote_payment(0, session.revision(), action.clone())
        .unwrap();
    assert_eq!(plan.energy, 0);
    assert_eq!(plan.power.iter().map(|p| p.amount).sum::<u16>(), 3);
    assert_eq!(quote.costs.iter().map(|p| p.amount).sum::<u16>(), 3);
    assert_eq!(
        [session.observe(0).unwrap(), session.observe(1).unwrap()],
        before
    );
    apply(
        &mut session,
        Action::BeginPayment {
            action: Box::new(action),
        },
    );
    let pending = session.observe(0).unwrap();
    assert_eq!(pending["decision"]["type"], "payment");
    assert!(session.game.legend[0].ready && session.game.legend[0].empowered);
    let quote = session
        .quote_payment(0, session.revision(), Action::CompletePayment)
        .unwrap();
    apply(
        &mut session,
        Action::WithPowerPayment {
            action: Box::new(Action::CompletePayment),
            payment: quote.suggested,
        },
    );
    assert!(!session.game.legend[0].ready && !session.game.legend[0].empowered);
    assert!(!session.game.players[1].units[0].ready);
    apply(&mut session, Action::Pass);
    apply(&mut session, Action::Pass);
    assert!(session.game.players[1].units[0].ready);
}
