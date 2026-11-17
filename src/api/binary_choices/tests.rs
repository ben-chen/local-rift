use super::*;
use crate::game::test_support::{position, ready_unit};

fn session(actor: usize) -> Session {
    let mut s = Session::new(ResetOptions::default()).unwrap();
    s.game = position(actor, 0, 486040);
    s.game.legend[actor].ready = true;
    s.game.players[actor].power_pool = [0; 6];
    s.game.players[actor].hand = vec![485];
    let unit_id = ready_unit(&mut s.game, actor, 71);
    s.game.players[actor].units[0].ready = false;
    super::super::test_support::apply(
        &mut s,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: actor,
                unit_id,
            }],
        },
    );
    s
}

fn query(s: &mut Session, actor: usize) -> Value {
    s.handle(json!({"command":"binary_choices","player":actor,"revision":s.revision()}))
}

#[test]
fn binary_choices_api_reports_exact_answers_and_validates_without_mutation() {
    for actor in 0..2 {
        let mut s = session(actor);
        let before = super::super::test_support::views(&s);
        let revision = s.revision();
        let reply = query(&mut s, actor);
        assert_eq!(reply["ok"], true, "{reply}");
        assert_eq!(reply["result"]["scope"], "choose");
        assert_eq!(reply["result"]["exhaustive"], true);
        let options = reply["result"]["options"].as_array().unwrap();
        assert_eq!(options.len(), 2);
        assert_eq!(options[0]["enabled"], false);
        assert_eq!(options[1]["enabled"], true);
        for option in options {
            let validation = s.handle(json!({"command":"validate","player":actor,
                "revision":revision,"action":option["action"]}));
            assert_eq!(validation["ok"], option["enabled"]);
        }
        assert_eq!(super::super::test_support::views(&s), before);
        assert_eq!(s.revision(), revision);
        let applied = s.handle(json!({"command":"apply","player":actor,
            "revision":revision,"action":options[1]["action"]}));
        assert_eq!(applied["ok"], true, "{applied}");
        assert_eq!(s.revision(), revision + 1);
    }
}

#[test]
fn binary_choices_api_rejects_stale_other_seat_and_unrelated_questions() {
    for actor in 0..2 {
        let mut s = session(actor);
        let before = super::super::test_support::views(&s);
        let revision = s.revision();
        for (player, rev) in [(1 - actor, revision), (actor, revision + 1)] {
            let reply =
                s.handle(json!({"command":"binary_choices","player":player,"revision":rev}));
            assert_eq!(reply["ok"], false);
            assert!(reply.get("result").is_none());
            assert_eq!(super::super::test_support::views(&s), before);
            assert_eq!(s.revision(), revision);
        }
        s.game.decision = Some(Decision::Predict {
            player: actor,
            card_id: 485,
        });
        let reply = query(&mut s, 1 - actor);
        assert_eq!(reply["ok"], false);
        assert!(!reply.to_string().contains("Friendship"));
        s.game.decision = Some(Decision::GrantFlow { player: actor });
        let reply = query(&mut s, actor);
        assert_eq!(reply["ok"], false);
        assert!(
            reply["error"]["message"]
                .as_str()
                .unwrap()
                .contains("binary")
        );
    }
}

#[test]
fn binary_choices_api_accept_supports_explicit_power_and_preserves_response_window() {
    use super::super::test_support::{apply, request, views};
    for actor in 0..2 {
        let mut s = session(actor);
        s.game.players[actor].any_power = 1;
        let reply = query(&mut s, actor);
        assert_eq!(reply["result"]["options"][0]["enabled"], true);
        let action = reply["result"]["options"][0]["action"].clone();
        // The answer must not reveal an opponent's unseen hand or deck order.
        s.game.players[1 - actor].hand = vec![485, 484];
        s.game.players[1 - actor].deck.reverse();
        assert_eq!(query(&mut s, actor), reply);
        let before = views(&s);
        let quote = request(&mut s, "quote_payment", action.clone());
        assert_eq!(quote["ok"], true, "{quote}");
        assert_eq!(quote["result"]["suggested"]["any"], 1);
        assert_eq!(views(&s), before);
        let paid = request(
            &mut s,
            "apply",
            json!({
                "type":"with_power_payment", "action":action,
                "payment":quote["result"]["suggested"]
            }),
        );
        assert_eq!(paid["ok"], true, "{paid}");
        assert!(!s.game.legend[actor].ready);
        assert!(!s.game.players[actor].units[0].ready);
        assert_eq!(s.game.players[actor].any_power, 0);
        assert_eq!(s.game.chain.len(), 2);
        apply(&mut s, Action::Pass);
        apply(&mut s, Action::Pass);
        assert!(s.game.players[actor].units[0].ready);
    }
}
