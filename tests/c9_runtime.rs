//! Public-action regressions for simultaneous Kill, Channel, Hunt and live rules.
use local_rift::activated::AbilitySource;
use local_rift::cards::Domain;
use local_rift::game::play::{ExecutionChoices, PlayChoices, RepeatSelection};
use local_rift::game::{
    Action, ChainAbility, ChainSource, Decision, Difficulty, Game, Target, Unit,
};
use local_rift::gear::GearState;

fn game() -> Game {
    let mut g = Game::new_for_agents(3, 3, 87331, Difficulty::Tactical);
    g.start_match().unwrap();
    g.battlefields = [2, 2]; // Minefield has no passive damage modifier.
    for p in 0..2 {
        g.legend[p].ready = false;
        let state = &mut g.players[p];
        state.hand.clear();
        state.units.clear();
        state.gear.clear();
        state.hidden.clear();
        state.discard.clear();
        state.champion = None;
        state.runes = [0; 6];
        state.exhausted_runes = [0; 6];
        state.energy_pool = 60;
        state.energy = 60;
        state.power_pool = [20; 6];
        state.any_power = 0;
    }
    g.battlefield_owner = [None; 2];
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
    if let Some(lane) = lane {
        g.battlefield_owner[lane] = Some(p);
    }
    Target::Unit {
        player: p,
        unit_id: id,
    }
}
fn u(g: &Game, t: Target) -> &Unit {
    let Target::Unit { player, unit_id } = t else {
        panic!()
    };
    g.players[player]
        .units
        .iter()
        .find(|u| u.id == unit_id)
        .unwrap()
}
fn um(g: &mut Game, t: Target) -> &mut Unit {
    let Target::Unit { player, unit_id } = t else {
        panic!()
    };
    g.players[player]
        .units
        .iter_mut()
        .find(|u| u.id == unit_id)
        .unwrap()
}
fn gear(g: &mut Game, p: usize, id: u32, card_id: usize, attached_to: Option<u32>) {
    g.players[p].gear.push(GearState {
        id,
        card_id,
        attached_to,
        ready: true,
        temporary: false,
        token: false,
    });
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
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn drain(g: &mut Game) {
    for _ in 0..100 {
        let action = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::Predict { .. }) => Action::Choose { accept: false },
            Some(Decision::PredictCards {
                choice_id, cards, ..
            }) => Action::ResolvePredict {
                choice_id: *choice_id,
                recycle: vec![],
                top_order: (0..cards.len()).collect(),
            },
            Some(Decision::TokenPlacement { .. }) => Action::PlaceToken { lane: None },
            Some(Decision::TokenPlay { .. }) => Action::FinalizeTokenPlay {
                lane: None,
                accelerate: false,
            },
            Some(Decision::BattlefieldOrder { lanes, .. }) => {
                Action::ChooseBattlefield { lane: lanes[0] }
            }
            Some(Decision::TriggerTargets { .. }) => g
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|a| matches!(a, Action::ChooseTriggerTargets { .. }))
                .unwrap(),
            Some(other) => panic!("unexpected choice {other:?}"),
            None if g.chain.is_empty() => return,
            None => Action::Pass,
        };
        g.apply(action).unwrap();
    }
    panic!("unsettled");
}
fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action).is_err());
    assert_eq!(format!("{g:?}"), before);
}
fn portal(g: &mut Game) {
    let id = 7000 + g.players[0].gear.len() as u32;
    gear(g, 0, id, 256, None);
    g.apply(Action::ActivateAbility {
        source: AbilitySource::Gear {
            player: 0,
            gear_id: id,
        },
        ability_index: 0,
        targets: vec![],
    })
    .unwrap();
    resolve(g);
}
fn repeated(g: &mut Game, card: usize, base: Vec<Target>, extra: Vec<Vec<Target>>) {
    for _ in &extra {
        portal(g);
    }
    let hand_index = g.players[0].hand.len();
    g.players[0].hand.push(card);
    let action = Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets: base,
    };
    let view = g.play_choices(&action).unwrap();
    assert_eq!(view.repeats.len(), extra.len());
    let repeats = view
        .repeats
        .iter()
        .zip(extra)
        .map(|(option, targets)| RepeatSelection {
            option_id: option.id,
            choices: ExecutionChoices {
                targets,
                mode: None,
            },
        })
        .collect();
    g.apply(Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats,
            ..Default::default()
        },
    })
    .unwrap();
}
#[test]
fn ruination_kills_both_bases_and_fields_without_choosing_or_damage() {
    let mut g = game();
    for (p, id, card, lane) in [
        (0, 1000, 271, None),
        (0, 1001, 69, Some(0)),
        (1, 1002, 0, None),
        (1, 1003, 31, Some(1)),
    ] {
        let t = unit(&mut g, p, id, card, lane);
        um(&mut g, t).protected = true;
    }
    cast(&mut g, 308, vec![]);
    resolve(&mut g);
    assert!(g.players.iter().all(|p| p.units.is_empty()));
    assert_eq!(
        g.players[0].discard.iter().filter(|&&id| id == 308).count(),
        1
    );
    assert!(g.chain.is_empty());
    let mut empty = game();
    cast(&mut empty, 308, vec![]);
    resolve(&mut empty);
    assert_eq!(empty.players[0].discard, vec![308]);
}
#[test]
fn ruination_replacement_choices_preserve_parent_and_one_original_death_set() {
    let mut g = game();
    let a = unit(&mut g, 1, 1000, 69, Some(0));
    let b = unit(&mut g, 1, 1001, 70, None);
    gear(&mut g, 1, 2000, 21, None);
    cast(&mut g, 308, vec![]);
    resolve(&mut g);
    assert!(
        matches!(g.decision,Some(Decision::DeathReplacement{ref options,..}) if options.len()==2)
    );
    assert!(g.chain.iter().any(|i| i.card_id == 308));
    assert!(!g.players[0].discard.contains(&308));
    assert_eq!(u(&g, a).damage, 0);
    assert_eq!(u(&g, b).damage, 0);
    g.apply(Action::ChooseDeathReplacement {
        unit_id: 1000,
        gear_id: Some(2000),
    })
    .unwrap();
    assert_eq!(u(&g, a).lane, None);
    assert!(!u(&g, a).ready);
    assert!(!g.players[1].units.iter().any(|u| u.id == 1001));
    assert!(g.players[0].discard.contains(&308));
}
#[test]
fn repeated_ruination_waits_for_replacements_then_kills_the_recalled_survivor() {
    let mut g = game();
    unit(&mut g, 1, 1000, 69, Some(0));
    gear(&mut g, 1, 2000, 21, None);
    gear(&mut g, 1, 2001, 10, Some(1000));
    repeated(&mut g, 308, vec![], vec![vec![], vec![]]);
    resolve(&mut g);
    assert!(g.resolution_progress().is_some());
    g.apply(Action::ChooseDeathReplacement {
        unit_id: 1000,
        gear_id: Some(2000),
    })
    .unwrap();
    assert!(g.players[1].units.is_empty());
    assert!(g.players[1].discard.contains(&69));
    assert_eq!(
        g.players[0].discard.iter().filter(|&&x| x == 308).count(),
        1
    );
    assert!(g.resolution_progress().is_none());
}
#[test]
fn ruination_suppresses_dying_observers_but_keeps_deathknell() {
    let mut g = game();
    unit(&mut g, 0, 1000, 175, None); // Viktor observes friendly deaths only while present.
    unit(&mut g, 0, 1001, 68, None); // Vanguard Sergeant draws on death.
    unit(&mut g, 1, 1002, 69, None);
    cast(&mut g, 308, vec![]);
    resolve(&mut g);
    drain(&mut g);
    assert!(g.players[0].units.is_empty());
    assert_eq!(g.players[0].hand.len(), 1);
}
#[test]
fn channel_fallback_counts_actual_exhausted_runes_including_partial_success() {
    for card in [309, 310] {
        for available in 0..=3 {
            let mut g = game();
            g.players[0].rune_deck = vec![Domain::Body; available];
            cast(&mut g, card, vec![]);
            let energy = g.players[0].energy;
            resolve(&mut g);
            let wanted = if card == 309 { 1 } else { 2 };
            let actual = available.min(wanted);
            assert_eq!(g.players[0].total_runes(), actual as u8);
            assert_eq!(
                g.players[0].exhausted_runes.iter().sum::<u8>(),
                actual as u8
            );
            assert_eq!(g.players[0].energy, energy);
            assert_eq!(g.players[0].hand.len(), usize::from(actual < wanted));
        }
    }
}
#[test]
fn repeated_catalyst_fallback_burnout_never_draws_its_resolving_parent() {
    let mut g = game();
    g.players[0].rune_deck = vec![Domain::Body];
    repeated(&mut g, 310, vec![], vec![vec![]]);
    g.players[0].deck = vec![69];
    g.players[0].discard = vec![70];
    resolve(&mut g);
    assert_eq!(g.players[0].hand.len(), 2);
    assert!(!g.players[0].hand.contains(&310));
    assert_eq!(g.players[0].total_runes(), 1);
    assert_eq!(g.players[0].discard, vec![310]);
}
#[test]
fn countered_channel_spell_has_no_channel_or_fallback() {
    let mut g = game();
    g.players[0].rune_deck.clear();
    cast(&mut g, 310, vec![]);
    g.apply(Action::Pass).unwrap();
    let id = g.chain[0].id;
    cast(&mut g, 86, vec![Target::Spell { chain_id: id }]);
    resolve(&mut g);
    assert_eq!(g.players[0].hand.len(), 0);
    assert_eq!(g.players[0].total_runes(), 0);
}
#[test]
fn global_might_includes_bases_and_anthem_ready_noops_do_not_trigger() {
    let mut g = game();
    let ready = unit(&mut g, 0, 1000, 157, None);
    let tired = unit(&mut g, 0, 1001, 157, None);
    let enemy = unit(&mut g, 1, 1002, 69, None);
    um(&mut g, tired).ready = false;
    cast(&mut g, 311, vec![]);
    resolve(&mut g);
    assert_eq!(u(&g, ready).temporary_buff, 5);
    assert_eq!(u(&g, enemy).temporary_buff, 0);
    cast(&mut g, 312, vec![]);
    resolve(&mut g);
    assert_eq!(u(&g, ready).temporary_buff, 8);
    assert_eq!(u(&g, tired).temporary_buff, 8);
    assert_eq!(g.chain.len(), 1);
    drain(&mut g);
    assert_eq!(u(&g, ready).temporary_buff, 8);
    assert_eq!(u(&g, tired).temporary_buff, 10);
}
#[test]
fn unchecked_power_exhausts_bases_and_queues_observers_before_lethal_packets() {
    let mut g = game();
    let base = unit(&mut g, 0, 1000, 69, None);
    unit(&mut g, 0, 1001, 194, Some(0));
    unit(&mut g, 1, 1002, 69, Some(1));
    cast(&mut g, 313, vec![]);
    resolve(&mut g);
    assert!(!u(&g, base).ready);
    assert_eq!(u(&g, base).damage, 0);
    assert!(g.players[0].discard.contains(&194));
    assert!(g.players[1].discard.contains(&69));
    assert!(g.players[0].discard.contains(&313));
    assert!(g.chain.iter().any(|i| i.card_id == 194));
    drain(&mut g);
    let mut g = game();
    unit(&mut g, 1, 1000, 69, Some(0));
    cast(&mut g, 313, vec![]);
    resolve(&mut g);
    assert!(g.players[1].units.is_empty());
}
fn trigger_targets(g: &mut Game, targets: Vec<Target>) {
    assert!(
        matches!(g.decision, Some(Decision::TriggerTargets { .. })),
        "{:?}",
        g.decision
    );
    g.apply(Action::ChooseTriggerTargets { targets }).unwrap();
}
#[test]
fn dragon_kill_survives_source_departure_and_sandshifter_rechecks_live_bound() {
    let mut g = game();
    let enemy = unit(&mut g, 1, 1000, 69, None);
    cast(&mut g, 314, vec![]);
    trigger_targets(&mut g, vec![enemy]);
    let source = g.players[0]
        .units
        .iter()
        .find(|u| u.card_id == 314)
        .unwrap()
        .id;
    let other = unit(&mut g, 1, 1001, 70, None);
    cast(
        &mut g,
        16,
        vec![
            Target::Unit {
                player: 0,
                unit_id: source,
            },
            other,
        ],
    );
    resolve(&mut g);
    assert!(g.players[0].hand.contains(&314));
    resolve(&mut g);
    assert!(g.players[1].units.is_empty());
    let mut g = game();
    let enemy = unit(&mut g, 1, 1000, 301, None);
    cast(&mut g, 315, vec![]);
    trigger_targets(&mut g, vec![enemy]);
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 7, vec![enemy]);
    resolve(&mut g);
    resolve(&mut g);
    assert_eq!(g.unit_might(u(&g, enemy)).unwrap(), 4);
}
#[test]
fn targeted_kill_trigger_can_be_countered_with_not_so_fast() {
    let mut g = game();
    let enemy = unit(&mut g, 1, 1000, 69, None);
    cast(&mut g, 314, vec![]);
    trigger_targets(&mut g, vec![enemy]);
    g.apply(Action::Pass).unwrap();
    let chain_id = g.chain[0].id;
    cast(&mut g, 12, vec![Target::Spell { chain_id }]);
    resolve(&mut g);
    assert_eq!(u(&g, enemy).damage, 0);
    assert!(g.chain.is_empty());
}
#[test]
fn ruined_rex_death_targets_and_deals_four_after_source_is_gone() {
    let mut g = game();
    let rex = unit(&mut g, 0, 1000, 316, None);
    let enemy = unit(&mut g, 1, 1001, 69, None);
    cast(&mut g, 294, vec![rex]);
    resolve(&mut g);
    trigger_targets(&mut g, vec![enemy]);
    resolve(&mut g);
    assert_eq!(u(&g, enemy).damage, 4);
    assert!(g.players[0].discard.contains(&316));
}
#[test]
fn tracker_optional_distinct_any_targets_and_two_packet_damage() {
    for count in 0..=2 {
        let mut g = game();
        let a = unit(&mut g, 0, 1000, 69, None);
        let b = unit(&mut g, 1, 1001, 69, None);
        cast(&mut g, 333, vec![]);
        reject(
            &mut g,
            Action::ChooseTriggerTargets {
                targets: vec![a, a],
            },
        );
        trigger_targets(&mut g, vec![a, b][..count].to_vec());
        resolve(&mut g);
        assert_eq!(u(&g, a).damage, u64::from(count >= 1));
        assert_eq!(u(&g, b).damage, u64::from(count == 2));
    }
}
#[test]
fn ready_mech_enters_without_ready_event_ordinary_mech_enters_exhausted() {
    for card in [318, 319] {
        let mut g = game();
        let observer = unit(&mut g, 0, 1000, 196, None); // Ready observer must not see initial entry
        let before = u(&g, observer).temporary_buff;
        cast(&mut g, card, vec![]);
        let source = g.players[0]
            .units
            .iter()
            .find(|u| u.card_id == card)
            .unwrap();
        assert_eq!(source.ready, card == 319);
        assert!(!g.players[0].units.iter().any(|u| u.token));
        resolve(&mut g);
        let token = g.players[0].units.iter().find(|u| u.token).unwrap();
        assert_eq!(token.ready, card == 319);
        assert_eq!(g.unit_might(token).unwrap(), 3);
        assert_eq!(token.lane, None);
        assert_eq!(u(&g, observer).temporary_buff, before);
        assert!(g.chain.is_empty());
        assert_eq!(token.temporary_buff, 0);
    }
}
#[test]
fn mech_here_tracks_live_source_and_does_not_fall_back_after_departure() {
    let mut g = game();
    unit(&mut g, 0, 1000, 69, Some(0));
    g.players[0].hand.push(318);
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: Some(0),
        targets: vec![],
    })
    .unwrap();
    let source = g.players[0]
        .units
        .iter()
        .find(|u| u.card_id == 318)
        .unwrap()
        .id;
    cast(
        &mut g,
        9,
        vec![Target::Unit {
            player: 0,
            unit_id: source,
        }],
    );
    resolve(&mut g);
    resolve(&mut g);
    assert_eq!(
        g.players[0].units.iter().find(|u| u.token).unwrap().lane,
        None
    );
    let mut g = game();
    let other = unit(&mut g, 1, 1000, 70, None);
    cast(&mut g, 318, vec![]);
    let source = g.players[0]
        .units
        .iter()
        .find(|u| u.card_id == 318)
        .unwrap()
        .id;
    cast(
        &mut g,
        16,
        vec![
            Target::Unit {
                player: 0,
                unit_id: source,
            },
            other,
        ],
    );
    resolve(&mut g);
    resolve(&mut g);
    assert!(g.players[0].units.is_empty());
}
#[test]
fn buff_counter_live_predicates_ignore_temporary_might_empower_and_token_base() {
    let mut g = game();
    let elder = unit(&mut g, 0, 1000, 321, None);
    let bully = unit(&mut g, 0, 1001, 322, None);
    um(&mut g, elder).temporary_buff = 2;
    g.empowered_units.push(1000);
    assert_eq!(g.unit_might(u(&g, elder)).unwrap(), 6);
    assert!(!g.unit_keywords(u(&g, bully)).unwrap().ganking);
    cast(&mut g, 28, vec![bully]);
    resolve(&mut g);
    assert!(!g.unit_keywords(u(&g, bully)).unwrap().ganking);
    g.players[0].hand.push(27);
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    })
    .unwrap();
    g.apply(Action::ChooseTarget {
        target: Some(elder),
    })
    .unwrap();
    resolve(&mut g);
    assert_eq!(g.unit_might(u(&g, elder)).unwrap(), 8);
    g.buffed_units.push(1001);
    um(&mut g, bully).buff = 1;
    assert!(g.unit_keywords(u(&g, bully)).unwrap().ganking);
    g.buffed_units.retain(|&id| id != 1001);
    um(&mut g, bully).buff = 0;
    assert!(!g.unit_keywords(u(&g, bully)).unwrap().ganking);
}
#[test]
fn local_and_token_auras_use_real_location_and_token_identity() {
    let mut g = game();
    let garen = unit(&mut g, 0, 1000, 320, None);
    let hound = unit(&mut g, 0, 1001, 323, None);
    let enemy = unit(&mut g, 1, 1002, 69, None);
    let lone = unit(&mut g, 0, 1003, 323, Some(1));
    assert_eq!(g.unit_might(u(&g, garen)).unwrap(), 5);
    assert_eq!(g.unit_might(u(&g, hound)).unwrap(), 4);
    assert_eq!(g.unit_might(u(&g, lone)).unwrap(), 2);
    assert_eq!(
        g.unit_might(u(&g, enemy)).unwrap(),
        u64::from(g.catalog[69].might)
    );
    let shepherd = unit(&mut g, 0, 1004, 324, None);
    let token = unit(&mut g, 0, 1005, 48, Some(0));
    um(&mut g, token).token = true;
    um(&mut g, token).buff = 1;
    assert_eq!(g.unit_might(u(&g, token)).unwrap(), 2);
    assert_eq!(g.unit_might(u(&g, shepherd)).unwrap(), 4);
    gear(&mut g, 0, 2000, 13, None);
    g.players[0].gear.last_mut().unwrap().token = true;
    assert_eq!(g.unit_might(u(&g, token)).unwrap(), 2);
}
#[test]
fn rune_and_level_thresholds_are_live_with_exhausted_runes_and_attached_sword() {
    let mut g = game();
    let yi = unit(&mut g, 0, 1000, 325, None);
    let seer = unit(&mut g, 0, 1001, 326, None);
    g.players[0].runes = [7, 0, 0, 0, 0, 0];
    g.players[0].exhausted_runes = [7, 0, 0, 0, 0, 0];
    g.players[0].rune_deck = vec![Domain::Body];
    assert_eq!(g.unit_might(u(&g, yi)).unwrap(), 4);
    cast(&mut g, 309, vec![]);
    resolve(&mut g);
    assert_eq!(g.unit_might(u(&g, yi)).unwrap(), 8);
    g.players[0].xp = 10;
    assert_eq!(g.unit_might(u(&g, seer)).unwrap(), 6);
    g.players[0].xp = 11;
    assert_eq!(g.unit_might(u(&g, seer)).unwrap(), 10);
    gear(&mut g, 0, 2000, 330, None);
    g.players[0].xp = 2;
    g.apply(Action::Equip {
        gear_id: 2000,
        target: yi,
    })
    .unwrap();
    resolve(&mut g);
    assert_eq!(g.unit_might(u(&g, yi)).unwrap(), 9);
    g.players[0].xp = 3;
    assert_eq!(g.unit_might(u(&g, yi)).unwrap(), 10);
    g.players[0].gear[0].attached_to = None;
    assert_eq!(g.unit_might(u(&g, yi)).unwrap(), 8);
}
fn conquer(g: &mut Game, id: u32) {
    g.apply(Action::Move {
        unit_id: id,
        lane: Some(0),
    })
    .unwrap();
    resolve(g);
    while let Some(Decision::OrderTriggers { ref options, .. }) = g.decision {
        let trigger_id = options[0].id;
        g.apply(Action::OrderTrigger { trigger_id }).unwrap();
    }
}
#[test]
fn hunt_is_one_captured_sum_and_preserves_all_legacy_hunt_values() {
    for (card, base) in [(22, 2), (153, 3), (154, 1)] {
        let mut g = game();
        let target = unit(&mut g, 0, 1000, card, None);
        gear(&mut g, 0, 2000, 328, Some(1000));
        gear(&mut g, 0, 2001, 328, Some(1000));
        assert_eq!(g.unit_keywords(u(&g, target)).unwrap().hunt, base + 2);
        conquer(&mut g, 1000);
        let hunts: Vec<_> = g
            .chain
            .iter()
            .filter(|i| matches!(i.ability, Some(ChainAbility::Hunt { .. })))
            .collect();
        assert_eq!(hunts.len(), 1);
        assert!(matches!(hunts[0].ability,Some(ChainAbility::Hunt{amount,..}) if amount==base+2));
        assert_eq!(
            g.chain_source(hunts[0]),
            ChainSource::Unit {
                player: 0,
                unit_id: 1000
            }
        );
        let other = unit(&mut g, 1, 1001, 70, None);
        cast(&mut g, 16, vec![target, other]);
        resolve(&mut g);
        assert!(g.players[0].hand.contains(&card));
        assert!(
            g.players[0]
                .gear
                .iter()
                .all(|gear| gear.attached_to.is_none())
        );
        drain(&mut g);
        assert_eq!(g.players[0].xp, u64::from(base + 2));
    }
}
#[test]
fn token_hunt_uses_captured_token_name_and_physical_unit_source() {
    let mut g = game();
    let target = unit(&mut g, 0, 1000, 48, None);
    um(&mut g, target).token = true;
    um(&mut g, target).buff = 1;
    gear(&mut g, 0, 2000, 328, Some(1000));
    conquer(&mut g, 1000);
    let hunt = g
        .chain
        .iter()
        .find(|i| matches!(i.ability, Some(ChainAbility::Hunt { .. })))
        .unwrap();
    assert_eq!(g.chain_name(hunt), "Tentacle token · Hunt 1");
    assert_eq!(
        g.chain_source(hunt),
        ChainSource::Unit {
            player: 0,
            unit_id: 1000
        }
    );
    drain(&mut g);
    assert_eq!(g.players[0].xp, 1);
}
#[test]
fn boneshiver_channels_on_conquer_but_not_hold() {
    let mut g = game();
    unit(&mut g, 0, 1000, 69, None);
    gear(&mut g, 0, 2000, 329, Some(1000));
    let before = g.players[0].total_runes();
    conquer(&mut g, 1000);
    drain(&mut g);
    assert_eq!(g.players[0].total_runes(), before + 1);
    assert_eq!(g.players[0].exhausted_runes.iter().sum::<u8>(), 1);
    let mut g = game();
    unit(&mut g, 1, 1000, 69, Some(0));
    gear(&mut g, 1, 2000, 329, Some(1000));
    g.players[1].rune_deck.clear();
    g.apply(Action::EndTurn).unwrap();
    assert!(!g.chain.iter().any(|i| i.card_id == 329));
}
#[test]
fn protector_quick_draw_is_free_attach_then_equip_costs_one_energy() {
    let mut g = game();
    let t = unit(&mut g, 0, 1000, 69, None);
    cast(&mut g, 327, vec![]);
    g.apply(Action::ChooseTarget { target: Some(t) }).unwrap();
    let energy = g.players[0].energy;
    resolve(&mut g);
    assert_eq!(g.players[0].energy, energy);
    assert!(g.unit_keywords(u(&g, t)).unwrap().tank);
    assert_eq!(
        g.unit_might(u(&g, t)).unwrap(),
        u64::from(g.catalog[69].might) + 1
    );
    let id = g.players[0].gear[0].id;
    g.players[0].gear[0].attached_to = None;
    let power = g.players[0].power_pool;
    g.apply(Action::Equip {
        gear_id: id,
        target: t,
    })
    .unwrap();
    assert_eq!(g.players[0].energy, energy - 1);
    assert_eq!(g.players[0].power_pool, power);
}
#[test]
fn monument_intrinsic_temporary_deflect_stacks_and_expiry_precedes_hold() {
    let mut g = game();
    let enemy = unit(&mut g, 1, 1000, 92, Some(0));
    gear(&mut g, 1, 2000, 331, None);
    gear(&mut g, 1, 2001, 331, None);
    assert_eq!(g.unit_keywords(u(&g, enemy)).unwrap().deflect, 3);
    g.players[0].hand.push(7);
    let quote = g
        .cost_plan(&Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![enemy],
        })
        .unwrap();
    assert_eq!(quote.power.iter().map(|c| c.amount).sum::<u16>(), 3);
    let mut g = game();
    cast(&mut g, 331, vec![]);
    assert!(g.players[0].gear[0].temporary);
    g.apply(Action::EndTurn).unwrap();
    drain(&mut g);
    g.apply(Action::EndTurn).unwrap();
    assert!(g.chain.iter().any(|i| i.card_id == 331));
    assert!(!g.players[0].discard.contains(&331));
    drain(&mut g);
    assert!(g.players[0].discard.contains(&331));
}

#[test]
fn turn_to_dust_temporary_stays_active_when_equipped_before_or_after_grant() {
    for grant_before_equip in [true, false] {
        let mut g = game();
        let bearer = unit(&mut g, 0, 1000, 69, None);
        cast(&mut g, 3, vec![]);
        let gear_id = g.players[0].gear[0].id;
        let gear_target = Target::Gear { player: 0, gear_id };
        if grant_before_equip {
            cast(&mut g, 66, vec![gear_target]);
            drain(&mut g);
        }
        g.apply(Action::Equip {
            gear_id,
            target: bearer,
        })
        .unwrap();
        drain(&mut g);
        if !grant_before_equip {
            cast(&mut g, 66, vec![gear_target]);
            drain(&mut g);
        }
        assert_eq!(g.players[0].gear[0].attached_to, Some(1000));
        assert_eq!(
            g.gear_temporary_status(0, gear_id),
            Some(local_rift::gear::GearTemporaryStatus {
                intrinsic: false,
                granted: true,
                active: true,
            })
        );
        g.apply(Action::EndTurn).unwrap();
        drain(&mut g);
        g.apply(Action::EndTurn).unwrap();
        assert_eq!(g.chain.iter().filter(|item| matches!(item.ability,
            Some(ChainAbility::Triggered(local_rift::game::TriggeredAbility::DestroyGear { gear_id: id })) if id == gear_id
        )).count(), 1);
        assert!(!g.players[0].discard.contains(&3), "expiry is respondable");
        drain(&mut g);
        assert!(g.players[0].gear.is_empty());
        assert!(g.players[0].discard.contains(&3));
        assert_eq!(u(&g, bearer).id, 1000, "only the gear is killed");
        assert_eq!(g.gear_temporary_status(0, gear_id), None);
    }
}
#[test]
fn vox_conquer_self_target_and_ribbon_move_target_can_be_elsewhere() {
    let mut g = game();
    let vox = unit(&mut g, 0, 1000, 317, None);
    conquer(&mut g, 1000);
    trigger_targets(&mut g, vec![vox]);
    drain(&mut g);
    assert_eq!(u(&g, vox).temporary_buff, 8);
    let mut g = game();
    let ribbon = unit(&mut g, 0, 1000, 332, None);
    let friend = unit(&mut g, 0, 1001, 69, Some(1));
    g.apply(Action::Move {
        unit_id: 1000,
        lane: Some(0),
    })
    .unwrap();
    reject(
        &mut g,
        Action::ChooseTriggerTargets {
            targets: vec![ribbon],
        },
    );
    trigger_targets(&mut g, vec![friend]);
    resolve(&mut g);
    assert_eq!(u(&g, friend).temporary_buff, 1);
}
#[test]
fn rune_might_loss_during_payment_defers_cleanup_and_failed_payment_rolls_back() {
    for rollback in [false, true] {
        let mut g = game();
        let yi = unit(&mut g, 0, 1000, 325, None);
        um(&mut g, yi).damage = 5;
        g.players[0].runes = [8, 0, 0, 0, 0, 0];
        g.players[0].exhausted_runes = [8, 0, 0, 0, 0, 0];
        g.players[0].power_pool = [0; 6];
        g.players[0].energy_pool = if rollback { 0 } else { 2 };
        g.players[0].energy = g.players[0].energy_pool;
        g.players[0].hand.push(7);
        let before = g.players[0].runes;
        g.apply(Action::BeginPayment {
            action: Box::new(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![yi],
            }),
        })
        .unwrap();
        g.apply(Action::RecycleRune {
            domain: Domain::Fury,
            exhausted: true,
        })
        .unwrap();
        assert_eq!(g.unit_might(u(&g, yi)).unwrap(), 4);
        assert_eq!(u(&g, yi).damage, 5);
        assert!(g.players[0].units.iter().any(|u| u.id == 1000));
        if rollback {
            assert!(g.can_cancel_payment().unwrap());
            g.apply(Action::Choose { accept: false }).unwrap();
            assert_eq!(g.players[0].runes, before);
            assert_eq!(g.unit_might(u(&g, yi)).unwrap(), 8);
            assert_eq!(g.players[0].hand, vec![7]);
        } else {
            g.apply(Action::CompletePayment).unwrap();
            // Cost finalization has ended: before Discipline resolves, state
            // cleanup correctly kills the now-lethal Yi. Its target is then gone.
            assert!(!g.players[0].units.iter().any(|u| u.id == 1000));
            resolve(&mut g);
            assert!(g.players[0].discard.contains(&325));
        }
    }
}
#[test]
fn simultaneous_ready_events_share_one_orderable_batch() {
    let mut g = game();
    let a = unit(&mut g, 0, 1000, 157, None);
    let b = unit(&mut g, 0, 1001, 157, None);
    um(&mut g, a).ready = false;
    um(&mut g, b).ready = false;
    cast(&mut g, 312, vec![]);
    resolve(&mut g);
    assert!(matches!(g.decision,Some(Decision::OrderTriggers{ref options,..}) if options.len()==2));
    assert_eq!(u(&g, a).temporary_buff, 3);
    assert_eq!(u(&g, b).temporary_buff, 3);
    assert!(g.players[0].discard.contains(&312));
    drain(&mut g);
    assert_eq!(u(&g, a).temporary_buff, 5);
    assert_eq!(u(&g, b).temporary_buff, 5);
}
#[test]
fn hunt_stacks_on_hold_and_herald_on_play_xp_remains_separate() {
    let mut g = game();
    unit(&mut g, 1, 1000, 154, Some(0));
    gear(&mut g, 1, 2000, 328, Some(1000));
    gear(&mut g, 1, 2001, 328, Some(1000));
    g.apply(Action::EndTurn).unwrap();
    drain(&mut g);
    assert_eq!(g.players[1].xp, 3);
    let mut g = game();
    cast(&mut g, 154, vec![]);
    resolve(&mut g);
    assert_eq!(g.players[0].xp, 2);
}
#[test]
fn global_ready_is_not_buff_and_unchecked_packets_share_live_source_snapshot() {
    for reversed in [false, true] {
        let mut g = game();
        unit(&mut g, 0, 1000, 259, Some(0));
        let enemy = unit(&mut g, 1, 1001, 69, Some(1));
        um(&mut g, enemy).temporary_buff = 20;
        if reversed {
            g.players[0].units.reverse();
            g.players[1].units.reverse();
        }
        cast(&mut g, 313, vec![]);
        resolve(&mut g);
        assert_eq!(u(&g, enemy).damage, 13);
        assert!(g.players[0].discard.contains(&259));
    }
}
#[test]
fn original_explicit_kill_victims_stay_fixed_across_multiple_recall_choices() {
    let mut g = game();
    let a = unit(&mut g, 1, 1000, 320, None);
    let b = unit(&mut g, 1, 1001, 323, None);
    gear(&mut g, 1, 2000, 10, Some(1000));
    gear(&mut g, 1, 2001, 21, None);
    assert_eq!(g.unit_might(u(&g, b)).unwrap(), 4);
    cast(&mut g, 308, vec![]);
    resolve(&mut g);
    g.apply(Action::ChooseDeathReplacement {
        unit_id: 1000,
        gear_id: Some(2000),
    })
    .unwrap();
    // Garen's recalled replacement is finished; the shared Hourglass can now
    // replace only Ramhound's original event, never kill Garen a second time.
    assert_eq!(u(&g, a).damage, 0);
    assert_eq!(u(&g, b).damage, 0);
    assert!(!u(&g, a).ready && !u(&g, b).ready);
    assert!(g.players[1].gear.is_empty());
    assert!(g.players[0].discard.contains(&308));
}

#[test]
fn tracker_ai_selects_useful_enemy_damage_and_can_choose_zero_without_enemies() {
    for difficulty in [Difficulty::Easy, Difficulty::Tactical] {
        let mut g = game();
        g.difficulty = difficulty;
        unit(&mut g, 0, 1000, 69, None);
        let a = unit(&mut g, 1, 1001, 69, None);
        let b = unit(&mut g, 1, 1002, 70, None);
        cast(&mut g, 333, vec![]);
        g.step_ai().unwrap();
        let targets = &g.chain.last().unwrap().targets;
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&a) && targets.contains(&b));
        let mut g = game();
        g.difficulty = difficulty;
        unit(&mut g, 0, 1000, 69, None);
        cast(&mut g, 333, vec![]);
        g.step_ai().unwrap();
        assert!(g.chain.last().unwrap().targets.is_empty());
    }
}
