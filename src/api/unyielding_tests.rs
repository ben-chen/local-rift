use super::*;

fn apply(s: &mut Session, action: Value) {
    let result = s.handle(json!({"command":"apply","revision":s.revision(),"player":s.game.priority(),"action":action}));
    assert_eq!(result["ok"], true, "{result}");
}

#[test]
fn unyielding_api_has_no_targets_and_exposes_global_duration_to_both_seats() {
    for actor in 0..2 {
        let mut s = Session::new(ResetOptions {
            decks: [3, 3],
            ..Default::default()
        })
        .unwrap();
        s.game.choose_starting_player(actor).unwrap();
        s.start(0).unwrap();
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
        s.game.players[actor].hand = vec![441];
        let catalog = catalog();
        assert_eq!(
            catalog["cards"][441]["play_targets"],
            json!({"kind":null,"maximum":0,"required":0})
        );
        assert_eq!(
            catalog["cards"][441]["official_text"],
            s.game.catalog[441].official_text
        );
        for viewer in 0..2 {
            assert_eq!(s.observe(viewer).unwrap()["turn_effects"], json!([]));
        }
        apply(
            &mut s,
            json!({"type":"play_card_targets","hand_index":0,"lane":null,"targets":[]}),
        );
        assert_eq!(s.observe(actor).unwrap()["turn_effects"], json!([]));
        apply(&mut s, json!({"type":"pass"}));
        apply(&mut s, json!({"type":"pass"}));
        for viewer in 0..2 {
            let effect = &s.observe(viewer).unwrap()["turn_effects"][0];
            assert_eq!(effect["type"], "prevent_spell_and_ability_damage");
            assert_eq!(effect["scope"], "both_players");
            assert_eq!(effect["source_card_id"], 441);
            assert_eq!(effect["amount"], "all");
            assert_eq!(effect["expires"], "end_of_turn");
            assert_eq!(effect["prevents_unit_damage"], false);
            assert_eq!(effect["expires_turn"], s.game.turn);
        }
        apply(&mut s, json!({"type":"end_turn"}));
        for viewer in 0..2 {
            assert_eq!(s.observe(viewer).unwrap()["turn_effects"], json!([]));
        }
    }
}
