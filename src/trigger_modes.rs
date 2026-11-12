//! Modal triggers choose before responses; later instruction costs resolve afterward; the selected mode survives source departure.
use super::*;
use crate::trigger_rules;

impl Game {
    /// The mode's later instruction cost is deliberately not checked here.
    /// Targets and unused-mode restrictions still apply during finalization.
    pub fn trigger_mode_available(&self, mode: usize) -> Result<bool, String> {
        let Some(Decision::TriggerMode {
            player,
            card_id,
            rule_index,
            mut context,
            ..
        }) = self.decision
        else {
            return Ok(false);
        };
        let parent = &trigger_rules::trigger_rules(card_id)[rule_index];
        let Some(chosen) = parent.modes.get(mode) else {
            return Ok(false);
        };
        let source_id = context.source.unit_id.or(context.source.gear_id);
        if parent.modes_once_each_turn
            && source_id.is_some_and(|id| {
                self.trigger_state
                    .modes_used
                    .contains(&(id, card_id, rule_index, mode))
            })
        {
            return Ok(false);
        }
        if chosen.rule.targets.min == 0 {
            return Ok(true);
        }
        context.mode = Some(mode);
        let mut probe = self.clone();
        probe.decision = Some(Decision::TriggerTargets {
            player,
            card_id,
            rule_index,
            context,
        });
        let mut capacity = None;
        for action in probe.trigger_target_actions_mode(false)? {
            match probe.funded_action_witness(&action) {
                Ok(Some(_)) => return Ok(true),
                Ok(None) => {}
                Err(error) if crate::numeric::is_capacity(&error) => capacity = Some(error),
                Err(error) => return Err(error),
            }
        }
        if let Some(error) = capacity {
            return Err(error);
        }
        Ok(false)
    }

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
        if !self.trigger_mode_available(mode)? {
            return Err("Choose an unused mode with legal targets. Its later Buff cost is checked on resolution.".into());
        }
        let chosen = self
            .trigger_mode_options()
            .get(mode)
            .ok_or("Choose one of this ability's modes.")?;
        let rule = &chosen.rule;
        if trigger_rules::trigger_rules(card_id)[rule_index].modes_once_each_turn
            && let Some(id) = context.source.unit_id.or(context.source.gear_id)
        {
            self.trigger_state
                .modes_used
                .push((id, card_id, rule_index, mode));
        }
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
