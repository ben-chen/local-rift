//! Controlled replacement positions; all play and replacement choices use JSON.
use super::*;
use crate::{game::Unit, gear::GearState};

fn position(actor: usize, replacement: bool) -> Session {
    let mut session = Session::new(ResetOptions {
        decks: [2, 2],
        first_player: actor,
        ..Default::default()
    })
    .unwrap();
    session.start(0).unwrap();
    session.game.battlefields = vec![9, 9];
    session.game.battlefield_owner = vec![None; 2];
    for p in &mut session.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy = 10;
        p.energy_pool = 10;
        p.power_pool = [10; 6];
    }
    session.game.players[actor].hand = vec![447];
    let enemy = actor;
    session.game.players[enemy].units.push(Unit {
        id: 44701,
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
    let mut recipient = session.game.players[actor].units[0].clone();
    recipient.id = 44704;
    recipient.lane = None;
    session.game.players[actor].units.push(recipient);
    if replacement {
        session.game.players[enemy].gear = [44702, 44703]
            .into_iter()
            .map(|id| GearState {
                id,
                card_id: 21,
                lane: None,
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            })
            .collect();
    }
    session
}

fn apply(session: &mut Session, action: Action) {
    let result = session.handle(json!({"command":"apply", "revision":session.revision(),
        "player":session.game.priority(), "action":action_to_json(&action)}));
    assert_eq!(result["ok"], true, "{result}");
}

fn play(session: &mut Session, actor: usize) {
    apply(
        session,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![
                Target::Unit {
                    player: actor,
                    unit_id: 44701,
                },
                Target::Unit {
                    player: actor,
                    unit_id: 44704,
                },
            ],
        },
    );
    apply(session, Action::Pass);
    apply(session, Action::Pass);
}

#[test]
fn deathgrip_protocol_keeps_replacement_before_private_draw_and_rejects_stale_choices() {
    for actor in 0..2 {
        let mut session = position(actor, true);
        play(&mut session, actor);
        let enemy = actor;
        let before = [session.observe(0).unwrap(), session.observe(1).unwrap()];
        assert_eq!(before[actor]["decision"]["type"], "death_replacement");
        assert_eq!(before[0]["chain"], before[1]["chain"]);
        assert!(session.game.players[enemy].hand.is_empty());
        let action = Action::ChooseDeathReplacement {
            unit_id: 44701,
            gear_id: Some(44702),
        };
        let revision = session.revision();
        for (player, submitted) in [(1 - actor, revision), (actor, revision - 1)] {
            let rejected = session.handle(json!({"command":"apply","revision":submitted,
                "player":player,"action":action_to_json(&action)}));
            assert_eq!(rejected["ok"], false);
            assert_eq!(session.revision(), revision);
            assert_eq!(
                [session.observe(0).unwrap(), session.observe(1).unwrap()],
                before
            );
        }
        apply(&mut session, action);
        assert_eq!(session.game.players[enemy].hand, vec![71]);
        assert_eq!(session.game.players[enemy].units[0].lane, None);
        assert_eq!(session.game.players[enemy].units[1].temporary_buff, 0);
        let owner = session.observe(enemy).unwrap();
        let other = session.observe(1 - actor).unwrap();
        assert_eq!(owner["players"][enemy]["hand"].as_array().unwrap().len(), 1);
        assert!(other["players"][enemy]["hand"].is_null());
        assert_eq!(owner["damage_attribution"], other["damage_attribution"]);
    }
}

#[test]
fn deathgrip_api_reports_kill_without_fabricated_damage_and_preserves_private_draws() {
    for actor in 0..2 {
        let mut session = position(actor, false);
        play(&mut session, actor);
        let view = session.observe(actor).unwrap();
        assert_eq!(session.game.players[actor].units.len(), 1);
        assert_eq!(session.game.players[actor].units[0].temporary_buff, 8);
        assert_eq!(session.game.players[actor].hand, vec![71]);
        assert!(session.observe(1 - actor).unwrap()["players"][actor]["hand"].is_null());
        assert!(view["players"][1 - actor]["hand"].is_null());
        let ledger = &view["damage_attribution"]["last"];
        assert_eq!(ledger["parent"]["source"]["card_id"], 447);
        assert_eq!(ledger["damage"], json!([]));
        assert_eq!(ledger["kills"][0]["kind"], "instruction");
        assert_eq!(ledger["kills"][0]["credits"][0]["player"], actor);
    }
}
