//! Reachable multi-battlefield staging using only the original bundled cards.
use local_rift::game::{Action, Decision, Difficulty, Game, Target, Unit};

fn defender(id: u32, lane: usize) -> Unit {
    Unit {
        id,
        card_id: 18,
        lane: Some(lane),
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

fn ambush_chain(rengars: usize) -> Game {
    let mut game = Game::new(0, 1, 729, Difficulty::Tactical);
    game.start_match().unwrap();
    for player in &mut game.players {
        player.hand.clear();
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.exhausted_runes = [0; 6];
        player.energy_pool = 0;
        player.energy = 12;
    }
    game.players[0].runes = [0, 6, 0, 0, 6, 0];
    game.players[1].runes = [0, 5, 0, 7, 0, 0];
    game.players[0].hand = vec![7, 9]; // Discipline and Flash.
    game.players[1].hand = vec![30; rengars];
    game.players[0].units = vec![defender(100, 0), defender(101, 1)];
    game.battlefield_owner = [Some(0), Some(0)];
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player: 0,
            unit_id: 100,
        }],
    })
    .unwrap();
    // Decline Irelia's optional ready trigger; Discipline remains on the chain.
    assert!(matches!(game.decision, Some(Decision::Legend { .. })));
    game.apply(Action::Choose { accept: false }).unwrap();
    for lane in 0..rengars {
        assert_eq!(game.priority(), 0);
        game.apply(Action::Pass).unwrap();
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: Some(lane),
            target: None,
        })
        .unwrap();
        assert!(
            game.showdown.is_none(),
            "the existing chain postpones combat"
        );
    }
    game
}

fn resolve_chain(game: &mut Game) {
    for _ in 0..30 {
        if game.chain.is_empty() {
            return;
        }
        assert!(
            game.decision.is_none(),
            "unexpected decision: {:?}",
            game.decision
        );
        game.apply(Action::Pass).unwrap();
    }
    panic!("chain stalled");
}

#[test]
fn ambush_contender_is_attacker_and_gets_initial_focus() {
    // CR190.3.a.1 / 345 / 466.2.c.1: the Rengar player applied Contested.
    let mut game = ambush_chain(1);
    resolve_chain(&mut game);
    let window = game.showdown.as_ref().unwrap();
    assert_eq!(window.attacker, 1);
    assert_eq!(window.focus, 1);
    assert_eq!(game.priority(), 1);
}

#[test]
fn turn_player_chooses_between_two_staged_combats() {
    // CR323.12. Neither deployment resolves combat while Discipline is pending.
    let mut game = ambush_chain(2);
    resolve_chain(&mut game);
    assert!(
        game.showdown.is_none(),
        "neither field opens before the choice"
    );
    assert_eq!(
        game.decision,
        Some(Decision::BattlefieldOrder {
            player: 0,
            lanes: vec![0, 1],
            combat: true,
        })
    );
    assert_eq!(game.priority(), 0);
    assert_eq!(
        game.legal_actions().unwrap(),
        vec![
            Action::ChooseBattlefield { lane: 0 },
            Action::ChooseBattlefield { lane: 1 }
        ]
    );
    let before_players = game.players.clone();
    let before_decision = game.decision.clone();
    assert!(game.apply(Action::ChooseBattlefield { lane: 99 }).is_err());
    assert_eq!(game.players, before_players);
    assert_eq!(game.decision, before_decision);
    game.apply(Action::ChooseBattlefield { lane: 1 }).unwrap();
    let window = game.showdown.as_ref().unwrap();
    assert_eq!((window.lane, window.attacker, window.focus), (1, 1, 1));

    // The other staged combat remains pending and retains its own contender.
    for _ in 0..100 {
        if game
            .showdown
            .as_ref()
            .is_some_and(|window| window.lane == 0)
        {
            break;
        }
        if game.decision.is_some() {
            game.step_ai().unwrap();
        } else {
            game.apply(Action::Pass).unwrap();
        }
    }
    let window = game.showdown.as_ref().unwrap();
    assert_eq!((window.lane, window.attacker, window.focus), (0, 1, 1));
}

#[test]
fn noncombat_showdown_opens_before_a_staged_combat() {
    // CR323.11 comes before 323.12, independent of battlefield index.
    let mut game = ambush_chain(2);
    game.players[0].hidden.push(local_rift::hidden::HiddenCard {
        id: 999,
        card_id: 44,
        lane: 1,
        hidden_turn: game.turn - 1,
        revealed: false,
    });
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player: 0,
            unit_id: 101,
        }],
    })
    .unwrap();
    assert!(matches!(game.decision, Some(Decision::Legend { .. })));
    game.apply(Action::Choose { accept: false }).unwrap();
    resolve_chain(&mut game);
    let window = game.showdown.as_ref().unwrap();
    assert_eq!(window.lane, 1, "field1 has the sole non-combat showdown");
    assert!(!window.combat);
    assert_eq!(window.attacker, 1);
    assert_eq!(window.focus, 1);
    assert_eq!(
        game.battlefield_owner[1], None,
        "empty former controller loses control before the new showdown opens"
    );
    assert!(
        game.players[0].hidden.is_empty(),
        "loss of control clears the hidden slot at the same cleanup"
    );
}

#[test]
fn a_staged_combat_waits_for_the_showdown_to_be_open_and_preserves_contender() {
    // CR323.13 / 466.2: arriving during a chain stages combat; it does not
    // grant combat designations until the ongoing showdown is open again.
    let mut game = ambush_chain(0);
    resolve_chain(&mut game);
    game.players[0].units[0].lane = None;
    game.battlefield_owner[0] = None;
    game.players[0].hand = vec![7];
    game.players[1].hand = vec![30];
    game.apply(Action::Move {
        unit_id: 100,
        lane: Some(0),
    })
    .unwrap();
    assert!(!game.showdown.as_ref().unwrap().combat);
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player: 0,
            unit_id: 100,
        }],
    })
    .unwrap();
    game.apply(Action::Choose { accept: false }).unwrap();
    game.apply(Action::Pass).unwrap();
    game.apply(Action::PlayCard {
        hand_index: 0,
        lane: Some(0),
        target: None,
    })
    .unwrap();
    assert!(
        !game.showdown.as_ref().unwrap().combat,
        "a closed chain cannot open staged combat"
    );
    resolve_chain(&mut game);
    let window = game.showdown.as_ref().unwrap();
    assert!(window.combat);
    assert_eq!(
        window.attacker, 0,
        "original Contested applier stays attacker"
    );
    assert_eq!(
        window.focus, 1,
        "chain initiator's opponent gains focus normally"
    );
}

#[test]
fn withdrawing_the_contender_removes_that_staged_combat() {
    let mut game = ambush_chain(2);
    let withdrawn = game.players[1]
        .units
        .iter()
        .find(|unit| unit.lane == Some(0))
        .unwrap()
        .id;
    game.players[1].hand = vec![9];
    game.players[1].energy_pool = 2;
    game.players[1].energy += 2;
    game.apply(Action::Pass).unwrap();
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player: 1,
            unit_id: withdrawn,
        }],
    })
    .unwrap();
    resolve_chain(&mut game);
    assert!(game.decision.is_none());
    let window = game.showdown.as_ref().unwrap();
    assert_eq!((window.lane, window.attacker), (1, 1));
    assert_eq!(game.battlefield_owner[0], Some(0));
}

#[test]
fn turn_player_also_chooses_between_multiple_noncombat_showdowns() {
    let mut game = ambush_chain(2);
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
    for _ in 0..8 {
        match game.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => {
                game.apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap();
            }
            Some(Decision::Legend { .. }) => {
                game.apply(Action::Choose { accept: false }).unwrap();
            }
            None => break,
            other => panic!("unexpected choice: {other:?}"),
        }
    }
    resolve_chain(&mut game);
    assert_eq!(
        game.decision,
        Some(Decision::BattlefieldOrder {
            player: 0,
            lanes: vec![0, 1],
            combat: false
        })
    );
    game.apply(Action::ChooseBattlefield { lane: 1 }).unwrap();
    assert_eq!(game.showdown.as_ref().unwrap().lane, 1);
    assert!(!game.showdown.as_ref().unwrap().combat);
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    resolve_chain(&mut game);
    assert_eq!(game.showdown.as_ref().unwrap().lane, 0);
    assert_eq!(game.showdown.as_ref().unwrap().attacker, 1);
}
