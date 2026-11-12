//! Baron uses real play/payment/movement actions on controlled positions.
//! Timing anchors: Core355.1,355.2,369.3 and the Radiance FAQ.
use super::*;
use crate::battlefields::BARON_PIT_ID;

fn position(actor: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 147219, Difficulty::Tactical);
    g.start_match().unwrap();
    g.active = actor;
    g.priority_player = actor;
    g.battlefields = vec![7, 9];
    g.battlefield_owner = vec![Some(actor), None];
    for p in &mut g.players {
        p.units.clear();
        p.gear.clear();
        p.hand.clear();
        p.hidden.clear();
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
    g.legend[actor] = LegendState::from_legend(10).unwrap();
    let id = g.create_unit(actor, 71, false, 0);
    let u = g
        .unit_mut(Target::Unit {
            player: actor,
            unit_id: id,
        })
        .unwrap();
    u.lane = Some(0);
    u.ready = true;
    g.players[actor].hand = vec![440];
    g
}

fn play(lane: Option<usize>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane,
        targets: vec![],
    }
}

fn settle(g: &mut Game) {
    for _ in 0..40 {
        if !g.in_window() {
            return;
        }
        match g.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => g
                .apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap(),
            Some(Decision::Legend { .. }) => g.apply(Action::Choose { accept: false }).unwrap(),
            None => g.apply(Action::Pass).unwrap(),
            other => panic!("Unexpected Baron decision: {other:?}"),
        }
    }
    panic!("Baron sequence did not settle");
}

fn baron(g: &Game, actor: usize) -> &Unit {
    g.players[actor]
        .units
        .iter()
        .find(|u| u.card_id == 440)
        .unwrap()
}

#[test]
fn baron_new_pit_preserves_exile_and_hunters_circle_finalization_facts() {
    for actor in 0..2 {
        for lane in [None, Some(0)] {
            for staged in [false, true] {
                let mut g = position(actor);
                let before = format!("{g:?}");
                let view = g.play_choices(&play(lane)).unwrap();
                assert_eq!(view.baron_pit_entry, Some(2));
                assert_eq!(view.legend_entry_ready.is_some(), lane.is_some());
                assert_eq!(format!("{g:?}"), before);
                if staged {
                    g.apply(Action::BeginPayment {
                        action: Box::new(play(lane)),
                    })
                    .unwrap();
                    assert_eq!(g.battlefields, vec![7, 9, BARON_PIT_ID]);
                    assert!(g.players[actor].hand.is_empty());
                    assert_eq!(g.players[actor].units.len(), 1);
                    assert_eq!(g.players[actor].energy, 100);
                    assert_eq!(g.battlefield_owner[2], None);
                    assert_eq!(
                        g.payment_plan_view(actor).unwrap().energy,
                        if lane.is_some() { 9 } else { 10 }
                    );
                    g.apply(Action::CompletePayment).unwrap();
                } else {
                    g.apply(play(lane)).unwrap();
                }
                assert_eq!(g.battlefields, vec![7, 9, BARON_PIT_ID]);
                assert_eq!(baron(&g, actor).lane, Some(2));
                assert_eq!(baron(&g, actor).ready, lane.is_some());
                assert_eq!(g.unit_might(baron(&g, actor)).unwrap(), 12);
                assert_eq!(
                    g.players[actor].energy,
                    if lane.is_some() { 91 } else { 90 }
                );
                assert_eq!(g.players[actor].power_pool[Domain::Chaos as usize], 17);
                assert_eq!(
                    g.play_history.units_or_gears_here[actor],
                    vec![u16::from(lane.is_some()), 0, 0]
                );
                settle(&mut g);
                assert_eq!(g.battlefield_owner[2], Some(actor));
                g.validate_numeric_state().unwrap();
            }
        }
    }
}

#[test]
fn baron_payment_undo_refusal_and_failed_play_restore_pit_and_resources() {
    for actor in 0..2 {
        for refuse in [false, true] {
            let mut g = position(actor);
            let before = format!("{g:?}");
            g.apply(Action::BeginPayment {
                action: Box::new(play(Some(0))),
            })
            .unwrap();
            assert_eq!(g.battlefield_count(), 3);
            g.apply(if refuse {
                Action::RefusePayment
            } else {
                Action::Choose { accept: false }
            })
            .unwrap();
            assert_eq!(format!("{g:?}"), before);
        }
        for lane in [Some(1), Some(2), Some(usize::MAX)] {
            let mut g = position(actor);
            let before = format!("{g:?}");
            assert!(g.apply(play(lane)).is_err());
            assert_eq!(format!("{g:?}"), before);
        }
        let mut g = position(actor);
        g.players[actor].energy = 0;
        g.players[actor].energy_pool = 0;
        let before = format!("{g:?}");
        assert!(g.apply(play(None)).is_err());
        assert_eq!(format!("{g:?}"), before);
        g.apply(Action::BeginPayment {
            action: Box::new(play(None)),
        })
        .unwrap();
        let pending = format!("{g:?}");
        assert!(g.apply(Action::CompletePayment).is_err());
        assert_eq!(format!("{g:?}"), pending);
        g.apply(Action::RefusePayment).unwrap();
        assert_eq!(format!("{g:?}"), before);
    }
}

#[test]
fn baron_existing_pit_does_not_redirect_another_play_or_disappear_with_its_creator() {
    for actor in 0..2 {
        let mut g = position(actor);
        g.apply(play(None)).unwrap();
        settle(&mut g);
        let first = Target::Unit {
            player: actor,
            unit_id: baron(&g, actor).id,
        };
        g.kill_unit(first).unwrap();
        g.after_resolution().unwrap();
        assert_eq!(g.battlefields, vec![7, 9, BARON_PIT_ID]);
        g.players[actor].hand = vec![440];
        assert_eq!(g.play_choices(&play(None)).unwrap().baron_pit_entry, None);
        g.apply(Action::BeginPayment {
            action: Box::new(play(None)),
        })
        .unwrap();
        g.apply(Action::CompletePayment).unwrap();
        assert_eq!(baron(&g, actor).lane, None);
        assert!(!baron(&g, actor).ready);
        assert_eq!(g.battlefield_count(), 3);
        g.rematch(219147);
        assert_eq!(g.battlefields, vec![7, 9]);
    }
}

#[test]
fn baron_pit_allows_entry_from_another_field_without_granting_ganking_for_exit() {
    for actor in 0..2 {
        let mut g = position(actor);
        let mover = g.players[actor].units[0].id;
        g.apply(play(None)).unwrap();
        settle(&mut g);
        let into_pit = Action::Move {
            unit_id: mover,
            lane: Some(2),
        };
        assert!(g.legal_actions().unwrap().contains(&into_pit));
        g.apply(into_pit).unwrap();
        settle(&mut g);
        let target = Target::Unit {
            player: actor,
            unit_id: mover,
        };
        assert_eq!(g.unit_ref(target).unwrap().lane, Some(2));
        assert!(
            !g.unit_keywords(g.unit_ref(target).unwrap())
                .unwrap()
                .ganking
        );
        g.ready_unit_by(actor, actor, mover);
        let out = Action::Move {
            unit_id: mover,
            lane: Some(1),
        };
        assert!(!g.legal_actions().unwrap().contains(&out));
        let before = format!("{g:?}");
        assert!(g.apply(out).is_err());
        assert_eq!(format!("{g:?}"), before);
        g.apply(Action::Move {
            unit_id: mover,
            lane: None,
        })
        .unwrap();
        assert_eq!(g.unit_ref(target).unwrap().lane, None);
    }
}

#[test]
fn baron_aura_stacks_for_other_friendly_units_and_ends_when_its_source_leaves() {
    for actor in 0..2 {
        let mut g = position(actor);
        let first_friend = g.players[actor].units[0].id;
        let enemy = g.create_unit(1 - actor, 71, false, 0);
        g.apply(play(None)).unwrap();
        settle(&mut g);
        let first = Target::Unit {
            player: actor,
            unit_id: baron(&g, actor).id,
        };
        g.players[actor].hand = vec![440];
        g.apply(play(None)).unwrap();
        settle(&mut g);
        let friend = Target::Unit {
            player: actor,
            unit_id: first_friend,
        };
        assert_eq!(g.unit_might(g.unit_ref(friend).unwrap()).unwrap(), 9);
        for u in &g.players[actor].units {
            if u.card_id == 440 {
                assert_eq!(g.unit_might(u).unwrap(), 14);
            }
        }
        assert_eq!(
            g.unit_might(
                g.unit_ref(Target::Unit {
                    player: 1 - actor,
                    unit_id: enemy
                })
                .unwrap()
            )
            .unwrap(),
            5
        );
        g.kill_unit(first).unwrap();
        g.after_resolution().unwrap();
        settle(&mut g);
        assert_eq!(g.unit_might(g.unit_ref(friend).unwrap()).unwrap(), 7);
        let survivor = Target::Unit {
            player: actor,
            unit_id: baron(&g, actor).id,
        };
        g.recall_unit(
            actor,
            match survivor {
                Target::Unit { unit_id, .. } => unit_id,
                _ => unreachable!(),
            },
            false,
            false,
        );
        g.after_resolution().unwrap();
        // Recall moves a unit to base; its global aura remains active there.
        assert_eq!(g.unit_might(g.unit_ref(friend).unwrap()).unwrap(), 7);
        g.bounce(survivor);
        g.after_resolution().unwrap();
        assert_eq!(g.unit_might(g.unit_ref(friend).unwrap()).unwrap(), 5);
        assert_eq!(g.battlefield_count(), 3);
    }
}

#[test]
fn baron_rejects_enemy_choices_but_not_friendly_choices_or_untargeted_area_damage() {
    for actor in 0..2 {
        let mut g = position(actor);
        g.apply(play(None)).unwrap();
        settle(&mut g);
        let target = Target::Unit {
            player: actor,
            unit_id: baron(&g, actor).id,
        };
        assert!(g.can_choose_unit(actor, target));
        assert!(!g.can_choose_unit(1 - actor, target));
        g.active = 1 - actor;
        g.priority_player = 1 - actor;
        g.players[1 - actor].hand = vec![53, 299];
        let before = format!("{g:?}");
        assert!(
            g.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![target, target]
            })
            .is_err()
        );
        assert_eq!(format!("{g:?}"), before);
        g.apply(Action::PlayCardWithTargets {
            hand_index: 1,
            lane: None,
            targets: vec![Target::Battlefield { lane: 2 }],
        })
        .unwrap();
        settle(&mut g);
        assert_eq!(g.unit_ref(target).unwrap().damage, 3);
    }
}

#[test]
fn baron_instructed_trash_and_banish_plays_keep_entry_replacement_through_payment() {
    for actor in 0..2 {
        for banish in [false, true] {
            let mut g = position(actor);
            g.players[actor].hand.clear();
            if banish {
                g.players[actor].banish.push(440);
                g.queue_banish_play(actor, 0, InstructedPlayKind::ArcaneShift, None);
            } else {
                g.players[actor].discard.push(440);
                g.queue_card_play(actor, 0, InstructedPlayKind::BaitedHook);
            }
            g.after_resolution().unwrap();
            let action = Action::FinalizePendingPlay {
                lane: Some(0),
                targets: vec![],
                repeat_targets: vec![],
                repeat: false,
                accelerate: false,
                bonus: false,
            };
            g.apply(Action::BeginPayment {
                action: Box::new(action),
            })
            .unwrap();
            assert_eq!(g.battlefield_count(), 3);
            g.apply(Action::CompletePayment).unwrap();
            settle(&mut g);
            assert_eq!(baron(&g, actor).lane, Some(2));
            assert!(baron(&g, actor).ready);
        }
    }
}

#[test]
fn baron_pit_cannot_be_selected_as_a_starting_battlefield() {
    let mut g = Game::new_for_agents(3, 3, 147, Difficulty::Tactical);
    for actor in 0..2 {
        assert!(g.choose_battlefield(actor, BARON_PIT_ID).is_err());
    }
    assert!(
        crate::battlefields::BATTLEFIELDS
            .iter()
            .all(|b| b.id != BARON_PIT_ID)
    );
    assert_eq!(
        crate::battlefields::battlefield(BARON_PIT_ID).source_id,
        "unl-t01"
    );
}

#[test]
fn baron_pit_definition_matches_independent_manifest_and_archived_official_text() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../assets/data/battlefield_tokens.json")).unwrap();
    let catalog: serde_json::Value =
        serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap();
    let row = &manifest["battlefields"][0];
    let source = catalog["printings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["official_id"] == "unl-t01")
        .unwrap();
    let field = crate::battlefields::battlefield(BARON_PIT_ID);
    assert_eq!(row["id"], BARON_PIT_ID);
    assert_eq!(row["source_id"], field.source_id);
    assert_eq!(row["name"], field.name);
    assert_eq!(row["official_text"], field.official_text);
    assert_eq!(row["official_text"], source["effective"]["rules_text"]);
    assert_eq!(row["official_html"], source["effective"]["rules_html"]);
    assert_eq!(
        source["effective"]["super_types"],
        serde_json::json!(["token"])
    );
}
