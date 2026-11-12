//! Modal triggers choose before costs; the selected mode survives source departure.
use super::*;
use crate::trigger_rules;

impl Game {
    pub fn trigger_mode_options(&self) -> &'static [trigger_rules::TriggerMode] {
        let Some(Decision::TriggerMode {
            card_id,
            rule_index,
            ..
        }) = self.decision
        else {
            return &[];
        };
        trigger_rules::trigger_rules(card_id)[rule_index].modes
    }

    pub(super) fn choose_trigger_mode(
        &mut self,
        choice_id: u32,
        mode: usize,
    ) -> Result<(), String> {
        let Some(Decision::TriggerMode {
            player,
            choice_id: current,
            card_id,
            rule_index,
            mut context,
        }) = self.decision.clone()
        else {
            return Err("No triggered ability is awaiting a mode.".into());
        };
        if current != choice_id {
            return Err("This trigger mode choice is stale.".into());
        }
        let chosen = self
            .trigger_mode_options()
            .get(mode)
            .ok_or("Choose one of this ability's modes.")?;
        let rule = &chosen.rule;
        context.mode = Some(mode);
        self.decision = None;
        if rule.object_cost == Some(trigger_rules::TriggerObjectCost::BanishSource) {
            self.decision = Some(Decision::TriggerObjectCost {
                player,
                card_id,
                rule_index,
                context,
            });
        } else if rule.targets.max > 0 || rule.optional {
            self.decision = Some(Decision::TriggerTargets {
                player,
                card_id,
                rule_index,
                context,
            });
        } else if rule.base_cost.is_some() {
            self.decision =
                Some(self.trigger_payment_decision(card_id, rule_index, context, Vec::new())?);
        } else {
            self.queue_finalized_ability(
                player,
                card_id,
                Vec::new(),
                ChainAbility::CardTrigger {
                    rule_index,
                    context,
                },
            );
        }
        self.discard_unfinalizable_trigger()?;
        Ok(())
    }
}
