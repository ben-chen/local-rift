//! Activated gear completion is distinct from activation and Quick-Draw.
use local_rift::game::{Action, ChainAbility, Decision, Difficulty, Game, Target, Unit};
use local_rift::gear::GearState;

fn game() -> Game {
    let mut game = Game::new_for_agents(3, 3, 781, Difficulty::Tactical);
    game.start_match().unwrap();
    for player in &mut game.players {
        player.hand.clear();
        player.units.clear();
        player.gear.clear();
        player.runes = [2; 6];
        player.exhausted_runes = [0; 6];
        player.energy_pool = 0;
        player.energy = 12;
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

fn gear(game: &mut Game, id: u32, card_id: usize) {
    game.players[0].gear.push(GearState {
        id,
        card_id,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
}

fn cast(game: &mut Game, card_id: usize, target: Target) {
    let player = game.priority();
    game.players[player].hand.push(card_id);
    game.apply(Action::PlayCardWithTargets {
        hand_index: game.players[player].hand.len() - 1,
        lane: None,
        targets: vec![target],
    })
    .unwrap();
}

fn resolve_top(game: &mut Game) {
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
}

fn prize_buff(game: &Game) -> i16 {
    game.players[0]
        .units
        .iter()
        .find(|unit| unit.id == 100)
        .unwrap()
        .temporary_buff
}

#[test]
fn equip_completion_creates_a_separate_respondable_prize_trigger() {
    let mut game = game();
    let prize = unit(&mut game, 0, 100, 215, None);
    let enemy = unit(&mut game, 1, 200, 72, None);
    gear(&mut game, 101, 3);
    game.apply(Action::Equip {
        gear_id: 101,
        target: prize,
    })
    .unwrap();
    assert_eq!(
        game.chain.len(),
        1,
        "Prize cannot trigger before Equip resolves"
    );
    assert!(matches!(
        game.chain[0].ability,
        Some(ChainAbility::Equip { .. })
    ));
    assert_eq!(prize_buff(&game), 0);

    game.apply(Action::Pass).unwrap();
    assert_eq!(game.priority(), 1);
    cast(&mut game, 7, enemy); // Opponent can respond to the attachment itself.
    resolve_top(&mut game);
    assert_eq!(game.chain.len(), 1);
    assert_eq!(game.players[0].gear[0].attached_to, None);
    assert_eq!(prize_buff(&game), 0);

    resolve_top(&mut game);
    assert_eq!(game.players[0].gear[0].attached_to, Some(100));
    assert_eq!(game.chain.len(), 1);
    assert_eq!(game.chain[0].card_id, 215);
    assert_eq!(
        prize_buff(&game),
        0,
        "Prize's trigger also waits for responses"
    );
    game.apply(Action::Pass).unwrap();
    assert_eq!(game.priority(), 1);
    game.apply(Action::Pass).unwrap();
    assert_eq!(prize_buff(&game), 1);
}

#[test]
fn equip_with_an_invalidated_target_still_resolves_and_triggers_prize() {
    let mut game = game();
    unit(&mut game, 0, 100, 215, None);
    let bearer = unit(&mut game, 0, 102, 72, Some(0));
    gear(&mut game, 101, 3);
    game.apply(Action::Equip {
        gear_id: 101,
        target: bearer,
    })
    .unwrap();
    game.apply(Action::Pass).unwrap();
    cast(&mut game, 11, bearer); // Gust removes the chosen target before Equip.
    resolve_top(&mut game);
    assert!(game.players[0].units.iter().all(|unit| unit.id != 102));
    assert_eq!(game.chain.len(), 1);
    resolve_top(&mut game);
    assert_eq!(game.players[0].gear[0].attached_to, None);
    assert_eq!(game.chain.len(), 1);
    assert_eq!(game.chain[0].card_id, 215);
    resolve_top(&mut game);
    assert_eq!(prize_buff(&game), 1);
}

#[test]
fn prize_that_leaves_before_equip_resolves_does_not_observe_it() {
    let mut game = game();
    let prize = unit(&mut game, 0, 100, 215, Some(0));
    let bearer = unit(&mut game, 0, 102, 72, None);
    gear(&mut game, 101, 3);
    game.apply(Action::Equip {
        gear_id: 101,
        target: bearer,
    })
    .unwrap();
    game.apply(Action::Pass).unwrap();
    cast(&mut game, 11, prize);
    resolve_top(&mut game);
    resolve_top(&mut game);
    assert_eq!(game.players[0].gear[0].attached_to, Some(102));
    assert!(game.chain.is_empty());
    assert!(game.decision.is_none());
}

#[test]
fn quick_draw_attachment_is_triggered_not_an_activated_gear_ability() {
    let mut game = game();
    let prize = unit(&mut game, 0, 100, 215, None);
    game.players[0].hand.push(58);
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    })
    .unwrap();
    assert!(matches!(
        game.decision,
        Some(Decision::OnPlayTarget { card_id: 58, .. })
    ));
    game.apply(Action::ChooseTarget {
        target: Some(prize),
    })
    .unwrap();
    assert_eq!(game.chain.len(), 1);
    assert!(matches!(
        game.chain[0].ability,
        Some(ChainAbility::QuickDrawAttach { .. })
    ));
    resolve_top(&mut game);
    assert_eq!(game.players[0].gear[0].attached_to, Some(100));
    assert!(game.chain.is_empty());
    assert_eq!(prize_buff(&game), 0);
}

#[test]
fn resource_gear_completes_immediately_but_prize_still_waits_on_chain() {
    let mut game = game();
    unit(&mut game, 0, 100, 215, None);
    gear(&mut game, 101, 42);
    let priority = game.priority();
    game.apply(Action::ActivateGear { gear_id: 101 }).unwrap();
    assert_eq!(game.priority(), priority);
    assert!(!game.players[0].gear[0].ready);
    assert_eq!(game.players[0].power_pool[4], 1);
    assert_eq!(game.chain.len(), 1);
    assert_eq!(game.chain[0].card_id, 215);
    assert_eq!(prize_buff(&game), 0);
    resolve_top(&mut game);
    assert_eq!(prize_buff(&game), 1);
}
