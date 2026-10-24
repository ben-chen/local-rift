//! Explicit live passives. These predicates are acyclic and never mutate Might.
use crate::{
    cards::CardId,
    empower::BoardObject,
    game::{Game, Unit, UnitKeywords},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCondition {
    Always,
    Empowered,
    NotEmpowered,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recipients {
    SelfUnit,
    FriendlyUnits,
    FriendlyEmpoweredUnits,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassiveKeyword {
    Assault(u8),
    Shield(u8),
    Deflect(u8),
    Ganking,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassiveEffect {
    Might(i16),
    Keyword(PassiveKeyword),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassiveRule {
    pub condition: SourceCondition,
    pub recipients: Recipients,
    pub effect: PassiveEffect,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassiveContribution {
    pub source: BoardObject,
    pub card_id: CardId,
    pub rule_index: usize,
    pub active: bool,
    pub effect: PassiveEffect,
}
const fn own(effect: PassiveEffect) -> PassiveRule {
    PassiveRule {
        condition: SourceCondition::Empowered,
        recipients: Recipients::SelfUnit,
        effect,
    }
}
pub fn passive_rules(id: CardId) -> &'static [PassiveRule] {
    use PassiveEffect::*;
    use PassiveKeyword::*;
    match id {
        24 | 283 | 288 => const { &[own(Might(1))] },
        272 => const { &[own(Keyword(Assault(3)))] },
        273 => const { &[own(Might(7))] },
        274 | 279 => const { &[own(Keyword(Deflect(1))), own(Keyword(Shield(3)))] },
        275 => const { &[own(Might(1)), own(Keyword(Ganking))] },
        276 => const { &[own(Might(2)), own(Keyword(Ganking))] },
        277 => const { &[own(Might(1)), own(Keyword(Deflect(2)))] },
        278 => const { &[own(Might(3))] },
        280 => const { &[own(Keyword(Deflect(1))), own(Keyword(Assault(2)))] },
        281 => {
            const {
                &[
                    PassiveRule {
                        condition: SourceCondition::NotEmpowered,
                        recipients: Recipients::FriendlyUnits,
                        effect: Might(1),
                    },
                    PassiveRule {
                        condition: SourceCondition::Empowered,
                        recipients: Recipients::FriendlyUnits,
                        effect: Might(2),
                    },
                ]
            }
        }
        282 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::Empowered,
                    recipients: Recipients::FriendlyEmpoweredUnits,
                    effect: Might(2),
                }]
            }
        }
        286 => const { &[own(Might(2))] },
        289 => const { &[own(Keyword(Assault(2)))] },
        _ => &[],
    }
}
impl Game {
    pub fn passive_contributions(&self, target: BoardObject) -> Vec<PassiveContribution> {
        let BoardObject::Unit { player, unit_id } = target else {
            return vec![];
        };
        if self
            .players
            .get(player)
            .is_none_or(|p| !p.units.iter().any(|u| u.id == unit_id))
        {
            return vec![];
        }
        let mut result = Vec::new();
        for owner in 0..2 {
            let sources = self.players[owner]
                .units
                .iter()
                .filter(|u| !u.token)
                .map(|u| {
                    (
                        BoardObject::Unit {
                            player: owner,
                            unit_id: u.id,
                        },
                        u.card_id,
                    )
                })
                .chain(
                    self.players[owner]
                        .gear
                        .iter()
                        .filter(|g| !g.token && g.attached_to.is_none())
                        .map(|g| {
                            (
                                BoardObject::Gear {
                                    player: owner,
                                    gear_id: g.id,
                                },
                                g.card_id,
                            )
                        }),
                );
            for (source, card_id) in sources {
                for (rule_index, rule) in passive_rules(card_id).iter().enumerate() {
                    let recipient = match rule.recipients {
                        Recipients::SelfUnit => source == target,
                        _ => owner == player,
                    };
                    if !recipient {
                        continue;
                    }
                    let source_active = match rule.condition {
                        SourceCondition::Always => true,
                        SourceCondition::Empowered => self.object_empowered(source),
                        SourceCondition::NotEmpowered => !self.object_empowered(source),
                    };
                    let active = source_active
                        && (rule.recipients != Recipients::FriendlyEmpoweredUnits
                            || self.object_empowered(target));
                    result.push(PassiveContribution {
                        source,
                        card_id,
                        rule_index,
                        active,
                        effect: rule.effect,
                    });
                }
            }
        }
        result
    }
    fn unit_passives(&self, u: &Unit) -> Vec<PassiveContribution> {
        self.players
            .iter()
            .position(|p| p.units.iter().any(|x| x.id == u.id))
            .map(|player| {
                self.passive_contributions(BoardObject::Unit {
                    player,
                    unit_id: u.id,
                })
            })
            .unwrap_or_default()
    }
    pub(crate) fn continuous_might_bonus(&self, u: &Unit) -> i16 {
        self.unit_passives(u)
            .iter()
            .filter(|p| p.active)
            .map(|p| match p.effect {
                PassiveEffect::Might(n) => n,
                _ => 0,
            })
            .sum()
    }
    pub(crate) fn continuous_keywords(&self, u: &Unit) -> UnitKeywords {
        let mut out = UnitKeywords::default();
        for p in self.unit_passives(u).into_iter().filter(|p| p.active) {
            match p.effect {
                PassiveEffect::Keyword(PassiveKeyword::Assault(n)) => {
                    out.assault = out.assault.saturating_add(n)
                }
                PassiveEffect::Keyword(PassiveKeyword::Shield(n)) => {
                    out.shield = out.shield.saturating_add(n)
                }
                PassiveEffect::Keyword(PassiveKeyword::Deflect(n)) => {
                    out.deflect = out.deflect.saturating_add(n)
                }
                PassiveEffect::Keyword(PassiveKeyword::Ganking) => out.ganking = true,
                _ => {}
            }
        }
        out
    }
}
