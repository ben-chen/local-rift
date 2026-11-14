//! Conditional play checks use finalization history (CR420.4.b), whereas
//! triggered play abilities wait for successful resolution (CR420.4.a).
use super::*;

#[cfg(test)]
#[path = "spell_history_tests.rs"]
mod tests;

impl Game {
    /// Whether an opponent finalized a different physical spell this turn.
    /// Its later resolution, countering, or change of controller is irrelevant.
    pub fn opponent_played_other_spell(&self, actor: usize, target_chain_id: u32) -> bool {
        let opponent = match actor {
            0 => 1,
            1 => 0,
            _ => return false,
        };
        self.play_history.finalized_spells[opponent]
            .iter()
            .any(|&id| id != target_chain_id)
    }
}
