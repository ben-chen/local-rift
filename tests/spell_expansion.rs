//! Public actions for the second expansion's seven spells. Board fixtures
//! isolate interactions; full custom-deck games separately check conservation.
use local_rift::game::{Action, Decision, Difficulty, Game, Target, Unit};

fn position() -> Game {
    let mut game = Game::new(3, 3, 417, Difficulty::Tactical);
    game.start_match().unwrap();
    game.battlefields = [0, 0];
    game.battlefield_owner = [None; 2];
    for player in &mut game.players {
        player.hand.clear();
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.discard.clear();
        player.banish.clear();
        player.runes = [4; 6];
        player.exhausted_runes = [0; 6];
        player.energy_pool = 0;
        player.energy = 24;
        player.power_pool = [0; 6];
        player.any_power = 0;
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
fn target(player: usize, unit_id: u32) -> Target {
    Target::Unit { player, unit_id }
}
fn cast(game: &mut Game, card: usize, targets: Vec<Target>) -> Result<(), String> {
    let hand_index = game.players[game.priority()]
        .hand
        .iter()
        .position(|&id| id == card)
        .unwrap();
    game.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
}
fn resolve_one(game: &mut Game) {
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
}
fn settle(game: &mut Game) {
    for _ in 0..60 {
        assert!(
            game.decision.is_none(),
            "Unexpected choice: {:?}",
            game.decision
        );
        if game.chain.is_empty() || game.winner.is_some() {
            return;
        }
        game.apply(Action::Pass).unwrap();
    }
    panic!("Chain did not finish");
}

#[test]
fn smoke_screen_reduces_might_to_one_and_expires_at_turn_end() {
    let mut game = position();
    game.players[0].hand = vec![81];
    game.players[1].units.push(unit(100, 72, None)); // 3 Might.
    cast(&mut game, 81, vec![target(1, 100)]).unwrap();
    assert_eq!(game.unit_might(&game.players[1].units[0]).unwrap(), 3);
    settle(&mut game);
    assert_eq!(game.unit_might(&game.players[1].units[0]).unwrap(), 1);
    assert_eq!(game.players[1].units[0].temporary_buff, -2);
    game.apply(Action::EndTurn).unwrap();
    assert_eq!(game.unit_might(&game.players[1].units[0]).unwrap(), 3);
}

#[test]
fn challenge_requires_friendly_then_enemy_while_clash_allows_any_two_distinct_units() {
    for (card, owners, allowed) in [
        (82, [0, 1], true),
        (82, [1, 0], false),
        (82, [0, 0], false),
        (83, [0, 0], true),
        (83, [1, 1], true),
        (83, [1, 0], true),
    ] {
        let mut game = position();
        game.players[0].hand = vec![card];
        game.players[owners[0]].units.push(unit(100, 72, None));
        game.players[owners[1]].units.push(unit(101, 70, None)); // 10 Might.
        let before = format!("{game:?}");
        let result = cast(
            &mut game,
            card,
            vec![target(owners[0], 100), target(owners[1], 101)],
        );
        assert_eq!(result.is_ok(), allowed, "Card {card} / {owners:?}");
        if allowed {
            settle(&mut game);
            assert!(game.players[owners[0]].units.iter().all(|u| u.id != 100));
            assert_eq!(
                game.players[owners[1]]
                    .units
                    .iter()
                    .find(|u| u.id == 101)
                    .unwrap()
                    .damage,
                3
            );
        } else {
            assert_eq!(format!("{game:?}"), before);
        }
    }
    let mut game = position();
    game.players[0].hand = vec![83];
    game.players[0].units.push(unit(100, 70, None));
    assert!(cast(&mut game, 83, vec![target(0, 100), target(0, 100)]).is_err());
}

#[test]
fn fight_uses_current_might_and_unit_damage_ignores_spell_damage_field_bonus() {
    let mut game = position();
    game.battlefields[0] = 3; // Void Gate adds only spell/ability damage.
    game.battlefield_owner[0] = Some(0);
    game.players[0].hand = vec![83, 7]; // Discipline: +2 Might, draw1.
    game.players[0].units = vec![unit(100, 72, Some(0)), unit(101, 70, Some(0))];
    cast(&mut game, 83, vec![target(0, 100), target(0, 101)]).unwrap();
    cast(&mut game, 7, vec![target(0, 100)]).unwrap();
    settle(&mut game);
    assert_eq!(game.players[0].units.len(), 1);
    assert_eq!(game.players[0].units[0].id, 101);
    assert_eq!(game.players[0].units[0].damage, 5);
}

#[test]
fn fight_does_no_damage_if_one_unit_leaves_before_resolution() {
    let mut game = position();
    game.players[0].hand = vec![82];
    game.players[1].hand = vec![11]; // Gust returns a battlefield unit of at most3 Might.
    game.players[0].units.push(unit(100, 72, Some(0)));
    game.battlefield_owner[0] = Some(0);
    game.players[1].units.push(unit(101, 70, None));
    cast(&mut game, 82, vec![target(0, 100), target(1, 101)]).unwrap();
    game.apply(Action::Pass).unwrap();
    cast(&mut game, 11, vec![target(0, 100)]).unwrap();
    settle(&mut game);
    assert!(game.players[0].units.is_empty());
    assert_eq!(game.players[1].units[0].damage, 0);
}

#[test]
fn back_to_back_requires_two_friendly_targets_and_resolves_surviving_target() {
    let mut game = position();
    game.players[0].hand = vec![85];
    game.players[1].hand = vec![11];
    game.players[0].units = vec![unit(100, 72, Some(0)), unit(101, 70, None)];
    game.battlefield_owner[0] = Some(0);
    game.players[1].units.push(unit(102, 70, None));
    for targets in [
        vec![],
        vec![target(0, 100)],
        vec![target(0, 100), target(0, 100)],
        vec![target(0, 100), target(1, 102)],
    ] {
        let before = format!("{game:?}");
        assert!(cast(&mut game, 85, targets).is_err());
        assert_eq!(format!("{game:?}"), before);
    }
    cast(&mut game, 85, vec![target(0, 100), target(0, 101)]).unwrap();
    game.apply(Action::Pass).unwrap();
    cast(&mut game, 11, vec![target(0, 100)]).unwrap();
    settle(&mut game);
    assert_eq!(game.players[0].units.len(), 1);
    assert_eq!(game.unit_might(&game.players[0].units[0]).unwrap(), 12);
}

#[test]
fn lunar_boon_retains_chain_card_during_discard_and_cannot_draw_itself_on_burnout() {
    let mut game = position();
    game.players[0].hand = vec![84, 70];
    game.players[0].deck = vec![69];
    cast(&mut game, 84, vec![]).unwrap();
    resolve_one(&mut game);
    assert!(matches!(
        game.decision,
        Some(Decision::Discard {
            count: 1,
            draw_after: 2,
            ..
        })
    ));
    assert_eq!(game.chain.len(), 1);
    assert!(game.players[0].discard.is_empty());
    let before = format!("{game:?}");
    assert!(game.apply(Action::Pass).is_err());
    assert!(game.apply(Action::SelectCards { indices: vec![] }).is_err());
    assert_eq!(format!("{game:?}"), before);
    game.apply(Action::SelectCards { indices: vec![0] })
        .unwrap();
    assert_eq!(game.players[0].hand, vec![69, 70]);
    assert_eq!(game.players[0].discard, vec![84]);
    assert!(game.chain.is_empty());
    assert!(game.decision.is_none());
    assert_eq!(game.players[1].score, 1);
}

#[test]
fn lunar_boon_discards_as_many_as_possible_and_draws_before_burnout_victory() {
    let mut game = position();
    game.players[0].hand = vec![84];
    game.players[0].deck = vec![];
    game.players[0].discard = vec![69, 70];
    game.players[1].score = 7;
    cast(&mut game, 84, vec![]).unwrap();
    resolve_one(&mut game);
    game.apply(Action::SelectCards { indices: vec![] }).unwrap();
    assert_eq!(game.players[0].hand.len(), 2);
    assert!(!game.players[0].hand.contains(&84));
    assert_eq!(game.players[0].discard, vec![84]);
    assert_eq!(game.winner, Some(1));
}

#[test]
fn fizz_replayed_lunar_boon_recycles_only_after_its_draws_finish() {
    let mut game = position();
    game.players[0].hand = vec![36, 70];
    game.players[0].discard = vec![84];
    game.players[0].deck = vec![69];
    cast(&mut game, 36, vec![]).unwrap();
    assert!(matches!(game.decision, Some(Decision::OptionalPlay { .. })));
    game.apply(Action::Choose { accept: true }).unwrap();
    assert!(matches!(game.decision, Some(Decision::TrashTarget { .. })));
    game.apply(Action::SelectCards { indices: vec![0] })
        .unwrap();
    resolve_one(&mut game);
    assert!(matches!(
        game.decision,
        Some(Decision::PendingPlay { card_id: 84, .. })
    ));
    assert!(game.players[0].discard.is_empty());
    game.apply(Action::FinalizePendingPlay {
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate: false,
        bonus: false,
    })
    .unwrap();
    resolve_one(&mut game);
    assert!(game.chain.last().unwrap().recycle_after);
    game.apply(Action::SelectCards { indices: vec![0] })
        .unwrap();
    assert_eq!(game.players[0].hand, vec![69, 70]);
    assert_eq!(game.players[0].deck, vec![84]);
    assert!(game.players[0].discard.is_empty());
    assert!(game.chain.is_empty());
}

#[test]
fn wind_wall_counters_own_or_enemy_spell_regardless_of_cost() {
    for owner in [0, 1] {
        let mut game = position();
        game.players[0].hand = vec![77]; // Progress Day costs6E1P, beyond Defy's energy limit.
        game.players[owner].hand.push(86);
        let before = game.players[0].deck.len();
        cast(&mut game, 77, vec![]).unwrap();
        let chain_id = game.chain[0].id;
        if owner == 1 {
            game.apply(Action::Pass).unwrap();
        }
        cast(&mut game, 86, vec![Target::Spell { chain_id }]).unwrap();
        settle(&mut game);
        assert_eq!(game.players[0].deck.len(), before);
        assert!(game.players[0].discard.contains(&77));
        assert!(game.players[owner].discard.contains(&86));
    }
}

#[test]
fn wind_wall_cannot_counter_a_triggered_ability() {
    let mut game = position();
    game.players[0].hand = vec![35, 86]; // Ezreal's on-play discard/draw is an ability.
    cast(&mut game, 35, vec![]).unwrap();
    assert!(game.chain[0].ability.is_some());
    let chain_id = game.chain[0].id;
    let before = format!("{game:?}");
    assert!(cast(&mut game, 86, vec![Target::Spell { chain_id }]).is_err());
    assert_eq!(format!("{game:?}"), before);
}

#[test]
fn recruit_vanguard_chooses_four_legal_locations_then_plays_exhausted_one_might_tokens() {
    let mut game = position();
    game.players[0].hand = vec![87];
    game.players[0].units.push(unit(100, 72, Some(0)));
    game.battlefield_owner[0] = Some(0);
    cast(&mut game, 87, vec![]).unwrap();
    resolve_one(&mut game);
    let before = format!("{game:?}");
    assert!(game.apply(Action::PlaceToken { lane: Some(1) }).is_err());
    assert_eq!(format!("{game:?}"), before);
    for (n, lane) in [None, Some(0), None, Some(0)].into_iter().enumerate() {
        assert!(
            matches!(game.decision, Some(Decision::TokenPlacement { card_id: 87, remaining, .. }) if usize::from(remaining) == 4-n)
        );
        assert!(
            game.chain.is_empty(),
            "The parent spell finishes before token finalization"
        );
        assert_eq!(game.players[0].discard, vec![87]);
        assert_eq!(game.players[0].units.iter().filter(|u| u.token).count(), n);
        game.apply(Action::PlaceToken { lane }).unwrap();
    }
    let tokens: Vec<_> = game.players[0].units.iter().filter(|u| u.token).collect();
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens.iter().filter(|u| u.lane == Some(0)).count(), 2);
    assert!(
        tokens
            .iter()
            .all(|u| !u.ready && game.unit_might(u).unwrap() == 1)
    );
    assert!(tokens.iter().all(|u| game.unit_name(u) == "Recruit token"));
    assert_eq!(game.players[0].discard, vec![87]);
}

#[test]
fn action_and_normal_expansion_spells_cannot_respond_but_reactions_can() {
    for card in 81..=87 {
        let mut game = position();
        game.players[0].hand = vec![77];
        game.players[1].hand = vec![card];
        game.players[0].units.push(unit(100, 70, None));
        game.players[1].units = vec![unit(101, 70, None), unit(102, 70, None)];
        cast(&mut game, 77, vec![]).unwrap();
        let chain_id = game.chain[0].id;
        game.apply(Action::Pass).unwrap();
        let targets = match card {
            81 => vec![target(0, 100)],
            82 | 83 => vec![target(1, 101), target(0, 100)],
            85 => vec![target(1, 101), target(1, 102)],
            86 => vec![Target::Spell { chain_id }],
            _ => vec![],
        };
        assert_eq!(
            cast(&mut game, card, targets).is_ok(),
            matches!(card, 81 | 84 | 85 | 86),
            "card{card}"
        );
    }
}
