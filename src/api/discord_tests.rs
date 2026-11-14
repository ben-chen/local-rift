use super::*;
use crate::game::test_support::{decline_legend_action, position, ready_unit};

#[test]
fn discord_api_queries_and_subset_actions_preserve_actor_revision_and_original_targets() {
    for actor in 0..2 {
        let mut s = Session::new(ResetOptions::default()).unwrap();
        s.start(0).unwrap();
        s.game = position(actor, 0, 465100);
        s.game.players[actor].hand = vec![465];
        let two = Target::Unit {
            player: 1 - actor,
            unit_id: ready_unit(&mut s.game, 1 - actor, 88),
        };
        let three = Target::Unit {
            player: 1 - actor,
            unit_id: ready_unit(&mut s.game, 1 - actor, 89),
        };
        let one = Target::Unit {
            player: 1 - actor,
            unit_id: ready_unit(&mut s.game, 1 - actor, 34),
        };
        let action = |targets: Vec<Target>| Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets,
        };
        let before = test_support::views(&s);
        let query = s.handle(json!({"command":"target_candidates","player":actor,"revision":s.revision(),"action":action_to_json(&action(vec![two,three])),"execution":0}));
        assert_eq!(query["ok"], true, "{query}");
        assert_eq!(query["result"]["candidates"], json!([]));
        assert_eq!(test_support::views(&s), before);
        let bad = test_support::request(
            &mut s,
            "apply",
            action_to_json(&action(vec![two, three, one])),
        );
        assert_eq!(bad["ok"], false);
        assert_eq!(test_support::views(&s), before);
        test_support::apply(&mut s, action(vec![two, three]));
        if s.game.priority() == actor {
            test_support::apply(&mut s, Action::Pass);
        }
        assert_eq!(s.game.priority(), 1 - actor);
        s.game.players[1 - actor].hand = vec![7];
        test_support::apply(&mut s, action(vec![two]));
        for _ in 0..60 {
            if matches!(s.game.decision, Some(Decision::ResolutionSubset { .. })) {
                break;
            }
            let action = decline_legend_action(&s.game).unwrap_or(Action::Pass);
            test_support::apply(&mut s, action);
        }
        assert!(matches!(
            s.game.decision,
            Some(Decision::ResolutionSubset { .. })
        ));
        let views = test_support::views(&s);
        let serialized = views[actor].to_string();
        assert!(
            serialized.contains("resolution_subset")
                && serialized.contains("requires_maximal_subset")
        );
        let revision = s.revision();
        let choose = Action::ChooseResolutionSubset {
            targets: vec![three],
        };
        assert_eq!(parse_action(&action_to_json(&choose)).unwrap(), choose);
        let wrong = s.handle(json!({"command":"apply","player":1-actor,"revision":revision,"action":action_to_json(&choose)}));
        assert_eq!(wrong["ok"], false);
        let omitted = test_support::request(
            &mut s,
            "apply",
            json!({"type":"choose_resolution_subset","targets":[]}),
        );
        assert_eq!(omitted["ok"], false);
        assert_eq!(s.revision(), revision);
        assert_eq!(test_support::views(&s), views);
        let valid = test_support::request(&mut s, "validate", action_to_json(&choose));
        assert_eq!(valid["ok"], true, "{valid}");
        assert_eq!(test_support::views(&s), views);
        test_support::apply(&mut s, choose);
        assert_eq!(s.revision(), revision + 1);
        assert!(s.game.players[1 - actor].hand.contains(&89));
        assert!(
            s.game.players[1 - actor]
                .units
                .iter()
                .any(|u| u.card_id == 88)
        );
        assert!(
            s.game.players[1 - actor]
                .units
                .iter()
                .any(|u| u.card_id == 34)
        );
    }
}
