//! Controlled rune setup; subsequent identity/payment transitions use public JSON.
use super::*;

fn position(owner: usize) -> Session {
    let mut session = Session::new(ResetOptions {
        decks: [1, 1],
        first_player: owner,
        ..Default::default()
    })
    .unwrap();
    session.start(0).unwrap();
    for p in &mut session.game.players {
        p.set_rune_setup([0, 0, 0, 3, 0, 0], [0, 0, 0, 1, 0, 0])
            .unwrap();
        p.rune_deck = vec![Domain::Body; 3];
        p.energy_pool = 0;
        p.energy = 2;
        p.hand.clear();
        p.deck = vec![71; 20];
    }
    session
}
fn views(session: &Session) -> [Value; 2] {
    [session.observe(0).unwrap(), session.observe(1).unwrap()]
}
fn apply(session: &mut Session, action: Value) {
    let before = session.revision();
    let response = session.handle(json!({"command":"apply","player":session.game.priority(),"revision":before,"action":action}));
    assert_eq!(response["ok"], true, "{response}");
    assert_eq!(session.revision(), before + 1);
}
fn reject(session: &mut Session, player: usize, revision: u64, action: Value, code: &str) {
    let before = views(session);
    let exact = format!("{:?}", session.game);
    let old_revision = session.revision();
    let response = session
        .handle(json!({"command":"apply","player":player,"revision":revision,"action":action}));
    assert_eq!(response["ok"], false, "{response}");
    assert_eq!(response["error"]["code"], code, "{response}");
    assert_eq!(session.revision(), old_revision);
    assert_eq!(views(session), before);
    assert_eq!(format!("{:?}", session.game), exact);
}
fn object_action(id: u32, recycle: bool) -> Value {
    json!({"type":"activate_rune_object","rune_id":id,"recycle":recycle})
}

#[test]
fn physical_runes_are_public_consistent_and_queries_do_not_reveal_private_order() {
    for owner in 0..2 {
        let mut session = position(owner);
        session.game.players[owner].hand = vec![71, 109];
        session.game.players[1 - owner].hand = vec![109, 71, 109];
        let exact = format!("{:?}", session.game);
        let before = views(&session);
        for player in 0..2 {
            assert_eq!(
                before[0]["players"][player]["rune_objects"],
                before[1]["players"][player]["rune_objects"]
            );
            let objects = before[0]["players"][player]["rune_objects"]
                .as_array()
                .unwrap();
            assert_eq!(objects.len(), 3);
            assert_eq!(objects.iter().filter(|r| r["ready"] == true).count(), 2);
            assert!(objects.iter().all(|r| r["domain"] == "Body"));
            assert_eq!(
                before[0]["players"][player]["runes"],
                json!([0, 0, 0, 3, 0, 0])
            );
            assert_eq!(
                before[0]["players"][player]["exhausted_runes"],
                json!([0, 0, 0, 1, 0, 0])
            );
            assert!(before[player]["players"][player]["hand"].is_array());
            assert!(before[1 - player]["players"][player]["hand"].is_null());
            for view in &before {
                assert!(view["players"][player].get("rune_deck").is_none());
                assert!(view["players"][player].get("next_rune_id").is_none());
            }
        }
        let id = session.game.players[owner]
            .rune_objects()
            .iter()
            .find(|r| r.ready)
            .unwrap()
            .id;
        let response = session.handle(json!({"command":"validate","player":owner,"revision":session.revision(),"action":object_action(id,false)}));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(views(&session), before);
        assert_eq!(format!("{:?}", session.game), exact);
    }
}

#[test]
fn exact_rune_activation_recycling_and_rechannel_never_rebind_a_stale_id() {
    for owner in 0..2 {
        let mut session = position(owner);
        let originals = session.game.players[owner].rune_objects().to_vec();
        let selected = originals.iter().find(|r| r.ready).unwrap().id;
        let revision = session.revision();
        reject(
            &mut session,
            1 - owner,
            revision,
            object_action(selected, false),
            "wrong_player",
        );
        reject(
            &mut session,
            owner,
            revision - 1,
            object_action(selected, false),
            "stale_revision",
        );
        reject(
            &mut session,
            owner,
            revision,
            json!({"type":"activate_rune_object","rune_id":selected,"recycle":"no"}),
            "invalid_request",
        );
        apply(&mut session, object_action(selected, false));
        for rune in session.game.players[owner].rune_objects() {
            let old = originals.iter().find(|x| x.id == rune.id).unwrap();
            assert_eq!(
                rune.ready,
                if rune.id == selected {
                    false
                } else {
                    old.ready
                }
            );
        }
        let revision = session.revision();
        reject(
            &mut session,
            owner,
            revision,
            object_action(selected, false),
            "illegal_action",
        );
        apply(&mut session, object_action(selected, true));
        assert!(
            !session.game.players[owner]
                .rune_objects()
                .iter()
                .any(|r| r.id == selected)
        );
        assert_eq!(session.game.players[owner].power_pool[3], 1);
        let revision = session.revision();
        reject(
            &mut session,
            owner,
            revision,
            object_action(selected, true),
            "illegal_action",
        );
        apply(&mut session, json!({"type":"end_turn"}));
        apply(&mut session, json!({"type":"end_turn"}));
        assert_eq!(session.game.priority(), owner);
        assert!(
            session.game.players[owner]
                .rune_objects()
                .iter()
                .any(|r| r.domain == Domain::Body && !originals.iter().any(|old| old.id == r.id))
        );
        let revision = session.revision();
        reject(
            &mut session,
            owner,
            revision,
            object_action(selected, true),
            "illegal_action",
        );
        for old in originals.iter().filter(|r| r.id != selected) {
            assert!(
                session.game.players[owner]
                    .rune_objects()
                    .iter()
                    .any(|r| r.id == old.id)
            );
        }
    }
}

#[test]
fn legacy_domain_selectors_match_the_exact_physical_rune_actions() {
    for owner in 0..2 {
        for recycle in [false, true] {
            let mut legacy = position(owner);
            let mut exact = legacy.clone();
            let id = legacy.game.players[owner]
                .rune_objects()
                .iter()
                .find(|r| r.ready != recycle)
                .unwrap()
                .id;
            apply(
                &mut legacy,
                json!({"type":"activate_rune","domain":"body","recycle":recycle}),
            );
            apply(&mut exact, object_action(id, recycle));
            assert_eq!(views(&legacy), views(&exact));
        }
    }
}

#[test]
fn payment_undo_and_refuse_restore_recycled_physical_rune_and_private_world() {
    for owner in 0..2 {
        for refuse in [false, true] {
            let mut session = position(owner);
            session.game.players[owner].hand = vec![10];
            session.game.players[owner].energy_pool = 20;
            session.game.players[owner].energy = 22;
            session.game.players[owner].power_pool = [5; 6];
            let before = views(&session);
            let exact = format!("{:?}", session.game);
            let id = session.game.players[owner].rune_objects()[0].id;
            apply(
                &mut session,
                json!({"type":"begin_payment","action":{"type":"play_card","hand_index":0,"lane":null}}),
            );
            apply(&mut session, object_action(id, true));
            assert!(
                !session.game.players[owner]
                    .rune_objects()
                    .iter()
                    .any(|r| r.id == id)
            );
            for (seat, view) in views(&session).iter().enumerate() {
                assert_eq!(view["payment_plan"]["action"].is_null(), seat != owner);
                assert!(view["payment_plan"].get("original").is_none());
                assert!(view["payment_plan"].get("frozen").is_none());
                assert!(view.get("finalization_payment_origin").is_none());
            }
            apply(
                &mut session,
                if refuse {
                    json!({"type":"refuse_payment"})
                } else {
                    json!({"type":"choose","accept":false})
                },
            );
            let mut expected = before;
            for view in &mut expected {
                view["revision"] = json!(session.revision());
            }
            assert_eq!(views(&session), expected);
            assert_eq!(format!("{:?}", session.game), exact);
        }
    }
}
