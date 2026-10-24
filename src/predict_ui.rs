//! A private, local draft: no deck changes until the complete partition is confirmed.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PredictDraft {
    pub choice_id: u32,
    pub recycle: Vec<usize>,
    pub top_order: Vec<usize>,
}
impl PredictDraft {
    fn new(choice_id: u32, count: usize) -> Self {
        Self {
            choice_id,
            recycle: vec![],
            top_order: (0..count).collect(),
        }
    }
    fn toggle(&mut self, index: usize) {
        if let Some(position) = self.recycle.iter().position(|i| *i == index) {
            self.recycle.remove(position);
            self.top_order.push(index);
        } else if let Some(position) = self.top_order.iter().position(|i| *i == index) {
            self.top_order.remove(position);
            self.recycle.push(index);
        }
    }
    fn action(&self) -> Action {
        Action::ResolvePredict {
            choice_id: self.choice_id,
            recycle: self.recycle.clone(),
            top_order: self.top_order.clone(),
        }
    }
    fn move_kept(&mut self, index: usize, later: bool) {
        let Some(position) = self.top_order.iter().position(|&i| i == index) else {
            return;
        };
        let other = if later {
            position.checked_add(1)
        } else {
            position.checked_sub(1)
        };
        if let Some(other) = other.filter(|&i| i < self.top_order.len()) {
            self.top_order.swap(position, other);
        }
    }
}
impl App {
    pub(super) fn sync_predict_draft(&mut self) {
        if let Some(Decision::PredictCards {
            player: 0,
            choice_id,
            cards,
        }) = &self.game.decision
        {
            if self
                .predict_draft
                .as_ref()
                .is_none_or(|draft| draft.choice_id != *choice_id)
            {
                self.predict_draft = Some(PredictDraft::new(*choice_id, cards.len()));
                self.selection = None;
                self.hover_card = None;
                self.hover_object = None;
                self.hover_rect = None;
            }
        } else {
            self.predict_draft = None;
        }
    }
    pub(super) fn reset_predict_draft(&mut self) {
        self.predict_draft = None;
        self.sync_predict_draft();
    }
    pub(super) fn predict_panel(&mut self) {
        let Some(draft) = self.predict_draft.clone() else {
            return;
        };
        self.ui.title("Predict", 1178.0, 366.0, 22.0, TEXT);
        self.ui.wrapped_lines("Only you see these cards. Choose any to recycle to the bottom, then order the cards you keep on top. Nothing changes until you confirm.", 1178.0, 397.0, 209.0, 12.0, TEXT, 5);
        self.ui.text(
            &format!(
                "{} kept · {} recycled",
                draft.top_order.len(),
                draft.recycle.len()
            ),
            1178.0,
            477.0,
            11.0,
            GOLD,
        );
        let action = draft.action();
        if self.ui.button(
            Rect::new(1178.0, 493.0, 212.0, 34.0),
            "Confirm Predict",
            true,
            self.game.clone().apply(action.clone()).is_ok(),
        ) {
            self.action(action);
        }
        if self.ui.button(
            Rect::new(1178.0, 534.0, 212.0, 27.0),
            "Reset choices · unspent",
            false,
            true,
        ) {
            self.reset_predict_draft();
        }
    }
    pub(super) fn predict_tray(&mut self, cards: &[usize]) {
        let Some(draft) = self.predict_draft.clone() else {
            return;
        };
        self.ui.text(
            "PRIVATE · CHOOSE WHAT STAYS ON TOP",
            766.0,
            671.0,
            10.0,
            GOLD,
        );
        for (index, card_id) in cards.iter().enumerate() {
            let x = 255.0 + index as f32 * 240.0;
            let r = Rect::new(x, 700.0, 220.0, 135.0);
            let recycled = draft.recycle.contains(&index);
            self.ui.rect(r, PANEL);
            self.ui.border(r, if recycled { MUTED } else { TEAL });
            if let Some(texture) = &self.art[*card_id] {
                let bounds = Rect::new(
                    texture.width() * 0.08,
                    texture.height() * 0.12,
                    texture.width() * 0.84,
                    texture.height() * 0.43,
                );
                let ratio = (bounds.w / 216.0).min(bounds.h / 87.0);
                let crop = Rect::new(
                    bounds.x + (bounds.w - 216.0 * ratio) / 2.0,
                    bounds.y + (bounds.h - 87.0 * ratio) / 2.0,
                    216.0 * ratio,
                    87.0 * ratio,
                );
                self.ui.image(
                    texture,
                    Rect::new(x + 2.0, 702.0, 216.0, 87.0),
                    Some(crop),
                    WHITE,
                );
            }
            self.ui.ellipsis(
                self.game.catalog[*card_id].name,
                x + 9.0,
                806.0,
                202.0,
                12.0,
                TEXT,
            );
            let status = if recycled {
                "Recycle · bottom".to_string()
            } else {
                let position = draft.top_order.iter().position(|i| *i == index).unwrap();
                if position == 0 {
                    "1 · next draw".to_string()
                } else {
                    format!("{} · draw later", position + 1)
                }
            };
            self.ui.text(
                &status,
                x + 9.0,
                824.0,
                11.0,
                if recycled { MUTED } else { GOLD },
            );
            if cards.len() > 2 {
                let position = draft.top_order.iter().position(|&i| i == index);
                for (later, offset, label, enabled) in [
                    (false, 156.0, "<", position.is_some_and(|p| p > 0)),
                    (
                        true,
                        188.0,
                        ">",
                        position.is_some_and(|p| p + 1 < draft.top_order.len()),
                    ),
                ] {
                    if self.ui.button(
                        Rect::new(x + offset, 809.0, 28.0, 26.0),
                        label,
                        false,
                        enabled,
                    ) {
                        self.predict_draft.as_mut().unwrap().move_kept(index, later);
                    }
                }
            }
            if self.ui.button(
                Rect::new(x, 843.0, 220.0, 31.0),
                if recycled {
                    "Keep on top"
                } else {
                    "Recycle this card"
                },
                false,
                true,
            ) {
                self.predict_draft.as_mut().unwrap().toggle(index);
            }
            if self.ui.hover(r) {
                self.hover_candidate = Some(*card_id);
            }
            if self.ui.hover(r) && self.ui.secondary_pressed() {
                self.inspect = Some(*card_id);
            }
        }
        if cards.len() > 2 {
            self.ui.title("Draw order", 977.0, 724.0, 17.0, TEXT);
            self.ui.wrapped_lines(
                "1 is your next draw. Use each card's arrows to move it earlier or later. Recycled cards go to the bottom in random order.",
                977.0, 750.0, 144.0, 12.0, MUTED, 7,
            );
            return;
        }
        self.ui.title("Kept-card order", 770.0, 724.0, 18.0, TEXT);
        self.ui.wrapped_lines(
            "The card marked 1 is your next draw. Recycled cards go to the bottom in random order.",
            770.0,
            750.0,
            335.0,
            12.0,
            MUTED,
            3,
        );
        if self.ui.button(
            Rect::new(770.0, 811.0, 335.0, 32.0),
            "Swap kept order",
            false,
            draft.top_order.len() == 2,
        ) {
            self.predict_draft.as_mut().unwrap().top_order.swap(0, 1);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_cards_keep_occurrence_identity_and_order() {
        let mut draft = PredictDraft::new(7, 2);
        draft.toggle(0);
        assert_eq!(draft.top_order, vec![1]);
        assert_eq!(draft.recycle, vec![0]);
        draft.toggle(0);
        assert_eq!(draft.top_order, vec![1, 0]);
        draft.toggle(1);
        draft.toggle(0);
        assert!(draft.top_order.is_empty());
        assert_eq!(draft.recycle, vec![1, 0]);
    }
    #[test]
    fn three_occurrences_can_reach_every_draw_order_without_recycling() {
        let mut seen = std::collections::BTreeSet::new();
        let mut pending = vec![PredictDraft::new(18, 3)];
        while let Some(draft) = pending.pop() {
            if !seen.insert(draft.top_order.clone()) {
                continue;
            }
            assert!(draft.recycle.is_empty());
            for index in 0..3 {
                for later in [false, true] {
                    let mut next = draft.clone();
                    next.move_kept(index, later);
                    pending.push(next);
                }
            }
        }
        assert_eq!(seen.len(), 6);
    }
}
