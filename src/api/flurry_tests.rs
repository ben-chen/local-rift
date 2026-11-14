//! Both seats use the same public mode, target and token interfaces as the UI.
use super::*;

fn position(actor: usize) -> Session {
    let mut s = Session::new(ResetOptions::default()).unwrap();
    s.start(0).unwrap();
    s.game = crate::game::test_support::position(actor, 0, 463100);
    s.game.players[actor].hand = vec![463];
    s
}

fn request(s: &mut Session, command: &str, action: Value) -> Value {
    s.handle(json!({"command":command,"revision":s.revision(),"player":s.game.priority(),"action":action}))
}

fn play(mode: usize, targets: Value) -> Value {
    json!({"type":"with_play_choices","action":{"type":"play_card_targets","hand_index":0,"targets":targets},"choices":{"mode":mode}})
}

fn pass_to_choice(s: &mut Session) {
    for _ in 0..20 {
        if s.game.decision.is_some() || s.game.chain.is_empty() {
            return;
        }
        assert_eq!(request(s, "apply", json!({"type":"pass"}))["ok"], true);
    }
    panic!("chain did not settle");
}

#[test]
fn flurry_api_mode_queries_are_read_only_and_birds_have_public_token_identity() {
    for actor in 0..2 {
        let mut s = position(actor);
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        let revision = s.revision();
        let draft = play(1, json!([]));
        let modes = request(&mut s, "play_choices", draft.clone());
        assert_eq!(modes["ok"], true, "{modes}");
        assert_eq!(
            modes["result"]["modes"][0]["target_kinds"],
            json!(["spell"])
        );
        assert_eq!(modes["result"]["modes"][1]["target_kinds"], json!([]));
        assert_eq!(request(&mut s, "validate", draft.clone())["ok"], true);
        assert_eq!(s.revision(), revision);
        assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
        let invalid = request(&mut s, "apply", play(0, json!([])));
        assert_eq!(invalid["ok"], false);
        assert_eq!(s.revision(), revision);
        assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
        assert_eq!(request(&mut s, "apply", draft)["ok"], true);
        pass_to_choice(&mut s);
        for _ in 0..4 {
            let result = request(&mut s, "apply", json!({"type":"place_token","lane":null}));
            assert_eq!(result["ok"], true, "{result}");
            pass_to_choice(&mut s);
        }
        for viewer in 0..2 {
            let view = s.observe(viewer).unwrap();
            let units = view["players"][actor]["units"].as_array().unwrap();
            assert_eq!(units.len(), 4);
            for unit in units {
                assert_eq!(unit["token"], true);
                assert_eq!(unit["name"], "Bird token");
                assert_eq!(unit["might"], 1);
                assert_eq!(unit["keywords"]["deflect"], 1);
                assert_eq!(unit["tags"], json!(["Bird"]));
            }
        }
    }
}

#[test]
fn flurry_api_counter_targets_and_finalized_mode_survive_wire_roundtrip() {
    for actor in 0..2 {
        let mut s = position(actor);
        s.game.players[actor].hand = vec![78, 463];
        assert_eq!(
            request(
                &mut s,
                "apply",
                json!({"type":"play_card_targets","hand_index":0,"targets":[]})
            )["ok"],
            true
        );
        if s.game.priority() != actor {
            assert_eq!(request(&mut s, "apply", json!({"type":"pass"}))["ok"], true);
        }
        let id = s.game.chain[0].id;
        let draft = play(0, json!([]));
        let before = s.observe(actor).unwrap();
        let query = s.handle(json!({"command":"target_candidates","player":actor,"revision":s.revision(),"action":draft,"execution":0}));
        assert_eq!(query["ok"], true, "{query}");
        assert_eq!(query["result"]["next_slot"]["kind"], "spell");
        assert_eq!(
            query["result"]["candidates"],
            json!([{"type":"spell","chain_id":id}])
        );
        assert_eq!(s.observe(actor).unwrap(), before);
        let selected = play(0, json!([{"type":"spell","chain_id":id}]));
        assert_eq!(request(&mut s, "apply", selected)["ok"], true);
        assert_eq!(s.game.chain.last().unwrap().executions[0].mode, Some(0));
        pass_to_choice(&mut s);
        assert!(s.game.chain.is_empty());
        assert_eq!(s.game.players[actor].discard.len(), 2);
    }
}
