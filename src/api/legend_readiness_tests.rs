//! A legend target finalizes before responses; its new state is chosen afterward.
use super::test_support::{apply, request, views};
use super::*;
use crate::game::test_support::position;

fn session(actor: usize) -> Session {
    let mut s = Session::new(ResetOptions::default()).unwrap();
    s.game = position(actor, 0, 455002);
    s.game.players[actor].hand = vec![455];
    apply(
        &mut s,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        },
    );
    s
}

#[test]
fn entourage_api_target_query_and_resolution_choices_are_public_and_read_only() {
    for actor in 0..2 {
        for owner in 0..2 {
            let mut s = session(actor);
            let target = Target::Legend {
                player: owner,
                legend_id: s.game.legend[owner].kind.id(),
            };
            let encoded = target_to_json(&target);
            let before = views(&s);
            let revision = s.revision();
            let response = s.handle(
                json!({"command":"trigger_target_candidates", "player":actor,
                "revision":revision,"targets":[]}),
            );
            assert_eq!(response["ok"], true, "{response}");
            assert_eq!(views(&s), before);
            assert_eq!(s.revision(), revision);
            let choice = &before[actor]["decision"];
            assert_eq!(choice["type"], "trigger_targets");
            assert!(choice["candidates"].as_array().unwrap().contains(&encoded));
            assert_eq!(choice["candidates"].as_array().unwrap().len(), 2);
            let select = json!({"type":"choose_trigger_targets","targets":[encoded]});
            assert_eq!(request(&mut s, "apply", select)["ok"], true);
            assert!(!s.game.legend[owner].ready);
            assert!(s.game.decision.is_none());
            apply(&mut s, Action::Pass);
            apply(&mut s, Action::Pass);
            let before = views(&s);
            let choice = &before[actor]["decision"];
            assert_eq!(choice["type"], "legend_readiness");
            assert_eq!(choice["target"], encoded);
            assert_eq!(choice["timing"], "resolution");
            assert_eq!(choice["currently_ready"], false);
            assert_eq!(before[1 - actor]["decision"], *choice);
            let action =
                json!({"type":"resolve_readiness", "choice_id":choice["choice_id"], "ready":true});
            let revision = s.revision();
            assert_eq!(request(&mut s, "validate", action.clone())["ok"], true);
            assert_eq!(views(&s), before);
            assert_eq!(s.revision(), revision);
            assert_eq!(request(&mut s, "apply", action)["ok"], true);
            assert!(s.game.legend[owner].ready);
            assert!(s.game.chain.is_empty());
            assert!(s.game.decision.is_none());
        }
    }
}

#[test]
fn entourage_api_rejects_wrong_seat_stale_and_mid_resolution_actions_atomically() {
    for actor in 0..2 {
        let mut s = session(actor);
        let target = Target::Legend {
            player: actor,
            legend_id: s.game.legend[actor].kind.id(),
        };
        apply(
            &mut s,
            Action::ChooseTriggerTargets {
                targets: vec![target],
            },
        );
        apply(&mut s, Action::Pass);
        apply(&mut s, Action::Pass);
        let Some(Decision::LegendReadiness { choice_id, .. }) = s.game.decision else {
            panic!("missing choice")
        };
        let action = json!({"type":"resolve_readiness","choice_id":choice_id,"ready":false});
        let before = views(&s);
        let revision = s.revision();
        for (player, rev, action) in [
            (1 - actor, revision, action.clone()),
            (actor, revision + 1, action.clone()),
            (
                actor,
                revision,
                json!({"type":"resolve_readiness","choice_id":choice_id+1,"ready":true}),
            ),
            (actor, revision, json!({"type":"pass"})),
            (
                actor,
                revision,
                json!({"type":"resolve_readiness","choice_id":choice_id}),
            ),
            (actor, revision, json!({"type":"choose","accept":false})),
            (
                actor,
                revision,
                json!({"type":"resolve_readiness","choice_id":choice_id,"ready":"true"}),
            ),
        ] {
            let result =
                s.handle(json!({"command":"apply","player":player,"revision":rev,"action":action}));
            assert_eq!(result["ok"], false, "{result}");
            assert_eq!(views(&s), before);
            assert_eq!(s.revision(), revision);
        }
        assert_eq!(
            request(&mut s, "apply", action.clone())["ok"],
            true,
            "exhausting an exhausted legend is legal"
        );
        let before = views(&s);
        let revision = s.revision();
        assert_eq!(request(&mut s, "apply", action)["ok"], false);
        assert_eq!(views(&s), before);
        assert_eq!(s.revision(), revision);
    }
}
