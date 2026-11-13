//! Controlled starting boards, followed by actual card plays and public actions.
use super::*;
use crate::game::*;
use crate::play::{ExecutionChoices, PlayChoices, RepeatSelection};

fn position(actor: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 429418, Difficulty::Tactical);
    g.choose_starting_player(actor).unwrap();
    g.start_match().unwrap();
    g.battlefields = vec![9, 9];
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
        p.energy = 100;
        p.energy_pool = 100;
        p.power_pool = [20; 6];
    }
    for legend in &mut g.legend {
        legend.ready = false;
    }
    g
}
fn unit(g: &mut Game, player: usize, might: i64, lane: Option<usize>) -> Target {
    let id = g.create_unit(player, 69, false, 0);
    let u = g.players[player].units.last_mut().unwrap();
    u.lane = lane;
    u.ready = true;
    u.temporary_buff = might - 8;
    Target::Unit {
        player,
        unit_id: id,
    }
}
fn cast(g: &mut Game, card: usize, targets: Vec<Target>) -> u32 {
    let p = g.priority();
    let index = g.players[p].hand.len();
    g.players[p].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index: index,
        lane: None,
        targets,
    })
    .unwrap();
    g.chain.last().unwrap().id
}
fn pair(g: &mut Game) {
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn settle(g: &mut Game) {
    for _ in 0..100 {
        let action = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            None if !g.chain.is_empty() => Action::Pass,
            None => return,
            other => panic!("Unexpected decision {other:?}"),
        };
        g.apply(action).unwrap();
    }
    panic!("Chain failed to finish");
}
fn source(target: Target) -> DamageSource {
    let Target::Unit { player, unit_id } = target else {
        panic!()
    };
    DamageSource::Object(ChainSource::Unit { player, unit_id })
}
fn spell(chain_id: u32, player: usize, card_id: usize) -> DamageSource {
    DamageSource::Chain {
        chain_id,
        player,
        card_id,
        ability: false,
    }
}

#[test]
fn fight_dealers_and_responsible_players_are_independent_of_its_spell_caster() {
    for actor in 0..2 {
        let mut g = position(actor);
        let a = unit(&mut g, actor, 3, Some(0));
        let b = unit(&mut g, 1 - actor, 3, Some(0));
        g.spell_ability_damage_prevented = true;
        let parent = cast(&mut g, 82, vec![a, b]);
        pair(&mut g);
        let ledger = g.last_damage_ledger().unwrap();
        assert_eq!(
            ledger.damage,
            vec![
                DamagePacket {
                    player: 1 - actor,
                    target: a,
                    amount: 3,
                    sources: vec![source(b)]
                },
                DamagePacket {
                    player: actor,
                    target: b,
                    amount: 3,
                    sources: vec![source(a)]
                },
            ]
        );
        assert_eq!(ledger.kills.len(), 2);
        for (target, responsible) in [(a, 1 - actor), (b, actor)] {
            let kill = ledger
                .kills
                .iter()
                .find(|kill| kill.target == target)
                .unwrap();
            assert_eq!(kill.kind, KillKind::Cleanup);
            assert_eq!(
                kill.credits,
                vec![KillCredit {
                    player: responsible,
                    sources: vec![spell(parent, actor, 82)]
                }]
            );
        }
    }
}

#[test]
fn repeat_keeps_one_physical_parent_across_distinct_deals() {
    for actor in 0..2 {
        let mut g = position(actor);
        let a = unit(&mut g, 1 - actor, 8, Some(0));
        let b = unit(&mut g, 1 - actor, 8, Some(1));
        g.players[actor].hand = vec![227]; // Shock Blast: Deal4.
        g.play_grants.push(PlayGrant {
            id: 42990,
            player: actor,
            source: ChainSource::Card { card_id: 11 },
            expires_turn: g.turn,
            kind: PlayGrantKind::RepeatNextSpell,
        });
        let action = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![a],
        };
        let option = g.play_choices(&action).unwrap().repeats[0].id;
        g.apply(Action::WithPlayChoices {
            action: Box::new(action),
            choices: PlayChoices {
                repeats: vec![RepeatSelection {
                    option_id: option,
                    choices: ExecutionChoices {
                        targets: vec![b],
                        ..Default::default()
                    },
                }],
                ..Default::default()
            },
        })
        .unwrap();
        let parent = g.chain.last().unwrap().id;
        pair(&mut g);
        let ledger = g.last_damage_ledger().unwrap();
        assert_eq!(ledger.damage.len(), 2);
        for packet in &ledger.damage {
            assert_eq!(packet.player, actor);
            assert_eq!(packet.amount, 4);
            assert_eq!(packet.sources, vec![spell(parent, actor, 227)]);
        }
        assert_eq!(
            ledger.damage.iter().map(|p| p.target).collect::<Vec<_>>(),
            vec![a, b]
        );
        assert!(ledger.kills.is_empty());
    }
}

#[test]
fn prevention_zero_and_old_damage_do_not_create_false_kill_credit() {
    for actor in 0..2 {
        let mut g = position(actor);
        let target = unit(&mut g, 1 - actor, 8, Some(0));
        cast(&mut g, 227, vec![target]);
        pair(&mut g);
        assert_eq!(g.last_damage_ledger().unwrap().damage[0].amount, 4);
        g.spell_ability_damage_prevented = true;
        cast(&mut g, 227, vec![target]);
        pair(&mut g);
        assert!(g.last_damage_ledger().unwrap().damage.is_empty());
        assert_eq!(g.unit_ref(target).unwrap().damage, 4);
        // Minus4 Might makes the old damage lethal, but dealt no damage itself.
        cast(&mut g, 81, vec![target]);
        pair(&mut g);
        let ledger = g.last_damage_ledger().unwrap();
        assert!(ledger.damage.is_empty());
        assert_eq!(
            ledger.kills,
            vec![KillRecord {
                target,
                kind: KillKind::Cleanup,
                credits: vec![]
            }]
        );
    }
}

#[test]
fn direct_kill_has_spell_credit_without_inventing_damage() {
    for actor in 0..2 {
        let mut g = position(actor);
        let target = unit(&mut g, 1 - actor, 8, Some(0));
        let parent = cast(&mut g, 294, vec![target]);
        pair(&mut g);
        let ledger = g.last_damage_ledger().unwrap();
        assert!(ledger.damage.is_empty());
        assert_eq!(
            ledger.kills,
            vec![KillRecord {
                target,
                kind: KillKind::Instruction,
                credits: vec![KillCredit {
                    player: actor,
                    sources: vec![spell(parent, actor, 294)]
                }]
            }]
        );
    }
}

#[test]
fn replacement_choice_keeps_the_deal_but_never_credits_a_saved_unit_as_killed() {
    for actor in 0..2 {
        let mut g = position(actor);
        let target = unit(&mut g, 1 - actor, 1, Some(0));
        let Target::Unit { unit_id, .. } = target else {
            unreachable!()
        };
        for gear_id in [42970, 42971] {
            g.players[1 - actor].gear.push(GearState {
                id: gear_id,
                card_id: 10,
                lane: None,
                attached_to: Some(unit_id),
                ready: true,
                temporary: false,
                token: false,
            });
        }
        assert!(g.unit_might(g.unit_ref(target).unwrap()).unwrap() <= 4);
        let parent = cast(&mut g, 227, vec![target]);
        pair(&mut g);
        let Some(Decision::DeathReplacement { ref options, .. }) = g.decision else {
            panic!("{:?}", g.decision)
        };
        let chosen = options[0];
        let pending = g.pending_damage_ledger().unwrap();
        assert_eq!(pending.damage[0].sources, vec![spell(parent, actor, 227)]);
        assert!(!pending.kills.iter().any(|kill| kill.target == target));
        g.apply(Action::ChooseDeathReplacement {
            unit_id: chosen.unit_id,
            gear_id: chosen.gear_id,
        })
        .unwrap();
        assert_eq!(g.unit_ref(target).unwrap().lane, None);
        assert_eq!(g.unit_ref(target).unwrap().damage, 0);
        let last = g.last_damage_ledger().unwrap();
        assert_eq!(last.damage[0].target, target);
        assert!(!last.kills.iter().any(|kill| kill.target == target));
    }
}

#[test]
fn mass_gear_kill_retains_each_physical_victim_and_the_same_parent() {
    for actor in 0..2 {
        let mut g = position(actor);
        for (player, state) in g.players.iter_mut().enumerate() {
            state.gear.push(GearState {
                id: 42960 + player as u32,
                card_id: 232,
                lane: None,
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            });
        }
        let parent = cast(&mut g, 363, vec![]);
        pair(&mut g);
        let ledger = g.last_damage_ledger().unwrap();
        assert!(ledger.damage.is_empty());
        assert_eq!(ledger.kills.len(), 2);
        for player in 0..2 {
            assert!(ledger.kills.contains(&KillRecord {
                target: Target::Gear {
                    player,
                    gear_id: 42960 + player as u32
                },
                kind: KillKind::Instruction,
                credits: vec![KillCredit {
                    player: actor,
                    sources: vec![spell(parent, actor, 363)]
                }],
            }));
        }
    }
}

#[test]
fn salvage_and_temporary_gear_keep_their_own_kill_instructions() {
    for actor in 0..2 {
        for temporary in [false, true] {
            let mut g = position(actor);
            let owner = 1 - actor;
            let gear_id = 42965;
            let target = Target::Gear {
                player: owner,
                gear_id,
            };
            g.players[owner].gear.push(GearState {
                id: gear_id,
                card_id: 232,
                lane: None,
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            });
            let card_id = if temporary { 66 } else { 41 };
            let mut parent = cast(&mut g, card_id, vec![target]);
            pair(&mut g);
            let expected = if temporary {
                assert!(g.last_damage_ledger().unwrap().kills.is_empty());
                // The granted Temporary resolves in the owner's next Beginning.
                g.apply(Action::EndTurn).unwrap();
                let item = g.chain.last().unwrap();
                assert!(matches!(item.ability, Some(ChainAbility::Temporary { .. })));
                parent = item.id;
                let source = DamageSource::Chain {
                    chain_id: parent,
                    player: owner,
                    card_id: item.card_id,
                    ability: true,
                };
                pair(&mut g);
                KillCredit {
                    player: owner,
                    sources: vec![
                        source,
                        DamageSource::Object(ChainSource::Gear {
                            player: owner,
                            gear_id,
                        }),
                    ],
                }
            } else {
                KillCredit {
                    player: actor,
                    sources: vec![spell(parent, actor, card_id)],
                }
            };
            let ledger = g.last_damage_ledger().unwrap();
            assert!(ledger.damage.is_empty());
            assert_eq!(
                ledger.kills,
                vec![KillRecord {
                    target,
                    kind: KillKind::Instruction,
                    credits: vec![expected]
                }]
            );
            assert!(g.players[owner].gear.is_empty());
        }
    }
}

#[test]
fn gear_ability_retains_its_physical_creator_in_a_controlled_departure_fixture() {
    for actor in 0..2 {
        let mut g = position(actor);
        let target = unit(&mut g, 1 - actor, 2, Some(0));
        let gear_id = 42980;
        g.players[actor].gear.push(GearState {
            id: gear_id,
            card_id: 232,
            lane: None,
            attached_to: None,
            ready: true,
            temporary: false,
            token: false,
        });
        g.apply(Action::ActivateAbility {
            source: AbilitySource::Gear {
                player: actor,
                gear_id,
            },
            ability_index: 0,
            targets: vec![target],
        })
        .unwrap();
        let parent = g.chain.last().unwrap().id;
        assert_eq!(
            g.chain_source(g.chain.last().unwrap()),
            ChainSource::Gear {
                player: actor,
                gear_id
            }
        );
        // Controlled source departure, not a claimed legal Reaction sequence.
        // The activation and its subsequent resolution use public actions.
        assert!(g.remove_gear_from_board(actor, gear_id));
        assert!(g.players[actor].gear.is_empty());
        pair(&mut g);
        let expected = vec![
            DamageSource::Chain {
                chain_id: parent,
                player: actor,
                card_id: 232,
                ability: true,
            },
            DamageSource::Object(ChainSource::Gear {
                player: actor,
                gear_id,
            }),
        ];
        let ledger = g.last_damage_ledger().unwrap();
        assert_eq!(ledger.damage.len(), 1);
        assert_eq!(ledger.damage[0].sources, expected);
        assert_eq!(ledger.kills[0].credits[0].sources, expected);
    }
}

#[test]
fn combat_uses_both_physical_groups_and_has_no_spell_kill_credit() {
    for actor in 0..2 {
        let mut g = position(actor);
        let a = unit(&mut g, actor, 2, None);
        let b = unit(&mut g, actor, 2, None);
        let c = unit(&mut g, 1 - actor, 4, Some(0));
        let ids = [a, b].map(|target| {
            if let Target::Unit { unit_id, .. } = target {
                unit_id
            } else {
                unreachable!()
            }
        });
        g.battlefield_owner[0] = Some(1 - actor);
        g.apply(Action::MoveGroup {
            unit_ids: ids.to_vec(),
            lane: Some(0),
        })
        .unwrap();
        settle(&mut g);
        pair(&mut g);
        for _ in 0..2 {
            let Some(Decision::CombatDamage {
                player,
                lane,
                total,
            }) = g.decision
            else {
                panic!("{:?}", g.decision)
            };
            g.apply(Action::AssignDamage {
                assignments: g.automatic_damage_assignment(player, lane, total).unwrap(),
            })
            .unwrap();
        }
        let ledger = g.last_damage_ledger().unwrap();
        assert_eq!(ledger.parent, DamageParent::Combat { lane: 0 });
        assert_eq!(ledger.damage.len(), 3);
        assert_eq!(ledger.kills.len(), 3);
        for packet in &ledger.damage {
            assert_eq!(
                packet.sources,
                if packet.player == actor {
                    vec![source(a), source(b)]
                } else {
                    vec![source(c)]
                }
            );
        }
        for kill in &ledger.kills {
            assert_eq!(kill.kind, KillKind::CombatCleanup);
            assert_eq!(
                kill.credits[0].sources,
                if kill.target == c {
                    vec![source(a), source(b)]
                } else {
                    vec![source(c)]
                }
            );
        }
    }
}
