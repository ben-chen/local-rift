//! Small official-text scenarios; all responses and choices use engine actions.
use super::*;
use test_support::{
    combat_position, position, ready_unit, reject_unchanged, settle_unopposed_chain,
};

fn play(targets: Vec<Target>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets,
    }
}

fn unit(game: &mut Game, owner: usize, card: CardId) -> Target {
    Target::Unit {
        player: owner,
        unit_id: ready_unit(game, owner, card),
    }
}

#[test]
fn thwonk_accepts_either_players_attackers_but_never_defenders_or_base_units() {
    for actor in 0..2 {
        for attacker_owner in 0..2 {
            let (mut game, attacker, defender) = combat_position(attacker_owner, 48401);
            if game.priority() != actor {
                game.apply(Action::Pass).unwrap();
            }
            game.players[actor].hand = vec![484];
            let base = unit(&mut game, attacker_owner, 71);
            assert_eq!(
                game.card_target_candidates(actor, 484, None, &[]).unwrap(),
                vec![attacker]
            );
            for invalid in [vec![], vec![defender], vec![base], vec![attacker, attacker]] {
                reject_unchanged(&mut game, play(invalid));
            }
            let energy = game.players[actor].energy_pool;
            let power = game.players[actor].power_pool;
            game.apply(play(vec![attacker])).unwrap();
            assert_eq!(game.players[actor].energy_pool, energy - 2);
            assert_eq!(game.players[actor].power_pool, power);
            settle_unopposed_chain(&mut game);
            let affected = game.unit_ref(attacker).unwrap();
            assert!(affected.stunned && affected.ready);
            assert_eq!(affected.damage, 0);
            assert!(!game.unit_ref(defender).unwrap().stunned);
        }
    }
}

#[test]
fn thwonk_repeat_uses_independent_targets_and_stunning_twice_does_not_return() {
    for actor in 0..2 {
        for same in [false, true] {
            let (mut game, first, _) = combat_position(1 - actor, 48402);
            let second = unit(&mut game, 1 - actor, 71);
            game.unit_mut(second).unwrap().lane = Some(0);
            game.apply(Action::Pass).unwrap();
            game.players[actor].hand = vec![484];
            let energy = game.players[actor].energy_pool;
            game.apply(Action::PlayCardOptions {
                hand_index: 0,
                lane: None,
                targets: vec![first],
                repeat_targets: vec![if same { first } else { second }],
                repeat: true,
                accelerate: false,
                bonus: false,
            })
            .unwrap();
            assert_eq!(game.players[actor].energy_pool, energy - 4);
            settle_unopposed_chain(&mut game);
            assert!(game.unit_ref(first).unwrap().stunned);
            assert_eq!(game.unit_ref(second).unwrap().stunned, !same);
            assert!(game.players[1 - actor].hand.is_empty());
        }
    }
}

#[test]
fn thwonk_action_timing_and_live_attacker_rechecks_survive_responses() {
    for actor in 0..2 {
        let (mut game, attacker, _) = combat_position(1 - actor, 48403);
        game.apply(Action::Pass).unwrap();
        game.players[actor].hand = vec![7, 484];
        game.apply(play(vec![attacker])).unwrap();
        reject_unchanged(&mut game, play(vec![attacker]));

        let (mut game, attacker, defender) = combat_position(1 - actor, 48404);
        game.apply(Action::Pass).unwrap();
        game.players[actor].hand = vec![484];
        game.players[1 - actor].hand = vec![16];
        game.apply(play(vec![attacker])).unwrap();
        game.apply(Action::Pass).unwrap();
        game.apply(play(vec![attacker, defender])).unwrap();
        settle_unopposed_chain(&mut game);
        assert!(game.players.iter().all(|p| p.units.is_empty()));
        assert!(game.players[actor].discard.contains(&484));
    }
}

#[test]
fn friendship_counts_distinct_tags_across_own_board_and_expires_at_turn_end() {
    for actor in 0..2 {
        for target_owner in 0..2 {
            let mut game = position(actor, 0, 48501);
            let target = unit(&mut game, target_owner, 71);
            for (index, card) in [116, 107, 338, 88, 116, 88].into_iter().enumerate() {
                let tag_unit = unit(&mut game, actor, card);
                if index % 2 == 0 {
                    game.unit_mut(tag_unit).unwrap().lane = Some(1);
                }
            }
            game.players[actor].hand = vec![485];
            game.apply(play(vec![target])).unwrap();
            settle_unopposed_chain(&mut game);
            assert_eq!(game.unit_ref(target).unwrap().temporary_buff, 4);
            assert_eq!(
                game.unit_ref(target).unwrap().buff,
                0,
                "giving Might is not Buff"
            );
            game.apply(Action::EndTurn).unwrap();
            assert_eq!(game.unit_ref(target).unwrap().temporary_buff, 0);
        }
    }
}

#[test]
fn friendship_ignores_opponents_private_zones_and_factory_tags() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 48502);
        let target = unit(&mut game, actor, 71);
        for card in [116, 107, 338, 88] {
            unit(&mut game, 1 - actor, card);
        }
        game.players[actor].discard = vec![116, 107, 338, 88];
        game.players[actor].hand = vec![485, 116, 107, 338, 88];
        // Synthetic identity fixture: a token never borrows a factory's Poro tag.
        let token = unit(&mut game, actor, 88);
        game.unit_mut(token).unwrap().token = true;
        game.apply(play(vec![target])).unwrap();
        settle_unopposed_chain(&mut game);
        assert_eq!(game.unit_ref(target).unwrap().temporary_buff, 0);
    }
}

#[test]
fn friendship_recounts_after_a_response_removes_the_last_unit_with_a_tag() {
    for actor in 0..2 {
        let mut game = position(actor, 0, 48503);
        let target = unit(&mut game, actor, 71);
        let bird = unit(&mut game, actor, 116);
        unit(&mut game, actor, 88);
        let enemy = unit(&mut game, 1 - actor, 71);
        game.players[actor].hand = vec![485];
        game.players[1 - actor].hand = vec![16];
        game.apply(play(vec![target])).unwrap();
        while let Some(action) = test_support::decline_legend_action(&game) {
            game.apply(action).unwrap();
        }
        assert!(game.decision.is_none(), "{:?}", game.decision);
        game.apply(Action::Pass).unwrap();
        game.apply(play(vec![enemy, bird])).unwrap();
        settle_unopposed_chain(&mut game);
        assert_eq!(game.unit_ref(target).unwrap().temporary_buff, 1);
        assert!(game.players[actor].hand.contains(&116));
    }
}
