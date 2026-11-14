//! A later optional instruction retains its finalized trigger while resolving.
//! Core383.3.a.3 separates this choice from optional trigger finalization.
use super::*;
use crate::trigger_rules::Effect;

impl Game {
    pub(super) fn trigger_return_target_score(
        &self,
        actor: usize,
        targets: &[Target],
    ) -> Result<i128, String> {
        let [Target::Battlefield { lane }, target @ Target::Unit { .. }] = targets else {
            return Ok(0);
        };
        let count = self
            .players
            .iter()
            .flat_map(|p| &p.units)
            .filter(|u| u.lane == Some(*lane))
            .count();
        let value = if count >= 4 {
            if let Some(unit) = self.unit_ref(*target) {
                let printed_energy = self
                    .board_target_facts(*target)
                    .map_or(0, |facts| facts.printed_energy);
                30 + i128::from(self.unit_might(unit)?) * 4 + i128::from(printed_energy) * 2
            } else {
                0
            }
        } else {
            0
        };
        Ok(value - i128::from(self.deflect_cost(actor, targets)?) * 5)
    }
    fn trigger_return_threshold(item: &ChainItem) -> Option<usize> {
        let Some(ChainAbility::CardTrigger {
            rule_index,
            context,
        }) = item.ability
        else {
            return None;
        };
        let rule = context.rule(item.card_id, rule_index)?;
        // This descriptor is a complete instruction. A future sequence with
        // further instructions must retain an explicit continuation as well.
        let [Effect::OptionalReturnAtCrowdedBattlefield { minimum_units }] = rule.effects else {
            return None;
        };
        Some(*minimum_units)
    }

    fn trigger_return_target(&self, item: &ChainItem) -> Result<Option<Target>, String> {
        let Some(minimum) = Self::trigger_return_threshold(item) else {
            return Ok(None);
        };
        let Some(ChainAbility::CardTrigger {
            rule_index,
            context,
        }) = item.ability
        else {
            return Ok(None);
        };
        let [Target::Battlefield { lane }, target @ Target::Unit { .. }] = item.targets.as_slice()
        else {
            return Ok(None);
        };
        for (slot, &chosen) in item.targets.iter().enumerate() {
            if !self.trigger_rule_target_valid(
                item.card_id,
                rule_index,
                context,
                chosen,
                slot,
                &item.targets,
            )? {
                return Ok(None);
            }
        }
        let count = self
            .players
            .iter()
            .flat_map(|p| &p.units)
            .filter(|u| u.lane == Some(*lane))
            .count();
        Ok((count >= minimum).then_some(*target))
    }

    pub(super) fn begin_trigger_return(&mut self) -> Result<bool, String> {
        let item = self.chain.last().expect("resolving trigger");
        if Self::trigger_return_threshold(item).is_none() {
            return Ok(false);
        }
        if let Some(target) = self.trigger_return_target(item)? {
            self.decision = Some(Decision::TriggerReturn {
                player: item.player,
                chain_id: item.id,
                card_id: item.card_id,
                target,
            });
        } else {
            self.chain.pop();
            self.after_resolution()?;
        }
        Ok(true)
    }

    pub(super) fn finish_trigger_return(&mut self, accept: bool) -> Result<(), String> {
        let Some(Decision::TriggerReturn {
            player,
            chain_id,
            card_id,
            target,
        }) = self.decision
        else {
            return Err("No resolving ability is offering a return.".into());
        };
        let item = self
            .chain
            .last()
            .ok_or("The resolving ability is missing.")?;
        if item.id != chain_id
            || item.card_id != card_id
            || item.player != player
            || Self::trigger_return_threshold(item).is_none()
            || item.targets.get(1) != Some(&target)
        {
            return Err("The resolving ability no longer matches this choice.".into());
        }
        let eligible = self.trigger_return_target(item)?;
        self.decision = None;
        if accept && eligible == Some(target) {
            self.return_unit_to_hand(target);
        }
        self.chain.pop();
        self.after_resolution()
    }
}
