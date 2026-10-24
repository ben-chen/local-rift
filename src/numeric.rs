//! Checked dynamic combat values. Printed constants retain their compact types.
use crate::game::{BATTLEFIELDS, Game, Unit, UnitKeywords};
use crate::rules::{self, StaticAbility};
pub type Might = u64;
pub type SignedMight = i64;
pub type DamageAmount = u64;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitStats {
    pub signed_might: SignedMight,
    pub might: Might,
    pub keywords: UnitKeywords,
}
pub(crate) fn capacity(what: &str) -> String {
    format!("engine_capacity: {what} exceeds the supported numeric range")
}
pub(crate) fn add_u64(a: u64, b: u64, what: &str) -> Result<u64, String> {
    a.checked_add(b).ok_or_else(|| capacity(what))
}
pub(crate) fn signed(value: i128, what: &str) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| capacity(what))
}
pub(crate) fn is_capacity(error: &str) -> bool {
    error.starts_with("engine_capacity:")
}
impl Game {
    pub fn unit_might(&self, unit: &Unit) -> Result<Might, String> {
        Ok(self.try_unit_stats(unit)?.might)
    }

    pub fn unit_keywords(&self, unit: &Unit) -> Result<UnitKeywords, String> {
        let mut keywords = UnitKeywords::default();
        let owner = self
            .players
            .iter()
            .position(|p| p.units.iter().any(|u| u.id == unit.id));
        if unit.token {
            keywords.deflect = rules::token_spec(unit.card_id).is_some_and(|s| s.deflect) as u8;
        } else {
            for ability in rules::static_abilities(unit.card_id) {
                match *ability {
                    StaticAbility::Hunt(value) => {
                        keywords.hunt = keywords.hunt.saturating_add(value)
                    }
                    StaticAbility::Assault(value) => {
                        keywords.assault = add_u64(keywords.assault, u64::from(value), "assault")?
                    }
                    StaticAbility::Shield(value) => {
                        keywords.shield = add_u64(keywords.shield, u64::from(value), "shield")?
                    }
                    StaticAbility::Deflect(value) => {
                        keywords.deflect = keywords.deflect.saturating_add(value)
                    }
                    StaticAbility::Tank => keywords.tank = true,
                    StaticAbility::Backline => keywords.backline = true,
                    StaticAbility::Ganking => keywords.ganking = true,
                    StaticAbility::Ambush => keywords.ambush = true,
                    StaticAbility::LevelSixDeflectAndGanking
                        if owner.is_some_and(|p| self.players[p].xp >= 6) =>
                    {
                        keywords.deflect = keywords.deflect.saturating_add(1);
                        keywords.ganking = true;
                    }
                    _ => {}
                }
            }
            keywords.accelerate = rules::accelerate_cost(unit.card_id);
        }
        let conditional = self.continuous_keywords(unit)?;
        keywords.assault = add_u64(keywords.assault, conditional.assault, "assault")?;
        keywords.shield = add_u64(keywords.shield, conditional.shield, "shield")?;
        keywords.deflect = keywords.deflect.saturating_add(conditional.deflect);
        keywords.ganking |= conditional.ganking;
        let gear = self.gear_keywords(unit.id)?;
        keywords.hunt = keywords.hunt.saturating_add(gear.hunt);
        keywords.assault = add_u64(keywords.assault, gear.assault, "assault")?;
        keywords.shield = add_u64(keywords.shield, gear.shield, "shield")?;
        keywords.deflect = keywords.deflect.saturating_add(gear.deflect);
        keywords.tank |= gear.tank;
        keywords.backline |= gear.backline;
        keywords.ganking |= gear.ganking;
        for (_, value) in self
            .legend
            .iter()
            .flat_map(|l| &l.assault_grants)
            .filter(|(id, _)| *id == unit.id)
        {
            keywords.assault = add_u64(keywords.assault, u64::from(*value), "assault")?;
        }
        Ok(keywords)
    }

    /// The default automatic ordering. A unit with both Tank and Backline may
    /// instead be assigned last under CR467.2.c.8; explicit validation permits both.
    pub fn damage_priority(&self, unit: &Unit) -> Result<u8, String> {
        let keywords = self.unit_keywords(unit)?;
        Ok(if keywords.tank {
            0
        } else if keywords.backline {
            2
        } else {
            1
        })
    }

    // CR143.2.c.1: references treat negative Might as zero, but calculations
    // of increases/decreases (including Swap) use the actual signed value.
    pub(crate) fn actual_unit_might(&self, unit: &Unit) -> Result<SignedMight, String> {
        Ok(self.try_unit_stats(unit)?.signed_might)
    }

    pub fn try_unit_stats(&self, unit: &Unit) -> Result<UnitStats, String> {
        let base = if unit.token {
            0
        } else {
            self.catalog
                .get(unit.card_id)
                .ok_or("Invalid unit card identity.")?
                .might
        };
        let keywords = self.unit_keywords(unit)?;
        let mut value = i128::from(base) + i128::from(unit.buff) + i128::from(unit.temporary_buff);
        if let Some(w) = &self.showdown
            && w.combat
            && unit.lane == Some(w.lane)
        {
            if self.players[w.attacker]
                .units
                .iter()
                .any(|u| u.id == unit.id)
            {
                value += i128::from(keywords.assault);
            } else if self.players[1 - w.attacker]
                .units
                .iter()
                .any(|u| u.id == unit.id)
            {
                value += i128::from(keywords.shield);
            }
        }
        value += self.continuous_might_bonus(unit)?;
        value += i128::from(self.gear_might_bonus(unit.id)?);
        value += i128::from(self.legend_might_bonus(unit));
        let signed_might = signed(value, "unit Might")?;
        Ok(UnitStats {
            signed_might,
            might: signed_might.max(0) as u64,
            keywords,
        })
    }

    pub fn try_lane_might(&self, player: usize, lane: usize) -> Result<Might, String> {
        let state = self.players.get(player).ok_or("Invalid player.")?;
        if lane >= BATTLEFIELDS {
            return Err("Invalid battlefield.".into());
        }
        state
            .units
            .iter()
            .filter(|u| u.lane == Some(lane) && !u.stunned)
            .try_fold(0, |sum, unit| {
                add_u64(sum, self.unit_might(unit)?, "battlefield Might")
            })
    }
    pub fn lane_might(&self, player: usize, lane: usize) -> Result<Might, String> {
        self.try_lane_might(player, lane)
    }
    /// Validate derived quantities without performing cleanup or changing state.
    /// A lethal unit may legally remain while its parent effect is resolving.
    pub fn validate_numeric_state(&self) -> Result<(), String> {
        for player in 0..self.players.len() {
            for unit in &self.players[player].units {
                self.try_unit_stats(unit)?;
            }
            for lane in 0..BATTLEFIELDS {
                self.try_lane_might(player, lane)?;
            }
        }
        Ok(())
    }
}

/// An invalid action is false; capacity is an unresolved computation, not illegality.
pub(crate) fn legal_result(result: Result<(), String>) -> Result<bool, String> {
    match result {
        Ok(()) => Ok(true),
        Err(e) if is_capacity(&e) => Err(e),
        Err(_) => Ok(false),
    }
}
