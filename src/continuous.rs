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
    AttackingWithAnother,
    CombatAlone,
    Buffed,
    OtherFriendlyHere,
    ControllerRunesAtLeast(u8),
    ControllerXpAtLeast(u8),
    ControllerGainedXpThisTurn,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recipients {
    SelfUnit,
    FriendlyUnits,
    FriendlyEmpoweredUnits,
    OtherFriendlyUnitsHere,
    FriendlyTokenUnits,
    AttachedBearer,
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
    MightPerFriendlyTemporaryHere { amount: u8 },
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
    /// Exact evaluated value shared by previews and final numeric aggregation.
    pub computed_might: Option<i128>,
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
        352 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::Always,
                    recipients: Recipients::SelfUnit,
                    effect: MightPerFriendlyTemporaryHere { amount: 1 },
                }]
            }
        }
        337 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::ControllerXpAtLeast(6),
                    recipients: Recipients::SelfUnit,
                    effect: Might(1),
                }]
            }
        }
        338 => {
            const {
                &[
                    PassiveRule {
                        condition: SourceCondition::ControllerXpAtLeast(3),
                        recipients: Recipients::SelfUnit,
                        effect: Might(1),
                    },
                    PassiveRule {
                        condition: SourceCondition::ControllerXpAtLeast(3),
                        recipients: Recipients::SelfUnit,
                        effect: Keyword(Ganking),
                    },
                ]
            }
        }
        339 => {
            const {
                &[
                    PassiveRule {
                        condition: SourceCondition::ControllerXpAtLeast(3),
                        recipients: Recipients::SelfUnit,
                        effect: Might(1),
                    },
                    PassiveRule {
                        condition: SourceCondition::ControllerXpAtLeast(3),
                        recipients: Recipients::SelfUnit,
                        effect: Keyword(Deflect(1)),
                    },
                ]
            }
        }
        340 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::ControllerXpAtLeast(3),
                    recipients: Recipients::SelfUnit,
                    effect: Might(1),
                }]
            }
        }
        342 => {
            const {
                &[
                    PassiveRule {
                        condition: SourceCondition::ControllerGainedXpThisTurn,
                        recipients: Recipients::SelfUnit,
                        effect: Might(1),
                    },
                    PassiveRule {
                        condition: SourceCondition::ControllerGainedXpThisTurn,
                        recipients: Recipients::SelfUnit,
                        effect: Keyword(Ganking),
                    },
                ]
            }
        }
        320 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::Always,
                    recipients: Recipients::OtherFriendlyUnitsHere,
                    effect: Might(1),
                }]
            }
        }
        321 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::Buffed,
                    recipients: Recipients::SelfUnit,
                    effect: Might(1),
                }]
            }
        }
        322 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::Buffed,
                    recipients: Recipients::SelfUnit,
                    effect: Keyword(Ganking),
                }]
            }
        }
        323 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::OtherFriendlyHere,
                    recipients: Recipients::SelfUnit,
                    effect: Might(1),
                }]
            }
        }
        324 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::Always,
                    recipients: Recipients::FriendlyTokenUnits,
                    effect: Might(1),
                }]
            }
        }
        325 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::ControllerRunesAtLeast(8),
                    recipients: Recipients::SelfUnit,
                    effect: Might(4),
                }]
            }
        }
        326 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::ControllerXpAtLeast(11),
                    recipients: Recipients::SelfUnit,
                    effect: Might(4),
                }]
            }
        }
        330 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::ControllerXpAtLeast(3),
                    recipients: Recipients::AttachedBearer,
                    effect: Might(1),
                }]
            }
        }
        331 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::Always,
                    recipients: Recipients::FriendlyUnits,
                    effect: Keyword(Deflect(1)),
                }]
            }
        }
        304 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::AttackingWithAnother,
                    recipients: Recipients::SelfUnit,
                    effect: Might(2),
                }]
            }
        }
        305 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::CombatAlone,
                    recipients: Recipients::SelfUnit,
                    effect: Might(2),
                }]
            }
        }
        306 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::Always,
                    recipients: Recipients::OtherFriendlyUnitsHere,
                    effect: Keyword(Assault(1)),
                }]
            }
        }
        307 => {
            const {
                &[PassiveRule {
                    condition: SourceCondition::Always,
                    recipients: Recipients::OtherFriendlyUnitsHere,
                    effect: Keyword(Shield(1)),
                }]
            }
        }
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
    pub fn passive_contributions(
        &self,
        target: BoardObject,
    ) -> Result<Vec<PassiveContribution>, String> {
        let BoardObject::Unit { player, unit_id } = target else {
            return Ok(vec![]);
        };
        if self
            .players
            .get(player)
            .is_none_or(|p| !p.units.iter().any(|u| u.id == unit_id))
        {
            return Ok(vec![]);
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
                        .filter(|g| !g.token)
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
                    let attachment = if let BoardObject::Gear { gear_id, .. } = source {
                        self.players[owner]
                            .gear
                            .iter()
                            .find(|gear| gear.id == gear_id)
                            .and_then(|gear| gear.attached_to)
                    } else {
                        None
                    };
                    if matches!(source, BoardObject::Gear { .. })
                        && attachment.is_some() != (rule.recipients == Recipients::AttachedBearer)
                    {
                        continue;
                    }
                    let recipient = match rule.recipients {
                        Recipients::AttachedBearer => {
                            owner == player && attachment == Some(unit_id)
                        }
                        Recipients::FriendlyTokenUnits => {
                            owner == player
                                && self.players[player]
                                    .units
                                    .iter()
                                    .any(|unit| unit.id == unit_id && unit.token)
                        }
                        Recipients::SelfUnit => source == target,
                        Recipients::OtherFriendlyUnitsHere => {
                            owner == player
                                && source != target
                                && match source {
                                    BoardObject::Unit {
                                        unit_id: source_id, ..
                                    } => self.players[owner]
                                        .units
                                        .iter()
                                        .find(|u| u.id == source_id)
                                        .zip(
                                            self.players[player]
                                                .units
                                                .iter()
                                                .find(|u| u.id == unit_id),
                                        )
                                        .is_some_and(|(a, b)| a.lane == b.lane),
                                    BoardObject::Gear { .. } | BoardObject::Legend { .. } => false,
                                }
                        }
                        _ => owner == player,
                    };
                    if !recipient {
                        continue;
                    }
                    let source_active = match rule.condition {
                        SourceCondition::Always => true,
                        SourceCondition::Buffed => {
                            matches!(source, BoardObject::Unit { unit_id, .. } if self.buffed_units.contains(&unit_id))
                        }
                        SourceCondition::ControllerRunesAtLeast(count) => {
                            self.players[owner].total_runes() >= count
                        }
                        SourceCondition::ControllerXpAtLeast(count) => {
                            self.players[owner].xp >= u64::from(count)
                        }
                        SourceCondition::ControllerGainedXpThisTurn => {
                            self.gained_xp_this_turn(owner)
                        }
                        SourceCondition::OtherFriendlyHere => match source {
                            BoardObject::Unit { unit_id, .. } => self.players[owner]
                                .units
                                .iter()
                                .find(|u| u.id == unit_id)
                                .is_some_and(|source| {
                                    self.players[owner].units.iter().any(|unit| {
                                        unit.id != source.id && unit.lane == source.lane
                                    })
                                }),
                            _ => false,
                        },
                        SourceCondition::Empowered => self.object_empowered(source),
                        SourceCondition::NotEmpowered => !self.object_empowered(source),
                        SourceCondition::AttackingWithAnother | SourceCondition::CombatAlone => {
                            match source {
                                BoardObject::Unit { unit_id, .. } => self.players[owner]
                                    .units
                                    .iter()
                                    .find(|u| u.id == unit_id)
                                    .is_some_and(|u| {
                                        self.showdown.as_ref().is_some_and(|w| {
                                            w.combat
                                                && u.lane == Some(w.lane)
                                                && if rule.condition
                                                    == SourceCondition::AttackingWithAnother
                                                {
                                                    w.attacker == owner
                                                        && self.players[owner]
                                                            .units
                                                            .iter()
                                                            .filter(|v| v.lane == u.lane)
                                                            .count()
                                                            > 1
                                                } else {
                                                    self.players[owner]
                                                        .units
                                                        .iter()
                                                        .filter(|v| v.lane == u.lane)
                                                        .count()
                                                        == 1
                                                }
                                        })
                                    }),
                                BoardObject::Gear { .. } | BoardObject::Legend { .. } => false,
                            }
                        }
                    };
                    let active = source_active
                        && (rule.recipients != Recipients::FriendlyEmpoweredUnits
                            || self.object_empowered(target));
                    let computed_might = match rule.effect {
                        PassiveEffect::Might(amount) => {
                            Some(if active { i128::from(amount) } else { 0 })
                        }
                        PassiveEffect::Keyword(_) => None,
                        PassiveEffect::MightPerFriendlyTemporaryHere { amount } => {
                            let lane = match source {
                                BoardObject::Unit { player, unit_id } if active => self.players
                                    [player]
                                    .units
                                    .iter()
                                    .find(|u| u.id == unit_id)
                                    .and_then(|u| u.lane),
                                _ => None,
                            };
                            let count = if let Some(lane) = lane {
                                self.players[owner]
                                    .units
                                    .iter()
                                    .filter(|unit| {
                                        unit.lane == Some(lane)
                                            && self.has_temporary(BoardObject::Unit {
                                                player: owner,
                                                unit_id: unit.id,
                                            })
                                    })
                                    .try_fold(0_i128, |count, _| {
                                        count.checked_add(1).ok_or_else(|| {
                                            crate::numeric::capacity("Temporary unit count")
                                        })
                                    })?
                            } else {
                                0
                            };
                            Some(
                                count
                                    .checked_mul(i128::from(amount))
                                    .ok_or_else(|| crate::numeric::capacity("continuous Might"))?,
                            )
                        }
                    };
                    result.push(PassiveContribution {
                        source,
                        card_id,
                        rule_index,
                        active,
                        effect: rule.effect,
                        computed_might,
                    });
                }
            }
        }
        Ok(result)
    }
    fn unit_passives(&self, u: &Unit) -> Result<Vec<PassiveContribution>, String> {
        let Some(player) = self
            .players
            .iter()
            .position(|p| p.units.iter().any(|x| x.id == u.id))
        else {
            return Ok(vec![]);
        };
        self.passive_contributions(BoardObject::Unit {
            player,
            unit_id: u.id,
        })
    }
    pub(crate) fn continuous_might_bonus(&self, u: &Unit) -> Result<i128, String> {
        self.unit_passives(u)?
            .iter()
            .filter_map(|p| p.computed_might)
            .try_fold(0_i128, |sum, amount| {
                sum.checked_add(amount)
                    .ok_or_else(|| crate::numeric::capacity("continuous Might"))
            })
    }
    pub(crate) fn continuous_keywords(&self, u: &Unit) -> Result<UnitKeywords, String> {
        let mut out = UnitKeywords::default();
        for p in self.unit_passives(u)?.into_iter().filter(|p| p.active) {
            match p.effect {
                PassiveEffect::Keyword(PassiveKeyword::Assault(n)) => {
                    out.assault = crate::numeric::add_u64(out.assault, u64::from(n), "assault")?
                }
                PassiveEffect::Keyword(PassiveKeyword::Shield(n)) => {
                    out.shield = crate::numeric::add_u64(out.shield, u64::from(n), "shield")?
                }
                PassiveEffect::Keyword(PassiveKeyword::Deflect(n)) => {
                    out.deflect = out.deflect.saturating_add(n)
                }
                PassiveEffect::Keyword(PassiveKeyword::Ganking) => out.ganking = true,
                _ => {}
            }
        }
        Ok(out)
    }
}
