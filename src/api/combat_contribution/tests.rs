//! Public protocol coverage from labeled, controlled board positions.
use super::*;
use crate::game::Unit;

#[test]
fn orianna_api_explains_public_gear_requirement_and_updates_after_ally_movement() {
    for owner in 0..2 {
        let mut s = position(owner);
        s.game.battlefield_owner = vec![Some(owner), Some(owner)];
        unit(&mut s, owner, 430, 43000, Some(0));
        let ally = unit(&mut s, owner, 71, 43001, None);
        let first = observed_unit(&s, owner, 43000);
        assert_eq!(first["might"], 4);
        assert_eq!(first["keywords"]["shield"], 1);
        assert_eq!(
            first["combat_contribution"]["requirement"],
            json!({"type":"friendly_gear_here","met":false})
        );
        assert_eq!(first["combat_contribution"]["damage"], 0);
        assert!(
            observed_unit(&s, owner, 43001)["combat_contribution"]
                .get("requirement")
                .is_none()
        );
        s.game.players[owner].hand.push(3);
        submit(
            &mut s,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            },
        );
        while !s.game.chain.is_empty() {
            submit(&mut s, Action::Pass);
        }
        let gear_id = s.game.players[owner].gear[0].id;
        submit(
            &mut s,
            Action::Equip {
                gear_id,
                target: ally,
            },
        );
        while !s.game.chain.is_empty() {
            submit(&mut s, Action::Pass);
        }
        assert_eq!(
            observed_unit(&s, owner, 43000)["combat_contribution"]["requirement"]["met"],
            false
        );
        let action = Action::Move {
            unit_id: 43001,
            lane: Some(0),
        };
        let before = observed_unit(&s, owner, 43000);
        assert_eq!(query(&mut s, "validate", &action)["ok"], true);
        assert_eq!(observed_unit(&s, owner, 43000), before);
        submit(&mut s, action);
        let after = observed_unit(&s, owner, 43000);
        assert_eq!(after["combat_contribution"]["requirement"]["met"], true);
        assert_eq!(after["combat_contribution"]["damage"], 4);
        assert_eq!(
            after["combat_contribution"]["prevents_combat_damage"],
            false
        );
        assert_eq!(after["might"], before["might"]);
    }
}

fn position(actor: usize) -> Session {
    let mut s = Session::new(ResetOptions {
        decks: [3, 3],
        first_player: actor,
        battlefields: Some([0, 0]),
        ..ResetOptions::default()
    })
    .unwrap();
    s.start(0).unwrap();
    s.game.battlefield_owner = vec![Some(1 - actor), None];
    for state in &mut s.game.players {
        state.units.clear();
        state.gear.clear();
        state.hidden.clear();
        state.hand.clear();
        state.discard.clear();
        state.banish.clear();
        state.champion = None;
        state.deck = vec![71; 20];
        state.set_rune_setup([0; 6], [0; 6]).unwrap();
        state.energy_pool = 100;
        state.energy = 100;
        state.power_pool = [20; 6];
        state.any_power = 0;
    }
    for legend in &mut s.game.legend {
        legend.ready = false;
    }
    s
}

fn unit(s: &mut Session, player: usize, card_id: usize, id: u32, lane: Option<usize>) -> Target {
    s.game.players[player].units.push(Unit {
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
    });
    Target::Unit {
        player,
        unit_id: id,
    }
}

fn observed_unit(s: &Session, player: usize, id: u32) -> Value {
    let before = format!("{:?}", s.game);
    let revision = s.revision();
    let views = [s.observe(0).unwrap(), s.observe(1).unwrap()];
    let find = |view: &Value| {
        view["players"][player]["units"]
            .as_array()
            .unwrap()
            .iter()
            .find(|unit| unit["id"] == id)
            .unwrap()
            .clone()
    };
    assert_eq!(find(&views[0]), find(&views[1]));
    assert_eq!(format!("{:?}", s.game), before);
    assert_eq!(s.revision(), revision);
    find(&views[0])
}

fn query(s: &mut Session, command: &str, action: &Action) -> Value {
    let before = format!("{:?}", s.game);
    let revision = s.revision();
    let response = s.handle(json!({"command":command,"revision":revision,
        "player":s.game.priority(),"action":action_to_json(action)}));
    assert_eq!(format!("{:?}", s.game), before);
    assert_eq!(s.revision(), revision);
    response
}

fn submit(s: &mut Session, action: Action) {
    let revision = s.revision();
    let response = s.handle(json!({"command":"apply","revision":revision,
        "player":s.game.priority(),"action":action_to_json(&action)}));
    assert_eq!(response["ok"], true, "{response}");
    assert_eq!(s.revision(), revision + 1);
}

fn reject(s: &mut Session, request: Value, code: &str) {
    let views = [s.observe(0).unwrap(), s.observe(1).unwrap()];
    let game = format!("{:?}", s.game);
    let revision = s.revision();
    let response = s.handle(request);
    assert_eq!(response["ok"], false, "{response}");
    assert_eq!(response["error"]["code"], code, "{response}");
    assert_eq!(s.revision(), revision);
    assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], views);
    assert_eq!(format!("{:?}", s.game), game);
}

fn cast(s: &mut Session, card: usize, targets: Vec<Target>) {
    let actor = s.game.priority();
    let hand_index = s.game.players[actor].hand.len();
    s.game.players[actor].hand.push(card);
    submit(
        s,
        Action::PlayCardWithTargets {
            hand_index,
            lane: None,
            targets,
        },
    );
}

fn passes(s: &mut Session) {
    submit(s, Action::Pass);
    submit(s, Action::Pass);
}

fn settle_chain(s: &mut Session) {
    for _ in 0..50 {
        match s.game.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => submit(
                s,
                Action::OrderTrigger {
                    trigger_id: options[0].id,
                },
            ),
            None if !s.game.chain.is_empty() => submit(s, Action::Pass),
            None => return,
            other => panic!("Unexpected decision {other:?}"),
        }
    }
    panic!("Chain did not settle");
}

fn ezreal_combat(owner: usize, defending: bool) -> (Session, Target, Target) {
    let mut s = position(if defending { 1 - owner } else { owner });
    let ezreal = unit(&mut s, owner, 412, 60000, defending.then_some(0));
    let enemy = unit(&mut s, 1 - owner, 70, 60001, (!defending).then_some(0));
    submit(
        &mut s,
        Action::Move {
            unit_id: if defending { 60001 } else { 60000 },
            lane: Some(0),
        },
    );
    assert!(matches!(
        s.game.decision,
        Some(Decision::TriggerTargets { card_id: 412, .. })
    ));
    (s, ezreal, enemy)
}

#[test]
fn observations_separate_full_might_contribution_marked_damage_and_real_stun() {
    for owner in 0..2 {
        for lane in [None, Some(0)] {
            let mut s = position(owner);
            unit(&mut s, owner, 412, 60000, lane);
            unit(&mut s, owner, 71, 60001, lane);
            s.game.players[owner].units[0].damage = 1;
            for stunned in [false, true] {
                s.game.players[owner].units[0].stunned = stunned;
                let ez = observed_unit(&s, owner, 60000);
                assert_eq!(ez["might"], 3);
                assert_eq!(ez["damage"], 1);
                assert_eq!(ez["stunned"], stunned);
                assert_eq!(
                    ez["combat_contribution"],
                    json!({"might":3,"might_exact":"3",
                    "damage":0,"damage_exact":"0","stunned":stunned,"prevents_combat_damage":true})
                );
                let ally = observed_unit(&s, owner, 60001);
                assert_eq!(ally["combat_contribution"]["damage"], 5);
                assert_eq!(ally["combat_contribution"]["prevents_combat_damage"], false);
            }
            s.game.players[owner].units[1].stunned = true;
            assert_eq!(
                observed_unit(&s, owner, 60001)["combat_contribution"]["damage"],
                0
            );
        }
    }
}

#[test]
fn contribution_exact_values_floor_negative_might_and_report_capacity_without_mutation() {
    for owner in 0..2 {
        let mut s = position(owner);
        unit(&mut s, owner, 71, 60000, None);
        for might in [(1_i64 << 53) + 1, i64::MAX, -7] {
            s.game.players[owner].units[0].temporary_buff = might - 5;
            let view = observed_unit(&s, owner, 60000);
            let expected = might.max(0) as u64;
            assert_eq!(view["signed_might_exact"], might.to_string());
            assert_eq!(
                view["combat_contribution"]["might_exact"],
                expected.to_string()
            );
            assert_eq!(
                view["combat_contribution"]["damage_exact"],
                expected.to_string()
            );
        }
        s.game.players[owner].units[0].temporary_buff = i64::MAX;
        let before = format!("{:?}", s.game);
        let revision = s.revision();
        for viewer in 0..2 {
            let result = s.handle(json!({"command":"observe","player":viewer}));
            assert_eq!(result["error"]["code"], "engine_capacity", "{result}");
            assert_eq!(format!("{:?}", s.game), before);
            assert_eq!(s.revision(), revision);
        }
    }
}

#[test]
fn rengar_wire_validation_uses_retained_attacker_after_last_friendly_leaves() {
    for actor in 0..2 {
        let mut s = position(actor);
        let attacker = unit(&mut s, actor, 71, 60000, None);
        unit(&mut s, 1 - actor, 70, 60001, Some(0));
        s.game.players[actor].hand.push(411);
        let play = |lane| Action::PlayCardWithTargets {
            hand_index: 0,
            lane: Some(lane),
            targets: vec![],
        };
        assert_eq!(
            query(&mut s, "validate", &play(0))["error"]["code"],
            "illegal_action"
        );
        submit(
            &mut s,
            Action::Move {
                unit_id: 60000,
                lane: Some(0),
            },
        );
        cast(&mut s, 9, vec![attacker]);
        passes(&mut s);
        assert_eq!(s.game.lane_count(actor, 0), 0);
        submit(&mut s, Action::Pass);
        assert_eq!(s.game.priority(), actor);
        assert_eq!(query(&mut s, "validate", &play(0))["result"]["legal"], true);
        assert_eq!(
            query(&mut s, "validate", &play(1))["error"]["code"],
            "illegal_action"
        );
        for (revision, player, action, code) in [
            (s.revision() - 1, actor, play(0), "stale_revision"),
            (s.revision(), 1 - actor, play(0), "wrong_player"),
            (s.revision(), actor, play(1), "illegal_action"),
        ] {
            reject(
                &mut s,
                json!({"command":"apply","revision":revision,"player":player,
                "action":action_to_json(&action)}),
                code,
            );
        }
        submit(&mut s, play(0));
        let rengar = s.game.players[actor]
            .units
            .iter()
            .find(|u| u.card_id == 411)
            .unwrap();
        let view = observed_unit(&s, actor, rengar.id);
        assert_eq!(view["lane"], 0);
        assert_eq!(view["ready"], false);
        assert_eq!(view["might"], 5);
        assert_eq!(view["combat_contribution"]["damage"], 5);
    }
}

#[test]
fn ezreal_attack_and_defend_wire_trigger_uses_live_might_after_a_reaction() {
    for owner in 0..2 {
        for defending in [false, true] {
            let (mut s, ezreal, enemy) = ezreal_combat(owner, defending);
            submit(
                &mut s,
                Action::ChooseTriggerTargets {
                    targets: vec![enemy],
                },
            );
            assert_eq!(observed_unit(&s, 1 - owner, 60001)["damage"], 0);
            assert!(!s.game.chain.is_empty());
            cast(&mut s, 7, vec![ezreal]);
            settle_chain(&mut s);
            let ez = observed_unit(&s, owner, 60000);
            let target = observed_unit(&s, 1 - owner, 60001);
            assert!(ez["might"].as_u64().unwrap() > 3);
            assert_eq!(target["damage"], ez["might"]);
            assert_eq!(ez["combat_contribution"]["damage"], 0);
            assert_eq!(ez["stunned"], false);
        }
    }
}

#[test]
fn ezreal_trigger_rechecks_here_after_either_physical_unit_moves() {
    for owner in 0..2 {
        for move_source in [false, true] {
            let (mut s, ezreal, enemy) = ezreal_combat(owner, false);
            submit(
                &mut s,
                Action::ChooseTriggerTargets {
                    targets: vec![enemy],
                },
            );
            let actor = if move_source { owner } else { 1 - owner };
            if s.game.priority() != actor {
                submit(&mut s, Action::Pass);
            }
            cast(&mut s, 9, vec![if move_source { ezreal } else { enemy }]);
            settle_chain(&mut s);
            assert_eq!(observed_unit(&s, 1 - owner, 60001)["damage"], 0);
        }
    }
}

#[test]
fn zero_combat_contribution_does_not_prevent_receiving_lethal_combat_damage() {
    for owner in 0..2 {
        let (mut s, _, enemy) = ezreal_combat(owner, false);
        submit(
            &mut s,
            Action::ChooseTriggerTargets {
                targets: vec![enemy],
            },
        );
        passes(&mut s);
        assert_eq!(observed_unit(&s, 1 - owner, 60001)["damage"], 3);
        passes(&mut s);
        assert!(
            matches!(s.game.decision, Some(Decision::CombatDamage { player, total:10, .. }) if player == 1-owner)
        );
        for viewer in 0..2 {
            let view = s.observe(viewer).unwrap();
            assert_eq!(view["decision"]["total_exact"], "10");
        }
        submit(
            &mut s,
            Action::AssignDamage {
                assignments: vec![(60000, 10)],
            },
        );
        assert!(s.game.players[owner].units.iter().all(|u| u.id != 60000));
        assert!(
            s.game.players[1 - owner]
                .units
                .iter()
                .any(|u| u.id == 60001)
        );
        assert_eq!(s.game.players[owner].score, 0);
    }
}

#[test]
fn self_move_payment_and_action_timing_do_not_grant_reaction_card_play() {
    for owner in 0..2 {
        let mut s = position(owner);
        unit(&mut s, owner, 412, 60000, Some(0));
        s.game.players[owner].units[0].ready = false;
        s.game.battlefield_owner[0] = Some(owner);
        let action = Action::ActivateAbility {
            source: AbilitySource::Unit {
                player: owner,
                unit_id: 60000,
            },
            ability_index: 0,
            targets: vec![],
        };
        let quote = query(&mut s, "quote_payment", &action);
        assert_eq!(quote["ok"], true, "{quote}");
        assert_eq!(quote["result"]["suggested"]["colored"][2], 1);
        let before = s.game.players[owner].power_pool[2];
        let energy = s.game.players[owner].energy;
        submit(
            &mut s,
            Action::BeginPayment {
                action: Box::new(action.clone()),
            },
        );
        assert_eq!(s.game.players[owner].power_pool[2], before);
        submit(&mut s, Action::CompletePayment);
        assert_eq!(s.game.players[owner].power_pool[2], before - 1);
        assert_eq!(s.game.players[owner].energy, energy);
        assert_eq!(observed_unit(&s, owner, 60000)["lane"], 0);
        assert_eq!(s.game.chain.len(), 1);
        let actor = s.game.priority();
        s.game.players[actor].hand.push(412);
        let play = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        };
        assert_eq!(
            query(&mut s, "validate", &play)["error"]["code"],
            "illegal_action"
        );
        let revision = s.revision();
        reject(
            &mut s,
            json!({"command":"apply","revision":revision,"player":actor,
            "action":action_to_json(&play)}),
            "illegal_action",
        );
        settle_chain(&mut s);
        let ez = observed_unit(&s, owner, 60000);
        assert!(ez["lane"].is_null());
        assert_eq!(ez["ready"], false);
        assert_eq!(
            query(&mut s, "validate", &action)["error"]["code"],
            "illegal_action"
        );
    }
}

// C27 controlled-position protocol regressions. These exercise public requests;
// separately recorded legal forty-card games cover complete match construction.
fn c27_attack(owner: usize, card: usize) -> (Session, Target, Target) {
    let mut session = position(owner);
    let source = unit(&mut session, owner, card, 60000, None);
    let enemy = unit(&mut session, 1 - owner, 70, 60001, Some(0));
    session.game.players[1 - owner].units[0].temporary_buff = 20;
    submit(
        &mut session,
        Action::Move {
            unit_id: 60000,
            lane: Some(0),
        },
    );
    assert!(matches!(session.game.decision,
        Some(Decision::TriggerTargets { card_id, .. }) if card_id == card));
    (session, source, enemy)
}

#[test]
fn c27_observations_expose_printed_tags_keyword_values_and_combat_prohibition() {
    for owner in 0..2 {
        for (card, might, assault, shield, tank, prevention) in [
            (413, 6, 0, 0, false, false),
            (414, 4, 0, 0, false, true),
            (415, 2, 1, 0, false, false),
            (416, 4, 0, 3, true, false),
        ] {
            let mut session = position(owner);
            unit(&mut session, owner, card, 60000, None);
            let view = observed_unit(&session, owner, 60000);
            assert_eq!(view["might"], might);
            assert_eq!(view["keywords"]["assault"], assault);
            assert_eq!(view["keywords"]["assault_exact"], assault.to_string());
            assert_eq!(view["keywords"]["shield"], shield);
            assert_eq!(view["keywords"]["shield_exact"], shield.to_string());
            assert_eq!(view["keywords"]["tank"], tank);
            assert_eq!(
                view["combat_contribution"]["prevents_combat_damage"],
                prevention
            );
            assert_eq!(
                view["combat_contribution"]["damage"],
                if prevention { 0 } else { might }
            );
            assert_eq!(view["stunned"], false);
            if card == 416 {
                assert_eq!(view["tags"], json!(["Noxus"]));
            }
        }
    }
}

#[test]
fn c27_yasuo_wire_trigger_rechecks_current_might_and_here() {
    for owner in 0..2 {
        for move_source in [false, true] {
            let (mut session, source, enemy) = c27_attack(owner, 413);
            let view = session.observe(owner).unwrap();
            assert_eq!(view["decision"]["target_rule"]["here_basis"], "live_source");
            submit(
                &mut session,
                Action::ChooseTriggerTargets {
                    targets: vec![enemy],
                },
            );
            cast(&mut session, if move_source { 9 } else { 7 }, vec![source]);
            settle_chain(&mut session);
            let yasuo = observed_unit(&session, owner, 60000);
            let enemy = observed_unit(&session, 1 - owner, 60001);
            if move_source {
                assert!(yasuo["lane"].is_null());
                assert_eq!(enemy["damage"], 0);
            } else {
                assert!(yasuo["might"].as_u64().unwrap() > 6);
                assert_eq!(enemy["damage"], yasuo["might"]);
            }
        }
    }
}

#[test]
fn c27_graves_wire_area_damage_avoids_choice_deflect_and_other_locations() {
    for owner in 0..2 {
        let mut session = position(owner);
        unit(&mut session, owner, 414, 60000, None);
        unit(&mut session, 1 - owner, 70, 60001, Some(0));
        unit(&mut session, 1 - owner, 92, 60002, Some(0));
        unit(&mut session, 1 - owner, 70, 60003, Some(1));
        session.game.players[1 - owner].units[1].temporary_buff = 20;
        assert_eq!(
            observed_unit(&session, 1 - owner, 60002)["keywords"]["deflect"],
            1
        );
        let pools = session.game.players[owner].power_pool;
        submit(
            &mut session,
            Action::Move {
                unit_id: 60000,
                lane: Some(0),
            },
        );
        assert!(session.game.decision.is_none());
        assert_eq!(session.game.players[owner].power_pool, pools);
        assert_eq!(session.game.chain.len(), 1);
        let revision = session.revision();
        let actor = session.game.priority();
        reject(
            &mut session,
            json!({"command":"apply","revision":revision,
            "player":actor,"action":{"type":"choose_trigger_targets","targets":[]}}),
            "illegal_action",
        );
        assert_eq!(observed_unit(&session, 1 - owner, 60001)["damage"], 0);
        settle_chain(&mut session);
        for id in [60001, 60002] {
            assert_eq!(observed_unit(&session, 1 - owner, id)["damage"], 4);
        }
        assert_eq!(observed_unit(&session, 1 - owner, 60003)["damage"], 0);
        let graves = observed_unit(&session, owner, 60000);
        assert_eq!(graves["might"], 4);
        assert_eq!(graves["combat_contribution"]["damage"], 0);
        assert_eq!(session.game.players[owner].power_pool, pools);
    }
}

#[test]
fn c27_lucian_wire_damage_tracks_current_assault_after_an_aura_departure() {
    for owner in 0..2 {
        for remove_aura in [false, true] {
            let mut session = position(owner);
            unit(&mut session, owner, 415, 60000, None);
            let aura = unit(&mut session, owner, 306, 60002, None);
            let enemy = unit(&mut session, 1 - owner, 70, 60001, Some(0));
            submit(
                &mut session,
                Action::MoveGroup {
                    unit_ids: vec![60000, 60002],
                    lane: Some(0),
                },
            );
            assert_eq!(
                observed_unit(&session, owner, 60000)["keywords"]["assault"],
                2
            );
            submit(
                &mut session,
                Action::ChooseTriggerTargets {
                    targets: vec![enemy],
                },
            );
            if remove_aura {
                cast(&mut session, 9, vec![aura]);
            }
            settle_chain(&mut session);
            let lucian = observed_unit(&session, owner, 60000);
            let assault = if remove_aura { 1 } else { 2 };
            assert_eq!(lucian["keywords"]["assault"], assault);
            assert_eq!(lucian["keywords"]["assault_exact"], assault.to_string());
            assert_eq!(lucian["might"], 2 + assault);
            assert_eq!(observed_unit(&session, 1 - owner, 60001)["damage"], assault);
        }
    }
}

fn c27_tusker_move(owner: usize) -> (Session, Target, Target) {
    let mut session = position(owner);
    session.game.battlefield_owner[0] = Some(owner);
    session.game.battlefield_owner[1] = Some(1 - owner);
    unit(&mut session, owner, 416, 60000, None);
    let aura = unit(&mut session, owner, 307, 60002, None);
    unit(&mut session, owner, 71, 60003, Some(0));
    unit(&mut session, owner, 71, 60004, None);
    let enemy = unit(&mut session, 1 - owner, 70, 60001, Some(1));
    submit(
        &mut session,
        Action::MoveGroup {
            unit_ids: vec![60000, 60002],
            lane: Some(0),
        },
    );
    assert!(matches!(
        session.game.decision,
        Some(Decision::TriggerTargets { card_id: 416, .. })
    ));
    (session, aura, enemy)
}

#[test]
fn c27_tusker_wire_optional_choice_validates_battlefields_and_rejects_base_from_coarse_pool() {
    for owner in 0..2 {
        let (mut session, _, _) = c27_tusker_move(owner);
        let view = session.observe(owner).unwrap();
        let decision = &view["decision"];
        assert_eq!(decision["type"], "trigger_targets");
        assert_eq!(decision["required"], 0);
        assert_eq!(decision["maximum"], 1);
        assert_eq!(decision["target_rule"]["battlefield"], true);
        assert_eq!(decision["target_rule"]["allegiance"], "any");
        assert!(decision["target_rule"]["here_basis"].is_null());
        let candidates = decision["candidates"].as_array().unwrap();
        for (player, id, present) in [
            (owner, 60000, true),
            (owner, 60003, true),
            // Candidates are a coarse target-kind pool, not validated choices.
            (owner, 60004, true),
            (1 - owner, 60001, true),
        ] {
            assert_eq!(
                candidates.contains(&target_to_json(&Target::Unit {
                    player,
                    unit_id: id
                })),
                present
            );
        }
        let revision = session.revision();
        let decline = Action::ChooseTriggerTargets { targets: vec![] };
        for (request_revision, player, code) in [
            (revision - 1, owner, "stale_revision"),
            (revision, 1 - owner, "wrong_player"),
        ] {
            reject(
                &mut session,
                json!({"command":"apply","revision":request_revision,
                "player":player,"action":action_to_json(&decline)}),
                code,
            );
        }
        reject(
            &mut session,
            json!({"command":"apply","revision":revision,"player":owner,
            "action":action_to_json(&Action::ChooseTriggerTargets {targets:vec![Target::Unit {player:owner,unit_id:60004}]})}),
            "illegal_action",
        );
        submit(&mut session, decline);
        settle_chain(&mut session);
        assert_eq!(observed_unit(&session, 1 - owner, 60001)["damage"], 0);
    }
}

#[test]
fn c27_tusker_wire_damage_uses_live_shield_without_defending_bonus_might() {
    for owner in 0..2 {
        for remove_aura in [false, true] {
            let (mut session, aura, enemy) = c27_tusker_move(owner);
            let before = observed_unit(&session, owner, 60000);
            assert_eq!(before["keywords"]["shield"], 4);
            assert_eq!(before["might"], 4);
            submit(
                &mut session,
                Action::ChooseTriggerTargets {
                    targets: vec![enemy],
                },
            );
            if remove_aura {
                cast(&mut session, 9, vec![aura]);
            }
            settle_chain(&mut session);
            let shield = if remove_aura { 3 } else { 4 };
            let tusker = observed_unit(&session, owner, 60000);
            assert_eq!(tusker["keywords"]["shield"], shield);
            assert_eq!(tusker["keywords"]["shield_exact"], shield.to_string());
            assert_eq!(tusker["might"], 4);
            assert_eq!(observed_unit(&session, 1 - owner, 60001)["damage"], shield);
        }
    }
}

#[test]
fn gust_api_separates_public_trash_cost_from_the_single_unit_target() {
    for owner in 0..2 {
        let mut s = position(owner);
        let target = unit(&mut s, owner, 71, 43100, None);
        s.game.players[owner].hand = vec![431];
        s.game.players[0].discard = vec![71, 7];
        s.game.players[1].discard = vec![7, 71];
        submit(
            &mut s,
            Action::WithPlayChoices {
                action: Box::new(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                }),
                choices: crate::play::PlayChoices {
                    bonus: true,
                    ..Default::default()
                },
            },
        );
        submit(
            &mut s,
            Action::ChooseTriggerTargets {
                targets: vec![target],
            },
        );
        let Some(Decision::TriggerPayment { choice_id, .. }) = s.game.decision else {
            panic!()
        };
        for viewer in 0..2 {
            let view = s.observe(viewer).unwrap();
            let choice = &view["decision"];
            assert_eq!(choice["type"], "trigger_payment");
            assert_eq!(choice["targets"], json!([target_to_json(&target)]));
            assert_eq!(choice["object_cost"]["type"], "banish_from_any_trash");
            assert_eq!(choice["object_cost"]["is_target"], false);
            assert_eq!(choice["object_cost"]["paid_before_responses"], true);
            assert_eq!(
                choice["object_cost"]["candidates"]
                    .as_array()
                    .unwrap()
                    .len(),
                4
            );
        }
        let action = Action::ResolveTriggerTrashPayment {
            choice_id,
            trash_player: 1 - owner,
            trash_index: 1,
        };
        assert_eq!(parse_action(&action_to_json(&action)).unwrap(), action);
        assert_eq!(query(&mut s, "validate", &action)["ok"], true);
        let revision = s.revision();
        reject(
            &mut s,
            json!({"command":"apply","revision":revision-1,"player":owner,"action":action_to_json(&action)}),
            "stale_revision",
        );
        reject(
            &mut s,
            json!({"command":"apply","revision":revision,"player":1-owner,"action":action_to_json(&action)}),
            "wrong_player",
        );
        let banished = s.game.players[1 - owner].discard[1];
        submit(&mut s, action);
        assert_eq!(s.game.players[1 - owner].banish, vec![banished]);
        assert_eq!(s.game.chain.last().unwrap().targets, vec![target]);
        assert_eq!(observed_unit(&s, owner, 43100)["keywords"]["assault"], 0);
        settle_chain(&mut s);
        assert_eq!(observed_unit(&s, owner, 43100)["keywords"]["assault"], 2);
    }
}

#[test]
fn edge_api_keeps_hidden_identity_private_then_exposes_delayed_attachment() {
    for owner in 0..2 {
        let mut s = position(owner);
        s.game.battlefield_owner = vec![Some(owner), None];
        let target = unit(&mut s, owner, 430, 43200, Some(0));
        let away = unit(&mut s, owner, 71, 43201, None);
        s.game.players[owner].hand = vec![432];
        submit(
            &mut s,
            Action::Hide {
                hand_index: 0,
                lane: 0,
            },
        );
        let hidden_id = s.game.players[owner].hidden[0].id;
        assert_eq!(
            s.observe(owner).unwrap()["players"][owner]["hidden"][0]["card_id"],
            432
        );
        assert!(s.observe(1 - owner).unwrap()["players"][owner]["hidden"][0]["card_id"].is_null());
        s.game.turn += 1;
        let reveal = Action::PlayHidden {
            hidden_id,
            targets: vec![],
        };
        let revision = s.revision();
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        assert_eq!(query(&mut s, "validate", &reveal)["ok"], true);
        assert_eq!(s.revision(), revision);
        assert_eq!(before, [s.observe(0).unwrap(), s.observe(1).unwrap()]);
        submit(&mut s, reveal);
        for viewer in 0..2 {
            let view = s.observe(viewer).unwrap();
            assert_eq!(view["decision"]["type"], "trigger_targets");
            assert_eq!(view["decision"]["card_id"], 432);
            assert_eq!(view["players"][owner]["gear"][0]["card_id"], 432);
            assert!(view["players"][owner]["gear"][0]["attached_to"].is_null());
        }
        assert_eq!(
            query(
                &mut s,
                "validate",
                &Action::ChooseTriggerTargets {
                    targets: vec![away]
                }
            )["ok"],
            false
        );
        submit(
            &mut s,
            Action::ChooseTriggerTargets {
                targets: vec![target],
            },
        );
        assert_eq!(
            observed_unit(&s, owner, 43200)["combat_contribution"]["damage"],
            0
        );
        settle_chain(&mut s);
        for viewer in 0..2 {
            let view = s.observe(viewer).unwrap();
            assert_eq!(view["players"][owner]["gear"][0]["attached_to"], 43200);
        }
        assert_eq!(observed_unit(&s, owner, 43200)["might"], 6);
        assert_eq!(
            observed_unit(&s, owner, 43200)["combat_contribution"]["damage"],
            6
        );
    }
}
