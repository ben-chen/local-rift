use super::test_support::{apply, request, views};
use super::*;
use crate::game::test_support::{decline_legend_action, position, ready_unit};

#[test]
fn movement_resolution_api_preserves_declaration_checks_and_ready_after_response() {
    for actor in 0..2 {
        let mut session = Session {
            game: position(actor, 0, 486105),
            ..Session::default()
        };
        let unit_id = ready_unit(&mut session.game, actor, 71);
        session.game.players[actor].units[0].ready = false;
        session.game.players[actor].units[0].lane = Some(0);
        session.game.battlefield_owner[0] = Some(actor);
        session.game.players[actor].hand = vec![14, 9];
        let target = Target::Unit {
            player: actor,
            unit_id,
        };
        let before = views(&session);
        let revision = session.revision();
        for (lane, expected) in [(Some(0), false), (None, true)] {
            let draft = Action::PlayCardWithTargets {
                hand_index: 0,
                lane,
                targets: vec![],
            };
            let query = session.handle(json!({
                "command":"target_candidates", "revision":revision,
                "player":actor, "action":action_to_json(&draft), "execution":0,
            }));
            assert_eq!(query["ok"], true, "{query}");
            assert_eq!(
                query["result"]["candidates"]
                    .as_array()
                    .unwrap()
                    .contains(&target_to_json(&target)),
                expected
            );
            let action = Action::PlayCardWithTargets {
                hand_index: 0,
                lane,
                targets: vec![target],
            };
            let validation = request(&mut session, "validate", action_to_json(&action));
            assert_eq!(validation["ok"], expected, "{validation}");
            if !expected {
                let rejected = request(&mut session, "apply", action_to_json(&action));
                assert_eq!(rejected["ok"], false);
            }
            assert_eq!(session.revision(), revision);
            assert_eq!(views(&session), before);
        }
        apply(
            &mut session,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![target],
            },
        );
        while let Some(action) = decline_legend_action(&session.game) {
            apply(&mut session, action);
        }
        if session.game.priority() != actor {
            apply(&mut session, Action::Pass);
        }
        apply(
            &mut session,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![target],
            },
        );
        for _ in 0..30 {
            if session.game.chain.is_empty() && session.game.decision.is_none() {
                break;
            }
            let action = decline_legend_action(&session.game).unwrap_or(Action::Pass);
            apply(&mut session, action);
        }
        assert!(session.game.chain.is_empty());
        assert!(session.game.decision.is_none());
        let unit = &session.game.players[actor].units[0];
        assert_eq!(unit.id, unit_id);
        assert_eq!(unit.lane, None);
        assert!(unit.ready);
        assert_eq!(session.game.players[actor].discard.len(), 2);
    }
}
