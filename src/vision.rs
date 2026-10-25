//! Vision is one entry-triggered Predict per instance, including live grants.
//! The bearer is the ability source; grant provenance is explanatory metadata.
use super::*;
use crate::continuous::{PassiveEffect, PassiveKeyword, PassiveMatch};
use crate::empower::BoardObject;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisionOrigin {
    Printed,
    Granted {
        source: BoardObject,
        card_id: CardId,
        rule_index: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisionInstance {
    pub source: BoardObject,
    pub origin: VisionOrigin,
    /// Printed gear rules are inactive while attached, but remain characteristics.
    pub active: bool,
}

impl VisionInstance {
    fn from_grant(source: BoardObject, grant: PassiveMatch) -> Option<Self> {
        (grant.active && grant.effect == PassiveEffect::Keyword(PassiveKeyword::Vision)).then_some(
            Self {
                source,
                origin: VisionOrigin::Granted {
                    source: grant.source,
                    card_id: grant.card_id,
                    rule_index: grant.rule_index,
                },
                active: true,
            },
        )
    }
}

impl Game {
    /// Current, source-attributed instances. Missing/non-permanent objects return
    /// an empty list. No numeric expression or hidden information is evaluated.
    pub fn vision_instances(&self, source: BoardObject) -> Result<Vec<VisionInstance>, String> {
        Ok(self.vision_snapshot(source))
    }

    fn vision_snapshot(&self, source: BoardObject) -> Vec<VisionInstance> {
        let (card_id, token, printed_active) = match source {
            BoardObject::Unit { player, unit_id } => {
                let Some(unit) = self
                    .players
                    .get(player)
                    .and_then(|p| p.units.iter().find(|unit| unit.id == unit_id))
                else {
                    return vec![];
                };
                (unit.card_id, unit.token, true)
            }
            BoardObject::Gear { player, gear_id } => {
                let Some(gear) = self
                    .players
                    .get(player)
                    .and_then(|p| p.gear.iter().find(|gear| gear.id == gear_id))
                else {
                    return vec![];
                };
                (gear.card_id, gear.token, gear.attached_to.is_none())
            }
            BoardObject::Legend { .. } => return vec![],
        };
        let mut instances = Vec::new();
        if !token {
            for ability in rules::static_abilities(card_id) {
                if *ability == StaticAbility::Vision {
                    instances.push(VisionInstance {
                        source,
                        origin: VisionOrigin::Printed,
                        active: printed_active,
                    });
                }
            }
        }
        instances.extend(
            self.passive_matches(source)
                .into_iter()
                .filter_map(|grant| VisionInstance::from_grant(source, grant)),
        );
        instances
    }

    pub(super) fn queue_entry_vision(&mut self, source: BoardObject) {
        // Capture after entry and before any response. Each captured instance
        // stays valid if the bearer or granting source subsequently leaves.
        let instances = self.vision_snapshot(source);
        if instances.is_empty() {
            return;
        }
        let (player, card_id, source_name) = match source {
            BoardObject::Unit { player, unit_id } => {
                let Some(unit) = self
                    .players
                    .get(player)
                    .and_then(|p| p.units.iter().find(|unit| unit.id == unit_id))
                else {
                    return;
                };
                (player, unit.card_id, self.unit_name(unit))
            }
            BoardObject::Gear { player, gear_id } => {
                let Some(gear) = self
                    .players
                    .get(player)
                    .and_then(|p| p.gear.iter().find(|gear| gear.id == gear_id))
                else {
                    return;
                };
                (player, gear.card_id, self.gear_name(gear).to_owned())
            }
            BoardObject::Legend { .. } => return,
        };
        for instance in instances.into_iter().filter(|instance| instance.active) {
            // Deliberately retain the caller's batch: all instances from this
            // one Play share ordering with its other simultaneous observers.
            self.queue_ability(
                player,
                card_id,
                vec![],
                ChainAbility::Vision {
                    source,
                    source_name: source_name.clone(),
                    origin: instance.origin,
                },
            );
        }
    }
}

#[cfg(test)]
#[path = "vision/tests.rs"]
mod tests;
