//! Legends are in their own public zone, without a board location (CR107.4).
//! Targets finalize before responses; a non-bulleted either/or choice waits
//! until resolution (CR355.3, 355.5 and 355.18).
use super::*;
use crate::trigger_rules::{self, Effect};

#[cfg(test)]
#[path = "legend_readiness_tests.rs"]
mod tests;

impl Game {
    /// Champion legends cannot leave their zone. Pin both seat and identity;
    /// future non-champion legends will require their own physical instances.
    pub fn legend_target_ready(&self, target: Target) -> Option<bool> {
        let Target::Legend { player, legend_id } = target else {
            return None;
        };
        self.legend
            .get(player)
            .filter(|l| l.kind.id() == legend_id)
            .map(|l| l.ready)
    }

    pub(crate) fn legend_readiness_score(&self, actor: usize, target: Target) -> i128 {
        let Target::Legend { player, .. } = target else {
            return 0;
        };
        self.legend_target_ready(target).map_or(-100, |ready| {
            if player == actor && !ready {
                50
            } else if player != actor && ready {
                40
            } else {
                0
            }
        })
    }

    fn readiness_instruction(item: &ChainItem) -> Option<(usize, TriggerContext)> {
        let Some(ChainAbility::CardTrigger {
            rule_index,
            context,
        }) = item.ability
        else {
            return None;
        };
        let rule = trigger_rules::selected_rule(item.card_id, rule_index, context.mode)?;
        // Do not silently discard a future sequence's remaining instructions.
        (rule.effects == [Effect::ChooseLegendReadiness]).then_some((rule_index, context))
    }

    fn readiness_target(&self, item: &ChainItem) -> Result<Option<Target>, String> {
        let Some((rule_index, context)) = Self::readiness_instruction(item) else {
            return Ok(None);
        };
        let [target] = item.targets.as_slice() else {
            return Ok(None);
        };
        Ok(self
            .trigger_rule_target_valid(
                item.card_id,
                rule_index,
                context,
                *target,
                0,
                &item.targets,
            )?
            .then_some(*target))
    }

    pub(super) fn begin_legend_readiness(&mut self) -> Result<bool, String> {
        let item = self.chain.last().expect("resolving trigger");
        if Self::readiness_instruction(item).is_none() {
            return Ok(false);
        }
        if let Some(target) = self.readiness_target(item)? {
            self.decision = Some(Decision::LegendReadiness {
                player: item.player,
                choice_id: item.id,
                card_id: item.card_id,
                target,
            });
        } else {
            self.chain.pop();
            self.after_resolution()?;
        }
        Ok(true)
    }

    pub(super) fn resolve_legend_readiness(
        &mut self,
        choice_id: u32,
        ready: bool,
    ) -> Result<(), String> {
        let Some(Decision::LegendReadiness {
            player,
            choice_id: current,
            card_id,
            target,
        }) = self.decision
        else {
            return Err("No resolving ability is choosing ready or exhaust.".into());
        };
        if choice_id != current {
            return Err("This readiness choice is stale.".into());
        }
        let item = self
            .chain
            .last()
            .ok_or("The resolving ability is missing.")?;
        if item.id != current
            || item.player != player
            || item.card_id != card_id
            || Self::readiness_instruction(item).is_none()
            || item.targets.as_slice() != [target]
        {
            return Err("The resolving ability no longer matches this choice.".into());
        }
        if self.readiness_target(item)? == Some(target)
            && let Target::Legend {
                player: owner,
                legend_id,
            } = target
        {
            let changed = self.legend[owner].ready != ready;
            self.legend[owner].ready = ready;
            let name = crate::legend_catalog::legend(legend_id)
                .expect("validated legend")
                .name;
            self.push_log(format!(
                "{}'s {name} {}{}.",
                self.players[owner].name,
                if changed { "is now " } else { "remains " },
                if ready { "ready" } else { "exhausted" }
            ));
        }
        self.decision = None;
        self.chain.pop();
        self.after_resolution()
    }
}
