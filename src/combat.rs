//! Resumable combat-resolution tasks, in official Core 468 order.
use super::*;

/// The next outstanding task. Prior effects must finish before it can run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatStep {
    /// Special lethal cleanup, followed by its one-time global heal and recall.
    Cleanup,
    /// Wait for combat damage and cleanup effects before determining the result.
    DetermineResult,
    /// Wait for result effects before establishing control.
    EstablishControl,
    /// Wait for control/Conquer effects before removing combat designations.
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatResult {
    Winner { player: usize },
    NoResult,
}

/// Public, non-private progress. The closed showdown remains in `showdown` to
/// retain combat designations; it must not be presented as a new focus window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatResolution {
    pub lane: usize,
    pub attacker: usize,
    pub step: CombatStep,
    pub attackers_recalled: bool,
    /// Captured once at Core 468.3, unaffected by later Conquer responses.
    pub result: Option<CombatResult>,
}

impl Game {
    pub fn combat_resolution(&self) -> Option<CombatResolution> {
        self.combat_resolution
    }

    /// True only when a player may take a new showdown focus action. Combat
    /// damage, pending choices and completion checkpoints are not new windows.
    pub fn showdown_focus_open(&self) -> bool {
        self.showdown.is_some()
            && self.pending_combat.is_none()
            && self.combat_resolution.is_none()
            && self.chain.is_empty()
            && self.decision.is_none()
            && self.winner.is_none()
    }

    pub(super) fn finish_combat_special_cleanup(&mut self) {
        let combat = self.combat_resolution.expect("combat cleanup pending");
        debug_assert_eq!(combat.step, CombatStep::Cleanup);
        // Core 468.1.a inserts these tasks after lethal deaths, within the same
        // cleanup. Neither task is repeated by the resulting normal cleanups.
        for state in &mut self.players {
            for unit in &mut state.units {
                unit.damage = 0;
            }
        }
        let recalled = self.lane_count(combat.attacker, combat.lane) > 0
            && self.lane_count(1 - combat.attacker, combat.lane) > 0;
        if recalled {
            for unit in &mut self.players[combat.attacker].units {
                if unit.lane == Some(combat.lane) {
                    unit.lane = None;
                }
            }
            self.push_log("The defense holds. Surviving attackers are recalled to base.".into());
        }
        let pending = self.combat_resolution.as_mut().unwrap();
        pending.attackers_recalled = recalled;
        pending.step = CombatStep::DetermineResult;
    }

    pub(super) fn advance_combat_resolution(&mut self) -> Result<(), String> {
        let combat = self.combat_resolution.expect("combat resolution pending");
        debug_assert!(self.chain.is_empty());
        debug_assert!(self.decision.is_none());
        debug_assert!(self.pending_triggers.is_empty());
        self.chain_initiator = None;
        match combat.step {
            CombatStep::Cleanup => unreachable!("special cleanup precedes finalization"),
            CombatStep::DetermineResult => {
                let occupants = [
                    self.lane_count(0, combat.lane),
                    self.lane_count(1, combat.lane),
                ];
                let result = match (
                    combat.attackers_recalled,
                    occupants[0] > 0,
                    occupants[1] > 0,
                ) {
                    (false, true, false) => CombatResult::Winner { player: 0 },
                    (false, false, true) => CombatResult::Winner { player: 1 },
                    _ => CombatResult::NoResult,
                };
                let pending = self.combat_resolution.as_mut().unwrap();
                pending.result = Some(result);
                pending.step = CombatStep::EstablishControl;
                self.push_log(match result {
                    CombatResult::Winner { player } => format!(
                        "Combat result at battlefield {}: {} wins the combat.",
                        combat.lane + 1,
                        self.players[player].name
                    ),
                    CombatResult::NoResult => format!(
                        "Combat result at battlefield {}: no result.",
                        combat.lane + 1
                    ),
                });
                if let CombatResult::Winner { player } = result {
                    self.emit_event(EventSnapshot::battlefield(
                        Occurrence::CombatWin,
                        player,
                        combat.lane,
                    ));
                }
                if result == CombatResult::NoResult && occupants.iter().all(|&count| count > 0) {
                    // 468.3.d.1 explicitly stages the next showdown/combat.
                    // Opening waits until this combat has ended completely.
                    if !self
                        .pending_entries
                        .iter()
                        .any(|&(lane, _)| lane == combat.lane)
                    {
                        self.pending_entries.push((combat.lane, combat.attacker));
                    }
                }
            }
            CombatStep::EstablishControl => {
                self.combat_resolution.as_mut().unwrap().step = CombatStep::End;
                if !self
                    .pending_entries
                    .iter()
                    .any(|&(lane, _)| lane == combat.lane)
                {
                    self.establish_control(combat.lane)?;
                }
            }
            CombatStep::End => {
                // 468.7: only now do attacker/defender characteristics (including
                // Assault/Shield) cease. The ensuing ordinary cleanup can kill
                // a unit whose Might has fallen below damage from responses.
                self.combat_resolution = None;
                self.showdown = None;
                self.trigger_state.combat_seen.clear();
                self.priority_player = self.active;
                self.push_log("Combat ends; combat designations expire.".into());
            }
        }
        self.trigger_batch += 1;
        Ok(())
    }
}

#[cfg(test)]
#[path = "combat/tests.rs"]
mod tests;
