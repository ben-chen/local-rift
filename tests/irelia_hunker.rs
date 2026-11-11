//! Hunker Down: readying, independent draw, and restrictions on either player's Move.
use local_rift::game::{Action, Decision, Difficulty, Game, Target, Unit};

const HUNKER: usize = 425;
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
#[test]
fn friendly_only_readies_and_draws_even_when_already_ready_for_both_seats() {
    for owner in 0..2 {
        for ready in [false, true] {
            let mut g = position(owner);
            let target = unit(&mut g, owner, 500, 73, None);
            unit(&mut g, 1 - owner, 501, 73, None);
            g.players[owner].units[0].ready = ready;
            g.players[owner].deck = vec![71];
            g.players[owner].hand = vec![HUNKER];
            assert_eq!(
                g.card_target_candidates(owner, HUNKER, None, &[]).unwrap(),
                vec![target]
            );
            reject(
                &mut g,
                Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![Target::Unit {
                        player: 1 - owner,
                        unit_id: 501,
                    }],
                },
            );
            g.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![target],
            })
            .unwrap();
            assert!(!g.movement_locked(owner, 500));
            resolve(&mut g);
            assert!(g.players[owner].units[0].ready);
            assert_eq!(g.players[owner].hand, vec![71]);
            assert_eq!(g.players[owner].energy, 49);
            for actor in 0..2 {
                assert!(g.movement_prevented(actor, owner, 500));
            }
        }
    }
}
#[test]
fn irelia_choose_and_actual_ready_each_grow_might_once() {
    for ready in [false, true] {
        let mut g = position(0);
        let target = unit(&mut g, 0, 500, 0, None);
        g.players[0].units[0].ready = ready;
        cast(&mut g, HUNKER, vec![target]);
        settle(&mut g);
        assert_eq!(
            g.players[0].units[0].temporary_buff,
            if ready { 1 } else { 2 }
        );
        assert!(g.movement_locked(0, 500));
    }
}
#[test]
fn independent_draw_survives_departed_target_but_counter_stops_every_instruction() {
    for counter in [false, true] {
        let mut g = position(0);
        let target = unit(&mut g, 0, 500, 73, None);
        g.players[0].units[0].ready = false;
        g.players[0].deck = vec![71];
        cast(&mut g, HUNKER, vec![target]);
        if counter {
            let chain_id = g.chain[0].id;
            cast(&mut g, 86, vec![Target::Spell { chain_id }]);
        } else {
            // Public response returns the original target to hand.
            cast(&mut g, 377, vec![target]);
        }
        settle(&mut g);
        assert_eq!(g.players[0].hand.contains(&71), !counter);
        assert!(!g.movement_locked(0, 500));
        if counter {
            assert!(!g.players[0].units[0].ready);
        } else {
            assert!(g.players[0].units.is_empty());
        }
    }
}
#[test]
fn controller_basic_and_group_moves_are_rejected_atomically_then_expire() {
    for owner in 0..2 {
        let mut g = position(owner);
        let target = unit(&mut g, owner, 500, 73, None);
        unit(&mut g, owner, 501, 73, None);
        cast(&mut g, HUNKER, vec![target]);
        resolve(&mut g);
        reject(
            &mut g,
            Action::Move {
                unit_id: 500,
                lane: Some(0),
            },
        );
        reject(
            &mut g,
            Action::MoveGroup {
                unit_ids: vec![501, 500],
                lane: Some(0),
            },
        );
        assert!(
            !g.legal_actions()
                .unwrap()
                .iter()
                .any(|a| matches!(a, Action::Move { unit_id: 500, .. }))
        );
        g.apply(Action::EndTurn).unwrap();
        assert!(!g.movement_locked(owner, 500));
        assert!(!g.movement_prevented(1 - owner, owner, 500));
    }
}
#[test]
fn friendly_flash_can_choose_locked_unit_and_still_moves_the_other_target() {
    let mut g = position(0);
    let a = unit(&mut g, 0, 500, 73, Some(0));
    let b = unit(&mut g, 0, 501, 73, Some(1));
    g.battlefield_owner = [Some(0); 2];
    cast(&mut g, HUNKER, vec![a]);
    resolve(&mut g);
    cast(&mut g, 9, vec![a, b]);
    resolve(&mut g);
    assert_eq!(g.players[0].units[0].lane, Some(0));
    assert_eq!(g.players[0].units[1].lane, None);
}
#[test]
fn enemy_charm_cannot_move_hunkered_unit_for_either_seat() {
    for owner in 0..2 {
        let mut g = position(1 - owner);
        let target = unit(&mut g, owner, 500, 73, None);
        g.players[1 - owner].hand = vec![4];
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: Some(0),
            targets: vec![target],
        })
        .unwrap();
        g.apply(Action::Pass).unwrap();
        assert_eq!(g.priority(), owner);
        cast(&mut g, HUNKER, vec![target]);
        resolve(&mut g);
        resolve(&mut g);
        assert_eq!(g.players[owner].units[0].lane, None);
        assert!(g.players[1 - owner].discard.contains(&4));
    }
}
#[test]
fn blocked_ride_the_wind_still_readies_and_does_not_trigger_move_effects() {
    let mut g = position(0);
    let target = unit(&mut g, 0, 500, 17, Some(0)); // Stellacorn Herder draws on Move.
    g.battlefield_owner[0] = Some(0);
    g.players[0].deck = vec![71, 73];
    cast(&mut g, HUNKER, vec![target]);
    resolve(&mut g);
    g.players[0].units[0].ready = false;
    cast(&mut g, 14, vec![target]);
    resolve(&mut g);
    assert!(g.players[0].units[0].ready);
    assert_eq!(g.players[0].units[0].lane, Some(0));
    assert_eq!(g.players[0].deck, vec![71]);
}
#[test]
fn returning_to_hand_and_replaying_does_not_retain_the_old_physical_lock() {
    let mut g = position(0);
    let target = unit(&mut g, 0, 500, 73, None);
    cast(&mut g, HUNKER, vec![target]);
    resolve(&mut g);
    cast(&mut g, 377, vec![target]);
    resolve(&mut g);
    assert!(g.players[0].units.is_empty());
    let i = g.players[0].hand.iter().position(|&id| id == 73).unwrap();
    g.apply(Action::PlayCardWithTargets {
        hand_index: i,
        lane: None,
        targets: vec![],
    })
    .unwrap();
    let replayed = &g.players[0].units[0];
    assert_ne!(replayed.id, 500);
    assert!(!g.movement_locked(0, replayed.id));
}
#[test]
fn tideturner_moves_the_unrestricted_half_when_either_unit_is_locked() {
    for lock_source in [false, true] {
        let mut g = position(0);
        let a = unit(&mut g, 0, 500, 18, None);
        let b = unit(&mut g, 0, 501, 73, Some(0));
        g.battlefield_owner[0] = Some(0);
        cast(&mut g, HUNKER, vec![if lock_source { a } else { b }]);
        resolve(&mut g);
        // The optional ability is already waiting to be finalized.
        g.decision = Some(Decision::SwapUnit {
            player: 0,
            unit_id: 500,
        });
        g.apply(Action::ChooseTarget { target: Some(b) }).unwrap();
        resolve(&mut g);
        assert_eq!(
            g.players[0].units[0].lane,
            if lock_source { None } else { Some(0) }
        );
        assert_eq!(
            g.players[0].units[1].lane,
            if lock_source { None } else { Some(0) }
        );
    }
}

#[test]
fn hunker_forty_card_irelia_finishes_all_presets_seats_starters_and_levels() {
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
                        .extend([(421, 3), (422, 2), (423, 1), (HUNKER, 3)]);
                    assert_eq!(practice.card_count(), 40);
                    practice.validate().unwrap();
                    let mut presets = [0, opponent];
                    let mut custom = [Some(practice), None];
                    if seat == 1 {
                        presets.swap(0, 1);
                        custom.swap(0, 1);
                    }
                    let seed = 425_000 + (level * 100 + opponent * 4 + seat * 2 + starter) as u64;
                    let mut g =
                        Game::new_with_custom(presets, custom, seed, difficulty, false).unwrap();
                    g.choose_starting_player(starter).unwrap();
                    g.start_match().unwrap();
                    for step in 0..2400 {
                        if g.winner.is_some() {
                            break;
                        }
                        for item in &g.chain {
                            if [421, 422, 423, HUNKER].contains(&item.card_id) {
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
                    for (card, expected) in [(421, 3), (422, 2), (423, 1), (HUNKER, 3)] {
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
    assert!(observed.contains(&421) && observed.contains(&422) && observed.contains(&HUNKER));
}
