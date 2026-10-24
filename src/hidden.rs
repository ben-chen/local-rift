//! Facedown zones and the Hidden keyword (Core Rules 107.3, 323.6a, 422, 811).
//!
//! Hiding is not playing a card. Only a later turn grants the facedown card
//! Reaction and the base-cost waiver. Additional costs still apply. The
//! renderer and advisor should use `hidden_visible_card` rather than expose an
//! opponent's private identity. Units and gear still finalize immediately;
//! their triggered abilities, and spells, use the ordinary response chain.

use crate::cards::{CardId, CardKind};
use crate::game::{Action, BATTLEFIELDS, Game, Phase, Target};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiddenCard {
    pub id: u32,
    pub card_id: CardId,
    pub lane: usize,
    pub hidden_turn: u32,
    /// Public knowledge gained through an explicit reveal effect. Scuttle
    /// Crab instead grants a temporary look permission tracked by the game.
    /// Revealing does not itself play the card or turn it face up.
    pub revealed: bool,
}

impl Game {
    pub fn hidden_visible_card(
        &self,
        viewer: usize,
        owner: usize,
        hidden_id: u32,
    ) -> Option<CardId> {
        let card = self
            .players
            .get(owner)?
            .hidden
            .iter()
            .find(|card| card.id == hidden_id)?;
        (viewer == owner
            || card.revealed
            || self.hidden_view.get(viewer).copied().unwrap_or(false)
            || self.phase == Phase::Finished)
            .then_some(card.card_id)
    }

    fn hidden_validate_slot(
        &self,
        player: usize,
        card_id: CardId,
        lane: usize,
    ) -> Result<(), String> {
        if self.phase != Phase::Playing || player >= self.players.len() {
            return Err("Hide cards during a match.".into());
        }
        if player != self.active || player != self.priority() || self.in_window() {
            return Err("Hide during your main phase with no chain or showdown open.".into());
        }
        if !self
            .catalog
            .get(card_id)
            .is_some_and(|card| card.keywords.contains(&"Hidden"))
        {
            return Err("This card does not have Hidden.".into());
        }
        if lane >= BATTLEFIELDS || self.battlefield_owner[lane] != Some(player) {
            return Err("Hide at a battlefield you control.".into());
        }
        if self
            .players
            .iter()
            .any(|state| state.hidden.iter().any(|card| card.lane == lane))
        {
            return Err("This battlefield already has its one facedown card.".into());
        }
        if self.available_power(player, None) < 1 {
            return Err("Hiding costs one power of any domain.".into());
        }
        Ok(())
    }

    pub fn hidden_can_hide(
        &self,
        player: usize,
        hand_index: usize,
        lane: usize,
    ) -> Result<(), String> {
        let card_id = *self
            .players
            .get(player)
            .and_then(|state| state.hand.get(hand_index))
            .ok_or("Choose a card in your hand.")?;
        self.hidden_validate_slot(player, card_id, lane)
    }

    pub fn hidden_can_hide_champion(&self, player: usize, lane: usize) -> Result<(), String> {
        let card_id = self
            .players
            .get(player)
            .and_then(|state| state.champion)
            .ok_or("Your chosen champion is no longer in its zone.")?;
        self.hidden_validate_slot(player, card_id, lane)
    }

    fn hidden_place(&mut self, player: usize, card_id: CardId, lane: usize) {
        let id = self.next_unit_id;
        self.next_unit_id += 1;
        self.players[player].hidden.push(HiddenCard {
            id,
            card_id,
            lane,
            hidden_turn: self.turn,
            revealed: false,
        });
        // The public log must not reveal the hidden card's identity.
        self.push_log(format!(
            "{} hides a card at battlefield {}. It can be played beginning next turn.",
            self.players[player].name,
            lane + 1
        ));
    }

    pub(crate) fn hide_card(
        &mut self,
        player: usize,
        hand_index: usize,
        lane: usize,
    ) -> Result<(), String> {
        self.hidden_can_hide(player, hand_index, lane)?;
        self.spend_power(player, None, 1)?;
        let card_id = self.players[player].hand.remove(hand_index);
        self.hidden_place(player, card_id, lane);
        Ok(())
    }

    pub(crate) fn hide_champion(&mut self, player: usize, lane: usize) -> Result<(), String> {
        self.hidden_can_hide_champion(player, lane)?;
        self.spend_power(player, None, 1)?;
        let card_id = self.players[player].champion.take().unwrap();
        self.hidden_place(player, card_id, lane);
        Ok(())
    }

    /// Applies both on finalization and resolution. Leaving the original
    /// battlefield makes a hidden spell's target illegal even if its ordinary
    /// card text would allow the target at another battlefield (CR359.3.e2a).
    pub fn hidden_target_allowed(&self, card_id: CardId, lane: usize, target: Target) -> bool {
        let location = match target {
            Target::Unit { player, unit_id } => self
                .players
                .get(player)
                .and_then(|state| state.units.iter().find(|unit| unit.id == unit_id))
                .map(|unit| unit.lane),
            Target::Gear { player, gear_id } => self
                .players
                .get(player)
                .and_then(|state| state.gear.iter().find(|gear| gear.id == gear_id))
                .map(|_| self.gear_location(player, gear_id)),
            Target::Spell { .. } | Target::Battlefield { .. } => return false,
        };
        // Tideturner's explicit 'another location' can never choose here, so
        // CR811.1.d3 permits its target outside the hidden battlefield.
        if card_id == 18 {
            location.is_some_and(|location| location != Some(lane))
        } else {
            location == Some(Some(lane))
        }
    }

    pub fn hidden_can_play(
        &self,
        player: usize,
        hidden_id: u32,
        targets: &[Target],
    ) -> Result<Vec<Target>, String> {
        if self.phase != Phase::Playing || player != self.priority() || self.decision.is_some() {
            return Err(
                "Play a hidden card when you have priority and no choice is pending.".into(),
            );
        }
        let card = self
            .players
            .get(player)
            .and_then(|state| state.hidden.iter().find(|card| card.id == hidden_id))
            .ok_or("That facedown card is no longer there.")?;
        if card.hidden_turn >= self.turn {
            return Err("A card cannot be played on the same turn it was hidden.".into());
        }
        if self.battlefield_owner[card.lane] != Some(player) {
            return Err("The battlefield is no longer under your control.".into());
        }
        let resolved = if card.card_id == 44 && targets.is_empty() {
            // Automatic target selection must obey Hidden before considering
            // public-board strength or cost, not pick a pair at another field.
            let local: Vec<_> = self
                .players
                .iter()
                .enumerate()
                .flat_map(|(owner, state)| {
                    state
                        .units
                        .iter()
                        .filter(move |unit| unit.lane == Some(card.lane))
                        .map(move |unit| Target::Unit {
                            player: owner,
                            unit_id: unit.id,
                        })
                })
                .collect();
            let mut choice = None;
            'pairs: for (index, &first) in local.iter().enumerate() {
                for &second in &local[index + 1..] {
                    if let Ok(valid) = self.validate_hidden_targets(
                        player,
                        card.card_id,
                        card.lane,
                        &[first, second],
                    ) {
                        choice = Some(valid);
                        break 'pairs;
                    }
                }
            }
            choice.ok_or("Choose two legal units at this hidden card's battlefield, with power for any Deflect costs.")?
        } else {
            self.validate_hidden_targets(player, card.card_id, card.lane, targets)?
        };
        if resolved
            .iter()
            .any(|&target| !self.hidden_target_allowed(card.card_id, card.lane, target))
        {
            return Err("A card played from Hidden must choose targets at its battlefield, except explicit different-location targets.".into());
        }
        Ok(resolved)
    }

    pub(crate) fn play_hidden(
        &mut self,
        player: usize,
        hidden_id: u32,
        targets: Vec<Target>,
    ) -> Result<(), String> {
        let resolved = self.hidden_can_play(player, hidden_id, &targets)?;
        let index = self.players[player]
            .hidden
            .iter()
            .position(|card| card.id == hidden_id)
            .unwrap();
        let card_id = self.players[player].hidden[index].card_id;
        if let Some(result) = self.pay_active_play(player, card_id) {
            result?;
        } else {
            self.spend_deflect(
                player,
                self.catalog[card_id].kind == CardKind::Spell,
                &resolved,
            )?;
        }
        let card = self.players[player].hidden.remove(index);
        self.push_log(format!(
            "{} reveals and plays {} from battlefield {}.",
            self.players[player].name,
            self.catalog[card_id].name,
            card.lane + 1
        ));
        self.queue_hidden_card(player, card_id, card.lane, resolved);
        Ok(())
    }

    /// Call after actual battlefield-control cleanup, not merely when a field
    /// becomes contested. The defending player retains the card in a showdown.
    pub(crate) fn cleanup_lost_hidden(&mut self) {
        for player in 0..self.players.len() {
            let lost: Vec<_> = self.players[player]
                .hidden
                .iter()
                .enumerate()
                .filter(|(_, card)| self.battlefield_owner[card.lane] != Some(player))
                .map(|(index, _)| index)
                .collect();
            for index in lost.into_iter().rev() {
                let card = self.players[player].hidden.remove(index);
                self.players[player].discard.push(card.card_id);
                self.push_log(format!(
                    "{} is revealed and put in the trash after {} loses control of battlefield {}.",
                    self.catalog[card.card_id].name,
                    self.players[player].name,
                    card.lane + 1
                ));
            }
        }
    }

    /// Candidate actions read only the actor's facedown identities. The enemy's
    /// board is public; their hand, deck and facedown identities are not used.
    pub fn hidden_actions(&self, player: usize) -> Vec<Action> {
        if player != self.priority() || player >= self.players.len() {
            return Vec::new();
        }
        let mut actions = Vec::new();
        for (hand_index, _) in self.players[player].hand.iter().enumerate() {
            for lane in 0..BATTLEFIELDS {
                if self.hidden_can_hide(player, hand_index, lane).is_ok() {
                    actions.push(Action::Hide { hand_index, lane });
                }
            }
        }
        for lane in 0..BATTLEFIELDS {
            if self.hidden_can_hide_champion(player, lane).is_ok() {
                actions.push(Action::HideChampion { lane });
            }
        }
        for card in &self.players[player].hidden {
            let mut choices = vec![Vec::new()];
            let targets: Vec<_> = self
                .players
                .iter()
                .enumerate()
                .flat_map(|(owner, state)| {
                    state.units.iter().map(move |unit| Target::Unit {
                        player: owner,
                        unit_id: unit.id,
                    })
                })
                .filter(|&target| self.hidden_target_allowed(card.card_id, card.lane, target))
                .collect();
            if card.card_id == 44 {
                // Switcheroo requires two distinct units, with both at the
                // hidden field. Both friendly and enemy choices are legal.
                for (index, &first) in targets.iter().enumerate() {
                    for &second in &targets[index + 1..] {
                        choices.push(vec![first, second]);
                    }
                }
            } else {
                choices.extend(targets.into_iter().map(|target| vec![target]));
            }
            for targets in choices {
                if let Ok(resolved) = self.hidden_can_play(player, card.id, &targets) {
                    let action = Action::PlayHidden {
                        hidden_id: card.id,
                        targets: resolved,
                    };
                    if !actions.contains(&action) {
                        actions.push(action);
                    }
                }
            }
        }
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::Domain;
    use crate::game::{Difficulty, Unit, domain_index};

    fn game() -> Game {
        let mut game = Game::new(3, 3, 87, Difficulty::Easy);
        game.start_match().unwrap();
        for state in &mut game.players {
            state.units.clear();
            state.hand.clear();
            state.discard.clear();
            state.gear.clear();
            state.hidden.clear();
            state.runes = [0; 6];
            state.exhausted_runes = [0; 6];
            state.power_pool = [0; 6];
            state.any_power = 0;
            state.energy = 0;
        }
        game.battlefield_owner = [Some(0), Some(1)];
        unit(&mut game, 0, 25, Some(0));
        unit(&mut game, 1, 25, Some(1));
        game
    }

    fn unit(game: &mut Game, player: usize, card_id: usize, lane: Option<usize>) -> Target {
        let unit_id = game.next_unit_id;
        game.next_unit_id += 1;
        game.players[player].units.push(Unit {
            id: unit_id,
            card_id,
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
        Target::Unit { player, unit_id }
    }

    fn hide(game: &mut Game, card_id: usize) -> u32 {
        game.players[0].hand = vec![card_id];
        game.players[0].power_pool[domain_index(Domain::Mind)] = 1;
        game.apply(Action::Hide {
            hand_index: 0,
            lane: 0,
        })
        .unwrap();
        game.players[0].hidden[0].id
    }

    fn make_old(game: &mut Game) {
        game.players[0].hidden[0].hidden_turn = game.turn - 1;
    }

    #[test]
    fn hiding_pays_any_power_without_playing_or_leaking_card_identity() {
        let mut game = game();
        let hidden_id = hide(&mut game, 21);
        assert!(game.players[0].hand.is_empty());
        assert_eq!(game.players[0].power_pool, [0; 6]);
        assert!(game.players[0].gear.is_empty());
        assert!(game.chain.is_empty());
        assert_eq!(game.hidden_visible_card(0, 0, hidden_id), Some(21));
        assert_eq!(game.hidden_visible_card(1, 0, hidden_id), None);
        assert!(!game.log.last().unwrap().contains("Zhonya"));
    }

    #[test]
    fn one_slot_control_and_keyword_requirements_reject_atomically() {
        let mut game = game();
        game.players[0].hand = vec![21, 25];
        game.players[0].any_power = 2;
        let before = game.players.clone();
        assert!(
            game.apply(Action::Hide {
                hand_index: 0,
                lane: 1
            })
            .is_err()
        );
        assert!(
            game.apply(Action::Hide {
                hand_index: 1,
                lane: 0
            })
            .is_err()
        );
        assert_eq!(game.players, before);
        game.apply(Action::Hide {
            hand_index: 0,
            lane: 0,
        })
        .unwrap();
        game.players[0].hand = vec![13];
        let before = game.players.clone();
        assert!(
            game.apply(Action::Hide {
                hand_index: 0,
                lane: 0
            })
            .is_err()
        );
        assert_eq!(game.players, before);
    }

    #[test]
    fn same_turn_reveal_is_forbidden_even_if_card_has_reaction() {
        let mut game = game();
        let hidden_id = hide(&mut game, 21);
        let before = game.players.clone();
        assert!(
            game.apply(Action::PlayHidden {
                hidden_id,
                targets: Vec::new()
            })
            .is_err()
        );
        assert_eq!(game.players, before);
    }

    #[test]
    fn later_turn_unit_ignores_base_cost_and_enters_its_hidden_field_exhausted() {
        let mut game = game();
        let hidden_id = hide(&mut game, 13);
        make_old(&mut game);
        assert_eq!(game.players[0].energy, 0);
        assert_eq!(game.players[0].total_runes(), 0);
        game.apply(Action::PlayHidden {
            hidden_id,
            targets: Vec::new(),
        })
        .unwrap();
        assert!(game.players[0].hidden.is_empty());
        let pyke = game.players[0]
            .units
            .iter()
            .find(|unit| unit.card_id == 13)
            .unwrap();
        assert_eq!(pyke.lane, Some(0));
        assert!(!pyke.ready);
        assert_eq!(game.players[0].energy, 0);
        assert!(game.players[0].discard.is_empty());
    }

    #[test]
    fn hidden_spell_restricts_all_targets_to_original_field() {
        let mut game = game();
        let first = Target::Unit {
            player: 0,
            unit_id: game.players[0].units[0].id,
        };
        let second = unit(&mut game, 0, 22, Some(0));
        let outside = Target::Unit {
            player: 1,
            unit_id: game.players[1].units[0].id,
        };
        let hidden_id = hide(&mut game, 44);
        make_old(&mut game);
        let before = game.players.clone();
        assert!(
            game.apply(Action::PlayHidden {
                hidden_id,
                targets: vec![first, outside]
            })
            .is_err()
        );
        assert_eq!(game.players, before);
        game.apply(Action::PlayHidden {
            hidden_id,
            targets: vec![first, second],
        })
        .unwrap();
        assert_eq!(game.chain.len(), 1);
        assert!(game.chain[0].from_hidden);
        assert_eq!(game.chain[0].lane, Some(0));
        assert_eq!(game.players[0].energy, 0);
    }

    #[test]
    fn tideturner_has_the_explicit_different_location_target_exception() {
        let mut game = game();
        let here = Target::Unit {
            player: 0,
            unit_id: game.players[0].units[0].id,
        };
        let elsewhere = unit(&mut game, 0, 22, None);
        assert!(!game.hidden_target_allowed(18, 0, here));
        assert!(game.hidden_target_allowed(18, 0, elsewhere));
        assert!(!game.hidden_target_allowed(44, 0, elsewhere));
    }

    #[test]
    fn hidden_cards_are_revealed_and_trashed_when_control_is_lost() {
        let mut game = game();
        hide(&mut game, 21);
        game.battlefield_owner[0] = Some(1);
        game.cleanup_lost_hidden();
        assert!(game.players[0].hidden.is_empty());
        assert_eq!(game.players[0].discard, vec![21]);
        assert!(game.log.last().unwrap().contains("Zhonya's Hourglass"));
    }

    #[test]
    fn contested_field_retains_hidden_card_until_control_really_changes() {
        let mut game = game();
        let hidden_id = hide(&mut game, 21);
        unit(&mut game, 1, 22, Some(0));
        assert!(game.contested(0));
        game.cleanup_lost_hidden();
        assert_eq!(game.players[0].hidden[0].id, hidden_id);
    }

    #[test]
    fn explicit_reveal_and_end_of_game_reveal_respect_privacy() {
        let mut game = game();
        let hidden_id = hide(&mut game, 21);
        assert_eq!(game.hidden_visible_card(1, 0, hidden_id), None);
        game.players[0].hidden[0].revealed = true;
        assert_eq!(game.hidden_visible_card(1, 0, hidden_id), Some(21));
        assert_eq!(game.players[0].hidden.len(), 1);
        assert!(game.players[0].gear.is_empty());
        game.players[0].hidden[0].revealed = false;
        game.phase = Phase::Finished;
        assert_eq!(game.hidden_visible_card(1, 0, hidden_id), Some(21));
    }

    #[test]
    fn champion_zone_can_hide_a_champion_with_hidden() {
        let mut game = game();
        game.players[0].champion = Some(13);
        game.players[0].any_power = 1;
        game.apply(Action::HideChampion { lane: 0 }).unwrap();
        assert!(game.players[0].champion.is_none());
        assert_eq!(game.players[0].hidden[0].card_id, 13);
        assert_eq!(game.players[0].any_power, 0);
    }

    #[test]
    fn generated_hidden_actions_never_select_off_field_spell_targets() {
        let mut game = game();
        unit(&mut game, 0, 22, Some(0));
        unit(&mut game, 1, 22, Some(1));
        hide(&mut game, 44);
        make_old(&mut game);
        let actions = game.hidden_actions(0);
        assert!(!actions.is_empty());
        for action in actions {
            if let Action::PlayHidden { targets, .. } = action {
                assert_eq!(targets.len(), 2);
                for target in targets {
                    assert!(game.hidden_target_allowed(44, 0, target));
                }
            }
        }
    }

    #[test]
    fn hidden_spell_waives_base_cost_but_still_pays_deflect() {
        let mut game = game();
        let friendly = Target::Unit {
            player: 0,
            unit_id: game.players[0].units[0].id,
        };
        let vex = unit(&mut game, 1, 20, Some(0));
        let hidden_id = hide(&mut game, 44);
        make_old(&mut game);
        // The opponent's entering Vex is the contender and gets first focus.
        // Pass that focus back to the Hidden card's controller.
        assert_eq!(game.priority(), 1);
        game.apply(Action::Pass).unwrap();
        let before = game.players.clone();
        assert!(
            game.apply(Action::PlayHidden {
                hidden_id,
                targets: vec![friendly, vex]
            })
            .is_err()
        );
        assert_eq!(game.players, before);
        game.players[0].any_power = 1;
        game.apply(Action::PlayHidden {
            hidden_id,
            targets: vec![friendly, vex],
        })
        .unwrap();
        assert_eq!(game.players[0].any_power, 0);
        assert_eq!(game.players[0].energy, 0);
    }

    #[test]
    fn targets_that_leave_hidden_field_are_rechecked_on_resolution() {
        let mut game = game();
        let first = Target::Unit {
            player: 0,
            unit_id: game.players[0].units[0].id,
        };
        let second = unit(&mut game, 0, 22, Some(0));
        let hidden_id = hide(&mut game, 44);
        make_old(&mut game);
        game.apply(Action::PlayHidden {
            hidden_id,
            targets: vec![first, second],
        })
        .unwrap();
        game.players[0].units[1].lane = Some(1);
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert_eq!(game.players[0].units[0].temporary_buff, 0);
        assert_eq!(game.players[0].units[1].temporary_buff, 0);
        assert!(game.players[0].discard.contains(&44));
    }

    #[test]
    fn hiding_is_not_allowed_during_a_showdown() {
        let mut game = game();
        game.players[0].hand = vec![21];
        game.players[0].any_power = 1;
        game.showdown = Some(crate::game::Showdown {
            lane: 0,
            attacker: 1,
            combat: true,
            focus: 0,
            passes: 0,
        });
        let before = game.players.clone();
        assert!(
            game.apply(Action::Hide {
                hand_index: 0,
                lane: 0
            })
            .is_err()
        );
        assert_eq!(game.players, before);
    }

    #[test]
    fn a_ready_hidden_card_can_react_on_the_opponents_turn() {
        let mut game = game();
        let hidden_id = hide(&mut game, 21);
        game.apply(Action::EndTurn).unwrap();
        while game.decision.is_some() || !game.chain.is_empty() {
            if game.decision.is_some() {
                game.apply(Action::Choose { accept: false }).unwrap();
            } else {
                game.apply(Action::Pass).unwrap();
            }
        }
        game.players[1].hand = vec![7];
        game.players[1].runes = [0; 6];
        game.players[1].runes[domain_index(Domain::Calm)] = 6;
        game.players[1].exhausted_runes = [0; 6];
        game.players[1].energy = 6;
        let target = Target::Unit {
            player: 1,
            unit_id: game.players[1].units[0].id,
        };
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: Some(target),
        })
        .unwrap();
        game.apply(Action::Pass).unwrap();
        assert_eq!(game.priority(), 0);
        assert_eq!(game.active, 1);
        game.apply(Action::PlayHidden {
            hidden_id,
            targets: Vec::new(),
        })
        .unwrap();
        assert!(game.players[0].hidden.is_empty());
        assert_eq!(game.players[0].gear[0].card_id, 21);
        assert!(game.chain.iter().any(|item| item.card_id == 7));
    }

    #[test]
    fn automatic_hidden_targets_stay_local_even_with_stronger_units_elsewhere() {
        let mut game = game();
        unit(&mut game, 0, 22, Some(0));
        unit(&mut game, 1, 40, Some(1));
        let hidden_id = hide(&mut game, 44);
        make_old(&mut game);
        let targets = game.hidden_can_play(0, hidden_id, &[]).unwrap();
        assert_eq!(targets.len(), 2);
        assert!(
            targets
                .iter()
                .all(|&target| game.hidden_target_allowed(44, 0, target))
        );
    }

    #[test]
    fn scuttle_look_permission_expires_without_permanently_revealing_cards() {
        let mut game = game();
        let hidden_id = hide(&mut game, 21);
        game.hidden_view[1] = true;
        assert_eq!(game.hidden_visible_card(1, 0, hidden_id), Some(21));
        assert!(!game.players[0].hidden[0].revealed);
        game.apply(Action::EndTurn).unwrap();
        assert_eq!(game.hidden_visible_card(1, 0, hidden_id), None);
    }
}
