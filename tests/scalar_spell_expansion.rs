//! Public-action regressions for response-sensitive spells and total costs.
use local_rift::game::{Action, Decision, Difficulty, FlowSource, Game, Target, Unit};
use local_rift::gear::GearState;
use local_rift::hidden::HiddenCard;

fn position() -> Game {
    let mut game = Game::new(3, 3, 782, Difficulty::Tactical);
    game.start_match().unwrap();
    game.battlefields = [0, 0];
    game.battlefield_owner = [None; 2];
    for state in &mut game.players {
        state.hand.clear();
        state.units.clear();
        state.gear.clear();
        state.hidden.clear();
        state.discard.clear();
        state.runes = [0; 6];
        state.exhausted_runes = [0; 6];
        state.energy = 40;
        state.energy_pool = 40;
        state.power_pool = [10; 6];
        state.any_power = 0;
    }
    game
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
fn t(player: usize, unit_id: u32) -> Target {
    Target::Unit { player, unit_id }
}
fn cast(game: &mut Game, id: usize, targets: Vec<Target>) {
    let index = game.players[game.priority()]
        .hand
        .iter()
        .position(|&card| card == id)
        .unwrap();
    game.apply(Action::PlayCardWithTargets {
        hand_index: index,
        lane: None,
        targets,
    })
    .unwrap();
}
fn settle(game: &mut Game) {
    for _ in 0..150 {
        if game.winner.is_some()
            || (game.chain.is_empty() && game.decision.is_none() && !game.in_window())
        {
            return;
        }
        if game.decision.is_some() {
            game.step_ai().unwrap();
        } else {
            game.apply(Action::Pass).unwrap();
        }
    }
    panic!("Unfinished spell: {:?}", game.decision);
}
fn repeated(targets: Vec<Target>, repeat_targets: Vec<Target>) -> Action {
    Action::PlayCardOptions {
        hand_index: 0,
        lane: None,
        targets,
        repeat_targets,
        repeat: true,
        accelerate: false,
        bonus: false,
    }
}

#[test]
fn upstage_comedy_repeats_on_distinct_owners_and_pays_the_extra_cost() {
    let mut game = position();
    for owner in 0..2 {
        let mut u = unit(100 + owner as u32, 70, None);
        u.ready = false;
        game.players[owner].units.push(u);
    }
    game.players[0].hand = vec![216];
    game.apply(repeated(vec![t(0, 100)], vec![t(1, 101)]))
        .unwrap();
    assert_eq!(game.players[0].energy, 36);
    assert!(!game.players[0].units[0].ready);
    settle(&mut game);
    assert!(game.players.iter().all(|p| p.units[0].ready));
    assert_eq!(game.players[0].discard, vec![216]);
}

#[test]
fn marching_orders_restricts_enemy_to_field_and_repeat_has_no_midway_cleanup() {
    let mut game = position();
    game.players[0].hand = vec![217];
    game.players[0].units.push(unit(100, 72, None)); // 3 Might.
    game.players[1].units.push(unit(101, 73, None)); // 4 Might.
    let before = format!("{game:?}");
    assert!(
        game.apply(repeated(
            vec![t(0, 100), t(1, 101)],
            vec![t(0, 100), t(1, 101)]
        ))
        .is_err()
    );
    assert_eq!(format!("{game:?}"), before);
    game.players[1].units[0].lane = Some(0);
    game.battlefield_owner[0] = Some(1);
    game.apply(repeated(
        vec![t(0, 100), t(1, 101)],
        vec![t(0, 100), t(1, 101)],
    ))
    .unwrap();
    assert_eq!(game.players[0].energy, 34);
    settle(&mut game);
    assert!(
        game.players.iter().all(|p| p.units.is_empty()),
        "Both fight instructions finish before lethal cleanup"
    );
}

#[test]
fn gentlemens_duel_buffs_before_damage_and_keeps_buff_if_enemy_leaves() {
    for remove_enemy in [false, true] {
        let mut game = position();
        game.players[0].hand = vec![218];
        game.players[0].units.push(unit(100, 72, None));
        game.players[1].units.push(unit(101, 73, None));
        cast(&mut game, 218, vec![t(0, 100), t(1, 101)]);
        if remove_enemy {
            game.players[1].units.clear();
        }
        settle(&mut game);
        assert_eq!(game.unit_might(&game.players[0].units[0]), 6);
        assert_eq!(
            game.players[0].units[0].damage,
            if remove_enemy { 0 } else { 4 }
        );
        assert!(game.players[1].units.is_empty());
    }
}

#[test]
fn gentlemens_duel_does_not_buff_the_enemy_if_friendly_target_leaves() {
    let mut game = position();
    game.players[0].hand = vec![218];
    game.players[0].units.push(unit(100, 72, None));
    game.players[1].units.push(unit(101, 70, None));
    cast(&mut game, 218, vec![t(0, 100), t(1, 101)]);
    game.players[0].units.clear();
    settle(&mut game);
    assert_eq!(game.players[1].units[0].temporary_buff, 0);
    assert_eq!(game.players[1].units[0].damage, 0);
}

#[test]
fn wages_creates_exhausted_gold_even_when_its_damage_target_leaves() {
    for remove_target in [false, true] {
        let mut game = position();
        game.players[0].hand = vec![219];
        game.players[1].units.push(unit(100, 70, Some(0)));
        game.battlefield_owner[0] = Some(1);
        cast(&mut game, 219, vec![t(1, 100)]);
        assert!(game.players[0].gear.is_empty());
        if remove_target {
            game.players[1].units.clear();
        }
        settle(&mut game);
        assert_eq!(game.players[0].gear.len(), 1);
        assert!(game.players[0].gear[0].is_gold());
        assert!(!game.players[0].gear[0].ready);
        if !remove_target {
            assert_eq!(game.players[1].units[0].damage, 3);
        }
    }
}

#[test]
fn desert_call_repeats_as_two_independently_placed_sand_soldiers() {
    let mut game = position();
    game.players[0].hand = vec![220];
    game.players[0].units.push(unit(100, 70, Some(0)));
    game.battlefield_owner[0] = Some(0);
    game.apply(repeated(vec![], vec![])).unwrap();
    assert_eq!(game.players[0].energy, 36);
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    assert!(matches!(
        game.decision,
        Some(Decision::TokenPlacement { .. })
    ));
    assert_eq!(game.players[0].discard, vec![220]);
    game.apply(Action::PlaceToken { lane: Some(0) }).unwrap();
    game.apply(Action::PlaceToken { lane: None }).unwrap();
    let tokens: Vec<_> = game.players[0].units.iter().filter(|u| u.token).collect();
    assert_eq!(tokens.len(), 2);
    assert_eq!(
        tokens.iter().map(|u| u.lane).collect::<Vec<_>>(),
        vec![Some(0), None]
    );
    for token in tokens {
        assert_eq!(game.unit_name(token), "Sand Soldier token");
        assert_eq!(game.unit_might(token), 2);
        assert!(!token.ready);
    }
}

#[test]
fn guttural_roar_uses_empowered_state_at_resolution() {
    for empowered in [false, true] {
        let mut game = position();
        game.players[0].hand = vec![221];
        game.players[0].units.push(unit(100, 70, None));
        cast(&mut game, 221, vec![t(0, 100)]);
        if empowered {
            game.empowered_units.push(100);
        }
        settle(&mut game);
        assert_eq!(
            game.players[0].units[0].temporary_buff,
            if empowered { 4 } else { 2 }
        );
    }
}

#[test]
fn monster_harpoon_checks_current_facedown_control() {
    for has_hidden in [false, true] {
        let mut game = position();
        game.players[0].hand = vec![222];
        game.players[1].units.push(unit(100, 70, Some(0)));
        game.battlefield_owner[0] = Some(1);
        cast(&mut game, 222, vec![t(1, 100)]);
        if has_hidden {
            game.players[0].hidden.push(HiddenCard {
                id: 102,
                card_id: 21,
                lane: 1,
                hidden_turn: 0,
                revealed: false,
            });
        }
        settle(&mut game);
        assert_eq!(
            game.players[1].units[0].damage,
            if has_hidden { 4 } else { 2 }
        );
    }
}

#[test]
fn sudden_storm_only_doubles_against_an_actual_attacker() {
    for target_owner in [0, 1] {
        let mut game = position();
        game.players[0].units.push(unit(100, 70, None));
        game.players[1].units.push(unit(101, 70, Some(0)));
        game.battlefield_owner[0] = Some(1);
        game.apply(Action::Move {
            unit_id: 100,
            lane: Some(0),
        })
        .unwrap();
        let actor = game.priority();
        game.players[actor].hand = vec![223];
        cast(
            &mut game,
            223,
            vec![t(target_owner, 100 + target_owner as u32)],
        );
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert_eq!(
            game.players[target_owner].units[0].damage,
            if target_owner == 0 { 4 } else { 2 }
        );
    }
}

#[test]
fn right_of_conquest_draws_sequentially_without_drawing_itself_on_burnout() {
    let mut game = position();
    game.players[0].hand = vec![224];
    game.players[0].deck = vec![72];
    game.players[0].discard = vec![70, 73];
    game.players[0].units = vec![unit(100, 70, Some(0)), unit(101, 70, Some(1))];
    game.battlefield_owner = [Some(0); 2];
    cast(&mut game, 224, vec![]);
    settle(&mut game);
    let mut hand = game.players[0].hand.clone();
    hand.sort();
    assert_eq!(hand, vec![70, 72, 73]);
    assert_eq!(game.players[0].discard, vec![224]);
    assert_eq!(game.players[1].score, 1);
}

#[test]
fn show_of_strength_counts_current_might_including_equipment() {
    let mut game = position();
    game.players[0].hand = vec![225];
    game.players[0].units = vec![
        unit(100, 72, None),
        unit(101, 73, None),
        unit(102, 70, None),
    ];
    game.players[0].gear.push(GearState {
        id: 103,
        card_id: 117,
        attached_to: Some(100),
        ready: true,
        temporary: false,
        token: false,
    });
    cast(&mut game, 225, vec![]);
    // A response could raise a formerly non-Mighty unit before resolution.
    game.players[0].units[1].temporary_buff = 1;
    settle(&mut game);
    assert_eq!(game.players[0].hand.len(), 3);
}

#[test]
fn sky_splitter_cost_uses_highest_current_might_without_changing_printed_cost() {
    for friendly_might in [0, 3, 10] {
        let mut game = position();
        game.players[0].hand = vec![226];
        if friendly_might > 0 {
            game.players[0]
                .units
                .push(unit(100, if friendly_might == 3 { 72 } else { 70 }, None));
        }
        game.players[1].units.push(unit(101, 70, Some(0)));
        game.battlefield_owner[0] = Some(1);
        cast(&mut game, 226, vec![t(1, 101)]);
        assert_eq!(
            game.players[0].energy,
            40 - 8_u8.saturating_sub(friendly_might)
        );
        assert_eq!(game.catalog[226].cost, 8);
        settle(&mut game);
        assert_eq!(game.players[1].units[0].damage, 5);
    }
}

#[test]
fn shock_blast_discount_counts_legend_gear_or_unit_but_not_stale_identity() {
    for object in ["none", "legend", "unit", "gear", "gone"] {
        let mut game = position();
        game.players[0].hand = vec![227];
        game.players[1].units.push(unit(101, 70, Some(0)));
        game.battlefield_owner[0] = Some(1);
        match object {
            "legend" => game.legend[0].empowered = true,
            "unit" => {
                game.players[0].units.push(unit(100, 70, None));
                game.empowered_units.push(100);
            }
            "gear" => {
                game.players[0].gear.push(GearState {
                    id: 100,
                    card_id: 117,
                    attached_to: None,
                    ready: true,
                    temporary: false,
                    token: false,
                });
                game.empowered_units.push(100);
            }
            "gone" => game.empowered_units.push(100),
            _ => {}
        }
        cast(&mut game, 227, vec![t(1, 101)]);
        assert_eq!(
            game.players[0].energy,
            if matches!(object, "none" | "gone") {
                37
            } else {
                39
            }
        );
        settle(&mut game);
        assert_eq!(game.players[1].units[0].damage, 4);
    }
}

#[test]
fn against_odds_requires_a_friendly_field_target_and_counts_only_enemies_there() {
    let mut game = position();
    game.players[0].hand = vec![228];
    game.players[0].units.push(unit(100, 70, None));
    assert!(!game.valid_card_target(0, 228, None, t(0, 100), &[]));
    game.players[0].units[0].lane = Some(0);
    game.players[1].units = vec![
        unit(101, 70, Some(0)),
        unit(102, 70, Some(0)),
        unit(103, 70, Some(1)),
    ];
    assert!(!game.valid_card_target(0, 228, None, t(1, 101), &[]));
    cast(&mut game, 228, vec![t(0, 100)]);
    game.players[1].units[1].lane = Some(1);
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    assert_eq!(game.players[0].units[0].temporary_buff, 2);
}

#[test]
fn high_note_counts_exhausted_units_at_resolution_and_has_no_might_floor() {
    let mut game = position();
    game.players[0].hand = vec![229];
    game.players[0].units = (100..104).map(|id| unit(id, 70, None)).collect();
    game.players[1].units.push(unit(104, 72, None));
    cast(&mut game, 229, vec![t(1, 104)]);
    for unit in &mut game.players[0].units {
        unit.ready = false;
    }
    settle(&mut game);
    assert_eq!(game.players[1].units[0].temporary_buff, -4);
    assert_eq!(game.unit_might(&game.players[1].units[0]), 0);
}

#[test]
fn granted_flow_also_applies_live_sky_splitter_discount() {
    let mut game = position();
    game.players[0].units = vec![unit(100, 33, None), unit(101, 70, None)];
    game.players[0].discard = vec![226];
    game.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    assert!(matches!(game.decision, Some(Decision::GrantFlow { .. })));
    game.apply(Action::SelectCards { indices: vec![0] })
        .unwrap();
    settle(&mut game);
    game.players[1].units.push(unit(102, 70, Some(1)));
    game.battlefield_owner[1] = Some(1);
    let energy = game.players[0].energy;
    game.apply(Action::PlayFromTrashOptions {
        trash_index: 0,
        lane: None,
        targets: vec![t(1, 102)],
        repeat_targets: vec![],
        repeat: false,
        accelerate: false,
        bonus: false,
        flow: Some(FlowSource::Granted),
    })
    .unwrap();
    assert_eq!(game.players[0].energy, energy);
    settle(&mut game);
    assert_eq!(game.players[0].banish, vec![226]);
}

#[test]
fn convenience_ready_prefers_an_exhausted_friendly_over_a_larger_ready_unit() {
    let mut game = position();
    game.players[0].hand = vec![216];
    game.players[0].units = vec![unit(100, 70, None), unit(101, 72, None)];
    game.players[0].units[1].ready = false;
    game.apply(Action::PlayCard {
        hand_index: 0,
        lane: None,
        target: None,
    })
    .unwrap();
    assert_eq!(game.chain[0].targets, vec![t(0, 101)]);
    settle(&mut game);
    assert!(game.players[0].units[1].ready);
}
