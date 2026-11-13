//! Deathgrip's linked Kill/Might instructions and independent Draw (SFD errata).
use super::*;

#[derive(Clone, Debug)]
pub(super) struct DeathgripContinuation {
    pub player: usize,
    pub sacrifice: Option<Target>,
    pub recipient: Option<Target>,
    pub killed_might: Option<u64>,
}

impl Game {
    pub(super) fn begin_deathgrip(
        &mut self,
        player: usize,
        targets: &[Target],
    ) -> Result<(), String> {
        // Preserve ordered slots when either target becomes invalid. A valid
        // recipient must never take the departed sacrifice's place.
        let mut valid = [None, None];
        for (slot, &target) in targets.iter().take(2).enumerate() {
            if self.official_target_valid(player, 447, None, target, slot)? {
                valid[slot] = Some(target);
            }
        }
        debug_assert!(self.pending_deathgrip.is_none());
        self.pending_deathgrip = Some(DeathgripContinuation {
            player,
            sacrifice: valid[0],
            recipient: valid[1],
            killed_might: None,
        });
        if let Some(target) = valid[0] {
            self.kill_unit(target)?;
        }
        self.finish_deathgrip()
    }

    pub(super) fn finish_deathgrip(&mut self) -> Result<(), String> {
        if self.pending_instruction_deaths.is_some() {
            return Ok(());
        }
        if let Some(tail) = self.pending_deathgrip.take() {
            if let (Some(target), Some(might)) = (tail.recipient, tail.killed_might) {
                self.modify_might(target, i128::from(might), 0)?;
            }
            self.draw(tail.player, 1)?;
        }
        Ok(())
    }
}
