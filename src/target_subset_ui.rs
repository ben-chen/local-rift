//! Select a legal subset without listing exponentially many combinations.
use super::*;

impl App {
    pub(super) fn subset_candidate(&self, target: Target) -> Result<bool, String> {
        let Some(Decision::ResolutionSubset {
            candidates,
            maximum,
            ..
        }) = &self.game.decision
        else {
            return Ok(false);
        };
        if !candidates.contains(&target) {
            return Ok(false);
        }
        if self.targets.contains(&target) {
            return Ok(true);
        }
        self.game
            .targets_fit_might_budget(&[self.targets.as_slice(), &[target]].concat(), *maximum)
    }

    pub(super) fn toggle_subset(&mut self, target: Target) -> Result<(), String> {
        if let Some(index) = self.targets.iter().position(|t| *t == target) {
            self.targets.remove(index);
        } else if self.subset_candidate(target)? {
            self.targets.push(target);
        } else {
            self.notify_target_choice("Choose an original target that fits the remaining Might. Click a selected unit to remove it.");
        }
        Ok(())
    }

    pub(super) fn subset_panel(&mut self, maximum: u64) -> Result<(), String> {
        self.ui
            .title("Choose remaining targets", 1178.0, 366.0, 19.0, TEXT);
        self.ui.wrapped_lines(
            &format!("Responses changed their Might. Choose original targets totaling at most {maximum}. Include every other target that still fits."),
            1178.0, 396.0, 209.0, 12.0, MUTED, 6,
        );
        let might = self.targets.iter().try_fold(0u64, |sum, &t| {
            let Target::Unit { player, unit_id } = t else {
                return Err("Choose a unit.".to_string());
            };
            let unit = self.game.players[player]
                .units
                .iter()
                .find(|u| u.id == unit_id)
                .ok_or("Missing selected target.")?;
            sum.checked_add(self.game.unit_might(unit)?)
                .ok_or_else(|| "Might total overflow.".to_string())
        })?;
        self.ui.text(
            &format!(
                "{} {} · {might}/{maximum} Might",
                self.targets.len(),
                if self.targets.len() == 1 {
                    "unit"
                } else {
                    "units"
                }
            ),
            1178.0,
            493.0,
            14.0,
            GOLD,
        );
        let valid = self.game.validate_resolution_subset(&self.targets);
        if self.ui.button(
            Rect::new(1178.0, 515.0, 209.0, 35.0),
            "Confirm targets",
            false,
            valid.is_ok(),
        ) {
            self.action(Action::ChooseResolutionSubset {
                targets: self.targets.clone(),
            });
        }
        if let Err(reason) = valid {
            self.ui
                .wrapped_lines(&reason, 1178.0, 570.0, 209.0, 11.0, MUTED, 5);
        }
        Ok(())
    }

    pub(super) fn subset_tray(&mut self, candidates: &[Target]) -> Result<(), String> {
        self.ui.text(
            "Original targets · click to select or remove",
            780.0,
            671.0,
            9.0,
            GOLD,
        );
        self.hand_scroll = self.hand_scroll.min(candidates.len().saturating_sub(6));
        for (slot, &target) in candidates.iter().skip(self.hand_scroll).take(6).enumerate() {
            let Target::Unit { player, unit_id } = target else {
                continue;
            };
            let Some(unit) = self.game.players[player]
                .units
                .iter()
                .find(|u| u.id == unit_id)
            else {
                continue;
            };
            let (card_id, might) = (unit.card_id, self.game.unit_might(unit)?);
            let selected = self.targets.contains(&target);
            let rect = self.tile(card_id, slot, &target_label(&self.game, target), selected);
            self.ui.text(
                &format!(
                    "{might} Might · {}",
                    if selected {
                        "Selected"
                    } else {
                        "Click to choose"
                    }
                ),
                rect.x + 8.0,
                rect.y + 153.0,
                10.0,
                GOLD,
            );
            if self.subset_candidate(target)? {
                self.ui.border(rect, TEAL);
            }
            if self.ui.hover(rect) {
                self.hover_object_candidate = Some((player, unit_id));
                self.hover_candidate = Some(card_id);
            }
            if self.ui.clicked(rect) {
                self.toggle_subset(target)?;
            }
        }
        self.tray_scroll(candidates.len(), 6);
        Ok(())
    }
}
