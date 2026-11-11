//! Current combat contribution is distinct from Might and marked damage.
use super::*;

pub(super) fn to_json(value: crate::numeric::CombatContribution) -> Value {
    json!({"might":value.might,"might_exact":value.might.to_string(),
        "damage":value.damage,"damage_exact":value.damage.to_string(),
        "stunned":value.stunned,"prevents_combat_damage":value.prevents_combat_damage})
}

#[cfg(test)]
mod tests;
