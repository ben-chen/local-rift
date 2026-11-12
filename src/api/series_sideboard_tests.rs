use super::*;

fn position(actor: usize) -> Session {
    let mut s = Session::new(ResetOptions {
        decks: [3, 3],
        ..Default::default()
    })
    .unwrap();
    s.game.choose_starting_player(actor).unwrap();
    s.start(0).unwrap();
    s.game.battlefields = vec![3, 3];
    s.game.battlefield_owner = vec![None; 2];
    for p in &mut s.game.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 50;
        p.energy = 50;
        p.power_pool = [20; 6];
    }
    s
}
fn apply(s: &mut Session, a: Action) {
    let reply = s.handle(json!({"command":"apply","revision":s.revision(),
        "player":s.game.priority(),"action":action_to_json(&a)}));
    assert_eq!(reply["ok"], true, "{reply}");
}

#[test]
fn decree_reveal_is_public_only_during_choice_and_failed_choices_are_atomic() {
    for actor in 0..2 {
        let mut s = position(actor);
        s.game.players[actor].hand = vec![433];
        s.game.players[1 - actor].hand = vec![7, 283, 79];
        assert!(s.observe(actor).unwrap()["players"][1 - actor]["hand"].is_null());
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
        for viewer in 0..2 {
            let view = s.observe(viewer).unwrap();
            assert_eq!(view["players"][1 - actor]["hand"], json!([7, 283, 79]));
            assert_eq!(view["decision"]["type"], "recycle_revealed_hand");
            assert_eq!(view["decision"]["domain"], "Mind");
            assert_eq!(view["decision"]["eligible_indices"], json!([1, 2]));
        }
        let before = s.observe(actor).unwrap();
        let rev = s.revision();
        assert!(
            s.apply_action(actor, rev, Action::SelectCards { indices: vec![0] })
                .is_err()
        );
        assert!(
            s.apply_action(1 - actor, rev, Action::SelectCards { indices: vec![1] })
                .is_err()
        );
        assert_eq!(s.revision(), rev);
        assert_eq!(s.observe(actor).unwrap(), before);
        apply(&mut s, Action::SelectCards { indices: vec![1] });
        assert!(s.observe(actor).unwrap()["players"][1 - actor]["hand"].is_null());
        assert_eq!(
            s.observe(1 - actor).unwrap()["players"][1 - actor]["hand"],
            json!([7, 79])
        );
        assert_eq!(s.game.players[1 - actor].deck[0], 283);
    }
}

#[test]
fn catalog_exposes_complete_registrations_and_exact_lacerate_target_and_flow_cost() {
    let cat = catalog();
    // Source registration validation is exercised by the same public reset API.
    for p in [0, 2, 4, 5] {
        let mut s = Session::default();
        let reply=s.handle(json!({"command":"series_begin","revision":0,"registrations":[{"preset":p},{"preset":5}],"designated_player":0}));
        assert_eq!(reply["ok"], true, "preset {p}: {reply}");
    }
    assert!(!cat.is_null());
    let mut s = position(0);
    s.game.players[0].hand = vec![434];
    for (owner, id, card) in [(0, 100, 73), (1, 101, 283)] {
        s.game.players[owner].units.push(crate::game::Unit {
            id,
            card_id: card,
            lane: None,
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        });
    }
    let view = s
        .target_candidates(
            0,
            s.revision(),
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            },
            0,
        )
        .unwrap();
    assert_eq!(view.candidates.len(), 2);
    s.game.empowered_units.push(101);
    apply(
        &mut s,
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: 1,
                unit_id: 101,
            }],
        },
    );
    apply(&mut s, Action::Pass);
    apply(&mut s, Action::Pass);
    assert!(s.game.players[1].units.is_empty());
    assert_eq!(s.game.players[0].units.len(), 1);
    assert_eq!(
        s.observe(0).unwrap()["players"][0]["flow_costs"],
        json!([{"trash_index":0,"costs":[{"source":"printed","energy":4,"power":2,"colored":true}]}])
    );
}
