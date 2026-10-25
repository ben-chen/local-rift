//! Controlled starting positions, followed by actual public plays and responses.
use super::*;
use crate::continuous::{PassiveEffect, PassiveKeyword};

fn position() -> Game {
    let mut game = Game::new_for_agents(3, 3, 22003, Difficulty::Tactical);
    game.start_match().unwrap();
    game.battlefields = [2, 2];
    game.battlefield_owner = [None; 2];
    for player in &mut game.players {
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.hand.clear();
        player.discard.clear();
        player.banish.clear();
        player.champion = None;
        player.deck = vec![96, 71, 7];
        player.runes = [0; 6];
        player.exhausted_runes = [0; 6];
        player.energy_pool = 100;
        player.energy = 100;
        player.power_pool = [20; 6];
        player.any_power = 0;
    }
    for legend in &mut game.legend {
        legend.ready = false;
    }
    game.trigger_state.draws = [0; 2];
    game
}
fn seeded_unit(game: &mut Game, player: usize, card_id: CardId, lane: Option<usize>) -> u32 {
    let id = game.create_unit(player, card_id, false, 0);
    game.players[player].units.last_mut().unwrap().lane = lane;
    id
}
fn source(player: usize, unit_id: u32) -> BoardObject {
    BoardObject::Unit { player, unit_id }
}
fn unit_target(player: usize, unit_id: u32) -> Target {
    Target::Unit { player, unit_id }
}
fn cast(game: &mut Game, card_id: CardId, targets: Vec<Target>) {
    let player = game.priority();
    let hand_index = game.players[player].hand.len();
    game.players[player].hand.push(card_id);
    game.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
}
fn finalize_order(game: &mut Game) {
    for _ in 0..30 {
        let Some(Decision::OrderTriggers { options, .. }) = &game.decision else {
            return;
        };
        let trigger_id = options[0].id;
        game.apply(Action::OrderTrigger { trigger_id }).unwrap();
    }
    panic!("trigger order did not finish");
}
fn until_predict(game: &mut Game) -> CardId {
    for _ in 0..80 {
        finalize_order(game);
        if let Some(Decision::Predict { card_id, .. }) = game.decision {
            return card_id;
        }
        assert!(game.decision.is_none(), "unexpected {:?}", game.decision);
        assert!(!game.chain.is_empty(), "no pending Predict");
        game.apply(Action::Pass).unwrap();
    }
    panic!("Predict did not resolve");
}
fn finish(game: &mut Game) -> usize {
    let mut predictions = 0;
    for _ in 0..160 {
        finalize_order(game);
        match game.decision.clone() {
            Some(Decision::Predict { .. }) => {
                predictions += 1;
                game.apply(Action::Choose { accept: false }).unwrap();
            }
            Some(Decision::TokenPlacement { .. }) => {
                game.apply(Action::PlaceToken { lane: None }).unwrap()
            }
            None if !game.chain.is_empty() => game.apply(Action::Pass).unwrap(),
            None => return predictions,
            other => panic!("unexpected {other:?}"),
        }
    }
    panic!("Vision sequence did not finish");
}
fn vision_items(game: &Game) -> Vec<&ChainItem> {
    game.chain
        .iter()
        .filter(|item| matches!(item.ability, Some(ChainAbility::Vision { .. })))
        .collect()
}
fn count(game: &Game, player: usize, id: u32) -> u64 {
    game.unit_keywords(
        game.players[player]
            .units
            .iter()
            .find(|u| u.id == id)
            .unwrap(),
    )
    .unwrap()
    .vision
}

#[test]
fn seer_intrinsic_and_other_grants_capture_only_the_entering_unit() {
    let mut game = position();
    cast(&mut game, 398, vec![]);
    let first = game.players[0].units[0].id;
    assert_eq!(vision_items(&game).len(), 1);
    assert_eq!(count(&game, 0, first), 1);
    assert_eq!(finish(&mut game), 1);
    cast(&mut game, 398, vec![]);
    let second = game.players[0].units[1].id;
    assert_eq!(count(&game, 0, first), 2); // characteristic changed, no first-unit trigger
    assert_eq!(count(&game, 0, second), 2);
    assert!(
        matches!(game.decision, Some(Decision::OrderTriggers { ref options, .. }) if options.len()==2)
    );
    finalize_order(&mut game);
    assert_eq!(vision_items(&game).len(), 2);
    assert!(
        vision_items(&game)
            .iter()
            .all(|item| game.chain_source(item)
                == ChainSource::Unit {
                    player: 0,
                    unit_id: second
                })
    );
    assert_eq!(finish(&mut game), 2);
    assert_eq!(game.trigger_state.draws[0], 0);
}

#[test]
fn forecaster_self_grant_stacks_without_a_printed_extra_instance() {
    let mut game = position();
    seeded_unit(&mut game, 1, 399, Some(1));
    cast(&mut game, 399, vec![]);
    let first = game.players[0].units[0].id;
    assert_eq!(vision_items(&game).len(), 1);
    assert!(
        matches!(vision_items(&game)[0].ability, Some(ChainAbility::Vision { origin:VisionOrigin::Granted{source:BoardObject::Unit{player:0,unit_id},card_id:399,rule_index:0},.. }) if unit_id==first)
    );
    assert_eq!(finish(&mut game), 1);
    cast(&mut game, 399, vec![]);
    let second = game.players[0].units[1].id;
    finalize_order(&mut game);
    assert_eq!(vision_items(&game).len(), 2);
    assert_eq!(count(&game, 0, second), 2);
    assert!(
        vision_items(&game)
            .iter()
            .all(|item| game.chain_source(item)
                == ChainSource::Unit {
                    player: 0,
                    unit_id: second
                })
    );
    assert_eq!(finish(&mut game), 2);
}

#[test]
fn global_vision_passives_share_authoritative_characteristics_and_recipient_filters() {
    let mut game = position();
    let seer = seeded_unit(&mut game, 0, 398, Some(0));
    let forecaster = seeded_unit(&mut game, 0, 399, Some(1));
    let mech = seeded_unit(&mut game, 0, 69, None);
    let ordinary = seeded_unit(&mut game, 0, 71, Some(1));
    let enemy = seeded_unit(&mut game, 1, 69, None);
    assert_eq!(count(&game, 0, seer), 1);
    assert_eq!(count(&game, 0, forecaster), 2);
    assert_eq!(count(&game, 0, mech), 2);
    assert_eq!(count(&game, 0, ordinary), 1);
    assert_eq!(count(&game, 1, enemy), 0);
    for id in [seer, forecaster, mech, ordinary] {
        let grants = game
            .passive_contributions(source(0, id))
            .unwrap()
            .into_iter()
            .filter(|p| p.active && p.effect == PassiveEffect::Keyword(PassiveKeyword::Vision))
            .count();
        let instances = game.vision_instances(source(0, id)).unwrap();
        assert_eq!(
            grants,
            instances
                .iter()
                .filter(|v| matches!(v.origin, VisionOrigin::Granted { .. }))
                .count()
        );
    }
    assert!(game.chain.is_empty());
    assert!(game.pending_triggers.is_empty());
    assert!(game.vision_instances(source(99, mech)).unwrap().is_empty());
    assert!(
        game.vision_instances(source(0, u32::MAX))
            .unwrap()
            .is_empty()
    );
    assert!(
        game.vision_instances(BoardObject::Legend { player: 0 })
            .unwrap()
            .is_empty()
    );
}

#[test]
fn two_visions_keep_the_same_top_then_recycle_independently_without_draws() {
    for first_recycles in [false, true] {
        let mut game = position();
        seeded_unit(&mut game, 0, 398, None);
        cast(&mut game, 400, vec![]);
        finalize_order(&mut game);
        let ids: Vec<_> = vision_items(&game).iter().map(|item| item.id).collect();
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
        assert_eq!(until_predict(&mut game), 7);
        game.apply(Action::Choose {
            accept: first_recycles,
        })
        .unwrap();
        assert_eq!(
            until_predict(&mut game),
            if first_recycles { 71 } else { 7 }
        );
        game.apply(Action::Choose { accept: false }).unwrap();
        assert!(game.chain.is_empty());
        assert_eq!(game.trigger_state.draws[0], 0);
        assert!(game.players[0].hand.is_empty());
        assert_eq!(game.players[0].deck.len(), 3);
    }
}

#[test]
fn visions_reinspect_after_a_real_draw_reaction_between_resolutions() {
    let mut game = position();
    seeded_unit(&mut game, 0, 398, None);
    cast(&mut game, 400, vec![]);
    assert_eq!(until_predict(&mut game), 7);
    game.apply(Action::Choose { accept: false }).unwrap();
    if game.priority() != 0 {
        game.apply(Action::Pass).unwrap();
    }
    assert_eq!(game.priority(), 0);
    cast(&mut game, 133, vec![]); // real Reaction: Draw 1, no optional Repeat paid
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    assert_eq!(game.players[0].hand, [7]);
    assert_eq!(game.trigger_state.draws[0], 1);
    assert_eq!(until_predict(&mut game), 71);
    game.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(game.trigger_state.draws[0], 1);
}

#[test]
fn vision_empty_and_duplicate_decks_never_burn_out_or_draw() {
    for top in [vec![], vec![7], vec![7, 7]] {
        let mut game = position();
        seeded_unit(&mut game, 0, 398, None);
        game.players[0].deck = top.clone();
        cast(&mut game, 400, vec![]);
        if top.is_empty() {
            assert_eq!(finish(&mut game), 0);
        } else {
            assert_eq!(until_predict(&mut game), 7);
            game.apply(Action::Choose { accept: true }).unwrap();
            assert_eq!(until_predict(&mut game), 7);
            game.apply(Action::Choose { accept: true }).unwrap();
        }
        assert_eq!(game.players[0].deck, top);
        assert_eq!(game.players[1].score, 0);
        assert_eq!(game.trigger_state.draws[0], 0);
    }
}

#[test]
fn real_star_crossed_source_departure_preserves_each_captured_instance() {
    for remove_bearer in [false, true] {
        let mut game = position();
        let grant = seeded_unit(&mut game, 0, 398, None);
        let opposing = seeded_unit(&mut game, 1, 71, None);
        cast(&mut game, 400, vec![]);
        let bearer = game.players[0].units.last().unwrap().id;
        finalize_order(&mut game);
        let target = if remove_bearer { bearer } else { grant };
        cast(
            &mut game,
            16,
            vec![unit_target(0, target), unit_target(1, opposing)],
        );
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert!(!game.players[0].units.iter().any(|unit| unit.id == target));
        assert_eq!(vision_items(&game).len(), 2);
        for item in vision_items(&game) {
            assert_eq!(
                game.chain_source(item),
                ChainSource::Unit {
                    player: 0,
                    unit_id: bearer
                }
            );
            assert_eq!(game.chain_name(item), "Jeweled Colossus · Vision");
        }
        assert_eq!(finish(&mut game), 2);
        if !remove_bearer {
            cast(&mut game, 400, vec![]);
            assert_eq!(vision_items(&game).len(), 1);
            assert_eq!(finish(&mut game), 1);
        }
    }
}

#[test]
fn actual_mech_token_play_uses_bearer_identity_not_creator_or_grant_source() {
    let mut game = position();
    seeded_unit(&mut game, 0, 398, Some(0));
    seeded_unit(&mut game, 0, 399, Some(1));
    let enemy = seeded_unit(&mut game, 1, 71, None);
    cast(&mut game, 138, vec![]); // Iterative Design, a Spell with no Mech tag
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    assert!(matches!(
        game.decision,
        Some(Decision::TokenPlacement { .. })
    ));
    game.apply(Action::PlaceToken { lane: None }).unwrap();
    let token = game.players[0]
        .units
        .iter()
        .find(|unit| unit.token)
        .unwrap()
        .clone();
    assert_eq!(token.card_id, 138);
    assert_eq!(count(&game, 0, token.id), 2);
    finalize_order(&mut game);
    assert_eq!(vision_items(&game).len(), 2);
    for item in vision_items(&game) {
        assert_eq!(
            game.chain_source(item),
            ChainSource::Unit {
                player: 0,
                unit_id: token.id
            }
        );
        assert_eq!(game.chain_name(item), "Mech token · Vision");
        assert!(matches!(
            item.ability,
            Some(ChainAbility::Vision {
                origin: VisionOrigin::Granted { .. },
                ..
            })
        ));
    }
    // The token can disappear through a real response without turning its
    // surviving abilities into abilities of its creator or the grant sources.
    cast(
        &mut game,
        16,
        vec![unit_target(0, token.id), unit_target(1, enemy)],
    );
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    assert!(!game.players[0].units.iter().any(|unit| unit.id == token.id));
    for item in vision_items(&game) {
        assert_eq!(
            game.chain_source(item),
            ChainSource::Unit {
                player: 0,
                unit_id: token.id
            }
        );
        assert_eq!(game.chain_name(item), "Mech token · Vision");
    }
    assert_eq!(finish(&mut game), 2);
}

#[test]
fn actual_non_mech_children_get_one_seer_instance_each() {
    let mut game = position();
    seeded_unit(&mut game, 0, 398, None);
    seeded_unit(&mut game, 0, 399, None);
    cast(&mut game, 87, vec![]); // four Recruit token plays
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
    assert_eq!(finish(&mut game), 4);
    let tokens: Vec<_> = game.players[0]
        .units
        .iter()
        .filter(|unit| unit.token)
        .collect();
    assert_eq!(tokens.len(), 4);
    assert!(
        tokens
            .iter()
            .all(|unit| game.unit_keywords(unit).unwrap().vision == 1)
    );
    assert_eq!(game.trigger_state.draws[0], 0);
}

#[test]
fn new_aura_and_actual_movement_do_not_retrigger_existing_vision() {
    let mut game = position();
    let existing = seeded_unit(&mut game, 0, 69, None);
    cast(&mut game, 399, vec![]);
    assert_eq!(finish(&mut game), 1); // only the newly entered Forecaster
    game.players[0]
        .units
        .iter_mut()
        .find(|unit| unit.id == existing)
        .unwrap()
        .ready = true;
    game.apply(Action::Move {
        unit_id: existing,
        lane: Some(0),
    })
    .unwrap();
    assert!(vision_items(&game).is_empty());
    assert!(!game.pending_triggers.iter().any(|pending| matches!(
        pending.effect,
        PendingTriggerEffect::Ability {
            ability: ChainAbility::Vision { .. },
            ..
        }
    )));
}

#[test]
fn intrinsic_legacy_unit_and_gear_migrate_once_with_physical_sources() {
    for card_id in [193, 242] {
        let mut game = position();
        cast(&mut game, card_id, vec![]);
        assert_eq!(vision_items(&game).len(), 1);
        let item = vision_items(&game)[0];
        let expected = if card_id == 193 {
            ChainSource::Unit {
                player: 0,
                unit_id: game.players[0].units[0].id,
            }
        } else {
            ChainSource::Gear {
                player: 0,
                gear_id: game.players[0].gear[0].id,
            }
        };
        assert_eq!(game.chain_source(item), expected);
        assert!(matches!(
            item.ability,
            Some(ChainAbility::Vision {
                origin: VisionOrigin::Printed,
                ..
            })
        ));
        assert_eq!(finish(&mut game), 1);
        assert_eq!(game.trigger_state.draws[0], 0);
    }
}

#[test]
fn printed_gear_characteristic_survives_inactive_attachment_without_unit_grants() {
    let mut game = position();
    let bearer = seeded_unit(&mut game, 0, 398, None);
    seeded_unit(&mut game, 0, 399, None);
    cast(&mut game, 242, vec![]);
    assert_eq!(finish(&mut game), 1);
    let id = game.players[0].gear[0].id;
    // Explicit attached-gear state tests Core135.4; Divining Shells has no Equip.
    game.players[0].gear[0].attached_to = Some(bearer);
    let instances = game
        .vision_instances(BoardObject::Gear {
            player: 0,
            gear_id: id,
        })
        .unwrap();
    assert_eq!(instances.len(), 1);
    assert!(!instances[0].active);
    assert_eq!(instances[0].origin, VisionOrigin::Printed);
}

#[test]
fn colossus_shield_is_only_defending_and_vision_does_not_change_might() {
    let mut game = position();
    let id = seeded_unit(&mut game, 0, 400, Some(0));
    seeded_unit(&mut game, 1, 69, Some(0));
    let u = game.players[0]
        .units
        .iter()
        .find(|unit| unit.id == id)
        .unwrap()
        .clone();
    assert_eq!(game.unit_might(&u).unwrap(), 5);
    assert_eq!(game.unit_keywords(&u).unwrap().shield, 1);
    for (attacker, expected) in [(0, 5), (1, 6)] {
        game.showdown = Some(Showdown {
            lane: 0,
            attacker,
            focus: attacker,
            passes: 0,
            combat: true,
        });
        assert_eq!(game.unit_might(&u).unwrap(), expected);
    }
}

#[test]
fn vision_order_shares_the_entry_batch_with_the_units_other_on_play_effect() {
    for vision_first in [false, true] {
        let mut game = position();
        seeded_unit(&mut game, 0, 398, None);
        cast(&mut game, 139, vec![]); // Cloud Drake's own on-play Draw 1
        let Some(Decision::OrderTriggers { options, .. }) = &game.decision else {
            panic!("missing simultaneous order");
        };
        assert_eq!(options.len(), 2);
        let chosen = options
            .iter()
            .find(|option| option.label.ends_with(" · Vision") == vision_first)
            .unwrap()
            .id;
        let batches: Vec<_> = game
            .pending_triggers
            .iter()
            .map(|pending| pending.batch)
            .collect();
        assert!(batches.iter().all(|b| *b == batches[0]));
        game.apply(Action::OrderTrigger { trigger_id: chosen })
            .unwrap();
        finalize_order(&mut game);
        let top = until_predict(&mut game);
        // First finalized is below the second on the chain.
        assert_eq!(top, if vision_first { 71 } else { 7 });
        game.apply(Action::Choose { accept: false }).unwrap();
        finish(&mut game);
        assert_eq!(game.trigger_state.draws[0], 1);
        assert_eq!(game.players[0].hand, [7]);
    }
}

#[test]
fn inactive_grant_predicates_do_not_create_a_vision_characteristic() {
    let mut game = position();
    seeded_unit(&mut game, 0, 398, None);
    let bearer = seeded_unit(&mut game, 0, 71, None);
    let target = source(0, bearer);
    let mut grant = game
        .passive_matches(target)
        .into_iter()
        .find(|p| p.effect == PassiveEffect::Keyword(PassiveKeyword::Vision))
        .unwrap();
    assert!(VisionInstance::from_grant(target, grant).is_some());
    // Pure predicate boundary: no currently admitted conditional Vision card
    // is invented here. An inactive descriptor must not grant the ability.
    grant.active = false;
    assert!(VisionInstance::from_grant(target, grant).is_none());
}

#[test]
fn intrinsic_vision_keeps_existing_second_card_trigger_selection_order() {
    for player in 0..2 {
        for card_id in [193, 242] {
            let mut game = position();
            game.active = player;
            game.priority_player = player;
            let darius = seeded_unit(&mut game, player, 183, None);
            cast(&mut game, 71, vec![]);
            finish(&mut game);
            cast(&mut game, card_id, vec![]);
            let Some(Decision::OrderTriggers { options, .. }) = &game.decision else {
                panic!("Darius and intrinsic Vision must share an ordering choice");
            };
            assert_eq!(
                options
                    .iter()
                    .map(|option| option.card_id)
                    .collect::<Vec<_>>(),
                [183, card_id],
                "existing source traversal keeps Darius before the entering permanent"
            );
            assert!(options[0].id < options[1].id);
            let first = options[0].id;
            game.apply(Action::OrderTrigger { trigger_id: first })
                .unwrap();
            let Some(ChainAbility::CardTrigger { context, .. }) = &game.chain[0].ability else {
                panic!("the first trigger ID must still select Darius");
            };
            assert_eq!(context.source.unit_id, Some(darius));
            assert_eq!(finish(&mut game), 1);
        }
    }
}
