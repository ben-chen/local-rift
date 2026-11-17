//! Move instructions retain their actor and absolute locations. Recall is a
//! separate action and never emits a Move event (Core 449.2.c / 458).
use super::*;
use board_targets::BoardLocation;
use triggers::MoveFacts;

impl Game {
    /// Existing relative callers mean the target's base, never the actor's.
    pub(crate) fn move_effect(
        &mut self,
        actor: usize,
        target: Target,
        destination: Option<usize>,
        ready: bool,
    ) -> Result<(), String> {
        let Target::Unit { player, .. } = target else {
            return Ok(());
        };
        self.move_effect_at(
            actor,
            target,
            BoardLocation::from_lane(player, destination),
            ready,
        )
    }

    pub(crate) fn move_effect_at(
        &mut self,
        actor: usize,
        target: Target,
        destination: BoardLocation,
        ready: bool,
    ) -> Result<(), String> {
        let Target::Unit { player, unit_id } = target else {
            return Ok(());
        };
        let Some(origin) = self.board_target_facts(target).map(|facts| facts.location) else {
            return Ok(());
        };
        if let Some(movement) = self.relocate_for_effect(actor, target, destination)? {
            self.on_move(player, unit_id, movement);
            self.trigger_batch += 1;
            if let Some(lane) = movement.destination.battlefield() {
                self.stage_entry(lane, player);
            }
        }
        // Ready is a separate instruction even if the Move was negated (359.3.e.14).
        if ready {
            self.ready_unit_by(actor, player, unit_id);
        }
        if let Some(old) = origin.battlefield()
            && self
                .board_target_facts(target)
                .is_none_or(|facts| facts.location != origin)
        {
            self.refresh_control(old)?;
        }
        Ok(())
    }

    /// Mutate locations before publishing a simultaneous group's Move events.
    /// A prevented Move does nothing. An invalid destination instead Recalls;
    /// these are different cases and neither produces Move history.
    fn relocate_for_effect(
        &mut self,
        actor: usize,
        target: Target,
        destination: BoardLocation,
    ) -> Result<Option<MoveFacts>, String> {
        let Target::Unit { player, unit_id } = target else {
            return Ok(None);
        };
        let Some(unit) = self.unit_ref(target) else {
            return Ok(None);
        };
        let origin = BoardLocation::from_lane(player, unit.lane);
        if actor >= self.players.len()
            || match destination {
                BoardLocation::Base { player } => player >= self.players.len(),
                BoardLocation::Battlefield { lane } => lane >= self.battlefield_count(),
            }
        {
            return Err("Choose a location on this board.".into());
        }
        if self.movement_prevented(actor, player, unit_id) {
            return Ok(None);
        }
        if matches!(destination, BoardLocation::Base { player: base } if base != player) {
            self.recall_unit(player, unit_id, false, false);
            return Ok(None);
        }
        if origin == destination {
            return Ok(None);
        }
        self.unit_mut(target).unwrap().lane = destination.battlefield();
        Ok(Some(MoveFacts {
            actor,
            origin,
            destination,
        }))
    }

    pub(super) fn resolve_swap(
        &mut self,
        actor: usize,
        unit_id: u32,
        target: Target,
    ) -> Result<(), String> {
        let source = self.resolve_board_reference(Target::Unit {
            player: actor,
            unit_id,
        });
        let Some((a, b)) = self
            .board_target_facts(source)
            .zip(self.board_target_facts(target))
            .map(|(a, b)| (a.location, b.location))
        else {
            return Ok(());
        };
        let Target::Unit {
            player: owner,
            unit_id: other,
        } = target
        else {
            return Ok(());
        };
        let Target::Unit {
            player: source_controller,
            ..
        } = source
        else {
            unreachable!();
        };
        if a == b || owner != actor {
            return Ok(());
        }
        // Retain both original locations, and finish both relocations before
        // publishing the group's events. Each unit can independently be unable
        // to Move, or Recall instead of entering the other player's base.
        let mut moved = Vec::new();
        for (object, controller, id, destination) in [
            (source, source_controller, unit_id, b),
            (target, owner, other, a),
        ] {
            if let Some(facts) = self.relocate_for_effect(actor, object, destination)? {
                moved.push((controller, id, facts));
            }
        }
        for &(controller, id, facts) in &moved {
            self.on_move(controller, id, facts);
        }
        self.trigger_batch += 1;
        for (controller, _, facts) in moved {
            if let Some(lane) = facts.destination.battlefield() {
                self.stage_entry(lane, controller);
            }
        }
        for lane in [a, b].into_iter().filter_map(BoardLocation::battlefield) {
            self.refresh_control(lane)?;
        }
        Ok(())
    }

    pub(super) fn on_move(&mut self, player: usize, unit_id: u32, movement: triggers::MoveFacts) {
        let origin = movement.origin.battlefield();
        let destination = movement.destination.battlefield();
        if movement.origin != movement.destination {
            self.back_alley_move(origin, unit_id);
        }
        if let Some(unit) = self.players[player]
            .units
            .iter()
            .find(|u| u.id == unit_id)
            .cloned()
        {
            if origin != destination
                && unit.lane == destination
                && !self.moved_units.contains(&unit_id)
            {
                self.moved_units.push(unit_id);
            }
            let mut event = EventSnapshot::unit(Occurrence::Moved, player, unit);
            event.player = movement.actor;
            event.movement = Some(movement);
            self.emit_event(event);
        }
        let Some(unit) = self.players[player].units.iter().find(|u| u.id == unit_id) else {
            return;
        };
        if unit.token {
            return;
        }
        let source = unit.card_id;
        let ability = match source {
            2 if destination.is_some() => Some(TriggeredAbility::Might { unit_id, amount: 2 }),
            17 => Some(TriggeredAbility::Draw(1)),
            43 => {
                self.queue_trigger_choice(player, 43, Decision::BurnForMight { player, unit_id });
                None
            }
            47 => Some(TriggeredAbility::Loot { count: 1, draw: 1 }),
            _ => None,
        };
        if let Some(ability) = ability {
            self.queue_ability(player, source, Vec::new(), ChainAbility::Triggered(ability));
        }
    }
}
