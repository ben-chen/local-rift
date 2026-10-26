//! Battlefield gear and Command's explicit, per-execution modal choices.
use super::*;

impl Game {
    pub(super) fn play_modes(card_id: CardId) -> Vec<PlayModeOption> {
        if card_id != 410 {
            return Vec::new();
        }
        vec![
            PlayModeOption {
                index: 0,
                label: "Move a friendly Deploy gear to a battlefield",
                target_kinds: vec![
                    rules::PlayTargetKind::Gear,
                    rules::PlayTargetKind::Battlefield,
                ],
            },
            PlayModeOption {
                index: 1,
                label: "Inspect five cards and optionally draw a Gear",
                target_kinds: vec![],
            },
        ]
    }

    pub(super) fn validate_play_mode(
        card_id: CardId,
        mode: Option<usize>,
        complete: bool,
    ) -> Result<(), String> {
        if card_id == 410 {
            if mode.is_some_and(|m| m > 1) || (complete && mode.is_none()) {
                return Err("Choose one of Command: Attack's two modes.".into());
            }
        } else if mode.is_some() {
            return Err("This card has no mode selection.".into());
        }
        Ok(())
    }

    pub(super) fn execution_target_spec(
        card_id: CardId,
        mode: Option<usize>,
        slot: usize,
    ) -> rules::PlayTargetSpec {
        if card_id != 410 {
            return rules::play_target_spec(card_id);
        }
        if mode == Some(0) {
            rules::PlayTargetSpec {
                kind: Some(if slot == 0 {
                    rules::PlayTargetKind::Gear
                } else {
                    rules::PlayTargetKind::Battlefield
                }),
                maximum: 2,
                required: 2,
            }
        } else {
            rules::PlayTargetSpec {
                kind: None,
                maximum: 0,
                required: 0,
            }
        }
    }

    pub(super) fn command_attack_candidates(
        &self,
        actor: usize,
        mode: Option<usize>,
        prior: &[Target],
    ) -> Vec<Target> {
        if mode != Some(0) {
            return Vec::new();
        }
        if prior.is_empty() {
            self.players[actor]
                .gear
                .iter()
                .filter(|gear| {
                    self.gear_deploy_status(actor, gear.id)
                        .is_some_and(|s| s.characteristic)
                })
                .map(|gear| Target::Gear {
                    player: actor,
                    gear_id: gear.id,
                })
                .collect()
        } else if let [Target::Gear { player, gear_id }] = prior {
            self.players
                .get(*player)
                .and_then(|p| p.gear.iter().find(|g| g.id == *gear_id))
                .filter(|gear| {
                    *player == actor
                        && self
                            .gear_deploy_status(actor, gear.id)
                            .is_some_and(|s| s.characteristic)
                })
                .map_or_else(Vec::new, |gear| {
                    (0..BATTLEFIELDS)
                        .filter(|&lane| self.gear_location(actor, gear.id) != Some(lane))
                        .map(|lane| Target::Battlefield { lane })
                        .collect()
                })
        } else {
            Vec::new()
        }
    }

    pub(super) fn validate_command_attack(
        &self,
        actor: usize,
        mode: Option<usize>,
        targets: &[Target],
        complete: bool,
    ) -> Result<(), String> {
        Self::validate_play_mode(410, mode, complete)?;
        let spec = Self::execution_target_spec(410, mode, targets.len());
        if targets.len() > spec.maximum || (complete && targets.len() < spec.required) {
            return Err("The selected Command mode requires a Deploy gear and a different battlefield, or no targets for the draw mode.".into());
        }
        for (index, target) in targets.iter().enumerate() {
            if !self
                .command_attack_candidates(actor, mode, &targets[..index])
                .contains(target)
            {
                return Err("Choose a friendly Deploy gear, then a different battlefield.".into());
            }
        }
        Ok(())
    }

    pub(super) fn command_execution_options(&self, player: usize) -> Vec<ExecutionChoices> {
        let mut executions = vec![ExecutionChoices {
            mode: Some(1),
            targets: vec![],
        }];
        for gear in self.command_attack_candidates(player, Some(0), &[]) {
            for field in self.command_attack_candidates(player, Some(0), &[gear]) {
                executions.push(ExecutionChoices {
                    mode: Some(0),
                    targets: vec![gear, field],
                });
            }
        }
        executions
    }

    pub(super) fn command_play_candidates(
        &self,
        action: &Action,
        prepare: bool,
        limit: usize,
    ) -> Result<Vec<Action>, String> {
        let executions = self.command_execution_options(self.priority());
        let mut result = Vec::new();
        let mut capacity = None;
        for execution in &executions {
            let inner = match action {
                Action::PlayCard { hand_index, .. } => Action::PlayCardWithTargets {
                    hand_index: *hand_index,
                    lane: None,
                    targets: execution.targets.clone(),
                },
                Action::PlayFromTrash { trash_index, .. } => Action::PlayFromTrashWithTargets {
                    trash_index: *trash_index,
                    lane: None,
                    targets: execution.targets.clone(),
                },
                Action::FinalizePendingPlay { .. } => Action::FinalizePendingPlay {
                    lane: None,
                    targets: execution.targets.clone(),
                    repeat_targets: vec![],
                    repeat: false,
                    accelerate: false,
                    bonus: false,
                },
                _ => continue,
            };
            let base = PlayChoices {
                mode: execution.mode,
                ..Default::default()
            };
            let wrapped = Action::WithPlayChoices {
                action: Box::new(inner.clone()),
                choices: base.clone(),
            };
            let view = self.play_choices(&wrapped)?;
            let mut selections = vec![base.clone()];
            for repeat in view.repeats.iter().take(3) {
                for repeated in &executions {
                    let mut choice = base.clone();
                    choice.repeats.push(RepeatSelection {
                        option_id: repeat.id,
                        choices: repeated.clone(),
                    });
                    selections.push(choice);
                }
            }
            for choices in selections {
                let action = Action::WithPlayChoices {
                    action: Box::new(inner.clone()),
                    choices,
                };
                let legal = if prepare {
                    self.clone().begin_payment(action.clone())
                } else {
                    self.probe_action(&action)
                };
                match legal {
                    Ok(()) => result.push(action),
                    Err(error) if crate::numeric::is_capacity(&error) => capacity = Some(error),
                    Err(_) => {}
                }
                if result.len() >= limit {
                    return Ok(result);
                }
            }
        }
        if result.is_empty()
            && let Some(error) = capacity
        {
            return Err(error);
        }
        Ok(result)
    }

    pub(super) fn command_choice_score(
        &self,
        actor: usize,
        mode: Option<usize>,
        targets: &[Target],
    ) -> i128 {
        match mode {
            Some(1) => {
                if self.players[actor].hand.len() < 7 {
                    20
                } else {
                    5
                }
            }
            Some(0) => {
                let [
                    Target::Gear { player, gear_id },
                    Target::Battlefield { lane },
                ] = targets
                else {
                    return 0;
                };
                let Some(gear) = self.players[*player].gear.iter().find(|g| g.id == *gear_id)
                else {
                    return 0;
                };
                if gear.attached_to.is_some() {
                    return 0;
                }
                let old = self.gear_location(*player, *gear_id);
                let rescued =
                    old.is_some_and(|field| self.battlefield_owner[field] == Some(1 - actor));
                (self.lane_count(actor, *lane) as i128
                    - old.map_or(0, |field| self.lane_count(actor, field)) as i128)
                    * 12
                    + if rescued { 35 } else { 0 }
            }
            _ => 0,
        }
    }

    pub(super) fn deploy_choice(
        &self,
        decision: &Decision,
        actions: &[Action],
    ) -> Result<Option<Action>, String> {
        let Decision::TriggerTargets {
            player,
            card_id: card @ (404..=406),
            context,
            ..
        } = decision
        else {
            return Ok(None);
        };
        let mut scored = Vec::new();
        for action in actions {
            let Action::ChooseTriggerTargets { targets } = action else {
                continue;
            };
            let mut value = 0;
            for &target in targets {
                let Some(unit) = self.unit_ref(target) else {
                    continue;
                };
                value += match *card {
                    406 => {
                        if matches!(target,Target::Unit { player: owner, .. } if owner==*player)
                            && !unit.ready
                        {
                            40 + i128::from(self.unit_might(unit)?)
                        } else {
                            -10
                        }
                    }
                    405 => {
                        if unit.buff == 0 {
                            20 + i128::from(self.unit_might(unit)?)
                                + if unit.lane.is_some() { 10 } else { 0 }
                        } else {
                            0
                        }
                    }
                    404 => {
                        let lane = context
                            .source
                            .gear_id
                            .and_then(|id| self.gear_location(*player, id));
                        let strength = self.players[*player]
                            .units
                            .iter()
                            .filter(|ally| lane.is_some() && ally.lane == lane)
                            .try_fold(0u128, |total, ally| {
                                self.unit_might(ally).map(|might| total + u128::from(might))
                            })?;
                        if strength > u128::from(self.unit_might(unit)?) {
                            20
                        } else {
                            -10
                        }
                    }
                    _ => 0,
                };
            }
            scored.push((value, action));
        }
        Ok(scored
            .into_iter()
            .max_by_key(|(value, _)| *value)
            .map(|(_, action)| action.clone()))
    }

    pub(super) fn move_deploy_gear(&mut self, actor: usize, targets: &[Target]) {
        // Resolution rechecks the physical object and destination. Gear alone
        // neither contests a battlefield nor establishes control (Core830).
        if self
            .validate_command_attack(actor, Some(0), targets, true)
            .is_err()
        {
            return;
        }
        let [
            Target::Gear { player, gear_id },
            Target::Battlefield { lane },
        ] = targets
        else {
            return;
        };
        let gear = self.players[*player]
            .gear
            .iter_mut()
            .find(|gear| gear.id == *gear_id)
            .unwrap();
        // Core718.5.b permits targeting an attached card; 718.5.c prevents
        // only the independent Move, without invalidating finalization (358.3.a).
        if gear.attached_to.is_some() {
            return;
        }
        gear.lane = Some(*lane);
        let card_id = gear.card_id;
        self.push_log(format!(
            "{} moves to battlefield {}.",
            self.catalog[card_id].name,
            lane + 1
        ));
    }
}

#[cfg(test)]
#[path = "deploy/tests.rs"]
mod tests;
