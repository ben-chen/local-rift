//! CR829.1.c.3: each granted/printed Flow instance keeps its own cost.
use local_rift::game::{Action, Decision, Difficulty, FlowSource, Game, Unit};
use local_rift::payment::PowerPayment;

fn position(granted: bool) -> Game {
    let mut game = Game::new(2, 3, 51, Difficulty::Tactical);
    game.start_match().unwrap();
    game.players[0].hand.clear();
    game.players[0].discard = vec![39]; // Lightning Rush, printed Flow 2 + Any.
    game.players[0].units.push(Unit {
        id: 100,
        card_id: 33,
        lane: None,
        ready: true,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    });
    game.battlefields[0] = 0;
    if granted {
        game.apply(Action::Move {
            unit_id: 100,
            lane: Some(0),
        })
        .unwrap();
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert!(matches!(
            game.decision,
            Some(Decision::GrantFlow { player: 0 })
        ));
        game.apply(Action::SelectCards { indices: vec![0] })
            .unwrap();
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert_eq!(game.granted_flow_indices(0), &[0]);
    }
    let player = &mut game.players[0];
    player.energy = 6;
    player.energy_pool = 6;
    player.runes = [0; 6];
    player.exhausted_runes = [0; 6];
    player.power_pool = [1, 0, 0, 0, 0, 0]; // Fury is valid Any power for printed Flow.
    player.any_power = 0;
    game
}
fn action(flow: Option<FlowSource>) -> Action {
    Action::PlayFromTrashOptions {
        trash_index: 0,
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate: false,
        bonus: false,
        flow,
    }
}

#[test]
fn distinct_instances_offer_and_charge_the_selected_cost() {
    let game = position(true);
    let choices = game.flow_costs(0, 0);
    assert_eq!(choices.len(), 2);
    assert!(
        choices
            .iter()
            .any(|f| f.source == FlowSource::Granted && f.energy == 1 && f.power == 0)
    );
    assert!(
        choices.iter().any(|f| f.source == FlowSource::Printed
            && f.energy == 2
            && f.power == 1
            && !f.colored)
    );
    for (source, energy, power) in [(FlowSource::Granted, 5, 1), (FlowSource::Printed, 4, 0)] {
        let mut played = game.clone();
        played.apply(action(Some(source))).unwrap();
        assert_eq!(played.players[0].energy, energy);
        assert_eq!(played.players[0].power_pool[0], power);
        assert!(played.players[0].discard.is_empty());
        assert!(
            played
                .chain
                .iter()
                .any(|item| item.card_id == 39 && item.banish_after)
        );
    }
}

#[test]
fn explicit_unpayable_or_missing_instance_is_atomic_and_never_falls_back() {
    let mut game = position(true);
    game.players[0].power_pool = [0; 6];
    let before = format!("{game:?}");
    assert!(game.apply(action(Some(FlowSource::Printed))).is_err());
    assert_eq!(format!("{game:?}"), before);
    game.apply(action(Some(FlowSource::Granted))).unwrap();
    let mut game = position(false);
    let before = format!("{game:?}");
    assert!(game.apply(action(Some(FlowSource::Granted))).is_err());
    assert_eq!(format!("{game:?}"), before);
    assert!(game.flow_costs(99, 0).is_empty());
    assert!(game.flow_costs(0, 999).is_empty());
}

#[test]
fn quote_and_exact_payment_use_the_chosen_flow_instance() {
    let mut game = position(true);
    let before = format!("{game:?}");
    let printed = game
        .quote_power_payment(&action(Some(FlowSource::Printed)))
        .unwrap();
    assert_eq!(printed.suggested.colored[0], 1);
    assert_eq!(printed.costs.iter().map(|c| c.amount).sum::<u16>(), 1);
    let granted = game
        .quote_power_payment(&action(Some(FlowSource::Granted)))
        .unwrap();
    assert_eq!(granted.suggested, PowerPayment::default());
    assert_eq!(format!("{game:?}"), before);
    game.apply(Action::WithPowerPayment {
        action: Box::new(action(Some(FlowSource::Printed))),
        payment: printed.suggested,
    })
    .unwrap();
    assert_eq!(game.players[0].energy, 4);
    assert_eq!(game.players[0].power_pool[0], 0);
}

#[test]
fn default_uses_available_flow_and_granted_flow_expires() {
    let mut game = position(true);
    game.apply(action(None)).unwrap();
    assert_eq!(game.players[0].energy, 5);
    let mut game = position(true);
    game.apply(Action::EndTurn).unwrap();
    assert_eq!(game.flow_costs(0, 0).len(), 1);
    assert_eq!(game.flow_costs(0, 0)[0].source, FlowSource::Printed);
}

#[test]
fn flow_does_not_grant_reaction_timing() {
    let mut game = position(true);
    game.players[0].energy = 12;
    game.players[0].energy_pool = 12;
    game.players[0].hand = vec![79]; // Reaction -10 Might opens a closed chain.
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![local_rift::game::Target::Unit {
            player: 0,
            unit_id: 100,
        }],
    })
    .unwrap();
    // The caster retains priority, but Lightning Rush is a normal spell.
    let before = format!("{game:?}");
    assert!(game.apply(action(Some(FlowSource::Granted))).is_err());
    assert_eq!(format!("{game:?}"), before);
}
