use super::*;

#[test]
fn riven_champion_api_exposes_unused_modes_and_spends_buff_only_after_responses() {
    for actor in 0..2 {
        let mut s = blade_session(actor);
        s.game.players[actor].units[0].card_id = 439;
        let mut enemy = s.game.players[actor].units[1].clone();
        enemy.id = 60003;
        s.game.players[1 - actor].units.push(enemy);
        let gear_id = s.game.players[actor].gear[0].id;
        apply(
            &mut s,
            json!({"type":"equip_exhaust_unit","gear_id":gear_id,"target":{"type":"unit","player":actor,"unit_id":60002},"exhaust_unit_id":60001}),
        );
        let first = s.observe(actor).unwrap();
        let first_id = first["decision"]["choice_id"].clone();
        for viewer in 0..2 {
            let d = s.observe(viewer).unwrap()["decision"].clone();
            assert_eq!(d["type"], "trigger_mode");
            assert_eq!(d["options"].as_array().unwrap().len(), 3);
            for mode in 0..3 {
                assert_eq!(d["options"][mode]["available"], true);
                assert_eq!(d["options"][mode]["used_this_turn"], false);
            }
            assert_eq!(
                d["options"][1]["resolution_object_cost"],
                json!({"type":"spend_source_buff","unit_id":60001,"player":actor,"targets_unit":false})
            );
        }
        let revision = s.revision();
        let action = json!({"type":"choose_trigger_mode","choice_id":first_id,"mode":0});
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        assert_eq!(
            s.handle(
                json!({"command":"validate","player":actor,"revision":revision,"action":action})
            )["ok"],
            true
        );
        assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
        apply(&mut s, action);
        assert_eq!(s.game.players[actor].units[0].buff, 0);
        for _ in 0..4 {
            apply(&mut s, json!({"type":"pass"}));
        }
        assert_eq!(s.game.players[actor].units[0].buff, 1);
        assert_eq!(s.game.players[actor].gear[0].attached_to, Some(60002));
        s.game.ready_unit_by(actor, actor, 60001);
        apply(&mut s, json!({"type":"move","unit_id":60001,"lane":0}));
        let d = s.observe(actor).unwrap()["decision"].clone();
        assert_eq!(d["options"][0]["available"], false);
        assert_eq!(d["options"][0]["used_this_turn"], true);
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        let bad = json!({"type":"choose_trigger_mode","choice_id":d["choice_id"],"mode":0});
        let response = s
            .handle(json!({"command":"apply","player":actor,"revision":s.revision(),"action":bad}));
        assert_eq!(response["ok"], false);
        assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
        let stale=s.handle(json!({"command":"apply","player":actor,"revision":revision,"action":{"type":"choose_trigger_mode","choice_id":first_id,"mode":1}}));
        assert_eq!(stale["error"]["code"], "stale_revision");
        apply(
            &mut s,
            json!({"type":"choose_trigger_mode","choice_id":d["choice_id"],"mode":1}),
        );
        let enemy = json!({"type":"unit","player":1-actor,"unit_id":60003});
        let candidates = trigger_query(&mut s, actor, json!([]));
        assert_eq!(candidates["result"]["candidates"], json!([enemy]));
        apply(
            &mut s,
            json!({"type":"choose_trigger_targets","targets":[enemy]}),
        );
        assert_eq!(s.game.players[actor].units[0].buff, 1);
        let item = s.observe(actor).unwrap()["chain"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()
            .clone();
        assert_eq!(
            item["targets"],
            json!([enemy]),
            "source Buff cost is not a target"
        );
        apply(&mut s, json!({"type":"pass"}));
        apply(&mut s, json!({"type":"pass"}));
        assert_eq!(s.game.players[actor].units[0].buff, 0);
        assert!(s.game.players[1 - actor].units[0].stunned);
        assert!(s.observe(actor).unwrap()["players"][1 - actor]["hand"].is_null());
    }
}

#[test]
fn riven_champion_api_stun_availability_checks_deflect_but_not_later_buff_cost() {
    let mut s = blade_session(0);
    s.game.players[0].units[0].card_id = 439;
    let mut enemy = s.game.players[0].units[1].clone();
    enemy.id = 60003;
    enemy.card_id = 92;
    s.game.players[1].units.push(enemy);
    let gear_id = s.game.players[0].gear[0].id;
    apply(
        &mut s,
        json!({"type":"equip_exhaust_unit","gear_id":gear_id,"target":{"type":"unit","player":0,"unit_id":60002},"exhaust_unit_id":60001}),
    );
    assert_eq!(
        s.observe(0).unwrap()["decision"]["options"][1]["available"],
        false
    );
    assert_eq!(s.game.players[0].units[0].buff, 0);
    s.game.players[0].any_power = 1;
    let d = s.observe(0).unwrap()["decision"].clone();
    assert_eq!(d["options"][1]["available"], true);
    apply(
        &mut s,
        json!({"type":"choose_trigger_mode","choice_id":d["choice_id"],"mode":1}),
    );
    apply(
        &mut s,
        json!({"type":"choose_trigger_targets","targets":[{"type":"unit","player":1,"unit_id":60003}]}),
    );
    assert_eq!(
        s.game.players[0].any_power, 0,
        "Deflect is paid at finalization"
    );
    apply(&mut s, json!({"type":"pass"}));
    apply(&mut s, json!({"type":"pass"}));
    assert!(
        !s.game.players[1].units[0].stunned,
        "no Buff at resolution prevents Stun"
    );
}

fn apply(s: &mut Session, action: Value) {
    let response = s.handle(json!({"command":"apply","revision":s.revision(),
        "player":s.game.priority(),"action":action}));
    assert_eq!(response["ok"], true, "{response}");
}

fn blade_session(actor: usize) -> Session {
    let mut s = Session::new(ResetOptions {
        decks: [3, 3],
        ..Default::default()
    })
    .unwrap();
    s.game.choose_starting_player(actor).unwrap();
    s.start(0).unwrap();
    s.game.battlefields = vec![9, 9];
    s.game.battlefield_owner = vec![None; 2];
    for p in &mut s.game.players {
        p.units.clear();
        p.gear.clear();
        p.hand.clear();
        p.hidden.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy = 20;
        p.energy_pool = 20;
        p.power_pool = [0; 6];
    }
    for legend in &mut s.game.legend {
        legend.ready = false;
        legend.spell_power = 0;
    }
    s.game.gear_play_at(actor, 438, None).unwrap();
    for id in [60001, 60002] {
        s.game.players[actor].units.push(crate::game::Unit {
            id,
            card_id: 71,
            lane: None,
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        });
    }
    s.game.players[1 - actor].hand = vec![429];
    s
}

#[test]
fn blade_api_describes_both_costs_and_exact_object_cost_without_spending_on_queries() {
    let data = catalog();
    let equip = &data["cards"][438]["equipment"];
    assert_eq!(equip["equip_domain"], "Any");
    assert_eq!(equip["equip_power"], 1);
    assert_eq!(
        equip["alternate_cost"],
        json!({"type":"exhaust_friendly_unit","ready_required":true,
        "may_be_bearer":true,"targets_unit":false,"action":"equip_exhaust_unit"})
    );
    for actor in 0..2 {
        let mut s = blade_session(actor);
        let gear_id = s.game.players[actor].gear[0].id;
        let action = json!({"type":"equip_exhaust_unit","gear_id":gear_id,
            "target":{"type":"unit","player":actor,"unit_id":60001},"exhaust_unit_id":60002});
        assert_eq!(action_to_json(&parse_action(&action).unwrap()), action);
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        for command in ["cost_plan", "quote_payment", "validate"] {
            let response = s.handle(
                json!({"command":command,"player":actor,"revision":s.revision(),"action":action}),
            );
            assert_eq!(response["ok"], true, "{response}");
            if command == "cost_plan" {
                assert_eq!(
                    response["result"]["object_costs"],
                    json!([{"type":"exhaust_unit","player":actor,"unit_id":60002}])
                );
                assert_eq!(response["result"]["power"], json!([]));
            }
            assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
        }
        let revision = s.revision();
        apply(&mut s, action.clone());
        for viewer in 0..2 {
            let view = s.observe(viewer).unwrap();
            assert_eq!(view["players"][actor]["units"][0]["ready"], true);
            assert_eq!(view["players"][actor]["units"][1]["ready"], false);
            assert!(view["players"][actor]["gear"][0]["attached_to"].is_null());
            assert_eq!(
                view["chain"].as_array().unwrap().last().unwrap()["targets"],
                json!([action["target"]])
            );
        }
        let before = s.observe(actor).unwrap();
        let stale =
            s.handle(json!({"command":"apply","player":actor,"revision":revision,"action":action}));
        assert_eq!(stale["error"]["code"], "stale_revision");
        assert_eq!(s.observe(actor).unwrap(), before);
        apply(&mut s, json!({"type":"pass"}));
        apply(&mut s, json!({"type":"pass"}));
        for viewer in 0..2 {
            let view = s.observe(viewer).unwrap();
            assert_eq!(view["players"][actor]["gear"][0]["attached_to"], 60001);
            assert_eq!(view["players"][actor]["units"][0]["might"], 8);
            assert_eq!(
                view["players"][actor]["units"][0]["keywords"]["ganking"],
                true
            );
        }
        assert!(s.observe(actor).unwrap()["players"][1 - actor]["hand"].is_null());
    }
}

#[test]
fn blade_api_bad_object_costs_and_overpayment_preserve_every_object() {
    let mut s = blade_session(0);
    let gear_id = s.game.players[0].gear[0].id;
    let valid = json!({"type":"equip_exhaust_unit","gear_id":gear_id,
        "target":{"type":"unit","player":0,"unit_id":60001},"exhaust_unit_id":60002});
    s.game.players[0].any_power = 1;
    let mut missing = valid.clone();
    missing.as_object_mut().unwrap().remove("exhaust_unit_id");
    let mut nonexistent = valid.clone();
    nonexistent["exhaust_unit_id"] = json!(999999);
    let mut wrong_target = valid.clone();
    wrong_target["target"]["player"] = json!(1);
    let mut unexpected = valid.clone();
    unexpected["mode"] = json!(0);
    let overpay = json!({"type":"with_power_payment","action":valid,"payment":{"colored":[0,0,0,0,0,0],"any":1,"spell":0,"recycle":[0,0,0,0,0,0]}});
    for action in [missing, nonexistent, wrong_target, unexpected, overpay] {
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        let response =
            s.handle(json!({"command":"apply","player":0,"revision":s.revision(),"action":action}));
        assert_eq!(response["ok"], false, "{response}");
        assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
    }
    // Exact resource selection accepts the ordinary Power alternative as usual.
    apply(
        &mut s,
        json!({"type":"with_power_payment","action":{"type":"equip","gear_id":gear_id,
        "target":{"type":"unit","player":0,"unit_id":60001}},
        "payment":{"colored":[0,0,0,0,0,0],"any":1,"spell":0,"recycle":[0,0,0,0,0,0]}}),
    );
    assert_eq!(s.game.players[0].any_power, 0);
    assert!(s.game.players[0].units.iter().all(|u| u.ready));
}

fn stranger_position(actor: usize, hidden: bool, crowded: bool) -> Session {
    let mut s = Session::new(ResetOptions {
        decks: [3, 3],
        ..Default::default()
    })
    .unwrap();
    s.game.choose_starting_player(actor).unwrap();
    s.start(0).unwrap();
    s.game.battlefields = vec![9, 9];
    s.game.battlefield_owner = vec![Some(actor), None];
    for p in &mut s.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 50;
        p.energy = 50;
        p.power_pool = [20; 6];
    }
    for legend in &mut s.game.legend {
        legend.ready = false;
    }
    for (player, id, lane) in [
        (actor, 100, Some(0)),
        (1 - actor, 101, Some(0)),
        (1 - actor, 102, Some(1)),
    ] {
        s.game.players[player].units.push(crate::game::Unit {
            id,
            card_id: 71,
            lane,
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        });
    }
    if crowded {
        let mut extra = s.game.players[actor].units[0].clone();
        extra.id = 103;
        s.game.players[actor].units.push(extra);
    }
    s.game.players[1 - actor].hand = vec![429];
    if hidden {
        s.game.players[actor]
            .hidden
            .push(crate::hidden::HiddenCard {
                id: 50000,
                card_id: 437,
                lane: 0,
                hidden_turn: 0,
                revealed: false,
            });
        let before = s.observe(1 - actor).unwrap();
        assert!(before["players"][actor]["hidden"][0]["card_id"].is_null());
        apply(
            &mut s,
            json!({"type":"play_hidden","hidden_id":50000,"targets":[]}),
        );
    } else {
        s.game.players[actor].hand = vec![437];
        apply(&mut s, json!({"type":"play_card","hand_index":0,"lane":0}));
    }
    assert_eq!(
        s.observe(actor).unwrap()["decision"]["type"],
        "trigger_targets"
    );
    s
}

fn trigger_query(s: &mut Session, actor: usize, targets: Value) -> Value {
    let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
    let revision = s.revision();
    let response = s.handle(
        json!({"command":"trigger_target_candidates", "revision":revision,
        "player":actor, "targets":targets}),
    );
    assert_eq!(s.revision(), revision);
    assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
    response
}

#[test]
fn bedeviling_api_responses_counter_trigger_or_bounce_its_exact_hidden_source() {
    for actor in 0..2 {
        for hidden in [false, true] {
            for counter in [false, true] {
                let mut s = stranger_position(actor, hidden, true);
                // Keep four units there even after the source returns to hand.
                let mut extra = s.game.players[actor].units[0].clone();
                extra.id = 104;
                s.game.players[actor].units.push(extra);
                let source_id = s.game.players[actor]
                    .units
                    .iter()
                    .find(|u| u.card_id == 437)
                    .unwrap()
                    .id;
                s.game.players[1 - actor].hand = vec![if counter { 12 } else { 11 }];
                apply(
                    &mut s,
                    json!({"type":"choose_trigger_targets","targets":[
                    {"type":"battlefield","lane":0},{"type":"unit","player":1-actor,"unit_id":101}]}),
                );
                let chain_id = s.game.chain.last().unwrap().id;
                apply(&mut s, json!({"type":"pass"}));
                assert_eq!(s.game.priority(), 1 - actor);
                let target = if counter {
                    json!({"type":"spell","chain_id":chain_id})
                } else {
                    json!({"type":"unit","player":actor,"unit_id":source_id})
                };
                apply(
                    &mut s,
                    json!({"type":"play_card_targets","hand_index":0,"targets":[target]}),
                );
                for _ in 0..8 {
                    if s.game.decision.is_some() || s.game.chain.is_empty() {
                        break;
                    }
                    apply(&mut s, json!({"type":"pass"}));
                }
                if !hidden && !counter {
                    assert_eq!(
                        s.observe(actor).unwrap()["decision"]["type"],
                        "trigger_return"
                    );
                    apply(&mut s, json!({"type":"choose","accept":true}));
                } else {
                    assert!(s.game.decision.is_none());
                }
                assert!(s.game.chain.is_empty());
                assert_eq!(
                    s.game.players[1 - actor].units.iter().any(|u| u.id == 101),
                    hidden || counter
                );
                assert_eq!(
                    s.game.players[actor]
                        .units
                        .iter()
                        .any(|u| u.id == source_id),
                    counter
                );
                assert!(s.observe(1 - actor).unwrap()["players"][actor]["hand"].is_null());
            }
        }
    }
}

#[test]
fn bedeviling_api_target_query_accounts_for_deflect_and_decline_does_not_refund_it() {
    let mut s = stranger_position(0, false, true);
    s.game.players[1]
        .units
        .iter_mut()
        .find(|u| u.id == 101)
        .unwrap()
        .card_id = 273;
    s.game.players[0].power_pool = [0; 6];
    s.game.players[0].any_power = 0;
    let field = json!({"type":"battlefield","lane":0});
    let enemy = json!({"type":"unit","player":1,"unit_id":101});
    let query = trigger_query(&mut s, 0, json!([field]));
    assert_eq!(query["ok"], true, "{query}");
    assert_eq!(query["result"]["candidates"], json!([]));
    assert_eq!(trigger_query(&mut s, 0, json!([field, enemy]))["ok"], false);
    s.game.players[0].any_power = 1;
    assert_eq!(
        trigger_query(&mut s, 0, json!([field]))["result"]["candidates"],
        json!([enemy])
    );
    apply(
        &mut s,
        json!({"type":"choose_trigger_targets","targets":[field,enemy]}),
    );
    assert_eq!(s.game.players[0].any_power, 0);
    apply(&mut s, json!({"type":"pass"}));
    apply(&mut s, json!({"type":"pass"}));
    apply(&mut s, json!({"type":"choose","accept":false}));
    assert_eq!(s.game.players[0].any_power, 0);
    assert!(s.game.players[1].units.iter().any(|u| u.id == 101));
    assert_eq!(trigger_query(&mut s, 0, json!([]))["ok"], false);
}

#[test]
fn bedeviling_api_queries_ordered_slots_without_mutation_or_private_information() {
    for actor in 0..2 {
        for hidden in [false, true] {
            let mut s = stranger_position(actor, hidden, false);
            let before = s.observe(actor).unwrap();
            assert_eq!(before["decision"]["target_rule"]["battlefield_first"], true);
            assert_eq!(before["decision"]["required"], 2);
            assert!(before["players"][1 - actor]["hand"].is_null());
            assert_eq!(
                before["decision"],
                s.observe(1 - actor).unwrap()["decision"]
            );
            let field = json!({"type":"battlefield","lane":0});
            let enemy = json!({"type":"unit","player":1-actor,"unit_id":101});
            let first = trigger_query(&mut s, actor, json!([]));
            assert_eq!(first["ok"], true, "{first}");
            assert_eq!(first["result"]["next_slot"]["kind"], "battlefield");
            assert_eq!(first["result"]["can_confirm"], false);
            assert_eq!(
                first["result"]["candidates"],
                if hidden {
                    json!([field])
                } else {
                    json!([field, {"type":"battlefield","lane":1}])
                }
            );
            let second = trigger_query(&mut s, actor, json!([field]));
            assert_eq!(second["result"]["next_slot"]["kind"], "unit");
            assert_eq!(second["result"]["candidates"], json!([enemy]));
            assert_eq!(second["result"]["can_confirm"], false);
            let complete = trigger_query(&mut s, actor, json!([field, enemy]));
            assert_eq!(complete["result"]["can_confirm"], true);
            assert!(complete["result"]["next_slot"].is_null());
            assert_eq!(complete["result"]["candidates"], json!([]));
            for invalid in [
                json!([enemy]),
                json!([field, {"type":"unit","player":actor,"unit_id":100}]),
                json!([field, {"type":"unit","player":1-actor,"unit_id":102}]),
                json!([field, enemy, enemy]),
            ] {
                assert_eq!(trigger_query(&mut s, actor, invalid)["ok"], false);
            }
            assert_eq!(
                trigger_query(&mut s, 1 - actor, json!([]))["error"]["code"],
                "wrong_player"
            );
            let revision = s.revision();
            apply(
                &mut s,
                json!({"type":"choose_trigger_targets","targets":[field, enemy]}),
            );
            let finalized = s.observe(actor).unwrap();
            let stale = s.handle(json!({"command":"trigger_target_candidates","revision":revision,"player":actor,"targets":[]}));
            assert_eq!(stale["error"]["code"], "stale_revision");
            assert_eq!(s.observe(actor).unwrap(), finalized);
            // Three units including Stranger: target selection is still mandatory,
            // but the optional return is unavailable when the ability resolves.
            apply(&mut s, json!({"type":"pass"}));
            apply(&mut s, json!({"type":"pass"}));
            assert!(s.game.decision.is_none() && s.game.chain.is_empty());
            assert!(s.game.players[1 - actor].units.iter().any(|u| u.id == 101));
        }
    }
}

#[test]
fn bedeviling_api_retains_resolution_and_rejects_responses_until_accept_or_decline() {
    for actor in 0..2 {
        for accept in [false, true] {
            let mut s = stranger_position(actor, false, true);
            let enemy = json!({"type":"unit","player":1-actor,"unit_id":101});
            let revision = s.revision();
            apply(
                &mut s,
                json!({"type":"choose_trigger_targets","targets":[{"type":"battlefield","lane":0},enemy]}),
            );
            assert!(s.game.decision.is_none());
            let finalized = s.observe(actor).unwrap();
            let chain_id = finalized["chain"].as_array().unwrap().last().unwrap()["id"].clone();
            apply(&mut s, json!({"type":"pass"}));
            apply(&mut s, json!({"type":"pass"}));
            let before = s.observe(actor).unwrap();
            assert_eq!(
                before["decision"],
                s.observe(1 - actor).unwrap()["decision"]
            );
            assert_eq!(before["decision"]["type"], "trigger_return");
            assert_eq!(before["decision"]["chain_id"], chain_id);
            assert_eq!(before["decision"]["target"], enemy);
            assert_eq!(before["decision"]["timing"], "resolution");
            assert_eq!(before["resolution_progress"]["chain_id"], chain_id);
            for action in [
                json!({"type":"pass"}),
                json!({"type":"end_turn"}),
                json!({"type":"choose_trigger_targets","targets":[]}),
            ] {
                let response = s.handle(json!({"command":"apply","revision":s.revision(),"player":actor,"action":action}));
                assert_eq!(response["ok"], false, "{response}");
                assert_eq!(s.observe(actor).unwrap(), before);
            }
            let wrong = s.handle(json!({"command":"apply","revision":s.revision(),"player":1-actor,"action":{"type":"choose","accept":accept}}));
            assert_eq!(wrong["error"]["code"], "wrong_player");
            let stale = s.handle(json!({"command":"apply","revision":revision,"player":actor,"action":{"type":"choose","accept":accept}}));
            assert_eq!(stale["error"]["code"], "stale_revision");
            assert_eq!(s.observe(actor).unwrap(), before);
            let preview = s.handle(json!({"command":"validate","revision":s.revision(),"player":actor,"action":{"type":"choose","accept":accept}}));
            assert_eq!(preview["ok"], true, "{preview}");
            assert_eq!(s.observe(actor).unwrap(), before);
            apply(&mut s, json!({"type":"choose","accept":accept}));
            assert!(s.game.decision.is_none() && s.game.chain.is_empty());
            assert_eq!(
                s.game.players[1 - actor].units.iter().any(|u| u.id == 101),
                !accept
            );
            assert_eq!(
                s.game.players[1 - actor].hand,
                if accept { vec![429, 71] } else { vec![429] }
            );
            assert!(s.observe(actor).unwrap()["players"][1 - actor]["hand"].is_null());
        }
    }
}

#[test]
fn blitzcrank_api_exposes_optional_pull_to_both_viewers_and_rejects_stale_choices() {
    for actor in 0..2 {
        let mut s = Session::new(ResetOptions {
            decks: [3, 3],
            ..Default::default()
        })
        .unwrap();
        s.game.choose_starting_player(actor).unwrap();
        s.start(0).unwrap();
        s.game.battlefields = vec![9, 9];
        s.game.battlefield_owner = vec![Some(actor), None];
        for p in &mut s.game.players {
            p.units.clear();
            p.gear.clear();
            p.hidden.clear();
            p.hand.clear();
            p.champion = None;
            p.set_rune_setup([0; 6], [0; 6]).unwrap();
            p.energy_pool = 50;
            p.energy = 50;
            p.power_pool = [20; 6];
        }
        for legend in &mut s.game.legend {
            legend.ready = false;
        }
        s.game.players[actor].hand = vec![436];
        s.game.players[1 - actor].hand = vec![429];
        for (player, id, lane) in [(actor, 100, Some(0)), (1 - actor, 101, None)] {
            s.game.players[player].units.push(crate::game::Unit {
                id,
                card_id: 71,
                lane,
                ready: true,
                damage: 0,
                buff: 0,
                temporary_buff: 0,
                token: false,
                protected: false,
                stunned: false,
                banish_on_death: false,
            });
        }
        apply(&mut s, json!({"type":"play_card","hand_index":0,"lane":0}));
        // A permanent enters immediately; its play trigger is the item whose
        // target is chosen before either player receives a response window.
        let own = s.observe(actor).unwrap();
        let other = s.observe(1 - actor).unwrap();
        assert_eq!(own["decision"], other["decision"]);
        assert_eq!(own["decision"]["type"], "trigger_targets");
        assert_eq!(own["decision"]["card_id"], 436);
        assert_eq!(own["decision"]["required"], 0);
        assert_eq!(own["decision"]["maximum"], 1);
        assert!(own["players"][1 - actor]["hand"].is_null());
        assert_eq!(other["players"][1 - actor]["hand"], json!([429]));
        let preview = trigger_query(&mut s, actor, json!([]));
        assert_eq!(preview["ok"], true, "{preview}");
        assert_eq!(preview["result"]["can_confirm"], true); // Optional whole trigger.
        assert_eq!(preview["result"]["next_slot"]["kind"], "unit");
        assert_eq!(
            preview["result"]["candidates"],
            json!([{"type":"unit","player":1-actor,"unit_id":101}])
        );
        let revision = s.revision();
        let choice = json!({"type":"choose_trigger_targets","targets":[{"type":"unit","player":1-actor,"unit_id":101}]});
        let validated = s.handle(
            json!({"command":"validate","revision":revision,"player":actor,"action":choice}),
        );
        assert_eq!(validated["ok"], true, "{validated}");
        assert_eq!(s.observe(actor).unwrap(), own);
        apply(&mut s, choice.clone());
        let finalized = s.observe(actor).unwrap();
        let stale =
            s.handle(json!({"command":"apply","revision":revision,"player":actor,"action":choice}));
        assert_eq!(stale["ok"], false);
        assert_eq!(s.observe(actor).unwrap(), finalized);
        assert!(
            finalized["chain"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["card_id"] == 436)
        );
        apply(&mut s, json!({"type":"pass"}));
        apply(&mut s, json!({"type":"pass"}));
        for viewer in 0..2 {
            let view = s.observe(viewer).unwrap();
            let moved = view["players"][1 - actor]["units"]
                .as_array()
                .unwrap()
                .iter()
                .find(|u| u["id"] == 101)
                .unwrap();
            assert_eq!(moved["lane"], 0);
            assert_eq!(moved["ready"], true);
        }
    }
}

#[test]
fn riven_exile_api_previews_ready_entry_and_preserves_private_information() {
    for actor in 0..2 {
        let mut s = blade_session(actor);
        s.game.legend[actor] = crate::legends::LegendState::from_legend(10).unwrap();
        s.game.legend[actor].ready = false;
        s.game.players[actor].units[0].lane = Some(0);
        s.game.battlefield_owner[0] = Some(actor);
        s.game.players[actor].hand = vec![439];
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        for lane in [Value::Null, json!(0)] {
            let action =
                json!({"type":"play_card_targets","hand_index":0,"lane":lane,"targets":[]});
            let result = s.handle(json!({"command":"play_choices","player":actor,"revision":s.revision(),"action":action}));
            assert_eq!(result["ok"], true, "{result}");
            if lane.is_null() {
                assert!(result["result"]["legend_entry_ready"].is_null());
            } else {
                assert_eq!(
                    result["result"]["legend_entry_ready"],
                    json!({"type":"legend","player":actor,"legend_id":10})
                );
            }
        }
        assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
        let revision = s.revision();
        let action = json!({"type":"play_card_targets","hand_index":0,"lane":0,"targets":[]});
        let denied = s.handle(
            json!({"command":"apply","player":1-actor,"revision":revision,"action":action}),
        );
        assert_eq!(denied["ok"], false);
        assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
        apply(&mut s, action.clone());
        for viewer in 0..2 {
            let obs = s.observe(viewer).unwrap();
            let u = obs["players"][actor]["units"]
                .as_array()
                .unwrap()
                .iter()
                .find(|u| u["card_id"] == 439)
                .unwrap();
            assert_eq!(u["ready"], true);
            assert_eq!(u["lane"], 0);
            assert_eq!(obs["players"][actor]["legend"]["kind"], "riven");
            assert!(obs["players"][1 - viewer]["hand"].is_null());
        }
        let after = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        assert_eq!(
            s.handle(json!({"command":"apply","player":actor,"revision":revision,"action":action}))
                ["ok"],
            false
        );
        assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], after);
    }
}

#[test]
fn dynamic_fields_api_accepts_existing_added_slots_and_rejects_missing_slots_atomically() {
    for actor in 0..2 {
        let mut s = blade_session(actor);
        let lane = s.game.add_battlefield(9);
        s.game.players[actor].units[0].lane = Some(lane);
        s.game.battlefield_owner[lane] = Some(actor);
        s.game.players[actor].hand = vec![439];
        s.game.legend[actor] = crate::legends::LegendState::from_legend(10).unwrap();
        for viewer in 0..2 {
            let obs = s.observe(viewer).unwrap();
            assert_eq!(obs["battlefields"].as_array().unwrap().len(), 3);
            assert_eq!(obs["battlefield_owner"][lane], actor);
            assert_eq!(obs["scored_this_turn"][actor].as_array().unwrap().len(), 3);
        }
        let action = json!({"type":"play_card_targets","hand_index":0,"lane":lane,"targets":[]});
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        let revision = s.revision();
        for missing in [lane + 1, usize::MAX] {
            let mut invalid = action.clone();
            invalid["lane"] = json!(missing);
            let result = s.handle(
                json!({"command":"apply","player":actor,"revision":revision,"action":invalid}),
            );
            assert_eq!(result["error"]["code"], "illegal_action", "{result}");
            assert_eq!(s.revision(), revision);
            assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
        }
        apply(&mut s, action);
        let unit = s.game.players[actor]
            .units
            .iter()
            .find(|u| u.card_id == 439)
            .unwrap();
        assert_eq!(unit.lane, Some(lane));
        assert!(unit.ready);

        s.game.queue_token_play(actor, 62, 1, None, false);
        s.game.after_resolution().unwrap();
        let token = json!({"type":"finalize_token_play","lane":lane,"accelerate":false});
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        let revision = s.revision();
        for missing in [lane + 1, usize::MAX] {
            let mut invalid = token.clone();
            invalid["lane"] = json!(missing);
            let result = s.handle(
                json!({"command":"apply","player":actor,"revision":revision,"action":invalid}),
            );
            assert_eq!(result["error"]["code"], "illegal_action", "{result}");
            assert_eq!(s.revision(), revision);
            assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
        }
        apply(&mut s, token);
        let token = s.game.players[actor].units.last().unwrap();
        assert!(token.token);
        assert!(token.ready);
        assert_eq!(token.lane, Some(lane));
    }
}

#[test]
fn baron_api_exposes_pending_pit_without_exposing_private_hands_and_rolls_back_cleanly() {
    for actor in 0..2 {
        let mut s = blade_session(actor);
        s.game.players[actor].hand = vec![440, 71];
        s.game.players[actor].power_pool = [4; 6];
        s.game.players[actor].units[0].lane = Some(0);
        s.game.battlefields[0] = 7;
        s.game.battlefield_owner[0] = Some(actor);
        s.game.legend[actor] = crate::legends::LegendState::from_legend(10).unwrap();
        let action = json!({"type":"play_card_targets","hand_index":0,"lane":0,"targets":[]});
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        let response=s.handle(json!({"command":"play_choices","player":actor,"revision":s.revision(),"action":action}));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(
            response["result"]["entry_replacement"],
            json!({"source_card_id":440,"battlefield_id":1000,"lane":2,"adds_battlefield":true})
        );
        assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
        apply(&mut s, json!({"type":"begin_payment","action":action}));
        for viewer in 0..2 {
            let obs = s.observe(viewer).unwrap();
            assert_eq!(obs["battlefields"], json!([7, 9, 1000]));
            assert!(obs["battlefield_owner"][2].is_null());
        }
        assert!(s.observe(1 - actor).unwrap()["players"][actor]["hand"].is_null());
        apply(&mut s, json!({"type":"choose","accept":false}));
        assert_eq!(s.game.battlefields, vec![7, 9]);
        assert_eq!(s.game.players[actor].hand, vec![440, 71]);
        apply(&mut s, json!({"type":"begin_payment","action":action}));
        apply(&mut s, json!({"type":"complete_payment"}));
        let unit = s.game.players[actor]
            .units
            .iter()
            .find(|u| u.card_id == 440)
            .unwrap();
        assert!(unit.ready);
        assert_eq!(unit.lane, Some(2));
        assert_eq!(s.game.players[actor].energy, 11);
        assert_eq!(s.game.battlefield_count(), 3);
    }
    let catalog = super::catalog();
    assert_eq!(catalog["token_battlefields"][0]["official_id"], "unl-t01");
    assert_eq!(catalog["token_battlefields"][0]["can_start_game"], false);
    assert!(
        catalog["battlefields"]
            .as_array()
            .unwrap()
            .iter()
            .all(|b| b["id"] != 1000)
    );
    assert!(
        Session::new(ResetOptions {
            battlefields: Some([1000, 0]),
            ..Default::default()
        })
        .is_err()
    );
}
