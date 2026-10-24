//! Registered legend powers through public actions, independent of chosen cards.
use local_rift::deck::CustomDeck;
use local_rift::game::{
    Action, ChainAbility, ChainSource, Decision, Difficulty, Game, Target, Unit,
};
use local_rift::legends::LegendAbility;

fn position(legend: usize) -> Game {
    let mut game = Game::new_with_custom(
        [0, 3],
        [Some(CustomDeck::from_legend(legend).unwrap()), None],
        963,
        Difficulty::Tactical,
        false,
    )
    .unwrap();
    game.start_match().unwrap();
    game.battlefields = [0, 0];
    for player in &mut game.players {
        player.hand.clear();
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.discard.clear();
        player.runes = [0; 6];
        player.exhausted_runes = [0; 6];
        player.energy_pool = 20;
        player.energy = 20;
        player.power_pool = [4; 6];
    }
    game
}

fn unit(game: &mut Game, player: usize, id: u32, card_id: usize, lane: Option<usize>) -> Target {
    game.players[player].units.push(Unit {
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
    if let Some(lane) = lane {
        game.battlefield_owner[lane] = Some(player);
    }
    Target::Unit {
        player,
        unit_id: id,
    }
}

fn play(game: &mut Game, card_id: usize, targets: Vec<Target>) {
    let player = game.priority();
    let hand_index = game.players[player].hand.len();
    game.players[player].hand.push(card_id);
    game.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
}

fn resolve(game: &mut Game) {
    assert!(game.decision.is_none(), "{:?}", game.decision);
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
}

fn activate(game: &mut Game) {
    game.apply(Action::ActivateLegend { target: None }).unwrap();
}

fn assert_rejected(game: &mut Game, action: Action) {
    let before = format!("{game:?}");
    assert!(game.apply(action).is_err());
    assert_eq!(format!("{game:?}"), before);
}

#[test]
fn darius_can_exhaust_with_inactive_legion_without_resources_or_priority_transfer() {
    let mut game = position(4);
    let energy = game.players[0].energy;
    assert_eq!(game.cards_played_this_turn(0), 0);
    assert!(
        game.legend_actions(0)
            .contains(&Action::ActivateLegend { target: None })
    );
    activate(&mut game);
    assert_eq!(game.players[0].energy, energy);
    assert_eq!(game.players[0].energy_pool, energy);
    assert!(!game.legend[0].ready);
    assert_eq!(game.priority(), 0);
    assert!(game.chain.is_empty());
    assert_rejected(&mut game, Action::ActivateLegend { target: None });
}

#[test]
fn darius_legion_counts_finalized_spells_before_resolution_and_after_a_counter() {
    for counter in [false, true] {
        let mut game = position(4);
        let own = unit(&mut game, 0, 100, 72, None);
        let other = unit(&mut game, 0, 101, 72, None);
        play(&mut game, 85, vec![own, other]);
        if counter {
            game.apply(Action::Pass).unwrap();
            let chain_id = game.chain[0].id;
            play(&mut game, 86, vec![Target::Spell { chain_id }]);
            resolve(&mut game);
            assert!(game.chain.is_empty());
            assert!(game.players[0].discard.contains(&85));
        }
        let chain = game.chain.clone();
        let energy = game.players[0].energy;
        assert_eq!(game.priority(), 0);
        activate(&mut game);
        assert_eq!(game.players[0].energy, energy + 1);
        assert_eq!(game.chain, chain);
        assert_eq!(game.priority(), 0);
        assert_eq!(game.cards_played_this_turn(0), 1);
    }
}

#[test]
fn darius_resource_activation_preserves_pending_payment_and_rejects_unrelated_choices() {
    let mut game = position(4);
    let first = unit(&mut game, 0, 100, 72, None);
    let second = unit(&mut game, 0, 101, 72, None);
    play(&mut game, 85, vec![first, second]);
    let chain_id = game.chain[0].id;
    game.apply(Action::Pass).unwrap();
    play(&mut game, 37, vec![Target::Spell { chain_id }]);
    resolve(&mut game);
    assert!(matches!(
        game.decision,
        Some(Decision::CounterTax {
            player: 0,
            cost: 2,
            ..
        })
    ));
    game.players[0].energy = 1;
    game.players[0].energy_pool = 1;
    let choice = game.decision.clone();
    let energy = game.players[0].energy;
    activate(&mut game);
    assert_eq!(game.decision, choice);
    assert_eq!(game.players[0].energy, energy + 1);
    assert_eq!(game.priority(), 0);
    game.apply(Action::Choose { accept: true }).unwrap();
    assert_eq!(game.players[0].energy, 0);
    assert_eq!(game.chain.len(), 1);
    assert_eq!(game.chain[0].id, chain_id);
    resolve(&mut game);
    assert_eq!(game.players[0].units[0].temporary_buff, 2);

    let mut game = position(4);
    game.decision = Some(Decision::Discard {
        player: 0,
        count: 1,
        draw_after: 2,
    });
    assert_rejected(&mut game, Action::ActivateLegend { target: None });
    assert!(game.legend[0].ready);
}

#[test]
fn darius_rejects_a_spurious_target_before_exhausting() {
    let mut game = position(4);
    let target = unit(&mut game, 0, 100, 72, None);
    assert_rejected(
        &mut game,
        Action::ActivateLegend {
            target: Some(target),
        },
    );
}

#[test]
fn darius_energy_expires_and_the_legend_readies_only_on_its_own_turn() {
    let mut game = position(4);
    play(&mut game, 96, vec![]);
    activate(&mut game);
    game.apply(Action::EndTurn).unwrap();
    assert!(!game.legend[0].ready);
    assert_eq!(game.players[0].energy_pool, 0);
    assert_eq!(game.cards_played_this_turn(0), 0);
    game.apply(Action::EndTurn).unwrap();
    assert!(game.legend[0].ready);
    assert_eq!(game.cards_played_this_turn(0), 0);
}

#[test]
fn receiving_a_played_token_does_not_enable_darius_legion() {
    let mut game = position(4);
    game.apply(Action::EndTurn).unwrap();
    game.players[1].energy_pool = 20;
    game.players[1].energy += 20;
    play(&mut game, 187, vec![]); // Walking Roost makes its opponent play a Bird.
    resolve(&mut game);
    assert!(matches!(
        game.decision,
        Some(Decision::TokenPlacement { player: 0, .. })
    ));
    game.apply(Action::PlaceToken { lane: None }).unwrap();
    assert_eq!(game.players[0].units.len(), 1);
    assert!(game.players[0].units[0].token);
    assert_eq!(game.cards_played_this_turn(0), 0);
    // The opponent's next spell supplies a real response window.
    play(&mut game, 133, vec![]);
    game.apply(Action::Pass).unwrap();
    let energy = game.players[0].energy;
    activate(&mut game);
    assert_eq!(game.players[0].energy, energy);
    assert_eq!(game.priority(), 0);
}

#[test]
fn viktor_pays_energy_before_a_respondable_ability_then_finalizes_its_recruit() {
    let mut game = position(5);
    unit(&mut game, 0, 100, 72, Some(1));
    let energy = game.players[0].energy;
    activate(&mut game);
    assert_eq!(game.players[0].energy, energy - 1);
    assert!(!game.legend[0].ready);
    assert_eq!(game.players[0].units.len(), 1);
    assert_eq!(game.chain.len(), 1);
    assert_eq!(
        game.chain_source(&game.chain[0]),
        ChainSource::Legend {
            player: 0,
            legend_id: 5
        }
    );
    assert_eq!(
        game.chain[0].ability,
        Some(ChainAbility::Legend(LegendAbility::ViktorRecruit))
    );
    game.apply(Action::Pass).unwrap();
    assert_eq!(game.priority(), 1);
    assert_eq!(game.players[0].units.len(), 1);
    game.apply(Action::Pass).unwrap();
    assert!(game.chain.is_empty());
    assert!(matches!(
        game.decision,
        Some(Decision::TokenPlacement {
            player: 0,
            remaining: 1,
            ..
        })
    ));
    assert_eq!(game.pending_tokens().len(), 1);
    assert_rejected(&mut game, Action::PlaceToken { lane: Some(0) });
    game.apply(Action::PlaceToken { lane: Some(1) }).unwrap();
    let recruit = game.players[0].units.iter().find(|u| u.token).unwrap();
    assert_eq!(game.unit_name(recruit), "Recruit token");
    assert_eq!(game.unit_might(recruit).unwrap(), 1);
    assert_eq!(recruit.lane, Some(1));
    assert!(!recruit.ready);
    assert_eq!(game.cards_played_this_turn(0), 0);
    assert!(game.pending_tokens().is_empty());
}

#[test]
fn viktor_activation_is_neutral_main_only_and_cost_failures_are_atomic() {
    let mut game = position(5);
    game.players[0].energy = 0;
    game.players[0].energy_pool = 0;
    assert_rejected(&mut game, Action::ActivateLegend { target: None });
    game.players[0].energy = 20;
    game.players[0].energy_pool = 20;
    let own = unit(&mut game, 0, 100, 72, None);
    assert_rejected(&mut game, Action::ActivateLegend { target: Some(own) });
    play(&mut game, 133, vec![]);
    assert_rejected(&mut game, Action::ActivateLegend { target: None });
    resolve(&mut game);
    game.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    assert!(game.showdown.is_some());
    assert_rejected(&mut game, Action::ActivateLegend { target: None });

    let mut game = position(5);
    game.apply(Action::EndTurn).unwrap();
    play(&mut game, 133, vec![]);
    game.apply(Action::Pass).unwrap();
    assert_eq!(game.priority(), 0);
    assert_rejected(&mut game, Action::ActivateLegend { target: None });
}

#[test]
fn viktor_token_entry_triggers_observers_after_the_legend_ability_finishes() {
    let mut game = position(5);
    unit(&mut game, 1, 200, 20, Some(1)); // Vex's opponent-played-unit trigger.
    activate(&mut game);
    resolve(&mut game);
    assert!(game.chain.is_empty());
    game.apply(Action::PlaceToken { lane: None }).unwrap();
    assert_eq!(game.cards_played_this_turn(0), 0);
    assert_eq!(game.chain.len(), 1);
    assert_eq!(game.chain[0].card_id, 20);
    assert_eq!(
        game.chain_source(&game.chain[0]),
        ChainSource::Card { card_id: 20 }
    );
    assert!(!game.players[0].units[0].stunned);
    resolve(&mut game);
    assert!(game.players[0].units[0].stunned);
}

#[test]
fn lux_uses_successful_completion_and_produces_its_own_response_window() {
    let mut game = position(6);
    play(&mut game, 77, vec![]); // Progress Day's printed cost is six.
    assert_eq!(game.chain.len(), 1);
    assert_eq!(
        game.chain_source(&game.chain[0]),
        ChainSource::Card { card_id: 77 }
    );
    assert!(game.players[0].hand.is_empty());
    resolve(&mut game);
    assert_eq!(game.players[0].hand.len(), 4);
    assert!(game.players[0].discard.contains(&77));
    assert_eq!(game.chain.len(), 1);
    assert_eq!(
        game.chain[0].ability,
        Some(ChainAbility::Legend(LegendAbility::LuxDraw))
    );
    assert_eq!(
        game.chain_source(&game.chain[0]),
        ChainSource::Legend {
            player: 0,
            legend_id: 6
        }
    );
    game.apply(Action::Pass).unwrap();
    assert_eq!(game.priority(), 1);
    assert_eq!(game.players[0].hand.len(), 4);
    game.apply(Action::Pass).unwrap();
    assert_eq!(game.players[0].hand.len(), 5);
    assert!(game.chain.is_empty());
}

#[test]
fn a_countered_qualifying_spell_does_not_trigger_lux() {
    let mut game = position(6);
    play(&mut game, 77, vec![]);
    let chain_id = game.chain[0].id;
    game.apply(Action::Pass).unwrap();
    play(&mut game, 86, vec![Target::Spell { chain_id }]);
    resolve(&mut game);
    assert!(game.chain.is_empty());
    assert!(game.players[0].hand.is_empty());
    assert_eq!(game.cards_played_this_turn(0), 1);
}

#[test]
fn lux_observes_a_qualifying_reaction_on_the_opponents_turn() {
    let mut game = position(6);
    let enemy = unit(&mut game, 1, 200, 72, None);
    game.apply(Action::EndTurn).unwrap();
    play(&mut game, 133, vec![]);
    game.apply(Action::Pass).unwrap();
    game.players[0].energy_pool = 7;
    game.players[0].energy = 7;
    play(&mut game, 79, vec![enemy]);
    resolve(&mut game);
    assert_eq!(game.chain.len(), 2);
    assert_eq!(
        game.chain.last().unwrap().ability,
        Some(ChainAbility::Legend(LegendAbility::LuxDraw))
    );
    assert_eq!(game.active, 1);
    resolve(&mut game);
    assert_eq!(game.players[0].hand.len(), 1);
}

#[test]
fn lux_does_not_observe_expensive_permanents_or_low_cost_repeated_spells() {
    let mut game = position(6);
    play(&mut game, 69, vec![]); // Mega Mech is a unit, despite its eight-energy cost.
    assert!(game.chain.is_empty());
    game.players[0].hand = vec![133];
    game.apply(Action::PlayCardOptions {
        hand_index: 0,
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: true,
        accelerate: false,
        bonus: false,
    })
    .unwrap();
    resolve(&mut game);
    assert_eq!(game.players[0].hand.len(), 2);
    assert!(game.chain.is_empty());
    assert_eq!(game.cards_played_this_turn(0), 2);
}

#[test]
fn lux_and_a_unit_observing_the_same_spell_can_be_ordered_as_one_event() {
    let mut game = position(6);
    unit(&mut game, 0, 100, 182, None);
    play(&mut game, 77, vec![]);
    resolve(&mut game);
    let Some(Decision::OrderTriggers { ref options, .. }) = game.decision else {
        panic!(
            "Expected simultaneous Lux legend/unit observers: {:?}",
            game.decision
        );
    };
    assert_eq!(options.len(), 2);
    let first = options[1].id;
    game.apply(Action::OrderTrigger { trigger_id: first })
        .unwrap();
    assert_eq!(game.chain.len(), 2);
    let sources: Vec<_> = game
        .chain
        .iter()
        .map(|item| game.chain_source(item))
        .collect();
    assert!(sources.contains(&ChainSource::Card { card_id: 182 }));
    assert!(sources.contains(&ChainSource::Legend {
        player: 0,
        legend_id: 6
    }));
    resolve(&mut game);
    resolve(&mut game);
    assert_eq!(game.players[0].hand.len(), 5);
    assert_eq!(game.players[0].units[0].temporary_buff, 3);
}

#[test]
fn automatic_players_keep_darius_ready_when_its_effect_would_be_empty() {
    for difficulty in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        let mut game = position(4);
        game.difficulty = difficulty;
        game.players[0].champion = None;
        game.step_ai().unwrap();
        assert!(game.legend[0].ready, "{difficulty:?}");
        assert_eq!(game.active, 1, "{difficulty:?}");
    }
}
