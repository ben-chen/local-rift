//! Shared meaning and eligibility of public trash choices.
use crate::cards::{CardDef, CardId, CardKind};
use crate::game::{ChainAbility, Game, InstructedPlayKind, Target};
use crate::trigger_rules::{Effect, trigger_rules};

/// An immutable position at a particular trash-history revision. This is a
/// reference to one occurrence, not a current index or a card-definition ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reference {
    pub index: usize,
    pub revision: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Departure(usize);

#[derive(Clone, Debug)]
enum Change {
    Remove {
        player: usize,
        index: usize,
    },
    Restore {
        player: usize,
        index: usize,
        departure: Departure,
    },
    Clear {
        player: usize,
    },
}

/// Only removal, rollback insertion, and burnout change existing positions.
/// Normal arrivals append. Keeping changes here lets every immutable copy of a
/// target (chain, Repeat, payment draft, bot action) follow the same occurrence.
#[derive(Clone, Debug, Default)]
pub(crate) struct History(Vec<Change>);

impl History {
    pub(crate) fn reference(&self, index: usize) -> Reference {
        Reference {
            index,
            revision: self.0.len(),
        }
    }

    pub(crate) fn remove(&mut self, player: usize, index: usize) -> Departure {
        let departure = Departure(self.0.len());
        self.0.push(Change::Remove { player, index });
        departure
    }

    pub(crate) fn restore(&mut self, player: usize, index: usize, departure: Departure) {
        self.0.push(Change::Restore {
            player,
            index,
            departure,
        });
    }

    pub(crate) fn clear(&mut self, player: usize) {
        self.0.push(Change::Clear { player });
    }

    pub(crate) fn resolve(&self, player: usize, reference: Reference) -> Option<usize> {
        let mut index = Some(reference.index);
        let mut departure = None;
        for (revision, change) in self.0.get(reference.revision..)?.iter().enumerate() {
            match *change {
                Change::Remove {
                    player: owner,
                    index: removed,
                } if owner == player => {
                    if index == Some(removed) {
                        index = None;
                        departure = Some(Departure(reference.revision + revision));
                    } else if let Some(current) = &mut index
                        && *current > removed
                    {
                        *current -= 1;
                    }
                }
                Change::Restore {
                    player: owner,
                    index: restored,
                    departure: original,
                } if owner == player => {
                    if departure == Some(original) {
                        index = Some(restored);
                        departure = None;
                    } else if let Some(current) = &mut index
                        && *current >= restored
                    {
                        *current = current.checked_add(1)?;
                    }
                }
                Change::Clear { player: owner } if owner == player => {
                    // An already detached pending card isn't in the burnout.
                    // Its exact refused-play restoration can still reverse it.
                    index = None;
                }
                _ => {}
            }
        }
        index
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Play(InstructedPlayKind),
    ReturnToHand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellRule {
    pub kinds: &'static [CardKind],
    pub owner: crate::rules::Allegiance,
    pub minimum: usize,
    pub maximum: usize,
    pub outcome: Outcome,
}

impl Game {
    /// Capture an existing public trash occurrence. Subsequent index shifts
    /// do not change this token; an actual departure invalidates it.
    pub fn trash_target(&self, player: usize, index: usize) -> Option<Target> {
        let card_id = *self.players.get(player)?.discard.get(index)?;
        Some(Target::Trash {
            player,
            card_id,
            reference: self.trash_history.reference(index),
        })
    }

    pub fn trash_target_index(&self, target: Target) -> Option<usize> {
        let Target::Trash {
            player,
            card_id,
            reference,
        } = target
        else {
            return None;
        };
        let index = self.trash_history.resolve(player, reference)?;
        (*self.players.get(player)?.discard.get(index)? == card_id).then_some(index)
    }

    /// Old and newly offered tokens may identify the same surviving card.
    pub fn same_target(&self, first: Target, second: Target) -> bool {
        match (first, second) {
            (Target::Trash { player: a, .. }, Target::Trash { player: b, .. }) if a == b => self
                .trash_target_index(first)
                .is_some_and(|i| self.trash_target_index(second) == Some(i)),
            _ => first == second,
        }
    }

    pub(crate) fn trash_spell_target_valid(
        &self,
        actor: usize,
        rule: SpellRule,
        target: Target,
    ) -> bool {
        let Target::Trash {
            player, card_id, ..
        } = target
        else {
            return false;
        };
        let owner_allowed = match rule.owner {
            crate::rules::Allegiance::Friendly => player == actor,
            crate::rules::Allegiance::Enemy => player != actor,
            crate::rules::Allegiance::Any => true,
            crate::rules::Allegiance::None | crate::rules::Allegiance::Mixed => false,
        };
        owner_allowed
            && self.trash_target_index(target).is_some()
            && self
                .catalog
                .get(card_id)
                .is_some_and(|card| rule.kinds.contains(&card.kind))
    }

    pub(crate) fn trash_target_score(&self, actor: usize, target: Target) -> i128 {
        let Target::Trash {
            player, card_id, ..
        } = target
        else {
            return 0;
        };
        if self.trash_target_index(target).is_none() {
            return 0;
        }
        let card = &self.catalog[card_id];
        let value = 40 + i128::from(card.cost) * 5 + i128::from(card.might) * 8;
        if player == actor { value } else { -value }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChoiceRule {
    pub kinds: &'static [CardKind],
    pub optional: bool,
    pub max_energy: Option<u8>,
    pub max_power: Option<u8>,
    pub outcome: Outcome,
}

impl ChoiceRule {
    pub fn allows(self, card: &CardDef) -> bool {
        self.kinds.contains(&card.kind)
            && self.max_energy.is_none_or(|n| card.cost <= n)
            && self.max_power.is_none_or(|n| card.power_cost <= n)
    }
}

pub fn choice_rule(source: CardId) -> Option<ChoiceRule> {
    let (kinds, max_energy, max_power, instruction): (&'static [CardKind], _, _, _) = match source {
        36 => (&[CardKind::Spell], Some(3), None, InstructedPlayKind::Fizz),
        38 => (&[CardKind::Unit], None, None, InstructedPlayKind::LastRites),
        45 => (
            &[CardKind::Unit],
            Some(3),
            Some(1),
            InstructedPlayKind::Matriarch,
        ),
        _ => {
            return trigger_rules(source)
                .iter()
                .find_map(|rule| match rule.effects {
                    [Effect::ReturnTrashToHand { kinds }] => Some(ChoiceRule {
                        kinds,
                        optional: rule.optional,
                        max_energy: None,
                        max_power: None,
                        outcome: Outcome::ReturnToHand,
                    }),
                    [Effect::PlayTrash { kinds, instruction }] => Some(ChoiceRule {
                        kinds,
                        optional: rule.optional,
                        max_energy: None,
                        max_power: None,
                        outcome: Outcome::Play(*instruction),
                    }),
                    _ => None,
                });
        }
    };
    Some(ChoiceRule {
        kinds,
        optional: false,
        max_energy,
        max_power,
        outcome: Outcome::Play(instruction),
    })
}

pub fn returns_to_hand(source: CardId) -> bool {
    choice_rule(source).is_some_and(|rule| rule.outcome == Outcome::ReturnToHand)
}

pub fn optional(source: CardId) -> bool {
    choice_rule(source).is_some_and(|rule| rule.optional)
}

impl ChainAbility {
    pub(crate) fn trash_index(&self) -> Option<usize> {
        match self {
            Self::PlayChosen { trash_index, .. }
            | Self::ReviveChosen { trash_index, .. }
            | Self::GrantFlowChosen { trash_index, .. }
            | Self::ReturnChosen { trash_index, .. } => Some(*trash_index),
            _ => None,
        }
    }

    /// All trash movements use this accessor so each choice tracks its exact copy.
    pub(crate) fn trash_index_mut(&mut self) -> Option<&mut usize> {
        match self {
            Self::PlayChosen { trash_index, .. }
            | Self::ReviveChosen { trash_index, .. }
            | Self::GrantFlowChosen { trash_index, .. }
            | Self::ReturnChosen { trash_index, .. } => Some(trash_index),
            _ => None,
        }
    }
}
