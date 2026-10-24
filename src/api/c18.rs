//! Clocktower's private Predict and public Banish choices through the wire API.
use super::*;
use crate::game::Unit;

fn session(owner: usize, top: usize) -> Session {
    let mut s = Session::new(ResetOptions::default()).unwrap();
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
        p.energy = 30;
        p.energy_pool = 30;
        p.power_pool = [5; 6];
    }
    for l in &mut s.game.legend {
        l.ready = false;
    }
    s.game.players[owner].deck = vec![70, 71, top];
    s.game.players[owner].units.push(Unit {
        id: 18001,
        card_id: 384,
        lane: Some(0),
        ready: true,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    });
    s
}
fn call(s: &mut Session, command: &str, action: Value) -> Value {
    s.handle(json!({"command":command,"player":s.game.priority(),"revision":s.revision(),"action":action}))
}
fn resolve(s: &mut Session) {
    for _ in 0..30 {
        if s.game.decision.is_some() || s.game.chain.is_empty() {
            return;
        }
        let result = call(s, "apply", json!({"type":"pass"}));
        assert_eq!(result["ok"], true, "{result}");
    }
    panic!("Chain did not finish");
}
fn death(s: &mut Session, owner: usize) {
    // A lethal-damage fixture reaches cleanup through the public dispatcher.
    s.game.players[owner].units[0].damage = 8;
    s.game.players[0].hand = vec![133];
    assert_eq!(
        call(
            s,
            "apply",
            json!({"type":"play_card_targets","hand_index":0,"targets":[]})
        )["ok"],
        true
    );
    resolve(s);
    assert!(matches!(s.game.decision, Some(Decision::BanishPlay { .. })));
}
#[test]
fn c18_predict_hides_cards_from_other_observer_and_stale_choice_keeps_revision() {
    let mut s = session(0, 72);
    s.game.players[0].units.clear();
    s.game.players[0].hand = vec![384];
    assert_eq!(
        call(&mut s, "apply", json!({"type":"play_card","hand_index":0}))["ok"],
        true
    );
    resolve(&mut s);
    let a = s.observe(0).unwrap();
    let b = s.observe(1).unwrap();
    assert_eq!(a["decision"]["cards"], json!([72, 71, 70]));
    assert!(b["decision"]["cards"].is_null());
    assert_eq!(b["decision"]["count"], 3);
    assert!(b["players"][0]["deck"].is_null());
    let choice = a["decision"]["choice_id"].as_u64().unwrap();
    let rev = s.revision();
    let invalid =
        json!({"type":"resolve_predict","choice_id":choice+1,"recycle":[],"top_order":[0,1,2]});
    assert_eq!(call(&mut s, "apply", invalid)["ok"], false);
    assert_eq!(s.revision(), rev);
    assert_eq!(s.observe(0).unwrap(), a);
}
#[test]
fn c18_public_offer_pending_instruction_and_payment_quote_agree_for_both_seats() {
    for owner in 0..2 {
        let mut s = session(owner, 145);
        death(&mut s, owner);
        let public = s.observe(1 - owner).unwrap();
        let own = s.observe(owner).unwrap();
        assert_eq!(public["decision"], own["decision"]);
        assert_eq!(public["decision"]["type"], "banish_play");
        assert_eq!(public["decision"]["card_id"], 145);
        assert_eq!(public["decision"]["origin"], json!({"type":"banish"}));
        assert_eq!(public["decision"]["optional"], true);
        assert_eq!(
            call(&mut s, "apply", json!({"type":"choose","accept":true}))["ok"],
            true
        );
        let before = s.observe(owner).unwrap();
        assert_eq!(before["decision"]["kind"], "clocktower_guardian");
        assert_eq!(
            before["decision"]["instruction"]["base_cost_waiver"],
            "energy"
        );
        assert_eq!(
            before["pending_card_plays"],
            s.observe(1 - owner).unwrap()["pending_card_plays"]
        );
        let a = json!({"type":"finalize_pending_play","lane":null,"targets":[],"accelerate":true});
        let choices = call(&mut s, "play_choices", a.clone());
        assert_eq!(choices["ok"], true, "{choices}");
        assert_eq!(choices["result"]["origin"], json!({"type":"banish"}));
        let q = call(&mut s, "quote_payment", a.clone());
        assert_eq!(q["ok"], true, "{q}");
        assert_eq!(s.observe(owner).unwrap(), before);
        let paid =
            json!({"type":"with_power_payment","action":a,"payment":q["result"]["suggested"]});
        assert_eq!(call(&mut s, "apply", paid)["ok"], true);
        assert_eq!(s.game.players[owner].units[0].card_id, 145);
        assert!(s.game.players[owner].units[0].ready);
    }
}
