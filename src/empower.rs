//! Actor-aware binary Empower transitions (Core 412.4, 442–443).
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BoardObject {
    Unit { player: usize, unit_id: u32 },
    Gear { player: usize, gear_id: u32 },
    Legend { player: usize },
}
impl BoardObject {
    pub fn player(self) -> usize {
        match self {
            Self::Unit { player, .. } | Self::Gear { player, .. } | Self::Legend { player } => {
                player
            }
        }
    }
}
impl Game {
    pub fn object_empowered(&self, object: BoardObject) -> bool {
        let Some(state) = self.players.get(object.player()) else {
            return false;
        };
        match object {
            BoardObject::Unit { unit_id, .. } => {
                state.units.iter().any(|u| u.id == unit_id) && self.unit_empowered(unit_id)
            }
            BoardObject::Gear { gear_id, .. } => {
                state.gear.iter().any(|g| g.id == gear_id) && self.unit_empowered(gear_id)
            }
            BoardObject::Legend { player } => self.legend[player].empowered,
        }
    }
    pub(crate) fn empower_by(&mut self, actor: usize, object: BoardObject) -> bool {
        if actor >= 2 || object.player() >= 2 || self.object_empowered(object) {
            return false;
        }
        let owner = object.player();
        let mut event = EventSnapshot::simple(Occurrence::Empowered, actor);
        event.subject_player = owner;
        match object {
            BoardObject::Unit { unit_id, .. } => {
                let Some(unit) = self.players[owner]
                    .units
                    .iter()
                    .find(|u| u.id == unit_id)
                    .cloned()
                else {
                    return false;
                };
                event.unit = Some(unit.clone());
                event.card_id = Some(unit.card_id);
                event.token = unit.token;
                self.empowered_units.push(unit_id);
            }
            BoardObject::Gear { gear_id, .. } => {
                let Some(gear) = self.players[owner].gear.iter().find(|g| g.id == gear_id) else {
                    return false;
                };
                event.gear_id = Some(gear_id);
                event.card_id = Some(gear.card_id);
                event.token = gear.token;
                self.empowered_units.push(gear_id);
            }
            BoardObject::Legend { player } => self.legend[player].empowered = true,
        }
        self.trigger_batch += 1;
        if self.legend[actor].kind == crate::legends::LegendKind::Ambessa
            && object != (BoardObject::Legend { player: actor })
        {
            self.queue_ability(
                actor,
                self.legend[actor].kind.champion_id(),
                vec![],
                ChainAbility::Legend(LegendAbility::MatriarchEmpower),
            );
        }
        if event.card_id == Some(45) && event.unit.as_ref().is_some_and(|u| !u.token) {
            self.queue_trigger_choice(owner, 45, Decision::ReviveUnit { player: owner });
        }
        self.emit_event(event);
        self.trigger_batch += 1;
        true
    }
    pub(crate) fn disempower_by(&mut self, _actor: usize, object: BoardObject) -> bool {
        if !self.object_empowered(object) {
            return false;
        }
        match object {
            BoardObject::Unit { unit_id, .. } => self.empowered_units.retain(|id| *id != unit_id),
            BoardObject::Gear { gear_id, .. } => self.empowered_units.retain(|id| *id != gear_id),
            BoardObject::Legend { player } => self.legend[player].empowered = false,
        }
        true
    }
    pub(crate) fn empower_object_legacy(
        &mut self,
        actor: usize,
        owner: usize,
        id: Option<u32>,
        temporary: Option<usize>,
    ) {
        let object = match id {
            Some(id)
                if self
                    .players
                    .get(owner)
                    .is_some_and(|p| p.units.iter().any(|u| u.id == id)) =>
            {
                BoardObject::Unit {
                    player: owner,
                    unit_id: id,
                }
            }
            Some(id) => BoardObject::Gear {
                player: owner,
                gear_id: id,
            },
            None => BoardObject::Legend { player: owner },
        };
        self.empower_by(actor, object);
        if let Some(controller) = temporary
            && !self.ending_turn
        {
            self.temporary_empower.push((controller, owner, id));
        }
    }
}
