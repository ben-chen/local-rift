//! Return-to-hand transitions do not kill, recall, or trigger departure deaths.
use super::*;

#[derive(Clone, Copy)]
enum UnitDestination {
    Hand,
    Banish,
}

impl Game {
    /// Returns the owner for a linked instruction, even when a token vanishes.
    /// No admitted card transfers control; the stored seat is also the owner.
    pub(crate) fn return_unit_to_hand(&mut self, target: Target) -> Option<usize> {
        self.depart_unit_without_kill(target, UnitDestination::Hand)
    }

    /// Core 428.2.a: Banish is not Kill. Do not offer death replacements or
    /// emit death events, even when a Guardian Angel is attached.
    pub(crate) fn banish_unit_directly(&mut self, target: Target) -> Option<usize> {
        self.depart_unit_without_kill(target, UnitDestination::Banish)
    }

    fn depart_unit_without_kill(
        &mut self,
        target: Target,
        destination: UnitDestination,
    ) -> Option<usize> {
        let Target::Unit { player, unit_id } = target else {
            return None;
        };
        let index = self
            .players
            .get(player)?
            .units
            .iter()
            .position(|u| u.id == unit_id)?;
        self.empowered_units.retain(|&id| id != unit_id);
        self.granted_temporary_units.retain(|&id| id != unit_id);
        self.instructed_kills.retain(|&id| id != unit_id);
        self.gear_unit_left_board(player, unit_id);
        let unit = self.players[player].units.remove(index);
        self.push_log(format!(
            "{} {}.",
            self.unit_name(&unit),
            match destination {
                UnitDestination::Hand => "returns to its owner's hand",
                UnitDestination::Banish => "is banished",
            }
        ));
        if !unit.token {
            match destination {
                UnitDestination::Hand => self.players[player].hand.push(unit.card_id),
                UnitDestination::Banish => self.players[player].banish.push(unit.card_id),
            }
        }
        Some(player)
    }

    /// A rescue heuristic uses only board state and the public chain. Returning
    /// a token cannot save it, whereas a physical card can be replayed later.
    pub(crate) fn friendly_return_score(
        &self,
        actor: usize,
        target: Target,
    ) -> Result<i128, String> {
        let Some(unit) = self.unit_ref(target) else {
            return Ok(0);
        };
        if !matches!(target, Target::Unit { player, .. } if player == actor) || unit.token {
            return Ok(-100);
        }
        let threatened = self
            .chain
            .iter()
            .any(|item| item.player != actor && item.targets.contains(&target));
        let temporary = self.has_temporary(empower::BoardObject::Unit {
            player: actor,
            unit_id: unit.id,
        });
        Ok(self.return_target_score(actor, target)?
            + if threatened { 80 } else { 0 }
            + if temporary { 50 } else { 0 }
            + i128::from(unit.damage) * 4)
    }

    pub(crate) fn banish_or_return_score(
        &self,
        actor: usize,
        target: Target,
        maximum: u8,
    ) -> Result<i128, String> {
        let Some(unit) = self.unit_ref(target) else {
            return Ok(0);
        };
        let permanent = self.unit_might(unit)? <= u64::from(maximum) && !unit.token;
        let bonus = if permanent { 35 } else { 0 };
        Ok(self.return_target_score(actor, target)?
            + if matches!(target, Target::Unit { player, .. } if player == actor) {
                -bonus
            } else {
                bonus
            })
    }

    pub(crate) fn board_objects(&self) -> Vec<Target> {
        self.players
            .iter()
            .enumerate()
            .flat_map(|(player, state)| {
                state
                    .units
                    .iter()
                    .map(move |unit| Target::Unit {
                        player,
                        unit_id: unit.id,
                    })
                    .chain(state.gear.iter().map(move |gear| Target::Gear {
                        player,
                        gear_id: gear.id,
                    }))
            })
            .collect()
    }

    /// Capture the entire qualifying set before any departures change auras.
    pub(crate) fn units_at_most(&self, maximum: u8) -> Result<Vec<Target>, String> {
        let mut targets = Vec::new();
        for (player, state) in self.players.iter().enumerate() {
            for unit in &state.units {
                if self.unit_might(unit)? <= u64::from(maximum) {
                    targets.push(Target::Unit {
                        player,
                        unit_id: unit.id,
                    });
                }
            }
        }
        Ok(targets)
    }

    /// The caller fixes one simultaneous set. Do not resolve children or run
    /// lethal cleanup between departures. An attached gear occurs only once.
    pub(crate) fn return_board_targets(&mut self, targets: &[Target]) {
        for &target in targets {
            match target {
                Target::Unit { .. } => self.bounce(target),
                Target::Gear { player, gear_id } => {
                    let Some(state) = self.players.get_mut(player) else {
                        continue;
                    };
                    let Some(index) = state.gear.iter().position(|g| g.id == gear_id) else {
                        continue;
                    };
                    let gear = state.gear.remove(index);
                    self.granted_temporary_gear.retain(|&id| id != gear_id);
                    self.empowered_units.retain(|&id| id != gear_id);
                    if !gear.token {
                        state.hand.push(gear.card_id);
                    }
                    self.push_log(if gear.token {
                        "A gear token leaves the board and ceases to exist.".into()
                    } else {
                        format!(
                            "{} returns to its owner's hand.",
                            self.catalog[gear.card_id].name
                        )
                    });
                }
                _ => {}
            }
        }
    }

    /// Public information only: tempo gained by returning an opposing object,
    /// discounted because a physical card can be played again from hand.
    pub(crate) fn return_target_score(&self, actor: usize, target: Target) -> Result<i128, String> {
        let Some(facts) = self.board_target_facts(target) else {
            return Ok(0);
        };
        let value = if let Some(unit) = self.unit_ref(target) {
            12 + i128::from(self.unit_might(unit)?) * 3
                + if unit.lane.is_some() { 20 } else { 0 }
                + if unit.ready { 8 } else { 0 }
        } else {
            10 + i128::from(facts.printed_energy) * 4
        } + if facts.token { 12 } else { 0 };
        Ok(if facts.controller == actor {
            -value
        } else {
            value
        })
    }
}

#[cfg(test)]
#[path = "board_returns/tests.rs"]
mod tests;
