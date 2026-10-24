//! CR354.3/355.5/355.10/358.5: targeted trash instructions and pending real cards.
use local_rift::game::{
    Action, ChainAbility, Decision, Difficulty, Game, InstructedPlayKind, Target, Unit,
};
use local_rift::gear::GearState;

fn position() -> Game {
    let mut g = Game::new(3, 3, 3543, Difficulty::Tactical);
    g.start_match().unwrap();
    g.battlefields = [0, 0];
    g.battlefield_owner = [None; 2];
    for p in &mut g.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.discard.clear();
        p.runes = [6; 6];
        p.exhausted_runes = [0; 6];
        p.energy_pool = 0;
        p.energy = 36;
        p.power_pool = [0; 6];
        p.any_power = 0;
    }
    g
}
fn unit(id: u32, card_id: usize, lane: Option<usize>) -> Unit {
    Unit {
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
    }
}
fn cast(g: &mut Game, id: usize, targets: Vec<Target>) {
    let p = g.priority();
    let hand_index = g.players[p].hand.len();
    g.players[p].hand.push(id);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
}
fn resolve(g: &mut Game) {
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn finalize(targets: Vec<Target>) -> Action {
    Action::FinalizePendingPlay {
        lane: None,
        targets,
        repeat_targets: vec![],
        repeat: false,
        accelerate: false,
        bonus: false,
    }
}
fn fizz_target(g: &mut Game, index: usize) {
    cast(g, 36, vec![]);
    assert!(matches!(
        g.decision,
        Some(Decision::OptionalPlay { card_id: 36, .. })
    ));
    g.apply(Action::Choose { accept: true }).unwrap();
    assert!(matches!(
        g.decision,
        Some(Decision::TrashTarget {
            source_card_id: 36,
            ..
        })
    ));
    g.apply(Action::SelectCards {
        indices: vec![index],
    })
    .unwrap();
}

#[test]
fn fizz_selects_trash_before_responses_then_finalizes_selected_pending_spell() {
    let mut g = position();
    g.players[0].discard = vec![7, 67];
    fizz_target(&mut g, 0);
    assert!(matches!(
        g.chain.last().unwrap().ability,
        Some(ChainAbility::PlayChosen {
            trash_index: 0,
            card_id: 7,
            kind: InstructedPlayKind::Fizz
        })
    ));
    assert!(g.pending_cards().is_empty());
    assert_eq!(g.players[0].discard, vec![7, 67]);
    resolve(&mut g);
    assert!(g.chain.is_empty());
    assert_eq!(g.players[0].discard, vec![67]);
    assert_eq!(g.pending_cards()[0].card_id, 7);
    assert!(matches!(
        g.decision,
        Some(Decision::PendingPlay {
            kind: InstructedPlayKind::Fizz,
            ..
        })
    ));
    assert!(!g.can_decline_trash_play().unwrap());
    assert!(g.apply(Action::Choose { accept: false }).is_err());
    let fizz = g.players[0].units[0].id;
    let energy = g.players[0].energy;
    g.apply(finalize(vec![Target::Unit {
        player: 0,
        unit_id: fizz,
    }]))
    .unwrap();
    assert_eq!(g.players[0].energy, energy);
    assert!(g.pending_cards().is_empty());
    assert!(g.chain.last().unwrap().recycle_after);
    resolve(&mut g);
    assert_eq!(g.players[0].deck.first(), Some(&7));
}

#[test]
fn targeted_trash_object_is_not_replaced_after_response_burnout() {
    let mut g = position();
    g.players[0].discard = vec![7, 7, 69, 70];
    fizz_target(&mut g, 0);
    g.players[0].deck.clear();
    cast(&mut g, 78, vec![]);
    resolve(&mut g);
    assert!(matches!(
        g.chain.last().unwrap().ability,
        Some(ChainAbility::PlayChosen {
            trash_index: usize::MAX,
            ..
        })
    ));
    // Put an identical card at the former index; it is a different object.
    g.players[0].discard.insert(0, 7);
    resolve(&mut g);
    assert!(g.pending_cards().is_empty());
    assert!(g.decision.is_none());
    assert_eq!(g.players[0].discard[0], 7);
}

#[test]
fn fizz_can_target_an_unpayable_spell_but_failed_finalization_returns_it() {
    let mut g = position();
    g.players[0].discard = vec![67];
    fizz_target(&mut g, 0);
    g.players[0].runes = [0; 6];
    g.players[0].exhausted_runes = [0; 6];
    g.players[0].energy = 0;
    g.players[0].units.push(unit(100, 72, Some(0)));
    g.legend[0].ready = false;
    resolve(&mut g);
    assert_eq!(g.pending_cards().len(), 1);
    assert!(g.can_decline_trash_play().unwrap());
    assert!(
        g.apply(finalize(vec![Target::Unit {
            player: 0,
            unit_id: 100
        }]))
        .is_err()
    );
    assert_eq!(g.pending_cards().len(), 1);
    g.apply(Action::Choose { accept: false }).unwrap();
    assert!(g.pending_cards().is_empty());
    assert_eq!(g.players[0].discard, vec![67]);
}

#[test]
fn pending_spell_can_generate_matching_power_without_losing_its_identity() {
    let mut g = position();
    g.players[0].discard = vec![67];
    fizz_target(&mut g, 0);
    g.players[0].runes = [0; 6];
    g.players[0].exhausted_runes = [0; 6];
    g.players[0].energy = 0;
    g.players[0].units.push(unit(100, 72, Some(0)));
    g.players[0].gear.push(GearState {
        id: 101,
        card_id: 127,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    resolve(&mut g);
    let pending = g.pending_cards();
    assert!(!g.can_decline_trash_play().unwrap());
    g.apply(Action::ActivateGear { gear_id: 101 }).unwrap();
    assert_eq!(g.pending_cards(), pending);
    g.apply(finalize(vec![Target::Unit {
        player: 0,
        unit_id: 100,
    }]))
    .unwrap();
    assert!(g.chain.last().unwrap().recycle_after);
}

#[test]
fn matriarch_child_moves_to_pending_before_optional_cost_and_board_entry() {
    let mut g = position();
    g.players[0].discard = vec![96];
    g.decision = Some(Decision::ReviveUnit { player: 0 });
    g.apply(Action::PlayFromTrash {
        trash_index: 0,
        lane: None,
        target: None,
    })
    .unwrap();
    assert!(g.players[0].units.is_empty());
    resolve(&mut g);
    assert!(g.players[0].discard.is_empty());
    assert!(g.players[0].units.is_empty());
    assert_eq!(g.pending_cards()[0].card_id, 96);
    let before = g.players[0].energy;
    g.apply(Action::FinalizePendingPlay {
        lane: None,
        targets: vec![],
        repeat_targets: vec![],
        repeat: false,
        accelerate: true,
        bonus: false,
    })
    .unwrap();
    assert!(g.players[0].units[0].ready);
    assert_eq!(before - g.players[0].energy, 1);
    assert!(g.pending_cards().is_empty());
}

#[test]
fn last_rites_selects_without_paying_then_pending_unit_pays_its_cost() {
    let mut g = position();
    g.players[0].discard = vec![103];
    g.decision = Some(Decision::OptionalPlay {
        player: 0,
        card_id: 38,
        gear_id: Some(101),
    });
    g.apply(Action::Choose { accept: true }).unwrap();
    let before = g.players[0].energy;
    g.apply(Action::SelectCards { indices: vec![0] }).unwrap();
    assert_eq!(g.players[0].energy, before);
    resolve(&mut g);
    assert!(g.players[0].discard.is_empty());
    assert!(g.players[0].units.is_empty());
    g.apply(finalize(vec![])).unwrap();
    assert_eq!(before - g.players[0].energy, 3);
    assert!(g.players[0].units[0].ready);
}

#[test]
fn pending_cancellation_checks_payable_explicit_targets_not_only_preferred_enemy() {
    let mut g = position();
    g.players[0].discard = vec![74];
    fizz_target(&mut g, 0);
    g.players[0].units.push(unit(100, 72, Some(0)));
    g.players[1].units.push(unit(101, 0, Some(1)));
    g.players[0].runes = [0; 6];
    g.players[0].exhausted_runes = [0; 6];
    g.players[0].energy = 0;
    g.legend[0].ready = false;
    resolve(&mut g);
    assert!(!g.can_decline_trash_play().unwrap());
    let legal = g.legal_actions().unwrap();
    assert!(legal.contains(&finalize(vec![Target::Unit {
        player: 0,
        unit_id: 100
    }])));
    assert!(g.apply(Action::Choose { accept: false }).is_err());
    g.apply(finalize(vec![Target::Unit {
        player: 0,
        unit_id: 100,
    }]))
    .unwrap();
}

#[test]
fn optional_zero_target_pending_spell_is_a_legal_mandatory_completion() {
    let mut g = position();
    g.players[0].discard = vec![9];
    fizz_target(&mut g, 0);
    resolve(&mut g);
    assert!(!g.can_decline_trash_play().unwrap());
    assert!(g.legal_actions().unwrap().contains(&finalize(vec![])));
    g.apply(finalize(vec![])).unwrap();
}

#[test]
fn cancelling_failed_finalization_restores_granted_flow_and_original_trash_position() {
    let mut g = position();
    g.players[0].discard = vec![69, 67, 70];
    g.decision = Some(Decision::GrantFlow { player: 0 });
    g.apply(Action::SelectCards { indices: vec![1] }).unwrap();
    resolve(&mut g);
    let flow = g.flow_costs(0, 1);
    assert!(!flow.is_empty());
    fizz_target(&mut g, 1);
    g.players[0].runes = [0; 6];
    g.players[0].exhausted_runes = [0; 6];
    g.players[0].energy = 0;
    g.legend[0].ready = false;
    resolve(&mut g);
    assert_eq!(g.players[0].discard, vec![69, 70]);
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(g.players[0].discard, vec![69, 67, 70]);
    assert_eq!(g.flow_costs(0, 1), flow);
}
