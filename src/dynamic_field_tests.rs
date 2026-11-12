//! Additional-field infrastructure in controlled positions. These tests do not
//! substitute for Baron Nashor's eventual entry-replacement acceptance cases.
use super::*;

fn position(actor: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 147149, Difficulty::Tactical);
    g.start_match().unwrap();
    g.active = actor;
    g.priority_player = actor;
    g.battlefields = vec![9, 9];
    for p in &mut g.players {
        p.units.clear();
        p.gear.clear();
        p.hand.clear();
        p.hidden.clear();
        p.champion = None;
        p.discard.clear();
        p.banish.clear();
        p.deck = vec![71; 20];
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy = 30;
        p.energy_pool = 30;
        p.power_pool = [10; 6];
    }
    g
}

fn unit(g: &mut Game, actor: usize, lane: Option<usize>) -> u32 {
    let id = g.create_unit(actor, 71, false, 0);
    let u = g.players[actor].units.last_mut().unwrap();
    u.lane = lane;
    u.ready = true;
    id
}

#[test]
fn dynamic_fields_append_preserves_existing_history_and_turn_reset_covers_all_fields() {
    let mut g = position(0);
    g.scored_this_turn[0] = true;
    g.player_scored_this_turn[0][0] = true;
    g.spring_used[1][1] = true;
    g.play_history.units_or_gears_here[1][1] = 2;
    for count in 3..=4 {
        assert_eq!(g.add_battlefield(9), count - 1);
        assert_eq!(g.battlefield_count(), count);
        assert_eq!(g.battlefields, vec![9; count]);
        assert!(g.scored_this_turn[0]);
        assert!(g.player_scored_this_turn[0][0]);
        assert!(g.spring_used[1][1]);
        assert_eq!(g.play_history.units_or_gears_here[1][1], 2);
        assert!(!g.scored_this_turn[count - 1]);
        assert_eq!(g.play_history.units_or_gears_here[1][count - 1], 0);
        g.validate_numeric_state().unwrap();
    }
    g.start_turn().unwrap();
    assert_eq!(g.battlefield_count(), 4);
    assert!(g.scored_this_turn.iter().all(|used| !used));
    assert!(g.player_scored_this_turn.iter().flatten().all(|used| !used));
    assert!(g.spring_used.iter().flatten().all(|used| !used));
    assert!(
        g.play_history
            .units_or_gears_here
            .iter()
            .flatten()
            .all(|n| *n == 0)
    );
    g.validate_numeric_state().unwrap();
}

#[test]
fn dynamic_fields_final_conquest_requires_every_physical_field() {
    for actor in 0..2 {
        for count in 3..=4 {
            let mut g = position(actor);
            for _ in 2..count {
                g.add_battlefield(9);
            }
            g.players[actor].score = 7;
            let hand = g.players[actor].hand.len();
            for lane in 0..count - 1 {
                g.score_battlefield(actor, lane, false).unwrap();
                assert_eq!(g.winner, None);
                assert_eq!(g.players[actor].score, 7);
            }
            assert_eq!(g.players[actor].hand.len(), hand + count - 1);
            g.score_battlefield(actor, count - 1, false).unwrap();
            assert_eq!(g.winner, Some(actor));
            assert_eq!(g.players[actor].score, 8);
            assert!(g.player_scored_this_turn[actor].iter().all(|x| *x));
            assert!(g.player_scored_this_turn[1 - actor].iter().all(|x| !*x));
        }
    }
}

#[test]
fn dynamic_fields_move_play_and_target_candidates_include_the_added_field() {
    for actor in 0..2 {
        let mut g = position(actor);
        let lane = g.add_battlefield(9);
        let mover = unit(&mut g, actor, None);
        let movement = Action::Move {
            unit_id: mover,
            lane: Some(lane),
        };
        assert!(g.legal_actions().unwrap().contains(&movement));
        g.apply(movement).unwrap();
        for _ in 0..8 {
            if g.showdown.is_none() && g.chain.is_empty() && g.decision.is_none() {
                break;
            }
            g.apply(Action::Pass).unwrap();
        }
        assert_eq!(g.battlefield_owner[lane], Some(actor));
        assert_eq!(g.players[actor].score, 1);
        g.players[actor].hand.push(439);
        g.legend[actor] = LegendState::from_legend(10).unwrap();
        let play = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: Some(lane),
            targets: vec![],
        };
        assert!(g.play_choices(&play).unwrap().legend_entry_ready.is_some());
        g.apply(play).unwrap();
        let riven = g.players[actor]
            .units
            .iter()
            .find(|u| u.card_id == 439)
            .unwrap();
        assert!(riven.ready);
        assert_eq!(riven.lane, Some(lane));
        assert_eq!(g.play_history.units_or_gears_here[actor][lane], 1);
        g.validate_numeric_state().unwrap();
        let before = format!("{g:?}");
        assert!(
            g.apply(Action::Move {
                unit_id: riven.id,
                lane: Some(usize::MAX)
            })
            .is_err()
        );
        assert_eq!(format!("{g:?}"), before);
    }
}

#[test]
fn dynamic_fields_tokens_and_rematches_do_not_lose_or_reuse_added_field_state() {
    for actor in 0..2 {
        let mut g = position(actor);
        let lane = g.add_battlefield(9);
        g.queue_token_play(actor, 62, 1, Some(Some(lane)), false);
        g.after_resolution().unwrap();
        assert_eq!(g.players[actor].units[0].lane, Some(lane));
        assert_eq!(g.play_history.units_or_gears_here[actor][lane], 1);
        g.rematch(149147);
        assert_eq!(g.battlefields, vec![9, 9]);
        assert_eq!(g.battlefield_owner, vec![None; 2]);
        assert_eq!(g.play_history.units_or_gears_here, [vec![0; 2], vec![0; 2]]);
        g.validate_numeric_state().unwrap();
    }
}

#[test]
fn dynamic_fields_malformed_public_vectors_fail_before_an_action_mutates_state() {
    let mut g = position(0);
    g.battlefields.push(9);
    let before = format!("{g:?}");
    assert!(
        g.apply(Action::EndTurn)
            .unwrap_err()
            .contains("Battlefield state")
    );
    assert_eq!(format!("{g:?}"), before);
}
