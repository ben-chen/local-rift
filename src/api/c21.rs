//! Protocol tests seed positions, then exercise ordinary public commands.
use super::*;
use crate::gear::GearState;

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
        p.hidden.clear();
        p.champion = None;
        p.runes = [0; 6];
        p.exhausted_runes = [0; 6];
        p.energy = 40;
        p.energy_pool = 40;
        p.power_pool = [8; 6];
        p.xp = 0;
    }
    for legend in &mut s.game.legend {
        legend.ready = false;
    }
    s.game.trigger_state.draws = [0; 2];
    s
}
fn call(s: &mut Session, command: &str, action: Value) -> Value {
    s.handle(json!({"command":command,"revision":s.revision(),"player":s.game.priority(),"action":action}))
}
fn apply(s: &mut Session, action: Value) {
    let r = call(s, "apply", action);
    assert_eq!(r["ok"], true, "{r}");
}
fn views(s: &Session) -> [Value; 2] {
    [s.observe(0).unwrap(), s.observe(1).unwrap()]
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
fn cast(s: &mut Session, owner: usize, card: usize) {
    s.game.players[owner].hand = vec![card];
    apply(s, json!({"type":"play_card","hand_index":0}));
    settle(s);
}
fn bloom(s: &mut Session, owner: usize) {
    s.game.players[owner].gear.push(GearState {
        id: 21001,
        card_id: 397,
        attached_to: None,
        lane: None,
        ready: true,
        temporary: false,
        token: false,
    });
    apply(
        s,
        json!({"type":"activate_ability","source":{"type":"gear","player":owner,"gear_id":21001},"ability_index":0,"targets":[]}),
    );
    settle(s);
}

#[test]
fn c21_ornn_keeps_private_occurrences_and_reveals_only_the_chosen_gear() {
    for owner in 0..2 {
        let mut s = session(owner);
        s.game.players[owner].deck = vec![71, 195, 96, 195, 133];
        cast(&mut s, owner, 395);
        let before = views(&s);
        let own = &before[owner]["decision"];
        assert_eq!(own["type"], "draw_look");
        assert_eq!(own["kind"], "ornn");
        assert_eq!(own["source_card_id"], 395);
        assert_eq!(own["cards"], json!([133, 195, 96, 195]));
        assert_eq!(own["eligible"], json!([1, 3]));
        assert_eq!(before[1 - owner]["decision"]["count"], 4);
        assert!(before[1 - owner]["decision"].get("cards").is_none());
        assert!(before[1 - owner]["decision"].get("eligible").is_none());
        let id = own["choice_id"].as_u64().unwrap();
        for index in [0, 2, 4] {
            assert_eq!(
                call(
                    &mut s,
                    "apply",
                    json!({"type":"resolve_draw_look","choice_id":id,"index":index})
                )["ok"],
                false
            );
            assert_eq!(views(&s), before);
        }
        apply(
            &mut s,
            json!({"type":"resolve_draw_look","choice_id":id,"index":3}),
        );
        let revealed = views(&s);
        assert_eq!(revealed[0]["decision"], revealed[1]["decision"]);
        assert_eq!(revealed[owner]["decision"]["card_id"], 195);
        assert_eq!(revealed[owner]["decision"]["outcome"], "draw");
        assert_eq!(s.game.players[owner].hand, vec![195]);
        assert_eq!(
            s.game.players[owner]
                .deck
                .iter()
                .filter(|&&c| c == 195)
                .count(),
            1
        );
        assert_eq!(s.game.players[owner].deck.last(), Some(&71));
        assert_eq!(
            revealed[owner]["players"][owner]["cards_drawn_this_turn_exact"],
            "1"
        );
        apply(&mut s, json!({"type":"choose","accept":true}));
        assert!(s.game.decision.is_none());
    }
}

#[test]
fn c21_clairvoyance_exposes_public_tail_but_draws_only_after_private_predict_partition() {
    for owner in 0..2 {
        let mut s = session(owner);
        s.game.players[owner].deck = vec![71, 70, 195, 96, 145, 133];
        cast(&mut s, owner, 396);
        let before = views(&s);
        let own = &before[owner];
        assert_eq!(own["decision"]["type"], "predict_cards");
        assert_eq!(own["decision"]["cards"], json!([133, 145, 96, 195, 70]));
        assert!(before[1 - owner]["decision"].get("cards").is_none());
        let chain_id = s.game.chain.last().unwrap().id;
        let tail = json!({"chain_id":chain_id,"player":owner,"source_card_id":396,"remaining_draw":2,"remaining_xp":0,"remaining_xp_exact":"0"});
        for view in &before {
            assert_eq!(view["predict_continuation"], tail);
            assert_eq!(view["resolution_progress"]["chain_id"], chain_id);
            assert_eq!(view["players"][owner]["cards_drawn_this_turn_exact"], "0");
        }
        assert!(s.game.players[owner].hand.is_empty());
        assert!(s.game.players[owner].discard.is_empty());
        let id = own["decision"]["choice_id"].as_u64().unwrap();
        let valid =
            json!({"type":"resolve_predict","choice_id":id,"recycle":[0,2],"top_order":[3,1,4]});
        for invalid in [
            json!({"type":"resolve_predict","choice_id":id+1,"recycle":[0,2],"top_order":[3,1,4]}),
            json!({"type":"resolve_predict","choice_id":id,"recycle":[0,2],"top_order":[3,1,1]}),
            json!({"type":"resolve_predict","choice_id":id,"recycle":[0,2],"top_order":[3,1]}),
        ] {
            assert_eq!(call(&mut s, "apply", invalid)["ok"], false);
            assert_eq!(views(&s), before);
        }
        assert_eq!(call(&mut s, "validate", valid.clone())["ok"], true);
        assert_eq!(views(&s), before);
        apply(&mut s, valid.clone());
        assert_eq!(s.game.players[owner].hand, vec![195, 145]);
        assert_eq!(s.game.players[owner].deck.last(), Some(&70));
        assert_eq!(s.game.players[owner].discard, vec![396]);
        for view in views(&s) {
            assert!(view["predict_continuation"].is_null());
            assert!(view["resolution_progress"].is_null());
            assert_eq!(view["players"][owner]["cards_drawn_this_turn_exact"], "2");
        }
        let after = views(&s);
        assert_eq!(call(&mut s, "apply", valid)["ok"], false);
        assert_eq!(views(&s), after);
    }
}

#[test]
fn c21_bloom_cost_departure_keeps_public_source_and_defers_draw_and_xp() {
    for owner in 0..2 {
        let mut s = session(owner);
        s.game.players[owner].deck = vec![71, 195, 96];
        let energy = s.game.players[owner].energy_pool;
        let uses = s.game.play_history.gear_activations[owner];
        bloom(&mut s, owner);
        assert!(s.game.players[owner].gear.is_empty());
        assert_eq!(s.game.players[owner].discard, vec![397]);
        assert_eq!(s.game.players[owner].energy_pool, energy - 1);
        // This public history counts played abilities, not completed uses.
        assert_eq!(s.game.play_history.gear_activations[owner], uses + 1);
        assert_eq!(s.game.players[owner].xp, 0);
        assert!(s.game.players[owner].hand.is_empty());
        let before = views(&s);
        let id = before[owner]["decision"]["choice_id"].as_u64().unwrap();
        for view in &before {
            assert_eq!(view["predict_continuation"]["source_card_id"], 397);
            assert_eq!(view["predict_continuation"]["remaining_draw"], 1);
            assert_eq!(view["predict_continuation"]["remaining_xp_exact"], "1");
            assert_eq!(view["resolution_progress"]["execution"], 0);
            assert_eq!(view["resolution_progress"]["executions"], 1);
            assert_eq!(view["chain"].as_array().unwrap().len(), 1);
        }
        assert!(before[1 - owner]["decision"].get("cards").is_none());
        apply(
            &mut s,
            json!({"type":"resolve_predict","choice_id":id,"recycle":[],"top_order":[1,0]}),
        );
        assert_eq!(s.game.players[owner].hand, vec![195]);
        assert_eq!(s.game.players[owner].xp, 1);
        assert_eq!(s.game.play_history.gear_activations[owner], uses + 1);
        assert!(s.game.chain.is_empty());
        assert!(s.observe(owner).unwrap()["predict_continuation"].is_null());
    }
}

#[test]
fn c21_bloom_xp_capacity_rolls_back_the_deferred_draw_choice_and_revision() {
    for owner in 0..2 {
        let mut s = session(owner);
        s.game.players[owner].deck = vec![71, 195, 96];
        s.game.players[owner].xp = u64::MAX;
        bloom(&mut s, owner);
        let before = views(&s);
        let full = format!("{:?}", s.game);
        let id = before[owner]["decision"]["choice_id"].as_u64().unwrap();
        let action = json!({"type":"resolve_predict","choice_id":id,"recycle":[0],"top_order":[1]});
        for command in ["validate", "apply"] {
            let r = call(&mut s, command, action.clone());
            assert_eq!(r["ok"], false, "{r}");
            assert_eq!(r["error"]["code"], "engine_capacity", "{r}");
            assert_eq!(views(&s), before);
            assert_eq!(format!("{:?}", s.game), full);
        }
    }
}

#[test]
fn c21_bloom_catalog_reports_all_printed_costs_without_an_equipment_target() {
    let c = catalog();
    let rule = &c["cards"][397]["activated_abilities"][0];
    assert_eq!(rule["energy"], 1);
    assert_eq!(rule["exhaust"], true);
    assert_eq!(rule["kill_source"], true);
    assert_eq!(rule["targets"]["min"], 0);
    assert_eq!(rule["targets"]["max"], 0);
    assert_eq!(rule["adds_resources"], false);
    assert_eq!(rule["timing"], "main");
    for owner in 0..2 {
        assert!(session(owner).observe(owner).unwrap()["predict_continuation"].is_null());
    }
}
