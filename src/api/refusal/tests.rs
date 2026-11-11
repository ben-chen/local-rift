//! Controlled board positions; all payment/child transitions use public JSON.
use super::*;
use crate::game::Unit;

fn position(owner: usize) -> Session {
    let mut s = Session::new(ResetOptions {
        decks: [1, 1],
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
        p.deck = vec![71, 109, 71, 109];
        p.set_rune_setup([2; 6], [0; 6]).unwrap();
        p.energy_pool = 20;
        p.energy = 32;
        p.power_pool = [5; 6];
        p.any_power = 0;
    }
    for l in &mut s.game.legend {
        l.ready = false;
    }
    s
}
fn unit(card_id: usize, id: u32) -> Unit {
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
fn views(s: &Session) -> [Value; 2] {
    [s.observe(0).unwrap(), s.observe(1).unwrap()]
}
fn apply(s: &mut Session, action: Value) {
    let r = s.handle(json!({"command":"apply","player":s.game.priority(),
        "revision":s.revision(),"action":action}));
    assert_eq!(r["ok"], true, "{r}");
}
fn reject(s: &mut Session, request: Value, code: &str) {
    let before = views(s);
    let revision = s.revision();
    let r = s.handle(request);
    assert_eq!(r["error"]["code"], code, "{r}");
    assert_eq!(s.revision(), revision);
    assert_eq!(views(s), before);
}
fn refuse_errors(s: &mut Session, owner: usize) {
    for (player, revision, action, code) in [
        (
            1 - owner,
            s.revision(),
            json!({"type":"refuse_payment"}),
            "wrong_player",
        ),
        (
            owner,
            s.revision() - 1,
            json!({"type":"refuse_payment"}),
            "stale_revision",
        ),
        (
            owner,
            s.revision(),
            json!({"type":"refuse_payment","pay":false}),
            "invalid_request",
        ),
        (
            owner,
            s.revision(),
            json!({"type":"begin_payment","action":{"type":"refuse_payment"}}),
            "invalid_request",
        ),
    ] {
        reject(
            s,
            json!({"command":"apply","player":player,"revision":revision,"action":action}),
            code,
        );
    }
}
fn begin(s: &mut Session, action: Value) {
    apply(s, json!({"type":"begin_payment","action":action}));
}
fn add(s: &mut Session) {
    apply(
        s,
        json!({"type":"activate_rune","domain":"mind","recycle":false}),
    );
}
fn settle(s: &mut Session) {
    for _ in 0..60 {
        if s.game.decision.is_some() || s.game.chain.is_empty() {
            return;
        }
        apply(s, json!({"type":"pass"}));
    }
    panic!("chain did not settle");
}
fn same_except_revision(s: &Session, mut old: [Value; 2]) {
    for v in &mut old {
        v["revision"] = json!(s.revision());
    }
    assert_eq!(views(s), old);
}
fn plan(s: &Session, owner: usize, kind: &str) {
    for (seat, v) in views(s).iter().enumerate() {
        assert_eq!(v["payment_plan"]["refusal_kind"], kind);
        assert_eq!(v["payment_plan"]["can_refuse"], true);
        assert_eq!(v["decision"]["can_refuse"], true);
        assert_eq!(v["decision"]["can_cancel"], true);
        assert_eq!(v["payment_plan"]["action"].is_null(), seat != owner);
        assert!(v.get("finalization_payment_origin").is_none());
        assert!(v["payment_plan"].get("original").is_none());
        assert!(v["players"][1 - seat]["hand"].is_null());
    }
}
fn child() -> Value {
    json!({"type":"finalize_pending_play","lane":null,"targets":[]})
}
fn cast(s: &mut Session, card: usize, targets: Value) {
    let p = s.game.priority();
    s.game.players[p].hand.push(card);
    let index = s.game.players[p].hand.len() - 1;
    let lane = (card == 403).then_some(0);
    apply(
        s,
        json!({"type":"play_card_targets","hand_index":index,"lane":lane,"targets":targets}),
    );
    settle(s);
}

#[test]
fn ordinary_funded_refusal_and_undo_restore_both_private_views() {
    for owner in 0..2 {
        for card in [9, 10, 15] {
            for refuse in [false, true] {
                let mut s = position(owner);
                s.game.players[owner].hand = vec![71, card, 71];
                let before = views(&s);
                begin(
                    &mut s,
                    json!({"type":"play_card_targets","hand_index":1,"lane":null,"targets":[]}),
                );
                plan(&s, owner, "ordinary");
                refuse_errors(&mut s, owner);
                add(&mut s);
                apply(
                    &mut s,
                    if refuse {
                        json!({"type":"refuse_payment"})
                    } else {
                        json!({"type":"choose","accept":false})
                    },
                );
                same_except_revision(&s, before);
                let revision = s.revision();
                reject(
                    &mut s,
                    json!({"command":"apply","player":owner,"revision":revision,"action":{"type":"refuse_payment"}}),
                    "illegal_action",
                );
            }
        }
    }
}

#[test]
fn clocktower_child_refusal_restores_exact_origin_without_undoing_parent() {
    for owner in 0..2 {
        let mut s = position(owner);
        s.game.players[owner].units.push(unit(384, 700));
        s.game.players[owner].deck = vec![71, 109];
        s.game.players[owner].banish = vec![109, 71, 109];
        cast(
            &mut s,
            294,
            json!([{"type":"unit","player":owner,"unit_id":700}]),
        );
        assert_eq!(views(&s)[0]["decision"]["type"], "banish_play");
        apply(&mut s, json!({"type":"choose","accept":true}));
        let before = views(&s);
        let id = before[0]["decision"]["pending_id"].clone();
        add(&mut s);
        let before_plan = views(&s);
        begin(&mut s, child());
        add(&mut s);
        plan(&s, owner, "pending_card");
        refuse_errors(&mut s, owner);
        apply(&mut s, json!({"type":"choose","accept":false}));
        same_except_revision(&s, before_plan);
        assert_eq!(views(&s)[0]["decision"]["pending_id"], id);
        begin(&mut s, child());
        apply(&mut s, json!({"type":"refuse_payment"}));
        for (seat, before_view) in before.iter().enumerate() {
            let after = s.observe(seat).unwrap();
            let mut expected = before_view["players"].clone();
            expected[owner]["banish"] = json!([109, 71, 109, 109]);
            assert_eq!(after["players"], expected);
            assert!(after["pending_card_plays"].as_array().unwrap().is_empty());
            assert!(after["payment_plan"].is_null());
        }
        assert!(s.game.players[owner].discard.contains(&384));
        assert!(s.game.players[owner].discard.contains(&294));
    }
}

#[test]
fn reksai_accelerated_token_refusal_consumes_one_child_and_keeps_siblings() {
    for owner in 0..2 {
        let mut s = position(owner);
        s.game.players[owner].units.push(unit(265, 700));
        cast(&mut s, 87, json!([]));
        let before = views(&s);
        assert_eq!(before[0]["decision"]["type"], "token_play");
        assert_eq!(before[0]["decision"]["remaining"], 4);
        let first = before[0]["decision"]["token_id"].clone();
        let intent = json!({"type":"finalize_token_play","lane":null,"accelerate":true});
        begin(&mut s, intent.clone());
        plan(&s, owner, "pending_token");
        add(&mut s);
        apply(&mut s, json!({"type":"choose","accept":false}));
        same_except_revision(&s, before);
        begin(&mut s, intent);
        refuse_errors(&mut s, owner);
        apply(&mut s, json!({"type":"refuse_payment"}));
        let now = views(&s);
        assert_ne!(now[0]["decision"]["token_id"], first);
        assert_eq!(now[0]["decision"]["remaining"], 3);
        assert!(!s.game.players[owner].units.iter().any(|u| u.token));
        assert!(s.game.players[owner].discard.contains(&87));
        begin(
            &mut s,
            json!({"type":"finalize_token_play","lane":null,"accelerate":false}),
        );
        for v in views(&s) {
            assert!(v["payment_plan"]["refusal_kind"].is_null());
            assert_eq!(v["decision"]["can_refuse"], false);
        }
        let revision = s.revision();
        reject(
            &mut s,
            json!({"command":"apply","player":owner,"revision":revision,"action":{"type":"refuse_payment"}}),
            "illegal_action",
        );
        apply(&mut s, json!({"type":"complete_payment"}));
        for _ in 0..2 {
            apply(
                &mut s,
                json!({"type":"finalize_token_play","lane":null,"accelerate":false}),
            );
        }
        assert_eq!(
            s.game.players[owner]
                .units
                .iter()
                .filter(|u| u.token)
                .count(),
            3
        );
    }
}

#[test]
fn poster_trigger_refusal_undoes_direct_and_staged_add_but_preserves_source() {
    for owner in 0..2 {
        let mut s = position(owner);
        cast(&mut s, 403, json!([]));
        let before = views(&s);
        let d = before[0]["decision"].clone();
        assert_eq!(d["type"], "trigger_payment");
        let intent =
            json!({"type":"resolve_trigger_payment","choice_id":d["choice_id"],"pay":true});
        add(&mut s);
        let before_plan = views(&s);
        begin(&mut s, intent.clone());
        add(&mut s);
        plan(&s, owner, "trigger");
        refuse_errors(&mut s, owner);
        for v in views(&s) {
            assert_eq!(v["trigger_payment_context"], d);
        }
        apply(&mut s, json!({"type":"choose","accept":false}));
        same_except_revision(&s, before_plan);
        begin(&mut s, intent);
        apply(&mut s, json!({"type":"refuse_payment"}));
        for (seat, before_view) in before.iter().enumerate() {
            let after = s.observe(seat).unwrap();
            assert_eq!(after["players"], before_view["players"]);
            assert!(after["trigger_payment_context"].is_null());
            assert!(after["chain"].as_array().unwrap().is_empty());
        }
        assert_eq!(s.game.players[owner].gear.len(), 1);
        assert!(!s.game.players[owner].units.iter().any(|u| u.token));
    }
}
