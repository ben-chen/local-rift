//! Independent consistency checks for the public cost-preview surface.
use local_rift::game::{Action, Decision, Difficulty, FlowSource, Game};

fn position() -> Game {
    let mut game = Game::new_for_agents(3, 3, 991, Difficulty::Tactical);
    game.start_match().unwrap();
    game.battlefields = [3, 3];
    game.battlefield_owner = [None; 2];
    game.legend[0].ready = false;
    game.legend[1].ready = false;
    for player in &mut game.players {
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.hand.clear();
        player.champion = None;
        player.discard.clear();
        player.runes = [0; 6];
        player.exhausted_runes = [0; 6];
        player.energy = 50;
        player.energy_pool = 50;
        player.power_pool = [10; 6];
        player.any_power = 0;
    }
    game
}

#[test]
fn flow_cost_previews_keep_exact_trash_identity_and_reject_an_unavailable_instance() {
    let mut game = position();
    game.players[0].discard = vec![39, 39];
    game.decision = Some(Decision::GrantFlow { player: 0 });
    game.apply(Action::SelectCards { indices: vec![0] })
        .unwrap();
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    assert_eq!(game.granted_flow_indices(0), &[0]);

    let action = |trash_index, source| Action::PlayFromTrashOptions {
        trash_index,
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate: false,
        bonus: false,
        flow: Some(source),
    };
    for (index, source, expected_energy, expected_power) in [
        (0, FlowSource::Granted, 1, 0),
        (0, FlowSource::Printed, 2, 1),
        (1, FlowSource::Printed, 2, 1),
    ] {
        let action = action(index, source);
        let before = format!("{game:?}");
        let plan = game.cost_plan(&action).unwrap();
        assert_eq!(format!("{game:?}"), before);
        assert_eq!(plan.energy, expected_energy);
        assert_eq!(
            plan.power.iter().map(|cost| cost.amount).sum::<u16>(),
            expected_power
        );
        let mut direct = game.clone();
        direct.apply(action).unwrap();
        assert_eq!(
            u32::from(game.players[0].energy - direct.players[0].energy),
            expected_energy
        );
    }

    let unavailable = action(1, FlowSource::Granted);
    let before = format!("{game:?}");
    let mut direct = game.clone();
    assert!(direct.apply(unavailable.clone()).is_err());
    assert_eq!(format!("{direct:?}"), before);
    assert!(game.play_choices(&unavailable).is_err());
    assert!(game.cost_plan(&unavailable).is_err());
    assert_eq!(format!("{game:?}"), before);
}

#[test]
fn trash_cost_previews_match_source_departure_direct_play_and_frozen_payment() {
    for (discard, index, expected) in [(vec![40], 0, 10), (vec![69, 40, 70], 1, 8)] {
        let mut game = position();
        game.players[0].discard = discard;
        game.decision = Some(Decision::LastRites {
            player: 0,
            gear_id: 777,
        });
        let action = Action::PlayFromTrashWithTargets {
            trash_index: index,
            lane: None,
            targets: vec![],
        };
        let before = format!("{game:?}");
        let choices = game.play_choices(&action).unwrap();
        let cost = game.cost_plan(&action).unwrap();
        assert_eq!(format!("{game:?}"), before, "previews must be pure");

        let mut direct = game.clone();
        direct.apply(action.clone()).unwrap();
        let spent = game.players[0].energy - direct.players[0].energy;
        assert_eq!(u32::from(spent), expected);

        let mut staged = game.clone();
        staged
            .apply(Action::BeginPayment {
                action: Box::new(action),
            })
            .unwrap();
        assert_eq!(staged.payment_plan_view(0).unwrap().energy, expected);
        staged.apply(Action::CompletePayment).unwrap();
        assert_eq!(staged.players[0].energy, direct.players[0].energy);
        assert_eq!(staged.players[0].discard, direct.players[0].discard);

        assert_eq!(cost.energy, expected, "cost plan counted its source card");
        assert_eq!(choices.costs.energy, expected);
    }
}
