//! The banished trash card pays a cost; only the unit is an effect target.
use super::*;

impl App {
    pub(super) fn gust_trash_payment_panel(
        &mut self,
        choice_id: u32,
        targets: &[Target],
        deflect: u16,
    ) -> Result<(), String> {
        self.ui
            .title("Gust Monk · banish cost", 1178.0, 366.0, 18.0, TEXT);
        self.ui.wrapped_lines("Choose one card from either trash below. Banish it to finalize the ability; the opponent can then respond.",1178.0,398.0,212.0,11.0,TEXT,4);
        self.ui.wrapped_lines(
            &targets
                .iter()
                .map(|&t| target_label(&self.game, t))
                .collect::<Vec<_>>()
                .join(" · "),
            1178.0,
            466.0,
            212.0,
            11.0,
            TEAL,
            2,
        );
        if deflect > 0 {
            self.ui.text(
                &format!("Also pay {deflect} Any Power · Deflect"),
                1178.0,
                510.0,
                10.0,
                GOLD,
            );
        }
        if self.ui.button(
            Rect::new(1178.0, 534.0, 212.0, 27.0),
            "Decline this ability",
            false,
            true,
        ) {
            self.action(Action::ResolveTriggerPayment {
                choice_id,
                pay: false,
            });
        }
        self.ui.wrapped_lines("The banished card is a cost, not a target. Assault 2 is gained only when the ability resolves, and lasts this turn.",1178.0,579.0,212.0,10.0,MUTED,4);
        Ok(())
    }

    pub(super) fn gust_trash_cost_tray(&mut self, choice_id: u32) {
        let cards: Vec<_> = self
            .game
            .players
            .iter()
            .enumerate()
            .flat_map(|(owner, p)| {
                p.discard
                    .iter()
                    .enumerate()
                    .map(move |(index, &id)| (owner, index, id))
            })
            .collect();
        self.ui
            .text("BANISH COST · BOTH TRASHES", 780.0, 671.0, 9.0, GOLD);
        self.hand_scroll = self.hand_scroll.min(cards.len().saturating_sub(6));
        if cards.is_empty() {
            self.ui
                .title("Both trashes are empty.", 270.0, 757.0, 23.0, MUTED);
            self.ui.text(
                "Decline this ability. Gust Monk remains in play.",
                270.0,
                789.0,
                13.0,
                TEXT,
            );
        }
        for (slot, &(owner, index, id)) in cards.iter().skip(self.hand_scroll).take(6).enumerate() {
            let r = self.tile(id, slot, self.game.catalog[id].name, false);
            self.ui
                .rect(Rect::new(r.x + 3.0, r.y + 125.0, r.w - 6.0, 53.0), PANEL);
            self.ui.text(
                if owner == 0 {
                    "Your trash"
                } else {
                    "Opponent's trash"
                },
                r.x + 9.0,
                r.y + 139.0,
                10.0,
                GOLD,
            );
            let action = Action::ResolveTriggerTrashPayment {
                choice_id,
                trash_player: owner,
                trash_index: index,
            };
            let payable = self.game.clone().apply(action.clone()).is_ok();
            let generate = !payable && self.can_begin_payment(&action);
            if self.ui.button(
                Rect::new(r.x + 7.0, r.y + 148.0, r.w - 14.0, 25.0),
                if generate {
                    "Choose · pay resources"
                } else {
                    "Banish · pay cost"
                },
                true,
                payable || generate,
            ) {
                if generate {
                    self.begin_ui_payment(action);
                } else {
                    self.action(action);
                }
                break;
            }
            let face = Rect::new(r.x, r.y, r.w, 123.0);
            if self.ui.hover(face) {
                self.hover_candidate = Some(id);
                if self.ui.secondary_pressed() {
                    self.inspect_printed_card(id);
                }
            }
        }
        self.tray_scroll(cards.len(), 6);
    }
}
