//! Public protocol checks exercise the actual play and allocation commands.
use super::*;
use crate::game::Unit;

fn position(actor: usize, might: i64) -> (Session, Target, Target, Target) {
    let mut s = Session::new(ResetOptions {
        decks: [3, 3],
        first_player: actor,
        ..Default::default()
    })
    .unwrap();
    s.start(0).unwrap();
    s.game.battlefields = vec![9, 9];
    s.game.battlefield_owner = vec![None; 2];
    for p in &mut s.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.xp = 0;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy = 100;
        p.energy_pool = 100;
        p.power_pool = [20; 6];
    }
    for legend in &mut s.game.legend {
        legend.ready = false;
    }
    for (player, id, power, lane) in [
        (actor, 9001, might, None),
        (1 - actor, 9002, 9, Some(0)),
        (1 - actor, 9003, 9, Some(1)),
    ] {
        s.game.players[player].units.push(Unit {
            id,
            card_id: 69,
            lane,
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: power - 8,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        });
    }
    s.game.players[actor].hand = vec![443];
    (
        s,
        Target::Unit {
            player: actor,
            unit_id: 9001,
        },
        Target::Unit {
            player: 1 - actor,
            unit_id: 9002,
        },
        Target::Unit {
            player: 1 - actor,
            unit_id: 9003,
        },
    )
}
fn play(targets: Vec<Target>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets,
    }
}
fn request(s: &mut Session, command: &str, action: Value) -> Value {
    s.handle(json!({"command":command,"revision":s.revision(),"player":s.game.priority(),"action":action}))
}
fn apply(s: &mut Session, action: Action) {
    let result = request(s, "apply", action_to_json(&action));
    assert_eq!(result["ok"], true, "{result}");
}
fn views(s: &Session) -> [Value; 2] {
    [s.observe(0).unwrap(), s.observe(1).unwrap()]
}

// A controlled grant isolates Repeat; all subsequent choices use the public
// protocol. This fixture is not presented as a tournament-legal deck recipe.
fn repeat_play(s: &mut Session, first: Vec<Target>, second: Vec<Target>) {
    s.game.play_grants.push(crate::play::PlayGrant {
        id: 44390,
        player: s.game.priority(),
        source: crate::game::ChainSource::Card { card_id: 443 },
        expires_turn: s.game.turn,
        kind: crate::play::PlayGrantKind::RepeatNextSpell,
    });
    let base = action_to_json(&play(first));
    let before = views(s);
    let preview = request(s, "play_choices", base.clone());
    assert_eq!(preview["ok"], true, "{preview}");
    assert_eq!(views(s), before);
    let action = json!({"type":"with_play_choices", "action":base,
        "choices":{"repeats":[{"option_id":preview["result"]["repeats"][0]["id"],
        "choices":{"targets":second.iter().map(target_to_json).collect::<Vec<_>>()}}]}});
    let checked = request(s, "validate", action.clone());
    assert_eq!(checked["ok"], true, "{checked}");
    assert_eq!(views(s), before);
    let played = request(s, "apply", action);
    assert_eq!(played["ok"], true, "{played}");
    for view in views(s) {
        let chain = view["chain"].as_array().unwrap();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0]["card_id"], 443);
        assert_eq!(chain[0]["executions"].as_array().unwrap().len(), 2);
    }
}

fn order_xp(s: &mut Session, expected: usize) {
    for _ in 0..expected {
        let observed = s.observe(s.game.priority()).unwrap();
        if observed["decision"]["type"] != "order_triggers" {
            break;
        }
        let id = observed["decision"]["options"][0]["id"].as_u64().unwrap() as u32;
        apply(s, Action::OrderTrigger { trigger_id: id });
    }
    let both = views(s);
    assert_eq!(both[0]["chain"], both[1]["chain"]);
    for (viewer, view) in both.iter().enumerate() {
        assert!(view["decision"].is_null());
        assert!(view["players"][1 - viewer]["hand"].is_null());
        let rewards = view["chain"].as_array().unwrap();
        assert_eq!(rewards.len(), expected);
        for reward in rewards {
            assert_eq!(reward["card_id"], 443);
            assert_eq!(reward["ability"], json!({"type":"xp","amount":1}));
        }
        let mut ids: Vec<_> = rewards.iter().map(|r| r["id"].as_u64().unwrap()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(
            ids.len(),
            expected,
            "Each reward has its own chain identity"
        );
    }
}

#[test]
fn alpha_api_repeat_parent_rewards_are_separate_public_respondable_abilities() {
    for actor in 0..2 {
        let (mut s, source, a, b) = position(actor, 9);
        s.game.players[1 - actor].hand = vec![136]; // Frigid Touch response.
        repeat_play(&mut s, vec![source, a], vec![source, b]);
        apply(&mut s, Action::Pass);
        apply(&mut s, Action::Pass);
        assert!(s.game.players[1 - actor].units.is_empty());
        assert_eq!(s.game.players[actor].discard, vec![443]);
        order_xp(&mut s, 4);
        assert_eq!(s.game.players[actor].xp, 0);

        if s.game.priority() == actor {
            apply(&mut s, Action::Pass);
        }
        apply(&mut s, play(vec![source]));
        assert_eq!(
            s.observe(actor).unwrap()["chain"].as_array().unwrap().len(),
            5
        );
        apply(&mut s, Action::Pass);
        apply(&mut s, Action::Pass);
        assert_eq!(s.game.players[actor].xp, 0);
        order_xp(&mut s, 4);
        for xp in 1..=4 {
            apply(&mut s, Action::Pass);
            assert_eq!(s.game.players[actor].xp, xp - 1);
            apply(&mut s, Action::Pass);
            assert_eq!(s.game.players[actor].xp, xp);
        }
        assert!(s.game.chain.is_empty());
    }
}

#[test]
fn alpha_api_repeat_allocation_waits_for_parent_cleanup_and_rejects_replays() {
    for actor in 0..2 {
        let (mut s, source, a, b) = position(actor, 5);
        s.game.players[1 - actor].units[0].temporary_buff = 3 - 8;
        s.game.players[1 - actor].units[1].temporary_buff = 7 - 8;
        repeat_play(&mut s, vec![source, a, b], vec![source, a, b]);
        apply(&mut s, Action::Pass);
        apply(&mut s, Action::Pass);
        let first_revision = s.revision();
        apply(
            &mut s,
            Action::AssignDamage {
                assignments: vec![(9002, 3), (9003, 2)],
            },
        );
        assert_eq!(
            s.game.players[1 - actor].units.len(),
            2,
            "The first execution cannot kill units before the parent finishes"
        );
        assert!(!s.game.players[actor].discard.contains(&443));
        let before = views(&s);
        let revision = s.revision();
        for (player, submitted_revision, action) in [
            (
                actor,
                first_revision,
                json!({"type":"assign_damage","assignments":[[9002,3],[9003,2]]}),
            ),
            (
                1 - actor,
                revision,
                json!({"type":"assign_damage","assignments":[[9002,1],[9003,4]]}),
            ),
            (actor, revision, json!({"type":"pass"})),
            (
                actor,
                revision,
                json!({"type":"assign_damage","assignments":[[9002,0],[9003,5]]}),
            ),
        ] {
            let result = s.handle(json!({"command":"apply","player":player,
                "revision":submitted_revision,"action":action}));
            assert_eq!(result["ok"], false, "{result}");
            assert_eq!(s.revision(), revision);
            assert_eq!(views(&s), before);
        }
        apply(
            &mut s,
            Action::AssignDamage {
                assignments: vec![(9002, 1), (9003, 4)],
            },
        );
        assert_eq!(s.game.players[1 - actor].units.len(), 1);
        assert_eq!(s.game.players[1 - actor].units[0].id, 9003);
        assert_eq!(s.game.players[1 - actor].units[0].damage, 6);
        assert_eq!(s.game.players[actor].discard, vec![443]);
        order_xp(&mut s, 2);
    }
}

#[test]
fn alpha_api_target_previews_are_ordered_dynamic_and_read_only() {
    for actor in 0..2 {
        let (mut s, source, a, b) = position(actor, 5);
        let before = views(&s);
        let revision = s.revision();
        for (prefix, expected) in [
            (vec![], vec![source]),
            (vec![source], vec![a, b]),
            (vec![source, a], vec![b]),
            (vec![source, a, b], vec![]),
        ] {
            let reply = s.handle(
                json!({"command":"target_candidates","revision":revision,"player":actor,
                "execution":0,"action":action_to_json(&play(prefix))}),
            );
            assert_eq!(reply["ok"], true, "{reply}");
            assert_eq!(reply["result"]["spec"]["required"], 2);
            assert_eq!(reply["result"]["spec"]["maximum"], 3);
            assert_eq!(
                reply["result"]["candidates"],
                json!(expected.iter().map(target_to_json).collect::<Vec<_>>())
            );
            assert_eq!(s.revision(), revision);
            assert_eq!(views(&s), before);
        }
        for command in ["play_choices", "cost_plan", "quote_payment", "validate"] {
            let reply = request(&mut s, command, action_to_json(&play(vec![source, a, b])));
            assert_eq!(reply["ok"], true, "{reply}");
            assert_eq!(s.revision(), revision);
            assert_eq!(views(&s), before);
        }
        let metadata = targeting::play_targets_to_json(443);
        assert_eq!(metadata["damage_assignment"]["timing"], "resolution");
        assert_eq!(metadata["damage_assignment"]["dealer"], "chosen_unit");
        assert!(metadata["maximum"].is_null());
    }
}

#[test]
fn alpha_api_allocation_is_public_exact_and_rejects_invalid_or_stale_requests_atomically() {
    for actor in 0..2 {
        let (mut s, source, a, b) = position(actor, 5);
        apply(&mut s, play(vec![source, a, b]));
        assert!(views(&s).iter().all(|view| view["decision"].is_null()));
        apply(&mut s, Action::Pass);
        apply(&mut s, Action::Pass);
        for (viewer, view) in views(&s).into_iter().enumerate() {
            let d = &view["decision"];
            assert_eq!(d["type"], "split_damage");
            assert_eq!(d["player"], actor);
            assert_eq!(d["total_exact"], "5");
            assert_eq!(d["required_recipients"], 2);
            assert_eq!(d["lethal_first"], false);
            assert_eq!(
                d["targets"],
                json!([target_to_json(&a), target_to_json(&b)])
            );
            assert!(view["players"][1 - viewer]["hand"].is_null());
            assert!(view["players"][viewer]["hand"].is_array());
        }
        let before = views(&s);
        let revision = s.revision();
        let state = format!("{:?}", s.game);
        for assignments in [
            json!([]),
            json!([[9002, 5]]),
            json!([[9002, 0], [9003, 5]]),
            json!([[9002, 2], [9002, 3]]),
            json!([[9001, 2], [9003, 3]]),
            json!([[9002, 2], [9003, 2]]),
            json!([[9002, "02"], [9003, 3]]),
            json!([[9002, "18446744073709551616"], [9003, 3]]),
            json!([[9002, 2.5], [9003, 2.5]]),
        ] {
            for command in ["validate", "apply"] {
                let reply = request(
                    &mut s,
                    command,
                    json!({"type":"assign_damage","assignments":assignments}),
                );
                assert_eq!(reply["ok"], false, "{reply}");
                assert_eq!(s.revision(), revision);
                assert_eq!(views(&s), before);
                assert_eq!(format!("{:?}", s.game), state);
            }
        }
        let allocation = json!({"type":"assign_damage","assignments":[[9002,"2"],[9003,"3"]]});
        for (player, expected_revision) in [(1 - actor, revision), (actor, revision - 1)] {
            let reply=s.handle(json!({"command":"apply","player":player,"revision":expected_revision,"action":allocation}));
            assert_eq!(reply["ok"], false, "{reply}");
            assert_eq!(s.revision(), revision);
            assert_eq!(views(&s), before);
        }
        let reply = request(&mut s, "apply", allocation);
        assert_eq!(reply["ok"], true, "{reply}");
        assert_eq!(s.revision(), revision + 1);
        assert_eq!(s.game.players[1 - actor].units[0].damage, 2);
        assert_eq!(s.game.players[1 - actor].units[1].damage, 3);
    }
}

#[test]
fn alpha_api_large_damage_preserves_decimal_precision() {
    let total = 9_007_199_254_740_993i64;
    let (mut s, source, a, b) = position(0, total);
    apply(&mut s, play(vec![source, a, b]));
    apply(&mut s, Action::Pass);
    apply(&mut s, Action::Pass);
    assert_eq!(
        s.observe(0).unwrap()["decision"]["total_exact"],
        total.to_string()
    );
    let reply = request(
        &mut s,
        "apply",
        json!({"type":"assign_damage","assignments":[[9002,"9007199254740992"],[9003,"1"]]}),
    );
    assert_eq!(reply["ok"], true, "{reply}");
    assert!(s.game.players[1].units.iter().all(|u| u.id != 9002));
    assert_eq!(
        s.game.players[1]
            .units
            .iter()
            .find(|u| u.id == 9003)
            .unwrap()
            .damage,
        1
    );
    assert_eq!(s.game.players[0].xp, 0, "The reward is still on the chain");
}
