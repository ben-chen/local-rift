use super::*;
use crate::game::test_support::position;

#[test]
fn entry_api_describes_free_choice_and_exposes_accepted_restriction_to_both_players() {
    for actor in 0..2 {
        let mut s = Session::new(ResetOptions::default()).unwrap();
        s.start(0).unwrap();
        s.game = position(actor, 0, 467100);
        s.game.players[actor].hand = vec![467];
        let revision = s.revision();
        let action = json!({"type":"play_card_targets","hand_index":0,"lane":null,"targets":[]});
        let preview = s
            .play_choices(actor, revision, parse_action(&action).unwrap())
            .unwrap();
        let preview = play_choices::view_to_json(&preview);
        assert_eq!(preview["entry_ready"]["is_cost"], false);
        assert_eq!(preview["entry_ready"]["prevents_movement"], true);
        assert!(preview["bonus"].is_null());
        let chosen =
            json!({"type":"with_play_choices","action":action,"choices":{"entry_ready":true}});
        assert_eq!(
            action_to_json(&parse_action(&chosen).unwrap())["choices"]["entry_ready"],
            true
        );
        let result =
            s.handle(json!({"command":"apply","revision":revision,"player":actor,"action":chosen}));
        assert_eq!(result["ok"], true, "{result}");
        for viewer in 0..2 {
            let view = s.observe(viewer).unwrap();
            let unit = &view["players"][actor]["units"][0];
            assert_eq!(unit["ready"], true);
            assert_eq!(unit["movement_locked"], true);
            assert_eq!(unit["movement_restriction"]["scope"], "any_player");
        }
    }
}

#[test]
fn entry_api_rejects_wrong_card_and_wrong_type_without_mutation() {
    let mut s = Session::new(ResetOptions::default()).unwrap();
    s.start(0).unwrap();
    s.game = position(0, 0, 466100);
    s.game.players[0].hand = vec![466];
    let revision = s.revision();
    let before = s.observe(0).unwrap();
    for choices in [
        json!({"entry_ready":true}),
        json!({"bonus":true}),
        json!({"entry_ready":"yes"}),
    ] {
        let result = s.handle(json!({"command":"apply","revision":revision,"player":0,
            "action":{"type":"with_play_choices","action":{"type":"play_card_targets","hand_index":0,"lane":null,"targets":[]},"choices":choices}}));
        assert_eq!(result["ok"], false, "{result}");
        assert_eq!(s.revision(), revision);
        assert_eq!(s.observe(0).unwrap(), before);
    }
    assert!(
        play_choices::choices_to_json(&crate::play::PlayChoices::default())
            .get("entry_ready")
            .is_none()
    );
}
