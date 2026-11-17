//! Shared descriptions of decisions answered with `Action::Choose`.
//!
//! These are the two answers to a pending binary question, not an enumeration
//! of every action: resource abilities may also be available during payment.
use crate::game::{Action, Decision, Game};
use crate::legends::LegendTrigger;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryChoiceOption {
    pub action: Action,
    pub label: &'static str,
    pub unavailable_reason: Option<String>,
}

impl BinaryChoiceOption {
    pub fn enabled(&self) -> bool {
        self.unavailable_reason.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryChoiceView {
    pub player: usize,
    pub options: [BinaryChoiceOption; 2],
}

impl Game {
    /// Describe both answers without changing state, payment, priority or RNG.
    /// Availability comes from executing the actual command on a disposable
    /// copy, so the description cannot bypass a cost or invent another target.
    /// Other decision shapes return None; they require their own typed choices.
    pub fn binary_choices(&self) -> Result<Option<BinaryChoiceView>, String> {
        self.validate_numeric_state()?;
        let Some(decision) = &self.decision else {
            return Ok(None);
        };
        let (yes, no) = match decision {
            Decision::SharedResources { .. } => ("Cards · each draw 1", "Runes · each channel 1"),
            Decision::RevealHand { .. } => ("Continue", "No · decline"),
            Decision::TriggerReturn { .. } => ("Return to hand", "Leave on board"),
            Decision::TriggerObjectCost { .. } => ("Banish Prefect · accept", "No · decline"),
            Decision::BanishPlay { .. } => ("Play this card", "Leave in Banish"),
            Decision::OptionalPlay { .. } => ("Use this ability", "No · decline"),
            Decision::Predict { .. } => ("Recycle to bottom", "Keep on top"),
            Decision::Legend { trigger, .. } => match trigger {
                LegendTrigger::IreliaChoose { .. } => ("Pay 1 Power", "No · decline"),
                LegendTrigger::IreliaConquer => ("Pay 1 Energy", "No · decline"),
            },
            Decision::Battlefield { .. }
            | Decision::CounterTax { .. }
            | Decision::BurnForMight { .. } => ("Yes · pay the cost", "No · decline"),
            _ => return Ok(None),
        };
        let option = |accept, label| {
            let action = Action::Choose { accept };
            let unavailable_reason = self.probe_action(&action).err();
            BinaryChoiceOption {
                action,
                label,
                unavailable_reason,
            }
        };
        Ok(Some(BinaryChoiceView {
            player: decision.player(),
            options: [option(true, yes), option(false, no)],
        }))
    }
}

#[cfg(test)]
mod tests;
