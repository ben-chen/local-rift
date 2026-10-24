//! Public play-target metadata and battlefield target protocol regressions.
use super::*;
use crate::rules::{PlayTargetKind, play_target_spec};

pub(super) fn play_targets_to_json(card_id: crate::cards::CardId) -> Value {
    let spec = play_target_spec(card_id);
    json!({"kind":spec.kind.map(|kind|match kind {
        PlayTargetKind::Unit=>"unit", PlayTargetKind::Gear=>"gear",
        PlayTargetKind::Spell=>"spell", PlayTargetKind::Battlefield=>"battlefield",
    }),"maximum":spec.maximum,"required":spec.required})
}

#[cfg(test)]
mod tests;
