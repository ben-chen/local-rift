//! Explicit trigger descriptors for the reviewed event-driven card batch.
//! These are interpreted by the runtime, never inferred from printed keywords.
//! Timing and event boundaries follow Core Rules 337, 354-355 and 383.

use crate::rules::{Allegiance, PlayTargetKind, TokenPlacement};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    ControllerBeginning,
    ControllerEnding,
    SelfPlayed,
    FriendlyUnitPlayed,
    FriendlyGearPlayed,
    FriendlySpellPlayed,
    FriendlyCardPlayed,
    FriendlyHiddenPlayed,
    SelfReady,
    FriendlyReady,
    SelfExhausted,
    SelfBuffed,
    SelfEmpowered,
    SelfMoved,
    SelfDeath,
    FriendlyDeath,
    EnemyDeath,
    SelfAttack,
    SelfDefend,
    FriendlyAttack,
    FriendlyDefend,
    SelfConquer,
    SelfHold,
    FriendlyDraw,
    GearActivated,
    FriendlyBattlefieldHold,
    OpponentBattlefieldHold,
    FriendlyBattlefieldConquer,
    FriendlyCombatWin,
}

/// Conditions are evaluated when the event occurs. They are not effect-side
/// conditions to be rechecked during resolution (CR383.2.a.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    ControllerGearAtLeast(u8),
    ControllerXpAtLeast(u8),
    Other,
    FromBattlefield,
    ToBattlefield,
    PowerAtLeast(u8),
    EnergyAtLeast(u8),
    SecondCard,
    OpponentTurn,
    NonRecruit,
    Buffed,
    StunnedHere,
    OtherFriendlyHere(u8),
    Alone,
    FirstEachTurn,
    SecondDraw,
    SourceEmpowered,
    SourceAtEventBattlefield,
    ControllerControlsSourceBattlefield,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventLocationConstraint {
    None,
    MovedToOrFromBattlefield,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativeMightConstraint {
    None,
    LessThanSource,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativeEnergyConstraint {
    None,
    AtMostSourceMight,
}
/// Ordinary Here follows the live source. Deathknell may explicitly refer to
/// the location recorded before its source left the board (Core 808.1.d.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HereLocationBasis {
    LiveSource,
    SourceDeparture,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetRule {
    pub kind: PlayTargetKind,
    pub min: u8,
    pub max: u8,
    pub allegiance: Allegiance,
    pub another: bool,
    pub battlefield: bool,
    pub here: bool,
    pub here_basis: HereLocationBasis,
    pub max_might: Option<u8>,
    pub max_energy: Option<u8>,
    pub tag: Option<&'static str>,
    pub distinct: bool,
    pub event_locations: EventLocationConstraint,
    pub relative_might: RelativeMightConstraint,
    pub relative_energy: RelativeEnergyConstraint,
}
impl TargetRule {
    pub const NONE: Self = Self {
        kind: PlayTargetKind::Unit,
        min: 0,
        max: 0,
        allegiance: Allegiance::None,
        another: false,
        battlefield: false,
        here: false,
        here_basis: HereLocationBasis::LiveSource,
        max_might: None,
        max_energy: None,
        tag: None,
        distinct: true,
        event_locations: EventLocationConstraint::None,
        relative_might: RelativeMightConstraint::None,
        relative_energy: RelativeEnergyConstraint::None,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operand {
    Source,
    Subject,
    Targets,
    AllHere,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Amount {
    Fixed(u8),
    FriendlyUnits,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    KillSourceGear,
    ReturnSourceGear,
    MoveTargetsToSourceBattlefield,
    DrawLook(crate::game::DrawLookKind),
    RevealMovedTop,
    ClocktowerDeathknell,
    DazzlingAurora,
    KillUnitOrGear,
    ReturnUnitsAtMost(u8),
    KillGear {
        buff_source_if_killed: bool,
    },
    GiveTemporary(Operand),
    Xp(Amount),
    Draw(u8),
    DrawBoth(u8),
    Loot {
        discard: u8,
        draw: u8,
    },
    Might {
        who: Operand,
        amount: i8,
        minimum: u8,
    },
    Ready(Operand),
    Buff(Operand),
    Damage(u8),
    Bounce,
    FightSource,
    Stun,
    Predict,
    PredictMany(u8),
    Channel {
        count: u8,
        exhausted: bool,
    },
    KillTargets,
    DamageIfSourceEmpowered {
        base: u8,
        empowered: u8,
    },
    Tokens {
        count: u8,
        might: u8,
        gear: bool,
        opponent: bool,
        location: TokenPlacement,
        entry_ready: bool,
    },
    Points(u8),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerTextArea {
    Rules,
    Equipment,
}
/// A cost appearing first in a triggered instruction is paid during
/// finalization (Core 204.1.c, 404.1.b.1), before responses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerBaseCost {
    pub energy: u16,
    pub power: u16,
    pub domain: Option<crate::cards::Domain>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerRule {
    pub text_area: TriggerTextArea,
    pub event: Event,
    pub conditions: &'static [Condition],
    pub optional: bool,
    pub base_cost: Option<TriggerBaseCost>,
    pub targets: TargetRule,
    pub effects: &'static [Effect],
}
const ANY: TargetRule = TargetRule {
    min: 1,
    max: 1,
    allegiance: Allegiance::Any,
    ..TargetRule::NONE
};
const FRIENDLY: TargetRule = TargetRule {
    allegiance: Allegiance::Friendly,
    ..ANY
};
const ENEMY_HERE: TargetRule = TargetRule {
    allegiance: Allegiance::Enemy,
    here: true,
    ..ANY
};
const fn rule(
    event: Event,
    conditions: &'static [Condition],
    optional: bool,
    targets: TargetRule,
    effects: &'static [Effect],
) -> TriggerRule {
    TriggerRule {
        text_area: TriggerTextArea::Rules,
        event,
        conditions,
        optional,
        base_cost: None,
        targets,
        effects,
    }
}

const fn equipment_rule(
    event: Event,
    conditions: &'static [Condition],
    optional: bool,
    targets: TargetRule,
    effects: &'static [Effect],
) -> TriggerRule {
    TriggerRule {
        text_area: TriggerTextArea::Equipment,
        ..rule(event, conditions, optional, targets, effects)
    }
}

/// One explicit mapping per reviewed card; other identities have no rules here.
/// Equipment rules describe the abilities of the attached bearer. The runtime
/// retains both gear identity and bearer identity when finalizing each trigger.
pub fn trigger_rules(card_id: usize) -> &'static [TriggerRule] {
    use Amount::*;
    use Condition::*;
    use Effect::*;
    use Event::*;
    use Operand::*;
    use TokenPlacement::*;
    const DEPLOY_HOLD: TriggerRule = rule(
        OpponentBattlefieldHold,
        &[SourceAtEventBattlefield],
        false,
        TargetRule::NONE,
        &[KillSourceGear],
    );
    const POSTER: TriggerRule = TriggerRule {
        base_cost: Some(TriggerBaseCost {
            energy: 1,
            power: 0,
            domain: None,
        }),
        ..rule(
            SelfPlayed,
            &[],
            true,
            TargetRule::NONE,
            &[Tokens {
                count: 1,
                might: 1,
                gear: false,
                opponent: false,
                location: Base,
                entry_ready: false,
            }],
        )
    };
    match card_id {
        401 => const { &[DEPLOY_HOLD] },
        402 => {
            const {
                &[
                    DEPLOY_HOLD,
                    rule(
                        FriendlyCombatWin,
                        &[SourceAtEventBattlefield],
                        false,
                        TargetRule::NONE,
                        &[Draw(1)],
                    ),
                ]
            }
        }
        403 => {
            const {
                &[
                    DEPLOY_HOLD,
                    POSTER,
                    TriggerRule {
                        event: FriendlyBattlefieldConquer,
                        conditions: &[SourceAtEventBattlefield],
                        ..POSTER
                    },
                ]
            }
        }
        404 => {
            const {
                &[
                    DEPLOY_HOLD,
                    rule(
                        SelfPlayed,
                        &[],
                        true,
                        TargetRule {
                            allegiance: Allegiance::Enemy,
                            ..ANY
                        },
                        &[MoveTargetsToSourceBattlefield],
                    ),
                ]
            }
        }
        405 => {
            const {
                &[
                    DEPLOY_HOLD,
                    rule(SelfPlayed, &[], false, FRIENDLY, &[Buff(Targets)]),
                    rule(
                        FriendlyBattlefieldConquer,
                        &[SourceAtEventBattlefield],
                        false,
                        FRIENDLY,
                        &[Buff(Targets)],
                    ),
                ]
            }
        }
        406 => {
            const {
                &[
                    DEPLOY_HOLD,
                    TriggerRule {
                        base_cost: Some(TriggerBaseCost {
                            energy: 0,
                            power: 1,
                            domain: Some(crate::cards::Domain::Body),
                        }),
                        ..rule(
                            SelfPlayed,
                            &[],
                            true,
                            TargetRule { here: true, ..ANY },
                            &[Ready(Targets)],
                        )
                    },
                ]
            }
        }
        407 => {
            const {
                &[
                    DEPLOY_HOLD,
                    rule(
                        ControllerEnding,
                        &[ControllerControlsSourceBattlefield],
                        false,
                        TargetRule::NONE,
                        &[Tokens {
                            count: 1,
                            might: 0,
                            gear: true,
                            opponent: false,
                            location: Base,
                            entry_ready: false,
                        }],
                    ),
                ]
            }
        }
        408 => {
            const {
                &[
                    DEPLOY_HOLD,
                    rule(
                        SelfPlayed,
                        &[],
                        false,
                        TargetRule::NONE,
                        &[Loot {
                            discard: 1,
                            draw: 3,
                        }],
                    ),
                    rule(
                        FriendlyBattlefieldHold,
                        &[SourceAtEventBattlefield],
                        false,
                        TargetRule::NONE,
                        &[ReturnSourceGear],
                    ),
                ]
            }
        }
        395 => {
            const {
                &[
                    rule(
                        SelfPlayed,
                        &[],
                        false,
                        TargetRule::NONE,
                        &[DrawLook(crate::game::DrawLookKind::Ornn)],
                    ),
                    rule(
                        SelfHold,
                        &[],
                        false,
                        TargetRule::NONE,
                        &[DrawLook(crate::game::DrawLookKind::Ornn)],
                    ),
                ]
            }
        }
        390 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[DrawLook(crate::game::DrawLookKind::FateWeaver)],
                )]
            }
        }
        391 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[DrawLook(crate::game::DrawLookKind::TrifarianCaptain)],
                )]
            }
        }
        392..=394 => {
            const {
                &[rule(
                    SelfMoved,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[RevealMovedTop],
                )]
            }
        }
        386 => {
            const {
                &[rule(
                    ControllerEnding,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[DazzlingAurora],
                )]
            }
        }
        384 => {
            const RULES: &[TriggerRule] = &[
                rule(SelfPlayed, &[], false, TargetRule::NONE, &[PredictMany(3)]),
                rule(SelfHold, &[], false, TargetRule::NONE, &[PredictMany(3)]),
                rule(
                    SelfDeath,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[ClocktowerDeathknell],
                ),
            ];
            RULES
        }
        376 => {
            const RULES: &[TriggerRule] = &[rule(
                SelfPlayed,
                &[],
                true,
                TargetRule {
                    battlefield: true,
                    ..FRIENDLY
                },
                &[Bounce],
            )];
            RULES
        }
        374 => {
            const RULES: &[TriggerRule] = &[rule(
                SelfDeath,
                &[],
                true,
                TargetRule {
                    kind: PlayTargetKind::UnitOrGear,
                    here: true,
                    here_basis: HereLocationBasis::SourceDeparture,
                    ..ANY
                },
                &[KillUnitOrGear],
            )];
            RULES
        }
        375 => {
            const RULES: &[TriggerRule] = &[rule(
                SelfPlayed,
                &[],
                false,
                TargetRule::NONE,
                &[ReturnUnitsAtMost(2)],
            )];
            RULES
        }
        366..=369 => {
            const RAKE: TriggerRule = rule(
                SelfPlayed,
                &[],
                true,
                TargetRule {
                    kind: PlayTargetKind::Gear,
                    ..ANY
                },
                &[KillGear {
                    buff_source_if_killed: false,
                }],
            );
            const GOATHERD: TriggerRule = TriggerRule {
                targets: TargetRule {
                    max_energy: Some(4),
                    ..RAKE.targets
                },
                ..RAKE
            };
            const DEMOLITIONIST: TriggerRule = TriggerRule {
                event: SelfConquer,
                targets: TargetRule {
                    relative_energy: RelativeEnergyConstraint::AtMostSourceMight,
                    ..RAKE.targets
                },
                ..RAKE
            };
            const ADAPTATRON: TriggerRule = TriggerRule {
                event: SelfConquer,
                effects: &[KillGear {
                    buff_source_if_killed: true,
                }],
                ..RAKE
            };
            match card_id {
                366 => &[RAKE],
                367 => &[GOATHERD],
                368 => &[DEMOLITIONIST],
                _ => &[ADAPTATRON],
            }
        }
        370 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[ControllerGearAtLeast(2)],
                    false,
                    TargetRule::NONE,
                    &[Ready(Operand::Source)],
                )]
            }
        }
        351 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[OpponentTurn],
                    false,
                    TargetRule::NONE,
                    &[GiveTemporary(Operand::Source)],
                )]
            }
        }
        357 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 1,
                        might: 3,
                        gear: false,
                        opponent: false,
                        location: Here,
                        entry_ready: true,
                    }],
                )]
            }
        }
        358 => {
            const {
                &[
                    rule(
                        SelfPlayed,
                        &[],
                        false,
                        TargetRule::NONE,
                        &[Tokens {
                            count: 1,
                            might: 3,
                            gear: false,
                            opponent: false,
                            location: Base,
                            entry_ready: true,
                        }],
                    ),
                    rule(
                        ControllerBeginning,
                        &[],
                        false,
                        TargetRule::NONE,
                        &[Tokens {
                            count: 1,
                            might: 3,
                            gear: false,
                            opponent: false,
                            location: Base,
                            entry_ready: true,
                        }],
                    ),
                ]
            }
        }
        359 => {
            const {
                &[rule(
                    SelfHold,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 1,
                        might: 3,
                        gear: false,
                        opponent: false,
                        location: Here,
                        entry_ready: true,
                    }],
                )]
            }
        }
        360 => {
            const {
                &[
                    rule(
                        SelfPlayed,
                        &[],
                        false,
                        TargetRule::NONE,
                        &[Tokens {
                            count: 1,
                            might: 3,
                            gear: false,
                            opponent: false,
                            location: Base,
                            entry_ready: true,
                        }],
                    ),
                    rule(
                        SelfDeath,
                        &[],
                        false,
                        TargetRule::NONE,
                        &[Tokens {
                            count: 1,
                            might: 3,
                            gear: false,
                            opponent: false,
                            location: Base,
                            entry_ready: true,
                        }],
                    ),
                ]
            }
        }
        361 => {
            const {
                &[rule(
                    SelfDeath,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[
                        Channel {
                            count: 2,
                            exhausted: true,
                        },
                        Draw(1),
                    ],
                )]
            }
        }

        348 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Xp(Amount::Fixed(1))],
                )]
            }
        }
        341 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[ControllerXpAtLeast(6)],
                    false,
                    TargetRule::NONE,
                    &[Draw(1)],
                )]
            }
        }
        343 => {
            const {
                &[rule(
                    SelfMoved,
                    &[ToBattlefield],
                    false,
                    TargetRule::NONE,
                    &[Xp(Fixed(2))],
                )]
            }
        }
        344 => {
            const {
                &[rule(
                    SelfMoved,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Xp(Fixed(1))],
                )]
            }
        }
        314 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule {
                        allegiance: Allegiance::Enemy,
                        ..ANY
                    },
                    &[KillTargets],
                )]
            }
        }
        315 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule {
                        allegiance: Allegiance::Enemy,
                        max_might: Some(3),
                        ..ANY
                    },
                    &[KillTargets],
                )]
            }
        }
        316 => {
            const {
                &[rule(
                    SelfDeath,
                    &[],
                    false,
                    TargetRule {
                        allegiance: Allegiance::Enemy,
                        ..ANY
                    },
                    &[Damage(4)],
                )]
            }
        }
        317 => {
            const {
                &[rule(
                    SelfConquer,
                    &[],
                    false,
                    FRIENDLY,
                    &[Might {
                        who: Targets,
                        amount: 8,
                        minimum: 0,
                    }],
                )]
            }
        }
        318 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 1,
                        might: 3,
                        gear: false,
                        opponent: false,
                        location: Here,
                        entry_ready: false,
                    }],
                )]
            }
        }
        319 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 1,
                        might: 3,
                        gear: false,
                        opponent: false,
                        location: Here,
                        entry_ready: true,
                    }],
                )]
            }
        }
        329 => {
            const {
                &[equipment_rule(
                    SelfConquer,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Channel {
                        count: 1,
                        exhausted: true,
                    }],
                )]
            }
        }
        332 => {
            const {
                &[rule(
                    SelfMoved,
                    &[ToBattlefield],
                    false,
                    TargetRule {
                        another: true,
                        ..FRIENDLY
                    },
                    &[Might {
                        who: Targets,
                        amount: 1,
                        minimum: 0,
                    }],
                )]
            }
        }
        333 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule {
                        min: 0,
                        max: 2,
                        ..ANY
                    },
                    &[Damage(1)],
                )]
            }
        }
        300 | 301 => {
            const {
                &[rule(
                    SelfDeath,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Channel {
                        count: 1,
                        exhausted: true,
                    }],
                )]
            }
        }
        302 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Channel {
                        count: 1,
                        exhausted: true,
                    }],
                )]
            }
        }
        303 => {
            const {
                &[rule(
                    SelfDeath,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[PredictMany(2)],
                )]
            }
        }
        283 => {
            const {
                &[rule(
                    SelfEmpowered,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[PredictMany(2)],
                )]
            }
        }
        284 => {
            const {
                &[rule(
                    SelfMoved,
                    &[SourceEmpowered],
                    false,
                    TargetRule::NONE,
                    &[Draw(1)],
                )]
            }
        }
        285 => {
            const {
                &[rule(
                    SelfDeath,
                    &[SourceEmpowered],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 2,
                        might: 1,
                        gear: false,
                        opponent: false,
                        location: Base,
                        entry_ready: false,
                    }],
                )]
            }
        }
        286 => {
            const {
                &[rule(
                    SelfDeath,
                    &[SourceEmpowered],
                    false,
                    TargetRule::NONE,
                    &[Channel {
                        count: 2,
                        exhausted: true,
                    }],
                )]
            }
        }
        287 => {
            const {
                &[rule(
                    SelfConquer,
                    &[SourceEmpowered],
                    false,
                    TargetRule::NONE,
                    &[Points(1)],
                )]
            }
        }
        288 => {
            const {
                &[rule(
                    SelfMoved,
                    &[],
                    true,
                    TargetRule {
                        event_locations: EventLocationConstraint::MovedToOrFromBattlefield,
                        ..ANY
                    },
                    &[DamageIfSourceEmpowered {
                        base: 1,
                        empowered: 2,
                    }],
                )]
            }
        }
        289 => {
            const {
                &[rule(
                    SelfAttack,
                    &[SourceEmpowered],
                    false,
                    TargetRule {
                        relative_might: RelativeMightConstraint::LessThanSource,
                        ..ENEMY_HERE
                    },
                    &[KillTargets],
                )]
            }
        }
        246 => const { &[rule(SelfPlayed, &[], false, TargetRule::NONE, &[Draw(1)])] },
        152 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Xp(Fixed(1))],
                )]
            }
        }
        154 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Xp(Fixed(2))],
                )]
            }
        }
        155 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Xp(FriendlyUnits)],
                )]
            }
        }
        156 => {
            const {
                &[rule(
                    FriendlyGearPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Ready(Source)],
                )]
            }
        }
        157 => {
            const {
                &[rule(
                    SelfReady,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Might {
                        who: Source,
                        amount: 2,
                        minimum: 0,
                    }],
                )]
            }
        }
        158 => {
            const {
                &[rule(
                    SelfBuffed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Ready(Source)],
                )]
            }
        }
        159 => {
            const {
                &[rule(
                    FriendlyDeath,
                    &[Other],
                    false,
                    TargetRule::NONE,
                    &[Xp(Fixed(1))],
                )]
            }
        }
        160 => {
            const {
                &[rule(
                    SelfHold,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 2,
                        might: 0,
                        gear: true,
                        opponent: false,
                        location: Base,
                        entry_ready: false,
                    }],
                )]
            }
        }
        161 => {
            const {
                &[rule(
                    SelfConquer,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 1,
                        might: 0,
                        gear: true,
                        opponent: false,
                        location: Base,
                        entry_ready: false,
                    }],
                )]
            }
        }
        162 => {
            const {
                &[rule(
                    SelfMoved,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 1,
                        might: 0,
                        gear: true,
                        opponent: false,
                        location: Base,
                        entry_ready: false,
                    }],
                )]
            }
        }
        163 => {
            const {
                &[rule(
                    FriendlyHiddenPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 1,
                        might: 0,
                        gear: true,
                        opponent: false,
                        location: Base,
                        entry_ready: false,
                    }],
                )]
            }
        }
        164 => {
            const {
                &[rule(
                    FriendlyHiddenPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Might {
                        who: Source,
                        amount: 2,
                        minimum: 0,
                    }],
                )]
            }
        }
        165 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule {
                        another: true,
                        tag: Some("Mech"),
                        ..FRIENDLY
                    },
                    &[Ready(Targets)],
                )]
            }
        }
        166 => {
            const {
                &[rule(
                    SelfAttack,
                    &[],
                    false,
                    TargetRule {
                        another: true,
                        ..FRIENDLY
                    },
                    &[Ready(Targets)],
                )]
            }
        }
        167 => const { &[rule(SelfAttack, &[], false, ENEMY_HERE, &[Damage(1)])] },
        168 => {
            const {
                &[
                    rule(
                        SelfAttack,
                        &[],
                        false,
                        ENEMY_HERE,
                        &[Might {
                            who: Targets,
                            amount: -2,
                            minimum: 1,
                        }],
                    ),
                    rule(
                        SelfDefend,
                        &[],
                        false,
                        ENEMY_HERE,
                        &[Might {
                            who: Targets,
                            amount: -2,
                            minimum: 1,
                        }],
                    ),
                ]
            }
        }
        169 => {
            const {
                &[rule(
                    SelfMoved,
                    &[FromBattlefield],
                    false,
                    TargetRule::NONE,
                    &[Might {
                        who: Source,
                        amount: 2,
                        minimum: 0,
                    }],
                )]
            }
        }
        170 => {
            const {
                &[rule(
                    FriendlyUnitPlayed,
                    &[Other],
                    false,
                    TargetRule::NONE,
                    &[Might {
                        who: Source,
                        amount: 2,
                        minimum: 0,
                    }],
                )]
            }
        }
        171 => {
            const {
                &[rule(
                    SelfDeath,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Loot {
                        discard: 2,
                        draw: 2,
                    }],
                )]
            }
        }
        172 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Loot {
                        discard: 1,
                        draw: 0,
                    }],
                )]
            }
        }
        173 => {
            const {
                &[rule(
                    FriendlySpellPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Might {
                        who: Source,
                        amount: 2,
                        minimum: 0,
                    }],
                )]
            }
        }
        174 => {
            const {
                &[rule(
                    FriendlyUnitPlayed,
                    &[Other],
                    false,
                    TargetRule::NONE,
                    &[Buff(Source)],
                )]
            }
        }
        175 => {
            const {
                &[rule(
                    FriendlyCardPlayed,
                    &[OpponentTurn],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 1,
                        might: 1,
                        gear: false,
                        opponent: false,
                        location: Base,
                        entry_ready: false,
                    }],
                )]
            }
        }
        176 => {
            const {
                &[rule(
                    FriendlyDeath,
                    &[Other, NonRecruit],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 1,
                        might: 1,
                        gear: false,
                        opponent: false,
                        location: Base,
                        entry_ready: false,
                    }],
                )]
            }
        }
        177 => {
            const {
                &[rule(
                    SelfDeath,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 3,
                        might: 1,
                        gear: false,
                        opponent: false,
                        location: Base,
                        entry_ready: false,
                    }],
                )]
            }
        }
        178 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 1,
                        might: 2,
                        gear: false,
                        opponent: false,
                        location: Here,
                        entry_ready: false,
                    }],
                )]
            }
        }
        179 => {
            const {
                &[rule(
                    SelfMoved,
                    &[ToBattlefield],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 1,
                        might: 1,
                        gear: false,
                        opponent: false,
                        location: Here,
                        entry_ready: false,
                    }],
                )]
            }
        }
        180 => {
            const {
                &[rule(
                    SelfMoved,
                    &[ToBattlefield],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 3,
                        might: 1,
                        gear: false,
                        opponent: false,
                        location: Here,
                        entry_ready: false,
                    }],
                )]
            }
        }
        181 => {
            const {
                &[rule(
                    FriendlyCardPlayed,
                    &[PowerAtLeast(2)],
                    false,
                    TargetRule::NONE,
                    &[Draw(1)],
                )]
            }
        }
        182 => {
            const {
                &[rule(
                    FriendlySpellPlayed,
                    &[EnergyAtLeast(5)],
                    false,
                    TargetRule::NONE,
                    &[Might {
                        who: Source,
                        amount: 3,
                        minimum: 0,
                    }],
                )]
            }
        }
        183 => {
            const {
                &[rule(
                    FriendlyCardPlayed,
                    &[SecondCard],
                    false,
                    TargetRule::NONE,
                    &[
                        Might {
                            who: Source,
                            amount: 2,
                            minimum: 0,
                        },
                        Ready(Source),
                    ],
                )]
            }
        }
        184 => {
            const {
                &[rule(
                    FriendlyCardPlayed,
                    &[EnergyAtLeast(7)],
                    false,
                    TargetRule::NONE,
                    &[Ready(Source)],
                )]
            }
        }
        185 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Might {
                        who: Source,
                        amount: 3,
                        minimum: 0,
                    }],
                )]
            }
        }
        186 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[DrawBoth(1)],
                )]
            }
        }
        187 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 1,
                        might: 1,
                        gear: false,
                        opponent: true,
                        location: Default,
                        entry_ready: false,
                    }],
                )]
            }
        }
        188 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    true,
                    TargetRule {
                        another: true,
                        battlefield: true,
                        max_might: Some(3),
                        ..ANY
                    },
                    &[Bounce],
                )]
            }
        }
        189 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule {
                        another: true,
                        battlefield: true,
                        ..ANY
                    },
                    &[Bounce],
                )]
            }
        }
        190 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule {
                        min: 2,
                        max: 2,
                        allegiance: Allegiance::Mixed,
                        another: true,
                        ..ANY
                    },
                    &[Bounce],
                )]
            }
        }
        191 => {
            const {
                &[rule(
                    SelfPlayed,
                    &[],
                    false,
                    TargetRule {
                        allegiance: Allegiance::Enemy,
                        battlefield: true,
                        ..ANY
                    },
                    &[FightSource],
                )]
            }
        }
        192 => const { &[rule(SelfPlayed, &[], false, ANY, &[Stun])] },
        194 => {
            const {
                &[rule(
                    SelfExhausted,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Predict],
                )]
            }
        }
        195 => {
            const {
                &[rule(
                    FriendlyDraw,
                    &[SecondDraw],
                    false,
                    FRIENDLY,
                    &[Might {
                        who: Targets,
                        amount: 2,
                        minimum: 0,
                    }],
                )]
            }
        }
        196 => {
            const {
                &[rule(
                    FriendlyReady,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Might {
                        who: Subject,
                        amount: 1,
                        minimum: 0,
                    }],
                )]
            }
        }
        197 => {
            const {
                &[
                    rule(
                        FriendlyAttack,
                        &[Alone],
                        false,
                        TargetRule::NONE,
                        &[Might {
                            who: Subject,
                            amount: 1,
                            minimum: 0,
                        }],
                    ),
                    rule(
                        FriendlyDefend,
                        &[Alone],
                        false,
                        TargetRule::NONE,
                        &[Might {
                            who: Subject,
                            amount: 1,
                            minimum: 0,
                        }],
                    ),
                ]
            }
        }
        198 => {
            const {
                &[rule(
                    FriendlyDeath,
                    &[Buffed],
                    false,
                    FRIENDLY,
                    &[Buff(Targets)],
                )]
            }
        }
        199 => {
            const {
                &[equipment_rule(
                    SelfDeath,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Draw(1)],
                )]
            }
        }
        200 => {
            const {
                &[equipment_rule(
                    SelfConquer,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Loot {
                        discard: 1,
                        draw: 1,
                    }],
                )]
            }
        }
        201 => {
            const {
                &[equipment_rule(
                    SelfConquer,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 1,
                        might: 0,
                        gear: true,
                        opponent: false,
                        location: Base,
                        entry_ready: false,
                    }],
                )]
            }
        }
        202 => {
            const {
                &[equipment_rule(
                    SelfHold,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 2,
                        might: 0,
                        gear: true,
                        opponent: false,
                        location: Base,
                        entry_ready: false,
                    }],
                )]
            }
        }
        203 => {
            const {
                &[equipment_rule(
                    SelfMoved,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Tokens {
                        count: 1,
                        might: 1,
                        gear: false,
                        opponent: false,
                        location: Here,
                        entry_ready: false,
                    }],
                )]
            }
        }
        204 => {
            const {
                &[equipment_rule(
                    SelfConquer,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Buff(Source)],
                )]
            }
        }
        205 => {
            const {
                &[equipment_rule(
                    SelfMoved,
                    &[ToBattlefield],
                    false,
                    TargetRule::NONE,
                    &[Might {
                        who: Source,
                        amount: 2,
                        minimum: 0,
                    }],
                )]
            }
        }
        206 => {
            const {
                &[
                    equipment_rule(SelfAttack, &[], false, ENEMY_HERE, &[Damage(2)]),
                    equipment_rule(SelfDefend, &[], false, ENEMY_HERE, &[Damage(2)]),
                ]
            }
        }
        207 => {
            const {
                &[equipment_rule(
                    SelfHold,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Points(1)],
                )]
            }
        }
        208 => const { &[rule(SelfHold, &[], false, TargetRule::NONE, &[Points(1)])] },
        209 => {
            const {
                &[rule(
                    SelfHold,
                    &[OtherFriendlyHere(1)],
                    false,
                    TargetRule::NONE,
                    &[Points(1)],
                )]
            }
        }
        210 => {
            const {
                &[rule(
                    SelfHold,
                    &[OtherFriendlyHere(1)],
                    false,
                    TargetRule::NONE,
                    &[Draw(1)],
                )]
            }
        }
        211 => {
            const {
                &[rule(
                    SelfHold,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Buff(AllHere)],
                )]
            }
        }
        212 => {
            const {
                &[rule(
                    FriendlyDeath,
                    &[Other],
                    false,
                    TargetRule::NONE,
                    &[Might {
                        who: Source,
                        amount: 2,
                        minimum: 0,
                    }],
                )]
            }
        }
        213 => {
            const {
                &[rule(
                    EnemyDeath,
                    &[StunnedHere],
                    false,
                    TargetRule::NONE,
                    &[Draw(1)],
                )]
            }
        }
        214 => {
            const {
                &[rule(
                    FriendlyDeath,
                    &[FirstEachTurn],
                    false,
                    TargetRule::NONE,
                    &[Draw(1)],
                )]
            }
        }
        215 => {
            const {
                &[rule(
                    GearActivated,
                    &[],
                    false,
                    TargetRule::NONE,
                    &[Might {
                        who: Source,
                        amount: 1,
                        minimum: 0,
                    }],
                )]
            }
        }
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_event_batch_card_has_an_explicit_nonempty_mapping() {
        for id in 152..=215 {
            assert!(
                !trigger_rules(id).is_empty()
                    || crate::rules::static_abilities(id).iter().any(|a| {
                        matches!(
                            a,
                            crate::rules::StaticAbility::Hunt(_)
                                | crate::rules::StaticAbility::Vision
                        )
                    }),
                "missing trigger metadata for {id}"
            );
            for rule in trigger_rules(id) {
                assert!(rule.targets.min <= rule.targets.max);
                assert!(!rule.effects.is_empty());
            }
        }
        assert!(trigger_rules(151).is_empty());
        assert!(trigger_rules(216).is_empty());
        assert!(trigger_rules(usize::MAX).is_empty());
    }
    #[test]
    fn optional_and_multi_target_triggers_preserve_printed_choices() {
        let wind = &trigger_rules(188)[0];
        assert!(wind.optional);
        assert_eq!((wind.targets.min, wind.targets.max), (1, 1));
        assert!(wind.targets.another && wind.targets.battlefield);
        assert_eq!(wind.targets.max_might, Some(3));
        let beast = &trigger_rules(190)[0];
        assert_eq!((beast.targets.min, beast.targets.max), (2, 2));
        assert_eq!(beast.targets.allegiance, Allegiance::Mixed);
        assert!(beast.targets.another && beast.targets.distinct);
    }
    #[test]
    fn tokens_preserve_owner_and_location_and_hold_conditions_are_explicit() {
        assert_eq!(
            trigger_rules(187)[0].effects,
            &[Effect::Tokens {
                count: 1,
                might: 1,
                gear: false,
                opponent: true,
                location: TokenPlacement::Default,
                entry_ready: false,
            }]
        );
        assert_eq!(
            trigger_rules(178)[0].effects,
            &[Effect::Tokens {
                count: 1,
                might: 2,
                gear: false,
                opponent: false,
                location: TokenPlacement::Here,
                entry_ready: false,
            }]
        );
        assert_eq!(
            trigger_rules(209)[0].conditions,
            &[Condition::OtherFriendlyHere(1)]
        );
        assert_eq!(
            trigger_rules(214)[0].conditions,
            &[Condition::FirstEachTurn]
        );
    }
}
