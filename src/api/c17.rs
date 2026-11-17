//! Banish-origin choices preserve public identity, destination and quote semantics.
use super::*;
use crate::game::Unit;

fn session() -> Session {
    let mut s = Session::new(ResetOptions::default()).unwrap();
    s.start(0).unwrap();
    s.game.battlefields = vec![7, 9];
    s.game.battlefield_owner = vec![None; 2];
    for p in &mut s.game.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.discard.clear();
        p.banish.clear();
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy = 30;
        p.energy_pool = 30;
        p.power_pool = [5; 6];
    }
    s.game.legend[0] = crate::legends::LegendState::from_legend(9).unwrap();
    s
}
fn unit(s: &mut Session, owner: usize, id: u32, lane: Option<usize>) -> Value {
    s.game.players[owner].units.push(Unit {
        id,
        card_id: 70,
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
    json!({"type":"unit","player":owner,"unit_id":id})
}
fn call(s: &mut Session, command: &str, action: Value) -> Value {
    s.handle(json!({"command":command,"player":s.game.priority(),"revision":s.revision(),"action":action}))
}
fn resolve(s: &mut Session) {
    for _ in 0..30 {
        if s.game.decision.is_some() || s.game.chain.is_empty() {
            return;
        }
        assert_eq!(call(s, "apply", json!({"type":"pass"}))["ok"], true);
    }
    panic!("Unfinished chain");
}
#[test]
fn c17_wire_exposes_banish_origin_free_base_cost_and_prescribed_destination() {
    for (spell, owner, lane, destination) in [
        (380, 0, None, json!({"mode":"base"})),
        (381, 1, Some(1), json!({"mode":"fixed","lane":1})),
        (
            381,
            0,
            None,
            json!({"mode":"fixed","lane":null,"base_player":0}),
        ),
        (
            381,
            1,
            None,
            json!({"mode":"fixed","lane":null,"base_player":1}),
        ),
        (382, 0, None, json!({"mode":"any_battlefield"})),
    ] {
        let mut s = session();
        let target = unit(&mut s, owner, 991, lane);
        s.game.players[0].hand = vec![spell];
        assert_eq!(
            call(
                &mut s,
                "apply",
                json!({"type":"play_card_targets","hand_index":0,"targets":[target]})
            )["ok"],
            true
        );
        resolve(&mut s);
        assert_eq!(s.game.priority(), owner);
        let a = s.observe(0).unwrap();
        let b = s.observe(1).unwrap();
        assert_eq!(a["pending_card_plays"], b["pending_card_plays"]);
        assert_eq!(a["decision"]["origin"], json!({"type":"banish"}));
        assert_eq!(a["decision"]["instruction"]["destination"], destination);
        assert_eq!(a["decision"]["instruction"]["base_cost_waiver"], "all");
        let dest = match spell {
            380 => None,
            381 => lane,
            _ => Some(0),
        };
        let action = json!({"type":"finalize_pending_play","lane":dest,"targets":[],"repeat_targets":[],"repeat":false,"accelerate":false,"bonus":false});
        let quote = call(&mut s, "play_choices", action.clone());
        assert_eq!(quote["ok"], true);
        assert_eq!(quote["result"]["origin"], json!({"type":"banish"}));
        assert_eq!(s.observe(0).unwrap(), a);
        let invalid = json!({"type":"finalize_pending_play","lane":if dest.is_none(){Some(1)}else{None},"targets":[]});
        assert_eq!(call(&mut s, "apply", invalid)["ok"], false);
        assert_eq!(s.observe(0).unwrap(), a);
        assert_eq!(call(&mut s, "apply", action)["ok"], true);
        assert_eq!(s.game.players[owner].units[0].lane, dest);
    }
}

#[test]
fn fixed_pending_token_bases_expose_absolute_destination_to_both_players() {
    use crate::game::board_targets::BoardLocation;
    for controller in 0..2 {
        for base in 0..2 {
            let mut s = session();
            s.game.queue_token_play_at(
                controller,
                147,
                1,
                Some(BoardLocation::Base { player: base }),
                false,
                false,
            );
            let own = s.observe(controller).unwrap();
            let other = s.observe(1 - controller).unwrap();
            assert_eq!(own["pending_token_plays"], other["pending_token_plays"]);
            assert_eq!(own["pending_token_plays"][0]["player"], controller);
            assert_eq!(
                own["pending_token_plays"][0]["destination"],
                json!({"mode":"fixed", "lane":null, "base_player":base})
            );
        }
    }
}
#[test]
fn c17_wire_arcane_parent_finishes_and_hidden_opponent_hand_stays_private() {
    let mut s = session();
    let own = unit(&mut s, 0, 991, None);
    let enemy = unit(&mut s, 1, 992, Some(0));
    s.game.players[0].hand = vec![383, 70];
    assert_eq!(
        call(
            &mut s,
            "apply",
            json!({"type":"play_card_targets","hand_index":0,"targets":[own,enemy]})
        )["ok"],
        true
    );
    resolve(&mut s);
    let public = s.observe(1).unwrap();
    assert!(public["players"][0]["hand"].is_null());
    assert_eq!(public["players"][0]["banish"], json!([383]));
    assert_eq!(public["players"][1]["units"][0]["damage"], 3);
    assert_eq!(public["pending_card_plays"][0]["card_id"], 70);
    assert_eq!(public["decision"]["kind"], "arcane_shift");
    assert_eq!(
        public["decision"]["instruction"]["destination"],
        json!({"mode":"normal"})
    );
    assert_eq!(
        public["players"][0]["legend"]["targeting_progress"]["completed"],
        1
    );
}

#[test]
fn token_choices_and_pending_instructions_agree_on_each_physical_base() {
    for actor in 0..2 {
        let mut s = session();
        s.game = crate::game::test_support::position(actor, 0, 468305);
        unit(&mut s, actor, 991, None);
        s.game.players[actor].units[0].card_id = 265;
        s.game.players[actor].hand = vec![319];
        assert_eq!(
            call(
                &mut s,
                "apply",
                json!({"type":"play_card_targets","hand_index":0,"lane":null,"targets":[]})
            )["ok"],
            true
        );
        resolve(&mut s);
        assert!(matches!(s.game.decision, Some(Decision::TokenPlay { .. })));
        let own = s.observe(actor).unwrap();
        let other = s.observe(1 - actor).unwrap();
        let expected = json!({"mode":"fixed","lane":null,"base_player":actor});
        assert_eq!(own["pending_token_plays"], other["pending_token_plays"]);
        assert_eq!(own["pending_token_plays"][0]["destination"], expected);
        assert_eq!(own["decision"]["destination"], expected);
        let before = (s.revision(), format!("{:?}", s.game));
        assert_eq!(
            call(
                &mut s,
                "apply",
                json!({"type":"finalize_token_play","lane":0,"accelerate":false})
            )["ok"],
            false
        );
        assert_eq!((s.revision(), format!("{:?}", s.game)), before);
        assert_eq!(
            call(
                &mut s,
                "apply",
                json!({"type":"finalize_token_play","lane":null,"accelerate":false})
            )["ok"],
            true
        );
        assert!(
            s.game.players[actor]
                .units
                .iter()
                .any(|u| u.token && u.ready && u.lane.is_none())
        );
        assert!(s.game.pending_tokens().is_empty());
    }
}
#[test]
fn c17_signatures_are_legal_only_in_their_matching_legend_decks() {
    for (card, legend) in [(382, 8), (383, 9)] {
        for id in 0..crate::legend_catalog::legends().len() {
            assert_eq!(
                crate::deck::eligible_cards(id).contains(&card),
                id == legend
            );
        }
        let mut deck = CustomDeck::from_legend(legend).unwrap();
        deck.cards.push((card, 3));
        deck.validate().unwrap();
        deck.cards.last_mut().unwrap().1 = 4;
        assert!(deck.validate().is_err());
    }
}
