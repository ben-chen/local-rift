//! CR315.2.b.2: holding every controlled battlefield is one outstanding task.
use local_rift::game::{Action, ChainAbility, Decision, Difficulty, Game, TriggeredAbility, Unit};
use local_rift::gear::GearState;

fn position(score: u8, card_id: usize) -> Game {
    let mut game = Game::new(3, 3, 801, Difficulty::Tactical);
    game.start_match().unwrap();
    for player in &mut game.players {
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.hand.clear();
    }
    game.players[1].units = (0..2)
        .map(|lane| Unit {
            id: 100 + lane as u32,
            card_id,
            lane: Some(lane),
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        })
        .collect();
    game.battlefield_owner = [Some(1), Some(1)];
    game.players[1].score = score;
    game
}

#[test]
fn seven_points_and_two_holds_finish_at_nine_before_victory() {
    // CR194.2 / 323.1 / 474: victory is checked in cleanup, not mid-task.
    let mut game = position(7, 69);
    game.apply(Action::EndTurn).unwrap();
    assert_eq!(game.winner, Some(1));
    assert_eq!(game.players[1].score, 9);
    assert_eq!(game.player_scored_this_turn[1], [true, true]);
}

#[test]
fn six_points_and_two_holds_finish_at_eight_before_victory() {
    let mut game = position(6, 69);
    game.apply(Action::EndTurn).unwrap();
    assert_eq!(game.winner, Some(1));
    assert_eq!(game.players[1].score, 8);
    assert_eq!(game.player_scored_this_turn[1], [true, true]);
}

#[test]
fn both_hold_triggers_wait_for_all_points_then_finish_before_channel_draw() {
    // Master Yi generates two XP triggers, one for each held field. The
    // controller orders simultaneous triggers only after the Hold task.
    let mut game = position(5, 22);
    let deck = game.players[1].deck.clone();
    let runes = game.players[1].total_runes();
    game.apply(Action::EndTurn).unwrap();
    assert_eq!(game.players[1].score, 7);
    assert_eq!(game.player_scored_this_turn[1], [true, true]);
    assert_eq!(game.players[1].xp, 0);
    assert_eq!(game.players[1].deck, deck);
    assert_eq!(game.players[1].total_runes(), runes);
    while let Some(Decision::OrderTriggers { options, .. }) = &game.decision {
        let trigger_id = options[0].id;
        game.apply(Action::OrderTrigger { trigger_id }).unwrap();
    }
    assert_eq!(game.chain.len(), 2);
    assert!(game.chain.iter().all(|item| matches!(
        item.ability,
        Some(ChainAbility::Triggered(TriggeredAbility::Xp(2)))
    )));
    for _ in 0..4 {
        game.apply(Action::Pass).unwrap();
    }
    assert_eq!(game.players[1].xp, 4);
    assert_eq!(game.players[1].total_runes(), runes + 3);
    assert_eq!(game.players[1].hand.len(), 1);
    assert_eq!(game.players[1].deck.len(), deck.len() - 1);
    assert_eq!(game.winner, None);
}

#[test]
fn winning_hold_does_not_finalize_hold_triggers_or_channel_or_draw() {
    let mut game = position(7, 22);
    game.players[1].gear.push(GearState {
        id: 102,
        card_id: 38, // Last Rites also observes its bearer's Hold.
        attached_to: Some(100),
        ready: true,
        temporary: false,
        token: false,
    });
    let deck = game.players[1].deck.clone();
    let runes = game.players[1].runes;
    let rune_deck = game.players[1].rune_deck.clone();
    game.apply(Action::EndTurn).unwrap();
    assert_eq!(game.winner, Some(1));
    assert_eq!(game.players[1].score, 9);
    assert_eq!(game.player_scored_this_turn[1], [true, true]);
    assert_eq!(game.players[1].xp, 0);
    assert_eq!(game.players[1].deck, deck);
    assert_eq!(game.players[1].runes, runes);
    assert_eq!(game.players[1].rune_deck, rune_deck);
    assert!(game.players[1].hand.is_empty());
    assert!(game.chain.is_empty());
    assert!(game.decision.is_none());
}
