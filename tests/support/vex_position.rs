//! A real pending Vex trigger, followed by the engine's control instruction.
use local_rift::game::{Action, Difficulty, Game, Target, Unit};

pub fn position(original_player: usize) -> (Game, Target) {
    let mut game = Game::new_for_agents(0, 0, 468408, Difficulty::Tactical);
    game.choose_starting_player(original_player).unwrap();
    game.start_match().unwrap();
    game.battlefields = vec![3, 3];
    game.battlefield_owner = vec![Some(original_player), Some(1 - original_player)];
    for p in &mut game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 20;
        p.energy = 20;
        p.power_pool = [5; 6];
    }
    for legend in &mut game.legend {
        legend.ready = false;
    }
    game.players[1 - original_player].units.push(Unit {
        id: 900_003,
        card_id: 20,
        lane: Some(1),
        ready: true,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    });
    game.players[original_player].hand = vec![71];
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: Some(0),
        targets: vec![],
    })
    .unwrap();
    let unit_id = game.players[original_player].units[0].id;
    assert!(game.chain.iter().any(|item| item.card_id == 20));
    let target = game
        .take_control_of_battlefield_unit(
            1 - original_player,
            Target::Unit {
                player: original_player,
                unit_id,
            },
        )
        .unwrap();
    for _ in 0..20 {
        if game.chain.is_empty() && game.decision.is_none() {
            return (game, target);
        }
        game.apply(Action::Pass).unwrap();
    }
    panic!("Vex fixture did not resolve");
}
