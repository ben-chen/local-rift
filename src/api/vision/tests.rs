//! Controlled positions exercise the real JSON command/view boundary.
use super::*;
use crate::game::Unit;
use crate::gear::GearState;

fn position(owner: usize) -> Session {
    let mut session = Session::new(ResetOptions {
        decks: [3, 3],
        first_player: owner,
        ..Default::default()
    })
    .unwrap();
    session.start(0).unwrap();
    session.game.battlefields = [7, 9];
    session.game.battlefield_owner = [None; 2];
    for player in &mut session.game.players {
        player.units.clear();
        player.gear.clear();
        player.hand.clear();
        player.discard.clear();
        player.hidden.clear();
        player.champion = None;
        player.deck = vec![71, 70, 133];
        player.runes = [0; 6];
        player.exhausted_runes = [0; 6];
        player.energy = 40;
        player.energy_pool = 40;
        player.power_pool = [8; 6];
    }
    for legend in &mut session.game.legend {
        legend.ready = false;
    }
    session.game.trigger_state.draws = [0; 2];
    session
}

fn unit(id: u32, card_id: usize, token: bool) -> Unit {
    Unit {
        id,
        card_id,
        lane: None,
        ready: true,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token,
        protected: false,
        stunned: false,
        banish_on_death: false,
    }
}

fn apply(session: &mut Session, action: Value) {
    let result = session.handle(json!({"command":"apply","revision":session.revision(),
        "player":session.game.priority(),"action":action}));
    assert_eq!(
        result["ok"], true,
        "action={action} decision={:?} {result}",
        session.game.decision
    );
}

fn views(session: &Session) -> [Value; 2] {
    [session.observe(0).unwrap(), session.observe(1).unwrap()]
}

fn advance_to_predict(session: &mut Session) {
    for _ in 0..40 {
        match session.observe(session.game.priority()).unwrap()["decision"].clone() {
            Value::Object(decision) if decision["type"] == "predict" => return,
            Value::Object(decision) if decision["type"] == "order_triggers" => {
                let id = decision["options"][0]["id"].clone();
                apply(session, json!({"type":"order_trigger","trigger_id":id}));
            }
            Value::Null => apply(session, json!({"type":"pass"})),
            unexpected => panic!("Unexpected choice: {unexpected}"),
        }
    }
    panic!("Vision did not reach its private choice");
}

#[test]
fn live_instances_expose_actual_bearers_and_public_grant_provenance_from_both_seats() {
    for owner in 0..2 {
        let mut session = position(owner);
        session.game.players[owner].units = vec![
            unit(22001, 398, false),
            unit(22002, 399, false),
            unit(22003, 400, false),
            unit(22004, 138, true),
            unit(22005, 34, true),
        ];
        session.game.players[owner].units[0].lane = Some(0);
        session.game.players[owner].units[1].lane = Some(1);
        session.game.players[1 - owner].units = vec![unit(22006, 399, false)];
        let observations = views(&session);
        for (first, second) in observations[0]["players"]
            .as_array()
            .unwrap()
            .iter()
            .zip(observations[1]["players"].as_array().unwrap())
        {
            assert_eq!(first["units"], second["units"]);
        }
        let units = observations[owner]["players"][owner]["units"]
            .as_array()
            .unwrap();
        for (index, expected) in [1, 2, 2, 2, 1].into_iter().enumerate() {
            assert_eq!(units[index]["keywords"]["vision"], expected);
            assert_eq!(
                units[index]["keywords"]["vision_exact"],
                expected.to_string()
            );
            let instances = units[index]["vision_instances"].as_array().unwrap();
            assert_eq!(instances.len(), expected as usize);
            for instance in instances {
                assert_eq!(
                    instance["source"],
                    json!({"type":"unit","player":owner,"unit_id":22001+index})
                );
                assert_eq!(instance["active"], true);
            }
        }
        assert_eq!(units[3]["name"], "Mech token");
        assert_eq!(units[3]["tags"], json!(["Mech"]));
        assert_eq!(units[4]["tags"], json!(["Bird"]));
        let origins: Vec<_> = units[3]["vision_instances"]
            .as_array()
            .unwrap()
            .iter()
            .map(|instance| instance["origin"]["card_id"].as_u64().unwrap())
            .collect();
        assert_eq!(origins, vec![398, 399]);
        assert!(
            units[3]["vision_instances"]
                .as_array()
                .unwrap()
                .iter()
                .all(|instance| instance["origin"]["type"] == "granted")
        );
        assert_eq!(
            observations[owner]["players"][1 - owner]["units"][0]["keywords"]["vision"],
            1
        );
    }
}

#[test]
fn each_vision_remains_a_separate_private_predict_with_no_draw_or_invalid_mutation() {
    for owner in 0..2 {
        let mut session = position(owner);
        session.game.players[owner]
            .units
            .push(unit(22101, 398, false));
        session.game.players[owner].hand = vec![400];
        apply(&mut session, json!({"type":"play_card","hand_index":0}));
        for _ in 0..2 {
            advance_to_predict(&mut session);
            let before = views(&session);
            assert_eq!(before[owner]["decision"]["card_id"], 133);
            assert_eq!(
                before[1 - owner]["decision"],
                json!({"type":"predict","player":owner})
            );
            assert_eq!(before[owner]["predict_continuation"], Value::Null);
            let mut alternate = session.clone();
            *alternate.game.players[owner].deck.last_mut().unwrap() = 71;
            let Some(Decision::Predict { card_id, .. }) = alternate.game.decision.as_mut() else {
                panic!()
            };
            *card_id = 71;
            assert_eq!(alternate.observe(1 - owner).unwrap(), before[1 - owner]);
            assert_ne!(alternate.observe(owner).unwrap(), before[owner]);
            for (player, revision) in [
                (1 - owner, session.revision()),
                (owner, session.revision() - 1),
            ] {
                let rejected = session.handle(json!({"command":"apply","player":player,
                    "revision":revision,"action":{"type":"choose","accept":false}}));
                assert_eq!(rejected["ok"], false);
                assert_eq!(views(&session), before);
            }
            apply(&mut session, json!({"type":"choose","accept":false}));
        }
        assert!(session.game.decision.is_none());
        assert_eq!(session.game.players[owner].deck, vec![71, 70, 133]);
        assert_eq!(session.game.trigger_state.draws[owner], 0);
    }
}

#[test]
fn intrinsic_gear_characteristic_and_filtered_catalog_rules_are_unambiguous() {
    for owner in 0..2 {
        let mut session = position(owner);
        session.game.players[owner]
            .units
            .push(unit(22201, 398, false));
        session.game.players[owner].gear.push(GearState {
            id: 22202,
            card_id: 242,
            attached_to: None,
            ready: true,
            temporary: false,
            token: false,
        });
        let observed = views(&session);
        for view in &observed {
            let instances = &view["players"][owner]["gear"][0]["vision_instances"];
            assert_eq!(
                instances,
                &json!([{"source":{"type":"gear","player":owner,"gear_id":22202},
                "origin":{"type":"printed"},"active":true}])
            );
        }
        // A controlled attached-state fixture distinguishes a characteristic
        // from active rules; it does not claim Shells has an Equip action.
        session.game.players[owner].gear[0].attached_to = Some(22201);
        for view in views(&session) {
            let instances = view["players"][owner]["gear"][0]["vision_instances"]
                .as_array()
                .unwrap();
            assert_eq!(instances.len(), 1);
            assert_eq!(instances[0]["active"], false);
        }
    }
    let catalog = catalog();
    let seer = &catalog["cards"][398]["passive_rules"][0];
    assert_eq!(seer["recipients"], "other_friendly_units");
    assert!(seer.get("recipient_tag").is_none());
    assert_eq!(seer["effect"], json!({"type":"keyword","keyword":"vision"}));
    let forecaster = &catalog["cards"][399]["passive_rules"][0];
    assert_eq!(forecaster["recipients"], "friendly_units_with_tag");
    assert_eq!(forecaster["recipient_tag"], "Mech");
    for card in catalog["cards"].as_array().unwrap().iter().take(398) {
        for rule in card["passive_rules"].as_array().unwrap() {
            assert!(rule.get("recipient_tag").is_none());
        }
    }
}

#[test]
fn queued_vision_keeps_bearer_and_grant_origin_after_a_real_return_reaction() {
    for owner in 0..2 {
        let mut session = position(owner);
        session.game.players[owner]
            .units
            .push(unit(22301, 398, false));
        session.game.players[1 - owner]
            .units
            .push(unit(22302, 71, false));
        session.game.players[owner].hand = vec![400, 16];
        apply(&mut session, json!({"type":"play_card","hand_index":0}));
        for _ in 0..40 {
            let observed = session.observe(owner).unwrap();
            if observed["decision"].is_null()
                && observed["chain"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|item| item["ability"]["type"] == "vision")
                    .count()
                    == 2
            {
                break;
            }
            match &observed["decision"] {
                Value::Object(decision) if decision["type"] == "order_triggers" => {
                    apply(
                        &mut session,
                        json!({"type":"order_trigger","trigger_id":decision["options"][0]["id"]}),
                    );
                }
                Value::Null => apply(&mut session, json!({"type":"pass"})),
                other => panic!("Unexpected pre-resolution choice: {other}"),
            }
        }
        let entered = session.game.players[owner]
            .units
            .iter()
            .find(|unit| unit.card_id == 400)
            .unwrap()
            .id;
        let before = views(&session);
        assert_eq!(before[0]["chain"], before[1]["chain"]);
        let chain = before[owner]["chain"].as_array().unwrap();
        assert_eq!(chain.len(), 2);
        let mut origins = Vec::new();
        for item in chain {
            let bearer = json!({"type":"unit","player":owner,"unit_id":entered});
            assert_eq!(item["source"], bearer);
            assert_eq!(item["ability"]["source"], bearer);
            assert_eq!(item["ability"]["type"], "vision");
            assert_eq!(item["ability"]["source_name"], "Jeweled Colossus");
            let origin = &item["ability"]["origin"];
            origins.push(origin["type"].as_str().unwrap().to_owned());
            if origin["type"] == "granted" {
                assert_eq!(
                    origin["source"],
                    json!({"type":"unit","player":owner,"unit_id":22301})
                );
                assert_eq!(origin["card_id"], 398);
                assert_eq!(origin["rule_index"], 0);
            }
        }
        origins.sort();
        assert_eq!(origins, ["granted", "printed"]);
        if session.game.priority() != owner {
            apply(&mut session, json!({"type":"pass"}));
        }
        apply(
            &mut session,
            json!({"type":"play_card_targets","hand_index":0,"targets":[
                {"type":"unit","player":owner,"unit_id":22301},
                {"type":"unit","player":1-owner,"unit_id":22302}
            ]}),
        );
        for _ in 0..10 {
            if session.game.players[owner]
                .units
                .iter()
                .all(|unit| unit.id != 22301)
            {
                break;
            }
            apply(&mut session, json!({"type":"pass"}));
        }
        assert!(session.game.players[owner].hand.contains(&398));
        let after = views(&session);
        for view in &after {
            assert_eq!(view["chain"], before[owner]["chain"]);
            assert_eq!(view["players"][owner]["units"][0]["keywords"]["vision"], 1);
        }
        for _ in 0..2 {
            advance_to_predict(&mut session);
            apply(&mut session, json!({"type":"choose","accept":false}));
        }
        assert_eq!(session.game.trigger_state.draws[owner], 0);
    }
}
