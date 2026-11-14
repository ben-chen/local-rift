use super::*;

#[test]
fn explicit_date_validation_reports_upcoming_bans_and_rejects_active_ones_read_only() {
    let mut session = Session::default();
    let deck = CustomDeck::from_preset(0).unwrap().to_json();
    let before = [session.observe(0).unwrap(), session.observe(1).unwrap()];
    let revision = session.revision();
    let current =
        session.handle(json!({"command":"validate_deck","deck":deck,"format_date":"2026-11-22"}));
    assert_eq!(current["ok"], true, "{current}");
    assert_eq!(current["result"]["format"]["scope"], "global_standard_1v1");
    assert_eq!(
        current["result"]["format"]["upcoming"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for date in [json!("2026-11-23"), json!("2026-02-30"), json!(20261123)] {
        let response =
            session.handle(json!({"command":"validate_deck","deck":deck,"format_date":date}));
        assert_eq!(response["ok"], false, "{response}");
        assert_eq!(session.revision(), revision);
        assert_eq!(
            [session.observe(0).unwrap(), session.observe(1).unwrap()],
            before
        );
    }
    let snapshot = session.handle(json!({"command":"validate_deck","deck":deck}));
    assert_eq!(snapshot["ok"], true);
    assert!(snapshot["result"].get("format").is_none());
}

#[test]
fn date_specific_registration_checks_include_sideboard_and_fields_without_mutation() {
    let mut session = Session::default();
    let before = session.observe(0).unwrap();
    let current = session.handle(json!({"command":"validate_registration","registration":{"preset":4},"format_date":"2026-11-22"}));
    assert_eq!(current["ok"], true, "{current}");
    let upcoming = current["result"]["format"]["upcoming"].as_array().unwrap();
    assert!(
        upcoming
            .iter()
            .any(|c| c["section"] == "sideboard" && c["name"] == "Brynhir Thundersong")
    );
    let future = session.handle(json!({"command":"validate_registration","registration":{"preset":4},"format_date":"2026-11-23"}));
    assert_eq!(future["ok"], false);
    assert!(
        future["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Star Spring")
    );
    assert_eq!(session.observe(0).unwrap(), before);
    assert_eq!(session.revision(), 0);
}

#[test]
fn dated_game_start_validates_both_decks_and_actual_overridden_battlefields() {
    let mut session = Session::default();
    let before = [session.observe(0).unwrap(), session.observe(1).unwrap()];
    for decks in [[0, 4], [4, 0]] {
        let response = session.handle(
            json!({"command":"reset","revision":0,"decks":decks,"format_date":"2026-11-23"}),
        );
        assert_eq!(response["ok"], false, "{response}");
        assert_eq!(
            [session.observe(0).unwrap(), session.observe(1).unwrap()],
            before
        );
    }
    // The Rengar main and its default Seat of Power remain legal.
    let accepted = session
        .handle(json!({"command":"reset","revision":0,"decks":[4,4],"format_date":"2026-11-23"}));
    assert_eq!(accepted["ok"], true, "{accepted}");
    assert_eq!(session.observe(0).unwrap()["format"]["as_of"], "2026-11-23");
    assert_eq!(session.game.format_date(), Some("2026-11-23"));
    let saved = session.observe(0).unwrap();
    let blocked = session.handle(json!({"command":"reset","revision":1,"decks":[4,4],"battlefields":[11,12],"format_date":"2026-11-23"}));
    assert_eq!(blocked["ok"], false);
    assert_eq!(session.revision(), 1);
    assert_eq!(session.observe(0).unwrap(), saved);
    let archived = session.handle(json!({"command":"reset","revision":1,"decks":[0,0]}));
    assert_eq!(archived["ok"], true);
    assert!(session.observe(0).unwrap().get("format").is_none());
    assert_eq!(session.game.format_date(), None);
}

#[test]
fn dated_series_propagates_the_same_format_into_each_new_game() {
    use crate::match_series::Registration;
    let mut registration = Registration::from_preset(4).unwrap();
    registration.deck.name = "Dated Rengar controller fixture".into();
    registration.deck.battlefield = 0;
    registration.battlefields = [0, 13, 14];
    registration.sideboard.retain(|&(id, _)| id != 51);
    crate::legality::validate_registration_at(&registration, "2026-11-23").unwrap();
    let mut session = Session::default();
    let response = session.handle(json!({"command":"series_begin","revision":0,
        "registrations":[registration.to_json(),registration.to_json()],
        "designated_player":0,"format_date":"2026-11-23"}));
    assert_eq!(response["ok"], true, "{response}");
    for number in 1..=2 {
        for player in 0..2 {
            let view = session.observe(player).unwrap();
            let field = view["series"]["available_battlefields"][0]
                .as_u64()
                .unwrap() as usize;
            session
                .series_battlefield(player, session.revision(), field)
                .unwrap();
        }
        let chooser = session.observe(0).unwrap()["series"]["designated_player"]
            .as_u64()
            .unwrap() as usize;
        session
            .series_play_order(chooser, session.revision(), chooser)
            .unwrap();
        assert_eq!(session.game.format_date(), Some("2026-11-23"));
        assert_eq!(session.observe(0).unwrap()["series"]["game_number"], number);
        if number == 1 {
            session
                .series_concede(0, session.revision(), false)
                .unwrap();
            for player in 0..2 {
                session
                    .series_sideboard(
                        player,
                        session.revision(),
                        registration.deck.cards.clone(),
                        registration.deck.chosen_champion,
                    )
                    .unwrap();
            }
        }
    }
}

#[test]
fn dated_series_start_rejects_banned_pool_and_preserves_prior_match() {
    let mut session = Session::default();
    let start = session.handle(json!({"command":"series_begin","revision":0,
        "registrations":[{"preset":0},{"preset":5}],"designated_player":0,"format_date":"2026-11-22"}));
    assert_eq!(start["ok"], true, "{start}");
    assert_eq!(session.observe(1).unwrap()["format"]["as_of"], "2026-11-22");
    let before = [session.observe(0).unwrap(), session.observe(1).unwrap()];
    let blocked = session.handle(json!({"command":"series_begin","revision":1,
        "registrations":[{"preset":4},{"preset":4}],"designated_player":0,"format_date":"2026-11-23"}));
    assert_eq!(blocked["ok"], false);
    assert_eq!(session.revision(), 1);
    assert_eq!(
        [session.observe(0).unwrap(), session.observe(1).unwrap()],
        before
    );
}
