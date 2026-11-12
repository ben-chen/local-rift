//! Current combat contribution is distinct from Might and marked damage.
use super::*;
use crate::game::Unit;

pub(super) fn to_json(value: crate::numeric::CombatContribution) -> Value {
    json!({"might":value.might,"might_exact":value.might.to_string(),
        "damage":value.damage,"damage_exact":value.damage.to_string(),
        "stunned":value.stunned,"prevents_combat_damage":value.prevents_combat_damage})
}

pub(super) fn for_unit(game: &Game, unit: &Unit) -> Result<Value, ApiError> {
    let mut value = to_json(
        game.unit_combat_contribution(unit)
            .map_err(ApiError::illegal)?,
    );
    if let Some(met) = game.unit_combat_gear_requirement(unit) {
        value["requirement"] = json!({"type":"friendly_gear_here","met":met});
    }
    Ok(value)
}

#[cfg(test)]
mod tests;
