mod companion;
use companion::{AdviceLevel, Companion, draw_mote};
mod scene;
mod smoke;
mod ui;
use local_rift::{
    cards::{CardKind, Domain},
    game::{Action, Decision, Difficulty, Game, Phase, PlayOptions, Target, domain_index},
};
use macroquad::prelude::*;
use scene::{SceneState, SceneUnit};
use std::{collections::HashSet, path::PathBuf};
use ui::GOLD;
use ui::*;
include!(concat!(env!("OUT_DIR"), "/card_art.rs"));

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Practice,
    Decks,
    Guide,
    Settings,
}
#[derive(Clone, Copy, PartialEq)]
enum Selection {
    Hand(usize),
    Unit(u32),
    Champion,
    Gear(u32),
    Legend,
    Trash(usize),
    Hidden(u32),
}
#[derive(Clone, Copy, PartialEq)]
enum Tray {
    Hand,
    Board,
    Loadout,
    Trash,
    Hidden,
}
#[derive(Clone, PartialEq)]
struct TokenPreview {
    name: String,
    might: u8,
    rules: String,
}
#[derive(Clone)]
enum DragSource {
    Hand { index: usize, card_id: usize },
    Units { ids: Vec<u32>, card_id: usize },
}
#[derive(Clone)]
struct DragState {
    source: DragSource,
    origin: Vec2,
    active: bool,
    previous_selection: Option<Selection>,
    previous_group: Vec<u32>,
}
#[derive(Clone, Copy)]
enum DropZone {
    Base,
    Battlefield(usize),
    Unit(usize, u32),
    Chain(u32),
}

struct Preferences {
    coach: bool,
    advanced: bool,
    animation: bool,
    tactical: bool,
    wins: u32,
    losses: u32,
}
impl Preferences {
    fn path() -> PathBuf {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|s| PathBuf::from(s).join(".local/share")))
            .unwrap_or_else(std::env::temp_dir)
            .join("local-rift/preferences.txt")
    }
    fn load() -> Self {
        let content = std::fs::read_to_string(Self::path()).unwrap_or_default();
        let value = |key: &str| {
            content
                .lines()
                .find_map(|l| l.strip_prefix(&format!("{key}=")))
        };
        Self {
            coach: value("coach") != Some("false"),
            advanced: value("advanced") == Some("true"),
            animation: value("animation") != Some("false"),
            tactical: value("tactical") == Some("true"),
            wins: value("wins").and_then(|v| v.parse().ok()).unwrap_or(0),
            losses: value("losses").and_then(|v| v.parse().ok()).unwrap_or(0),
        }
    }
    fn save(&self) {
        let p = Self::path();
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(
            p,
            format!(
                "coach={}\nadvanced={}\nanimation={}\ntactical={}\nwins={}\nlosses={}\n",
                self.coach, self.advanced, self.animation, self.tactical, self.wins, self.losses
            ),
        );
    }
}
struct App {
    ui: Ui,
    game: Game,
    screen: Screen,
    in_match: bool,
    player_deck: usize,
    opponent_deck: usize,
    browse_deck: usize,
    selection: Option<Selection>,
    mulligan: HashSet<usize>,
    art: Vec<Option<Texture2D>>,
    prefs: Preferences,
    companion: Companion,
    toast: String,
    toast_until: f64,
    next_ai: f64,
    orbit: f32,
    deck_scroll: usize,
    hand_scroll: usize,
    inspect: Option<usize>,
    lesson: Option<usize>,
    group: Vec<u32>,
    targets: Vec<Target>,
    repeat_targets: Vec<Target>,
    editing_repeat: bool,
    destination: Option<usize>,
    tray: Tray,
    gear_owner: usize,
    choices: Vec<usize>,
    options: PlayOptions,
    fields: [usize; 2],
    starting_player: usize,
    show_journal: bool,
    journal_page: usize,
    chain_cursor: usize,
    damage_order: Vec<u32>,
    confirm_restart: bool,
    recorded: bool,
    drag: Option<DragState>,
    drop_zones: Vec<(DropZone, Rect)>,
    hover_candidate: Option<usize>,
    hover_token_candidate: Option<TokenPreview>,
    hover_token: Option<TokenPreview>,
    hover_card: Option<usize>,
    hover_since: f64,
    hover_rect: Option<Rect>,
    hover_scroll: usize,
    inspect_scroll: usize,
    inspect_last: Option<usize>,
    rune_menu: Option<Domain>,
    rune_menu_at: Vec2,
    field_hover: Option<usize>,
    field_inspect: Option<usize>,
}
fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|t| t.as_nanos() as u64)
        .unwrap_or(42)
}
impl App {
    fn new() -> Self {
        let prefs = Preferences::load();
        let game = Game::new(
            0,
            1,
            seed(),
            if prefs.tactical {
                Difficulty::Tactical
            } else {
                Difficulty::Easy
            },
        );
        let mut art = vec![None; game.catalog.len()];
        for &(id, bytes) in CARD_ART {
            if id < art.len() {
                let tex = Texture2D::from_file_with_format(bytes, None);
                tex.set_filter(FilterMode::Linear);
                art[id] = Some(tex);
            }
        }
        Self {
            ui: Ui::new(),
            game,
            screen: Screen::Practice,
            in_match: false,
            player_deck: 0,
            opponent_deck: 1,
            browse_deck: 0,
            selection: None,
            mulligan: HashSet::new(),
            art,
            companion: Companion::new(if prefs.advanced {
                AdviceLevel::Advanced
            } else {
                AdviceLevel::Beginner
            }),
            prefs,
            toast: String::new(),
            toast_until: 0.0,
            next_ai: 0.0,
            orbit: 0.0,
            deck_scroll: 0,
            hand_scroll: 0,
            inspect: None,
            lesson: None,
            group: Vec::new(),
            targets: Vec::new(),
            repeat_targets: Vec::new(),
            editing_repeat: false,
            destination: None,
            tray: Tray::Hand,
            gear_owner: 0,
            choices: Vec::new(),
            options: PlayOptions::default(),
            fields: [0, 1],
            starting_player: 0,
            show_journal: false,
            journal_page: 0,
            chain_cursor: 0,
            damage_order: Vec::new(),
            confirm_restart: false,
            recorded: false,
            drag: None,
            drop_zones: Vec::new(),
            hover_candidate: None,
            hover_token_candidate: None,
            hover_token: None,
            hover_card: None,
            hover_since: 0.0,
            hover_rect: None,
            hover_scroll: 0,
            inspect_scroll: 0,
            inspect_last: None,
            rune_menu: None,
            rune_menu_at: vec2(898.0, 395.0),
            field_hover: None,
            field_inspect: None,
        }
    }
    fn reset(&mut self) {
        self.game = Game::new(
            self.player_deck,
            self.opponent_deck,
            seed(),
            if self.prefs.tactical {
                Difficulty::Tactical
            } else {
                Difficulty::Easy
            },
        );
        self.game
            .choose_starting_player(if self.starting_player == 2 {
                (seed() % 2) as usize
            } else {
                self.starting_player
            })
            .expect("valid starting player");
        for player in 0..2 {
            self.game
                .choose_battlefield(player, self.fields[player])
                .expect("valid selected battlefield");
        }
        self.drag = None;
        self.hover_card = None;
        self.show_journal = false;
        self.damage_order.clear();
        self.tray = Tray::Hand;
        self.repeat_targets.clear();
        self.editing_repeat = false;
        self.group.clear();
        self.destination = None;
        self.options = PlayOptions::default();
        self.choices.clear();
        self.targets.clear();
        self.selection = None;
        self.mulligan.clear();
        self.in_match = true;
        self.recorded = false;
        self.confirm_restart = false;
        self.hand_scroll = 0;
    }
    fn notify(&mut self, s: impl Into<String>) {
        self.toast = s.into();
        self.toast_until = get_time() + 4.5;
    }
    fn action(&mut self, a: Action) {
        let preserve_selection = matches!(
            a,
            Action::ActivateRune { .. } | Action::SetOptionalDiscount { .. }
        );
        let adding_resource = matches!(
            a,
            Action::ActivateGear { .. }
                | Action::ActivateLegend { .. }
                | Action::ActivateRune { .. }
        ) && self.game.decision.is_some();
        match self.game.apply(a) {
            Ok(()) => {
                if preserve_selection {
                    return;
                }
                if adding_resource {
                    self.tray = Tray::Hand;
                }
                self.damage_order.clear();
                self.selection = None;
                self.group.clear();
                self.targets.clear();
                self.repeat_targets.clear();
                self.editing_repeat = false;
                self.destination = None;
                self.choices.clear();
                self.options = PlayOptions::default();
                self.next_ai = get_time() + 0.7;
            }
            Err(e) => self.notify(e),
        }
    }
    fn update(&mut self) {
        self.ui.resize();
        self.companion.observe(&self.game);
        if self.in_match && self.game.winner.is_some() && !self.recorded {
            if self.game.winner == Some(0) {
                self.prefs.wins += 1;
            } else {
                self.prefs.losses += 1;
            }
            self.prefs.save();
            self.recorded = true;
        }
        if self.screen == Screen::Practice
            && self.inspect.is_none()
            && self.lesson.is_none()
            && !self.confirm_restart
            && !self.show_journal
            && self.field_inspect.is_none()
        {
            if self.in_match
                && self.game.phase == Phase::Playing
                && self.game.priority() == 1
                && self.drag.is_none()
                && get_time() > self.next_ai
            {
                if let Err(e) = self.game.step_ai() {
                    self.notify(e);
                }
                self.next_ai = get_time() + if self.prefs.animation { 0.68 } else { 0.05 };
            }
            if is_key_pressed(KeyCode::Space)
                && self.drag.is_none()
                && self.in_match
                && self.game.phase == Phase::Playing
                && self.game.priority() == 0
            {
                self.pass_or_end();
            }
            if is_key_pressed(KeyCode::H) {
                self.screen = Screen::Guide;
            }
            if is_key_pressed(KeyCode::Escape) {
                self.cancel_drag();
                self.selection = None;
            }
            let keys = [
                KeyCode::Key1,
                KeyCode::Key2,
                KeyCode::Key3,
                KeyCode::Key4,
                KeyCode::Key5,
                KeyCode::Key6,
                KeyCode::Key7,
                KeyCode::Key8,
                KeyCode::Key9,
            ];
            for (i, k) in keys.iter().enumerate() {
                if is_key_pressed(*k)
                    && self.drag.is_none()
                    && i < self.game.players[0].hand.len()
                    && self.game.phase == Phase::Playing
                    && self.game.priority() == 0
                {
                    self.selection = Some(Selection::Hand(i));
                    self.targets.clear();
                    self.destination = None;
                    self.options = PlayOptions::default();
                    self.repeat_targets.clear();
                    self.editing_repeat = false;
                }
            }
        } else if is_key_pressed(KeyCode::Escape) {
            self.inspect = None;
            self.field_inspect = None;
            self.lesson = None;
            self.confirm_restart = false;
            self.show_journal = false;
            self.screen = Screen::Practice;
        }
        if self.screen == Screen::Decks && self.ui.mouse().y > 430.0 && self.hover_card.is_none() {
            let wheel = mouse_wheel().1;
            if wheel < 0.0 {
                self.deck_scroll = self.deck_scroll.saturating_add(1).min(
                    self.game.presets[self.browse_deck]
                        .cards
                        .len()
                        .saturating_sub(14),
                );
            } else if wheel > 0.0 {
                self.deck_scroll = self.deck_scroll.saturating_sub(1);
            }
        }
    }
    fn draw(&mut self) {
        clear_background(BG);
        self.ui.rect(Rect::new(0.0, 0.0, 1440.0, 900.0), BG);
        self.field_hover = None;
        self.hover_candidate = None;
        self.hover_token_candidate = None;
        self.drop_zones.clear();
        if self.hover_rect.is_some_and(|r| r.contains(self.ui.mouse()))
            && self.ui.secondary_pressed()
            && self.hover_token.is_none()
        {
            self.inspect = self.hover_card;
        }
        self.ui.enabled = self.inspect.is_none()
            && self.lesson.is_none()
            && !self.confirm_restart
            && !self.show_journal
            && self.rune_menu.is_none()
            && self.field_inspect.is_none()
            && !self.hover_rect.is_some_and(|r| r.contains(self.ui.mouse()));
        self.sidebar();
        match self.screen {
            Screen::Practice => self.practice(),
            Screen::Decks => self.decks(),
            Screen::Guide => self.guide(),
            Screen::Settings => self.settings(),
        }
        if self.toast_until > get_time() {
            let width = (self.ui.width(&self.toast, 13.0) + 40.0).min(930.0);
            let r = Rect::new(720.0 - width / 2.0, 84.0, width, 42.0);
            self.ui.rect(r, Color::new(0.16, 0.24, 0.25, 1.0));
            self.ui.border(r, TEAL);
            self.ui.ellipsis(
                &self.toast,
                r.x + 20.0,
                r.y + 26.0,
                width - 40.0,
                13.0,
                TEXT,
            );
        }
        if let Some(id) = self.inspect {
            self.inspector(id);
        }
        if let Some(deck) = self.lesson {
            self.legend_lesson(deck);
        }
        if self.confirm_restart {
            self.restart_dialog();
        }
        if self.show_journal {
            self.journal();
        }
        self.drag_overlay();
        self.hover_preview();
        self.resource_tooltip();
        self.rune_popover();
        self.battlefield_help();
        self.ui.enabled = true;
    }
    fn sidebar(&mut self) {
        let u = &self.ui;
        u.rect(
            Rect::new(0.0, 0.0, 214.0, 900.0),
            Color::new(0.039, 0.061, 0.074, 1.0),
        );
        u.line(vec2(214.0, 0.0), vec2(214.0, 900.0), 1.0, LINE);
        u.diamond(vec2(38.0, 43.0), 14.0, GOLD);
        u.diamond(vec2(38.0, 43.0), 7.0, BG);
        u.line(vec2(38.0, 22.0), vec2(38.0, 64.0), 1.0, GOLD);
        u.title("RIFTBOUND", 61.0, 43.0, 20.0, TEXT);
        u.text("THE PRACTICE GROUNDS", 62.0, 60.0, 8.0, MUTED);
        u.text("YOUR SANCTUARY", 25.0, 123.0, 10.0, MUTED);
        for (i, (label, screen, _icon)) in [
            ("Practice", Screen::Practice, "◇"),
            ("Deck library", Screen::Decks, "▤"),
            ("Learn to play", Screen::Guide, "?"),
            ("Settings", Screen::Settings, "⚙"),
        ]
        .iter()
        .enumerate()
        {
            let y = 145.0 + i as f32 * 52.0;
            let r = Rect::new(14.0, y, 185.0, 44.0);
            let selected = self.screen == *screen;
            if selected {
                u.rect(r, Color::new(0.075, 0.14, 0.15, 1.0));
                u.rect(Rect::new(14.0, y + 9.0, 2.0, 26.0), TEAL);
            } else if u.hover(r) {
                u.rect(r, PANEL);
            }
            let c = if selected { TEAL } else { MUTED };
            match i {
                0 => {
                    u.diamond(vec2(36.0, y + 22.0), 8.0, c);
                    u.diamond(vec2(36.0, y + 22.0), 5.0, PANEL);
                }
                1 => {
                    u.border(Rect::new(29.0, y + 13.0, 12.0, 17.0), c);
                    u.line(vec2(33.0, y + 10.0), vec2(44.0, y + 10.0), 1.0, c);
                }
                2 => u.bold("?", 31.0, y + 28.0, 18.0, c),
                _ => {
                    u.circle(vec2(36.0, y + 22.0), 7.0, c);
                    u.circle(vec2(36.0, y + 22.0), 4.0, PANEL);
                }
            }
            u.text(
                label,
                61.0,
                y + 27.0,
                13.0,
                if selected { TEXT } else { MUTED },
            );
            if u.clicked(r) {
                self.screen = *screen;
            }
        }
        u.line(vec2(25.0, 383.0), vec2(189.0, 383.0), 1.0, LINE);
        u.text("PRACTICE RECORD", 25.0, 417.0, 10.0, MUTED);
        u.title(&self.prefs.wins.to_string(), 25.0, 456.0, 30.0, TEXT);
        u.text("wins", 60.0, 453.0, 11.0, MUTED);
        u.title(&self.prefs.losses.to_string(), 113.0, 456.0, 30.0, TEXT);
        u.text("losses", 148.0, 453.0, 11.0, MUTED);
        if self.prefs.coach {
            u.text("MOTE · YOUR COMPANION", 25.0, 505.0, 9.0, GOLD);
            let center = u.offset + vec2(66.0, 547.0) * u.scale;
            draw_mote(
                center,
                u.scale * 0.95,
                if self.prefs.animation {
                    get_time() as f32
                } else {
                    0.0
                },
                TEAL,
            );
            u.title("Mote", 110.0, 544.0, 23.0, TEXT);
            u.text("A little guidance.", 110.0, 562.0, 8.0, MUTED);
            for (i, level) in [AdviceLevel::Beginner, AdviceLevel::Advanced]
                .iter()
                .enumerate()
            {
                if u.button(
                    Rect::new(22.0 + i as f32 * 88.0, 584.0, 83.0, 30.0),
                    level.label(),
                    self.companion.level() == *level,
                    true,
                ) {
                    self.companion.set_level(*level);
                    self.prefs.advanced = *level == AdviceLevel::Advanced;
                    self.prefs.save();
                }
            }
            u.rect(Rect::new(22.0, 627.0, 171.0, 170.0), PANEL);
            u.wrapped(self.companion.message(), 34.0, 650.0, 147.0, 11.0, TEXT);
            if u.button(
                Rect::new(34.0, 761.0, 147.0, 26.0),
                "Another thought",
                false,
                true,
            ) {
                self.companion.next_tip(&self.game);
            }
        } else {
            u.text("Mote is resting.", 26.0, 741.0, 12.0, MUTED);
            u.text("Wake in Settings", 26.0, 762.0, 10.0, MUTED);
        }
        u.circle(vec2(29.0, 851.0), 3.0, TEAL);
        u.text("Offline & ready to play", 41.0, 855.0, 11.0, MUTED);
        u.text("Unofficial fan practice client", 25.0, 878.0, 9.0, MUTED);
    }
    fn header(&self, title: &str, subtitle: &str) {
        self.ui.title(title, 244.0, 45.0, 29.0, TEXT);
        self.ui.text(subtitle, 245.0, 70.0, 12.0, MUTED);
        self.ui
            .line(vec2(244.0, 91.0), vec2(1410.0, 91.0), 1.0, LINE);
    }
    fn champion_art(&self, deck: usize, r: Rect, tint: Color) {
        let id = self.game.presets[deck].chosen_champion;
        if let Some(Some(tex)) = self.art.get(id) {
            let w = tex.width();
            let h = tex.height();
            self.ui.image(
                tex,
                r,
                Some(Rect::new(w * 0.08, h * 0.12, w * 0.84, h * 0.43)),
                tint,
            );
        } else {
            self.ui.rect(r, rgb(self.game.presets[deck].accent));
            self.ui
                .diamond(vec2(r.x + r.w / 2.0, r.y + r.h / 2.0), r.h * 0.3, GOLD);
        }
    }
    fn practice(&mut self) {
        self.header(
            if self.in_match {
                "A little closer to mastery."
            } else {
                "The practice grounds."
            },
            "A quiet place to learn. An opponent that's always ready.",
        );
        if self.ui.button(
            Rect::new(1246.0, 25.0, 164.0, 42.0),
            if self.in_match {
                "New match"
            } else {
                "Start practice  >"
            },
            !self.in_match,
            true,
        ) {
            if self.in_match {
                self.confirm_restart = true;
            } else {
                self.reset();
            }
        }
        let p = self.ui.physical(Rect::new(232.0, 105.0, 910.0, 536.0));
        let mut units = Vec::new();
        for player in 0..2 {
            for unit in &self.game.players[player].units {
                units.push(SceneUnit {
                    id: unit.id,
                    owner: player,
                    location: unit.lane.map(|i| i + 1).unwrap_or(0),
                    ready: unit.ready,
                    might: self.game.unit_might(unit),
                    accent: self.game.presets[if player == 0 {
                        self.player_deck
                    } else {
                        self.opponent_deck
                    }]
                    .accent,
                });
            }
        }
        if !self.in_match {
            units = vec![
                SceneUnit {
                    id: 900,
                    owner: 0,
                    location: 0,
                    ready: true,
                    might: 4,
                    accent: [91, 198, 178],
                },
                SceneUnit {
                    id: 901,
                    owner: 1,
                    location: 0,
                    ready: true,
                    might: 5,
                    accent: [217, 166, 104],
                },
            ];
        }
        let state = SceneState {
            units,
            selected: match self.selection {
                Some(Selection::Unit(id)) => Some(id),
                _ => None,
            },
            owners: self.game.battlefield_owner,
            time: if self.prefs.animation {
                get_time() as f32
            } else {
                0.0
            },
            orbit: self.orbit,
        };
        let hits = scene::draw_scene(p, &state);
        self.drop_zones
            .push((DropZone::Base, self.ui.logical(hits.bases[0])));
        for (lane, rect) in hits.battlefields.iter().enumerate() {
            self.drop_zones
                .push((DropZone::Battlefield(lane), self.ui.logical(*rect)));
        }
        self.draw_playmat(&hits);
        let mouse = self.ui.offset + self.ui.mouse() * self.ui.scale;
        let mut hit_unit = false;
        let top_unit = hits
            .units
            .iter()
            .rev()
            .find(|(_, r)| r.contains(mouse))
            .map(|(id, _)| *id);
        for (id, r) in &hits.units {
            if !self.in_match {
                break;
            }
            let found = self.game.players.iter().enumerate().find_map(|(i, p)| {
                p.units
                    .iter()
                    .find(|u| u.id == *id)
                    .map(|u| (i, u.card_id, u.token, self.game.unit_name(u)))
            });
            if let Some((owner, card_id, token, unit_name)) = found {
                let card_rect = self.ui.logical(*r);
                let unit = self.game.players[owner]
                    .units
                    .iter()
                    .find(|unit| unit.id == *id)
                    .unwrap();
                self.board_card(
                    card_rect,
                    card_id,
                    owner,
                    unit.ready,
                    self.game.unit_might(unit),
                    unit.damage,
                    token,
                    &unit_name,
                    state.selected == Some(*id) || self.group.contains(id),
                    self.targets.contains(&Target::Unit {
                        player: owner,
                        unit_id: *id,
                    }),
                );
                self.drop_zones
                    .push((DropZone::Unit(owner, *id), card_rect));
                if let Some(Decision::OnPlayTarget {
                    player,
                    card_id,
                    source_id,
                    lane,
                }) = self.game.decision
                    && self.game.on_play_target_valid(
                        player,
                        card_id,
                        source_id,
                        lane,
                        Target::Unit {
                            player: owner,
                            unit_id: *id,
                        },
                    )
                {
                    self.ui.border(
                        Rect::new(
                            card_rect.x - 3.0,
                            card_rect.y - 3.0,
                            card_rect.w + 6.0,
                            card_rect.h + 6.0,
                        ),
                        TEAL,
                    );
                }
                if self.ui.hover(card_rect) {
                    self.hover_candidate = Some(card_id);
                    if token {
                        self.hover_token_candidate = Some(TokenPreview {
                            name: unit_name.clone(),
                            might: self.game.unit_might(unit),
                            rules: if card_id == 34 {
                                "Deflect: an opponent must pay 1 extra power to choose this Bird with a spell or ability.".into()
                            } else {
                                "This created unit has no printed abilities.".into()
                            },
                        });
                    }
                }
                if top_unit == Some(*id) {
                    hit_unit = true;
                }
                let gear_hovered = self.gear_badges(card_rect, owner, *id);
                if top_unit == Some(*id) && self.ui.enabled && !gear_hovered {
                    hit_unit = true;
                    if self.ui.secondary_pressed() {
                        if token {
                            self.notify(format!("{unit_name}: a created unit. Its current might is shown on the board.{}",if card_id==34 {" This Bird has Deflect."}else{""}));
                        } else {
                            self.inspect = Some(card_id);
                        }
                    }
                    if self.ui.primary_pressed()
                        && self.game.priority() == 0
                        && self.game.phase == Phase::Playing
                    {
                        if owner == 0 && self.game.decision.is_none() && !self.targeting_selection()
                        {
                            let ids = if self.group.contains(id) {
                                self.group.clone()
                            } else {
                                vec![*id]
                            };
                            self.arm_drag(DragSource::Units { ids, card_id });
                        }
                        if matches!(
                            self.game.decision,
                            Some(
                                Decision::SwapUnit { .. }
                                    | Decision::EmpowerHere { .. }
                                    | Decision::OnPlayTarget { .. }
                            )
                        ) {
                            self.action(Action::ChooseTarget {
                                target: Some(Target::Unit {
                                    player: owner,
                                    unit_id: *id,
                                }),
                            });
                        } else {
                            match self.selection {
                                _ if self.targeting_selection() => {
                                    let target = Target::Unit {
                                        player: owner,
                                        unit_id: *id,
                                    };
                                    let targets = if self.editing_repeat {
                                        &mut self.repeat_targets
                                    } else {
                                        &mut self.targets
                                    };
                                    if let Some(pos) = targets.iter().position(|t| *t == target) {
                                        targets.remove(pos);
                                    } else if targets.len() < 3 {
                                        targets.push(target);
                                    }
                                    let count = targets.len();
                                    self.notify(format!(
                                        "{count} target(s) chosen for {}. Confirm with Play.",
                                        if self.editing_repeat {
                                            "the repeat"
                                        } else {
                                            "the first effect"
                                        }
                                    ));
                                }
                                _ if owner == 0
                                    && (is_key_down(KeyCode::LeftShift)
                                        || is_key_down(KeyCode::RightShift)) =>
                                {
                                    if let Some(Selection::Unit(first)) = self.selection {
                                        if self.group.is_empty() {
                                            self.group.push(first);
                                        }
                                        if let Some(index) =
                                            self.group.iter().position(|unit| unit == id)
                                        {
                                            self.group.remove(index);
                                        } else {
                                            self.group.push(*id);
                                        }
                                        self.notify(format!("{} units grouped. Choose a battlefield to move together.",self.group.len()));
                                    } else {
                                        self.selection = Some(Selection::Unit(*id));
                                        self.group = vec![*id];
                                    }
                                }
                                _ if owner == 0 => {
                                    self.group.clear();
                                    self.selection = if self.selection == Some(Selection::Unit(*id))
                                    {
                                        None
                                    } else {
                                        Some(Selection::Unit(*id))
                                    }
                                }
                                _ => {
                                    self.notify("Select a spell first to target an opposing unit.")
                                }
                            }
                        }
                    }
                }
            }
        }
        if !self.in_match {
            for (index, (_, rect)) in hits.units.iter().enumerate() {
                let deck = if index == 0 {
                    self.player_deck
                } else {
                    self.opponent_deck
                };
                let id = self.game.presets[deck].chosen_champion;
                let card = self.game.catalog[id];
                self.board_card(
                    self.ui.logical(*rect),
                    id,
                    index,
                    true,
                    card.might,
                    0,
                    false,
                    card.name,
                    false,
                    false,
                );
            }
        }
        for (i, r) in hits.battlefields.iter().enumerate() {
            if !hit_unit
                && r.contains(mouse)
                && mouse.y > r.y + 72.0 * self.ui.scale
                && self.ui.enabled
                && self.ui.primary_pressed()
                && self.in_match
                && self.game.priority() == 0
                && self.game.phase == Phase::Playing
            {
                if let Some(Selection::Unit(id)) = self.selection {
                    self.move_selection(id, Some(i));
                } else if let Some(Selection::Hand(index)) = self.selection {
                    if self.game.catalog[self.game.players[0].hand[index]].kind == CardKind::Unit {
                        self.action(Action::PlayCardOptions {
                            hand_index: index,
                            lane: Some(i),
                            targets: self.targets.clone(),
                            repeat_targets: self.repeat_targets.clone(),
                            repeat: self.options.repeat,
                            accelerate: self.options.accelerate,
                            bonus: self.options.bonus,
                        });
                    } else {
                        self.destination = Some(i);
                        self.notify(format!(
                            "Destination: battlefield {}. Choose targets, then Play.",
                            i + 1
                        ));
                    }
                } else if matches!(self.selection, Some(Selection::Trash(_))) {
                    self.destination = Some(i);
                    self.notify(format!(
                        "Destination: field {}. Choose targets, then Play.",
                        i + 1
                    ));
                } else if self.selection == Some(Selection::Champion) {
                    self.action(Action::PlayChampionOptions {
                        lane: Some(i),
                        target: self.targets.first().copied(),
                        accelerate: self.options.accelerate,
                    });
                }
            }
        }
        if self.in_match {
            self.match_panel();
            self.hand();
        } else {
            self.setup_panel();
            self.matchup();
        }
        if self.in_match && self.game.winner.is_some() {
            self.victory();
        }
    }
    fn pass_or_end(&mut self) {
        if self.game.decision.is_some() {
            return;
        }
        self.action(
            if self.game.chain.is_empty() && self.game.showdown.is_none() {
                Action::EndTurn
            } else {
                Action::Pass
            },
        );
    }
    fn move_selection(&mut self, id: u32, lane: Option<usize>) {
        if self.group.contains(&id) && self.group.len() > 1 {
            self.action(Action::MoveGroup {
                unit_ids: self.group.clone(),
                lane,
            });
        } else {
            self.action(Action::Move { unit_id: id, lane });
        }
    }
    fn response_ribbon(&mut self) {
        self.ui.rect(Rect::new(1170.0, 232.0, 228.0, 108.0), BG);
        self.chain_cursor = self
            .chain_cursor
            .min(self.game.chain.len().saturating_sub(1));
        if let Some(item) = self.game.chain.iter().rev().nth(self.chain_cursor).cloned() {
            let r = Rect::new(1178.0, 253.0, 212.0, 40.0);
            self.drop_zones.push((DropZone::Chain(item.id), r));
            self.ui.text(
                &format!(
                    "CHAIN · {} / {} · NEWEST FIRST",
                    self.chain_cursor + 1,
                    self.game.chain.len()
                ),
                1178.0,
                247.0,
                8.0,
                GOLD,
            );
            self.ui.wrapped_lines(
                &self.game.chain_name(&item),
                1178.0,
                269.0,
                212.0,
                12.0,
                TEXT,
                2,
            );
            if self.ui.hover(r) {
                self.hover_candidate = Some(item.card_id);
            }
            if self.ui.button(
                Rect::new(1178.0, 307.0, 28.0, 26.0),
                "<",
                false,
                self.chain_cursor > 0,
            ) {
                self.chain_cursor -= 1;
            }
            if self.ui.button(
                Rect::new(1211.0, 307.0, 28.0, 26.0),
                ">",
                false,
                self.chain_cursor + 1 < self.game.chain.len(),
            ) {
                self.chain_cursor += 1;
            }
            if self.ui.button(
                Rect::new(1244.0, 307.0, 146.0, 26.0),
                "Choose spell",
                false,
                self.selection.is_some() && self.game.priority() == 0,
            ) {
                let target = Target::Spell { chain_id: item.id };
                let targets = if self.editing_repeat {
                    &mut self.repeat_targets
                } else {
                    &mut self.targets
                };
                if targets.contains(&target) {
                    targets.clear();
                } else {
                    *targets = vec![target];
                }
            }
        } else if let Some(showdown) = &self.game.showdown {
            self.ui.text(
                &format!("SHOWDOWN · FIELD {}", showdown.lane + 1),
                1178.0,
                252.0,
                11.0,
                GOLD,
            );
            self.ui.wrapped(
                "Both players may respond before the field is settled. Pass when you're ready.",
                1178.0,
                279.0,
                209.0,
                11.0,
                MUTED,
            );
        }
    }

    fn damage_assignment(&self, lane: usize, total: u16) -> Vec<(u32, u16)> {
        let suggested = self.game.automatic_damage_assignment(0, lane, total);
        let mut order = self.damage_order.clone();
        for (id, _) in &suggested {
            if !order.contains(id) {
                order.push(*id);
            }
        }
        let mut left = total;
        let mut allocation = Vec::new();
        for (i, id) in order.iter().enumerate() {
            if left == 0 {
                break;
            }
            if let Some(unit) = self.game.players[1]
                .units
                .iter()
                .find(|u| u.id == *id && u.lane == Some(lane))
            {
                let lethal = self
                    .game
                    .unit_might(unit)
                    .saturating_sub(unit.damage)
                    .max(1) as u16;
                let amount = if i + 1 == order.len() {
                    left
                } else {
                    left.min(lethal)
                };
                if amount > 0 {
                    allocation.push((*id, amount));
                    left -= amount;
                }
            }
        }
        allocation
    }
    fn combat_tray(&mut self, lane: usize, total: u16) {
        let units = self.game.players[1]
            .units
            .iter()
            .filter(|u| u.lane == Some(lane))
            .cloned()
            .collect::<Vec<_>>();
        let allocation = self.damage_assignment(lane, total);
        self.ui.text("Choose damage order", 780.0, 671.0, 9.0, GOLD);
        self.hand_scroll = self.hand_scroll.min(units.len().saturating_sub(6));
        for (slot, unit) in units.iter().skip(self.hand_scroll).take(6).enumerate() {
            let r = self.tile(
                unit.card_id,
                slot,
                &self.game.unit_name(unit),
                self.damage_order.contains(&unit.id),
            );
            if unit.token {
                self.ui
                    .rect(Rect::new(r.x + 2.0, r.y + 2.0, 136.0, 94.0), BG);
                self.ui.diamond(vec2(r.x + 70.0, r.y + 38.0), 18.0, TEAL);
            }
            let amount = allocation
                .iter()
                .find(|(id, _)| *id == unit.id)
                .map_or(0, |(_, n)| *n);
            let order = allocation
                .iter()
                .position(|(id, _)| *id == unit.id)
                .map(|n| format!("#{} · ", n + 1))
                .unwrap_or_default();
            self.ui
                .rect(Rect::new(r.x + 2.0, r.y + 68.0, 136.0, 28.0), PANEL);
            self.ui.text(
                &format!("{order}{amount} DAMAGE"),
                r.x + 10.0,
                r.y + 87.0,
                10.0,
                GOLD,
            );
            if self.ui.clicked(r) {
                if let Some(i) = self.damage_order.iter().position(|id| *id == unit.id) {
                    self.damage_order.remove(i);
                } else {
                    self.damage_order.push(unit.id);
                }
            }
            if self.ui.hover(r) {
                self.hover_candidate = Some(unit.card_id);
            }
            if self.ui.hover(r) && self.ui.secondary_pressed() {
                self.inspect = Some(unit.card_id);
            }
        }
        self.tray_scroll(units.len(), 6);
    }
    fn decision_panel(&mut self) {
        use local_rift::legends::LegendTrigger;
        let Some(decision) = self.game.decision.clone() else {
            return;
        };
        if self.game.resource_payment_pending()
            && matches!(self.selection, Some(Selection::Gear(_) | Selection::Legend))
        {
            self.selection_panel();
            return;
        }
        if matches!(
            decision,
            Decision::LastRites { .. } | Decision::ReplaySpell { .. } | Decision::ReviveUnit { .. }
        ) && matches!(self.selection, Some(Selection::Trash(_)))
        {
            self.selection_panel();
            return;
        }
        if let Decision::TokenPlacement { remaining, .. } = decision {
            self.ui.title("Place a Tentacle", 1178.0, 366.0, 21.0, TEXT);
            self.ui.wrapped(
                &format!("{remaining} remaining. Choose your base or a battlefield you control."),
                1178.0,
                397.0,
                209.0,
                12.0,
                MUTED,
            );
            for (index, lane) in [None, Some(0), Some(1)].into_iter().enumerate() {
                if self.ui.button(
                    Rect::new(1178.0, 455.0 + index as f32 * 35.0, 212.0, 29.0),
                    &lane
                        .map(|lane| format!("Field {}", lane + 1))
                        .unwrap_or_else(|| "Your base".into()),
                    false,
                    lane.is_none_or(|lane| self.game.battlefield_owner[lane] == Some(0)),
                ) {
                    self.action(Action::PlaceToken { lane });
                }
            }
            return;
        }
        if let Decision::CombatDamage { lane, total, .. } = decision {
            self.ui
                .title("Assign combat damage", 1178.0, 366.0, 20.0, TEXT);
            self.ui.wrapped_lines(&format!("Distribute {total} damage. Click enemy cards below in order. Assign lethal before moving to the next unit: Tank first, Backline last. Both sides deal damage together."),1178.0,397.0,209.0,12.0,MUTED,5);
            if self.ui.button(
                Rect::new(1178.0, 493.0, 212.0, 34.0),
                "Confirm damage order",
                true,
                true,
            ) {
                self.action(Action::AssignDamage {
                    assignments: self.damage_assignment(lane, total),
                });
            }
            if self.ui.button(
                Rect::new(1178.0, 534.0, 212.0, 27.0),
                "Use suggested order",
                false,
                true,
            ) {
                self.damage_order.clear();
            }
            return;
        }
        let (title,body)=match &decision{
            Decision::CombatDamage{..}|Decision::TokenPlacement{..}=>unreachable!(),
            Decision::OptionalPlay{card_id,..}=>("Use this triggered ability?",format!("{} offers a play from your trash. Accept to put the ability on the chain, or decline now. After accepting, choose a legal payable card when it resolves.",self.game.catalog[*card_id].name)),
            Decision::ResolutionTargets{..}=>("Choose the location", "Responses separated your targets. Choose which legal group below this spell should affect.".into()),
            Decision::OrderTriggers{..}=>("Order your triggers", "Choose which ability to put on the chain next. The last ability placed will resolve first. Pending triggers cannot be countered yet.".into()),
            Decision::OnPlayTarget{card_id,..}=>("Choose its target",format!("{} has entered play. Choose a highlighted unit for its ability, on the board or in the Board tab.",self.game.catalog[*card_id].name)),
            Decision::DeathReplacement{..}=>("Choose what saves them", "Choose a unit and replacement from the cards below. This decides which gear or banishment changes the death.".into()),
            Decision::RevealHand{..}=>("Scuttle Crab's discovery","The opponent's hand is revealed below. You can also inspect their hidden cards in the Hidden tab until this turn ends.".into()),
            Decision::Battlefield{..}=>("Sunken Temple","You conquered with a Mighty unit. Pay 1 energy to draw a card?".into()),
            Decision::ReviveUnit{..}=>("Return to the fight","Choose a unit from your trash with printed might 4 or less. Play it at base for free, or skip.".into()),
            Decision::SwapUnit{..}=>("Tideturner","Click another friendly unit at a different location to swap places, or skip. This movement does not exhaust either unit.".into()),
            Decision::EmpowerHere{..}=>("Scrapheap's gift","Click a unit at Scrapheap's location to empower it, empower your legend, or skip.".into()),
            Decision::Predict{card_id,..}=>("Predict",format!("Top card: {}. Recycle it to the bottom, or keep it on top?",self.game.catalog[*card_id].name)),
            Decision::CounterTax{cost,..}=>("Keep your spell?",format!("Pay {cost} energy, or let Hard Bargain counter your spell.")),
            Decision::Legend{trigger,..}=>match trigger{LegendTrigger::IreliaChoose{..}=>("Blade Dancer", "Pay 1 power and exhaust your legend to ready the chosen unit? This ability can be responded to.".into()),LegendTrigger::IreliaConquer=>("Dance again?","Pay 1 energy to ready your legend after conquering?".into())},
            Decision::BurnForMight{..}=>("Burn for might?","Put your top card into the trash to give Shadow Order Disciple +1 might this turn?".into()),
            Decision::Discard{count,..}=>("Choose a discard",format!("Select {count} card(s) below, then confirm. These go to your trash before you draw.")),
            Decision::TopThree{..}=>("Lightning Rush","Choose one of your top three cards below to draw. The rest go to your trash. You may choose none.".into()),
            Decision::Sabotage{..}=>("Sabotage","Your opponent's hand is revealed below. Choose a non-unit card to recycle.".into()),
            Decision::GrantFlow{..}=>("Tempest's echo","Choose a spell in your trash below. It gains Flow equal to its printed cost for this turn.".into()),
            Decision::LastRites{..}=>("Last Rites","Choose a unit from your trash below to play, paying its cost. You may skip.".into()),
            Decision::ReplaySpell{..}=>("Fizz's trick","Choose a spell in your trash with energy cost 3 or less. You still pay power. It recycles after being played.".into()),
        };
        self.ui.title(title, 1178.0, 366.0, 22.0, TEXT);
        self.ui
            .wrapped_lines(&body, 1178.0, 397.0, 209.0, 12.0, MUTED, 5);
        if matches!(
            decision,
            Decision::DeathReplacement { .. }
                | Decision::OnPlayTarget { .. }
                | Decision::OrderTriggers { .. }
                | Decision::ResolutionTargets { .. }
        ) {
            return;
        }
        let simple = matches!(
            decision,
            Decision::RevealHand { .. }
                | Decision::OptionalPlay { .. }
                | Decision::Battlefield { .. }
                | Decision::Predict { .. }
                | Decision::CounterTax { .. }
                | Decision::Legend { .. }
                | Decision::BurnForMight { .. }
        );
        if matches!(decision, Decision::EmpowerHere { .. })
            && self.ui.button(
                Rect::new(1178.0, 488.0, 212.0, 34.0),
                "Empower my legend",
                false,
                true,
            )
        {
            self.action(Action::ChooseTarget { target: None });
        }
        if simple {
            if self.ui.button(
                Rect::new(1178.0, 490.0, 212.0, 32.0),
                if matches!(decision, Decision::RevealHand { .. }) {
                    "Continue"
                } else if matches!(decision, Decision::OptionalPlay { .. }) {
                    "Use this ability"
                } else if matches!(decision, Decision::Predict { .. }) {
                    "Recycle to bottom"
                } else {
                    "Yes · pay the cost"
                },
                true,
                true,
            ) {
                self.action(Action::Choose { accept: true });
            }
            if self.ui.button(
                Rect::new(1178.0, 531.0, 212.0, 31.0),
                if matches!(decision, Decision::Predict { .. }) {
                    "Keep on top"
                } else {
                    "No · decline"
                },
                false,
                true,
            ) {
                self.action(Action::Choose { accept: false });
            }
        } else {
            if let Decision::Discard { count, .. } = decision {
                if self.ui.button(
                    Rect::new(1178.0, 491.0, 212.0, 34.0),
                    "Confirm discard",
                    true,
                    self.choices.len() == (count as usize).min(self.game.players[0].hand.len()),
                ) {
                    self.action(Action::SelectCards {
                        indices: self.choices.clone(),
                    });
                }
            } else if self.ui.button(
                Rect::new(1178.0, 531.0, 212.0, 31.0),
                if matches!(
                    decision,
                    Decision::Sabotage { .. } | Decision::GrantFlow { .. }
                ) {
                    "Continue without a target"
                } else {
                    "Skip optional choice"
                },
                false,
                match decision {
                    Decision::Sabotage { opponent, .. } => !self.game.players[opponent]
                        .hand
                        .iter()
                        .any(|id| self.game.catalog[*id].kind != CardKind::Unit),
                    Decision::LastRites { .. } | Decision::ReplaySpell { .. } => {
                        self.game.can_decline_trash_play()
                    }
                    Decision::GrantFlow { .. } => !self.game.players[0]
                        .discard
                        .iter()
                        .any(|id| self.game.catalog[*id].kind == CardKind::Spell),
                    _ => true,
                },
            ) {
                if matches!(
                    decision,
                    Decision::Sabotage { .. } | Decision::GrantFlow { .. }
                ) {
                    self.action(Action::SelectCards { indices: vec![] });
                } else if matches!(decision, Decision::SwapUnit { .. }) {
                    self.action(Action::ChooseTarget { target: None });
                } else {
                    self.action(Action::Choose { accept: false });
                }
            }
        }
    }
    fn targeting_selection(&self) -> bool {
        match self.selection {
            Some(Selection::Hand(i)) => self.game.players[0]
                .hand
                .get(i)
                .is_some_and(|id| self.game.catalog[*id].kind != CardKind::Unit && *id != 58),
            Some(
                Selection::Trash(_) | Selection::Hidden(_) | Selection::Gear(_) | Selection::Legend,
            ) => true,
            _ => false,
        }
    }
    fn tile(&self, id: usize, slot: usize, label: &str, selected: bool) -> Rect {
        let x = 245.0 + slot as f32 * 150.0;
        let y = if selected { 688.0 } else { 697.0 };
        let r = Rect::new(x, y, 140.0, 179.0);
        let card = self.game.catalog[id];
        self.ui.rect(r, PANEL);
        if let Some(tex) = &self.art[id] {
            self.ui.image(
                tex,
                Rect::new(x + 2.0, y + 2.0, 136.0, 96.0),
                Some(Rect::new(
                    tex.width() * 0.08,
                    tex.height() * 0.12,
                    tex.width() * 0.84,
                    tex.height() * 0.43,
                )),
                WHITE,
            );
        }
        self.ui
            .ellipsis(label, x + 9.0, y + 119.0, 123.0, 11.0, TEXT);
        self.ui.text(
            card.kind.name(),
            x + 9.0,
            y + 140.0,
            9.0,
            rgb(card.domain.color()),
        );
        self.ui.text(
            &format!("{} energy · {} power", card.cost, card.power_cost),
            x + 9.0,
            y + 161.0,
            9.0,
            MUTED,
        );
        self.ui.border(r, if selected { GOLD } else { LINE });
        r
    }
    fn board_tray(&mut self) {
        let units: Vec<_> = self
            .game
            .players
            .iter()
            .enumerate()
            .flat_map(|(owner, player)| player.units.iter().map(move |unit| (owner, unit.clone())))
            .collect();
        self.hand_scroll = self.hand_scroll.min(units.len().saturating_sub(6));
        for (slot, (owner, unit)) in units.iter().skip(self.hand_scroll).take(6).enumerate() {
            let r = self.tile(
                unit.card_id,
                slot,
                &self.game.unit_name(unit),
                self.selection == Some(Selection::Unit(unit.id)) || self.group.contains(&unit.id),
            );
            self.ui
                .rect(Rect::new(r.x + 3.0, r.y + 128.0, r.w - 6.0, 45.0), PANEL);
            if unit.token {
                self.ui
                    .rect(Rect::new(r.x + 2.0, r.y + 2.0, 136.0, 96.0), BG);
                self.ui.diamond(vec2(r.x + 70.0, r.y + 45.0), 22.0, TEAL);
            }
            self.ui.text(
                &format!(
                    "{} · {}",
                    if *owner == 0 { "YOU" } else { "AI" },
                    unit.lane
                        .map(|lane| format!("FIELD {}", lane + 1))
                        .unwrap_or_else(|| "BASE".into())
                ),
                r.x + 9.0,
                r.y + 145.0,
                9.0,
                if *owner == 0 { TEAL } else { GOLD },
            );
            self.ui.text(
                &format!(
                    "{} might · {}",
                    self.game.unit_might(unit),
                    if unit.ready { "ready" } else { "exhausted" }
                ),
                r.x + 9.0,
                r.y + 166.0,
                9.0,
                MUTED,
            );
            if self.ui.hover(r) {
                self.hover_candidate = Some(unit.card_id);
                if unit.token {
                    self.hover_token_candidate = Some(TokenPreview {
                        name: self.game.unit_name(unit),
                        might: self.game.unit_might(unit),
                        rules: if unit.card_id == 34 {
                            "Deflect: an opponent pays 1 extra power to choose this Bird with a spell or ability.".into()
                        } else {
                            "This created unit has no printed abilities.".into()
                        },
                    });
                }
            }
            if self.ui.hover(r) && self.ui.secondary_pressed() && !unit.token {
                self.inspect = Some(unit.card_id);
            }
            let gear_hovered = self.gear_badges(Rect::new(r.x, r.y, r.w, 112.0), *owner, unit.id);
            if self.ui.clicked(r) && self.game.priority() == 0 && !gear_hovered {
                let target = Target::Unit {
                    player: *owner,
                    unit_id: unit.id,
                };
                if self.game.decision.is_some() {
                    self.action(Action::ChooseTarget {
                        target: Some(target),
                    });
                } else if self.targeting_selection() {
                    if let Some(index) = self.targets.iter().position(|t| *t == target) {
                        self.targets.remove(index);
                    } else {
                        self.targets.push(target);
                    }
                } else if *owner == 0 {
                    let ids = if self.group.contains(&unit.id) {
                        self.group.clone()
                    } else {
                        vec![unit.id]
                    };
                    self.arm_drag(DragSource::Units {
                        ids,
                        card_id: unit.card_id,
                    });
                    self.selection = Some(Selection::Unit(unit.id));
                }
            }
        }
        self.tray_scroll(units.len(), 6);
        if units.is_empty() {
            self.ui
                .title("No units on the board yet.", 270.0, 757.0, 23.0, MUTED);
        }
    }
    fn other_tray(&mut self) {
        if self.tray == Tray::Board {
            self.board_tray();
            return;
        }
        if self.tray == Tray::Hidden {
            let cards = self.game.players[0].hidden.clone();
            for (slot, card) in cards.iter().enumerate() {
                let r = self.tile(
                    card.card_id,
                    slot,
                    self.game.catalog[card.card_id].name,
                    self.selection == Some(Selection::Hidden(card.id)),
                );
                self.ui.text(
                    &format!("FIELD {} · FACE DOWN", card.lane + 1),
                    r.x + 9.0,
                    r.y + 88.0,
                    8.0,
                    TEAL,
                );
                if self.ui.clicked(r) {
                    self.selection = Some(Selection::Hidden(card.id));
                    self.targets.clear();
                }
            }
            let enemy = self.game.players[1].hidden.clone();
            for (slot, card) in enemy.iter().enumerate() {
                if let Some(id) = self.game.hidden_visible_card(0, 1, card.id) {
                    self.tile(id, slot + 3, self.game.catalog[id].name, false);
                } else {
                    let x = 245.0 + (slot + 3) as f32 * 150.0;
                    self.ui.rect(Rect::new(x, 697.0, 140.0, 179.0), PANEL);
                    self.ui.border(Rect::new(x, 697.0, 140.0, 179.0), LINE);
                    self.ui.diamond(vec2(x + 70.0, 760.0), 23.0, GOLD);
                    self.ui
                        .text("OPPONENT · HIDDEN", x + 14.0, 823.0, 9.0, MUTED);
                    self.ui.text(
                        &format!("Field {}", card.lane + 1),
                        x + 48.0,
                        848.0,
                        11.0,
                        MUTED,
                    );
                }
            }
            if cards.is_empty() && enemy.is_empty() {
                self.ui
                    .title("A secret for another turn.", 270.0, 757.0, 23.0, MUTED);
                self.ui.text("Hide a card at a controlled battlefield for 1 power. Reveal it on a later turn.",270.0,790.0,12.0,MUTED);
            }
            return;
        }

        if self.tray == Tray::Loadout {
            if self.ui.button(
                Rect::new(780.0, 648.0, 152.0, 29.0),
                if self.gear_owner == 0 {
                    "Your loadout"
                } else {
                    "Opponent's loadout"
                },
                false,
                true,
            ) {
                self.gear_owner = 1 - self.gear_owner;
                self.hand_scroll = 0;
            }
            let owner = self.gear_owner;
            let deck = if owner == 0 {
                self.player_deck
            } else {
                self.opponent_deck
            };
            let champion = self.game.players[owner].champion;
            let id = self.game.presets[deck].chosen_champion;
            let r = self.tile(
                id,
                0,
                "Legend power",
                self.selection == Some(Selection::Legend) && owner == 0,
            );
            self.ui.text(
                if self.game.legend[owner].ready {
                    "READY"
                } else {
                    "EXHAUSTED"
                },
                r.x + 9.0,
                r.y + 88.0,
                9.0,
                GOLD,
            );
            if self.ui.clicked(r) && owner == 0 {
                self.selection = Some(Selection::Legend);
                self.targets.clear();
            }
            if self.ui.hover(r) && self.ui.secondary_pressed() {
                self.lesson = Some(deck);
            }
            if let Some(id) = champion {
                let r = self.tile(
                    id,
                    1,
                    "Chosen champion",
                    self.selection == Some(Selection::Champion) && owner == 0,
                );
                if self.ui.clicked(r) && owner == 0 {
                    self.selection = Some(Selection::Champion);
                }
                if self.ui.hover(r) {
                    self.hover_candidate = Some(id);
                }
                if self.ui.hover(r) && self.ui.secondary_pressed() {
                    self.inspect = Some(id);
                }
            }
            let gears = self.game.players[owner].gear.clone();
            self.hand_scroll = self.hand_scroll.min(gears.len().saturating_sub(4));
            for (slot, gear) in gears.iter().skip(self.hand_scroll).take(4).enumerate() {
                let r = self.tile(
                    gear.card_id,
                    slot + 2,
                    self.game.gear_name(gear),
                    self.selection == Some(Selection::Gear(gear.id)),
                );
                self.ui.text(
                    if gear.attached_to.is_some() {
                        "ATTACHED"
                    } else {
                        "AT BASE"
                    },
                    r.x + 9.0,
                    r.y + 88.0,
                    9.0,
                    TEAL,
                );
                if self.ui.clicked(r) {
                    if self.targeting_selection()
                        && !matches!(self.selection, Some(Selection::Gear(_)))
                    {
                        self.targets = vec![Target::Gear {
                            player: owner,
                            gear_id: gear.id,
                        }];
                    } else if owner == 0 {
                        self.selection = Some(Selection::Gear(gear.id));
                        self.targets.clear();
                        self.choices.clear();
                    }
                }
                if self.ui.hover(r) {
                    self.hover_candidate = Some(gear.card_id);
                }
                if self.ui.hover(r) && self.ui.secondary_pressed() {
                    self.inspect = Some(gear.card_id);
                }
            }
            self.tray_scroll(gears.len(), 4);
        } else {
            if self.ui.button(
                Rect::new(780.0, 648.0, 152.0, 29.0),
                if self.gear_owner == 0 {
                    "Your trash  >"
                } else {
                    "AI trash  >"
                },
                false,
                true,
            ) {
                self.gear_owner = 1 - self.gear_owner;
                self.hand_scroll = 0;
                self.selection = None;
            }
            let owner = self.gear_owner;
            let cards = self.game.players[owner].discard.clone();
            self.hand_scroll = self.hand_scroll.min(cards.len().saturating_sub(6));
            for (slot, (index, id)) in cards
                .iter()
                .enumerate()
                .skip(self.hand_scroll)
                .take(6)
                .enumerate()
            {
                let chosen = self.choices.contains(&index)
                    || self.selection == Some(Selection::Trash(index));
                let r = self.tile(*id, slot, self.game.catalog[*id].name, chosen);
                if self.ui.clicked(r) {
                    if owner == 1 {
                        self.inspect = Some(*id);
                    } else if matches!(self.selection, Some(Selection::Gear(_))) {
                        if let Some(i) = self.choices.iter().position(|x| *x == index) {
                            self.choices.remove(i);
                        } else if self.choices.len() < 2 {
                            self.choices.push(index);
                        }
                    } else {
                        self.selection = Some(Selection::Trash(index));
                        self.targets.clear();
                    }
                }
                if self.ui.hover(r) {
                    self.hover_candidate = Some(*id);
                }
                if self.ui.hover(r) && self.ui.secondary_pressed() {
                    self.inspect = Some(*id);
                }
            }
            self.tray_scroll(cards.len(), 6);
            if cards.is_empty() {
                self.ui.title(
                    if owner == 0 {
                        "Your trash is empty."
                    } else {
                        "The opponent's trash is empty."
                    },
                    270.0,
                    757.0,
                    23.0,
                    MUTED,
                );
            }
        }
    }
    fn tray_scroll(&mut self, len: usize, visible: usize) {
        self.ui
            .text(&format!("{} cards", len), 985.0, 671.0, 10.0, MUTED);
        if self.ui.button(
            Rect::new(1047.0, 650.0, 34.0, 28.0),
            "<",
            false,
            self.hand_scroll > 0,
        ) {
            self.hand_scroll -= 1;
        }
        if self.ui.button(
            Rect::new(1087.0, 650.0, 34.0, 28.0),
            ">",
            false,
            self.hand_scroll + visible < len,
        ) {
            self.hand_scroll += 1;
        }
    }
    fn choice_tray(&mut self) -> bool {
        let Some(decision) = self.game.decision.clone() else {
            return false;
        };
        if let Decision::CombatDamage { lane, total, .. } = decision {
            self.combat_tray(lane, total);
            return true;
        }
        if let Decision::ResolutionTargets {
            card_id, groups, ..
        } = decision
        {
            self.ui.text("Choose a location", 780.0, 671.0, 9.0, GOLD);
            self.hand_scroll = self.hand_scroll.min(groups.len().saturating_sub(6));
            for (slot, (index, group)) in groups
                .iter()
                .enumerate()
                .skip(self.hand_scroll)
                .take(6)
                .enumerate()
            {
                let location = group
                    .first()
                    .and_then(|target| match *target {
                        Target::Unit { player, unit_id } => self.game.players[player]
                            .units
                            .iter()
                            .find(|unit| unit.id == unit_id)
                            .map(|unit| {
                                unit.lane
                                    .map(|lane| format!("Field {}", lane + 1))
                                    .unwrap_or_else(|| {
                                        format!("{} base", if player == 0 { "Your" } else { "AI" })
                                    })
                            }),
                        _ => None,
                    })
                    .unwrap_or_else(|| "No targets".into());
                let r = self.tile(card_id, slot, &location, false);
                self.ui
                    .rect(Rect::new(r.x + 3.0, r.y + 128.0, r.w - 6.0, 47.0), PANEL);
                self.ui.text(
                    &format!("{} chosen unit(s)", group.len()),
                    r.x + 9.0,
                    r.y + 150.0,
                    10.0,
                    GOLD,
                );
                if self.ui.clicked(r) {
                    self.action(Action::ChooseResolutionGroup { index });
                }
                if self.ui.hover(r) {
                    self.hover_candidate = Some(card_id);
                }
            }
            self.tray_scroll(groups.len(), 6);
            return true;
        }
        if let Decision::OrderTriggers { options, .. } = decision {
            self.ui.text("Choose next trigger", 780.0, 671.0, 9.0, GOLD);
            self.hand_scroll = self.hand_scroll.min(options.len().saturating_sub(6));
            for (slot, option) in options.iter().skip(self.hand_scroll).take(6).enumerate() {
                let r = self.tile(
                    option.card_id,
                    slot,
                    self.game.catalog[option.card_id].name,
                    false,
                );
                self.ui
                    .rect(Rect::new(r.x + 3.0, r.y + 128.0, r.w - 6.0, 47.0), PANEL);
                self.ui
                    .wrapped_lines(&option.label, r.x + 9.0, r.y + 141.0, 122.0, 9.0, GOLD, 3);
                if self.ui.hover(r) {
                    self.hover_candidate = Some(option.card_id);
                }
                if self.ui.clicked(r) {
                    self.action(Action::OrderTrigger {
                        trigger_id: option.id,
                    });
                }
            }
            self.tray_scroll(options.len(), 6);
            return true;
        }
        if let Decision::DeathReplacement { player, options } = decision {
            self.ui
                .text("Choose a replacement", 780.0, 671.0, 9.0, GOLD);
            self.hand_scroll = self.hand_scroll.min(options.len().saturating_sub(6));
            for (slot, option) in options.iter().skip(self.hand_scroll).take(6).enumerate() {
                let Some(unit) = self.game.players[player]
                    .units
                    .iter()
                    .find(|unit| unit.id == option.unit_id)
                else {
                    continue;
                };
                let card_id = unit.card_id;
                let name = self.game.unit_name(unit);
                let label = option
                    .gear_id
                    .and_then(|id| {
                        self.game.players[player]
                            .gear
                            .iter()
                            .find(|gear| gear.id == id)
                    })
                    .map(|gear| self.game.gear_name(gear).to_owned())
                    .unwrap_or_else(|| "Banish instead".into());
                let r = self.tile(card_id, slot, &name, false);
                self.ui
                    .rect(Rect::new(r.x + 3.0, r.y + 128.0, r.w - 6.0, 47.0), PANEL);
                self.ui
                    .wrapped_lines(&label, r.x + 9.0, r.y + 145.0, 122.0, 10.0, GOLD, 2);
                if self.ui.hover(r) {
                    self.hover_candidate = Some(card_id);
                }
                if self.ui.clicked(r) {
                    self.action(Action::ChooseDeathReplacement {
                        unit_id: option.unit_id,
                        gear_id: option.gear_id,
                    });
                }
            }
            self.tray_scroll(options.len(), 6);
            return true;
        }
        let cards = match &decision {
            Decision::Discard { .. } => self.game.players[0].hand.clone(),
            Decision::TopThree { cards, .. } | Decision::RevealHand { cards, .. } => cards.clone(),
            Decision::Sabotage { opponent, .. } => self.game.players[*opponent].hand.clone(),
            Decision::ReviveUnit { .. }
            | Decision::GrantFlow { .. }
            | Decision::LastRites { .. }
            | Decision::ReplaySpell { .. } => self.game.players[0].discard.clone(),
            _ => return false,
        };
        self.ui.text("Choose a card below", 780.0, 671.0, 9.0, GOLD);
        self.hand_scroll = self.hand_scroll.min(cards.len().saturating_sub(6));
        for (slot, (index, id)) in cards
            .iter()
            .enumerate()
            .skip(self.hand_scroll)
            .take(6)
            .enumerate()
        {
            let r = self.tile(
                *id,
                slot,
                self.game.catalog[*id].name,
                self.choices.contains(&index),
            );
            if self.ui.clicked(r) {
                match decision {
                    Decision::RevealHand { .. } => self.inspect = Some(*id),
                    Decision::Discard { count, .. } => {
                        if let Some(i) = self.choices.iter().position(|x| *x == index) {
                            self.choices.remove(i);
                        } else if self.choices.len() < count as usize {
                            self.choices.push(index);
                        }
                    }
                    Decision::ReviveUnit { .. }
                    | Decision::LastRites { .. }
                    | Decision::ReplaySpell { .. } => {
                        self.selection = Some(Selection::Trash(index));
                        self.targets.clear();
                        self.destination = None;
                    }
                    _ => self.action(Action::SelectCards {
                        indices: vec![index],
                    }),
                }
            }
            if self.ui.hover(r) {
                self.hover_candidate = Some(*id);
            }
            if self.ui.hover(r) && self.ui.secondary_pressed() {
                self.inspect = Some(*id);
            }
        }
        self.tray_scroll(cards.len(), 6);
        true
    }
    fn legend_lesson(&mut self, deck: usize) {
        self.ui.rect(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            Color::new(0.0, 0.015, 0.02, 0.9),
        );
        self.ui.enabled = true;
        self.ui.rect(Rect::new(350.0, 140.0, 840.0, 623.0), PANEL);
        self.ui.border(Rect::new(350.0, 140.0, 840.0, 623.0), LINE);
        let p = &self.game.presets[deck];
        self.ui
            .pill(380.0, 170.0, "OFFICIAL LEGEND & STRATEGY", GOLD);
        self.ui.title(p.legend_name, 380.0, 237.0, 31.0, TEXT);
        let y = self
            .ui
            .wrapped(p.legend_text, 380.0, 282.0, 775.0, 14.0, TEXT);
        self.ui
            .text("THE REAL DECK'S PLAN", 380.0, y + 35.0, 11.0, TEAL);
        self.ui
            .wrapped(p.learning_notes, 380.0, y + 66.0, 775.0, 13.0, MUTED);
        self.ui.wrapped("These are the legend powers used in this client. Check individual card support for remaining differences; a simple AI is practice for decisions, not a competitive metagame benchmark.",380.0,644.0,770.0,11.0,GOLD);
        if self.ui.button(
            Rect::new(380.0, 701.0, 780.0, 38.0),
            "Back to the deck library",
            true,
            true,
        ) {
            self.lesson = None;
        }
    }
    fn setup_panel(&mut self) {
        self.ui.rect(Rect::new(1158.0, 108.0, 252.0, 533.0), PANEL);
        self.ui.border(Rect::new(1158.0, 108.0, 252.0, 533.0), LINE);
        self.ui
            .text("MAKE IT YOUR MATCH", 1178.0, 139.0, 10.0, GOLD);
        self.ui
            .title("Your pace. Your play.", 1178.0, 171.0, 21.0, TEXT);
        self.ui.wrapped(
            "Learn the rhythm of Riftbound with a patient AI sparring partner.",
            1178.0,
            198.0,
            207.0,
            12.0,
            MUTED,
        );
        self.ui
            .line(vec2(1178.0, 252.0), vec2(1390.0, 252.0), 1.0, LINE);
        self.ui.text("OPPONENT", 1178.0, 278.0, 10.0, MUTED);
        if self.ui.button(
            Rect::new(1178.0, 294.0, 101.0, 36.0),
            "Relaxed",
            !self.prefs.tactical,
            true,
        ) {
            self.prefs.tactical = false;
            self.prefs.save();
        }
        if self.ui.button(
            Rect::new(1287.0, 294.0, 103.0, 36.0),
            "Tactical",
            self.prefs.tactical,
            true,
        ) {
            self.prefs.tactical = true;
            self.prefs.save();
        }
        self.ui.wrapped(
            if self.prefs.tactical {
                "Plans trades and protects points. A little more room to grow."
            } else {
                "Simple, forgiving choices. Perfect for your first few matches."
            },
            1178.0,
            352.0,
            210.0,
            11.0,
            MUTED,
        );
        self.ui
            .line(vec2(1178.0, 402.0), vec2(1390.0, 402.0), 1.0, LINE);
        self.ui
            .text("THE GOAL IS SIMPLE", 1178.0, 428.0, 10.0, GOLD);
        for (i, s) in [
            "Play units using your runes.",
            "Conquer and hold battlefields.",
            "Be the first to reach 8 points.",
        ]
        .iter()
        .enumerate()
        {
            let y = 455.0 + i as f32 * 29.0;
            self.ui.text(&format!("0{}", i + 1), 1178.0, y, 10.0, TEAL);
            self.ui.text(s, 1202.0, y, 10.5, TEXT);
        }
        if self.ui.button(
            Rect::new(1178.0, 533.0, 212.0, 30.0),
            &format!(
                "First turn: {}   >",
                ["You", "Opponent", "Random"][self.starting_player]
            ),
            false,
            true,
        ) {
            self.starting_player = (self.starting_player + 1) % 3;
        }
        self.ui.text(
            "Check the guide for current rules limits.",
            1178.0,
            581.0,
            9.0,
            MUTED,
        );
        if self.ui.button(
            Rect::new(1178.0, 594.0, 212.0, 30.0),
            "Open beginner's guide  >",
            false,
            true,
        ) {
            self.screen = Screen::Guide;
        }
    }
    fn matchup(&mut self) {
        self.ui
            .text("CHOOSE YOUR MATCHUP", 245.0, 674.0, 10.0, MUTED);
        self.ui.text(
            "4 tournament presets · support shown per card",
            1100.0,
            674.0,
            10.0,
            MUTED,
        );
        for side in 0..2 {
            let x = 245.0 + side as f32 * 594.0;
            let deck = if side == 0 {
                self.player_deck
            } else {
                self.opponent_deck
            };
            let r = Rect::new(x, 693.0, 570.0, 163.0);
            self.ui.rect(r, PANEL);
            self.champion_art(
                deck,
                Rect::new(x, 693.0, 205.0, 163.0),
                Color::new(0.72, 0.80, 0.78, 1.0),
            );
            for n in 0..50 {
                self.ui.rect(
                    Rect::new(x + 155.0 + n as f32, 693.0, 1.0, 163.0),
                    Color::new(PANEL.r, PANEL.g, PANEL.b, n as f32 / 50.0),
                );
            }
            let p = &self.game.presets[deck];
            self.ui.text(
                if side == 0 {
                    "YOU PLAY AS"
                } else {
                    "YOUR AI OPPONENT"
                },
                x + 218.0,
                718.0,
                10.0,
                if side == 0 { TEAL } else { GOLD },
            );
            self.ui.title(
                p.champion.split(',').next().unwrap_or(p.champion),
                x + 218.0,
                751.0,
                26.0,
                TEXT,
            );
            self.ui
                .ellipsis(p.name, x + 218.0, 775.0, 330.0, 12.0, MUTED);
            self.ui.border(r, LINE);
            if self.ui.button(
                Rect::new(x + 218.0, 796.0, 168.0, 36.0),
                "Change deck  >",
                false,
                true,
            ) {
                self.browse_deck = deck;
                self.screen = Screen::Decks;
                self.deck_scroll = 0;
            }
            if self
                .ui
                .button(Rect::new(x + 402.0, 796.0, 60.0, 36.0), "<", false, true)
            {
                let i = (deck + self.game.presets.len() - 1) % self.game.presets.len();
                if side == 0 {
                    self.player_deck = i;
                    self.fields[0] = local_rift::battlefields::default_battlefield(i);
                } else {
                    self.opponent_deck = i;
                    self.fields[1] = local_rift::battlefields::default_battlefield(i);
                }
            }
            if self
                .ui
                .button(Rect::new(x + 470.0, 796.0, 60.0, 36.0), ">", false, true)
            {
                let i = (deck + 1) % self.game.presets.len();
                if side == 0 {
                    self.player_deck = i;
                    self.fields[0] = local_rift::battlefields::default_battlefield(i);
                } else {
                    self.opponent_deck = i;
                    self.fields[1] = local_rift::battlefields::default_battlefield(i);
                }
            }
        }
        for side in 0..2 {
            let x = 245.0 + side as f32 * 594.0;
            let name = local_rift::battlefields::battlefield(self.fields[side]).name;
            if self.ui.button(
                Rect::new(x, 864.0, 570.0, 27.0),
                &format!("Battlefield: {name}   >"),
                false,
                true,
            ) {
                self.fields[side] =
                    (self.fields[side] + 1) % local_rift::battlefields::BATTLEFIELDS.len();
            }
        }
    }
    fn draw_playmat(&mut self, hits: &scene::SceneHits) {
        for owner in 0..2 {
            let rail = Rect::new(245.0, if owner == 1 { 110.0 } else { 571.0 }, 883.0, 66.0);
            self.ui.rune_rail(
                rail,
                if owner == 0 {
                    "YOUR RUNE ZONE"
                } else {
                    "OPPONENT RUNE ZONE"
                },
                &self.reserve(owner),
            );
            if owner == 0 {
                for (index, slot) in self.reserve(0).slots.iter().enumerate() {
                    if let Some(slot) = slot
                        && self.ui.clicked(Rect::new(
                            rail.x + 169.0 + index as f32 * 33.0,
                            rail.y + 18.0,
                            32.0,
                            34.0,
                        ))
                    {
                        self.rune_menu = Some(slot.domain);
                        self.rune_menu_at =
                            vec2((self.ui.mouse().x - 123.0).clamp(245.0, 898.0), 395.0);
                    }
                }
            }
            let base = self.ui.logical(hits.bases[owner]);
            self.ui.rect(base, Color::new(0.035, 0.063, 0.075, 1.0));
            self.ui.border(base, if owner == 0 { TEAL } else { GOLD });
            self.ui.bold(
                if owner == 0 { "YOUR BASE" } else { "AI BASE" },
                base.x + 11.0,
                base.y + 20.0,
                10.0,
                if owner == 0 { TEAL } else { GOLD },
            );
            let count = self.game.players[owner]
                .units
                .iter()
                .filter(|unit| unit.lane.is_none())
                .count();
            self.ui.text(
                &format!("{count} units"),
                base.x + 11.0,
                base.y + 39.0,
                9.0,
                MUTED,
            );
            if count > 10 {
                self.ui
                    .text("More in Board >", base.x + 11.0, base.y + 58.0, 8.0, GOLD);
            }
        }
        for lane in 0..2 {
            let r = self.ui.logical(hits.battlefields[lane]);
            let field_id = if self.in_match {
                self.game.battlefields[lane]
            } else {
                self.fields[lane]
            };
            let field = local_rift::battlefields::battlefield(field_id);
            self.ui.rect(r, Color::new(0.045, 0.075, 0.083, 1.0));
            self.ui.border(
                r,
                match self.game.battlefield_owner[lane] {
                    Some(0) => TEAL,
                    Some(_) => GOLD,
                    None => LINE,
                },
            );
            self.ui.bold(
                &format!("{} · {}", lane + 1, field.name),
                r.x + 12.0,
                r.y + 19.0,
                13.0,
                TEXT,
            );
            self.ui.wrapped(
                field.official_text,
                r.x + 12.0,
                r.y + 39.0,
                r.w - 36.0,
                12.0,
                TEXT,
            );
            self.ui.line(
                vec2(r.x + 9.0, r.y + 83.0),
                vec2(r.x + r.w - 9.0, r.y + 83.0),
                1.0,
                LINE,
            );
            self.ui.text("AI", r.x + 11.0, r.y + 113.0, 9.0, GOLD);
            self.ui.text("YOU", r.x + 8.0, r.y + 189.0, 9.0, TEAL);
            self.ui.line(
                vec2(r.x + 9.0, r.y + 150.0),
                vec2(r.x + r.w - 9.0, r.y + 150.0),
                1.0,
                LINE,
            );
            if self.ui.hover(Rect::new(r.x, r.y, r.w, 69.0)) {
                self.field_hover = Some(field_id);
            }
            if self.ui.button(
                Rect::new(r.x + r.w - 27.0, r.y + 7.0, 20.0, 20.0),
                "i",
                false,
                true,
            ) || (self.ui.hover(Rect::new(r.x, r.y, r.w, 69.0)) && self.ui.secondary_pressed())
            {
                self.field_inspect = Some(field_id);
            }
            for owner in 0..2 {
                let count = self.game.lane_count(owner, lane);
                if count > 5 {
                    self.ui.text(
                        &format!("+{} in Board", count - 5),
                        r.x + 5.0,
                        r.y + if owner == 1 { 140.0 } else { 222.0 },
                        8.0,
                        GOLD,
                    );
                }
            }
        }
    }
    fn battlefield_help(&mut self) {
        let pinned = self.field_inspect.is_some();
        let Some(id) = self.field_inspect.or(self
            .field_hover
            .filter(|_| self.hover_card.is_none() && self.drag.is_none()))
        else {
            return;
        };
        if self.inspect.is_some() || self.rune_menu.is_some() {
            return;
        }
        let field = local_rift::battlefields::battlefield(id);
        let r = Rect::new(424.0, 292.0, 540.0, 240.0);
        if pinned {
            self.ui.rect(
                Rect::new(0.0, 0.0, 1440.0, 900.0),
                Color::new(0.0, 0.02, 0.03, 0.78),
            );
        }
        self.ui.rect(r, PANEL);
        self.ui.border(r, GOLD);
        self.ui
            .text("BATTLEFIELD", r.x + 22.0, r.y + 25.0, 10.0, GOLD);
        self.ui
            .title(field.name, r.x + 22.0, r.y + 62.0, 26.0, TEXT);
        self.ui.wrapped(
            field.official_text,
            r.x + 22.0,
            r.y + 96.0,
            r.w - 44.0,
            14.0,
            TEXT,
        );
        self.ui.text(
            "Conquer and hold this location to score points.",
            r.x + 22.0,
            r.y + 179.0,
            11.0,
            MUTED,
        );
        if pinned {
            self.ui.enabled = true;
            if self.ui.button(
                Rect::new(r.x + 22.0, r.y + 195.0, r.w - 44.0, 30.0),
                "Back to the mat",
                true,
                true,
            ) {
                self.field_inspect = None;
            }
        } else {
            self.ui.text(
                "Click the i to keep this open.",
                r.x + 22.0,
                r.y + 214.0,
                10.0,
                TEAL,
            );
        }
    }
    fn gear_badges(&mut self, r: Rect, owner: usize, unit_id: u32) -> bool {
        let gears: Vec<_> = self.game.players[owner]
            .gear
            .iter()
            .filter(|gear| gear.attached_to == Some(unit_id))
            .cloned()
            .collect();
        let mut hovered = false;
        for (index, gear) in gears.iter().enumerate() {
            let chip = Rect::new(
                r.x + 2.0,
                r.y + r.h - 35.0 - index as f32 * 14.0,
                r.w - 4.0,
                14.0,
            );
            self.ui.rect(chip, Color::new(0.11, 0.13, 0.09, 0.98));
            self.ui.border(chip, GOLD);
            let name = self.game.gear_name(gear);
            self.ui
                .ellipsis(name, chip.x + 3.0, chip.y + 11.0, chip.w - 6.0, 9.0, GOLD);
            if self.ui.hover(chip) {
                self.hover_candidate = Some(gear.card_id);
                self.hover_token_candidate = None;
                hovered = true;
            }
            if self.ui.clicked(chip) || (self.ui.hover(chip) && self.ui.secondary_pressed()) {
                self.inspect = Some(gear.card_id);
            }
        }
        hovered
    }
    fn reserve(&self, player: usize) -> RuneReserve {
        let state = &self.game.players[player];
        RuneReserve::new(
            state.runes,
            state.exhausted_runes,
            state.power_pool,
            state.any_power,
            self.game.legend[player].spell_power,
            state.energy_pool,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn board_card(
        &self,
        r: Rect,
        id: usize,
        owner: usize,
        ready: bool,
        might: u8,
        damage: u8,
        token: bool,
        name: &str,
        selected: bool,
        targeted: bool,
    ) {
        let color = if owner == 0 { TEAL } else { GOLD };
        self.ui.rect(
            Rect::new(r.x + 3.0, r.y + 5.0, r.w, r.h),
            Color::new(0.0, 0.0, 0.0, 0.5),
        );
        self.ui.rect(r, PANEL);
        let art = Rect::new(r.x + 2.0, r.y + 3.0, r.w - 4.0, r.h - 23.0);
        if !token && let Some(tex) = self.art.get(id).and_then(Option::as_ref) {
            self.ui.image(
                tex,
                art,
                Some(Rect::new(
                    tex.width() * 0.08,
                    tex.height() * 0.12,
                    tex.width() * 0.84,
                    tex.height() * 0.43,
                )),
                WHITE,
            );
        } else {
            self.ui.rect(art, Color::new(0.08, 0.20, 0.23, 1.0));
            self.ui
                .diamond(vec2(r.x + r.w / 2.0, r.y + 30.0), 16.0, color);
            self.ui.text("TOKEN", r.x + 9.0, r.y + 53.0, 8.0, TEXT);
        }
        self.ui
            .rect(Rect::new(r.x + 2.0, r.y + 3.0, r.w - 4.0, 3.0), color);
        self.ui
            .rect(Rect::new(r.x + 3.0, r.y + 8.0, 18.0, 19.0), BG);
        self.ui
            .bold(&might.to_string(), r.x + 7.0, r.y + 22.0, 13.0, TEXT);
        if damage > 0 {
            self.ui.pill(
                r.x + r.w - 27.0,
                r.y + 9.0,
                &format!("-{damage}"),
                Color::new(0.93, 0.43, 0.38, 1.0),
            );
        }
        self.ui.ellipsis(
            name.split(',').next().unwrap_or(name),
            r.x + 4.0,
            r.y + r.h - 10.0,
            r.w - 8.0,
            9.0,
            TEXT,
        );
        self.ui.rect(
            Rect::new(r.x + 3.0, r.y + r.h - 6.0, r.w - 6.0, 3.0),
            if ready { color } else { LINE },
        );
        if ready {
            self.ui
                .diamond(vec2(r.x + r.w - 9.0, r.y + 14.0), 3.5, TEAL);
        } else {
            self.ui.line(
                vec2(r.x + r.w - 14.0, r.y + 14.0),
                vec2(r.x + r.w - 5.0, r.y + 14.0),
                2.0,
                GOLD,
            );
            self.ui.rect(
                Rect::new(r.x + 2.0, r.y + art.h - 8.0, r.w - 4.0, 11.0),
                Color::new(0.03, 0.06, 0.07, 0.9),
            );
            self.ui
                .text("EXHAUSTED", r.x + 5.0, r.y + art.h + 0.5, 6.7, GOLD);
        }
        self.ui.border(
            r,
            if targeted {
                Color::new(0.96, 0.65, 0.38, 1.0)
            } else if selected {
                GOLD
            } else {
                color
            },
        );
        if selected || targeted {
            self.ui
                .border(Rect::new(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0), GOLD);
        }
    }
    fn arm_drag(&mut self, source: DragSource) {
        if self.ui.primary_down() {
            self.drag = Some(DragState {
                source,
                origin: self.ui.mouse(),
                active: false,
                previous_selection: self.selection,
                previous_group: self.group.clone(),
            });
        }
    }
    fn cancel_drag(&mut self) {
        if let Some(drag) = self.drag.take() {
            self.selection = drag.previous_selection;
            self.group = drag.previous_group;
        }
    }
    fn drop_action(&self, source: &DragSource, zone: DropZone) -> Result<(Action, bool), String> {
        if self.game.priority() != 0
            || self.game.phase != Phase::Playing
            || self.game.decision.is_some()
        {
            return Err("Wait until you have priority.".into());
        }
        let lane = match zone {
            DropZone::Base => None,
            DropZone::Battlefield(lane) => Some(lane),
            DropZone::Unit(owner, id) => self.game.players[owner]
                .units
                .iter()
                .find(|unit| unit.id == id)
                .and_then(|unit| unit.lane),
            DropZone::Chain(_) => None,
        };
        let (action, confirm) = match source {
            DragSource::Units { ids, .. } => {
                if matches!(zone, DropZone::Chain(_)) {
                    return Err("Drop units on a battlefield or your base.".into());
                }
                (
                    if ids.len() > 1 {
                        Action::MoveGroup {
                            unit_ids: ids.clone(),
                            lane,
                        }
                    } else {
                        Action::Move {
                            unit_id: ids[0],
                            lane,
                        }
                    },
                    false,
                )
            }
            DragSource::Hand { index, card_id } => {
                if self.game.players[0].hand.get(*index) != Some(card_id) {
                    return Err("This card is no longer in your hand.".into());
                }
                use local_rift::rules::OfficialEffect as E;
                let card = self.game.catalog[*card_id];
                let effect = local_rift::rules::effect(*card_id);
                let maximum = match effect {
                    E::Damage { targets, .. } => targets as usize,
                    E::DualMight | E::BounceBoth | E::SwapMight | E::Fight { .. } => 2,
                    E::MoveFriendlyToBase { up_to } => up_to as usize,
                    E::ModifyMight { .. }
                    | E::Bounce { .. }
                    | E::MoveEnemy
                    | E::MoveFriendlyAndReady
                    | E::ReadyAnother
                    | E::BuffAnother => 1,
                    _ => 0,
                };
                let movement = matches!(effect, E::MoveEnemy | E::MoveFriendlyAndReady);
                let mut targets = self.targets.clone();
                match zone {
                    DropZone::Unit(player, unit_id)
                        if maximum > 0 || card.kind == CardKind::Gear =>
                    {
                        let target = Target::Unit { player, unit_id };
                        if !targets.contains(&target) {
                            targets.push(target);
                        }
                    }
                    DropZone::Chain(chain_id) if matches!(effect, E::Counter(_)) => {
                        targets = vec![Target::Spell { chain_id }]
                    }
                    DropZone::Unit(..) if card.kind == CardKind::Spell => {
                        return Err("This spell does not choose a unit.".into());
                    }
                    _ => {}
                }
                if maximum > 0 && targets.is_empty() && card.kind == CardKind::Spell {
                    return Err("Drop this spell on a unit to choose its target.".into());
                }
                if matches!(effect, E::Counter(_)) && !matches!(zone, DropZone::Chain(_)) {
                    return Err("Drop a counterspell on the pending spell above the board.".into());
                }
                let destination = if card.kind == CardKind::Unit {
                    lane
                } else if movement {
                    if matches!(zone, DropZone::Battlefield(_) | DropZone::Base) {
                        lane
                    } else {
                        self.destination
                    }
                } else {
                    None
                };
                let confirm = maximum > 1 || movement || self.options.repeat;
                (
                    Action::PlayCardOptions {
                        hand_index: *index,
                        lane: destination,
                        targets,
                        repeat_targets: self.repeat_targets.clone(),
                        repeat: self.options.repeat,
                        accelerate: self.options.accelerate,
                        bonus: self.options.bonus,
                    },
                    confirm,
                )
            }
        };
        if confirm
            && let Action::PlayCardOptions {
                hand_index,
                lane,
                ref targets,
                ..
            } = action
        {
            let card_id = self.game.players[0].hand[hand_index];
            for (index, target) in targets.iter().enumerate() {
                if matches!(target, Target::Unit { .. })
                    && !self
                        .game
                        .valid_card_target(0, card_id, lane, *target, &targets[..index])
                {
                    return Err("This unit is not a legal target for that card.".into());
                }
            }
            if !targets.is_empty() {
                let mut trial = self.game.clone();
                trial.apply(Action::PlayCard {
                    hand_index,
                    lane,
                    target: targets.first().copied(),
                })?;
                return Ok((action, true));
            }
        }
        let mut trial = self.game.clone();
        trial.apply(action.clone())?;
        Ok((action, confirm))
    }
    fn drag_overlay(&mut self) {
        let Some(mut drag) = self.drag.clone() else {
            return;
        };
        if self.ui.secondary_pressed()
            || self.screen != Screen::Practice
            || self.inspect.is_some()
            || self.rune_menu.is_some()
            || self.confirm_restart
        {
            self.cancel_drag();
            return;
        }
        if self.ui.mouse().distance(drag.origin) > 7.0 && self.ui.primary_down() {
            drag.active = true;
            self.hover_card = None;
            if let DragSource::Units { ids, .. } = &drag.source {
                self.group = ids.clone();
                self.selection = Some(Selection::Unit(ids[0]));
            }
        }
        if !drag.active {
            if self.ui.primary_released() || !self.ui.primary_down() {
                self.drag = None;
            } else {
                self.drag = Some(drag);
            }
            return;
        }
        let mouse = self.ui.mouse();
        let zone = self
            .drop_zones
            .iter()
            .rev()
            .find(|(_, r)| r.contains(mouse))
            .copied();
        let result = zone.map(|(zone, _)| self.drop_action(&drag.source, zone));
        for (zone, r) in &self.drop_zones {
            if matches!(zone, DropZone::Chain(_)) && !matches!(drag.source, DragSource::Hand { .. })
            {
                continue;
            }
            if self.drop_action(&drag.source, *zone).is_ok() {
                self.ui.rect(
                    *r,
                    Color::new(
                        0.30,
                        0.86,
                        0.71,
                        if r.contains(mouse) { 0.12 } else { 0.035 },
                    ),
                );
                self.ui
                    .border(*r, if r.contains(mouse) { GOLD } else { TEAL });
                if !matches!(zone, DropZone::Unit(..)) {
                    self.ui.pill(
                        r.x + 8.0,
                        r.y + r.h - 22.0,
                        match zone {
                            DropZone::Base => "YOUR BASE",
                            DropZone::Battlefield(_) => "DROP HERE",
                            _ => "COUNTER THIS SPELL",
                        },
                        TEAL,
                    );
                }
            }
        }
        if self.ui.primary_released() {
            match result {
                Some(Ok((
                    Action::PlayCardOptions {
                        hand_index,
                        lane,
                        targets,
                        repeat_targets,
                        repeat,
                        accelerate,
                        bonus,
                    },
                    true,
                ))) => {
                    self.drag = None;
                    self.selection = Some(Selection::Hand(hand_index));
                    self.destination = lane;
                    self.targets = targets;
                    self.repeat_targets = repeat_targets;
                    self.options = PlayOptions {
                        repeat,
                        accelerate,
                        bonus,
                    };
                    self.notify("Target selected. Choose any other targets or destination, then press Play.");
                }
                Some(Ok((action, _))) => {
                    self.drag = None;
                    self.action(action);
                }
                Some(Err(reason)) => {
                    self.cancel_drag();
                    self.notify(reason);
                }
                None => {
                    self.cancel_drag();
                    self.notify("Drop cancelled. No cards or resources were spent.");
                }
            }
            return;
        }
        let valid = matches!(result, Some(Ok(_)));
        let card_id = match drag.source {
            DragSource::Hand { card_id, .. } | DragSource::Units { card_id, .. } => card_id,
        };
        let ghost = Rect::new(
            (mouse.x + 14.0).min(1340.0),
            (mouse.y - 55.0).clamp(20.0, 760.0),
            80.0,
            110.0,
        );
        self.ui.rect(ghost, PANEL);
        if let Some(tex) = self.art.get(card_id).and_then(Option::as_ref) {
            self.ui
                .image(tex, ghost, None, Color::new(1.0, 1.0, 1.0, 0.85));
        }
        self.ui.border(ghost, if valid { TEAL } else { GOLD });
        let label = if valid {
            "Release to place · right-click cancels"
        } else {
            "Find a highlighted destination · right-click cancels"
        };
        self.ui.rect(Rect::new(416.0, 603.0, 575.0, 28.0), PANEL);
        self.ui
            .text(label, 430.0, 621.0, 11.0, if valid { TEAL } else { MUTED });
        self.drag = Some(drag);
    }
    fn hover_preview(&mut self) {
        if self.inspect.is_some()
            || self.lesson.is_some()
            || self.show_journal
            || self.confirm_restart
            || self.drag.as_ref().is_some_and(|drag| drag.active)
        {
            self.hover_card = None;
            self.hover_rect = None;
            return;
        }
        let inside = self.hover_rect.is_some_and(|r| r.contains(self.ui.mouse()));
        if !inside
            && (self.hover_candidate != self.hover_card
                || self.hover_token_candidate != self.hover_token)
        {
            self.hover_card = self.hover_candidate;
            self.hover_token = self.hover_token_candidate.clone();
            self.hover_since = get_time();
            self.hover_scroll = 0;
            self.hover_rect = None;
        }
        let Some(id) = self.hover_card else {
            return;
        };
        if get_time() - self.hover_since < 0.3 {
            return;
        }
        let r = *self.hover_rect.get_or_insert_with(|| {
            Rect::new(
                if self.ui.mouse().x > 720.0 {
                    250.0
                } else {
                    776.0
                },
                176.0,
                350.0,
                458.0,
            )
        });
        self.ui.rect(
            Rect::new(r.x + 5.0, r.y + 6.0, r.w, r.h),
            Color::new(0.0, 0.0, 0.0, 0.5),
        );
        self.ui.rect(r, PANEL);
        self.ui.border(r, TEAL);
        if let Some(token) = &self.hover_token {
            self.ui
                .diamond(vec2(r.x + r.w / 2.0, r.y + 65.0), 32.0, TEAL);
            self.ui
                .title(&token.name, r.x + 18.0, r.y + 139.0, 23.0, TEXT);
            self.ui.bold(
                &format!("{} CURRENT MIGHT · CREATED UNIT", token.might),
                r.x + 18.0,
                r.y + 172.0,
                11.0,
                GOLD,
            );
            self.ui.wrapped(
                &token.rules,
                r.x + 18.0,
                r.y + 211.0,
                r.w - 36.0,
                13.0,
                TEXT,
            );
            self.ui.text(
                "Tokens are represented by their own rules.",
                r.x + 18.0,
                r.y + r.h - 24.0,
                10.0,
                MUTED,
            );
            return;
        }
        if let Some(tex) = self.art.get(id).and_then(Option::as_ref) {
            self.ui.image(
                tex,
                Rect::new(r.x + 1.0, r.y + 1.0, r.w - 2.0, 111.0),
                Some(Rect::new(
                    tex.width() * 0.08,
                    tex.height() * 0.12,
                    tex.width() * 0.84,
                    tex.height() * 0.35,
                )),
                WHITE,
            );
        }
        let card = self.game.catalog[id];
        self.ui.rect(
            Rect::new(r.x + 1.0, r.y + 84.0, r.w - 2.0, 65.0),
            Color::new(0.04, 0.07, 0.08, 0.94),
        );
        self.ui
            .wrapped(card.name, r.x + 16.0, r.y + 107.0, r.w - 32.0, 17.0, TEXT);
        self.ui.text(
            &format!(
                "{} · {}",
                card.domains
                    .iter()
                    .map(|domain| domain.name())
                    .collect::<Vec<_>>()
                    .join(" / "),
                card.kind.name()
            ),
            r.x + 16.0,
            r.y + 165.0,
            10.0,
            GOLD,
        );
        self.ui.text(
            &format!(
                "{} energy · {} power{}",
                card.cost,
                card.power_cost,
                if card.kind == CardKind::Unit {
                    format!(" · {} might", card.might)
                } else {
                    String::new()
                }
            ),
            r.x + 16.0,
            r.y + 184.0,
            10.0,
            MUTED,
        );
        let notes = self.game.effect_description(id);
        let text = if notes.is_empty() {
            card.official_text.to_owned()
        } else {
            format!("{}\n\nPractice note\n{}", card.official_text, notes)
        };
        let max = self.ui.scroll_text(
            &text,
            Rect::new(r.x + 16.0, r.y + 199.0, r.w - 32.0, 204.0),
            12.0,
            &mut self.hover_scroll,
        );
        self.ui.line(
            vec2(r.x + 16.0, r.y + 416.0),
            vec2(r.x + r.w - 16.0, r.y + 416.0),
            1.0,
            LINE,
        );
        self.ui.text(
            if max > 0 {
                "Scroll to read all text · right-click to pin"
            } else {
                "Full card text · right-click to pin"
            },
            r.x + 16.0,
            r.y + 438.0,
            10.0,
            MUTED,
        );
        if inside && self.ui.secondary_pressed() {
            self.inspect = Some(id);
            self.inspect_scroll = 0;
        }
    }
    fn rune_popover(&mut self) {
        let Some(domain) = self.rune_menu else {
            return;
        };
        let r = Rect::new(self.rune_menu_at.x, self.rune_menu_at.y, 246.0, 170.0);
        self.ui.enabled = true;
        self.ui.rect(r, PANEL);
        self.ui.border(r, rgb(domain.color()));
        self.ui.bold(
            &format!("{} rune", domain.name()),
            r.x + 16.0,
            r.y + 27.0,
            16.0,
            TEXT,
        );
        self.ui.text(
            "Prepare resources; payment stays automatic.",
            r.x + 16.0,
            r.y + 48.0,
            9.0,
            MUTED,
        );
        for (index, recycle) in [false, true].into_iter().enumerate() {
            let action = Action::ActivateRune { domain, recycle };
            let mut check = self.game.clone();
            let valid = self.game.priority() == 0 && check.apply(action.clone()).is_ok();
            if self.ui.button(
                Rect::new(r.x + 16.0, r.y + 61.0 + index as f32 * 42.0, 214.0, 34.0),
                if recycle {
                    "Recycle → matching power"
                } else {
                    "Exhaust → store 1 energy"
                },
                false,
                valid,
            ) {
                self.rune_menu = None;
                self.action(action);
                break;
            }
        }
        if (self.ui.primary_pressed()
            && !r.contains(self.ui.mouse())
            && !Rect::new(245.0, 571.0, 883.0, 66.0).contains(self.ui.mouse()))
            || self.ui.secondary_pressed()
            || is_key_pressed(KeyCode::Escape)
        {
            self.rune_menu = None;
        }
    }
    fn resource_tooltip(&self) {
        if !self.in_match
            || self.screen != Screen::Practice
            || self.inspect.is_some()
            || self.lesson.is_some()
            || self.show_journal
            || self.confirm_restart
            || self.rune_menu.is_some()
        {
            return;
        }
        let owner = if self.ui.hover(Rect::new(245.0, 571.0, 883.0, 66.0)) {
            0
        } else if self.ui.hover(Rect::new(245.0, 110.0, 883.0, 66.0)) {
            1
        } else {
            return;
        };
        let reserve = self.reserve(owner);
        let r = if owner == 0 {
            Rect::new(826.0, 350.0, 318.0, 212.0)
        } else {
            Rect::new(826.0, 182.0, 318.0, 212.0)
        };
        self.ui.rect(r, Color::new(0.035, 0.07, 0.085, 0.99));
        self.ui.border(r, GOLD);
        self.ui.bold(
            if owner == 0 {
                "YOUR RUNE RESERVE"
            } else {
                "OPPONENT'S RUNE RESERVE"
            },
            r.x + 14.0,
            r.y + 23.0,
            10.0,
            GOLD,
        );
        self.ui.wrapped("Upright runes are ready. Sideways runes are exhausted. Empty sockets represent runes in the rune deck.",r.x+14.0,r.y+46.0,290.0,10.5,TEXT);
        let player = &self.game.players[owner];
        let domains = [
            Domain::Fury,
            Domain::Calm,
            Domain::Mind,
            Domain::Body,
            Domain::Chaos,
            Domain::Order,
        ];
        let summary = domains
            .into_iter()
            .filter_map(|domain| {
                let i = domain_index(domain);
                (player.runes[i] > 0).then(|| {
                    format!(
                        "{} {}/{} ready",
                        domain.name(),
                        player.runes[i].saturating_sub(player.exhausted_runes[i]),
                        player.runes[i]
                    )
                })
            })
            .collect::<Vec<_>>()
            .join(" · ");
        self.ui.text(&summary, r.x + 14.0, r.y + 102.0, 9.5, TEAL);
        self.ui.wrapped("Click a rune to store energy or recycle for matching power. Exhausted runes can still recycle. Payment is automatic.",r.x+14.0,r.y+123.0,290.0,10.0,MUTED);
        self.ui.text(
            &format!(
                "{} in play · {} in deck · {} stored energy",
                reserve.ready + reserve.exhausted,
                reserve.in_deck,
                reserve.stored_energy
            ),
            r.x + 14.0,
            r.y + 176.0,
            9.0,
            GOLD,
        );
        let generated = domains
            .into_iter()
            .filter_map(|domain| {
                let amount = player.power_pool[domain_index(domain)];
                (amount > 0).then(|| format!("{} {amount}", domain.name()))
            })
            .collect::<Vec<_>>()
            .join(" · ");
        self.ui.ellipsis(
            &format!(
                "Generated: {} · Any {}",
                if generated.is_empty() {
                    "none"
                } else {
                    &generated
                },
                reserve.any_power
            ),
            r.x + 14.0,
            r.y + 197.0,
            290.0,
            9.0,
            TEAL,
        );
    }
    fn match_panel(&mut self) {
        self.ui.rect(Rect::new(1158.0, 108.0, 252.0, 770.0), PANEL);
        self.ui.border(Rect::new(1158.0, 108.0, 252.0, 770.0), LINE);
        self.ui.text(
            &format!(
                "TURN {:02}  ·  {} AI",
                self.game.turn,
                if self.prefs.tactical {
                    "TACTICAL"
                } else {
                    "RELAXED"
                }
            ),
            1178.0,
            136.0,
            10.0,
            MUTED,
        );
        self.ui.title(
            &format!("{}", self.game.players[0].score),
            1185.0,
            187.0,
            42.0,
            TEAL,
        );
        self.ui.text("/ 8", 1221.0, 183.0, 15.0, MUTED);
        self.ui.text("YOU", 1185.0, 207.0, 9.0, TEAL);
        self.ui.title(
            &format!("{}", self.game.players[1].score),
            1321.0,
            187.0,
            42.0,
            GOLD,
        );
        self.ui.text("/ 8", 1357.0, 183.0, 15.0, MUTED);
        self.ui.text("OPPONENT", 1314.0, 207.0, 9.0, GOLD);
        self.ui.text("vs", 1275.0, 180.0, 12.0, MUTED);
        self.ui
            .line(vec2(1178.0, 225.0), vec2(1390.0, 225.0), 1.0, LINE);
        if self.game.in_window() {
            self.response_ribbon();
        } else {
            self.ui.text("YOUR LEGEND", 1178.0, 245.0, 9.0, GOLD);
            self.ui.ellipsis(
                self.game.presets[self.player_deck].legend_name,
                1178.0,
                269.0,
                212.0,
                15.0,
                TEXT,
            );
            self.ui.text(
                if self.game.legend[0].ready {
                    "READY"
                } else {
                    "EXHAUSTED"
                },
                1178.0,
                290.0,
                9.0,
                TEAL,
            );
            if self.ui.button(
                Rect::new(1178.0, 306.0, 212.0, 27.0),
                "Legend & gear  >",
                false,
                true,
            ) {
                self.tray = Tray::Loadout;
                self.hand_scroll = 0;
            }
        }
        self.ui
            .line(vec2(1178.0, 345.0), vec2(1390.0, 345.0), 1.0, LINE);
        if self.game.phase == Phase::Mulligan {
            self.ui.title("A good opening.", 1178.0, 366.0, 22.0, TEXT);
            self.ui.wrapped("Choose up to 2 cards to replace, or keep your hand. Your champion starts in its own zone.",1178.0,397.0,208.0,12.0,MUTED);
            if self.ui.button(
                Rect::new(1178.0, 493.0, 212.0, 43.0),
                if self.mulligan.is_empty() {
                    "Keep hand & begin  >"
                } else {
                    "Replace & begin  >"
                },
                true,
                true,
            ) {
                let mut indices = self.mulligan.iter().copied().collect::<Vec<_>>();
                indices.sort_unstable();
                match self.game.mulligan(&indices) {
                    Ok(()) => {
                        if let Err(e) = self.game.start_match() {
                            self.notify(e);
                        }
                        self.mulligan.clear();
                    }
                    Err(e) => self.notify(e),
                }
            }
        } else if self.game.priority() == 1 && self.game.winner.is_none() {
            self.ui.title("Their move.", 1178.0, 366.0, 23.0, TEXT);
            self.ui.wrapped("Watch how your opponent spends runes and moves units. Your next turn will begin automatically.",1178.0,397.0,210.0,12.0,MUTED);
            for i in 0..3 {
                self.ui.circle(
                    vec2(1258.0 + i as f32 * 22.0, 489.0),
                    3.5,
                    Color::new(
                        GOLD.r,
                        GOLD.g,
                        GOLD.b,
                        0.3 + 0.7 * ((get_time() as f32 * 2.0 - i as f32).sin() * 0.5 + 0.5),
                    ),
                );
            }
        } else if self.game.decision.is_some() {
            self.decision_panel();
        } else {
            self.selection_panel();
        }
        if self.game.phase == Phase::Playing
            && self.ui.button(
                Rect::new(1178.0, 574.0, 212.0, 44.0),
                if self.game.chain.is_empty() && self.game.showdown.is_none() {
                    "End turn     [SPACE]"
                } else {
                    "Pass priority  [SPACE]"
                },
                true,
                self.game.priority() == 0 && self.game.decision.is_none(),
            )
        {
            self.pass_or_end();
        }
        self.ui
            .line(vec2(1178.0, 639.0), vec2(1390.0, 639.0), 1.0, LINE);
        if self.ui.button(
            Rect::new(1178.0, 650.0, 212.0, 24.0),
            "MATCH JOURNAL  >",
            false,
            true,
        ) {
            self.show_journal = true;
            self.journal_page = 0;
        }
        let entries: Vec<_> = self.game.log.iter().rev().take(4).collect();
        for (i, line) in entries.iter().enumerate() {
            let y = 691.0 + i as f32 * 34.0;
            self.ui
                .circle(vec2(1181.0, y - 4.0), 2.0, if i == 0 { TEAL } else { LINE });
            self.ui.ellipsis(
                line,
                1192.0,
                y,
                197.0,
                10.0,
                if i == 0 { TEXT } else { MUTED },
            );
        }
        let player = &self.game.players[0];
        self.ui
            .line(vec2(1178.0, 816.0), vec2(1390.0, 816.0), 1.0, LINE);
        self.ui.text(
            &format!(
                "Deck {} · Trash {} · AI hand {} · XP {}",
                player.deck.len(),
                player.discard.len(),
                self.game.players[1].hand.len(),
                player.xp
            ),
            1178.0,
            838.0,
            9.0,
            MUTED,
        );
        self.ui.text(
            "Drag to play · hover to read · right-click to pin",
            1178.0,
            861.0,
            9.0,
            MUTED,
        );
    }
    fn selection_panel(&mut self) {
        match self.selection {
            Some(Selection::Unit(id)) => {
                if let Some(unit) = self.game.players[0].units.iter().find(|u| u.id == id) {
                    let card_id = unit.card_id;
                    let ready = unit.ready;
                    let lane = unit.lane;
                    self.ui
                        .ellipsis(&self.game.unit_name(unit), 1178.0, 359.0, 212.0, 16.0, TEXT);
                    self.ui.text(
                        if ready {
                            "READY TO MOVE"
                        } else {
                            "EXHAUSTED · READY NEXT TURN"
                        },
                        1178.0,
                        383.0,
                        9.0,
                        if ready { TEAL } else { GOLD },
                    );
                    if self.ui.button(
                        Rect::new(1178.0, 403.0, 212.0, 34.0),
                        "Move to field 1",
                        false,
                        ready
                            && (lane.is_none()
                                || self.game.gear_grants_ganking(id)
                                || card_id == 22 && self.game.players[0].xp >= 6)
                            && self.game.chain.is_empty()
                            && self.game.showdown.is_none(),
                    ) {
                        self.move_selection(id, Some(0));
                    }
                    if self.ui.button(
                        Rect::new(1178.0, 445.0, 212.0, 34.0),
                        "Move to field 2",
                        false,
                        ready
                            && (lane.is_none()
                                || self.game.gear_grants_ganking(id)
                                || card_id == 22 && self.game.players[0].xp >= 6)
                            && self.game.chain.is_empty()
                            && self.game.showdown.is_none(),
                    ) {
                        self.move_selection(id, Some(1));
                    }
                    if self.ui.button(
                        Rect::new(1178.0, 487.0, 212.0, 34.0),
                        "Return to base",
                        false,
                        ready
                            && lane.is_some()
                            && self.game.chain.is_empty()
                            && self.game.showdown.is_none(),
                    ) {
                        self.move_selection(id, None);
                    }
                    let group_available = self.game.players[0]
                        .units
                        .iter()
                        .filter(|u| u.ready && u.lane == lane)
                        .map(|u| u.id)
                        .collect::<Vec<_>>();
                    if matches!(card_id, 24 | 45) {
                        if self.ui.button(
                            Rect::new(1178.0, 533.0, 102.0, 27.0),
                            "Empower",
                            false,
                            true,
                        ) {
                            self.action(Action::EmpowerUnit {
                                unit_id: id,
                                pay_power: false,
                            });
                        }
                        if card_id == 24
                            && self.ui.button(
                                Rect::new(1288.0, 533.0, 102.0, 27.0),
                                "Use power",
                                false,
                                true,
                            )
                        {
                            self.action(Action::EmpowerUnit {
                                unit_id: id,
                                pay_power: true,
                            });
                        }
                    } else {
                        if self.ui.button(
                            Rect::new(1178.0, 533.0, 212.0, 27.0),
                            &if self.group.len() > 1 {
                                format!("{} units grouped · clear", self.group.len())
                            } else {
                                format!("Group all {} ready here", group_available.len())
                            },
                            false,
                            group_available.len() > 1
                                && self.game.showdown.is_none()
                                && self.game.chain.is_empty(),
                        ) {
                            if self.group.len() > 1 {
                                self.group.clear();
                            } else {
                                self.group = group_available;
                            }
                        }
                    }
                } else {
                    self.selection = None;
                }
            }
            selection @ (Some(Selection::Hand(_))
            | Some(Selection::Trash(_))
            | Some(Selection::Champion)) => {
                let index = match selection {
                    Some(Selection::Hand(i)) | Some(Selection::Trash(i)) => i,
                    _ => 0,
                };
                let champion = self.selection == Some(Selection::Champion);
                let from_trash = matches!(self.selection, Some(Selection::Trash(_)));
                let card_id = if from_trash {
                    self.game.players[0].discard.get(index).copied()
                } else if champion {
                    self.game.players[0].champion
                } else {
                    self.game.players[0].hand.get(index).copied()
                };
                if let Some(card_id) = card_id {
                    let card = self.game.catalog[card_id];
                    self.ui
                        .ellipsis(card.name, 1178.0, 359.0, 212.0, 16.0, TEXT);
                    self.ui.text(
                        &format!("{} ENERGY · {} POWER", card.cost, card.power_cost),
                        1178.0,
                        383.0,
                        9.0,
                        rgb(card.domain.color()),
                    );
                    self.ui.ellipsis(
                        &card_brief(&self.game, card_id),
                        1178.0,
                        406.0,
                        209.0,
                        10.0,
                        MUTED,
                    );
                    self.ui.text(
                        "Hover the card to read its full text.",
                        1178.0,
                        423.0,
                        9.0,
                        MUTED,
                    );
                    let discounts = self.game.players[0]
                        .units
                        .iter()
                        .filter(|unit| unit.card_id == 35 && !unit.token)
                        .count()
                        .min(255) as u8;
                    if discounts > 0 && matches!(card_id, 29 | 37 | 49 | 50 | 64) {
                        let r = Rect::new(1178.0, 432.0, 212.0, 18.0);
                        let power = self.game.optional_power_discount[0].min(discounts);
                        self.ui.border(r, LINE);
                        self.ui.text(
                            &format!("Discount: {power} power · {} energy  >", discounts - power),
                            1184.0,
                            445.0,
                            9.0,
                            TEAL,
                        );
                        if self.ui.clicked(r) {
                            self.action(Action::SetOptionalDiscount {
                                power: (power + 1) % (discounts + 1),
                            });
                        }
                    }
                    {
                        if matches!(card_id, 37 | 50)
                            && self.ui.button(
                                Rect::new(1178.0, 454.0, 102.0, 28.0),
                                "Repeat",
                                self.options.repeat,
                                true,
                            )
                        {
                            self.options.repeat = !self.options.repeat;
                            self.editing_repeat = false;
                        }
                        if self.options.repeat
                            && matches!(card_id, 37 | 50)
                            && self.ui.button(
                                Rect::new(1288.0, 454.0, 102.0, 28.0),
                                if self.editing_repeat {
                                    "Repeat targets"
                                } else {
                                    "First targets"
                                },
                                self.editing_repeat,
                                true,
                            )
                        {
                            self.editing_repeat = !self.editing_repeat;
                        }
                        if matches!(card_id, 49 | 64)
                            && self.ui.button(
                                Rect::new(1178.0, 454.0, 102.0, 28.0),
                                "Accelerate",
                                self.options.accelerate,
                                true,
                            )
                        {
                            self.options.accelerate = !self.options.accelerate;
                        }
                        if card_id == 29
                            && self.ui.button(
                                Rect::new(1178.0, 454.0, 102.0, 28.0),
                                "Pay +2 buff",
                                self.options.bonus,
                                true,
                            )
                        {
                            self.options.bonus = !self.options.bonus;
                        }
                        if !from_trash && card.official_text.starts_with("[Hidden]") {
                            for lane in 0..2 {
                                let valid = if champion {
                                    self.game.hidden_can_hide_champion(0, lane)
                                } else {
                                    self.game.hidden_can_hide(0, index, lane)
                                }
                                .is_ok();
                                if self.ui.button(
                                    Rect::new(1178.0 + lane as f32 * 110.0, 454.0, 102.0, 28.0),
                                    &format!("Hide at {}", lane + 1),
                                    false,
                                    valid,
                                ) {
                                    self.action(if champion {
                                        Action::HideChampion { lane }
                                    } else {
                                        Action::Hide {
                                            hand_index: index,
                                            lane,
                                        }
                                    });
                                }
                            }
                        }
                    }
                    let affordable = (from_trash || self.game.can_afford(0, &card))
                        && self.game.card_supported(card_id);
                    let label = if card.kind == CardKind::Unit {
                        "Deploy to base"
                    } else {
                        if self.targets.is_empty() {
                            "Play card"
                        } else {
                            "Play on chosen targets"
                        }
                    };
                    if self.ui.button(
                        Rect::new(1178.0, 493.0, 212.0, 34.0),
                        label,
                        false,
                        affordable,
                    ) {
                        let action = if from_trash {
                            Action::PlayFromTrashOptions {
                                trash_index: index,
                                lane: self.destination,
                                targets: self.targets.clone(),
                                repeat_targets: self.repeat_targets.clone(),
                                repeat: self.options.repeat,
                                accelerate: self.options.accelerate,
                                bonus: self.options.bonus,
                            }
                        } else if champion {
                            Action::PlayChampionOptions {
                                lane: self.destination,
                                target: self.targets.first().copied(),
                                accelerate: self.options.accelerate,
                            }
                        } else {
                            Action::PlayCardOptions {
                                hand_index: index,
                                lane: self.destination,
                                targets: self.targets.clone(),
                                repeat_targets: self.repeat_targets.clone(),
                                repeat: self.options.repeat,
                                accelerate: self.options.accelerate,
                                bonus: self.options.bonus,
                            }
                        };
                        self.action(action);
                    }
                    if !affordable {
                        self.ui.text(
                            if !self.game.card_supported(card_id) {
                                "This card's effect is not implemented."
                            } else {
                                "Need more energy or matching power."
                            },
                            1178.0,
                            548.0,
                            9.0,
                            GOLD,
                        );
                    } else if card.kind == CardKind::Unit {
                        for lane in 0..2 {
                            let action = if champion {
                                Action::PlayChampionOptions {
                                    lane: Some(lane),
                                    target: self.targets.first().copied(),
                                    accelerate: self.options.accelerate,
                                }
                            } else if from_trash {
                                Action::PlayFromTrashOptions {
                                    trash_index: index,
                                    lane: Some(lane),
                                    targets: self.targets.clone(),
                                    repeat_targets: self.repeat_targets.clone(),
                                    repeat: self.options.repeat,
                                    accelerate: self.options.accelerate,
                                    bonus: self.options.bonus,
                                }
                            } else {
                                Action::PlayCardOptions {
                                    hand_index: index,
                                    lane: Some(lane),
                                    targets: self.targets.clone(),
                                    repeat_targets: self.repeat_targets.clone(),
                                    repeat: self.options.repeat,
                                    accelerate: self.options.accelerate,
                                    bonus: self.options.bonus,
                                }
                            };
                            if self.ui.button(
                                Rect::new(1178.0 + lane as f32 * 110.0, 534.0, 102.0, 27.0),
                                if lane == 0 {
                                    "To field 1"
                                } else {
                                    "To field 2"
                                },
                                false,
                                self.game.battlefield_owner[lane] == Some(0)
                                    || card.official_text.contains("[Ambush]"),
                            ) {
                                self.action(action);
                            }
                        }
                    } else if card.kind != CardKind::Unit {
                        self.ui.text(
                            &format!(
                                "Targets {} · repeat {} · click units",
                                self.targets.len(),
                                self.repeat_targets.len()
                            ),
                            1178.0,
                            548.0,
                            9.0,
                            TEAL,
                        );
                    }
                } else {
                    self.selection = None;
                }
            }
            Some(Selection::Hidden(id)) => {
                if let Some(card) = self.game.players[0]
                    .hidden
                    .iter()
                    .find(|h| h.id == id)
                    .cloned()
                {
                    self.ui.ellipsis(
                        self.game.catalog[card.card_id].name,
                        1178.0,
                        359.0,
                        210.0,
                        16.0,
                        TEXT,
                    );
                    self.ui.wrapped_lines(
                        self.game.catalog[card.card_id].official_text,
                        1178.0,
                        391.0,
                        210.0,
                        10.0,
                        MUTED,
                        7,
                    );
                    if self.ui.button(
                        Rect::new(1178.0, 516.0, 212.0, 38.0),
                        "Reveal & play for free",
                        false,
                        self.game.hidden_can_play(0, id, &self.targets).is_ok(),
                    ) {
                        self.action(Action::PlayHidden {
                            hidden_id: id,
                            targets: self.targets.clone(),
                        });
                    }
                } else {
                    self.selection = None;
                }
            }
            Some(Selection::Legend) => {
                let preset = &self.game.presets[self.player_deck];
                self.ui.title("Your legend", 1178.0, 363.0, 22.0, TEXT);
                self.ui
                    .wrapped_lines(preset.legend_text, 1178.0, 392.0, 210.0, 11.0, MUTED, 8);
                let legend = &self.game.legend[0];
                self.ui.text(
                    if legend.ready { "READY" } else { "EXHAUSTED" },
                    1178.0,
                    526.0,
                    9.0,
                    TEAL,
                );
                if self.ui.button(
                    Rect::new(1178.0, 534.0, 212.0, 28.0),
                    "Use legend power",
                    false,
                    self.player_deck == 2 || self.player_deck == 3,
                ) {
                    self.action(Action::ActivateLegend {
                        target: self.targets.first().copied(),
                    });
                }
            }
            Some(Selection::Gear(id)) => {
                if let Some(gear) = self.game.players[0]
                    .gear
                    .iter()
                    .find(|g| g.id == id)
                    .cloned()
                {
                    self.ui
                        .ellipsis(self.game.gear_name(&gear), 1178.0, 359.0, 210.0, 16.0, TEXT);
                    self.ui.wrapped_lines(
                        self.game.catalog[gear.card_id].official_text,
                        1178.0,
                        391.0,
                        210.0,
                        10.0,
                        MUTED,
                        6,
                    );
                    if self.game.gear_can_activate(0, id) {
                        if self.ui.button(
                            Rect::new(1178.0, 497.0, 212.0, 34.0),
                            "Activate gear",
                            false,
                            true,
                        ) {
                            self.action(Action::ActivateGear { gear_id: id });
                        }
                    } else {
                        self.ui.text(
                            "Click a friendly unit, then attach.",
                            1178.0,
                            490.0,
                            9.0,
                            TEAL,
                        );
                        if self.ui.button(
                            Rect::new(1178.0, 505.0, 212.0, 34.0),
                            if gear.card_id == 38 {
                                "Attach · recycle selected 2"
                            } else {
                                "Attach to chosen unit"
                            },
                            false,
                            !self.targets.is_empty(),
                        ) && let Some(target) = self.targets.first().copied()
                        {
                            self.action(if gear.card_id == 38 {
                                Action::EquipWithTrash {
                                    gear_id: id,
                                    target,
                                    trash_indices: self.choices.clone(),
                                }
                            } else {
                                Action::Equip {
                                    gear_id: id,
                                    target,
                                }
                            });
                        }
                        if gear.card_id == 38 {
                            self.ui.text(
                                "Choose 2 trash cards in the Trash tab.",
                                1178.0,
                                554.0,
                                9.0,
                                MUTED,
                            );
                        }
                    }
                } else {
                    self.selection = None;
                }
            }
            None => {
                self.ui.title("Make your move.", 1178.0, 365.0, 22.0, TEXT);
                let hint = if self.prefs.coach {
                    if !self.game.chain.is_empty() {
                        "You have priority. Play a Reaction or pass. Two consecutive passes resolve the newest spell.".into()
                    } else if self.game.showdown.is_some() {
                        "Open showdown: play an Action or Reaction, or pass. Two passes close the showdown.".into()
                    } else {
                        self.game.action_hint()
                    }
                } else {
                    "Select a card from your hand or a unit on the board.".to_owned()
                };
                self.ui.wrapped(&hint, 1178.0, 396.0, 208.0, 12.0, MUTED);
                if let Some(champion) = self.game.players[0].champion {
                    self.ui.text("CHOSEN CHAMPION", 1178.0, 484.0, 9.0, GOLD);
                    self.ui.ellipsis(
                        self.game.catalog[champion].name,
                        1178.0,
                        506.0,
                        208.0,
                        12.0,
                        TEXT,
                    );
                    if self.ui.button(
                        Rect::new(1178.0, 519.0, 212.0, 31.0),
                        "Select champion",
                        false,
                        true,
                    ) {
                        self.selection = Some(Selection::Champion);
                    }
                }
            }
        }
    }
    fn hand(&mut self) {
        if self.game.phase != Phase::Mulligan {
            for (i, (label, tray)) in [
                ("Hand", Tray::Hand),
                ("Loadout", Tray::Loadout),
                ("Trash", Tray::Trash),
                ("Hidden", Tray::Hidden),
                ("Board", Tray::Board),
            ]
            .iter()
            .enumerate()
            {
                if self.ui.button(
                    Rect::new(245.0 + i as f32 * 106.0, 648.0, 98.0, 29.0),
                    label,
                    self.tray == *tray,
                    true,
                ) {
                    self.tray = *tray;
                    self.hand_scroll = 0;
                    if *tray == Tray::Board
                        || (*tray == Tray::Trash && self.gear_owner == 1)
                        || !matches!(self.selection, Some(Selection::Gear(_)))
                    {
                        self.selection = None;
                        self.targets.clear();
                    }
                }
            }
            if self.game.decision.is_some()
                && self.game.priority() == 0
                && !(self.tray == Tray::Loadout && self.game.resource_payment_pending())
                && self.choice_tray()
            {
                return;
            }
            if self.tray != Tray::Hand {
                self.other_tray();
                return;
            }
        }

        self.ui.text(
            if self.game.phase == Phase::Mulligan {
                "YOUR OPENING HAND · SELECT UP TO 2 TO REPLACE"
            } else {
                ""
            },
            245.0,
            671.0,
            10.0,
            MUTED,
        );
        let len = self.game.players[0].hand.len();
        self.hand_scroll = self.hand_scroll.min(len.saturating_sub(6));
        self.ui
            .text(&format!("{} cards", len), 985.0, 671.0, 10.0, MUTED);
        if self.ui.button(
            Rect::new(1047.0, 650.0, 34.0, 28.0),
            "<",
            false,
            self.hand_scroll > 0,
        ) {
            self.hand_scroll -= 1;
        }
        if self.ui.button(
            Rect::new(1087.0, 650.0, 34.0, 28.0),
            ">",
            false,
            self.hand_scroll + 6 < len,
        ) {
            self.hand_scroll += 1;
        }
        let visible = self.game.players[0]
            .hand
            .iter()
            .enumerate()
            .skip(self.hand_scroll)
            .take(6)
            .map(|(i, id)| (i, *id))
            .collect::<Vec<_>>();
        for (slot, (index, id)) in visible.into_iter().enumerate() {
            let x = 245.0 + slot as f32 * 150.0;
            let selected =
                self.selection == Some(Selection::Hand(index)) || self.mulligan.contains(&index);
            let y = if selected { 688.0 } else { 697.0 };
            let r = Rect::new(x, y, 140.0, 179.0);
            let c = self.game.catalog[id];
            let can = self.game.can_afford(0, &c) && self.game.card_supported(id);
            let active = self.game.phase == Phase::Mulligan || self.game.priority() == 0;
            self.ui.rect(r, PANEL);
            if let Some(tex) = &self.art[id] {
                self.ui.image(
                    tex,
                    Rect::new(x + 2.0, y + 2.0, 136.0, 94.0),
                    Some(Rect::new(
                        tex.width() * 0.08,
                        tex.height() * 0.12,
                        tex.width() * 0.84,
                        tex.height() * 0.43,
                    )),
                    if active {
                        WHITE
                    } else {
                        Color::new(0.55, 0.55, 0.55, 1.0)
                    },
                );
            }
            self.ui.rect(Rect::new(x + 7.0, y + 7.0, 26.0, 27.0), BG);
            self.ui
                .bold(&c.cost.to_string(), x + 15.0, y + 26.0, 16.0, TEAL);
            if c.power_cost > 0 {
                self.ui.pill(
                    x + 86.0,
                    y + 9.0,
                    &format!("{} P", c.power_cost),
                    rgb(c.domain.color()),
                );
            }
            self.ui
                .ellipsis(c.name, x + 9.0, y + 115.0, 123.0, 11.0, TEXT);
            self.ui.text(
                c.kind.name(),
                x + 9.0,
                y + 134.0,
                9.0,
                rgb(c.domain.color()),
            );
            let desc = card_brief(&self.game, id);
            self.ui
                .ellipsis(&desc, x + 9.0, y + 155.0, 123.0, 10.0, MUTED);
            self.ui
                .text("HOVER TO READ", x + 9.0, y + 171.0, 7.0, MUTED);
            if self.ui.hover(r) {
                self.hover_candidate = Some(id);
            }
            self.ui
                .text(&format!("{}", index + 1), x + 123.0, y + 169.0, 9.0, MUTED);
            self.ui.border(
                r,
                if selected {
                    GOLD
                } else if can && active {
                    TEAL
                } else {
                    LINE
                },
            );
            if self.mulligan.contains(&index) {
                self.ui.pill(x + 25.0, y + 55.0, "REPLACE", GOLD);
            }
            if self.ui.hover(r) && self.ui.secondary_pressed() {
                self.inspect = Some(id);
            }
            if self.ui.clicked(r) && active {
                if self.game.phase == Phase::Mulligan {
                    if !self.mulligan.remove(&index) {
                        if self.mulligan.len() < 2 {
                            self.mulligan.insert(index);
                        } else {
                            self.notify("You can replace at most 2 opening cards.");
                        }
                    }
                } else if self.game.phase == Phase::Playing {
                    self.arm_drag(DragSource::Hand { index, card_id: id });
                    self.selection = if selected {
                        None
                    } else {
                        Some(Selection::Hand(index))
                    };
                    self.targets.clear();
                    self.destination = None;
                    self.options = PlayOptions::default();
                    self.repeat_targets.clear();
                    self.editing_repeat = false;
                }
            }
        }
        if len == 0 {
            self.ui.title(
                "An empty hand. A board full of possibility.",
                270.0,
                757.0,
                23.0,
                MUTED,
            );
            self.ui.text(
                "Move your ready units, or end your turn to draw.",
                270.0,
                789.0,
                12.0,
                MUTED,
            );
        }
    }
    fn decks(&mut self) {
        self.header(
            "Find your playstyle.",
            "Tournament deck lists, ready for a little practice. Every matchup is yours to try.",
        );
        self.ui.pill(1234.0, 33.0, "4 PRELOADED DECKS", TEAL);
        for index in 0..self.game.presets.len() {
            let x = 245.0 + index as f32 * 296.0;
            let r = Rect::new(x, 116.0, 274.0, 278.0);
            let p = &self.game.presets[index];
            let selected = self.browse_deck == index;
            self.champion_art(
                index,
                Rect::new(x, 116.0, 274.0, 175.0),
                if selected {
                    WHITE
                } else {
                    Color::new(0.70, 0.75, 0.74, 1.0)
                },
            );
            self.ui.rect(Rect::new(x, 291.0, 274.0, 103.0), PANEL);
            self.ui.title(
                p.champion.split(',').next().unwrap_or(p.champion),
                x + 17.0,
                321.0,
                25.0,
                TEXT,
            );
            self.ui
                .ellipsis(p.name, x + 17.0, 345.0, 245.0, 10.0, MUTED);
            for (j, d) in p.domains.iter().enumerate() {
                self.ui
                    .circle(vec2(x + 21.0 + j as f32 * 89.0, 372.0), 3.0, rgb(d.color()));
                self.ui.text(
                    d.name(),
                    x + 31.0 + j as f32 * 89.0,
                    376.0,
                    10.0,
                    rgb(d.color()),
                );
            }
            self.ui.pill(
                x + 14.0,
                130.0,
                if index == 3 {
                    "HISTORICAL · MAY 2026"
                } else {
                    "COMPETITIVE · SEP 2026"
                },
                if index == 3 { GOLD } else { TEAL },
            );
            self.ui.border(r, if selected { TEAL } else { LINE });
            if self.ui.clicked(r) {
                self.browse_deck = index;
                self.deck_scroll = 0;
            }
        }
        let p = &self.game.presets[self.browse_deck];
        self.ui.rect(Rect::new(245.0, 418.0, 361.0, 427.0), PANEL);
        self.ui.text("THE GAME PLAN", 265.0, 449.0, 10.0, GOLD);
        self.ui.title(
            p.champion.split(',').next().unwrap_or(p.champion),
            265.0,
            482.0,
            28.0,
            TEXT,
        );
        let end = self
            .ui
            .wrapped(p.strategy, 265.0, 513.0, 320.0, 12.0, MUTED);
        self.ui.pill(265.0, end + 12.0, p.difficulty, TEAL);
        if self.ui.button(
            Rect::new(265.0, 599.0, 320.0, 30.0),
            "Legend & matchup strategy",
            false,
            true,
        ) {
            self.lesson = Some(self.browse_deck);
        }
        self.ui.wrapped("40 cards including 1 chosen champion. Choose targets, optional costs, and trigger order as you play. These dated tournament lists include their original cards and rune splits.",265.0,640.0,316.0,11.0,MUTED);
        if self.ui.button(
            Rect::new(265.0, 728.0, 152.0, 40.0),
            if self.player_deck == self.browse_deck {
                "Your deck"
            } else {
                "Play as this"
            },
            true,
            !self.in_match,
        ) {
            self.player_deck = self.browse_deck;
            self.fields[0] = local_rift::battlefields::default_battlefield(self.browse_deck);
        }
        if self.ui.button(
            Rect::new(429.0, 728.0, 156.0, 40.0),
            if self.opponent_deck == self.browse_deck {
                "Opponent"
            } else {
                "Play against"
            },
            false,
            !self.in_match,
        ) {
            self.opponent_deck = self.browse_deck;
            self.fields[1] = local_rift::battlefields::default_battlefield(self.browse_deck);
        }
        if self.in_match {
            self.ui.text(
                "Finish or reset your match to change decks.",
                265.0,
                793.0,
                10.0,
                GOLD,
            );
        } else if self.ui.button(
            Rect::new(265.0, 782.0, 320.0, 38.0),
            "Back to the practice grounds  >",
            false,
            true,
        ) {
            self.screen = Screen::Practice;
        }
        self.ui.text("THE LIST", 634.0, 444.0, 10.0, GOLD);
        self.ui.text("40 CARDS", 1314.0, 444.0, 10.0, MUTED);
        let list = self.game.presets[self.browse_deck].cards.clone();
        for (j, (id, count)) in list.iter().skip(self.deck_scroll).take(14).enumerate() {
            let col = j / 7;
            let row = j % 7;
            let x = 634.0 + col as f32 * 396.0;
            let y = 463.0 + row as f32 * 51.0;
            let card = self.game.catalog[*id];
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
            self.ui
                .text(card.kind.name(), x + 42.0, y + 34.0, 9.0, MUTED);
            self.ui
                .text(&format!("×{count}"), x + 345.0, y + 27.0, 12.0, GOLD);
            if self.ui.hover(r) {
                self.hover_candidate = Some(*id);
            }
            if self.ui.clicked(r) {
                self.inspect = Some(*id);
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
        self.ui.text("Official Riot tournament snapshots and card text. Hover to read a card; click to pin its full details.",245.0,883.0,10.0,MUTED);
    }
    fn guide(&mut self) {
        self.header(
            "Your first steps on the Rift.",
            "A few simple ideas. A lot of room to grow.",
        );
        if self.ui.button(
            Rect::new(1240.0, 25.0, 170.0, 42.0),
            "Back to practice  >",
            true,
            true,
        ) {
            self.screen = Screen::Practice;
        }
        for (i,(title,body)) in [
            ("Build your opening","Each player draws 4 cards. Replace up to 2 once. Your chosen champion waits outside your deck, ready to deploy when you can pay its cost."),
            ("Let runes do the work","Channel 2 runes each turn (the second player gets 3 on their first turn). Exhaust runes for energy; recycle matching-domain runes for power. Payment is automatic."),
            ("Deploy, then advance","Units enter exhausted at your base or a controlled battlefield. They ready next turn. Select ready units, then move together; ordinary moves exhaust them."),
            ("Take a battlefield","Move ready units together into an unowned or enemy battlefield to open a showdown. Both players can respond. Two passes resolve it; combat damage is simultaneous."),
            ("Hold what you conquer","Conquering earns 1 point. Holding at the start of your turn earns 1 point. Each battlefield can score for you only once during a turn. First to 8 wins."),
            ("Earn the final point","Holding can win immediately. To win by conquering, you must have scored both battlefields this turn. Otherwise your conquest draws a card instead.")
        ].iter().enumerate(){let col=i%3;let row=i/3;let x=245.0+col as f32*396.0;let y=117.0+row as f32*215.0;self.ui.rect(Rect::new(x,y,373.0,192.0),PANEL);self.ui.title(&format!("0{}",i+1),x+20.0,y+40.0,27.0,GOLD);self.ui.bold(title,x+70.0,y+37.0,14.0,TEXT);self.ui.wrapped(body,x+20.0,y+72.0,330.0,12.0,MUTED);}
        self.ui.rect(Rect::new(245.0, 561.0, 767.0, 285.0), PANEL);
        self.ui.text(
            "WHAT THIS PRACTICE MODE SIMPLIFIES",
            267.0,
            592.0,
            11.0,
            GOLD,
        );
        self.ui.wrapped("This is a working rules engine in development. Real tournament lists, printed stats and official rules text are included. Supported effects follow their actual text. This client is still in development; check docs/RULES.md and the current official rules for its remaining limits.",267.0,622.0,718.0,12.0,TEXT);
        self.ui.wrapped("Spells use a last-in, first-out chain. Reactions can respond to pending spells. Action cards work in an open showdown. Both players must pass. Legends, equipment, Hidden and trash replay are supported. Choose combat damage and spell targets yourself. Some trigger, replacement and payment choices are automated; inspect cards for individual limits.",267.0,709.0,718.0,12.0,MUTED);
        self.ui.wrapped("Use this client to learn resources, sequencing and battlefield scoring. Learn exact card interactions from the official rules before competitive play.",267.0,800.0,718.0,11.0,TEAL);
        self.ui.rect(Rect::new(1034.0, 561.0, 376.0, 285.0), PANEL);
        self.ui
            .text("AT YOUR FINGERTIPS", 1056.0, 592.0, 11.0, GOLD);
        for (i, (key, label)) in [
            ("CLICK", "Select card, unit or battlefield"),
            ("SHIFT + CLICK", "Add / remove a unit from a group"),
            ("RIGHT CLICK", "Inspect a card or unit"),
            ("1–9", "Select a card in your hand"),
            ("SPACE", "Pass priority / end turn"),
            ("ESC", "Cancel selection / close view"),
            ("H", "Open this guide"),
        ]
        .iter()
        .enumerate()
        {
            let y = 626.0 + i as f32 * 32.0;
            self.ui.text(key, 1056.0, y, 9.0, TEAL);
            self.ui.text(label, 1146.0, y, 10.0, MUTED);
        }
        self.ui.text("Official rules: playriftbound.com/en-us/rules-hub/  ·  Source details in docs/SOURCES.md",245.0,883.0,11.0,MUTED);
    }
    fn settings(&mut self) {
        self.header(
            "Make yourself at home.",
            "Small comforts for a better practice session.",
        );
        self.ui.rect(Rect::new(245.0, 119.0, 1165.0, 343.0), PANEL);
        for (i, (title, description, on)) in [
            (
                "Mote, your companion",
                "A quiet companion with Beginner and Advanced advice. Choose a level in the sidebar.",
                self.prefs.coach,
            ),
            (
                "Ambient animation",
                "Gentle fireflies, floating details and paced AI turns.",
                self.prefs.animation,
            ),
            (
                "Tactical opponent",
                "A more deliberate AI. Takes effect when you start a new match.",
                self.prefs.tactical,
            ),
        ]
        .iter()
        .enumerate()
        {
            let y = 144.0 + i as f32 * 106.0;
            self.ui.bold(title, 271.0, y + 23.0, 16.0, TEXT);
            self.ui.text(description, 271.0, y + 51.0, 12.0, MUTED);
            if self.ui.button(
                Rect::new(1253.0, y + 7.0, 130.0, 40.0),
                if *on { "ON" } else { "OFF" },
                *on,
                true,
            ) {
                match i {
                    0 => self.prefs.coach = !self.prefs.coach,
                    1 => self.prefs.animation = !self.prefs.animation,
                    _ => self.prefs.tactical = !self.prefs.tactical,
                };
                self.prefs.save();
            }
            if i < 2 {
                self.ui
                    .line(vec2(271.0, y + 85.0), vec2(1383.0, y + 85.0), 1.0, LINE);
            }
        }
        self.ui
            .title("Built for a little practice.", 271.0, 529.0, 27.0, TEXT);
        self.ui.wrapped("A native Rust client. Your game and all 69 card images work offline. Preferences and your win/loss record are saved on this device. Matches stay in memory while the app is open.",271.0,566.0,700.0,13.0,MUTED);
        self.ui.wrapped("Riftbound and its characters and card illustrations belong to Riot Games. This is an unofficial, unaffiliated fan practice project. Bundled tournament lists are dated snapshots; the live meta and legality may change.",271.0,674.0,700.0,12.0,MUTED);
        self.ui.text(
            "Card and tournament data checked 24 October 2026",
            271.0,
            784.0,
            11.0,
            GOLD,
        );
        self.ui
            .text("LOCAL RIFT  /  0.1.0", 271.0, 818.0, 10.0, MUTED);
    }
    fn inspector(&mut self, id: usize) {
        self.ui.rect(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            Color::new(0.0, 0.015, 0.02, 0.88),
        );
        self.ui.enabled = true;
        let r = Rect::new(348.0, 139.0, 842.0, 622.0);
        self.ui.rect(r, PANEL);
        self.ui.border(r, LINE);
        if let Some(tex) = &self.art[id] {
            self.ui
                .image(tex, Rect::new(372.0, 164.0, 360.0, 503.0), None, WHITE);
        }
        let card = self.game.catalog[id];
        self.ui.pill(764.0, 174.0, "CARD INSPECTOR", TEAL);
        self.ui.wrapped(card.name, 764.0, 239.0, 390.0, 25.0, TEXT);
        self.ui.text(
            &format!(
                "{} · {}",
                card.domains
                    .iter()
                    .map(|domain| domain.name())
                    .collect::<Vec<_>>()
                    .join(" / "),
                card.kind.name()
            ),
            764.0,
            311.0,
            13.0,
            rgb(card.domain.color()),
        );
        self.ui.text(
            &format!(
                "{} ENERGY   {} POWER   {} MIGHT",
                card.cost, card.power_cost, card.might
            ),
            764.0,
            346.0,
            11.0,
            GOLD,
        );
        self.ui
            .line(vec2(764.0, 371.0), vec2(1158.0, 371.0), 1.0, LINE);
        if self.inspect_last != Some(id) {
            self.inspect_last = Some(id);
            self.inspect_scroll = 0;
        }
        self.ui.bold("CARD TEXT", 764.0, 405.0, 11.0, TEAL);
        let notes = self.game.effect_description(id);
        let text = if notes.is_empty() {
            card.official_text.to_owned()
        } else {
            format!("{}\n\nPractice note\n{}", card.official_text, notes)
        };
        let max = self.ui.scroll_text(
            &text,
            Rect::new(764.0, 427.0, 394.0, 218.0),
            13.0,
            &mut self.inspect_scroll,
        );
        self.ui.text(
            if max > 0 {
                "Scroll to read the complete card and practice notes."
            } else {
                "Complete official text"
            },
            764.0,
            665.0,
            10.0,
            MUTED,
        );
        if self.ui.button(
            Rect::new(764.0, 682.0, 394.0, 44.0),
            "Back to the game  [ESC]",
            true,
            true,
        ) {
            self.inspect = None;
        }
        self.ui
            .text("Official card art © Riot Games", 372.0, 705.0, 10.0, MUTED);
    }
    fn journal(&mut self) {
        self.ui.rect(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            Color::new(0.0, 0.015, 0.02, 0.92),
        );
        self.ui.enabled = true;
        self.ui.rect(Rect::new(310.0, 95.0, 920.0, 710.0), PANEL);
        self.ui.border(Rect::new(310.0, 95.0, 920.0, 710.0), LINE);
        self.ui
            .title("The story of this match", 340.0, 144.0, 30.0, TEXT);
        self.ui.text(
            "Latest first · review plays, responses, costs and scoring",
            340.0,
            173.0,
            12.0,
            MUTED,
        );
        for (i, entry) in self
            .game
            .log
            .iter()
            .rev()
            .skip(self.journal_page * 8)
            .take(8)
            .enumerate()
        {
            let y = 215.0 + i as f32 * 59.0;
            self.ui.circle(vec2(345.0, y - 4.0), 2.5, TEAL);
            self.ui.wrapped_lines(entry, 360.0, y, 825.0, 12.0, TEXT, 3);
        }
        if self.ui.button(
            Rect::new(340.0, 715.0, 140.0, 35.0),
            "< Newer",
            false,
            self.journal_page > 0,
        ) {
            self.journal_page -= 1;
        }
        if self.ui.button(
            Rect::new(491.0, 715.0, 140.0, 35.0),
            "Older >",
            false,
            (self.journal_page + 1) * 8 < self.game.log.len(),
        ) {
            self.journal_page += 1;
        }
        if self.ui.button(
            Rect::new(880.0, 715.0, 320.0, 35.0),
            "Return to the board  [ESC]",
            true,
            true,
        ) {
            self.show_journal = false;
        }
    }
    fn restart_dialog(&mut self) {
        self.ui.rect(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            Color::new(0.0, 0.015, 0.02, 0.88),
        );
        self.ui.enabled = true;
        self.ui.rect(Rect::new(492.0, 310.0, 500.0, 245.0), PANEL);
        self.ui.border(Rect::new(492.0, 310.0, 500.0, 245.0), LINE);
        self.ui.title("A fresh start?", 520.0, 356.0, 30.0, TEXT);
        self.ui.wrapped("Return to setup and choose your next matchup. This unfinished game will not affect your practice record.",520.0,394.0,435.0,13.0,MUTED);
        if self.ui.button(
            Rect::new(520.0, 479.0, 207.0, 44.0),
            "Keep playing",
            false,
            true,
        ) {
            self.confirm_restart = false;
        }
        if self.ui.button(
            Rect::new(747.0, 479.0, 214.0, 44.0),
            "Choose matchup",
            true,
            true,
        ) {
            self.in_match = false;
            self.selection = None;
            self.confirm_restart = false;
            self.game = Game::new(
                self.player_deck,
                self.opponent_deck,
                seed(),
                Difficulty::Easy,
            );
        }
    }
    fn victory(&mut self) {
        self.ui.rect(
            Rect::new(364.0, 235.0, 644.0, 306.0),
            Color::new(0.035, 0.078, 0.09, 0.98),
        );
        self.ui.border(Rect::new(364.0, 235.0, 644.0, 306.0), GOLD);
        self.ui.pill(394.0, 264.0, "PRACTICE COMPLETE", TEAL);
        self.ui.title(
            if self.game.winner == Some(0) {
                "The Rift is yours."
            } else {
                "A lesson well played."
            },
            394.0,
            335.0,
            38.0,
            TEXT,
        );
        self.ui.text(
            &format!(
                "Final score   {} – {}",
                self.game.players[0].score, self.game.players[1].score
            ),
            394.0,
            373.0,
            17.0,
            GOLD,
        );
        self.ui.text(
            "Every match is another step. Ready for the next one?",
            394.0,
            409.0,
            13.0,
            MUTED,
        );
        if self.ui.button(
            Rect::new(394.0, 453.0, 269.0, 47.0),
            "Play again",
            true,
            true,
        ) {
            self.reset();
        }
        if self.ui.button(
            Rect::new(685.0, 453.0, 291.0, 47.0),
            "Choose another matchup",
            false,
            true,
        ) {
            self.in_match = false;
            self.selection = None;
            self.game = Game::new(
                self.player_deck,
                self.opponent_deck,
                seed(),
                Difficulty::Easy,
            );
        }
    }
}
fn card_brief(game: &Game, id: usize) -> String {
    use local_rift::rules::OfficialEffect::*;
    match local_rift::rules::effect(id) {
        Damage {
            amount,
            targets,
            draw,
            ..
        } => format!(
            "Deal {amount} · {targets} target(s){}",
            if draw > 0 { " + draw" } else { "" }
        ),
        ModifyMight { amount, draw, .. } => {
            format!("{amount:+} might{}", if draw > 0 { " · draw" } else { "" })
        }
        Draw(n) => format!("Draw {n}"),
        Counter(_) => "Counter a spell".into(),
        DualMight => "+2 / -2 might".into(),
        Bounce { .. } | BounceBoth => "Return units to hand".into(),
        MoveFriendlyToBase { .. } => "Move units to base".into(),
        MoveEnemy | MoveFriendlyAndReady => "Move a unit".into(),
        Tokens { count, might } => format!("{count} × {might}-might tokens"),
        ExtraTurn => "Take an extra turn".into(),
        _ => {
            if game.catalog[id].kind == CardKind::Unit {
                format!("{} printed might", game.catalog[id].might)
            } else {
                "Inspect rules text".into()
            }
        }
    }
}
fn window_conf() -> Conf {
    let small = std::env::args().any(|a| a == "--small");
    Conf {
        window_title: "Riftbound · The Practice Grounds".to_owned(),
        window_width: if small { 1152 } else { 1440 },
        window_height: if small { 720 } else { 900 },
        window_resizable: true,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}
#[macroquad::main(window_conf)]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let screenshot = args
        .iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let mut app = App::new();
    if args.iter().any(|a| a == "--decks") {
        app.screen = Screen::Decks;
    }
    if args.iter().any(|a| a == "--guide") {
        app.screen = Screen::Guide;
    }
    if args.iter().any(|a| a == "--match") {
        app.reset();
        app.game.start_match().unwrap();
    }
    if args.iter().any(|arg| arg == "--energy-preview") {
        app.in_match = true;
        let gear_preview = args.iter().any(|arg| arg == "--gear-preview");
        app.player_deck = if gear_preview { 0 } else { 3 };
        app.opponent_deck = if gear_preview { 2 } else { 1 };
        app.prefs.tactical = true;
        let seeds: Vec<u64> = if gear_preview {
            (1..128).collect()
        } else {
            vec![13]
        };
        'preview: for preview_seed in seeds {
            let mut preview_game = Game::new(
                app.player_deck,
                app.opponent_deck,
                preview_seed,
                Difficulty::Tactical,
            );
            preview_game.start_match().unwrap();
            for _ in 0..400 {
                let player = &preview_game.players[0];
                if preview_game.active == 0
                    && preview_game.turn >= 7
                    && player.total_runes() >= 6
                    && player.runes.iter().filter(|count| **count > 0).count() >= 2
                    && player.energy >= 2
                    && player.energy < player.total_runes()
                    && player.units.len() >= 2
                    && (!gear_preview || player.gear.iter().any(|gear| gear.attached_to.is_some()))
                    && preview_game.players[1].units.len() <= 3
                    && preview_game.chain.is_empty()
                    && preview_game.decision.is_none()
                {
                    println!(
                        "Resource preview seed {preview_seed}, turn {}",
                        preview_game.turn
                    );
                    app.game = preview_game;
                    break 'preview;
                }
                if preview_game.winner.is_some() {
                    break;
                }
                preview_game.step_ai().unwrap();
            }
        }
        // Deterministic visual stress fixture, used only by the screenshot flag.
        if args.iter().any(|arg| arg == "--energy-full") {
            app.game.players[0].runes = [6, 0, 6, 0, 0, 0];
            app.game.players[0].exhausted_runes = [2, 0, 3, 0, 0, 0];
            app.game.players[0].rune_deck.clear();
            app.game.players[0].energy = 7;
            app.game.players[0].power_pool[domain_index(Domain::Chaos)] = 2;
            app.game.players[0].any_power = 1;
            app.game.legend[0].spell_power = 3;
        }
        if args.iter().any(|arg| arg == "--board-full") {
            for owner in 0..2 {
                let template = app.game.players[owner].units[0].clone();
                app.game.players[owner].units.clear();
                for index in 0..16 {
                    let mut unit = template.clone();
                    unit.id = 60_000 + (owner * 100 + index) as u32;
                    unit.lane = if index < 8 {
                        None
                    } else {
                        Some((index - 8) / 4)
                    };
                    unit.ready = index % 2 == 0;
                    app.game.players[owner].units.push(unit);
                }
            }
            app.tray = Tray::Board;
        }
        app.next_ai = f64::INFINITY;
        app.companion.observe(&app.game);
    }
    if let Some(index) = args.iter().position(|arg| arg == "--inspect-card")
        && let Some(id) = args
            .get(index + 1)
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|id| *id < app.game.catalog.len())
    {
        app.inspect = Some(id);
    }
    let preview_hover = args.iter().any(|arg| arg == "--hover-preview");
    let smoke_test = args.iter().any(|a| a == "--ui-smoke");
    let mut frames = 0;
    loop {
        if smoke_test {
            app.next_ai = f64::INFINITY;
        }
        app.update();
        if preview_hover {
            app.ui.input_override = Some((vec2(315.0, 760.0), false, false));
            app.hover_card = app.game.players[0].hand.first().copied();
            app.hover_since = -1.0;
        }
        let smoke_done = smoke_test && smoke::run_frame(&mut app, frames);
        app.draw();
        if smoke_done {
            println!("UI smoke: all native click checks passed.");
            if let Some(path) = &screenshot {
                get_screen_data().export_png(path);
            }
            break;
        }
        frames += 1;
        if frames == 12
            && !smoke_test
            && let Some(path) = &screenshot
        {
            get_screen_data().export_png(path);
            break;
        }
        next_frame().await;
    }
}
