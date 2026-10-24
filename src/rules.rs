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
    /// Seal of Discord: persistent gear, exhaust to add Chaos power.
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
        _ => Unsupported,
    }
}

pub fn targeting(id: usize) -> Targeting {
    use Allegiance::*;
    let allegiance = match id {
        4 | 32 => Enemy,
        8 | 9 | 14 | 19 | 27 => Friendly,
        16 | 29 => Mixed,
        5 | 7 | 11 | 23 | 26 | 28 | 41 | 44 | 50 | 52 | 53 | 55 | 61 | 62 | 63 | 66 | 67 | 74
        | 75 | 76 | 79 | 80 => Any,
        _ => None,
    };
    Targeting {
        allegiance,
        distinct: matches!(id, 5 | 9 | 16 | 44 | 50 | 61),
        same_location: matches!(id, 44 | 50),
    }
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
    Tokens,
    Accelerate,
    Ambush,
    Assault,
    ExtraTurn,
    Temporary,
    SpellTriggers,
    BanishOnDeath,
}

/// Additional systems that must work before the complete card can be advertised
/// as accurate. Presence here does not mean the engine has implemented it.
pub fn requirements(id: usize) -> &'static [Mechanic] {
    use Mechanic::*;
    match id {
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
    pub equip_domain: Domain,
    pub equip_power: u8,
    pub recycle_trash: u8,
    pub quick_draw: bool,
    pub ability: EquipmentAbility,
}

pub fn equipment(id: usize) -> Option<EquipmentRule> {
    let (might, equip_domain, recycle_trash, quick_draw, ability) = match id {
        3 => (2, Domain::Chaos, 0, false, EquipmentAbility::Ganking),
        10 => (1, Domain::Calm, 0, false, EquipmentAbility::GuardianAngel),
        38 => (2, Domain::Chaos, 2, false, EquipmentAbility::LastRites),
        58 => (2, Domain::Fury, 0, true, EquipmentAbility::None),
        _ => return None,
    };
    Some(EquipmentRule {
        might,
        equip_domain,
        equip_power: 1,
        recycle_trash,
        quick_draw,
        ability,
    })
}

/// Passive and activated abilities that are not represented by the immediate
/// operation. These must not be silently discarded for a "base-stats" unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticAbility {
    Deflect,
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
        0 | 20 => &[Deflect],
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
        _ => &[],
    }
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
}

/// Printed Flow costs, excluding temporary Flow granted by Kennen's trigger.
/// A card played using Flow is banished afterwards instead of going to trash.
pub fn flow_cost(id: usize) -> Option<FlowCost> {
    match id {
        19 => Some(FlowCost {
            energy: 2,
            any_power: 0,
        }),
        26 => Some(FlowCost {
            energy: 4,
            any_power: 0,
        }),
        39 => Some(FlowCost {
            energy: 2,
            any_power: 1,
        }),
        48 => Some(FlowCost {
            energy: 3,
            any_power: 0,
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
        37 => Some(RepeatCost {
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
        13 => Some(TokenSpec {
            name: "Gold",
            might: 0,
            count: 1,
            is_gear: true,
            deflect: false,
            tag: None,
        }),
        34 => Some(TokenSpec {
            name: "Bird",
            might: 1,
            count: 1,
            is_gear: false,
            deflect: true,
            tag: None,
        }),
        48 => Some(TokenSpec {
            name: "Tentacle",
            might: 1,
            count: 2,
            is_gear: false,
            deflect: false,
            tag: Some("Bilgewater"),
        }),
        54 => Some(TokenSpec {
            name: "Mech",
            might: 3,
            count: 2,
            is_gear: false,
            deflect: false,
            tag: None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::cards;

    #[test]
    fn all_catalog_entries_have_official_classification() {
        let cards = cards();
        assert_eq!(cards.len(), 81);
        for card in cards {
            assert_ne!(
                effect(card.id),
                OfficialEffect::Unsupported,
                "{}",
                card.name
            );
            assert_ne!(support_status(card.id), SupportStatus::UnknownCard);
        }
        assert_eq!(effect(81), OfficialEffect::Unsupported);
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
