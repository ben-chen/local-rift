//! Explicit play-time modes, including independently selected Repeat executions.
use super::*;
use rules::{ModeEffect, PlayTargetKind, PlayTargetSpec};

impl Game {
    pub(super) fn play_modes(card_id: CardId) -> Vec<PlayModeOption> {
        rules::spell_modes(card_id)
            .iter()
            .enumerate()
            .map(|(index, mode)| PlayModeOption {
                index,
                label: mode.label,
                target_kinds: match mode.effect {
                    ModeEffect::MoveDeployGear => {
                        vec![PlayTargetKind::Gear, PlayTargetKind::Battlefield]
                    }
                    ModeEffect::Counter(_) => vec![PlayTargetKind::Spell],
                    ModeEffect::InspectGear | ModeEffect::Tokens { .. } => vec![],
                },
            })
            .collect()
    }

    pub(super) fn validate_play_mode(
        card: CardId,
        mode: Option<usize>,
        complete: bool,
    ) -> Result<(), String> {
        let modes = rules::spell_modes(card);
        if modes.is_empty() {
            if mode.is_some() {
                return Err("This card has no mode selection.".into());
            }
        } else if mode.is_some_and(|m| m >= modes.len()) || (complete && mode.is_none()) {
            return Err("Choose one of this card's modes.".into());
        }
        Ok(())
    }

    pub(super) fn execution_target_spec(
        card: CardId,
        mode: Option<usize>,
        slot: usize,
    ) -> PlayTargetSpec {
        if card == 442 {
            return PlayTargetSpec {
                kind: Some(if slot == 0 {
                    PlayTargetKind::Unit
                } else {
                    PlayTargetKind::Spell
                }),
                maximum: 2,
                required: 2,
            };
        }
        let modes = rules::spell_modes(card);
        if modes.is_empty() {
            return rules::play_target_spec(card);
        }
        match mode.and_then(|m| modes.get(m)).map(|m| m.effect) {
            Some(ModeEffect::MoveDeployGear) => PlayTargetSpec {
                kind: Some(if slot == 0 {
                    PlayTargetKind::Gear
                } else {
                    PlayTargetKind::Battlefield
                }),
                maximum: 2,
                required: 2,
            },
            Some(ModeEffect::Counter(_)) => PlayTargetSpec {
                kind: Some(PlayTargetKind::Spell),
                maximum: 1,
                required: 1,
            },
            _ => PlayTargetSpec {
                kind: None,
                maximum: 0,
                required: 0,
            },
        }
    }

    pub(super) fn modal_target_candidates(
        &self,
        actor: usize,
        card: CardId,
        mode: Option<usize>,
        prior: &[Target],
    ) -> Vec<Target> {
        match mode
            .and_then(|m| rules::spell_modes(card).get(m))
            .map(|m| m.effect)
        {
            Some(ModeEffect::MoveDeployGear) => {
                self.command_attack_candidates(actor, Some(0), prior)
            }
            Some(ModeEffect::Counter(kind)) if prior.is_empty() => self
                .chain
                .iter()
                .filter(|item| self.counter_kind_target_valid(actor, kind, item.id))
                .map(|item| Target::Spell { chain_id: item.id })
                .collect(),
            _ => vec![],
        }
    }

    pub(super) fn validate_modal_targets(
        &self,
        actor: usize,
        card: CardId,
        mode: Option<usize>,
        targets: &[Target],
        complete: bool,
    ) -> Result<(), String> {
        Self::validate_play_mode(card, mode, complete)?;
        let spec = Self::execution_target_spec(card, mode, targets.len());
        if targets.len() > spec.maximum || (complete && targets.len() < spec.required) {
            return Err("Choose the required targets for the selected mode.".into());
        }
        for (index, target) in targets.iter().enumerate() {
            if !self
                .modal_target_candidates(actor, card, mode, &targets[..index])
                .contains(target)
            {
                return Err("A target is not legal for the selected mode.".into());
            }
        }
        Ok(())
    }

    pub(super) fn modal_execution_options(
        &self,
        player: usize,
        card: CardId,
    ) -> Vec<ExecutionChoices> {
        // Preserve Command's established ordering while sharing finalization.
        if card == 410 {
            return self.command_execution_options(player);
        }
        let mut executions = Vec::new();
        for (index, mode) in rules::spell_modes(card).iter().enumerate() {
            match mode.effect {
                ModeEffect::Counter(_) => {
                    for target in self.modal_target_candidates(player, card, Some(index), &[]) {
                        executions.push(ExecutionChoices {
                            destination: None,
                            mode: Some(index),
                            targets: vec![target],
                        });
                    }
                }
                ModeEffect::Tokens { .. } | ModeEffect::InspectGear => {
                    executions.push(ExecutionChoices {
                        destination: None,
                        mode: Some(index),
                        targets: vec![],
                    })
                }
                ModeEffect::MoveDeployGear => {
                    for gear in self.modal_target_candidates(player, card, Some(index), &[]) {
                        for field in
                            self.modal_target_candidates(player, card, Some(index), &[gear])
                        {
                            executions.push(ExecutionChoices {
                                destination: None,
                                mode: Some(index),
                                targets: vec![gear, field],
                            });
                        }
                    }
                }
            }
        }
        executions
    }

    pub(super) fn resolve_modal_execution(
        &mut self,
        item: &ChainItem,
        mode: Option<usize>,
    ) -> Result<(), String> {
        let selected = mode
            .and_then(|m| rules::spell_modes(item.card_id).get(m))
            .ok_or("The finalized spell execution has no valid mode.")?;
        match selected.effect {
            ModeEffect::MoveDeployGear => self.move_deploy_gear(item.player, &item.targets),
            ModeEffect::InspectGear => {
                self.look_to_draw(item.player, item.card_id, DrawLookKind::CommandAttack)
            }
            ModeEffect::Counter(kind) => self.resolve_counter_kind(item, kind),
            ModeEffect::Tokens { count, might } => {
                self.queue_effect_tokens(item.player, item.card_id, count, might, item.lane)
            }
        }
        Ok(())
    }

    pub(super) fn modal_choice_score(
        &self,
        actor: usize,
        card: CardId,
        mode: Option<usize>,
        targets: &[Target],
    ) -> i128 {
        match mode
            .and_then(|m| rules::spell_modes(card).get(m))
            .map(|m| m.effect)
        {
            Some(ModeEffect::MoveDeployGear | ModeEffect::InspectGear) => {
                self.command_choice_score(actor, mode, targets)
            }
            Some(ModeEffect::Tokens { count, might }) => {
                i128::from(count) * (10 + i128::from(might))
            }
            Some(ModeEffect::Counter(_)) => targets
                .first()
                .and_then(|target| match target {
                    Target::Spell { chain_id } => {
                        self.chain.iter().find(|item| item.id == *chain_id)
                    }
                    _ => None,
                })
                .map_or(0, |item| {
                    let card = self.catalog[item.card_id];
                    let value = 40 + i128::from(card.cost) * 12 + i128::from(card.power_cost) * 15;
                    if item.player == actor { -value } else { value }
                }),
            None => 0,
        }
    }
}
