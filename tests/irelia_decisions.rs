//! Reviewed Irelia decisions, separate from match-completion checks.
//! A bounded midgame board is established explicitly; public moves produce the
//! actual conquest trigger, and all subsequent payments/resolutions are real.
use local_rift::game::{Action, ChainAbility, Decision, Difficulty, Game, Target, Unit};
use local_rift::legends::{LegendAbility, LegendTrigger};

fn unit(id: u32, card_id: usize, ready: bool) -> Unit {
    Unit {
        id,
        card_id,
        lane: None,
        ready,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    }
}

fn conquest(player: usize, ready: bool, difficulty: Difficulty, rune_power: bool) -> Game {
    let decks = if player == 0 { [0, 1] } else { [1, 0] };
    let mut game = Game::new_for_agents(decks[0], decks[1], 51001, difficulty);
    game.choose_starting_player(player).unwrap();
    for seat in 0..2 {
        game.choose_battlefield(seat, 3).unwrap(); // Void Gate has no conquest cost.
        game.mulligan_player(seat, &[]).unwrap();
    }
    game.start_match().unwrap();
    for state in &mut game.players {
        state.hand.clear();
        state.units.clear();
        state.gear.clear();
        state.hidden.clear();
        state.champion = None;
        state.set_rune_setup([0; 6], [0; 6]).unwrap();
        state.energy_pool = 0;
        state.energy = 0;
        state.power_pool = [0; 6];
        state.any_power = 0;
    }
    game.players[player].energy_pool = 4;
    game.players[player].energy = 4;
    game.players[player].any_power = 1;
    if rune_power {
        game.players[player]
            .set_rune_setup([0, 3, 0, 0, 3, 0], [0; 6])
            .unwrap();
        game.players[player].energy_pool = 0;
        game.players[player].energy = 6;
        game.players[player].any_power = 0;
    }
    game.players[player].score = 6;
    game.players[player].hand = vec![7]; // Discipline chooses a friendly unit.
    game.players[player].units = vec![unit(51001, 13, true), unit(51002, 0, false)];
    game.legend[player].ready = ready;
    game.apply(Action::Move {
        unit_id: 51001,
        lane: Some(0),
    })
    .unwrap();
    for _ in 0..8 {
        if game.decision.is_some() {
            break;
        }
        game.apply(Action::Pass).unwrap();
    }
    assert_eq!(game.players[player].score, 7);
    assert!(matches!(game.decision, Some(Decision::Legend {
        player: owner, trigger: LegendTrigger::IreliaConquer,
    }) if owner == player));
    game
}

fn resolve(game: &mut Game) {
    for _ in 0..16 {
        if !game.in_window() {
            return;
        }
        assert!(
            game.decision.is_none(),
            "unexpected choice: {:?}",
            game.decision
        );
        game.apply(Action::Pass).unwrap();
    }
    panic!("response window did not finish");
}

#[test]
fn every_level_preserves_energy_when_irelias_legend_is_already_ready() {
    for player in 0..2 {
        for level in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let mut game = conquest(player, true, level, true);
            let energy = game.players[player].energy;
            assert!(game.step_ai().unwrap());
            assert_eq!(game.players[player].energy, energy, "{player}/{level:?}");
            assert!(!game.chain.iter().any(|item| matches!(
                item.ability,
                Some(ChainAbility::Legend(LegendAbility::IreliaRefresh))
            )));
        }
    }
}

#[test]
fn redundant_refresh_remains_a_legal_human_choice() {
    let mut game = conquest(0, true, Difficulty::Tactical, true);
    let energy = game.players[0].energy;
    game.apply(Action::Choose { accept: true }).unwrap();
    assert_eq!(game.players[0].energy, energy - 1);
    resolve(&mut game);
    assert!(game.legend[0].ready);
}

#[test]
fn paying_for_an_exhausted_legend_enables_a_winning_second_conquest() {
    for player in 0..2 {
        for level in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            for rune_power in [false, true] {
                let mut game = conquest(player, false, level, rune_power);
                let energy = game.players[player].energy;
                assert!(game.step_ai().unwrap());
                assert_eq!(
                    game.players[player].energy,
                    energy - 1,
                    "{player}/{level:?}"
                );
                assert!(!game.legend[player].ready, "ready must wait for responses");
                resolve(&mut game);
                assert!(game.legend[player].ready);
                game.apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![Target::Unit {
                        player,
                        unit_id: 51002,
                    }],
                })
                .unwrap();
                // Irelia's unit bonus and legend trigger share the timing boundary.
                for _ in 0..8 {
                    if matches!(
                        game.decision,
                        Some(Decision::Legend {
                            trigger: LegendTrigger::IreliaChoose { unit_id: 51002 },
                            ..
                        })
                    ) {
                        break;
                    }
                    if matches!(game.decision, Some(Decision::OrderTriggers { .. })) {
                        let action = game.legal_actions().unwrap()[0].clone();
                        game.apply(action).unwrap();
                    } else {
                        game.apply(Action::Pass).unwrap();
                    }
                }
                assert!(matches!(
                    game.decision,
                    Some(Decision::Legend {
                        trigger: LegendTrigger::IreliaChoose { unit_id: 51002 },
                        ..
                    })
                ));
                game.apply(Action::Choose { accept: true }).unwrap();
                resolve(&mut game);
                assert!(
                    game.players[player]
                        .units
                        .iter()
                        .find(|u| u.id == 51002)
                        .unwrap()
                        .ready
                );
                game.apply(Action::Move {
                    unit_id: 51002,
                    lane: Some(1),
                })
                .unwrap();
                resolve(&mut game);
                assert_eq!(game.winner, Some(player), "{player}/{level:?}");
            }
        }
    }
}

#[test]
fn strategic_executes_the_reset_choose_ready_and_winning_move() {
    for player in 0..2 {
        let mut game = conquest(player, false, Difficulty::Strategic, true);
        let turn = game.turn;
        for _ in 0..48 {
            if game.winner.is_some() || game.turn != turn {
                break;
            }
            if game.priority() == player {
                assert!(game.step_ai().unwrap());
            } else {
                // A public opponent with no response passes; the planner must
                // actually execute its line, rather than just value the reset.
                game.apply(Action::Pass).unwrap();
            }
        }
        assert_eq!(game.winner, Some(player), "{player}: {:?}", game.log);
        assert_eq!(game.turn, turn);
    }
}

#[test]
fn deeper_reset_planning_ignores_hidden_opponent_cards_and_deck_order() {
    for player in 0..2 {
        let mut first = conquest(player, false, Difficulty::Strategic, true);
        first.players[1 - player].hand = vec![1, 6, 12];
        let mut changed = first.clone();
        // Keep public hand size identical; only private identities/order differ.
        changed.players[1 - player].hand = vec![5, 7, 11];
        changed.players[1 - player].deck.reverse();
        changed.players[player].deck.reverse();
        first.step_ai().unwrap();
        changed.step_ai().unwrap();
        assert_eq!(first.players[player].energy, 5);
        assert_eq!(changed.players[player].energy, first.players[player].energy);
        assert_eq!(changed.legend[player].ready, first.legend[player].ready);
        assert_eq!(changed.chain, first.chain);
    }
}

#[test]
fn strategic_preserves_energy_without_a_followup_for_the_exhausted_legend() {
    for player in 0..2 {
        let mut game = conquest(player, false, Difficulty::Strategic, true);
        game.players[player].hand.clear();
        let energy = game.players[player].energy;
        game.step_ai().unwrap();
        assert_eq!(game.players[player].energy, energy);
        assert!(!game.legend[player].ready);
        assert!(game.chain.is_empty());
    }
}

fn unit_ready_choice(player: usize, card_id: usize, ready: bool, level: Difficulty) -> Game {
    let mut game = conquest(player, true, level, true);
    game.apply(Action::Choose { accept: false }).unwrap();
    resolve(&mut game);
    let unit = game.players[player]
        .units
        .iter_mut()
        .find(|u| u.id == 51002)
        .unwrap();
    unit.card_id = card_id;
    unit.ready = ready;
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player,
            unit_id: 51002,
        }],
    })
    .unwrap();
    for _ in 0..8 {
        if matches!(game.decision, Some(Decision::Legend {
            player: owner, trigger: LegendTrigger::IreliaChoose { unit_id: 51002 },
        }) if owner == player)
        {
            return game;
        }
        assert!(matches!(
            game.decision,
            Some(Decision::OrderTriggers { .. })
        ));
        let action = game.legal_actions().unwrap()[0].clone();
        game.apply(action).unwrap();
    }
    panic!("Irelia's choice was not reached");
}

#[test]
fn all_levels_preserve_power_and_legend_for_already_ready_units() {
    // Core 416.1.b-c: an already-ready unit cannot produce another Ready event.
    // This includes Irelia, Fervent; her when-chosen bonus is independent.
    for player in 0..2 {
        for card in [0, 2, 13, 17] {
            // Irelia, Akali, Pyke, Stellacorn Herder.
            for level in [
                Difficulty::Easy,
                Difficulty::Tactical,
                Difficulty::Strategic,
            ] {
                let mut game = unit_ready_choice(player, card, true, level);
                let runes = game.players[player].total_runes();
                game.step_ai().unwrap();
                assert_eq!(
                    game.players[player].total_runes(),
                    runes,
                    "{player}/{card}/{level:?}"
                );
                assert!(game.legend[player].ready);
                assert!(!game.chain.iter().any(|item| matches!(
                    item.ability,
                    Some(ChainAbility::Legend(LegendAbility::IreliaReady { .. }))
                )));
            }
        }
    }
}

#[test]
fn all_levels_pay_to_ready_an_exhausted_unit_for_the_winning_route() {
    for player in 0..2 {
        for card in [0, 2, 13, 17] {
            for level in [
                Difficulty::Easy,
                Difficulty::Tactical,
                Difficulty::Strategic,
            ] {
                let mut game = unit_ready_choice(player, card, false, level);
                let runes = game.players[player].total_runes();
                game.step_ai().unwrap();
                assert_eq!(
                    game.players[player].total_runes(),
                    runes - 1,
                    "{player}/{card}/{level:?}"
                );
                assert!(!game.legend[player].ready);
                resolve(&mut game);
                game.apply(Action::Move {
                    unit_id: 51002,
                    lane: Some(1),
                })
                .unwrap();
                resolve(&mut game);
                assert_eq!(game.winner, Some(player));
            }
        }
    }
}

#[test]
fn human_redundant_ready_is_legal_but_does_not_repeat_fervents_bonus() {
    for player in 0..2 {
        let mut accepted = unit_ready_choice(player, 0, true, Difficulty::Tactical);
        let mut declined = accepted.clone();
        accepted.apply(Action::Choose { accept: true }).unwrap();
        declined.apply(Action::Choose { accept: false }).unwrap();
        resolve(&mut accepted);
        resolve(&mut declined);
        let a = accepted.players[player]
            .units
            .iter()
            .find(|u| u.id == 51002)
            .unwrap();
        let d = declined.players[player]
            .units
            .iter()
            .find(|u| u.id == 51002)
            .unwrap();
        assert_eq!(
            a.temporary_buff, 3,
            "Discipline +2 and one when-chosen bonus"
        );
        assert_eq!(a, d, "paying for an already-ready unit adds no Ready bonus");
        assert_eq!(
            accepted.players[player].total_runes() + 1,
            declined.players[player].total_runes()
        );
        assert!(!accepted.legend[player].ready && declined.legend[player].ready);
    }
}
