//! Public battlefield choice and delayed-trigger identities.
use super::*;
use crate::game::field_triggers::{
    BattlefieldSource, BattlefieldTargetKind, DelayedBattlefieldTrigger,
};

pub(super) fn add_scoring_view(game: &Game, view: &mut Value) {
    let restrictions: Vec<_> = (0..game.battlefield_count())
        .filter(|&lane| game.battlefield_score_unlock(0, lane).is_some())
        .map(|lane| {
            json!({
                "lane":lane, "battlefield_id":game.battlefields[lane],
                "players":(0..2).map(|player| json!({
                    "player":player, "turns_started":game.player_turns_started(player),
                    "turns_until_unlocked":game.battlefield_score_unlock(player, lane).unwrap(),
                    "scoring_unlocked":game.battlefield_score_unlock(player, lane)==Some(0),
                })).collect::<Vec<_>>()
            })
        })
        .collect();
    if !restrictions.is_empty() {
        view["battlefield_scoring"] = json!(restrictions);
    }
}

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
mod admissions;
#[cfg(test)]
mod tests;
