//! Public battlefield choice and delayed-trigger identities.
use super::*;
use crate::game::field_triggers::{
    BattlefieldSource, BattlefieldTargetKind, DelayedBattlefieldTrigger,
};

pub(super) fn source_to_json(source: &BattlefieldSource) -> Value {
    json!({"type":"battlefield","lane":source.lane,"battlefield_id":source.battlefield_id})
}

pub(super) fn kind_to_json(kind: &BattlefieldTargetKind) -> Value {
    match kind {
        BattlefieldTargetKind::StarSpring { played_unit_id } => {
            json!({"type":"star_spring","played_unit_id":played_unit_id})
        }
        BattlefieldTargetKind::AbandonedHall => json!({"type":"abandoned_hall"}),
        BattlefieldTargetKind::TargonPeakReady => json!({"type":"targon_peak_ready"}),
    }
}

pub(super) fn maximum(kind: &BattlefieldTargetKind) -> usize {
    match kind {
        BattlefieldTargetKind::TargonPeakReady => 2,
        BattlefieldTargetKind::StarSpring { .. } | BattlefieldTargetKind::AbandonedHall => 1,
    }
}

pub(super) fn delayed_to_json(trigger: &DelayedBattlefieldTrigger) -> Value {
    json!({"id":trigger.id,"player":trigger.player,"source":source_to_json(&trigger.source),"turn":trigger.turn})
}

#[cfg(test)]
mod tests;
