//! Third expansion's shared Repeat/Flow rules through public actions.
use local_rift::cards::Domain;
use local_rift::game::{
    Action, Decision, Difficulty, FlowSource, Game, Target, Unit, domain_index,
};

fn position() -> Game {
    let mut game = Game::new(3, 3, 471, Difficulty::Tactical);
    game.start_match().unwrap();
    game.battlefields = [0, 0];
    for player in &mut game.players {
        player.hand.clear();
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.discard.clear();
        player.banish.clear();
        player.runes = [0; 6];
        player.exhausted_runes = [0; 6];
        player.energy_pool = 12;
        player.energy = 12;
        player.power_pool = [3; 6];
        player.any_power = 0;
    }
    game
}
fn unit(id: u32) -> Unit {
    Unit {
        id,
        card_id: 72,
        lane: None,
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
fn cast(repeat: bool, targets: Vec<Target>, repeat_targets: Vec<Target>) -> Action {
    Action::PlayCardOptions {
        hand_index: 0,
        lane: None,
        targets,
        repeat_targets,
        repeat,
        accelerate: false,
        bonus: false,
    }
}
fn settle(game: &mut Game) {
    for _ in 0..40 {
        if game.chain.is_empty() || game.winner.is_some() {
            return;
        }
        assert!(
            game.decision.is_none(),
            "Unexpected choice {:?}",
            game.decision
        );
        game.apply(Action::Pass).unwrap();
    }
    panic!("chain did not finish");
}

#[test]
fn draw_repeat_pays_once_and_keeps_spell_out_of_its_own_burnout() {
    let mut game = position();
    game.players[0].hand = vec![133];
    game.players[0].deck = vec![72];
    game.players[0].discard = vec![73];
    game.apply(cast(true, vec![], vec![])).unwrap();
    assert_eq!(game.players[0].energy, 8);
    settle(&mut game);
    assert_eq!(game.players[0].hand, vec![72, 73]);
    assert_eq!(game.players[0].discard, vec![133]);
    assert_eq!(game.players[1].score, 1);
}

#[test]
fn draw_without_repeat_and_hidden_consult_draw_exactly_printed_count() {
    for (id, draws) in [(133, 1), (132, 2)] {
        let mut game = position();
        game.players[0].hand = vec![id];
        game.apply(cast(false, vec![], vec![])).unwrap();
        settle(&mut game);
        assert_eq!(game.players[0].hand.len(), draws);
        assert_eq!(game.players[0].discard, vec![id]);
    }
}

#[test]
fn repeated_buffs_can_choose_different_targets_and_expire() {
    let mut game = position();
    game.players[0].hand = vec![135];
    game.players[0].units = vec![unit(100), unit(101)];
    game.apply(cast(true, vec![target(0, 100)], vec![target(0, 101)]))
        .unwrap();
    settle(&mut game);
    assert_eq!(game.unit_might(&game.players[0].units[0]).unwrap(), 5);
    assert_eq!(game.unit_might(&game.players[0].units[1]).unwrap(), 5);
    game.apply(Action::EndTurn).unwrap();
    assert_eq!(game.unit_might(&game.players[0].units[0]).unwrap(), 3);
}

#[test]
fn repeated_reduction_has_no_one_might_floor() {
    let mut game = position();
    game.players[0].hand = vec![136];
    game.players[1].units = vec![unit(100)];
    game.apply(cast(true, vec![target(1, 100)], vec![target(1, 100)]))
        .unwrap();
    settle(&mut game);
    assert_eq!(game.unit_might(&game.players[1].units[0]).unwrap(), 0);
    assert_eq!(game.players[1].units[0].temporary_buff, -4);
}

#[test]
fn bonds_requires_two_distinct_friendly_targets_for_each_iteration() {
    let mut game = position();
    game.players[0].hand = vec![137];
    game.players[0].units = vec![unit(100), unit(101), unit(102)];
    game.players[1].units = vec![unit(103)];
    for targets in [
        vec![target(0, 100)],
        vec![target(0, 100), target(0, 100)],
        vec![target(0, 100), target(1, 103)],
    ] {
        let before = format!("{game:?}");
        assert!(game.apply(cast(false, targets, vec![])).is_err());
        assert_eq!(format!("{game:?}"), before);
    }
    game.apply(cast(
        true,
        vec![target(0, 100), target(0, 101)],
        vec![target(0, 101), target(0, 102)],
    ))
    .unwrap();
    settle(&mut game);
    assert_eq!(
        game.players[0]
            .units
            .iter()
            .map(|u| game.unit_might(u).unwrap())
            .collect::<Vec<_>>(),
        vec![4, 5, 4]
    );
}

#[test]
fn insufficient_repeat_payment_is_atomic() {
    let mut game = position();
    game.players[0].hand = vec![133];
    game.players[0].energy = 3;
    game.players[0].energy_pool = 3;
    let before = format!("{game:?}");
    assert!(game.apply(cast(true, vec![], vec![])).is_err());
    assert_eq!(format!("{game:?}"), before);
    game.apply(cast(false, vec![], vec![])).unwrap();
}

fn flow() -> Action {
    Action::PlayFromTrashOptions {
        trash_index: 0,
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate: false,
        bonus: false,
        flow: Some(FlowSource::Printed),
    }
}

#[test]
fn dredge_up_draws_from_trash_and_is_banished_by_flow() {
    let mut game = position();
    game.players[0].discard = vec![134];
    game.apply(flow()).unwrap();
    assert_eq!(game.players[0].energy, 10);
    settle(&mut game);
    assert_eq!(game.players[0].hand.len(), 1);
    assert_eq!(game.players[0].banish, vec![134]);
    assert!(game.players[0].discard.is_empty());
}

#[test]
fn iterative_design_flow_requires_mind_and_creates_a_pending_mech() {
    let mut game = position();
    game.players[0].discard = vec![138];
    game.players[0].power_pool = [0; 6];
    game.players[0].power_pool[domain_index(Domain::Fury)] = 1;
    assert!(game.flow_costs(0, 0)[0].colored);
    let before = format!("{game:?}");
    assert!(game.apply(flow()).is_err());
    assert_eq!(format!("{game:?}"), before);
    game.players[0].power_pool[domain_index(Domain::Mind)] = 1;
    game.apply(flow()).unwrap();
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    assert!(matches!(
        game.decision,
        Some(Decision::TokenPlacement { .. })
    ));
    assert_eq!(game.players[0].banish, vec![138]);
    assert!(game.players[0].units.is_empty());
    game.apply(Action::PlaceToken { lane: None }).unwrap();
    assert_eq!(game.unit_name(&game.players[0].units[0]), "Mech token");
    assert_eq!(game.unit_might(&game.players[0].units[0]).unwrap(), 3);
}
