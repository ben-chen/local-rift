//! Public play-target metadata and battlefield target protocol regressions.
use super::*;
use crate::play::{HiddenTargetRestriction, PlayTargetView};
use crate::rules::{PlayTargetKind, play_target_spec};

impl Session {
    /// Preview an ordered trigger target prefix using the same slot, location,
    /// protection and Deflect checks as the native client. No choice is committed.
    pub fn trigger_target_candidates(
        &self,
        player: usize,
        revision: u64,
        targets: Vec<Target>,
    ) -> Result<Value, ApiError> {
        self.require_revision(revision)?;
        self.require_actor(player)?;
        self.game
            .validate_numeric_state()
            .map_err(ApiError::illegal)?;
        let (maximum, required) = self
            .game
            .trigger_target_bounds()
            .ok_or_else(|| ApiError::illegal("No triggered ability is choosing targets."))?;
        if targets.len() > maximum {
            return Err(ApiError::illegal("Too many targets for this ability."));
        }
        for (index, &target) in targets.iter().enumerate() {
            if !self
                .game
                .valid_trigger_target(target, &targets[..index])
                .map_err(ApiError::illegal)?
            {
                return Err(ApiError::illegal(
                    "A target does not meet this ability's ordered requirements.",
                ));
            }
        }
        let mut candidates = Vec::new();
        if targets.len() < maximum {
            for target in self
                .game
                .trigger_target_candidates()
                .map_err(ApiError::illegal)?
            {
                if self
                    .game
                    .valid_trigger_target(target, &targets)
                    .map_err(ApiError::illegal)?
                {
                    candidates.push(target_to_json(&target));
                }
            }
        }
        let kind = self
            .game
            .trigger_target_rule()
            .map_or(PlayTargetKind::Unit, |rule| {
                if rule.battlefield_first && targets.is_empty() {
                    PlayTargetKind::Battlefield
                } else {
                    rule.kind
                }
            });
        let can_confirm = self
            .game
            .clone()
            .apply(Action::ChooseTriggerTargets {
                targets: targets.clone(),
            })
            .is_ok();
        Ok(
            json!({"targets":targets.iter().map(target_to_json).collect::<Vec<_>>(),
            "maximum":maximum,"required":required,"can_confirm":can_confirm,
            "next_slot":(targets.len()<maximum).then(||json!({"index":targets.len(),"kind":target_kind(kind)})),
            "candidates":candidates}),
        )
    }

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
        PlayTargetKind::Rune => "rune",
        PlayTargetKind::Unit => "unit",
        PlayTargetKind::Gear => "gear",
        PlayTargetKind::Spell => "spell",
        PlayTargetKind::Battlefield => "battlefield",
        PlayTargetKind::UnitOrGear => "unit_or_gear",
    }
}

pub(super) fn play_targets_to_json(card_id: crate::cards::CardId) -> Value {
    let spec = play_target_spec(card_id);
    let mut value =
        json!({"kind":spec.kind.map(target_kind),"maximum":spec.maximum,"required":spec.required});
    if crate::rules::targeting(card_id).moved_this_turn {
        value["condition"] =
            json!({"type":"moved_this_turn","controller":"enemy","rechecked_on_resolution":true});
    }
    if card_id == 426 {
        value["conditional_slots"] = json!({"after":"show_off","instance":0,"kind":"rune",
            "maximum":2,"required":0,"controller":"any","after_required_unit":true});
    }
    if card_id == 442 {
        value["ordered_slots"] = json!([
            {"index":0,"kind":"unit","controller":"friendly","location":"battlefield"},
            {"index":1,"kind":"spell","controller":"enemy","includes_abilities":true,
             "chooses_slot":0,"other_friendly_units":0,"counts_live_physical_relationships":true,
             "includes_repeat_choices":true,"rechecked_on_resolution":true}
        ]);
    }
    if card_id == 443 {
        value["maximum"] = Value::Null;
        value["maximum_rule"] =
            json!("one source plus at most its current Might in enemy battlefield units");
        value["ordered_slots"] = json!([
            {"index":0,"kind":"unit","controller":"friendly"},
            {"from_index":1,"kind":"unit","controller":"enemy","location":"battlefield","distinct":true}
        ]);
        value["damage_assignment"] = json!({"timing":"resolution","positive_per_retained_target":true,
            "excess_targets":"drop_exact_excess_if_damage_decreases","dealer":"chosen_unit"});
    }
    if let Some(domain) = crate::rules::targeting(card_id).threatened_by_domain {
        value["condition"] = json!({
            "type":"in_combat_with_unit_or_chosen_by_spell",
            "controller":"enemy",
            "domain":domain.name(),
            "rechecked_on_resolution":true
        });
    }
    if let crate::rules::OfficialEffect::Counter(crate::rules::CounterKind::MinimumPower(minimum)) =
        crate::rules::effect(card_id)
    {
        value["condition"] = json!({"type":"spell_printed_power_at_least","minimum":minimum});
    }
    value
}

pub(super) fn view_to_json(view: &PlayTargetView) -> Value {
    let mut value = json!({"card_id":view.card_id,"origin":play_choices::origin_to_json(view.origin),
        "execution":view.execution,"targets":view.targets.iter().map(target_to_json).collect::<Vec<_>>(),
        "spec":{"kind":view.spec.kind.map(target_kind),"maximum":view.spec.maximum,"required":view.spec.required},
        "next_slot":view.next_slot.as_ref().map(|slot|json!({
            "index":slot.index,"kind":target_kind(slot.kind),"different_location_from":slot.different_location_from,
            "hidden":slot.hidden.map(|restriction|match restriction {
                HiddenTargetRestriction::AtBattlefield{lane}=>json!({"type":"at_battlefield","lane":lane}),
                HiddenTargetRestriction::OutsideBattlefield{lane}=>json!({"type":"outside_battlefield","lane":lane}),
            })})),
        "candidates":view.candidates.iter().map(target_to_json).collect::<Vec<_>>()});
    if let Some(destination) = view.destination {
        value["destination"] = play_choices::destination_to_json(destination);
    }
    value
}

#[cfg(test)]
mod repulse_tests;
#[cfg(test)]
mod tests;
