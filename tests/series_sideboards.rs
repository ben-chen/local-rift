//! Exact spell semantics required by the published LA sideboards.
use local_rift::{
    activated::AbilitySource,
    cards::{Domain, cards},
    game::{Action, Decision, Difficulty, Game, Showdown, Target, Unit},
    gear::GearState,
    match_ai::Scouting,
    match_series::Registration,
    play::{ExecutionChoices, PlayChoices, RepeatSelection},
};

const DECREE: usize = 433;
const LACERATE: usize = 434;

fn position() -> Game {
    position_for(0)
}
fn position_for(player: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 113434, Difficulty::Tactical);
    g.choose_starting_player(player).unwrap();
    g.start_match().unwrap();
    g.battlefields = vec![3, 3];
    g.battlefield_owner = vec![None; 2];
    for p in &mut g.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 50;
        p.energy = 50;
        p.power_pool = [20; 6];
        p.any_power = 0;
    }
    g
}
fn unit(g: &mut Game, owner: usize, id: u32, card: usize, lane: Option<usize>) -> Target {
    g.players[owner].units.push(Unit {
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
        player: owner,
        unit_id: id,
    }
}
fn cast(g: &mut Game, card: usize, targets: Vec<Target>) {
    let actor = g.priority();
    let index = g.players[actor].hand.len();
    g.players[actor].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index: index,
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
fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action).is_err());
    assert_eq!(format!("{g:?}"), before);
}
fn grant_repeat(g: &mut Game) {
    g.players[0].gear.push(GearState {
        id: 9000,
        card_id: 256,
        lane: None,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    g.apply(Action::ActivateAbility {
        source: AbilitySource::Gear {
            player: 0,
            gear_id: 9000,
        },
        ability_index: 0,
        targets: vec![],
    })
    .unwrap();
    resolve(g);
}
fn repeat(g: &Game, action: Action, targets: Vec<Target>) -> Action {
    let available = g.play_choices(&action).unwrap();
    assert_eq!(available.repeats.len(), 1);
    Action::WithPlayChoices {
        action: Box::new(action),
        choices: PlayChoices {
            repeats: available
                .repeats
                .iter()
                .map(|r| RepeatSelection {
                    option_id: r.id,
                    choices: ExecutionChoices {
                        destination: None,
                        targets: targets.clone(),
                        mode: None,
                    },
                })
                .collect(),
            ..Default::default()
        },
    }
}

#[test]
fn registrations_match_source_pools_and_all_are_legal() {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../assets/data/preset_admissions/series_los_angeles/registrations.json"
    ))
    .unwrap();
    for row in v["registrations"].as_array().unwrap() {
        let i = row["preset"].as_u64().unwrap() as usize;
        let r = Registration::from_preset(i).unwrap();
        r.validate().unwrap();
        let metadata = local_rift::cards::presets()[i].registration.unwrap();
        assert_eq!(metadata.pilot, row["pilot"].as_str().unwrap());
        assert_eq!(metadata.event, row["event"].as_str().unwrap());
        assert_eq!(metadata.date, row["date"].as_str().unwrap());
        assert_eq!(
            u64::from(metadata.placing),
            row["placing"].as_u64().unwrap()
        );
        assert_eq!(serde_json::to_value(r.sideboard).unwrap(), row["sideboard"]);
        assert_eq!(
            serde_json::to_value(r.battlefields).unwrap(),
            row["battlefields"]
        );
    }
}

#[test]
fn decree_selects_mind_units_and_multidomain_cards_then_recycles_to_bottom() {
    let dual = cards()
        .iter()
        .find(|c| c.domains.len() > 1 && c.domains.contains(&Domain::Mind))
        .unwrap()
        .id;
    for chosen in [79, 283, dual] {
        let mut g = position();
        g.players[1].hand = vec![7, chosen, 23];
        g.players[1].deck = vec![8, 9];
        cast(&mut g, DECREE, vec![]);
        resolve(&mut g);
        assert!(matches!(
            g.decision,
            Some(Decision::RecycleRevealedHand {
                player: 0,
                opponent: 1,
                domain: Domain::Mind
            })
        ));
        reject(&mut g, Action::SelectCards { indices: vec![] });
        reject(&mut g, Action::SelectCards { indices: vec![0] });
        reject(&mut g, Action::SelectCards { indices: vec![99] });
        reject(
            &mut g,
            Action::SelectCards {
                indices: vec![1, 1],
            },
        );
        g.apply(Action::SelectCards { indices: vec![1] }).unwrap();
        assert_eq!(g.players[1].hand, vec![7, 23]);
        assert_eq!(g.players[1].deck, vec![chosen, 8, 9]);
        assert!(g.players[1].discard.is_empty());
        assert!(g.players[0].discard.contains(&DECREE));
    }
}

#[test]
fn decree_no_mind_or_empty_hand_reveals_and_requires_explicit_continue() {
    for hand in [vec![], vec![7, 23]] {
        let mut g = position();
        g.players[1].hand = hand.clone();
        cast(&mut g, DECREE, vec![]);
        resolve(&mut g);
        assert!(g.decision.is_some());
        reject(&mut g, Action::SelectCards { indices: vec![0] });
        g.apply(Action::SelectCards { indices: vec![] }).unwrap();
        assert_eq!(g.players[1].hand, hand);
    }
}

#[test]
fn repeated_decree_reveals_updated_hand_and_handles_exhausted_eligible_cards() {
    let mut g = position();
    grant_repeat(&mut g);
    g.players[1].hand = vec![79, 7];
    g.players[0].hand = vec![DECREE];
    let a = repeat(
        &g,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        },
        vec![],
    );
    g.apply(a).unwrap();
    resolve(&mut g);
    g.apply(Action::SelectCards { indices: vec![0] }).unwrap();
    assert!(matches!(
        g.decision,
        Some(Decision::RecycleRevealedHand { .. })
    ));
    assert_eq!(g.players[1].hand, vec![7]);
    g.apply(Action::SelectCards { indices: vec![] }).unwrap();
    assert!(g.chain.is_empty());
    assert_eq!(
        g.players[0]
            .discard
            .iter()
            .filter(|&&id| id == DECREE)
            .count(),
        1
    );
}

#[test]
fn hand_scouting_only_records_cards_after_the_legal_reveal() {
    let mut g = position();
    g.players[1].hand = vec![79, 283];
    let mut scout = Scouting::default();
    scout.observe_game(&g, 0).unwrap();
    let hidden = format!("{scout:?}");
    // Empty public zones: merely changing hidden cards cannot change scouting.
    g.players[1].hand = vec![7, 23];
    let mut other = Scouting::default();
    other.observe_game(&g, 0).unwrap();
    assert_eq!(hidden, format!("{other:?}"));
    g.players[1].hand = vec![79, 283];
    cast(&mut g, DECREE, vec![]);
    resolve(&mut g);
    scout.observe_game(&g, 0).unwrap();
    assert_eq!(scout.to_json()["seen_cards"], serde_json::json!([79, 283]));
}

#[test]
fn lacerate_targets_either_player_in_base_or_fields_without_initial_might_gate() {
    for owner in 0..2 {
        for lane in [None, Some(0), Some(1)] {
            let mut g = position();
            let small = unit(&mut g, owner, 100, 23, lane);
            let big = unit(&mut g, owner, 101, 73, lane);
            let options = g.card_target_candidates(0, LACERATE, None, &[]).unwrap();
            assert!(options.contains(&small) && options.contains(&big));
            cast(&mut g, LACERATE, vec![big]);
            resolve(&mut g);
            assert!(g.players[owner].units.iter().any(|u| u.id == 101));
            cast(&mut g, LACERATE, vec![small]);
            resolve(&mut g);
            assert!(!g.players[owner].units.iter().any(|u| u.id == 100));
        }
    }
}

#[test]
fn disempower_precedes_might_check_and_removes_empowered_deathknell() {
    for card in [273, 277, 278, 283, 285] {
        let mut g = position();
        let t = unit(&mut g, 1, 100, card, None);
        g.empowered_units.push(100);
        cast(&mut g, LACERATE, vec![t]);
        resolve(&mut g);
        assert!(!g.unit_empowered(100));
        assert!(g.players[1].units.is_empty(), "card {card}");
        assert!(g.players[1].discard.contains(&card));
        assert!(g.chain.is_empty(), "Empowered Deathknell must be absent");
    }
}

#[test]
fn large_empowered_unit_survives_but_loses_status() {
    let mut g = position();
    let t = unit(&mut g, 1, 100, 275, None);
    g.empowered_units.push(100);
    cast(&mut g, LACERATE, vec![t]);
    resolve(&mut g);
    assert!(!g.unit_empowered(100));
    assert_eq!(g.players[1].units.len(), 1);
}

#[test]
fn response_buff_saves_target_and_response_debuff_can_make_it_die() {
    for (card, response, survives) in [(23, 7, true), (73, 79, false)] {
        let mut g = position();
        let t = unit(&mut g, 1, 100, card, None);
        cast(&mut g, LACERATE, vec![t]);
        g.apply(Action::Pass).unwrap();
        cast(&mut g, response, vec![t]);
        resolve(&mut g);
        resolve(&mut g);
        assert_eq!(!g.players[1].units.is_empty(), survives);
    }
}

#[test]
fn removed_physical_target_does_not_transfer_to_another_copy() {
    let mut g = position();
    let t = unit(&mut g, 1, 100, 283, None);
    g.empowered_units.push(100);
    cast(&mut g, LACERATE, vec![t]);
    g.players[1].units.clear();
    g.empowered_units.clear();
    unit(&mut g, 1, 101, 283, None);
    g.empowered_units.push(101);
    resolve(&mut g);
    assert!(g.unit_empowered(101));
    assert_eq!(g.players[1].units.len(), 1);
}

#[test]
fn flow_pays_four_energy_two_order_and_banishes_instead_of_trashing() {
    let mut g = position();
    let t = unit(&mut g, 1, 100, 23, None);
    g.players[0].discard = vec![LACERATE];
    g.players[0].energy = 4;
    g.players[0].energy_pool = 4;
    g.players[0].power_pool = [0; 6];
    g.players[0].power_pool[local_rift::game::domain_index(Domain::Order)] = 1;
    let a = Action::PlayFromTrashWithTargets {
        trash_index: 0,
        lane: None,
        targets: vec![t],
    };
    reject(&mut g, a.clone());
    g.players[0].power_pool[local_rift::game::domain_index(Domain::Order)] = 2;
    g.apply(a).unwrap();
    assert_eq!(g.players[0].energy, 0);
    assert_eq!(g.players[0].power_pool, [0; 6]);
    resolve(&mut g);
    assert_eq!(g.players[0].banish, vec![LACERATE]);
    assert!(g.players[0].discard.is_empty());
}

#[test]
fn normal_timing_applies_to_both_spells_and_lacerate_flow() {
    let mut g = position();
    let t = unit(&mut g, 1, 100, 23, Some(0));
    g.showdown = Some(Showdown {
        lane: 0,
        attacker: 0,
        focus: 0,
        passes: 0,
        combat: true,
    });
    for card in [DECREE, LACERATE] {
        g.players[0].hand = vec![card];
        reject(
            &mut g,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: if card == LACERATE { vec![t] } else { vec![] },
            },
        );
    }
    g.players[0].discard = vec![LACERATE];
    reject(
        &mut g,
        Action::PlayFromTrashWithTargets {
            trash_index: 0,
            lane: None,
            targets: vec![t],
        },
    );
}

#[test]
fn repeated_lacerate_uses_separate_targets_and_disempowers_each() {
    let mut g = position();
    grant_repeat(&mut g);
    let a = unit(&mut g, 1, 100, 283, None);
    let b = unit(&mut g, 1, 101, 275, None);
    g.empowered_units.extend([100, 101]);
    g.players[0].hand = vec![LACERATE];
    let action = repeat(
        &g,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![a],
        },
        vec![b],
    );
    g.apply(action).unwrap();
    resolve(&mut g);
    assert_eq!(g.players[1].units.len(), 1);
    assert_eq!(g.players[1].units[0].id, 101);
    assert!(!g.unit_empowered(101));
}

#[test]
fn tactical_ai_selects_useful_enemy_over_large_or_friendly_targets() {
    let mut g = position();
    unit(&mut g, 0, 99, 23, None);
    unit(&mut g, 1, 100, 73, None);
    unit(&mut g, 1, 101, 283, None);
    g.empowered_units.push(101);
    g.players[0].hand = vec![LACERATE];
    g.step_ai().unwrap();
    assert_eq!(
        g.chain.last().unwrap().targets,
        vec![Target::Unit {
            player: 1,
            unit_id: 101
        }]
    );
}

#[test]
fn decree_ai_choices_are_legal_for_all_levels_and_both_seats() {
    for difficulty in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        for player in 0..2 {
            let mut g = position_for(player);
            g.difficulty = difficulty;
            g.players[1 - player].hand = vec![7, 283, 79];
            cast(&mut g, DECREE, vec![]);
            resolve(&mut g);
            g.step_ai().unwrap();
            assert_eq!(g.players[1 - player].hand.len(), 2);
            assert!(g.players[1 - player].hand.contains(&7));
            assert!(g.decision.is_none());
        }
    }
}

#[test]
fn counters_stop_both_effects_and_flow_is_still_banished() {
    for (id, flow) in [(DECREE, false), (LACERATE, false), (LACERATE, true)] {
        let mut g = position();
        let t = unit(&mut g, 1, 100, 283, None);
        g.empowered_units.push(100);
        g.players[1].hand = vec![79];
        let targets = if id == LACERATE { vec![t] } else { vec![] };
        if flow {
            g.players[0].discard = vec![id];
            g.apply(Action::PlayFromTrashWithTargets {
                trash_index: 0,
                lane: None,
                targets,
            })
            .unwrap();
        } else {
            cast(&mut g, id, targets);
        }
        let chain_id = g.chain[0].id;
        g.apply(Action::Pass).unwrap();
        cast(&mut g, 86, vec![Target::Spell { chain_id }]);
        resolve(&mut g);
        assert!(g.chain.is_empty());
        assert!(g.decision.is_none());
        assert!(g.unit_empowered(100));
        assert_eq!(g.players[1].hand, vec![79]);
        assert_eq!(g.players[0].banish.contains(&id), flow);
        assert_eq!(g.players[0].discard.contains(&id), !flow);
    }
}

#[test]
fn lacerate_deflect_is_paid_before_disempower_and_failed_payment_is_atomic() {
    let mut g = position();
    let t = unit(&mut g, 1, 100, 277, None);
    g.empowered_units.push(100);
    g.players[0].hand = vec![LACERATE];
    g.players[0].power_pool = [0; 6];
    g.players[0].power_pool[local_rift::game::domain_index(Domain::Order)] = 1;
    let a = Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![t],
    };
    reject(&mut g, a.clone());
    assert!(g.unit_empowered(100));
    g.players[0].any_power = 2;
    g.apply(a).unwrap();
    assert_eq!(g.players[0].any_power, 0);
    resolve(&mut g);
    assert!(g.players[1].units.is_empty());
}

#[test]
fn new_sideboard_roles_use_public_evidence_and_preserve_registered_pool() {
    use local_rift::match_ai::{MatchPolicy, recommend};
    use local_rift::match_series::MatchSeries;
    for (preset, answer) in [(4, DECREE), (2, LACERATE)] {
        let own = Registration::from_preset(preset).unwrap();
        let enemy = Registration::from_preset(5).unwrap();
        let mut s = MatchSeries::new([own.clone(), enemy.clone()], 0).unwrap();
        s.choose_battlefield(0, own.battlefields[0]).unwrap();
        s.choose_battlefield(1, enemy.battlefields[0]).unwrap();
        s.choose_play_order(0, 0).unwrap();
        s.concede_game(1).unwrap();
        let view = s.observe(0).unwrap();
        let mut g = position();
        g.players[1].hand = vec![283];
        let mut scout = Scouting::default();
        scout.observe_game(&g, 0).unwrap();
        let hidden = recommend(
            &view,
            0,
            &scout,
            Difficulty::Strategic,
            MatchPolicy::Adaptive,
        )
        .unwrap();
        assert!(!hidden.deck.cards.iter().any(|&(id, _)| id == answer));
        unit(&mut g, 1, 100, 283, None);
        scout.observe_game(&g, 0).unwrap();
        let plan = recommend(
            &view,
            0,
            &scout,
            Difficulty::Strategic,
            MatchPolicy::Adaptive,
        )
        .unwrap();
        assert!(plan.deck.cards.iter().any(|&(id, n)| id == answer && n > 0));
        own.configure(plan.deck.cards, plan.deck.chosen_champion)
            .unwrap();
    }
}

#[test]
fn disempower_does_not_bypass_guardian_angel_death_replacement() {
    let mut g = position();
    let t = unit(&mut g, 1, 100, 285, Some(0));
    g.empowered_units.push(100);
    g.players[1].gear.push(GearState {
        id: 101,
        card_id: 10,
        lane: Some(0),
        attached_to: Some(100),
        ready: true,
        temporary: false,
        token: false,
    });
    cast(&mut g, LACERATE, vec![t]);
    resolve(&mut g);
    assert_eq!(g.players[1].units.len(), 1);
    assert_eq!(g.players[1].units[0].id, 100);
    assert_eq!(g.players[1].units[0].lane, None);
    assert!(!g.players[1].units[0].ready);
    assert!(!g.unit_empowered(100));
    assert!(g.players[1].gear.is_empty());
    assert!(g.players[1].discard.contains(&10));
    assert!(!g.players[1].discard.contains(&285));
}

#[test]
fn choosing_friendly_irelia_still_triggers_before_lacerate_resolves() {
    let mut g = position();
    g.legend[0].kind = local_rift::legends::LegendKind::Irelia;
    g.legend[0].ready = false;
    let t = unit(&mut g, 0, 100, 0, None);
    let before = g.unit_might(&g.players[0].units[0]).unwrap();
    cast(&mut g, LACERATE, vec![t]);
    let mut saw_growth = false;
    while !g.chain.is_empty() || g.decision.is_some() {
        saw_growth |= g
            .chain
            .iter()
            .any(|item| item.card_id == 0 && item.ability.is_some());
        if matches!(g.decision, Some(Decision::Legend { .. })) {
            g.apply(Action::Choose { accept: false }).unwrap();
        } else if g.decision.is_some() {
            g.apply(g.legal_actions().unwrap().remove(0)).unwrap();
        } else {
            resolve(&mut g);
        }
    }
    assert!(saw_growth);
    assert_eq!(g.players[0].units.len(), 1);
    assert_eq!(g.unit_might(&g.players[0].units[0]).unwrap(), before + 1);
}

#[test]
fn ai_preserves_lacerate_and_resources_when_it_cannot_help() {
    for difficulty in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        for flow in [false, true] {
            let mut g = position();
            g.difficulty = difficulty;
            unit(&mut g, 0, 99, 23, None);
            g.players[0].units[0].ready = false;
            unit(&mut g, 1, 100, 73, None);
            if flow {
                g.players[0].discard = vec![LACERATE];
            } else {
                g.players[0].hand = vec![LACERATE];
            }
            for _ in 0..12 {
                if g.active != 0 {
                    break;
                }
                g.step_ai().unwrap();
                assert!(
                    !g.chain.iter().any(|c| c.card_id == LACERATE),
                    "{difficulty:?}/flow={flow}"
                );
            }
            assert_eq!(
                g.active, 1,
                "AI should end its turn instead of spending on ineffective removal"
            );
            assert!(if flow {
                g.players[0].discard.contains(&LACERATE)
            } else {
                g.players[0].hand.contains(&LACERATE)
            });
            assert_eq!(g.players[0].units.len(), 1);
        }
    }
}
