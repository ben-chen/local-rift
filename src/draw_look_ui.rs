//! A private look stages an exact occurrence; its later public reveal is read-only.
use super::*;
use local_rift::game::{DrawLookKind, RevealOutcome};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DrawLookDraft {
    pub choice_id: u32,
    pub selection: Option<Option<usize>>,
}

impl DrawLookDraft {
    fn new(choice_id: u32) -> Self {
        Self {
            choice_id,
            selection: None,
        }
    }
    pub(super) fn select(&mut self, index: usize, eligible: &[usize]) {
        if eligible.contains(&index) {
            self.selection = Some(Some(index));
        }
    }
    fn action(&self) -> Option<Action> {
        self.selection.map(|index| Action::ResolveDrawLook {
            choice_id: self.choice_id,
            index,
        })
    }
}

fn eligibility_hint(kind: DrawLookKind) -> &'static str {
    match kind {
        DrawLookKind::DoubleTrouble => {
            "Choose a unit to reveal and draw. Spells and gear cannot be chosen."
        }
        DrawLookKind::FateWeaver => {
            "Choose a spell with printed Energy cost 4 or more to reveal and draw."
        }
        DrawLookKind::TrifarianCaptain => {
            "Choose a unit or gear with printed costs of at most 3 Energy AND 1 Power to reveal and draw."
        }
    }
}

pub(super) fn ineligible_hint(
    kind: DrawLookKind,
    card: &local_rift::cards::CardDef,
) -> &'static str {
    match kind {
        DrawLookKind::DoubleTrouble => "Requires a unit",
        DrawLookKind::FateWeaver if card.kind != CardKind::Spell => "Requires a spell",
        DrawLookKind::FateWeaver => "Printed Energy below 4",
        DrawLookKind::TrifarianCaptain if card.kind == CardKind::Spell => "Requires unit or gear",
        DrawLookKind::TrifarianCaptain if card.cost > 3 && card.power_cost > 1 => {
            "Energy above 3 · Power above 1"
        }
        DrawLookKind::TrifarianCaptain if card.cost > 3 => "Printed Energy above 3",
        DrawLookKind::TrifarianCaptain if card.power_cost > 1 => "Printed Power above 1",
        _ => "Not eligible for this draw",
    }
}

pub(super) fn reveal_outcome_text(player: usize, outcome: RevealOutcome) -> String {
    let owner = if player == 0 {
        "your"
    } else {
        "your opponent's"
    };
    match outcome {
        RevealOutcome::Draw => format!(
            "This card was revealed and drawn into {owner} hand. Continue only acknowledges the result; it does not draw again."
        ),
        RevealOutcome::Recycle => format!(
            "This card was revealed and recycled into {owner} deck. Continue only acknowledges the result."
        ),
        RevealOutcome::Trash => format!(
            "This card was revealed and put into {owner} trash. Continue only acknowledges the result."
        ),
    }
}

impl App {
    pub(super) fn sync_draw_look_draft(&mut self) {
        let next = match &self.game.decision {
            Some(Decision::DrawLook {
                player: 0,
                choice_id,
                ..
            }) => Some(*choice_id),
            _ => None,
        };
        if self.draw_look_draft.as_ref().map(|draft| draft.choice_id) != next {
            self.draw_look_draft = next.map(DrawLookDraft::new);
            self.selection = None;
            self.hover_card = None;
            self.hover_object = None;
            self.hover_rect = None;
            self.hand_scroll = 0;
        }
    }

    pub(super) fn draw_look_panel(&mut self) {
        let Some(Decision::DrawLook {
            player: 0,
            source_card_id,
            cards,
            eligible,
            kind,
            ..
        }) = self.game.decision.clone()
        else {
            return;
        };
        let Some(draft) = self.draw_look_draft.clone() else {
            return;
        };
        self.ui
            .title("Choose a card to draw", 1178.0, 366.0, 19.0, TEXT);
        self.ui.ellipsis(
            self.game.catalog[source_card_id].name,
            1178.0,
            390.0,
            212.0,
            12.0,
            GOLD,
        );
        self.ui
            .wrapped_lines(eligibility_hint(kind), 1178.0, 410.0, 209.0, 11.0, TEXT, 4);
        self.ui.wrapped_lines(
            "Other looked cards recycle in random order. No play cost is paid to draw.",
            1178.0,
            463.0,
            209.0,
            10.0,
            GOLD,
            2,
        );
        let action = draft.action();
        let valid = action
            .as_ref()
            .is_some_and(|action| self.game.clone().apply(action.clone()).is_ok());
        if self.ui.button(
            Rect::new(1178.0, 493.0, 212.0, 34.0),
            if draft.selection == Some(None) {
                "Confirm · recycle all"
            } else {
                "Confirm chosen card"
            },
            true,
            valid,
        ) {
            self.action(action.unwrap());
        }
        if self.ui.button(
            Rect::new(1178.0, 534.0, 212.0, 27.0),
            "Recycle all",
            draft.selection == Some(None),
            true,
        ) {
            self.draw_look_draft.as_mut().unwrap().selection = Some(None);
        }
        if self.ui.button(
            Rect::new(1178.0, 566.0, 212.0, 25.0),
            "Clear choice · unchanged",
            false,
            draft.selection.is_some(),
        ) {
            self.draw_look_draft.as_mut().unwrap().selection = None;
        }
        self.ui.text(
            &format!(
                "{} looked · {} eligible · private",
                cards.len(),
                eligible.len()
            ),
            1178.0,
            610.0,
            10.0,
            MUTED,
        );
    }

    pub(super) fn revealed_card_panel(&mut self) {
        let Some(Decision::RevealedCard {
            player,
            source_card_id,
            card_id,
            outcome,
        }) = self.game.decision
        else {
            return;
        };
        self.ui
            .title("A card was revealed", 1178.0, 366.0, 19.0, TEXT);
        self.ui.ellipsis(
            self.game.catalog[source_card_id].name,
            1178.0,
            390.0,
            212.0,
            12.0,
            GOLD,
        );
        self.ui.wrapped_lines(
            &reveal_outcome_text(player, outcome),
            1178.0,
            414.0,
            209.0,
            11.0,
            TEXT,
            5,
        );
        if self.ui.button(
            Rect::new(1178.0, 493.0, 212.0, 34.0),
            "Continue",
            true,
            true,
        ) {
            self.action(Action::Choose { accept: true });
        }
        if self.ui.button(
            Rect::new(1178.0, 534.0, 212.0, 27.0),
            "Inspect revealed card",
            false,
            true,
        ) {
            self.inspect_printed_card(card_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_draw_occurrences_are_staged_and_clearable() {
        let mut draft = DrawLookDraft::new(52);
        assert!(draft.action().is_none());
        draft.select(2, &[0, 2]);
        assert_eq!(
            draft.action(),
            Some(Action::ResolveDrawLook {
                choice_id: 52,
                index: Some(2)
            })
        );
        draft.select(1, &[0, 2]);
        assert_eq!(draft.selection, Some(Some(2)));
        draft.selection = None;
        assert!(draft.action().is_none());
        draft.selection = Some(None);
        assert_eq!(
            draft.action(),
            Some(Action::ResolveDrawLook {
                choice_id: 52,
                index: None
            })
        );
        assert_eq!(DrawLookDraft::new(53).selection, None);
    }
    #[test]
    fn outcome_copy_describes_completed_movement_without_another_draw() {
        assert!(reveal_outcome_text(0, RevealOutcome::Draw).contains("does not draw again"));
        assert!(reveal_outcome_text(1, RevealOutcome::Recycle).contains("opponent's deck"));
        assert!(reveal_outcome_text(0, RevealOutcome::Trash).contains("your trash"));
        assert!(eligibility_hint(DrawLookKind::TrifarianCaptain).contains("AND"));
    }
}
