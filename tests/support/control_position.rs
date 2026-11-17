//! Preserve the actual deck multiset while authoring a control-transfer position.
use local_rift::cards::CardKind;
use local_rift::deck::CustomDeck;
use local_rift::game::{Difficulty, Game, Target, Unit};
use local_rift::gear::GearState;

pub fn position(preset: usize, owner: usize) -> (Game, [CustomDeck; 2], Target, Option<u32>) {
    let specs = [
        CustomDeck::from_preset(preset).unwrap(),
        CustomDeck::from_preset(preset).unwrap(),
    ];
    let mut game = Game::new_for_agents(preset, preset, 468407, Difficulty::Tactical);
    game.choose_starting_player(owner).unwrap();
    game.start_match().unwrap();
    let index = game.players[owner]
        .deck
        .iter()
        .position(|&id| game.catalog[id].kind == CardKind::Unit)
        .unwrap();
    let card_id = game.players[owner].deck.remove(index);
    let unit_id = 900_001;
    game.players[owner].units.push(Unit {
        id: unit_id,
        card_id,
        lane: Some(0),
        ready: true,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    });
    game.battlefield_owner[0] = Some(owner);
    let equipment = game.players[owner]
        .deck
        .iter()
        .position(|&id| local_rift::rules::equipment(id).is_some())
        .map(|index| {
            let card_id = game.players[owner].deck.remove(index);
            let gear_id = 900_002;
            game.players[owner].gear.push(GearState {
                id: gear_id,
                card_id,
                lane: None,
                ready: true,
                attached_to: Some(unit_id),
                temporary: false,
                token: false,
            });
            gear_id
        });
    (
        game,
        specs,
        Target::Unit {
            player: owner,
            unit_id,
        },
        equipment,
    )
}
