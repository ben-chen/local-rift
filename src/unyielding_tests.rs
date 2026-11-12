//! Current Core Rules 418.6 and 438, exercised with actual card plays.
use super::*;

const SPIRIT: usize = 441;

fn position(actor: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 441441, Difficulty::Tactical);
    g.choose_starting_player(actor).unwrap();
    g.start_match().unwrap();
    g.battlefields = vec![9, 9]; // No conquest draw/trash triggers in these positions.
    g.battlefield_owner = vec![None; 2];
    for p in &mut g.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.banish.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 100;
        p.energy = 100;
        p.power_pool = [20; 6];
        p.any_power = 0;
    }
    for legend in &mut g.legend {
        legend.ready = false;
    }
    g
}
fn unit(g: &mut Game, owner: usize, card: CardId, lane: Option<usize>) -> Target {
    let id = g.create_unit(owner, card, false, 0);
    let u = g.players[owner].units.last_mut().unwrap();
    u.lane = lane;
    u.ready = true;
    Target::Unit {
        player: owner,
        unit_id: id,
    }
}
fn cast(g: &mut Game, id: CardId, targets: Vec<Target>) {
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
fn focus(g: &mut Game, player: usize) {
    if g.priority() != player {
        g.apply(Action::Pass).unwrap();
    }
    assert_eq!(g.priority(), player);
}
fn settle(g: &mut Game) {
    for _ in 0..80 {
        let action = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            None if !g.chain.is_empty() => Action::Pass,
            None => return,
            other => panic!("Unexpected choice {other:?}"),
        };
        g.apply(action).unwrap();
    }
    panic!("Chain failed to settle");
}

#[test]
fn unyielding_reaction_prevents_every_spell_packet_and_preserves_existing_damage() {
    for caster in 0..2 {
        let mut g = position(1 - caster);
        let target = unit(&mut g, caster, 69, None);
        g.unit_mut(target).unwrap().damage = 1;
        cast(&mut g, 53, vec![target, target]);
        focus(&mut g, caster);
        cast(&mut g, SPIRIT, vec![]);
        assert!(!g.spell_ability_damage_prevented);
        assert!(g.chain.last().unwrap().targets.is_empty());
        settle(&mut g);
        assert!(g.spell_ability_damage_prevented);
        assert_eq!(g.unit_ref(target).unwrap().damage, 1);
        assert_eq!(g.players[caster].energy, 99);
        assert_eq!(g.players[caster].power_pool[domain_index(Domain::Body)], 19);
        assert!(g.players[1 - caster].discard.contains(&53));
    }
}

#[test]
fn unyielding_protects_both_players_and_later_entries_without_removing_draws() {
    for caster in 0..2 {
        let mut g = position(caster);
        cast(&mut g, SPIRIT, vec![]);
        settle(&mut g);
        for owner in 0..2 {
            let target = unit(&mut g, owner, 69, Some(owner));
            let before = g.players[caster].deck.len();
            cast(&mut g, 67, vec![target]);
            settle(&mut g);
            assert_eq!(g.unit_ref(target).unwrap().damage, 0);
            assert_eq!(g.players[caster].deck.len(), before - 1);
        }
    }
}

#[test]
fn unyielding_counter_and_failed_payment_do_not_create_protection() {
    for caster in 0..2 {
        let mut g = position(caster);
        g.players[caster].hand = vec![SPIRIT];
        g.players[caster].power_pool = [0; 6];
        let before = format!("{g:?}");
        assert!(
            g.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![]
            })
            .is_err()
        );
        assert_eq!(format!("{g:?}"), before);
        g.players[caster].power_pool = [20; 6];
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        })
        .unwrap();
        let chain_id = g.chain.last().unwrap().id;
        focus(&mut g, 1 - caster);
        cast(&mut g, 86, vec![Target::Spell { chain_id }]);
        settle(&mut g);
        assert!(!g.spell_ability_damage_prevented);
        assert!(g.players[caster].discard.contains(&SPIRIT));
    }
}

#[test]
fn unyielding_does_not_prevent_damage_explicitly_dealt_by_units() {
    for caster in 0..2 {
        let mut g = position(caster);
        let friendly = unit(&mut g, caster, 69, None);
        let enemy = unit(&mut g, 1 - caster, 73, None);
        cast(&mut g, SPIRIT, vec![]);
        settle(&mut g);
        cast(&mut g, 82, vec![friendly, enemy]);
        settle(&mut g);
        assert!(g.unit_ref(enemy).is_none());
        assert_eq!(g.unit_ref(friendly).unwrap().damage, 4);
    }
}

#[test]
fn unyielding_blocks_gear_and_unit_abilities_including_all_live_bonuses() {
    for caster in 0..2 {
        for source_card in [232, 238] {
            let mut g = position(caster);
            g.battlefields[0] = 3; // Void Gate adds spell/ability damage.
            let target = unit(&mut g, 1 - caster, 69, Some(0));
            unit(&mut g, caster, 259, None); // Annie adds another Bonus Damage.
            let source = if source_card == 232 {
                g.players[caster].gear.push(GearState {
                    id: 9000,
                    card_id: 232,
                    lane: None,
                    attached_to: None,
                    ready: true,
                    temporary: false,
                    token: false,
                });
                AbilitySource::Gear {
                    player: caster,
                    gear_id: 9000,
                }
            } else {
                let Target::Unit { unit_id, .. } = unit(&mut g, caster, 238, Some(0)) else {
                    unreachable!()
                };
                AbilitySource::Unit {
                    player: caster,
                    unit_id,
                }
            };
            g.apply(Action::ActivateAbility {
                source,
                ability_index: 0,
                targets: vec![target],
            })
            .unwrap();
            focus(&mut g, 1 - caster);
            cast(&mut g, SPIRIT, vec![]);
            settle(&mut g);
            assert_eq!(g.unit_ref(target).unwrap().damage, 0);
        }
    }
}

#[test]
fn unyielding_expires_after_the_turn_and_does_not_stack() {
    for caster in 0..2 {
        let mut g = position(caster);
        cast(&mut g, SPIRIT, vec![]);
        settle(&mut g);
        cast(&mut g, SPIRIT, vec![]);
        settle(&mut g);
        g.apply(Action::EndTurn).unwrap();
        settle(&mut g);
        assert!(!g.spell_ability_damage_prevented);
        let target = unit(&mut g, caster, 69, Some(0));
        g.players[1 - caster].energy = 100;
        g.players[1 - caster].energy_pool = 100;
        g.players[1 - caster].power_pool = [20; 6];
        cast(&mut g, 67, vec![target]);
        settle(&mut g);
        assert_eq!(g.unit_ref(target).unwrap().damage, 4);
    }
}

#[test]
fn unyielding_prevents_bonus_packets_even_above_the_numeric_storage_limit() {
    let mut g = position(0);
    let target = unit(&mut g, 1, 69, Some(0));
    cast(&mut g, SPIRIT, vec![]);
    settle(&mut g);
    let context = damage::DamageContext {
        player: 0,
        bonus_damage: u64::MAX,
        dealer: damage::DamageDealer::Spell { card_id: 53 },
    };
    assert_eq!(g.damage_amount(context, target, u64::MAX).unwrap(), 0);
    g.deal_damage(context, target, u64::MAX).unwrap();
    assert_eq!(g.unit_ref(target).unwrap().damage, 0);
    let unit_context = damage::DamageContext {
        dealer: damage::DamageDealer::Unit,
        ..context
    };
    assert_eq!(g.damage_amount(unit_context, target, 7).unwrap(), 7);
}

#[test]
fn unyielding_remains_active_through_ending_triggers_and_expires_in_ending_cleanup() {
    let mut g = position(0);
    let target = unit(&mut g, 1, 69, None);
    cast(&mut g, SPIRIT, vec![]);
    settle(&mut g);
    // Isolate the expiration boundary: actual ending trigger resolution occurs
    // between these two engine stages, not after finish_ending.
    g.begin_ending();
    assert!(g.spell_ability_damage_prevented);
    g.deal_damage(
        damage::DamageContext {
            player: 0,
            bonus_damage: 0,
            dealer: damage::DamageDealer::Ability,
        },
        target,
        3,
    )
    .unwrap();
    assert_eq!(g.unit_ref(target).unwrap().damage, 0);
    g.finish_ending().unwrap();
    assert!(!g.spell_ability_damage_prevented);
}

#[test]
fn unyielding_prevents_trigger_damage_but_normal_combat_still_kills_units() {
    for caster in 0..2 {
        let mut g = position(caster);
        let source = unit(&mut g, caster, 413, None); // Yasuo's attack damage.
        let enemy = unit(&mut g, 1 - caster, 69, Some(0));
        cast(&mut g, SPIRIT, vec![]);
        settle(&mut g);
        let Target::Unit { unit_id, .. } = source else {
            unreachable!()
        };
        g.apply(Action::Move {
            unit_id,
            lane: Some(0),
        })
        .unwrap();
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![enemy],
        })
        .unwrap();
        settle(&mut g);
        assert_eq!(g.unit_ref(enemy).unwrap().damage, 0);
        // No additional cards or abilities are played while finishing combat.
        for _ in 0..30 {
            if g.showdown.is_none() && g.decision.is_none() {
                break;
            }
            let action = match &g.decision {
                Some(Decision::CombatDamage {
                    player,
                    lane,
                    total,
                }) => Action::AssignDamage {
                    assignments: g
                        .automatic_damage_assignment(*player, *lane, *total)
                        .unwrap(),
                },
                None => Action::Pass,
                other => panic!("Unexpected combat choice {other:?}"),
            };
            g.apply(action).unwrap();
        }
        assert!(
            g.unit_ref(source).is_none(),
            "Combat damage must still defeat Yasuo"
        );
        assert!(g.spell_ability_damage_prevented);
    }
}

#[test]
fn unyielding_ai_saves_units_and_ignores_harmless_or_unit_damage_chains() {
    for caster in 0..2 {
        for level in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let mut g = position(1 - caster);
            g.difficulty = level;
            let target = unit(&mut g, caster, 73, Some(0));
            cast(&mut g, 67, vec![target]);
            focus(&mut g, caster);
            g.players[caster].hand = vec![SPIRIT];
            assert!(g.damage_prevention_score().unwrap() > 0);
            g.step_ai().unwrap();
            assert_eq!(
                g.chain.last().unwrap().card_id,
                SPIRIT,
                "{level:?}/{caster}"
            );
            settle(&mut g);
            assert!(g.unit_ref(target).is_some());
            assert_eq!(g.damage_prevention_score().unwrap(), 0);
        }
        let mut g = position(1 - caster);
        let own = unit(&mut g, 1 - caster, 69, None);
        let enemy = unit(&mut g, caster, 73, None);
        cast(&mut g, 82, vec![own, enemy]);
        focus(&mut g, caster);
        g.players[caster].hand = vec![SPIRIT];
        assert_eq!(g.damage_prevention_score().unwrap(), 0);
    }
}

#[test]
fn unyielding_ai_does_not_read_hidden_hands_or_deck_order() {
    let mut g = position(1);
    let target = unit(&mut g, 0, 73, Some(0));
    cast(&mut g, 67, vec![target]);
    focus(&mut g, 0);
    let score = g.damage_prevention_score().unwrap();
    let mut changed = g.clone();
    changed.players[1].hand = vec![441, 53, 86, 67];
    changed.players[0].deck = vec![299; 20];
    changed.players[1].deck = vec![69; 20];
    assert_eq!(changed.damage_prevention_score().unwrap(), score);
}

#[test]
fn unyielding_ai_preserves_damage_resources_but_values_independent_draws() {
    for level in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        let mut g = position(0);
        g.difficulty = level;
        let target = unit(&mut g, 1, 69, Some(0));
        g.players[0].gear.push(GearState {
            id: 9000,
            card_id: 232,
            lane: None,
            attached_to: None,
            ready: true,
            temporary: false,
            token: false,
        });
        cast(&mut g, SPIRIT, vec![]);
        settle(&mut g);
        assert_eq!(g.ai_card_score(53).unwrap(), 0);
        assert!(g.ai_card_score(67).unwrap() > 0);
        assert_eq!(
            g.ability_ai_score(
                AbilitySource::Gear {
                    player: 0,
                    gear_id: 9000
                },
                0,
                &[target]
            )
            .unwrap(),
            None
        );
    }
}
