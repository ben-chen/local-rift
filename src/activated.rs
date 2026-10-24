//! Explicit printed activations for the reviewed unit/gear foundation.
//! Costs and targets finalize before responses; Add resolves immediately.
use crate::cards::{CardId, Domain};
use crate::game::{Action, ChainAbility, Game, Phase, Target};
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
pub struct AbilityTargets {
    pub min: u8,
    pub max: u8,
    pub allegiance: Allegiance,
    pub battlefield: bool,
    pub exhausted: bool,
}

impl AbilityTargets {
    const NONE: Self = Self {
        min: 0,
        max: 0,
        allegiance: Allegiance::None,
        battlefield: false,
        exhausted: false,
    };
    const UNIT: Self = Self {
        min: 1,
        max: 1,
        allegiance: Allegiance::Any,
        ..Self::NONE
    };
    const FIELD: Self = Self {
        battlefield: true,
        ..Self::UNIT
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AbilityEffect {
    Might { amount: i16, minimum: u8 },
    Buff,
    Damage(u8),
    SourceMightDamage,
    Recruits(u8),
    Draw(u8),
    Points(u8),
    AddEnergy(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivatedRule {
    pub label: &'static str,
    pub timing: AbilityTiming,
    pub energy: u8,
    pub power: u8,
    pub domain: Option<Domain>,
    pub exhaust: bool,
    pub requires_battlefield: bool,
    pub targets: AbilityTargets,
    effect: AbilityEffect,
}

impl ActivatedRule {
    pub fn adds_resources(&self) -> bool {
        matches!(self.effect, AbilityEffect::AddEnergy(_))
    }
}

const fn exhaust(
    label: &'static str,
    targets: AbilityTargets,
    effect: AbilityEffect,
) -> ActivatedRule {
    ActivatedRule {
        label,
        timing: AbilityTiming::Main,
        energy: 0,
        power: 0,
        domain: None,
        exhaust: true,
        requires_battlefield: false,
        targets,
        effect,
    }
}

/// Stable zero-based ability indices belong to each explicit source printing.
pub fn activated_rules(card_id: CardId) -> &'static [ActivatedRule] {
    match card_id {
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
                    AbilityEffect::Recruits(3),
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
                label: "Draw 1",
                timing: AbilityTiming::Main,
                energy: 1,
                power: 1,
                domain: Some(Domain::Mind),
                exhaust: false,
                requires_battlefield: true,
                targets: AbilityTargets::NONE,
                effect: AbilityEffect::Draw(1),
            },
            ActivatedRule {
                label: "Score 1 point",
                timing: AbilityTiming::Main,
                energy: 4,
                power: 4,
                domain: Some(Domain::Mind),
                exhaust: true,
                requires_battlefield: true,
                targets: AbilityTargets::NONE,
                effect: AbilityEffect::Points(1),
            },
        ],
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

    /// Every live printed source ability, including ones currently unavailable.
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
                (0..activated_rules(card_id).len()).map(move |ability_index| ActivatedOption {
                    source,
                    card_id,
                    ability_index,
                })
            })
            .collect()
    }

    fn ability_target_valid(&self, actor: usize, rule: &ActivatedRule, target: Target) -> bool {
        if rule.targets.max == 0 || !self.can_choose_unit(actor, target) {
            return false;
        }
        let Target::Unit { player, .. } = target else {
            return false;
        };
        let Some(unit) = self.unit_ref(target) else {
            return false;
        };
        match rule.targets.allegiance {
            Allegiance::Friendly if player != actor => return false,
            Allegiance::Enemy if player == actor => return false,
            Allegiance::None => return false,
            _ => {}
        }
        (!rule.targets.battlefield || unit.lane.is_some())
            && (!rule.targets.exhausted || !unit.ready)
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
                state.units.iter().map(move |u| Target::Unit {
                    player,
                    unit_id: u.id,
                })
            })
            .filter(|&target| self.ability_target_valid(source.player(), rule, target))
            .collect()
    }

    fn pay_ability_costs(
        &mut self,
        player: usize,
        rule: &ActivatedRule,
        targets: &[Target],
    ) -> Result<(), String> {
        if self.players[player].energy < rule.energy {
            return Err(format!("This ability needs {} energy.", rule.energy));
        }
        self.pay_energy(player, rule.energy);
        self.spend_power(player, rule.domain, rule.power)?;
        self.spend_deflect(player, false, targets)
    }

    pub fn validate_ability(
        &self,
        source: AbilitySource,
        ability_index: usize,
        targets: &[Target],
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
                .any(|&target| !self.ability_target_valid(player, rule, target))
        {
            return Err("Choose exactly the eligible targets for this ability.".into());
        }
        // One shared payment simulation covers overlapping colored and Deflect
        // obligations, including explicit source restrictions from the wrapper.
        self.clone().pay_ability_costs(player, rule, targets)
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
            for targets in groups {
                if self
                    .validate_ability(option.source, option.ability_index, &targets)
                    .is_ok()
                {
                    actions.push(Action::ActivateAbility {
                        source: option.source,
                        ability_index: option.ability_index,
                        targets,
                    });
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
        self.validate_ability(source, ability_index, &targets)?;
        let player = source.player();
        let card_id = self.ability_source_state(source).unwrap().0;
        let rule = &activated_rules(card_id)[ability_index];
        self.pay_ability_costs(player, rule, &targets)?;
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
        if rule.adds_resources() {
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
            .filter(|&target| self.ability_target_valid(player, rule, target))
            .collect();
        match rule.effect {
            AbilityEffect::Might { amount, minimum } => {
                for target in valid {
                    self.modify_might(target, amount, minimum);
                }
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
                        let amount = self.battlefield_damage(target, amount);
                        if let Some(unit) = self.unit_mut(target) {
                            unit.damage = unit.damage.saturating_add(amount);
                        }
                    }
                }
            }
            AbilityEffect::Recruits(count) => {
                for _ in 0..count {
                    self.queue_token_play(player, card_id, 1, None, false);
                }
            }
            AbilityEffect::Draw(count) => self.draw(player, count),
            AbilityEffect::Points(count) => {
                self.players[player].score = self.players[player].score.saturating_add(count)
            }
            AbilityEffect::AddEnergy(amount) => {
                self.players[player].energy_pool =
                    self.players[player].energy_pool.saturating_add(amount);
                self.players[player].energy = self.players[player].energy.saturating_add(amount);
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
            AbilityEffect::Points(amount) => Some(if self.players[player].score.saturating_add(amount) >= 8 { 1000 } else { 120 }),
            AbilityEffect::Recruits(_) => Some(90),
            AbilityEffect::Draw(_) => (self.players[player].hand.len() < 8 && (!self.players[player].deck.is_empty() || !self.players[player].discard.is_empty())).then_some(30),
            AbilityEffect::Buff => unit.filter(|unit| unit.buff == 0).map(|_| 65),
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
            AbilityEffect::Might { .. } => unit.filter(|_| !enemy_target).and_then(|unit| {
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
            AbilityEffect::AddEnergy(_) => {
                let mut probe = self.clone();
                probe.activate_ability(source, index, Vec::new()).ok()?;
                let opens_play = self.players[player].hand.iter().any(|&id| {
                    self.validate_play(id, None, None).is_err() && probe.validate_play(id, None, None).is_ok()
                }) || self.players[player].champion.is_some_and(|id| self.validate_play(id, None, None).is_err() && probe.validate_play(id, None, None).is_ok());
                opens_play.then_some(80)
            }
        }
    }

    /// Only fixed, cost-free Add effects are considered here. Later conversion
    /// abilities need an obligation-aware search, not an unconditional greedy pass.
    pub(crate) fn generate_ability_resources(&mut self, player: usize) {
        for option in self.activated_options(player) {
            let rule = &activated_rules(option.card_id)[option.ability_index];
            if rule.adds_resources() && rule.energy == 0 && rule.power == 0 {
                let _ = self.activate_ability(option.source, option.ability_index, Vec::new());
            }
        }
    }
}
