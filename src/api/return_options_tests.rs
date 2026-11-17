use super::*;
use crate::game::{return_options_tests::combat, test_support::position};
use test_support::{apply, views};

#[test]
fn dread_api_choices_costs_and_resolution_share_the_attacker_restriction() {
    for actor in 0..2 {
        let (game, attacker, defender) = combat(actor);
        let mut s = Session {
            game,
            revision: 0,
            series: None,
            format_date: None,
        };
        let action = |targets, repeat_targets| Action::PlayCardOptions {
            hand_index: 0,
            lane: None,
            targets,
            repeat_targets,
            repeat: true,
            accelerate: false,
            bonus: false,
        };
        let draft = action(vec![], vec![]);
        let before = views(&s);
        assert_eq!(
            s.target_candidates(actor, 0, draft.clone(), 0)
                .unwrap()
                .candidates,
            vec![attacker]
        );
        let full = action(vec![attacker], vec![attacker]);
        for command in ["validate", "cost_plan", "play_choices", "quote_payment"] {
            let r = s.handle(json!({"command":command,"player":actor,"revision":0,"action":action_to_json(&full)}));
            assert_eq!(r["ok"], true, "{r}");
            assert_eq!(views(&s), before);
        }
        for (player, revision, bad) in [
            (1 - actor, 0, full.clone()),
            (actor, 99, full.clone()),
            (actor, 0, action(vec![defender], vec![attacker])),
            (actor, 0, action(vec![attacker], vec![defender])),
        ] {
            let r = s.handle(json!({"command":"apply","player":player,"revision":revision,"action":action_to_json(&bad)}));
            assert_eq!(r["ok"], false);
            assert_eq!(views(&s), before);
            assert_eq!(s.revision(), 0);
        }
        assert_eq!(
            targeting::play_targets_to_json(483)["condition"],
            json!({"type":"attacking","controller":"enemy","rechecked_on_resolution":true})
        );
        apply(&mut s, full);
        for _ in 0..20 {
            if s.game.chain.is_empty() {
                break;
            }
            apply(&mut s, Action::Pass);
        }
        assert!(s.game.chain.is_empty());
        assert_eq!(s.game.players[1 - actor].hand, vec![71]);
        for view in views(&s) {
            assert!(
                view["players"][1 - actor]["units"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
    }
}

#[test]
fn morbid_api_requires_one_own_trash_unit_without_revealing_private_hands() {
    for actor in 0..2 {
        let mut s = Session {
            game: position(actor, 0, 48210),
            revision: 0,
            series: None,
            format_date: None,
        };
        s.game.players[actor].hand = vec![482];
        s.game.players[actor].discard = vec![3, 71];
        s.game.players[1 - actor].discard = vec![71];
        let target = s.game.trash_target(actor, 1).unwrap();
        let action = |targets| Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets,
        };
        let candidates = s.target_candidates(actor, 0, action(vec![]), 0).unwrap();
        assert_eq!(candidates.candidates, vec![target]);
        assert_eq!((candidates.spec.required, candidates.spec.maximum), (1, 1));
        apply(&mut s, action(vec![target]));
        for _ in 0..20 {
            if s.game.chain.is_empty() {
                break;
            }
            apply(&mut s, Action::Pass);
        }
        assert_eq!(s.game.players[actor].hand, vec![71]);
        assert_eq!(s.game.players[actor].discard, vec![3, 482]);
        assert!(s.observe(1 - actor).unwrap()["players"][actor]["hand"].is_null());
    }
}
