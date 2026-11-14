//! Keyword grants follow a physical unit through movement and expire at Ending
//! cleanup, after ending triggers. Leaving the board removes every such grant.
use crate::game::{Game, Target};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnKeyword {
    Shield(u8),
    Tank,
}

impl Game {
    pub(crate) fn grant_turn_keyword(&mut self, target: Target, keyword: TurnKeyword) {
        if let Target::Unit { unit_id, .. } = target
            && self.unit_ref(target).is_some()
        {
            self.turn_keyword_grants.push((unit_id, keyword));
        }
    }

    /// The temporary portion of Shield; printed and equipped Shield are separate.
    pub fn turn_shield(&self, unit_id: u32) -> Result<u64, String> {
        self.turn_keyword_grants
            .iter()
            .try_fold(0, |sum, (id, keyword)| {
                if *id == unit_id
                    && let TurnKeyword::Shield(amount) = keyword
                {
                    crate::numeric::add_u64(sum, u64::from(*amount), "shield")
                } else {
                    Ok(sum)
                }
            })
    }

    /// Tank is redundant, so several grants still impose a single restriction.
    pub fn turn_tank(&self, unit_id: u32) -> bool {
        self.turn_keyword_grants
            .contains(&(unit_id, TurnKeyword::Tank))
    }
}
