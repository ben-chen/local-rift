use super::*;
use crate::game::test_support::{decline_legend_action, position};
use test_support::{apply, views};

fn advance(s: &mut Session) {
    let action = if let Some(action) = decline_legend_action(&s.game) {
        action
    } else if let Some(Decision::OrderTriggers { options, .. }) = &s.game.decision {
        Action::OrderTrigger {
            trigger_id: options[0].id,
        }
    } else {
        assert!(s.game.decision.is_none() && !s.game.chain.is_empty());
        Action::Pass
    };
    apply(s, action);
}

#[test]
fn recovery_api_explains_choices_preserves_atomicity_and_filters_hands() {
    for actor in 0..2 {
        for (source, chosen, wrong, kind) in [(474, 96, 8, "unit"), (475, 8, 96, "spell")] {
            let mut s = Session {
                game: position(actor, 0, 475100),
                revision: 0,
                series: None,
                format_date: None,
            };
            s.game.players[actor].hand = vec![source];
            s.game.players[actor].discard = vec![wrong, chosen, chosen];
            apply(
                &mut s,
                Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                },
            );
            for _ in 0..60 {
                if matches!(s.game.decision, Some(Decision::TrashTarget { .. })) {
                    break;
                }
                advance(&mut s);
            }
            let before = views(&s);
            assert_eq!(before[actor]["decision"]["outcome"], "return_to_hand");
            assert_eq!(before[actor]["decision"]["card_kind"], kind);
            let revision = s.revision();
            for player in [actor, 1 - actor] {
                let result = s.handle(
                    json!({"command":"apply", "revision":revision, "player":player,
                    "action":{"type":"select_cards", "indices":[0]}}),
                );
                assert_eq!(result["ok"], false, "{result}");
                assert_eq!(s.revision(), revision);
                assert_eq!(views(&s), before);
            }
            let result = s.handle(
                json!({"command":"apply", "revision":revision, "player":actor,
                "action":{"type":"select_cards", "indices":[2]}}),
            );
            assert_eq!(result["ok"], true, "{result}");
            let ability = ability_to_json(s.game.chain.last().unwrap().ability.as_ref().unwrap());
            assert_eq!(
                ability,
                json!({"type":"return_chosen", "trash_index":2,"card_id":chosen})
            );
            for _ in 0..60 {
                if s.game.decision.is_none() && s.game.chain.is_empty() {
                    break;
                }
                advance(&mut s);
            }
            assert_eq!(s.game.players[actor].hand, vec![chosen]);
            let after = views(&s);
            assert_eq!(after[actor]["players"][actor]["hand"], json!([chosen]));
            assert!(after[1 - actor]["players"][actor]["hand"].is_null());
            assert_eq!(after[1 - actor]["players"][actor]["hand_count"], 1);
        }
    }
}

#[test]
fn scout_api_vision_reveals_top_card_only_to_its_controller() {
    for actor in 0..2 {
        let mut s = Session {
            game: position(actor, 0, 473100),
            revision: 0,
            series: None,
            format_date: None,
        };
        s.game.players[actor].hand = vec![473];
        s.game.players[actor].deck = vec![96, 71, 8];
        apply(
            &mut s,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: Some(0),
                targets: vec![],
            },
        );
        for _ in 0..60 {
            if matches!(s.game.decision, Some(Decision::Predict { .. })) {
                break;
            }
            advance(&mut s);
        }
        let view = views(&s);
        assert_eq!(view[actor]["decision"]["card_id"], 8);
        assert!(view[1 - actor]["decision"]["card_id"].is_null());
        apply(&mut s, Action::Choose { accept: true });
        assert_eq!(s.game.players[actor].deck, vec![8, 96, 71]);
    }
}
