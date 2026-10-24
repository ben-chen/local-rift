//! Independent protocol tests use controlled board positions in legal legend
//! homes, followed by real commands. Host-bound fixtures are explicitly marked.
use super::*;
use crate::game::Unit;
use crate::gear::GearState;
use crate::hidden::HiddenCard;
use crate::play::{ExecutionChoices, PlayChoices, RepeatSelection};

const FADING: usize = 349;
const LAST_STAND: usize = 350;
const MOMENTARY: usize = 351;
const PETAL: usize = 352;
const SHADOW: usize = 353;
const SMOKE: usize = 354;
const BURST: usize = 355;
const CALL: usize = 356;
const MOTHER: usize = 357;
const QUEEN: usize = 358;
const TREVOR: usize = 359;
const FOUNTAIN: usize = 360;
const TASTY: usize = 361;

fn position(legend: usize) -> Session {
    let mut session = Session::new(ResetOptions {
        custom_decks: [Some(CustomDeck::from_legend(legend).unwrap()), None],
        decks: [0, 3],
        ..Default::default()
    })
    .unwrap();
    session.start(0).unwrap();
    session.game.battlefields = [3, 3];
    session.game.battlefield_owner = [None; 2];
    for player in &mut session.game.players {
        player.hand.clear();
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.discard.clear();
        player.champion = None;
        player.runes = [0; 6];
        player.exhausted_runes = [0; 6];
        player.energy_pool = 60;
        player.energy = 60;
        player.power_pool = [0; 6];
        player.any_power = 60;
        player.xp = 0;
    }
    session.game.legend[0].ready = false;
    session.game.legend[1].ready = false;
    session
}

fn unit(id: u32, card_id: usize, lane: Option<usize>) -> Unit {
    Unit {
        id,
        card_id,
        lane,
        ready: true,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    }
}

fn gear(id: u32, card_id: usize, attached_to: Option<u32>) -> GearState {
    GearState {
        id,
        card_id,
        attached_to,
        ready: true,
        temporary: false,
        token: false,
    }
}

fn target(player: usize, unit_id: u32) -> Target {
    Target::Unit { player, unit_id }
}
fn gear_target(player: usize, gear_id: u32) -> Target {
    Target::Gear { player, gear_id }
}
fn play(targets: Vec<Target>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets,
    }
}
fn views(session: &Session) -> [Value; 2] {
    [session.observe(0).unwrap(), session.observe(1).unwrap()]
}
fn apply(session: &mut Session, action: Action) {
    let response = session.handle(json!({"command":"apply","player":session.game.priority(),
        "revision":session.revision(),"action":action_to_json(&action)}));
    assert_eq!(response["ok"], true, "{response}");
}
fn until(session: &mut Session, done: impl Fn(&Session) -> bool) {
    for _ in 0..80 {
        if done(session) {
            return;
        }
        let action = match session.game.decision.as_ref() {
            None => Action::Pass,
            Some(Decision::Legend { .. }) => Action::Choose { accept: false },
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            other => panic!("Unexpected mandatory choice: {other:?}"),
        };
        apply(session, action);
    }
    panic!("Requested protocol state was not reached");
}
fn settle(session: &mut Session) {
    until(session, |s| {
        s.game.chain.is_empty() && s.game.decision.is_none()
    });
}
fn query(session: &mut Session, action: Action, execution: usize) -> Value {
    let before = views(session);
    let state = format!("{:?}", session.game);
    let response = session.handle(
        json!({"command":"target_candidates","player":session.game.priority(),
        "revision":session.revision(),"action":action_to_json(&action),"execution":execution}),
    );
    assert_eq!(views(session), before);
    assert_eq!(format!("{:?}", session.game), state);
    response
}
fn candidates(response: &Value) -> Vec<Target> {
    assert_eq!(response["ok"], true, "{response}");
    response["result"]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| parse_target(value).unwrap())
        .collect()
}
fn own_unit(session: &Session, id: u32) -> Value {
    session.observe(0).unwrap()["players"][0]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["id"] == id)
        .unwrap()
        .clone()
}
fn portal(session: &mut Session) {
    session.game.players[0].gear.push(gear(8_000, 256, None));
    apply(
        session,
        Action::ActivateAbility {
            source: AbilitySource::Gear {
                player: 0,
                gear_id: 8_000,
            },
            ability_index: 0,
            targets: vec![],
        },
    );
    settle(session);
}

#[test]
fn token_catalog_separates_sprite_identity_from_factory_and_instruction() {
    let catalog = catalog();
    assert_eq!(
        catalog["cards"][FADING]["play_targets"],
        json!({"kind":"unit_or_gear","maximum":1,"required":1})
    );
    assert_eq!(
        catalog["cards"][MOMENTARY]["tags"],
        json!(["Shadow Isles", "Demon"])
    );
    assert_eq!(catalog["cards"][SMOKE]["play_targets"]["required"], 2);
    for id in [BURST, CALL, MOTHER, QUEEN, TREVOR, FOUNTAIN] {
        let token = &catalog["cards"][id]["created_token"];
        assert_eq!(
            token,
            &json!({"name":"Sprite","might":3,"kind":"unit",
            "domains":[],"tags":["Fae"],"deflect":false,"temporary":true,
            "count":if id==BURST {2} else {1}})
        );
        assert!(token.get("entry_ready").is_none());
    }
    assert!(catalog["cards"][TASTY]["created_token"].is_null());
    assert_eq!(catalog["cards"][FOUNTAIN]["kind"], "Gear");
    assert_eq!(
        catalog["cards"][PETAL]["passive_rules"][0]["effect"],
        json!({"type":"might_per_friendly_temporary_here","amount":1})
    );
}

#[test]
fn mixed_target_query_accepts_gear_anywhere_but_units_only_at_a_field() {
    let mut session = position(0);
    session.game.players[0].hand = vec![FADING];
    session.game.players[0].units = vec![unit(5_000, 71, None), unit(5_001, 71, Some(0))];
    // Steel Paws gives its bearer Deflect, which must not be copied to gear.
    let mut bearer = unit(6_000, 273, Some(1));
    bearer.buff = 2;
    session.game.players[1].units = vec![bearer];
    session.game.players[0].gear = vec![gear(7_000, 21, None)];
    session.game.players[1].gear = vec![gear(7_001, 21, Some(6_000))];
    let response = query(&mut session, play(vec![]), 0);
    let choices = candidates(&response);
    assert!(!choices.contains(&target(0, 5_000)));
    for choice in [
        target(0, 5_001),
        target(1, 6_000),
        gear_target(0, 7_000),
        gear_target(1, 7_001),
    ] {
        assert!(choices.contains(&choice), "{choice:?}: {response}");
    }
    assert_eq!(response["result"]["next_slot"]["index"], 0);
    let complete = query(&mut session, play(vec![gear_target(1, 7_001)]), 0);
    assert!(candidates(&complete).is_empty());
    assert!(complete["result"]["next_slot"].is_null());
    for invalid in [
        vec![target(0, 5_000)],
        vec![Target::Battlefield { lane: 0 }],
        vec![gear_target(1, 7_001), target(0, 5_001)],
    ] {
        assert_eq!(query(&mut session, play(invalid), 0)["ok"], false);
    }
    let plan = session
        .cost_plan(0, session.revision(), play(vec![gear_target(1, 7_001)]))
        .unwrap();
    assert_eq!(plan.power.iter().map(|cost| cost.amount).sum::<u16>(), 1);
    apply(&mut session, play(vec![gear_target(1, 7_001)]));
    settle(&mut session);
    let observed = session.observe(0).unwrap();
    assert_eq!(
        observed["players"][1]["gear"][0]["temporary_status"],
        json!({"intrinsic":false,"granted":true,"active":true})
    );
    assert_eq!(
        observed["players"][1]["units"][0]["temporary_status"]["granted"],
        false
    );
}

#[test]
fn target_queries_reject_stale_wrong_seat_and_malformed_drafts_without_changes() {
    let mut session = position(3);
    session.game.players[0].hand = vec![SMOKE];
    session.game.players[0].units = vec![unit(5_000, 69, None), unit(5_001, 69, Some(0))];
    let valid = json!({"command":"target_candidates","player":0,"revision":session.revision(),
        "action":action_to_json(&play(vec![])),"execution":0});
    let before = views(&session);
    let state = format!("{:?}", session.game);
    for (key, value, code) in [
        ("revision", json!(session.revision() - 1), "stale_revision"),
        ("player", json!(1), "wrong_player"),
        ("execution", json!(-1), "invalid_request"),
        ("execution", json!(0.5), "invalid_request"),
        ("execution", json!("0"), "invalid_request"),
        ("execution", json!(1), "illegal_action"),
        ("extra", json!(true), "invalid_request"),
    ] {
        let mut request = valid.clone();
        request[key] = value;
        let reply = session.handle(request);
        assert_eq!(reply["error"]["code"], code, "{reply}");
        assert_eq!(views(&session), before);
        assert_eq!(format!("{:?}", session.game), state);
    }
    let mut request = valid;
    request["action"]["targets"] =
        json!([{"type":"unit","player":0,"unit_id":5_000,"temporary":true}]);
    assert_eq!(session.handle(request)["error"]["code"], "invalid_request");
    assert_eq!(views(&session), before);
}

#[test]
fn hidden_smoke_queries_preserve_origin_order_prefix_and_opponent_privacy() {
    let mut session = position(3);
    session.game.battlefield_owner[0] = Some(0);
    session.game.players[0].units = vec![
        unit(5_000, 69, Some(0)),
        unit(5_001, 69, None),
        unit(5_002, 69, Some(1)),
    ];
    session.game.players[0].hidden.push(HiddenCard {
        id: 9_000,
        card_id: SMOKE,
        lane: 0,
        hidden_turn: 0,
        revealed: false,
    });
    let draft = |targets| Action::PlayHidden {
        hidden_id: 9_000,
        targets,
    };
    let first = query(&mut session, draft(vec![]), 0);
    assert_eq!(candidates(&first), vec![target(0, 5_000)]);
    assert_eq!(first["result"]["origin"], json!({"type":"hidden","lane":0}));
    assert_eq!(
        first["result"]["next_slot"],
        json!({"index":0,
        "hidden":{"type":"at_battlefield","lane":0},"different_location_from":null})
    );
    let second = query(&mut session, draft(vec![target(0, 5_000)]), 0);
    assert_eq!(candidates(&second).len(), 2);
    assert_eq!(
        second["result"]["next_slot"],
        json!({"index":1,"hidden":null,"different_location_from":0})
    );
    assert_eq!(
        query(&mut session, draft(vec![target(0, 5_001)]), 0)["ok"],
        false
    );
    assert_eq!(
        query(
            &mut session,
            draft(vec![target(0, 5_000), target(0, 5_000)]),
            0
        )["ok"],
        false
    );
    assert_eq!(
        session.observe(1).unwrap()["players"][0]["hidden"][0]["card_id"],
        Value::Null
    );
    let mut changed = session.clone();
    changed.game.players[0].hidden[0].card_id = CALL;
    assert_eq!(changed.observe(1).unwrap(), session.observe(1).unwrap());
    let hand = session.game.players[0].hand.len();
    apply(
        &mut session,
        draft(vec![target(0, 5_000), target(0, 5_001)]),
    );
    settle(&mut session);
    assert_eq!(
        session.game.players[0].hand.len(),
        hand + 1,
        "No Temporary pair still draws"
    );
    assert_eq!(session.game.players[0].units[0].lane, Some(0));
    assert_eq!(session.game.players[0].units[1].lane, None);
}

#[test]
fn sprite_children_report_distinct_identity_and_finish_after_the_parent() {
    let mut session = position(3);
    session.game.players[0].hand = vec![BURST];
    apply(&mut session, play(vec![]));
    until(&mut session, |s| {
        matches!(
            s.game.decision,
            Some(Decision::TokenPlay { .. } | Decision::TokenPlacement { .. })
        )
    });
    assert_eq!(session.game.players[0].discard, vec![BURST]);
    assert!(session.game.players[0].units.is_empty());
    let first = session.observe(0).unwrap();
    assert_eq!(first["pending_token_plays"].as_array().unwrap().len(), 2);
    let token_id = first["decision"]["token_id"].clone();
    assert_eq!(first["pending_token_plays"][0]["id"], token_id);
    assert_eq!(
        first["pending_token_plays"][0]["token_identity"],
        json!({
        "name":"Sprite","might":3,"kind":"unit","domains":[],"tags":["Fae"],
        "deflect":false,"temporary":true})
    );
    assert_eq!(first["pending_token_plays"][0]["entry_ready"], true);
    let before = views(&session);
    let rejected = session.handle(
        json!({"command":"apply","player":0,"revision":session.revision(),
        "action":{"type":"place_token","lane":null,"temporary":false}}),
    );
    assert_eq!(rejected["error"]["code"], "invalid_request");
    assert_eq!(views(&session), before);
    apply(&mut session, Action::PlaceToken { lane: None });
    until(&mut session, |s| {
        matches!(
            s.game.decision,
            Some(Decision::TokenPlay { .. } | Decision::TokenPlacement { .. })
        )
    });
    let next = session.observe(0).unwrap();
    assert_ne!(next["decision"]["token_id"], token_id);
    apply(&mut session, Action::PlaceToken { lane: None });
    settle(&mut session);
    let observed = session.observe(0).unwrap();
    let units = observed["players"][0]["units"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    for sprite in units {
        assert_eq!(sprite["name"], "Sprite token");
        assert_eq!(sprite["tags"], json!(["Fae"]));
        assert_eq!(sprite["might"], 3);
        assert_eq!(sprite["ready"], true);
        assert!(
            !session
                .game
                .buffed_units
                .contains(&(sprite["id"].as_u64().unwrap() as u32))
        );
        assert_eq!(
            sprite["temporary_status"],
            json!({"intrinsic":true,"granted":false,"active":true})
        );
    }
}

#[test]
fn shadow_repeats_keep_the_original_target_but_each_draw_is_independent() {
    // Viktor's Mind/Order identity can legally use Portal and Shadow's Call.
    let mut session = position(5);
    portal(&mut session);
    session.game.players[0].units = vec![unit(5_000, 69, None)];
    session.game.players[0].hand = vec![SHADOW];
    let original = play(vec![target(0, 5_000)]);
    let options = session
        .play_choices(0, session.revision(), original.clone())
        .unwrap();
    assert_eq!(options.repeats.len(), 1);
    let action = Action::WithPlayChoices {
        action: Box::new(original),
        choices: PlayChoices {
            repeats: vec![RepeatSelection {
                option_id: options.repeats[0].id,
                choices: ExecutionChoices {
                    targets: vec![target(0, 5_000)],
                    mode: None,
                },
            }],
            ..Default::default()
        },
    };
    let reply = query(&mut session, action.clone(), 1);
    assert!(candidates(&reply).is_empty());
    assert_eq!(
        reply["result"]["targets"],
        json!([{"type":"unit","player":0,"unit_id":5_000}])
    );
    let draws = session.game.trigger_state.draws[0];
    apply(&mut session, action);
    settle(&mut session);
    assert_eq!(session.game.trigger_state.draws[0], draws + 4);
    assert_eq!(session.game.players[0].hand.len(), 4);
    assert_eq!(session.game.players[0].discard, vec![SHADOW]);
    assert_eq!(
        own_unit(&session, 5_000)["temporary_status"],
        json!({"intrinsic":false,"granted":true,"active":true})
    );
    session.game.players[0].hand = vec![SHADOW];
    assert!(!candidates(&query(&mut session, play(vec![]), 0)).contains(&target(0, 5_000)));
}

#[test]
fn petal_reports_the_checked_evaluated_bonus_without_replacing_its_descriptor() {
    let mut session = position(5);
    session.game.players[0].units = vec![unit(5_000, PETAL, Some(0))];
    session.game.players[0].hand = vec![SHADOW];
    apply(&mut session, play(vec![target(0, 5_000)]));
    settle(&mut session);
    let observed = own_unit(&session, 5_000);
    assert_eq!(observed["might"], 3);
    assert_eq!(
        observed["passive_contributions"][0]["computed_might_exact"],
        "1"
    );
    assert_eq!(
        observed["passive_contributions"][0]["effect"],
        json!({"type":"might_per_friendly_temporary_here","amount":1})
    );
    apply(
        &mut session,
        Action::Move {
            unit_id: 5_000,
            lane: None,
        },
    );
    let base = own_unit(&session, 5_000);
    assert_eq!(base["might"], 2);
    assert_eq!(
        base["temporary_status"]["granted"], true,
        "Moving preserves provenance"
    );
    assert_eq!(
        base["passive_contributions"][0]["computed_might_exact"],
        "0"
    );
}

#[test]
fn last_stand_uses_six_real_executions_and_a_fresh_might_snapshot() {
    let mut session = position(0);
    session.game.players[0].units = vec![unit(5_000, TASTY, Some(0)), unit(5_001, 266, Some(0))];
    session.game.players[1].units = vec![unit(6_000, 69, Some(0))];
    session.game.showdown = Some(crate::game::Showdown {
        lane: 0,
        attacker: 0,
        focus: 0,
        passes: 0,
        combat: true,
    });
    session.game.players[0].hand = vec![LAST_STAND; 3];
    for expected in [24, 96, 384] {
        if session.game.priority() != 0 {
            apply(&mut session, Action::Pass);
        }
        let original = play(vec![target(0, 5_000)]);
        let options = session
            .play_choices(0, session.revision(), original.clone())
            .unwrap();
        assert_eq!(
            options.repeats.len(),
            1,
            "Syndra supplies a distinct Repeat"
        );
        let action = Action::WithPlayChoices {
            action: Box::new(original),
            choices: PlayChoices {
                repeats: vec![RepeatSelection {
                    option_id: options.repeats[0].id,
                    choices: ExecutionChoices {
                        targets: vec![target(0, 5_000)],
                        mode: None,
                    },
                }],
                ..Default::default()
            },
        };
        apply(&mut session, action);
        settle(&mut session);
        assert_eq!(own_unit(&session, 5_000)["might"], expected);
    }
    let observed = own_unit(&session, 5_000);
    assert_eq!(observed["might_exact"], "384");
    assert_eq!(observed["temporary_buff_exact"], "378");
    assert_eq!(
        observed["temporary_status"],
        json!({"intrinsic":false,"granted":true,"active":true})
    );
}

#[test]
fn payment_query_uses_the_stored_detached_card_and_never_mutates_it() {
    let mut session = position(0);
    session.game.players[0].hand = vec![FADING];
    session.game.players[0].gear = vec![gear(7_000, 21, None)];
    apply(
        &mut session,
        Action::BeginPayment {
            action: Box::new(play(vec![gear_target(0, 7_000)])),
        },
    );
    assert!(matches!(
        session.game.decision,
        Some(Decision::Payment { .. })
    ));
    assert!(session.game.players[0].hand.is_empty());
    let response = query(&mut session, Action::CompletePayment, 0);
    assert!(candidates(&response).is_empty());
    assert_eq!(response["result"]["card_id"], FADING);
    assert_eq!(
        response["result"]["targets"],
        json!([{"type":"gear","player":0,"gear_id":7_000}])
    );
    apply(&mut session, Action::CompletePayment);
    settle(&mut session);
    assert_eq!(
        session.observe(0).unwrap()["players"][0]["gear"][0]["temporary_status"]["granted"],
        true
    );
}

#[test]
fn hidden_smoke_suggestions_follow_the_same_ordered_slots_as_the_query() {
    let mut session = position(3);
    session.game.battlefield_owner[0] = Some(0);
    session.game.players[0].units = vec![
        unit(5_000, 69, None),
        unit(5_001, 69, Some(0)),
        unit(5_002, 69, Some(1)),
    ];
    session.game.players[0].hidden.push(HiddenCard {
        id: 9_000,
        card_id: SMOKE,
        lane: 0,
        hidden_turn: 0,
        revealed: false,
    });
    let actions = session.game.hidden_actions(0).unwrap();
    let plays: Vec<_> = actions
        .into_iter()
        .filter(|a| matches!(a, Action::PlayHidden { .. }))
        .collect();
    assert_eq!(plays.len(), 2);
    for action in plays {
        let response = query(&mut session, action.clone(), 0);
        assert_eq!(
            response["result"]["targets"][0],
            json!({"type":"unit","player":0,"unit_id":5_001})
        );
        assert!(candidates(&response).is_empty());
        let mut branch = session.clone();
        apply(&mut branch, action);
        settle(&mut branch);
    }
}
