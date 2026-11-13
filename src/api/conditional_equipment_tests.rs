use super::*;
use crate::game::Unit;

fn position(actor: usize, card: usize) -> Session {
    let mut s = Session::new(ResetOptions {
        decks: [3, 3],
        first_player: actor,
        ..Default::default()
    })
    .unwrap();
    s.start(0).unwrap();
    s.game.battlefields = vec![9, 9];
    s.game.battlefield_owner = vec![Some(actor), None];
    for p in &mut s.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.discard.clear();
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy = 20;
        p.energy_pool = 20;
        p.power_pool = [10; 6];
    }
    for legend in &mut s.game.legend {
        legend.ready = false;
    }
    for id in [91001, 91002] {
        s.game.players[actor].units.push(Unit {
            id,
            card_id: 69,
            lane: Some(0),
            ready: false,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        });
    }
    s.game.players[actor].hand = vec![card];
    s.game.players[1 - actor].hand = vec![71];
    s
}
fn apply(s: &mut Session, action: Action) {
    let reply=s.handle(json!({"command":"apply","revision":s.revision(),"player":s.game.priority(),"action":action_to_json(&action)}));
    assert_eq!(reply["ok"], true, "{reply}");
}
fn settle(s: &mut Session) {
    for _ in 0..60 {
        assert!(s.game.decision.is_none(), "{:?}", s.game.decision);
        if s.game.chain.is_empty() {
            return;
        }
        apply(s, Action::Pass);
    }
    panic!("Chain failed to settle");
}
fn views(s: &Session) -> [Value; 2] {
    [s.observe(0).unwrap(), s.observe(1).unwrap()]
}

#[test]
fn conditional_equipment_api_reports_public_attachment_history_and_exact_live_bonuses() {
    for actor in 0..2 {
        for (card, condition) in [
            (444, "bearer_attachment_this_turn"),
            (445, "bearer_exactly_one_other_friendly_unit_here"),
        ] {
            let mut s = position(actor, card);
            let catalog = s.handle(json!({"command":"catalog"}));
            assert_eq!(catalog["ok"], true);
            assert_eq!(
                catalog["result"]["cards"][card]["passive_rules"][0]["condition"],
                condition
            );
            apply(
                &mut s,
                Action::PlayCard {
                    hand_index: 0,
                    lane: None,
                    target: None,
                },
            );
            settle(&mut s);
            let gear_id = s.game.players[actor].gear[0].id;
            let target = Target::Unit {
                player: actor,
                unit_id: 91001,
            };
            let action = Action::Equip { gear_id, target };
            let before = views(&s);
            let revision = s.revision();
            let reply=s.handle(json!({"command":"validate","revision":revision,"player":actor,"action":action_to_json(&action)}));
            assert_eq!(reply["ok"], true, "{reply}");
            assert_eq!(views(&s), before);
            assert_eq!(s.revision(), revision);
            apply(&mut s, action.clone());
            assert_eq!(
                s.observe(actor).unwrap()["players"][actor]["gear"][0]["attached_this_turn"],
                false
            );
            settle(&mut s);
            for viewer in 0..2 {
                let view = s.observe(viewer).unwrap();
                assert_eq!(view["players"][actor]["gear"][0]["attached_to"], 91001);
                assert_eq!(
                    view["players"][actor]["gear"][0]["attached_this_turn"],
                    true
                );
                let unit = &view["players"][actor]["units"][0];
                assert_eq!(unit["might"], 11);
                let bonus = unit["passive_contributions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|v| v["card_id"] == card)
                    .unwrap();
                assert_eq!(bonus["active"], true);
                assert_eq!(bonus["computed_might_exact"], "2");
                assert_eq!(bonus["source"]["gear_id"], gear_id);
                assert!(view["players"][1 - viewer]["hand"].is_null());
            }
            let before = views(&s);
            let revision = s.revision();
            let invalid=s.handle(json!({"command":"apply","revision":revision,"player":actor,"action":action_to_json(&action)}));
            assert_eq!(
                invalid["ok"], false,
                "attached gear cannot activate its printed Equip"
            );
            assert_eq!(views(&s), before);
            assert_eq!(s.revision(), revision);
            apply(&mut s, Action::EndTurn);
            settle(&mut s);
            let view = s.observe(actor).unwrap();
            assert_eq!(
                view["players"][actor]["gear"][0]["attached_this_turn"],
                false
            );
            assert_eq!(
                view["players"][actor]["units"][0]["might"],
                if card == 444 { 9 } else { 11 }
            );
        }
    }
}
