//! Exact private occurrences and public reveal records through JSON dispatch.
use super::*;

fn session(owner: usize) -> Session {
    let mut s = Session::new(ResetOptions {
        first_player: owner,
        ..Default::default()
    })
    .unwrap();
    s.start(0).unwrap();
    s.game.battlefields = [7, 9];
    s.game.battlefield_owner = [None; 2];
    for p in &mut s.game.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.discard.clear();
        p.banish.clear();
        p.runes = [0; 6];
        p.exhausted_runes = [0; 6];
        p.energy = 40;
        p.energy_pool = 40;
        p.power_pool = [8; 6];
    }
    for legend in &mut s.game.legend {
        legend.ready = false;
    }
    s
}
fn call(s: &mut Session, command: &str, action: Value) -> Value {
    s.handle(json!({"command":command,"revision":s.revision(),"player":s.game.priority(),"action":action}))
}
fn apply(s: &mut Session, action: Value) {
    let r = call(s, "apply", action);
    assert_eq!(r["ok"], true, "{r}");
}
fn settle(s: &mut Session) {
    for _ in 0..40 {
        if s.game.decision.is_some() || s.game.chain.is_empty() {
            return;
        }
        apply(s, json!({"type":"pass"}));
    }
    panic!("unsettled chain");
}

#[test]
fn c20_private_duplicate_selection_becomes_one_public_reveal_without_exposing_remainder() {
    for owner in 0..2 {
        let mut s = session(owner);
        s.game.players[owner].deck = vec![71, 145, 145, 133];
        s.game.players[owner].hand = vec![389];
        apply(&mut s, json!({"type":"play_card","hand_index":0}));
        settle(&mut s);
        let own = s.observe(owner).unwrap();
        let other = s.observe(1 - owner).unwrap();
        assert_eq!(own["decision"]["type"], "draw_look");
        assert_eq!(own["decision"]["cards"], json!([133, 145, 145]));
        assert_eq!(own["decision"]["eligible"], json!([1, 2]));
        assert_eq!(other["decision"]["count"], 3);
        assert!(other["decision"].get("cards").is_none());
        assert!(other["decision"].get("eligible").is_none());
        let id = own["decision"]["choice_id"].as_u64().unwrap();
        for action in [
            json!({"type":"resolve_draw_look","choice_id":id+1,"index":1}),
            json!({"type":"resolve_draw_look","choice_id":id,"index":0}),
            json!({"type":"resolve_draw_look","choice_id":id,"index":3}),
            json!({"type":"choose","accept":false}),
        ] {
            assert_eq!(call(&mut s, "apply", action)["ok"], false);
            assert_eq!(s.observe(owner).unwrap(), own);
            assert_eq!(s.observe(1 - owner).unwrap(), other);
        }
        let action = json!({"type":"resolve_draw_look","choice_id":id,"index":2});
        assert_eq!(call(&mut s, "validate", action.clone())["ok"], true);
        assert_eq!(s.observe(owner).unwrap(), own);
        let wrong = s.handle(
            json!({"command":"apply","revision":s.revision(),"player":1-owner,"action":action}),
        );
        assert_eq!(wrong["ok"], false);
        assert_eq!(s.observe(owner).unwrap(), own);
        apply(&mut s, action.clone());
        let revealed = s.observe(owner).unwrap();
        assert_eq!(
            revealed["decision"],
            s.observe(1 - owner).unwrap()["decision"]
        );
        assert_eq!(revealed["decision"]["type"], "revealed_card");
        assert_eq!(revealed["decision"]["card_id"], 145);
        assert_eq!(revealed["decision"]["outcome"], "draw");
        assert!(revealed["decision"].get("cards").is_none());
        assert_eq!(s.game.players[owner].hand, vec![145]);
        assert_eq!(
            s.game.players[owner]
                .deck
                .iter()
                .filter(|&&c| c == 145)
                .count(),
            1
        );
        assert_eq!(s.game.players[owner].deck.last(), Some(&71));
        assert!(
            s.game.players[owner].discard.is_empty(),
            "parent has not finished during acknowledgement"
        );
        assert_eq!(
            call(&mut s, "apply", json!({"type":"choose","accept":false}))["ok"],
            false
        );
        assert_eq!(s.observe(owner).unwrap(), revealed);
        apply(&mut s, json!({"type":"choose","accept":true}));
        assert_eq!(s.game.players[owner].discard, vec![389]);
        let finished = s.observe(owner).unwrap();
        assert_eq!(call(&mut s, "apply", action)["ok"], false);
        assert_eq!(s.observe(owner).unwrap(), finished);
    }
}

#[test]
fn c20_draw_selection_wire_requires_exact_nullable_index_and_no_unknown_fields() {
    for a in [
        json!({"type":"resolve_draw_look","choice_id":7,"index":null}),
        json!({"type":"resolve_draw_look","choice_id":7,"index":2}),
    ] {
        assert_eq!(action_to_json(&parse_action(&a).unwrap()), a);
    }
    for a in [
        json!({"type":"resolve_draw_look","choice_id":7}),
        json!({"type":"resolve_draw_look","choice_id":7,"index":-1}),
        json!({"type":"resolve_draw_look","choice_id":7,"index":0.5}),
        json!({"type":"resolve_draw_look","choice_id":7,"index":1,"card_id":145}),
    ] {
        assert!(parse_action(&a).is_err(), "{a}");
    }
}

#[test]
fn c20_declining_an_eligible_look_is_private_and_recycles_every_occurrence() {
    for owner in 0..2 {
        let mut s = session(owner);
        s.game.players[owner].deck = vec![71, 145, 145, 133];
        s.game.players[owner].hand = vec![389];
        apply(&mut s, json!({"type":"play_card","hand_index":0}));
        settle(&mut s);
        let id = s.observe(owner).unwrap()["decision"]["choice_id"]
            .as_u64()
            .unwrap();
        apply(
            &mut s,
            json!({"type":"resolve_draw_look","choice_id":id,"index":null}),
        );
        assert!(s.game.decision.is_none());
        assert!(s.game.players[owner].hand.is_empty());
        assert_eq!(s.game.players[owner].deck.len(), 4);
        assert_eq!(s.game.players[owner].deck.last(), Some(&71));
        assert_eq!(s.game.players[owner].discard, vec![389]);
    }
}
