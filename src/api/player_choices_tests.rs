use super::*;
use crate::game::test_support::{position, ready_unit};
use crate::gear::GearState;

#[test]
fn each_player_api_exposes_non_targeting_choices_and_rejects_wrong_or_stale_requests() {
    for actor in 0..2 {
        let mut s = Session::new(ResetOptions::default()).unwrap();
        s.start(0).unwrap();
        s.game = position(actor, 0, 48010);
        s.game.players[actor].hand = vec![480];
        for (owner, player) in s.game.players.iter_mut().enumerate() {
            player.gear.push(GearState {
                id: 60000 + owner as u32,
                card_id: 3,
                lane: None,
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            });
        }
        test_support::apply(
            &mut s,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            },
        );
        for _ in 0..10 {
            if s.game.decision.is_some() {
                break;
            }
            test_support::apply(&mut s, Action::Pass);
        }
        let Some(Decision::PlayerObject { choice_id, .. }) = s.game.decision else {
            panic!("{:?}", s.game.decision)
        };
        let views = test_support::views(&s);
        let decision = &views[actor]["decision"];
        assert_eq!(decision["type"], "player_object");
        assert_eq!(decision["is_target"], false);
        assert_eq!(decision["optional"], false);
        assert_eq!(decision["candidates"].as_array().unwrap().len(), 1);
        let action = Action::ChoosePlayerObject {
            choice_id,
            object: Some(Target::Gear {
                player: actor,
                gear_id: 60000 + actor as u32,
            }),
        };
        assert_eq!(parse_action(&action_to_json(&action)).unwrap(), action);
        let revision = s.revision();
        for bad in [
            json!({"type":"choose_player_object","choice_id":choice_id,"object":null}),
            json!({"type":"choose_player_object","choice_id":choice_id}),
            json!({"type":"choose_player_object","choice_id":choice_id+1,"object":action_to_json(&action)["object"]}),
            json!({"type":"choose_player_object","choice_id":choice_id,"object":action_to_json(&action)["object"],"target":null}),
        ] {
            let result = test_support::request(&mut s, "apply", bad);
            assert_eq!(result["ok"], false);
            assert_eq!(test_support::views(&s), views);
            assert_eq!(s.revision(), revision);
        }
        let wrong=s.handle(json!({"command":"apply","revision":revision,"player":1-actor,"action":action_to_json(&action)}));
        assert_eq!(wrong["ok"], false);
        let valid = test_support::request(&mut s, "validate", action_to_json(&action));
        assert_eq!(valid["ok"], true);
        assert_eq!(test_support::views(&s), views);
        test_support::apply(&mut s, action);
        assert_eq!(s.game.priority(), 1 - actor);
        let after = test_support::views(&s);
        let result = test_support::request(
            &mut s,
            "apply",
            json!({"type":"choose_player_object","choice_id":choice_id,"object":{"type":"gear","player":1-actor,"gear_id":60001-actor}}),
        );
        assert_eq!(result["ok"], false);
        assert_eq!(test_support::views(&s), after);
    }
}

#[test]
fn optional_return_api_updates_the_next_players_options_and_allows_null() {
    for actor in 0..2 {
        let mut s = Session::new(ResetOptions::default()).unwrap();
        s.start(0).unwrap();
        s.game = position(actor, 0, 48110);
        s.game.players[actor].hand = vec![481];
        let a = ready_unit(&mut s.game, actor, 0);
        ready_unit(&mut s.game, 1 - actor, 0);
        test_support::apply(
            &mut s,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            },
        );
        for _ in 0..10 {
            if s.game.decision.is_some() {
                break;
            }
            test_support::apply(&mut s, Action::Pass);
        }
        let Some(Decision::PlayerObject { choice_id, .. }) = s.game.decision else {
            panic!("no return choice")
        };
        assert_eq!(s.game.priority(), 1 - actor);
        let first = s.observe(1 - actor).unwrap();
        assert_eq!(first["decision"]["optional"], true);
        assert_eq!(first["decision"]["candidates"].as_array().unwrap().len(), 2);
        test_support::apply(
            &mut s,
            Action::ChoosePlayerObject {
                choice_id,
                object: Some(Target::Unit {
                    player: actor,
                    unit_id: a,
                }),
            },
        );
        let second = s.observe(actor).unwrap();
        assert_eq!(
            second["decision"]["candidates"].as_array().unwrap().len(),
            1
        );
        let next = second["decision"]["choice_id"].as_u64().unwrap() as u32;
        test_support::apply(
            &mut s,
            Action::ChoosePlayerObject {
                choice_id: next,
                object: None,
            },
        );
        assert!(s.game.decision.is_none());
        assert!(s.game.players[actor].hand.contains(&0));
    }
}
