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
