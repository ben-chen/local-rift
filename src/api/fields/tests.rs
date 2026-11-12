//! Controlled positions, then public JSON actions. Legal-deck setup is a separate external gate.
use super::*;
use crate::game::Unit;

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
fn position(owner: usize, field: usize) -> Session {
    let mut s = Session::new(ResetOptions {
        decks: [1, 1],
        first_player: owner,
        battlefields: Some([field, 3]),
        ..Default::default()
    })
    .unwrap();
    s.start(0).unwrap();
    for p in &mut s.game.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.champion = None;
        p.deck = vec![71; 30];
        p.rune_deck = vec![Domain::Body; 4];
        p.set_rune_setup([0, 0, 0, 3, 0, 0], [0, 0, 0, 2, 0, 0])
            .unwrap();
        p.energy_pool = 40;
        p.energy = 41;
        p.power_pool = [8; 6];
    }
    s.game.battlefield_owner = vec![None; 2];
    s
}
fn views(s: &Session) -> [Value; 2] {
    [s.observe(0).unwrap(), s.observe(1).unwrap()]
}
fn apply(s: &mut Session, action: Value) {
    let rev = s.revision();
    let r = s.handle(
        json!({"command":"apply","player":s.game.priority(),"revision":rev,"action":action}),
    );
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(s.revision(), rev + 1);
}
fn reject(s: &mut Session, action: Value) {
    let before = views(s);
    let exact = format!("{:?}", s.game);
    let rev = s.revision();
    let r = s.handle(
        json!({"command":"apply","player":s.game.priority(),"revision":rev,"action":action}),
    );
    assert_eq!(r["ok"], false, "{r}");
    assert_eq!(r["error"]["code"], "illegal_action", "{r}");
    assert_eq!(s.revision(), rev);
    assert_eq!(views(s), before);
    assert_eq!(format!("{:?}", s.game), exact);
}
fn pass_until_choice_or_empty(s: &mut Session) {
    for _ in 0..20 {
        if s.game.decision.is_some() || s.game.chain.is_empty() {
            return;
        }
        apply(s, json!({"type":"pass"}));
    }
    panic!("chain did not settle");
}
fn conquer(s: &mut Session, owner: usize) {
    s.game.players[owner].units.push(unit(28_001, None));
    apply(s, json!({"type":"move","unit_id":28001,"lane":0}));
    // Entering an empty field opens an uncontested showdown before control.
    for _ in 0..40 {
        if s.game.decision.is_some() {
            break;
        }
        if s.game.showdown.is_none() && s.game.chain.is_empty() {
            break;
        }
        apply(s, json!({"type":"pass"}));
    }
    assert_eq!(s.game.players[owner].score, 1);
}
fn choice(s: &Session) -> Value {
    s.observe(s.game.priority()).unwrap()["decision"].clone()
}
fn choose_cost(s: &mut Session) -> u64 {
    let d = choice(s);
    assert_eq!(d["type"], "battlefield_cost");
    assert_eq!(
        d["source"],
        json!({"type":"battlefield","lane":0,"battlefield_id":11})
    );
    assert_eq!(d["cost_unit_ids"], json!([28001]));
    assert!(d.get("targets").is_none());
    let id = d["choice_id"].as_u64().unwrap();
    reject(
        s,
        json!({"type":"select_battlefield_cost","choice_id":id+100,"unit_id":28001}),
    );
    reject(
        s,
        json!({"type":"select_battlefield_cost","choice_id":id,"unit_id":29001}),
    );
    apply(
        s,
        json!({"type":"select_battlefield_cost","choice_id":id,"unit_id":28001}),
    );
    assert_eq!(choice(s)["choice_id"], id);
    assert_eq!(choice(s)["cost_unit_id"], 28001);
    id
}

#[test]
fn dais_cost_and_physical_source_are_public_and_paid_before_response_and_token() {
    for owner in 0..2 {
        let mut s = position(owner, 11);
        // A real optional token cost exposes the pending token source; without
        // it the fixed-destination, zero-cost token correctly auto-finalizes.
        let mut reksai = unit(28_002, None);
        reksai.card_id = 265;
        s.game.players[owner].units.push(reksai);
        conquer(&mut s, owner);
        let id = choose_cost(&mut s);
        let before = s.game.players[owner].energy;
        for view in views(&s) {
            assert_eq!(view["battlefield_payment_context"], view["decision"]);
            assert_eq!(view["decision"]["cost"]["energy"], 1);
        }
        apply(
            &mut s,
            json!({"type":"resolve_battlefield_payment","choice_id":id,"pay":true}),
        );
        assert_eq!(s.game.players[owner].energy, before - 1);
        assert!(s.game.players[owner].units.iter().all(|u| u.id != 28001));
        assert_eq!(s.game.players[owner].hand, vec![71]);
        let v = views(&s);
        assert_eq!(v[0]["chain"], v[1]["chain"]);
        assert_eq!(v[0]["chain"][0]["targets"], json!([]));
        assert_eq!(
            v[0]["chain"][0]["source"],
            json!({"type":"battlefield","lane":0,"battlefield_id":11})
        );
        assert!(v[0]["battlefield_payment_context"].is_null());
        assert!(s.game.players[owner].units.iter().all(|u| !u.token));
        pass_until_choice_or_empty(&mut s);
        assert_eq!(choice(&s)["type"], "token_play");
        for view in views(&s) {
            assert_eq!(
                view["pending_token_plays"][0]["source"],
                json!({"type":"battlefield","lane":0,"battlefield_id":11})
            );
        }
        apply(
            &mut s,
            json!({"type":"finalize_token_play","lane":0,"accelerate":false}),
        );
        pass_until_choice_or_empty(&mut s);
        let token = s.game.players[owner]
            .units
            .iter()
            .find(|u| u.token)
            .unwrap();
        assert!(token.token);
        assert_eq!(token.lane, Some(0));
        assert_eq!(s.game.players[owner].score, 1);
    }
}

#[test]
fn dais_staged_undo_restores_pinned_choice_and_decline_restores_direct_add() {
    for owner in 0..2 {
        let mut s = position(owner, 11);
        conquer(&mut s, owner);
        let id = choose_cost(&mut s);
        let before = views(&s);
        let objects = s.game.players[owner].rune_objects().to_vec();
        let rune = objects[0].id;
        apply(
            &mut s,
            json!({"type":"activate_rune_object","rune_id":rune,"recycle":true}),
        );
        let direct = views(&s);
        apply(
            &mut s,
            json!({"type":"begin_payment","action":{"type":"resolve_battlefield_payment","choice_id":id,"pay":true}}),
        );
        for (seat, v) in views(&s).iter().enumerate() {
            assert_eq!(v["battlefield_payment_context"]["choice_id"], id);
            assert_eq!(v["battlefield_payment_context"]["cost_unit_id"], 28001);
            assert_eq!(v["payment_plan"]["refusal_kind"], "trigger");
            assert_eq!(v["payment_plan"]["action"].is_null(), seat != owner);
            assert!(v.get("finalization_payment_origin").is_none());
        }
        apply(&mut s, json!({"type":"choose","accept":false}));
        let mut expected = direct;
        for v in &mut expected {
            v["revision"] = json!(s.revision());
        }
        assert_eq!(views(&s), expected);
        apply(
            &mut s,
            json!({"type":"resolve_battlefield_payment","choice_id":id,"pay":false}),
        );
        assert_eq!(s.game.players[owner].rune_objects(), objects);
        for (seat, v) in views(&s).iter().enumerate() {
            assert_eq!(v["players"], before[seat]["players"]);
            assert!(v["decision"].is_null());
            assert!(v["battlefield_payment_context"].is_null());
        }
        assert_eq!(s.game.players[owner].score, 1);
        assert_eq!(s.game.players[owner].units[0].id, 28001);
    }
}

#[test]
fn peak_delays_target_choice_and_recycled_identity_does_not_ready_replacement() {
    for owner in 0..2 {
        let mut s = position(owner, 14);
        conquer(&mut s, owner);
        let own = s.game.players[owner].rune_objects().to_vec();
        let foe = s.game.players[1 - owner].rune_objects().to_vec();
        for v in views(&s) {
            let delayed = v["delayed_battlefield_triggers"].as_array().unwrap();
            assert_eq!(delayed.len(), 1);
            assert_eq!(delayed[0]["source"]["battlefield_id"], 14);
            assert_eq!(delayed[0]["player"], owner);
            assert!(delayed[0].get("targets").is_none());
            assert!(v["decision"].is_null());
        }
        apply(&mut s, json!({"type":"end_turn"}));
        let d = choice(&s);
        assert_eq!(d["kind"]["type"], "targon_peak_ready");
        assert_eq!(d["maximum"], 2);
        let id = d["choice_id"].as_u64().unwrap();
        let a = json!({"type":"rune","player":owner,"rune_id":own[0].id});
        let b = json!({"type":"rune","player":1-owner,"rune_id":foe[0].id});
        reject(
            &mut s,
            json!({"type":"resolve_battlefield_targets","choice_id":id,"targets":[a,a]}),
        );
        reject(
            &mut s,
            json!({"type":"resolve_battlefield_targets","choice_id":id,"targets":[{"type":"rune","player":owner,"rune_id":999999}]}),
        );
        apply(
            &mut s,
            json!({"type":"resolve_battlefield_targets","choice_id":id,"targets":[a,b]}),
        );
        assert_eq!(s.game.priority(), owner);
        apply(
            &mut s,
            json!({"type":"activate_rune_object","rune_id":own[0].id,"recycle":true}),
        );
        pass_until_choice_or_empty(&mut s);
        assert!(
            s.game.players[owner]
                .rune_objects()
                .iter()
                .all(|r| r.id != own[0].id)
        );
        assert!(
            !s.game.players[owner]
                .rune_objects()
                .iter()
                .find(|r| r.id == own[1].id)
                .unwrap()
                .ready
        );
        assert!(
            s.game.players[1 - owner]
                .rune_objects()
                .iter()
                .find(|r| r.id == foe[0].id)
                .unwrap()
                .ready
        );
        assert!(s.game.delayed_battlefield_triggers().is_empty());
    }
}

#[test]
fn star_spring_consumes_first_play_and_excludes_the_played_unit() {
    for owner in 0..2 {
        for decline in [false, true] {
            let mut s = position(owner, 12);
            s.game.players[owner].units.push(unit(28001, Some(0)));
            s.game.battlefield_owner[0] = Some(owner);
            s.game.players[owner].hand = vec![109, 109];
            apply(&mut s, json!({"type":"play_card","hand_index":0,"lane":0}));
            pass_until_choice_or_empty(&mut s);
            let d = choice(&s);
            assert_eq!(d["kind"]["type"], "star_spring");
            let played = d["kind"]["played_unit_id"].as_u64().unwrap();
            assert_ne!(played, 28001);
            for v in views(&s) {
                assert_eq!(v["star_spring_used"][owner][0], true);
            }
            assert_eq!(
                d["candidates"],
                json!([{"type":"unit","player":owner,"unit_id":28001}])
            );
            let id = d["choice_id"].as_u64().unwrap();
            reject(
                &mut s,
                json!({"type":"resolve_battlefield_targets","choice_id":id,"targets":[{"type":"unit","player":owner,"unit_id":played}]}),
            );
            let targets = if decline {
                json!([])
            } else {
                d["candidates"].clone()
            };
            apply(
                &mut s,
                json!({"type":"resolve_battlefield_targets","choice_id":id,"targets":targets}),
            );
            if !decline {
                assert_eq!(
                    s.game.players[owner]
                        .units
                        .iter()
                        .find(|u| u.id == 28001)
                        .unwrap()
                        .lane,
                    Some(0)
                );
            }
            pass_until_choice_or_empty(&mut s);
            assert_eq!(
                s.game.players[owner]
                    .units
                    .iter()
                    .find(|u| u.id == 28001)
                    .unwrap()
                    .lane,
                if decline { Some(0) } else { None }
            );
            apply(&mut s, json!({"type":"play_card","hand_index":0,"lane":0}));
            pass_until_choice_or_empty(&mut s);
            assert!(choice(&s).is_null());
            assert!(s.game.chain.is_empty());
            apply(&mut s, json!({"type":"end_turn"}));
            for v in views(&s) {
                assert_eq!(
                    v["star_spring_used"],
                    json!([[false, false], [false, false]])
                );
            }
        }
    }
}

#[test]
fn abandoned_hall_waits_for_successful_spell_and_gives_temporary_might() {
    for owner in 0..2 {
        let mut s = position(owner, 13);
        s.game.players[owner].units.push(unit(28001, Some(0)));
        s.game.players[owner].units.push(unit(28002, None));
        s.game.battlefield_owner[0] = Some(owner);
        s.game.players[owner].hand = vec![63];
        apply(
            &mut s,
            json!({"type":"play_card_targets","hand_index":0,"targets":[{"type":"unit","player":owner,"unit_id":28002}]}),
        );
        assert!(choice(&s).is_null());
        assert_eq!(s.game.players[owner].units[0].temporary_buff, 0);
        pass_until_choice_or_empty(&mut s);
        let d = choice(&s);
        assert_eq!(d["kind"]["type"], "abandoned_hall");
        assert_eq!(d["source"]["battlefield_id"], 13);
        assert_eq!(
            d["candidates"],
            json!([{"type":"unit","player":owner,"unit_id":28001}])
        );
        apply(
            &mut s,
            json!({"type":"resolve_battlefield_targets","choice_id":d["choice_id"],"targets":d["candidates"]}),
        );
        assert_eq!(s.game.players[owner].units[0].temporary_buff, 0);
        pass_until_choice_or_empty(&mut s);
        assert_eq!(s.game.players[owner].units[0].temporary_buff, 1);
        assert_eq!(s.game.players[owner].units[0].buff, 0);
        assert_eq!(s.game.players[owner].units[1].temporary_buff, -1);
        apply(&mut s, json!({"type":"end_turn"}));
        assert_eq!(s.game.players[owner].units[0].temporary_buff, 0);
    }
}
