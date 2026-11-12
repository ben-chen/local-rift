//! A targeting relationship survives board movement and temporary mistargeting,
//! but not departure to a non-board zone (CR359.3.e.3–4, 359.3.e.9.a).
use super::*;

impl Game {
    /// All distinct currently present physical units/gear chosen by this item,
    /// including every Repeat. This deliberately does not revalidate the source
    /// item's original location, Might, allegiance, or protection restrictions.
    pub(super) fn live_chosen_board_targets(&self, item: &ChainItem) -> Vec<Target> {
        let mut live = Vec::new();
        for target in item.targets.iter().chain(&item.repeat_targets).chain(
            item.executions
                .iter()
                .flat_map(|execution| &execution.targets),
        ) {
            let current = match *target {
                Target::Unit { unit_id, .. } => {
                    self.players.iter().enumerate().find_map(|(player, state)| {
                        state
                            .units
                            .iter()
                            .any(|unit| unit.id == unit_id)
                            .then_some(Target::Unit { player, unit_id })
                    })
                }
                Target::Gear { gear_id, .. } => {
                    self.players.iter().enumerate().find_map(|(player, state)| {
                        state
                            .gear
                            .iter()
                            .any(|gear| gear.id == gear_id)
                            .then_some(Target::Gear { player, gear_id })
                    })
                }
                _ => None,
            };
            if let Some(target) = current
                && !live.contains(&target)
            {
                live.push(target);
            }
        }
        live
    }

    pub(super) fn repulse_unit_valid(&self, actor: usize, target: Target) -> bool {
        matches!(target, Target::Unit { player, .. } if player == actor)
            && self.can_choose_unit(actor, target)
            && self
                .unit_ref(target)
                .is_some_and(|unit| unit.lane.is_some())
    }

    pub(super) fn repulse_chain_target_valid(
        &self,
        actor: usize,
        chosen: Target,
        chain_id: u32,
    ) -> bool {
        let Some(item) = self
            .chain
            .iter()
            .find(|item| item.id == chain_id && item.player != actor)
        else {
            return false;
        };
        let units: Vec<_> = self
            .live_chosen_board_targets(item)
            .into_iter()
            .filter(|target| matches!(target, Target::Unit { player, .. } if *player == actor))
            .collect();
        units == [chosen]
    }

    pub(super) fn repulse_candidates(&self, actor: usize, prior: &[Target]) -> Vec<Target> {
        if actor >= self.players.len() {
            return vec![];
        }
        match prior {
            [] => self.players[actor]
                .units
                .iter()
                .map(|unit| Target::Unit {
                    player: actor,
                    unit_id: unit.id,
                })
                .filter(|&target| {
                    self.repulse_unit_valid(actor, target)
                        && self
                            .chain
                            .iter()
                            .any(|item| self.repulse_chain_target_valid(actor, target, item.id))
                })
                .collect(),
            [unit] if self.repulse_unit_valid(actor, *unit) => self
                .chain
                .iter()
                .rev()
                .filter(|item| self.repulse_chain_target_valid(actor, *unit, item.id))
                .map(|item| Target::Spell { chain_id: item.id })
                .collect(),
            _ => vec![],
        }
    }

    pub(super) fn validate_repulse_targets(
        &self,
        actor: usize,
        targets: &[Target],
        partial: bool,
    ) -> Result<(), String> {
        if targets.len() > 2 || (!partial && targets.len() != 2) {
            return Err("Repulse chooses a friendly unit at a battlefield, then an enemy spell or ability choosing it and no other friendly unit.".into());
        }
        for (index, target) in targets.iter().enumerate() {
            if !self
                .repulse_candidates(actor, &targets[..index])
                .contains(target)
            {
                return Err("Repulse needs a friendly battlefield unit and an enemy chain item choosing that unit and no other friendly unit still on the board.".into());
            }
        }
        Ok(())
    }

    pub(super) fn prepare_repulse_targets(
        &self,
        actor: usize,
        provided: &[Target],
        automatic: bool,
    ) -> Result<Vec<Target>, String> {
        self.validate_repulse_targets(actor, provided, true)?;
        let mut targets = provided.to_vec();
        if automatic {
            while targets.len() < 2 {
                let target = self
                    .repulse_candidates(actor, &targets)
                    .first()
                    .copied()
                    .ok_or("No linked unit and chain item can be chosen by Repulse.")?;
                targets.push(target);
            }
        }
        self.validate_repulse_targets(actor, &targets, false)?;
        Ok(targets)
    }

    pub(super) fn resolve_repulse(&mut self, actor: usize, targets: &[Target]) {
        if self.validate_repulse_targets(actor, targets, false).is_ok()
            && let Some(Target::Spell { chain_id }) = targets.get(1)
        {
            self.counter_spell(*chain_id, false);
        } else {
            self.push_log("Repulse has no legal linked targets; nothing is countered.".into());
        }
    }
}
