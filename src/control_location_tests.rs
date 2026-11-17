//! CR182, 354.3, 355.17, 358.3 and 359.3.f: Here is a physical location.
use super::*;
use test_support::{position, settle_unopposed_chain, transfer_control_for_fixture};

fn here_cards(cards: &[CardId]) {
    for actor in 0..2 {
        for &card in cards {
            for (destination, departed) in [
                (None, false),
                (Some(0), false),
                (Some(1), false),
                (None, true),
            ] {
                let mut g = position(actor, 0, 468301);
                g.battlefield_owner[0] = Some(actor);
                g.players[actor].hand = vec![card];
                g.apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: Some(0),
                    targets: vec![],
                })
                .unwrap();
                let id = g.players[actor]
                    .units
                    .iter()
                    .find(|u| u.card_id == card)
                    .unwrap()
                    .id;
                assert!(
                    g.chain
                        .iter()
                        .any(|i| i.card_id == card && i.player == actor)
                );
                let current = transfer_control_for_fixture(&mut g, id);
                if departed {
                    g.return_unit_to_hand(current);
                } else if destination.is_none() {
                    g.recall_unit(1 - actor, id, false, false);
                } else {
                    // Authored response boundary: the pending ability follows
                    // its source's current field even after control changes.
                    g.unit_mut(current).unwrap().lane = destination;
                    g.battlefield_owner[destination.unwrap()] = Some(1 - actor);
                }
                let before = (g.players[actor].energy, g.players[actor].power_pool);
                let mut clone = g.clone();
                settle_unopposed_chain(&mut g);
                settle_unopposed_chain(&mut clone);
                assert_eq!(format!("{g:?}"), format!("{clone:?}"));
                let tokens: Vec<_> = g.players[actor].units.iter().filter(|u| u.token).collect();
                let expected = usize::from(!departed && destination.is_some());
                assert_eq!(
                    tokens.len(),
                    expected,
                    "card {card}, actor {actor}, destination {destination:?}, departed {departed}"
                );
                assert!(g.players[1 - actor].units.iter().all(|u| !u.token));
                if let Some(token) = tokens.first() {
                    assert_eq!(token.lane, destination);
                    assert_eq!(token.ready, matches!(card, 319 | 357));
                    assert_eq!(token.card_id, card);
                }
                assert_eq!(
                    (g.players[actor].energy, g.players[actor].power_pool),
                    before
                );
                assert!(g.pending_tokens().is_empty());
            }
        }
    }
}

#[test]
fn legacy_here_tokens_follow_current_source_location_and_original_controller() {
    here_cards(&[147, 148]);
}

#[test]
fn metadata_here_tokens_cannot_fall_back_from_an_opposing_base() {
    here_cards(&[178, 318, 319, 357]);
}

#[test]
fn movement_triggered_here_tokens_follow_the_bearer_without_changing_actor() {
    for actor in 0..2 {
        for equipment in [false, true] {
            for destination in [None, Some(1)] {
                let mut g = position(actor, 0, 468304);
                let card = if equipment { 203 } else { 179 };
                let id = g.create_unit(actor, if equipment { 71 } else { card }, false, 0);
                g.players[actor].units.last_mut().unwrap().ready = true;
                if equipment {
                    g.gear_play_at(actor, card, None).unwrap();
                    let gear_id = g.players[actor].gear.last().unwrap().id;
                    assert!(g.attach_equipment(actor, gear_id, id));
                }
                g.battlefield_owner[0] = Some(actor);
                g.apply(Action::Move {
                    unit_id: id,
                    lane: Some(0),
                })
                .unwrap();
                assert!(
                    g.chain
                        .iter()
                        .any(|item| item.card_id == card && item.player == actor)
                );
                let current = transfer_control_for_fixture(&mut g, id);
                if let Some(lane) = destination {
                    g.unit_mut(current).unwrap().lane = Some(lane);
                    g.battlefield_owner[lane] = Some(1 - actor);
                } else {
                    g.recall_unit(1 - actor, id, false, false);
                }
                let mut clone = g.clone();
                settle_unopposed_chain(&mut g);
                settle_unopposed_chain(&mut clone);
                assert_eq!(format!("{g:?}"), format!("{clone:?}"));
                let tokens: Vec<_> = g.players[actor].units.iter().filter(|u| u.token).collect();
                assert_eq!(
                    tokens.len(),
                    usize::from(destination.is_some()),
                    "actor {actor}, card {card}, destination {destination:?}"
                );
                assert!(
                    tokens
                        .iter()
                        .all(|u| u.lane == destination && u.card_id == card)
                );
                assert!(g.players[1 - actor].units.iter().all(|u| !u.token));
                assert!(g.pending_tokens().is_empty());
            }
        }
    }
}

#[test]
fn impossible_fixed_token_play_restores_its_grant_without_spending_or_entering() {
    use board_targets::BoardLocation;
    for actor in 0..2 {
        let mut g = position(actor, 0, 468302);
        let grant = g.grant_play(
            actor,
            ChainSource::Card { card_id: 65 },
            PlayGrantKind::ReadyNextUnit,
        );
        let before = (
            g.players[actor].energy,
            g.players[actor].power_pool,
            g.players[actor].xp,
            g.players[actor].score,
        );
        let location = BoardLocation::Base { player: 1 - actor };
        g.queue_token_play_at(actor, 147, 1, Some(location), false, false);
        assert_eq!(g.pending_tokens()[0].location, Some(location));
        assert!(
            g.reserved_play_grants
                .iter()
                .any(|(_, grants)| grants.iter().any(|g| g.id == grant))
        );
        g.after_resolution().unwrap();
        assert!(g.pending_tokens().is_empty() && g.decision.is_none());
        assert!(g.players.iter().all(|p| p.units.is_empty()));
        assert!(g.play_grants.iter().any(|g| g.id == grant));
        assert!(g.reserved_play_grants.is_empty());
        assert_eq!(
            (
                g.players[actor].energy,
                g.players[actor].power_pool,
                g.players[actor].xp,
                g.players[actor].score
            ),
            before
        );
        // The next legal token play still receives the same next-unit grant.
        g.queue_token_play(actor, 147, 1, Some(None), false);
        g.after_resolution().unwrap();
        assert_eq!(g.players[actor].units.len(), 1);
        assert!(g.players[actor].units[0].ready);
        assert!(!g.play_grants.iter().any(|g| g.id == grant));
    }
}
