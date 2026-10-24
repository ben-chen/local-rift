//! First twelve expansion cards use reviewed operations, not keyword inference.
//! These small public-API positions deliberately isolate the rule being tested;
//! custom-deck integration separately checks legal starting deck construction.
use local_rift::cards::{self, CardKind};
use local_rift::game::{Action, Difficulty, Game, Target, Unit};
use local_rift::rules::{self, OfficialEffect};

const EXPANSION: [&str; 12] = [
    "ogn-088-298",
    "ogn-142-298",
    "ogn-049-298",
    "ogn-175-298",
    "ogn-219-298",
    "ogs-003-024",
    "ogn-085-298",
    "ogn-154-298",
    "ogn-114-298",
    "sfd-087-221",
    "unl-066-219",
    "ogn-172-298",
];

fn position() -> Game {
    let mut game = Game::new(3, 3, 4711, Difficulty::Tactical);
    game.start_match().unwrap();
    game.battlefields = [0, 0]; // Sunken Temple: no damage modifier.
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
    panic!("Expansion chain failed to settle");
}

#[test]
fn append_only_ids_have_official_facts_and_explicit_support() {
    let catalog = cards::cards();
    assert!(catalog.len() >= 81);
    assert_eq!(catalog[0].official_id, "sfd-057-221");
    assert_eq!(catalog[68].official_id, "ogn-096-298");
    for (offset, official_id) in EXPANSION.iter().enumerate() {
        let card = catalog[69 + offset];
        assert_eq!(card.id, 69 + offset);
        assert_eq!(card.official_id, *official_id);
        assert_ne!(rules::effect(card.id), OfficialEffect::Unsupported);
        assert!(rules::triggers(card.id).is_empty());
        assert!(rules::static_abilities(card.id).is_empty());
    }
    let unknown = catalog.len();
    assert_eq!(rules::effect(unknown), OfficialEffect::Unsupported);
    assert_eq!(rules::effect(usize::MAX), OfficialEffect::Unsupported);
    let mut game = position();
    let mut unmapped = game.catalog[69];
    unmapped.id = unknown;
    game.catalog.push(unmapped);
    game.players[0].hand = vec![unknown];
    assert!(!game.card_supported(unknown));
    assert!(cast(&mut game, unknown, vec![]).is_err());
}

#[test]
fn five_vanilla_units_keep_empty_text_and_enter_exhausted_without_fake_triggers() {
    for id in 69..=73 {
        let mut game = position();
        let card = game.catalog[id];
        assert_eq!(card.kind, CardKind::Unit);
        assert!(card.official_text.is_empty());
        assert!(card.keywords.is_empty());
        game.players[0].hand = vec![id];
        cast(&mut game, id, vec![]).unwrap();
        let entered = &game.players[0].units[0];
        assert_eq!(entered.card_id, id);
        assert_eq!(entered.lane, None);
        assert!(!entered.ready);
        assert_eq!(game.unit_might(entered).unwrap(), u64::from(card.might));
        assert_eq!(game.players[0].energy, 24 - u16::from(card.cost));
        assert!(game.chain.is_empty());
        assert!(game.decision.is_none());
    }
}

#[test]
fn vanilla_deployment_accepts_only_base_or_controlled_battlefields() {
    for (owner, allowed) in [(None, false), (Some(1), false), (Some(0), true)] {
        let mut game = position();
        game.players[0].hand = vec![72];
        game.battlefield_owner[0] = owner;
        let result = game.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: Some(0),
            targets: vec![],
        });
        assert_eq!(result.is_ok(), allowed);
        if allowed {
            assert_eq!(game.players[0].units[0].lane, Some(0));
        } else {
            assert_eq!(game.players[0].hand, vec![72]);
        }
    }
}

#[test]
fn new_untargeted_cards_reject_explicit_targets_without_spending() {
    for id in [69, 70, 71, 72, 73, 77, 78] {
        let mut game = position();
        game.players[0].hand = vec![id];
        game.players[0].units.push(unit(100, 72, None));
        assert!(cast(&mut game, id, vec![target(0, 100)]).is_err());
        assert_eq!(game.players[0].hand, vec![id]);
        assert_eq!(game.players[0].energy, 24);
        assert_eq!(game.players[0].exhausted_runes, [0; 6]);
    }
}

#[test]
fn battlefield_spells_require_exactly_one_unit_at_a_battlefield() {
    for id in [74, 75, 80] {
        for targets in [
            vec![],
            vec![target(1, 101)],
            vec![target(1, 102), target(1, 102)],
        ] {
            let mut game = position();
            game.players[0].hand = vec![id];
            game.players[1].units = vec![unit(101, 70, None), unit(102, 70, Some(0))];
            assert!(cast(&mut game, id, targets).is_err(), "card {id}");
            assert_eq!(game.players[0].hand, vec![id]);
            assert_eq!(game.players[0].energy, 24);
        }
    }
}

#[test]
fn damage_spells_can_choose_either_players_unit_and_use_exact_amounts() {
    for (id, amount) in [(74, 2), (75, 6)] {
        for owner in 0..2 {
            let mut game = position();
            game.players[0].hand = vec![id];
            game.players[owner].units.push(unit(101, 70, Some(0)));
            game.battlefield_owner[0] = Some(owner);
            cast(&mut game, id, vec![target(owner, 101)]).unwrap();
            assert_eq!(
                game.players[owner].units[0].damage, 0,
                "Damage waits for resolution"
            );
            settle(&mut game);
            assert_eq!(game.players[owner].units[0].damage, amount);
            assert!(game.players[0].discard.contains(&id));
        }
    }
}

#[test]
fn battlefield_target_that_moves_to_base_before_resolution_is_not_damaged() {
    let mut game = position();
    game.players[0].hand = vec![75];
    game.players[1].hand = vec![9]; // Flash moves up to two friendly units to base.
    game.players[1].units.push(unit(101, 70, Some(0)));
    game.battlefield_owner[0] = Some(1);
    cast(&mut game, 75, vec![target(1, 101)]).unwrap();
    game.apply(Action::Pass).unwrap();
    cast(&mut game, 9, vec![target(1, 101)]).unwrap();
    settle(&mut game);
    assert_eq!(game.players[1].units[0].lane, None);
    assert_eq!(game.players[1].units[0].damage, 0);
    assert!(game.players[0].discard.contains(&75));
}

#[test]
fn action_and_ordinary_spells_cannot_be_used_as_reactions() {
    for response in [69, 74, 75, 76, 77, 80] {
        let mut game = position();
        game.players[0].hand = vec![77];
        game.players[1].hand = vec![response];
        game.players[0].units.push(unit(100, 70, Some(0)));
        cast(&mut game, 77, vec![]).unwrap();
        game.apply(Action::Pass).unwrap();
        assert_eq!(game.priority(), 1);
        let targets = if matches!(response, 74 | 75 | 76 | 80) {
            vec![target(0, 100)]
        } else {
            vec![]
        };
        assert!(cast(&mut game, response, targets).is_err());
        assert_eq!(game.players[1].energy, 24);
        assert_eq!(game.players[1].hand, vec![response]);
    }
}

#[test]
fn premonition_responds_and_draws_three_before_progress_day_draws_four() {
    let mut game = position();
    game.players[0].hand = vec![77];
    game.players[1].hand = vec![78];
    game.players[0].deck = vec![69, 70, 71, 72];
    game.players[1].deck = vec![73, 72, 71];
    cast(&mut game, 77, vec![]).unwrap();
    game.apply(Action::Pass).unwrap();
    cast(&mut game, 78, vec![]).unwrap();
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    assert_eq!(game.players[1].hand, vec![71, 72, 73]);
    assert!(game.players[0].hand.is_empty());
    settle(&mut game);
    assert_eq!(game.players[0].hand, vec![72, 71, 70, 69]);
}

#[test]
fn premonition_requires_three_matching_power_and_does_not_spend_on_failure() {
    let mut game = position();
    game.players[0].hand = vec![78];
    game.players[0].runes = [10, 0, 2, 0, 0, 0];
    game.players[0].energy = 12;
    assert!(cast(&mut game, 78, vec![]).is_err());
    assert_eq!(game.players[0].runes, [10, 0, 2, 0, 0, 0]);
    assert_eq!(game.players[0].exhausted_runes, [0; 6]);
    assert_eq!(game.players[0].energy, 12);
    game.players[0].runes = [9, 0, 3, 0, 0, 0];
    cast(&mut game, 78, vec![]).unwrap();
    assert_eq!(game.players[0].runes[2], 0);
    assert_eq!(game.players[0].energy, 10);
    assert_eq!(game.players[0].energy_pool, 1); // The third Mind rune taps before recycling.
}

#[test]
fn zero_power_incinerate_still_pays_enemy_deflect_before_entering_the_chain() {
    let mut game = position();
    game.players[0].hand = vec![74];
    game.players[0].runes = [0; 6];
    game.players[0].energy_pool = 2;
    game.players[0].energy = 2;
    game.players[1].units.push(unit(101, 20, Some(0))); // Vex has Deflect.
    game.battlefield_owner[0] = Some(1);
    assert!(cast(&mut game, 74, vec![target(1, 101)]).is_err());
    assert_eq!(game.players[0].hand, vec![74]);
    assert_eq!(game.players[0].energy_pool, 2);
    game.players[0].any_power = 1;
    cast(&mut game, 74, vec![target(1, 101)]).unwrap();
    assert_eq!(game.players[0].energy_pool, 0);
    assert_eq!(game.players[0].any_power, 0);
    settle(&mut game);
    assert_eq!(game.players[1].units[0].damage, 2);
}

#[test]
fn new_spells_are_counterable_and_countered_draw_does_not_draw() {
    let mut game = position();
    game.players[0].hand = vec![77];
    game.players[1].hand = vec![37]; // Hard Bargain: counter unless its controller pays 2.
    let deck_size = game.players[0].deck.len();
    cast(&mut game, 77, vec![]).unwrap();
    let chain_id = game.chain[0].id;
    game.apply(Action::Pass).unwrap();
    cast(&mut game, 37, vec![Target::Spell { chain_id }]).unwrap();
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    assert!(matches!(
        game.decision,
        Some(local_rift::game::Decision::CounterTax { .. })
    ));
    game.apply(Action::Choose { accept: false }).unwrap();
    settle(&mut game);
    assert!(game.players[0].hand.is_empty());
    assert_eq!(game.players[0].deck.len(), deck_size);
    assert!(game.players[0].discard.contains(&77));
}

#[test]
fn action_spell_can_open_a_showdown_chain_but_ordinary_draw_cannot() {
    let mut game = position();
    game.players[0].units.push(unit(100, 70, None));
    game.players[1].units.push(unit(101, 70, Some(0)));
    game.battlefield_owner[0] = Some(1);
    game.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    assert!(game.showdown.is_some());
    assert!(game.chain.is_empty());
    let actor = game.priority();
    game.players[actor].hand = vec![77, 74];
    assert!(cast(&mut game, 77, vec![]).is_err());
    cast(&mut game, 74, vec![target(1, 101)]).unwrap();
    settle(&mut game);
    assert_eq!(game.players[1].units[0].damage, 2);
}

#[test]
fn draw_four_completes_after_one_burnout_without_drawing_the_unfinished_spell() {
    let mut game = position();
    game.players[0].hand = vec![77];
    game.players[0].deck = vec![69];
    game.players[0].discard = vec![70, 71, 72];
    cast(&mut game, 77, vec![]).unwrap();
    settle(&mut game);
    let mut drawn = game.players[0].hand.clone();
    drawn.sort_unstable();
    assert_eq!(drawn, vec![69, 70, 71, 72]);
    assert_eq!(game.players[1].score, 1);
    assert_eq!(game.players[0].discard, vec![77]);
}

#[test]
fn might_spells_allow_both_sides_at_base_and_expire_at_end_of_turn() {
    for (spell, expected) in [(76, 17), (79, 0)] {
        for owner in 0..2 {
            let mut game = position();
            game.players[0].hand = vec![spell];
            game.players[owner].units.push(unit(100, 70, None));
            cast(&mut game, spell, vec![target(owner, 100)]).unwrap();
            settle(&mut game);
            assert_eq!(
                game.unit_might(&game.players[owner].units[0]).unwrap(),
                expected
            );
            assert_eq!(
                game.players[owner].units.len(),
                1,
                "Zero might alone does not kill a unit"
            );
            game.apply(Action::EndTurn).unwrap();
            assert_eq!(game.unit_might(&game.players[owner].units[0]).unwrap(), 10);
        }
    }
}

#[test]
fn moonlight_affliction_is_a_reaction_and_reduction_obeys_damage_cleanup() {
    let mut game = position();
    game.players[0].hand = vec![75];
    game.players[1].hand = vec![79];
    game.players[1].units.push(unit(101, 70, Some(0)));
    game.battlefield_owner[0] = Some(1);
    game.players[0].units.push(unit(102, 69, None));
    game.players[0].units[0].damage = 1;
    cast(&mut game, 75, vec![target(1, 101)]).unwrap();
    game.apply(Action::Pass).unwrap();
    cast(&mut game, 79, vec![target(0, 102)]).unwrap();
    settle(&mut game);
    assert!(game.players[0].units.is_empty());
    assert!(game.players[0].discard.contains(&69));
    assert_eq!(game.players[1].units[0].damage, 6);
}

#[test]
fn rebuke_has_no_might_cap_and_returns_either_players_card_to_hand() {
    for owner in 0..2 {
        let mut game = position();
        game.players[0].hand = vec![80];
        game.players[owner].units.push(unit(101, 70, Some(0)));
        game.battlefield_owner[0] = Some(owner);
        cast(&mut game, 80, vec![target(owner, 101)]).unwrap();
        settle(&mut game);
        assert!(game.players[owner].units.is_empty());
        assert!(game.players[owner].hand.contains(&70));
        assert!(game.players[0].discard.contains(&80));
    }
}
