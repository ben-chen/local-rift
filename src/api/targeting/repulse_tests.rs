use super::*;
use crate::game::Unit;

fn apply(s: &mut Session, action: Action) {
    let response = s.handle(json!({"command":"apply", "revision":s.revision(),
        "player":s.game.priority(), "action":action_to_json(&action)}));
    assert_eq!(response["ok"], true, "{response}");
}

fn play(targets: Vec<Target>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets,
    }
}

fn position(defender: usize, repeat: bool) -> (Session, Target, Target) {
    let mut s = Session::new(ResetOptions {
        decks: [3, 3],
        first_player: 1 - defender,
        ..Default::default()
    })
    .unwrap();
    s.start(0).unwrap();
    s.game.battlefields = vec![9, 9];
    s.game.battlefield_owner = vec![None; 2];
    for (player, state) in s.game.players.iter_mut().enumerate() {
        state.hand.clear();
        state.gear.clear();
        state.hidden.clear();
        state.champion = None;
        state.units = vec![Unit {
            id: 500 + player as u32,
            card_id: 69,
            lane: Some(0),
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        }];
        state.set_rune_setup([0; 6], [0; 6]).unwrap();
        state.energy = 20;
        state.energy_pool = 20;
        state.power_pool = [10; 6];
    }
    let target = Target::Unit {
        player: defender,
        unit_id: 500 + defender as u32,
    };
    let attacker = 1 - defender;
    s.game.players[attacker].hand = vec![if repeat { 50 } else { 53 }];
    let action = if repeat {
        Action::PlayCardOptions {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: attacker,
                unit_id: 500 + attacker as u32,
            }],
            repeat_targets: vec![target],
            repeat: true,
            accelerate: false,
            bonus: false,
        }
    } else {
        play(vec![target, target])
    };
    apply(&mut s, action);
    let spell = Target::Spell {
        chain_id: s.game.chain.last().unwrap().id,
    };
    apply(&mut s, Action::Pass);
    assert_eq!(s.game.priority(), defender);
    s.game.players[defender].hand = vec![442];
    (s, target, spell)
}

#[test]
fn repulse_api_previews_both_linked_slots_without_spending_or_revealing_hands() {
    for defender in 0..2 {
        let (mut s, unit, spell) = position(defender, false);
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        let revision = s.revision();
        let spec = play_targets_to_json(442);
        assert_eq!(spec["ordered_slots"][0]["kind"], "unit");
        assert_eq!(spec["ordered_slots"][1]["includes_abilities"], true);
        assert_eq!(spec["ordered_slots"][1]["includes_repeat_choices"], true);
        for (targets, kind, candidates) in [
            (vec![], "unit", vec![unit]),
            (vec![unit], "spell", vec![spell]),
        ] {
            let response = s.handle(json!({"command":"target_candidates", "player":defender,
                "revision":revision, "execution":0, "action":action_to_json(&play(targets))}));
            assert_eq!(response["ok"], true, "{response}");
            assert_eq!(response["result"]["next_slot"]["kind"], kind);
            assert_eq!(
                response["result"]["candidates"],
                json!(candidates.iter().map(target_to_json).collect::<Vec<_>>())
            );
        }
        let action = play(vec![unit, spell]);
        for command in [
            "target_candidates",
            "play_choices",
            "cost_plan",
            "quote_payment",
            "validate",
        ] {
            let mut request = json!({"command":command, "player":defender,
                "revision":revision, "action":action_to_json(&action)});
            if command == "target_candidates" {
                request["execution"] = json!(0);
            }
            let response = s.handle(request);
            assert_eq!(response["ok"], true, "{response}");
            if command == "target_candidates" {
                assert!(response["result"]["next_slot"].is_null());
            }
            assert_eq!(s.revision(), revision);
            assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
        }
        apply(&mut s, action);
        for viewer in 0..2 {
            let observed = s.observe(viewer).unwrap();
            let counter = observed["chain"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["card_id"] == 442)
                .unwrap();
            assert_eq!(
                counter["targets"],
                json!([target_to_json(&unit), target_to_json(&spell)])
            );
        }
        for _ in 0..2 {
            apply(&mut s, Action::Pass);
        }
        assert!(s.game.chain.is_empty());
        assert_eq!(s.game.players[defender].units[0].damage, 0);
        assert_eq!(s.game.players[defender].energy, 19);
        assert_eq!(
            s.game.players[defender].power_pool[crate::game::domain_index(Domain::Body)],
            9
        );
    }
}

#[test]
fn repulse_api_rejects_incomplete_wrong_and_stale_choices_atomically() {
    for defender in 0..2 {
        let (mut s, unit, spell) = position(defender, false);
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        let revision = s.revision();
        for targets in [
            vec![],
            vec![unit],
            vec![spell, unit],
            vec![unit, Target::Spell { chain_id: 999999 }],
        ] {
            let response = s.handle(json!({"command":"apply", "player":defender,
                "revision":revision, "action":action_to_json(&play(targets))}));
            assert_eq!(response["ok"], false, "{response}");
            assert_eq!(s.revision(), revision);
            assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
        }
        let stale = s.handle(json!({"command":"target_candidates", "player":defender,
            "revision":revision - 1, "execution":0, "action":action_to_json(&play(vec![unit]))}));
        assert_eq!(stale["ok"], false);
        assert_eq!(stale["error"]["code"], "stale_revision");
        assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
        s.game.players[defender].power_pool = [0; 6];
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        let response = s.handle(json!({"command":"apply", "player":defender,
            "revision":revision, "action":action_to_json(&play(vec![unit, spell]))}));
        assert_eq!(response["ok"], false, "{response}");
        assert_eq!(s.revision(), revision);
        assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
    }
}

#[test]
fn repulse_api_finds_a_unit_chosen_only_by_repeat_and_counters_the_whole_spell() {
    for defender in 0..2 {
        let (mut s, unit, spell) = position(defender, true);
        let response = s.handle(json!({"command":"target_candidates", "player":defender,
            "revision":s.revision(), "execution":0, "action":action_to_json(&play(vec![unit]))}));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(
            response["result"]["candidates"],
            json!([target_to_json(&spell)])
        );
        apply(&mut s, play(vec![unit, spell]));
        for _ in 0..2 {
            apply(&mut s, Action::Pass);
        }
        assert!(s.game.chain.is_empty());
        for player in &s.game.players {
            assert_eq!(player.units[0].damage, 0);
        }
        assert!(s.game.players[1 - defender].discard.contains(&50));
    }
}
