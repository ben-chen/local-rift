//! Explicit trigger descriptors for the reviewed event-driven card batch.
//! These are interpreted by the runtime, never inferred from printed keywords.
//! Timing and event boundaries follow Core Rules 337, 354-355 and 383.

use crate::rules::{Allegiance, TokenPlacement};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
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
}

/// Conditions are evaluated when the event occurs. They are not effect-side
/// conditions to be rechecked during resolution (CR383.2.a.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetRule {
    pub min: u8,
    pub max: u8,
    pub allegiance: Allegiance,
    pub another: bool,
    pub battlefield: bool,
    pub here: bool,
    pub max_might: Option<u8>,
    pub tag: Option<&'static str>,
    pub distinct: bool,
}
impl TargetRule {
    pub const NONE: Self = Self {
        min: 0,
        max: 0,
        allegiance: Allegiance::None,
        another: false,
        battlefield: false,
        here: false,
        max_might: None,
        tag: None,
        distinct: true,
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
    Tokens {
        count: u8,
        might: u8,
        gear: bool,
        opponent: bool,
        location: TokenPlacement,
    },
    Points(u8),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerRule {
    pub event: Event,
    pub conditions: &'static [Condition],
    pub optional: bool,
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
        event,
        conditions,
        optional,
        targets,
        effects,
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
    match card_id {
        242 => const { &[rule(SelfPlayed, &[], false, TargetRule::NONE, &[Predict])] },
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
        153 => {
            const {
                &[
                    rule(SelfConquer, &[], false, TargetRule::NONE, &[Xp(Fixed(3))]),
                    rule(SelfHold, &[], false, TargetRule::NONE, &[Xp(Fixed(3))]),
                ]
            }
        }
        154 => {
            const {
                &[
                    rule(SelfPlayed, &[], false, TargetRule::NONE, &[Xp(Fixed(2))]),
                    rule(SelfConquer, &[], false, TargetRule::NONE, &[Xp(Fixed(1))]),
                    rule(SelfHold, &[], false, TargetRule::NONE, &[Xp(Fixed(1))]),
                ]
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
        193 => const { &[rule(SelfPlayed, &[], false, TargetRule::NONE, &[Predict])] },
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
        199 => const { &[rule(SelfDeath, &[], false, TargetRule::NONE, &[Draw(1)])] },
        200 => {
            const {
                &[rule(
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
                    }],
                )]
            }
        }
        202 => {
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
                    }],
                )]
            }
        }
        203 => {
            const {
                &[rule(
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
                    }],
                )]
            }
        }
        204 => {
            const {
                &[rule(
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
                &[rule(
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
                    rule(SelfAttack, &[], false, ENEMY_HERE, &[Damage(2)]),
                    rule(SelfDefend, &[], false, ENEMY_HERE, &[Damage(2)]),
                ]
            }
        }
        207 => const { &[rule(SelfHold, &[], false, TargetRule::NONE, &[Points(1)])] },
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
                !trigger_rules(id).is_empty(),
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
                location: TokenPlacement::Default
            }]
        );
        assert_eq!(
            trigger_rules(178)[0].effects,
            &[Effect::Tokens {
                count: 1,
                might: 2,
                gear: false,
                opponent: false,
                location: TokenPlacement::Here
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
