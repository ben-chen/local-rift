//! Reviewed 103–151 unit paths; these focused positions are not full-pool conformance.
use local_rift::game::{Action, ChainAbility, Decision, Difficulty, Game, Showdown, Target, Unit};

fn position() -> Game {
    let mut g = Game::new(3, 3, 103151, Difficulty::Tactical);
    g.start_match().unwrap();
    g.battlefields = [0, 0];
    g.battlefield_owner = [None; 2];
    for p in &mut g.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.discard.clear();
        p.runes = [12; 6];
        p.exhausted_runes = [0; 6];
        p.energy_pool = 0;
        p.energy = 72;
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
fn cast(g: &mut Game, id: usize) {
    let p = g.priority();
    let i = g.players[p].hand.len();
    g.players[p].hand.push(id);
    g.apply(Action::PlayCardWithTargets {
        hand_index: i,
        lane: None,
        targets: vec![],
    })
    .unwrap();
}
fn resolve(g: &mut Game) {
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn settle(g: &mut Game) {
    for _ in 0..60 {
        assert!(g.decision.is_none(), "{:?}", g.decision);
        if g.chain.is_empty() {
            return;
        }
        g.apply(Action::Pass).unwrap();
    }
    panic!("did not settle");
}

#[test]
fn shield_is_defender_might_and_does_not_change_assignment_priority() {
    for (id, shield, tank) in [
        (104, 2, false),
        (107, 2, true),
        (109, 2, true),
        (110, 1, false),
        (111, 2, false),
        (112, 1, true),
        (113, 2, true),
        (115, 3, false),
        (116, 1, false),
    ] {
        let mut g = position();
        g.players[0].units.push(unit(100, id, Some(0)));
        let base = g.catalog[id].might;
        assert_eq!(g.unit_might(&g.players[0].units[0]), base);
        assert_eq!(g.unit_keywords(&g.players[0].units[0]).shield, shield);
        assert_eq!(
            g.damage_priority(&g.players[0].units[0]),
            if tank { 0 } else { 1 }
        );
        g.showdown = Some(Showdown {
            lane: 0,
            attacker: 1,
            focus: 1,
            passes: 0,
            combat: false,
        });
        assert_eq!(
            g.unit_might(&g.players[0].units[0]),
            base,
            "neutral showdown isn't defense"
        );
        g.showdown.as_mut().unwrap().combat = true;
        assert_eq!(g.unit_might(&g.players[0].units[0]), base + shield);
        g.showdown.as_mut().unwrap().attacker = 0;
        assert_eq!(
            g.unit_might(&g.players[0].units[0]),
            base + g.unit_keywords(&g.players[0].units[0]).assault
        );
    }
}

#[test]
fn shield_counts_toward_lethal_damage_assignment() {
    let mut g = position();
    g.players[0].units.push(unit(100, 72, Some(0)));
    g.players[1].units = vec![unit(101, 107, Some(0)), unit(102, 70, Some(0))];
    g.showdown = Some(Showdown {
        lane: 0,
        attacker: 0,
        focus: 0,
        passes: 0,
        combat: true,
    });
    g.players[0].units[0].temporary_buff = 4 - g.catalog[72].might as i16;
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::CombatDamage {
            player: 0,
            total: 4,
            ..
        })
    ));
    // Tank has printed 1 plus Shield 2; one damage is not lethal.
    assert!(
        g.apply(Action::AssignDamage {
            assignments: vec![(101, 1), (102, 3)]
        })
        .is_err()
    );
    g.apply(Action::AssignDamage {
        assignments: vec![(101, 3), (102, 1)],
    })
    .unwrap();
}

#[test]
fn intrinsic_ready_entry_does_not_trigger_irelias_ready_event() {
    for id in [103, 106, 114, 115] {
        let mut g = position();
        cast(&mut g, id);
        assert!(g.players[0].units[0].ready);
        assert!(g.chain.is_empty());
    }
}

#[test]
fn battering_ram_discount_counts_finalized_countered_cards_and_has_a_floor() {
    let mut g = position();
    g.players[0].units.push(unit(100, 70, None));
    g.players[0].hand.push(7);
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player: 0,
            unit_id: 100,
        }],
    })
    .unwrap();
    let spell = g.chain.last().unwrap().id;
    g.apply(Action::Pass).unwrap();
    g.players[1].hand.push(86);
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Spell { chain_id: spell }],
    })
    .unwrap();
    settle(&mut g);
    let before = g.players[0].energy;
    cast(&mut g, 131);
    assert_eq!(before - g.players[0].energy, 4);
    for _ in 0..5 {
        cast(&mut g, 69);
    }
    let before = g.players[0].energy;
    cast(&mut g, 131);
    assert_eq!(before - g.players[0].energy, 1);
}

#[test]
fn appended_targeted_play_triggers_choose_after_entry_and_allow_self() {
    for (id, amount) in [(141, 3), (142, 8), (143, 2), (146, -2)] {
        let mut g = position();
        cast(&mut g, id);
        let source = g.players[0].units[0].id;
        assert!(matches!(g.decision,Some(Decision::OnPlayTarget{card_id,..})if card_id==id));
        g.apply(Action::ChooseTarget {
            target: Some(Target::Unit {
                player: 0,
                unit_id: source,
            }),
        })
        .unwrap();
        assert_eq!(g.players[0].units[0].temporary_buff, 0);
        resolve(&mut g);
        let expected =
            (g.catalog[id].might as i16 + amount).max(if id == 146 { 1 } else { 0 }) as u8;
        assert_eq!(g.unit_might(&g.players[0].units[0]), expected);
    }
}

#[test]
fn rex_and_mage_enforce_enemy_and_battlefield_restrictions() {
    for id in [144, 145] {
        let mut g = position();
        g.players[1].units = vec![unit(100, 72, None), unit(101, 72, Some(0))];
        cast(&mut g, id);
        let source = g.players[0].units[0].id;
        assert!(
            g.apply(Action::ChooseTarget {
                target: Some(Target::Unit {
                    player: 0,
                    unit_id: source
                })
            })
            .is_err()
        );
        if id == 144 {
            assert!(
                g.apply(Action::ChooseTarget {
                    target: Some(Target::Unit {
                        player: 1,
                        unit_id: 100
                    })
                })
                .is_err()
            );
        }
        g.apply(Action::ChooseTarget {
            target: Some(Target::Unit {
                player: 1,
                unit_id: 101,
            }),
        })
        .unwrap();
        resolve(&mut g);
        if id == 144 {
            assert!(!g.players[1].units.iter().any(|u| u.id == 101));
        } else {
            assert_eq!(
                g.players[1]
                    .units
                    .iter()
                    .find(|u| u.id == 101)
                    .unwrap()
                    .temporary_buff,
                -3
            );
        }
    }
}

#[test]
fn cloud_drake_and_evershade_use_respondable_draw_and_loot() {
    let mut g = position();
    let deck = g.deck_count(0);
    cast(&mut g, 139);
    assert_eq!(g.deck_count(0), deck);
    resolve(&mut g);
    assert_eq!(g.deck_count(0), deck - 1);
    cast(&mut g, 140);
    resolve(&mut g);
    assert!(matches!(
        g.decision,
        Some(Decision::Discard {
            count: 1,
            draw_after: 1,
            ..
        })
    ));
    assert!(matches!(
        g.chain.last().unwrap().ability,
        Some(ChainAbility::CardEffect { .. })
    ));
    g.apply(Action::SelectCards { indices: vec![0] }).unwrap();
    assert!(g.chain.is_empty());
}

#[test]
fn appended_here_tokens_follow_live_source_and_keep_distinct_token_identity() {
    for id in [147, 148] {
        let mut g = position();
        cast(&mut g, id);
        let source = g.players[0].units[0].id;
        // Move the source with a responding effect; the token uses current Here.
        g.players[0].units[0].lane = Some(0);
        g.battlefield_owner[0] = Some(0);
        resolve(&mut g);
        let token = g.players[0].units.iter().find(|u| u.token).unwrap();
        assert_eq!(token.lane, Some(0));
        assert_eq!(token.card_id, id);
        assert_eq!(
            g.unit_keywords(token).deflect,
            if id == 148 { 1 } else { 0 }
        );
        assert!(g.players[0].units.iter().any(|u| u.id == source));
    }
}

#[test]
fn missing_here_source_produces_no_token() {
    for id in [147, 148] {
        let mut g = position();
        cast(&mut g, id);
        g.players[0].units.clear();
        resolve(&mut g);
        assert!(g.players[0].units.is_empty());
        assert!(g.pending_tokens().is_empty());
    }
}

#[test]
fn trove_golem_plays_four_exhausted_gold_without_unit_tokens() {
    let mut g = position();
    cast(&mut g, 151);
    assert!(g.players[0].gear.is_empty());
    resolve(&mut g);
    assert_eq!(g.players[0].gear.len(), 4);
    assert!(g.players[0].gear.iter().all(|x| x.token && !x.ready));
    assert_eq!(g.players[0].units.len(), 1);
}

#[test]
fn appended_deathknells_use_pending_gold_and_bird_factories() {
    for id in [149, 150] {
        let mut g = position();
        g.players[0].units.push(unit(100, id, None));
        g.players[0].hand.push(74);
        g.players[0].units[0].lane = Some(0);
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: 0,
                unit_id: 100,
            }],
        })
        .unwrap();
        if id == 150 {
            g.players[0].units[0].damage = 1;
        }
        resolve(&mut g);
        assert!(g.players[0].discard.contains(&id));
        assert!(g.players[0].gear.is_empty());
        resolve(&mut g);
        if id == 149 {
            assert!(g.players[0].gear[0].token && !g.players[0].gear[0].ready);
        } else {
            let bird = &g.players[0].units[0];
            assert!(bird.token && bird.lane.is_none());
            assert_eq!(g.unit_keywords(bird).deflect, 1);
        }
    }
}

#[test]
fn hidden_blastcone_fae_targets_its_entry_field_and_rechecks_source_and_target() {
    for remove_source in [false, true] {
        let mut g = position();
        g.players[0].units = vec![unit(100, 70, Some(0)), unit(101, 70, Some(1))];
        g.battlefield_owner = [Some(0), Some(0)];
        g.players[0].hand = vec![146];
        g.apply(Action::Hide {
            hand_index: 0,
            lane: 0,
        })
        .unwrap();
        let hidden = g.players[0].hidden[0].id;
        g.turn += 1;
        g.apply(Action::PlayHidden {
            hidden_id: hidden,
            targets: vec![],
        })
        .unwrap();
        let fae = g.players[0].units.last().unwrap().id;
        assert_eq!(g.hidden_play_origin(fae), Some(0));
        assert!(
            g.apply(Action::ChooseTarget {
                target: Some(Target::Unit {
                    player: 0,
                    unit_id: 101
                })
            })
            .is_err()
        );
        g.apply(Action::ChooseTarget {
            target: Some(Target::Unit {
                player: 0,
                unit_id: 100,
            }),
        })
        .unwrap();
        // Either removing the source or moving the target out makes the play
        // effect mistarget under CR811.1.d.3/.a, without cancelling the ability.
        let response = if remove_source { 11 } else { 9 };
        let target = if remove_source { fae } else { 100 };
        g.players[0].hand.push(response);
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: 0,
                unit_id: target,
            }],
        })
        .unwrap();
        resolve(&mut g);
        resolve(&mut g);
        assert_eq!(
            g.players[0]
                .units
                .iter()
                .find(|u| u.id == 100)
                .unwrap()
                .temporary_buff,
            0
        );
    }
}

#[test]
fn hidden_tornado_empower_mistargets_after_its_source_leaves() {
    let mut g = position();
    g.players[0].units.push(unit(100, 70, Some(0)));
    g.battlefield_owner[0] = Some(0);
    g.players[0].hand = vec![46];
    g.apply(Action::Hide {
        hand_index: 0,
        lane: 0,
    })
    .unwrap();
    let hidden = g.players[0].hidden[0].id;
    g.turn += 1;
    g.apply(Action::PlayHidden {
        hidden_id: hidden,
        targets: vec![],
    })
    .unwrap();
    let tornado = g.players[0].units.last().unwrap().id;
    assert!(matches!(g.decision,Some(Decision::EmpowerHere{source_id:Some(id),..})if id==tornado));
    g.apply(Action::ChooseTarget {
        target: Some(Target::Unit {
            player: 0,
            unit_id: 100,
        }),
    })
    .unwrap();
    g.players[0].hand.push(11);
    g.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player: 0,
            unit_id: tornado,
        }],
    })
    .unwrap();
    resolve(&mut g);
    resolve(&mut g);
    assert!(!g.unit_empowered(100));
}
