//! Limited plays share their origin, printed-cost waiver and prescribed location.
//! Their parent resolves completely before child choices and payments (CR354.3).
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaseCostWaiver {
    None,
    Energy,
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayDestination {
    Normal,
    Base,
    Exact(Option<usize>),
    AnyBattlefield,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayInstruction {
    /// Whole-cost Energy discount, applied after component discounts (CR356.4).
    pub energy_discount: u8,
    pub cost: BaseCostWaiver,
    pub destination: PlayDestination,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum PendingOrigin {
    Trash {
        index: usize,
        flow: bool,
        target_chain_ids: Vec<u32>,
    },
    Banish {
        index: usize,
    },
}

impl PendingCard {
    pub fn play_origin(&self) -> PlayOrigin {
        match self.origin {
            PendingOrigin::Trash { .. } => PlayOrigin::Trash,
            PendingOrigin::Banish { .. } => PlayOrigin::Banish,
        }
    }

    pub(crate) fn had_granted_flow(&self) -> bool {
        matches!(self.origin, PendingOrigin::Trash { flow: true, .. })
    }
}

impl InstructedPlayKind {
    pub(crate) fn instruction(self, previous_location: Option<usize>) -> PlayInstruction {
        use BaseCostWaiver::{All, Energy, None};
        use PlayDestination::{AnyBattlefield, Base, Exact, Normal};
        let (cost, destination) = match self {
            Self::Fizz | Self::ClocktowerGuardian => (Energy, Normal),
            Self::LastRites | Self::Reinforce | Self::WildClaw => (None, Normal),
            Self::BaitedHook | Self::DazzlingAurora => (All, Normal),
            Self::Matriarch | Self::PortalRescue => (All, Base),
            Self::TemporalBreach => (All, Exact(previous_location)),
            Self::ThrillOfTheHunt => (All, AnyBattlefield),
            Self::ArcaneShift => (All, Normal),
        };
        PlayInstruction {
            cost,
            destination,
            energy_discount: if matches!(self, Self::Reinforce | Self::WildClaw) {
                5
            } else {
                0
            },
        }
    }
}

impl PlayInstruction {
    pub(crate) fn validate_destination(self, lane: Option<usize>) -> Result<(), String> {
        let allowed = match self.destination {
            PlayDestination::Normal => true,
            PlayDestination::Base => lane.is_none(),
            PlayDestination::Exact(location) => lane == location,
            PlayDestination::AnyBattlefield => lane.is_some_and(|l| l < BATTLEFIELDS),
        };
        if allowed {
            Ok(())
        } else {
            Err(match self.destination {
                PlayDestination::Base => "This instruction plays the unit at its owner's base.",
                PlayDestination::Exact(_) => "Play the unit at the location it was banished from.",
                PlayDestination::AnyBattlefield => "This instruction requires a battlefield.",
                PlayDestination::Normal => unreachable!(),
            }
            .into())
        }
    }
}

impl Game {
    /// Looking/banishing is not drawing: an empty deck does not burn out.
    pub(crate) fn clocktower_deathknell(&mut self, player: usize) {
        let Some(card_id) = self.players[player].deck.pop() else {
            return;
        };
        let banish_index = self.players[player].banish.len();
        self.players[player].banish.push(card_id);
        self.push_log(format!(
            "Clocktower Guardian banishes {} from {}'s deck.",
            self.catalog[card_id].name, self.players[player].name
        ));
        self.enqueue_decision(Decision::BanishPlay {
            player,
            banish_index,
            card_id,
            kind: InstructedPlayKind::ClocktowerGuardian,
        });
    }

    pub(super) fn queue_banish_play(
        &mut self,
        player: usize,
        index: usize,
        kind: InstructedPlayKind,
        previous_location: Option<usize>,
    ) -> u32 {
        let card_id = self.players[player].banish.remove(index);
        let id = self.next_chain_id;
        self.next_chain_id += 1;
        self.queue_pending_card(PendingCard {
            id,
            player,
            card_id,
            kind,
            instruction: kind.instruction(previous_location),
            origin: PendingOrigin::Banish { index },
        });
        id
    }
    pub(crate) fn replay_target_score(&self, actor: usize, target: Target) -> Result<i128, String> {
        let Some(unit) = self.unit_ref(target) else {
            return Ok(0);
        };
        if matches!(target, Target::Unit { player, .. } if player != actor) {
            return Ok(if unit.token { 70 } else { 5 });
        }
        if unit.token {
            return Ok(-100);
        }
        let threatened = self
            .chain
            .iter()
            .any(|i| i.player != actor && i.targets.contains(&target));
        let temporary = self.has_temporary(empower::BoardObject::Unit {
            player: actor,
            unit_id: unit.id,
        });
        Ok(i128::from(unit.damage) * 8
            + if threatened { 80 } else { 0 }
            + if temporary { 65 } else { 0 }
            + if unit.stunned { 15 } else { 0 }
            - 8)
    }

    /// The same waiver feeds the quote and finalization; extra costs remain payable.
    pub(crate) fn current_play_instruction(&self) -> Option<PlayInstruction> {
        if let Some(card) = &self.pending_card {
            return Some(card.instruction);
        }
        let kind = match self.decision {
            Some(Decision::ReplaySpell { .. }) => InstructedPlayKind::Fizz,
            Some(Decision::LastRites { .. }) => InstructedPlayKind::LastRites,
            Some(Decision::ReviveUnit { .. } | Decision::RevivePlay { .. }) => {
                InstructedPlayKind::Matriarch
            }
            _ => return None,
        };
        Some(kind.instruction(None))
    }

    pub(crate) fn banish_and_replay(&mut self, target: Target, kind: InstructedPlayKind) {
        let Some(unit) = self.unit_ref(target) else {
            return;
        };
        let location = unit.lane;
        let token = unit.token;
        if let Some(owner) = self.banish_unit_directly(target) {
            // Tokens disappear upon leaving the board. Their creator's card is
            // never a substitute for the vanished object (CR108.4/420.3.c).
            if !token && !self.silenced[owner] {
                let index = self.players[owner].banish.len() - 1;
                self.queue_banish_play(owner, index, kind, location);
            }
        }
    }

    pub(crate) fn queue_pending_card(&mut self, card: PendingCard) {
        self.trigger_batch += 1;
        self.reserve_play_grants(card.player, self.catalog[card.card_id].kind, card.id, false);
        self.pending_triggers.push(PendingTrigger {
            batch: self.trigger_batch,
            option: TriggerOption {
                id: card.id,
                card_id: card.card_id,
                label: "Pending instructed card play".into(),
            },
            player: card.player,
            effect: PendingTriggerEffect::Card(card),
        });
        self.trigger_batch += 1;
    }
}

#[cfg(test)]
#[path = "instructed_play_tests.rs"]
mod tests;
