//! Authored rules positions with public actions for every subsequent transition.
use super::*;

pub(crate) fn position(actor: usize, preset: usize, seed: u64) -> Game {
    let mut game = Game::new_for_agents(preset, preset, seed, Difficulty::Tactical);
    game.choose_starting_player(actor).unwrap();
    game.start_match().unwrap();
    game.battlefields = vec![3, 3];
    game.battlefield_owner = vec![None; 2];
    for state in &mut game.players {
        state.units.clear();
        state.gear.clear();
        state.hidden.clear();
        state.hand.clear();
        state.discard.clear();
        state.banish.clear();
        state.champion = None;
        state.deck = vec![71; 20];
        state.set_rune_setup([0; 6], [0; 6]).unwrap();
        state.energy_pool = 20;
        state.energy = 20;
        state.power_pool = [5; 6];
        state.any_power = 0;
    }
    for legend in &mut game.legend {
        legend.ready = false;
    }
    game
}

pub(crate) fn ready_unit(game: &mut Game, owner: usize, card: CardId) -> u32 {
    let id = game.create_unit(owner, card, false, 0);
    game.players[owner].units.last_mut().unwrap().ready = true;
    id
}

/// Authored combat position with one vanilla unit on each side. The attacking
/// turn player has initial focus; callers pass normally to give the defender
/// a turn. Card-specific hands and effects belong in the scenario itself.
pub(crate) fn combat_position(attacker: usize, seed: u64) -> (Game, Target, Target) {
    let mut game = position(attacker, 0, seed);
    let mut unit = |owner| {
        let unit_id = ready_unit(&mut game, owner, 71);
        let target = Target::Unit {
            player: owner,
            unit_id,
        };
        game.unit_mut(target).unwrap().lane = Some(0);
        target
    };
    let first = unit(attacker);
    let second = unit(1 - attacker);
    game.battlefield_owner[0] = Some(1 - attacker);
    game.open_showdown(0, attacker).unwrap();
    assert_eq!(game.priority(), attacker);
    (game, first, second)
}

/// Authored state boundary, not a control-taking game action. Callers arrange
/// the destination/contest state explicitly; only identity and owner persist.
pub(crate) fn transfer_control_for_fixture(game: &mut Game, id: u32) -> Target {
    let old = game.unit_controller(id).unwrap();
    let owner = game
        .board_owner(Target::Unit {
            player: old,
            unit_id: id,
        })
        .unwrap();
    let index = game.players[old]
        .units
        .iter()
        .position(|u| u.id == id)
        .unwrap();
    let unit = game.players[old].units.remove(index);
    let player = 1 - old;
    game.players[player].units.push(unit);
    if player == owner {
        game.board_owner_overrides.remove(&id);
    } else {
        game.board_owner_overrides.insert(id, owner);
    }
    Target::Unit {
        player,
        unit_id: id,
    }
}

/// Resolve an unopposed chain through public actions. Tests must supply every
/// non-ordering choice except optional legend occurrences explicitly declined.
pub(crate) fn settle_unopposed_chain(game: &mut Game) {
    for _ in 0..100 {
        if game.chain.is_empty() && game.decision.is_none() {
            return;
        }
        let action = if let Some(action) = decline_legend_action(game) {
            action
        } else if let Some(Decision::OrderTriggers { options, .. }) = &game.decision {
            Action::OrderTrigger {
                trigger_id: options[0].id,
            }
        } else {
            assert!(game.decision.is_none(), "unexpected {:?}", game.decision);
            Action::Pass
        };
        game.apply(action).unwrap();
    }
    panic!("chain did not finish");
}

/// Choose and decline an optional legend occurrence through ordinary actions.
/// Exhausting a legend does not suppress its trigger; it can still be ordered.
pub(crate) fn decline_legend_action(game: &Game) -> Option<Action> {
    match &game.decision {
        Some(Decision::OrderTriggers { options, .. }) => options.iter().find_map(|option| {
            game.pending_triggers
                .iter()
                .any(|pending| {
                    pending.option.id == option.id
                        && matches!(
                            pending.effect,
                            PendingTriggerEffect::Choice(Decision::Legend { .. })
                        )
                })
                .then_some(Action::OrderTrigger {
                    trigger_id: option.id,
                })
        }),
        Some(Decision::Legend { .. }) => Some(Action::Choose { accept: false }),
        _ => None,
    }
}

#[track_caller]
pub(crate) fn reject_unchanged(game: &mut Game, action: Action) {
    let before = format!("{game:?}");
    assert!(game.apply(action).is_err(), "invalid action was accepted");
    assert_eq!(
        format!("{game:?}"),
        before,
        "rejected action changed the position"
    );
}
