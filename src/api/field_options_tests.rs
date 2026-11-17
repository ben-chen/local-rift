use super::*;
use crate::game::test_support::{decline_legend_action, position, ready_unit};
use test_support::{apply, request, views};

fn session(actor: usize) -> Session {
    Session {
        game: position(actor, 0, 472100),
        revision: 0,
        series: None,
        format_date: None,
    }
}

fn progress(s: &mut Session, until: Option<usize>) {
    for _ in 0..60 {
        if let Some(card) = until {
            if matches!(s.game.decision, Some(Decision::TriggerTargets { card_id, .. }) if card_id == card)
            {
                return;
            }
        } else if s.game.decision.is_none() && s.game.chain.is_empty() {
            return;
        }
        let action = if let Some(action) = decline_legend_action(&s.game) {
            action
        } else if let Some(Decision::OrderTriggers { options, .. }) = &s.game.decision {
            Action::OrderTrigger {
                trigger_id: options[0].id,
            }
        } else {
            assert!(
                s.game.decision.is_none() && !s.game.chain.is_empty(),
                "unexpected {:?}",
                s.game.decision
            );
            Action::Pass
        };
        apply(s, action);
    }
    panic!("API scenario did not reach its expected boundary");
}

#[test]
fn field_options_api_open_deployment_and_non_dragon_choice_round_trip() {
    for actor in 0..2 {
        let mut s = session(actor);
        let unit_id = ready_unit(&mut s.game, 1 - actor, 193);
        let dragon = ready_unit(&mut s.game, 1 - actor, 471);
        s.game.players[actor].hand = vec![471];
        let target = Target::Unit {
            player: 1 - actor,
            unit_id,
        };
        apply(
            &mut s,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: Some(0),
                targets: vec![],
            },
        );
        progress(&mut s, Some(471));
        let before = views(&s);
        let choice = &before[actor]["decision"];
        assert_eq!(choice["target_rule"]["excluded_tag"], "Dragon");
        assert_eq!(choice["required"], 0);
        let eligible = s
            .trigger_target_candidates(actor, s.revision(), vec![])
            .unwrap();
        assert_eq!(eligible["candidates"], json!([target_to_json(&target)]));
        let revision = s.revision();
        let rejected = request(
            &mut s,
            "apply",
            action_to_json(&Action::ChooseTriggerTargets {
                targets: vec![Target::Unit {
                    player: 1 - actor,
                    unit_id: dragon,
                }],
            }),
        );
        assert_eq!(rejected["ok"], false);
        assert_eq!(s.revision(), revision);
        assert_eq!(views(&s), before);
        apply(
            &mut s,
            Action::ChooseTriggerTargets {
                targets: vec![target],
            },
        );
        progress(&mut s, None);
        assert!(s.game.unit_ref(target).is_none());
        assert_eq!(s.game.players[1 - actor].hand, vec![193]);
        let drake = s.game.players[actor]
            .units
            .iter()
            .find(|u| u.card_id == 471)
            .unwrap();
        assert_eq!(drake.lane, Some(0));
    }
}

#[test]
fn field_options_api_rostrum_choice_and_resources_match_both_public_views() {
    for actor in 0..2 {
        let mut s = session(actor);
        let unit_id = ready_unit(&mut s.game, actor, 193);
        let target = Target::Unit {
            player: actor,
            unit_id,
        };
        s.game.players[actor].hand = vec![472];
        s.game.players[actor].rune_deck = vec![Domain::Chaos];
        apply(
            &mut s,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            },
        );
        progress(&mut s, None);
        let gear_id = s.game.players[actor].gear[0].id;
        apply(
            &mut s,
            Action::ActivateAbility {
                source: AbilitySource::Gear {
                    player: actor,
                    gear_id,
                },
                ability_index: 0,
                targets: vec![],
            },
        );
        progress(&mut s, Some(472));
        let view = s.observe(actor).unwrap();
        assert_eq!(view["decision"]["required"], 1);
        assert_eq!(
            view["decision"]["candidates"],
            json!([target_to_json(&target)])
        );
        apply(
            &mut s,
            Action::ChooseTriggerTargets {
                targets: vec![target],
            },
        );
        progress(&mut s, None);
        for view in views(&s) {
            let unit = &view["players"][actor]["units"][0];
            assert_eq!(unit["might"], 5);
            assert_eq!(view["players"][actor]["gear"][0]["ready"], false);
        }
        assert_eq!(s.game.players[actor].hand, vec![71]);
        assert_eq!(s.game.players[actor].total_runes(), 1);
        assert_eq!(s.game.players[actor].ready_runes(Domain::Chaos), 0);
    }
}

#[test]
fn field_options_api_tomb_quotes_actual_targeted_cost_without_mutating() {
    for actor in 0..2 {
        let mut s = session(actor);
        s.game.battlefields[0] = crate::battlefields::SANDSWEPT_TOMB_ID;
        let unit_id = ready_unit(&mut s.game, actor, 193);
        s.game.players[actor].units[0].lane = Some(0);
        s.game.players[actor].hand = vec![28];
        let action = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: actor,
                unit_id,
            }],
        };
        let before = views(&s);
        let revision = s.revision();
        let quote = s.play_choices(actor, revision, action.clone()).unwrap();
        assert_eq!(quote.costs.energy, 1);
        assert_eq!(
            quote
                .costs
                .power
                .iter()
                .map(|p| u32::from(p.amount))
                .sum::<u32>(),
            1
        );
        assert_eq!(s.revision(), revision);
        assert_eq!(views(&s), before);
        apply(&mut s, action);
        progress(&mut s, None);
        assert_eq!(s.game.players[actor].energy, 19);
        assert_eq!(s.game.players[actor].power_pool[3], 4);
    }
}
