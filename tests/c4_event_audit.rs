//! Independent event/provenance audit. Initial board positions are fixtures;
//! every subsequent rules transition uses the public action API.
use local_rift::activated::AbilitySource;
use local_rift::game::{Action, ChainAbility, Decision, Difficulty, Game, Target, Unit};
use local_rift::gear::GearState;
use local_rift::play::PlayGrantKind;

fn position() -> Game {
    let mut g = Game::new_for_agents(3, 3, 64417, Difficulty::Tactical);
    g.start_match().unwrap();
    g.battlefields = [3, 3];
    g.battlefield_owner = [None, None];
    for p in 0..2 {
        g.legend[p].ready = false;
        let s = &mut g.players[p];
        s.hand.clear();
        s.units.clear();
        s.gear.clear();
        s.hidden.clear();
        s.discard.clear();
        s.champion = None;
        s.runes = [0; 6];
        s.exhausted_runes = [0; 6];
        s.energy_pool = 50;
        s.energy = 50;
        s.power_pool = [20; 6];
        s.any_power = 0;
    }
    g
}
fn unit(g: &mut Game, p: usize, id: u32, card: usize, lane: Option<usize>) -> Target {
    g.players[p].units.push(Unit {
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
    if let Some(l) = lane {
        g.battlefield_owner[l] = Some(p);
    }
    Target::Unit {
        player: p,
        unit_id: id,
    }
}
fn gear(g: &mut Game, id: u32, card: usize) -> AbilitySource {
    g.players[0].gear.push(GearState {
        id,
        card_id: card,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    AbilitySource::Gear {
        player: 0,
        gear_id: id,
    }
}
fn cast(g: &mut Game, p: usize, card: usize, lane: Option<usize>, targets: Vec<Target>) {
    assert_eq!(g.priority(), p);
    let i = g.players[p].hand.len();
    g.players[p].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index: i,
        lane,
        targets,
    })
    .unwrap();
}
fn activate(g: &mut Game, source: AbilitySource) {
    g.apply(Action::ActivateAbility {
        source,
        ability_index: 0,
        targets: vec![],
    })
    .unwrap();
}
fn resolve_one(g: &mut Game) {
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn settle(g: &mut Game) {
    for _ in 0..150 {
        let a = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::TokenPlacement { .. }) => Action::PlaceToken { lane: None },
            Some(Decision::TokenPlay { .. }) => Action::FinalizeTokenPlay {
                lane: None,
                accelerate: false,
            },
            Some(other) => panic!("Unexpected decision in audit fixture: {other:?}"),
            None if g.in_window() => Action::Pass,
            None => return,
        };
        g.apply(a).unwrap();
    }
    panic!("Audit fixture did not settle");
}
fn heron_grants(g: &Game) -> usize {
    g.play_grants
        .iter()
        .filter(|v| {
            matches!(
                v.kind,
                PlayGrantKind::DiscountNextCard {
                    energy: 2,
                    power: 2
                }
            )
        })
        .count()
}
fn source_abilities(g: &Game, card: usize) -> usize {
    g.chain
        .iter()
        .filter(|v| v.card_id == card && matches!(v.ability, Some(ChainAbility::CardEffect { .. })))
        .count()
}

#[test]
fn heron_counts_first_completed_card_after_an_earlier_card_is_countered() {
    // CR420.4.a and426.1.b: countered cards do not count as played for triggers.
    let mut g = position();
    unit(&mut g, 0, 8000, 258, Some(0));
    cast(&mut g, 0, 263, None, vec![]);
    let id = g.chain.last().unwrap().id;
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 1, 6, None, vec![Target::Spell { chain_id: id }]);
    settle(&mut g);
    assert_eq!(heron_grants(&g), 0);
    cast(&mut g, 0, 123, None, vec![]);
    assert_eq!(source_abilities(&g, 258), 1);
    settle(&mut g);
    assert_eq!(heron_grants(&g), 1);
}

#[test]
fn heron_first_completed_reaction_does_not_retrigger_for_older_pending_spell() {
    // The ordinal is part of the triggered play condition, not a Legion check.
    let mut g = position();
    let heron = unit(&mut g, 0, 8001, 258, Some(0));
    cast(&mut g, 0, 263, None, vec![]);
    cast(&mut g, 0, 8, None, vec![heron]);
    resolve_one(&mut g);
    assert_eq!(source_abilities(&g, 258), 1);
    assert_eq!(heron_grants(&g), 0, "Heron ability is still respondable");
    settle(&mut g);
    assert_eq!(
        heron_grants(&g),
        1,
        "Already queued Confront cannot consume a new next-play grant"
    );
}

#[test]
fn heron_entering_after_first_completed_card_cannot_restart_that_ordinal() {
    let mut g = position();
    unit(&mut g, 0, 8002, 69, Some(0));
    cast(&mut g, 0, 123, None, vec![]);
    cast(&mut g, 0, 258, Some(0), vec![]);
    settle(&mut g);
    assert_eq!(heron_grants(&g), 0);
    cast(&mut g, 0, 123, None, vec![]);
    settle(&mut g);
    assert_eq!(heron_grants(&g), 0);
}

#[test]
fn next_card_discount_is_not_consumed_by_instructed_unit_token_plays() {
    // Tokens are played units but not cards. Next-card Heron cannot attach to one.
    let mut g = position();
    unit(&mut g, 0, 8003, 258, Some(0));
    let armory = gear(&mut g, 8004, 235);
    cast(&mut g, 0, 123, None, vec![]);
    settle(&mut g);
    assert_eq!(heron_grants(&g), 1);
    activate(&mut g, armory);
    resolve_one(&mut g);
    assert_eq!(
        heron_grants(&g),
        1,
        "Queueing tokens must preserve a card-only grant"
    );
    settle(&mut g);
    assert_eq!(g.players[0].units.iter().filter(|v| v.token).count(), 3);
    assert_eq!(heron_grants(&g), 1);
    let i = g.players[0].hand.len();
    g.players[0].hand.push(69);
    let a = Action::PlayCardWithTargets {
        hand_index: i,
        lane: None,
        targets: vec![],
    };
    assert_eq!(g.cost_plan(&a).unwrap().energy, 5);
    g.apply(a).unwrap();
    assert_eq!(heron_grants(&g), 0);
}

#[test]
fn separate_nami_hold_grants_produce_separate_delayed_ready_buff_triggers() {
    let mut g = position();
    unit(&mut g, 0, 8005, 261, Some(0));
    unit(&mut g, 0, 8006, 261, Some(0));
    g.players[0].runes[0] = 6;
    g.players[0].energy = 56;
    g.apply(Action::EndTurn).unwrap();
    g.apply(Action::EndTurn).unwrap();
    settle(&mut g);
    assert_eq!(
        g.play_grants
            .iter()
            .filter(|v| v.kind == PlayGrantKind::ReadyBuffNextUnit)
            .count(),
        2
    );
    cast(&mut g, 0, 157, None, vec![]);
    if let Some(Decision::OrderTriggers { options, .. }) = &g.decision {
        let id = options[0].id;
        g.apply(Action::OrderTrigger { trigger_id: id }).unwrap();
    }
    assert_eq!(
        source_abilities(&g, 261),
        2,
        "Each independent next-unit trigger must survive"
    );
    settle(&mut g);
    let feline = g.players[0]
        .units
        .iter()
        .find(|v| v.card_id == 157)
        .unwrap();
    assert!(feline.ready);
    assert_eq!(feline.buff, 1);
    assert_eq!(
        feline.temporary_buff, 2,
        "The first actual Ready triggers Feline; ready entry would not"
    );
}

#[test]
fn magma_wurm_ready_entry_excludes_itself_and_never_emits_a_ready_event() {
    let mut g = position();
    cast(&mut g, 0, 264, None, vec![]);
    assert!(!g.players[0].units[0].ready);
    cast(&mut g, 0, 264, None, vec![]);
    assert!(
        g.players[0].units[1].ready,
        "A different Wurm grants ready entry"
    );
    cast(&mut g, 0, 157, None, vec![]);
    settle(&mut g);
    let feline = g.players[0].units.last().unwrap();
    assert!(feline.ready);
    assert_eq!(feline.temporary_buff, 0);
}

#[test]
fn failed_payment_undo_restores_tome_grant_and_the_original_announced_spell() {
    let mut g = position();
    let tome = gear(&mut g, 8007, 255);
    let target = unit(&mut g, 1, 8008, 69, Some(0));
    activate(&mut g, tome);
    settle(&mut g);
    g.players[0].hand.push(260);
    g.players[0].energy = 0;
    g.players[0].energy_pool = 0;
    let before = format!("{g:?}");
    g.apply(Action::BeginPayment {
        action: Box::new(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        }),
    })
    .unwrap();
    assert!(g.play_grants.is_empty());
    assert!(g.apply(Action::CompletePayment).is_err());
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn failed_instructed_play_restores_a_reserved_next_spell_grant() {
    // CR358.5 undoes an unfinalizable play, including a consumed next-play effect.
    let mut g = position();
    let tome = gear(&mut g, 8010, 255);
    let target = unit(&mut g, 1, 8011, 69, Some(0));
    g.players[0].discard.push(67);
    g.players[0].power_pool = [0; 6];
    g.players[0].power_pool[4] = 1; // Fizz can be played; Void Seeker's Fury cannot.
    activate(&mut g, tome);
    settle(&mut g);
    assert_eq!(g.play_grants.len(), 1);
    cast(&mut g, 0, 36, None, vec![]);
    g.apply(Action::Choose { accept: true }).unwrap();
    g.apply(Action::SelectCards { indices: vec![0] }).unwrap();
    resolve_one(&mut g);
    assert!(matches!(g.decision, Some(Decision::PendingPlay { .. })));
    assert!(
        g.play_grants.is_empty(),
        "The grant is reserved by the pending spell"
    );
    assert!(g.can_decline_trash_play());
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(g.players[0].discard, vec![67]);
    assert_eq!(
        g.play_grants.len(),
        1,
        "Undo must restore the delayed grant as well as the card"
    );
    cast(&mut g, 0, 260, None, vec![target]);
    assert_eq!(g.chain.last().unwrap().bonus_damage, 1);
}

#[test]
fn live_annie_bonus_disappears_after_a_real_reaction_but_captured_tome_bonus_remains() {
    let mut g = position();
    let tome = gear(&mut g, 8012, 255);
    let annie = unit(&mut g, 0, 8013, 259, None);
    let target = unit(&mut g, 1, 8014, 69, Some(0));
    let opposing = unit(&mut g, 1, 8015, 69, None);
    activate(&mut g, tome);
    settle(&mut g);
    cast(&mut g, 0, 260, None, vec![target]);
    assert_eq!(g.chain.last().unwrap().bonus_damage, 1);
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 1, 16, None, vec![opposing, annie]); // Star-Crossed is a printed Reaction.
    settle(&mut g);
    assert!(!g.players[0].units.iter().any(|u| u.id == 8013));
    assert_eq!(
        g.players[1]
            .units
            .iter()
            .find(|u| u.id == 8014)
            .unwrap()
            .damage,
        4,
        "Curse2 + captured Tome1 + target-local VoidGate1; no departed Annie"
    );
}
