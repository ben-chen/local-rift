//! Public conditional effects and activation descriptors. These are explanations;
//! observed Might/keywords and the shared cost-plan query remain authoritative.
use super::*;
use crate::activated::{AbilityCostModifier, AbilityTargets};
use crate::continuous::{
    PassiveContribution, PassiveEffect, PassiveKeyword, PassiveRule, Recipients, SourceCondition,
};
use crate::empower::BoardObject;
use crate::legends::{LegendActivation, LegendActivationEffect, LegendTiming};
use crate::rules::Allegiance;

fn board_object_to_json(source: BoardObject) -> Value {
    match source {
        BoardObject::Unit { player, unit_id } => {
            json!({"type":"unit","player":player,"unit_id":unit_id})
        }
        BoardObject::Gear { player, gear_id } => {
            json!({"type":"gear","player":player,"gear_id":gear_id})
        }
        BoardObject::Legend { player } => json!({"type":"legend","player":player}),
    }
}

pub(super) fn contribution_to_json(contribution: &PassiveContribution) -> Value {
    json!({"source":board_object_to_json(contribution.source),"card_id":contribution.card_id,
        "rule_index":contribution.rule_index,"active":contribution.active,
        "effect":effect_to_json(contribution.effect)})
}

fn effect_to_json(effect: PassiveEffect) -> Value {
    match effect {
        PassiveEffect::Might(amount) => json!({"type":"might","amount":amount}),
        PassiveEffect::Keyword(keyword) => match keyword {
            PassiveKeyword::Assault(amount) => {
                json!({"type":"keyword","keyword":"assault","amount":amount})
            }
            PassiveKeyword::Shield(amount) => {
                json!({"type":"keyword","keyword":"shield","amount":amount})
            }
            PassiveKeyword::Deflect(amount) => {
                json!({"type":"keyword","keyword":"deflect","amount":amount})
            }
            PassiveKeyword::Ganking => json!({"type":"keyword","keyword":"ganking"}),
        },
    }
}

pub(super) fn passive_rule_to_json(index: usize, rule: &PassiveRule) -> Value {
    json!({"rule_index":index,
        "condition":match rule.condition {SourceCondition::Always=>"always",SourceCondition::Empowered=>"empowered",SourceCondition::NotEmpowered=>"not_empowered",
            SourceCondition::AttackingWithAnother=>"attacking_with_another",SourceCondition::CombatAlone=>"combat_alone",
            SourceCondition::Buffed=>"buffed",SourceCondition::OtherFriendlyHere=>"other_friendly_here",
            SourceCondition::ControllerRunesAtLeast(_)=>"controller_runes_at_least",SourceCondition::ControllerXpAtLeast(_)=>"controller_xp_at_least"},
        "condition_threshold":match rule.condition {SourceCondition::ControllerRunesAtLeast(n)|SourceCondition::ControllerXpAtLeast(n)=>Some(n),_=>None},
        "recipients":match rule.recipients {Recipients::SelfUnit=>"self_unit",Recipients::FriendlyUnits=>"friendly_units",Recipients::FriendlyEmpoweredUnits=>"friendly_empowered_units",
            Recipients::OtherFriendlyUnitsHere=>"other_friendly_units_here",Recipients::FriendlyTokenUnits=>"friendly_token_units",Recipients::AttachedBearer=>"attached_bearer"},
        "effect":effect_to_json(rule.effect)})
}

pub(super) fn cost_modifier_to_json(modifier: &AbilityCostModifier) -> Value {
    match modifier {
        AbilityCostModifier::EnergyPerControlledRune { amount } => {
            json!({"type":"energy_per_controlled_rune","amount":amount})
        }
        AbilityCostModifier::EnergyIfRunesAtMost { maximum, amount } => {
            json!({"type":"energy_if_runes_at_most","maximum":maximum,"amount":amount})
        }
    }
}

fn targets_to_json(targets: AbilityTargets) -> Value {
    json!({"min":targets.min,"max":targets.max,
        "kind":match targets.kind {AbilityTargetKind::Unit=>"unit",AbilityTargetKind::Gear=>"gear"},
        "another":targets.another,"battlefield":targets.battlefield,"exhausted":targets.exhausted,
        "allegiance":match targets.allegiance {Allegiance::None=>"none",Allegiance::Any=>"any",Allegiance::Friendly=>"friendly",Allegiance::Enemy=>"enemy",Allegiance::Mixed=>"mixed"}})
}

pub(super) fn legend_activation_to_json(activation: LegendActivation) -> Value {
    json!({"timing":match activation.timing {LegendTiming::Main=>"main",LegendTiming::Action=>"action",LegendTiming::Reaction=>"reaction"},
        "energy":activation.energy,"power":activation.power,"power_domain":activation.domain.map(|domain|domain.name()),
        "exhaust":activation.exhaust,"disempower":activation.disempower,"targets":targets_to_json(activation.targets),
        "effect":match activation.effect {LegendActivationEffect::SpellPower=>"spell_power",LegendActivationEffect::Assault=>"assault",
            LegendActivationEffect::LegionEnergy=>"legion_energy",LegendActivationEffect::Recruit=>"recruit",LegendActivationEffect::Ready=>"ready"}})
}

#[cfg(test)]
mod tests;
