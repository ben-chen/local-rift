use super::*;
use crate::game::test_support::{combat_position, position, ready_unit};
use test_support::{apply, views};

#[test]
fn thwonk_api_offers_friendly_attackers_and_preserves_read_only_queries() {
    for actor in 0..2 {
        let (mut game, attacker, defender) = combat_position(actor, 48410);
        game.players[actor].hand = vec![484];
        let mut s = Session {
            game,
            revision: 0,
            series: None,
            format_date: None,
        };
        let action = |targets| Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets,
        };
        let before = views(&s);
        let response = s.handle(
            json!({"command":"target_candidates","player":actor,"revision":0,
            "action":action_to_json(&action(vec![])),"execution":0}),
        );
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(
            targeting::play_targets_to_json(484)["condition"],
            json!({"type":"attacking","controller":"any","rechecked_on_resolution":true})
        );
        assert_eq!(
            targeting::play_targets_to_json(483)["condition"]["controller"],
            "enemy"
        );
        assert_eq!(
            s.target_candidates(actor, 0, action(vec![]), 0)
                .unwrap()
                .candidates,
            vec![attacker]
        );
        for command in ["validate", "cost_plan", "play_choices", "quote_payment"] {
            let result = s.handle(json!({"command":command,"player":actor,"revision":0,
                "action":action_to_json(&action(vec![attacker]))}));
            assert_eq!(result["ok"], true, "{result}");
            assert_eq!(views(&s), before);
        }
        let invalid = s.handle(json!({"command":"apply","player":actor,"revision":0,
            "action":action_to_json(&action(vec![defender]))}));
        assert_eq!(invalid["ok"], false);
        assert_eq!(views(&s), before);
        apply(&mut s, action(vec![attacker]));
        for _ in 0..40 {
            if s.game.chain.is_empty() && s.game.decision.is_none() {
                break;
            }
            let next =
                crate::game::test_support::decline_legend_action(&s.game).unwrap_or(Action::Pass);
            apply(&mut s, next);
        }
        assert!(s.game.chain.is_empty() && s.game.decision.is_none());
        assert!(s.game.players[actor].units[0].stunned);
    }
}

#[test]
fn friendship_api_allows_either_side_and_uses_live_tag_total() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 48510);
        ready_unit(&mut game, actor, 88);
        ready_unit(&mut game, actor, 116);
        let target = Target::Unit {
            player: 1 - actor,
            unit_id: ready_unit(&mut game, 1 - actor, 71),
        };
        game.players[actor].hand = vec![485];
        let mut s = Session {
            game,
            revision: 0,
            series: None,
            format_date: None,
        };
        apply(
            &mut s,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![target],
            },
        );
        while !s.game.chain.is_empty() {
            apply(&mut s, Action::Pass);
        }
        assert_eq!(s.game.players[1 - actor].units[0].temporary_buff, 2);
    }
}
