//! Temporary characteristics, granted provenance and the single Beginning event.
use super::*;
use crate::empower::BoardObject;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TemporaryStatus {
    pub intrinsic: bool,
    pub granted: bool,
    pub active: bool,
}
impl Game {
    /// Physical source and captured name for an expiry awaiting trigger order.
    pub fn pending_temporary_source(&self, trigger_id: u32) -> Option<(BoardObject, &str)> {
        let pending = self
            .pending_triggers
            .iter()
            .find(|trigger| trigger.option.id == trigger_id)?;
        match &pending.effect {
            PendingTriggerEffect::Ability {
                ability:
                    ChainAbility::Temporary {
                        source,
                        source_name,
                    },
                ..
            } => Some((*source, source_name)),
            _ => None,
        }
    }
    pub fn temporary_status(&self, object: BoardObject) -> Option<TemporaryStatus> {
        match object {
            BoardObject::Unit { player, unit_id } => {
                let unit = self
                    .players
                    .get(player)?
                    .units
                    .iter()
                    .find(|u| u.id == unit_id)?;
                let intrinsic = if unit.token {
                    rules::token_spec(unit.card_id).is_some_and(|s| s.temporary)
                } else {
                    rules::static_abilities(unit.card_id).contains(&StaticAbility::Temporary)
                };
                let granted = self.granted_temporary_units.contains(&unit_id);
                Some(TemporaryStatus {
                    intrinsic,
                    granted,
                    active: intrinsic || granted,
                })
            }
            BoardObject::Gear { player, gear_id } => {
                let gear = self
                    .players
                    .get(player)?
                    .gear
                    .iter()
                    .find(|g| g.id == gear_id)?;
                let granted = self.granted_temporary_gear.contains(&gear_id);
                let intrinsic = !gear.token
                    && (rules::static_abilities(gear.card_id).contains(&StaticAbility::Temporary)
                        || (gear.temporary && !granted)); // legacy explicit-state fixtures
                Some(TemporaryStatus {
                    intrinsic,
                    granted,
                    active: granted || (intrinsic && gear.attached_to.is_none()),
                })
            }
            BoardObject::Legend { .. } => None,
        }
    }
    pub fn has_temporary(&self, object: BoardObject) -> bool {
        self.temporary_status(object)
            .is_some_and(|s| s.intrinsic || s.granted)
    }
    pub(crate) fn grant_temporary(&mut self, target: Target) {
        match target {
            Target::Unit { player, unit_id }
                if self
                    .players
                    .get(player)
                    .is_some_and(|p| p.units.iter().any(|u| u.id == unit_id)) =>
            {
                if !self.granted_temporary_units.contains(&unit_id) {
                    self.granted_temporary_units.push(unit_id);
                }
            }
            Target::Gear { player, gear_id } => {
                self.gear_make_temporary(player, 66, Target::Gear { player, gear_id })
            }
            _ => {}
        }
    }
    pub(crate) fn temporary_beginning(&mut self, player: usize) {
        self.trigger_batch += 1;
        let units: Vec<_> = self.players[player]
            .units
            .iter()
            .filter(|u| {
                self.temporary_status(BoardObject::Unit {
                    player,
                    unit_id: u.id,
                })
                .is_some_and(|s| s.active)
            })
            .map(|u| {
                (
                    u.card_id,
                    BoardObject::Unit {
                        player,
                        unit_id: u.id,
                    },
                    self.unit_name(u),
                )
            })
            .collect();
        let gears: Vec<_> = self.players[player]
            .gear
            .iter()
            .filter(|g| {
                self.gear_temporary_status(player, g.id)
                    .is_some_and(|s| s.active)
            })
            .map(|g| {
                (
                    g.card_id,
                    BoardObject::Gear {
                        player,
                        gear_id: g.id,
                    },
                    self.gear_name(g).to_owned(),
                )
            })
            .collect();
        for (card_id, source, source_name) in units.into_iter().chain(gears) {
            self.queue_ability(
                player,
                card_id,
                vec![],
                ChainAbility::Temporary {
                    source,
                    source_name,
                },
            );
        }
        self.emit_event(EventSnapshot::simple(
            Occurrence::ControllerBeginning,
            player,
        ));
        self.trigger_batch += 1;
    }
    pub(super) fn resolve_temporary(&mut self, source: BoardObject) -> Result<(), String> {
        match source {
            BoardObject::Unit { player, unit_id } => {
                self.kill_unit(Target::Unit { player, unit_id })?
            }
            BoardObject::Gear { player, gear_id } => {
                self.gear_destroy(player, gear_id);
            }
            BoardObject::Legend { .. } => {}
        }
        Ok(())
    }
}

impl Game {
    pub(super) fn current_hidden_slot_valid(
        &self,
        card_id: CardId,
        slot: usize,
        target: Target,
    ) -> bool {
        let Some(item) = self
            .resolution_frame
            .as_ref()
            .and_then(|f| self.chain.iter().find(|i| i.id == f.chain_id))
        else {
            return true;
        };
        !item.from_hidden
            || item.card_id != card_id
            || item
                .lane
                .is_some_and(|lane| self.hidden_target_allowed_at(card_id, lane, slot, target))
    }
    pub(super) fn current_hidden_origin(&self) -> Option<usize> {
        self.resolution_frame
            .as_ref()
            .and_then(|f| self.chain.iter().find(|i| i.id == f.chain_id))
            .filter(|i| i.from_hidden)
            .and_then(|i| i.lane)
    }
    pub(super) fn exchange_temporary_locations(
        &mut self,
        player: usize,
        first: Target,
        second: Target,
    ) -> Result<(), String> {
        let Some((a, b)) = self
            .unit_ref(first)
            .zip(self.unit_ref(second))
            .map(|(a, b)| (a.lane, b.lane))
        else {
            return Ok(());
        };
        let (
            Target::Unit {
                player: pa,
                unit_id: ia,
            },
            Target::Unit {
                player: pb,
                unit_id: ib,
            },
        ) = (first, second)
        else {
            return Ok(());
        };
        if pa != player
            || pb != player
            || a == b
            || !(self.has_temporary(empower::BoardObject::Unit {
                player: pa,
                unit_id: ia,
            }) || self.has_temporary(empower::BoardObject::Unit {
                player: pb,
                unit_id: ib,
            }))
        {
            return Ok(());
        }
        let mut moved = Vec::new();
        for (target, id, from, to) in [(first, ia, a, b), (second, ib, b, a)] {
            if !self.movement_locks.contains(&(player, id)) {
                self.unit_mut(target).unwrap().lane = to;
                moved.push((id, from, to));
            }
        }
        for &(id, from, to) in &moved {
            self.on_move(player, id, from, to);
        }
        self.trigger_batch += 1;
        for &(_, from, to) in &moved {
            for lane in [from, to].into_iter().flatten() {
                self.stage_entry(lane, player);
            }
        }
        for lane in [a, b].into_iter().flatten() {
            self.refresh_control(lane)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "temporary/tests.rs"]
mod tests;
