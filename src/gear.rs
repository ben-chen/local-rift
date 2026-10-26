//! Persistent gear, equipment and resource abilities.
//!
//! Printed rules come from the official catalog, the 2025-10-21 Zhonya erratum,
//! and Core Rules 149, 716-725, 816, 818-819. Attachments are separate objects:
//! losing a unit does not put its equipment in the trash. Attached equipment's
//! printed rules text is inactive; granted rules text remains active. Its might
//! bonus and effect text apply to the unit.

#[cfg(test)]
use crate::cards::Domain;
use crate::cards::{CardId, CardKind};
use crate::game::{
    Action, ChainAbility, Game, Phase, Target, TriggeredAbility, UnitKeywords, domain_index,
};
use crate::rules::{self, StaticAbility};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GearState {
    pub id: u32,
    pub card_id: CardId,
    /// Independent physical location; attached gear follows its bearer instead.
    /// Detaching preserves the bearer's location until the next cleanup Recall.
    pub lane: Option<usize>,
    /// Attachment does not erase this object's own identity or characteristics.
    pub attached_to: Option<u32>,
    pub ready: bool,
    pub temporary: bool,
    /// The only current gear token is Gold. Its source card is not a real card
    /// in this object's zone and must not be counted or returned to the trash.
    pub token: bool,
}

impl GearState {
    pub fn is_gold(&self) -> bool {
        self.token
    }
}

/// Temporary's characteristic and its active expiry ability are distinct:
/// attachment suppresses printed text, but not a granted ability (CR135.4.b).
pub type GearTemporaryStatus = crate::temporary::TemporaryStatus;

/// Characteristics remain referenceable when attachment suppresses printed text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GearDeployStatus {
    pub characteristic: bool,
    pub printed_abilities_active: bool,
    pub opponent_hold_trigger_active: bool,
}

impl Game {
    pub fn gear_deploy_status(&self, player: usize, gear_id: u32) -> Option<GearDeployStatus> {
        let gear = self
            .players
            .get(player)?
            .gear
            .iter()
            .find(|g| g.id == gear_id)?;
        let characteristic =
            !gear.token && rules::static_abilities(gear.card_id).contains(&StaticAbility::Deploy);
        let printed_abilities_active = gear.attached_to.is_none();
        Some(GearDeployStatus {
            characteristic,
            printed_abilities_active,
            opponent_hold_trigger_active: characteristic && printed_abilities_active,
        })
    }

    pub fn gear_temporary_status(
        &self,
        player: usize,
        gear_id: u32,
    ) -> Option<GearTemporaryStatus> {
        self.temporary_status(crate::empower::BoardObject::Gear { player, gear_id })
    }

    pub fn gear_name(&self, gear: &GearState) -> &str {
        if gear.is_gold() {
            "Gold"
        } else {
            self.catalog[gear.card_id].name
        }
    }

    pub fn gear_location(&self, player: usize, gear_id: u32) -> Option<usize> {
        let gear = self
            .players
            .get(player)?
            .gear
            .iter()
            .find(|g| g.id == gear_id)?;
        match gear.attached_to {
            Some(unit_id) => {
                self.players[player]
                    .units
                    .iter()
                    .find(|unit| unit.id == unit_id)?
                    .lane
            }
            None => gear.lane,
        }
    }

    /// Resolve playing an already paid-for gear. Gear normally enters ready
    /// (CR149.1); an explicit entry replacement can make it enter exhausted.
    /// Quick-Draw's free attachment is a separate triggered chain ability.
    #[cfg(test)]
    pub(crate) fn gear_play(
        &mut self,
        player: usize,
        card_id: CardId,
        _target: Option<Target>,
    ) -> Result<(), String> {
        self.gear_play_at(player, card_id, None)
    }

    pub(crate) fn gear_play_at(
        &mut self,
        player: usize,
        card_id: CardId,
        lane: Option<usize>,
    ) -> Result<(), String> {
        if player >= self.players.len()
            || !self
                .catalog
                .get(card_id)
                .is_some_and(|c| c.kind == CardKind::Gear)
        {
            return Err("That is not a gear card.".into());
        }
        let deploy = rules::static_abilities(card_id).contains(&StaticAbility::Deploy);
        // Core811.1.d.1.a overrides ordinary gear's Base-only entry when
        // played from Hidden. Cleanup323.6 subsequently recalls non-Deploy gear.
        let hidden = self.trigger_state.hidden_play;
        if lane.is_some_and(|lane| lane >= crate::game::BATTLEFIELDS)
            || (deploy && lane.is_none())
            || (!deploy && lane.is_some() && !hidden)
        {
            return Err(if deploy {
                "Deploy gear must enter a battlefield.".into()
            } else {
                "This gear must enter its controller's base.".into()
            });
        }
        let id = self.next_unit_id;
        self.next_unit_id += 1;
        let ready = !rules::static_abilities(card_id).contains(&StaticAbility::EntersExhausted);
        self.players[player].gear.push(GearState {
            id,
            card_id,
            lane,
            attached_to: None,
            ready,
            temporary: rules::static_abilities(card_id).contains(&StaticAbility::Temporary),
            token: false,
        });
        self.push_log(format!(
            "{} plays {} to {}, {}.",
            self.players[player].name,
            self.catalog[card_id].name,
            lane.map_or_else(
                || "base".to_string(),
                |lane| format!("battlefield {}", lane + 1)
            ),
            if ready { "ready" } else { "exhausted" }
        ));
        if rules::equipment(card_id).is_some_and(|rule| rule.quick_draw) {
            self.request_on_play_target(player, card_id, id, None)?;
        }
        self.gear_play_completed(player, card_id, id, false);
        Ok(())
    }

    /// Pyke's actual Gold gear token. It begins exhausted when his text says so.
    pub(crate) fn gear_gold(&mut self, player: usize, exhausted: bool) -> u32 {
        let id = self.next_unit_id;
        self.next_unit_id += 1;
        self.players[player].gear.push(GearState {
            id,
            card_id: 13,
            lane: None,
            attached_to: None,
            ready: !exhausted,
            temporary: false,
            token: true,
        });
        self.push_log(format!(
            "{} creates a Gold gear token.",
            self.players[player].name
        ));
        self.gear_play_completed(player, 13, id, true);
        id
    }

    fn gear_friendly_target(&self, actor: usize, target: Target) -> bool {
        match target {
            Target::Unit { player, unit_id } if player == actor => self
                .players
                .get(player)
                .is_some_and(|p| p.units.iter().any(|unit| unit.id == unit_id)),
            _ => false,
        }
    }

    /// Validate before spending anything. Equip is an ordinary activated
    /// ability, including on a card whose Quick-Draw allows reaction *plays*.
    pub fn gear_can_equip(
        &self,
        player: usize,
        gear_id: u32,
        target: Target,
    ) -> Result<(), String> {
        if self.phase != Phase::Playing || player >= self.players.len() {
            return Err("Equipment can be used during a match.".into());
        }
        if player != self.priority() || player != self.active || self.in_window() {
            return Err("Equip during your main phase with no chain or showdown open.".into());
        }
        let gear = self.players[player]
            .gear
            .iter()
            .find(|gear| gear.id == gear_id)
            .ok_or("That gear is no longer on the board.")?;
        if gear.token || rules::equipment(gear.card_id).is_none() {
            return Err("This gear has no Equip ability.".into());
        }
        if gear.attached_to.is_some() {
            return Err("Attached equipment's Equip ability is inactive.".into());
        }
        if !self.gear_friendly_target(player, target) {
            return Err("Choose a unit you control to equip.".into());
        }
        let rule = rules::equipment(gear.card_id).unwrap();
        let state = &self.players[player];
        self.clone()
            .pay_action_cost(&Action::Equip { gear_id, target })?;
        if state.discard.len() < rule.recycle_trash as usize {
            return Err(format!(
                "Equip also requires recycling {} cards from your trash.",
                rule.recycle_trash
            ));
        }
        Ok(())
    }

    pub(crate) fn gear_equip(
        &mut self,
        player: usize,
        gear_id: u32,
        target: Target,
    ) -> Result<(), String> {
        self.gear_equip_with_trash(player, gear_id, target, &[])
    }

    pub(crate) fn gear_equip_with_trash(
        &mut self,
        player: usize,
        gear_id: u32,
        target: Target,
        trash_indices: &[usize],
    ) -> Result<(), String> {
        self.gear_can_equip(player, gear_id, target)?;
        let card_id = self.players[player]
            .gear
            .iter()
            .find(|gear| gear.id == gear_id)
            .unwrap()
            .card_id;
        let rule = rules::equipment(card_id).unwrap();
        if trash_indices.len() != rule.recycle_trash as usize {
            return Err(format!(
                "Choose exactly {} cards from your trash to recycle for this Equip cost.",
                rule.recycle_trash
            ));
        }
        let mut distinct = trash_indices.to_vec();
        distinct.sort_unstable();
        distinct.dedup();
        if distinct.len() != trash_indices.len()
            || distinct
                .iter()
                .any(|index| *index >= self.players[player].discard.len())
        {
            return Err("Choose different cards that are still in your trash.".into());
        }
        // Pay energy first so the same rune may be exhausted and then recycled.
        self.pay_action_cost(&Action::Equip { gear_id, target })?;
        if !distinct.is_empty() {
            self.recycle_cards(player, &distinct);
        }
        self.queue_ability(
            player,
            card_id,
            vec![target],
            ChainAbility::Equip { gear_id, target },
        );
        Ok(())
    }

    pub(crate) fn gear_resolve_equip(&mut self, player: usize, gear_id: u32, target: Target) {
        if !self.gear_friendly_target(player, target) {
            self.push_log("Equip has no legal target left and does not attach.".into());
            return;
        }
        let Target::Unit { unit_id, .. } = target else {
            return;
        };
        let Some(gear) = self.players[player]
            .gear
            .iter_mut()
            .find(|gear| gear.id == gear_id)
        else {
            self.push_log("Equip's gear left the board before the ability resolved.".into());
            return;
        };
        if gear.attached_to.is_some() || gear.token || rules::equipment(gear.card_id).is_none() {
            return;
        }
        gear.attached_to = Some(unit_id);
        let name = self.catalog[gear.card_id].name;
        self.push_log(format!("{} attaches to unit #{}.", name, unit_id));
    }

    pub fn gear_can_activate(&self, player: usize, gear_id: u32) -> bool {
        self.phase == Phase::Playing
            && player == self.priority()
            && (self.decision.is_none() || self.resource_payment_pending())
            && self.players.get(player).is_some_and(|state| {
                state.gear.iter().any(|gear| {
                    gear.id == gear_id
                        && gear.ready
                        && gear.attached_to.is_none()
                        && (gear.is_gold() || rules::resource_gear(gear.card_id).is_some())
                })
            })
    }

    /// Resource abilities resolve immediately and cannot be reacted to. These
    /// add floating resources; they never create or channel an extra rune.
    pub(crate) fn gear_activate(&mut self, player: usize, gear_id: u32) -> Result<(), String> {
        if !self.gear_can_activate(player, gear_id) {
            return Err("Choose a ready resource gear while you have priority.".into());
        }
        let index = self.players[player]
            .gear
            .iter()
            .position(|gear| gear.id == gear_id)
            .unwrap();
        let is_gold = self.players[player].gear[index].is_gold();
        let card_id = self.players[player].gear[index].card_id;
        self.players[player].gear[index].ready = false;
        if is_gold {
            self.gear_destroy(player, gear_id);
            self.players[player].any_power = self.players[player].any_power.saturating_add(1);
            self.push_log(format!(
                "{} spends Gold to add one Any power.",
                self.players[player].name
            ));
        } else {
            let ability = rules::resource_gear(card_id).expect("validated resource gear");
            self.players[player].energy_pool = self.players[player]
                .energy_pool
                .saturating_add(ability.energy.into());
            self.players[player].energy = self.players[player]
                .energy
                .saturating_add(ability.energy.into());
            if let Some(domain) = ability.domain {
                let slot = domain_index(domain);
                self.players[player].power_pool[slot] =
                    self.players[player].power_pool[slot].saturating_add(ability.power.into());
            } else {
                self.players[player].any_power = self.players[player]
                    .any_power
                    .saturating_add(ability.power.into());
            }
            self.push_log(format!(
                "{} exhausts {} to add {} energy and {} {}power.",
                self.players[player].name,
                self.catalog[card_id].name,
                ability.energy,
                ability.power,
                ability
                    .domain
                    .map_or(String::new(), |domain| format!("{} ", domain.name()))
            ));
        }
        self.play_history.gear_activations[player] += 1;
        self.gear_ability_used(player);
        Ok(())
    }

    /// Power available from legal immediate resource abilities. Energy-only
    /// gear cannot pay Deflect or any other power requirement.
    #[cfg(test)]
    pub(crate) fn gear_available_power(&self, player: usize) -> u8 {
        self.players.get(player).map_or(0, |state| {
            state
                .gear
                .iter()
                .filter(|gear| gear.ready && gear.attached_to.is_none())
                .map(|gear| {
                    if gear.is_gold() {
                        1
                    } else {
                        rules::resource_gear(gear.card_id).map_or(0, |ability| ability.power)
                    }
                })
                .fold(0_u8, u8::saturating_add)
        })
    }

    /// Only attached equipment contributes its effect-panel keywords. The
    /// equipment's own rules panel (including Quick-Draw) remains inactive.
    pub fn gear_keywords(&self, unit_id: u32) -> Result<UnitKeywords, String> {
        let mut keywords = UnitKeywords::default();
        for rule in self
            .players
            .iter()
            .flat_map(|state| &state.gear)
            .filter(|gear| !gear.token && gear.attached_to == Some(unit_id))
            .filter_map(|gear| rules::equipment(gear.card_id))
        {
            for ability in rule.grants {
                match *ability {
                    StaticAbility::Hunt(amount) => {
                        keywords.hunt = keywords.hunt.saturating_add(amount)
                    }
                    StaticAbility::Assault(amount) => {
                        keywords.assault =
                            crate::numeric::add_u64(keywords.assault, u64::from(amount), "assault")?
                    }
                    StaticAbility::Shield(amount) => {
                        keywords.shield =
                            crate::numeric::add_u64(keywords.shield, u64::from(amount), "shield")?
                    }
                    StaticAbility::Deflect(amount) => {
                        keywords.deflect = keywords.deflect.saturating_add(amount)
                    }
                    StaticAbility::Tank => keywords.tank = true,
                    StaticAbility::Backline => keywords.backline = true,
                    StaticAbility::Ganking => keywords.ganking = true,
                    _ => {}
                }
            }
        }
        Ok(keywords)
    }

    pub fn gear_might_bonus(&self, unit_id: u32) -> Result<u64, String> {
        self.players
            .iter()
            .flat_map(|player| &player.gear)
            .filter(|gear| gear.attached_to == Some(unit_id) && !gear.token)
            .filter_map(|gear| rules::equipment(gear.card_id))
            .try_fold(0_u64, |total, rule| {
                crate::numeric::add_u64(total, u64::from(rule.might), "equipment Might")
            })
    }

    pub fn gear_grants_ganking(&self, unit_id: u32) -> bool {
        self.players
            .iter()
            .flat_map(|p| &p.gear)
            .filter(|g| !g.token && g.attached_to == Some(unit_id))
            .filter_map(|g| rules::equipment(g.card_id))
            .any(|r| r.grants.contains(&StaticAbility::Ganking))
    }

    /// One replacement applies to a death. Prefer an attached Guardian Angel
    /// before a global Hourglass. Choosing between several applicable
    /// replacement effects remains a documented automation in this client.
    #[cfg(test)]
    pub(crate) fn gear_prevent_death(&mut self, player: usize, unit_id: u32) -> bool {
        if !self
            .players
            .get(player)
            .is_some_and(|p| p.units.iter().any(|unit| unit.id == unit_id))
        {
            return false;
        }
        let guardian = self.players[player]
            .gear
            .iter()
            .find(|gear| !gear.token && gear.card_id == 10 && gear.attached_to == Some(unit_id));
        let hourglass = self.players[player]
            .gear
            .iter()
            .find(|gear| !gear.token && gear.card_id == 21 && gear.attached_to.is_none());
        let Some(gear) = guardian.or(hourglass) else {
            return false;
        };
        let gear_id = gear.id;
        let card_id = gear.card_id;
        self.gear_destroy(player, gear_id);
        // The official October 2025 erratum explicitly adds healing to Zhonya.
        self.recall_unit(player, unit_id, true, true);
        self.push_log(format!(
            "{} prevents unit #{}'s death; it heals and recalls exhausted.",
            self.catalog[card_id].name, unit_id
        ));
        true
    }

    pub(crate) fn gear_replace_death(&mut self, player: usize, unit_id: u32, gear_id: u32) {
        let Some(gear) = self.players[player]
            .gear
            .iter()
            .find(|gear| gear.id == gear_id)
        else {
            return;
        };
        let card_id = gear.card_id;
        self.gear_destroy(player, gear_id);
        self.recall_unit(player, unit_id, true, true);
        self.push_log(format!(
            "{} prevents unit #{}'s death; it heals and recalls exhausted.",
            self.catalog[card_id].name, unit_id
        ));
    }

    /// Called on every actual unit departure (death, banish, return to hand).
    /// A recall or move is not a departure and retains attachments.
    pub(crate) fn gear_unit_left_board(&mut self, player: usize, unit_id: u32) {
        let lane = self.players[player]
            .units
            .iter()
            .find(|unit| unit.id == unit_id)
            .and_then(|unit| unit.lane);
        for gear in &mut self.players[player].gear {
            if gear.attached_to == Some(unit_id) {
                gear.lane = lane;
                gear.attached_to = None;
                // Do not ready or exhaust a detached gear: its state persists.
            }
        }
    }

    /// Core 323 step 5: non-Unit gear is recalled unless Deploy exempts it.
    /// Attached cards stay with their bearer; this is not an independent move.
    pub(crate) fn cleanup_gear_locations(&mut self) {
        for state in &mut self.players {
            for gear in &mut state.gear {
                if gear.attached_to.is_none()
                    && (gear.token
                        || !rules::static_abilities(gear.card_id).contains(&StaticAbility::Deploy))
                {
                    gear.lane = None;
                }
            }
        }
    }

    pub(crate) fn gear_destroy(&mut self, player: usize, gear_id: u32) -> bool {
        self.capture_gear_death(player, gear_id);
        self.remove_gear_from_board(player, gear_id)
    }

    /// Raw departure after the whole instruction has captured its death events.
    /// Callers must distinguish Kill from other departures before calling this.
    pub(crate) fn remove_gear_from_board(&mut self, player: usize, gear_id: u32) -> bool {
        let Some(state) = self.players.get_mut(player) else {
            return false;
        };
        let Some(index) = state.gear.iter().position(|gear| gear.id == gear_id) else {
            return false;
        };
        let gear = state.gear.remove(index);
        self.granted_temporary_gear.retain(|&id| id != gear_id);
        self.empowered_units.retain(|&id| id != gear_id);
        if !gear.token {
            state.discard.push(gear.card_id);
        }
        true
    }

    pub(crate) fn gear_make_temporary(&mut self, _actor: usize, _card_id: CardId, target: Target) {
        if let Target::Gear { player, gear_id } = target
            && let Some(gear) = self
                .players
                .get_mut(player)
                .and_then(|state| state.gear.iter_mut().find(|gear| gear.id == gear_id))
        {
            gear.temporary = true;
            if !self.granted_temporary_gear.contains(&gear_id) {
                self.granted_temporary_gear.push(gear_id);
            }
        }
    }

    pub(crate) fn gear_awaken(&mut self, player: usize) {
        for gear in &mut self.players[player].gear {
            gear.ready = true;
        }
    }

    #[cfg(test)]
    pub(crate) fn gear_beginning(&mut self, player: usize) {
        self.temporary_beginning(player);
    }

    #[cfg(test)]
    pub(crate) fn gear_begin_turn(&mut self, player: usize) {
        self.gear_awaken(player);
        self.gear_beginning(player);
        self.after_resolution().unwrap();
    }

    pub(crate) fn gear_conquer_hold(&mut self, player: usize, unit_id: u32) {
        let sources: Vec<_> = self.players[player]
            .gear
            .iter()
            .filter(|gear| !gear.token && gear.card_id == 38 && gear.attached_to == Some(unit_id))
            .map(|gear| gear.id)
            .collect();
        for gear_id in sources {
            self.queue_ability(
                player,
                38,
                Vec::new(),
                ChainAbility::Triggered(TriggeredAbility::LastRites { gear_id }),
            );
        }
    }

    /// Useful legal equipment actions for AI and UI. Last Rites' human action
    /// accepts any pair of trash indices; this candidate list supplies one
    /// legal pair to avoid generating a quadratic list for the AI each frame.
    pub fn gear_actions(&self, player: usize) -> Vec<Action> {
        let Some(state) = self.players.get(player) else {
            return Vec::new();
        };
        let mut actions = Vec::new();
        for gear in &state.gear {
            if self.gear_can_activate(player, gear.id) {
                actions.push(Action::ActivateGear { gear_id: gear.id });
            }
            if gear.token || rules::equipment(gear.card_id).is_none() {
                continue;
            }
            for unit in &state.units {
                let target = Target::Unit {
                    player,
                    unit_id: unit.id,
                };
                if self.gear_can_equip(player, gear.id, target).is_err() {
                    continue;
                }
                if gear.card_id == 38 {
                    actions.push(Action::EquipWithTrash {
                        gear_id: gear.id,
                        target,
                        trash_indices: vec![0, 1],
                    });
                } else {
                    actions.push(Action::Equip {
                        gear_id: gear.id,
                        target,
                    });
                }
            }
        }
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Decision, Difficulty, Unit};

    fn game() -> Game {
        let mut game = Game::new(3, 3, 42, Difficulty::Easy);
        game.start_match().unwrap();
        for state in &mut game.players {
            state.units.clear();
            state.gear.clear();
            state.hand.clear();
            state.discard.clear();
            state.power_pool = [0; 6];
            state.any_power = 0;
        }
        game
    }

    fn unit(game: &mut Game, player: usize, lane: Option<usize>) -> Target {
        let id = game.next_unit_id;
        game.next_unit_id += 1;
        game.players[player].units.push(Unit {
            id,
            card_id: 25,
            lane,
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        });
        Target::Unit {
            player,
            unit_id: id,
        }
    }

    fn id(target: Target) -> u32 {
        let Target::Unit { unit_id, .. } = target else {
            panic!("unit expected")
        };
        unit_id
    }

    fn give_power(game: &mut Game, player: usize, domain: Domain, count: u8) {
        game.players[player].runes = [0; 6];
        game.players[player].exhausted_runes = [0; 6];
        game.players[player].runes[domain_index(domain)] = count;
        game.players[player].energy = u16::from(count);
    }

    fn gear(game: &mut Game, player: usize, card_id: usize) -> u32 {
        game.gear_play(player, card_id, None).unwrap();
        game.players[player].gear.last().unwrap().id
    }

    fn resolve_chain(game: &mut Game) {
        game.after_resolution().unwrap();
        if matches!(game.decision, Some(Decision::OptionalPlay { .. })) {
            game.apply(Action::Choose { accept: true }).unwrap();
        }
        let mut guard = 20;
        while !game.chain.is_empty() {
            assert!(guard > 0, "chain failed to resolve");
            game.apply(Action::Pass).unwrap();
            guard -= 1;
        }
    }

    #[test]
    fn equipment_stays_on_board_and_bonus_waits_for_equip_resolution() {
        let mut game = game();
        let target = unit(&mut game, 0, None);
        let boots = gear(&mut game, 0, 3);
        assert!(game.players[0].gear[0].ready);
        assert!(game.players[0].discard.is_empty());
        give_power(&mut game, 0, Domain::Chaos, 1);
        game.apply(Action::Equip {
            gear_id: boots,
            target,
        })
        .unwrap();
        assert_eq!(game.gear_might_bonus(id(target)).unwrap(), 0);
        assert_eq!(game.players[0].runes[domain_index(Domain::Chaos)], 0);
        assert_eq!(game.chain.len(), 1);
        resolve_chain(&mut game);
        assert_eq!(game.gear_might_bonus(id(target)).unwrap(), 2);
        assert!(game.gear_grants_ganking(id(target)));
        assert_eq!(game.players[0].gear[0].attached_to, Some(id(target)));
        assert!(game.players[0].discard.is_empty());
    }

    #[test]
    fn attached_equipment_cannot_activate_equip_again() {
        let mut game = game();
        let first = unit(&mut game, 0, None);
        let second = unit(&mut game, 0, None);
        let boots = gear(&mut game, 0, 3);
        game.gear_resolve_equip(0, boots, first);
        give_power(&mut game, 0, Domain::Chaos, 2);
        let before = game.players.clone();
        assert!(
            game.apply(Action::Equip {
                gear_id: boots,
                target: second
            })
            .is_err()
        );
        assert_eq!(game.players, before);
    }

    #[test]
    fn quick_draw_attachment_is_free_and_separate_from_the_card() {
        let mut game = game();
        let target = unit(&mut game, 0, None);
        give_power(&mut game, 0, Domain::Fury, 0);
        game.gear_play(0, 58, Some(target)).unwrap();
        game.after_resolution().unwrap();
        assert!(matches!(game.decision, Some(Decision::OnPlayTarget { .. })));
        game.apply(Action::ChooseTarget {
            target: Some(target),
        })
        .unwrap();
        assert_eq!(game.chain.len(), 1);
        assert_eq!(game.gear_might_bonus(id(target)).unwrap(), 0);
        resolve_chain(&mut game);
        assert_eq!(game.gear_might_bonus(id(target)).unwrap(), 2);
        assert_eq!(game.players[0].total_runes(), 0);
    }

    #[test]
    fn equip_target_leaving_fizzles_without_refunding_cost() {
        let mut game = game();
        let target = unit(&mut game, 0, None);
        let boots = gear(&mut game, 0, 3);
        give_power(&mut game, 0, Domain::Chaos, 1);
        game.gear_equip(0, boots, target).unwrap();
        game.players[0].units.clear();
        resolve_chain(&mut game);
        assert_eq!(game.players[0].gear[0].attached_to, None);
        assert_eq!(game.players[0].total_runes(), 0);
    }

    #[test]
    fn guardian_angel_is_consumed_and_heals_recalls_exhausts_unit() {
        let mut game = game();
        let target = unit(&mut game, 0, Some(0));
        let guardian = gear(&mut game, 0, 10);
        game.gear_resolve_equip(0, guardian, target);
        game.players[0].units[0].damage = 9;
        game.remove_defeated().unwrap();
        assert_eq!(game.players[0].units.len(), 1);
        let survivor = &game.players[0].units[0];
        assert_eq!(
            (survivor.damage, survivor.lane, survivor.ready),
            (0, None, false)
        );
        assert!(game.players[0].gear.is_empty());
        assert_eq!(game.players[0].discard, vec![10]);
    }

    #[test]
    fn zhonya_uses_corrected_official_healing_text() {
        let mut game = game();
        unit(&mut game, 0, Some(0));
        gear(&mut game, 0, 21);
        game.players[0].units[0].damage = 9;
        game.remove_defeated().unwrap();
        assert_eq!(game.players[0].units.len(), 1);
        assert_eq!(game.players[0].units[0].damage, 0);
        assert_eq!(game.players[0].units[0].lane, None);
        assert!(!game.players[0].units[0].ready);
        assert_eq!(game.players[0].discard, vec![21]);
    }

    #[test]
    fn only_one_death_replacement_is_consumed() {
        let mut game = game();
        let target = unit(&mut game, 0, Some(0));
        let guardian = gear(&mut game, 0, 10);
        game.gear_resolve_equip(0, guardian, target);
        gear(&mut game, 0, 21);
        assert!(game.gear_prevent_death(0, id(target)));
        assert_eq!(game.players[0].discard, vec![10]);
        assert_eq!(game.players[0].gear[0].card_id, 21);
    }

    #[test]
    fn equipment_detaches_without_changing_its_ready_state() {
        let mut game = game();
        let target = unit(&mut game, 0, Some(1));
        let boots = gear(&mut game, 0, 3);
        game.gear_resolve_equip(0, boots, target);
        game.players[0].gear[0].ready = false;
        assert_eq!(game.gear_location(0, boots), Some(1));
        game.gear_unit_left_board(0, id(target));
        assert_eq!(game.players[0].gear[0].attached_to, None);
        assert!(!game.players[0].gear[0].ready);
        assert_eq!(game.gear_location(0, boots), Some(1));
        game.cleanup_gear_locations();
        assert_eq!(game.gear_location(0, boots), None);
        assert_eq!(game.gear_might_bonus(id(target)).unwrap(), 0);
        assert!(game.players[0].discard.is_empty());
    }

    #[test]
    fn seal_adds_floating_power_without_making_runes_or_a_chain_item() {
        let mut game = game();
        let seal = gear(&mut game, 0, 42);
        let runes_before = game.players[0].runes;
        game.apply(Action::ActivateGear { gear_id: seal }).unwrap();
        assert_eq!(game.players[0].runes, runes_before);
        assert_eq!(game.players[0].power_pool[domain_index(Domain::Chaos)], 1);
        assert!(game.chain.is_empty());
        assert!(!game.players[0].gear[0].ready);
        assert!(game.apply(Action::ActivateGear { gear_id: seal }).is_err());
        game.spend_power(0, Some(Domain::Chaos), 1).unwrap();
        assert_eq!(game.players[0].runes, runes_before);
        assert_eq!(game.players[0].power_pool[domain_index(Domain::Chaos)], 0);
    }

    #[test]
    fn gold_is_a_single_use_any_power_resource_and_never_a_trash_card() {
        let mut game = game();
        let gold = game.gear_gold(0, false);
        game.gear_activate(0, gold).unwrap();
        assert_eq!(game.players[0].any_power, 1);
        assert!(game.players[0].gear.is_empty());
        assert!(game.players[0].discard.is_empty());
        let runes_before = game.players[0].runes;
        game.spend_power(0, Some(Domain::Fury), 1).unwrap();
        assert_eq!(game.players[0].any_power, 0);
        assert_eq!(game.players[0].runes, runes_before);
    }

    #[test]
    fn resource_gear_can_pay_for_fizz_replay_without_dismissing_its_choice() {
        for use_gold in [false, true] {
            let mut game = game();
            let target = unit(&mut game, 0, Some(0));
            give_power(&mut game, 0, Domain::Chaos, 0);
            let resource = if use_gold {
                game.gear_gold(0, false)
            } else {
                gear(&mut game, 0, 42)
            };
            game.players[0].discard = vec![14]; // Ride the Wind costs Chaos power.
            let decision = Decision::ReplaySpell { player: 0 };
            game.decision = Some(decision.clone());
            let replay = Action::PlayFromTrash {
                trash_index: 0,
                lane: None,
                target: Some(target),
            };
            assert!(game.apply(replay.clone()).is_err());
            let resource_action = Action::ActivateGear { gear_id: resource };
            assert!(game.gear_actions(0).contains(&resource_action));
            game.apply(resource_action).unwrap();
            assert_eq!(game.decision, Some(decision));
            assert_eq!(game.priority(), 0);
            assert!(game.chain.is_empty());
            game.apply(replay).unwrap();
            assert!(game.decision.is_none());
            assert_eq!(game.chain.len(), 1);
            assert!(game.chain[0].recycle_after);
            assert_eq!(game.players[0].total_runes(), 0);
            assert_eq!(game.players[0].any_power, 0);
            assert_eq!(game.players[0].power_pool, [0; 6]);
        }
    }

    #[test]
    fn resource_gear_cannot_interrupt_an_unrelated_card_selection() {
        let mut game = game();
        let seal = gear(&mut game, 0, 42);
        game.decision = Some(Decision::Discard {
            player: 0,
            count: 1,
            draw_after: 1,
        });
        let before = game.players.clone();
        assert!(game.apply(Action::ActivateGear { gear_id: seal }).is_err());
        assert_eq!(game.players, before);
        assert!(!game.gear_can_activate(0, seal));
    }

    #[test]
    fn intrinsic_temporary_is_inactive_while_attached_then_kills_at_next_beginning() {
        let mut game = game();
        let target = unit(&mut game, 0, None);
        let monument = gear(&mut game, 0, 331);
        // Explicit attachment-state fixture: Monument has printed Temporary,
        // but the included card itself has no Equip activation.
        game.players[0].gear[0].attached_to = Some(id(target));
        assert_eq!(
            game.gear_temporary_status(0, monument),
            Some(GearTemporaryStatus {
                intrinsic: true,
                granted: false,
                active: false,
            })
        );
        game.gear_begin_turn(0);
        assert_eq!(game.players[0].gear.len(), 1);
        game.gear_unit_left_board(0, id(target));
        game.gear_begin_turn(0);
        assert_eq!(game.players[0].gear.len(), 1);
        assert!(matches!(
            game.chain.last().unwrap().ability,
            Some(ChainAbility::Temporary {
                source: crate::empower::BoardObject::Gear { .. },
                ..
            })
        ));
        resolve_chain(&mut game);
        assert!(game.players[0].gear.is_empty());
        assert_eq!(game.players[0].discard, vec![331]);
    }

    #[test]
    fn intrinsic_and_granted_temporary_make_one_expiry_and_departure_clears_grant() {
        let mut game = game();
        let monument = gear(&mut game, 0, 331);
        let target = Target::Gear {
            player: 0,
            gear_id: monument,
        };
        game.gear_make_temporary(0, 66, target);
        game.gear_make_temporary(0, 66, target);
        assert_eq!(
            game.gear_temporary_status(0, monument),
            Some(GearTemporaryStatus {
                intrinsic: true,
                granted: true,
                active: true,
            })
        );
        game.gear_begin_turn(0);
        assert_eq!(game.chain.len(), 1);
        resolve_chain(&mut game);
        assert!(!game.granted_temporary_gear.contains(&monument));
        assert_eq!(game.gear_temporary_status(0, monument), None);
        let fresh = gear(&mut game, 0, 331);
        assert_eq!(
            game.gear_temporary_status(0, fresh),
            Some(GearTemporaryStatus {
                intrinsic: true,
                granted: false,
                active: true,
            })
        );
        assert_eq!(game.gear_temporary_status(9, fresh), None);
    }

    #[test]
    fn last_rites_recycles_selected_cards_and_rejects_invalid_cost_atomically() {
        let mut game = game();
        let target = unit(&mut game, 0, None);
        let rites = gear(&mut game, 0, 38);
        give_power(&mut game, 0, Domain::Chaos, 1);
        game.players[0].discard = vec![7, 11, 25];
        let before = game.players.clone();
        assert!(
            game.gear_equip_with_trash(0, rites, target, &[0, 0])
                .is_err()
        );
        assert!(game.gear_equip(0, rites, target).is_err());
        assert_eq!(game.players, before);
        let deck_len = game.players[0].deck.len();
        game.gear_equip_with_trash(0, rites, target, &[0, 2])
            .unwrap();
        assert_eq!(game.players[0].discard, vec![11]);
        assert_eq!(game.players[0].deck.len(), deck_len + 2);
        resolve_chain(&mut game);
        assert_eq!(game.gear_might_bonus(id(target)).unwrap(), 2);
    }

    #[test]
    fn last_rites_opens_optional_trash_play_for_attached_unit_only() {
        let mut game = game();
        let target = unit(&mut game, 0, Some(0));
        let other = unit(&mut game, 0, Some(0));
        let rites = gear(&mut game, 0, 38);
        game.gear_resolve_equip(0, rites, target);
        game.players[0].discard = vec![25];
        game.gear_conquer_hold(0, id(other));
        assert!(game.decision.is_none());
        game.gear_conquer_hold(0, id(target));
        assert!(game.decision.is_none());
        resolve_chain(&mut game);
        assert_eq!(
            game.decision,
            Some(Decision::TrashTarget {
                player: 0,
                source_card_id: 38,
                gear_id: Some(rites)
            })
        );
    }

    #[test]
    fn boots_allow_battlefield_to_battlefield_standard_movement() {
        let mut game = game();
        let target = unit(&mut game, 0, Some(0));
        game.battlefield_owner[0] = Some(0);
        assert!(
            game.apply(Action::Move {
                unit_id: id(target),
                lane: Some(1)
            })
            .is_err()
        );
        let boots = gear(&mut game, 0, 3);
        game.gear_resolve_equip(0, boots, target);
        game.apply(Action::Move {
            unit_id: id(target),
            lane: Some(1),
        })
        .unwrap();
        assert_eq!(game.players[0].units[0].lane, Some(1));
        assert!(!game.players[0].units[0].ready);
        assert_eq!(game.gear_location(0, boots), Some(1));
    }

    #[test]
    fn salvage_removes_equipment_and_loss_of_might_can_be_lethal() {
        let mut game = game();
        let target = unit(&mut game, 1, None);
        let boots = gear(&mut game, 1, 3);
        game.gear_resolve_equip(1, boots, target);
        game.players[1].units[0].damage = 2;
        assert!(game.unit_might(&game.players[1].units[0]).unwrap() > 2);
        game.players[0].hand = vec![41];
        let domain = game.catalog[41].domain;
        give_power(&mut game, 0, domain, 6);
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: Some(Target::Gear {
                player: 1,
                gear_id: boots,
            }),
        })
        .unwrap();
        resolve_chain(&mut game);
        assert!(game.players[1].gear.is_empty());
        assert!(game.players[1].units.is_empty());
        assert!(game.players[1].discard.contains(&3));
        assert!(game.players[1].discard.contains(&25));
        assert_eq!(game.players[0].hand.len(), 1);
    }

    #[test]
    fn last_rites_plays_selected_trash_unit_with_its_actual_cost() {
        let mut game = game();
        let target = unit(&mut game, 0, Some(0));
        let rites = gear(&mut game, 0, 38);
        game.gear_resolve_equip(0, rites, target);
        game.players[0].discard = vec![25];
        give_power(&mut game, 0, Domain::Fury, 4);
        let energy = game.players[0].energy;
        game.gear_conquer_hold(0, id(target));
        resolve_chain(&mut game);
        game.apply(Action::SelectCards { indices: vec![0] })
            .unwrap();
        assert_eq!(game.players[0].discard, vec![25]);
        resolve_chain(&mut game);
        assert!(matches!(
            game.decision,
            Some(Decision::PendingPlay { card_id: 25, .. })
        ));
        assert!(game.players[0].discard.is_empty());
        game.apply(Action::FinalizePendingPlay {
            lane: None,
            targets: vec![],
            repeat_targets: vec![],
            repeat: false,
            accelerate: false,
            bonus: false,
        })
        .unwrap();
        assert!(game.players[0].discard.is_empty());
        assert_eq!(game.players[0].units.len(), 2);
        assert_eq!(
            game.players[0].energy,
            energy - u16::from(game.catalog[25].cost)
        );
        assert!(!game.players[0].units.last().unwrap().ready);
    }

    #[test]
    fn floating_power_expires_and_seal_readies_on_next_own_turn() {
        let mut game = game();
        let seal = gear(&mut game, 0, 42);
        game.gear_activate(0, seal).unwrap();
        game.apply(Action::EndTurn).unwrap();
        assert_eq!(game.players[0].power_pool, [0; 6]);
        assert!(!game.players[0].gear[0].ready);
        game.apply(Action::EndTurn).unwrap();
        assert!(game.players[0].gear[0].ready);
    }

    #[test]
    fn every_colored_seal_adds_its_own_power_without_runes_or_priority_change() {
        for (card, domain) in [
            (42, Domain::Chaos),
            (125, Domain::Calm),
            (126, Domain::Mind),
            (127, Domain::Fury),
            (128, Domain::Body),
            (129, Domain::Order),
        ] {
            let mut game = game();
            give_power(&mut game, 0, Domain::Mind, 0);
            let seal = gear(&mut game, 0, card);
            assert_eq!(game.gear_available_power(0), 1);
            game.apply(Action::ActivateGear { gear_id: seal }).unwrap();
            assert_eq!(game.priority(), 0);
            assert!(game.chain.is_empty());
            assert_eq!(game.players[0].energy, 0);
            assert_eq!(game.players[0].total_runes(), 0);
            let mut expected = [0; 6];
            expected[domain_index(domain)] = 1;
            assert_eq!(game.players[0].power_pool, expected);
            assert_eq!(game.gear_available_power(0), 0);
            let before = game.players.clone();
            assert!(game.apply(Action::ActivateGear { gear_id: seal }).is_err());
            assert_eq!(game.players, before);
        }
    }

    #[test]
    fn conduit_adds_floating_energy_during_payment_and_is_never_counted_as_power() {
        let mut game = game();
        give_power(&mut game, 0, Domain::Mind, 0);
        let conduit = gear(&mut game, 0, 130);
        assert_eq!(game.gear_available_power(0), 0);
        let decision = Decision::CounterTax {
            player: 0,
            chain_id: 9000,
            cost: 1,
        };
        game.decision = Some(decision.clone());
        game.apply(Action::ActivateGear { gear_id: conduit })
            .unwrap();
        assert_eq!(game.decision, Some(decision));
        assert_eq!(game.players[0].energy_pool, 1);
        assert_eq!(game.players[0].energy, 1);
        assert_eq!(game.players[0].total_runes(), 0);
        assert_eq!(game.players[0].power_pool, [0; 6]);
        assert_eq!(game.players[0].any_power, 0);
        game.pay_energy(0, 1);
        assert_eq!(game.players[0].energy_pool, 0);
        assert_eq!(game.players[0].energy, 0);
    }

    #[test]
    fn new_resource_gears_cannot_bypass_discard_choices() {
        for card in 125..=130 {
            let mut game = game();
            let resource = gear(&mut game, 0, card);
            game.decision = Some(Decision::Discard {
                player: 0,
                count: 1,
                draw_after: 1,
            });
            let before = game.players.clone();
            assert!(!game.gear_can_activate(0, resource));
            assert!(
                game.apply(Action::ActivateGear { gear_id: resource })
                    .is_err()
            );
            assert_eq!(game.players, before);
        }
    }

    #[test]
    fn ntofo_equip_spends_energy_only_and_grants_shield_after_resolution() {
        let mut game = game();
        let target = unit(&mut game, 0, None);
        let ntofo = gear(&mut game, 0, 124);
        give_power(&mut game, 0, Domain::Mind, 0);
        game.players[0].power_pool[domain_index(Domain::Order)] = 1;
        let before = game.players.clone();
        assert!(
            game.apply(Action::Equip {
                gear_id: ntofo,
                target
            })
            .is_err()
        );
        assert_eq!(game.players, before);
        game.players[0].energy_pool = 1;
        game.players[0].energy = 1;
        game.apply(Action::Equip {
            gear_id: ntofo,
            target,
        })
        .unwrap();
        assert_eq!(game.players[0].energy, 0);
        assert_eq!(game.players[0].power_pool[domain_index(Domain::Order)], 1);
        assert_eq!(game.gear_keywords(id(target)).unwrap().shield, 0);
        resolve_chain(&mut game);
        assert_eq!(game.gear_keywords(id(target)).unwrap().shield, 2);
        assert_eq!(game.gear_might_bonus(id(target)).unwrap(), 0);
    }

    #[test]
    fn equipment_keyword_grants_stack_and_disappear_on_detachment() {
        let mut game = game();
        let target = unit(&mut game, 0, None);
        game.players[0].units[0].card_id = 104; // Garen: Assault2 + Shield2.
        for (offset, card_id) in [3, 120, 121, 122, 122, 123, 124].into_iter().enumerate() {
            game.players[0].gear.push(GearState {
                id: 1000 + offset as u32,
                card_id,
                lane: None,
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            });
        }
        assert_eq!(
            game.gear_keywords(id(target)).unwrap(),
            UnitKeywords::default()
        );
        for item in &mut game.players[0].gear {
            item.attached_to = Some(id(target));
        }
        let keywords = game.unit_keywords(&game.players[0].units[0]).unwrap();
        assert_eq!(keywords.assault, 4);
        assert_eq!(keywords.shield, 6);
        assert_eq!(keywords.deflect, 2);
        assert!(keywords.tank && keywords.ganking);
        assert!(!keywords.ambush);
        assert_eq!(game.gear_might_bonus(id(target)).unwrap(), 5); // Boots2 + Shield1 + Hexdrinkers2.
        game.gear_unit_left_board(0, id(target));
        assert_eq!(
            game.gear_keywords(id(target)).unwrap(),
            UnitKeywords::default()
        );
        assert_eq!(
            game.unit_keywords(&game.players[0].units[0])
                .unwrap()
                .shield,
            2
        );
    }
}
