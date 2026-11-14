//! Live prohibitions, evaluated from the current controller and board location.
use crate::{
    cards::CardId,
    game::{Game, Target},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestrictionKind {
    UnitPlayBaseOnly,
    UnitGearReadyByEffect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RestrictionSource {
    pub kind: RestrictionKind,
    pub controller: usize,
    pub unit_id: u32,
    pub card_id: CardId,
    pub lane: usize,
}

fn battlefield_restrictions(card_id: CardId) -> &'static [RestrictionKind] {
    match card_id {
        464 => &[
            RestrictionKind::UnitPlayBaseOnly,
            RestrictionKind::UnitGearReadyByEffect,
        ],
        _ => &[],
    }
}

impl Game {
    /// Public board facts only. A contested battlefield still activates Warden.
    pub fn restrictions_affecting(&self, player: usize) -> Vec<RestrictionSource> {
        if player >= self.players.len() {
            return Vec::new();
        }
        self.players
            .iter()
            .enumerate()
            .filter(|(owner, _)| *owner != player)
            .flat_map(|(controller, state)| {
                state.units.iter().filter_map(move |unit| {
                    (!unit.token)
                        .then_some(unit.lane)
                        .flatten()
                        .map(|lane| (controller, unit, lane))
                })
            })
            .flat_map(|(controller, unit, lane)| {
                battlefield_restrictions(unit.card_id)
                    .iter()
                    .map(move |&kind| RestrictionSource {
                        kind,
                        controller,
                        unit_id: unit.id,
                        card_id: unit.card_id,
                        lane,
                    })
            })
            .collect()
    }

    pub fn ready_effect_restrictions(&self, target: Target) -> Vec<RestrictionSource> {
        let player = match target {
            Target::Unit { player, .. } if self.unit_ref(target).is_some() => player,
            Target::Gear { player, gear_id }
                if self
                    .players
                    .get(player)
                    .is_some_and(|state| state.gear.iter().any(|gear| gear.id == gear_id)) =>
            {
                player
            }
            _ => return Vec::new(),
        };
        self.restrictions_affecting(player)
            .into_iter()
            .filter(|source| source.kind == RestrictionKind::UnitGearReadyByEffect)
            .collect()
    }
}
