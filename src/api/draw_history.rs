//! Protocol boundary fixtures seed history, then use ordinary commands.
use super::*;

fn position() -> Session {
    let mut s = Session::new(ResetOptions {
        decks: [3, 3],
        ..Default::default()
    })
    .unwrap();
    s.start(0).unwrap();
    s.game.battlefields = [2, 2];
    s.game.battlefield_owner = [None; 2];
    for p in &mut s.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.deck = vec![96; 8];
        p.runes = [0; 6];
        p.exhausted_runes = [0; 6];
        p.energy_pool = 40;
        p.energy = 40;
        p.power_pool = [8; 6];
    }
    for legend in &mut s.game.legend {
        legend.ready = false;
    }
    s
}
fn request(s: &mut Session, command: &str, action: Action) -> Value {
    let mut request = json!({"command": command, "player": s.game.priority()});
    if command != "suggest" {
        request["revision"] = json!(s.revision());
    }
    if command != "suggest" && command != "step_ai" {
        request["action"] = action_to_json(&action);
    }
    s.handle(request)
}
fn apply(s: &mut Session, action: Action) {
    let r = request(s, "apply", action);
    assert_eq!(r["ok"], true, "{r}");
}
fn views(s: &Session) -> [Value; 2] {
    [s.observe(0).unwrap(), s.observe(1).unwrap()]
}

#[test]
fn draw_history_keeps_numeric_wire_and_an_always_exact_public_decimal() {
    let mut s = position();
    for value in [0_u64, 2, 65_535, 65_536, (1_u64 << 53) + 1, u64::MAX] {
        s.game.trigger_state.draws = [value; 2];
        for view in views(&s) {
            let roundtrip: Value = serde_json::from_str(&view.to_string()).unwrap();
            for owner in 0..2 {
                assert_eq!(
                    roundtrip["players"][owner]["cards_drawn_this_turn"].as_u64(),
                    Some(value)
                );
                assert_eq!(
                    roundtrip["players"][owner]["cards_drawn_this_turn_exact"],
                    value.to_string()
                );
            }
        }
    }
}

#[test]
fn draw_capacity_is_a_structured_atomic_error_for_validation_apply_ai_and_suggestions() {
    let mut s = position();
    s.game.trigger_state.draws[0] = u64::MAX - 1;
    s.game.players[0].hand = vec![77];
    apply(
        &mut s,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        },
    );
    apply(&mut s, Action::Pass);
    let before = views(&s);
    let full = format!("{:?}", s.game);
    let revision = s.revision();
    for command in ["validate", "apply", "step_ai", "suggest"] {
        let r = request(&mut s, command, Action::Pass);
        assert_eq!(r["ok"], false, "{command}: {r}");
        assert_eq!(r["error"]["code"], "engine_capacity", "{command}: {r}");
        assert_eq!(s.revision(), revision);
        assert_eq!(views(&s), before);
        assert_eq!(format!("{:?}", s.game), full);
    }
}

#[test]
fn overflowing_private_selection_stays_private_and_decline_can_commit() {
    let mut s = position();
    s.game.trigger_state.draws[0] = u64::MAX;
    s.game.players[0].hand = vec![389];
    s.game.players[0].deck = vec![71, 96, 96];
    apply(
        &mut s,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        },
    );
    apply(&mut s, Action::Pass);
    apply(&mut s, Action::Pass);
    let Some(Decision::DrawLook { choice_id, .. }) = s.game.decision else {
        panic!("missing look")
    };
    let before = views(&s);
    let revision = s.revision();
    assert!(before[1]["decision"].get("cards").is_none());
    for command in ["validate", "apply"] {
        let r = request(
            &mut s,
            command,
            Action::ResolveDrawLook {
                choice_id,
                index: Some(1),
            },
        );
        assert_eq!(r["error"]["code"], "engine_capacity", "{r}");
        assert_eq!(views(&s), before);
        assert_eq!(s.revision(), revision);
    }
    let suggestions = request(&mut s, "suggest", Action::Pass);
    assert_eq!(suggestions["ok"], true, "{suggestions}");
    apply(
        &mut s,
        Action::ResolveDrawLook {
            choice_id,
            index: None,
        },
    );
    assert_eq!(s.game.trigger_state.draws[0], u64::MAX);
    assert_eq!(s.game.players[0].deck.len(), 3);
}
