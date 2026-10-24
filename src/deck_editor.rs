//! Native deck editor. Tournament snapshots stay immutable; custom slots are
//! validated as a whole before their separate files or active matchup change.
use super::{App, Preferences, Screen};
use crate::ui::GOLD;
use crate::ui::*;
use local_rift::{
    battlefields::{battlefield, default_battlefield},
    deck::{CustomDeck, eligible_cards, eligible_champions},
    game::domain_index,
};
use macroquad::prelude::*;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum DeckFilter {
    All,
    Included,
    Additional,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum InputField {
    Name,
    Search,
}
#[derive(Clone)]
pub(super) struct DeckEditor {
    pub deck: CustomDeck,
    pub query: String,
    pub filter: DeckFilter,
    pub scroll: usize,
    input: Option<InputField>,
    replace_input: bool,
}
impl DeckEditor {
    fn new(deck: CustomDeck) -> Self {
        Self {
            deck,
            query: String::new(),
            filter: DeckFilter::All,
            scroll: 0,
            input: None,
            replace_input: false,
        }
    }
    pub fn count(&self, card_id: usize) -> u8 {
        self.deck
            .cards
            .iter()
            .find(|(id, _)| *id == card_id)
            .map_or(0, |(_, n)| *n)
    }
    fn change_count(&mut self, card_id: usize, delta: i8) {
        let count = self.count(card_id).saturating_add_signed(delta).min(3);
        self.deck.cards.retain(|(id, _)| *id != card_id);
        if count > 0 {
            self.deck.cards.push((card_id, count));
        }
        self.deck.cards.sort_unstable_by_key(|(id, _)| *id);
    }
    pub fn visible_cards(&self) -> Vec<usize> {
        let catalog = local_rift::cards::cards();
        let presets = local_rift::cards::presets();
        let query = self.query.to_lowercase();
        let mut ids: Vec<_> = eligible_cards(self.deck.legend)
            .into_iter()
            .filter(|&id| {
                let card = &catalog[id];
                let in_filter = match self.filter {
                    DeckFilter::All => true,
                    DeckFilter::Included => self.count(id) > 0,
                    DeckFilter::Additional => !presets
                        .iter()
                        .any(|p| p.cards.iter().any(|(c, _)| *c == id)),
                };
                in_filter
                    && (query.is_empty()
                        || card.name.to_lowercase().contains(&query)
                        || card.official_text.to_lowercase().contains(&query))
            })
            .collect();
        ids.sort_by(|&a, &b| {
            catalog[a]
                .cost
                .cmp(&catalog[b].cost)
                .then(catalog[a].name.cmp(catalog[b].name))
        });
        ids
    }
}

fn editing_label(ui: &Ui, text: &str, width: f32, editing: bool) -> String {
    if !editing {
        return text.to_owned();
    }
    let mut label = format!("{text}|");
    while ui.width(&label, 12.0) > width && label.chars().count() > 1 {
        label.remove(0);
    }
    label
}

pub(super) fn saved_path(owner: usize) -> PathBuf {
    Preferences::path().with_file_name(if owner == 0 {
        "custom-you.json"
    } else {
        "custom-ai.json"
    })
}
pub(super) fn load_saved_deck(path: &std::path::Path) -> Result<Option<CustomDeck>, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{}: invalid JSON ({error})", path.display()))?;
    CustomDeck::from_json(&value)
        .map(Some)
        .map_err(|error| format!("{}: {error}", path.display()))
}
pub(super) fn load_saved_decks_with_errors() -> ([Option<CustomDeck>; 2], Vec<String>) {
    let mut errors = Vec::new();
    let decks = std::array::from_fn(|owner| match load_saved_deck(&saved_path(owner)) {
        Ok(deck) => deck,
        Err(error) => {
            errors.push(error);
            None
        }
    });
    (decks, errors)
}
pub(super) fn load_saved_decks() -> [Option<CustomDeck>; 2] {
    load_saved_decks_with_errors().0
}
fn write_saved_deck(owner: usize, deck: &CustomDeck) -> Result<(), String> {
    deck.validate()?;
    let path = saved_path(owner);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create your deck folder: {e}"))?;
    }
    let bytes = serde_json::to_vec_pretty(&deck.to_json()).map_err(|e| e.to_string())?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes).map_err(|e| format!("Could not save your deck: {e}"))?;
    std::fs::rename(&temporary, &path)
        .map_err(|e| format!("Could not finish saving your deck: {e}"))
}

impl App {
    pub(super) fn active_custom_decks(&self) -> [Option<CustomDeck>; 2] {
        std::array::from_fn(|owner| {
            self.prefs.use_custom[owner]
                .then(|| self.custom_decks[owner].clone())
                .flatten()
        })
    }
    pub(super) fn select_preset(&mut self, owner: usize, index: usize) {
        if owner == 0 {
            self.player_deck = index;
        } else {
            self.opponent_deck = index;
        }
        self.fields[owner] = default_battlefield(index);
        self.prefs.use_custom[owner] = false;
        self.prefs.save();
    }
    pub(super) fn open_deck_editor(&mut self, saved_owner: Option<usize>) {
        let result = if let Some(owner) = saved_owner {
            self.custom_decks[owner]
                .clone()
                .ok_or_else(|| "No saved custom deck in this slot yet.".to_owned())
        } else {
            CustomDeck::from_preset(self.browse_deck)
        };
        match result {
            Ok(deck) => {
                self.deck_editor = Some(DeckEditor::new(deck));
                self.screen = Screen::DeckEditor;
                self.hover_card = None;
                self.hover_rect = None;
                self.inspect = None;
            }
            Err(error) => self.notify(error),
        }
    }
    pub(super) fn save_editor_deck(
        &mut self,
        owner: usize,
        deck: &CustomDeck,
    ) -> Result<(), String> {
        if self.in_match {
            return Err("Return to matchup setup before changing a saved deck.".into());
        }
        write_saved_deck(owner, deck)?;
        self.custom_decks[owner] = Some(deck.clone());
        self.prefs.use_custom[owner] = true;
        self.fields[owner] = deck.battlefield;
        self.prefs.save();
        self.notify(format!(
            "Saved {} for {}. It is selected for the next match.",
            deck.name,
            if owner == 0 { "you" } else { "the AI" }
        ));
        Ok(())
    }
    pub(super) fn update_deck_editor(&mut self) {
        let Some(editor) = self.deck_editor.as_mut() else {
            self.screen = Screen::Decks;
            return;
        };
        if is_key_pressed(KeyCode::Escape) {
            if editor.input.take().is_none() {
                self.screen = Screen::Decks;
            }
            return;
        }
        if let Some(field) = editor.input {
            let value = match field {
                InputField::Name => &mut editor.deck.name,
                InputField::Search => &mut editor.query,
            };
            let selecting_all = (is_key_down(KeyCode::LeftControl)
                || is_key_down(KeyCode::RightControl))
                && is_key_pressed(KeyCode::A);
            if selecting_all {
                editor.replace_input = true;
            }
            while let Some(character) = get_char_pressed() {
                if !selecting_all
                    && !character.is_control()
                    && (editor.replace_input || value.chars().count() < 100)
                {
                    if editor.replace_input {
                        value.clear();
                        editor.replace_input = false;
                    }
                    value.push(character);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                if editor.replace_input {
                    value.clear();
                    editor.replace_input = false;
                } else {
                    value.pop();
                }
            }
            if is_key_pressed(KeyCode::Enter) {
                editor.input = None;
            }
            if field == InputField::Search {
                editor.scroll = 0;
            }
        }
        if self.hover_card.is_none()
            && Rect::new(565.0, 180.0, 845.0, 620.0).contains(self.ui.mouse())
        {
            let wheel = self.ui.wheel();
            if wheel < 0.0 {
                editor.scroll =
                    (editor.scroll + 1).min(editor.visible_cards().len().saturating_sub(9));
            } else if wheel > 0.0 {
                editor.scroll = editor.scroll.saturating_sub(1);
            }
        }
    }
    pub(super) fn draw_deck_editor(&mut self) {
        let Some(mut editor) = self.deck_editor.take() else {
            self.screen = Screen::Decks;
            return;
        };
        self.header("Build your next idea.", "A separate custom list, using supported cards legal for your legend. Presets keep their original lists.");
        self.ui.rect(Rect::new(245.0, 116.0, 300.0, 731.0), PANEL);
        self.ui.rect(Rect::new(565.0, 116.0, 845.0, 731.0), PANEL);
        let legend =
            local_rift::legend_catalog::legend(editor.deck.legend).expect("validated legend");
        self.ui.text("YOUR LEGEND", 265.0, 142.0, 10.0, GOLD);
        self.ui.ellipsis(
            &super::legend_label(legend),
            265.0,
            164.0,
            218.0,
            13.0,
            TEXT,
        );
        if self
            .ui
            .button(Rect::new(492.0, 145.0, 33.0, 28.0), ">", false, true)
        {
            self.legend_picker = Some(legend.id);
            editor.input = None;
        }
        if self.ui.hover(Rect::new(265.0, 145.0, 218.0, 30.0)) {
            self.legend_hover = Some(legend.id);
            if self.ui.secondary_pressed() {
                self.lesson = Some(legend.id);
            }
        }
        self.ui.text("DECK NAME", 265.0, 194.0, 9.0, MUTED);
        let name_rect = Rect::new(265.0, 204.0, 260.0, 36.0);
        self.ui.rect(name_rect, BG);
        self.ui.border(
            name_rect,
            if editor.input == Some(InputField::Name) {
                TEAL
            } else {
                LINE
            },
        );
        let name_editing = editor.input == Some(InputField::Name);
        if name_editing && editor.replace_input {
            self.ui.rect(
                Rect::new(272.0, 209.0, 222.0, 25.0),
                Color::new(0.12, 0.27, 0.27, 1.0),
            );
        }
        let name_label = editing_label(
            &self.ui,
            &editor.deck.name,
            218.0,
            name_editing && !editor.replace_input,
        );
        self.ui
            .ellipsis(&name_label, 275.0, 227.0, 224.0, 12.0, TEXT);
        self.ui.text("edit", 499.0, 226.0, 8.0, MUTED);
        if self.ui.clicked(name_rect) {
            while get_char_pressed().is_some() {}
            editor.input = Some(InputField::Name);
            editor.replace_input = true;
        }
        let validation = editor.deck.validate();
        self.ui.bold(
            &format!("{} cards", editor.deck.card_count()),
            265.0,
            277.0,
            23.0,
            if validation.is_ok() { TEAL } else { GOLD },
        );
        self.ui.text(
            "40 minimum · chosen champion included",
            265.0,
            299.0,
            10.0,
            MUTED,
        );
        let status = validation.as_ref().map_or_else(
            |error| error.as_str(),
            |_| "Ready to save. Up to 3 copies per card; 3 Signature cards total.",
        );
        self.ui.wrapped(
            status,
            265.0,
            321.0,
            260.0,
            11.0,
            if validation.is_ok() { TEAL } else { GOLD },
        );
        self.ui.text("CHOSEN CHAMPION", 265.0, 390.0, 10.0, GOLD);
        let champion = self.game.catalog[editor.deck.chosen_champion];
        let champion_rect = Rect::new(265.0, 402.0, 260.0, 42.0);
        self.ui.rect(champion_rect, BG);
        self.ui
            .ellipsis(champion.name, 275.0, 427.0, 211.0, 12.0, TEXT);
        let champions = eligible_champions(editor.deck.legend);
        if self.ui.button(
            Rect::new(491.0, 407.0, 29.0, 30.0),
            ">",
            false,
            champions.len() > 1,
        ) {
            let current = champions
                .iter()
                .position(|&id| id == editor.deck.chosen_champion)
                .unwrap_or(0);
            let id = champions[(current + 1) % champions.len()];
            editor.deck.chosen_champion = id;
            if editor.count(id) == 0 {
                editor.change_count(id, 1);
            }
        }
        if self.ui.hover(Rect::new(265.0, 402.0, 220.0, 42.0)) {
            self.hover_candidate = Some(champion.id);
            if self.ui.secondary_pressed() {
                self.inspect = Some(champion.id);
            }
        }
        let rune_count: usize = editor.deck.runes.iter().map(|&n| usize::from(n)).sum();
        self.ui.text(
            &format!("RUNE DECK · {rune_count} / 12"),
            265.0,
            475.0,
            10.0,
            if rune_count == 12 { GOLD } else { TEAL },
        );
        for (row, domain) in legend.domains.into_iter().enumerate() {
            let y = 487.0 + row as f32 * 45.0;
            let index = domain_index(domain);
            self.ui
                .circle(vec2(271.0, y + 18.0), 4.0, rgb(domain.color()));
            self.ui.text(domain.name(), 285.0, y + 22.0, 13.0, TEXT);
            if self.ui.button(
                Rect::new(401.0, y + 2.0, 31.0, 31.0),
                "-",
                false,
                editor.deck.runes[index] > 0,
            ) {
                editor.deck.runes[index] -= 1;
            }
            self.ui.bold(
                &editor.deck.runes[index].to_string(),
                451.0,
                y + 24.0,
                16.0,
                rgb(domain.color()),
            );
            if self.ui.button(
                Rect::new(490.0, y + 2.0, 31.0, 31.0),
                "+",
                false,
                editor.deck.runes[index] < 12,
            ) {
                editor.deck.runes[index] += 1;
            }
        }
        self.ui.text("YOUR BATTLEFIELD", 265.0, 602.0, 10.0, GOLD);
        if self.ui.button(
            Rect::new(265.0, 614.0, 260.0, 33.0),
            &format!("{}  >", battlefield(editor.deck.battlefield).name),
            false,
            true,
        ) {
            editor.input = None;
            self.open_field_picker(super::FieldPickerTarget::Editor, editor.deck.battlefield);
        }
        self.ui.wrapped_lines(
            battlefield(editor.deck.battlefield).official_text,
            265.0,
            669.0,
            260.0,
            10.5,
            MUTED,
            4,
        );
        self.ui.text(
            "Choose the field to read its complete rules.",
            265.0,
            737.0,
            9.0,
            TEAL,
        );
        // Validation is repeated on save; this status may precede an edit in this
        // frame, so a stale enabled button still cannot persist an invalid list.
        for owner in 0..2 {
            if self.ui.button(
                Rect::new(265.0 + owner as f32 * 134.0, 753.0, 126.0, 36.0),
                if owner == 0 {
                    "Save for You"
                } else {
                    "Save for AI"
                },
                owner == 0,
                validation.is_ok() && !self.in_match,
            ) && let Err(error) = self.save_editor_deck(owner, &editor.deck)
            {
                self.notify(error);
            }
        }
        if self.in_match {
            self.ui.text(
                "Return to setup to save a different deck.",
                265.0,
                812.0,
                10.0,
                GOLD,
            );
        }
        if self.ui.button(
            Rect::new(265.0, 802.0, 260.0, 29.0),
            "Back to library",
            false,
            true,
        ) {
            self.screen = Screen::Decks;
            editor.input = None;
        }

        let search_rect = Rect::new(585.0, 134.0, 371.0, 36.0);
        self.ui.rect(search_rect, BG);
        self.ui.border(
            search_rect,
            if editor.input == Some(InputField::Search) {
                TEAL
            } else {
                LINE
            },
        );
        let search_editing = editor.input == Some(InputField::Search);
        if search_editing && editor.replace_input {
            self.ui.rect(
                Rect::new(592.0, 139.0, 327.0, 25.0),
                Color::new(0.12, 0.27, 0.27, 1.0),
            );
        }
        let search_label = editing_label(
            &self.ui,
            &editor.query,
            318.0,
            search_editing && !editor.replace_input,
        );
        self.ui.ellipsis(
            if editor.query.is_empty() {
                "Search name or rules text…"
            } else {
                &search_label
            },
            596.0,
            157.0,
            318.0,
            12.0,
            if editor.query.is_empty() { MUTED } else { TEXT },
        );
        if self.ui.clicked(search_rect) {
            while get_char_pressed().is_some() {}
            editor.input = Some(InputField::Search);
            editor.replace_input = true;
        }
        if self.ui.button(
            Rect::new(925.0, 139.0, 25.0, 26.0),
            "×",
            false,
            !editor.query.is_empty(),
        ) {
            editor.query.clear();
            editor.scroll = 0;
        }
        for (index, (filter, label)) in [
            (DeckFilter::All, "All cards"),
            (DeckFilter::Included, "In deck"),
            (DeckFilter::Additional, "More cards"),
        ]
        .into_iter()
        .enumerate()
        {
            if self.ui.button(
                Rect::new(968.0 + index as f32 * 144.0, 134.0, 134.0, 36.0),
                label,
                editor.filter == filter,
                true,
            ) {
                editor.filter = filter;
                editor.scroll = 0;
                editor.input = None;
            }
        }
        let ids = editor.visible_cards();
        editor.scroll = editor.scroll.min(ids.len().saturating_sub(9));
        self.ui.text(
            "SUPPORTED CARDS IN YOUR LEGEND'S DOMAINS",
            585.0,
            194.0,
            10.0,
            GOLD,
        );
        self.ui.text("COPIES", 1296.0, 194.0, 10.0, MUTED);
        for (row, id) in ids.iter().skip(editor.scroll).take(9).copied().enumerate() {
            let card = self.game.catalog[id];
            let y = 208.0 + row as f32 * 63.0;
            let rect = Rect::new(585.0, y, 805.0, 57.0);
            self.ui.rect(
                rect,
                if editor.count(id) > 0 {
                    Color::new(0.075, 0.12, 0.13, 1.0)
                } else {
                    BG
                },
            );
            self.ui
                .rect(Rect::new(585.0, y, 3.0, 57.0), rgb(card.domain.color()));
            if let Some(texture) = self.art.get(id).and_then(Option::as_ref) {
                self.ui.image(
                    texture,
                    Rect::new(593.0, y + 4.0, 52.0, 49.0),
                    Some(Rect::new(0.0, 0.0, texture.width(), texture.height() * 0.6)),
                    WHITE,
                );
            }
            self.ui
                .ellipsis(card.name, 659.0, y + 23.0, 565.0, 14.0, TEXT);
            self.ui.ellipsis(
                &format!(
                    "{} · {} energy · {} power · {}",
                    card.kind.name(),
                    card.cost,
                    card.power_cost,
                    card.domains
                        .iter()
                        .map(|d| d.name())
                        .collect::<Vec<_>>()
                        .join(" / ")
                ),
                659.0,
                y + 43.0,
                565.0,
                10.5,
                MUTED,
            );
            let count = editor.count(id);
            if self.ui.button(
                Rect::new(1258.0, y + 13.0, 31.0, 31.0),
                "-",
                false,
                count > 0,
            ) {
                editor.change_count(id, -1);
            }
            self.ui.bold(
                &count.to_string(),
                1310.0,
                y + 35.0,
                17.0,
                if count > 0 { TEAL } else { MUTED },
            );
            if self.ui.button(
                Rect::new(1350.0, y + 13.0, 31.0, 31.0),
                "+",
                false,
                count < 3,
            ) {
                editor.change_count(id, 1);
            }
            let preview = Rect::new(585.0, y, 653.0, 57.0);
            if self.ui.hover(preview) {
                self.hover_candidate = Some(id);
                if self.ui.primary_pressed() || self.ui.secondary_pressed() {
                    self.inspect = Some(id);
                    editor.input = None;
                }
            }
        }
        if ids.is_empty() {
            self.ui.wrapped(
                "No supported cards match this filter. Try another search or view all cards.",
                608.0,
                254.0,
                650.0,
                14.0,
                MUTED,
            );
        }
        self.ui.text(
            &format!(
                "{}–{} of {} cards · hover or click to read · scroll to browse",
                if ids.is_empty() { 0 } else { editor.scroll + 1 },
                (editor.scroll + 9).min(ids.len()),
                ids.len()
            ),
            585.0,
            817.0,
            11.0,
            MUTED,
        );
        if self.ui.button(
            Rect::new(1286.0, 797.0, 45.0, 31.0),
            "Up",
            false,
            editor.scroll > 0,
        ) {
            editor.scroll = editor.scroll.saturating_sub(9);
        }
        if self.ui.button(
            Rect::new(1343.0, 797.0, 47.0, 31.0),
            "Dn",
            false,
            editor.scroll + 9 < ids.len(),
        ) {
            editor.scroll = (editor.scroll + 9).min(ids.len().saturating_sub(9));
        }
        self.ui.text("Custom decks are saved locally in separate You / AI slots. Only validated lists can start a match.", 245.0, 881.0, 11.0, MUTED);
        self.deck_editor = Some(editor);
    }
}

impl App {
    pub(super) fn draw_legend_picker(&mut self) {
        let Some(selected) = self.legend_picker else {
            return;
        };
        let definition =
            local_rift::legend_catalog::legend(selected).expect("registered selection");
        self.hover_card = None;
        self.hover_rect = None;
        self.ui.rect(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            Color::new(0.0, 0.015, 0.02, 0.9),
        );
        self.ui.rect(Rect::new(315.0, 135.0, 820.0, 630.0), PANEL);
        self.ui.border(Rect::new(315.0, 135.0, 820.0, 630.0), GOLD);
        self.ui.enabled = true;
        self.ui
            .title("Choose your legend.", 340.0, 180.0, 28.0, TEXT);
        self.ui.text(
            &format!(
                "{} supported legends · {} sourced tournament presets remain separate",
                local_rift::legend_catalog::legends().len(),
                self.game.presets.len()
            ),
            340.0,
            207.0,
            12.0,
            MUTED,
        );
        for (row, legend) in local_rift::legend_catalog::legends().iter().enumerate() {
            if self.ui.button(
                Rect::new(340.0, 233.0 + row as f32 * 53.0, 214.0, 43.0),
                legend.champion,
                selected == legend.id,
                true,
            ) {
                self.legend_picker = Some(legend.id);
            }
        }
        self.ui.ellipsis(
            &super::legend_label(definition),
            580.0,
            255.0,
            525.0,
            21.0,
            TEXT,
        );
        self.draw_legend_art(
            selected,
            Rect::new(580.0, 278.0, 147.0, 206.0),
            false,
            WHITE,
        );
        self.ui
            .wrapped(definition.text, 747.0, 298.0, 355.0, 13.0, TEXT);
        self.ui.text(
            &format!(
                "{} / {}",
                definition.domains[0].name(),
                definition.domains[1].name()
            ),
            580.0,
            510.0,
            13.0,
            rgb(definition.accent),
        );
        self.ui
            .text("Default chosen champion", 580.0, 540.0, 11.0, GOLD);
        self.ui.ellipsis(
            self.game.catalog[definition.default_champion].name,
            580.0,
            564.0,
            525.0,
            14.0,
            TEXT,
        );
        self.ui.wrapped("Start with an authored 40-card training list, then edit cards, champion and runes. These training lists are not tournament or meta recommendations.", 580.0, 600.0, 525.0, 12.0, MUTED);
        if self.ui.button(
            Rect::new(580.0, 679.0, 525.0, 42.0),
            "Create editable training list",
            true,
            true,
        ) {
            match CustomDeck::from_legend(selected) {
                Ok(deck) => {
                    self.deck_editor = Some(DeckEditor::new(deck));
                    self.screen = Screen::DeckEditor;
                    self.legend_picker = None;
                    self.toast_until = 0.0;
                }
                Err(error) => self.notify(error),
            }
        }
        if self.ui.button(
            Rect::new(340.0, 679.0, 214.0, 42.0),
            "Cancel · keep current list",
            false,
            true,
        ) {
            self.legend_picker = None;
        }
    }
    pub(super) fn saved_deck_notice(&mut self) {
        if self.deck_load_errors.is_empty() {
            return;
        }
        self.hover_card = None;
        self.hover_rect = None;
        self.ui.rect(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            Color::new(0.0, 0.015, 0.02, 0.9),
        );
        self.ui.rect(Rect::new(350.0, 220.0, 740.0, 430.0), PANEL);
        self.ui.border(Rect::new(350.0, 220.0, 740.0, 430.0), GOLD);
        self.ui
            .title("A saved deck needs attention.", 376.0, 267.0, 27.0, TEXT);
        self.ui.wrapped("The affected custom slot could not be loaded. Its file is unchanged; you can repair it before restarting, or continue with a preset.", 376.0, 304.0, 688.0, 13.0, MUTED);
        let mut y = 370.0;
        for error in &self.deck_load_errors {
            y = self.ui.wrapped(error, 376.0, y, 688.0, 11.0, GOLD) + 23.0;
        }
        self.ui.enabled = true;
        if self.ui.button(
            Rect::new(376.0, 589.0, 688.0, 38.0),
            "Continue with available decks",
            true,
            true,
        ) {
            self.deck_load_errors.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_loader_reports_invalid_files_without_replacing_them() {
        let path = std::env::temp_dir().join(format!(
            "local-rift-ui-deck-loader-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        assert!(load_saved_deck(&path).unwrap().is_none());
        for bytes in [b"{broken".to_vec(), br#"{"legend":999}"#.to_vec()] {
            std::fs::write(&path, &bytes).unwrap();
            assert!(
                load_saved_deck(&path)
                    .unwrap_err()
                    .contains(path.to_str().unwrap())
            );
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        }
        for legend in local_rift::legend_catalog::legends() {
            let deck = CustomDeck::from_legend(legend.id).unwrap();
            std::fs::write(&path, serde_json::to_vec(&deck.to_json()).unwrap()).unwrap();
            assert_eq!(load_saved_deck(&path).unwrap(), Some(deck));
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn every_legend_has_an_editable_valid_training_list() {
        for legend in local_rift::legend_catalog::legends() {
            let editor = DeckEditor::new(CustomDeck::from_legend(legend.id).unwrap());
            assert_eq!(editor.deck.card_count(), 40);
            assert!(editor.deck.validate().is_ok());
            assert!(editor.visible_cards().contains(&legend.default_champion));
            assert_eq!(editor.deck.legend, legend.id);
        }
    }

    #[test]
    fn count_changes_keep_one_row_and_remove_zero() {
        let mut editor = DeckEditor::new(CustomDeck::from_preset(2).unwrap());
        let id = eligible_cards(2)
            .into_iter()
            .find(|id| editor.count(*id) == 0)
            .unwrap();
        editor.change_count(id, 1);
        editor.change_count(id, 1);
        assert_eq!(editor.count(id), 2);
        assert_eq!(
            editor.deck.cards.iter().filter(|(c, _)| *c == id).count(),
            1
        );
        editor.change_count(id, -1);
        editor.change_count(id, -1);
        assert!(!editor.deck.cards.iter().any(|(c, _)| *c == id));
    }
    #[test]
    fn additional_filter_has_supported_cards_absent_from_all_presets() {
        let mut count = 0;
        for legend in 0..4 {
            let mut editor = DeckEditor::new(CustomDeck::from_preset(legend).unwrap());
            editor.filter = DeckFilter::Additional;
            for id in editor.visible_cards() {
                assert!(eligible_cards(legend).contains(&id));
                assert!(
                    !local_rift::cards::presets()
                        .iter()
                        .any(|p| p.cards.iter().any(|(c, _)| *c == id))
                );
                count += 1;
            }
        }
        assert!(count >= 12);
    }
}
