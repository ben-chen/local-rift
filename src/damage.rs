//! Deal-action provenance. Unit combat and Fight never inherit spell bonuses.
use super::{CardId, Game, Target};

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
    pub bonus_damage: u16,
    pub dealer: DamageDealer,
}

impl Game {
    /// CR714–715: add all applicable bonuses to each actual Deal packet.
    /// A zero/no Deal does not become damage merely because bonuses exist.
    pub(crate) fn damage_amount(&self, context: DamageContext, target: Target, amount: u8) -> u8 {
        if amount == 0 || context.dealer == DamageDealer::Unit {
            return amount;
        }
        let mut total =
            u32::from(self.battlefield_damage(target, amount)) + u32::from(context.bonus_damage);
        total = total.saturating_add(
            self.players[context.player]
                .units
                .iter()
                .filter(|unit| !unit.token && unit.card_id == 259)
                .count()
                .min(u32::MAX as usize) as u32,
        );
        if let DamageDealer::Spell { card_id: 260 } = context.dealer {
            // The physical spell remains on the chain, so it cannot count
            // itself. Flow/replay already removed it from trash before Play.
            let name = self.catalog[260].name;
            let copies = self.players[context.player]
                .discard
                .iter()
                .filter(|&&card| self.catalog[card].name == name)
                .count();
            total = total.saturating_add(copies.min(u32::MAX as usize) as u32);
        }
        total.min(u32::from(u8::MAX)) as u8
    }

    pub(crate) fn deal_damage(&mut self, context: DamageContext, target: Target, amount: u8) {
        let amount = self.damage_amount(context, target, amount);
        if let Some(unit) = self.unit_mut(target) {
            unit.damage = unit.damage.saturating_add(amount);
        }
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
        assert_eq!(game.damage_amount(context, target(102), 3), 9);
        assert_eq!(game.damage_amount(context, target(103), 3), 8);
        let opposing = DamageContext {
            player: 1,
            bonus_damage: 0,
            dealer: DamageDealer::Ability,
        };
        assert_eq!(game.damage_amount(opposing, target(103), 3), 3);
        game.players[0].units.pop();
        assert_eq!(game.damage_amount(context, target(102), 3), 8);
        // Captured Tome bonuses survive their original gear; live Annie does not.
        game.players[0].units.clear();
        assert_eq!(game.damage_amount(context, target(102), 3), 7);
        let opposing = DamageContext {
            player: 1,
            bonus_damage: 0,
            dealer: DamageDealer::Ability,
        };
        assert_eq!(game.damage_amount(opposing, target(103), 3), 3);
    }

    #[test]
    fn unit_damage_and_zero_deal_cannot_gain_spell_or_ability_bonus() {
        let mut game = position();
        let context = DamageContext {
            player: 0,
            bonus_damage: 9,
            dealer: DamageDealer::Unit,
        };
        assert_eq!(game.damage_amount(context, target(102), 3), 3);
        game.deal_damage(context, target(102), 3);
        assert_eq!(game.players[1].units[0].damage, 3);
        let spell = DamageContext {
            dealer: DamageDealer::Spell { card_id: 260 },
            ..context
        };
        game.players[0].discard = vec![260, 260];
        assert_eq!(game.damage_amount(spell, target(102), 0), 0);
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
        assert_eq!(game.damage_amount(context, target(103), 2), 3);
        game.players[0].discard.push(260);
        assert_eq!(game.damage_amount(context, target(103), 2), 4);
        game.players[0].discard.retain(|&card| card != 260);
        assert_eq!(game.damage_amount(context, target(103), 2), 2);
        let ability = DamageContext {
            dealer: DamageDealer::Ability,
            ..context
        };
        game.players[0].discard.push(260);
        assert_eq!(game.damage_amount(ability, target(103), 2), 2);
    }
}
