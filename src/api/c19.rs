//! Public dispatcher coverage for private selections and linked play payments.
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
    for l in &mut s.game.legend {
        l.ready = false;
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
    panic!("Chain did not finish");
}
fn play_look(s: &mut Session, owner: usize, card: usize) {
    s.game.players[owner].deck = vec![71, 70, 145, 10, 145, 133];
    s.game.players[owner].hand = vec![card];
    apply(s, json!({"type":"play_card","hand_index":0}));
    settle(s);
}
#[test]
fn c19_look_identity_eligibility_and_duplicate_occurrences_are_private_for_each_seat() {
    for owner in 0..2 {
        let mut s = session(owner);
        play_look(&mut s, owner, 387);
        let own = s.observe(owner).unwrap();
        let other = s.observe(1 - owner).unwrap();
        assert_eq!(own["decision"]["type"], "deck_look_play");
        assert_eq!(own["decision"]["cards"], json!([133, 145, 10, 145, 70]));
        assert_eq!(own["decision"]["eligible"], json!([1, 3, 4]));
        assert!(other["decision"].get("cards").is_none());
        assert!(other["decision"].get("eligible").is_none());
        assert_eq!(other["decision"]["count"], 5);
        assert!(other["players"][owner]["deck"].is_null());
        let choice = own["decision"]["choice_id"].as_u64().unwrap();
        let rev = s.revision();
        for a in [
            json!({"type":"resolve_deck_look","choice_id":choice+1,"index":1}),
            json!({"type":"resolve_deck_look","choice_id":choice,"index":2}),
        ] {
            assert_eq!(call(&mut s, "apply", a)["ok"], false);
            assert_eq!(s.revision(), rev);
            assert_eq!(s.observe(owner).unwrap(), own);
        }
        apply(
            &mut s,
            json!({"type":"resolve_deck_look","choice_id":choice,"index":3}),
        );
        assert_eq!(s.game.pending_cards()[0].card_id, 145);
        assert_eq!(
            s.game.players[owner]
                .deck
                .iter()
                .filter(|&&c| c == 145)
                .count(),
            1
        );
        assert_eq!(s.game.players[owner].discard, vec![387]);
        let pending = s.observe(owner).unwrap();
        assert_eq!(
            pending["decision"]["instruction"]["base_cost_waiver"],
            "none"
        );
        assert_eq!(
            pending["decision"]["instruction"]["total_energy_discount"],
            5
        );
        assert_eq!(
            pending["pending_card_plays"],
            s.observe(1 - owner).unwrap()["pending_card_plays"]
        );
        let action =
            json!({"type":"finalize_pending_play","lane":null,"targets":[],"accelerate":true});
        let quote = call(&mut s, "quote_payment", action.clone());
        assert_eq!(quote["ok"], true, "{quote}");
        assert_eq!(s.observe(owner).unwrap(), pending);
        apply(
            &mut s,
            json!({"type":"with_power_payment","action":action,"payment":quote["result"]["suggested"]}),
        );
        assert_eq!(s.game.players[owner].units[0].card_id, 145);
        assert!(s.game.players[owner].units[0].ready);
    }
}
#[test]
fn c19_wire_selection_requires_explicit_nullable_index_and_rejects_unknown_fields() {
    for a in [
        json!({"type":"resolve_deck_look","choice_id":2,"index":null}),
        json!({"type":"resolve_deck_look","choice_id":2,"index":4}),
    ] {
        assert_eq!(action_to_json(&parse_action(&a).unwrap()), a);
    }
    for a in [
        json!({"type":"resolve_deck_look","choice_id":2}),
        json!({"type":"resolve_deck_look","choice_id":2,"index":-1}),
        json!({"type":"resolve_deck_look","choice_id":2,"index":1.5}),
        json!({"type":"resolve_deck_look","choice_id":2,"index":1,"card_id":145}),
    ] {
        assert!(parse_action(&a).is_err(), "{a}");
    }
}
#[test]
fn c19_public_aurora_reveal_and_wild_claw_reflexive_keep_public_fixed_identity() {
    for owner in 0..2 {
        let mut s = session(owner);
        s.game.players[owner].hand = vec![386];
        s.game.players[owner].deck = vec![70, 145, 133, 10];
        apply(&mut s, json!({"type":"play_card","hand_index":0}));
        apply(&mut s, json!({"type":"end_turn"}));
        settle(&mut s);
        let a = s.observe(owner).unwrap();
        let b = s.observe(1 - owner).unwrap();
        assert_eq!(a["decision"], b["decision"]);
        assert_eq!(a["decision"]["cards"], json!([10, 133, 145]));
        assert_eq!(a["decision"]["type"], "revealed_deck");
        apply(&mut s, json!({"type":"choose","accept":true}));
        assert_eq!(
            s.observe(owner).unwrap()["decision"]["kind"],
            "dazzling_aurora"
        );
        let mut s = session(owner);
        play_look(&mut s, owner, 388);
        let choice = s.observe(owner).unwrap()["decision"]["choice_id"]
            .as_u64()
            .unwrap();
        apply(
            &mut s,
            json!({"type":"resolve_deck_look","choice_id":choice,"index":2}),
        );
        apply(
            &mut s,
            json!({"type":"finalize_pending_play","lane":null,"targets":[]}),
        );
        let a = s.observe(owner).unwrap();
        assert_eq!(a["decision"]["type"], "empower_played");
        assert_eq!(a["decision"], s.observe(1 - owner).unwrap()["decision"]);
        assert_eq!(a["decision"]["target"]["type"], "gear");
        let id = s.game.players[owner].gear[0].id;
        assert!(!s.game.unit_empowered(id));
        apply(&mut s, json!({"type":"choose","accept":true}));
        assert!(!s.game.unit_empowered(id));
        settle(&mut s);
        assert!(s.game.unit_empowered(id));
    }
}
