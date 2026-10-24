//! Independent source-derived C4 examples. Initial positions are explicit test
//! fixtures; after setup, every tested transition uses public game actions.
use local_rift::activated::AbilitySource;
use local_rift::game::{Action, Decision, Difficulty, Game, Target, Unit};
use local_rift::gear::GearState;
use local_rift::play::{ExecutionChoices, PlayChoices, RepeatSelection};

fn position() -> Game {
    let mut game = Game::new_for_agents(3, 3, 44017, Difficulty::Tactical);
    game.start_match().unwrap();
    game.battlefields = [3, 3];
    game.battlefield_owner = [None, None];
    for player in &mut game.players {
        player.hand.clear();
        player.champion = None;
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.discard.clear();
        player.runes = [0; 6];
        player.exhausted_runes = [0; 6];
        player.energy_pool = 50;
        player.energy = 50;
        player.power_pool = [20; 6];
        player.any_power = 0;
    }
    game.legend[0].ready = false;
    game.legend[1].ready = false;
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
fn gear(game: &mut Game, id: u32, card_id: usize) -> AbilitySource {
    game.players[0].gear.push(GearState {
        id,
        card_id,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    AbilitySource::Gear {
        player: 0,
        gear_id: id,
    }
}
fn activate(game: &mut Game, source: AbilitySource) {
    game.apply(Action::ActivateAbility {
        source,
        ability_index: 0,
        targets: vec![],
    })
    .unwrap();
}
fn cast(game: &mut Game, player: usize, card: usize, targets: Vec<Target>) {
    assert_eq!(game.priority(), player);
    let hand_index = game.players[player].hand.len();
    game.players[player].hand.push(card);
    game.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
}
fn settle(game: &mut Game) {
    for _ in 0..100 {
        if let Some(Decision::OrderTriggers { options, .. }) = &game.decision {
            let trigger_id = options[0].id;
            game.apply(Action::OrderTrigger { trigger_id }).unwrap();
        } else if game.decision.is_some() {
            panic!(
                "Unexpected mandatory decision in this source example: {:?}",
                game.decision
            );
        } else if game.in_window() {
            game.apply(Action::Pass).unwrap();
        } else {
            return;
        }
    }
    panic!("Source example failed to finish its response windows");
}

#[test]
fn sun_disc_checks_its_legion_instruction_on_resolution_and_counts_countered_own_reactions() {
    // Current Core135.2/377/407/721/727/812/420.4: the condition is inside
    // the effect, and a finalized countered card still satisfies card history.
    for case in 0..4 {
        let mut game = position();
        let source = gear(&mut game, 900, 254); // A Disc from a previous turn.
        let friendly = unit(&mut game, 0, 901, 69, None);
        let enemy = unit(&mut game, 1, 902, 69, None);
        activate(&mut game, source);
        assert!(
            game.play_grants.is_empty(),
            "Activation costs alone cannot create the grant"
        );
        match case {
            1 => {
                game.apply(Action::Pass).unwrap();
                cast(&mut game, 1, 8, vec![enemy]); // Opponent's real En Garde.
            }
            2 | 3 => {
                cast(&mut game, 0, 8, vec![friendly]);
                if case == 3 {
                    let chain_id = game.chain.last().unwrap().id;
                    game.apply(Action::Pass).unwrap();
                    cast(&mut game, 1, 6, vec![Target::Spell { chain_id }]); // Actual Defy.
                }
            }
            _ => {}
        }
        settle(&mut game);
        cast(&mut game, 0, 157, vec![]); // Fretful Feline observes actual Ready.
        settle(&mut game);
        let feline = game.players[0]
            .units
            .iter()
            .find(|u| u.card_id == 157)
            .unwrap();
        assert_eq!(feline.ready, case >= 2, "Legion case {case}");
        assert_eq!(
            feline.temporary_buff, 0,
            "Entering ready is not a Ready action"
        );
        assert!(!game.players[0].gear[0].ready);
    }
}

#[test]
fn sun_disc_can_count_its_own_card_play_and_grants_only_the_next_unit() {
    // Origins FAQ's explicit Sun Disc example, retained alongside currentCore812.
    let mut game = position();
    cast(&mut game, 0, 254, vec![]);
    let gear_id = game.players[0].gear.last().unwrap().id;
    activate(&mut game, AbilitySource::Gear { player: 0, gear_id });
    settle(&mut game);
    for expected_ready in [true, false] {
        cast(&mut game, 0, 157, vec![]);
        settle(&mut game);
        let feline = game.players[0].units.last().unwrap();
        assert_eq!(feline.ready, expected_ready);
        assert_eq!(feline.temporary_buff, 0);
    }
}

#[test]
fn two_tomes_attach_to_the_same_next_spell_even_when_that_spell_deals_no_damage() {
    let mut game = position();
    let one = gear(&mut game, 900, 255);
    let two = gear(&mut game, 901, 255);
    let friendly = unit(&mut game, 0, 902, 69, None);
    let enemy = unit(&mut game, 1, 903, 69, Some(0));
    activate(&mut game, one);
    settle(&mut game);
    activate(&mut game, two);
    settle(&mut game);
    cast(&mut game, 0, 8, vec![friendly]);
    assert_eq!(game.chain.last().unwrap().bonus_damage, 2);
    assert!(game.play_grants.is_empty());
    settle(&mut game);
    cast(&mut game, 0, 260, vec![enemy]);
    assert_eq!(game.chain.last().unwrap().bonus_damage, 0);
    settle(&mut game);
    assert_eq!(game.players[1].units[0].damage, 3); //2 + target-local Void Gate.
}

#[test]
fn repeated_curse_counts_live_trash_but_never_the_resolving_physical_spell() {
    let mut game = position();
    let portal = gear(&mut game, 900, 256);
    unit(&mut game, 0, 901, 259, None); // Annie.
    let target = unit(&mut game, 1, 902, 69, Some(0));
    game.players[1].units[0].buff = 20; // Explicit high-Might fixture survives both packets.
    game.players[0].discard.push(260);
    activate(&mut game, portal);
    settle(&mut game);
    game.players[0].hand.push(260);
    let action = Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![target],
    };
    let options = game.play_choices(&action).unwrap();
    assert_eq!(options.repeats.len(), 1);
    game.apply(Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats: vec![RepeatSelection {
                option_id: options.repeats[0].id,
                choices: ExecutionChoices {
                    targets: vec![target],
                    mode: None,
                },
            }],
            ..Default::default()
        },
    })
    .unwrap();
    assert_eq!(game.chain.last().unwrap().executions.len(), 2);
    assert_eq!(
        game.players[0]
            .discard
            .iter()
            .filter(|&&id| id == 260)
            .count(),
        1
    );
    settle(&mut game);
    // Eachpacket:2 +1 own-trash Curse +1 live Annie +1 target-local Void Gate.
    assert_eq!(game.players[1].units[0].damage, 10);
    assert_eq!(
        game.players[0]
            .discard
            .iter()
            .filter(|&&id| id == 260)
            .count(),
        2
    );
}

#[test]
fn repeated_token_instructions_finish_the_physical_spell_before_any_token_enters() {
    let mut game = position();
    let portal = gear(&mut game, 900, 256);
    activate(&mut game, portal);
    settle(&mut game);
    game.players[0].hand.push(220); // Desert's Call has its own printed Repeat.
    let action = Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    };
    let view = game.play_choices(&action).unwrap();
    assert_eq!(view.repeats.len(), 2);
    game.apply(Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats: view
                .repeats
                .iter()
                .map(|option| RepeatSelection {
                    option_id: option.id,
                    choices: ExecutionChoices::default(),
                })
                .collect(),
            ..Default::default()
        },
    })
    .unwrap();
    assert_eq!(game.chain.len(), 1);
    assert_eq!(game.chain[0].executions.len(), 3);
    assert!(game.players[0].units.is_empty());
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    assert!(game.chain.is_empty());
    assert_eq!(
        game.players[0]
            .discard
            .iter()
            .filter(|&&id| id == 220)
            .count(),
        1
    );
    assert_eq!(game.trigger_state.completed_cards[0], 1);
    assert!(game.players[0].units.is_empty());
    for placed in 0..3 {
        assert_eq!(game.pending_tokens().len(), 3 - placed);
        assert!(matches!(
            game.decision,
            Some(Decision::TokenPlacement { .. })
        ));
        game.apply(Action::PlaceToken { lane: None }).unwrap();
        assert_eq!(game.players[0].units.len(), placed + 1);
        assert_eq!(game.trigger_state.completed_cards[0], 1);
    }
    assert!(game.pending_tokens().is_empty());
    assert!(
        game.players[0]
            .units
            .iter()
            .all(|u| u.token && game.unit_might(u).unwrap() == 2)
    );
}

#[test]
fn fight_damage_stays_unit_damage_with_annie_and_a_consumed_tome_grant() {
    let mut game = position();
    let tome = gear(&mut game, 900, 255);
    unit(&mut game, 0, 901, 259, None);
    let friendly = unit(&mut game, 0, 902, 69, None);
    game.players[0].units[1].buff = 2; //10M against8M.
    let enemy = unit(&mut game, 1, 903, 69, Some(0));
    activate(&mut game, tome);
    settle(&mut game);
    cast(&mut game, 0, 29, vec![friendly, enemy]); // Rampage, no optional Might.
    settle(&mut game);
    let survivor = game.players[0].units.iter().find(|u| u.id == 902).unwrap();
    assert_eq!(survivor.damage, 8);
    assert!(game.players[1].units.is_empty());
    assert!(game.play_grants.is_empty());
}
