//! Shared meaning and eligibility of public trash choices.
use crate::cards::{CardDef, CardId, CardKind};
use crate::game::ChainAbility;
use crate::trigger_rules::{Effect, trigger_rules};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Play,
    ReturnToHand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChoiceRule {
    pub kind: CardKind,
    pub max_energy: Option<u8>,
    pub max_power: Option<u8>,
    pub outcome: Outcome,
}

impl ChoiceRule {
    pub fn allows(self, card: &CardDef) -> bool {
        card.kind == self.kind
            && self.max_energy.is_none_or(|n| card.cost <= n)
            && self.max_power.is_none_or(|n| card.power_cost <= n)
    }
}

pub fn choice_rule(source: CardId) -> Option<ChoiceRule> {
    let (kind, max_energy, max_power) = match source {
        36 => (CardKind::Spell, Some(3), None),
        38 => (CardKind::Unit, None, None),
        45 => (CardKind::Unit, Some(3), Some(1)),
        _ => {
            return trigger_rules(source)
                .iter()
                .find_map(|rule| match rule.effects {
                    [Effect::ReturnTrashToHand { kind }] => Some(ChoiceRule {
                        kind: *kind,
                        max_energy: None,
                        max_power: None,
                        outcome: Outcome::ReturnToHand,
                    }),
                    _ => None,
                });
        }
    };
    Some(ChoiceRule {
        kind,
        max_energy,
        max_power,
        outcome: Outcome::Play,
    })
}

pub fn returns_to_hand(source: CardId) -> bool {
    choice_rule(source).is_some_and(|rule| rule.outcome == Outcome::ReturnToHand)
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
