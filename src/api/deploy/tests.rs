//! Controlled positions, followed by actual public action/query/view flows.
//! End-to-end legal-deck setup coverage belongs to the external acceptance run.
use super::*;
use crate::game::Unit;
use crate::gear::GearState;

fn position(owner: usize) -> Session {
    let mut s = Session::new(ResetOptions {
        decks: [3, 3],
        first_player: owner,
        battlefields: Some([3, 3]),
        ..Default::default()
    })
    .unwrap();
    s.start(0).unwrap();
    s.game.battlefield_owner = [None; 2];
    for p in &mut s.game.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.discard.clear();
        p.banish.clear();
        p.hidden.clear();
        p.champion = None;
        p.deck = vec![71, 195, 71, 195, 133];
        p.runes = [0; 6];
        p.exhausted_runes = [0; 6];
        p.energy = 40;
        p.energy_pool = 40;
        p.power_pool = [8; 6];
        p.any_power = 0;
    }
    for legend in &mut s.game.legend {
        legend.ready = false;
    }
    s.game.trigger_state.draws = [0; 2];
    s
}

fn unit(id: u32, lane: Option<usize>) -> Unit {
    Unit {
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
    }
}

fn gear(id: u32, card_id: usize, lane: Option<usize>) -> GearState {
    GearState {
        id,
        card_id,
        attached_to: None,
        lane,
        ready: true,
        temporary: false,
        token: false,
    }
}

fn views(s: &Session) -> [Value; 2] {
    [s.observe(0).unwrap(), s.observe(1).unwrap()]
}

fn request(s: &mut Session, command: &str, action: Value) -> Value {
    s.handle(json!({"command":command,"revision":s.revision(),
        "player":s.game.priority(),"action":action}))
}

fn apply(s: &mut Session, action: Value) {
    let response = request(s, "apply", action.clone());
    assert_eq!(response["ok"], true, "{action}: {response}");
}

fn reject(s: &mut Session, action: Value, code: &str) {
    let request = json!({"command":"apply","revision":s.revision(),
        "player":s.game.priority(),"action":action});
    reject_request(s, request, code);
}

fn reject_request(s: &mut Session, request: Value, code: &str) {
    let before = views(s);
    let revision = s.revision();
    let response = s.handle(request);
    assert_eq!(response["error"]["code"], code, "{response}");
    assert_eq!(s.revision(), revision);
    assert_eq!(views(s), before);
}

fn finish_effect(s: &mut Session) {
    for _ in 0..40 {
        let view = s.observe(s.game.priority()).unwrap();
        if view["decision"].is_null() && s.game.chain.is_empty() {
            return;
        }
        let action = match view["decision"]["type"].as_str() {
            None => json!({"type":"pass"}),
            Some("token_play") => {
                json!({"type":"finalize_token_play","lane":null,"accelerate":false})
            }
            Some("order_triggers") => {
                json!({"type":"order_trigger","trigger_id":view["decision"]["options"][0]["id"]})
            }
            other => panic!("Unexpected effect choice {other:?}: {}", view["decision"]),
        };
        apply(s, action);
    }
    panic!("Effect did not finish");
}

fn pass_to_choice(s: &mut Session) {
    for _ in 0..30 {
        if s.game.decision.is_some() || s.game.chain.is_empty() {
            return;
        }
        apply(s, json!({"type":"pass"}));
    }
    panic!("Chain did not settle");
}

fn command(mode: usize, targets: Value) -> Value {
    json!({"type":"with_play_choices", "action":{
        "type":"play_card_targets","hand_index":0,"targets":targets},
        "choices":{"mode":mode}})
}

fn target_query(s: &mut Session, action: Value, execution: usize) -> Value {
    let before = views(s);
    let revision = s.revision();
    let response = s.handle(
        json!({"command":"target_candidates","player":s.game.priority(),
        "revision":revision,"action":action,"execution":execution}),
    );
    assert_eq!(s.revision(), revision);
    assert_eq!(views(s), before);
    assert_eq!(response["ok"], true, "{response}");
    response["result"].clone()
}

#[test]
fn deploy_entries_are_public_at_each_field_without_establishing_control() {
    for owner in 0..2 {
        for lane in 0..2 {
            for controller in [None, Some(owner), Some(1 - owner)] {
                for card in 401..=408 {
                    let mut s = position(owner);
                    if let Some(controller) = controller {
                        s.game.players[controller]
                            .units
                            .push(unit(24_001, Some(lane)));
                    }
                    s.game.battlefield_owner[lane] = controller;
                    s.game.players[owner].hand = vec![card];
                    let before = views(&s);
                    reject(
                        &mut s,
                        json!({"type":"play_card","hand_index":0,"lane":null}),
                        "illegal_action",
                    );
                    reject(
                        &mut s,
                        json!({"type":"play_card","hand_index":0,"lane":2}),
                        "invalid_request",
                    );
                    apply(
                        &mut s,
                        json!({"type":"play_card","hand_index":0,"lane":lane}),
                    );
                    let after = views(&s);
                    assert_eq!(
                        after[0]["battlefield_owner"],
                        before[0]["battlefield_owner"]
                    );
                    assert!(after[0]["showdown"].is_null());
                    for seat in 0..2 {
                        let object = &after[seat]["players"][owner]["gear"][0];
                        assert_eq!(object["card_id"], card);
                        assert_eq!(object["lane"], lane);
                        assert!(object["attached_to"].is_null());
                        assert_eq!(
                            object["deploy_status"],
                            json!({"characteristic":true,
                            "printed_abilities_active":true,"opponent_hold_trigger_active":true})
                        );
                        assert_eq!(
                            after[seat]["players"][owner]["score"],
                            before[seat]["players"][owner]["score"]
                        );
                    }
                    assert_eq!(
                        after[0]["players"][owner]["gear"],
                        after[1]["players"][owner]["gear"]
                    );
                }
            }
        }
    }
}

#[test]
fn deploy_observation_distinguishes_characteristic_from_attached_printed_rules() {
    for owner in 0..2 {
        let mut s = position(owner);
        s.game.players[owner].units.push(unit(24_001, Some(1)));
        // Explicit state fixture only; this does not claim that an admitted
        // interaction can attach an ordinary Deploy card today.
        let mut attached = gear(24_002, 401, Some(0));
        attached.attached_to = Some(24_001);
        s.game.players[owner].gear = vec![attached, gear(24_003, 195, None)];
        for view in views(&s) {
            let items = &view["players"][owner]["gear"];
            assert_eq!(items[0]["lane"], 1);
            assert_eq!(
                items[0]["deploy_status"],
                json!({"characteristic":true,
                "printed_abilities_active":false,"opponent_hold_trigger_active":false})
            );
            assert_eq!(items[1]["deploy_status"]["characteristic"], false);
            assert_eq!(
                items[1]["deploy_status"]["opponent_hold_trigger_active"],
                false
            );
        }
    }
}

#[test]
fn hidden_ordinary_gear_records_field_entry_then_is_observed_back_at_base() {
    for owner in 0..2 {
        for lane in 0..2 {
            let mut s = position(owner);
            s.game.players[owner].units.push(unit(24_001, Some(lane)));
            s.game.battlefield_owner[lane] = Some(owner);
            s.game.players[owner]
                .hidden
                .push(crate::hidden::HiddenCard {
                    id: 24_002,
                    card_id: 21, // Zhonya's Hourglass has printed Hidden.
                    lane,
                    hidden_turn: 0,
                    revealed: false,
                });
            let before = views(&s);
            let history = before[owner]["play_history"]["units_or_gears_here"][owner][lane]
                .as_u64()
                .unwrap();
            let energy = s.game.players[owner].energy;
            apply(
                &mut s,
                json!({"type":"play_hidden","hidden_id":24002,"targets":[]}),
            );
            for view in views(&s) {
                let item = &view["players"][owner]["gear"][0];
                assert_eq!(item["card_id"], 21);
                assert!(item["lane"].is_null());
                assert_eq!(item["deploy_status"]["characteristic"], false);
                assert_eq!(
                    view["play_history"]["units_or_gears_here"][owner][lane],
                    history + 1
                );
                assert_eq!(view["battlefield_owner"][lane], owner);
            }
            assert_eq!(s.game.players[owner].energy, energy);
            assert!(s.game.players[owner].hidden.is_empty());
            assert!(s.game.players[owner].discard.is_empty());
        }
    }
}

#[test]
fn command_reports_exact_gear_then_destination_and_moves_only_the_selected_copy() {
    for owner in 0..2 {
        let mut s = position(owner);
        s.game.players[owner].hand = vec![410];
        s.game.players[owner].gear = vec![
            gear(24_001, 402, Some(0)),
            gear(24_002, 402, Some(1)),
            gear(24_003, 195, None),
        ];
        s.game.players[1 - owner].gear = vec![gear(24_004, 402, Some(0))];
        let draft = command(0, json!([]));
        let options = request(&mut s, "play_choices", draft.clone());
        assert_eq!(options["ok"], true, "{options}");
        assert_eq!(options["result"]["modes"][0]["index"], 0);
        assert_eq!(
            options["result"]["modes"][0]["target_kinds"],
            json!(["gear", "battlefield"])
        );
        assert_eq!(options["result"]["modes"][1]["index"], 1);
        assert_eq!(options["result"]["modes"][1]["target_kinds"], json!([]));
        let first = target_query(&mut s, draft, 0);
        assert_eq!(first["next_slot"]["kind"], "gear");
        assert_eq!(
            first["candidates"],
            json!([
            {"type":"gear","player":owner,"gear_id":24001},
            {"type":"gear","player":owner,"gear_id":24002}])
        );
        let target = json!({"type":"gear","player":owner,"gear_id":24001});
        let next = target_query(&mut s, command(0, json!([target])), 0);
        assert_eq!(next["next_slot"]["kind"], "battlefield");
        assert_eq!(next["candidates"], json!([{"type":"battlefield","lane":1}]));
        for targets in [
            json!([target,{"type":"battlefield","lane":0}]),
            json!([{"type":"gear","player":owner,"gear_id":24003},{"type":"battlefield","lane":1}]),
            json!([{"type":"gear","player":1-owner,"gear_id":24004},{"type":"battlefield","lane":1}]),
            json!([{"type":"battlefield","lane":1},target]),
        ] {
            reject(&mut s, command(0, targets), "illegal_action");
        }
        let selected = command(0, json!([target,{"type":"battlefield","lane":1}]));
        apply(&mut s, selected);
        let pending = views(&s);
        assert_eq!(pending[0]["chain"][0]["executions"][0]["mode"], 0);
        assert_eq!(pending[0]["players"][owner]["gear"][0]["lane"], 0);
        pass_to_choice(&mut s);
        for view in views(&s) {
            assert_eq!(view["players"][owner]["gear"][0]["lane"], 1);
            assert_eq!(view["players"][owner]["gear"][1]["lane"], 1);
            assert_eq!(view["battlefield_owner"], json!([null, null]));
        }
    }
}

#[test]
fn attached_deploy_is_a_legal_command_target_but_cannot_move_separately() {
    for owner in 0..2 {
        let mut s = position(owner);
        s.game.players[owner].hand = vec![410];
        s.game.players[owner].units.push(unit(24_001, Some(1)));
        s.game.battlefield_owner[1] = Some(owner);
        // Controlled invariant fixture: no claim that an admitted effect can
        // attach these Deploy cards. Core718.5.b/c still defines this state.
        let mut attached = gear(24_002, 401, Some(0));
        attached.attached_to = Some(24_001);
        s.game.players[owner].gear.push(attached);
        let target = json!({"type":"gear","player":owner,"gear_id":24002});
        let first = target_query(&mut s, command(0, json!([])), 0);
        assert_eq!(first["candidates"], json!([target]));
        let next = target_query(&mut s, command(0, json!([target])), 0);
        // Physical location is the bearer's lane, not the stored independent
        // lane. A prevented instruction does not itself prohibit targeting.
        assert_eq!(next["candidates"], json!([{"type":"battlefield","lane":0}]));
        apply(
            &mut s,
            command(0, json!([target,{"type":"battlefield","lane":0}])),
        );
        assert_eq!(s.game.chain.len(), 1);
        finish_effect(&mut s);
        for view in views(&s) {
            assert_eq!(view["players"][owner]["gear"][0]["lane"], 1);
            assert_eq!(view["players"][owner]["gear"][0]["attached_to"], 24001);
            assert_eq!(view["players"][owner]["units"][0]["lane"], 1);
            assert_eq!(view["battlefield_owner"][1], owner);
        }
        assert_eq!(s.game.players[owner].discard, vec![410]);
    }
}

#[test]
fn command_draw_mode_keeps_occurrences_private_and_reveals_only_the_selection() {
    for owner in 0..2 {
        for index in [None, Some(3)] {
            let mut s = position(owner);
            s.game.players[owner].hand = vec![410];
            apply(&mut s, command(1, json!([])));
            pass_to_choice(&mut s);
            let before = views(&s);
            let choice = &before[owner]["decision"];
            assert_eq!(choice["type"], "draw_look");
            assert_eq!(choice["kind"], "command_attack");
            assert_eq!(choice["source_card_id"], 410);
            assert_eq!(choice["cards"], json!([133, 195, 71, 195, 71]));
            assert_eq!(choice["eligible"], json!([1, 3]));
            assert_eq!(before[1 - owner]["decision"]["count"], 5);
            assert!(before[1 - owner]["decision"].get("cards").is_none());
            assert!(before[1 - owner]["decision"].get("eligible").is_none());
            let id = choice["choice_id"].clone();
            reject(
                &mut s,
                json!({"type":"resolve_draw_look","choice_id":id,"index":0}),
                "illegal_action",
            );
            apply(
                &mut s,
                json!({"type":"resolve_draw_look","choice_id":id,"index":index}),
            );
            if index.is_some() {
                for view in views(&s) {
                    assert_eq!(view["decision"]["type"], "revealed_card");
                    assert_eq!(view["decision"]["card_id"], 195);
                }
                apply(&mut s, json!({"type":"choose","accept":true}));
            }
            pass_to_choice(&mut s);
            let after = views(&s);
            assert_eq!(
                after[owner]["players"][owner]["hand"],
                if index.is_some() {
                    json!([195])
                } else {
                    json!([])
                }
            );
            assert_eq!(
                after[owner]["players"][owner]["cards_drawn_this_turn"],
                usize::from(index.is_some())
            );
            assert!(s.game.chain.is_empty());
            assert!(s.game.decision.is_none());
            assert_eq!(
                s.game.players[owner]
                    .discard
                    .iter()
                    .filter(|&&id| id == 410)
                    .count(),
                1
            );
        }
    }
}

#[test]
fn command_mode_wire_errors_and_missing_or_nonmodal_modes_are_atomic() {
    for owner in 0..2 {
        let mut s = position(owner);
        s.game.players[owner].hand = vec![410, 9];
        for (mode, code) in [
            (json!(null), "illegal_action"),
            (json!(2), "illegal_action"),
            (json!(-1), "invalid_request"),
            (json!("0"), "invalid_request"),
            (json!(false), "invalid_request"),
        ] {
            reject(
                &mut s,
                json!({"type":"with_play_choices","action":{
                "type":"play_card_targets","hand_index":0,"targets":[]},"choices":{"mode":mode}}),
                code,
            );
        }
        reject(
            &mut s,
            json!({"type":"play_card_targets","hand_index":0,"targets":[]}),
            "illegal_action",
        );
        reject(
            &mut s,
            json!({"type":"with_play_choices","action":{
            "type":"play_card_targets","hand_index":1,"targets":[]},"choices":{"mode":0}}),
            "illegal_action",
        );
    }
}

#[test]
fn command_base_and_repeat_modes_remain_independent_through_private_choice() {
    for owner in 0..2 {
        let mut s = position(owner);
        s.game.players[owner].hand = vec![410];
        s.game.players[owner].gear = vec![gear(24_001, 402, Some(0))];
        // Explicit prior resolved Portal activation; no fabricated generic Move.
        s.game.play_grants.push(crate::play::PlayGrant {
            id: 24_100,
            player: owner,
            source: ChainSource::Card { card_id: 256 },
            expires_turn: s.game.turn,
            kind: crate::play::PlayGrantKind::RepeatNextSpell,
        });
        let mut action = command(1, json!([]));
        let preview = request(&mut s, "play_choices", action.clone());
        assert_eq!(preview["ok"], true, "{preview}");
        let option = preview["result"]["repeats"][0]["id"].clone();
        action["choices"]["repeats"] = json!([{"option_id":option,"choices":{"mode":0,"targets":[
            {"type":"gear","player":owner,"gear_id":24001},{"type":"battlefield","lane":1}]}}]);
        let repeat = target_query(&mut s, action.clone(), 1);
        assert!(repeat["next_slot"].is_null());
        apply(&mut s, action);
        let before = views(&s);
        assert_eq!(before[0]["chain"][0]["executions"][0]["mode"], 1);
        assert_eq!(before[0]["chain"][0]["executions"][1]["mode"], 0);
        pass_to_choice(&mut s);
        let private = views(&s);
        assert_eq!(private[owner]["decision"]["kind"], "command_attack");
        assert_eq!(private[owner]["players"][owner]["gear"][0]["lane"], 0);
        let choice = private[owner]["decision"]["choice_id"].clone();
        apply(
            &mut s,
            json!({"type":"resolve_draw_look","choice_id":choice,"index":null}),
        );
        assert!(s.game.chain.is_empty());
        assert!(s.game.decision.is_none());
        assert_eq!(s.game.gear_location(owner, 24_001), Some(1));
        assert_eq!(
            s.game.players[owner]
                .discard
                .iter()
                .filter(|&&id| id == 410)
                .count(),
            1
        );
    }
}

#[test]
fn poster_optional_base_cost_is_public_and_paid_before_a_respondable_trigger() {
    for owner in 0..2 {
        for pay in [false, true] {
            let mut s = position(owner);
            s.game.players[owner].hand = vec![403];
            apply(&mut s, json!({"type":"play_card","hand_index":0,"lane":1}));
            let before = views(&s);
            let decision = before[owner]["decision"].clone();
            assert_eq!(decision["type"], "trigger_payment");
            assert_eq!(decision["card_id"], 403);
            assert_eq!(decision["base_cost"]["energy"], 1);
            assert_eq!(decision["base_cost"]["power"], 0);
            assert_eq!(decision["deflect"], 0);
            assert_eq!(decision["targets"], json!([]));
            assert_eq!(
                decision["context"]["source"]["gear_id"],
                before[owner]["players"][owner]["gear"][0]["id"]
            );
            for view in &before {
                assert_eq!(view["decision"], decision);
                assert_eq!(view["trigger_payment_context"], decision);
                assert!(view["chain"].as_array().unwrap().is_empty());
                assert!(
                    view["players"][owner]["units"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                );
            }
            let id = decision["choice_id"].clone();
            let action = json!({"type":"resolve_trigger_payment","choice_id":id,"pay":pay});
            for malformed in [
                json!({"type":"resolve_trigger_payment","choice_id":id}),
                json!({"type":"resolve_trigger_payment","choice_id":id,"pay":null}),
                json!({"type":"resolve_trigger_payment","choice_id":id,"pay":1}),
                json!({"type":"resolve_trigger_payment","choice_id":4_294_967_296_u64,"pay":true}),
            ] {
                reject(&mut s, malformed, "invalid_request");
            }
            let revision = s.revision();
            reject_request(
                &mut s,
                json!({"command":"apply","player":1-owner,
                "revision":revision,"action":action}),
                "wrong_player",
            );
            reject_request(
                &mut s,
                json!({"command":"apply","player":owner,
                "revision":revision-1,"action":action}),
                "stale_revision",
            );
            reject(&mut s, json!({"type":"pass"}), "illegal_action");
            reject(&mut s, json!({"type":"end_turn"}), "illegal_action");
            let energy = s.game.players[owner].energy;
            apply(&mut s, action.clone());
            assert_eq!(s.game.players[owner].energy, energy - u16::from(pay));
            assert!(views(&s)[0]["trigger_payment_context"].is_null());
            assert_eq!(!s.game.chain.is_empty(), pay);
            assert!(s.game.players[owner].units.is_empty());
            reject(&mut s, action, "illegal_action");
            finish_effect(&mut s);
            assert_eq!(s.game.players[owner].units.len(), usize::from(pay));
            if pay {
                let recruit = &views(&s)[owner]["players"][owner]["units"][0];
                assert_eq!(recruit["token"], true);
                assert!(recruit["lane"].is_null());
            }
        }
    }
}

#[test]
fn trigger_payment_cancel_restores_choice_source_and_add_actions_before_decline() {
    for owner in 0..2 {
        for initial_energy in [1, 40] {
            let mut s = position(owner);
            s.game.players[owner].energy = initial_energy;
            s.game.players[owner].energy_pool = initial_energy;
            s.game.players[owner].hand = vec![403];
            let mut gold = gear(24_001, 13, None);
            gold.token = true;
            s.game.players[owner].gear.push(gold);
            apply(&mut s, json!({"type":"play_card","hand_index":0,"lane":0}));
            let before = views(&s);
            let id = before[owner]["decision"]["choice_id"].clone();
            assert_eq!(before[owner]["decision"]["type"], "trigger_payment");
            let accept = json!({"type":"resolve_trigger_payment","choice_id":id,"pay":true});
            if initial_energy == 1 {
                reject(&mut s, accept.clone(), "illegal_action");
            }
            apply(&mut s, json!({"type":"begin_payment","action":accept}));
            for view in views(&s) {
                assert_eq!(view["decision"]["type"], "payment");
                assert_eq!(view["trigger_payment_context"], before[owner]["decision"]);
            }
            assert!(views(&s)[1 - owner]["payment_plan"]["action"].is_null());
            apply(&mut s, json!({"type":"activate_gear","gear_id":24001}));
            assert_eq!(s.game.players[owner].any_power, 1);
            assert!(s.game.players[owner].gear.iter().all(|g| g.id != 24_001));
            assert_eq!(
                views(&s)[0]["trigger_payment_context"],
                before[owner]["decision"]
            );
            // Trigger resource costs remain declineable during Pay Costs, even
            // when funded. Undo restores the attempt and its Add effects first.
            apply(&mut s, json!({"type":"choose","accept":false}));
            let restored = views(&s);
            for seat in 0..2 {
                let mut expected = before[seat].clone();
                expected["revision"] = json!(s.revision());
                assert_eq!(restored[seat], expected);
            }
            apply(
                &mut s,
                json!({"type":"resolve_trigger_payment","choice_id":id,"pay":false}),
            );
            assert!(s.game.chain.is_empty());
            assert!(s.game.players[owner].units.is_empty());
            assert!(views(&s)[0]["trigger_payment_context"].is_null());
        }
    }
}

#[test]
fn trigger_decline_undoes_direct_add_and_preserves_the_pre_payment_source_and_targets() {
    for owner in 0..2 {
        for nested_plan in [false, true] {
            let mut s = position(owner);
            s.game.players[owner].power_pool = [0; 6];
            s.game.players[owner].runes[3] = 1;
            let mut target = unit(24_003, Some(0));
            target.ready = false;
            s.game.players[owner].units.push(target);
            s.game.battlefield_owner[0] = Some(owner);
            s.game.players[owner].hand = vec![406];
            for id in [24_001, 24_002] {
                let mut gold = gear(id, 13, None);
                gold.token = true;
                s.game.players[owner].gear.push(gold);
            }
            apply(&mut s, json!({"type":"play_card","hand_index":0,"lane":0}));
            apply(
                &mut s,
                json!({"type":"choose_trigger_targets","targets":[
                {"type":"unit","player":owner,"unit_id":24003}]}),
            );
            let original = views(&s);
            let choice = original[owner]["decision"].clone();
            let id = choice["choice_id"].clone();
            apply(&mut s, json!({"type":"activate_gear","gear_id":24001}));
            let after_first_add = views(&s);
            assert_eq!(after_first_add[owner]["trigger_payment_context"], choice);
            assert_eq!(s.game.players[owner].any_power, 1);
            if nested_plan {
                apply(
                    &mut s,
                    json!({"type":"begin_payment","action":{
                    "type":"resolve_trigger_payment","choice_id":id,"pay":true}}),
                );
                assert_eq!(views(&s)[owner]["decision"]["can_cancel"], true);
            }
            apply(&mut s, json!({"type":"activate_gear","gear_id":24002}));
            apply(
                &mut s,
                json!({"type":"activate_rune","domain":"body","recycle":false}),
            );
            apply(
                &mut s,
                json!({"type":"recycle_rune","domain":"body","exhausted":true}),
            );
            assert_eq!(views(&s)[owner]["trigger_payment_context"], choice);
            assert_eq!(s.game.players[owner].runes[3], 0);
            assert_eq!(s.game.players[owner].exhausted_runes[3], 0);
            assert_eq!(s.game.players[owner].any_power, 2);
            if nested_plan {
                apply(&mut s, json!({"type":"choose","accept":false}));
                for (seat, view) in views(&s).iter().enumerate() {
                    let mut expected = after_first_add[seat].clone();
                    expected["revision"] = json!(s.revision());
                    for (key, value) in expected.as_object().unwrap() {
                        assert_eq!(&view[key], value, "public field {key}");
                    }
                }
            }
            apply(
                &mut s,
                json!({"type":"resolve_trigger_payment","choice_id":id,"pay":false}),
            );
            for (seat, view) in views(&s).iter().enumerate() {
                let mut expected = original[seat].clone();
                expected["revision"] = json!(s.revision());
                expected["decision"] = Value::Null;
                expected["trigger_payment_context"] = Value::Null;
                expected["resource_payment_pending"] = json!(false);
                for (key, value) in expected.as_object().unwrap() {
                    assert_eq!(&view[key], value, "public field {key}");
                }
            }
            reject(
                &mut s,
                json!({"type":"resolve_trigger_payment","choice_id":id,"pay":false}),
                "illegal_action",
            );
        }
    }
}

#[test]
fn successful_trigger_payment_keeps_add_effects_and_later_decline_cannot_undo_them() {
    for owner in 0..2 {
        let mut s = position(owner);
        s.game.players[owner].power_pool = [0; 6];
        let mut target = unit(24_003, Some(0));
        target.ready = false;
        s.game.players[owner].units.push(target);
        s.game.battlefield_owner[0] = Some(owner);
        s.game.players[owner].hand = vec![406, 403];
        let mut gold = gear(24_001, 13, None);
        gold.token = true;
        s.game.players[owner].gear.push(gold);
        apply(&mut s, json!({"type":"play_card","hand_index":0,"lane":0}));
        apply(
            &mut s,
            json!({"type":"choose_trigger_targets","targets":[
            {"type":"unit","player":owner,"unit_id":24003}]}),
        );
        let first_id = views(&s)[owner]["decision"]["choice_id"].clone();
        apply(&mut s, json!({"type":"activate_gear","gear_id":24001}));
        apply(
            &mut s,
            json!({"type":"with_power_payment","action":{
            "type":"resolve_trigger_payment","choice_id":first_id,"pay":true},
            "payment":{"colored":[0,0,0,0,0,0],"any":1,"spell":0,"recycle":[0,0,0,0,0,0]}}),
        );
        assert_eq!(s.game.players[owner].any_power, 0);
        assert!(s.game.players[owner].gear.iter().all(|g| g.id != 24_001));
        assert!(!s.game.players[owner].units[0].ready);
        finish_effect(&mut s);
        assert!(s.game.players[owner].units[0].ready);
        apply(&mut s, json!({"type":"play_card","hand_index":0,"lane":1}));
        let second = views(&s)[owner]["decision"].clone();
        assert_eq!(second["card_id"], 403);
        assert_ne!(second["choice_id"], first_id);
        let before_decline = views(&s);
        apply(
            &mut s,
            json!({"type":"resolve_trigger_payment","choice_id":second["choice_id"],"pay":false}),
        );
        for (seat, view) in views(&s).iter().enumerate() {
            let mut expected = before_decline[seat].clone();
            expected["revision"] = json!(s.revision());
            expected["decision"] = Value::Null;
            expected["trigger_payment_context"] = Value::Null;
            expected["resource_payment_pending"] = json!(false);
            for (key, value) in expected.as_object().unwrap() {
                assert_eq!(&view[key], value, "public field {key}");
            }
        }
        assert!(s.game.players[owner].gear.iter().all(|g| g.id != 24_001));
        assert_eq!(s.game.players[owner].any_power, 0);
    }
}

#[test]
fn nested_resource_instructions_keep_the_public_trigger_and_finish_before_decline() {
    for owner in 0..2 {
        for staged in [false, true] {
            let mut s = position(owner);
            s.game.players[owner].hand = vec![403];
            s.game.players[owner].gear.push(gear(24_001, 244, None));
            s.game.players[owner].gear.push(gear(24_002, 244, None));
            apply(&mut s, json!({"type":"play_card","hand_index":0,"lane":0}));
            let original = views(&s);
            let choice = original[owner]["decision"].clone();
            let id = choice["choice_id"].clone();
            if staged {
                apply(
                    &mut s,
                    json!({"type":"begin_payment","action":{
                    "type":"resolve_trigger_payment","choice_id":id,"pay":true}}),
                );
            }
            for gear_id in [24_001, 24_002] {
                apply(
                    &mut s,
                    json!({"type":"activate_ability","source":{
                    "type":"gear","player":owner,"gear_id":gear_id},"ability_index":0,"targets":[]}),
                );
                for view in views(&s) {
                    assert_eq!(view["decision"]["type"], "resource_conversion");
                    assert_eq!(view["trigger_payment_context"], choice);
                }
                reject(
                    &mut s,
                    json!({"type":"resolve_trigger_payment","choice_id":id,"pay":false}),
                    "illegal_action",
                );
                reject(&mut s, json!({"type":"complete_payment"}), "illegal_action");
            }
            apply(&mut s, json!({"type":"resolve_conversion","amount":1}));
            assert_eq!(views(&s)[owner]["decision"]["type"], "resource_conversion");
            assert_eq!(views(&s)[owner]["trigger_payment_context"], choice);
            apply(&mut s, json!({"type":"resolve_conversion","amount":2}));
            assert_eq!(s.game.players[owner].any_power, 3);
            if staged {
                assert_eq!(views(&s)[owner]["decision"]["can_cancel"], true);
                apply(&mut s, json!({"type":"choose","accept":false}));
                for (seat, view) in views(&s).iter().enumerate() {
                    let mut expected = original[seat].clone();
                    expected["revision"] = json!(s.revision());
                    assert_eq!(view, &expected);
                }
            }
            apply(
                &mut s,
                json!({"type":"resolve_trigger_payment","choice_id":id,"pay":false}),
            );
            for (seat, view) in views(&s).iter().enumerate() {
                let mut expected = original[seat].clone();
                expected["revision"] = json!(s.revision());
                expected["decision"] = Value::Null;
                expected["trigger_payment_context"] = Value::Null;
                expected["resource_payment_pending"] = json!(false);
                for (key, value) in expected.as_object().unwrap() {
                    assert_eq!(&view[key], value, "public field {key}");
                }
            }
        }
    }
}

#[test]
fn portal_pins_enemy_target_before_exact_base_payment_and_resolves_after_responses() {
    for owner in 0..2 {
        let mut s = position(owner);
        let mut target = unit(24_001, Some(0));
        target.ready = false;
        s.game.players[1 - owner].units.push(target);
        s.game.battlefield_owner[0] = Some(1 - owner);
        s.game.players[owner].hand = vec![406];
        apply(&mut s, json!({"type":"play_card","hand_index":0,"lane":0}));
        assert_eq!(views(&s)[owner]["decision"]["type"], "trigger_targets");
        let target = json!({"type":"unit","player":1-owner,"unit_id":24001});
        apply(
            &mut s,
            json!({"type":"choose_trigger_targets","targets":[target]}),
        );
        let before = views(&s);
        let choice = &before[owner]["decision"];
        assert_eq!(choice["type"], "trigger_payment");
        assert_eq!(choice["targets"], json!([target]));
        assert_eq!(choice["base_cost"]["energy"], 0);
        assert_eq!(choice["base_cost"]["power"], 1);
        assert_eq!(choice["base_cost"]["domains"], json!(["body"]));
        assert!(s.game.chain.is_empty());
        assert!(!s.game.players[1 - owner].units[0].ready);
        let id = choice["choice_id"].clone();
        let accept = json!({"type":"resolve_trigger_payment","choice_id":id,"pay":true});
        let wrong = json!({"colored":[0,1,0,0,0,0],"any":0,"spell":0,"recycle":[0,0,0,0,0,0]});
        reject(
            &mut s,
            json!({"type":"with_power_payment","action":accept,"payment":wrong}),
            "illegal_action",
        );
        let payment = json!({"colored":[0,0,0,1,0,0],"any":0,"spell":0,"recycle":[0,0,0,0,0,0]});
        let exact = json!({"type":"with_power_payment","action":accept,"payment":payment});
        let quote = request(&mut s, "quote_payment", accept.clone());
        assert_eq!(quote["ok"], true, "{quote}");
        assert_eq!(views(&s), before);
        apply(&mut s, exact);
        assert!(!s.game.players[1 - owner].units[0].ready);
        assert_eq!(s.game.chain.len(), 1);
        for view in views(&s) {
            assert_eq!(view["chain"][0]["targets"], json!([target]));
            assert!(view["trigger_payment_context"].is_null());
        }
        finish_effect(&mut s);
        assert!(s.game.players[1 - owner].units[0].ready);
    }
}

#[test]
fn portal_can_target_an_already_ready_unit_and_still_pays_its_base_cost() {
    for owner in 0..2 {
        let mut s = position(owner);
        s.game.players[1 - owner].units.push(unit(24_001, Some(0)));
        s.game.battlefield_owner[0] = Some(1 - owner);
        s.game.players[owner].hand = vec![406];
        apply(&mut s, json!({"type":"play_card","hand_index":0,"lane":0}));
        let target = json!({"type":"unit","player":1-owner,"unit_id":24001});
        apply(
            &mut s,
            json!({"type":"choose_trigger_targets","targets":[target]}),
        );
        let before = views(&s);
        let choice = &before[owner]["decision"];
        assert_eq!(choice["type"], "trigger_payment");
        assert_eq!(choice["targets"], json!([target]));
        let power_before = s.game.players[owner].power_pool[3];
        apply(
            &mut s,
            json!({"type":"resolve_trigger_payment","choice_id":choice["choice_id"],"pay":true}),
        );
        assert_eq!(s.game.players[owner].power_pool[3], power_before - 1);
        assert_eq!(s.game.chain.len(), 1);
        finish_effect(&mut s);
        assert!(s.game.players[1 - owner].units[0].ready);
        assert!(s.game.decision.is_none());
        assert!(s.game.chain.is_empty());
    }
}

#[test]
fn portal_reports_base_and_deflect_separately_and_disallows_spell_power() {
    for owner in 0..2 {
        let mut s = position(owner);
        let mut target = unit(24_001, Some(0));
        target.card_id = 287; // Printed Deflect2.
        target.ready = false;
        s.game.players[1 - owner].units.push(target);
        s.game.battlefield_owner[0] = Some(1 - owner);
        s.game.players[owner].hand = vec![406];
        s.game.players[owner].any_power = 2;
        s.game.legend[owner].spell_power = 3;
        apply(&mut s, json!({"type":"play_card","hand_index":0,"lane":0}));
        let target = json!({"type":"unit","player":1-owner,"unit_id":24001});
        apply(
            &mut s,
            json!({"type":"choose_trigger_targets","targets":[target]}),
        );
        let before = views(&s);
        let choice = &before[owner]["decision"];
        assert_eq!(choice["base_cost"]["power"], 1);
        assert_eq!(choice["base_cost"]["domains"], json!(["body"]));
        assert_eq!(choice["deflect"], 2);
        let accept =
            json!({"type":"resolve_trigger_payment","choice_id":choice["choice_id"],"pay":true});
        let plan = request(&mut s, "cost_plan", accept.clone());
        assert_eq!(plan["ok"], true, "{plan}");
        let powers = plan["result"]["power"].as_array().unwrap();
        assert_eq!(
            powers
                .iter()
                .map(|p| p["amount"].as_u64().unwrap())
                .sum::<u64>(),
            3
        );
        assert!(powers.iter().all(|p| p["spell_eligible"] == false));
        assert_eq!(views(&s), before);
        reject(
            &mut s,
            json!({"type":"with_power_payment","action":accept,
            "payment":{"colored":[0,0,0,0,0,0],"any":0,"spell":3,"recycle":[0,0,0,0,0,0]}}),
            "illegal_action",
        );
        apply(
            &mut s,
            json!({"type":"with_power_payment","action":accept,
            "payment":{"colored":[0,0,0,1,0,0],"any":2,"spell":0,"recycle":[0,0,0,0,0,0]}}),
        );
        assert_eq!(s.game.players[owner].any_power, 0);
        assert_eq!(s.game.legend[owner].spell_power, 3);
        assert_eq!(s.game.chain.len(), 1);
        assert!(!s.game.players[1 - owner].units[0].ready);
        finish_effect(&mut s);
        assert!(s.game.players[1 - owner].units[0].ready);
    }
}

#[test]
fn hunting_blind_rejects_same_location_and_moves_a_different_enemy_unit() {
    for owner in 0..2 {
        let mut s = position(owner);
        s.game.players[1 - owner].units = vec![unit(24_001, Some(0)), unit(24_002, Some(1))];
        s.game.battlefield_owner = [Some(1 - owner); 2];
        s.game.players[owner].hand = vec![404];
        apply(&mut s, json!({"type":"play_card","hand_index":0,"lane":0}));
        assert_eq!(views(&s)[owner]["decision"]["type"], "trigger_targets");
        reject(
            &mut s,
            json!({"type":"choose_trigger_targets","targets":[{"type":"unit","player":1-owner,"unit_id":24001}]}),
            "illegal_action",
        );
        apply(
            &mut s,
            json!({"type":"choose_trigger_targets","targets":[{"type":"unit","player":1-owner,"unit_id":24002}]}),
        );
        assert_eq!(s.game.players[1 - owner].units[1].lane, Some(1));
        finish_effect(&mut s);
        assert_eq!(s.game.players[1 - owner].units[1].lane, Some(0));
        assert!(views(&s)[0]["showdown"].is_null());
        assert_eq!(s.game.players[owner].score, 0);
    }
}

#[test]
fn dreamroot_draw_keeps_captured_result_when_a_legal_reaction_removes_the_winner() {
    for owner in 0..2 {
        let mut s = position(owner);
        s.game.players[owner].units.push(unit(24_001, None));
        let mut defender = unit(24_002, Some(0));
        defender.card_id = 72; // Vanilla 3 Might, no Deathknell to confound the window.
        s.game.players[1 - owner].units.push(defender);
        s.game.battlefield_owner[0] = Some(1 - owner);
        s.game.players[owner].hand = vec![402, 377];
        apply(&mut s, json!({"type":"play_card","hand_index":0,"lane":0}));
        apply(&mut s, json!({"type":"move","unit_id":24001,"lane":0}));
        apply(&mut s, json!({"type":"pass"}));
        apply(&mut s, json!({"type":"pass"}));
        for _ in 0..2 {
            let Some(Decision::CombatDamage {
                player,
                lane,
                total,
            }) = s.game.decision
            else {
                panic!(
                    "Expected simultaneous combat assignment: {:?}",
                    s.game.decision
                );
            };
            let assignments = s
                .game
                .automatic_damage_assignment(player, lane, total)
                .unwrap();
            apply(
                &mut s,
                action_to_json(&Action::AssignDamage { assignments }),
            );
        }
        let captured = views(&s);
        let result = json!({"type":"winner","player":owner});
        for view in &captured {
            assert_eq!(view["combat_resolution"]["step"], "establish_control");
            assert_eq!(view["combat_resolution"]["result"], result);
            assert_eq!(view["battlefield_owner"][0], 1 - owner);
            assert_eq!(view["players"][owner]["score"], 0);
            assert_eq!(view["showdown"]["focus_open"], false);
            assert_eq!(view["chain"].as_array().unwrap().len(), 1);
            assert_eq!(view["chain"][0]["card_id"], 402);
        }
        if s.game.priority() != owner {
            apply(&mut s, json!({"type":"pass"}));
        }
        // Retreat is a real Reaction. Command: Attack would be illegal here.
        apply(
            &mut s,
            json!({"type":"play_card_targets","hand_index":0,
            "targets":[{"type":"unit","player":owner,"unit_id":24001}]}),
        );
        apply(&mut s, json!({"type":"pass"}));
        apply(&mut s, json!({"type":"pass"}));
        let after_reaction = views(&s);
        for view in &after_reaction {
            assert_eq!(view["combat_resolution"]["result"], result);
            assert_eq!(view["combat_resolution"]["step"], "establish_control");
            assert_eq!(view["chain"][0]["card_id"], 402);
            assert_eq!(view["players"][owner]["cards_drawn_this_turn"], 0);
            assert!(
                view["players"][owner]["units"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
        finish_effect(&mut s);
        for view in views(&s) {
            assert_eq!(view["players"][owner]["cards_drawn_this_turn"], 1);
            assert!(view["battlefield_owner"][0].is_null());
            assert_eq!(view["players"][owner]["score"], 0);
            assert!(view["combat_resolution"].is_null());
            assert!(view["showdown"].is_null());
        }
    }
}
