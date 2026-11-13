use super::*;

fn position(actor: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 444445, Difficulty::Tactical);
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
        p.energy = 20;
        p.energy_pool = 20;
        p.power_pool = [10; 6];
    }
    for legend in &mut g.legend {
        legend.ready = false;
    }
    g
}
fn unit(g: &mut Game, p: usize, lane: Option<usize>) -> Target {
    let unit_id = g.create_unit(p, 69, false, 0);
    g.players[p].units.last_mut().unwrap().lane = lane;
    Target::Unit { player: p, unit_id }
}
fn uid(t: Target) -> u32 {
    let Target::Unit { unit_id, .. } = t else {
        panic!()
    };
    unit_id
}
fn might(g: &Game, t: Target) -> u64 {
    g.unit_might(g.unit_ref(t).unwrap()).unwrap()
}
fn settle(g: &mut Game) {
    for _ in 0..100 {
        match g.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => g
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            None if !g.chain.is_empty() => g.apply(Action::Pass).unwrap(),
            None => return,
            other => panic!("Unexpected {other:?}"),
        }
    }
    panic!("Chain failed to settle");
}
fn play_gear(g: &mut Game, card_id: usize) -> u32 {
    let p = g.priority();
    g.players[p].hand = vec![card_id];
    g.apply(Action::PlayCard {
        hand_index: 0,
        lane: None,
        target: None,
    })
    .unwrap();
    settle(g);
    g.players[p].gear.last().unwrap().id
}
fn equip(g: &mut Game, gear_id: u32, target: Target) {
    g.apply(Action::Equip { gear_id, target }).unwrap();
    settle(g);
}

#[test]
fn conditional_equipment_brutalizer_begins_only_on_actual_attachment_and_expires() {
    for actor in 0..2 {
        let mut g = position(actor);
        let bearer = unit(&mut g, actor, None);
        let gear = play_gear(&mut g, 444);
        assert_eq!(might(&g, bearer), 8);
        assert!(
            !g.gear_attachment_status(actor, gear)
                .unwrap()
                .attached_this_turn
        );
        g.apply(Action::Equip {
            gear_id: gear,
            target: bearer,
        })
        .unwrap();
        assert_eq!(might(&g, bearer), 8, "payment is not attachment");
        assert!(
            !g.gear_attachment_status(actor, gear)
                .unwrap()
                .attached_this_turn
        );
        settle(&mut g);
        assert_eq!(might(&g, bearer), 11);
        assert_eq!(
            g.gear_might_bonus(uid(bearer)).unwrap(),
            1,
            "printed bonus is separate from effect text"
        );
        assert!(
            g.gear_attachment_status(actor, gear)
                .unwrap()
                .attached_this_turn
        );
        g.unit_mut(bearer).unwrap().damage = 10;
        g.apply(Action::EndTurn).unwrap();
        settle(&mut g);
        assert_eq!(might(&g, bearer), 9);
        assert_eq!(
            g.unit_ref(bearer).unwrap().damage,
            0,
            "Core317.2 heals before this-turn effects expire"
        );
        assert!(
            !g.gear_attachment_status(actor, gear)
                .unwrap()
                .attached_this_turn
        );
        assert!(
            !g.attach_equipment(actor, gear, uid(bearer)),
            "same-bearer attempt is a no-op"
        );
        assert_eq!(might(&g, bearer), 9);
    }
}

#[test]
fn conditional_equipment_brutalizer_tracks_physical_gear_and_bearer() {
    for actor in 0..2 {
        let mut g = position(actor);
        let first = unit(&mut g, actor, None);
        let second = unit(&mut g, actor, None);
        let a = play_gear(&mut g, 444);
        let b = play_gear(&mut g, 444);
        equip(&mut g, a, first);
        equip(&mut g, b, first);
        assert_eq!(might(&g, first), 14);
        assert!(g.attach_equipment(actor, a, uid(second)));
        assert_eq!(might(&g, first), 11);
        assert_eq!(might(&g, second), 11);
        assert_eq!(
            g.gear_attachment_status(actor, a).unwrap().unit_id,
            Some(uid(second))
        );
        g.gear_destroy(actor, a);
        assert!(g.gear_attachment_status(actor, a).is_none());
        assert_eq!(might(&g, second), 8);
        let new_copy = play_gear(&mut g, 444);
        assert_ne!(new_copy, a);
        assert!(
            !g.gear_attachment_status(actor, new_copy)
                .unwrap()
                .attached_this_turn
        );
        // A departed bearer does not make the same printed card a continuation.
        g.gear_unit_left_board(actor, uid(first));
        g.players[actor].units.retain(|u| u.id != uid(first));
        assert!(
            !g.gear_attachment_status(actor, b)
                .unwrap()
                .attached_this_turn
        );
        let replacement = unit(&mut g, actor, None);
        assert_eq!(might(&g, replacement), 8);
    }
}

#[test]
fn conditional_equipment_hand_hammer_counts_exactly_one_other_friendly_battlefield_unit() {
    for actor in 0..2 {
        let mut g = position(actor);
        let bearer = unit(&mut g, actor, None);
        let ally = unit(&mut g, actor, None);
        let gear = play_gear(&mut g, 445);
        equip(&mut g, gear, bearer);
        assert_eq!(might(&g, bearer), 9, "two in base do not qualify");
        g.unit_mut(bearer).unwrap().lane = Some(0);
        assert_eq!(might(&g, bearer), 9, "alone at a battlefield");
        unit(&mut g, 1 - actor, Some(0));
        assert_eq!(might(&g, bearer), 9, "enemy units are not friendly");
        g.unit_mut(ally).unwrap().lane = Some(0);
        assert_eq!(might(&g, bearer), 11);
        let third = unit(&mut g, actor, Some(0));
        assert_eq!(might(&g, bearer), 9, "three friendly units turns it off");
        g.unit_mut(third).unwrap().lane = Some(1);
        g.unit_mut(ally).unwrap().token = true;
        assert_eq!(might(&g, bearer), 11, "a friendly token still counts");
        g.unit_mut(ally).unwrap().lane = Some(1);
        assert_eq!(might(&g, bearer), 9);
    }
}

#[test]
fn conditional_equipment_bonus_loss_causes_real_lethal_cleanup() {
    for card in [444, 445] {
        for actor in 0..2 {
            let mut g = position(actor);
            let bearer = unit(&mut g, actor, Some(0));
            let ally = unit(&mut g, actor, Some(0));
            g.battlefield_owner[0] = Some(actor);
            let gear = play_gear(&mut g, card);
            equip(&mut g, gear, bearer);
            assert_eq!(might(&g, bearer), 11);
            g.unit_mut(bearer).unwrap().damage = 10;
            if card == 444 {
                // Exercise the shared attachment instruction used by future
                // reattachment effects, not a forbidden second Equip activation.
                assert!(g.attach_equipment(actor, gear, uid(ally)));
                g.after_resolution().unwrap();
            } else {
                g.unit_mut(ally).unwrap().ready = true;
                g.apply(Action::Move {
                    unit_id: uid(ally),
                    lane: None,
                })
                .unwrap();
            }
            settle(&mut g);
            assert!(g.unit_ref(bearer).is_none(), "card{card} actor{actor}");
            let equipment = g.players[actor].gear.iter().find(|e| e.id == gear).unwrap();
            assert_eq!(
                equipment.attached_to,
                if card == 444 { Some(uid(ally)) } else { None }
            );
            assert_eq!(
                g.gear_location(actor, gear),
                if card == 444 { Some(0) } else { None }
            );
        }
    }
}

#[test]
fn conditional_equipment_irelia_choose_trigger_resolves_before_attachment() {
    for actor in 0..2 {
        for card in [444, 445] {
            let mut g = position(actor);
            let bearer = unit(&mut g, actor, Some(0));
            g.unit_mut(bearer).unwrap().card_id = 0;
            unit(&mut g, actor, Some(0));
            g.battlefield_owner[0] = Some(actor);
            let gear = play_gear(&mut g, card);
            assert_eq!(might(&g, bearer), 4);
            g.apply(Action::Equip {
                gear_id: gear,
                target: bearer,
            })
            .unwrap();
            assert_eq!(
                g.chain.len(),
                2,
                "Choosing Irelia queues her own ability above Equip"
            );
            g.apply(Action::Pass).unwrap();
            g.apply(Action::Pass).unwrap();
            assert_eq!(might(&g, bearer), 5);
            assert!(
                !g.gear_attachment_status(actor, gear)
                    .unwrap()
                    .attached_this_turn
            );
            settle(&mut g);
            assert_eq!(
                might(&g, bearer),
                8,
                "Printed bonus, conditional bonus and Irelia's trigger coexist"
            );
        }
    }
}

#[test]
fn conditional_equipment_departed_target_spends_cost_without_attachment_event() {
    for actor in 0..2 {
        for card in [444, 445] {
            let mut g = position(actor);
            let bearer = unit(&mut g, actor, Some(0));
            g.unit_mut(bearer).unwrap().card_id = 25; // Lonely Poro, within Gust's limit.
            g.battlefield_owner[0] = Some(actor);
            let gear = play_gear(&mut g, card);
            let power_before = g.players[actor].power_pool;
            g.apply(Action::Equip {
                gear_id: gear,
                target: bearer,
            })
            .unwrap();
            if g.priority() == actor {
                g.apply(Action::Pass).unwrap();
            }
            assert_eq!(g.priority(), 1 - actor);
            g.players[1 - actor].hand = vec![11];
            g.apply(Action::PlayCard {
                hand_index: 0,
                lane: None,
                target: Some(bearer),
            })
            .unwrap();
            settle(&mut g);
            assert!(g.unit_ref(bearer).is_none());
            assert!(g.players[actor].hand.contains(&25));
            let status = g.gear_attachment_status(actor, gear).unwrap();
            assert_eq!(status.unit_id, None);
            assert!(!status.attached_this_turn);
            assert_eq!(
                g.players[actor].power_pool[domain_index(Domain::Calm)],
                power_before[domain_index(Domain::Calm)] - 1
            );
            assert!(
                g.gear_attachments_this_turn
                    .iter()
                    .all(|(id, _)| *id != gear)
            );
        }
    }
}

#[test]
fn conditional_equipment_salvage_removes_bonus_before_lethal_cleanup_and_still_draws() {
    for actor in 0..2 {
        for card in [444, 445] {
            let mut g = position(actor);
            let bearer = unit(&mut g, actor, Some(0));
            unit(&mut g, actor, Some(0));
            g.battlefield_owner[0] = Some(actor);
            let gear = play_gear(&mut g, card);
            equip(&mut g, gear, bearer);
            g.unit_mut(bearer).unwrap().damage = 10;
            g.players[actor].hand = vec![41];
            let deck_before = g.players[actor].deck.len();
            g.apply(Action::PlayCard {
                hand_index: 0,
                lane: None,
                target: Some(Target::Gear {
                    player: actor,
                    gear_id: gear,
                }),
            })
            .unwrap();
            settle(&mut g);
            assert!(g.gear_attachment_status(actor, gear).is_none());
            assert!(g.unit_ref(bearer).is_none());
            assert!(g.players[actor].discard.contains(&card));
            assert!(g.players[actor].discard.contains(&69));
            assert_eq!(g.players[actor].deck.len(), deck_before - 1);
            assert_eq!(g.players[actor].hand, vec![71]);
        }
    }
}
