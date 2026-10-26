//! Independent look/reveal conformance; initial board/resources are controlled fixtures.
use super::*;
use crate::activated::AbilitySource;
use crate::gear::GearState;

fn position() -> Game {
    let mut g = Game::new_for_agents(3, 3, 20020, Difficulty::Tactical);
    g.start_match().unwrap();
    g.battlefields = [2, 2];
    g.battlefield_owner = [None; 2];
    for p in &mut g.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.runes = [0; 6];
        p.exhausted_runes = [0; 6];
        p.energy_pool = 100;
        p.energy = 100;
        p.power_pool = [20; 6];
        p.any_power = 0;
    }
    for legend in &mut g.legend {
        legend.ready = false;
    }
    g.trigger_state.draws = [0; 2];
    g
}
fn unit(g: &mut Game, player: usize, card_id: usize, lane: Option<usize>) -> Target {
    let id = g.create_unit(player, card_id, false, 0);
    let target = Target::Unit {
        player,
        unit_id: id,
    };
    g.unit_mut(target).unwrap().lane = lane;
    if let Some(lane) = lane {
        g.battlefield_owner[lane] = Some(player);
    }
    target
}
fn gear(g: &mut Game, player: usize, card_id: usize) -> u32 {
    let id = g.next_unit_id;
    g.next_unit_id += 1;
    g.players[player].gear.push(GearState {
        id,
        card_id,
        lane: None,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    id
}
fn cast(g: &mut Game, card: usize, targets: Vec<Target>) {
    let p = g.priority();
    let hand_index = g.players[p].hand.len();
    g.players[p].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
}
fn resolve(g: &mut Game) {
    for _ in 0..80 {
        if g.decision.is_some() || g.chain.is_empty() {
            return;
        }
        g.apply(Action::Pass).unwrap();
    }
    panic!("unsettled chain: {:?}", g.chain);
}
fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action).is_err());
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn lightning_rush_draws_selected_occurrence_and_emits_second_draw_observer() {
    let mut g = position();
    unit(&mut g, 0, 96, None);
    gear(&mut g, 0, 195);
    for expected in 1..=2 {
        g.players[0].deck = vec![70, 96, 145];
        cast(&mut g, 39, vec![]);
        resolve(&mut g);
        assert!(
            matches!(&g.decision, Some(Decision::TopThree { cards, .. }) if cards == &[145,96,70])
        );
        g.apply(Action::SelectCards { indices: vec![1] }).unwrap();
        assert_eq!(
            g.players[0].hand.iter().filter(|&&id| id == 96).count(),
            expected
        );
        assert_eq!(g.trigger_state.draws[0], expected as u64);
        assert!(g.players[0].deck.is_empty());
        assert!(g.players[0].discard.contains(&70));
        assert!(g.players[0].discard.contains(&145));
    }
    assert!(
        g.chain.iter().any(|i| i.card_id == 195)
            || matches!(
                g.decision,
                Some(Decision::TriggerTargets { card_id: 195, .. })
            )
    );
}

#[test]
fn lightning_rush_decline_and_empty_look_do_not_draw_or_burn_out() {
    for deck in [vec![70, 96, 145], vec![]] {
        let mut g = position();
        g.players[0].deck = deck.clone();
        let scores = [g.players[0].score, g.players[1].score];
        cast(&mut g, 39, vec![]);
        resolve(&mut g);
        g.apply(Action::SelectCards { indices: vec![] }).unwrap();
        assert_eq!(g.trigger_state.draws[0], 0);
        assert!(g.players[0].hand.is_empty());
        assert!(g.players[0].deck.is_empty());
        assert_eq!([g.players[0].score, g.players[1].score], scores);
        for card in deck {
            assert!(g.players[0].discard.contains(&card));
        }
    }
}

fn draw_look(g: &Game) -> (u32, Vec<CardId>, Vec<usize>) {
    let Some(Decision::DrawLook {
        choice_id,
        cards,
        eligible,
        ..
    }) = &g.decision
    else {
        panic!("expected a private draw look, got {:?}", g.decision);
    };
    (*choice_id, cards.clone(), eligible.clone())
}
fn choose_draw(g: &mut Game, index: Option<usize>) {
    let choice_id = draw_look(g).0;
    g.apply(Action::ResolveDrawLook { choice_id, index })
        .unwrap();
}
fn acknowledge(g: &mut Game) {
    assert!(matches!(g.decision, Some(Decision::RevealedCard { .. })));
    g.apply(Action::Choose { accept: true }).unwrap();
}
fn look_from_play(g: &mut Game, source: CardId, top: &[CardId]) {
    g.players[0].deck = top.iter().rev().copied().collect();
    cast(g, source, vec![]);
    resolve(g);
    assert!(matches!(g.decision, Some(Decision::DrawLook { .. })));
}
fn move_to_friendly_field(g: &mut Game, card: CardId) -> Target {
    unit(g, 0, 96, Some(0));
    let target = unit(g, 0, card, None);
    g.unit_mut(target).unwrap().ready = true;
    let Target::Unit { unit_id, .. } = target else {
        unreachable!()
    };
    g.apply(Action::Move {
        unit_id,
        lane: Some(0),
    })
    .unwrap();
    target
}

#[test]
fn double_trouble_selects_an_occurrence_reveals_only_it_and_recycles_remainder() {
    let mut g = position();
    g.players[0].deck = vec![71, 3, 96, 96];
    let hand_before = g.players[0].hand.len();
    cast(&mut g, 389, vec![]);
    resolve(&mut g);
    let (id, cards, eligible) = draw_look(&g);
    assert_eq!(cards, [96, 96, 3]);
    assert_eq!(eligible, [0, 1]);
    reject(
        &mut g,
        Action::ResolveDrawLook {
            choice_id: id + 1,
            index: Some(0),
        },
    );
    reject(
        &mut g,
        Action::ResolveDrawLook {
            choice_id: id,
            index: Some(2),
        },
    );
    reject(
        &mut g,
        Action::ResolveDrawLook {
            choice_id: id,
            index: Some(3),
        },
    );
    choose_draw(&mut g, Some(1));
    assert!(matches!(
        g.decision,
        Some(Decision::RevealedCard {
            player: 0,
            source_card_id: 389,
            card_id: 96,
            outcome: RevealOutcome::Draw
        })
    ));
    assert_eq!(g.players[0].hand.len(), hand_before + 1);
    assert_eq!(g.players[0].hand.last(), Some(&96));
    assert_eq!(g.trigger_state.draws[0], 1);
    assert_eq!(g.players[0].deck.last(), Some(&71));
    let mut rest = g.players[0].deck[..2].to_vec();
    rest.sort_unstable();
    assert_eq!(rest, [3, 96]);
    assert_eq!(g.chain.iter().filter(|i| i.card_id == 389).count(), 1);
    assert!(!g.players[0].discard.contains(&389));
    reject(&mut g, Action::Choose { accept: false });
    acknowledge(&mut g);
    assert!(g.players[0].discard.contains(&389));
    assert_eq!(g.trigger_state.completed_cards[0], 1);
}

#[test]
fn look_decline_no_eligible_empty_and_last_card_do_not_burn_out() {
    for (top, selected) in [
        (vec![3, 7], None),
        (vec![96], None),
        (vec![], None),
        (vec![96], Some(0)),
    ] {
        let mut g = position();
        let scores = [g.players[0].score, g.players[1].score];
        look_from_play(&mut g, 389, &top);
        choose_draw(&mut g, selected);
        if selected.is_some() {
            acknowledge(&mut g);
        }
        assert_eq!(g.trigger_state.draws[0], u64::from(selected.is_some()));
        assert_eq!(g.players[0].hand.len(), usize::from(selected.is_some()));
        assert_eq!(
            g.players[0].deck.len(),
            top.len() - usize::from(selected.is_some())
        );
        assert_eq!([g.players[0].score, g.players[1].score], scores);
        assert_eq!(g.players[0].discard, [389]);
        assert!(g.decision.is_none());
    }
}

#[test]
fn fate_weaver_filters_printed_spell_energy_and_keeps_trigger_mandatory() {
    let mut g = position();
    // Ezreal's live discount must not change the printed-four threshold.
    unit(&mut g, 0, 35, None);
    look_from_play(&mut g, 390, &[26, 67, 17, 117]);
    assert_eq!(draw_look(&g).2, [0]);
    assert!(g.players[0].units.iter().any(|u| u.card_id == 390));
    choose_draw(&mut g, Some(0));
    assert!(matches!(
        g.decision,
        Some(Decision::RevealedCard {
            card_id: 26,
            outcome: RevealOutcome::Draw,
            ..
        })
    ));
    acknowledge(&mut g);
    assert_eq!(g.players[0].hand, [26]);
    assert_eq!(g.players[0].deck.len(), 3);
}

#[test]
fn captain_requires_unit_or_gear_and_both_printed_cost_limits() {
    for top in [vec![33, 119, 117, 16], vec![230, 119, 117, 16]] {
        let mut g = position();
        look_from_play(&mut g, 391, &top);
        assert_eq!(draw_look(&g).2, [0]);
        let id = draw_look(&g).0;
        for index in 1..4 {
            reject(
                &mut g,
                Action::ResolveDrawLook {
                    choice_id: id,
                    index: Some(index),
                },
            );
        }
        choose_draw(&mut g, Some(0));
        acknowledge(&mut g);
        assert_eq!(g.players[0].hand, [top[0]]);
        assert_eq!(g.players[0].deck.len(), 3);
    }
}

#[test]
fn on_play_look_survives_real_return_to_hand_response() {
    for source in [390, 391] {
        let mut g = position();
        let enemy = unit(&mut g, 1, 70, None);
        let draw = if source == 390 { 26 } else { 33 };
        g.players[0].deck = vec![draw];
        cast(&mut g, source, vec![]);
        for _ in 0..12 {
            if g.chain
                .iter()
                .any(|i| i.card_id == source && i.ability.is_some())
            {
                break;
            }
            g.apply(Action::Pass).unwrap();
        }
        let own = g.players[0]
            .units
            .iter()
            .find(|u| u.card_id == source)
            .unwrap()
            .id;
        if g.priority() == 0 {
            g.apply(Action::Pass).unwrap();
        }
        cast(
            &mut g,
            16,
            vec![
                enemy,
                Target::Unit {
                    player: 0,
                    unit_id: own,
                },
            ],
        );
        resolve(&mut g);
        assert!(g.players[0].hand.contains(&source));
        assert!(!g.players[0].units.iter().any(|u| u.id == own));
        assert_eq!(draw_look(&g).1, [draw]);
        choose_draw(&mut g, Some(0));
        acknowledge(&mut g);
        assert!(g.players[0].hand.contains(&draw));
    }
}

#[test]
fn double_trouble_repeats_complete_draw_and_recycle_before_fresh_private_look() {
    let mut g = position();
    let portal = gear(&mut g, 0, 256);
    g.apply(Action::ActivateAbility {
        source: AbilitySource::Gear {
            player: 0,
            gear_id: portal,
        },
        ability_index: 0,
        targets: vec![],
    })
    .unwrap();
    resolve(&mut g);
    unit(&mut g, 0, 96, None);
    gear(&mut g, 0, 195);
    g.players[0].hand = vec![389];
    g.players[0].deck = vec![96, 96, 96];
    let action = Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    };
    let choices = g.play_choices(&action).unwrap();
    assert_eq!(choices.repeats.len(), 2);
    g.apply(Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats: choices
                .repeats
                .iter()
                .map(|r| RepeatSelection {
                    option_id: r.id,
                    choices: ExecutionChoices::default(),
                })
                .collect(),
            ..Default::default()
        },
    })
    .unwrap();
    resolve(&mut g);
    let mut prior_id = None;
    for execution in 0..3 {
        let (id, cards, _) = draw_look(&g);
        assert_ne!(Some(id), prior_id);
        prior_id = Some(id);
        assert_eq!(cards.len(), 3 - execution);
        choose_draw(&mut g, Some(0));
        assert!(matches!(
            g.decision,
            Some(Decision::RevealedCard { card_id: 96, .. })
        ));
        assert_eq!(g.players[0].deck.len(), 2 - execution);
        assert_eq!(g.chain.iter().filter(|i| i.card_id == 389).count(), 1);
        assert!(!g.chain.iter().any(|i| i.card_id == 195));
        assert!(!g.players[0].discard.contains(&389));
        acknowledge(&mut g);
    }
    assert_eq!(g.trigger_state.draws[0], 3);
    assert_eq!(g.players[0].hand, [96, 96, 96]);
    assert_eq!(
        g.players[0].discard.iter().filter(|&&id| id == 389).count(),
        1
    );
    assert!(
        g.chain.iter().any(|i| i.card_id == 195)
            || matches!(
                g.decision,
                Some(Decision::TriggerTargets { card_id: 195, .. })
            )
    );
}

#[test]
fn move_reveal_draw_recycle_and_trash_outcomes_use_actual_top_card() {
    for (source, card, expected) in [
        (392, 3, RevealOutcome::Draw),
        (392, 70, RevealOutcome::Recycle),
        (393, 71, RevealOutcome::Draw),
        (393, 92, RevealOutcome::Recycle),
        (394, 96, RevealOutcome::Draw),
        (394, 7, RevealOutcome::Trash),
    ] {
        let mut g = position();
        g.players[0].deck = vec![69, card];
        let target = move_to_friendly_field(&mut g, source);
        resolve(&mut g);
        assert!(
            matches!(g.decision, Some(Decision::RevealedCard { source_card_id, card_id, outcome, .. }) if source_card_id == source && card_id == card && outcome == expected)
        );
        assert_eq!(
            g.trigger_state.draws[0],
            u64::from(expected == RevealOutcome::Draw)
        );
        if expected == RevealOutcome::Draw {
            assert_eq!(g.players[0].hand, [card]);
        }
        if expected == RevealOutcome::Recycle {
            assert_eq!(g.players[0].deck, [card, 69]);
        }
        if expected == RevealOutcome::Trash {
            assert!(g.players[0].discard.contains(&card));
        }
        assert_eq!(
            g.unit_mut(target).unwrap().temporary_buff,
            if expected == RevealOutcome::Trash {
                2
            } else {
                0
            }
        );
        reject(&mut g, Action::Choose { accept: false });
        acknowledge(&mut g);
    }
}

#[test]
fn move_reveal_empty_deck_does_not_enter_pakaa_otherwise_branch() {
    for source in [392, 393, 394] {
        let mut g = position();
        g.players[0].deck.clear();
        let scores = [g.players[0].score, g.players[1].score];
        let target = move_to_friendly_field(&mut g, source);
        resolve(&mut g);
        assert!(g.decision.is_none());
        assert_eq!(g.unit_mut(target).unwrap().temporary_buff, 0);
        assert_eq!(g.trigger_state.draws[0], 0);
        assert!(g.players[0].hand.is_empty());
        assert!(g.players[0].discard.is_empty());
        assert_eq!([g.players[0].score, g.players[1].score], scores);
    }
}

#[test]
fn raving_witness_does_not_apply_board_auras_to_revealed_card_might() {
    let mut g = position();
    unit(&mut g, 0, 320, None);
    g.players[0].deck = vec![92]; // Printed 4, despite Garen on the board.
    move_to_friendly_field(&mut g, 393);
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::RevealedCard {
            card_id: 92,
            outcome: RevealOutcome::Recycle,
            ..
        })
    ));
    assert_eq!(g.trigger_state.draws[0], 0);
    acknowledge(&mut g);
}

#[test]
fn successive_move_triggers_inspect_current_deck_when_each_resolves() {
    let mut g = position();
    g.players[0].deck = vec![70, 3];
    let target = move_to_friendly_field(&mut g, 392);
    if g.priority() != 0 {
        g.apply(Action::Pass).unwrap();
    }
    cast(&mut g, 9, vec![target]); // A second actual Move, battlefield to base.
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::RevealedCard {
            card_id: 3,
            outcome: RevealOutcome::Draw,
            ..
        })
    ));
    acknowledge(&mut g);
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::RevealedCard {
            card_id: 70,
            outcome: RevealOutcome::Recycle,
            ..
        })
    ));
    acknowledge(&mut g);
    assert_eq!(g.trigger_state.draws[0], 1);
    assert_eq!(g.players[0].hand, [3]);
    assert_eq!(g.players[0].deck, [70]);
}

#[test]
fn pakaa_departure_in_response_preserves_reveal_and_trash_without_modifier() {
    let mut g = position();
    let enemy = unit(&mut g, 1, 70, None);
    g.players[0].deck = vec![7];
    let source = move_to_friendly_field(&mut g, 394);
    if g.priority() == 0 {
        g.apply(Action::Pass).unwrap();
    }
    cast(&mut g, 16, vec![enemy, source]);
    resolve(&mut g);
    assert!(g.unit_mut(source).is_none());
    assert!(g.players[0].hand.contains(&394));
    assert!(g.players[0].discard.contains(&7));
    assert!(matches!(
        g.decision,
        Some(Decision::RevealedCard {
            card_id: 7,
            outcome: RevealOutcome::Trash,
            ..
        })
    ));
    acknowledge(&mut g);
}

#[test]
fn countered_double_trouble_never_looks_draws_or_recycles() {
    let mut g = position();
    g.players[0].deck = vec![70, 96, 145];
    let deck = g.players[0].deck.clone();
    cast(&mut g, 389, vec![]);
    let chain_id = g.chain.last().unwrap().id;
    if g.priority() == 0 {
        g.apply(Action::Pass).unwrap();
    }
    cast(&mut g, 86, vec![Target::Spell { chain_id }]);
    resolve(&mut g);
    assert!(g.decision.is_none());
    assert_eq!(g.players[0].deck, deck);
    assert_eq!(g.trigger_state.draws[0], 0);
    assert!(g.players[0].discard.contains(&389));
    assert_eq!(g.trigger_state.completed_cards[0], 0);
}

#[test]
fn source_entry_ready_and_recall_are_not_move_events() {
    for source in [392, 393, 394] {
        let mut g = position();
        unit(&mut g, 0, 96, Some(0));
        g.players[0].deck = vec![96];
        cast(&mut g, source, vec![]);
        resolve(&mut g);
        assert!(g.decision.is_none());
        assert_eq!(g.players[0].deck, [96]);
        let unit_id = g.players[0]
            .units
            .iter()
            .find(|u| u.card_id == source)
            .unwrap()
            .id;
        let target = Target::Unit { player: 0, unit_id };
        cast(&mut g, 216, vec![target]);
        resolve(&mut g);
        assert!(g.decision.is_none());
        assert_eq!(g.players[0].deck, [96]);
        g.apply(Action::Move {
            unit_id,
            lane: Some(0),
        })
        .unwrap();
        resolve(&mut g);
        assert!(matches!(g.decision, Some(Decision::RevealedCard { .. })));
        acknowledge(&mut g);
        let hourglass = gear(&mut g, 0, 21);
        g.players[0].deck = vec![70];
        let draws = g.trigger_state.draws[0];
        cast(&mut g, 294, vec![target]);
        resolve(&mut g);
        if matches!(g.decision, Some(Decision::DeathReplacement { .. })) {
            g.apply(Action::ChooseDeathReplacement {
                unit_id,
                gear_id: Some(hourglass),
            })
            .unwrap();
        }
        resolve(&mut g);
        assert!(g.decision.is_none());
        assert_eq!(g.unit_ref(target).unwrap().lane, None);
        assert_eq!(g.players[0].deck, [70]);
        assert_eq!(g.trigger_state.draws[0], draws);
        assert!(!g.players[0].gear.iter().any(|item| item.id == hourglass));
    }
}

#[test]
fn revealed_inspection_keeps_cleanup_paused_until_parent_acknowledgement() {
    let mut g = position();
    let ally = unit(&mut g, 0, 96, None);
    look_from_play(&mut g, 389, &[96]);
    // Controlled resolving-state fixture: the shared continuation must not
    // run Cleanup at either private selection or public inspection boundary.
    let might = g.unit_might(g.unit_ref(ally).unwrap()).unwrap();
    g.unit_mut(ally).unwrap().damage = might;
    choose_draw(&mut g, Some(0));
    assert!(g.unit_ref(ally).is_some());
    assert!(g.chain.iter().any(|item| item.card_id == 389));
    assert!(g.resolution_progress().is_some());
    acknowledge(&mut g);
    assert!(g.unit_ref(ally).is_none());
    assert!(g.resolution_progress().is_none());
}

#[test]
fn pakaa_capacity_failure_rolls_back_reveal_trash_and_source_modifier() {
    let mut g = position();
    g.players[0].deck = vec![7];
    let source = move_to_friendly_field(&mut g, 394);
    // A bounded numeric-capacity fixture, not a naturally accumulated bonus.
    let printed = i64::from(g.catalog[394].might);
    g.unit_mut(source).unwrap().temporary_buff = i64::MAX - printed;
    for _ in 0..6 {
        let before = format!("{g:?}");
        match g.apply(Action::Pass) {
            Ok(()) => assert!(g.decision.is_none()),
            Err(error) => {
                assert!(error.starts_with("engine_capacity:"), "{error}");
                assert_eq!(format!("{g:?}"), before);
                assert_eq!(g.players[0].deck, [7]);
                assert!(!g.players[0].discard.contains(&7));
                return;
            }
        }
    }
    panic!("Pakaa's successful +2 should exceed the fixture's numeric capacity");
}

#[test]
fn private_remainder_does_not_leak_to_public_reveal_log() {
    let mut g = position();
    look_from_play(&mut g, 389, &[96, 70, 145]);
    let log_start = g.log.len();
    choose_draw(&mut g, Some(0));
    let public = g.log[log_start..].join("\n");
    assert!(public.contains(g.catalog[96].name));
    assert!(!public.contains(g.catalog[70].name));
    assert!(!public.contains(g.catalog[145].name));
    acknowledge(&mut g);
}
