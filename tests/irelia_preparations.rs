//! Battle Preparations: finalization snapshots and independently chosen unit/runes.
use local_rift::game::{Action, Decision, Difficulty, Game, Target, Unit};

use local_rift::cards::Domain;
use local_rift::game::domain_index;
use local_rift::gear::GearState;
use local_rift::play::{ExecutionChoices, PlayChoices, PlayGrant, PlayGrantKind, RepeatSelection};
use local_rift::show_off::{ShowOffObject, ShowOffSelection};
const PREP: usize = 426;
fn position(owner: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 113421, Difficulty::Tactical);
    g.choose_starting_player(owner).unwrap();
    g.start_match().unwrap();
    g.battlefields = [2, 2];
    g.battlefield_owner = [None; 2];
    for p in &mut g.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.discard.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 50;
        p.energy = 50;
        p.power_pool = [20; 6];
    }
    g
}
fn unit(g: &mut Game, player: usize, id: u32, card_id: usize, lane: Option<usize>) -> Target {
    g.players[player].units.push(Unit {
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
    Target::Unit {
        player,
        unit_id: id,
    }
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

fn settle(g: &mut Game) {
    for _ in 0..80 {
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
    panic!("Chain did not settle");
}
fn reject(g: &mut Game, action: Action) {
    let before = format!("{g:?}");
    assert!(g.apply(action).is_err());
    assert_eq!(format!("{g:?}"), before);
}

fn gear(g: &mut Game, player: usize, token: bool, attached: bool) -> u32 {
    let id = 700 + player as u32;
    g.players[player].gear.push(GearState {
        id,
        card_id: if token { 219 } else { 58 },
        lane: None,
        attached_to: attached.then_some(500),
        ready: true,
        temporary: false,
        token,
    });
    id
}
fn runes(g: &mut Game, owner: usize, exhausted: u8) -> Vec<Target> {
    let d = domain_index(Domain::Calm);
    let mut counts = [0; 6];
    let mut used = [0; 6];
    counts[d] = 3;
    used[d] = exhausted;
    g.players[owner].set_rune_setup(counts, used).unwrap();
    g.players[owner]
        .rune_objects()
        .iter()
        .map(|r| Target::Rune {
            player: owner,
            rune_id: r.id,
        })
        .collect()
}
fn draft(targets: Vec<Target>, object: Option<ShowOffObject>) -> Action {
    Action::WithPlayChoices {
        action: Box::new(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets,
        }),
        choices: PlayChoices {
            show_off: object
                .map(|object| ShowOffSelection {
                    instance: 0,
                    object,
                })
                .into_iter()
                .collect(),
            ..Default::default()
        },
    }
}
fn setup(owner: usize) -> (Game, Target, Vec<Target>) {
    let mut g = position(owner);
    let u = unit(&mut g, owner, 500, 73, None);
    let r = runes(&mut g, owner, 3);
    g.players[owner].hand = vec![PREP, 58, 73];
    (g, u, r)
}
#[test]
fn optional_gear_and_zero_one_two_runes_in_both_seats() {
    for owner in 0..2 {
        for count in 0..=2 {
            for show in [false, true] {
                let (mut g, u, r) = setup(owner);
                let before = g.unit_might(&g.players[owner].units[0]).unwrap();
                let mut targets = vec![u];
                if show {
                    targets.extend_from_slice(&r[..count]);
                }
                g.apply(draft(
                    targets,
                    show.then_some(ShowOffObject::Hand { hand_index: 1 }),
                ))
                .unwrap();
                assert_eq!(g.chain.last().unwrap().show_off.len(), usize::from(show));
                assert_eq!(g.players[owner].hand, vec![58, 73]);
                settle(&mut g);
                assert_eq!(
                    g.unit_might(&g.players[owner].units[0]).unwrap(),
                    before + 2
                );
                assert_eq!(
                    g.players[owner]
                        .rune_objects()
                        .iter()
                        .filter(|r| r.ready)
                        .count(),
                    if show { count } else { 0 }
                );
            }
        }
    }
}
#[test]
fn malformed_show_off_and_rune_choices_are_atomic() {
    let (mut g, u, r) = setup(0);
    for action in [
        draft(vec![u, r[0]], None),
        draft(vec![u], Some(ShowOffObject::Hand { hand_index: 0 })),
        draft(vec![u], Some(ShowOffObject::Hand { hand_index: 2 })),
        draft(vec![u], Some(ShowOffObject::Hand { hand_index: 999 })),
        draft(vec![u], Some(ShowOffObject::Gear { gear_id: 999 })),
        draft(
            vec![u, r[0], r[0]],
            Some(ShowOffObject::Hand { hand_index: 1 }),
        ),
        draft(
            vec![u, r[0], r[1], r[2]],
            Some(ShowOffObject::Hand { hand_index: 1 }),
        ),
        draft(vec![r[0]], Some(ShowOffObject::Hand { hand_index: 1 })),
        draft(vec![u, u], Some(ShowOffObject::Hand { hand_index: 1 })),
    ] {
        reject(&mut g, action);
    }
    let enemy = gear(&mut g, 1, false, false);
    reject(
        &mut g,
        draft(vec![u], Some(ShowOffObject::Gear { gear_id: enemy })),
    );
    for selections in [
        vec![ShowOffSelection {
            instance: 1,
            object: ShowOffObject::Hand { hand_index: 1 },
        }],
        vec![
            ShowOffSelection {
                instance: 0,
                object: ShowOffObject::Hand { hand_index: 1 }
            };
            2
        ],
    ] {
        let mut a = draft(vec![u], None);
        if let Action::WithPlayChoices { choices, .. } = &mut a {
            choices.show_off = selections;
        }
        reject(&mut g, a);
    }
}
#[test]
fn board_attached_and_gold_gear_can_be_shown_without_targeting_them() {
    for token in [false, true] {
        for attached in [false, true] {
            let (mut g, u, r) = setup(0);
            let id = gear(&mut g, 0, token, attached);
            g.apply(draft(
                vec![u, r[0]],
                Some(ShowOffObject::Gear { gear_id: id }),
            ))
            .unwrap();
            let item = g.chain.last().unwrap();
            assert_eq!(item.targets, vec![u, r[0]]);
            assert_eq!(item.show_off[0].token, token);
            g.players[0].gear.clear();
            settle(&mut g);
            assert!(g.players[0].rune_objects()[0].ready);
        }
    }
}
#[test]
fn both_players_runes_and_enemy_unit_are_legal() {
    let (mut g, _, r) = setup(0);
    let enemy = unit(&mut g, 1, 501, 73, None);
    let er = runes(&mut g, 1, 3);
    g.apply(draft(
        vec![enemy, r[0], er[0]],
        Some(ShowOffObject::Hand { hand_index: 1 }),
    ))
    .unwrap();
    settle(&mut g);
    assert_eq!(g.players[1].units[0].temporary_buff, 2);
    assert!(g.players[0].rune_objects()[0].ready && g.players[1].rune_objects()[0].ready);
}
#[test]
fn targets_are_independent_and_new_runes_do_not_replace_old_ones() {
    let (mut g, u, r) = setup(0);
    let original = g.clone();
    g.apply(draft(
        vec![u, r[0], r[1]],
        Some(ShowOffObject::Hand { hand_index: 1 }),
    ))
    .unwrap();
    g.players[0].units.clear();
    settle(&mut g);
    assert!(g.players[0].rune_objects()[0].ready);
    let mut g = original;
    g.apply(draft(
        vec![u, r[0], r[1]],
        Some(ShowOffObject::Hand { hand_index: 1 }),
    ))
    .unwrap();
    runes(&mut g, 0, 3);
    settle(&mut g);
    assert_eq!(g.players[0].units[0].temporary_buff, 2);
    assert!(g.players[0].rune_objects().iter().all(|r| !r.ready));
}
#[test]
fn payment_can_spend_shown_gold_and_ready_runes_used_for_energy() {
    for owner in 0..2 {
        let (mut g, u, _) = setup(owner);
        let r = runes(&mut g, owner, 0);
        let id = gear(&mut g, owner, true, false);
        g.players[owner].energy_pool = 0;
        g.players[owner].energy = 3;
        let a = draft(
            vec![u, r[0], r[1]],
            Some(ShowOffObject::Gear { gear_id: id }),
        );
        g.apply(Action::BeginPayment {
            action: Box::new(a),
        })
        .unwrap();
        assert_eq!(g.payment_plan_view(1 - owner).unwrap().show_off.len(), 1);
        g.apply(Action::ActivateGear { gear_id: id }).unwrap();
        assert!(g.players[owner].gear.is_empty());
        g.apply(Action::CompletePayment).unwrap();
        assert_eq!(g.chain.last().unwrap().show_off.len(), 1);
        settle(&mut g);
        assert_eq!(
            g.players[owner]
                .rune_objects()
                .iter()
                .filter(|r| r.ready)
                .count(),
            3
        );
    }
}
#[test]
fn recycling_a_chosen_rune_during_payment_requires_undo() {
    let (mut g, u, r) = setup(0);
    let before = format!("{g:?}");
    g.apply(Action::BeginPayment {
        action: Box::new(draft(
            vec![u, r[0]],
            Some(ShowOffObject::Hand { hand_index: 1 }),
        )),
    })
    .unwrap();
    let Target::Rune { rune_id, .. } = r[0] else {
        unreachable!()
    };
    g.apply(Action::ActivateRuneObject {
        rune_id,
        recycle: true,
    })
    .unwrap();
    reject(&mut g, Action::CompletePayment);
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(format!("{g:?}"), before);
}
#[test]
fn repeated_instructions_share_show_off_but_choose_their_own_runes() {
    let (mut g, u, r) = setup(0);
    let second = unit(&mut g, 0, 501, 73, None);
    g.play_grants.push(PlayGrant {
        id: 8000,
        player: 0,
        source: local_rift::game::ChainSource::Card { card_id: 426 },
        expires_turn: g.turn,
        kind: PlayGrantKind::RepeatNextSpell,
    });
    let mut action = draft(vec![u, r[0]], Some(ShowOffObject::Hand { hand_index: 1 }));
    let option = g.play_choices(&action).unwrap().repeats[0].id;
    if let Action::WithPlayChoices { choices, .. } = &mut action {
        choices.repeats.push(RepeatSelection {
            option_id: option,
            choices: ExecutionChoices {
                targets: vec![second, r[1], r[2]],
                mode: None,
            },
        });
    }
    g.apply(action).unwrap();
    assert_eq!(g.chain.last().unwrap().show_off.len(), 1);
    settle(&mut g);
    assert!(g.players[0].units.iter().all(|u| u.temporary_buff == 2));
    assert!(g.players[0].rune_objects().iter().all(|r| r.ready));
}
#[test]
fn previews_are_readonly_and_describe_ordered_optional_slots() {
    let (g, u, r) = setup(0);
    let before = format!("{g:?}");
    for show in [false, true] {
        let a = draft(
            vec![u],
            show.then_some(ShowOffObject::Hand { hand_index: 1 }),
        );
        let view = g.play_target_view(&a, 0).unwrap();
        assert_eq!(view.spec.maximum, if show { 3 } else { 1 });
        assert_eq!(view.candidates.contains(&r[0]), show);
        assert_eq!(g.play_choices(&a).unwrap().show_off[0].candidates.len(), 1);
    }
    assert_eq!(format!("{g:?}"), before);
}
#[test]
fn countering_preparations_prevents_both_effects() {
    let (mut g, u, r) = setup(0);
    g.apply(draft(
        vec![u, r[0]],
        Some(ShowOffObject::Hand { hand_index: 1 }),
    ))
    .unwrap();
    let chain_id = g.chain.last().unwrap().id;
    g.apply(Action::Pass).unwrap();
    cast(&mut g, 86, vec![Target::Spell { chain_id }]);
    resolve(&mut g);
    settle(&mut g);
    assert_eq!(g.players[0].units[0].temporary_buff, 0);
    assert!(!g.players[0].rune_objects()[0].ready);
}

#[test]
fn preparations_forty_card_irelia_finishes_all_presets_seats_starters_and_levels() {
    let mut observed = std::collections::BTreeSet::new();
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for opponent in 0..local_rift::cards::presets().len() {
            for seat in 0..2 {
                for starter in 0..2 {
                    let mut practice = local_rift::deck::CustomDeck::from_preset(0).unwrap();
                    practice.name = "Irelia 40-card priority verification".into();
                    // Authored sideboard-style substitutions; the stored event list is unchanged.
                    for (card, count) in [(6, 3), (17, 3), (19, 1), (11, 1), (1, 1)] {
                        let entry = practice.cards.iter_mut().find(|x| x.0 == card).unwrap();
                        assert_eq!(entry.1, count);
                        entry.1 = 0;
                    }
                    practice.cards.retain(|x| x.1 > 0);
                    practice
                        .cards
                        .extend([(421, 3), (422, 2), (423, 1), (PREP, 3)]);
                    assert_eq!(practice.card_count(), 40);
                    practice.validate().unwrap();
                    let mut presets = [0, opponent];
                    let mut custom = [Some(practice), None];
                    if seat == 1 {
                        presets.swap(0, 1);
                        custom.swap(0, 1);
                    }
                    let seed = 426_000 + (level * 100 + opponent * 4 + seat * 2 + starter) as u64;
                    let mut g =
                        Game::new_with_custom(presets, custom, seed, difficulty, false).unwrap();
                    g.choose_starting_player(starter).unwrap();
                    g.start_match().unwrap();
                    for step in 0..2400 {
                        if g.winner.is_some() {
                            break;
                        }
                        for item in &g.chain {
                            if [421, 422, 423, PREP].contains(&item.card_id) {
                                observed.insert(item.card_id);
                            }
                        }
                        assert!(g.step_ai().unwrap_or_else(|e|panic!("{difficulty:?} opponent{opponent} seat{seat} starter{starter} step{step}: {e}")));
                    }
                    assert!(
                        g.winner.is_some(),
                        "{difficulty:?}/{opponent}/{seat}/{starter}"
                    );
                    let p = &g.players[seat];
                    for (card, expected) in [(421, 3), (422, 2), (423, 1), (PREP, 3)] {
                        let copies = p
                            .deck
                            .iter()
                            .chain(&p.hand)
                            .chain(&p.discard)
                            .chain(&p.banish)
                            .filter(|&&id| id == card)
                            .count()
                            + p.units
                                .iter()
                                .filter(|u| !u.token && u.card_id == card)
                                .count()
                            + p.hidden.iter().filter(|h| h.card_id == card).count()
                            + g.chain
                                .iter()
                                .filter(|item| {
                                    item.player == seat
                                        && item.ability.is_none()
                                        && item.card_id == card
                                })
                                .count();
                        assert_eq!(
                            copies, expected,
                            "card{card}, {difficulty:?}/{opponent}/{seat}/{starter}"
                        );
                    }
                }
            }
        }
    }
    assert!(observed.contains(&421) && observed.contains(&422) && observed.contains(&PREP));
}

#[test]
fn paying_deflect_preserves_all_selected_runes_for_direct_exact_and_staged_casts() {
    for payment in 0..3 {
        let (mut g, _, r) = setup(0);
        let enemy = unit(&mut g, 1, 501, 92, None);
        g.players[0].power_pool = [0; 6];
        g.players[0].any_power = 0;
        let a = draft(
            vec![enemy, r[0], r[1]],
            Some(ShowOffObject::Hand { hand_index: 1 }),
        );
        assert_eq!(
            g.cost_plan(&a)
                .unwrap()
                .power
                .iter()
                .map(|p| p.amount)
                .sum::<u16>(),
            1
        );
        match payment {
            0 => g.apply(a).unwrap(),
            1 => {
                let quote = g.quote_power_payment(&a).unwrap();
                assert_eq!(quote.suggested.recycle[domain_index(Domain::Calm)], 1);
                g.apply(Action::WithPowerPayment {
                    action: Box::new(a),
                    payment: quote.suggested,
                })
                .unwrap();
            }
            _ => {
                g.apply(Action::BeginPayment {
                    action: Box::new(a),
                })
                .unwrap();
                g.apply(Action::CompletePayment).unwrap();
            }
        }
        for t in &r[..2] {
            let Target::Rune { rune_id, .. } = t else {
                unreachable!()
            };
            assert!(g.players[0].rune_objects().iter().any(|r| r.id == *rune_id));
        }
        assert_eq!(g.players[0].rune_objects().len(), 2);
        settle(&mut g);
        assert!(g.players[0].rune_objects().iter().all(|r| r.ready));
    }
}

#[test]
fn no_automatic_payment_may_recycle_its_only_rune_targets() {
    let (mut g, _, _) = setup(0);
    let enemy = unit(&mut g, 1, 501, 92, None);
    g.players[0]
        .set_rune_setup([0, 2, 0, 0, 0, 0], [0, 2, 0, 0, 0, 0])
        .unwrap();
    g.players[0].power_pool = [0; 6];
    g.players[0].any_power = 0;
    let targets = std::iter::once(enemy)
        .chain(g.players[0].rune_objects().iter().map(|r| Target::Rune {
            player: 0,
            rune_id: r.id,
        }))
        .collect();
    reject(
        &mut g,
        draft(targets, Some(ShowOffObject::Hand { hand_index: 1 })),
    );
}

#[test]
fn showing_attached_gear_adds_no_extra_irelia_choose_trigger() {
    let (mut g, u, r) = setup(0);
    g.players[0].units[0].card_id = 0;
    let gear_id = gear(&mut g, 0, false, true);
    g.apply(draft(
        vec![u, r[0], r[1]],
        Some(ShowOffObject::Gear { gear_id }),
    ))
    .unwrap();
    assert_eq!(g.chain.len(), 2);
    assert_eq!(g.chain[1].card_id, 0);
    settle(&mut g);
    assert_eq!(g.players[0].units[0].temporary_buff, 3);
}
