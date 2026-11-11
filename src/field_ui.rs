//! Battlefield choices use the engine's public physical objects and candidates.
use super::*;
use local_rift::game::field_triggers::{BattlefieldSource, BattlefieldTargetKind};

pub(super) fn source_name(source: BattlefieldSource) -> &'static str {
    local_rift::battlefields::battlefield(source.battlefield_id).name
}

pub(super) fn payment_name(game: &Game) -> Option<String> {
    let Decision::BattlefieldPayment {
        source, unit_id, ..
    } = game.battlefield_payment_context()?
    else {
        return None;
    };
    let unit = game
        .players
        .iter()
        .flat_map(|p| &p.units)
        .find(|u| u.id == unit_id)?;
    Some(format!(
        "{} · return {}",
        source_name(source),
        game.unit_name(unit)
    ))
}

impl App {
    pub(super) fn field_choice_active(&self) -> bool {
        matches!(
            self.game.decision,
            Some(
                Decision::BattlefieldTargets { player: 0, .. }
                    | Decision::BattlefieldCost { player: 0, .. }
            )
        )
    }

    pub(super) fn field_object_candidate(&self, target: Target) -> bool {
        if self.game.priority() != 0 {
            return false;
        }
        match self.game.decision {
            Some(Decision::BattlefieldTargets { player: 0, .. }) => {
                self.game.battlefield_choice_candidates().contains(&target)
            }
            Some(Decision::BattlefieldCost { player: 0, .. }) => {
                matches!(target, Target::Unit { player: 0, unit_id } if self.game.battlefield_cost_candidates().contains(&unit_id))
            }
            _ => false,
        }
    }

    pub(super) fn field_object_click(&mut self, target: Target) -> bool {
        if !self.field_choice_active() {
            return false;
        }
        if let Some(Decision::BattlefieldCost { choice_id, .. }) = self.game.decision {
            if let Target::Unit { unit_id, .. } = target
                && self.field_object_candidate(target)
            {
                self.action(Action::SelectBattlefieldCost {
                    choice_id,
                    unit_id: Some(unit_id),
                });
            } else {
                self.notify_target_choice(
                    "Choose a highlighted friendly unit here to return as the cost.",
                );
            }
            return true;
        }
        if let Some(i) = self.targets.iter().position(|t| *t == target) {
            self.targets.remove(i);
            self.toast_until = 0.0;
        } else if !self.field_object_candidate(target) {
            self.notify_target_choice("Choose a highlighted object for this battlefield ability.");
        } else {
            let maximum = if matches!(
                self.game.decision,
                Some(Decision::BattlefieldTargets {
                    kind: BattlefieldTargetKind::TargonPeakReady,
                    ..
                })
            ) {
                2
            } else {
                1
            };
            if self.targets.len() < maximum {
                self.targets.push(target);
                self.toast_until = 0.0;
            } else {
                self.notify_target_choice(
                    "All choices are filled. Click a chosen object to undo it.",
                );
            }
        }
        true
    }

    pub(super) fn battlefield_decision_panel(&mut self) -> Result<bool, String> {
        let Some(decision) = self.game.decision.clone() else {
            return Ok(false);
        };
        let (source, title, body, confirm, decline) = match decision {
            Decision::BattlefieldTargets {
                player: 0,
                choice_id,
                source,
                kind,
            } => {
                let (title, body) = match kind {
                    BattlefieldTargetKind::TargonPeakReady => (
                        "Ready up to 2 runes",
                        "Click highlighted runes in either rune zone. Choose zero, one or two distinct runes. Your opponent may respond before they ready.",
                    ),
                    BattlefieldTargetKind::StarSpring { .. } => (
                        "Move another unit?",
                        "Choose another highlighted friendly unit here to move to your base. This ability can be answered before the move.",
                    ),
                    BattlefieldTargetKind::AbandonedHall => (
                        "Give +1 Might?",
                        "Choose a highlighted friendly unit here. The +1 Might lasts this turn; your opponent can respond before it resolves.",
                    ),
                };
                (
                    source,
                    title,
                    body,
                    Some(Action::ResolveBattlefieldTargets {
                        choice_id,
                        targets: self.targets.clone(),
                    }),
                    Action::ResolveBattlefieldTargets {
                        choice_id,
                        targets: vec![],
                    },
                )
            }
            Decision::BattlefieldCost {
                player: 0,
                choice_id,
                source,
            } => (
                source,
                "Choose a return cost",
                "Click a highlighted friendly unit here. Returning it is part of the cost, not a target. You will confirm the energy payment next.",
                None,
                Action::SelectBattlefieldCost {
                    choice_id,
                    unit_id: None,
                },
            ),
            Decision::BattlefieldPayment {
                player: 0,
                choice_id,
                source,
                unit_id,
                cost,
            } => {
                self.ui
                    .title("Pay for this ability?", 1178.0, 366.0, 20.0, TEXT);
                self.ui
                    .wrapped_lines(source_name(source), 1178.0, 391.0, 212.0, 11.0, GOLD, 2);
                let unit = self.game.players[0]
                    .units
                    .iter()
                    .find(|u| u.id == unit_id)
                    .map(|u| self.game.unit_name(u))
                    .unwrap_or_else(|| "the chosen unit".into());
                self.ui.wrapped_lines(
                    &format!(
                        "{} + return {unit} to its owner's hand",
                        play_ui::value_label(&cost)
                    ),
                    1178.0,
                    425.0,
                    212.0,
                    11.0,
                    TEXT,
                    4,
                );
                let action = Action::ResolveBattlefieldPayment {
                    choice_id,
                    pay: true,
                };
                let payable = self.game.clone().apply(action.clone()).is_ok();
                let begin = !payable && self.can_begin_payment(&action);
                if self.ui.button(
                    Rect::new(1178.0, 493.0, 212.0, 34.0),
                    if begin {
                        "Begin payment · use resources"
                    } else {
                        "Pay & return · finalize"
                    },
                    true,
                    payable || begin,
                ) {
                    if begin {
                        self.begin_ui_payment(action);
                    } else {
                        self.action(action);
                    }
                }
                if self.ui.button(
                    Rect::new(1178.0, 534.0, 212.0, 27.0),
                    "Decline this ability",
                    false,
                    true,
                ) {
                    self.action(Action::ResolveBattlefieldPayment {
                        choice_id,
                        pay: false,
                    });
                }
                self.ui.wrapped_lines("Paid before responses. The return is not a target. Counters give no refund. Only resolution plays the token.",1178.0,574.0,212.0,10.0,MUTED,4);
                return Ok(true);
            }
            _ => return Ok(false),
        };
        self.ui.title(title, 1178.0, 366.0, 20.0, TEXT);
        self.ui
            .wrapped_lines(source_name(source), 1178.0, 391.0, 212.0, 11.0, GOLD, 2);
        self.ui
            .wrapped_lines(body, 1178.0, 425.0, 212.0, 10.5, TEXT, 4);
        if let Some(action) = confirm {
            let valid = self.game.clone().apply(action.clone()).is_ok();
            if self.ui.button(
                Rect::new(1178.0, 493.0, 212.0, 34.0),
                "Confirm choices",
                true,
                valid,
            ) {
                self.action(action);
            }
        }
        if self.ui.button(
            Rect::new(1178.0, 534.0, 212.0, 27.0),
            if matches!(decline, Action::ResolveBattlefieldTargets { .. }) {
                "Choose none"
            } else {
                "Decline this ability"
            },
            false,
            true,
        ) {
            self.action(decline);
        }
        let chosen = self
            .targets
            .iter()
            .map(|t| target_label(&self.game, *t))
            .collect::<Vec<_>>()
            .join(" · ");
        self.ui.wrapped_lines(
            &if chosen.is_empty() {
                "No objects chosen.".into()
            } else {
                chosen
            },
            1178.0,
            587.0,
            212.0,
            10.0,
            TEAL,
            3,
        );
        Ok(true)
    }
}
