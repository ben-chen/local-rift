//! Public battlefield-gear facts and trigger-finalization regression coverage.
use super::*;

pub(super) fn status_to_json(status: crate::gear::GearDeployStatus) -> Value {
    json!({
        "characteristic": status.characteristic,
        "printed_abilities_active": status.printed_abilities_active,
        "opponent_hold_trigger_active": status.opponent_hold_trigger_active,
    })
}

#[cfg(test)]
mod tests;
