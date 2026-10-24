//! Registered legends' printed powers, independent of deck presets/champions.
//!
//! Trigger costs are paid during finalization (Core 383.3); non-resource
//! abilities enter the ordinary chain. Kai'Sa's Add ability resolves immediately
//! without transferring priority (Core 430.2). Spell-only power expires with the
//! rune pool and cannot pay for units or legend costs.
use crate::game::{Action, ChainAbility, Game, Phase, Target, Unit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegendKind {
    Irelia,
    MasterYi,
    Kennen,
    KaiSa,
    Darius,
    Viktor,
    Lux,
    Ambessa,
    Rengar,
    Ezreal,
}

impl LegendKind {
    pub fn id(self) -> usize {
        match self {
            Self::Irelia => 0,
            Self::MasterYi => 1,
            Self::Kennen => 2,
            Self::KaiSa => 3,
            Self::Darius => 4,
            Self::Viktor => 5,
            Self::Lux => 6,
            Self::Ambessa => 7,
            Self::Rengar => 8,
            Self::Ezreal => 9,
        }
    }

    pub fn from_id(id: usize) -> Option<Self> {
        Some(match id {
            0 => Self::Irelia,
            1 => Self::MasterYi,
            2 => Self::Kennen,
            3 => Self::KaiSa,
            4 => Self::Darius,
            5 => Self::Viktor,
            6 => Self::Lux,
            7 => Self::Ambessa,
            8 => Self::Rengar,
            9 => Self::Ezreal,
            _ => return None,
        })
    }

    /// Default artwork/card reference only; never the legend's identity.
    pub fn champion_id(self) -> usize {
        crate::legend_catalog::legend(self.id())
            .expect("registered legend")
            .default_champion
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegendTiming {
    Main,
    Action,
    Reaction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegendActivationEffect {
    SpellPower,
    Assault,
    LegionEnergy,
    Recruit,
    Ready,
    Draw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegendActivation {
    pub timing: LegendTiming,
    pub energy: u8,
    pub power: u8,
    pub domain: Option<crate::cards::Domain>,
    pub targets: crate::activated::AbilityTargets,
    pub exhaust: bool,
    pub disempower: bool,
    pub effect: LegendActivationEffect,
}

pub fn legend_activation(kind: LegendKind) -> Option<LegendActivation> {
    let (timing, energy, disempower, effect) = match kind {
        LegendKind::KaiSa => (
            LegendTiming::Reaction,
            0,
            false,
            LegendActivationEffect::SpellPower,
        ),
        LegendKind::Kennen => (
            LegendTiming::Action,
            0,
            true,
            LegendActivationEffect::Assault,
        ),
        LegendKind::Darius => (
            LegendTiming::Reaction,
            0,
            false,
            LegendActivationEffect::LegionEnergy,
        ),
        LegendKind::Viktor => (
            LegendTiming::Main,
            1,
            false,
            LegendActivationEffect::Recruit,
        ),
        LegendKind::Ambessa => (LegendTiming::Main, 0, true, LegendActivationEffect::Ready),
        LegendKind::Ezreal => (
            LegendTiming::Reaction,
            0,
            false,
            LegendActivationEffect::Draw,
        ),
        _ => return None,
    };
    Some(LegendActivation {
        timing,
        energy,
        power: if kind == LegendKind::Ambessa { 1 } else { 0 },
        domain: None,
        targets: if matches!(kind, LegendKind::Kennen | LegendKind::Ambessa) {
            crate::activated::AbilityTargets::UNIT
        } else {
            crate::activated::AbilityTargets::NONE
        },
        exhaust: true,
        disempower,
        effect,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegendState {
    pub kind: LegendKind,
    pub ready: bool,
    pub empowered: bool,
    pub spell_power: u16,
    /// Each granted Assault instance lasts through the current turn. The unit
    /// identifier follows its owner even if that owner differs from the legend's.
    pub assault_grants: Vec<(u32, u8)>,
}

impl LegendState {
    pub fn from_legend(id: usize) -> Result<Self, String> {
        let kind = LegendKind::from_id(id).ok_or("Choose an implemented legend.")?;
        if crate::legend_catalog::legend(id).is_none() {
            return Err("Legend metadata is missing.".into());
        }
        Ok(Self {
            kind,
            ready: true,
            empowered: false,
            spell_power: 0,
            assault_grants: Vec::new(),
        })
    }

    /// Compatibility constructor for original four-legend Rust fixtures.
    /// Match construction uses from_legend; a chosen champion does not select
    /// or change its legend.
    pub fn new(champion_id: usize) -> Self {
        let kind = match champion_id {
            0 => LegendKind::Irelia,
            22 => LegendKind::MasterYi,
            33 => LegendKind::Kennen,
            49 => LegendKind::KaiSa,
            _ => panic!("Unknown preset legend for champion {champion_id}"),
        };
        Self {
            kind,
            ready: true,
            empowered: false,
            spell_power: 0,
            assault_grants: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegendAbility {
    IreliaReady { unit_id: u32 },
    IreliaRefresh,
    KennenEmpower,
    MatriarchEmpower,
    MatriarchReady { target: Target },
    KennenAssault { target: Target },
    ViktorRecruit,
    LuxDraw,
    EzrealDraw,
    RengarMight { target: Target },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegendTrigger {
    IreliaChoose { unit_id: u32 },
    IreliaConquer,
}

impl Game {
    /// Progress toward Ezreal's two qualifying choices, deliberately capped at
    /// the printed threshold. This is not a total targeting-event counter.
    pub fn ezreal_progress(&self, player: usize) -> u8 {
        self.play_history
            .ezreal_progress
            .get(player)
            .copied()
            .unwrap_or(0)
    }

    pub(crate) fn record_spell_or_unit_choices(&mut self, player: usize, targets: &[Target]) {
        for &target in targets {
            let enemy = match target {
                Target::Unit { player: owner, .. } => {
                    owner != player && self.unit_ref(target).is_some()
                }
                Target::Gear {
                    player: owner,
                    gear_id,
                } => {
                    owner != player
                        && self.players[owner]
                            .gear
                            .iter()
                            .any(|gear| gear.id == gear_id)
                }
                _ => false,
            };
            if enemy {
                self.play_history.ezreal_progress[player] =
                    (self.play_history.ezreal_progress[player] + 1).min(2);
            }
        }
    }

    pub(crate) fn ability_chooses_as_unit(&self, card_id: usize, ability: &ChainAbility) -> bool {
        use crate::activated::AbilitySource;
        match ability {
            ChainAbility::Activated { source, .. } | ChainAbility::Hunt { source, .. } => {
                matches!(source, AbilitySource::Unit { .. })
            }
            // An equipment's granted ability belongs to its bearer, even after
            // the equipment detaches or that unit leaves the board.
            ChainAbility::CardTrigger { context, .. } => context.source.unit_id.is_some(),
            ChainAbility::Temporary { source, .. } => {
                matches!(source, crate::empower::BoardObject::Unit { .. })
            }
            ChainAbility::Legend(_)
            | ChainAbility::Battlefield(_)
            | ChainAbility::Equip { .. }
            | ChainAbility::QuickDrawAttach { .. } => false,
            _ => self.catalog[card_id].kind == crate::cards::CardKind::Unit,
        }
    }

    pub(crate) fn legend_on_unit_completed(&mut self, player: usize) {
        if self.legend[player].kind == LegendKind::Rengar {
            self.queue_trigger_choice(
                player,
                self.legend_def(player).default_champion,
                crate::game::Decision::LegendTargets { player },
            );
        }
    }

    pub(crate) fn choose_legend_targets(
        &mut self,
        player: usize,
        targets: &[Target],
    ) -> Result<(), String> {
        if self.legend[player].kind != LegendKind::Rengar || targets.len() != 1 {
            return Err("Rengar must choose one unit for the Might bonus.".into());
        }
        let target = targets[0];
        if !self.legend_target_valid(player, target) {
            return Err("That unit cannot be chosen by this legend ability.".into());
        }
        self.spend_deflect(player, false, targets)?;
        self.decision = None;
        self.queue_ability(
            player,
            self.legend_def(player).default_champion,
            targets.to_vec(),
            ChainAbility::Legend(LegendAbility::RengarMight { target }),
        );
        Ok(())
    }

    pub(crate) fn legend_on_spell_completed(&mut self, player: usize, card_id: usize) {
        if self.legend[player].kind == LegendKind::Lux && self.catalog[card_id].cost >= 5 {
            self.queue_ability(
                player,
                self.legend_def(player).default_champion,
                Vec::new(),
                ChainAbility::Legend(LegendAbility::LuxDraw),
            );
        }
    }
    /// Derived bonuses must never be written into the printed or temporary
    /// might, because moving or adding another defender can remove them.
    pub(crate) fn legend_might_bonus(&self, unit: &Unit) -> u8 {
        let Some(window) = &self.showdown else {
            return 0;
        };
        if !window.combat || unit.lane != Some(window.lane) {
            return 0;
        }
        let Some(owner) = self
            .players
            .iter()
            .position(|player| player.units.iter().any(|candidate| candidate.id == unit.id))
        else {
            return 0;
        };
        if owner != window.attacker {
            if self.legend[owner].kind == LegendKind::MasterYi
                && self.lane_count(owner, window.lane) == 1
            {
                2
            } else {
                0
            }
        } else {
            // Granted Assault is combined with printed Assault by unit_keywords.
            0
        }
    }

    pub(crate) fn legend_on_choose(&self, player: usize, targets: &[Target]) -> Vec<LegendTrigger> {
        if self.legend[player].kind != LegendKind::Irelia {
            return Vec::new();
        }
        let mut triggers = Vec::new();
        for target in targets {
            if let Target::Unit {
                player: owner,
                unit_id,
            } = *target
                && owner == player
                && self.unit_ref(*target).is_some()
            {
                let trigger = LegendTrigger::IreliaChoose { unit_id };
                triggers.push(trigger);
            }
        }
        triggers
    }

    pub(crate) fn legend_on_conquer(&self, player: usize) -> Option<LegendTrigger> {
        (self.legend[player].kind == LegendKind::Irelia).then_some(LegendTrigger::IreliaConquer)
    }

    pub(crate) fn legend_on_offhand_play(&self, player: usize) -> Option<LegendAbility> {
        (self.legend[player].kind == LegendKind::Kennen).then_some(LegendAbility::KennenEmpower)
    }

    pub(crate) fn legend_begin_turn(&mut self, player: usize) {
        self.legend[player].ready = true;
        // Pools empty at the beginning of a main phase, in addition to cleanup.
        for legend in &mut self.legend {
            legend.spell_power = 0;
        }
    }

    pub(crate) fn legend_end_turn(&mut self) {
        for legend in &mut self.legend {
            legend.spell_power = 0;
            legend.assault_grants.clear();
        }
    }

    pub fn legend_target_valid(&self, actor: usize, target: Target) -> bool {
        if actor >= 2 {
            return false;
        }
        let Target::Unit { player, .. } = target else {
            return false;
        };
        let Some(unit) = self.unit_ref(target) else {
            return false;
        };
        if player == actor {
            return true;
        }
        if unit.protected || (!unit.token && unit.card_id == 31) {
            return false;
        }
        if !unit.token
            && unit.card_id == 2
            && !self
                .showdown
                .as_ref()
                .is_some_and(|window| unit.lane == Some(window.lane) && window.combat)
        {
            return false;
        }
        true
    }

    fn validate_legend_activation(
        &self,
        player: usize,
        target: Option<Target>,
    ) -> Result<LegendActivation, String> {
        if self.phase != Phase::Playing || player != self.priority() {
            return Err("Wait for priority before activating your legend.".into());
        }
        let activation = legend_activation(self.legend[player].kind).ok_or(
            "This legend has no activated ability; its printed effects apply automatically.",
        )?;
        let adds = matches!(
            activation.effect,
            LegendActivationEffect::SpellPower | LegendActivationEffect::LegionEnergy
        );
        if self.decision.is_some() && !(self.resource_payment_pending() && adds) {
            return Err("Finish the current choice before activating your legend.".into());
        }
        if activation.exhaust && !self.legend[player].ready {
            return Err("Your legend is exhausted.".into());
        }
        if activation.disempower && !self.legend[player].empowered {
            return Err("Your legend must be empowered to pay this activation cost.".into());
        }
        if self.legend[player].kind == LegendKind::Ezreal && self.ezreal_progress(player) < 2 {
            return Err("Choose enemy units or gear twice this turn with spells or unit abilities before using Ezreal.".into());
        }
        let allowed = match activation.timing {
            LegendTiming::Main => {
                self.is_main_phase()
                    && player == self.active
                    && self.chain.is_empty()
                    && self.showdown.is_none()
            }
            LegendTiming::Action => {
                self.chain.is_empty()
                    && (self.showdown.is_some() || (self.is_main_phase() && player == self.active))
            }
            LegendTiming::Reaction => true,
        };
        if !allowed {
            return Err(
                "This legend ability cannot be activated in the current timing window.".into(),
            );
        }
        if self.payment_energy_available(player) < activation.energy.into() {
            return Err(format!(
                "Pay {} energy to activate this legend.",
                activation.energy
            ));
        }
        if activation.targets.max > 0 {
            let target = target.ok_or("Choose a unit for this legend ability.")?;
            if !self.legend_target_valid(player, target) {
                return Err("That unit cannot be chosen by this legend ability.".into());
            }
            if self.available_power(player, None)
                < u16::from(activation.power) + u16::from(self.deflect_cost(player, &[target])?)
            {
                return Err("Choosing that unit requires additional power for Deflect.".into());
            }
        } else if target.is_some() {
            return Err("This legend ability does not choose a target.".into());
        }
        Ok(activation)
    }

    pub(crate) fn activate_legend(&mut self, target: Option<Target>) -> Result<(), String> {
        let player = self.priority();
        let activation = self.validate_legend_activation(player, target)?;
        self.pay_action_cost(&Action::ActivateLegend { target })?;
        if activation.exhaust {
            self.legend[player].ready = false;
        }
        if activation.disempower {
            self.disempower_by(player, crate::empower::BoardObject::Legend { player });
        }
        match activation.effect {
            LegendActivationEffect::Draw => {
                self.queue_ability(
                    player,
                    self.legend_def(player).default_champion,
                    Vec::new(),
                    ChainAbility::Legend(LegendAbility::EzrealDraw),
                );
            }
            LegendActivationEffect::Ready => {
                let target = target.expect("validated target");
                self.queue_ability(
                    player,
                    self.legend_def(player).default_champion,
                    vec![target],
                    ChainAbility::Legend(LegendAbility::MatriarchReady { target }),
                );
            }
            LegendActivationEffect::SpellPower => {
                self.legend[player].spell_power = self.legend[player].spell_power.saturating_add(1);
                self.push_log(format!(
                    "{} exhausts Kai'Sa's legend: add 1 flexible power for spells only.",
                    self.players[player].name
                ));
            }
            LegendActivationEffect::Assault => {
                let target = target.expect("validated target");
                self.queue_ability(
                    player,
                    self.legend_def(player).default_champion,
                    vec![target],
                    ChainAbility::Legend(LegendAbility::KennenAssault { target }),
                );
                self.push_log(format!(
                    "{} activates Kennen's legend. Assault 2 waits on the chain.",
                    self.players[player].name
                ));
            }
            LegendActivationEffect::LegionEnergy => {
                if self.cards_played_this_turn(player) > 0 {
                    self.players[player].energy_pool += 1;
                    self.players[player].energy += 1;
                    self.push_log(format!(
                        "{} exhausts Hand of Noxus: add 1 energy.",
                        self.players[player].name
                    ));
                } else {
                    self.push_log(format!(
                        "{} exhausts Hand of Noxus; Legion is inactive, so it adds no energy.",
                        self.players[player].name
                    ));
                }
            }
            LegendActivationEffect::Recruit => {
                self.queue_ability(
                    player,
                    self.legend_def(player).default_champion,
                    Vec::new(),
                    ChainAbility::Legend(LegendAbility::ViktorRecruit),
                );
            }
        }
        Ok(())
    }

    pub(crate) fn legend_ai_score(&self, player: usize, target: Option<Target>) -> Option<i32> {
        match self.legend[player].kind {
            LegendKind::Darius if self.cards_played_this_turn(player) == 0 => None,
            LegendKind::Ambessa => {
                let Target::Unit { player: owner, .. } = target? else {
                    return None;
                };
                let unit = self.unit_ref(target?)?;
                if owner != player || unit.ready {
                    return None;
                }
                Some(
                    if self.players[player].score >= 7
                        && (0..2).any(|lane| {
                            !self.player_scored_this_turn[player][lane]
                                && self.battlefield_owner[lane] != Some(player)
                                && self.lane_count(1 - player, lane) == 0
                        })
                    {
                        180
                    } else {
                        65
                    },
                )
            }
            _ => Some(35),
        }
    }

    /// Legal activations for this seat, including target and payment validation.
    pub fn legend_actions(&self, player: usize) -> Vec<Action> {
        if player >= self.players.len() {
            return Vec::new();
        }
        let mut targets = vec![None];
        if legend_activation(self.legend[player].kind).is_some_and(|a| a.targets.max > 0) {
            targets = self
                .players
                .iter()
                .enumerate()
                .flat_map(|(owner, state)| {
                    state.units.iter().map(move |unit| {
                        Some(Target::Unit {
                            player: owner,
                            unit_id: unit.id,
                        })
                    })
                })
                .collect();
        }
        targets
            .into_iter()
            .filter(|&target| self.validate_legend_activation(player, target).is_ok())
            .map(|target| Action::ActivateLegend { target })
            .collect()
    }

    pub(crate) fn resolve_legend_choice(
        &mut self,
        player: usize,
        trigger: LegendTrigger,
        accept: bool,
    ) -> Result<(), String> {
        if !accept {
            return Ok(());
        }
        if self.legend[player].kind != LegendKind::Irelia {
            return Err("This choice belongs to Irelia's legend.".into());
        }
        match trigger {
            LegendTrigger::IreliaChoose { unit_id } => {
                if !self.legend[player].ready {
                    return Err("Irelia's legend must be ready to pay this cost.".into());
                }
                if self.unit_ref(Target::Unit { player, unit_id }).is_none() {
                    return Err("The chosen friendly unit has left the board.".into());
                }
                self.spend_power(player, None, 1)?;
                self.legend[player].ready = false;
                // “It” references the original choice. It is not chosen again.
                self.queue_ability(
                    player,
                    0,
                    Vec::new(),
                    ChainAbility::Legend(LegendAbility::IreliaReady { unit_id }),
                );
                self.push_log(format!(
                    "{} pays 1 power and exhausts Irelia's legend to ready the chosen unit.",
                    self.players[player].name
                ));
            }
            LegendTrigger::IreliaConquer => {
                if self.payment_energy_available(player) < 1 {
                    return Err("Pay 1 energy to ready Irelia's legend.".into());
                }
                self.pay_energy(player, 1);
                self.queue_ability(
                    player,
                    0,
                    Vec::new(),
                    ChainAbility::Legend(LegendAbility::IreliaRefresh),
                );
                self.push_log(format!(
                    "{} pays 1 energy to ready Irelia's legend.",
                    self.players[player].name
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn resolve_legend_ability(
        &mut self,
        player: usize,
        ability: LegendAbility,
    ) -> Result<(), String> {
        match ability {
            LegendAbility::IreliaReady { unit_id } => {
                self.ready_unit(player, unit_id);
            }
            LegendAbility::IreliaRefresh => self.legend[player].ready = true,
            LegendAbility::KennenEmpower | LegendAbility::MatriarchEmpower => {
                self.empower_by(player, crate::empower::BoardObject::Legend { player });
            }
            LegendAbility::MatriarchReady { target } => {
                if self.legend_target_valid(player, target)
                    && let Target::Unit {
                        player: owner,
                        unit_id,
                    } = target
                {
                    self.ready_unit_by(player, owner, unit_id);
                }
            }
            LegendAbility::KennenAssault { target } => {
                if self.legend_target_valid(player, target)
                    && let Target::Unit { unit_id, .. } = target
                {
                    self.legend[player].assault_grants.push((unit_id, 2));
                }
            }
            LegendAbility::LuxDraw | LegendAbility::EzrealDraw => self.draw(player, 1),
            LegendAbility::RengarMight { target } => {
                if self.legend_target_valid(player, target) {
                    self.modify_might(target, 1, 0)?;
                }
            }
            LegendAbility::ViktorRecruit => {
                // Stable Recruit token-characteristics reference, independent
                // of this legend's chosen champion or presentation artwork.
                self.queue_token_play(player, 87, 1, None, false);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cards::{CardKind, Domain},
        game::{Decision, Difficulty, Showdown, domain_index},
    };

    fn game(deck: usize) -> Game {
        let mut game = Game::new(deck, (deck + 1) % 4, 91, Difficulty::Tactical);
        game.start_match().unwrap();
        game
    }

    fn unit(id: u32, lane: Option<usize>) -> Unit {
        Unit {
            id,
            card_id: 25,
            lane,
            ready: false,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        }
    }

    fn rich(game: &mut Game, player: usize) {
        let state = &mut game.players[player];
        state.runes = [2; 6];
        state.rune_deck.clear();
        state.exhausted_runes = [0; 6];
        state.energy = 12;
    }

    fn settle(game: &mut Game) {
        for _ in 0..30 {
            if let Some(Decision::CombatDamage {
                player,
                lane,
                total,
            }) = game.decision
            {
                game.apply(Action::AssignDamage {
                    assignments: game
                        .automatic_damage_assignment(player, lane, total)
                        .unwrap(),
                })
                .unwrap();
            } else if game.decision.is_some() {
                game.apply(Action::Choose { accept: false }).unwrap();
            } else if !game.chain.is_empty() {
                game.apply(Action::Pass).unwrap();
            } else {
                return;
            }
        }
        panic!("legend chain failed to finish");
    }

    #[test]
    fn yi_bonus_is_derived_from_defending_alone_and_disappears_for_a_group() {
        let mut game = game(1);
        game.players[0].units.push(unit(100, Some(0)));
        game.showdown = Some(Showdown {
            combat: true,
            lane: 0,
            attacker: 1,
            focus: 1,
            passes: 0,
        });
        let printed = u64::from(game.catalog[25].might);
        assert_eq!(
            game.unit_might(&game.players[0].units[0]).unwrap(),
            printed + 2
        );
        game.players[0].units.push(unit(101, Some(0)));
        assert_eq!(game.unit_might(&game.players[0].units[0]).unwrap(), printed);
        game.players[0].units.pop();
        game.showdown.as_mut().unwrap().attacker = 0;
        assert_eq!(game.unit_might(&game.players[0].units[0]).unwrap(), printed);
        game.showdown = None;
        assert_eq!(game.unit_might(&game.players[0].units[0]).unwrap(), printed);
    }

    #[test]
    fn a_neutral_showdown_does_not_assign_combat_bonuses() {
        let mut game = game(2);
        game.players[0].units.push(unit(100, Some(0)));
        game.legend[0].assault_grants.push((100, 2));
        game.showdown = Some(Showdown {
            lane: 0,
            attacker: 0,
            focus: 0,
            passes: 0,
            combat: false,
        });
        assert_eq!(
            game.unit_might(&game.players[0].units[0]).unwrap(),
            u64::from(game.catalog[25].might)
        );
        game.showdown.as_mut().unwrap().combat = true;
        assert_eq!(
            game.unit_might(&game.players[0].units[0]).unwrap(),
            u64::from(game.catalog[25].might) + 2
        );
    }

    #[test]
    fn kaisa_add_is_immediate_preserves_priority_and_is_spell_only() {
        let mut game = game(3);
        let old_priority = game.priority();
        game.apply(Action::ActivateLegend { target: None }).unwrap();
        assert_eq!(game.priority(), old_priority);
        assert!(game.chain.is_empty());
        assert!(!game.legend[0].ready);
        assert_eq!(game.legend[0].spell_power, 1);
        assert!(game.apply(Action::ActivateLegend { target: None }).is_err());
        // Flexible generated power must not become a recycled rune or energy.
        let state = &mut game.players[0];
        state.runes = [0; 6];
        state.runes[domain_index(Domain::Mind)] = 10;
        state.exhausted_runes = [0; 6];
        state.energy = 10;
        state.power_pool = [0; 6];
        state.any_power = 0;
        assert_eq!(game.catalog[55].kind, CardKind::Spell);
        assert!(
            game.can_afford(0, &game.catalog[55]).unwrap(),
            "spell can use Kai'Sa flexible power"
        );
        assert!(
            !game.can_afford(0, &game.catalog[54]).unwrap(),
            "unit cannot use spell-only power"
        );
        game.legend_end_turn();
        assert_eq!(game.legend[0].spell_power, 0);
        game.legend_begin_turn(0);
        assert!(game.legend[0].ready);
    }

    #[test]
    fn kaisa_can_generate_spell_power_during_fizz_replay_payment() {
        let mut game = game(3);
        game.players[0].runes = [0; 6];
        game.players[0].exhausted_runes = [0; 6];
        game.players[0].energy = 0;
        game.players[0].power_pool = [0; 6];
        game.players[0].any_power = 0;
        game.players[0].units.push(unit(100, Some(0)));
        game.players[0].discard = vec![14];
        let choice = Decision::ReplaySpell { player: 0 };
        game.decision = Some(choice.clone());
        let replay = Action::PlayFromTrash {
            trash_index: 0,
            lane: None,
            target: Some(Target::Unit {
                player: 0,
                unit_id: 100,
            }),
        };
        assert!(game.apply(replay.clone()).is_err());
        let add = Action::ActivateLegend { target: None };
        assert!(game.legend_actions(0).contains(&add));
        game.apply(add).unwrap();
        assert_eq!(game.decision, Some(choice));
        assert_eq!(game.priority(), 0);
        assert!(game.chain.is_empty());
        assert_eq!(game.legend[0].spell_power, 1);
        game.apply(replay).unwrap();
        assert!(game.decision.is_none());
        assert_eq!(game.legend[0].spell_power, 0);
        assert_eq!(game.chain.len(), 1);
    }

    #[test]
    fn resource_activation_preserves_last_rites_choice_but_cannot_pay_unit_power() {
        let mut game = game(3);
        game.players[0].runes = [0; 6];
        game.players[0].runes[domain_index(Domain::Mind)] = 3;
        game.players[0].exhausted_runes = [0; 6];
        game.players[0].energy = 3;
        game.players[0].power_pool = [0; 6];
        game.players[0].any_power = 0;
        game.players[0].discard = vec![35]; // Ezreal still needs Chaos power.
        let choice = Decision::LastRites {
            player: 0,
            gear_id: 100,
        };
        game.decision = Some(choice.clone());
        game.apply(Action::ActivateLegend { target: None }).unwrap();
        assert_eq!(game.decision, Some(choice.clone()));
        assert!(
            game.apply(Action::PlayFromTrash {
                trash_index: 0,
                lane: None,
                target: None,
            })
            .is_err()
        );
        assert_eq!(game.decision, Some(choice));
        assert_eq!(game.legend[0].spell_power, 1);
    }

    #[test]
    fn pending_payment_does_not_permit_non_resource_legend_abilities() {
        let mut game = game(2);
        game.legend[0].empowered = true;
        game.players[0].units.push(unit(100, None));
        game.decision = Some(Decision::ReplaySpell { player: 0 });
        assert!(game.legend_actions(0).is_empty());
        assert!(
            game.apply(Action::ActivateLegend {
                target: Some(Target::Unit {
                    player: 0,
                    unit_id: 100,
                }),
            })
            .is_err()
        );
        assert!(game.legend[0].ready);
        assert!(game.legend[0].empowered);
    }

    #[test]
    fn kennen_activation_is_respondable_and_assault_only_applies_while_attacking() {
        let mut game = game(2);
        game.players[0].units.push(unit(100, Some(0)));
        game.legend[0].empowered = true;
        let target = Target::Unit {
            player: 0,
            unit_id: 100,
        };
        game.apply(Action::ActivateLegend {
            target: Some(target),
        })
        .unwrap();
        assert!(!game.legend[0].ready && !game.legend[0].empowered);
        assert!(matches!(
            game.chain.last().unwrap().ability,
            Some(ChainAbility::Legend(LegendAbility::KennenAssault { .. }))
        ));
        assert_eq!(
            game.unit_might(&game.players[0].units[0]).unwrap(),
            u64::from(game.catalog[25].might)
        );
        settle(&mut game);
        game.showdown = Some(Showdown {
            combat: true,
            lane: 0,
            attacker: 0,
            focus: 0,
            passes: 0,
        });
        assert_eq!(
            game.unit_might(&game.players[0].units[0]).unwrap(),
            u64::from(game.catalog[25].might) + 2
        );
        game.showdown.as_mut().unwrap().attacker = 1;
        assert_eq!(
            game.unit_might(&game.players[0].units[0]).unwrap(),
            u64::from(game.catalog[25].might)
        );
        game.legend_end_turn();
        assert!(game.legend[0].assault_grants.is_empty());
    }

    #[test]
    fn kennen_can_choose_an_enemy_and_cannot_activate_inside_a_closed_chain() {
        let mut game = game(2);
        game.players[1].units.push(unit(100, None));
        game.legend[0].empowered = true;
        game.apply(Action::ActivateLegend {
            target: Some(Target::Unit {
                player: 1,
                unit_id: 100,
            }),
        })
        .unwrap();
        game.legend[0].ready = true;
        game.legend[0].empowered = true;
        let before = game.legend[0].clone();
        assert!(
            game.apply(Action::ActivateLegend {
                target: Some(Target::Unit {
                    player: 1,
                    unit_id: 100
                })
            })
            .is_err()
        );
        assert_eq!(
            game.legend[0], before,
            "illegal activation must not spend costs"
        );
    }

    #[test]
    fn kennen_pays_enemy_deflect_and_cannot_use_spell_only_power_for_it() {
        let mut game = game(2);
        let mut enemy = unit(100, None);
        enemy.card_id = 0; // Irelia has Deflect.
        game.players[1].units.push(enemy);
        game.legend[0].empowered = true;
        game.players[0].runes = [0; 6];
        game.players[0].exhausted_runes = [0; 6];
        game.players[0].energy = 0;
        game.legend[0].spell_power = 1;
        let action = Action::ActivateLegend {
            target: Some(Target::Unit {
                player: 1,
                unit_id: 100,
            }),
        };
        assert!(game.apply(action.clone()).is_err());
        assert!(game.legend[0].ready && game.legend[0].empowered);
        game.players[0].any_power = 1;
        game.apply(action).unwrap();
        assert_eq!(game.players[0].any_power, 0);
        assert_eq!(game.legend[0].spell_power, 1);
        assert!(!game.legend[0].ready && !game.legend[0].empowered);
    }

    #[test]
    fn irelia_choice_costs_finalize_then_ready_resolves_on_chain() {
        let mut game = game(0);
        rich(&mut game, 0);
        game.players[0].units.push(unit(100, None));
        let target = Target::Unit {
            player: 0,
            unit_id: 100,
        };
        let triggers = game.legend_on_choose(0, &[target, target]);
        assert_eq!(
            triggers,
            vec![
                LegendTrigger::IreliaChoose { unit_id: 100 },
                LegendTrigger::IreliaChoose { unit_id: 100 }
            ]
        );
        let runes = game.players[0].total_runes();
        game.resolve_legend_choice(0, triggers[0], true).unwrap();
        assert_eq!(game.players[0].total_runes(), runes - 1);
        assert!(!game.legend[0].ready);
        assert!(!game.players[0].units[0].ready, "ready waits for responses");
        assert_eq!(game.chain.len(), 1);
        settle(&mut game);
        assert!(game.players[0].units[0].ready);
        assert!(
            game.decision.is_none(),
            "referential ready does not choose the unit again"
        );
    }

    #[test]
    fn irelia_decline_and_unpayable_accept_leave_resources_unchanged() {
        let mut game = game(0);
        game.players[0].units.push(unit(100, None));
        let trigger = LegendTrigger::IreliaChoose { unit_id: 100 };
        let before = game.players[0].clone();
        game.resolve_legend_choice(0, trigger, false).unwrap();
        assert_eq!(game.players[0], before);
        game.players[0].runes = [0; 6];
        game.players[0].energy = 0;
        let before = game.players[0].clone();
        assert!(game.resolve_legend_choice(0, trigger, true).is_err());
        assert_eq!(game.players[0], before);
        assert!(game.legend[0].ready && game.chain.is_empty());
    }

    #[test]
    fn irelia_conquer_refresh_costs_energy_and_remains_respondable() {
        let mut game = game(0);
        game.legend[0].ready = false;
        let energy = game.players[0].energy;
        assert_eq!(
            game.legend_on_conquer(0),
            Some(LegendTrigger::IreliaConquer)
        );
        game.resolve_legend_choice(0, LegendTrigger::IreliaConquer, true)
            .unwrap();
        assert_eq!(game.players[0].energy, energy - 1);
        assert!(!game.legend[0].ready);
        settle(&mut game);
        assert!(game.legend[0].ready);
    }

    #[test]
    fn champion_play_triggers_kennen_empower_without_counting_an_extra_card() {
        let mut game = game(2);
        rich(&mut game, 0);
        game.apply(Action::PlayChampion {
            lane: None,
            target: None,
        })
        .unwrap();
        assert!(game.players[0].champion.is_none());
        while matches!(game.decision, Some(Decision::OrderTriggers { .. })) {
            let action = game.legal_actions().unwrap()[0].clone();
            game.apply(action).unwrap();
        }
        assert!(game.chain.iter().any(|item| matches!(
            item.ability,
            Some(ChainAbility::Legend(LegendAbility::KennenEmpower))
        )));
        settle(&mut game);
        assert!(game.legend[0].empowered);
    }

    #[test]
    fn chosen_friendly_card_opens_optional_irelia_trigger_above_the_spell() {
        let mut game = game(0);
        rich(&mut game, 0);
        game.players[0].units.push(unit(100, None));
        game.players[0].hand = vec![8]; // En Garde chooses a friendly unit.
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: Some(Target::Unit {
                player: 0,
                unit_id: 100,
            }),
        })
        .unwrap();
        assert!(matches!(
            game.decision,
            Some(Decision::Legend {
                player: 0,
                trigger: LegendTrigger::IreliaChoose { unit_id: 100 }
            })
        ));
        game.apply(Action::Choose { accept: true }).unwrap();
        assert_eq!(game.chain.len(), 2);
        assert!(matches!(
            game.chain.last().unwrap().ability,
            Some(ChainAbility::Legend(LegendAbility::IreliaReady {
                unit_id: 100
            }))
        ));
    }
}
