//! XP costs and equipment are explicit metadata, never client-selected payments.
use super::*;

pub(super) fn equipment_to_json(card_id: usize) -> Value {
    crate::rules::equipment(card_id).map_or(Value::Null, |rule| {
        let mut value = json!({"might":rule.might,"equip_energy":rule.equip_energy,
            "equip_power":rule.equip_power,
            "equip_domain":(rule.equip_power > 0).then(||rule.equip_domain.name()),
            "equip_xp":rule.equip_xp,"equip_xp_exact":rule.equip_xp.to_string(),
            "recycle_trash":rule.recycle_trash,"quick_draw":rule.quick_draw});
        if rule.equip_any_power {
            value["equip_domain"] = json!("Any");
        }
        if rule.equip_exhaust_unit {
            value["alternate_cost"] = json!({"type":"exhaust_friendly_unit","ready_required":true,
                "may_be_bearer":true,"targets_unit":false,"action":"equip_exhaust_unit"});
        }
        value
    })
}

#[cfg(test)]
mod tests;
