//! Public protocol checks for the three Irelia battlefield additions.
use super::*;
use crate::battlefields::{BACK_ALLEY_BAR_ID, CANDLELIT_SANCTUM_ID, FORGOTTEN_MONUMENT_ID};
use crate::game::test_support::{decline_legend_action, position, ready_unit};

fn session(actor: usize, field: usize) -> Session {
    let mut s = Session::new(ResetOptions::default()).unwrap();
    s.start(0).unwrap();
    s.game = position(actor, 0, 201819);
    s.game.battlefields = vec![field, 3];
    s
}

fn submit(s: &mut Session, action: Action) {
    let result = s.handle(json!({"command":"apply", "revision":s.revision(),
        "player":s.game.priority(), "action":action_to_json(&action)}));
    assert_eq!(result["ok"], true, "{result}");
}

#[test]
fn fields_api_scoring_unlock_is_public_and_uses_the_players_own_turn() {
    for actor in 0..2 {
        let s = session(actor, FORGOTTEN_MONUMENT_ID);
        for viewer in 0..2 {
            let view = s.observe(viewer).unwrap();
            let field = &view["battlefield_scoring"][0];
            assert_eq!(field["battlefield_id"], FORGOTTEN_MONUMENT_ID);
            assert_eq!(field["players"][actor]["turns_started"], 1);
            assert_eq!(field["players"][actor]["turns_until_unlocked"], 2);
            assert_eq!(field["players"][1 - actor]["turns_until_unlocked"], 3);
            assert_eq!(field["players"][actor]["scoring_unlocked"], false);
        }
    }
}

#[test]
fn fields_api_sanctum_hides_cards_but_exposes_source_and_accepts_the_same_choice_protocol() {
    for actor in 0..2 {
        let mut s = session(actor, CANDLELIT_SANCTUM_ID);
        s.game.players[actor].deck = vec![71, 70, 69, 68];
        let id = ready_unit(&mut s.game, actor, 71);
        submit(
            &mut s,
            Action::Move {
                unit_id: id,
                lane: Some(0),
            },
        );
        for _ in 0..60 {
            if matches!(s.game.decision, Some(Decision::PredictCards { .. })) {
                break;
            }
            let action = decline_legend_action(&s.game).unwrap_or(Action::Pass);
            submit(&mut s, action);
        }
        let own = s.observe(actor).unwrap();
        assert_eq!(own["decision"]["purpose"], "look_recycle");
        assert_eq!(
            own["decision"]["source"]["battlefield_id"],
            CANDLELIT_SANCTUM_ID
        );
        assert_eq!(own["decision"]["cards"], json!([68, 69]));
        let opponent = s.observe(1 - actor).unwrap();
        assert!(opponent["decision"].get("cards").is_none());
        let original = s.game.clone();
        s.game.players[actor].deck = vec![71, 70, 96, 97];
        if let Some(Decision::PredictCards { cards, .. }) = &mut s.game.decision {
            *cards = vec![97, 96];
        }
        assert_eq!(s.observe(1 - actor).unwrap(), opponent);
        s.game = original;
        let choice_id = own["decision"]["choice_id"].as_u64().unwrap() as u32;
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        let revision = s.revision();
        let rejected=s.handle(json!({"command":"apply", "revision":revision,"player":actor,
            "action":{"type":"resolve_predict","choice_id":choice_id,"recycle":[0],"top_order":[0]}}));
        assert_eq!(rejected["ok"], false);
        assert_eq!(s.revision(), revision);
        assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
        submit(
            &mut s,
            Action::ResolvePredict {
                choice_id,
                recycle: vec![0],
                top_order: vec![1],
            },
        );
        assert_eq!(s.game.players[actor].deck, vec![68, 71, 70, 69]);
    }
}

#[test]
fn fields_api_bar_references_the_unit_without_presenting_it_as_a_target() {
    let mut s = session(0, BACK_ALLEY_BAR_ID);
    let id = ready_unit(&mut s.game, 0, 71);
    s.game.players[0].units[0].lane = Some(0);
    s.game.battlefield_owner[0] = Some(0);
    submit(
        &mut s,
        Action::Move {
            unit_id: id,
            lane: None,
        },
    );
    for viewer in 0..2 {
        let view = s.observe(viewer).unwrap();
        let chain = view["chain"].as_array().unwrap();
        let ability = chain
            .iter()
            .find(|i| i["ability"]["type"] == "back_alley_might")
            .unwrap();
        assert_eq!(ability["ability"]["unit_id"], id);
        assert_eq!(ability["source"]["battlefield_id"], BACK_ALLEY_BAR_ID);
        assert_eq!(ability["targets"], json!([]));
    }
}
