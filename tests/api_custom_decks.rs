use local_rift::api::{Session, action_to_json, parse_action};
use local_rift::deck::CustomDeck;
use local_rift::game::{Action, FlowSource};
use serde_json::json;

#[test]
fn custom_reset_is_validated_and_each_seat_sees_only_its_own_list() {
    let mut session = Session::default();
    let first = CustomDeck::from_preset(2).unwrap();
    let second = CustomDeck::from_preset(0).unwrap();
    let decks = json!([first.to_json(), second.to_json()]);
    let validated = session.handle(json!({"command":"validate_deck","deck":decks[0]}));
    assert_eq!(validated["ok"], true, "{validated}");
    assert_eq!(validated["result"]["card_count"], 40);
    assert_eq!(session.revision(), 0);
    let reset =
        session.handle(json!({"command":"reset","revision":0,"seed":42,"custom_decks":decks}));
    assert_eq!(reset["ok"], true, "{reset}");
    for seat in 0..2 {
        let view = session.observe(seat).unwrap();
        assert_eq!(view["players"][seat]["custom"], true);
        assert!(view["players"][seat]["preset"].is_null());
        assert_eq!(view["players"][seat]["custom_deck"], decks[seat]);
        assert!(view["players"][1 - seat]["custom_deck"].is_null());
        assert!(view["players"][1 - seat]["hand"].is_null());
    }
    assert_eq!(session.observe(0).unwrap()["players"][0]["legend_id"], 2);
    assert_eq!(session.observe(0).unwrap()["players"][1]["legend_id"], 0);
}

#[test]
fn invalid_custom_decks_leave_state_and_revision_untouched() {
    let mut session = Session::default();
    let before = session.observe(0).unwrap();
    let valid = CustomDeck::from_preset(0).unwrap().to_json();
    let mut short = valid.clone();
    short["cards"] = json!([[0, 1]]);
    let mut wrong_champion = valid.clone();
    wrong_champion["chosen_champion"] = json!(33);
    for input in [
        json!([short, null]),
        json!([wrong_champion, null]),
        json!([valid]),
        json!([null, null, null]),
        json!(false),
    ] {
        let result = session.handle(json!({"command":"reset","revision":0,"custom_decks":input}));
        assert_eq!(result["ok"], false, "{result}");
        assert_eq!(session.revision(), 0);
        assert_eq!(session.observe(0).unwrap(), before);
    }
}

#[test]
fn catalog_identifies_official_cards_and_eligible_custom_choices() {
    let mut session = Session::default();
    let response = session.handle(json!({"command":"catalog"}));
    let catalog = &response["result"];
    assert_eq!(catalog["cards"][74]["official_id"], "ogs-003-024");
    assert!(
        catalog["presets"][3]["eligible_cards"]
            .as_array()
            .unwrap()
            .contains(&json!(74))
    );
    assert!(
        !catalog["presets"][2]["eligible_champions"]
            .as_array()
            .unwrap()
            .contains(&json!(20)),
        "sharing the Yordle tag does not make Vex a Kennen champion"
    );
}

#[test]
fn flow_sources_roundtrip_and_unknown_values_are_rejected() {
    for flow in [None, Some(FlowSource::Printed), Some(FlowSource::Granted)] {
        let action = Action::PlayFromTrashOptions {
            trash_index: 0,
            lane: None,
            targets: vec![],
            repeat_targets: vec![],
            repeat: false,
            accelerate: false,
            bonus: false,
            flow,
        };
        let encoded = action_to_json(&action);
        assert_eq!(action_to_json(&parse_action(&encoded).unwrap()), encoded);
    }
    assert!(
        parse_action(&json!({"type":"play_from_trash_options","trash_index":0,"flow":"typo"}))
            .is_err()
    );
    assert!(
        parse_action(&json!({"type":"play_from_trash_options","trash_index":0,"flow":3})).is_err()
    );
    assert!(parse_action(&json!({"type":"play_from_trash_options","trash_index":0})).is_ok());
}

#[test]
fn custom_deck_bot_matches_finish_with_new_cards() {
    for legend in 0..4 {
        let mut deck = CustomDeck::from_preset(legend).unwrap();
        // Constructed decks may exceed40; include every newly supported card
        // compatible with this legend, without altering the source preset.
        for id in local_rift::deck::eligible_cards(legend)
            .into_iter()
            .filter(|id| *id >= 69)
        {
            deck.cards.push((id, 1));
        }
        deck.validate().unwrap();
        let mut session = Session::default();
        let reset = session.handle(json!({"command":"reset","revision":0,"seed":91+legend,
            "custom_decks":[deck.to_json(),deck.to_json()]}));
        assert_eq!(reset["ok"], true, "{reset}");
        session.start(1).unwrap();
        for _ in 0..5000 {
            let view = session.observe(0).unwrap();
            if view["phase"] == "finished" {
                break;
            }
            let seat = view["priority"].as_u64().unwrap() as usize;
            session.step_ai(seat, session.revision()).unwrap();
        }
        assert_eq!(
            session.observe(0).unwrap()["phase"],
            "finished",
            "legend {legend}"
        );
    }
}
