//! Declarative official card behavior for the reviewed runtime catalog.
//!
//! This module describes rules; it does not claim that the game engine executes
//! every described operation. The engine must gate unsupported requirements.
//! An unavailable official operation must never be replaced by an invented effect.

use crate::cards::Domain;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterKind {
    Abandon,
    Defy,
    NotSoFast,
    HardBargain,
    Unconditional,
}

/// Amounts calculated when the instruction resolves, after responses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellMight {
    Empowered { base: i8, empowered: i8 },
    EnemiesHere { per_enemy: i8 },
    OwnExhausted { per_unit: i8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellDamage {
    OwnFacedown { base: u8, conditional: u8 },
    Attacking { base: u8, conditional: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfficialEffect {
    /// No immediate spell/on-play operation. Check triggers and requirements.
    None,
    Counter(CounterKind),
    /// `targets` is the maximum. See targeting() for repeated/distinct targets.
    Damage {
        amount: u8,
        targets: u8,
        battlefield_only: bool,
        draw: u8,
        banish_on_death: bool,
    },
    ModifyMight {
        amount: i8,
        minimum: u8,
        draw: u8,
        alone_bonus: i8,
    },
    /// +2 to one unit and -2 to another distinct unit for this turn.
    DualMight,
    Bounce {
        max_might: Option<u8>,
        battlefield_only: bool,
    },
    /// Return one friendly and one enemy unit to their owners' hands.
    BounceBoth,
    /// This is a MOVE to base, so move triggers fire; it is not a recall.
    MoveFriendlyToBase {
        up_to: u8,
    },
    MoveEnemy,
    MoveFriendlyAndReady,
    /// Swap two units' might this turn; both must share a battlefield.
    SwapMight,
    /// Friendly and enemy units deal their might to each other. Optional +2
    /// friendly might costs one additional Body power, before the duel.
    Fight {
        optional_buff: u8,
    },
    /// Give the first (friendly) target temporary Might, then fight the enemy.
    EmpoweredFight {
        amount: i8,
    },
    /// Damage the chosen battlefield unit, then play an exhausted Gold token.
    DamageAndGold {
        amount: u8,
    },
    ContextualMight(SpellMight),
    ContextualDamage(SpellDamage),
    /// Separate draw instructions: the fixed amount, then controlled fields.
    DrawForBattlefields {
        base: u8,
    },
    DrawForMighty,
    /// Reveal opponent's hand, choose a non-unit, recycle it.
    DisruptHand,
    Draw(u8),
    /// Discard first, then draw. This is mandatory, not merely net card gain.
    Loot {
        discard: u8,
        draw: u8,
    },
    Tokens {
        count: u8,
        might: u8,
    },
    /// Optionally kill a gear, then draw one even if no gear was chosen.
    DestroyGearAndDraw,
    /// Schedule a full extra turn, then banish this spell.
    ExtraTurn,
    /// First Mate: another unit of either player, never this newly played unit.
    ReadyAnother,
    /// Pit Rookie: another friendly unit gets its persistent +1 buff only if
    /// it does not already have a buff. This is not a temporary might increase.
    BuffAnother,
    /// Give the source unit its standard persistent +1 buff, if it has none.
    BuffSelf,
    Burn(u8),
    WeakenEnemies {
        amount: u8,
        minimum: u8,
    },
    SilenceOpponentsForTurn,
    /// Fizz: optional spell from own trash, energy <=3; ignore energy only,
    /// still pay power; recycle the spell after it is played.
    ReplayCheapSpell,
    /// Lightning Rush: look at top3, MAY choose one to draw, trash the rest.
    ChooseTopThreeBurnRest,
    /// Give a gear Temporary; it dies at its controller's next beginning
    /// before scoring, not immediately.
    TemporaryGear,
    Equipment,
    /// Persistent resource gear; exhaust to add the printed energy or power.
    PowerGear,
    /// Zhonya: persistent next-friendly-death replacement; kill this instead,
    /// recall the unit exhausted. See official text for Hidden.
    PreventNextDeathRecall,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Allegiance {
    None,
    Any,
    Friendly,
    Enemy,
    Mixed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Targeting {
    pub allegiance: Allegiance,
    /// If true, multi-target choices must name different units.
    pub distinct: bool,
    pub same_location: bool,
}

/// The immediate official operation. Unit triggers are also described below;
/// do not execute a Played trigger twice if this operation already represents it.
pub fn effect(id: usize) -> OfficialEffect {
    use OfficialEffect::*;
    match id {
        // These permanents have reviewed activated abilities in activated.rs,
        // not automatic effects when the card is played.
        230..=245 => None,
        0 | 2 | 13 | 17 | 18 | 20 | 22 | 24 | 25 | 30 | 31 | 34 | 43 | 45 | 46 | 47 | 49 | 54
        | 56 | 59 | 60 | 68 | 69 | 70 | 71 | 72 | 73 => None,
        1 => Counter(CounterKind::Abandon),
        3 | 10 | 38 | 58 => Equipment,
        4 => MoveEnemy,
        5 => DualMight,
        6 => Counter(CounterKind::Defy),
        7 => ModifyMight {
            amount: 2,
            minimum: 0,
            draw: 1,
            alone_bonus: 0,
        },
        8 => ModifyMight {
            amount: 1,
            minimum: 0,
            draw: 0,
            alone_bonus: 1,
        },
        9 => MoveFriendlyToBase { up_to: 2 },
        11 => Bounce {
            max_might: Some(3),
            battlefield_only: true,
        },
        12 => Counter(CounterKind::NotSoFast),
        14 => MoveFriendlyAndReady,
        15 | 57 => Draw(1),
        16 => BounceBoth,
        19 => ModifyMight {
            amount: 1,
            minimum: 0,
            draw: 0,
            alone_bonus: 0,
        },
        21 => PreventNextDeathRecall,
        23 => ReadyAnother,
        26 => ModifyMight {
            amount: 6,
            minimum: 0,
            draw: 0,
            alone_bonus: 0,
        },
        27 => BuffAnother,
        28 => ModifyMight {
            amount: 5,
            minimum: 0,
            draw: 0,
            alone_bonus: 0,
        },
        29 => Fight { optional_buff: 2 },
        32 => DisruptHand,
        33 => Burn(2),
        35 => Loot {
            discard: 1,
            draw: 2,
        },
        36 => ReplayCheapSpell,
        37 => Counter(CounterKind::HardBargain),
        39 => ChooseTopThreeBurnRest,
        40 => None,
        41 => DestroyGearAndDraw,
        44 => SwapMight,
        42 => PowerGear,
        48 => Tokens { count: 2, might: 1 },
        50 => Damage {
            amount: 1,
            targets: 3,
            battlefield_only: false,
            draw: 0,
            banish_on_death: false,
        },
        51 => SilenceOpponentsForTurn,
        52 => ModifyMight {
            amount: -4,
            minimum: 0,
            draw: 0,
            alone_bonus: 0,
        },
        53 => Damage {
            amount: 3,
            targets: 2,
            battlefield_only: false,
            draw: 0,
            banish_on_death: false,
        },
        55 => Damage {
            amount: 3,
            targets: 1,
            battlefield_only: true,
            draw: 0,
            banish_on_death: false,
        },
        61 => Damage {
            amount: 6,
            targets: 2,
            battlefield_only: false,
            draw: 0,
            banish_on_death: false,
        },
        62 => Damage {
            amount: 3,
            targets: 1,
            battlefield_only: true,
            draw: 0,
            banish_on_death: true,
        },
        63 => ModifyMight {
            amount: -1,
            minimum: 1,
            draw: 1,
            alone_bonus: 0,
        },
        64 => WeakenEnemies {
            amount: 3,
            minimum: 1,
        },
        65 => ExtraTurn,
        66 => TemporaryGear,
        67 => Damage {
            amount: 4,
            targets: 1,
            battlefield_only: true,
            draw: 1,
            banish_on_death: false,
        },
        74 => Damage {
            amount: 2,
            targets: 1,
            battlefield_only: true,
            draw: 0,
            banish_on_death: false,
        },
        75 => Damage {
            amount: 6,
            targets: 1,
            battlefield_only: true,
            draw: 0,
            banish_on_death: false,
        },
        76 => ModifyMight {
            amount: 7,
            minimum: 0,
            draw: 0,
            alone_bonus: 0,
        },
        77 => Draw(4),
        78 => Draw(3),
        79 => ModifyMight {
            amount: -10,
            minimum: 0,
            draw: 0,
            alone_bonus: 0,
        },
        80 => Bounce {
            max_might: Option::None,
            battlefield_only: true,
        },
        81 => ModifyMight {
            amount: -4,
            minimum: 1,
            draw: 0,
            alone_bonus: 0,
        },
        82 | 83 => Fight { optional_buff: 0 },
        84 => Loot {
            discard: 1,
            draw: 2,
        },
        85 | 99 => ModifyMight {
            amount: 2,
            minimum: 0,
            draw: 0,
            alone_bonus: 0,
        },
        86 => Counter(CounterKind::Unconditional),
        87 => Tokens { count: 4, might: 1 },
        88..=98 => None,
        100 => Loot {
            discard: 2,
            draw: 2,
        },
        101 => BuffSelf,
        102 => Tokens { count: 2, might: 1 },
        103..=116 | 131 | 149 | 150 => None,
        117..=124 => Equipment,
        125..=130 => PowerGear,
        132 => Draw(2),
        133 | 134 | 139 => Draw(1),
        135 | 143 => ModifyMight {
            amount: 2,
            minimum: 0,
            draw: 0,
            alone_bonus: 0,
        },
        136 => ModifyMight {
            amount: -2,
            minimum: 0,
            draw: 0,
            alone_bonus: 0,
        },
        137 => ModifyMight {
            amount: 1,
            minimum: 0,
            draw: 0,
            alone_bonus: 0,
        },
        138 => Tokens { count: 1, might: 3 },
        140 => Loot {
            discard: 1,
            draw: 1,
        },
        141 => ModifyMight {
            amount: 3,
            minimum: 0,
            draw: 0,
            alone_bonus: 0,
        },
        142 => ModifyMight {
            amount: 8,
            minimum: 0,
            draw: 0,
            alone_bonus: 0,
        },
        144 => Damage {
            amount: 6,
            targets: 1,
            battlefield_only: true,
            draw: 0,
            banish_on_death: false,
        },
        145 => ModifyMight {
            amount: -3,
            minimum: 0,
            draw: 0,
            alone_bonus: 0,
        },
        146 => ModifyMight {
            amount: -2,
            minimum: 1,
            draw: 0,
            alone_bonus: 0,
        },
        147 | 148 => Tokens { count: 1, might: 1 },
        151 => Tokens { count: 4, might: 0 },
        152 | 153 | 154 | 155 | 156 | 157 | 158 | 159 | 160 | 161 | 162 | 163 | 164 | 165 | 166
        | 167 | 168 | 169 | 170 | 171 | 172 | 173 | 174 | 175 | 176 | 177 | 178 | 179 | 180
        | 181 | 182 | 183 | 184 | 185 | 186 | 187 | 188 | 189 | 190 | 191 | 192 | 193 | 194
        | 195 | 196 | 197 | 198 | 208 | 209 | 210 | 211 | 212 | 213 | 214 | 215 => None,
        199..=207 => Equipment,
        216 => ReadyAnother,
        217 => Fight { optional_buff: 0 },
        218 => EmpoweredFight { amount: 3 },
        219 => DamageAndGold { amount: 3 },
        220 => Tokens { count: 1, might: 2 },
        221 => ContextualMight(SpellMight::Empowered {
            base: 2,
            empowered: 4,
        }),
        222 => ContextualDamage(SpellDamage::OwnFacedown {
            base: 2,
            conditional: 4,
        }),
        223 => ContextualDamage(SpellDamage::Attacking {
            base: 2,
            conditional: 4,
        }),
        224 => DrawForBattlefields { base: 1 },
        225 => DrawForMighty,
        226 => Damage {
            amount: 5,
            targets: 1,
            battlefield_only: true,
            draw: 0,
            banish_on_death: false,
        },
        227 => Damage {
            amount: 4,
            targets: 1,
            battlefield_only: true,
            draw: 0,
            banish_on_death: false,
        },
        228 => ContextualMight(SpellMight::EnemiesHere { per_enemy: 2 }),
        229 => ContextualMight(SpellMight::OwnExhausted { per_unit: -1 }),
        _ => Unsupported,
    }
}

pub fn targeting(id: usize) -> Targeting {
    use Allegiance::*;
    let allegiance = match id {
        4 | 32 | 144 | 145 => Enemy,
        8 | 9 | 14 | 19 | 27 | 85 | 137 | 228 => Friendly,
        16 | 29 | 82 | 217 | 218 => Mixed,
        5 | 7 | 11 | 23 | 26 | 28 | 41 | 44 | 50 | 52 | 53 | 55 | 61 | 62 | 63 | 66 | 67 | 74
        | 75 | 76 | 79 | 80 | 81 | 83 | 99 | 135 | 136 | 141 | 142 | 143 | 146 | 216 | 219
        | 221 | 222 | 223 | 226 | 227 | 229 => Any,
        _ => None,
    };
    Targeting {
        allegiance,
        distinct: matches!(
            id,
            5 | 9 | 16 | 44 | 50 | 61 | 82 | 83 | 85 | 137 | 217 | 218
        ),
        same_location: matches!(id, 44 | 50),
    }
}

/// Number of target choices accepted by the card: (maximum, required).
/// Location/allegiance restrictions remain in `targeting` and engine validation.
pub fn target_bounds(id: usize) -> (usize, usize) {
    use OfficialEffect::*;
    if matches!(id, 85 | 137) {
        return (2, 2);
    }
    match effect(id) {
        Damage { targets, .. } => (
            targets as usize,
            if matches!(id, 50 | 61) {
                0
            } else {
                targets as usize
            },
        ),
        DualMight | BounceBoth | SwapMight | Fight { .. } | EmpoweredFight { .. } => (2, 2),
        MoveFriendlyToBase { up_to } => (up_to as usize, 0),
        ModifyMight { .. }
        | ContextualMight(_)
        | ContextualDamage(_)
        | DamageAndGold { .. }
        | Bounce { .. }
        | MoveEnemy
        | MoveFriendlyAndReady
        | ReadyAnother
        | BuffAnother => (1, 1),
        _ => (0, 0),
    }
}

/// These permanents choose their play-trigger target after entering the board.
pub fn on_play_requires_target(id: usize) -> bool {
    matches!(id, 23 | 27 | 99 | 141..=146)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Played,
    Chosen,
    Readied,
    Moved,
    MovedToBattlefield,
    Conquered,
    Held,
    Died,
    FriendlySpellPlayed,
    EnemyUnitPlayed,
    EnemyUnitDied,
    Empowered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerEffect {
    Draw(u8),
    Loot { discard: u8, draw: u8 },
    BuffSelfForTurn(i8),
    GiveUnitMightForTurn(i8),
    DealToUnit(u8),
    BuffSelf,
    ReadyAnother,
    BuffAnother,
    Burn(u8),
    GainXp(u8),
    Tokens { count: u8, might: u8 },
    BirdWithDeflect,
    GoldExhausted,
    RevealEnemyHandAndHiddenGainXp,
    GiveTrashSpellFlow,
    StunAndPreventMovement,
    OptionalBurnForMight,
    ReviveCheapUnit,
    OptionalPlayTrashUnit,
    OptionalSwapLocations,
    OptionalEmpowerHereForTurn,
    SilenceOpponentsForTurn,
    WeakenEnemies { amount: u8, minimum: u8 },
    ReplayCheapSpell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    Always,
    /// No other friendly units were there immediately before this died.
    DiedAlone,
    ThisAtBattlefield,
    /// Additionally limited to once per turn for this source.
    ThisAtBattlefieldOncePerTurn,
    PlayedFromFacedown,
    /// Another card was already played this turn when this card was played.
    Legion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trigger {
    pub event: Event,
    pub condition: Condition,
    pub effect: TriggerEffect,
}

macro_rules! trigger {
    ($event:expr, $effect:expr $(,)?) => {
        Trigger {
            event: $event,
            condition: Condition::Always,
            effect: $effect,
        }
    };
}

pub fn triggers(id: usize) -> &'static [Trigger] {
    use Event::*;
    match id {
        0 => &[
            trigger!(Chosen, TriggerEffect::BuffSelfForTurn(1)),
            trigger!(Readied, TriggerEffect::BuffSelfForTurn(1)),
        ],
        2 => &[trigger!(
            MovedToBattlefield,
            TriggerEffect::BuffSelfForTurn(2),
        )],
        13 => &[Trigger {
            event: EnemyUnitDied,
            condition: Condition::ThisAtBattlefieldOncePerTurn,
            effect: TriggerEffect::GoldExhausted,
        }],
        15 => &[
            trigger!(Played, TriggerEffect::Draw(1)),
            trigger!(Died, TriggerEffect::RevealEnemyHandAndHiddenGainXp),
        ],
        17 => &[trigger!(Moved, TriggerEffect::Draw(1))],
        18 => &[trigger!(Played, TriggerEffect::OptionalSwapLocations)],
        20 => &[Trigger {
            event: EnemyUnitPlayed,
            condition: Condition::ThisAtBattlefield,
            effect: TriggerEffect::StunAndPreventMovement,
        }],
        22 => &[
            trigger!(Conquered, TriggerEffect::GainXp(2)),
            trigger!(Held, TriggerEffect::GainXp(2)),
        ],
        23 => &[trigger!(Played, TriggerEffect::ReadyAnother)],
        25 => &[Trigger {
            event: Died,
            condition: Condition::DiedAlone,
            effect: TriggerEffect::Draw(1),
        }],
        27 => &[trigger!(Played, TriggerEffect::BuffAnother)],
        33 => &[
            trigger!(Played, TriggerEffect::Burn(2)),
            trigger!(Conquered, TriggerEffect::GiveTrashSpellFlow),
        ],
        34 => &[trigger!(Died, TriggerEffect::BirdWithDeflect)],
        35 => &[trigger!(
            Played,
            TriggerEffect::Loot {
                discard: 1,
                draw: 2,
            },
        )],
        36 => &[trigger!(Played, TriggerEffect::ReplayCheapSpell)],
        38 => &[
            trigger!(Conquered, TriggerEffect::OptionalPlayTrashUnit),
            trigger!(Held, TriggerEffect::OptionalPlayTrashUnit),
        ],
        43 => &[trigger!(Moved, TriggerEffect::OptionalBurnForMight)],
        45 => &[trigger!(Empowered, TriggerEffect::ReviveCheapUnit)],
        46 => &[Trigger {
            event: Played,
            condition: Condition::PlayedFromFacedown,
            effect: TriggerEffect::OptionalEmpowerHereForTurn,
        }],
        47 => &[trigger!(
            Moved,
            TriggerEffect::Loot {
                discard: 1,
                draw: 1,
            },
        )],
        49 => &[trigger!(Conquered, TriggerEffect::Draw(1))],
        51 => &[trigger!(Played, TriggerEffect::SilenceOpponentsForTurn)],
        54 => &[trigger!(Died, TriggerEffect::Tokens { count: 2, might: 3 })],
        57 => &[trigger!(Played, TriggerEffect::Draw(1))],
        60 => &[trigger!(
            FriendlySpellPlayed,
            TriggerEffect::BuffSelfForTurn(1),
        )],
        64 => &[trigger!(
            Played,
            TriggerEffect::WeakenEnemies {
                amount: 3,
                minimum: 1,
            },
        )],
        68 => &[trigger!(Died, TriggerEffect::Draw(1))],
        99 => &[Trigger {
            event: Played,
            condition: Condition::Legion,
            effect: TriggerEffect::GiveUnitMightForTurn(2),
        }],
        100 => &[Trigger {
            event: Played,
            condition: Condition::Legion,
            effect: TriggerEffect::Loot {
                discard: 2,
                draw: 2,
            },
        }],
        101 => &[Trigger {
            event: Played,
            condition: Condition::Legion,
            effect: TriggerEffect::BuffSelf,
        }],
        102 => &[Trigger {
            event: Played,
            condition: Condition::Legion,
            effect: TriggerEffect::Tokens { count: 2, might: 1 },
        }],
        139 => &[trigger!(Played, TriggerEffect::Draw(1))],
        140 => &[trigger!(
            Played,
            TriggerEffect::Loot {
                discard: 1,
                draw: 1
            }
        )],
        141 => &[trigger!(Played, TriggerEffect::GiveUnitMightForTurn(3))],
        142 => &[trigger!(Played, TriggerEffect::GiveUnitMightForTurn(8))],
        143 => &[trigger!(Played, TriggerEffect::GiveUnitMightForTurn(2))],
        144 => &[trigger!(Played, TriggerEffect::DealToUnit(6))],
        145 => &[trigger!(Played, TriggerEffect::GiveUnitMightForTurn(-3))],
        146 => &[trigger!(Played, TriggerEffect::GiveUnitMightForTurn(-2))],
        147 | 148 => &[trigger!(
            Played,
            TriggerEffect::Tokens { count: 1, might: 1 }
        )],
        149 => &[trigger!(Died, TriggerEffect::GoldExhausted)],
        150 => &[trigger!(Died, TriggerEffect::BirdWithDeflect)],
        151 => &[trigger!(
            Played,
            TriggerEffect::Tokens { count: 4, might: 0 }
        )],
        _ => &[],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mechanic {
    Chain,
    Counterspell,
    Predict,
    OptionalCosts,
    MultipleTargets,
    Move,
    Bounce,
    NegativeMight,
    Gear,
    Equip,
    DeathReplacement,
    Protection,
    Deflect,
    Hidden,
    Backline,
    Tank,
    Shield,
    Stun,
    Gold,
    DeathTriggers,
    MoveTriggers,
    ChooseReadyTriggers,
    ConquerHoldTriggers,
    Xp,
    Empower,
    Fight,
    HandInspection,
    Burn,
    Flow,
    TrashPlay,
    CostReduction,
    ResourceAbilities,
    ActivatedAbilities,
    Tokens,
    Accelerate,
    Ambush,
    Assault,
    ExtraTurn,
    Temporary,
    SpellTriggers,
    BanishOnDeath,
    Legion,
}

/// Additional systems that must work before the complete card can be advertised
/// as accurate. Presence here does not mean the engine has implemented it.
pub fn requirements(id: usize) -> &'static [Mechanic] {
    use Mechanic::*;
    match id {
        240 => &[Gear, Chain, ActivatedAbilities],
        241 => &[Gear, Chain, ActivatedAbilities, Tokens],
        242 => &[Gear, Chain, ActivatedAbilities, Predict],
        243 => &[Gear, Chain, ActivatedAbilities, Empower],
        244 | 245 => &[Gear, ActivatedAbilities, ResourceAbilities],
        230 | 232 | 234 => &[Gear, Chain, ActivatedAbilities],
        231 => &[Gear, Chain, ActivatedAbilities, NegativeMight],
        233 | 237 | 239 => &[Chain, ActivatedAbilities],
        235 => &[Gear, Chain, ActivatedAbilities, Tokens],
        236 => &[ActivatedAbilities, ResourceAbilities],
        238 => &[Chain, ActivatedAbilities, Backline],
        152..=198 | 208..=215 => &[Chain],
        199..=207 => &[Gear, Equip, Chain],
        216 | 217 | 220 => &[Chain, OptionalCosts],
        218 => &[Chain, Fight, MultipleTargets],
        219 => &[Chain, Hidden, Tokens],
        221 => &[Chain, Empower],
        222 => &[Chain, Hidden],
        223 => &[Chain, Hidden],
        224 | 225 | 228 => &[Chain],
        226 | 227 => &[Chain, CostReduction],
        229 => &[Chain, NegativeMight],
        0 => &[Deflect, ChooseReadyTriggers],
        1 => &[Chain, Counterspell, Predict, Bounce],
        2 => &[Protection, MoveTriggers],
        3 => &[Gear, Equip, Move],
        4 => &[Move],
        5 => &[Chain, MultipleTargets, NegativeMight],
        6 | 12 => &[Chain, Counterspell],
        7 | 8 => &[Chain],
        9 => &[Chain, MultipleTargets, Move],
        10 => &[Gear, Equip, DeathReplacement],
        11 => &[Chain, Bounce],
        13 => &[Hidden, Backline, DeathTriggers, Gold],
        14 => &[Chain, Move],
        15 => &[DeathTriggers, HandInspection, Hidden, Xp],
        16 => &[Chain, MultipleTargets, Bounce],
        17 | 47 => &[MoveTriggers],
        18 => &[Hidden, Move, MultipleTargets],
        19 => &[Protection, Flow],
        20 => &[Deflect, Stun],
        21 => &[Gear, Hidden, DeathReplacement],
        22 => &[Xp, ConquerHoldTriggers, Deflect, Move],
        23 | 27 => &[],
        24 => &[Empower, OptionalCosts],
        25 => &[DeathTriggers],
        26 => &[Flow],
        28 => &[Chain],
        29 => &[Fight, OptionalCosts, MultipleTargets],
        30 => &[Ambush],
        31 => &[Protection],
        32 => &[HandInspection],
        33 => &[Burn, ConquerHoldTriggers, Flow],
        34 => &[DeathTriggers, Tokens, Deflect],
        35 => &[CostReduction, OptionalCosts],
        36 => &[TrashPlay],
        37 => &[Chain, Counterspell, OptionalCosts],
        38 => &[Gear, Equip, TrashPlay, ConquerHoldTriggers],
        39 => &[Burn, Flow],
        40 => &[CostReduction],
        41 => &[Chain, Gear],
        43 => &[MoveTriggers, Burn],
        44 => &[Chain, Hidden, MultipleTargets, NegativeMight],
        45 => &[Empower, TrashPlay],
        46 => &[Hidden, Empower],
        42 => &[Gear, ResourceAbilities],
        48 => &[Tokens, Flow],
        49 => &[Accelerate, ConquerHoldTriggers],
        50 => &[Chain, MultipleTargets, OptionalCosts],
        51 => &[Chain],
        52 => &[Chain, NegativeMight, Predict],
        53 | 61 => &[MultipleTargets],
        54 => &[DeathTriggers, Tokens],
        55 | 67 => &[Chain],
        56 => &[Ambush, Assault],
        57 => &[Tank],
        58 => &[Gear, Equip, Chain],
        59 => &[CostReduction],
        60 => &[SpellTriggers],
        62 => &[Chain, BanishOnDeath],
        63 => &[Chain, NegativeMight],
        64 => &[Accelerate, NegativeMight],
        65 => &[ExtraTurn],
        66 => &[Gear, Temporary],
        68 => &[DeathTriggers],
        74 | 75 | 76 | 78 => &[Chain],
        79 => &[Chain, NegativeMight],
        80 => &[Chain, Bounce],
        81 => &[Chain, NegativeMight],
        82 => &[Chain, Fight, MultipleTargets],
        83 => &[Fight, MultipleTargets],
        84 => &[Chain],
        85 => &[Chain, MultipleTargets],
        86 => &[Chain, Counterspell],
        87 => &[Chain, Tokens],
        88..=90 => &[Assault],
        91 => &[Tank],
        92 | 93 => &[Deflect],
        94 => &[Ambush],
        95 | 96 => &[Accelerate, OptionalCosts],
        97 => &[Accelerate, OptionalCosts, Assault],
        98 => &[Accelerate, OptionalCosts, Assault, Deflect, Move],
        99..=101 => &[Legion],
        102 => &[Legion, Tokens],
        103 | 106 | 114 => &[Move],
        104 | 111 => &[Assault, Shield],
        105 => &[Move],
        107 | 109 | 112 | 113 => &[Shield, Tank],
        108 => &[Hidden],
        110 | 115 | 116 => &[Shield],
        117 | 118 => &[Gear, Equip],
        119 => &[Gear, Equip, Chain],
        120 => &[Gear, Equip, Chain, Shield],
        121 => &[Gear, Equip, Tank],
        122 => &[Gear, Equip, Deflect],
        123 => &[Gear, Equip, Assault],
        124 => &[Gear, Equip, Shield],
        125..=130 => &[Gear, ResourceAbilities],
        131 => &[CostReduction],
        132 => &[Hidden, Chain],
        133 | 135 => &[Chain, OptionalCosts],
        136 => &[Chain, OptionalCosts, NegativeMight],
        137 => &[Chain, OptionalCosts, MultipleTargets],
        134 => &[Flow],
        138 => &[Flow, Tokens],
        139..=142 | 144 => &[Chain],
        143 => &[Chain, Accelerate, OptionalCosts],
        145 => &[Chain, Accelerate, OptionalCosts, NegativeMight],
        146 => &[Chain, Hidden, NegativeMight],
        147 | 148 | 151 => &[Chain, Tokens],
        149 | 150 => &[DeathTriggers, Tokens],
        _ => &[],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupportStatus {
    /// Immediate metadata has no additional system requirement. The engine
    /// still must implement the operation before permitting the card.
    CoreEffectDescribed,
    /// Not a claim of playable support: all listed systems need implementation.
    AdditionalSystemsRequired(&'static [Mechanic]),
    UnknownCard,
}

pub fn support_status(id: usize) -> SupportStatus {
    if matches!(effect(id), OfficialEffect::Unsupported) {
        return SupportStatus::UnknownCard;
    }
    let requirements = requirements(id);
    if requirements.is_empty() {
        SupportStatus::CoreEffectDescribed
    } else {
        SupportStatus::AdditionalSystemsRequired(requirements)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquipmentAbility {
    Ganking,
    GuardianAngel,
    LastRites,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EquipmentRule {
    pub might: u8,
    pub equip_energy: u8,
    pub equip_domain: Domain,
    pub equip_power: u8,
    pub recycle_trash: u8,
    pub quick_draw: bool,
    pub ability: EquipmentAbility,
    pub grants: &'static [StaticAbility],
}

pub fn equipment(id: usize) -> Option<EquipmentRule> {
    let (might, equip_domain, recycle_trash, quick_draw, ability) = match id {
        3 => (2, Domain::Chaos, 0, false, EquipmentAbility::Ganking),
        10 => (1, Domain::Calm, 0, false, EquipmentAbility::GuardianAngel),
        38 => (2, Domain::Chaos, 2, false, EquipmentAbility::LastRites),
        58 => (2, Domain::Fury, 0, true, EquipmentAbility::None),
        117 => (3, Domain::Order, 0, false, EquipmentAbility::None),
        118 => (2, Domain::Body, 0, false, EquipmentAbility::None),
        119 => (3, Domain::Calm, 0, true, EquipmentAbility::None),
        120 => (0, Domain::Mind, 0, true, EquipmentAbility::None),
        121 => (1, Domain::Calm, 0, false, EquipmentAbility::None),
        122 => (1, Domain::Body, 0, false, EquipmentAbility::None),
        123 => (0, Domain::Fury, 0, false, EquipmentAbility::None),
        124 => (0, Domain::Order, 0, false, EquipmentAbility::None),
        199 => (1, Domain::Order, 0, false, EquipmentAbility::None),
        200 => (1, Domain::Chaos, 0, false, EquipmentAbility::None),
        201 => (1, Domain::Chaos, 0, false, EquipmentAbility::None),
        202 => (2, Domain::Mind, 0, false, EquipmentAbility::None),
        203 => (0, Domain::Order, 0, false, EquipmentAbility::None),
        204 => (1, Domain::Body, 0, false, EquipmentAbility::None),
        205 => (1, Domain::Fury, 0, false, EquipmentAbility::None),
        206 => (0, Domain::Fury, 0, false, EquipmentAbility::None),
        207 => (2, Domain::Body, 0, false, EquipmentAbility::None),
        _ => return None,
    };
    Some(EquipmentRule {
        might,
        equip_energy: u8::from(id == 124),
        equip_domain,
        equip_power: u8::from(id != 124),
        recycle_trash,
        quick_draw,
        ability,
        grants: match id {
            3 => &[StaticAbility::Ganking],
            120 | 124 => &[StaticAbility::Shield(2)],
            121 => &[StaticAbility::Tank],
            122 => &[StaticAbility::Deflect(1)],
            123 => &[StaticAbility::Assault(2)],
            _ => &[],
        },
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceGear {
    pub energy: u8,
    pub power: u8,
    pub domain: Option<Domain>,
}

/// Unattached printed gear abilities. Gold tokens are handled by token identity.
pub fn resource_gear(id: usize) -> Option<ResourceGear> {
    let domain = match id {
        42 => Domain::Chaos,
        125 => Domain::Calm,
        126 => Domain::Mind,
        127 => Domain::Fury,
        128 => Domain::Body,
        129 => Domain::Order,
        130 => {
            return Some(ResourceGear {
                energy: 1,
                power: 0,
                domain: None,
            });
        }
        _ => return None,
    };
    Some(ResourceGear {
        energy: 0,
        power: 1,
        domain: Some(domain),
    })
}

/// Passive and activated abilities that are not represented by the immediate
/// operation. These must not be silently discarded for a "base-stats" unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticAbility {
    HighestMightEnergyDiscount,
    EmpoweredEnergyDiscount(u8),
    Deflect(u8),
    Shield(u8),
    EntersReady,
    /// Intrinsic entry state, not an Exhaust action or event.
    EntersExhausted,
    PerCardEnergyDiscount {
        amount: u8,
        minimum: u8,
    },
    Ganking,
    LegionOnPlay,
    UnchoosableByEnemies {
        except_in_combat: bool,
    },
    Hidden,
    Backline,
    Tank,
    Ambush,
    /// Rengar can also be played where enemies are present without allies.
    MayDeployToEnemyBattlefield,
    Assault(u8),
    Accelerate {
        energy: u8,
        power: u8,
        domain: Domain,
    },
    LevelSixDeflectAndGanking,
    Empower {
        energy: u8,
        power: u8,
        domain: Domain,
        either_cost: bool,
    },
    EmpoweredMight(i8),
    /// Cost is reduced by one energy per card currently in the owner's trash.
    TrashEnergyDiscount,
    /// Noxus Hopeful: two energy less after another card has been played.
    LegionEnergyDiscount(u8),
    /// Ezreal: optional additional costs cost one energy OR one power less.
    OptionalAdditionalCostDiscount,
    QuickDraw,
}

pub fn static_abilities(id: usize) -> &'static [StaticAbility] {
    use StaticAbility::*;
    match id {
        232 | 243 => &[EntersExhausted],
        233 => &[EntersReady],
        // Caitlyn spells out the exact Backline assignment restriction.
        238 => &[Backline],
        172 => &[Assault(2)],
        173 => &[Ambush],
        180 => &[Accelerate {
            energy: 1,
            power: 1,
            domain: Domain::Order,
        }],
        185 | 188 | 219 | 223 => &[Hidden],
        187 | 213 => &[Deflect(1)],
        209 => &[Shield(1)],
        211 => &[Backline],
        226 => &[HighestMightEnergyDiscount],
        227 => &[EmpoweredEnergyDiscount(2)],

        0 | 20 => &[Deflect(1)],
        2 => &[UnchoosableByEnemies {
            except_in_combat: true,
        }],
        13 => &[Hidden, Backline],
        18 | 21 | 44 | 46 => &[Hidden],
        22 => &[LevelSixDeflectAndGanking],
        24 => &[
            Empower {
                energy: 1,
                power: 1,
                domain: Domain::Body,
                either_cost: true,
            },
            EmpoweredMight(1),
        ],
        30 => &[Ambush, MayDeployToEnemyBattlefield],
        31 => &[UnchoosableByEnemies {
            except_in_combat: false,
        }],
        35 => &[OptionalAdditionalCostDiscount],
        40 => &[TrashEnergyDiscount],
        45 => &[Empower {
            energy: 2,
            power: 1,
            domain: Domain::Chaos,
            either_cost: false,
        }],
        49 => &[Accelerate {
            energy: 1,
            power: 1,
            domain: Domain::Fury,
        }],
        56 => &[Ambush, Assault(2)],
        57 => &[Tank],
        58 => &[QuickDraw],
        59 => &[LegionEnergyDiscount(2)],
        64 => &[Accelerate {
            energy: 1,
            power: 1,
            domain: Domain::Mind,
        }],
        88 | 90 => &[Assault(1)],
        89 => &[Assault(2)],
        91 => &[Tank],
        92 | 93 => &[Deflect(1)],
        94 => &[Ambush],
        95 | 96 => &[Accelerate {
            energy: 1,
            power: 1,
            domain: Domain::Fury,
        }],
        97 => &[
            Accelerate {
                energy: 1,
                power: 1,
                domain: Domain::Fury,
            },
            Assault(4),
        ],
        98 => &[
            Accelerate {
                energy: 1,
                power: 1,
                domain: Domain::Fury,
            },
            Assault(2),
            Deflect(1),
            Ganking,
        ],
        99..=102 => &[LegionOnPlay],
        103 | 114 => &[EntersReady],
        104 | 111 => &[Assault(2), Shield(2)],
        105 => &[Ganking],
        106 => &[Ganking, EntersReady],
        107 | 109 | 113 => &[Shield(2), Tank],
        108 | 132 | 146 => &[Hidden],
        110 | 116 => &[Shield(1)],
        112 => &[Shield(1), Tank],
        115 => &[EntersReady, Shield(3)],
        119 | 120 => &[QuickDraw],
        131 => &[PerCardEnergyDiscount {
            amount: 1,
            minimum: 1,
        }],
        143 => &[Accelerate {
            energy: 1,
            power: 1,
            domain: Domain::Fury,
        }],
        145 => &[Accelerate {
            energy: 1,
            power: 1,
            domain: Domain::Mind,
        }],
        _ => &[],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccelerateCost {
    pub energy: u8,
    pub power: u8,
    pub domain: Domain,
}

pub fn accelerate_cost(id: usize) -> Option<AccelerateCost> {
    static_abilities(id)
        .iter()
        .find_map(|ability| match *ability {
            StaticAbility::Accelerate {
                energy,
                power,
                domain,
            } => Some(AccelerateCost {
                energy,
                power,
                domain,
            }),
            _ => None,
        })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Followup {
    /// Look at the top card; its owner may recycle it.
    Predict,
    /// The chosen friendly unit cannot be chosen by enemy spells/abilities.
    GrantEnemyChoiceProtectionForTurn,
}

pub fn followups(id: usize) -> &'static [Followup] {
    match id {
        1 | 52 => &[Followup::Predict],
        19 => &[Followup::GrantEnemyChoiceProtectionForTurn],
        _ => &[],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowCost {
    pub energy: u8,
    pub any_power: u8,
    /// False means generic power; true requires one of the card's domains.
    pub colored: bool,
}

/// Printed Flow costs, excluding temporary Flow granted by Kennen's trigger.
/// A card played using Flow is banished afterwards instead of going to trash.
pub fn flow_cost(id: usize) -> Option<FlowCost> {
    match id {
        19 => Some(FlowCost {
            energy: 2,
            any_power: 0,
            colored: false,
        }),
        26 => Some(FlowCost {
            energy: 4,
            any_power: 0,
            colored: false,
        }),
        39 => Some(FlowCost {
            energy: 2,
            any_power: 1,
            colored: false,
        }),
        48 => Some(FlowCost {
            energy: 3,
            any_power: 0,
            colored: false,
        }),
        134 => Some(FlowCost {
            energy: 2,
            any_power: 0,
            colored: false,
        }),
        138 => Some(FlowCost {
            energy: 2,
            any_power: 1,
            colored: true,
        }),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepeatCost {
    pub energy: u8,
    pub power: u8,
    pub domain: Option<Domain>,
}

pub fn repeat_cost(id: usize) -> Option<RepeatCost> {
    match id {
        217 => Some(RepeatCost {
            energy: 3,
            power: 0,
            domain: None,
        }),
        37 | 133 | 135 | 136 | 137 | 216 | 220 => Some(RepeatCost {
            energy: 2,
            power: 0,
            domain: None,
        }),
        50 => Some(RepeatCost {
            energy: 1,
            power: 1,
            domain: Some(Domain::Mind),
        }),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenPlacement {
    Default,
    Base,
    Here,
}

pub fn token_placement(id: usize) -> TokenPlacement {
    match id {
        48 | 87 | 138 | 187 | 220 | 235 => TokenPlacement::Default,
        102 | 147 | 148 | 178 | 179 | 180 | 203 => TokenPlacement::Here,
        _ => TokenPlacement::Base,
    }
}

pub fn token_is_gear(id: usize) -> bool {
    token_spec(id).is_some_and(|token| token.is_gear)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenSpec {
    pub name: &'static str,
    pub might: u8,
    pub count: u8,
    pub is_gear: bool,
    pub deflect: bool,
    pub tag: Option<&'static str>,
}

pub fn token_spec(id: usize) -> Option<TokenSpec> {
    match id {
        178 | 220 => Some(TokenSpec {
            name: "Sand Soldier",
            might: 2,
            count: 1,
            is_gear: false,
            deflect: false,
            tag: Some("Shurima"),
        }),
        13 | 149 | 151 | 160 | 161 | 162 | 163 | 201 | 202 | 219 => Some(TokenSpec {
            name: "Gold",
            might: 0,
            count: match id {
                151 => 4,
                160 | 202 => 2,
                _ => 1,
            },
            is_gear: true,
            deflect: false,
            tag: None,
        }),
        34 | 148 | 150 | 187 => Some(TokenSpec {
            name: "Bird",
            might: 1,
            count: 1,
            is_gear: false,
            deflect: true,
            tag: Some("Bird"),
        }),
        48 => Some(TokenSpec {
            name: "Tentacle",
            might: 1,
            count: 2,
            is_gear: false,
            deflect: false,
            tag: Some("Bilgewater"),
        }),
        54 | 138 | 241 => Some(TokenSpec {
            name: "Mech",
            might: 3,
            count: if id == 54 { 2 } else { 1 },
            is_gear: false,
            deflect: false,
            tag: Some("Mech"),
        }),
        87 | 102 | 147 | 175 | 176 | 177 | 179 | 180 | 203 | 235 => Some(TokenSpec {
            name: "Recruit",
            might: 1,
            count: match id {
                87 => 4,
                102 => 2,
                177 | 180 | 235 => 3,
                _ => 1,
            },
            is_gear: false,
            deflect: false,
            tag: Some("Recruit"),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::cards;

    #[test]
    fn token_tags_and_characteristics_follow_core_rule_187() {
        // Core Rules 187.1, .4, .7 and .10 define these intrinsic token tags;
        // they are not inferred from the card that creates the token.
        for (id, name, might, tag, deflect) in [
            (34, "Bird", 1, "Bird", true),
            (48, "Tentacle", 1, "Bilgewater", false),
            (54, "Mech", 3, "Mech", false),
            (87, "Recruit", 1, "Recruit", false),
            (102, "Recruit", 1, "Recruit", false),
        ] {
            let token = token_spec(id).unwrap();
            assert_eq!(token.name, name);
            assert_eq!(token.might, might);
            assert_eq!(token.tag, Some(tag));
            assert_eq!(token.deflect, deflect);
            assert!(!token.is_gear);
        }
        assert_eq!(token_spec(13).unwrap().tag, None);
    }

    #[test]
    fn all_catalog_entries_have_official_classification() {
        let cards = cards();
        assert_eq!(cards.len(), 246);
        for card in cards {
            assert_ne!(
                effect(card.id),
                OfficialEffect::Unsupported,
                "{}",
                card.name
            );
            assert_ne!(support_status(card.id), SupportStatus::UnknownCard);
        }
        assert_eq!(effect(246), OfficialEffect::Unsupported);
    }

    #[test]
    fn effects_preserve_real_quantities_and_timing() {
        let catalog = cards();
        assert_eq!(catalog[53].name, "Falling Star");
        assert_eq!(
            effect(53),
            OfficialEffect::Damage {
                amount: 3,
                targets: 2,
                battlefield_only: false,
                draw: 0,
                banish_on_death: false
            }
        );
        assert!(!targeting(53).distinct);
        assert!(targeting(61).distinct);
        assert_eq!(catalog[48].name, "Up from the Deep");
        assert_eq!(effect(48), OfficialEffect::Tokens { count: 2, might: 1 });
        assert_eq!(catalog[42].name, "Seal of Discord");
        assert_eq!(effect(42), OfficialEffect::PowerGear);
        assert_eq!(effect(65), OfficialEffect::ExtraTurn);
        assert_eq!(effect(68), OfficialEffect::None);
        assert_eq!(
            triggers(68),
            &[trigger!(Event::Died, TriggerEffect::Draw(1))]
        );
        assert_eq!(
            effect(7),
            OfficialEffect::ModifyMight {
                amount: 2,
                minimum: 0,
                draw: 1,
                alone_bonus: 0
            }
        );
        assert_eq!(effect(9), OfficialEffect::MoveFriendlyToBase { up_to: 2 });
        assert_eq!(
            effect(11),
            OfficialEffect::Bounce {
                max_might: Some(3),
                battlefield_only: true
            }
        );
    }

    #[test]
    fn equipment_carries_its_secondary_panel_and_cost() {
        assert_eq!(equipment(3).unwrap().ability, EquipmentAbility::Ganking);
        assert_eq!(
            equipment(10).unwrap().ability,
            EquipmentAbility::GuardianAngel
        );
        assert_eq!(equipment(38).unwrap().recycle_trash, 2);
        assert!(equipment(58).unwrap().quick_draw);
        assert!(equipment(21).is_none());
    }
}
