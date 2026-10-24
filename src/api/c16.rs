//! Wire-level legend choices and public prerequisite progress.
use super::*;
use crate::game::Unit;

fn session(legend: usize) -> Session {
    let mut session = Session::new(ResetOptions {
        custom_decks: [Some(CustomDeck::from_legend(legend).unwrap()), None],
        ..ResetOptions::default()
    })
    .unwrap();
    session.start(0).unwrap();
    for p in &mut session.game.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.runes = [0; 6];
        p.exhausted_runes = [0; 6];
        p.energy = 20;
        p.energy_pool = 20;
        p.power_pool = [3; 6];
    }
    session
}
fn enemy(session: &mut Session) -> Value {
    session.game.players[1].units.push(Unit {
        id: 991,
        card_id: 72,
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
    json!({"type":"unit","player":1,"unit_id":991})
}
fn apply(session: &mut Session, action: Value) -> Value {
    session.handle(json!({"command":"apply","revision":session.revision(),"player":session.game.priority(),"action":action}))
}

#[test]
fn rengar_decision_has_a_legend_source_and_requires_one_target_on_the_wire() {
    let mut s = session(8);
    let target = enemy(&mut s);
    s.game.players[0].hand = vec![72];
    assert_eq!(
        apply(
            &mut s,
            json!({"type":"play_card_targets","hand_index":0,"targets":[]})
        )["ok"],
        true
    );
    let view = s.observe(0).unwrap();
    let d = &view["decision"];
    assert_eq!(d["type"], "legend_targets");
    assert_eq!(d["required"], 1);
    assert_eq!(d["maximum"], 1);
    assert_eq!(
        d["source"],
        json!({"type":"legend","player":0,"legend_id":8})
    );
    assert!(d["candidates"].as_array().unwrap().contains(&target));
    assert_eq!(
        apply(
            &mut s,
            json!({"type":"choose_trigger_targets","targets":[]})
        )["ok"],
        false
    );
    assert_eq!(s.observe(0).unwrap(), view);
    assert_eq!(
        apply(
            &mut s,
            json!({"type":"choose_trigger_targets","targets":[target]})
        )["ok"],
        true
    );
    for viewer in 0..2 {
        let view = s.observe(viewer).unwrap();
        let item = &view["chain"][0];
        assert_eq!(
            item["source"],
            json!({"type":"legend","player":0,"legend_id":8})
        );
        assert_eq!(item["ability"]["type"], "rengar_might");
    }
}

#[test]
fn ezreal_public_progress_and_activation_are_consistent_for_both_viewers() {
    let mut s = session(9);
    let target = enemy(&mut s);
    for value in 0..=2 {
        for viewer in 0..2 {
            assert_eq!(
                s.observe(viewer).unwrap()["players"][0]["legend"]["targeting_progress"],
                json!({"completed":value,"required":2})
            );
        }
        if value < 2 {
            s.game.players[0].hand.push(63);
            assert_eq!(
                apply(
                    &mut s,
                    json!({"type":"play_card_targets","hand_index":0,"targets":[target]})
                )["ok"],
                true
            );
        }
    }
    assert_eq!(apply(&mut s, json!({"type":"activate_legend"}))["ok"], true);
    let view = s.observe(1).unwrap();
    assert_eq!(
        view["chain"].as_array().unwrap().last().unwrap()["ability"]["type"],
        "ezreal_draw"
    );
    assert_eq!(view["players"][0]["legend"]["ready"], false);
    assert!(view["players"][0]["hand"].is_null());
}

#[test]
fn catalog_registers_both_legends_with_legal_independent_training_lists() {
    let mut s = session(9);
    let response = s.handle(json!({"command":"catalog"}));
    let c = &response["result"];
    for (id, chosen) in [(8, 30), (9, 35)] {
        let row = &c["legends"][id];
        assert_eq!(row["default_champion"], chosen);
        let deck = CustomDeck::from_json(&row["training_deck"]).unwrap();
        assert_eq!(deck.legend, id);
    }
    assert_eq!(c["legends"][9]["activation"]["effect"], "draw");
    assert_eq!(c["presets"].as_array().unwrap().len(), 4);
}
