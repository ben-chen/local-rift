//! Public Level and entry descriptors. Current costs and unit state remain
//! authoritative; these fields explain the printed conditional behavior.
use super::*;
use crate::rules::{EntryReadyRule, entry_ready_rule, xp_energy_discount_tiers};

pub(super) fn entry_ready_to_json(card_id: usize) -> Value {
    match entry_ready_rule(card_id) {
        None => Value::Null,
        Some(EntryReadyRule::Always) => json!({"condition":"always"}),
        Some(EntryReadyRule::ControllerXpAtLeast(threshold)) => {
            json!({"condition":"controller_xp_at_least","threshold":threshold})
        }
    }
}

pub(super) fn xp_discounts_to_json(card_id: usize) -> Value {
    json!(
        xp_energy_discount_tiers(card_id)
            .iter()
            .map(|tier| json!({"minimum_xp":tier.minimum_xp,"energy":tier.energy}))
            .collect::<Vec<_>>()
    )
}

#[cfg(test)]
mod tests;
