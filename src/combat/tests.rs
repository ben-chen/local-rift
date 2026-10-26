//! Controlled positions use registered card facts, then public actions.
use super::*;

fn position(attacker: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 230025, Difficulty::Tactical);
    g.start_match().unwrap();
    g.active = attacker;
    g.priority_player = attacker;
    g.battlefields = [3, 3]; // Void Gate: no extra combat/Conquer trigger.
    g.battlefield_owner = [Some(1 - attacker), None];
    for p in &mut g.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.deck = vec![71, 71, 71, 71];
        p.runes = [0; 6];
        p.exhausted_runes = [0; 6];
        p.energy_pool = 100;
        p.energy = 100;
        p.power_pool = [20; 6];
        p.any_power = 0;
    }
    for legend in &mut g.legend {
        legend.ready = false;
    }
    g.trigger_state.draws = [0; 2];
    g
}
fn unit(g: &mut Game, player: usize, card: CardId, lane: Option<usize>) -> u32 {
    let id = g.create_unit(player, card, false, 0);
    let u = g.players[player].units.last_mut().unwrap();
    u.lane = lane;
    u.ready = true;
    id
}
fn target(player: usize, unit_id: u32) -> Target {
    Target::Unit { player, unit_id }
}
fn cast(g: &mut Game, card: CardId, lane: Option<usize>, targets: Vec<Target>) {
    let p = g.priority();
    let hand_index = g.players[p].hand.len();
    g.players[p].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane,
        targets,
    })
    .unwrap_or_else(|error| panic!("cast {card} as player {p}: {error}; log={:?}", g.log));
}
fn pass_pair(g: &mut Game) {
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn start(g: &mut Game, id: u32) {
    g.apply(Action::Move {
        unit_id: id,
        lane: Some(0),
    })
    .unwrap();
    pass_pair(g);
}
fn assign_all(g: &mut Game) {
    for _ in 0..2 {
        let Some(Decision::CombatDamage {
            player,
            lane,
            total,
        }) = g.decision
        else {
            return;
        };
        let assignments = g.automatic_damage_assignment(player, lane, total).unwrap();
        g.apply(Action::AssignDamage { assignments }).unwrap();
    }
}
fn spirit(g: &mut Game, attacker: usize) -> u32 {
    let id = unit(g, attacker, 56, None); // Inferna: 1 + Assault 2.
    unit(g, 1 - attacker, 374, Some(0));
    start(g, id);
    assign_all(g);
    assert!(matches!(
        g.decision,
        Some(Decision::TriggerTargets { card_id: 374, .. })
    ));
    assert_eq!(
        g.combat_resolution().unwrap().step,
        CombatStep::DetermineResult
    );
    assert_eq!(g.battlefield_owner[0], Some(1 - attacker));
    assert!(!g.showdown_focus_open());
    id
}
fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action).is_err());
    assert_eq!(format!("{g:?}"), before);
}
fn finish_chain(g: &mut Game) {
    for _ in 0..80 {
        match g.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => g
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            Some(Decision::TokenPlacement { .. }) => {
                g.apply(Action::PlaceToken { lane: None }).unwrap()
            }
            None if !g.chain.is_empty() => g.apply(Action::Pass).unwrap(),
            None => return,
            other => panic!("unexpected decision {other:?}"),
        }
    }
    panic!("chain did not finish");
}

#[test]
fn deathknell_kills_last_attacker_before_control_score_draw_or_victory_in_both_seats() {
    for attacker in 0..2 {
        for (score, other_scored) in [(0, false), (7, false), (7, true)] {
            let mut g = position(attacker);
            g.players[attacker].score = score;
            g.player_scored_this_turn[attacker][1] = other_scored;
            let id = spirit(&mut g, attacker);
            assert_eq!(g.players[attacker].score, score);
            assert_eq!(g.trigger_state.draws, [0; 2]);
            assert_eq!(g.winner, None);
            g.apply(Action::ChooseTriggerTargets {
                targets: vec![target(attacker, id)],
            })
            .unwrap();
            assert!(g.in_combat(0));
            assert_eq!(g.players[attacker].score, score);
            assert_eq!(g.priority(), 1 - attacker);
            pass_pair(&mut g);
            assert!(g.players.iter().all(|p| p.units.is_empty()));
            assert_eq!(g.players[attacker].score, score);
            assert_eq!(g.trigger_state.draws, [0; 2]);
            assert!(!g.player_scored_this_turn[attacker][0]);
            assert_eq!(g.winner, None);
            assert_eq!(g.battlefield_owner[0], None);
            assert_eq!(g.combat_resolution(), None);
            assert!(g.showdown.is_none());
            assert_eq!(g.priority(), attacker);
            assert!(
                g.log
                    .iter()
                    .any(|line| line.contains("Combat result") && line.contains("no result"))
            );
        }
    }
}

#[test]
fn deathknell_choice_and_chain_reject_main_actions_without_reopening_combat() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        let spare = unit(&mut g, attacker, 71, None);
        let id = spirit(&mut g, attacker);
        reject(&mut g, Action::EndTurn);
        reject(
            &mut g,
            Action::Move {
                unit_id: spare,
                lane: Some(1),
            },
        );
        reject(&mut g, Action::Attack { lane: 0 });
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target(attacker, id)],
        })
        .unwrap();
        reject(&mut g, Action::EndTurn);
        reject(
            &mut g,
            Action::Move {
                unit_id: spare,
                lane: Some(1),
            },
        );
        g.players[g.priority()].hand.push(14); // Action-only Ride the Wind.
        let index = g.players[g.priority()].hand.len() - 1;
        reject(
            &mut g,
            Action::PlayCardWithTargets {
                hand_index: index,
                lane: Some(1),
                targets: vec![target(attacker, spare)],
            },
        );
        pass_pair(&mut g);
        assert_eq!(
            g.log
                .iter()
                .filter(|line| line.contains(" combat:"))
                .count(),
            1
        );
        assert!(g.showdown.is_none());
    }
}

#[test]
fn optional_deathknell_can_be_declined_then_ordinary_conquer_completes() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        let id = spirit(&mut g, attacker);
        g.apply(Action::ChooseTriggerTargets { targets: vec![] })
            .unwrap();
        assert_eq!(g.players[attacker].score, 1);
        assert_eq!(g.battlefield_owner[0], Some(attacker));
        assert_eq!(
            g.unit_might(g.unit_ref(target(attacker, id)).unwrap())
                .unwrap(),
            1
        );
        assert!(g.showdown.is_none());
        assert!(g.combat_resolution().is_none());
    }
}

#[test]
fn targeted_deathknell_remains_respondable_and_countering_it_changes_result() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        let id = spirit(&mut g, attacker);
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target(attacker, id)],
        })
        .unwrap();
        let trigger = g.chain.last().unwrap().id;
        g.apply(Action::Pass).unwrap();
        assert_eq!(g.priority(), attacker);
        cast(&mut g, 12, None, vec![Target::Spell { chain_id: trigger }]); // Not So Fast.
        pass_pair(&mut g);
        assert!(g.unit_ref(target(attacker, id)).is_some());
        assert_eq!(g.battlefield_owner[0], Some(attacker));
        assert_eq!(g.players[attacker].score, 1);
        assert!(g.showdown.is_none());
        assert_eq!(
            g.log
                .iter()
                .filter(|line| line.contains(" combat:"))
                .count(),
            1
        );
    }
}

#[test]
fn mutual_wipe_finishes_as_no_result_without_scoring() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        let id = unit(&mut g, attacker, 71, None);
        unit(&mut g, 1 - attacker, 71, Some(0));
        start(&mut g, id);
        assign_all(&mut g);
        assert_eq!(g.battlefield_owner[0], None);
        assert!(g.players.iter().all(|p| p.units.is_empty() && p.score == 0));
        assert!(
            g.log
                .iter()
                .any(|line| line.contains("Combat result") && line.contains("no result"))
        );
        assert!(!g.in_window());
    }
}

#[test]
fn combat_cleanup_recall_means_no_result_even_with_one_defender_remaining() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        let id = unit(&mut g, attacker, 70, None);
        g.unit_mut(target(attacker, id)).unwrap().stunned = true;
        unit(&mut g, 1 - attacker, 374, Some(0));
        start(&mut g, id);
        assign_all(&mut g);
        assert_eq!(g.unit_ref(target(attacker, id)).unwrap().lane, None);
        assert_eq!(g.battlefield_owner[0], Some(1 - attacker));
        assert!(g.players.iter().all(|p| p.score == 0));
        assert!(
            g.log
                .iter()
                .any(|line| line.contains("Combat result") && line.contains("no result"))
        );
        assert!(!g.log.iter().any(|line| line.contains("wins the combat")));
        assert!(!g.in_window());
    }
}

#[test]
fn special_cleanup_heals_aura_dependents_before_a_followup_normal_cleanup() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        let id = unit(&mut g, attacker, 70, None); // 10 Might.
        let captain = unit(&mut g, 1 - attacker, 320, Some(0)); // 5; other units +1.
        let phantom = unit(&mut g, 1 - attacker, 71, Some(0)); // 5 + 1.
        start(&mut g, id);
        g.apply(Action::AssignDamage {
            assignments: vec![(captain, 5), (phantom, 5)],
        })
        .unwrap();
        assign_all(&mut g);
        assert!(g.unit_ref(target(1 - attacker, captain)).is_none());
        let survivor = g.unit_ref(target(1 - attacker, phantom)).unwrap();
        assert_eq!(survivor.damage, 0);
        assert_eq!(g.unit_might(survivor).unwrap(), 5);
        assert_eq!(g.lane_count(1 - attacker, 0), 1);
        assert!(!g.in_window());
    }
}

#[test]
fn damage_reactions_keep_assault_until_end_and_are_not_healed_a_second_time() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        let id = unit(&mut g, attacker, 56, None);
        unit(&mut g, 1 - attacker, 68, Some(0)); // Watchful Sentry: Deathknell Draw.
        start(&mut g, id);
        assign_all(&mut g);
        assert_eq!(
            g.combat_resolution().unwrap().step,
            CombatStep::DetermineResult
        );
        assert_eq!(g.unit_ref(target(attacker, id)).unwrap().damage, 0);
        cast(&mut g, 297, None, vec![]); // Flurry of Blades: 1 to battlefield units.
        pass_pair(&mut g);
        let inferna = g.unit_ref(target(attacker, id)).unwrap();
        assert_eq!(inferna.damage, 2); // Void Gate adds 1 to spell damage.
        assert_eq!(g.unit_might(inferna).unwrap(), 3);
        assert_eq!(g.players[attacker].score, 0);
        assert_eq!(
            g.combat_resolution().unwrap().step,
            CombatStep::DetermineResult
        );
        pass_pair(&mut g);
        assert_eq!(g.trigger_state.draws[1 - attacker], 1);
        assert_eq!(g.players[attacker].score, 1); // Legitimate winner before expiry.
        assert!(g.unit_ref(target(attacker, id)).is_none());
        assert_eq!(g.battlefield_owner[0], None);
        let ended = g
            .log
            .iter()
            .position(|line| line.contains("Combat ends;"))
            .unwrap();
        let died = g
            .log
            .iter()
            .rposition(|line| line == "Inferna is defeated.")
            .unwrap();
        assert!(ended < died);
    }
}

#[test]
fn conquer_effects_finish_before_combat_designations_expire() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        let id = unit(&mut g, attacker, 56, None);
        g.players[attacker].gear.push(GearState {
            id: 60000,
            card_id: 201,
            lane: None,
            attached_to: Some(id),
            ready: true,
            temporary: false,
            token: false,
        });
        unit(&mut g, 1 - attacker, 374, Some(0));
        start(&mut g, id);
        assign_all(&mut g);
        g.apply(Action::ChooseTriggerTargets { targets: vec![] })
            .unwrap();
        let state = g.combat_resolution().unwrap();
        assert_eq!(state.step, CombatStep::End);
        assert_eq!(
            state.result,
            Some(CombatResult::Winner { player: attacker })
        );
        assert_eq!(g.players[attacker].score, 1);
        assert!(g.in_combat(0));
        assert!(!g.showdown_focus_open());
        assert_eq!(
            g.unit_might(g.unit_ref(target(attacker, id)).unwrap())
                .unwrap(),
            4 // Inferna + Assault 2 + equipment Might 1.
        );
        cast(&mut g, 297, None, vec![]);
        pass_pair(&mut g);
        assert!(g.unit_ref(target(attacker, id)).is_some());
        assert!(g.in_combat(0));
        finish_chain(&mut g);
        assert_eq!(
            g.players[attacker]
                .gear
                .iter()
                .filter(|gear| gear.token)
                .count(),
            1
        );
        assert!(g.unit_ref(target(attacker, id)).is_none());
        assert!(!g.in_combat(0));
    }
}

#[test]
fn deathknell_can_play_new_defenders_before_result_and_stage_a_second_combat() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        let id = unit(&mut g, attacker, 70, None);
        unit(&mut g, 1 - attacker, 384, Some(0)); // Clocktower Guardian.
        g.players[1 - attacker].deck.push(178); // Royal Guard -> Sand Soldier here.
        start(&mut g, id);
        assign_all(&mut g);
        assert_eq!(g.players[attacker].score, 0);
        pass_pair(&mut g);
        assert!(matches!(
            g.decision,
            Some(Decision::BanishPlay { card_id: 178, .. })
        ));
        assert_eq!(
            g.combat_resolution().unwrap().step,
            CombatStep::DetermineResult
        );
        g.apply(Action::Choose { accept: true }).unwrap();
        g.apply(Action::FinalizePendingPlay {
            lane: Some(0),
            targets: vec![],
            repeat_targets: vec![],
            repeat: false,
            accelerate: false,
            bonus: false,
        })
        .unwrap();
        assert_eq!(g.players[attacker].score, 0);
        assert!(g.chain.iter().any(|item| item.card_id == 178));
        finish_chain(&mut g);
        assert_eq!(g.lane_count(1 - attacker, 0), 2);
        assert_eq!(g.players[attacker].score, 0);
        assert_eq!(g.battlefield_owner[0], Some(1 - attacker));
        assert!(g.combat_resolution().is_none());
        assert!(g.showdown_focus_open());
        let window = g.showdown.as_ref().unwrap();
        assert_eq!(
            (window.lane, window.attacker, window.passes),
            (0, attacker, 0)
        );
        assert!(
            g.log
                .iter()
                .any(|line| line.contains("Combat result") && line.contains("no result"))
        );
        assert_eq!(
            g.log
                .iter()
                .filter(|line| line.contains(" combat:"))
                .count(),
            1
        );
        // The restaged combat waits for its actual new focus passes.
        pass_pair(&mut g);
        assign_all(&mut g);
        assert_eq!(g.players[attacker].score, 1);
        assert_eq!(
            g.log
                .iter()
                .filter(|line| line.contains(" combat:"))
                .count(),
            2
        );
    }
}

#[test]
fn another_staged_battlefield_waits_until_the_current_combat_really_ends() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        let id = unit(&mut g, attacker, 56, None);
        let other = unit(&mut g, attacker, 71, None);
        unit(&mut g, 1 - attacker, 374, Some(0));
        g.apply(Action::Move {
            unit_id: id,
            lane: Some(0),
        })
        .unwrap();
        cast(&mut g, 14, Some(1), vec![target(attacker, other)]); // Ride the Wind.
        pass_pair(&mut g);
        assert_eq!(g.unit_ref(target(attacker, other)).unwrap().lane, Some(1));
        pass_pair(&mut g);
        assign_all(&mut g);
        assert_eq!(g.showdown.as_ref().unwrap().lane, 0);
        assert!(g.pending_entries.iter().any(|&(lane, _)| lane == 1));
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target(attacker, id)],
        })
        .unwrap();
        assert_eq!(g.showdown.as_ref().unwrap().lane, 0);
        pass_pair(&mut g);
        assert_eq!(g.battlefield_owner[0], None);
        assert_eq!(g.showdown.as_ref().unwrap().lane, 1);
        assert_eq!(g.players[attacker].score, 0);
        pass_pair(&mut g);
        assert_eq!(g.players[attacker].score, 1);
        assert_eq!(g.battlefield_owner[1], Some(attacker));
    }
}

#[test]
fn replacement_choice_finishes_before_the_one_time_heal_and_recall() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        let id = unit(&mut g, attacker, 56, None);
        let base = unit(&mut g, attacker, 71, None);
        g.unit_mut(target(attacker, base)).unwrap().damage = 2;
        unit(&mut g, 1 - attacker, 70, Some(0));
        for gear_id in [60001, 60002] {
            g.players[attacker].gear.push(GearState {
                id: gear_id,
                card_id: 10,
                lane: None,
                attached_to: Some(id),
                ready: true,
                temporary: false,
                token: false,
            });
        }
        start(&mut g, id);
        assign_all(&mut g);
        assert!(matches!(
            g.decision,
            Some(Decision::DeathReplacement { .. })
        ));
        assert_eq!(g.combat_resolution().unwrap().step, CombatStep::Cleanup);
        assert_eq!(g.unit_ref(target(attacker, base)).unwrap().damage, 2);
        reject(&mut g, Action::Pass);
        g.apply(Action::ChooseDeathReplacement {
            unit_id: id,
            gear_id: Some(60001),
        })
        .unwrap();
        assert_eq!(g.unit_ref(target(attacker, id)).unwrap().lane, None);
        assert_eq!(g.unit_ref(target(attacker, base)).unwrap().damage, 0);
        assert_eq!(g.players[attacker].score, 0);
        assert!(
            g.log.iter().any(|line| line.contains("wins the combat")),
            "{:?}",
            g.log
        );
        // A death-replacement Recall precedes special-cleanup step 3d; only
        // recalls performed by that step itself force No Result (468.3.d).
        assert!(
            !g.log
                .iter()
                .any(|line| line.contains("Combat result") && line.contains("no result"))
        );
        assert!(!g.in_window());
    }
}

#[test]
fn defender_hidden_card_remains_until_the_establish_control_checkpoint() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        g.players[1 - attacker].hidden.push(HiddenCard {
            id: 61000,
            card_id: 188,
            lane: 0,
            hidden_turn: 0,
            revealed: false,
        });
        let id = spirit(&mut g, attacker);
        assert_eq!(g.players[1 - attacker].hidden.len(), 1);
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target(attacker, id)],
        })
        .unwrap();
        assert_eq!(g.players[1 - attacker].hidden.len(), 1);
        pass_pair(&mut g);
        assert!(g.players[1 - attacker].hidden.is_empty());
        assert!(g.players[1 - attacker].discard.contains(&188));
        assert_eq!(g.battlefield_owner[0], None);
        assert_eq!(g.players[attacker].score, 0);
    }
}

#[test]
fn leaving_and_reentering_during_the_same_combat_does_not_recheck_attack_trigger() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        let id = unit(&mut g, attacker, 166, None); // Twilight Reveler.
        let spare = unit(&mut g, attacker, 71, None);
        g.unit_mut(target(attacker, spare)).unwrap().ready = false;
        unit(&mut g, 1 - attacker, 70, Some(0));
        g.apply(Action::Move {
            unit_id: id,
            lane: Some(0),
        })
        .unwrap();
        assert!(matches!(
            g.decision,
            Some(Decision::TriggerTargets { card_id: 166, .. })
        ));
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target(attacker, spare)],
        })
        .unwrap();
        pass_pair(&mut g);
        assert!(g.unit_ref(target(attacker, spare)).unwrap().ready);
        assert_eq!(g.priority(), attacker, "Attack trigger must retain focus");
        cast(&mut g, 9, None, vec![target(attacker, id)]); // Flash to base.
        pass_pair(&mut g);
        assert_eq!(g.unit_ref(target(attacker, id)).unwrap().lane, None);
        assert!(g.trigger_state.combat_seen.contains(&id));
        // The first spell passed focus to the opponent; its pass returns it.
        g.apply(Action::Pass).unwrap();
        assert_eq!(g.priority(), attacker);
        cast(&mut g, 14, Some(0), vec![target(attacker, id)]);
        pass_pair(&mut g);
        assert!(
            g.decision.is_none(),
            "Attack is checked only once per combat"
        );
        assert_eq!(g.unit_ref(target(attacker, id)).unwrap().lane, Some(0));
        assert!(g.trigger_state.combat_seen.contains(&id));
    }
}

#[test]
fn responding_to_an_attack_trigger_does_not_transfer_the_original_focus() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        let id = unit(&mut g, attacker, 166, None);
        let spare = unit(&mut g, attacker, 71, None);
        let defender = unit(&mut g, 1 - attacker, 70, Some(0));
        g.apply(Action::Move {
            unit_id: id,
            lane: Some(0),
        })
        .unwrap();
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target(attacker, spare)],
        })
        .unwrap();
        g.apply(Action::Pass).unwrap();
        assert_eq!(g.priority(), 1 - attacker);
        cast(&mut g, 7, None, vec![target(1 - attacker, defender)]);
        finish_chain(&mut g);
        assert_eq!(g.priority(), attacker);
        assert_eq!(g.showdown.as_ref().unwrap().focus, attacker);
        assert!(g.showdown_focus_open());
        assert_eq!(g.trigger_state.draws[1 - attacker], 1);
    }
}

#[test]
fn actual_activated_ability_passes_focus_but_add_ability_does_not() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        let id = unit(&mut g, attacker, 71, None);
        let sage = unit(&mut g, attacker, 236, None);
        unit(&mut g, 1 - attacker, 70, Some(0));
        g.players[attacker].gear.push(GearState {
            id: 62000,
            card_id: 242,
            lane: None,
            attached_to: None,
            ready: true,
            temporary: false,
            token: false,
        });
        g.apply(Action::Move {
            unit_id: id,
            lane: Some(0),
        })
        .unwrap();
        assert!(g.showdown_focus_open());
        let energy = g.players[attacker].energy;
        g.apply(Action::ActivateAbility {
            source: AbilitySource::Unit {
                player: attacker,
                unit_id: sage,
            },
            ability_index: 0,
            targets: vec![],
        })
        .unwrap();
        assert_eq!(g.players[attacker].energy, energy + 1);
        assert!(g.chain.is_empty());
        assert_eq!(g.priority(), attacker);
        assert!(g.showdown_focus_open());
        g.apply(Action::ActivateAbility {
            source: AbilitySource::Gear {
                player: attacker,
                gear_id: 62000,
            },
            ability_index: 0,
            targets: vec![target(attacker, id)],
        })
        .unwrap();
        assert!(!g.chain.is_empty());
        assert!(!g.showdown_focus_open());
        pass_pair(&mut g);
        assert_eq!(g.priority(), 1 - attacker);
        assert_eq!(g.showdown.as_ref().unwrap().focus, 1 - attacker);
        assert!(g.showdown_focus_open());
    }
}

#[test]
fn instructed_permanent_and_its_tokens_preserve_trigger_opened_chain_focus() {
    for attacker in 0..2 {
        let mut g = position(attacker);
        let id = unit(&mut g, attacker, 167, None); // Crackshot Corsair.
        let guardian = unit(&mut g, 1 - attacker, 384, Some(0));
        g.unit_mut(target(1 - attacker, guardian)).unwrap().damage = 7;
        g.players[1 - attacker].deck.push(178);
        g.apply(Action::Move {
            unit_id: id,
            lane: Some(0),
        })
        .unwrap();
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target(1 - attacker, guardian)],
        })
        .unwrap();
        pass_pair(&mut g); // Attack deals 2 at Void Gate; Guardian dies.
        pass_pair(&mut g); // Actual Deathknell offers its banished Royal Guard.
        assert!(matches!(
            g.decision,
            Some(Decision::BanishPlay { card_id: 178, .. })
        ));
        g.apply(Action::Choose { accept: true }).unwrap();
        g.apply(Action::FinalizePendingPlay {
            lane: Some(0),
            targets: vec![],
            repeat_targets: vec![],
            repeat: false,
            accelerate: false,
            bonus: false,
        })
        .unwrap();
        finish_chain(&mut g);
        assert_eq!(g.lane_count(1 - attacker, 0), 2);
        assert_eq!(g.priority(), attacker);
        assert_eq!(g.showdown.as_ref().unwrap().focus, attacker);
        assert!(g.showdown_focus_open());
        assert!(g.combat_resolution().is_none());
    }
}

#[test]
fn attack_and_defend_condition_checks_remain_used_after_departure_in_both_seats() {
    // Mask of Foresight checks Alone only on first designation (383.4.e/f.2.b).
    for attacker in 0..2 {
        for defending in [false, true] {
            let player = if defending { 1 - attacker } else { attacker };
            let mut g = position(attacker);
            let subject = unit(&mut g, player, 71, Some(0));
            let companion = unit(&mut g, player, 71, Some(0));
            unit(&mut g, 1 - player, 70, Some(0));
            g.gear_play(player, 197, None).unwrap(); // Mask observes from Base.
            g.open_showdown(0, attacker).unwrap();
            assert!(g.chain.is_empty()); // not Alone on the only condition check
            assert!(g.trigger_state.combat_seen.contains(&subject));
            g.move_effect(player, target(player, subject), None, false)
                .unwrap();
            g.sync_combat_triggers();
            g.move_effect(player, target(player, companion), None, false)
                .unwrap();
            g.sync_combat_triggers();
            g.move_effect(player, target(player, subject), Some(0), false)
                .unwrap();
            g.sync_combat_triggers();
            g.advance_triggers().unwrap();
            assert!(
                g.chain.is_empty(),
                "becoming Alone later cannot retrigger Mask"
            );
            assert!(g.decision.is_none());
            assert_eq!(
                g.unit_ref(target(player, subject)).unwrap().temporary_buff,
                0
            );
        }
    }
}

#[test]
fn defend_reentry_cannot_repeat_ahri_effect_but_a_new_object_can_trigger() {
    for attacker in 0..2 {
        let defender = 1 - attacker;
        let mut g = position(attacker);
        let enemy = unit(&mut g, attacker, 70, Some(0));
        let source = unit(&mut g, defender, 168, Some(0));
        unit(&mut g, defender, 71, Some(0));
        g.open_showdown(0, attacker).unwrap();
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target(attacker, enemy)],
        })
        .unwrap();
        pass_pair(&mut g);
        let after_first = g.unit_ref(target(attacker, enemy)).unwrap().temporary_buff;
        assert_eq!(after_first, -2);
        g.move_effect(defender, target(defender, source), None, false)
            .unwrap();
        g.sync_combat_triggers();
        g.move_effect(defender, target(defender, source), Some(0), false)
            .unwrap();
        g.sync_combat_triggers();
        g.advance_triggers().unwrap();
        assert!(g.decision.is_none());
        assert!(g.chain.is_empty());
        assert_eq!(
            g.unit_ref(target(attacker, enemy)).unwrap().temporary_buff,
            after_first
        );
        // A returned/replayed card is a new physical object; it has not gained
        // this designation before, even though the printed identity matches.
        g.return_unit_to_hand(target(defender, source));
        let replayed = unit(&mut g, defender, 168, Some(0));
        assert_ne!(replayed, source);
        g.sync_combat_triggers();
        g.advance_triggers().unwrap();
        assert!(matches!(
            g.decision,
            Some(Decision::TriggerTargets { card_id: 168, .. })
        ));
        assert!(g.trigger_state.combat_seen.contains(&source));
        assert!(g.trigger_state.combat_seen.contains(&replayed));
    }
}
