//! The four preset legends' printed powers.
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
}

impl LegendKind {
    pub fn champion_id(self) -> usize {
        match self {
            Self::Irelia => 0,
            Self::MasterYi => 22,
            Self::Kennen => 33,
            Self::KaiSa => 49,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegendState {
    pub kind: LegendKind,
    pub ready: bool,
    pub empowered: bool,
    pub spell_power: u8,
    /// Each granted Assault instance lasts through the current turn. The unit
    /// identifier follows its owner even if that owner differs from the legend's.
    pub assault_grants: Vec<(u32, u8)>,
}

impl LegendState {
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
    KennenAssault { target: Target },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegendTrigger {
    IreliaChoose { unit_id: u32 },
    IreliaConquer,
}

impl Game {
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

    fn legend_target_valid(&self, actor: usize, target: Target) -> bool {
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

    pub(crate) fn activate_legend(&mut self, target: Option<Target>) -> Result<(), String> {
        if self.phase != Phase::Playing {
            return Err("Finish the current choice before activating your legend.".into());
        }
        let player = self.priority();
        if self.decision.is_some()
            && !(self.resource_payment_pending() && self.legend[player].kind == LegendKind::KaiSa)
        {
            return Err("Finish the current choice before activating your legend.".into());
        }
        if !self.legend[player].ready {
            return Err("Your legend is exhausted.".into());
        }
        match self.legend[player].kind {
            LegendKind::KaiSa => {
                if target.is_some() { return Err("Kai'Sa's resource ability does not choose a target.".into()); }
                self.legend[player].ready = false;
                self.legend[player].spell_power = self.legend[player].spell_power.saturating_add(1);
                self.push_log(format!("{} exhausts Kai'Sa's legend: add 1 flexible power for spells only.", self.players[player].name));
                // An Add ability neither joins the chain nor passes priority.
                Ok(())
            }
            LegendKind::Kennen => {
                if !self.legend[player].empowered { return Err("Empower Kennen's legend by playing a card from outside your hand first.".into()); }
                if !self.chain.is_empty() { return Err("Kennen's Action ability requires an open state.".into()); }
                if self.showdown.is_none() && player != self.active { return Err("Wait for your turn or an open showdown.".into()); }
                let target = target.ok_or("Choose a unit for Kennen's Assault ability.")?;
                if !self.legend_target_valid(player, target) { return Err("That unit cannot be chosen by this legend ability.".into()); }
                let deflect = self.deflect_cost(player, &[target]);
                if self.available_power(player, None) < deflect { return Err("Choosing that unit requires additional power for Deflect.".into()); }
                self.spend_power(player, None, deflect)?;
                self.legend[player].ready = false;
                self.legend[player].empowered = false;
                self.queue_ability(player, 33, vec![target], ChainAbility::Legend(LegendAbility::KennenAssault { target }));
                self.push_log(format!("{} activates Kennen's legend. Assault 2 waits on the chain.", self.players[player].name));
                Ok(())
            }
            LegendKind::Irelia => Err("Irelia's legend offers optional choices when you choose a friendly unit or conquer.".into()),
            LegendKind::MasterYi => Err("Master Yi's legend is passive: a friendly unit defending alone gets +2 might.".into()),
        }
    }

    pub(crate) fn legend_actions(&self, player: usize) -> Vec<Action> {
        if self.phase != Phase::Playing
            || self.priority() != player
            || (self.decision.is_some()
                && !(self.resource_payment_pending()
                    && self.legend[player].kind == LegendKind::KaiSa))
            || !self.legend[player].ready
        {
            return Vec::new();
        }
        match self.legend[player].kind {
            LegendKind::KaiSa => vec![Action::ActivateLegend { target: None }],
            LegendKind::Kennen
                if self.legend[player].empowered
                    && self.chain.is_empty()
                    && (self.showdown.is_some() || player == self.active) =>
            {
                self.players
                    .iter()
                    .enumerate()
                    .flat_map(|(owner, state)| {
                        state.units.iter().map(move |unit| Target::Unit {
                            player: owner,
                            unit_id: unit.id,
                        })
                    })
                    .filter(|target| {
                        self.legend_target_valid(player, *target)
                            && self.available_power(player, None)
                                >= self.deflect_cost(player, &[*target])
                    })
                    .map(|target| Action::ActivateLegend {
                        target: Some(target),
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
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
                if self.players[player].energy < 1 {
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

    pub(crate) fn resolve_legend_ability(&mut self, player: usize, ability: LegendAbility) {
        match ability {
            LegendAbility::IreliaReady { unit_id } => {
                self.ready_unit(player, unit_id);
            }
            LegendAbility::IreliaRefresh => self.legend[player].ready = true,
            LegendAbility::KennenEmpower => self.legend[player].empowered = true,
            LegendAbility::KennenAssault { target } => {
                if self.legend_target_valid(player, target)
                    && let Target::Unit { unit_id, .. } = target
                {
                    self.legend[player].assault_grants.push((unit_id, 2));
                }
            }
        }
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
                    assignments: game.automatic_damage_assignment(player, lane, total),
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
        let printed = game.catalog[25].might;
        assert_eq!(game.unit_might(&game.players[0].units[0]), printed + 2);
        game.players[0].units.push(unit(101, Some(0)));
        assert_eq!(game.unit_might(&game.players[0].units[0]), printed);
        game.players[0].units.pop();
        game.showdown.as_mut().unwrap().attacker = 0;
        assert_eq!(game.unit_might(&game.players[0].units[0]), printed);
        game.showdown = None;
        assert_eq!(game.unit_might(&game.players[0].units[0]), printed);
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
            game.unit_might(&game.players[0].units[0]),
            game.catalog[25].might
        );
        game.showdown.as_mut().unwrap().combat = true;
        assert_eq!(
            game.unit_might(&game.players[0].units[0]),
            game.catalog[25].might + 2
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
            game.can_afford(0, &game.catalog[55]),
            "spell can use Kai'Sa flexible power"
        );
        assert!(
            !game.can_afford(0, &game.catalog[54]),
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
            game.unit_might(&game.players[0].units[0]),
            game.catalog[25].might
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
            game.unit_might(&game.players[0].units[0]),
            game.catalog[25].might + 2
        );
        game.showdown.as_mut().unwrap().attacker = 1;
        assert_eq!(
            game.unit_might(&game.players[0].units[0]),
            game.catalog[25].might
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
            let action = game.legal_actions()[0].clone();
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
