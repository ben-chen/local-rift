//! Core 805/807/809/810/812/815/822/826 and the reviewed 88–102 cards.
//! These positions isolate interactions; they do not claim complete rules parity.
use local_rift::cards::Domain;
use local_rift::game::{
    Action, ChainAbility, Decision, Difficulty, Game, Showdown, Target, Unit, domain_index,
};
use local_rift::legends::{LegendAbility, LegendState};
use local_rift::rules;

fn position() -> Game {
    let mut game = Game::new(3, 3, 880102, Difficulty::Tactical);
    game.start_match().unwrap();
    game.battlefields = [0, 0];
    game.battlefield_owner = [None; 2];
    for player in &mut game.players {
        player.hand.clear();
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.discard.clear();
        player.runes = [6; 6];
        player.exhausted_runes = [0; 6];
        player.energy_pool = 0;
        player.energy = 36;
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

fn cast(game: &mut Game, id: usize, lane: Option<usize>, targets: Vec<Target>) {
    let player = game.priority();
    let index = game.players[player]
        .hand
        .iter()
        .position(|&card| card == id)
        .unwrap();
    game.apply(Action::PlayCardWithTargets {
        hand_index: index,
        lane,
        targets,
    })
    .unwrap();
}

fn resolve_one(game: &mut Game) {
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
}

fn settle(game: &mut Game) {
    for _ in 0..50 {
        assert!(
            game.decision.is_none(),
            "unexpected decision: {:?}",
            game.decision
        );
        if game.chain.is_empty() {
            return;
        }
        game.apply(Action::Pass).unwrap();
    }
    panic!("chain did not settle");
}

fn played_another(game: &mut Game) {
    game.players[0].hand.push(69);
    cast(game, 69, None, vec![]);
}

fn options(index: usize, accelerate: bool) -> Action {
    Action::PlayCardOptions {
        hand_index: index,
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate,
        bonus: false,
    }
}

#[test]
fn printed_assault_values_apply_only_to_actual_attackers_and_stack_with_grants() {
    for (card, assault) in [(56, 2), (88, 1), (89, 2), (90, 1), (97, 4), (98, 2)] {
        let mut game = position();
        game.players[0].units.push(unit(1000, card, Some(0)));
        game.legend[0].assault_grants.push((1000, 2));
        let printed = game.catalog[card].might;
        assert_eq!(
            game.unit_keywords(&game.players[0].units[0]).assault,
            assault + 2
        );
        assert_eq!(game.unit_might(&game.players[0].units[0]), printed);
        game.showdown = Some(Showdown {
            lane: 0,
            attacker: 0,
            focus: 0,
            passes: 0,
            combat: false,
        });
        assert_eq!(
            game.unit_might(&game.players[0].units[0]),
            printed,
            "neutral showdown isn't combat"
        );
        game.showdown.as_mut().unwrap().combat = true;
        assert_eq!(
            game.unit_might(&game.players[0].units[0]),
            printed + assault + 2
        );
        game.showdown.as_mut().unwrap().attacker = 1;
        assert_eq!(game.unit_might(&game.players[0].units[0]), printed);
        game.players[0].units[0].lane = None;
        game.showdown.as_mut().unwrap().attacker = 0;
        assert_eq!(game.unit_might(&game.players[0].units[0]), printed);
    }
}

#[test]
fn tank_and_backline_constrain_manual_and_automatic_damage_order() {
    let mut game = position();
    game.players[1].units = vec![
        unit(1000, 13, Some(0)),
        unit(1001, 91, Some(0)),
        unit(1002, 88, Some(0)),
    ];
    assert_eq!(
        game.automatic_damage_assignment(0, 0, 8),
        vec![(1001, 6), (1002, 2)]
    );
    game.decision = Some(Decision::CombatDamage {
        player: 0,
        lane: 0,
        total: 8,
    });
    let before = game.players.clone();
    assert!(
        game.apply(Action::AssignDamage {
            assignments: vec![(1002, 2), (1001, 6)]
        })
        .is_err()
    );
    assert_eq!(game.players, before);
    assert_eq!(game.damage_priority(&game.players[1].units[0]), 2);
    assert_eq!(game.damage_priority(&game.players[1].units[1]), 0);
}

#[test]
fn each_repeated_target_choice_pays_the_new_units_deflect() {
    for card in [92, 93, 98] {
        let mut game = position();
        game.players[1].units.push(unit(1000, card, None));
        game.players[0].hand = vec![53]; // Falling Star: two packets may choose the same unit.
        let runes: u8 = game.players[0].runes.iter().sum();
        cast(&mut game, 53, None, vec![target(1, 1000), target(1, 1000)]);
        assert_eq!(
            runes - game.players[0].runes.iter().sum::<u8>(),
            game.catalog[53].power_cost + 2
        );
        assert_eq!(game.chain.last().unwrap().targets.len(), 2);
    }
}

#[test]
fn token_abilities_do_not_inherit_the_creating_card_keywords_and_yi_still_levels() {
    let mut game = position();
    let mut bird = unit(1000, 34, None);
    bird.token = true;
    bird.buff = 1;
    let mut fake_recruit = unit(1001, 98, None);
    fake_recruit.token = true;
    fake_recruit.buff = 1;
    game.players[0].units = vec![bird, fake_recruit, unit(1002, 22, None)];
    assert_eq!(game.unit_keywords(&game.players[0].units[0]).deflect, 1);
    assert_eq!(
        game.unit_keywords(&game.players[0].units[1]),
        Default::default()
    );
    assert!(!game.unit_keywords(&game.players[0].units[2]).ganking);
    game.players[0].xp = 6;
    assert!(game.unit_keywords(&game.players[0].units[2]).ganking);
    assert_eq!(game.unit_keywords(&game.players[0].units[2]).deflect, 1);
}

#[test]
fn rengar_unseen_ganks_and_exhausts_but_a_normal_unit_cannot() {
    let mut game = position();
    game.players[0].units = vec![unit(1000, 98, Some(0)), unit(1001, 88, Some(0))];
    game.battlefield_owner[0] = Some(0);
    assert!(
        game.apply(Action::Move {
            unit_id: 1001,
            lane: Some(1)
        })
        .is_err()
    );
    assert!(game.legal_actions().contains(&Action::Move {
        unit_id: 1000,
        lane: Some(1)
    }));
    game.apply(Action::Move {
        unit_id: 1000,
        lane: Some(1),
    })
    .unwrap();
    assert_eq!(game.players[0].units[0].lane, Some(1));
    assert!(!game.players[0].units[0].ready);
    assert!(game.showdown.is_some());
}

#[test]
fn soulspinner_ambush_is_reaction_only_at_a_field_with_friendly_units() {
    let mut game = position();
    game.players[0].units.push(unit(1000, 69, Some(0)));
    game.players[0].hand = vec![7, 94];
    cast(&mut game, 7, None, vec![target(0, 1000)]);
    let before = game.players.clone();
    assert!(
        game.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![]
        })
        .is_err()
    );
    assert!(
        game.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: Some(1),
            targets: vec![]
        })
        .is_err()
    );
    assert_eq!(game.players, before);
    cast(&mut game, 94, Some(0), vec![]);
    assert!(
        game.players[0]
            .units
            .iter()
            .any(|u| u.card_id == 94 && u.lane == Some(0) && !u.ready)
    );
    assert_eq!(
        game.chain.len(),
        1,
        "a unit resolves immediately within the chain"
    );
}

#[test]
fn every_accelerate_mapping_pays_its_optional_cost_and_enters_ready() {
    for card in [49, 64, 95, 96, 97, 98] {
        let mut game = position();
        game.players[0].hand = vec![card];
        let before_energy = game.players[0].energy;
        let before_runes: u8 = game.players[0].runes.iter().sum();
        let cost = rules::accelerate_cost(card).unwrap();
        game.apply(options(0, true)).unwrap();
        assert!(game.players[0].units[0].ready);
        assert_eq!(
            before_energy - game.players[0].energy,
            game.catalog[card].cost + cost.energy
        );
        assert_eq!(
            before_runes - game.players[0].runes.iter().sum::<u8>(),
            game.catalog[card].power_cost + cost.power
        );
    }
    let mut game = position();
    game.players[0].hand = vec![96];
    game.players[0].runes = [0; 6];
    game.players[0].energy_pool = 3;
    game.players[0].energy = 3;
    let before = game.players.clone();
    assert!(game.apply(options(0, true)).is_err());
    assert_eq!(game.players, before, "unpayable Accelerate is atomic");
    game.apply(options(0, false)).unwrap();
    assert!(!game.players[0].units[0].ready);
}

#[test]
fn first_card_legion_units_do_not_gain_their_conditional_play_trigger() {
    for card in 99..=102 {
        let mut game = position();
        game.players[0].hand = vec![card];
        cast(&mut game, card, None, vec![]);
        assert!(game.chain.is_empty());
        assert!(game.decision.is_none());
        assert_eq!(game.players[0].units.len(), 1);
        assert_eq!(game.players[0].units[0].buff, 0);
        played_another(&mut game);
        assert!(
            game.chain.is_empty(),
            "Legion cannot create an old play event retroactively"
        );
    }
}

#[test]
fn dangerous_duo_finalizes_a_target_after_entry_and_can_choose_itself() {
    let mut game = position();
    played_another(&mut game);
    game.players[0].hand.push(99);
    cast(&mut game, 99, None, vec![]);
    let entered = game.players[0].units.last().unwrap().id;
    assert!(matches!(
        game.decision,
        Some(Decision::OnPlayTarget { card_id: 99, .. })
    ));
    game.apply(Action::ChooseTarget {
        target: Some(target(0, entered)),
    })
    .unwrap();
    assert_eq!(game.players[0].units.last().unwrap().temporary_buff, 0);
    settle(&mut game);
    assert_eq!(game.players[0].units.last().unwrap().temporary_buff, 2);
}

#[test]
fn countered_previous_card_still_enables_legion() {
    let mut game = position();
    game.players[0].units.push(unit(1000, 69, None));
    game.players[0].hand = vec![7, 99];
    game.players[1].hand = vec![6];
    cast(&mut game, 7, None, vec![target(0, 1000)]);
    let spell = game.chain[0].id;
    game.apply(Action::Pass).unwrap();
    cast(&mut game, 6, None, vec![Target::Spell { chain_id: spell }]);
    settle(&mut game);
    cast(&mut game, 99, None, vec![]);
    assert!(matches!(
        game.decision,
        Some(Decision::OnPlayTarget { card_id: 99, .. })
    ));
}

#[test]
fn scrapyard_champion_discards_two_then_draws_two_as_a_respondable_trigger() {
    let mut game = position();
    played_another(&mut game);
    game.players[0].hand = vec![100, 7, 8, 9];
    cast(&mut game, 100, None, vec![]);
    assert!(game.decision.is_none());
    resolve_one(&mut game);
    assert!(matches!(
        game.decision,
        Some(Decision::Discard {
            count: 2,
            draw_after: 2,
            ..
        })
    ));
    let deck = game.players[0].deck.len();
    game.apply(Action::SelectCards {
        indices: vec![0, 2],
    })
    .unwrap();
    assert!(game.players[0].discard.contains(&7) && game.players[0].discard.contains(&9));
    assert_eq!(game.players[0].deck.len(), deck - 2);
    assert_eq!(game.players[0].hand.len(), 3);
}

#[test]
fn gloryseeker_gains_one_persistent_buff_and_does_not_duplicate_an_existing_buff() {
    for already_buffed in [false, true] {
        let mut game = position();
        played_another(&mut game);
        game.players[0].hand.push(101);
        cast(&mut game, 101, None, vec![]);
        let id = game.players[0].units.last().unwrap().id;
        if already_buffed {
            game.buffed_units.push(id);
            game.players[0].units.last_mut().unwrap().buff = 1;
        }
        settle(&mut game);
        assert_eq!(game.players[0].units.last().unwrap().buff, 1);
        assert_eq!(game.buffed_units.iter().filter(|&&u| u == id).count(), 1);
        assert_eq!(game.players[0].units.last().unwrap().temporary_buff, 0);
    }
}

#[test]
fn vanguard_recruits_follow_current_here_and_cannot_use_a_departed_source() {
    for remove in [false, true] {
        let mut game = position();
        played_another(&mut game);
        game.players[0].units.push(unit(1000, 69, Some(0)));
        game.battlefield_owner[0] = Some(0);
        game.players[0].hand = vec![102, if remove { 11 } else { 9 }];
        cast(&mut game, 102, Some(0), vec![]);
        let captain = game.players[0].units.last().unwrap().id;
        cast(
            &mut game,
            if remove { 11 } else { 9 },
            None,
            vec![target(0, captain)],
        );
        settle(&mut game);
        let recruits: Vec<_> = game.players[0]
            .units
            .iter()
            .filter(|u| u.token && u.card_id == 102)
            .collect();
        assert_eq!(recruits.len(), if remove { 0 } else { 2 });
        assert!(
            recruits
                .iter()
                .all(|u| u.lane.is_none() && !u.ready && u.buff == 1)
        );
    }
}

#[test]
fn matriarch_selects_first_then_optional_accelerate_is_paid_at_actual_play() {
    for accelerate in [false, true] {
        let mut game = position();
        game.players[0].discard = vec![96];
        game.players[0].runes = [0; 6];
        game.players[0].energy = 0;
        game.decision = Some(Decision::ReviveUnit { player: 0 });
        game.apply(Action::PlayFromTrash {
            trash_index: 0,
            lane: None,
            target: None,
        })
        .unwrap();
        assert!(game.players[0].units.is_empty());
        resolve_one(&mut game);
        assert!(matches!(
            game.decision,
            Some(Decision::PendingPlay { card_id: 96, .. })
        ));
        assert!(game.apply(Action::Choose { accept: false }).is_err());
        if accelerate {
            game.players[0].runes[domain_index(Domain::Fury)] = 1;
            game.players[0].energy = 1;
        }
        game.apply(Action::FinalizePendingPlay {
            lane: None,
            targets: vec![],
            repeat_targets: vec![],
            repeat: false,
            accelerate,
            bonus: false,
        })
        .unwrap();
        assert_eq!(game.players[0].units[0].ready, accelerate);
        assert_eq!(
            game.players[0].energy, 0,
            "only Accelerate's extra energy is paid"
        );
        assert_eq!(
            game.players[0].runes.iter().sum::<u8>(),
            0,
            "only Accelerate's extra power is paid"
        );
        assert!(game.players[0].discard.is_empty());
    }
}

#[test]
fn ezreal_optional_discount_can_be_allocated_during_revived_unit_finalization() {
    let mut game = position();
    game.players[0].units.push(unit(1000, 35, None));
    game.players[0].discard = vec![96];
    game.players[0].runes = [0; 6];
    game.players[0].energy_pool = 1;
    game.players[0].energy = 1;
    game.decision = Some(Decision::ReviveUnit { player: 0 });
    game.apply(Action::PlayFromTrash {
        trash_index: 0,
        lane: None,
        target: None,
    })
    .unwrap();
    resolve_one(&mut game);
    let before = game.decision.clone();
    game.apply(Action::SetOptionalDiscount { power: 1 })
        .unwrap();
    assert_eq!(game.decision, before);
    game.apply(Action::FinalizePendingPlay {
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate: true,
        bonus: false,
    })
    .unwrap();
    assert!(game.players[0].units.last().unwrap().ready);
    assert_eq!(game.players[0].energy, 0);
}

#[test]
fn pending_token_entries_follow_parent_completion_and_preserve_pending_item_age() {
    for (card, count) in [(48, 2), (87, 4)] {
        let mut game = position();
        // Both spells fit Kennen's Chaos/Order identity. A prior conquer can
        // grant either spell Flow, so the final played event empowers Kennen.
        game.legend[0] = LegendState::new(33);
        game.players[1].units.push(unit(1000, 20, Some(0)));
        game.battlefield_owner[0] = Some(1);
        game.players[0].discard = vec![card];
        game.decision = Some(Decision::GrantFlow { player: 0 });
        game.apply(Action::SelectCards { indices: vec![0] })
            .unwrap();
        settle(&mut game);
        game.apply(Action::PlayFromTrash {
            trash_index: 0,
            lane: None,
            target: None,
        })
        .unwrap();
        resolve_one(&mut game);
        assert!(
            game.players[0].banish.contains(&card),
            "Flow spell completes before token finalization"
        );
        assert_eq!(game.pending_tokens().len(), count);
        assert!(game.players[0].units.iter().all(|unit| !unit.token));
        for n in 0..count {
            game.apply(Action::PlaceToken { lane: None }).unwrap();
            assert_eq!(
                game.players[0]
                    .units
                    .iter()
                    .filter(|unit| unit.token)
                    .count(),
                n + 1
            );
            assert_eq!(game.pending_tokens().len(), count - n - 1);
        }
        assert!(
            game.decision.is_none(),
            "one Vex trigger per distinct token play is not a simultaneous ordering choice"
        );
        assert_eq!(game.chain.len(), count + 1);
        assert!(matches!(
            game.chain.first().unwrap().ability,
            Some(ChainAbility::Legend(LegendAbility::KennenEmpower))
        ));
        let token_ids: Vec<_> = game.players[0]
            .units
            .iter()
            .filter(|u| u.token)
            .map(|u| u.id)
            .collect();
        for (item, expected) in game.chain.iter().skip(1).zip(token_ids) {
            assert!(
                matches!(item.ability, Some(ChainAbility::Triggered(local_rift::game::TriggeredAbility::Stun { unit_id, .. })) if unit_id == expected)
            );
        }
    }
}

#[test]
fn two_vex_observers_of_one_token_remain_a_simultaneous_ordering_choice() {
    let mut game = position();
    game.players[1].units = vec![unit(1000, 20, Some(0)), unit(1001, 20, Some(0))];
    game.battlefield_owner[0] = Some(1);
    game.players[0].hand = vec![48];
    cast(&mut game, 48, None, vec![]);
    resolve_one(&mut game);
    game.apply(Action::PlaceToken { lane: None }).unwrap();
    game.apply(Action::PlaceToken { lane: None }).unwrap();
    for expected_chain_len in [0, 2] {
        let Some(Decision::OrderTriggers { options, player }) = game.decision.clone() else {
            panic!("same-token observers should be ordered together");
        };
        assert_eq!(player, 1);
        assert_eq!(
            options.len(),
            2,
            "the next token's observers form their own later event"
        );
        assert_eq!(game.chain.len(), expected_chain_len);
        game.apply(Action::OrderTrigger {
            trigger_id: options[1].id,
        })
        .unwrap();
    }
    assert!(game.decision.is_none());
    assert_eq!(game.chain.len(), 4);
}
