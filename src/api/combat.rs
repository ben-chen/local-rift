//! Public combat progress. A retained showdown also stores combat designations;
//! its presence alone does not mean that players may pass focus.
use super::*;
use crate::game::{CombatResolution, CombatResult, CombatStep};

pub(super) fn resolution_to_json(progress: CombatResolution) -> Value {
    json!({
        "lane":progress.lane,
        "attacker":progress.attacker,
        // The next outstanding task, after current choices and chains finish.
        "step":match progress.step {
            CombatStep::Cleanup=>"cleanup",
            CombatStep::DetermineResult=>"determine_result",
            CombatStep::EstablishControl=>"establish_control",
            CombatStep::End=>"end",
        },
        "attackers_recalled":progress.attackers_recalled,
        // Captured at Determine Result; later responses do not rewrite it.
        "result":progress.result.map(|result|match result {
            CombatResult::Winner{player}=>json!({"type":"winner","player":player}),
            CombatResult::NoResult=>json!({"type":"no_result"}),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Unit;
    use crate::gear::GearState;

    fn position(attacker: usize) -> Session {
        let mut s = Session::new(ResetOptions {
            decks: [3, 3],
            first_player: attacker,
            battlefields: Some([3, 3]), // Void Gate; no Conquer draw.
            ..ResetOptions::default()
        })
        .unwrap();
        s.start(0).unwrap();
        s.game.battlefield_owner = [Some(1 - attacker), None];
        for state in &mut s.game.players {
            state.units.clear();
            state.gear.clear();
            state.hidden.clear();
            state.hand.clear();
            state.discard.clear();
            state.banish.clear();
            state.champion = None;
            state.deck = vec![71; 4];
            state.runes = [0; 6];
            state.exhausted_runes = [0; 6];
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

    fn unit(s: &mut Session, player: usize, card_id: usize, id: u32, lane: Option<usize>) {
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
    }

    fn submit(s: &mut Session, action: Action) {
        let response = s.handle(json!({"command":"apply","revision":s.revision(),
            "player":s.game.priority(),"action":action_to_json(&action)}));
        assert_eq!(response["ok"], true, "{response}");
    }

    fn passes(s: &mut Session) {
        submit(s, Action::Pass);
        submit(s, Action::Pass);
    }

    fn view(s: &Session) -> Value {
        let a = s.observe(0).unwrap();
        let b = s.observe(1).unwrap();
        assert_eq!(a["combat_resolution"], b["combat_resolution"]);
        assert_eq!(a["showdown"], b["showdown"]);
        a
    }

    fn reject(s: &mut Session, request: Value, code: &str) {
        let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
        let revision = s.revision();
        let response = s.handle(request);
        assert_eq!(response["ok"], false, "{response}");
        assert_eq!(response["error"]["code"], code, "{response}");
        assert_eq!(s.revision(), revision);
        assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
    }

    fn reject_main(s: &mut Session) {
        reject(
            s,
            json!({"command":"apply","revision":s.revision(),
            "player":s.game.priority(),"action":{"type":"end_turn"}}),
            "illegal_action",
        );
        let actions = s.suggestions(s.game.priority()).unwrap();
        assert!(!actions.iter().any(|suggestion| matches!(
            suggestion.action,
            Action::EndTurn
                | Action::Move { .. }
                | Action::MoveGroup { .. }
                | Action::Attack { .. }
        )));
    }

    fn damage(s: &mut Session) {
        for _ in 0..2 {
            let Some(Decision::CombatDamage {
                player,
                lane,
                total,
            }) = s.game.decision
            else {
                return;
            };
            let observed = view(s);
            assert!(observed["combat_resolution"].is_null());
            assert_eq!(observed["showdown"]["focus_open"], false);
            let assignments = s
                .game
                .automatic_damage_assignment(player, lane, total)
                .unwrap();
            submit(s, Action::AssignDamage { assignments });
        }
    }

    fn begin(s: &mut Session) {
        assert!(view(s)["combat_resolution"].is_null());
        submit(
            s,
            Action::Move {
                unit_id: 60000,
                lane: Some(0),
            },
        );
        let observed = view(s);
        assert_eq!(observed["showdown"]["focus_open"], true);
        passes(s);
        damage(s);
    }

    #[test]
    fn public_deathknell_flow_preserves_control_until_effects_finish_in_both_seats() {
        for attacker in 0..2 {
            let mut s = position(attacker);
            unit(&mut s, attacker, 56, 60000, None); // Inferna.
            unit(&mut s, 1 - attacker, 374, 60001, Some(0)); // Mournful Spirit.
            let drawn = s.game.trigger_state.draws;
            begin(&mut s);
            let observed = view(&s);
            assert_eq!(
                observed["combat_resolution"],
                json!({"lane":0,"attacker":attacker,
                "step":"determine_result","attackers_recalled":false,"result":null})
            );
            assert_eq!(observed["showdown"]["focus_open"], false);
            assert_eq!(observed["battlefield_owner"][0], 1 - attacker);
            assert_eq!(observed["players"][attacker]["score"], 0);
            assert!(matches!(
                s.game.decision,
                Some(Decision::TriggerTargets { card_id: 374, .. })
            ));
            reject_main(&mut s);
            let action = json!({"type":"choose_trigger_targets","targets":[
                {"type":"unit","player":attacker,"unit_id":60000}]});
            let actor = s.game.priority();
            let revision = s.revision();
            reject(
                &mut s,
                json!({"command":"apply","revision":revision,
                "player":1-actor,"action":action}),
                "wrong_player",
            );
            reject(
                &mut s,
                json!({"command":"apply","revision":revision-1,
                "player":actor,"action":action}),
                "stale_revision",
            );
            submit(
                &mut s,
                Action::ChooseTriggerTargets {
                    targets: vec![Target::Unit {
                        player: attacker,
                        unit_id: 60000,
                    }],
                },
            );
            assert!(!s.game.chain.is_empty());
            assert_eq!(view(&s)["combat_resolution"]["step"], "determine_result");
            assert_eq!(view(&s)["showdown"]["focus_open"], false);
            reject_main(&mut s);
            passes(&mut s);
            let completed = view(&s);
            assert!(completed["combat_resolution"].is_null());
            assert!(completed["showdown"].is_null());
            assert!(completed["battlefield_owner"][0].is_null());
            assert_eq!(completed["players"][attacker]["score"], 0);
            assert_eq!(s.game.trigger_state.draws, drawn);
            assert!(
                s.suggestions(attacker)
                    .unwrap()
                    .iter()
                    .any(|x| matches!(x.action, Action::EndTurn))
            );
        }
    }

    #[test]
    fn public_cleanup_choice_precedes_healing_and_control_in_both_seats() {
        for attacker in 0..2 {
            let mut s = position(attacker);
            unit(&mut s, attacker, 56, 60000, None);
            unit(&mut s, attacker, 71, 60002, None);
            s.game.players[attacker].units[1].damage = 2;
            unit(&mut s, 1 - attacker, 70, 60001, Some(0)); // Mountain Drake survives.
            for id in [61000, 61001] {
                s.game.players[attacker].gear.push(GearState {
                    id,
                    card_id: 10,
                    attached_to: Some(60000),
                    ready: true,
                    temporary: false,
                    token: false,
                });
            }
            begin(&mut s);
            let observed = view(&s);
            assert_eq!(observed["combat_resolution"]["step"], "cleanup");
            assert_eq!(observed["showdown"]["focus_open"], false);
            assert_eq!(observed["players"][attacker]["units"][1]["damage"], 2);
            assert!(matches!(
                s.game.decision,
                Some(Decision::DeathReplacement { .. })
            ));
            reject_main(&mut s);
            submit(
                &mut s,
                Action::ChooseDeathReplacement {
                    unit_id: 60000,
                    gear_id: Some(61000),
                },
            );
            let completed = view(&s);
            assert!(completed["combat_resolution"].is_null());
            assert!(completed["showdown"].is_null());
            assert_eq!(completed["players"][attacker]["units"][1]["damage"], 0);
            assert_eq!(completed["battlefield_owner"][0], 1 - attacker);
        }
    }

    #[test]
    fn public_conquer_chain_keeps_captured_result_and_designations_until_end() {
        for attacker in 0..2 {
            let mut s = position(attacker);
            unit(&mut s, attacker, 56, 60000, None);
            unit(&mut s, 1 - attacker, 374, 60001, Some(0));
            s.game.players[attacker].gear.push(GearState {
                id: 61000,
                card_id: 201,
                attached_to: Some(60000),
                ready: true,
                temporary: false,
                token: false,
            });
            begin(&mut s);
            submit(&mut s, Action::ChooseTriggerTargets { targets: vec![] });
            let observed = view(&s);
            assert_eq!(observed["combat_resolution"]["step"], "end");
            assert_eq!(
                observed["combat_resolution"]["result"],
                json!({"type":"winner","player":attacker})
            );
            assert_eq!(observed["showdown"]["focus_open"], false);
            assert_eq!(observed["players"][attacker]["units"][0]["might"], 4);
            assert_eq!(observed["players"][attacker]["score"], 1);
            assert!(!s.game.chain.is_empty());
            reject_main(&mut s);
            passes(&mut s); // Cull Conquer creates Gold; no additional choice.
            let completed = view(&s);
            assert!(completed["combat_resolution"].is_null());
            assert!(completed["showdown"].is_null());
            assert_eq!(completed["players"][attacker]["units"][0]["might"], 2);
            assert_eq!(
                s.game.players[attacker]
                    .gear
                    .iter()
                    .filter(|g| g.token)
                    .count(),
                1
            );
        }
    }

    #[test]
    fn public_attack_trigger_chain_returns_focus_to_the_original_holder() {
        for attacker in 0..2 {
            let mut s = position(attacker);
            unit(&mut s, attacker, 166, 60000, None); // Twilight Reveler Attack ready.
            unit(&mut s, attacker, 71, 60002, None);
            s.game.players[attacker].units[1].ready = false;
            unit(&mut s, 1 - attacker, 70, 60001, Some(0));
            submit(
                &mut s,
                Action::Move {
                    unit_id: 60000,
                    lane: Some(0),
                },
            );
            assert_eq!(view(&s)["showdown"]["focus_open"], false);
            assert!(matches!(
                s.game.decision,
                Some(Decision::TriggerTargets { card_id: 166, .. })
            ));
            submit(
                &mut s,
                Action::ChooseTriggerTargets {
                    targets: vec![Target::Unit {
                        player: attacker,
                        unit_id: 60002,
                    }],
                },
            );
            passes(&mut s);
            let observed = view(&s);
            assert_eq!(observed["showdown"]["focus_open"], true);
            assert_eq!(observed["showdown"]["focus"], attacker);
            assert_eq!(observed["priority"], attacker);
            assert!(observed["combat_resolution"].is_null());
            assert!(s.game.players[attacker].units[1].ready);
        }
    }

    // Controlled board positions followed only by public JSON action flows.
    // Initial fixture construction does not claim legal opening-deck reachability.
    fn once_focus(s: &mut Session, player: usize) {
        let observed = view(s);
        assert_eq!(observed["showdown"]["focus_open"], true);
        if s.game.priority() != player {
            submit(s, Action::Pass);
        }
        assert_eq!(s.game.priority(), player);
    }

    fn once_cast(s: &mut Session, lane: Option<usize>, targets: Vec<Target>) {
        submit(
            s,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane,
                targets,
            },
        );
        passes(s);
    }

    #[test]
    fn public_attack_and_defend_reentry_never_repeat_the_first_check() {
        for attacker in 0..2 {
            for defending in [false, true] {
                let owner = if defending { 1 - attacker } else { attacker };
                let enemy = 1 - owner;
                let mut s = position(attacker);
                unit(&mut s, owner, 168, 61000, defending.then_some(0)); // Ahri, Inquisitive.
                unit(&mut s, enemy, 71, 61001, (!defending).then_some(0));
                unit(&mut s, owner, 168, 61002, None); // A distinct physical object.
                s.game.players[owner].hand = vec![9, 14, 14]; // Flash, then Ride the Wind twice.
                submit(
                    &mut s,
                    Action::Move {
                        unit_id: if defending { 61001 } else { 61000 },
                        lane: Some(0),
                    },
                );
                let first = view(&s);
                assert_eq!(first["decision"]["type"], "trigger_targets");
                assert_eq!(first["decision"]["card_id"], 168);
                let target = Target::Unit {
                    player: enemy,
                    unit_id: 61001,
                };
                submit(
                    &mut s,
                    Action::ChooseTriggerTargets {
                        targets: vec![target],
                    },
                );
                passes(&mut s);
                once_focus(&mut s, owner);
                once_cast(
                    &mut s,
                    None,
                    vec![Target::Unit {
                        player: owner,
                        unit_id: 61000,
                    }],
                );
                assert!(
                    s.game.players[owner]
                        .units
                        .iter()
                        .find(|u| u.id == 61000)
                        .unwrap()
                        .lane
                        .is_none()
                );
                once_focus(&mut s, owner);
                once_cast(
                    &mut s,
                    Some(0),
                    vec![Target::Unit {
                        player: owner,
                        unit_id: 61000,
                    }],
                );
                let returned = view(&s);
                assert_eq!(returned["showdown"]["focus_open"], true);
                assert!(
                    returned["decision"].is_null(),
                    "same physical unit must not trigger again: {returned}"
                );
                assert!(returned["chain"].as_array().unwrap().is_empty());
                assert_eq!(
                    s.game.players[owner]
                        .units
                        .iter()
                        .find(|u| u.id == 61000)
                        .unwrap()
                        .lane,
                    Some(0)
                );
                // Another object entering this combat still gets its own first check.
                once_focus(&mut s, owner);
                once_cast(
                    &mut s,
                    Some(0),
                    vec![Target::Unit {
                        player: owner,
                        unit_id: 61002,
                    }],
                );
                let fresh = view(&s);
                assert_eq!(fresh["decision"]["type"], "trigger_targets");
                assert_eq!(fresh["decision"]["card_id"], 168);
                assert_eq!(fresh["decision"]["context"]["source"]["unit_id"], 61002);
                submit(
                    &mut s,
                    Action::ChooseTriggerTargets {
                        targets: vec![target],
                    },
                );
                passes(&mut s);
                assert_eq!(view(&s)["showdown"]["focus_open"], true);
            }
        }
    }

    #[test]
    fn public_attack_and_defend_condition_false_on_entry_is_never_rechecked() {
        for attacker in 0..2 {
            for defending in [false, true] {
                let owner = if defending { 1 - attacker } else { attacker };
                let mut s = position(attacker);
                unit(&mut s, owner, 71, 62000, defending.then_some(0));
                unit(&mut s, owner, 71, 62001, defending.then_some(0));
                unit(&mut s, 1 - owner, 70, 62002, (!defending).then_some(0));
                s.game.gear_play(owner, 197, None).unwrap(); // Mask of Foresight.
                s.game.players[owner].hand = vec![9, 14];
                if defending {
                    submit(
                        &mut s,
                        Action::Move {
                            unit_id: 62002,
                            lane: Some(0),
                        },
                    );
                } else {
                    submit(
                        &mut s,
                        Action::MoveGroup {
                            unit_ids: vec![62000, 62001],
                            lane: Some(0),
                        },
                    );
                }
                let initial = view(&s);
                assert!(initial["decision"].is_null());
                assert!(initial["chain"].as_array().unwrap().is_empty());
                assert!(
                    s.game.players[owner]
                        .units
                        .iter()
                        .all(|u| u.temporary_buff == 0)
                );
                once_focus(&mut s, owner);
                once_cast(
                    &mut s,
                    None,
                    vec![
                        Target::Unit {
                            player: owner,
                            unit_id: 62000,
                        },
                        Target::Unit {
                            player: owner,
                            unit_id: 62001,
                        },
                    ],
                );
                once_focus(&mut s, owner);
                once_cast(
                    &mut s,
                    Some(0),
                    vec![Target::Unit {
                        player: owner,
                        unit_id: 62000,
                    }],
                );
                let alone = view(&s);
                assert_eq!(alone["showdown"]["focus_open"], true);
                assert!(alone["decision"].is_null());
                assert!(alone["chain"].as_array().unwrap().is_empty());
                assert_eq!(
                    s.game.players[owner]
                        .units
                        .iter()
                        .filter(|u| u.lane == Some(0))
                        .count(),
                    1
                );
                assert_eq!(
                    s.game.players[owner]
                        .units
                        .iter()
                        .find(|u| u.id == 62000)
                        .unwrap()
                        .temporary_buff,
                    0,
                    "Mask's first false condition is consumed for this combat"
                );
            }
        }
    }
}
