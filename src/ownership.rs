//! Core 056/127: control does not change a card's non-Board destination.
use super::*;

impl Game {
    /// Ownership of a current unit or gear, independently of its controller.
    /// An invalid or departed board reference has no owner in this query.
    pub fn board_owner(&self, target: Target) -> Option<usize> {
        let (controller, id) = match target {
            Target::Unit { player, unit_id } if self.unit_ref(target).is_some() => {
                (player, unit_id)
            }
            Target::Gear { player, gear_id }
                if self
                    .players
                    .get(player)
                    .is_some_and(|state| state.gear.iter().any(|gear| gear.id == gear_id)) =>
            {
                (player, gear_id)
            }
            _ => return None,
        };
        Some(
            self.board_owner_overrides
                .get(&id)
                .copied()
                .unwrap_or(controller),
        )
    }

    /// Called once when a physical board object departs, after any replacement
    /// that keeps it on the board has been handled. Tokens also clear the entry.
    pub(crate) fn departed_board_owner(&mut self, controller: usize, id: u32) -> usize {
        self.board_owner_overrides.remove(&id).unwrap_or(controller)
    }
}

#[cfg(test)]
#[path = "ownership_tests.rs"]
mod tests;
