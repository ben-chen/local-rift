//! Non-targeting object choices made by each player during resolution (CR355.10.e).
//! The next player's options are read after the preceding action completes.
use super::*;

#[cfg(test)]
#[path = "player_choices_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    KillOwnGear,
    ReturnAnyUnit,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Self::KillOwnGear => "kill_own_gear",
            Self::ReturnAnyUnit => "return_unit",
        }
    }

    pub fn instruction(self) -> &'static str {
        match self {
            Self::KillOwnGear => "Choose one of your gear to kill.",
            Self::ReturnAnyUnit => "Return any unit to its owner's hand, or decline.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    TurnPlayerFirst,
    AfterController,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rule {
    pub kind: Kind,
    pub order: Order,
    pub optional: bool,
}

impl Rule {
    pub const ACCEPTABLE_LOSSES: Self = Self {
        kind: Kind::KillOwnGear,
        order: Order::TurnPlayerFirst,
        optional: false,
    };
    pub const WHIRLWIND: Self = Self {
        kind: Kind::ReturnAnyUnit,
        order: Order::AfterController,
        optional: true,
    };
}

impl Game {
    fn player_objects_for(&self, player: usize, kind: Kind) -> Vec<Target> {
        match kind {
            Kind::KillOwnGear => self.players[player]
                .gear
                .iter()
                .map(|gear| Target::Gear {
                    player,
                    gear_id: gear.id,
                })
                .collect(),
            Kind::ReturnAnyUnit => self
                .players
                .iter()
                .enumerate()
                .flat_map(|(owner, state)| {
                    state.units.iter().map(move |unit| Target::Unit {
                        player: owner,
                        unit_id: unit.id,
                    })
                })
                .collect(),
        }
    }

    /// Shared eligibility for the client, JSON API, bots and validation. These
    /// object references are not targets: protection and Deflect do not apply.
    pub fn player_object_candidates(&self) -> Vec<Target> {
        match self.decision {
            Some(Decision::PlayerObject { player, rule, .. }) => {
                self.player_objects_for(player, rule.kind)
            }
            _ => Vec::new(),
        }
    }

    fn player_object_order(&self, controller: usize, rule: Rule) -> Vec<usize> {
        let first = match rule.order {
            Order::TurnPlayerFirst => self.active,
            Order::AfterController => 1 - controller,
        };
        vec![first, 1 - first]
    }

    pub(super) fn begin_player_objects(
        &mut self,
        controller: usize,
        card_id: CardId,
        rule: Rule,
    ) -> Result<(), String> {
        let choice_id = self.next_chain_id;
        self.next_chain_id = choice_id
            .checked_add(2)
            .ok_or_else(|| crate::numeric::capacity("resolution choice IDs"))?;
        self.queue_player_object(
            card_id,
            rule,
            choice_id,
            self.player_object_order(controller, rule),
        );
        Ok(())
    }

    fn queue_player_object(
        &mut self,
        card_id: CardId,
        rule: Rule,
        mut choice_id: u32,
        mut remaining: Vec<usize>,
    ) {
        while !remaining.is_empty() {
            let player = remaining.remove(0);
            if !self.player_objects_for(player, rule.kind).is_empty() {
                self.enqueue_decision(Decision::PlayerObject {
                    player,
                    choice_id,
                    card_id,
                    rule,
                    remaining,
                });
                return;
            }
            // No object can be chosen. Do as much as possible and proceed.
            choice_id += 1;
        }
    }

    pub fn validate_player_object(
        &self,
        choice_id: u32,
        object: Option<Target>,
    ) -> Result<(), String> {
        let Some(Decision::PlayerObject {
            choice_id: current,
            rule,
            ..
        }) = &self.decision
        else {
            return Err("No each-player object choice is pending.".into());
        };
        if choice_id != *current {
            return Err("This resolution choice is stale.".into());
        }
        match object {
            Some(object) if self.player_object_candidates().contains(&object) => Ok(()),
            None if rule.optional => Ok(()),
            None => Err("Choose one of your gear; this instruction is mandatory.".into()),
            _ => Err("Choose an eligible object currently in play.".into()),
        }
    }

    fn perform_player_object(&mut self, kind: Kind, object: Target) {
        match kind {
            Kind::KillOwnGear => {
                self.kill_gear_target(object);
            }
            Kind::ReturnAnyUnit => {
                self.return_unit_to_hand(object);
            }
        }
    }

    pub(super) fn choose_player_object(
        &mut self,
        choice_id: u32,
        object: Option<Target>,
    ) -> Result<(), String> {
        self.validate_player_object(choice_id, object)?;
        let Some(Decision::PlayerObject {
            player,
            card_id,
            rule,
            remaining,
            ..
        }) = self.decision.take()
        else {
            unreachable!()
        };
        if let Some(object) = object {
            self.perform_player_object(rule.kind, object);
        } else {
            self.push_log(format!(
                "{} declines {}'s optional return.",
                self.players[player].name, self.catalog[card_id].name
            ));
        }
        self.queue_player_object(card_id, rule, choice_id + 1, remaining);
        // after_resolution keeps the parent execution suspended while a choice
        // remains, then completes it before releasing its triggered abilities.
        Ok(())
    }

    fn player_object_score(
        &self,
        player: usize,
        kind: Kind,
        object: Target,
    ) -> Result<i128, String> {
        match kind {
            Kind::KillOwnGear => Ok(self.gear_kill_score(player, object)),
            Kind::ReturnAnyUnit => self.return_target_score(player, object),
        }
    }

    fn preferred_player_object(
        &self,
        player: usize,
        rule: Rule,
    ) -> Result<(i128, Option<Target>), String> {
        let mut best = (if rule.optional { 0 } else { i128::MIN }, None);
        for object in self.player_objects_for(player, rule.kind) {
            let value = self.player_object_score(player, rule.kind, object)?;
            if value > best.0 {
                best = (value, Some(object));
            }
        }
        Ok(if best.0 == i128::MIN { (0, None) } else { best })
    }

    pub(super) fn player_object_ai_action(&self) -> Result<Option<Action>, String> {
        let Some(Decision::PlayerObject {
            player,
            choice_id,
            rule,
            ..
        }) = self.decision
        else {
            return Ok(None);
        };
        Ok(Some(Action::ChoosePlayerObject {
            choice_id,
            object: self.preferred_player_object(player, rule)?.1,
        }))
    }

    pub(super) fn player_objects_spell_score(
        &self,
        player: usize,
        rule: Rule,
    ) -> Result<i128, String> {
        let mut probe = self.clone();
        let mut value = 0;
        for owner in self.player_object_order(player, rule) {
            let (score, object) = probe.preferred_player_object(owner, rule)?;
            value += if owner == player { score } else { -score };
            if let Some(object) = object {
                probe.perform_player_object(rule.kind, object);
            }
        }
        Ok(value)
    }
}
