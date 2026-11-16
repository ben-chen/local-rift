//! Ready-entry replacements do not perform a Ready action.
use super::*;

impl Game {
    pub fn played_spell_this_turn(&self, player: usize) -> bool {
        self.play_history
            .finalized_spells
            .get(player)
            .is_some_and(|spells| !spells.is_empty())
    }

    pub fn entry_ready_option(&self, card_id: CardId) -> Option<&'static str> {
        (card_id == 467).then_some("Enter ready · cannot move this turn")
    }

    pub(crate) fn selected_entry_ready(&self, player: usize, card_id: CardId, token: bool) -> bool {
        !token
            && self.active_play.as_ref().is_some_and(|play| {
                play.player == player
                    && play.card_id == card_id
                    && ((card_id == 466 && play.choices.bonus)
                        || (card_id == 467 && play.choices.entry_ready))
            })
    }

    pub(crate) fn record_entry_restriction(
        &mut self,
        player: usize,
        card_id: CardId,
        token: bool,
        unit_id: u32,
    ) {
        if card_id == 467 && self.selected_entry_ready(player, card_id, token) {
            self.immobile_units.push((player, unit_id));
        }
    }
}

#[cfg(test)]
#[path = "entry_tests.rs"]
mod tests;
