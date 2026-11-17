use super::*;
use crate::game::{
    Target,
    test_support::{position, ready_unit, settle_unopposed_chain},
};

// An authored position; the pending choice itself is reached by a normal cast.
fn irelia(actor: usize) -> (Game, Target) {
    let mut game = position(actor, 0, 486010);
    game.legend[actor].ready = true;
    game.players[actor].power_pool = [0; 6];
    game.players[actor].hand = vec![485];
    let unit_id = ready_unit(&mut game, actor, 71);
    let target = Target::Unit {
        player: actor,
        unit_id,
    };
    game.unit_mut(target).unwrap().ready = false;
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![target],
    })
    .unwrap();
    assert!(matches!(game.decision, Some(Decision::Legend { .. })));
    (game, target)
}

#[test]
fn binary_choices_irelia_payment_is_live_read_only_and_respondable() {
    for actor in 0..2 {
        let (mut game, target) = irelia(actor);
        let before = format!("{game:?}");
        let view = game.binary_choices().unwrap().unwrap();
        assert_eq!(view.player, actor);
        assert_eq!(view.options[0].label, "Pay 1 Power");
        assert!(!view.options[0].enabled());
        assert!(view.options[0].unavailable_reason.is_some());
        assert!(view.options[1].enabled());
        assert_eq!(format!("{game:?}"), before);

        let mut decline = game.clone();
        decline.apply(view.options[1].action.clone()).unwrap();
        assert!(decline.legend[actor].ready);
        assert!(!decline.unit_ref(target).unwrap().ready);

        game.legend[actor].spell_power = 1;
        assert!(
            !game.binary_choices().unwrap().unwrap().options[0].enabled(),
            "spell-only power cannot pay a legend"
        );
        game.players[actor].any_power = 1;
        let accepted = game.binary_choices().unwrap().unwrap();
        assert!(accepted.options[0].enabled());
        game.apply(accepted.options[0].action.clone()).unwrap();
        assert_eq!(game.players[actor].any_power, 0);
        assert!(!game.legend[actor].ready);
        assert!(
            !game.unit_ref(target).unwrap().ready,
            "the ready effect waits for responses"
        );
        assert_eq!(game.chain.len(), 2);
        settle_unopposed_chain(&mut game);
        assert!(game.unit_ref(target).unwrap().ready);
    }
}

#[test]
fn binary_choices_irelia_exhaustion_and_conquer_energy_are_distinct_costs() {
    for actor in 0..2 {
        let (mut game, _) = irelia(actor);
        game.players[actor].any_power = 1;
        game.legend[actor].ready = false;
        let view = game.binary_choices().unwrap().unwrap();
        assert!(!view.options[0].enabled());
        assert!(
            view.options[0]
                .unavailable_reason
                .as_ref()
                .unwrap()
                .contains("ready")
        );

        // Authored conquest-trigger boundary; no claim that setup is a conquest.
        game.decision = Some(Decision::Legend {
            player: actor,
            trigger: LegendTrigger::IreliaConquer,
        });
        game.players[actor].energy_pool = 0;
        game.players[actor].energy = 0;
        let view = game.binary_choices().unwrap().unwrap();
        assert_eq!(view.options[0].label, "Pay 1 Energy");
        assert!(!view.options[0].enabled());
        assert!(view.options[1].enabled());
        game.players[actor].energy_pool = 1;
        game.players[actor].energy = 1;
        let view = game.binary_choices().unwrap().unwrap();
        assert!(view.options[0].enabled());
        game.apply(view.options[0].action.clone()).unwrap();
        assert!(!game.legend[actor].ready);
        assert_eq!(game.players[actor].energy, 0);
        settle_unopposed_chain(&mut game);
        assert!(game.legend[actor].ready);
    }
}

#[test]
fn binary_choices_predict_answers_preserve_hidden_state_until_applied() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 486020);
        game.players[actor].deck = vec![71, 485];
        game.decision = Some(Decision::Predict {
            player: actor,
            card_id: 485,
        });
        let before = format!("{game:?}");
        let view = game.binary_choices().unwrap().unwrap();
        assert!(view.options.iter().all(BinaryChoiceOption::enabled));
        assert_eq!(view.options[0].label, "Recycle to bottom");
        assert_eq!(view.options[1].label, "Keep on top");
        assert_eq!(format!("{game:?}"), before);
        let mut keep = game.clone();
        keep.apply(view.options[1].action.clone()).unwrap();
        assert_eq!(keep.players[actor].deck, vec![71, 485]);
        game.apply(view.options[0].action.clone()).unwrap();
        assert_eq!(game.players[actor].deck, vec![485, 71]);
    }
}

#[test]
fn binary_choices_do_not_mislabel_other_decision_shapes() {
    let mut game = position(0, 0, 486030);
    assert_eq!(game.binary_choices().unwrap(), None);
    game.decision = Some(Decision::GrantFlow { player: 0 });
    assert_eq!(game.binary_choices().unwrap(), None);
    game.decision = Some(Decision::CombatDamage {
        player: 0,
        lane: 0,
        total: 3,
    });
    assert_eq!(game.binary_choices().unwrap(), None);
}
