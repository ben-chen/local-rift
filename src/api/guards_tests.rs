//! Real protocol actions expose the token and the separate optional Ready ability.
use super::*;

fn position(actor: usize) -> Session {
    let mut s = Session::new(ResetOptions {
        decks: [3, 3],
        first_player: actor,
        ..Default::default()
    })
    .unwrap();
    s.start(0).unwrap();
    s.game.battlefields = vec![9, 9];
    s.game.battlefield_owner = vec![None; 2];
    for p in &mut s.game.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy = 10;
        p.energy_pool = 10;
        p.power_pool = [10; 6];
    }
    s.game.players[actor].hand = vec![448];
    s
}

fn apply(s: &mut Session, action: Action) {
    let response = s.handle(json!({"command":"apply", "revision":s.revision(),
        "player":s.game.priority(), "action":action_to_json(&action)}));
    assert_eq!(response["ok"], true, "{response}");
}

#[test]
fn guards_protocol_links_public_token_without_choosing_and_rejects_stale_payment() {
    for actor in 0..2 {
        for pay in [false, true] {
            let mut s = position(actor);
            apply(
                &mut s,
                Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                },
            );
            apply(&mut s, Action::Pass);
            apply(&mut s, Action::Pass);
            let pending = s.observe(actor).unwrap();
            assert_eq!(pending["decision"]["type"], "token_placement");
            assert_eq!(pending["players"][actor]["units"], json!([]));
            apply(&mut s, Action::PlaceToken { lane: None });
            let before = [s.observe(0).unwrap(), s.observe(1).unwrap()];
            let decision = &before[actor]["decision"];
            assert_eq!(decision["type"], "trigger_payment");
            assert_eq!(decision["paid_before_responses"], true);
            assert_eq!(decision["targets"], json!([]));
            let token = &before[actor]["players"][actor]["units"][0];
            assert_eq!(token["name"], "Sand Soldier token");
            assert_eq!(token["ready"], false);
            assert_eq!(token["token"], true);
            assert_eq!(decision["context"]["subject"]["unit_id"], token["id"]);
            assert_eq!(decision["cost"]["power"], 1);
            assert_eq!(decision["cost"]["domains"], json!(["order"]));
            assert_eq!(
                before[0]["players"][actor]["units"],
                before[1]["players"][actor]["units"]
            );
            let choice_id = decision["choice_id"].as_u64().unwrap() as u32;
            let action = Action::ResolveTriggerPayment { choice_id, pay };
            let revision = s.revision();
            for (player, submitted) in [(1 - actor, revision), (actor, revision - 1)] {
                let rejected = s.handle(json!({"command":"apply", "revision":submitted,
                    "player":player, "action":action_to_json(&action)}));
                assert_eq!(rejected["ok"], false);
                assert_eq!(s.revision(), revision);
                assert_eq!([s.observe(0).unwrap(), s.observe(1).unwrap()], before);
            }
            apply(&mut s, action);
            assert!(!s.game.players[actor].units[0].ready);
            if pay {
                let view = s.observe(actor).unwrap();
                assert_eq!(view["chain"].as_array().unwrap().len(), 1);
                assert!(!view["chain"][0]["ability"].is_null());
                assert_eq!(view["chain"][0]["targets"], json!([]));
                apply(&mut s, Action::Pass);
                apply(&mut s, Action::Pass);
            }
            assert_eq!(s.game.players[actor].units[0].ready, pay);
            assert!(s.observe(1 - actor).unwrap()["players"][actor]["hand"].is_null());
        }
    }
}

#[test]
fn guards_hidden_api_preserves_privacy_and_requires_separate_order_payment() {
    for actor in 0..2 {
        let mut s = position(actor);
        s.game.players[actor].hand.clear();
        s.game.players[actor].energy = 0;
        s.game.players[actor].energy_pool = 0;
        s.game.players[actor].power_pool = [0; 6];
        s.game.players[actor].units.push(crate::game::Unit {
            id: 44801,
            card_id: 69,
            lane: Some(0),
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        });
        s.game.battlefield_owner[0] = Some(actor);
        s.game.players[actor]
            .hidden
            .push(crate::hidden::HiddenCard {
                id: 44802,
                card_id: 448,
                lane: 0,
                hidden_turn: 0,
                revealed: false,
            });
        let other = s.observe(1 - actor).unwrap();
        assert!(other["players"][actor]["hidden"][0]["card_id"].is_null());
        apply(
            &mut s,
            Action::PlayHidden {
                hidden_id: 44802,
                targets: vec![],
            },
        );
        apply(&mut s, Action::Pass);
        apply(&mut s, Action::Pass);
        let view = s.observe(actor).unwrap();
        assert_eq!(view["decision"]["type"], "trigger_payment");
        let choice_id = view["decision"]["choice_id"].as_u64().unwrap() as u32;
        let token = s.game.players[actor]
            .units
            .iter()
            .find(|u| u.token)
            .unwrap();
        assert_eq!(token.lane, Some(0));
        assert!(!token.ready);
        let revision = s.revision();
        let rejected = s.handle(
            json!({"command":"apply", "revision":revision, "player":actor,
            "action":action_to_json(&Action::ResolveTriggerPayment { choice_id, pay: true })}),
        );
        assert_eq!(rejected["ok"], false);
        assert_eq!(s.revision(), revision);
        assert_eq!(s.observe(actor).unwrap(), view);
        apply(
            &mut s,
            Action::ResolveTriggerPayment {
                choice_id,
                pay: false,
            },
        );
        assert!(s.game.chain.is_empty());
        assert_eq!(s.game.players[actor].energy, 0);
    }
}
