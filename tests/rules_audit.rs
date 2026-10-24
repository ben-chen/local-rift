//! Independent public-API examples from the October 9, 2026 Core Rules.
//! Fixtures construct small positions; they are not deck-conservation tests.
use local_rift::game::{
    Action, ChainAbility, Decision, Difficulty, Game, Target, TriggeredAbility, Unit,
};
use local_rift::hidden::HiddenCard;

fn position() -> Game {
    let mut game = Game::new(3, 3, 71, Difficulty::Tactical);
    game.start_match().unwrap();
    for player in &mut game.players {
        player.hand.clear();
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.discard.clear();
        player.runes = [4, 4, 4, 4, 4, 4];
        player.exhausted_runes = [0; 6];
        player.energy_pool = 0;
        player.energy = 24;
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

fn finish_chain(game: &mut Game) {
    for _ in 0..40 {
        if game.chain.is_empty() || game.winner.is_some() {
            return;
        }
        assert!(
            game.decision.is_none(),
            "unexpected choice: {:?}",
            game.decision
        );
        game.apply(Action::Pass).unwrap();
    }
    panic!("chain did not finish");
}

#[test]
fn abandon_still_predicts_after_its_counter_target_leaves_the_chain() {
    // CR359.3.e.1/e.5: an impossible counter instruction does not negate the
    // independent Predict instruction that follows it.
    let mut game = position();
    game.players[0].units.push(unit(100, 59, None));
    game.players[0].hand = vec![7, 6]; // Discipline, then Defy our own spell.
    game.players[1].hand = vec![1]; // Abandon.
    game.players[1].deck = vec![69, 70];
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player: 0,
            unit_id: 100,
        }],
    })
    .unwrap();
    let original = game.chain.last().unwrap().id;
    game.apply(Action::Pass).unwrap();
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Spell { chain_id: original }],
    })
    .unwrap();
    game.apply(Action::Pass).unwrap();
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Spell { chain_id: original }],
    })
    .unwrap();
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    assert!(!game.chain.iter().any(|item| item.id == original));
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    assert_eq!(
        game.decision,
        Some(Decision::Predict {
            player: 1,
            card_id: 70
        })
    );
    game.apply(Action::Choose { accept: true }).unwrap();
    assert_eq!(game.players[1].deck, vec![70, 69]);
}

#[test]
fn switcheroo_uses_actual_negative_might_when_computing_its_modifiers() {
    // CR143.2.c.1 and434.1.b: calculating a Might change uses the actual
    // negative value, even though references and combat treat it as zero.
    let mut game = position();
    game.players[0].units = vec![unit(100, 72, Some(0)), unit(101, 70, Some(0))];
    game.battlefield_owner[0] = Some(0);
    game.players[0].hand = vec![79, 44, 76];
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player: 0,
            unit_id: 100,
        }],
    })
    .unwrap();
    finish_chain(&mut game);
    assert_eq!(game.unit_might(&game.players[0].units[0]).unwrap(), 0); // Actual -7.
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![
            Target::Unit {
                player: 0,
                unit_id: 100,
            },
            Target::Unit {
                player: 0,
                unit_id: 101,
            },
        ],
    })
    .unwrap();
    finish_chain(&mut game);
    assert_eq!(game.unit_might(&game.players[0].units[0]).unwrap(), 10);
    assert_eq!(game.unit_might(&game.players[0].units[1]).unwrap(), 0); // Actual -7 now.
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player: 0,
            unit_id: 101,
        }],
    })
    .unwrap();
    finish_chain(&mut game);
    assert_eq!(
        game.unit_might(&game.players[0].units[1]).unwrap(),
        0,
        "+7 must only erase the -7 deficit"
    );
}

#[test]
fn first_burnout_finishes_the_draw_before_cleanup_awards_victory() {
    // CR432.2.d completes the original draw. Immediate victory is restricted
    // to subsequent repeated burnouts (432.3.c.1); otherwise use CR323.1.
    let mut game = position();
    game.players[0].hand = vec![7]; // Discipline: +2 might, then draw 1.
    game.players[0].deck.clear();
    game.players[0].discard = vec![59];
    game.players[0].units.push(unit(100, 59, None));
    game.players[1].score = 7;
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player: 0,
            unit_id: 100,
        }],
    })
    .unwrap();
    finish_chain(&mut game);
    assert_eq!(
        game.players[0].hand,
        vec![59],
        "the first burnout must finish drawing"
    );
    assert_eq!(game.winner, Some(1));
}

fn empowered_by_tornado() -> Game {
    let mut game = position();
    game.players[0].units.push(unit(100, 24, Some(0)));
    game.battlefield_owner[0] = Some(0);
    game.players[0].hidden.push(HiddenCard {
        id: 101,
        card_id: 46,
        lane: 0,
        hidden_turn: 0,
        revealed: false,
    });
    game.apply(Action::PlayHidden {
        hidden_id: 101,
        targets: vec![],
    })
    .unwrap();
    assert!(matches!(game.decision, Some(Decision::EmpowerHere { .. })));
    game.apply(Action::ChooseTarget {
        target: Some(Target::Unit {
            player: 0,
            unit_id: 100,
        }),
    })
    .unwrap();
    finish_chain(&mut game);
    assert!(game.unit_empowered(100));
    game
}

#[test]
fn tornado_disempower_is_a_respondable_ending_trigger() {
    // CR317.1 precedes the Expiration Step, and 391.2 makes the scheduled
    // Disempower a delayed triggered ability rather than a passive duration.
    let mut game = empowered_by_tornado();
    let turn = game.turn;
    game.apply(Action::EndTurn).unwrap();
    assert_eq!(
        game.turn, turn,
        "the ending trigger resolves before the next turn"
    );
    assert!(game.unit_empowered(100), "Disempower waits for responses");
    assert!(
        !game.chain.is_empty(),
        "the delayed trigger must enter the chain"
    );
    finish_chain(&mut game);
    assert!(!game.unit_empowered(100));
    assert_eq!(game.turn, turn + 1);
}

#[test]
fn pyke_dying_simultaneously_cannot_see_an_enemy_death() {
    // CR383.2.c.2: a source that leaves its active zone at the same time as
    // its trigger condition is fulfilled cannot trigger on that event.
    let mut game = position();
    game.players[0].hand = vec![53];
    game.players[0].units.push(unit(100, 13, Some(0)));
    game.players[1].units.push(unit(101, 24, None));
    game.battlefield_owner[0] = Some(0);
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![
            Target::Unit {
                player: 0,
                unit_id: 100,
            },
            Target::Unit {
                player: 1,
                unit_id: 101,
            },
        ],
    })
    .unwrap();
    finish_chain(&mut game);
    assert!(game.players[0].units.is_empty());
    assert!(game.players[1].units.is_empty());
    assert!(
        game.players[0].gear.is_empty(),
        "Pyke left simultaneously, so no Gold trigger exists"
    );
}

#[test]
fn kennen_chooses_the_trash_spell_before_its_conquer_trigger_is_finalized() {
    // The trash is public: "give a spell in your trash" targets that object
    // under CR355.5, 355.9.a, 355.10.a.1. Choose before opponents respond.
    let mut game = position();
    game.battlefields[0] = 0; // Sunken Temple: Kennen is below its 5-might condition.
    game.players[0].units.push(unit(100, 33, None));
    game.players[0].discard = vec![7];
    game.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    assert!(
        matches!(game.decision, Some(Decision::GrantFlow { player: 0 })),
        "the spell must be chosen at finalization, not when the trigger resolves"
    );
    game.apply(Action::SelectCards { indices: vec![0] })
        .unwrap();
    assert!(
        !game.chain.is_empty(),
        "granting Flow waits for responses after choosing"
    );
    assert!(game.granted_flow_indices(0).is_empty());
    finish_chain(&mut game);
    assert_eq!(game.granted_flow_indices(0), &[0]);
}

#[test]
fn newly_triggered_irelia_does_not_jump_ahead_of_an_older_pending_vex_trigger() {
    // First Mate's play effect and opposing Vex trigger together. The turn
    // player finalizes First Mate first, then chooses Irelia. That NEW trigger
    // must wait behind Vex's already-pending trigger (CR337.1 / 337.1.b).
    let mut game = position();
    game.players[0].hand = vec![23];
    game.players[0].units.push(unit(100, 0, None));
    game.players[1].units.push(unit(101, 20, Some(0)));
    game.battlefield_owner[0] = Some(1);
    game.apply(Action::PlayCard {
        hand_index: 0,
        lane: None,
        target: None,
    })
    .unwrap();
    assert!(matches!(game.decision, Some(Decision::OnPlayTarget { .. })));
    game.apply(Action::ChooseTarget {
        target: Some(Target::Unit {
            player: 0,
            unit_id: 100,
        }),
    })
    .unwrap();
    assert_eq!(game.chain.len(), 3);
    assert!(matches!(
        game.chain[0].ability,
        Some(ChainAbility::CardEffect { .. })
    ));
    assert!(
        matches!(
            game.chain[1].ability,
            Some(ChainAbility::Triggered(TriggeredAbility::Stun { .. }))
        ),
        "Vex was already pending when choosing Irelia generated a new trigger"
    );
    assert!(matches!(
        game.chain[2].ability,
        Some(ChainAbility::Triggered(TriggeredAbility::Might { .. }))
    ));
}

#[test]
fn tornado_played_during_the_ending_step_does_not_schedule_next_turn_expiry() {
    // CR359.3.e.16: generating an end-of-this-turn trigger after that window
    // has passed does not make it trigger later or on the following turn.
    let mut game = empowered_by_tornado();
    game.players[0].units.push(unit(200, 24, Some(0)));
    game.players[0].hidden.push(HiddenCard {
        id: 201,
        card_id: 46,
        lane: 0,
        hidden_turn: 0,
        revealed: false,
    });
    game.apply(Action::EndTurn).unwrap();
    game.apply(Action::PlayHidden {
        hidden_id: 201,
        targets: vec![],
    })
    .unwrap();
    game.apply(Action::ChooseTarget {
        target: Some(Target::Unit {
            player: 0,
            unit_id: 200,
        }),
    })
    .unwrap();
    finish_chain(&mut game);
    assert!(game.unit_empowered(200));
    game.apply(Action::EndTurn).unwrap();
    assert!(
        game.unit_empowered(200),
        "the missed ending window does not roll forward"
    );
}

#[test]
fn a_flow_target_recycled_by_burnout_is_not_replaced_by_an_identical_copy() {
    let mut game = position();
    game.battlefields[0] = 0;
    game.players[0].units.push(unit(100, 33, None));
    game.players[0].discard = vec![7, 7];
    game.players[0].deck.clear();
    game.players[0].hand = vec![7];
    game.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    game.apply(Action::SelectCards { indices: vec![0] })
        .unwrap();
    // Discipline draws through burnout and itself later enters the trash at
    // the same index. It is not the object Kennen chose (CR359.3.e.4).
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player: 0,
            unit_id: 100,
        }],
    })
    .unwrap();
    finish_chain(&mut game);
    assert_eq!(game.players[0].discard, vec![7]);
    assert!(game.granted_flow_indices(0).is_empty());
}
