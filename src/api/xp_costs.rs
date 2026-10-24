//! XP costs and equipment are explicit metadata, never client-selected payments.
use super::*;

pub(super) fn equipment_to_json(card_id: usize) -> Value {
    crate::rules::equipment(card_id).map_or(Value::Null, |rule| {
        json!({"might":rule.might,"equip_energy":rule.equip_energy,
            "equip_power":rule.equip_power,
            "equip_domain":(rule.equip_power > 0).then(||rule.equip_domain.name()),
            "equip_xp":rule.equip_xp,"equip_xp_exact":rule.equip_xp.to_string(),
            "recycle_trash":rule.recycle_trash,"quick_draw":rule.quick_draw})
    })
}

#[cfg(test)]
mod tests;
