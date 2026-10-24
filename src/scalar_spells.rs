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
    ) {
        match effect {
            OfficialEffect::EmpoweredFight { amount } => {
                // Losing one target does not erase the independent Might
                // instruction. Never promote the surviving enemy to slot one.
                if let Some(&friendly) = valid.iter().find(|target| {
                    matches!(target, Target::Unit { player: owner, .. } if *owner == player)
                }) {
                    self.modify_might(friendly, i16::from(amount), 0);
                    if let Some(&enemy) = valid.iter().find(|target| {
                        matches!(target, Target::Unit { player: owner, .. } if *owner != player)
                    }) {
                        self.fight_units(friendly, enemy);
                    }
                }
            }
            OfficialEffect::DamageAndGold { amount } => {
                for &target in valid {
                    self.spell_damage(player, target, amount);
                }
                // A failed damage target does not stop the next instruction.
                // The token waits until the parent spell completes.
                self.queue_token_play(player, card_id, 0, Some(None), true);
            }
            OfficialEffect::ContextualDamage(condition) => {
                for &target in valid {
                    let amount = self.contextual_damage(player, target, condition);
                    self.spell_damage(player, target, amount);
                }
            }
            OfficialEffect::ContextualMight(condition) => {
                for &target in valid {
                    let amount = match condition {
                        SpellMight::Empowered { base, empowered } => {
                            if self
                                .unit_ref(target)
                                .is_some_and(|u| self.unit_empowered(u.id))
                            {
                                i16::from(empowered)
                            } else {
                                i16::from(base)
                            }
                        }
                        SpellMight::EnemiesHere { per_enemy } => {
                            let lane = self.target_lane(target);
                            let enemies = self.players[1 - player]
                                .units
                                .iter()
                                .filter(|unit| unit.lane == lane)
                                .count();
                            scaled_count(enemies, per_enemy)
                        }
                        SpellMight::OwnExhausted { per_unit } => scaled_count(
                            self.players[player]
                                .units
                                .iter()
                                .filter(|unit| !unit.ready)
                                .count(),
                            per_unit,
                        ),
                    };
                    self.modify_might(target, amount, 0);
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
                let count = self.players[player]
                    .units
                    .iter()
                    .filter(|unit| self.unit_might(unit) >= 5)
                    .count();
                self.draw_count(player, count);
            }
            _ => unreachable!("only contextual spell effects reach this resolver"),
        }
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

    fn spell_damage(&mut self, player: usize, target: Target, amount: u8) {
        let context = self.current_damage_context().unwrap_or(DamageContext {
            player,
            bonus_damage: 0,
            dealer: DamageDealer::Ability,
        });
        self.deal_damage(context, target, amount);
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

fn scaled_count(count: usize, per_unit: i8) -> i16 {
    let magnitude = count
        .saturating_mul(per_unit.unsigned_abs() as usize)
        .min(i16::MAX as usize) as i16;
    if per_unit < 0 { -magnitude } else { magnitude }
}
