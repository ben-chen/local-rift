use super::*;
use crate::game::test_support::{position, ready_unit};

#[test]
fn warden_api_exposes_public_sources_and_rejects_field_plays_atomically() {
    for actor in 0..2 {
        let mut s = Session::new(ResetOptions::default()).unwrap();
        s.start(0).unwrap();
        s.game = position(actor, 0, 464100);
        let warden = ready_unit(&mut s.game, 1 - actor, 464);
        s.game.players[1 - actor].units[0].lane = Some(1);
        ready_unit(&mut s.game, actor, 71);
        s.game.players[actor].units[0].lane = Some(0);
        s.game.battlefield_owner = vec![Some(actor), Some(1 - actor)];
        s.game.players[actor].hand = vec![71];
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        for view in &before {
            let sources = view["players"][actor]["board_restrictions"]
                .as_array()
                .unwrap();
            assert_eq!(sources.len(), 2);
            assert_eq!(sources[0]["kind"], "unit_play_base_only");
            assert_eq!(sources[1]["kind"], "unit_gear_ready_by_effect");
            assert_eq!(sources[0]["source"]["unit_id"], warden);
            assert!(
                view["players"][1 - actor]
                    .get("board_restrictions")
                    .is_none()
            );
        }
        let revision = s.revision();
        let result = s.handle(json!({"command":"apply","revision":revision,"player":actor,
            "action":{"type":"play_card_targets","hand_index":0,"lane":0,"targets":[]}}));
        assert_eq!(result["ok"], false);
        assert_eq!(s.revision(), revision);
        assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
        s.game.players[1 - actor].units[0].lane = None;
        assert!(
            s.observe(actor).unwrap()["players"][actor]
                .get("board_restrictions")
                .is_none()
        );
        let result = s.handle(json!({"command":"apply","revision":revision,"player":actor,
            "action":{"type":"play_card_targets","hand_index":0,"lane":0,"targets":[]}}));
        assert_eq!(result["ok"], true, "{result}");
    }
}
