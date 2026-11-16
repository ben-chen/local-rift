//! Scoring prohibitions and automatic battlefield instructions.
use super::*;
use crate::battlefields::{BACK_ALLEY_BAR_ID, FORGOTTEN_MONUMENT_ID};

impl Game {
    /// Remaining starts of this player's turns before this field permits scoring.
    /// None means this battlefield has no turn-based scoring restriction.
    pub fn battlefield_score_unlock(&self, player: usize, lane: usize) -> Option<u32> {
        (self.battlefields.get(lane) == Some(&FORGOTTEN_MONUMENT_ID))
            .then(|| 3u32.saturating_sub(self.turns_taken[player]))
    }

    pub fn player_turns_started(&self, player: usize) -> u32 {
        self.turns_taken[player]
    }

    pub(super) fn can_score_battlefield(&self, player: usize, lane: usize) -> bool {
        self.winner.is_none()
            && !self.player_scored_this_turn[player][lane]
            && self
                .battlefield_score_unlock(player, lane)
                .is_none_or(|left| left == 0)
    }

    pub(super) fn back_alley_move(&mut self, origin: Option<usize>, unit_id: u32) {
        let Some(lane) = origin.filter(|&lane| self.battlefields[lane] == BACK_ALLEY_BAR_ID) else {
            return;
        };
        // CR190.6: snapshot control at the occurrence, before movement cleanup.
        let controller = self.battlefield_owner[lane].unwrap_or(self.active);
        self.queue_ability(
            controller,
            0,
            Vec::new(),
            ChainAbility::Battlefield(BattlefieldAbility::BackAlleyMight { lane, unit_id }),
        );
    }

    pub(crate) fn resolve_back_alley(&mut self, unit_id: u32) -> Result<(), String> {
        // "It" retains the physical unit even if its controller changes. This
        // is not a newly chosen target and does not ask for Deflect payment.
        if let Some(player) = self
            .players
            .iter()
            .position(|state| state.units.iter().any(|u| u.id == unit_id))
        {
            self.modify_might(Target::Unit { player, unit_id }, 1, 0)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "field_effects_tests.rs"]
mod tests;
