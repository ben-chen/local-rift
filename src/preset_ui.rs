//! Inspect a sourced main deck, sideboard, and registered battlefield choices.
use super::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum PresetTab {
    #[default]
    Main,
    Sideboard,
    Battlefields,
}

impl App {
    pub(super) fn preset_list_len(&self) -> usize {
        let p = &self.game.presets[self.browse_deck];
        match (self.preset_tab, p.registration) {
            (PresetTab::Sideboard, Some(r)) => r.sideboard.len(),
            (PresetTab::Battlefields, Some(_)) => 0,
            _ => p.cards.len(),
        }
    }

    pub(super) fn draw_preset_contents(&mut self) {
        let p = self.game.presets[self.browse_deck].clone();
        if let Some(registration) = p.registration {
            let side_count: usize = registration
                .sideboard
                .iter()
                .map(|(_, count)| usize::from(*count))
                .sum();
            for (i, (tab, label)) in [
                (PresetTab::Main, "Main deck · 40".to_owned()),
                (PresetTab::Sideboard, format!("Sideboard · {side_count}")),
                (
                    PresetTab::Battlefields,
                    format!("Battlefields · {}", registration.battlefields.len()),
                ),
            ]
            .into_iter()
            .enumerate()
            {
                if self.ui.button(
                    Rect::new(634.0 + i as f32 * 182.0, 426.0, 172.0, 28.0),
                    &label,
                    self.preset_tab == tab,
                    true,
                ) {
                    self.preset_tab = tab;
                    self.deck_scroll = 0;
                }
            }
            if self.preset_tab == PresetTab::Battlefields {
                for (row, &field_id) in registration.battlefields.iter().enumerate() {
                    let field = local_rift::battlefields::battlefield(field_id);
                    let y = 463.0 + row as f32 * 117.0;
                    self.ui.rect(Rect::new(634.0, y, 770.0, 106.0), PANEL);
                    self.ui.title(field.name, 650.0, y + 25.0, 19.0, TEXT);
                    self.ui.wrapped_lines(
                        field.official_text,
                        650.0,
                        y + 48.0,
                        572.0,
                        11.0,
                        MUTED,
                        3,
                    );
                    if self.ui.button(
                        Rect::new(1248.0, y + 10.0, 140.0, 24.0),
                        "Full rules",
                        false,
                        true,
                    ) {
                        self.field_inspect = Some(field_id);
                    }
                    for owner in 0..2 {
                        let selected_deck = if owner == 0 {
                            self.player_deck
                        } else {
                            self.opponent_deck
                        };
                        let available = !self.in_match
                            && !self.prefs.use_custom[owner]
                            && selected_deck == self.browse_deck;
                        if self.ui.button(
                            Rect::new(1248.0, y + 40.0 + owner as f32 * 28.0, 140.0, 24.0),
                            if owner == 0 {
                                "Your battlefield"
                            } else {
                                "AI battlefield"
                            },
                            available && self.fields[owner] == field_id,
                            available,
                        ) {
                            self.fields[owner] = field_id;
                        }
                    }
                }
                self.ui.text(
                    "Select this deck for a seat to choose its battlefield above.",
                    634.0,
                    845.0,
                    11.0,
                    MUTED,
                );
                self.preset_source_footer();
                return;
            }
        } else {
            self.preset_tab = PresetTab::Main;
            self.ui.text("THE LIST", 634.0, 444.0, 10.0, GOLD);
            self.ui.text("40 CARDS", 1314.0, 444.0, 10.0, MUTED);
        }
        let list = if self.preset_tab == PresetTab::Sideboard {
            p.registration.unwrap().sideboard.to_vec()
        } else {
            p.cards.clone()
        };
        self.deck_scroll = self.deck_scroll.min(list.len().saturating_sub(14));
        for (j, &(id, count)) in list.iter().skip(self.deck_scroll).take(14).enumerate() {
            let x = 634.0 + (j / 7) as f32 * 396.0;
            let y = 463.0 + (j % 7) as f32 * 51.0;
            let card = self.game.catalog[id];
            let r = Rect::new(x, y, 377.0, 43.0);
            self.ui.rect(
                r,
                if self.ui.hover(r) {
                    Color::new(0.09, 0.14, 0.16, 1.0)
                } else {
                    PANEL
                },
            );
            self.ui
                .rect(Rect::new(x, y, 3.0, 43.0), rgb(card.domain.color()));
            self.ui
                .bold(&card.cost.to_string(), x + 14.0, y + 27.0, 15.0, TEAL);
            self.ui
                .ellipsis(card.name, x + 42.0, y + 19.0, 290.0, 11.0, TEXT);
            self.ui.text(
                if self.preset_tab == PresetTab::Main && id == p.chosen_champion {
                    "Chosen champion · included in the 40"
                } else {
                    card.kind.name()
                },
                x + 42.0,
                y + 34.0,
                9.0,
                MUTED,
            );
            self.ui
                .text(&format!("×{count}"), x + 345.0, y + 27.0, 12.0, GOLD);
            if self.ui.hover(r) {
                self.hover_candidate = Some(id);
            }
            if self.ui.clicked(r) {
                self.inspect_printed_card(id);
            }
        }
        let max = list.len().saturating_sub(14);
        self.ui.text(
            &format!(
                "{}–{} of {} unique cards · click to inspect",
                self.deck_scroll + 1,
                (self.deck_scroll + 14).min(list.len()),
                list.len()
            ),
            634.0,
            845.0,
            10.0,
            MUTED,
        );
        if self.ui.button(
            Rect::new(1310.0, 822.0, 42.0, 31.0),
            "Up",
            false,
            self.deck_scroll > 0,
        ) {
            self.deck_scroll = self.deck_scroll.saturating_sub(7);
        }
        if self.ui.button(
            Rect::new(1362.0, 822.0, 42.0, 31.0),
            "Dn",
            false,
            self.deck_scroll < max,
        ) {
            self.deck_scroll = (self.deck_scroll + 7).min(max);
        }
        self.preset_source_footer();
    }

    fn preset_source_footer(&mut self) {
        let note = if self.preset_tab == PresetTab::Sideboard {
            "Sideboard reference · customize a copy to practice swaps. Between-game sideboarding is not available yet."
        } else {
            "Sourced tournament lists and official card text. Hover to read a card; click to pin its full details."
        };
        self.ui.text(note, 245.0, 883.0, 10.0, MUTED);
    }
}
