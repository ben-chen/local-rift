use super::*;
use crate::game::test_support::{position, ready_unit};
use test_support::{apply, views};

#[test]
fn party_favors_api_exposes_both_answers_and_rejects_wrong_actor_stale_and_pass() {
    for actor in 0..2 {
        for cards in [false, true] {
            let mut s = Session {
                game: position(actor, 0, 494201),
                ..Session::default()
            };
            s.game.players[actor].hand = vec![494];
            apply(
                &mut s,
                Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                },
            );
            for _ in 0..20 {
                if matches!(s.game.decision, Some(Decision::SharedResources { .. })) {
                    break;
                }
                apply(&mut s, Action::Pass);
            }
            let before = views(&s);
            let revision = s.revision();
            for view in &before {
                assert_eq!(view["decision"]["type"], "shared_resources");
                assert_eq!(view["decision"]["player"], 1 - actor);
                assert_eq!(view["decision"]["controller"], actor);
                assert_eq!(view["decision"]["optional"], false);
                assert_eq!(view["decision"]["runes"]["exhausted"], true);
            }
            let choices =
                s.handle(json!({"command":"binary_choices","player":1-actor,"revision":revision}));
            assert_eq!(choices["ok"], true, "{choices}");
            let options = choices["result"]["options"].as_array().unwrap();
            assert_eq!(options.len(), 2);
            assert!(options.iter().all(|option| option["enabled"] == true));
            for option in options {
                let result = s.handle(json!({"command":"validate","player":1-actor,"revision":revision,"action":option["action"]}));
                assert_eq!(result["ok"], true, "{result}");
            }
            for (player, rev, action) in [
                (actor, revision, json!({"type":"choose","accept":cards})),
                (
                    1 - actor,
                    revision + 1,
                    json!({"type":"choose","accept":cards}),
                ),
                (1 - actor, revision, json!({"type":"pass"})),
            ] {
                let result = s.handle(
                    json!({"command":"apply","player":player,"revision":rev,"action":action}),
                );
                assert_eq!(result["ok"], false, "{result}");
                assert_eq!(views(&s), before);
                assert_eq!(s.revision(), revision);
            }
            let result=s.handle(json!({"command":"apply","player":1-actor,"revision":revision,"action":{"type":"choose","accept":cards}}));
            assert_eq!(result["ok"], true, "{result}");
            assert_eq!(s.revision(), revision + 1);
        }
    }
}

#[test]
fn ki_burst_api_reports_required_friendly_exhaust_cost_and_rejects_enemy_cost() {
    for actor in 0..2 {
        let mut s = Session {
            game: position(actor, 0, 491201),
            ..Session::default()
        };
        s.game.players[actor].hand = vec![491];
        let friendly = ready_unit(&mut s.game, actor, 71);
        let enemy = ready_unit(&mut s.game, 1 - actor, 71);
        let action = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: 1 - actor,
                unit_id: enemy,
            }],
        };
        let before = views(&s);
        let revision = s.revision();
        let query=s.handle(json!({"command":"play_choices","player":actor,"revision":revision,"action":action_to_json(&action)}));
        assert_eq!(query["ok"], true, "{query}");
        assert_eq!(query["result"]["unit_cost"]["optional"], false, "{query}");
        assert_eq!(
            query["result"]["unit_cost"]["candidates"],
            json!([friendly]),
            "{query}"
        );
        assert_eq!(views(&s), before);
        let paid = |unit| Action::WithPlayChoices {
            action: Box::new(action.clone()),
            choices: crate::play::PlayChoices {
                exhaust_unit: Some(unit),
                ..Default::default()
            },
        };
        let bad=s.handle(json!({"command":"apply","player":actor,"revision":revision,"action":action_to_json(&paid(enemy))}));
        assert_eq!(bad["ok"], false);
        assert_eq!(views(&s), before);
        apply(&mut s, paid(friendly));
        assert_eq!(s.game.players[actor].energy, 19);
        assert!(!s.game.players[actor].units[0].ready);
    }
}
