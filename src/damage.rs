//! Deal-action provenance. Unit combat and Fight never inherit spell bonuses.
use super::{CardId, Game, Target};
use crate::numeric::{DamageAmount, add_u64, capacity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageDealer {
    Spell { card_id: CardId },
    Ability,
    Unit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DamageContext {
    /// Controller of the resolving spell/ability, not the current priority seat.
    pub player: usize,
    /// Bonuses captured by this spell when its Play action began.
    pub bonus_damage: DamageAmount,
    pub dealer: DamageDealer,
}

impl Game {
    /// CR714–715: add all applicable bonuses to each actual Deal packet.
    /// A zero/no Deal does not become damage merely because bonuses exist.
    pub(crate) fn damage_amount(
        &self,
        context: DamageContext,
        target: Target,
        amount: DamageAmount,
    ) -> Result<DamageAmount, String> {
        if amount == 0 || context.dealer == DamageDealer::Unit {
            return Ok(amount);
        }
        let mut total = u128::from(amount) + u128::from(context.bonus_damage);
        if self
            .unit_ref(target)
            .and_then(|unit| unit.lane)
            .is_some_and(|lane| self.battlefields[lane] == 3)
        {
            total += 1;
        }
        total += self.players[context.player]
            .units
            .iter()
            .filter(|unit| !unit.token && unit.card_id == 259)
            .count() as u128;
        if let DamageDealer::Spell { card_id: 260 } = context.dealer {
            // The physical spell stays on the chain, not in its own trash.
            // Count the controller's live copies at each Deal instruction.
            total += self.players[context.player]
                .discard
                .iter()
                .filter(|&&card| self.catalog[card].name == self.catalog[260].name)
                .count() as u128;
        }
        DamageAmount::try_from(total).map_err(|_| capacity("damage packet"))
    }

    pub(crate) fn mark_damage(
        &mut self,
        target: Target,
        amount: DamageAmount,
    ) -> Result<(), String> {
        if let Some(unit) = self.unit_mut(target) {
            unit.damage = add_u64(unit.damage, amount, "marked damage")?;
        }
        Ok(())
    }

    pub(crate) fn deal_damage(
        &mut self,
        context: DamageContext,
        target: Target,
        amount: DamageAmount,
    ) -> Result<(), String> {
        // An absent recipient does not create a Deal packet whose bonuses
        // could fail numeric validation after the chosen target has left.
        if self.unit_ref(target).is_none() {
            return Ok(());
        }
        let amount = self.damage_amount(context, target, amount)?;
        self.mark_damage(target, amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Difficulty, Unit};

    fn unit(id: u32, card_id: usize, lane: Option<usize>) -> Unit {
        Unit {
            id,
            card_id,
            lane,
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        }
    }
    fn position() -> Game {
        let mut game = Game::new_for_agents(3, 3, 8472, Difficulty::Easy);
        game.start_match().unwrap();
        game.players[0].units = vec![unit(100, 259, None), unit(101, 259, None)];
        game.players[1].units = vec![unit(102, 69, Some(0)), unit(103, 69, Some(1))];
        game.battlefields = [3, 2];
        game
    }
    fn target(id: u32) -> Target {
        Target::Unit {
            player: 1,
            unit_id: id,
        }
    }

    #[test]
    fn bonuses_use_actor_and_live_sources_plus_captured_item_bonus_at_each_target() {
        let mut game = position();
        let context = DamageContext {
            player: 0,
            bonus_damage: 3,
            dealer: DamageDealer::Spell { card_id: 53 },
        };
        // Two live Annies, one item bonus of3, and only the first target at Void Gate.
        assert_eq!(game.damage_amount(context, target(102), 3).unwrap(), 9);
        assert_eq!(game.damage_amount(context, target(103), 3).unwrap(), 8);
        let opposing = DamageContext {
            player: 1,
            bonus_damage: 0,
            dealer: DamageDealer::Ability,
        };
        assert_eq!(game.damage_amount(opposing, target(103), 3).unwrap(), 3);
        game.players[0].units.pop();
        assert_eq!(game.damage_amount(context, target(102), 3).unwrap(), 8);
        // Captured Tome bonuses survive their original gear; live Annie does not.
        game.players[0].units.clear();
        assert_eq!(game.damage_amount(context, target(102), 3).unwrap(), 7);
        let opposing = DamageContext {
            player: 1,
            bonus_damage: 0,
            dealer: DamageDealer::Ability,
        };
        assert_eq!(game.damage_amount(opposing, target(103), 3).unwrap(), 3);
    }

    #[test]
    fn unit_damage_and_zero_deal_cannot_gain_spell_or_ability_bonus() {
        let mut game = position();
        let context = DamageContext {
            player: 0,
            bonus_damage: 9,
            dealer: DamageDealer::Unit,
        };
        assert_eq!(game.damage_amount(context, target(102), 3).unwrap(), 3);
        game.deal_damage(context, target(102), 3).unwrap();
        assert_eq!(game.players[1].units[0].damage, 3);
        let spell = DamageContext {
            dealer: DamageDealer::Spell { card_id: 260 },
            ..context
        };
        game.players[0].discard = vec![260, 260];
        assert_eq!(game.damage_amount(spell, target(102), 0).unwrap(), 0);
    }

    #[test]
    fn consuming_curse_counts_only_current_own_trash_copies_per_packet() {
        let mut game = position();
        game.players[0].units.clear();
        game.players[0].discard = vec![260, 25];
        game.players[1].discard = vec![260, 260];
        let context = DamageContext {
            player: 0,
            bonus_damage: 0,
            dealer: DamageDealer::Spell { card_id: 260 },
        };
        assert_eq!(game.damage_amount(context, target(103), 2).unwrap(), 3);
        game.players[0].discard.push(260);
        assert_eq!(game.damage_amount(context, target(103), 2).unwrap(), 4);
        game.players[0].discard.retain(|&card| card != 260);
        assert_eq!(game.damage_amount(context, target(103), 2).unwrap(), 2);
        let ability = DamageContext {
            dealer: DamageDealer::Ability,
            ..context
        };
        game.players[0].discard.push(260);
        assert_eq!(game.damage_amount(ability, target(103), 2).unwrap(), 2);
    }

    #[test]
    fn complete_packets_cross_old_bounds_and_overflow_without_partial_marking() {
        let mut game = position();
        game.players[0].units.clear();
        let context = DamageContext {
            player: 0,
            bonus_damage: 65536,
            dealer: DamageDealer::Spell { card_id: 53 },
        };
        assert_eq!(game.damage_amount(context, target(103), 3).unwrap(), 65539);
        game.deal_damage(context, target(103), 3).unwrap();
        assert_eq!(game.players[1].units[1].damage, 65539);

        let oversized = DamageContext {
            bonus_damage: u64::MAX,
            ..context
        };
        let before = format!("{game:?}");
        assert!(
            game.deal_damage(oversized, target(103), 1)
                .unwrap_err()
                .starts_with("engine_capacity:")
        );
        assert_eq!(format!("{game:?}"), before);

        // Marked damage can remain above Might inside an unresolved parent.
        game.players[1].units[1].damage = u64::MAX - 1;
        game.mark_damage(target(103), 1).unwrap();
        let before = format!("{game:?}");
        assert!(
            game.mark_damage(target(103), 1)
                .unwrap_err()
                .starts_with("engine_capacity:")
        );
        assert_eq!(format!("{game:?}"), before);
    }

    #[test]
    fn absent_recipient_does_not_create_an_overflowing_damage_packet() {
        let mut game = position();
        let context = DamageContext {
            player: 0,
            bonus_damage: u64::MAX,
            dealer: DamageDealer::Spell { card_id: 53 },
        };
        let before = format!("{game:?}");
        game.deal_damage(context, target(999), 1).unwrap();
        assert_eq!(format!("{game:?}"), before);
    }
}
