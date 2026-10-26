//! Public play-target metadata and battlefield target protocol regressions.
use super::*;
use crate::play::{HiddenTargetRestriction, PlayTargetView};
use crate::rules::{PlayTargetKind, play_target_spec};

impl Session {
    /// Preview the next target of an incomplete play without spending costs or
    /// changing its original/Repeat execution's ordered target prefix.
    pub fn target_candidates(
        &self,
        player: usize,
        revision: u64,
        action: Action,
        execution: usize,
    ) -> Result<PlayTargetView, ApiError> {
        self.require_revision(revision)?;
        self.require_actor(player)?;
        let action = parse_action(&action_to_json(&action))?;
        match catch_unwind(AssertUnwindSafe(|| {
            self.game.play_target_view(&action, execution)
        })) {
            Ok(result) => result.map_err(ApiError::illegal),
            Err(_) => Err(ApiError::new(
                "engine_error",
                "Could not preview these play targets.",
            )),
        }
    }
}

pub(super) fn target_kind(kind: PlayTargetKind) -> &'static str {
    match kind {
        PlayTargetKind::Unit => "unit",
        PlayTargetKind::Gear => "gear",
        PlayTargetKind::Spell => "spell",
        PlayTargetKind::Battlefield => "battlefield",
        PlayTargetKind::UnitOrGear => "unit_or_gear",
    }
}

pub(super) fn play_targets_to_json(card_id: crate::cards::CardId) -> Value {
    let spec = play_target_spec(card_id);
    json!({"kind":spec.kind.map(target_kind),"maximum":spec.maximum,"required":spec.required})
}

pub(super) fn view_to_json(view: &PlayTargetView) -> Value {
    json!({"card_id":view.card_id,"origin":play_choices::origin_to_json(view.origin),
        "execution":view.execution,"targets":view.targets.iter().map(target_to_json).collect::<Vec<_>>(),
        "spec":{"kind":view.spec.kind.map(target_kind),"maximum":view.spec.maximum,"required":view.spec.required},
        "next_slot":view.next_slot.as_ref().map(|slot|json!({
            "index":slot.index,"kind":target_kind(slot.kind),"different_location_from":slot.different_location_from,
            "hidden":slot.hidden.map(|restriction|match restriction {
                HiddenTargetRestriction::AtBattlefield{lane}=>json!({"type":"at_battlefield","lane":lane}),
                HiddenTargetRestriction::OutsideBattlefield{lane}=>json!({"type":"outside_battlefield","lane":lane}),
            })})),
        "candidates":view.candidates.iter().map(target_to_json).collect::<Vec<_>>()})
}

#[cfg(test)]
mod tests;
