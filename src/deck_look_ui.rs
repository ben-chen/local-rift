//! Owner-private deck choices retain physical indices until explicit confirmation.
use super::*;
use local_rift::game::InstructedPlayKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DeckLookDraft {
    pub choice_id: u32,
    /// None is unselected; Some(None) explicitly chooses to recycle everything.
    pub selection: Option<Option<usize>>,
}
impl DeckLookDraft {
    fn new(choice_id: u32) -> Self {
        Self {
            choice_id,
            selection: None,
        }
    }
    fn select(&mut self, index: usize, eligible: &[usize]) {
        if eligible.contains(&index) {
            self.selection = Some(Some(index));
        }
    }
    fn action(&self) -> Option<Action> {
        self.selection.map(|index| Action::ResolveDeckLook {
            choice_id: self.choice_id,
            index,
        })
    }
}

pub(super) fn pending_cost_hint(kind: InstructedPlayKind) -> Option<&'static str> {
    match kind {
        InstructedPlayKind::BaitedHook | InstructedPlayKind::DazzlingAurora => {
            Some("Base cost waived · extra costs still apply")
        }
        InstructedPlayKind::Reinforce | InstructedPlayKind::WildClaw => {
            Some("Total Energy reduced by 5 · Power still applies")
        }
        _ => None,
    }
}

fn eligibility_hint(kind: InstructedPlayKind, killed_might: Option<u64>) -> String {
    match kind {
        InstructedPlayKind::BaitedHook => killed_might.map_or_else(
            || "No unit was killed: you cannot choose a unit. Recycle all looked cards.".into(),
            |might| format!("Choose a unit with printed Might at most {}. The killed unit had {might} Might.", u128::from(might) + 1)),
        InstructedPlayKind::Reinforce => "Choose a unit. Spells and gear cannot be chosen.".into(),
        InstructedPlayKind::WildClaw => "Choose a unit or gear. Spells cannot be chosen.".into(),
        _ => "Choose an eligible highlighted card.".into(),
    }
}

fn ineligible_hint(
    kind: InstructedPlayKind,
    card: &local_rift::cards::CardDef,
    killed_might: Option<u64>,
) -> &'static str {
    match kind {
        InstructedPlayKind::BaitedHook if killed_might.is_none() => "No killed-unit reference",
        InstructedPlayKind::BaitedHook | InstructedPlayKind::Reinforce
            if card.kind != CardKind::Unit =>
        {
            "Requires a unit"
        }
        InstructedPlayKind::BaitedHook => "Above Hook's Might limit",
        InstructedPlayKind::WildClaw if card.kind == CardKind::Spell => "Requires unit or gear",
        _ => "Not eligible for this play",
    }
}

fn fixed_target(game: &Game, target: Option<Target>) -> Option<(String, usize, usize, u32)> {
    match target? {
        Target::Unit { player, unit_id } => game.players[player]
            .units
            .iter()
            .find(|unit| unit.id == unit_id)
            .map(|unit| (game.unit_name(unit), unit.card_id, player, unit_id)),
        Target::Gear { player, gear_id } => game.players[player]
            .gear
            .iter()
            .find(|gear| gear.id == gear_id)
            .map(|gear| {
                (
                    game.gear_name(gear).to_owned(),
                    gear.card_id,
                    player,
                    gear_id,
                )
            }),
        _ => None,
    }
}

impl App {
    pub(super) fn sync_deck_look_draft(&mut self) {
        let next = match &self.game.decision {
            Some(Decision::DeckLookPlay {
                player: 0,
                choice_id,
                ..
            }) => Some(*choice_id),
            _ => None,
        };
        if self.deck_look_draft.as_ref().map(|draft| draft.choice_id) != next {
            self.deck_look_draft = next.map(DeckLookDraft::new);
            self.selection = None;
            self.hover_card = None;
            self.hover_object = None;
            self.hover_rect = None;
            self.hand_scroll = 0;
        }
    }

    pub(super) fn deck_look_panel(&mut self) {
        let Some(Decision::DeckLookPlay {
            player: 0,
            source_card_id,
            cards,
            eligible,
            kind,
            killed_might,
            ..
        }) = self.game.decision.clone()
        else {
            return;
        };
        let Some(draft) = self.deck_look_draft.clone() else {
            return;
        };
        self.ui
            .title("Choose from your deck", 1178.0, 366.0, 20.0, TEXT);
        self.ui.ellipsis(
            self.game.catalog[source_card_id].name,
            1178.0,
            390.0,
            212.0,
            12.0,
            GOLD,
        );
        self.ui.wrapped_lines(
            &eligibility_hint(kind, killed_might),
            1178.0,
            410.0,
            209.0,
            11.0,
            TEXT,
            4,
        );
        self.ui.wrapped_lines(
            pending_cost_hint(kind).unwrap_or("Review the chosen card's costs next."),
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
            self.deck_look_draft.as_mut().unwrap().selection = Some(None);
        }
        if self.ui.button(
            Rect::new(1178.0, 566.0, 212.0, 25.0),
            "Clear choice · unchanged",
            false,
            draft.selection.is_some(),
        ) {
            self.deck_look_draft.as_mut().unwrap().selection = None;
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

    pub(super) fn revealed_deck_panel(&mut self) {
        let Some(Decision::RevealedDeck {
            source_card_id,
            ref cards,
            ..
        }) = self.game.decision
        else {
            return;
        };
        self.ui
            .title("Revealed from the deck", 1178.0, 366.0, 19.0, TEXT);
        self.ui.ellipsis(
            self.game.catalog[source_card_id].name,
            1178.0,
            392.0,
            212.0,
            12.0,
            GOLD,
        );
        self.ui.wrapped_lines(if cards.iter().any(|&id| self.game.catalog[id].kind == CardKind::Unit) { "These cards were publicly revealed. Other revealed cards have been recycled. Continue to finish this instruction and any pending unit play." } else { "These cards were publicly revealed and recycled. No unit was found to play." }, 1178.0, 416.0, 209.0, 11.5, TEXT, 5);
        if self.ui.button(
            Rect::new(1178.0, 493.0, 212.0, 34.0),
            "Continue",
            true,
            true,
        ) {
            self.action(Action::Choose { accept: true });
        }
    }

    pub(super) fn empower_played_panel(&mut self) {
        let Some(Decision::EmpowerPlayed {
            player: 0,
            source_card_id,
            target,
            ..
        }) = self.game.decision
        else {
            return;
        };
        let found = fixed_target(&self.game, target);
        self.ui
            .title("Empower this card?", 1178.0, 366.0, 18.5, TEXT);
        self.ui.ellipsis(
            self.game.catalog[source_card_id].name,
            1178.0,
            392.0,
            212.0,
            12.0,
            GOLD,
        );
        self.ui.wrapped_lines(&found.as_ref().map_or_else(
            || "The played object is no longer available. There is nothing here to Empower.".into(),
            |(name, ..)| format!("Empower {name}? Accepting puts an ability on the chain; opponents may respond. This applies only to this played object.")),
            1178.0, 416.0, 209.0, 11.0, TEXT, 5);
        if self.ui.button(
            Rect::new(1178.0, 493.0, 212.0, 34.0),
            "Yes · use Empower ability",
            true,
            found.is_some(),
        ) {
            self.action(Action::Choose { accept: true });
        }
        if self.ui.button(
            Rect::new(1178.0, 534.0, 212.0, 27.0),
            if found.is_some() {
                "No · decline"
            } else {
                "Continue"
            },
            false,
            true,
        ) {
            self.action(Action::Choose { accept: false });
        }
        if let Some((_, card_id, owner, object_id)) = found
            && self.ui.button(
                Rect::new(1178.0, 566.0, 212.0, 25.0),
                "Inspect the played card",
                false,
                true,
            )
        {
            self.inspect = Some(card_id);
            self.hover_object = Some((owner, object_id));
        }
    }

    /// Public reveals use the same strip, but never offer a selection.
    pub(super) fn deck_look_tray(&mut self) -> bool {
        let Some(decision) = self.game.decision.clone() else {
            return false;
        };
        let (cards, private) = match &decision {
            Decision::DeckLookPlay {
                player: 0, cards, ..
            } => (cards.clone(), true),
            Decision::RevealedDeck { cards, .. } => (cards.clone(), false),
            _ => return false,
        };
        self.ui.text(
            if private {
                "PRIVATE · TOP CARD FIRST · CONFIRM AFTER CHOOSING"
            } else {
                "PUBLIC REVEAL · READ ONLY · TOP CARD FIRST"
            },
            245.0,
            671.0,
            10.0,
            GOLD,
        );
        self.hand_scroll = self.hand_scroll.min(cards.len().saturating_sub(5));
        for (slot, (index, &card_id)) in cards
            .iter()
            .enumerate()
            .skip(self.hand_scroll)
            .take(5)
            .enumerate()
        {
            let x = 245.0 + slot as f32 * 176.0;
            let rect = Rect::new(x, 697.0, 160.0, 179.0);
            let card = self.game.catalog[card_id];
            let (eligible, status) = if let Decision::DeckLookPlay {
                eligible,
                kind,
                killed_might,
                ..
            } = &decision
            {
                let allowed = eligible.contains(&index);
                (
                    allowed,
                    if allowed {
                        "Eligible · choose this copy"
                    } else {
                        ineligible_hint(*kind, &card, *killed_might)
                    },
                )
            } else {
                (true, "Revealed · inspect rules")
            };
            let selected = private
                && self
                    .deck_look_draft
                    .as_ref()
                    .is_some_and(|draft| draft.selection == Some(Some(index)));
            self.ui.rect(rect, PANEL);
            if let Some(texture) = &self.art[card_id] {
                let bounds = Rect::new(
                    texture.width() * 0.08,
                    texture.height() * 0.12,
                    texture.width() * 0.84,
                    texture.height() * 0.43,
                );
                let scale = (bounds.w / 156.0).min(bounds.h / 94.0);
                let crop = Rect::new(
                    bounds.x + (bounds.w - 156.0 * scale) / 2.0,
                    bounds.y + (bounds.h - 94.0 * scale) / 2.0,
                    156.0 * scale,
                    94.0 * scale,
                );
                self.ui.image(
                    texture,
                    Rect::new(x + 2.0, 699.0, 156.0, 94.0),
                    Some(crop),
                    if eligible {
                        WHITE
                    } else {
                        Color::new(0.48, 0.48, 0.48, 1.0)
                    },
                );
            }
            self.ui
                .pill(x + 7.0, 704.0, &format!("{}", index + 1), GOLD);
            self.ui
                .ellipsis(card.name, x + 8.0, 813.0, 144.0, 11.0, TEXT);
            let stats = if card.kind == CardKind::Unit {
                format!("{} · {} printed Might", card.kind.name(), card.might)
            } else {
                card.kind.name().into()
            };
            self.ui.ellipsis(&stats, x + 8.0, 830.0, 144.0, 9.5, MUTED);
            self.ui.text(
                &format!("{} Energy · {} Power", card.cost, card.power_cost),
                x + 8.0,
                847.0,
                9.5,
                MUTED,
            );
            self.ui.ellipsis(
                if selected {
                    "CHOSEN · confirm on right"
                } else {
                    status
                },
                x + 8.0,
                866.0,
                144.0,
                9.0,
                if selected {
                    GOLD
                } else if eligible {
                    TEAL
                } else {
                    MUTED
                },
            );
            self.ui.border(
                rect,
                if selected {
                    GOLD
                } else if eligible {
                    TEAL
                } else {
                    LINE
                },
            );
            if private
                && self.ui.clicked(rect)
                && let Decision::DeckLookPlay { eligible, .. } = &decision
            {
                self.deck_look_draft
                    .as_mut()
                    .unwrap()
                    .select(index, eligible);
            }
            if self.ui.hover(rect) {
                self.hover_candidate = Some(card_id);
            }
            if self.ui.hover(rect) && self.ui.secondary_pressed() {
                self.inspect = Some(card_id);
            }
        }
        for (x, label, enabled, forward) in [
            (1047.0, "<", self.hand_scroll > 0, false),
            (1087.0, ">", self.hand_scroll + 5 < cards.len(), true),
        ] {
            if self
                .ui
                .button(Rect::new(x, 648.0, 34.0, 28.0), label, false, enabled)
            {
                if forward {
                    self.hand_scroll = (self.hand_scroll + 5).min(cards.len().saturating_sub(5));
                } else {
                    self.hand_scroll = self.hand_scroll.saturating_sub(5);
                }
            }
        }
        if cards.is_empty() {
            self.ui
                .title("No cards to inspect", 270.0, 753.0, 22.0, TEXT);
            self.ui.wrapped("Looking does not draw cards or cause burnout. Continue with the available choice on the right.",270.0,785.0,650.0,13.0,MUTED);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_occurrences_and_decline_remain_local_until_confirmation() {
        let mut draft = DeckLookDraft::new(17);
        assert!(draft.action().is_none());
        draft.select(3, &[1, 3]);
        assert_eq!(
            draft.action(),
            Some(Action::ResolveDeckLook {
                choice_id: 17,
                index: Some(3)
            })
        );
        draft.select(0, &[1, 3]);
        assert_eq!(draft.selection, Some(Some(3)));
        draft.select(1, &[1, 3]);
        assert_eq!(draft.selection, Some(Some(1)));
        draft.selection = Some(None);
        assert_eq!(
            draft.action(),
            Some(Action::ResolveDeckLook {
                choice_id: 17,
                index: None
            })
        );
        assert_eq!(DeckLookDraft::new(18).selection, None);
    }
    #[test]
    fn hook_limit_keeps_the_full_integer_range() {
        let text = eligibility_hint(InstructedPlayKind::BaitedHook, Some(u64::MAX));
        assert!(text.contains("18446744073709551616"));
        assert!(text.contains("18446744073709551615"));
        assert!(
            eligibility_hint(InstructedPlayKind::BaitedHook, None).contains("No unit was killed")
        );
    }
}
