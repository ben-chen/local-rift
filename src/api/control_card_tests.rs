use super::*;
use crate::game::test_support::{decline_legend_action, position, ready_unit};

#[test]
fn control_card_json_choices_and_owner_views_match_actual_resolution() {
    for actor in 0..2 {
        let mut session = Session::new(ResetOptions::default()).unwrap();
        session.start(0).unwrap();
        session.game = position(actor, 0, 469100);
        session.game.players[actor].hand = vec![469];
        session.game.players[actor].xp = 5;
        let unit_id = ready_unit(&mut session.game, 1 - actor, 71);
        session.game.players[1 - actor]
            .units
            .last_mut()
            .unwrap()
            .lane = Some(0);
        let target = Target::Unit {
            player: 1 - actor,
            unit_id,
        };
        let revision = session.revision();
        let draft = json!({"type":"with_play_choices", "action":{
            "type":"play_card_targets", "hand_index":0, "lane":null, "targets":[]
        }, "choices":{"bonus":true}});
        let action = parse_action(&draft).unwrap();
        let before = format!("{:?}", session.game);
        let view = session
            .target_candidates(actor, revision, action.clone(), 0)
            .unwrap();
        assert_eq!(view.candidates, vec![target]);
        let costs = session.play_choices(actor, revision, action).unwrap();
        assert_eq!(costs.costs.xp, 5);
        assert_eq!(session.revision(), revision);
        assert_eq!(format!("{:?}", session.game), before);
        let mut chosen = draft;
        chosen["action"]["targets"] = json!([target_to_json(&target)]);
        let result = session.handle(json!({"command":"apply", "revision":revision,
            "player":actor, "action":chosen}));
        assert_eq!(result["ok"], true, "{result}");
        for _ in 0..40 {
            if session.game.chain.is_empty() && session.game.decision.is_none() {
                break;
            }
            let action = if let Some(action) = decline_legend_action(&session.game) {
                action
            } else if let Some(Decision::OrderTriggers { options, .. }) = &session.game.decision {
                Action::OrderTrigger {
                    trigger_id: options[0].id,
                }
            } else {
                assert!(session.game.decision.is_none());
                Action::Pass
            };
            let result = session.handle(json!({"command":"apply", "revision":session.revision(),
                "player":session.game.priority(), "action":action_to_json(&action)}));
            assert_eq!(result["ok"], true, "{result}");
        }
        assert!(session.game.chain.is_empty() && session.game.decision.is_none());
        for viewer in 0..2 {
            let observation = session.observe(viewer).unwrap();
            let unit = &observation["players"][actor]["units"][0];
            assert_eq!(unit["id"], unit_id);
            assert_eq!(unit["owner"], 1 - actor);
            assert_eq!(unit["ready"], false);
            assert!(unit["lane"].is_null());
            assert_eq!(observation["players"][actor]["xp"], 0);
            assert!(
                observation["players"][1 - actor]["units"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
    }
}
