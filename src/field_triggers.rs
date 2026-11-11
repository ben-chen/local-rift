//! Battlefield-origin effects, composite costs and delayed ending triggers.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BattlefieldSource {
    pub lane: usize,
    pub battlefield_id: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattlefieldTargetKind {
    StarSpring { played_unit_id: u32 },
    AbandonedHall,
    TargonPeakReady,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DelayedBattlefieldTrigger {
    pub id: u32,
    pub player: usize,
    pub source: BattlefieldSource,
    pub turn: u32,
}

impl Game {
    /// A conservative optional pull, evaluated from public board arithmetic.
    /// The projection includes current auras and combat designations, not future
    /// reactions or trigger resolutions; it never inspects a hidden card.
    pub(super) fn faefolk_choice(
        &self,
        decision: &Decision,
        actions: &[Action],
    ) -> Result<Option<Action>, String> {
        let Decision::TriggerTargets {
            player,
            card_id: 417,
            context,
            ..
        } = decision
        else {
            return Ok(None);
        };
        let Some(lane) = context.movement.and_then(|m| m.destination) else {
            return Ok(None);
        };
        let enemy = 1 - player;
        let attacker = self
            .showdown
            .as_ref()
            .filter(|w| w.lane == lane)
            .map(|w| w.attacker)
            .or_else(|| {
                self.pending_entries
                    .iter()
                    .find(|(field, _)| *field == lane)
                    .map(|(_, p)| *p)
            })
            .unwrap_or(enemy);
        let mut best = actions
            .iter()
            .find(|a| matches!(a,Action::ChooseTriggerTargets { targets } if targets.is_empty()))
            .cloned()
            .map(|a| (0_i128, a));
        for action in actions {
            let Action::ChooseTriggerTargets { targets } = action else {
                continue;
            };
            let [target @ Target::Unit { player: owner, .. }] = targets.as_slice() else {
                continue;
            };
            if *owner != enemy {
                continue;
            }
            let Some(unit) = self.unit_ref(*target) else {
                continue;
            };
            let denies_hold = unit.lane.is_some_and(|origin| {
                origin != lane
                    && self.battlefield_owner[origin] == Some(enemy)
                    && self.lane_count(enemy, origin) == 1
            });
            let mut projected = self.clone();
            // Evaluation only: actual movement still occurs through the queued
            // trigger and the normal response/cleanup path after selection.
            projected.unit_mut(*target).unwrap().lane = Some(lane);
            projected.showdown = Some(Showdown {
                lane,
                attacker,
                focus: attacker,
                passes: 0,
                combat: true,
            });
            let mut damage = [0u64; 2];
            let mut enemy_health = 0u64;
            let mut friendly_survivor = 0u64;
            let mut protected_enemy = false;
            for (side, state) in projected.players.iter().enumerate() {
                for unit in state.units.iter().filter(|u| u.lane == Some(lane)) {
                    let contribution = projected.unit_combat_contribution(unit)?;
                    damage[side] = crate::numeric::add_u64(
                        damage[side],
                        contribution.damage,
                        "Faefolk projected combat damage",
                    )?;
                    let health = contribution.might.saturating_sub(unit.damage);
                    if side == *player {
                        friendly_survivor = friendly_survivor.max(health);
                    } else {
                        enemy_health = crate::numeric::add_u64(
                            enemy_health,
                            health,
                            "Faefolk projected enemy Might",
                        )?;
                        protected_enemy |= unit.protected;
                    }
                }
            }
            if protected_enemy
                || damage[*player] == 0
                || damage[*player] < enemy_health
                || friendly_survivor <= damage[enemy]
            {
                continue;
            }
            let score = 100 + i128::from(enemy_health) - i128::from(damage[enemy])
                + if denies_hold { 200 } else { 0 };
            if best.as_ref().is_none_or(|(prior, _)| score > *prior) {
                best = Some((score, action.clone()));
            }
        }
        Ok(best.map(|(_, action)| action))
    }
    /// A small public-information policy for new field choices. This ranks only
    /// actions already accepted by the canonical rules; it is not legality.
    pub(super) fn battlefield_choice(
        &self,
        decision: &Decision,
        actions: &[Action],
    ) -> Result<Option<Action>, String> {
        let mut best: Option<(i128, Action)> = None;
        for action in actions {
            let score = match (decision, action) {
                (
                    Decision::BattlefieldTargets {
                        player,
                        source,
                        kind,
                        ..
                    },
                    Action::ResolveBattlefieldTargets { targets, .. },
                ) => {
                    let mut score = 0;
                    for &target in targets {
                        score += match kind {
                            BattlefieldTargetKind::TargonPeakReady => {
                                let Target::Rune {
                                    player: owner,
                                    rune_id,
                                } = target
                                else {
                                    continue;
                                };
                                let Some(rune) = self.players[owner]
                                    .rune_objects()
                                    .iter()
                                    .find(|r| r.id == rune_id)
                                else {
                                    continue;
                                };
                                if rune.ready {
                                    -1
                                } else if owner == *player {
                                    10
                                } else {
                                    -10
                                }
                            }
                            BattlefieldTargetKind::AbandonedHall => {
                                let Some(unit) = self.unit_ref(target) else {
                                    continue;
                                };
                                10 + i128::from(self.unit_might(unit)?)
                            }
                            BattlefieldTargetKind::StarSpring { .. } => {
                                let Some(unit) = self.unit_ref(target) else {
                                    continue;
                                };
                                let other = self.players[*player]
                                    .units
                                    .iter()
                                    .any(|u| u.id != unit.id && u.lane == Some(source.lane));
                                if other && unit.damage > 0 {
                                    20 + i128::from(unit.damage)
                                } else {
                                    -1
                                }
                            }
                        };
                    }
                    score
                }
                (
                    Decision::BattlefieldCost { player, .. },
                    Action::SelectBattlefieldCost { unit_id, .. },
                ) => {
                    if let Some(unit_id) = unit_id {
                        let Some(unit) = self.unit_ref(Target::Unit {
                            player: *player,
                            unit_id: *unit_id,
                        }) else {
                            continue;
                        };
                        6 - 3 * i128::from(self.unit_might(unit)?) + 2 * i128::from(unit.damage)
                            - i128::from(unit.ready) * 2
                    } else {
                        0
                    }
                }
                _ => continue,
            };
            if best.as_ref().is_none_or(|(prior, _)| score > *prior) {
                best = Some((score, action.clone()));
            }
        }
        Ok(best.map(|(_, action)| action))
    }
    pub fn star_spring_used(&self) -> [[bool; 2]; 2] {
        self.spring_used
    }

    pub fn delayed_battlefield_triggers(&self) -> &[DelayedBattlefieldTrigger] {
        &self.delayed_battlefield_triggers
    }

    fn field_source(&self, lane: usize) -> BattlefieldSource {
        BattlefieldSource {
            lane,
            battlefield_id: self.battlefields[lane],
        }
    }

    pub(super) fn queue_field_targets(
        &mut self,
        player: usize,
        lane: usize,
        kind: BattlefieldTargetKind,
    ) {
        let source = self.field_source(lane);
        let choice_id = self.next_chain_id;
        self.queue_trigger_choice(
            player,
            0,
            Decision::BattlefieldTargets {
                player,
                choice_id,
                source,
                kind,
            },
        );
    }

    pub(crate) fn queue_dais_cost(&mut self, player: usize, lane: usize) {
        let source = self.field_source(lane);
        let choice_id = self.next_chain_id;
        self.queue_trigger_choice(
            player,
            0,
            Decision::BattlefieldCost {
                player,
                choice_id,
                source,
            },
        );
    }

    pub(super) fn battlefield_unit_played(&mut self, player: usize, unit_id: u32) {
        let Some(unit) = self.unit_ref(Target::Unit { player, unit_id }) else {
            return;
        };
        let Some(lane) = unit.lane else {
            return;
        };
        if unit.token || self.battlefields[lane] != 12 {
            return;
        }
        if self.spring_used[player][lane] {
            return;
        }
        // First successful qualifying event is consumed even if declined or
        // no other unit can be chosen (Core383,420.4).
        self.spring_used[player][lane] = true;
        self.queue_field_targets(
            player,
            lane,
            BattlefieldTargetKind::StarSpring {
                played_unit_id: unit_id,
            },
        );
    }

    pub(super) fn battlefield_spell_played(&mut self, player: usize) {
        for lane in 0..BATTLEFIELDS {
            if self.battlefields[lane] == 13 {
                self.queue_field_targets(player, lane, BattlefieldTargetKind::AbandonedHall);
            }
        }
    }

    pub fn battlefield_choice_candidates(&self) -> Vec<Target> {
        let Some(Decision::BattlefieldTargets {
            player,
            source,
            kind,
            ..
        }) = self.decision
        else {
            return Vec::new();
        };
        match kind {
            BattlefieldTargetKind::TargonPeakReady => self.players.iter().enumerate().flat_map(|(player,p)|
                p.rune_objects().iter().map(move |r| Target::Rune { player, rune_id:r.id })).collect(),
            _ => self.players[player].units.iter().filter(|u| u.lane == Some(source.lane)
                && !matches!(kind, BattlefieldTargetKind::StarSpring { played_unit_id } if played_unit_id == u.id))
                .map(|u| Target::Unit { player, unit_id:u.id }).collect(),
        }
    }

    pub fn battlefield_cost_candidates(&self) -> Vec<u32> {
        let Some(Decision::BattlefieldCost { player, source, .. }) = self.decision else {
            return Vec::new();
        };
        self.players[player]
            .units
            .iter()
            .filter(|u| u.lane == Some(source.lane))
            .map(|u| u.id)
            .collect()
    }

    pub(super) fn resolve_battlefield_targets(
        &mut self,
        choice_id: u32,
        targets: Vec<Target>,
    ) -> Result<(), String> {
        let Some(Decision::BattlefieldTargets {
            player,
            choice_id: current,
            source,
            kind,
        }) = self.decision
        else {
            return Err("No battlefield target choice is pending.".into());
        };
        if choice_id != current {
            return Err("That battlefield choice is stale.".into());
        }
        let maximum = if kind == BattlefieldTargetKind::TargonPeakReady {
            2
        } else {
            1
        };
        let candidates = self.battlefield_choice_candidates();
        if targets.len() > maximum
            || targets
                .iter()
                .enumerate()
                .any(|(i, t)| targets[..i].contains(t) || !candidates.contains(t))
        {
            return Err("Choose distinct eligible battlefield targets.".into());
        }
        self.decision = None;
        // Spring/Hall are optional; choosing zero declines. Peak instead has a
        // mandatory trigger with a legal zero-target instruction.
        if targets.is_empty() && kind != BattlefieldTargetKind::TargonPeakReady {
            return Ok(());
        }
        let ability = match kind {
            BattlefieldTargetKind::StarSpring { .. } => {
                BattlefieldAbility::StarSpring { lane: source.lane }
            }
            BattlefieldTargetKind::AbandonedHall => {
                BattlefieldAbility::AbandonedHall { lane: source.lane }
            }
            BattlefieldTargetKind::TargonPeakReady => {
                BattlefieldAbility::TargonPeakReady { lane: source.lane }
            }
        };
        self.queue_finalized_ability(player, 0, targets, ChainAbility::Battlefield(ability));
        Ok(())
    }

    pub(super) fn select_battlefield_cost(
        &mut self,
        choice_id: u32,
        unit_id: Option<u32>,
    ) -> Result<(), String> {
        let Some(Decision::BattlefieldCost {
            player,
            choice_id: current,
            source,
        }) = self.decision
        else {
            return Err("No battlefield cost choice is pending.".into());
        };
        if choice_id != current {
            return Err("That battlefield choice is stale.".into());
        }
        let Some(unit_id) = unit_id else {
            self.decision = None;
            return Ok(());
        };
        if !self.battlefield_cost_candidates().contains(&unit_id) {
            return Err("Pick a friendly unit at this battlefield for the cost.".into());
        }
        self.decision = Some(Decision::BattlefieldPayment {
            player,
            choice_id,
            source,
            unit_id,
            cost: CostValue {
                energy: 1,
                ..CostValue::default()
            },
        });
        Ok(())
    }

    pub(crate) fn resolve_battlefield_payment(
        &mut self,
        choice_id: u32,
        pay: bool,
    ) -> Result<(), String> {
        let Some(Decision::BattlefieldPayment {
            player,
            choice_id: current,
            source,
            unit_id,
            cost,
        }) = self.decision.clone()
        else {
            return Err("No battlefield payment is pending.".into());
        };
        if choice_id != current {
            return Err("That battlefield payment choice is stale.".into());
        }
        if !pay {
            self.rollback_finalization_payment_add();
            self.decision = None;
            return Ok(());
        }
        let target = Target::Unit { player, unit_id };
        if !self
            .unit_ref(target)
            .is_some_and(|u| u.lane == Some(source.lane))
        {
            return Err("The pinned unit cannot pay this return cost.".into());
        }
        if self.payment_energy_available(player) < cost.energy {
            return Err("Not enough Energy for Emperor's Dais.".into());
        }
        self.pay_energy(player, cost.energy);
        // Cost-only selection is not targeting (Core355.10.c.1). Return before
        // finalization; the token effect does not execute until after responses.
        if self.return_unit_to_hand(target).is_none() {
            return Err("The return cost could not be paid.".into());
        }
        self.decision = None;
        self.finalization_payment_origin = None;
        self.queue_finalized_ability(
            player,
            0,
            Vec::new(),
            ChainAbility::Battlefield(BattlefieldAbility::Dais { lane: source.lane }),
        );
        Ok(())
    }

    pub(super) fn dispatch_delayed_battlefields(&mut self) {
        let delayed = std::mem::take(&mut self.delayed_battlefield_triggers);
        for trigger in delayed {
            if trigger.turn == self.turn {
                self.queue_field_targets(
                    trigger.player,
                    trigger.source.lane,
                    BattlefieldTargetKind::TargonPeakReady,
                );
            }
        }
    }

    pub(crate) fn resolve_new_battlefield_ability(
        &mut self,
        player: usize,
        ability: BattlefieldAbility,
        targets: &[Target],
    ) -> Result<(), String> {
        match ability {
            BattlefieldAbility::Dais { lane } => {
                self.queue_token_play(player, 178, 2, Some(Some(lane)), false);
                if let Some(PendingTrigger {
                    effect: PendingTriggerEffect::Token(token),
                    ..
                }) = self.pending_triggers.last_mut()
                {
                    token.source = Some(ChainSource::Battlefield {
                        lane,
                        battlefield_id: 11,
                    });
                }
            }
            BattlefieldAbility::StarSpring { lane } => {
                for &target in targets {
                    if self.unit_ref(target).is_some_and(|u| u.lane == Some(lane))
                        && matches!(target, Target::Unit { player:p, .. } if p == player)
                    {
                        self.move_effect(player, target, None, false)?;
                    }
                }
            }
            BattlefieldAbility::AbandonedHall { lane } => {
                for &target in targets {
                    if self.unit_ref(target).is_some_and(|u| u.lane == Some(lane))
                        && matches!(target, Target::Unit { player:p, .. } if p == player)
                    {
                        self.modify_might(target, 1, 0)?;
                    }
                }
            }
            BattlefieldAbility::TargonPeakDelay { lane } => {
                // Core359.3.e.16: the ending boundary has already happened.
                if !self.ending_turn {
                    let id = self.next_chain_id;
                    self.next_chain_id = id
                        .checked_add(1)
                        .ok_or_else(|| crate::numeric::capacity("delayed occurrence"))?;
                    self.delayed_battlefield_triggers
                        .push(DelayedBattlefieldTrigger {
                            id,
                            player,
                            source: self.field_source(lane),
                            turn: self.turn,
                        });
                }
            }
            BattlefieldAbility::TargonPeakReady { .. } => {
                for &target in targets {
                    if let Target::Rune { player, rune_id } = target {
                        self.ready_rune_object(player, rune_id)?;
                    }
                }
            }
            BattlefieldAbility::ShadowTempleBurn { .. } => self.burn(player, 3)?,
            // The suspended discard/draw instruction is handled before popping
            // this chain item, just like other loot effects.
            BattlefieldAbility::ZaunWarrensLoot { .. } => {}
            _ => return Err("Not a C28 battlefield ability.".into()),
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "field_triggers/tests.rs"]
mod tests;
