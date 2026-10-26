//! Live board characteristics shared by targeting, costs and effect resolution.
//! Token factory cards are references, never the token's printed characteristics.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardLocation {
    Base { player: usize },
    Battlefield { lane: usize },
}

#[cfg(test)]
#[path = "board_targets/tests.rs"]
mod tests;
impl BoardLocation {
    pub fn battlefield(self) -> Option<usize> {
        match self {
            Self::Base { .. } => None,
            Self::Battlefield { lane } => Some(lane),
        }
    }
    pub(crate) fn from_lane(player: usize, lane: Option<usize>) -> Self {
        match lane {
            Some(lane) => Self::Battlefield { lane },
            None => Self::Base { player },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoardTargetFacts {
    pub controller: usize,
    pub kind: CardKind,
    pub token: bool,
    pub location: BoardLocation,
    pub printed_energy: u8,
    pub domains: &'static [Domain],
}

/// A legal target's controller remains available to linked instructions even
/// if a future replacement prevents its death. Invalid targets have no link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GearKillResult {
    Ignored,
    Resolved { controller: usize, killed: bool },
}

impl Game {
    pub(crate) fn gear_kill_score(&self, actor: usize, target: Target) -> i128 {
        let Target::Gear { player, gear_id } = target else {
            return 0;
        };
        let Some(gear) = self
            .players
            .get(player)
            .and_then(|p| p.gear.iter().find(|g| g.id == gear_id))
        else {
            return 0;
        };
        let value = if gear.token {
            10
        } else {
            25 + i128::from(self.catalog[gear.card_id].cost) * 5
                + if gear.attached_to.is_some() { 15 } else { 0 }
                - if gear.card_id == 360 && gear.attached_to.is_none() {
                    30
                } else {
                    0
                }
        };
        if player == actor { -value } else { value }
    }
    pub(crate) fn kill_gear_target(&mut self, target: Target) -> GearKillResult {
        self.kill_gear_targets(&[target])
            .into_iter()
            .next()
            .map_or(GearKillResult::Ignored, |(_, result)| result)
    }

    /// One fixed simultaneous set. Capture Deathknell before any departure;
    /// resolving children and ordinary lethal cleanup happen after the parent.
    pub(crate) fn kill_gear_targets(
        &mut self,
        targets: &[Target],
    ) -> Vec<(Target, GearKillResult)> {
        let mut victims = Vec::new();
        for &target in targets {
            if let Target::Gear { player, gear_id } = target
                && self.board_target_facts(target).is_some()
                && !victims.contains(&(player, gear_id))
            {
                victims.push((player, gear_id));
            }
        }
        for &(player, id) in &victims {
            self.capture_gear_death(player, id);
        }
        let result = victims
            .into_iter()
            .map(|(player, gear_id)| {
                let killed = self.remove_gear_from_board(player, gear_id);
                (
                    Target::Gear { player, gear_id },
                    GearKillResult::Resolved {
                        controller: player,
                        killed,
                    },
                )
            })
            .collect::<Vec<_>>();
        if !result.is_empty() {
            self.trigger_batch += 1;
        }
        result
    }

    /// Absence is distinct from either player's base. Attached gear follows its
    /// actual bearer, while retaining its own cost, domains and target rules.
    pub fn board_target_facts(&self, target: Target) -> Option<BoardTargetFacts> {
        let (player, card_id, token, kind, location) = match target {
            Target::Unit { player, .. } => {
                let unit = self.unit_ref(target)?;
                (
                    player,
                    unit.card_id,
                    unit.token,
                    CardKind::Unit,
                    BoardLocation::from_lane(player, unit.lane),
                )
            }
            Target::Gear { player, gear_id } => {
                let state = self.players.get(player)?;
                let gear = state.gear.iter().find(|g| g.id == gear_id)?;
                let lane = self.gear_location(player, gear_id);
                (
                    player,
                    gear.card_id,
                    gear.token,
                    CardKind::Gear,
                    BoardLocation::from_lane(player, lane),
                )
            }
            _ => return None,
        };
        let card = self.catalog.get(card_id)?;
        Some(BoardTargetFacts {
            controller: player,
            kind,
            token,
            location,
            printed_energy: if token { 0 } else { card.cost },
            domains: if token { &[] } else { card.domains },
        })
    }

    pub(crate) fn board_target_has_tag(&self, target: Target, tag: &str) -> bool {
        match target {
            Target::Unit { .. } => self.unit_ref(target).is_some_and(|unit| {
                if unit.token {
                    rules::token_spec(unit.card_id).is_some_and(|spec| spec.tag == Some(tag))
                } else {
                    self.catalog[unit.card_id].tags.contains(&tag)
                }
            }),
            Target::Gear { player, gear_id } => self.players.get(player).is_some_and(|p| {
                p.gear.iter().find(|g| g.id == gear_id).is_some_and(|gear| {
                    !gear.token && self.catalog[gear.card_id].tags.contains(&tag)
                })
            }),
            _ => false,
        }
    }

    pub(crate) fn can_choose_board_target(&self, actor: usize, target: Target) -> bool {
        actor < self.players.len()
            && self
                .board_target_facts(target)
                .is_some_and(|facts| match facts.kind {
                    CardKind::Unit => self.can_choose_unit(actor, target),
                    CardKind::Gear => true,
                    CardKind::Spell => false,
                })
    }

    pub(crate) fn board_target_matches_kind(
        &self,
        target: Target,
        kind: rules::PlayTargetKind,
    ) -> bool {
        self.board_target_facts(target)
            .is_some_and(|facts| match kind {
                rules::PlayTargetKind::Unit => facts.kind == CardKind::Unit,
                rules::PlayTargetKind::Gear => facts.kind == CardKind::Gear,
                rules::PlayTargetKind::UnitOrGear => true,
                rules::PlayTargetKind::Spell | rules::PlayTargetKind::Battlefield => false,
            })
    }
}
