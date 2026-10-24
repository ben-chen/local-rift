//! Independent protocol assertions: fixtures establish positions; wire requests
//! perform actual payments, plays, activations and response windows.
use super::*;
use crate::game::Unit;
use crate::gear::GearState;
use crate::payment::PowerPayment;

fn position() -> Session {
    let mut session = Session::new(ResetOptions {
        decks: [1, 1],
        ..Default::default()
    })
    .unwrap();
    session.start(0).unwrap();
    session.game.battlefields = [3, 3];
    session.game.battlefield_owner = [None; 2];
    for player in &mut session.game.players {
        player.hand.clear();
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.discard.clear();
        player.champion = None;
        player.runes = [0; 6];
        player.exhausted_runes = [0; 6];
        player.energy_pool = 0;
        player.energy = 0;
        player.power_pool = [0; 6];
        player.any_power = 0;
        player.xp = 0;
    }
    session.game.legend[0].ready = false;
    session.game.legend[1].ready = false;
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
fn gear(session: &mut Session, id: u32, card_id: usize) -> AbilitySource {
    session.game.players[0].gear.push(GearState {
        id,
        card_id,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    AbilitySource::Gear {
        player: 0,
        gear_id: id,
    }
}
fn activate(source: AbilitySource, ability_index: usize) -> Action {
    Action::ActivateAbility {
        source,
        ability_index,
        targets: vec![],
    }
}
fn views(session: &Session) -> [Value; 2] {
    [session.observe(0).unwrap(), session.observe(1).unwrap()]
}
fn request(session: &mut Session, command: &str, action: &Action) -> Value {
    session.handle(json!({"command":command,"revision":session.revision(),
        "player":session.game.priority(),"action":action_to_json(action)}))
}
fn result(session: &mut Session, command: &str, action: &Action) -> Value {
    let response = request(session, command, action);
    assert_eq!(response["ok"], true, "{command} {action:?}: {response}");
    response["result"].clone()
}
fn apply(session: &mut Session, action: Action) {
    result(session, "apply", &action);
}
fn exact(session: &mut Session, action: Action) {
    let before = views(session);
    let revision = session.revision();
    let quote = result(session, "quote_payment", &action);
    assert!(
        quote.get("xp").is_none(),
        "Power-only quote contract stays unchanged"
    );
    let wrapped = Action::WithPowerPayment {
        action: Box::new(action),
        payment: parse_payment(&quote["suggested"]).unwrap(),
    };
    result(session, "validate", &wrapped);
    assert_eq!(views(session), before);
    assert_eq!(session.revision(), revision);
    apply(session, wrapped);
}
fn rejected(session: &mut Session, action: &Action) {
    let before = views(session);
    let state = format!("{:?}", session.game);
    let revision = session.revision();
    for command in ["validate", "quote_payment", "apply"] {
        let response = request(session, command, action);
        assert_eq!(response["ok"], false, "{command} accepted {action:?}");
        assert_eq!(views(session), before);
        assert_eq!(format!("{:?}", session.game), state);
        assert_eq!(session.revision(), revision);
    }
}
fn settle(session: &mut Session) {
    for _ in 0..60 {
        if !session.game.in_window() {
            return;
        }
        if let Some(Decision::OrderTriggers { options, .. }) = &session.game.decision {
            let trigger_id = options[0].id;
            apply(session, Action::OrderTrigger { trigger_id });
        } else {
            assert!(
                session.game.decision.is_none(),
                "{:?}",
                session.game.decision
            );
            apply(session, Action::Pass);
        }
    }
    panic!("response window did not finish");
}
fn assert_xp(session: &Session, xp: u64, gained: bool) {
    for view in views(session) {
        assert_eq!(view["players"][0]["xp"].as_u64(), Some(xp));
        assert_eq!(view["players"][0]["xp_exact"], xp.to_string());
        assert_eq!(view["players"][0]["xp_gained_this_turn"], gained);
    }
}

#[test]
fn metadata_separates_xp_costs_level_existence_equipment_and_fixed_add_output() {
    let all = catalog();
    for id in [345, 346] {
        let rule = &all["cards"][id]["activated_abilities"][0];
        assert_eq!(rule["xp_cost"], 2);
        assert_eq!(rule["xp_cost_exact"], "2");
        assert_eq!(rule["timing"], "main");
        assert_eq!(rule["exhaust"], false);
        assert_eq!(rule["targets"]["max"], 0);
        assert!(rule["minimum_xp"].is_null());
        assert!(rule["fixed_resource_output"].is_null());
        assert!(all["cards"][id]["equipment"].is_null());
    }
    let honey = all["cards"][347]["activated_abilities"].as_array().unwrap();
    assert_eq!(honey.len(), 2);
    for (index, rule) in honey.iter().enumerate() {
        assert_eq!(rule["xp_cost"], 0);
        assert_eq!(rule["xp_cost_exact"], "0");
        assert_eq!(rule["exhaust"], true);
        assert_eq!(rule["timing"], "reaction");
        assert_eq!(rule["adds_resources"], true);
        assert_eq!(
            rule["minimum_xp"],
            if index == 0 { Value::Null } else { json!(6) }
        );
        assert_eq!(
            rule["fixed_resource_output"],
            json!({"energy":index,"power":1,"domain":null})
        );
    }
    assert_eq!(
        all["cards"][348]["equipment"],
        json!({"might":2,
        "equip_energy":0,"equip_power":0,"equip_domain":null,"equip_xp":1,
        "equip_xp_exact":"1","recycle_trash":0,"quick_draw":false})
    );
    for card in &all["cards"].as_array().unwrap()[..345] {
        if !card["equipment"].is_null() {
            assert_eq!(card["equipment"]["equip_xp"], 0);
            assert_eq!(card["equipment"]["equip_xp_exact"], "0");
        }
        for rule in card["activated_abilities"].as_array().unwrap() {
            assert_eq!(rule["xp_cost"], 0);
            assert!(rule["minimum_xp"].is_null());
        }
    }
    // Egg's conditional output and converters must not be described as fixed.
    assert!(all["cards"][252]["activated_abilities"][1]["fixed_resource_output"].is_null());
    assert!(all["cards"][244]["activated_abilities"][0]["fixed_resource_output"].is_null());
}

#[test]
fn direct_exact_power_and_frozen_xp_payments_debit_once_before_responses() {
    for card in [345, 346] {
        for starting_xp in [2_u64, 256, (1_u64 << 53) + 1, u64::MAX] {
            let mut initial = position();
            initial.game.players[0].xp = starting_xp;
            initial.game.players[0].units.push(unit(700, card));
            initial.game.players[0].units[0].ready = false; // No Exhaust cost.
            let action = activate(
                AbilitySource::Unit {
                    player: 0,
                    unit_id: 700,
                },
                0,
            );
            let before = views(&initial);
            let plan = result(&mut initial, "cost_plan", &action);
            assert_eq!(plan["xp"], 2);
            assert_eq!(plan["xp_exact"], "2");
            assert_eq!(plan["energy"], 0);
            assert_eq!(plan["power"], json!([]));
            assert_eq!(plan["components"][0]["cost"]["xp"], 2);
            assert_eq!(plan["components"][0]["cost"]["xp_exact"], "2");
            assert_eq!(views(&initial), before);
            for method in 0..3 {
                let mut session = initial.clone();
                match method {
                    0 => apply(&mut session, action.clone()),
                    1 => exact(&mut session, action.clone()),
                    _ => {
                        apply(
                            &mut session,
                            Action::BeginPayment {
                                action: Box::new(action.clone()),
                            },
                        );
                        assert_xp(&session, starting_xp, false);
                        for viewer in 0..2 {
                            let view = session.observe(viewer).unwrap();
                            assert_eq!(view["payment_plan"]["xp"], 2);
                            assert_eq!(view["payment_plan"]["xp_exact"], "2");
                            assert_eq!(view["payment_plan"]["action"].is_null(), viewer == 1);
                        }
                        exact(&mut session, Action::CompletePayment);
                    }
                }
                assert_xp(&session, starting_xp - 2, false);
                assert_eq!(session.game.players[0].units[0].buff, 0);
                assert_eq!(session.game.chain.len(), 1);
                assert_eq!(session.game.chain[0].targets, vec![]);
                settle(&mut session);
                assert_xp(&session, starting_xp - 2, false);
                assert_eq!(session.game.players[0].units[0].buff, 1);
            }
        }
    }
}

#[test]
fn missing_xp_quotes_fail_atomically_but_staged_cost_remains_visible_and_cancellable() {
    for starting_xp in [0, 1] {
        let mut session = position();
        session.game.players[0].xp = starting_xp;
        session.game.players[0].units.push(unit(700, 345));
        let action = activate(
            AbilitySource::Unit {
                player: 0,
                unit_id: 700,
            },
            0,
        );
        rejected(&mut session, &action);
        rejected(
            &mut session,
            &Action::WithPowerPayment {
                action: Box::new(action.clone()),
                payment: PowerPayment::default(),
            },
        );
        let plan = result(&mut session, "cost_plan", &action);
        assert_eq!(plan["xp"], 2);
        let original = format!("{:?}", session.game);
        apply(
            &mut session,
            Action::BeginPayment {
                action: Box::new(action),
            },
        );
        assert_xp(&session, starting_xp, false);
        rejected(&mut session, &Action::CompletePayment);
        assert_eq!(session.observe(0).unwrap()["decision"]["can_cancel"], true);
        exact(&mut session, Action::Choose { accept: false });
        assert_eq!(format!("{:?}", session.game), original);
    }
}

#[test]
fn self_buff_spend_preserves_positive_gain_history_and_cannot_choose_targets_or_xp() {
    let mut session = position();
    session.game.players[0].units.push(unit(700, 345));
    session.game.players[0].units.push(unit(701, 342)); // Wily Newtfish.
    session.game.gain_xp(0, 4).unwrap();
    let source = AbilitySource::Unit {
        player: 0,
        unit_id: 700,
    };
    let action = activate(source, 0);
    rejected(
        &mut session,
        &Action::ActivateAbility {
            source,
            ability_index: 0,
            targets: vec![Target::Unit {
                player: 0,
                unit_id: 700,
            }],
        },
    );
    for (field, value) in [
        ("xp", json!(0)),
        ("xp_cost", json!(0)),
        ("xp_exact", json!("0")),
    ] {
        let mut wire = action_to_json(&action);
        wire[field] = value;
        let before = views(&session);
        let revision = session.revision();
        let response =
            session.handle(json!({"command":"apply","revision":revision,"player":0,"action":wire}));
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "invalid_request");
        assert_eq!(session.revision(), revision);
        assert_eq!(views(&session), before);
    }
    for remaining in [2, 0] {
        exact(&mut session, action.clone());
        assert_xp(&session, remaining, true);
        settle(&mut session);
        assert_eq!(
            session.game.players[0].units[0].buff, 1,
            "second Buff does not stack"
        );
        assert_eq!(
            session.observe(0).unwrap()["players"][0]["units"][1]["keywords"]["ganking"],
            true
        );
        assert_eq!(
            session.observe(0).unwrap()["players"][0]["units"][1]["might"],
            5
        );
    }
}

#[test]
fn honeyfruit_threshold_exposes_both_options_and_adds_atomic_mixed_resources() {
    for (xp, index, energy) in [(5, 0, 0), (6, 0, 0), (6, 1, 1)] {
        let mut session = position();
        session.game.players[0].xp = xp;
        let source = gear(&mut session, 700, 347);
        for view in views(&session) {
            let options = view["players"][0]["activated_abilities"]
                .as_array()
                .unwrap();
            assert_eq!(
                options
                    .iter()
                    .map(|x| x["ability_index"].as_u64().unwrap())
                    .collect::<Vec<_>>(),
                if xp < 6 { vec![0] } else { vec![0, 1] }
            );
        }
        if xp < 6 {
            rejected(&mut session, &activate(source, 1));
        }
        exact(&mut session, activate(source, index));
        assert_eq!(session.game.priority(), 0);
        assert!(session.game.chain.is_empty());
        assert!(session.game.decision.is_none());
        assert_eq!(session.game.players[0].energy, energy);
        assert_eq!(session.game.players[0].energy_pool, energy);
        assert_eq!(session.game.players[0].any_power, 1);
        assert!(!session.game.players[0].gear[0].ready);
        assert_xp(&session, xp, false);
        rejected(&mut session, &activate(source, 0));
        rejected(&mut session, &activate(source, 1));
    }
}

#[test]
fn nested_add_during_unpayable_xp_restores_resources_readiness_and_history_on_rollback() {
    let mut session = position();
    session.game.players[0].units.push(unit(700, 345));
    session.game.gain_xp(0, 1).unwrap();
    let honey = gear(&mut session, 701, 347);
    let original = format!("{:?}", session.game);
    apply(
        &mut session,
        Action::BeginPayment {
            action: Box::new(activate(
                AbilitySource::Unit {
                    player: 0,
                    unit_id: 700,
                },
                0,
            )),
        },
    );
    exact(&mut session, activate(honey, 0));
    assert_eq!(session.game.players[0].any_power, 1);
    assert_eq!(session.observe(0).unwrap()["payment_plan"]["xp"], 2);
    rejected(&mut session, &Action::CompletePayment);
    exact(&mut session, Action::Choose { accept: false });
    assert_eq!(format!("{:?}", session.game), original);
    assert_xp(&session, 1, true);
    // The outer action guard must be restored after rollback too.
    rejected(&mut session, &activate(honey, 1));
}

#[test]
fn heirloom_play_trigger_and_equip_have_distinct_public_xp_and_attachment_timing() {
    let mut session = position();
    session.game.players[0].units.push(unit(700, 346));
    session.game.players[0].energy_pool = 2;
    session.game.players[0].energy = 2;
    session.game.players[0].hand.push(348);
    exact(
        &mut session,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        },
    );
    assert_xp(&session, 0, false);
    assert_eq!(session.game.players[0].gear.len(), 1);
    let gear_id = session.game.players[0].gear[0].id;
    assert_eq!(session.game.players[0].gear[0].attached_to, None);
    settle(&mut session);
    assert_xp(&session, 1, true);
    let action = Action::Equip {
        gear_id,
        target: Target::Unit {
            player: 0,
            unit_id: 700,
        },
    };
    let plan = result(&mut session, "cost_plan", &action);
    assert_eq!(plan["energy"], 0);
    assert_eq!(plan["power"], json!([]));
    assert_eq!(plan["xp"], 1);
    assert_eq!(plan["components"][0]["cost"]["xp_exact"], "1");
    exact(&mut session, action);
    assert_xp(&session, 0, true);
    assert_eq!(session.game.players[0].gear[0].attached_to, None);
    assert_eq!(
        session.observe(0).unwrap()["players"][0]["units"][0]["might"],
        2
    );
    settle(&mut session);
    assert_xp(&session, 0, true);
    for view in views(&session) {
        assert_eq!(view["players"][0]["gear"][0]["attached_to"], 700);
        assert_eq!(view["players"][0]["units"][0]["might"], 4);
    }
    assert!(
        session.game.chain.is_empty(),
        "Equip does not play the gear again"
    );
}

#[test]
fn honeyfruit_can_finish_frozen_energy_power_cost_without_spending_xp() {
    let mut session = position();
    session.game.players[0].xp = 6;
    session.game.players[0].hand.push(227); // Shock Blast3E1P.
    session.game.players[1].units.push(unit(700, 345));
    session.game.players[1].units[0].lane = Some(0);
    session.game.players[0].energy = 2;
    session.game.players[0].energy_pool = 2;
    let honey = gear(&mut session, 701, 347);
    let play = Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player: 1,
            unit_id: 700,
        }],
    };
    apply(
        &mut session,
        Action::BeginPayment {
            action: Box::new(play),
        },
    );
    let before = session.observe(0).unwrap()["payment_plan"].clone();
    assert_eq!(before["xp"], 0);
    assert_eq!(before["xp_exact"], "0");
    exact(&mut session, activate(honey, 1));
    assert_eq!(session.observe(0).unwrap()["payment_plan"], before);
    exact(&mut session, Action::CompletePayment);
    assert_xp(&session, 6, false);
    assert_eq!(session.game.players[0].energy, 0);
    assert_eq!(session.game.players[0].any_power, 0);
    assert_eq!(session.game.chain.last().unwrap().card_id, 227);
}

#[test]
fn mixed_add_capacity_errors_preserve_both_pools_source_and_revision() {
    for full_energy in [false, true] {
        let mut session = position();
        session.game.players[0].xp = 6;
        let source = gear(&mut session, 700, 347);
        if full_energy {
            session.game.players[0].energy = u16::MAX;
            session.game.players[0].energy_pool = u16::MAX;
        } else {
            session.game.players[0].any_power = u16::MAX;
        }
        let before = views(&session);
        let state = format!("{:?}", session.game);
        let revision = session.revision();
        let action = activate(source, 1);
        for command in ["validate", "quote_payment", "apply"] {
            let response = request(&mut session, command, &action);
            assert_eq!(response["ok"], false);
            assert_eq!(response["error"]["code"], "engine_capacity");
            assert_eq!(views(&session), before);
            assert_eq!(format!("{:?}", session.game), state);
            assert_eq!(session.revision(), revision);
        }
    }
}

#[test]
fn paying_xp_removes_lost_level_survivors_before_the_response_window() {
    for staged in [false, true] {
        let mut session = position();
        session.game.players[0].xp = 3;
        session.game.players[0].units.push(unit(700, 345));
        let mut level_unit = unit(701, 339); // Mosstomper is4M at Level3.
        level_unit.damage = 3;
        session.game.players[0].units.push(level_unit);
        let action = activate(
            AbilitySource::Unit {
                player: 0,
                unit_id: 700,
            },
            0,
        );
        if staged {
            apply(
                &mut session,
                Action::BeginPayment {
                    action: Box::new(action),
                },
            );
            assert_eq!(session.game.players[0].units.len(), 2);
            assert_xp(&session, 3, false);
            exact(&mut session, Action::CompletePayment);
        } else {
            exact(&mut session, action);
        }
        assert_xp(&session, 1, false);
        assert_eq!(
            session.game.players[0]
                .units
                .iter()
                .map(|u| u.id)
                .collect::<Vec<_>>(),
            vec![700]
        );
        assert_eq!(session.game.players[0].discard, vec![339]);
        assert_eq!(session.game.players[0].units[0].buff, 0);
        assert_eq!(
            session.game.chain.len(),
            1,
            "self-Buff still awaits responses"
        );
        settle(&mut session);
        assert_eq!(session.game.players[0].units[0].buff, 1);
    }
}
