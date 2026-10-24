//! Resource choice scenarios through public game actions (CR 163-167, 357.1).
use local_rift::cards::Domain;
use local_rift::game::{Action, Difficulty, Game, Target, Unit};
use local_rift::gear::GearState;
use local_rift::payment::PowerPayment;

fn position(card: usize) -> Game {
    let mut game = Game::new(3, 3, 17, Difficulty::Tactical);
    game.start_match().unwrap();
    let state = &mut game.players[0];
    state.hand = vec![card];
    state.runes = [0; 6];
    state.exhausted_runes = [0; 6];
    state.energy_pool = 12;
    state.energy = 12;
    state.power_pool = [0; 6];
    state.any_power = 0;
    game
}
fn unit(id: u32, card_id: usize) -> Unit {
    Unit {
        id,
        card_id,
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
fn play(targets: Vec<Target>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets,
    }
}
fn paid(action: Action, payment: PowerPayment) -> Action {
    Action::WithPowerPayment {
        action: Box::new(action),
        payment,
    }
}
fn enemy(id: u32) -> Target {
    Target::Unit {
        player: 1,
        unit_id: id,
    }
}

#[test]
fn recycling_can_choose_ready_or_exhausted_runes_of_the_same_domain() {
    let mut initial = position(59);
    initial.players[0].runes = [0, 0, 0, 0, 2, 0];
    initial.players[0].exhausted_runes = [0, 0, 0, 0, 1, 0];
    initial.players[0].energy_pool = 0;
    initial.players[0].energy = 1;
    let mut ready = initial.clone();
    ready
        .apply(Action::RecycleRune {
            domain: Domain::Chaos,
            exhausted: false,
        })
        .unwrap();
    assert_eq!(ready.players[0].runes[4], 1);
    assert_eq!(ready.players[0].exhausted_runes[4], 1);
    assert_eq!(ready.players[0].energy, 0);
    assert_eq!(
        ready.players[0].energy_pool, 0,
        "no implicit energy when explicitly recycling a ready rune"
    );
    assert_eq!(ready.players[0].power_pool[4], 1);
    let before = format!("{ready:?}");
    assert!(
        ready
            .apply(Action::RecycleRune {
                domain: Domain::Chaos,
                exhausted: false
            })
            .is_err()
    );
    assert_eq!(format!("{ready:?}"), before);
    let mut exhausted = initial;
    exhausted
        .apply(Action::RecycleRune {
            domain: Domain::Chaos,
            exhausted: true,
        })
        .unwrap();
    assert_eq!(exhausted.players[0].runes[4], 1);
    assert_eq!(exhausted.players[0].exhausted_runes[4], 0);
    assert_eq!(exhausted.players[0].energy, 1);
    assert_eq!(exhausted.players[0].power_pool[4], 1);
}

#[test]
fn nested_payment_wrappers_are_rejected_without_mutation() {
    let mut game = position(59);
    let before = format!("{game:?}");
    let action = paid(
        paid(Action::EndTurn, PowerPayment::default()),
        PowerPayment::default(),
    );
    assert!(game.apply(action).is_err());
    assert_eq!(format!("{game:?}"), before);
}

#[test]
fn player_can_preserve_spell_only_and_flexible_power_for_later() {
    let mut game = position(5); // Defiant Dance accepts Calm OR Chaos.
    game.players[0].power_pool[1] = 1;
    game.players[0].power_pool[4] = 1;
    game.players[0].any_power = 1;
    game.legend[0].spell_power = 1;
    game.players[1].units = vec![unit(100, 59), unit(101, 59)];
    let action = play(vec![enemy(100), enemy(101)]);
    let before = format!("{game:?}");
    let quote = game.quote_power_payment(&action).unwrap();
    assert_eq!(quote.suggested.spell, 1);
    assert_eq!(quote.suggested.total(), 1);
    assert_eq!(
        format!("{game:?}"),
        before,
        "quoting must not mutate or draw cards"
    );
    let mut payment = PowerPayment::default();
    payment.colored[4] = 1;
    game.apply(paid(action, payment)).unwrap();
    assert_eq!(game.players[0].power_pool[1], 1);
    assert_eq!(game.players[0].power_pool[4], 0);
    assert_eq!(game.players[0].any_power, 1);
    assert_eq!(game.legend[0].spell_power, 1);
}

#[test]
fn a_split_payment_covers_restricted_base_cost_and_each_deflect_choice() {
    let mut game = position(53); // Falling Star: 2 Fury; two choices of Irelia: 2 generic.
    game.players[1].units = vec![unit(100, 0)];
    game.players[0].power_pool[0] = 1;
    game.players[0].power_pool[4] = 1;
    game.players[0].any_power = 1;
    game.legend[0].spell_power = 1;
    let action = play(vec![enemy(100), enemy(100)]);
    let quote = game.quote_power_payment(&action).unwrap();
    assert_eq!(
        quote.costs.iter().map(|c| c.amount).collect::<Vec<_>>(),
        [2, 2]
    );
    let mut payment = PowerPayment {
        any: 1,
        spell: 1,
        ..Default::default()
    };
    payment.colored[0] = 1;
    payment.colored[4] = 1;
    game.apply(paid(action, payment)).unwrap();
    assert_eq!(game.players[0].power_pool, [0; 6]);
    assert_eq!(game.players[0].any_power, 0);
    assert_eq!(game.legend[0].spell_power, 0);
}

#[test]
fn explicit_rune_choice_preserves_other_runes_and_conserves_energy() {
    let mut game = position(5);
    game.players[1].units = vec![unit(100, 59), unit(101, 59)];
    game.players[0].runes = [0, 1, 0, 0, 1, 0];
    game.players[0].energy = 14;
    let mut payment = PowerPayment::default();
    payment.recycle[4] = 1;
    let deck_len = game.players[0].rune_deck.len();
    game.apply(paid(play(vec![enemy(100), enemy(101)]), payment))
        .unwrap();
    assert_eq!(game.players[0].runes, [0, 1, 0, 0, 0, 0]);
    assert_eq!(game.players[0].rune_deck.len(), deck_len + 1);
    assert_eq!(
        game.players[0].energy_pool, 12,
        "spend one energy then exhaust recycled rune for one"
    );
    assert_eq!(game.players[0].energy, 13);
}

#[test]
fn underpayment_overpayment_wrong_domain_and_unavailable_sources_are_atomic() {
    let mut game = position(53);
    game.players[1].units = vec![unit(100, 59)];
    game.players[0].power_pool = [3, 0, 0, 0, 3, 0];
    let action = play(vec![enemy(100), enemy(100)]);
    let before = format!("{game:?}");
    for payment in [
        PowerPayment::default(),
        PowerPayment {
            colored: [3, 0, 0, 0, 0, 0],
            ..Default::default()
        },
        PowerPayment {
            colored: [0, 0, 0, 0, 2, 0],
            ..Default::default()
        },
        PowerPayment {
            any: 2,
            ..Default::default()
        },
        PowerPayment {
            colored: [2, 0, 0, 0, 0, 0],
            spell: 1,
            ..Default::default()
        },
    ] {
        assert!(game.apply(paid(action.clone(), payment)).is_err());
        assert_eq!(format!("{game:?}"), before);
    }
}

#[test]
fn hiding_and_equip_use_selected_sources_but_cannot_spend_spell_only_power() {
    let mut game = position(21); // Hidden Hourglass.
    game.battlefield_owner[0] = Some(0);
    game.players[0].any_power = 1;
    game.players[0].power_pool[4] = 1;
    game.legend[0].spell_power = 1;
    let hide = Action::Hide {
        hand_index: 0,
        lane: 0,
    };
    let before = format!("{game:?}");
    assert!(
        game.apply(paid(
            hide.clone(),
            PowerPayment {
                spell: 1,
                ..Default::default()
            }
        ))
        .is_err()
    );
    assert_eq!(format!("{game:?}"), before);
    game.apply(paid(
        hide,
        PowerPayment {
            any: 1,
            ..Default::default()
        },
    ))
    .unwrap();
    assert_eq!(game.players[0].power_pool[4], 1);
    assert_eq!(game.legend[0].spell_power, 1);

    game.players[0].units.push(unit(200, 59));
    game.players[0].gear.push(GearState {
        id: 300,
        card_id: 3,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    let equip = Action::Equip {
        gear_id: 300,
        target: Target::Unit {
            player: 0,
            unit_id: 200,
        },
    };
    assert!(
        game.apply(paid(
            equip.clone(),
            PowerPayment {
                spell: 1,
                ..Default::default()
            }
        ))
        .is_err()
    );
    game.apply(paid(
        equip,
        PowerPayment {
            colored: [0, 0, 0, 0, 1, 0],
            ..Default::default()
        },
    ))
    .unwrap();
    assert_eq!(game.players[0].power_pool[4], 0);
    assert_eq!(game.legend[0].spell_power, 1);
}

#[test]
fn replay_uses_flow_generic_cost_and_preserves_colored_sources() {
    let mut game = position(59);
    game.players[0].discard = vec![39]; // Lightning Rush, Flow 2 Energy + 1 any-domain power.
    game.players[0].any_power = 1;
    game.players[0].power_pool[4] = 1;
    let action = Action::PlayFromTrashWithTargets {
        trash_index: 0,
        lane: None,
        targets: vec![],
    };
    let quote = game.quote_power_payment(&action).unwrap();
    assert_eq!(quote.suggested.total(), 1);
    game.apply(paid(
        action,
        PowerPayment {
            any: 1,
            ..Default::default()
        },
    ))
    .unwrap();
    assert_eq!(game.players[0].any_power, 0);
    assert_eq!(game.players[0].power_pool[4], 1);
    assert_eq!(game.players[0].energy, 10);
}

#[test]
fn quoted_defaults_reproduce_automatic_payment_through_live_games() {
    let mut verified = 0;
    let mut paid_quotes = 0;
    for deck in 0..4 {
        let mut game = Game::new(
            deck,
            (deck + 1) % 4,
            728 + deck as u64,
            Difficulty::Tactical,
        );
        game.start_match().unwrap();
        for _ in 0..240 {
            if game.winner.is_some() {
                break;
            }
            for action in game.legal_actions().into_iter().take(8) {
                let Ok(quote) = game.quote_power_payment(&action) else {
                    continue;
                };
                let mut automatic = game.clone();
                automatic.apply(action.clone()).unwrap();
                let mut exact = game.clone();
                paid_quotes += usize::from(quote.suggested.total() > 0);
                exact.apply(paid(action, quote.suggested)).unwrap();
                assert_eq!(format!("{automatic:?}"), format!("{exact:?}"));
                verified += 1;
            }
            game.step_ai().unwrap();
        }
    }
    assert!(verified > 1000, "only {verified} actions checked");
    assert!(
        paid_quotes > 100,
        "only {paid_quotes} actual payments checked"
    );
}
