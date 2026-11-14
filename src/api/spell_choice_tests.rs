//! Independent expected timing and visibility, driven through JSON commands.
use super::test_support::{apply, request, views};
use super::*;

const JAE: usize = 450;

fn position(actor: usize) -> (Session, Target) {
    let mut s = Session::new(ResetOptions {
        decks: [0, 0],
        first_player: actor,
        ..Default::default()
    })
    .unwrap();
    s.start(0).unwrap();
    for p in &mut s.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy = 20;
        p.energy_pool = 20;
        p.power_pool = [5; 6];
        p.any_power = 0;
    }
    for legend in &mut s.game.legend {
        legend.ready = false;
    }
    s.game.players[actor].hand = vec![JAE, 8];
    apply(
        &mut s,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        },
    );
    let unit = &s.game.players[actor].units[0];
    assert_eq!(unit.card_id, JAE);
    assert!(!unit.ready);
    assert_eq!(s.game.players[actor].energy, 15);
    assert_eq!(
        s.game.players[actor].power_pool[crate::game::domain_index(Domain::Chaos)],
        3
    );
    let target = Target::Unit {
        player: actor,
        unit_id: unit.id,
    };
    (s, target)
}

fn choose(s: &mut Session) {
    for _ in 0..8 {
        match s.game.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => apply(
                s,
                Action::OrderTrigger {
                    trigger_id: options[0].id,
                },
            ),
            Some(Decision::Legend { .. }) => apply(s, Action::Choose { accept: false }),
            None => return,
            other => panic!("unexpected decision: {other:?}"),
        }
    }
    panic!("choice sequence did not terminate");
}

fn buff(target: Target) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![target],
    }
}

#[test]
fn irelia_jae_api_preview_is_read_only_and_draw_waits_for_its_own_response_window() {
    for actor in 0..2 {
        let (mut s, target) = position(actor);
        let before = views(&s);
        let revision = s.revision();
        for command in ["validate", "play_choices"] {
            let response = request(&mut s, command, action_to_json(&buff(target)));
            assert_eq!(response["ok"], true, "{command}: {response}");
            assert_eq!(views(&s), before);
            assert_eq!(s.revision(), revision);
        }
        let candidates = s.handle(json!({"command":"target_candidates",
        "player":actor,"revision":revision,"execution":0,
        "action":action_to_json(&Action::PlayCardWithTargets {
            hand_index:0,lane:None,targets:vec![]
        })}));
        assert_eq!(candidates["ok"], true, "{candidates}");
        assert!(
            candidates["result"]["candidates"]
                .as_array()
                .unwrap()
                .contains(&target_to_json(&target))
        );
        assert_eq!(views(&s), before);
        assert_eq!(s.revision(), revision);
        apply(&mut s, buff(target));
        choose(&mut s);
        let observed = views(&s);
        assert_eq!(observed[0]["chain"], observed[1]["chain"]);
        let chain = observed[actor]["chain"].as_array().unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0]["card_id"], 8);
        assert_eq!(chain[1]["card_id"], JAE);
        assert_eq!(chain[1]["ability"]["type"], "card_trigger");
        assert_eq!(chain[1]["targets"], json!([]));
        assert_eq!(observed[actor]["players"][actor]["hand"], json!([]));
        apply(&mut s, Action::Pass);
        assert!(s.game.players[actor].hand.is_empty());
        apply(&mut s, Action::Pass);
        let resolved = views(&s);
        assert_eq!(resolved[actor]["chain"].as_array().unwrap().len(), 1);
        assert_eq!(resolved[actor]["players"][actor]["hand"], json!([71]));
        assert!(resolved[1 - actor]["players"][actor]["hand"].is_null());
        assert_eq!(resolved[1 - actor]["players"][actor]["hand_count"], 1);
        assert_eq!(
            s.game.unit_might(&s.game.players[actor].units[0]).unwrap(),
            5
        );
        apply(&mut s, Action::Pass);
        apply(&mut s, Action::Pass);
        assert_eq!(
            s.game.unit_might(&s.game.players[actor].units[0]).unwrap(),
            7
        );
        assert_eq!(s.game.players[actor].hand, vec![71]);
    }
}

#[test]
fn irelia_jae_api_failed_payment_never_publishes_a_draw_or_changes_either_view() {
    for actor in 0..2 {
        let (mut s, target) = position(actor);
        s.game.players[actor].energy = 0;
        s.game.players[actor].energy_pool = 0;
        let before = views(&s);
        let revision = s.revision();
        for command in ["validate", "apply"] {
            let response = request(&mut s, command, action_to_json(&buff(target)));
            assert_eq!(response["ok"], false);
            assert_eq!(views(&s), before);
            assert_eq!(s.revision(), revision);
        }
    }
}
