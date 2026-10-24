//! XP has no rules maximum (Core733). The host representation uses checked
//! arithmetic; reaching its capacity is an engine error, never a game rule.
use crate::{game::Game, rules::EntryReadyRule};

pub type XpAmount = u64;

impl Game {
    pub(crate) fn gain_xp(&mut self, player: usize, amount: XpAmount) -> Result<(), String> {
        let state = self.players.get_mut(player).ok_or("Invalid XP player.")?;
        let next = state
            .xp
            .checked_add(amount)
            .ok_or("engine_capacity: XP exceeds the supported integer representation.")?;
        state.xp = next;
        if amount > 0 {
            self.trigger_state.xp_gained[player] = true;
        }
        Ok(())
    }

    pub(crate) fn spend_xp(&mut self, player: usize, amount: XpAmount) -> Result<(), String> {
        if player >= self.players.len() {
            return Err("Invalid XP player.".into());
        }
        if self.payment_record_xp(player, amount)? {
            return Ok(());
        }
        let state = self.players.get_mut(player).ok_or("Invalid XP player.")?;
        state.xp = state.xp.checked_sub(amount).ok_or("Not enough XP.")?;
        Ok(())
    }

    pub fn gained_xp_this_turn(&self, player: usize) -> bool {
        self.trigger_state
            .xp_gained
            .get(player)
            .copied()
            .unwrap_or(false)
    }

    pub fn printed_entry_ready(&self, player: usize, card_id: usize) -> bool {
        let Some(state) = self.players.get(player) else {
            return false;
        };
        if self.catalog.get(card_id).is_none() {
            return false;
        }
        match crate::rules::entry_ready_rule(card_id) {
            Some(EntryReadyRule::Always) => true,
            Some(EntryReadyRule::ControllerXpAtLeast(n)) => state.xp >= XpAmount::from(n),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Difficulty;

    #[test]
    fn checked_gain_spend_and_history_do_not_saturate_or_fabricate_events() {
        let mut game = Game::new_for_agents(0, 0, 1, Difficulty::Easy);
        assert!(game.gain_xp(2, 1).is_err());
        assert!(game.spend_xp(2, 1).is_err());
        assert!(!game.gained_xp_this_turn(2));
        assert!(!game.printed_entry_ready(2, 334));
        assert!(!game.printed_entry_ready(0, usize::MAX));
        game.gain_xp(0, 0).unwrap();
        assert!(!game.gained_xp_this_turn(0));
        game.players[0].xp = u64::MAX - 2;
        game.gain_xp(0, 2).unwrap();
        assert_eq!(game.players[0].xp, u64::MAX);
        assert!(game.gained_xp_this_turn(0));
        assert!(
            game.gain_xp(0, 1)
                .unwrap_err()
                .starts_with("engine_capacity:")
        );
        assert_eq!(game.players[0].xp, u64::MAX);
        game.spend_xp(0, u64::MAX).unwrap();
        assert_eq!(game.players[0].xp, 0);
        assert!(game.gained_xp_this_turn(0));
        assert!(game.spend_xp(0, 1).is_err());
    }
}
