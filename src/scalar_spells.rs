//! Context-sensitive spell instructions. Values are evaluated on resolution;
//! costs are evaluated separately during finalization.
use super::damage::{DamageContext, DamageDealer};
use super::{Game, Target};
use crate::rules::{OfficialEffect, SpellDamage, SpellMight};

impl Game {
    pub(super) fn controls_empowered(&self, player: usize) -> bool {
        self.legend[player].empowered
            || self.players[player]
                .units
                .iter()
                .any(|unit| self.unit_empowered(unit.id))
            || self.players[player]
                .gear
                .iter()
                .any(|gear| self.unit_empowered(gear.id))
    }

    pub(super) fn resolve_scalar_spell(
        &mut self,
        player: usize,
        card_id: usize,
        effect: OfficialEffect,
        valid: &[Target],
    ) -> Result<(), String> {
        match effect {
            OfficialEffect::EmpoweredFight { amount } => {
                // Losing one target does not erase the independent Might
                // instruction. Never promote the surviving enemy to slot one.
                if let Some(&friendly) = valid.iter().find(|target| {
                    matches!(target, Target::Unit { player: owner, .. } if *owner == player)
                }) {
                    self.modify_might(friendly, i128::from(amount), 0)?;
                    if let Some(&enemy) = valid.iter().find(|target| {
                        matches!(target, Target::Unit { player: owner, .. } if *owner != player)
                    }) {
                        self.fight_units(friendly, enemy)?;
                    }
                }
            }
            OfficialEffect::DamageAndGold { amount } => {
                for &target in valid {
                    self.spell_damage(player, target, amount)?;
                }
                // A failed damage target does not stop the next instruction.
                // The token waits until the parent spell completes.
                self.queue_token_play(player, card_id, 0, Some(None), true);
            }
            OfficialEffect::ContextualDamage(condition) => {
                for &target in valid {
                    let amount = self.contextual_damage(player, target, condition);
                    self.spell_damage(player, target, amount)?;
                }
            }
            OfficialEffect::ContextualMight(condition) => {
                for &target in valid {
                    let amount = match condition {
                        SpellMight::ControllerXp {
                            minimum_xp,
                            below,
                            at_least,
                        } => i128::from(if self.players[player].xp >= u64::from(minimum_xp) {
                            at_least
                        } else {
                            below
                        }),
                        SpellMight::Empowered { base, empowered } => {
                            if self
                                .unit_ref(target)
                                .is_some_and(|u| self.unit_empowered(u.id))
                            {
                                i128::from(empowered)
                            } else {
                                i128::from(base)
                            }
                        }
                        SpellMight::EnemiesHere { per_enemy } => {
                            let lane = self.target_lane(target);
                            let enemies = self.players[1 - player]
                                .units
                                .iter()
                                .filter(|unit| unit.lane == lane)
                                .count();
                            scaled_count(enemies, per_enemy)?
                        }
                        SpellMight::OwnExhausted { per_unit } => scaled_count(
                            self.players[player]
                                .units
                                .iter()
                                .filter(|unit| !unit.ready)
                                .count(),
                            per_unit,
                        )?,
                    };
                    self.modify_might(target, amount, 0)?;
                }
            }
            OfficialEffect::DrawForBattlefields { base } => {
                self.draw(player, base);
                let count = self
                    .battlefield_owner
                    .iter()
                    .filter(|&&owner| owner == Some(player))
                    .count();
                self.draw_count(player, count);
            }
            OfficialEffect::DrawForMighty => {
                // A numeric failure is not a unit failing the printed test.
                // Determine the whole count before drawing any cards.
                let mut count = 0;
                for unit in &self.players[player].units {
                    if self.unit_might(unit)? >= 5 {
                        count += 1;
                    }
                }
                self.draw_count(player, count);
            }
            _ => unreachable!("only contextual spell effects reach this resolver"),
        }
        Ok(())
    }

    pub(super) fn contextual_damage(
        &self,
        player: usize,
        target: Target,
        condition: SpellDamage,
    ) -> u8 {
        match condition {
            SpellDamage::OwnFacedown { base, conditional } => {
                if self.players[player].hidden.is_empty() {
                    base
                } else {
                    conditional
                }
            }
            SpellDamage::Attacking { base, conditional } => {
                let attacking = self.showdown.as_ref().is_some_and(|showdown| {
                    showdown.combat
                        && matches!(target, Target::Unit { player: owner, .. } if owner == showdown.attacker)
                        && self.target_lane(target) == Some(showdown.lane)
                });
                if attacking { conditional } else { base }
            }
        }
    }

    fn spell_damage(&mut self, player: usize, target: Target, amount: u8) -> Result<(), String> {
        let context = self.current_damage_context().unwrap_or(DamageContext {
            player,
            bonus_damage: 0,
            dealer: DamageDealer::Ability,
        });
        self.deal_damage(context, target, u64::from(amount))
    }

    fn draw_count(&mut self, player: usize, count: usize) {
        for _ in 0..count {
            if self.winner.is_some() {
                break;
            }
            self.draw(player, 1);
        }
    }
}

fn scaled_count(count: usize, per_unit: i8) -> Result<i128, String> {
    // Keep the full intermediate until modify_might checks the resulting
    // aggregate. Large opposite-signed contributions may cancel legally.
    i128::try_from(count)
        .ok()
        .and_then(|count| count.checked_mul(i128::from(per_unit)))
        .ok_or_else(|| crate::numeric::capacity("count-based Might change"))
}

#[cfg(test)]
mod tests {
    use super::scaled_count;

    #[test]
    fn count_changes_have_no_byte_or_signed_sixteen_bit_cap() {
        for count in [255, 256, 32767, 32768, 65535, 65536, usize::MAX] {
            for per_unit in [i8::MIN, -2, -1, 0, 1, 2, i8::MAX] {
                assert_eq!(
                    scaled_count(count, per_unit).unwrap(),
                    (count as i128) * i128::from(per_unit)
                );
            }
        }
    }
}
