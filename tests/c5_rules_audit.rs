//! Independent examples from October9 Core437/442/443/383/478–482/827–828
//! and the official Vendetta FAQ. Fixtures establish a position; tested changes
//! then use public actions, including real responses rather than direct helpers.
use local_rift::activated::AbilitySource;
use local_rift::game::{Action, Decision, Difficulty, Game, Target, Unit};
use local_rift::gear::GearState;
use local_rift::legends::{LegendKind, LegendState};

fn position() -> Game {
    let mut game = Game::new_for_agents(3, 3, 55017, Difficulty::Tactical);
    game.start_match().unwrap();
    game.battlefields = [3, 3];
    game.battlefield_owner = [None; 2];
    for player in &mut game.players {
        player.hand.clear();
        player.champion = None;
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.discard.clear();
        player.runes = [0; 6];
        player.exhausted_runes = [0; 6];
        player.energy_pool = 50;
        player.energy = 50;
        player.power_pool = [20; 6];
        player.any_power = 0;
    }
    game.legend[0].ready = false;
    game.legend[1].ready = false;
    game
}
fn unit(g: &mut Game, owner: usize, id: u32, card_id: usize, lane: Option<usize>) -> Target {
    g.players[owner].units.push(Unit {
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
    });
    if let Some(lane) = lane {
        g.battlefield_owner[lane] = Some(owner);
    }
    Target::Unit {
        player: owner,
        unit_id: id,
    }
}
fn gear(g: &mut Game, owner: usize, id: u32, card_id: usize) -> AbilitySource {
    g.players[owner].gear.push(GearState {
        id,
        card_id,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    AbilitySource::Gear {
        player: owner,
        gear_id: id,
    }
}
fn activate(g: &mut Game, source: AbilitySource, targets: Vec<Target>) {
    g.apply(Action::ActivateAbility {
        source,
        ability_index: 0,
        targets,
    })
    .unwrap();
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
fn resolve_one(g: &mut Game) {
    assert!(g.decision.is_none(), "{:?}", g.decision);
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}
fn settle(g: &mut Game) {
    for _ in 0..100 {
        let action = match &g.decision {
            Some(Decision::OrderTriggers { options, .. }) => Some(Action::OrderTrigger {
                trigger_id: options[0].id,
            }),
            Some(Decision::TokenPlay { fixed_lane, .. }) => Some(Action::FinalizeTokenPlay {
                lane: fixed_lane.flatten(),
                accelerate: false,
            }),
            Some(Decision::PredictCards {
                choice_id, cards, ..
            }) => Some(Action::ResolvePredict {
                choice_id: *choice_id,
                recycle: vec![],
                top_order: (0..cards.len()).collect(),
            }),
            Some(other) => panic!("Unexpected decision {other:?}"),
            None if g.in_window() => Some(Action::Pass),
            None => None,
        };
        if let Some(action) = action {
            g.apply(action).unwrap();
        } else {
            return;
        }
    }
    panic!("Response windows did not finish");
}

#[test]
fn printed_empower_costs_are_paid_before_status_or_dependent_keywords_change() {
    // Core827: self-reference does not choose a target; costs precede response.
    let costs = [
        (272, 2, 1),
        (273, 7, 0),
        (274, 3, 0),
        (275, 2, 0),
        (276, 3, 0),
        (277, 2, 0),
        (278, 12, 0),
        (279, 12, 0),
        (280, 2, 0),
        (281, 6, 1),
        (282, 3, 1),
        (283, 2, 0),
        (284, 3, 0),
        (285, 1, 1),
        (286, 1, 2),
        (287, 8, 0),
        (288, 2, 1),
        (289, 1, 2),
    ];
    for (card, energy, power) in costs {
        let mut g = position();
        let source = if card == 281 {
            gear(&mut g, 0, 900, card)
        } else {
            unit(&mut g, 0, 900, card, None);
            AbilitySource::Unit {
                player: 0,
                unit_id: 900,
            }
        };
        let power_before: u16 = g.players[0].power_pool.iter().sum();
        activate(&mut g, source, vec![]);
        assert_eq!(g.players[0].energy, 50 - energy, "card {card}");
        assert_eq!(
            g.players[0].power_pool.iter().sum::<u16>(),
            power_before - power,
            "card {card}"
        );
        assert!(!g.unit_empowered(900));
        assert!(g.chain.last().unwrap().targets.is_empty());
        settle(&mut g);
        assert!(g.unit_empowered(900), "card {card}");
        let before = format!("{g:?}");
        assert!(
            g.apply(Action::ActivateAbility {
                source,
                ability_index: 0,
                targets: vec![]
            })
            .is_err()
        );
        assert_eq!(format!("{g:?}"), before);
    }
}

#[test]
fn conditional_keywords_are_live_and_never_invent_buff_counters() {
    for (card, assault, shield, deflect, ganking, bonus) in [
        (272, 3, 0, 0, false, 0),
        (273, 0, 0, 1, false, 7),
        (274, 0, 3, 1, false, 0),
        (275, 0, 0, 0, true, 1),
        (276, 0, 0, 0, true, 2),
        (277, 0, 0, 2, false, 1),
        (279, 0, 3, 1, false, 0),
        (280, 2, 0, 1, false, 0),
        (287, 0, 0, 2, false, 0),
    ] {
        let mut g = position();
        unit(&mut g, 0, 900, card, None);
        let printed = g.catalog[card].might;
        let before = g.unit_keywords(&g.players[0].units[0]);
        assert_eq!(before.assault, 0);
        assert_eq!(before.shield, 0);
        assert!(!before.ganking);
        assert_eq!(
            before.deflect,
            if card == 273 {
                1
            } else if card == 287 {
                2
            } else {
                0
            }
        );
        activate(
            &mut g,
            AbilitySource::Unit {
                player: 0,
                unit_id: 900,
            },
            vec![],
        );
        settle(&mut g);
        let u = &g.players[0].units[0];
        let k = g.unit_keywords(u);
        assert_eq!(
            (k.assault, k.shield, k.deflect, k.ganking),
            (assault, shield, deflect, ganking)
        );
        assert_eq!(g.unit_might(u), printed + bonus);
        assert_eq!((u.buff, u.temporary_buff), (0, 0));
    }
}

#[test]
fn aura_loss_repeats_simultaneous_cleanup_before_deathknell_finalization() {
    // Core321/322/383.2.c: losing Rage first kills Aurok and Viktor together;
    // then Aurok's departure kills Emissary. Viktor sees neither death.
    let mut g = position();
    gear(&mut g, 0, 900, 281);
    unit(&mut g, 0, 901, 282, None);
    unit(&mut g, 0, 902, 176, None);
    unit(&mut g, 0, 903, 285, None);
    g.empowered_units.extend([901, 903]);
    g.players[0].units[0].damage = 7;
    g.players[0].units[1].damage = 4;
    g.players[0].units[2].damage = 3;
    for u in &g.players[0].units {
        assert!(u.damage < g.unit_might(u));
    }
    cast(
        &mut g,
        41,
        vec![Target::Gear {
            player: 0,
            gear_id: 900,
        }],
    );
    resolve_one(&mut g);
    assert!(
        g.players[0].units.is_empty(),
        "all cleanup passes precede triggered tokens"
    );
    assert_eq!(g.chain.len(), 1);
    assert_eq!(
        g.chain[0].card_id, 285,
        "only Emissary's own dependent Deathknell"
    );
    settle(&mut g);
    assert_eq!(g.players[0].units.len(), 2);
    assert!(
        g.players[0]
            .units
            .iter()
            .all(|u| u.token && g.unit_name(u) == "Recruit token")
    );
    for id in [281, 282, 176, 285] {
        assert_eq!(g.players[0].discard.iter().filter(|&&c| c == id).count(), 1);
    }
}

#[test]
fn formula_empowering_enemy_gear_triggers_the_actors_legend_and_noop_does_not() {
    // Core412.4/442: controller of affected object is not the action's actor.
    let mut g = position();
    g.legend[0] = LegendState::from_legend(7).unwrap();
    g.legend[1] = LegendState::from_legend(7).unwrap();
    let formula = gear(&mut g, 0, 900, 243);
    gear(&mut g, 1, 901, 281);
    activate(
        &mut g,
        formula,
        vec![Target::Gear {
            player: 1,
            gear_id: 901,
        }],
    );
    assert!(!g.unit_empowered(901));
    resolve_one(&mut g);
    assert!(g.unit_empowered(901));
    assert!(
        !g.legend[0].empowered,
        "Matriarch itself is a respondable trigger"
    );
    assert_eq!(g.chain.len(), 1);
    assert_eq!(g.chain[0].player, 0);
    settle(&mut g);
    assert!(g.legend[0].empowered);
    assert!(!g.legend[1].empowered);
    let second = gear(&mut g, 0, 902, 243);
    activate(
        &mut g,
        second,
        vec![Target::Gear {
            player: 1,
            gear_id: 901,
        }],
    );
    resolve_one(&mut g);
    assert!(
        g.chain.is_empty(),
        "already empowered is not a second event"
    );
    assert_eq!(g.legend[0].kind, LegendKind::Ambessa);
}

#[test]
fn matriarch_ready_pays_disempower_exhaust_and_enemy_deflect_before_resolution() {
    let mut g = position();
    g.legend[0] = LegendState::from_legend(7).unwrap();
    g.legend[0].empowered = true;
    let target = unit(&mut g, 1, 900, 287, None); // printed Deflect2 even unempowered
    g.players[1].units[0].ready = false;
    let total = g.players[0].power_pool.iter().sum::<u16>();
    g.apply(Action::ActivateLegend {
        target: Some(target),
    })
    .unwrap();
    assert!(!g.legend[0].ready);
    assert!(!g.legend[0].empowered);
    assert_eq!(g.players[0].power_pool.iter().sum::<u16>(), total - 3);
    assert!(!g.players[1].units[0].ready);
    settle(&mut g);
    assert!(g.players[1].units[0].ready);
}

#[test]
fn rune_discount_counts_exhausted_controlled_runes_and_freezes_before_recycling() {
    use local_rift::cards::Domain;
    for (card, count, expected) in [
        (278, 0, 12),
        (278, 5, 7),
        (278, 12, 0),
        (279, 5, 7),
        (280, 4, 2),
        (280, 5, 5),
    ] {
        let mut g = position();
        unit(&mut g, 0, 900, card, None);
        g.players[0].runes = [count, 0, 0, 0, 0, 0];
        g.players[0].exhausted_runes = [count, 0, 0, 0, 0, 0];
        let action = Action::ActivateAbility {
            source: AbilitySource::Unit {
                player: 0,
                unit_id: 900,
            },
            ability_index: 0,
            targets: vec![],
        };
        assert_eq!(g.cost_plan(&action).unwrap().energy, expected);
        g.apply(Action::BeginPayment {
            action: Box::new(action),
        })
        .unwrap();
        assert_eq!(g.payment_plan_view(0).unwrap().energy, expected);
        if count > 0 {
            g.apply(Action::RecycleRune {
                domain: Domain::Fury,
                exhausted: true,
            })
            .unwrap();
        }
        assert_eq!(g.payment_plan_view(0).unwrap().energy, expected);
        g.apply(Action::CompletePayment).unwrap();
        assert_eq!(g.players[0].energy_pool, 50 - expected as u16);
        settle(&mut g);
        assert!(g.unit_empowered(900));
    }
}

#[test]
fn apprentice_predict_two_keeps_occurrence_order_and_never_burns_out_short_decks() {
    // Core437: all available cards looked at once, any subset recycled, rest ordered.
    for deck in [vec![], vec![6], vec![6, 8], vec![6, 6]] {
        let n = deck.len();
        let choices = match n {
            0 => vec![(vec![], vec![])],
            1 => vec![(vec![], vec![0]), (vec![0], vec![])],
            _ => vec![
                (vec![], vec![0, 1]),
                (vec![], vec![1, 0]),
                (vec![0], vec![1]),
                (vec![1], vec![0]),
                (vec![0, 1], vec![]),
            ],
        };
        for (recycle, top_order) in choices {
            let mut g = position();
            unit(&mut g, 0, 900, 283, None);
            g.players[0].deck = deck.clone();
            let hand = g.players[0].hand.clone();
            let scores = [g.players[0].score, g.players[1].score];
            activate(
                &mut g,
                AbilitySource::Unit {
                    player: 0,
                    unit_id: 900,
                },
                vec![],
            );
            for _ in 0..8 {
                if g.decision.is_some() || !g.in_window() {
                    break;
                }
                g.apply(Action::Pass).unwrap();
            }
            if n == 0 {
                assert!(g.decision.is_none());
            } else {
                let Decision::PredictCards {
                    choice_id, cards, ..
                } = g.decision.clone().unwrap()
                else {
                    panic!()
                };
                assert_eq!(cards, deck.iter().rev().copied().collect::<Vec<_>>());
                assert_eq!(g.players[0].deck, deck);
                let expected: Vec<_> = top_order.iter().map(|&i| cards[i]).collect();
                g.apply(Action::ResolvePredict {
                    choice_id,
                    recycle,
                    top_order,
                })
                .unwrap();
                assert_eq!(
                    g.players[0]
                        .deck
                        .iter()
                        .rev()
                        .take(expected.len())
                        .copied()
                        .collect::<Vec<_>>(),
                    expected
                );
            }
            settle(&mut g);
            assert_eq!(g.players[0].deck.len(), n);
            assert_eq!(g.players[0].hand, hand);
            assert_eq!([g.players[0].score, g.players[1].score], scores);
        }
    }
}

#[test]
fn akali_keeps_event_battlefield_after_returning_to_base_and_leaving_board() {
    // Vendetta FAQ specifically retains Akali's moved-to/from location after
    // removal. Her live Empowered clause is separate from that saved event.
    for removed in [false, true] {
        let mut g = position();
        g.battlefields = [4, 4]; // No Void Gate bonus in this source-local comparison.
        let akali = unit(&mut g, 0, 900, 288, Some(0));
        let victim = unit(&mut g, 1, 901, 69, Some(0));
        let other = unit(&mut g, 1, 902, 69, None);
        g.empowered_units.push(900);
        g.apply(Action::Move {
            unit_id: 900,
            lane: None,
        })
        .unwrap();
        assert!(g.valid_trigger_target(victim, &[]));
        assert!(!g.valid_trigger_target(other, &[]));
        g.apply(Action::ChooseTriggerTargets {
            targets: vec![victim],
        })
        .unwrap();
        if removed {
            cast(&mut g, 16, vec![akali, other]);
            resolve_one(&mut g);
        }
        settle(&mut g);
        assert_eq!(
            g.players[1]
                .units
                .iter()
                .find(|u| u.id == 901)
                .unwrap()
                .damage,
            if removed { 1 } else { 2 }
        );
        assert_eq!(g.players[0].units.iter().any(|u| u.id == 900), !removed);
    }
}

#[test]
fn witherclaw_deathknell_uses_predeath_empowered_status_and_channels_only_available_runes() {
    use local_rift::cards::Domain;
    for count in 0..=2 {
        let mut g = position();
        let target = unit(&mut g, 0, 900, 286, Some(0));
        g.players[0].rune_deck = vec![Domain::Order; count];
        activate(
            &mut g,
            AbilitySource::Unit {
                player: 0,
                unit_id: 900,
            },
            vec![],
        );
        settle(&mut g);
        let score = g.players[1].score;
        let hand = g.players[0].hand.len();
        cast(&mut g, 53, vec![target, target]);
        settle(&mut g);
        assert!(!g.players[0].units.iter().any(|u| u.id == 900));
        assert!(!g.unit_empowered(900));
        assert_eq!(g.players[0].runes[5], count as u8);
        assert_eq!(g.players[0].exhausted_runes[5], count as u8);
        assert!(g.players[0].rune_deck.is_empty());
        assert_eq!(g.players[1].score, score);
        assert_eq!(g.players[0].hand.len(), hand);
    }
}
