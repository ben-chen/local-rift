//! Legend behavior through public actions, including pending triggers and responses.
use local_rift::{
    activated::AbilitySource,
    deck::CustomDeck,
    game::{Action, ChainAbility, ChainSource, Decision, Difficulty, Game, Target, Unit},
    gear::GearState,
    legends::LegendAbility,
};

fn position(legend: usize) -> Game {
    let mut g = Game::new_with_custom(
        [0, 3],
        [Some(CustomDeck::from_legend(legend).unwrap()), None],
        21609,
        Difficulty::Tactical,
        false,
    )
    .unwrap();
    g.start_match().unwrap();
    g.battlefields = [0, 0];
    for p in &mut g.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.discard.clear();
        p.runes = [0; 6];
        p.exhausted_runes = [0; 6];
        p.energy_pool = 30;
        p.energy = 30;
        p.power_pool = [6; 6];
    }
    g
}
fn unit(g: &mut Game, player: usize, id: u32, card: usize, lane: Option<usize>) -> Target {
    g.players[player].units.push(Unit {
        id,
        card_id: card,
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
    Target::Unit {
        player,
        unit_id: id,
    }
}
fn gear(g: &mut Game, player: usize, id: u32, card: usize) -> Target {
    g.players[player].gear.push(GearState {
        id,
        card_id: card,
        ready: true,
        lane: None,
        attached_to: None,
        token: false,
        temporary: false,
    });
    Target::Gear {
        player,
        gear_id: id,
    }
}
fn play(g: &mut Game, card: usize, targets: Vec<Target>) {
    let p = g.priority();
    let i = g.players[p].hand.len();
    g.players[p].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index: i,
        lane: None,
        targets,
    })
    .unwrap();
}
fn resolve(g: &mut Game) {
    assert!(g.decision.is_none(), "{:?}", g.decision);
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn rejected(g: &mut Game, a: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(a).is_err());
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn previous_eight_training_lists_and_legend_entries_remain_identical() {
    let decks: serde_json::Value = serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c16/predecessor/training_decks.json"
    ))
    .unwrap();
    for id in 0..8 {
        assert_eq!(CustomDeck::from_legend(id).unwrap().to_json(), decks[id]);
    }
    let old: serde_json::Value = serde_json::from_str(include_str!(
        "../assets/data/card_admissions/c16/predecessor/legends.json"
    ))
    .unwrap();
    let current: serde_json::Value =
        serde_json::from_str(include_str!("../assets/data/legends.json")).unwrap();
    assert_eq!(
        &current["legends"].as_array().unwrap()[..8],
        old["legends"].as_array().unwrap()
    );
}

#[test]
fn rengar_must_choose_can_choose_new_unit_and_bonus_expires() {
    let mut g = position(8);
    play(&mut g, 72, vec![]);
    assert!(matches!(
        g.decision,
        Some(Decision::LegendTargets { player: 0 })
    ));
    assert_eq!(g.trigger_target_bounds(), Some((1, 1)));
    rejected(&mut g, Action::Choose { accept: false });
    rejected(&mut g, Action::ChooseTriggerTargets { targets: vec![] });
    let id = g.players[0].units[0].id;
    let target = Target::Unit {
        player: 0,
        unit_id: id,
    };
    let before = g.unit_might(&g.players[0].units[0]).unwrap();
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![target],
    })
    .unwrap();
    assert_eq!(g.unit_might(&g.players[0].units[0]).unwrap(), before);
    assert_eq!(
        g.chain_source(&g.chain[0]),
        ChainSource::Legend {
            player: 0,
            legend_id: 8
        }
    );
    assert!(matches!(
        g.chain[0].ability,
        Some(ChainAbility::Legend(LegendAbility::RengarMight { .. }))
    ));
    resolve(&mut g);
    assert_eq!(g.unit_might(&g.players[0].units[0]).unwrap(), before + 1);
    g.apply(Action::EndTurn).unwrap();
    assert_eq!(g.players[0].units[0].temporary_buff, 0);
}

#[test]
fn rengar_can_target_enemy_but_deflect_and_untargetability_are_paid_and_checked() {
    let mut g = position(8);
    let enemy = unit(&mut g, 1, 901, 93, None);
    let protected = unit(&mut g, 1, 902, 31, None);
    play(&mut g, 72, vec![]);
    rejected(
        &mut g,
        Action::ChooseTriggerTargets {
            targets: vec![protected],
        },
    );
    g.players[0].power_pool = [0; 6];
    g.players[0].any_power = 0;
    rejected(
        &mut g,
        Action::ChooseTriggerTargets {
            targets: vec![enemy],
        },
    );
    g.players[0].any_power = 1;
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![enemy],
    })
    .unwrap();
    assert_eq!(g.players[0].any_power, 0);
    resolve(&mut g);
    assert_eq!(g.players[1].units[0].temporary_buff, 1);
}

#[test]
fn rengar_mistargets_after_departure_but_allows_a_board_move() {
    for leave in [false, true] {
        let mut g = position(8);
        let target = unit(&mut g, 0, 903, 72, Some(0));
        play(&mut g, 72, vec![]);
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![target],
        })
        .unwrap();
        if leave {
            g.players[0].units.remove(0);
        } else {
            g.players[0].units[0].lane = None;
        }
        resolve(&mut g);
        assert_eq!(g.players[0].units[0].temporary_buff, i64::from(!leave));
    }
}

#[test]
fn rengar_tokens_trigger_once_without_inheriting_the_creator_trigger() {
    let mut g = position(8);
    play(&mut g, 319, vec![]);
    let mut bonuses = 0;
    let mut placements = 0;
    for _ in 0..40 {
        let action = match &g.decision {
            Some(Decision::LegendTargets { .. }) => {
                bonuses += 1;
                Action::ChooseTriggerTargets {
                    targets: vec![Target::Unit {
                        player: 0,
                        unit_id: g.players[0].units[0].id,
                    }],
                }
            }
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::TokenPlacement { .. }) => {
                placements += 1;
                Action::PlaceToken { lane: None }
            }
            Some(Decision::TokenPlay { .. }) => {
                placements += 1;
                Action::PlaceToken { lane: None }
            }
            Some(d) => panic!("Unexpected choice {d:?}"),
            None if !g.chain.is_empty() => Action::Pass,
            None => break,
        };
        g.apply(action).unwrap();
    }
    // The token has a prescribed location, so it finalizes there without a placement prompt.
    assert_eq!((bonuses, placements), (2, 0));
    assert_eq!(g.players[0].units.len(), 2);
    assert_eq!(g.players[0].units.iter().filter(|u| u.token).count(), 1);
}

#[test]
fn ezreal_requires_two_qualifying_targets_and_draws_on_a_reactable_chain() {
    let mut g = position(9);
    let enemy = unit(&mut g, 1, 904, 72, None);
    rejected(&mut g, Action::ActivateLegend { target: None });
    play(&mut g, 63, vec![enemy]);
    assert_eq!(g.ezreal_progress(0), 1);
    rejected(&mut g, Action::ActivateLegend { target: None });
    play(&mut g, 63, vec![enemy]);
    assert_eq!(g.ezreal_progress(0), 2);
    let hand = g.players[0].hand.len();
    g.apply(Action::ActivateLegend { target: None }).unwrap();
    assert!(!g.legend[0].ready);
    assert_eq!(g.players[0].hand.len(), hand);
    assert_eq!(
        g.chain_source(g.chain.last().unwrap()),
        ChainSource::Legend {
            player: 0,
            legend_id: 9
        }
    );
    resolve(&mut g);
    assert_eq!(g.players[0].hand.len(), hand + 1);
    rejected(&mut g, Action::ActivateLegend { target: None });
}

#[test]
fn ezreal_split_targets_and_repeat_count_individually_at_finalization() {
    for repeat in [false, true] {
        let mut g = position(9);
        let a = unit(&mut g, 1, 905, 72, None);
        let b = unit(&mut g, 1, 906, 72, None);
        g.players[0].hand.push(50);
        g.apply(Action::PlayCardOptions {
            hand_index: 0,
            lane: None,
            targets: if repeat { vec![a] } else { vec![a, b] },
            repeat_targets: if repeat { vec![a] } else { vec![] },
            repeat,
            accelerate: false,
            bonus: false,
        })
        .unwrap();
        assert_eq!(g.ezreal_progress(0), 2);
        assert_eq!(g.chain.len(), 1);
    }
}

#[test]
fn ezreal_counts_enemy_gear_and_unit_triggers_but_not_friendly_targets() {
    let mut g = position(9);
    let own = unit(&mut g, 0, 907, 72, None);
    let enemy = unit(&mut g, 1, 908, 72, None);
    play(&mut g, 63, vec![own]);
    assert_eq!(g.ezreal_progress(0), 0);
    resolve(&mut g);
    play(&mut g, 146, vec![]);
    assert!(matches!(g.decision, Some(Decision::OnPlayTarget { .. })));
    g.apply(Action::ChooseTarget {
        target: Some(enemy),
    })
    .unwrap();
    assert_eq!(g.ezreal_progress(0), 1);
    resolve(&mut g);
    let item = gear(&mut g, 1, 909, 231);
    play(&mut g, 66, vec![item]);
    assert_eq!(g.ezreal_progress(0), 2);
}

#[test]
fn ezreal_ignores_gear_abilities_and_spells_targeting_spells() {
    let mut g = position(9);
    let enemy = unit(&mut g, 1, 910, 72, None);
    gear(&mut g, 0, 911, 231);
    g.apply(Action::ActivateAbility {
        source: AbilitySource::Gear {
            player: 0,
            gear_id: 911,
        },
        ability_index: 0,
        targets: vec![enemy],
    })
    .unwrap();
    assert_eq!(g.ezreal_progress(0), 0);
    resolve(&mut g);
    g.apply(Action::EndTurn).unwrap();
    // This fixture has no runes; refill the reacting seat after turn cleanup.
    g.players[0].energy = 20;
    g.players[0].energy_pool = 20;
    play(&mut g, 63, vec![enemy]);
    g.apply(Action::Pass).unwrap();
    let chain_id = g.chain[0].id;
    play(&mut g, 37, vec![Target::Spell { chain_id }]);
    assert_eq!(g.ezreal_progress(0), 0);
}

#[test]
fn ezreal_failed_actions_and_queries_do_not_add_progress() {
    let mut g = position(9);
    let enemy = unit(&mut g, 1, 912, 72, None);
    g.players[0].hand = vec![50];
    let action = Action::PlayCardOptions {
        hand_index: 0,
        lane: None,
        targets: vec![enemy],
        repeat_targets: vec![enemy],
        repeat: true,
        accelerate: false,
        bonus: false,
    };
    let before = format!("{g:?}");
    g.quote_power_payment(&action).unwrap();
    assert_eq!(format!("{g:?}"), before);
    g.players[0].energy = 0;
    g.players[0].energy_pool = 0;
    g.players[0].power_pool = [0; 6];
    rejected(&mut g, action);
    assert_eq!(g.ezreal_progress(0), 0);
}

#[test]
fn ezreal_history_survives_a_counter_and_resets_for_both_seats_on_next_turn() {
    let mut g = position(9);
    let enemy = unit(&mut g, 1, 913, 72, None);
    play(&mut g, 63, vec![enemy]);
    let chain_id = g.chain[0].id;
    g.apply(Action::Pass).unwrap();
    play(&mut g, 86, vec![Target::Spell { chain_id }]);
    resolve(&mut g);
    assert_eq!(g.ezreal_progress(0), 1);
    assert!(g.chain.is_empty());
    g.play_history.ezreal_progress[1] = 2;
    g.apply(Action::EndTurn).unwrap();
    assert_eq!(g.play_history.ezreal_progress, [0, 0]);
}

#[test]
fn ezreal_metadata_unit_attack_trigger_counts_when_targets_finalize() {
    let mut g = position(9);
    let own = unit(&mut g, 0, 914, 168, None);
    let enemy = unit(&mut g, 1, 915, 72, Some(0));
    let Target::Unit { unit_id, .. } = own else {
        unreachable!()
    };
    g.battlefield_owner[0] = Some(1);
    g.apply(Action::Move {
        unit_id,
        lane: Some(0),
    })
    .unwrap();
    assert!(
        matches!(
            g.decision,
            Some(Decision::TriggerTargets { card_id: 168, .. })
        ),
        "{:?}",
        g.decision
    );
    assert_eq!(g.ezreal_progress(0), 0);
    g.apply(Action::ChooseTriggerTargets {
        targets: vec![enemy],
    })
    .unwrap();
    assert_eq!(g.ezreal_progress(0), 1);
}

#[test]
fn ezreal_non_resource_draw_cannot_interrupt_pending_payment() {
    let mut g = position(9);
    g.play_history.ezreal_progress[0] = 2;
    g.players[0].hand = vec![57];
    g.apply(Action::BeginPayment {
        action: Box::new(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        }),
    })
    .unwrap();
    rejected(&mut g, Action::ActivateLegend { target: None });
    assert!(g.legend[0].ready);
}

#[test]
fn rengar_unfinalizable_mandatory_trigger_is_removed_without_a_deadlock() {
    let mut g = position(8);
    play(&mut g, 319, vec![]);
    // The same play generated Rengar and the unit's token-creation trigger.
    let Some(Decision::OrderTriggers { options, .. }) = g.decision.clone() else {
        panic!("{:?}", g.decision)
    };
    let rengar = options
        .iter()
        .find(|o| o.label.contains("Pridestalker"))
        .unwrap()
        .id;
    g.players[0].units.clear();
    g.players[1].units.clear();
    g.apply(Action::OrderTrigger { trigger_id: rengar })
        .unwrap();
    assert!(!matches!(g.decision, Some(Decision::LegendTargets { .. })));
}
