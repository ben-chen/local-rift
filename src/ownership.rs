//! Core 056/127: control does not change a card's non-Board destination.
use super::*;

impl Game {
    /// Execute a control instruction for a unit already at a battlefield.
    /// The caller validates the spell's targets and finishes its remaining
    /// instructions before cleanup (CR321). This is not a player action.
    /// Control neither plays nor Moves the object, and equipment stays owned
    /// and controlled by its own player (CR190.3.a.2 / 718.5).
    pub fn take_control_of_battlefield_unit(
        &mut self,
        controller: usize,
        target: Target,
    ) -> Option<Target> {
        if controller >= self.players.len() {
            return None;
        }
        let Target::Unit { player, unit_id } = target else {
            return None;
        };
        let lane = self
            .unit_ref(target)?
            .lane
            .filter(|&lane| lane < self.battlefield_count())?;
        if controller == player {
            return Some(target);
        }
        let owner = self.board_owner(target)?;
        let index = self.players[player]
            .units
            .iter()
            .position(|u| u.id == unit_id)?;
        let unit = self.players[player].units.remove(index);
        self.push_log(format!(
            "{} takes control of {}.",
            self.players[controller].name,
            self.unit_name(&unit)
        ));
        self.players[controller].units.push(unit);
        if controller == owner {
            self.board_owner_overrides.remove(&unit_id);
        } else {
            self.board_owner_overrides.insert(unit_id, owner);
        }
        self.stage_entry(lane, controller);
        Some(Target::Unit {
            player: controller,
            unit_id,
        })
    }

    /// Physical board identity survives changes of control. This lookup is for
    /// stored references; it does not bypass validation of submitted targets.
    pub fn unit_controller(&self, unit_id: u32) -> Option<usize> {
        self.players
            .iter()
            .position(|state| state.units.iter().any(|unit| unit.id == unit_id))
    }

    pub fn gear_controller(&self, gear_id: u32) -> Option<usize> {
        self.players
            .iter()
            .position(|state| state.gear.iter().any(|gear| gear.id == gear_id))
    }

    /// Resolve a previously finalized board reference by physical identity.
    /// New action submissions still validate their supplied controller normally.
    /// Departed objects keep an invalid reference; a new copy cannot replace it.
    pub(crate) fn resolve_board_reference(&self, target: Target) -> Target {
        match target {
            Target::Unit { player, unit_id } => Target::Unit {
                player: self.unit_controller(unit_id).unwrap_or(player),
                unit_id,
            },
            Target::Gear { player, gear_id } => Target::Gear {
                player: self.gear_controller(gear_id).unwrap_or(player),
                gear_id,
            },
            _ => target,
        }
    }

    pub(crate) fn resolve_board_references(&self, targets: &[Target]) -> Vec<Target> {
        targets
            .iter()
            .map(|&target| self.resolve_board_reference(target))
            .collect()
    }

    /// The physical source can move between controllers while its ability keeps
    /// the controller captured at finalization (CR191.5.b / 359.3.f.4).
    pub(crate) fn resolve_ability_source(&self, source: AbilitySource) -> AbilitySource {
        match source {
            AbilitySource::Unit { player, unit_id } => AbilitySource::Unit {
                player: self.unit_controller(unit_id).unwrap_or(player),
                unit_id,
            },
            AbilitySource::Gear { player, gear_id } => AbilitySource::Gear {
                player: self.gear_controller(gear_id).unwrap_or(player),
                gear_id,
            },
        }
    }

    pub(super) fn refresh_resolving_targets(&mut self) {
        let Some(item) = self.chain.last() else {
            return;
        };
        let target = item.target.map(|t| self.resolve_board_reference(t));
        let targets = self.resolve_board_references(&item.targets);
        let repeats = self.resolve_board_references(&item.repeat_targets);
        let mut ability = item.ability.clone();
        match &mut ability {
            Some(
                ChainAbility::Equip { target, .. }
                | ChainAbility::QuickDrawAttach { target, .. }
                | ChainAbility::Swap { target, .. },
            ) => *target = self.resolve_board_reference(*target),
            Some(ChainAbility::EmpowerLinked {
                target: Some(target),
            }) => {
                *target = self.resolve_board_reference(*target);
            }
            _ => {}
        }
        let item = self.chain.last_mut().unwrap();
        item.target = target;
        item.targets = targets;
        item.repeat_targets = repeats;
        item.ability = ability;
    }

    /// Ownership of a current unit or gear, independently of its controller.
    /// An invalid or departed board reference has no owner in this query.
    pub fn board_owner(&self, target: Target) -> Option<usize> {
        let (controller, id) = match target {
            Target::Unit { player, unit_id } if self.unit_ref(target).is_some() => {
                (player, unit_id)
            }
            Target::Gear { player, gear_id }
                if self
                    .players
                    .get(player)
                    .is_some_and(|state| state.gear.iter().any(|gear| gear.id == gear_id)) =>
            {
                (player, gear_id)
            }
            _ => return None,
        };
        Some(
            self.board_owner_overrides
                .get(&id)
                .copied()
                .unwrap_or(controller),
        )
    }

    /// Called once when a physical board object departs, after any replacement
    /// that keeps it on the board has been handled. Tokens also clear the entry.
    pub(crate) fn departed_board_owner(&mut self, controller: usize, id: u32) -> usize {
        self.detached_gear_bases.remove(&id);
        self.board_owner_overrides.remove(&id).unwrap_or(controller)
    }
}

#[cfg(test)]
#[path = "ownership_tests.rs"]
mod tests;
