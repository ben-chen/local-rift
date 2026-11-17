//! One shared selection panel for non-targeting each-player instructions.
use super::*;

impl App {
    pub(super) fn player_object_panel(&mut self) -> Result<(), String> {
        let Some(Decision::PlayerObject {
            player,
            choice_id,
            card_id,
            rule,
            remaining,
        }) = self.game.decision.clone()
        else {
            return Ok(());
        };
        self.ui.title(
            if player == 0 {
                "Your resolution choice"
            } else {
                "Opponent is choosing"
            },
            1178.0,
            366.0,
            18.0,
            TEXT,
        );
        self.ui.wrapped_lines(
            self.game.catalog[card_id].name,
            1178.0,
            395.0,
            209.0,
            13.0,
            GOLD,
            2,
        );
        self.ui.wrapped_lines(
            rule.kind.instruction(),
            1178.0,
            432.0,
            209.0,
            12.0,
            MUTED,
            3,
        );
        if player != 0 {
            return Ok(());
        }
        let object = self
            .targets
            .first()
            .copied()
            .filter(|object| self.game.player_object_candidates().contains(object));
        if self.ui.button(
            Rect::new(1178.0, 490.0, 209.0, 34.0),
            "Confirm choice",
            true,
            object.is_some() && self.game.validate_player_object(choice_id, object).is_ok(),
        ) {
            self.action(Action::ChoosePlayerObject { choice_id, object });
        }
        if rule.optional
            && self.ui.button(
                Rect::new(1178.0, 531.0, 209.0, 29.0),
                "Decline return",
                false,
                true,
            )
        {
            self.action(Action::ChoosePlayerObject {
                choice_id,
                object: None,
            });
        }
        self.ui.wrapped_lines(if remaining.is_empty() {
            "The spell finishes after this choice. These choices are not targets."
        } else {
            "Your opponent chooses next. These choices are not targets; no responses are played between them."
        },1178.0,576.0,209.0,11.0,MUTED,5);
        Ok(())
    }

    pub(super) fn player_object_tray(&mut self) {
        let candidates = self.game.player_object_candidates();
        self.ui.text(
            "Eligible cards · select one, then confirm",
            780.0,
            671.0,
            9.0,
            GOLD,
        );
        self.hand_scroll = self.hand_scroll.min(candidates.len().saturating_sub(6));
        for (slot, &object) in candidates.iter().skip(self.hand_scroll).take(6).enumerate() {
            let (owner, id, card, location, detail) = match object {
                Target::Unit { player, unit_id } => {
                    let Some(unit) = self.game.players[player]
                        .units
                        .iter()
                        .find(|u| u.id == unit_id)
                    else {
                        continue;
                    };
                    let location = format!(
                        "{} · {}",
                        unit.lane
                            .map_or_else(|| "Base".into(), |lane| format!("Field {}", lane + 1)),
                        if unit.ready { "ready" } else { "exhausted" }
                    );
                    let detail = if self.game.board_owner(object) == Some(0) {
                        "Returns to your hand"
                    } else {
                        "Returns to opponent"
                    }
                    .to_string();
                    (player, unit_id, unit.card_id, location, detail)
                }
                Target::Gear { player, gear_id } => {
                    let Some(gear) = self.game.players[player]
                        .gear
                        .iter()
                        .find(|g| g.id == gear_id)
                    else {
                        continue;
                    };
                    let detail = gear
                        .attached_to
                        .and_then(|id| {
                            self.game
                                .players
                                .iter()
                                .flat_map(|p| &p.units)
                                .find(|u| u.id == id)
                        })
                        .map_or_else(
                            || "Unattached gear".into(),
                            |unit| format!("On {}", self.game.catalog[unit.card_id].name),
                        );
                    (
                        player,
                        gear_id,
                        gear.card_id,
                        gear_location_label(&self.game, player, gear_id),
                        detail,
                    )
                }
                _ => continue,
            };
            let selected = self.targets.contains(&object);
            let rect = self.tile(card, slot, &target_label(&self.game, object), selected);
            self.ui.border(rect, if selected { GOLD } else { TEAL });
            self.ui.pill(
                rect.x + 6.0,
                rect.y + 7.0,
                if owner == 0 { "YOU" } else { "OPPONENT" },
                if owner == 0 { TEAL } else { GOLD },
            );
            // Replace printed play costs with information relevant to choosing
            // this exact object already in play, including its owner on return.
            self.ui.rect(
                Rect::new(rect.x + 2.0, rect.y + 124.0, rect.w - 4.0, 52.0),
                PANEL,
            );
            self.ui
                .ellipsis(&location, rect.x + 8.0, rect.y + 137.0, 124.0, 9.0, MUTED);
            self.ui
                .ellipsis(&detail, rect.x + 8.0, rect.y + 151.0, 124.0, 9.0, MUTED);
            self.ui.text(
                if selected {
                    "Selected"
                } else {
                    "Click to choose"
                },
                rect.x + 8.0,
                rect.y + 169.0,
                10.0,
                GOLD,
            );
            if self.ui.hover(rect) {
                self.hover_candidate = Some(card);
                self.hover_object_candidate = Some((owner, id));
            }
            if self.ui.clicked(rect) {
                self.toggle_target(object);
            }
            if self.ui.hover(rect) && self.ui.secondary_pressed() {
                self.hover_object = Some((owner, id));
                self.inspect = Some(card);
            }
        }
        self.tray_scroll(candidates.len(), 6);
    }
}
