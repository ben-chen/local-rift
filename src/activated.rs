//! Explicit printed activations for the reviewed unit/gear foundation.
//! Costs and targets finalize before responses; Add resolves immediately.
use crate::cards::{CardId, Domain};
use crate::game::{Action, ChainAbility, Decision, Game, Phase, Target};
use crate::rules::Allegiance;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbilitySource {
    Unit { player: usize, unit_id: u32 },
    Gear { player: usize, gear_id: u32 },
}

impl AbilitySource {
    pub fn player(self) -> usize {
        match self {
            Self::Unit { player, .. } | Self::Gear { player, .. } => player,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbilityTiming {
    Main,
    Action,
    Reaction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbilityTargetKind {
    Unit,
    Gear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversionDirection {
    EnergyToPower,
    PowerToEnergy,
}

#[derive(Clone, Debug)]
pub(crate) struct ResourceFrame {
    pub suspended: Option<Decision>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceContinuationView {
    pub player: usize,
    pub task: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AbilityTargets {
    pub kind: AbilityTargetKind,
    pub another: bool,
    pub min: u8,
    pub max: u8,
    pub allegiance: Allegiance,
    pub battlefield: bool,
    pub exhausted: bool,
}

impl AbilityTargets {
    pub const NONE: Self = Self {
        kind: AbilityTargetKind::Unit,
        another: false,
        min: 0,
        max: 0,
        allegiance: Allegiance::None,
        battlefield: false,
        exhausted: false,
    };
    pub const UNIT: Self = Self {
        min: 1,
        max: 1,
        allegiance: Allegiance::Any,
        ..Self::NONE
    };
    pub const FIELD: Self = Self {
        battlefield: true,
        ..Self::UNIT
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AbilityEffect {
    Might { amount: i16, minimum: u8 },
    Buff,
    BuffSource,
    Add(ResourceOutput),
    Damage(u8),
    SourceMightDamage,
    Tokens,
    Draw(u8),
    Points(u8),
    AddEnergy(u8),
    Conversion(ConversionDirection),
    EmpowerGear,
    EmpowerSource,
    BuffOrMight(u8),
    MoveTargetToBase,
    MightIfSourceEmpowered { base: i16, empowered: i16 },
    AddEnergyIfSourceEmpowered { base: u8, empowered: u8 },
    PlayGrant(crate::play::PlayGrantKind),
    SunDisc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbilityCostModifier {
    EnergyPerControlledRune { amount: u16 },
    EnergyIfRunesAtMost { maximum: u8, amount: u16 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceOutput {
    pub energy: u16,
    pub power: u16,
    pub domain: Option<Domain>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivatedRule {
    pub xp_cost: crate::xp::XpAmount,
    pub minimum_xp: Option<u8>,
    pub cost_modifiers: &'static [AbilityCostModifier],
    pub label: &'static str,
    pub timing: AbilityTiming,
    pub energy: u8,
    pub power: u8,
    pub domain: Option<Domain>,
    pub exhaust: bool,
    pub recycle_count: u8,
    pub recycle_unit_only: bool,
    pub kill_source: bool,
    pub only_if_not_empowered: bool,
    pub disempower_source: bool,
    pub requires_battlefield: bool,
    pub targets: AbilityTargets,
    effect: AbilityEffect,
}

impl ActivatedRule {
    pub(crate) fn is_empower(&self) -> bool {
        matches!(self.effect, AbilityEffect::EmpowerSource)
    }
    pub fn adds_resources(&self) -> bool {
        matches!(
            self.effect,
            AbilityEffect::Add(_)
                | AbilityEffect::AddEnergy(_)
                | AbilityEffect::AddEnergyIfSourceEmpowered { .. }
                | AbilityEffect::Conversion(_)
        )
    }
    pub fn fixed_resource_output(&self) -> Option<ResourceOutput> {
        match self.effect {
            AbilityEffect::Add(output) => Some(output),
            AbilityEffect::AddEnergy(energy) => Some(ResourceOutput {
                energy: energy.into(),
                power: 0,
                domain: None,
            }),
            _ => None,
        }
    }
    pub fn conversion_direction(&self) -> Option<ConversionDirection> {
        if let AbilityEffect::Conversion(direction) = self.effect {
            Some(direction)
        } else {
            None
        }
    }
}

const fn exhaust(
    label: &'static str,
    targets: AbilityTargets,
    effect: AbilityEffect,
) -> ActivatedRule {
    ActivatedRule {
        xp_cost: 0,
        minimum_xp: None,
        cost_modifiers: &[],
        label,
        timing: AbilityTiming::Main,
        energy: 0,
        power: 0,
        domain: None,
        exhaust: true,
        recycle_count: 0,
        recycle_unit_only: false,
        kill_source: false,
        only_if_not_empowered: false,
        disempower_source: false,
        requires_battlefield: false,
        targets,
        effect,
    }
}

const fn empower(
    energy: u8,
    power: u8,
    domain: Option<Domain>,
    cost_modifiers: &'static [AbilityCostModifier],
) -> ActivatedRule {
    ActivatedRule {
        energy,
        power,
        domain,
        cost_modifiers,
        exhaust: false,
        only_if_not_empowered: true,
        ..exhaust(
            "Empower",
            AbilityTargets::NONE,
            AbilityEffect::EmpowerSource,
        )
    }
}

/// Stable zero-based ability indices belong to each explicit source printing.
pub fn activated_rules(card_id: CardId) -> &'static [ActivatedRule] {
    match card_id {
        345 | 346 => {
            const {
                &[ActivatedRule {
                    xp_cost: 2,
                    exhaust: false,
                    ..exhaust(
                        "Spend 2 XP: Buff me",
                        AbilityTargets::NONE,
                        AbilityEffect::BuffSource,
                    )
                }]
            }
        }
        347 => {
            const {
                &[
                    ActivatedRule {
                        timing: AbilityTiming::Reaction,
                        ..exhaust(
                            "Add 1 Any power",
                            AbilityTargets::NONE,
                            AbilityEffect::Add(ResourceOutput {
                                energy: 0,
                                power: 1,
                                domain: None,
                            }),
                        )
                    },
                    ActivatedRule {
                        timing: AbilityTiming::Reaction,
                        minimum_xp: Some(6),
                        ..exhaust(
                            "Add 1 energy and 1 Any power",
                            AbilityTargets::NONE,
                            AbilityEffect::Add(ResourceOutput {
                                energy: 1,
                                power: 1,
                                domain: None,
                            }),
                        )
                    },
                ]
            }
        }
        272 | 288 => const { &[empower(2, 1, Some(Domain::Fury), &[])] },
        273 => const { &[empower(7, 0, None, &[])] },
        274 | 276 | 284 => const { &[empower(3, 0, None, &[])] },
        275 | 277 | 283 => const { &[empower(2, 0, None, &[])] },
        278 | 279 => {
            const {
                &[empower(
                    12,
                    0,
                    None,
                    &[AbilityCostModifier::EnergyPerControlledRune { amount: 1 }],
                )]
            }
        }
        280 => {
            const {
                &[empower(
                    5,
                    0,
                    None,
                    &[AbilityCostModifier::EnergyIfRunesAtMost {
                        maximum: 4,
                        amount: 3,
                    }],
                )]
            }
        }
        281 => const { &[empower(6, 1, Some(Domain::Fury), &[])] },
        282 => const { &[empower(3, 1, Some(Domain::Order), &[])] },
        285 => const { &[empower(1, 1, Some(Domain::Order), &[])] },
        286 => const { &[empower(1, 2, None, &[])] },
        287 => const { &[empower(8, 0, None, &[])] },
        289 => const { &[empower(1, 2, Some(Domain::Order), &[])] },
        254 => {
            const {
                &[exhaust(
                    "Legion: next unit enters ready",
                    AbilityTargets::NONE,
                    AbilityEffect::SunDisc,
                )]
            }
        }
        255 => {
            const {
                &[exhaust(
                    "Next spell gets Bonus Damage 1",
                    AbilityTargets::NONE,
                    AbilityEffect::PlayGrant(crate::play::PlayGrantKind::BonusNextSpell(1)),
                )]
            }
        }
        256 => {
            const {
                &[ActivatedRule {
                    power: 1,
                    ..exhaust(
                        "Next spell gains Repeat at its printed cost",
                        AbilityTargets::NONE,
                        AbilityEffect::PlayGrant(crate::play::PlayGrantKind::RepeatNextSpell),
                    )
                }]
            }
        }
        267 => {
            const {
                &[ActivatedRule {
                    energy: 3,
                    exhaust: false,
                    only_if_not_empowered: true,
                    ..exhaust(
                        "Empower Applied Researchers",
                        AbilityTargets::NONE,
                        AbilityEffect::EmpowerSource,
                    )
                }]
            }
        }
        268 => {
            const {
                &[ActivatedRule {
                    energy: 4,
                    power: 1,
                    domain: Some(Domain::Calm),
                    exhaust: false,
                    only_if_not_empowered: true,
                    ..exhaust(
                        "Empower Helm of Suppression",
                        AbilityTargets::NONE,
                        AbilityEffect::EmpowerSource,
                    )
                }]
            }
        }
        230 | 233 => {
            const {
                &[exhaust(
                    "Give a unit +3 Might",
                    AbilityTargets::UNIT,
                    AbilityEffect::Might {
                        amount: 3,
                        minimum: 0,
                    },
                )]
            }
        }
        231 => {
            const {
                &[exhaust(
                    "Give a unit −1 Might (minimum 1)",
                    AbilityTargets::UNIT,
                    AbilityEffect::Might {
                        amount: -1,
                        minimum: 1,
                    },
                )]
            }
        }
        232 => {
            const {
                &[exhaust(
                    "Deal 2 at a battlefield",
                    AbilityTargets::FIELD,
                    AbilityEffect::Damage(2),
                )]
            }
        }
        234 => {
            const {
                &[exhaust(
                    "Buff an exhausted friendly unit",
                    AbilityTargets {
                        allegiance: Allegiance::Friendly,
                        exhausted: true,
                        ..AbilityTargets::UNIT
                    },
                    AbilityEffect::Buff,
                )]
            }
        }
        235 => {
            const {
                &[exhaust(
                    "Play three Recruits",
                    AbilityTargets::NONE,
                    AbilityEffect::Tokens,
                )]
            }
        }
        236 => {
            const {
                &[ActivatedRule {
                    timing: AbilityTiming::Reaction,
                    ..exhaust(
                        "Add 1 energy",
                        AbilityTargets::NONE,
                        AbilityEffect::AddEnergy(1),
                    )
                }]
            }
        }
        237 => {
            const {
                &[ActivatedRule {
                    power: 1,
                    domain: Some(Domain::Fury),
                    requires_battlefield: true,
                    ..exhaust(
                        "Deal 3 to a unit",
                        AbilityTargets::UNIT,
                        AbilityEffect::Damage(3),
                    )
                }]
            }
        }
        238 => {
            const {
                &[ActivatedRule {
                    requires_battlefield: true,
                    ..exhaust(
                        "Deal my Might at a battlefield",
                        AbilityTargets::FIELD,
                        AbilityEffect::SourceMightDamage,
                    )
                }]
            }
        }
        239 => &[
            ActivatedRule {
                xp_cost: 0,
                minimum_xp: None,
                label: "Draw 1",
                timing: AbilityTiming::Main,
                energy: 1,
                power: 1,
                domain: Some(Domain::Mind),
                exhaust: false,
                recycle_count: 0,
                recycle_unit_only: false,
                kill_source: false,
                cost_modifiers: &[],
                only_if_not_empowered: false,
                disempower_source: false,
                requires_battlefield: true,
                targets: AbilityTargets::NONE,
                effect: AbilityEffect::Draw(1),
            },
            ActivatedRule {
                xp_cost: 0,
                minimum_xp: None,
                label: "Score 1 point",
                timing: AbilityTiming::Main,
                energy: 4,
                power: 4,
                domain: Some(Domain::Mind),
                exhaust: true,
                recycle_count: 0,
                recycle_unit_only: false,
                kill_source: false,
                cost_modifiers: &[],
                only_if_not_empowered: false,
                disempower_source: false,
                requires_battlefield: true,
                targets: AbilityTargets::NONE,
                effect: AbilityEffect::Points(1),
            },
        ],
        240 => {
            const {
                &[ActivatedRule {
                    energy: 1,
                    recycle_count: 3,
                    ..exhaust(
                        "Recycle three cards, draw 1",
                        AbilityTargets::NONE,
                        AbilityEffect::Draw(1),
                    )
                }]
            }
        }
        241 => {
            const {
                &[ActivatedRule {
                    energy: 1,
                    power: 1,
                    domain: Some(Domain::Fury),
                    recycle_count: 1,
                    recycle_unit_only: true,
                    ..exhaust(
                        "Recycle a unit, play a Mech",
                        AbilityTargets::NONE,
                        AbilityEffect::Tokens,
                    )
                }]
            }
        }
        242 => {
            const {
                &[ActivatedRule {
                    timing: AbilityTiming::Action,
                    kill_source: true,
                    ..exhaust(
                        "Kill this: give a unit +2 Might",
                        AbilityTargets::UNIT,
                        AbilityEffect::Might {
                            amount: 2,
                            minimum: 0,
                        },
                    )
                }]
            }
        }
        243 => {
            const {
                &[exhaust(
                    "Empower another gear",
                    AbilityTargets {
                        kind: AbilityTargetKind::Gear,
                        another: true,
                        ..AbilityTargets::UNIT
                    },
                    AbilityEffect::EmpowerGear,
                )]
            }
        }
        244 => {
            const {
                &[ActivatedRule {
                    timing: AbilityTiming::Reaction,
                    ..exhaust(
                        "Convert energy to Any power",
                        AbilityTargets::NONE,
                        AbilityEffect::Conversion(ConversionDirection::EnergyToPower),
                    )
                }]
            }
        }
        245 => {
            const {
                &[ActivatedRule {
                    timing: AbilityTiming::Reaction,
                    ..exhaust(
                        "Convert power to energy",
                        AbilityTargets::NONE,
                        AbilityEffect::Conversion(ConversionDirection::PowerToEnergy),
                    )
                }]
            }
        }
        246 => {
            const {
                &[ActivatedRule {
                    energy: 1,
                    power: 1,
                    domain: Some(Domain::Calm),
                    kill_source: true,
                    ..exhaust(
                        "Kill this: draw 1",
                        AbilityTargets::NONE,
                        AbilityEffect::Draw(1),
                    )
                }]
            }
        }
        247 => {
            const {
                &[ActivatedRule {
                    energy: 1,
                    ..exhaust(
                        "Move a friendly battlefield unit to its base",
                        AbilityTargets {
                            allegiance: Allegiance::Friendly,
                            ..AbilityTargets::FIELD
                        },
                        AbilityEffect::MoveTargetToBase,
                    )
                }]
            }
        }
        248 => {
            const {
                &[exhaust(
                    "Buff a unit; if already buffed, +2 Might this turn",
                    AbilityTargets::UNIT,
                    AbilityEffect::BuffOrMight(2),
                )]
            }
        }
        249 => {
            const {
                &[ActivatedRule {
                    requires_battlefield: true,
                    ..exhaust(
                        "Play two Birds with Deflect",
                        AbilityTargets::NONE,
                        AbilityEffect::Tokens,
                    )
                }]
            }
        }
        250 => {
            const {
                &[
                    ActivatedRule {
                        only_if_not_empowered: true,
                        ..exhaust(
                            "Empower this",
                            AbilityTargets::NONE,
                            AbilityEffect::EmpowerSource,
                        )
                    },
                    ActivatedRule {
                        energy: 1,
                        disempower_source: true,
                        ..exhaust(
                            "Disempower this: draw 1",
                            AbilityTargets::NONE,
                            AbilityEffect::Draw(1),
                        )
                    },
                ]
            }
        }
        251 => {
            const {
                &[
                    ActivatedRule {
                        only_if_not_empowered: true,
                        ..exhaust(
                            "Empower this",
                            AbilityTargets::NONE,
                            AbilityEffect::EmpowerSource,
                        )
                    },
                    ActivatedRule {
                        energy: 1,
                        disempower_source: true,
                        ..exhaust(
                            "Disempower this: play a Mech to base",
                            AbilityTargets::NONE,
                            AbilityEffect::Tokens,
                        )
                    },
                ]
            }
        }
        252 => {
            const {
                &[
                    ActivatedRule {
                        energy: 1,
                        only_if_not_empowered: true,
                        ..exhaust(
                            "Empower this",
                            AbilityTargets::NONE,
                            AbilityEffect::EmpowerSource,
                        )
                    },
                    ActivatedRule {
                        timing: AbilityTiming::Reaction,
                        ..exhaust(
                            "Add 1 energy, or 2 if Empowered",
                            AbilityTargets::NONE,
                            AbilityEffect::AddEnergyIfSourceEmpowered {
                                base: 1,
                                empowered: 2,
                            },
                        )
                    },
                ]
            }
        }
        253 => {
            const {
                &[
                    ActivatedRule {
                        energy: 2,
                        exhaust: false,
                        only_if_not_empowered: true,
                        ..exhaust(
                            "Empower this",
                            AbilityTargets::NONE,
                            AbilityEffect::EmpowerSource,
                        )
                    },
                    exhaust(
                        "Give a unit +2 Might, or +4 if this is Empowered",
                        AbilityTargets::UNIT,
                        AbilityEffect::MightIfSourceEmpowered {
                            base: 2,
                            empowered: 4,
                        },
                    ),
                ]
            }
        }
        _ => const { &[] },
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivatedOption {
    pub source: AbilitySource,
    pub card_id: CardId,
    pub ability_index: usize,
}

impl Game {
    fn ability_source_state(&self, source: AbilitySource) -> Option<(CardId, bool, bool, bool)> {
        let player = self.players.get(source.player())?;
        match source {
            AbilitySource::Unit { unit_id, .. } => player
                .units
                .iter()
                .find(|u| u.id == unit_id && !u.token)
                .map(|u| (u.card_id, u.ready, u.lane.is_some(), true)),
            AbilitySource::Gear { gear_id, .. } => player
                .gear
                .iter()
                .find(|g| g.id == gear_id && !g.token)
                .map(|g| {
                    (
                        g.card_id,
                        g.ready,
                        self.gear_location(source.player(), gear_id).is_some(),
                        g.attached_to.is_none(),
                    )
                }),
        }
    }

    fn ability_source_empowered(&self, source: AbilitySource) -> bool {
        self.ability_source_state(source).is_some()
            && self.unit_empowered(match source {
                AbilitySource::Unit { unit_id, .. } => unit_id,
                AbilitySource::Gear { gear_id, .. } => gear_id,
            })
    }

    /// Existing printed abilities on live sources. Timing and payment may still
    /// prevent activation; inactive dependent abilities do not exist here.
    pub fn activated_options(&self, player: usize) -> Vec<ActivatedOption> {
        let Some(state) = self.players.get(player) else {
            return Vec::new();
        };
        state
            .units
            .iter()
            .filter(|u| !u.token)
            .map(|u| {
                (
                    AbilitySource::Unit {
                        player,
                        unit_id: u.id,
                    },
                    u.card_id,
                )
            })
            .chain(state.gear.iter().filter(|g| !g.token).map(|g| {
                (
                    AbilitySource::Gear {
                        player,
                        gear_id: g.id,
                    },
                    g.card_id,
                )
            }))
            .flat_map(|(source, card_id)| {
                activated_rules(card_id)
                    .iter()
                    .enumerate()
                    .filter(move |(_, rule)| {
                        rule.minimum_xp.is_none_or(|n| state.xp >= u64::from(n))
                    })
                    .map(move |(ability_index, _)| ActivatedOption {
                        source,
                        card_id,
                        ability_index,
                    })
            })
            .collect()
    }

    fn ability_target_valid(
        &self,
        source: AbilitySource,
        rule: &ActivatedRule,
        target: Target,
    ) -> bool {
        let actor = source.player();
        if rule.targets.max == 0 {
            return false;
        }
        let (player, battlefield, ready) = match (rule.targets.kind, target) {
            (AbilityTargetKind::Unit, Target::Unit { player, .. }) => {
                if !self.can_choose_unit(actor, target) {
                    return false;
                }
                let Some(unit) = self.unit_ref(target) else {
                    return false;
                };
                (player, unit.lane.is_some(), unit.ready)
            }
            (AbilityTargetKind::Gear, Target::Gear { player, gear_id }) => {
                let Some(gear) = self
                    .players
                    .get(player)
                    .and_then(|p| p.gear.iter().find(|g| g.id == gear_id))
                else {
                    return false;
                };
                if rule.targets.another && source == (AbilitySource::Gear { player, gear_id }) {
                    return false;
                }
                (
                    player,
                    self.gear_location(player, gear_id).is_some(),
                    gear.ready,
                )
            }
            _ => return false,
        };
        match rule.targets.allegiance {
            Allegiance::Friendly if player != actor => return false,
            Allegiance::Enemy if player == actor => return false,
            Allegiance::None => return false,
            _ => {}
        }
        (!rule.targets.battlefield || battlefield) && (!rule.targets.exhausted || !ready)
    }

    /// Structural candidates; full action validation also checks timing/costs.
    pub fn ability_targets(&self, source: AbilitySource, ability_index: usize) -> Vec<Target> {
        let Some((card_id, ..)) = self.ability_source_state(source) else {
            return Vec::new();
        };
        let Some(rule) = activated_rules(card_id).get(ability_index) else {
            return Vec::new();
        };
        self.players
            .iter()
            .enumerate()
            .flat_map(|(player, state)| {
                state
                    .units
                    .iter()
                    .map(move |u| Target::Unit {
                        player,
                        unit_id: u.id,
                    })
                    .chain(state.gear.iter().map(move |g| Target::Gear {
                        player,
                        gear_id: g.id,
                    }))
            })
            .filter(|&target| self.ability_target_valid(source, rule, target))
            .collect()
    }

    /// Trash selections are costs, never chosen targets (CR417.6).
    pub fn ability_cost_candidates(
        &self,
        source: AbilitySource,
        ability_index: usize,
    ) -> Vec<usize> {
        let Some((card_id, ..)) = self.ability_source_state(source) else {
            return Vec::new();
        };
        let Some(rule) = activated_rules(card_id).get(ability_index) else {
            return Vec::new();
        };
        if rule.recycle_count == 0 {
            return Vec::new();
        }
        self.players[source.player()]
            .discard
            .iter()
            .enumerate()
            .filter_map(|(i, &id)| {
                (!rule.recycle_unit_only || self.catalog[id].kind == crate::cards::CardKind::Unit)
                    .then_some(i)
            })
            .collect()
    }

    fn pay_ability_costs(
        &mut self,
        source: AbilitySource,
        ability_index: usize,
        targets: &[Target],
    ) -> Result<(), String> {
        self.pay_action_cost(&Action::ActivateAbility {
            source,
            ability_index,
            targets: targets.to_vec(),
        })
    }

    pub fn validate_ability(
        &self,
        source: AbilitySource,
        ability_index: usize,
        targets: &[Target],
    ) -> Result<(), String> {
        self.validate_ability_with_costs(source, ability_index, targets, &[])
    }

    pub fn validate_ability_with_costs(
        &self,
        source: AbilitySource,
        ability_index: usize,
        targets: &[Target],
        trash_indices: &[usize],
    ) -> Result<(), String> {
        let player = source.player();
        let (card_id, ready, battlefield, active) = self
            .ability_source_state(source)
            .ok_or("That ability source is no longer on the board.")?;
        let rule = activated_rules(card_id)
            .get(ability_index)
            .ok_or("That printed ability does not exist.")?;
        if self.phase != Phase::Playing || player != self.priority() {
            return Err("Wait for priority before using an ability.".into());
        }
        let announcement = self.payment_frozen_source().unwrap_or(self);
        if rule
            .minimum_xp
            .is_some_and(|n| announcement.players[player].xp < u64::from(n))
        {
            return Err("This dependent ability requires more XP.".into());
        }
        if let Some(output) = rule.fixed_resource_output() {
            let state = &self.players[player];
            let power = output.domain.map_or(state.any_power, |d| {
                state.power_pool[crate::game::domain_index(d)]
            });
            if state.energy_pool.checked_add(output.energy).is_none()
                || state.energy.checked_add(output.energy).is_none()
                || power.checked_add(output.power).is_none()
            {
                return Err(
                    "engine_capacity: Added resources exceed the supported integer representation."
                        .into(),
                );
            }
        }
        if !active {
            return Err("Attached gear's printed rules are inactive.".into());
        }
        if self.decision.is_some() && !(self.resource_payment_pending() && rule.adds_resources()) {
            return Err("Finish this choice before using an ordinary ability.".into());
        }
        let timing = match rule.timing {
            AbilityTiming::Main => {
                self.is_main_phase()
                    && self.active == player
                    && self.chain.is_empty()
                    && self.showdown.is_none()
            }
            AbilityTiming::Action => {
                self.chain.is_empty()
                    && (self.showdown.is_some() || (self.active == player && self.is_main_phase()))
            }
            AbilityTiming::Reaction => true,
        };
        if !timing {
            return Err("This ability cannot be used in this timing window.".into());
        }
        if rule.only_if_not_empowered && self.ability_source_empowered(source) {
            return Err("Use this ability only while its source is not Empowered.".into());
        }
        if rule.disempower_source && !self.ability_source_empowered(source) {
            return Err("The source must be Empowered to pay its Disempower cost.".into());
        }
        if rule.exhaust && !ready {
            return Err("The source must be ready to pay its Exhaust cost.".into());
        }
        if rule.requires_battlefield && !battlefield {
            return Err("Use this ability only while its source is at a battlefield.".into());
        }
        if targets.len() < usize::from(rule.targets.min)
            || targets.len() > usize::from(rule.targets.max)
            || targets
                .iter()
                .any(|&target| !self.ability_target_valid(source, rule, target))
        {
            return Err("Choose exactly the eligible targets for this ability.".into());
        }
        let candidates = self.ability_cost_candidates(source, ability_index);
        if trash_indices.len() != usize::from(rule.recycle_count)
            || trash_indices
                .iter()
                .enumerate()
                .any(|(i, n)| !candidates.contains(n) || trash_indices[..i].contains(n))
        {
            return Err(format!(
                "Choose {} different eligible trash cards to recycle as a cost.",
                rule.recycle_count
            ));
        }
        // One shared payment simulation covers overlapping colored and Deflect
        // obligations, including explicit source restrictions from the wrapper.
        self.clone()
            .pay_ability_costs(source, ability_index, targets)
    }

    pub fn ability_actions(&self, player: usize) -> Vec<Action> {
        let mut actions = Vec::new();
        for option in self.activated_options(player) {
            let rule = &activated_rules(option.card_id)[option.ability_index];
            let groups = if rule.targets.max == 0 {
                vec![Vec::new()]
            } else {
                self.ability_targets(option.source, option.ability_index)
                    .into_iter()
                    .map(|target| vec![target])
                    .collect()
            };
            let mut costs = Vec::new();
            fn combinations(
                candidates: &[usize],
                count: usize,
                selected: &mut Vec<usize>,
                out: &mut Vec<Vec<usize>>,
            ) {
                if count == 0 {
                    out.push(selected.clone());
                    return;
                }
                for i in 0..candidates.len() {
                    selected.push(candidates[i]);
                    combinations(&candidates[i + 1..], count - 1, selected, out);
                    selected.pop();
                }
            }
            combinations(
                &self.ability_cost_candidates(option.source, option.ability_index),
                usize::from(rule.recycle_count),
                &mut Vec::new(),
                &mut costs,
            );
            for targets in groups {
                for trash_indices in &costs {
                    if self
                        .validate_ability_with_costs(
                            option.source,
                            option.ability_index,
                            &targets,
                            trash_indices,
                        )
                        .is_ok()
                    {
                        actions.push(if trash_indices.is_empty() {
                            Action::ActivateAbility {
                                source: option.source,
                                ability_index: option.ability_index,
                                targets: targets.clone(),
                            }
                        } else {
                            Action::ActivateAbilityWithCosts {
                                source: option.source,
                                ability_index: option.ability_index,
                                targets: targets.clone(),
                                trash_indices: trash_indices.clone(),
                            }
                        });
                    }
                }
            }
        }
        actions
    }

    pub(crate) fn activate_ability(
        &mut self,
        source: AbilitySource,
        ability_index: usize,
        targets: Vec<Target>,
    ) -> Result<(), String> {
        self.activate_ability_with_costs(source, ability_index, targets, Vec::new())
    }

    pub(crate) fn activate_ability_with_costs(
        &mut self,
        source: AbilitySource,
        ability_index: usize,
        targets: Vec<Target>,
        trash_indices: Vec<usize>,
    ) -> Result<(), String> {
        self.validate_ability_with_costs(source, ability_index, &targets, &trash_indices)?;
        let player = source.player();
        let card_id = self.ability_source_state(source).unwrap().0;
        let rule = &activated_rules(card_id)[ability_index];
        self.pay_ability_costs(source, ability_index, &targets)?;
        self.recycle_cards(player, &trash_indices);
        if rule.disempower_source {
            let object = match source {
                AbilitySource::Unit { player, unit_id } => {
                    crate::empower::BoardObject::Unit { player, unit_id }
                }
                AbilitySource::Gear { player, gear_id } => {
                    crate::empower::BoardObject::Gear { player, gear_id }
                }
            };
            self.disempower_by(player, object);
        }
        if rule.exhaust {
            match source {
                AbilitySource::Unit { unit_id, .. } => self.exhaust_unit(player, unit_id),
                AbilitySource::Gear { gear_id, .. } => {
                    self.players[player]
                        .gear
                        .iter_mut()
                        .find(|g| g.id == gear_id)
                        .unwrap()
                        .ready = false
                }
            }
        }
        if rule.kill_source
            && let AbilitySource::Gear { gear_id, .. } = source
        {
            self.gear_destroy(player, gear_id);
        }
        if rule.adds_resources() {
            if matches!(source, AbilitySource::Gear { .. }) {
                self.play_history.gear_activations[player] += 1;
            }
            self.resolve_activated(source, card_id, ability_index, &targets);
        } else {
            self.queue_ability(
                player,
                card_id,
                targets,
                ChainAbility::Activated {
                    source,
                    card_id,
                    ability_index,
                },
            );
            // Finalization can change a Level-dependent board state through
            // its paid XP cost. Cleanup precedes the response window (319.4),
            // but remains deferred while a frozen payment is still replaying.
            self.after_resolution();
        }
        Ok(())
    }

    pub(crate) fn resolve_activated(
        &mut self,
        source: AbilitySource,
        card_id: CardId,
        ability_index: usize,
        targets: &[Target],
    ) {
        let Some(rule) = activated_rules(card_id).get(ability_index) else {
            return;
        };
        let player = source.player();
        let valid: Vec<_> = targets
            .iter()
            .copied()
            .filter(|&target| self.ability_target_valid(source, rule, target))
            .collect();
        match rule.effect {
            AbilityEffect::SunDisc => {
                if self.cards_played_this_turn(player) > 0 {
                    self.grant_play(
                        player,
                        crate::game::ChainSource::Card { card_id },
                        crate::play::PlayGrantKind::ReadyNextUnit,
                    );
                }
            }
            AbilityEffect::PlayGrant(kind) => {
                self.grant_play(player, crate::game::ChainSource::Card { card_id }, kind);
            }
            AbilityEffect::Might { amount, minimum } => {
                for target in valid {
                    self.modify_might(target, amount, minimum);
                }
            }
            AbilityEffect::BuffSource => {
                if let AbilitySource::Unit {
                    player: owner,
                    unit_id,
                } = source
                {
                    self.buff_unit_by(
                        player,
                        Target::Unit {
                            player: owner,
                            unit_id,
                        },
                    );
                }
            }
            AbilityEffect::Add(output) => {
                // Immediate Add sources validate both pools before paying any cost.
                let state = &mut self.players[player];
                state.energy_pool = state
                    .energy_pool
                    .checked_add(output.energy)
                    .expect("validated Add energy");
                state.energy = state
                    .energy
                    .checked_add(output.energy)
                    .expect("validated available energy");
                let power = output.domain.map_or(&mut state.any_power, |d| {
                    &mut state.power_pool[crate::game::domain_index(d)]
                });
                *power = power
                    .checked_add(output.power)
                    .expect("validated Add power");
            }
            AbilityEffect::Buff => {
                for target in valid {
                    self.buff_unit_by(player, target);
                }
            }
            AbilityEffect::Damage(_) | AbilityEffect::SourceMightDamage => {
                let amount = match rule.effect {
                    AbilityEffect::Damage(amount) => Some(amount),
                    _ => match source {
                        AbilitySource::Unit { player, unit_id } => self
                            .unit_ref(Target::Unit { player, unit_id })
                            .map(|u| self.unit_might(u)),
                        AbilitySource::Gear { .. } => None,
                    },
                };
                if let Some(amount) = amount {
                    for target in valid {
                        let amount = self.damage_amount(
                            crate::game::damage::DamageContext {
                                player,
                                bonus_damage: 0,
                                dealer: crate::game::damage::DamageDealer::Ability,
                            },
                            target,
                            amount,
                        );
                        if let Some(unit) = self.unit_mut(target) {
                            unit.damage = unit.damage.saturating_add(amount);
                        }
                    }
                }
            }
            AbilityEffect::Tokens => {
                if let Some(spec) = crate::rules::token_spec(card_id) {
                    let destination = match crate::rules::token_placement(card_id) {
                        crate::rules::TokenPlacement::Default => None,
                        crate::rules::TokenPlacement::Base => Some(None),
                        crate::rules::TokenPlacement::Here => {
                            if self.ability_source_state(source).is_none() {
                                return;
                            }
                            Some(match source {
                                AbilitySource::Unit { player, unit_id } => {
                                    self.unit_ref(Target::Unit { player, unit_id })
                                        .unwrap()
                                        .lane
                                }
                                AbilitySource::Gear { player, gear_id } => {
                                    self.gear_location(player, gear_id)
                                }
                            })
                        }
                    };
                    for _ in 0..spec.count {
                        self.queue_token_play(
                            player,
                            card_id,
                            spec.might,
                            destination,
                            spec.is_gear,
                        );
                    }
                }
            }
            AbilityEffect::EmpowerSource => {
                let id = match source {
                    AbilitySource::Unit { unit_id, .. } => unit_id,
                    AbilitySource::Gear { gear_id, .. } => gear_id,
                };
                self.empower_now(player, Some(id), None);
            }
            AbilityEffect::BuffOrMight(amount) => {
                for target in valid {
                    if self
                        .unit_ref(target)
                        .is_some_and(|unit| self.buffed_units.contains(&unit.id))
                    {
                        self.modify_might(target, i16::from(amount), 0);
                    } else {
                        self.buff_unit_by(player, target);
                    }
                }
            }
            AbilityEffect::MoveTargetToBase => {
                for target in valid {
                    self.move_effect(player, target, None, false);
                }
            }
            AbilityEffect::MightIfSourceEmpowered { base, empowered } => {
                let amount = if self.ability_source_empowered(source) {
                    empowered
                } else {
                    base
                };
                for target in valid {
                    self.modify_might(target, amount, 0);
                }
            }
            AbilityEffect::EmpowerGear => {
                for target in valid {
                    if let Target::Gear { player, gear_id } = target {
                        self.empower_object_legacy(source.player(), player, Some(gear_id), None);
                    }
                }
            }
            AbilityEffect::Conversion(direction) => {
                self.resource_frames.push(ResourceFrame {
                    suspended: self.decision.take(),
                });
                self.decision = Some(Decision::ResourceConversion {
                    player,
                    source,
                    card_id,
                    ability_index,
                    direction,
                });
                return;
            }
            AbilityEffect::Draw(count) => self.draw(player, count),
            AbilityEffect::Points(count) => {
                self.players[player].score = self.players[player].score.saturating_add(count)
            }
            AbilityEffect::AddEnergy(_) | AbilityEffect::AddEnergyIfSourceEmpowered { .. } => {
                let amount = match rule.effect {
                    AbilityEffect::AddEnergy(amount) => amount,
                    AbilityEffect::AddEnergyIfSourceEmpowered { base, empowered } => {
                        if self.ability_source_empowered(source) {
                            empowered
                        } else {
                            base
                        }
                    }
                    _ => unreachable!(),
                };
                self.players[player].energy_pool = self.players[player]
                    .energy_pool
                    .saturating_add(amount.into());
                self.players[player].energy =
                    self.players[player].energy.saturating_add(amount.into());
            }
        }
        if matches!(source, AbilitySource::Gear { .. }) {
            self.gear_ability_used(player);
        }
        self.push_log(format!(
            "{} resolves {}: {}.",
            self.players[player].name, self.catalog[card_id].name, rule.label
        ));
    }

    /// Immediate resources only; future Add effects remain separate actions.
    pub fn conversion_available(&self) -> Option<u16> {
        let Decision::ResourceConversion {
            player, direction, ..
        } = self.decision.as_ref()?
        else {
            return None;
        };
        let state = &self.players[*player];
        Some(match direction {
            ConversionDirection::EnergyToPower => state.energy,
            ConversionDirection::PowerToEnergy => (u32::from(state.any_power)
                + state.power_pool.iter().map(|&n| u32::from(n)).sum::<u32>()
                + state.runes.iter().map(|&n| u32::from(n)).sum::<u32>())
            .min(u32::from(u16::MAX)) as u16,
        })
    }

    pub fn resource_continuations(&self) -> Vec<ResourceContinuationView> {
        self.resource_frames
            .iter()
            .filter_map(|frame| frame.suspended.as_ref())
            .map(|decision| {
                let task = match decision {
                    Decision::Payment { .. } => "Payment",
                    Decision::ResourceConversion { .. } => "ResourceConversion",
                    Decision::CounterTax { .. } => "CounterTax",
                    Decision::PendingPlay { .. } => "PendingPlay",
                    Decision::TriggerTargets { .. } => "TriggerTargets",
                    Decision::OnPlayTarget { .. } => "OnPlayTarget",
                    Decision::Legend { .. } => "Legend",
                    Decision::Battlefield { .. } => "Battlefield",
                    Decision::LastRites { .. } => "LastRites",
                    Decision::ReplaySpell { .. } => "ReplaySpell",
                    Decision::RevivePlay { .. } => "RevivePlay",
                    _ => "PaymentChoice",
                };
                ResourceContinuationView {
                    player: decision.player(),
                    task,
                }
            })
            .collect()
    }

    pub(crate) fn resolve_conversion(&mut self, amount: u16) -> Result<(), String> {
        let Some(Decision::ResourceConversion {
            player,
            source,
            card_id,
            ability_index,
            direction,
        }) = self.decision.clone()
        else {
            return Err("There is no resource conversion to finish.".into());
        };
        let available = self.conversion_available().unwrap_or(0);
        if amount > available {
            return Err(format!(
                "Choose at most {available} currently payable resources."
            ));
        }
        // Validate the entire command before committing: a selected power budget
        // can be insufficient even when a different collection could pay.
        let mut probe = self.clone();
        match direction {
            ConversionDirection::EnergyToPower => {
                probe.pay_energy(player, amount);
                probe.players[player].any_power = probe.players[player]
                    .any_power
                    .checked_add(amount)
                    .ok_or("Any power pool capacity exceeded.")?;
            }
            ConversionDirection::PowerToEnergy => {
                probe.spend_power(player, None, amount)?;
                probe.players[player].energy_pool = probe.players[player]
                    .energy_pool
                    .checked_add(amount)
                    .ok_or("Stored energy pool capacity exceeded.")?;
                probe.players[player].energy = probe.players[player]
                    .energy
                    .checked_add(amount)
                    .ok_or("Energy pool capacity exceeded.")?;
            }
        }
        probe.decision = probe
            .resource_frames
            .pop()
            .ok_or("The suspended resource task is missing.")?
            .suspended;
        if matches!(source, AbilitySource::Gear { .. }) {
            probe.gear_ability_used(player);
        }
        probe.push_log(format!(
            "{} finishes {}: converts {} resource(s).",
            probe.players[player].name, probe.catalog[card_id].name, amount
        ));
        debug_assert!(activated_rules(card_id).get(ability_index).is_some());
        *self = probe;
        Ok(())
    }

    /// A deliberately conservative heuristic: public board value only. The
    /// strategic policy also evaluates these actions through its rollout model.
    pub(crate) fn ability_ai_score(
        &self,
        source: AbilitySource,
        index: usize,
        targets: &[Target],
    ) -> Option<i32> {
        let player = source.player();
        let (card_id, ..) = self.ability_source_state(source)?;
        let rule = activated_rules(card_id).get(index)?;
        let target = targets.first().copied();
        let unit = target.and_then(|target| self.unit_ref(target));
        let enemy_target =
            matches!(target, Some(Target::Unit { player: owner, .. }) if owner != player);
        match rule.effect {
            AbilityEffect::BuffSource => {
                let AbilitySource::Unit { unit_id, .. } = source else { return None; };
                let unit = self.unit_ref(Target::Unit { player, unit_id })?;
                if self.buffed_units.contains(&unit.id) { return None; }
                let mut paid = self.clone();
                paid.spend_xp(player, rule.xp_cost).ok()?;
                // Do not lose a damaged Level unit just to add a single buff.
                (!paid.players[player].units.iter().any(|u| u.damage > 0 && u.damage >= paid.unit_might(u))).then_some(65)
            }
            AbilityEffect::Points(amount) => Some(if self.players[player].score.saturating_add(amount) >= 8 { 1000 } else { 120 }),
            AbilityEffect::Tokens => Some(90),
            AbilityEffect::EmpowerSource => Some(if rule.exhaust { 16 } else { 24 }),
            AbilityEffect::MoveTargetToBase => unit.filter(|_| !enemy_target).and_then(|unit| {
                let origin = unit.lane?;
                if !unit.ready || self.unit_keywords(unit).ganking || self.movement_locked(player, unit.id) {
                    return None;
                }
                let another_mover = self.players[player].units.iter().any(|u| u.lane.is_none() && u.ready);
                let destination = (0..2).any(|lane| lane != origin
                    && self.battlefield_owner[lane] != Some(player)
                    && !self.player_scored_this_turn[player][lane]
                    && self.players[1-player].units.iter().all(|u| u.lane != Some(lane)));
                let retains_hold = self.players[player].units.iter().any(|u| u.id != unit.id && u.lane == Some(origin));
                (destination && !another_mover && (retains_hold || self.player_scored_this_turn[player][origin]))
                    .then_some(if self.players[player].score >= 7 && self.player_scored_this_turn[player][origin] { 1000 } else { 70 })
            }),
            AbilityEffect::EmpowerGear => targets.first().and_then(|target| match target {
                Target::Gear { player: owner, gear_id } if *owner == player && !self.unit_empowered(*gear_id) => Some(35),
                _ => None,
            }),
            AbilityEffect::Conversion(_) => None,
            AbilityEffect::Draw(_) => (self.players[player].hand.len() < 8 && (!self.players[player].deck.is_empty() || !self.players[player].discard.is_empty())).then_some(30),
            AbilityEffect::Buff | AbilityEffect::BuffOrMight(_) => unit.filter(|unit| {
                !enemy_target && (matches!(rule.effect, AbilityEffect::BuffOrMight(_)) || !self.buffed_units.contains(&unit.id))
            }).filter(|_| {
                !matches!(source, AbilitySource::Unit { unit_id, .. } if self.unit_ref(Target::Unit { player, unit_id }).is_some_and(|u| u.lane.is_none()))
                    || !(0..2).any(|lane| self.battlefield_owner[lane] != Some(player) && self.players[1-player].units.iter().all(|u| u.lane != Some(lane)))
            }).map(|_| 65),
            AbilityEffect::Damage(amount) => unit.filter(|_| enemy_target).map(|unit| {
                if unit.damage.saturating_add(self.battlefield_damage(target.unwrap(), amount)) >= self.unit_might(unit) { 150 } else { 40 }
            }),
            AbilityEffect::SourceMightDamage => {
                let AbilitySource::Unit { unit_id, .. } = source else { return None; };
                let amount = self.unit_might(self.unit_ref(Target::Unit { player, unit_id })?);
                unit.filter(|_| enemy_target && amount > 0).map(|unit| {
                    if unit.damage.saturating_add(self.battlefield_damage(target.unwrap(), amount)) >= self.unit_might(unit) { 150 } else { 35 }
                })
            }
            AbilityEffect::Might { amount, minimum } if amount < 0 => unit.filter(|unit| enemy_target && self.unit_might(unit) > minimum).and_then(|unit| {
                let reduced = self.unit_might(unit).saturating_sub(amount.unsigned_abs() as u8).max(minimum);
                if unit.damage > 0 && unit.damage >= reduced { Some(140) }
                else { unit.lane.filter(|&lane| self.players[player].units.iter().any(|u| u.lane == Some(lane) || (u.lane.is_none() && u.ready))).map(|_| 40) }
            }),
            AbilityEffect::Might { .. } | AbilityEffect::MightIfSourceEmpowered { .. } => unit.filter(|_| !enemy_target).and_then(|unit| {
                // Do not exhaust a ready mover merely to increase its Might,
                // or spend a temporary boost on an uncontested empty field.
                if matches!(source, AbilitySource::Unit { unit_id, .. } if unit_id == unit.id) { return None; }
                let relevant = if let Some(lane) = unit.lane {
                    self.players[1 - player].units.iter().any(|u| u.lane == Some(lane) || (u.ready && u.lane.is_none()))
                } else {
                    unit.ready && self.players[1 - player].units.iter().any(|u| u.lane.is_some())
                };
                let preserve_mover = matches!(source, AbilitySource::Unit { unit_id, .. } if self.unit_ref(Target::Unit { player, unit_id }).is_some_and(|u| u.lane.is_none()))
                    && (0..2).any(|lane| self.battlefield_owner[lane] != Some(player) && self.players[1 - player].units.iter().all(|u| u.lane != Some(lane)));
                (relevant && !preserve_mover).then_some(55)
            }),
            AbilityEffect::SunDisc => (self.cards_played_this_turn(player)>0 && self.players[player].hand.iter().any(|&id|self.catalog[id].kind==crate::cards::CardKind::Unit)).then_some(65),
            AbilityEffect::PlayGrant(_) => self.players[player].hand.iter().any(|&id|self.catalog[id].kind==crate::cards::CardKind::Spell).then_some(60),
            AbilityEffect::Add(_) | AbilityEffect::AddEnergy(_) | AbilityEffect::AddEnergyIfSourceEmpowered { .. } => {
                let mut probe = self.clone();
                probe.activate_ability(source, index, Vec::new()).ok()?;
                let opens_play = self.players[player].hand.iter().any(|&id| {
                    self.validate_play(id, None, None).is_err() && probe.validate_play(id, None, None).is_ok()
                }) || self.players[player].champion.is_some_and(|id| self.validate_play(id, None, None).is_err() && probe.validate_play(id, None, None).is_ok());
                opens_play.then_some(80)
            }
        }
    }
}
