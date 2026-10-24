mod ability_ui;
mod companion;
mod deck_editor;
mod play_ui;
mod predict_ui;
use companion::{AdviceLevel, Companion, draw_mote};
use deck_editor::DeckEditor;
mod scene;
mod smoke;
mod ui;
use local_rift::{
    activated::AbilitySource,
    cards::{CardKind, Domain},
    game::{
        Action, Decision, Difficulty, FlowSource, Game, Phase, PlayOptions, Target, domain_index,
    },
    numeric::{DamageAmount, Might},
    payment::{POWER_DOMAINS, PaymentQuote, PowerPayment},
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
    DeckEditor,
    Guide,
    Settings,
}
#[derive(Clone, Copy, PartialEq)]
enum Selection {
    Hand(usize),
    Unit(u32),
    Ability(AbilitySource, Option<usize>),
    Champion,
    Gear(u32),
    Legend,
    Trash(usize),
    Pending(u32),
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
    might: Might,
    gear: bool,
    rules: String,
}
const GOLD_RULES: &str = "Exhaust and kill this Gold to add 1 Any power. This resource ability can be used when you have priority, including during a pending payment, and cannot be reacted to.";
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

#[derive(Clone)]
struct PaymentDraft {
    action: Action,
    quote: PaymentQuote,
    chosen: PowerPayment,
    just_opened: bool,
}

struct Preferences {
    coach: bool,
    advanced: bool,
    animation: bool,
    difficulty: Difficulty,
    exact_payment: bool,
    use_custom: [bool; 2],
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
        Self::from_text(&content)
    }
    fn from_text(content: &str) -> Self {
        let value = |key: &str| {
            content
                .lines()
                .find_map(|l| l.strip_prefix(&format!("{key}=")))
        };
        Self {
            coach: value("coach") != Some("false"),
            advanced: value("advanced") == Some("true"),
            animation: value("animation") != Some("false"),
            difficulty: match value("difficulty") {
                Some("easy") => Difficulty::Easy,
                Some("tactical") => Difficulty::Tactical,
                Some("strategic") => Difficulty::Strategic,
                _ if value("tactical") == Some("true") => Difficulty::Tactical,
                _ => Difficulty::Easy,
            },
            exact_payment: value("exact_payment") == Some("true"),
            use_custom: [
                value("custom_you") == Some("true"),
                value("custom_ai") == Some("true"),
            ],
            wins: value("wins").and_then(|v| v.parse().ok()).unwrap_or(0),
            losses: value("losses").and_then(|v| v.parse().ok()).unwrap_or(0),
        }
    }
    fn to_text(&self) -> String {
        format!(
            "coach={}\nadvanced={}\nanimation={}\ndifficulty={}\nexact_payment={}\ncustom_you={}\ncustom_ai={}\nwins={}\nlosses={}\n",
            self.coach,
            self.advanced,
            self.animation,
            match self.difficulty {
                Difficulty::Easy => "easy",
                Difficulty::Tactical => "tactical",
                Difficulty::Strategic => "strategic",
            },
            self.exact_payment,
            self.use_custom[0],
            self.use_custom[1],
            self.wins,
            self.losses
        )
    }
    fn save(&self) {
        let p = Self::path();
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(p, self.to_text());
    }
}
#[derive(Clone, Copy)]
enum FieldPickerTarget {
    Setup(usize),
    Editor,
}
#[derive(Clone)]
struct FieldPicker {
    target: FieldPickerTarget,
    selected: usize,
    list_scroll: usize,
    rules_scroll: usize,
}

struct App {
    ui: Ui,
    game: Game,
    screen: Screen,
    in_match: bool,
    player_deck: usize,
    opponent_deck: usize,
    browse_deck: usize,
    custom_decks: [Option<local_rift::deck::CustomDeck>; 2],
    deck_editor: Option<DeckEditor>,
    selection: Option<Selection>,
    mulligan: HashSet<usize>,
    art: Vec<Option<Texture2D>>,
    legend_art: Vec<Option<Texture2D>>,
    field_art: Vec<Option<Texture2D>>,
    legend_picker: Option<usize>,
    legend_hover: Option<usize>,
    deck_load_errors: Vec<String>,
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
    ability_targets: Vec<Target>,
    ability_costs: Vec<usize>,
    conversion_amount: u16,
    conversion_editor: Option<String>,
    conversion_key: Option<String>,
    resource_drafts: Vec<(String, ability_ui::UiDraft)>,
    suspended_payments: Vec<ability_ui::SuspendedPayment>,
    payment_origin: Option<ability_ui::UiDraft>,
    repeat_targets: Vec<Target>,
    editing_repeat: bool,
    destination: Option<usize>,
    tray: Tray,
    gear_owner: usize,
    banish_view: bool,
    choices: Vec<usize>,
    predict_draft: Option<predict_ui::PredictDraft>,
    options: PlayOptions,
    play_draft: Option<play_ui::PlayDraft>,
    cost_editor: Option<play_ui::CostEditor>,
    show_play_effects: bool,
    play_effects_scroll: usize,
    flow_source: Option<FlowSource>,
    fields: [usize; 2],
    starting_player: usize,
    show_journal: bool,
    journal_page: usize,
    chain_cursor: usize,
    damage_order: Vec<u32>,
    damage_amounts: Option<Vec<(u32, DamageAmount)>>,
    damage_editor: Option<(u32, String)>,
    payment_draft: Option<PaymentDraft>,
    confirm_restart: bool,
    recorded: bool,
    drag: Option<DragState>,
    drop_zones: Vec<(DropZone, Rect)>,
    hover_candidate: Option<usize>,
    hover_token_candidate: Option<TokenPreview>,
    hover_token: Option<TokenPreview>,
    hover_card: Option<usize>,
    hover_object_candidate: Option<(usize, u32)>,
    hover_object: Option<(usize, u32)>,
    hover_since: f64,
    hover_rect: Option<Rect>,
    hover_scroll: usize,
    inspect_scroll: usize,
    inspect_last: Option<usize>,
    rune_menu: Option<Domain>,
    rune_menu_at: Vec2,
    field_hover: Option<usize>,
    field_inspect: Option<usize>,
    field_picker: Option<FieldPicker>,
    equipment_inspect: Option<(usize, u32)>,
    equipment_scroll: usize,
    trigger_choice_key: Option<String>,
    token_draft_id: Option<u32>,
}
fn optional_query<T>(result: Result<T, String>) -> Result<Option<T>, String> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.starts_with("engine_capacity:") => Err(error),
        Err(_) => Ok(None),
    }
}

fn parse_damage_amount(value: &str) -> Result<DamageAmount, String> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err("Enter a whole damage amount without spaces or leading zeros.".into());
    }
    value
        .parse()
        .map_err(|_| "Damage must be from 0 to 18446744073709551615.".into())
}

fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|t| t.as_nanos() as u64)
        .unwrap_or(42)
}
impl App {
    fn new() -> Self {
        let mut prefs = Preferences::load();
        let (custom_decks, deck_load_errors) = deck_editor::load_saved_decks_with_errors();
        for (owner, deck) in custom_decks.iter().enumerate() {
            prefs.use_custom[owner] &= deck.is_some();
        }
        // These remain tournament preset indices; custom legends have a separate registry.
        let selected = [0, 1];
        let fields = std::array::from_fn(|owner| {
            if prefs.use_custom[owner] {
                custom_decks[owner].as_ref().unwrap().battlefield
            } else {
                local_rift::battlefields::default_battlefield(selected[owner])
            }
        });
        let game = Game::new_with_custom(
            selected,
            std::array::from_fn(|owner| {
                if prefs.use_custom[owner] {
                    custom_decks[owner].clone()
                } else {
                    None
                }
            }),
            seed(),
            prefs.difficulty,
            true,
        )
        .expect("saved custom decks were validated on load");
        let mut art = vec![None; game.catalog.len()];
        for &(id, bytes) in CARD_ART {
            if id < art.len() {
                let tex = Texture2D::from_file_with_format(bytes, None);
                tex.set_filter(FilterMode::Linear);
                art[id] = Some(tex);
            }
        }
        let mut legend_art = vec![None; local_rift::legend_catalog::legends().len()];
        for &(id, bytes) in LEGEND_ART {
            if let Some(slot) = legend_art.get_mut(id) {
                let texture = Texture2D::from_file_with_format(bytes, None);
                texture.set_filter(FilterMode::Linear);
                *slot = Some(texture);
            }
        }
        let mut field_art = vec![None; local_rift::battlefields::BATTLEFIELDS.len()];
        for &(id, bytes) in FIELD_ART {
            if let Some(slot) = field_art.get_mut(id) {
                let texture = Texture2D::from_file_with_format(bytes, None);
                texture.set_filter(FilterMode::Linear);
                *slot = Some(texture);
            }
        }
        Self {
            ui: Ui::new(),
            game,
            screen: Screen::Practice,
            in_match: false,
            player_deck: selected[0],
            opponent_deck: selected[1],
            browse_deck: selected[0],
            custom_decks,
            deck_editor: None,
            selection: None,
            mulligan: HashSet::new(),
            art,
            legend_art,
            field_art,
            legend_picker: None,
            legend_hover: None,
            deck_load_errors,
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
            ability_targets: Vec::new(),
            ability_costs: Vec::new(),
            conversion_amount: 0,
            conversion_editor: None,
            conversion_key: None,
            resource_drafts: Vec::new(),
            suspended_payments: Vec::new(),
            payment_origin: None,
            repeat_targets: Vec::new(),
            editing_repeat: false,
            destination: None,
            tray: Tray::Hand,
            gear_owner: 0,
            banish_view: false,
            choices: Vec::new(),
            predict_draft: None,
            options: PlayOptions::default(),
            play_draft: None,
            cost_editor: None,
            show_play_effects: false,
            play_effects_scroll: 0,
            flow_source: None,
            fields,
            starting_player: 0,
            show_journal: false,
            journal_page: 0,
            chain_cursor: 0,
            damage_order: Vec::new(),
            damage_amounts: None,
            damage_editor: None,
            payment_draft: None,
            confirm_restart: false,
            recorded: false,
            drag: None,
            drop_zones: Vec::new(),
            hover_candidate: None,
            hover_token_candidate: None,
            hover_token: None,
            hover_card: None,
            hover_object_candidate: None,
            hover_object: None,
            hover_since: 0.0,
            hover_rect: None,
            hover_scroll: 0,
            inspect_scroll: 0,
            inspect_last: None,
            rune_menu: None,
            rune_menu_at: vec2(898.0, 395.0),
            field_hover: None,
            field_inspect: None,
            field_picker: None,
            equipment_inspect: None,
            equipment_scroll: 0,
            trigger_choice_key: None,
            token_draft_id: None,
        }
    }
    fn reset(&mut self) {
        let candidate = Game::new_with_custom(
            [self.player_deck, self.opponent_deck],
            self.active_custom_decks(),
            seed(),
            self.prefs.difficulty,
            true,
        );
        let mut game = match candidate {
            Ok(game) => game,
            Err(error) => {
                self.notify(error);
                return;
            }
        };
        game.choose_starting_player(if self.starting_player == 2 {
            (seed() % 2) as usize
        } else {
            self.starting_player
        })
        .expect("valid starting player");
        for player in 0..2 {
            game.choose_battlefield(player, self.fields[player])
                .expect("valid selected battlefield");
        }
        self.game = game;
        self.predict_draft = None;
        self.play_draft = None;
        self.cost_editor = None;
        self.drag = None;
        self.hover_card = None;
        self.payment_draft = None;
        self.suspended_payments.clear();
        self.payment_origin = None;
        self.resource_drafts.clear();
        self.ability_costs.clear();
        self.conversion_key = None;
        self.equipment_inspect = None;
        self.trigger_choice_key = None;
        self.token_draft_id = None;
        self.show_journal = false;
        self.damage_order.clear();
        self.damage_amounts = None;
        self.damage_editor = None;
        self.tray = Tray::Hand;
        self.repeat_targets.clear();
        self.editing_repeat = false;
        self.group.clear();
        self.destination = None;
        self.options = PlayOptions::default();
        self.flow_source = None;
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
        let a = self.canonical_action(a);
        match self.maybe_review_cost(&a) {
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => {
                self.notify(error);
                return;
            }
        }
        if !self.suspended_payments.is_empty() {
            match self.resource_visit_action_allowed(&a) {
                Ok(true) => {}
                Ok(false) => {
                    self.notify(
                        "Resume or cancel your saved payment before taking another action.",
                    );
                    return;
                }
                Err(error) => {
                    self.notify(error);
                    return;
                }
            }
        }
        if self.prefs.exact_payment
            && self.game.priority() == 0
            && !matches!(a, Action::WithPowerPayment { .. })
        {
            match self.game.quote_power_payment(&a) {
                Ok(quote) if quote.suggested.total() > 0 => {
                    self.payment_draft = Some(PaymentDraft {
                        action: a,
                        chosen: quote.suggested.clone(),
                        quote,
                        just_opened: true,
                    });
                    self.hover_card = None;
                    self.hover_rect = None;
                    self.rune_menu = None;
                    self.toast_until = 0.0;
                    return;
                }
                Ok(_) => {}
                Err(error) => {
                    self.notify(error);
                    return;
                }
            }
        }
        self.apply_action(a);
    }
    fn apply_action(&mut self, a: Action) {
        let inner = play_ui::core_action(&a);
        let finishing_plan = matches!(self.game.decision, Some(Decision::Payment { .. }))
            && matches!(inner, Action::CompletePayment | Action::Choose { .. });
        let cancelling_plan = finishing_plan && matches!(inner, Action::Choose { accept: false });
        let preserve_selection = matches!(
            inner,
            Action::ActivateRune { .. }
                | Action::RecycleRune { .. }
                | Action::SetOptionalDiscount { .. }
        );
        let adding_resource = matches!(
            inner,
            Action::ActivateGear { .. }
                | Action::ActivateAbility { .. }
                | Action::ActivateAbilityWithCosts { .. }
                | Action::ResolveConversion { .. }
                | Action::ActivateLegend { .. }
                | Action::ActivateRune { .. }
                | Action::RecycleRune { .. }
        ) && self.game.decision.is_some();
        match self.game.apply(a) {
            Ok(()) => {
                if finishing_plan {
                    self.suspended_payments.clear();
                    self.resource_drafts.clear();
                    let original = self.payment_origin.take();
                    if cancelling_plan {
                        if let Some(original) = original {
                            self.restore_ui_draft(original);
                        }
                        return;
                    }
                }
                if preserve_selection {
                    return;
                }
                if adding_resource && self.restore_resource_draft() {
                    return;
                }
                if adding_resource
                    && matches!(
                        self.game.decision,
                        Some(Decision::TriggerTargets { .. } | Decision::LegendTargets { .. })
                    )
                {
                    self.selection = None;
                    self.tray = Tray::Board;
                    return;
                }
                if adding_resource {
                    self.tray = Tray::Hand;
                }
                self.damage_order.clear();
                self.damage_amounts = None;
                self.damage_editor = None;
                self.selection = None;
                self.play_draft = None;
                self.group.clear();
                self.targets.clear();
                self.repeat_targets.clear();
                self.editing_repeat = false;
                self.destination = None;
                self.choices.clear();
                self.options = PlayOptions::default();
                self.flow_source = None;
                self.next_ai = get_time() + 0.7;
            }
            Err(e) => self.notify(e),
        }
    }
    fn update(&mut self) {
        self.ui.resize();
        if self.game.validate_numeric_state().is_err() {
            return;
        }
        if self.show_play_effects {
            if is_key_pressed(KeyCode::Escape) {
                self.show_play_effects = false;
            }
            return;
        }
        if self.cost_editor.is_some() {
            if is_key_pressed(KeyCode::Escape) {
                self.cost_editor = None;
            }
            return;
        }
        if self.legend_picker.is_some()
            || self.field_picker.is_some()
            || !self.deck_load_errors.is_empty()
        {
            if is_key_pressed(KeyCode::Escape) {
                self.legend_picker = None;
                self.field_picker = None;
            }
            return;
        }
        self.companion.observe(&self.game);
        if self.payment_draft.is_some() {
            if is_key_pressed(KeyCode::Escape) {
                self.payment_draft = None;
            }
            return;
        }
        if self.conversion_editor.is_some() {
            while let Some(character) = get_char_pressed() {
                if (character.is_ascii_digit() || character == '-')
                    && self.conversion_editor.as_ref().unwrap().len() < 6
                {
                    self.conversion_editor.as_mut().unwrap().push(character);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                self.conversion_editor.as_mut().unwrap().pop();
            }
            if is_key_pressed(KeyCode::Escape) {
                self.conversion_editor = None;
            } else if is_key_pressed(KeyCode::Enter)
                && let Err(error) = self.commit_conversion_editor()
            {
                self.notify(error);
            }
            return;
        }
        if !self.suspended_payments.is_empty() {
            return;
        }
        if self.screen == Screen::DeckEditor && self.inspect.is_none() {
            self.update_deck_editor();
            return;
        }
        if self.damage_editor.is_some() && self.inspect.is_none() {
            while let Some(character) = get_char_pressed() {
                if character.is_ascii_digit() {
                    let (_, value) = self.damage_editor.as_mut().unwrap();
                    value.push(character);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                self.damage_editor.as_mut().unwrap().1.pop();
            }
            if is_key_pressed(KeyCode::Escape) {
                self.damage_editor = None;
            } else if is_key_pressed(KeyCode::Enter)
                && let Err(error) = self.commit_damage_editor()
            {
                self.notify(error);
            }
            return;
        }
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
            && self.equipment_inspect.is_none()
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
                self.reset_predict_draft();
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
                    && self.game.decision.is_none()
                {
                    self.selection = Some(Selection::Hand(i));
                    self.targets.clear();
                    self.destination = None;
                    self.options = PlayOptions::default();
                    self.flow_source = None;
                    self.repeat_targets.clear();
                    self.editing_repeat = false;
                }
            }
        } else if is_key_pressed(KeyCode::Escape) {
            self.inspect = None;
            self.field_inspect = None;
            self.equipment_inspect = None;
            self.lesson = None;
            self.confirm_restart = false;
            self.show_journal = false;
            if self.screen != Screen::DeckEditor {
                self.screen = Screen::Practice;
            }
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
        if let Err(error) = self.draw_checked() {
            self.numeric_error_view(&error);
        }
    }
    fn numeric_error_view(&mut self, error: &str) {
        clear_background(BG);
        self.ui.enabled = true;
        self.ui.rect(Rect::new(0.0, 0.0, 1440.0, 900.0), BG);
        self.ui.title(
            "This position cannot be calculated",
            285.0,
            290.0,
            30.0,
            TEXT,
        );
        self.ui.wrapped("A numeric limit was reached. The game is paused; no replacement stats or damage amounts are being shown.", 285.0, 335.0, 820.0, 16.0, MUTED);
        self.ui.wrapped(error, 285.0, 405.0, 820.0, 14.0, GOLD);
        if self.ui.button(
            Rect::new(285.0, 555.0, 240.0, 42.0),
            "Start a new practice match",
            false,
            true,
        ) {
            self.reset();
        }
    }
    fn draw_checked(&mut self) -> Result<(), String> {
        self.game.validate_numeric_state()?;
        self.sync_predict_draft();
        self.sync_token_draft();
        self.sync_play_draft()?;
        let trigger_key = self
            .game
            .decision
            .as_ref()
            .filter(|decision| {
                matches!(
                    decision,
                    Decision::TriggerTargets { .. } | Decision::LegendTargets { .. }
                )
            })
            .map(|decision| format!("{decision:?}"));
        if trigger_key != self.trigger_choice_key {
            if trigger_key.is_some() {
                self.selection = None;
                self.targets.clear();
                self.repeat_targets.clear();
                self.editing_repeat = false;
                self.hand_scroll = 0;
                if self
                    .game
                    .trigger_target_rule()
                    .is_some_and(|rule| rule.kind == local_rift::rules::PlayTargetKind::Gear)
                {
                    self.tray = Tray::Loadout;
                    self.gear_owner = 1;
                } else if self
                    .game
                    .trigger_target_rule()
                    .is_some_and(|rule| rule.kind == local_rift::rules::PlayTargetKind::UnitOrGear)
                {
                    self.tray = Tray::Board;
                }
            }
            self.trigger_choice_key = trigger_key;
        }
        clear_background(BG);
        self.ui.rect(Rect::new(0.0, 0.0, 1440.0, 900.0), BG);
        self.field_hover = None;
        self.legend_hover = None;
        self.hover_candidate = None;
        self.hover_object_candidate = None;
        self.hover_token_candidate = None;
        self.drop_zones.clear();
        if self.hover_rect.is_some_and(|r| r.contains(self.ui.mouse()))
            && self.ui.secondary_pressed()
            && self.hover_token.is_none()
        {
            self.inspect = self.hover_card;
        }
        self.ui.enabled = !self.show_play_effects
            && self.cost_editor.is_none()
            && self.field_picker.is_none()
            && self.legend_picker.is_none()
            && self.deck_load_errors.is_empty()
            && self.inspect.is_none()
            && self.lesson.is_none()
            && !self.confirm_restart
            && !self.show_journal
            && self.rune_menu.is_none()
            && self.field_inspect.is_none()
            && self.equipment_inspect.is_none()
            && self.payment_draft.is_none()
            && !self.hover_rect.is_some_and(|r| r.contains(self.ui.mouse()));
        self.sidebar();
        match self.screen {
            Screen::Practice => self.practice()?,
            Screen::Decks => self.decks(),
            Screen::DeckEditor => self.draw_deck_editor(),
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
        self.equipment_help();
        self.suspended_payment_banner()?;
        self.payment_editor();
        self.legend_tooltip();
        self.draw_legend_picker();
        self.draw_field_picker();
        self.draw_cost_editor();
        self.draw_play_effects();
        self.saved_deck_notice();
        self.ui.enabled = true;
        Ok(())
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
            let selected = self.screen == *screen
                || (*screen == Screen::Decks && self.screen == Screen::DeckEditor);
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
    fn setup_legend(&self, owner: usize) -> &'static local_rift::legend_catalog::LegendDef {
        let preset = if owner == 0 {
            self.player_deck
        } else {
            self.opponent_deck
        };
        let id = self.custom_decks[owner]
            .as_ref()
            .filter(|_| self.prefs.use_custom[owner])
            .map_or(self.game.presets[preset].legend_id, |deck| deck.legend);
        local_rift::legend_catalog::legend(id).expect("validated setup legend")
    }
    fn setup_champion(&self, owner: usize) -> usize {
        let preset = if owner == 0 {
            self.player_deck
        } else {
            self.opponent_deck
        };
        self.custom_decks[owner]
            .as_ref()
            .filter(|_| self.prefs.use_custom[owner])
            .map_or(self.game.presets[preset].chosen_champion, |deck| {
                deck.chosen_champion
            })
    }
    fn draw_legend_art(&self, id: usize, rect: Rect, crop: bool, tint: Color) {
        if let Some(Some(texture)) = self.legend_art.get(id) {
            self.ui.image(
                texture,
                rect,
                crop.then(|| {
                    Rect::new(
                        texture.width() * 0.08,
                        texture.height() * 0.12,
                        texture.width() * 0.84,
                        texture.height() * 0.43,
                    )
                }),
                tint,
            );
        } else if let Some(legend) = local_rift::legend_catalog::legend(id) {
            self.ui.rect(rect, rgb(legend.accent));
        }
    }
    fn legend_tooltip(&mut self) {
        let Some(id) = self.legend_hover else {
            return;
        };
        if self.lesson.is_some()
            || self.inspect.is_some()
            || self.legend_picker.is_some()
            || self.payment_draft.is_some()
        {
            return;
        }
        let definition = local_rift::legend_catalog::legend(id).unwrap();
        self.hover_card = None;
        self.hover_rect = None;
        let r = Rect::new(
            (self.ui.mouse().x - 165.0).clamp(240.0, 1095.0),
            210.0,
            330.0,
            370.0,
        );
        self.ui.rect(r, PANEL);
        self.ui.border(r, GOLD);
        self.draw_legend_art(
            id,
            Rect::new(r.x + 14.0, r.y + 14.0, 92.0, 129.0),
            false,
            WHITE,
        );
        self.ui.wrapped(
            &legend_label(definition),
            r.x + 120.0,
            r.y + 34.0,
            195.0,
            16.0,
            TEXT,
        );
        self.ui
            .wrapped(definition.text, r.x + 14.0, r.y + 168.0, 302.0, 12.0, TEXT);
        self.ui.text(
            "Right-click to open full legend rules",
            r.x + 14.0,
            r.y + 352.0,
            10.0,
            GOLD,
        );
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
    fn practice(&mut self) -> Result<(), String> {
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
        if self.in_match
            && self.ui.button(
                Rect::new(1030.0, 34.0, 185.0, 32.0),
                &format!("Turn effects · {}", self.game.play_grants.len()),
                self.show_play_effects,
                true,
            )
        {
            self.show_play_effects = true;
            self.play_effects_scroll = 0;
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
                    might: self.game.unit_might(unit)?,
                    accent: self.game.legend_def(player).accent,
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
        self.draw_playmat(&hits)?;
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
                let equipped = self.game.players[owner]
                    .gear
                    .iter()
                    .any(|gear| gear.attached_to == Some(*id));
                self.board_card(
                    if equipped {
                        Rect::new(card_rect.x, card_rect.y, card_rect.w, card_rect.h - 20.0)
                    } else {
                        card_rect
                    },
                    card_id,
                    owner,
                    unit.ready,
                    self.game.unit_empowered(unit.id),
                    self.game
                        .has_temporary(local_rift::empower::BoardObject::Unit {
                            player: owner,
                            unit_id: *id,
                        }),
                    self.game.unit_might(unit)?,
                    unit.damage,
                    token,
                    &unit_name,
                    state.selected == Some(*id) || self.group.contains(id),
                    self.current_targets().contains(&Target::Unit {
                        player: owner,
                        unit_id: *id,
                    }) || self.ability_targets.contains(&Target::Unit {
                        player: owner,
                        unit_id: *id,
                    }) && matches!(self.selection, Some(Selection::Ability(..))),
                );
                self.drop_zones
                    .push((DropZone::Unit(owner, *id), card_rect));
                if self.ability_target_candidate(Target::Unit {
                    player: owner,
                    unit_id: *id,
                }) || self.card_target_candidate(Target::Unit {
                    player: owner,
                    unit_id: *id,
                })? {
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
                if self.game.trigger_target_bounds().is_some()
                    && self.game.valid_trigger_target(
                        Target::Unit {
                            player: owner,
                            unit_id: *id,
                        },
                        &self.targets,
                    )?
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
                    )?
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
                    self.hover_object_candidate = Some((owner, *id));
                    if token {
                        self.hover_token_candidate = Some(TokenPreview {
                            name: unit_name.clone(),
                            might: self.game.unit_might(unit)?,
                            gear: false,
                            rules: token_rules(card_id),
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
                            self.notify(token_rules(card_id));
                        } else {
                            self.inspect = Some(card_id);
                        }
                    }
                    if self.ui.primary_pressed()
                        && !self.targeting_battlefield()
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
                        if matches!(self.selection, Some(Selection::Ability(_, Some(_))))
                            || matches!(
                                self.game.decision,
                                Some(
                                    Decision::TriggerTargets { .. }
                                        | Decision::LegendTargets { .. }
                                )
                            )
                        {
                            self.toggle_target(Target::Unit {
                                player: owner,
                                unit_id: *id,
                            });
                        } else if matches!(
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
                                    self.toggle_target(target);
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
                let id = self.setup_champion(index);
                let card = self.game.catalog[id];
                self.board_card(
                    self.ui.logical(*rect),
                    id,
                    index,
                    true,
                    false,
                    false,
                    Might::from(card.might),
                    0,
                    false,
                    card.name,
                    false,
                    false,
                );
            }
        }
        for (i, r) in hits.battlefields.iter().enumerate() {
            if (!hit_unit || self.targeting_battlefield())
                && r.contains(mouse)
                && mouse.y > r.y + 63.0 * self.ui.scale
                && self.ui.enabled
                && self.ui.primary_pressed()
                && self.in_match
                && self.game.priority() == 0
                && self.game.phase == Phase::Playing
            {
                if self.targeting_battlefield() {
                    self.toggle_target(Target::Battlefield { lane: i });
                } else if let Some(Selection::Unit(id)) = self.selection {
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
                } else if matches!(
                    self.selection,
                    Some(Selection::Trash(_) | Selection::Pending(_))
                ) {
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
            self.match_panel()?;
            self.hand()?;
        } else {
            self.setup_panel();
            self.matchup();
        }
        if self.in_match && self.game.winner.is_some() {
            self.victory();
        }
        Ok(())
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
    fn response_ribbon(&mut self) -> Result<(), String> {
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
            let fields = chain_field_targets(&item);
            if let Some(progress) = self
                .game
                .resolution_progress()
                .filter(|progress| progress.chain_id == item.id)
            {
                self.ui.text(
                    &format!(
                        "Resolving {} / {}{}",
                        progress.execution + 1,
                        progress.executions,
                        fields
                            .get(progress.execution)
                            .map(|lane| format!(" · Field {}", lane + 1))
                            .unwrap_or_default()
                    ),
                    1178.0,
                    300.0,
                    9.0,
                    TEAL,
                );
            } else if !fields.is_empty() {
                self.ui.ellipsis(
                    &format!(
                        "Targets: {}",
                        fields
                            .iter()
                            .map(|lane| format!("Field {}", lane + 1))
                            .collect::<Vec<_>>()
                            .join(" > ")
                    ),
                    1178.0,
                    300.0,
                    212.0,
                    9.0,
                    GOLD,
                );
            } else if let Some(local_rift::game::ChainAbility::Hunt { amount, .. }) = &item.ability
            {
                self.ui.text(
                    &format!("Gain {amount} XP · amount fixed when triggered"),
                    1178.0,
                    300.0,
                    8.5,
                    TEAL,
                );
            } else if item.executions.len() > 1 {
                self.ui.text(
                    &format!(
                        "{} executions · targets chosen separately",
                        item.executions.len()
                    ),
                    1178.0,
                    300.0,
                    8.5,
                    TEAL,
                );
            }
            if self.ui.hover(r) {
                match self.game.chain_source(&item) {
                    local_rift::game::ChainSource::Unit { player, unit_id } => {
                        if let Some(unit) = self.game.players[player]
                            .units
                            .iter()
                            .find(|unit| unit.id == unit_id)
                        {
                            self.hover_candidate = Some(unit.card_id);
                            self.hover_object_candidate = Some((player, unit_id));
                            if unit.token {
                                self.hover_token_candidate = Some(TokenPreview {
                                    name: self.game.unit_name(unit),
                                    might: self.game.unit_might(unit)?,
                                    gear: false,
                                    rules: token_rules(unit.card_id),
                                });
                            }
                        }
                    }
                    local_rift::game::ChainSource::Card { card_id } => {
                        self.hover_candidate = Some(card_id)
                    }
                    local_rift::game::ChainSource::Gear { player, gear_id } => {
                        // The physical gear remains the source after departure.
                        self.hover_candidate = Some(item.card_id);
                        if self.game.players[player]
                            .gear
                            .iter()
                            .any(|gear| gear.id == gear_id)
                        {
                            self.hover_object_candidate = Some((player, gear_id));
                        }
                    }
                    local_rift::game::ChainSource::Legend { legend_id, .. } => {
                        self.legend_hover = Some(legend_id);
                        if self.ui.secondary_pressed() {
                            self.lesson = Some(legend_id);
                        }
                    }
                    local_rift::game::ChainSource::Battlefield { battlefield_id, .. } => {
                        self.field_hover = Some(battlefield_id);
                        if self.ui.secondary_pressed() {
                            self.field_inspect = Some(battlefield_id);
                        }
                    }
                }
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
                self.game.priority() == 0
                    && (self
                        .current_targets()
                        .contains(&Target::Spell { chain_id: item.id })
                        || self.card_target_candidate(Target::Spell { chain_id: item.id })?),
            ) {
                self.toggle_target(Target::Spell { chain_id: item.id });
            }
        } else if let Some(showdown) = &self.game.showdown {
            self.ui.text(
                &format!(
                    "{} · FIELD {}",
                    if matches!(self.game.decision, Some(Decision::CombatDamage { .. })) {
                        "COMBAT"
                    } else {
                        "SHOWDOWN"
                    },
                    showdown.lane + 1
                ),
                1178.0,
                252.0,
                11.0,
                GOLD,
            );
            self.ui.wrapped(
                if matches!(self.game.decision, Some(Decision::CombatDamage { .. })) {
                    "Both players assign damage before either side is hit. Spells and abilities wait until combat finishes."
                } else {
                    "Both players may respond before the field is settled. Pass when you're ready."
                },
                1178.0,
                279.0,
                209.0,
                11.0,
                MUTED,
            );
        }
        Ok(())
    }

    fn damage_assignment(
        &self,
        lane: usize,
        total: DamageAmount,
    ) -> Result<Vec<(u32, DamageAmount)>, String> {
        if let Some(amounts) = &self.damage_amounts {
            return Ok(amounts
                .iter()
                .copied()
                .filter(|(_, amount)| *amount > 0)
                .collect());
        }
        let suggested = self.game.automatic_damage_assignment(0, lane, total)?;
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
                    .unit_might(unit)?
                    .saturating_sub(unit.damage)
                    .max(1);
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
        Ok(allocation)
    }
    fn set_damage_amount(&mut self, id: u32, amount: DamageAmount) {
        if let Some(amounts) = &mut self.damage_amounts {
            self.toast_until = 0.0;
            if let Some((_, current)) = amounts.iter_mut().find(|(unit, _)| *unit == id) {
                *current = amount;
            } else {
                amounts.push((id, amount));
            }
        }
    }
    fn commit_damage_editor(&mut self) -> Result<(), String> {
        if let Some((id, value)) = &self.damage_editor {
            let amount = parse_damage_amount(value)?;
            self.set_damage_amount(*id, amount);
            self.damage_editor = None;
        }
        Ok(())
    }
    fn damage_validation(&self, lane: usize, total: DamageAmount) -> Result<(), String> {
        self.game.clone().apply(Action::AssignDamage {
            assignments: self.damage_assignment(lane, total)?,
        })
    }
    fn confirm_damage(&mut self, lane: usize, total: DamageAmount) {
        let result = self
            .commit_damage_editor()
            .and_then(|_| self.damage_validation(lane, total))
            .and_then(|_| self.damage_assignment(lane, total));
        match result {
            Ok(assignments) => self.action(Action::AssignDamage { assignments }),
            Err(error) => self.notify(error),
        }
    }
    fn combat_tray(&mut self, lane: usize, total: DamageAmount) -> Result<(), String> {
        let units = self.game.players[1]
            .units
            .iter()
            .filter(|u| u.lane == Some(lane))
            .cloned()
            .collect::<Vec<_>>();
        let manual = self.damage_amounts.is_some();
        if self.ui.button(
            Rect::new(780.0, 648.0, 170.0, 29.0),
            if manual {
                "Exact amounts  >"
            } else {
                "Edit amounts  >"
            },
            manual,
            true,
        ) {
            self.damage_amounts = if manual {
                None
            } else {
                Some(self.damage_assignment(lane, total)?)
            };
            self.damage_editor = None;
        }
        let allocation = self.damage_assignment(lane, total)?;
        self.hand_scroll = self.hand_scroll.min(units.len().saturating_sub(6));
        for (slot, unit) in units.iter().skip(self.hand_scroll).take(6).enumerate() {
            let r = self.tile(
                unit.card_id,
                slot,
                &self.game.unit_name(unit),
                if manual {
                    allocation.iter().any(|(id, _)| *id == unit.id)
                } else {
                    self.damage_order.contains(&unit.id)
                },
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
            let lethal = self
                .game
                .unit_might(unit)?
                .saturating_sub(unit.damage)
                .max(1);
            let keywords = self.game.unit_keywords(unit)?;
            let role = if keywords.tank && keywords.backline {
                " · FIRST / LAST"
            } else if keywords.tank {
                " · TANK"
            } else if keywords.backline {
                " · BACKLINE"
            } else {
                ""
            };
            let order = allocation
                .iter()
                .position(|(id, _)| *id == unit.id)
                .map(|n| format!("#{} · ", n + 1))
                .unwrap_or_default();
            self.ui
                .rect(Rect::new(r.x + 2.0, r.y + 68.0, 136.0, 28.0), PANEL);
            if amount > 999_999 {
                self.ui
                    .text(&format!("{order}DAMAGE"), r.x + 9.0, r.y + 78.0, 8.0, GOLD);
                self.ui
                    .text(&amount.to_string(), r.x + 9.0, r.y + 91.0, 9.0, GOLD);
            } else {
                self.ui.text(
                    &format!("{order}{amount} DAMAGE"),
                    r.x + 10.0,
                    r.y + 87.0,
                    10.0,
                    GOLD,
                );
            }
            self.ui
                .rect(Rect::new(r.x + 2.0, r.y + 124.0, 136.0, 53.0), PANEL);
            let lethal_label = format!("Lethal: {lethal}{role}");
            if self.ui.width(&lethal_label, 9.0) > 122.0 {
                self.ui
                    .text(&format!("LETHAL{role}"), r.x + 9.0, r.y + 129.0, 7.0, TEXT);
                self.ui
                    .text(&lethal.to_string(), r.x + 9.0, r.y + 141.0, 9.0, TEXT);
            } else {
                self.ui
                    .text(&lethal_label, r.x + 9.0, r.y + 136.0, 9.0, TEXT);
            }
            if manual {
                let step = if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                    10
                } else {
                    1
                };
                if self.ui.button(
                    Rect::new(r.x + 7.0, r.y + 143.0, 28.0, 28.0),
                    "-",
                    false,
                    amount > 0,
                ) {
                    self.damage_editor = None;
                    self.set_damage_amount(unit.id, amount.saturating_sub(step));
                }
                let editing = self.damage_editor.as_ref().filter(|(id, _)| *id == unit.id);
                let value = if editing.is_some() {
                    "Typing…".into()
                } else if self.ui.width(&amount.to_string(), 12.0) > 48.0 {
                    "Edit…".into()
                } else {
                    amount.to_string()
                };
                if self.ui.button(
                    Rect::new(r.x + 40.0, r.y + 143.0, 60.0, 28.0),
                    &value,
                    editing.is_some(),
                    true,
                ) {
                    while get_char_pressed().is_some() {}
                    self.damage_editor = Some((unit.id, String::new()));
                }
                if self.ui.button(
                    Rect::new(r.x + 105.0, r.y + 143.0, 28.0, 28.0),
                    "+",
                    false,
                    amount < total,
                ) {
                    self.damage_editor = None;
                    self.set_damage_amount(unit.id, amount.saturating_add(step).min(total));
                }
            } else {
                self.ui
                    .text("Click to choose order", r.x + 9.0, r.y + 161.0, 9.0, MUTED);
            }
            let card_face = Rect::new(r.x, r.y, r.w, 123.0);
            if self.ui.clicked(card_face) {
                if let Some(amounts) = &mut self.damage_amounts {
                    amounts.retain(|(id, _)| *id != unit.id);
                    amounts.push((unit.id, amount));
                    self.damage_editor = None;
                    self.toast_until = 0.0;
                } else if let Some(i) = self.damage_order.iter().position(|id| *id == unit.id) {
                    self.damage_order.remove(i);
                } else {
                    self.damage_order.push(unit.id);
                }
            }
            if self.ui.hover(card_face) {
                self.hover_candidate = Some(unit.card_id);
                self.hover_object_candidate = Some((1, unit.id));
            }
            if self.ui.hover(card_face) && self.ui.secondary_pressed() {
                self.inspect = Some(unit.card_id);
            }
        }
        self.tray_scroll(units.len(), 6);
        Ok(())
    }
    fn decision_panel(&mut self) -> Result<(), String> {
        use local_rift::legends::LegendTrigger;
        let Some(decision) = self.game.decision.clone() else {
            return Ok(());
        };
        if matches!(decision, Decision::PredictCards { player: 0, .. }) {
            self.predict_panel();
            return Ok(());
        }
        if self.game.resource_payment_pending()
            && matches!(
                self.selection,
                Some(
                    Selection::Gear(_)
                        | Selection::Legend
                        | Selection::Ability(..)
                        | Selection::Unit(_)
                )
            )
        {
            self.selection_panel()?;
            return Ok(());
        }
        if matches!(
            decision,
            Decision::TriggerTargets { .. } | Decision::LegendTargets { .. }
        ) {
            let source_name = match decision {
                Decision::TriggerTargets { card_id, .. } => self.game.catalog[card_id].name,
                Decision::LegendTargets { player } => self.game.legend_def(player).name,
                _ => unreachable!(),
            };
            let (maximum, required) = self.game.trigger_target_bounds().unwrap();
            self.ui
                .title("Choose ability targets", 1178.0, 366.0, 20.0, TEXT);
            self.ui
                .ellipsis(source_name, 1178.0, 392.0, 212.0, 12.0, GOLD);
            self.ui.wrapped_lines(
                &format!("{} / {maximum} selected. Choose highlighted {} in order, then confirm. Click a chosen target to undo.{}", self.targets.len(),
                    match self.game.trigger_target_rule().map(|r|r.kind) {
                        Some(local_rift::rules::PlayTargetKind::Gear) => "gear in either Loadout",
                        Some(local_rift::rules::PlayTargetKind::UnitOrGear) => "units or Loadout gear here",
                        _ => "units",
                    },
                    if required == 0 { " Choosing none declines." } else { " Choose all required targets." }),
                1178.0, 416.0, 209.0, 11.0, TEXT, 4,
            );
            if let Some(rule) = self.game.trigger_target_rule() {
                let limit = if let Some(location) = self.game.trigger_target_location() {
                    use local_rift::game::board_targets::BoardLocation;
                    let place = match location {
                        BoardLocation::Base { player: 0 } => "your base".to_owned(),
                        BoardLocation::Base { .. } => "opponent's base".to_owned(),
                        BoardLocation::Battlefield { lane } => format!("battlefield {}", lane + 1),
                    };
                    format!("Here: {place}")
                } else if let Some(energy) = rule.max_energy {
                    format!("Printed Energy: {energy} or less")
                } else if rule.relative_energy
                    == local_rift::trigger_rules::RelativeEnergyConstraint::AtMostSourceMight
                {
                    "Printed Energy ≤ source's current Might".into()
                } else {
                    String::new()
                };
                self.ui.ellipsis(&limit, 1178.0, 480.0, 209.0, 10.0, GOLD);
            }
            let action = Action::ChooseTriggerTargets {
                targets: self.targets.clone(),
            };
            let valid = self.game.clone().apply(action.clone()).is_ok();
            if self.ui.button(
                Rect::new(1178.0, 493.0, 212.0, 34.0),
                "Confirm targets",
                true,
                valid && !self.targets.is_empty(),
            ) {
                self.action(action);
            }
            if self.ui.button(
                Rect::new(1178.0, 534.0, 212.0, 27.0),
                if required == 0 {
                    "Decline this ability"
                } else {
                    "Clear selection"
                },
                false,
                true,
            ) {
                if required == 0 {
                    self.action(Action::ChooseTriggerTargets { targets: vec![] });
                } else {
                    self.targets.clear();
                }
            }
            return Ok(());
        }
        if matches!(decision, Decision::Payment { .. }) {
            self.locked_payment_panel()?;
            return Ok(());
        }
        if let Decision::ResourceConversion {
            card_id, direction, ..
        } = decision
        {
            self.conversion_panel(card_id, direction);
            return Ok(());
        }
        if let Decision::PendingPlay { pending_id, .. } = decision {
            if self.selection != Some(Selection::Pending(pending_id)) {
                self.selection = Some(Selection::Pending(pending_id));
                self.options = PlayOptions::default();
                self.flow_source = None;
                self.targets.clear();
                self.repeat_targets.clear();
                self.editing_repeat = false;
                self.destination = None;
            }
            self.selection_panel()?;
            if self.game.can_decline_trash_play()?
                && self.ui.button(
                    Rect::new(1178.0, 562.0, 212.0, 26.0),
                    "Cannot play · return to trash",
                    false,
                    true,
                )
            {
                self.action(Action::Choose { accept: false });
            }
            return Ok(());
        }
        if let Decision::RevivePlay { trash_index, .. } = decision {
            if self.selection != Some(Selection::Trash(trash_index)) {
                self.selection = Some(Selection::Trash(trash_index));
                self.options = PlayOptions::default();
                self.flow_source = None;
                self.targets.clear();
                self.destination = None;
            }
            self.selection_panel()?;
            return Ok(());
        }
        if matches!(
            decision,
            Decision::LastRites { .. } | Decision::ReplaySpell { .. } | Decision::ReviveUnit { .. }
        ) && matches!(self.selection, Some(Selection::Trash(_)))
        {
            self.selection_panel()?;
            return Ok(());
        }
        if let Decision::BattlefieldOrder { lanes, combat, .. } = &decision {
            self.ui
                .title("Choose a battlefield", 1178.0, 366.0, 20.0, TEXT);
            self.ui.wrapped_lines(if *combat {
                "Several combats are waiting. As the turn player, choose which battlefield opens first. The other fields stay pending."
            } else {
                "Several uncontested fields are waiting. Choose which showdown opens first; these resolve before pending combats."
            },1178.0,397.0,209.0,11.5,MUTED,4);
            for (slot, lane) in lanes.iter().enumerate() {
                let field = local_rift::battlefields::battlefield(self.game.battlefields[*lane]);
                if self.ui.button(
                    Rect::new(1178.0, 473.0 + slot as f32 * 42.0, 212.0, 34.0),
                    &format!("{} · {}", lane + 1, field.name),
                    false,
                    true,
                ) {
                    self.action(Action::ChooseBattlefield { lane: *lane });
                }
            }
            return Ok(());
        }
        if let Decision::TokenPlay {
            card_id,
            remaining,
            fixed_lane,
            ..
        } = decision
        {
            self.token_play_panel(card_id, remaining, fixed_lane)?;
            return Ok(());
        }
        if let Decision::TokenPlacement {
            card_id, remaining, ..
        } = decision
        {
            let token = self.game.current_pending_token();
            let name = local_rift::rules::token_spec(card_id).map_or("token", |spec| spec.name);
            self.ui
                .title(&format!("Place a {name}"), 1178.0, 366.0, 21.0, TEXT);
            self.ui.wrapped(
                &format!(
                    "{remaining} remaining. {}",
                    token.map_or_else(
                        || "Choose your base or a battlefield you control.".into(),
                        |token| play_ui::token_placement_description(&self.game, token)
                    )
                ),
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
                    self.game
                        .current_pending_token()
                        .and_then(|token| token.location)
                        .map_or_else(
                            || lane.is_none_or(|lane| self.game.battlefield_owner[lane] == Some(0)),
                            |fixed| fixed == lane,
                        ),
                ) {
                    self.action(Action::PlaceToken { lane });
                }
            }
            return Ok(());
        }
        if let Decision::CombatDamage { lane, total, .. } = decision {
            self.ui
                .title("Assign combat damage", 1178.0, 366.0, 20.0, TEXT);
            let allocation = self.damage_assignment(lane, total)?;
            let assigned: u128 = allocation
                .iter()
                .map(|(_, amount)| u128::from(*amount))
                .sum();
            let remainder = if assigned > u128::from(total) {
                format!("{} over", assigned - u128::from(total))
            } else {
                format!("{} remaining", u128::from(total) - assigned)
            };
            let compact = format!("{assigned} / {total} · {remainder}");
            let wide = self.ui.width(&compact, 12.0) > 212.0;
            if wide {
                for (line, text) in [
                    format!("Total: {total}"),
                    format!("Assigned: {assigned}"),
                    remainder,
                ]
                .iter()
                .enumerate()
                {
                    self.ui
                        .text(text, 1178.0, 389.0 + line as f32 * 15.0, 11.0, GOLD);
                }
            } else {
                self.ui.bold(&compact, 1178.0, 393.0, 12.0, GOLD);
            }
            let validation = self.damage_validation(lane, total);
            let mut dual_rank = false;
            for unit in self.game.players[1]
                .units
                .iter()
                .filter(|unit| unit.lane == Some(lane))
            {
                let keywords = self.game.unit_keywords(unit)?;
                dual_rank |= keywords.tank && keywords.backline;
            }
            if let Some((id, value)) = &self.damage_editor {
                let name = self.game.players[1]
                    .units
                    .iter()
                    .find(|unit| unit.id == *id)
                    .map(|unit| self.game.unit_name(unit))
                    .unwrap_or_else(|| "Selected unit".into());
                self.ui.ellipsis(
                    &format!("Damage to {name}"),
                    1178.0,
                    439.0,
                    212.0,
                    10.0,
                    TEXT,
                );
                let input = Rect::new(1178.0, 447.0, 212.0, 29.0);
                self.ui.rect(input, BG);
                self.ui.border(input, TEAL);
                let text = format!("{value}|");
                // The full u64 range fits; excessive input is visibly invalid, never truncated on commit.
                self.ui.ellipsis(
                    &text,
                    input.x + 6.0,
                    input.y + 20.0,
                    input.w - 12.0,
                    12.0,
                    TEXT,
                );
                self.ui
                    .text("Enter: set · Escape: cancel", 1178.0, 487.0, 9.0, MUTED);
            } else {
                let guidance = if let Err(error) = &validation {
                    format!(
                        "{error} Edit the amounts below; confirming keeps an invalid draft here."
                    )
                } else if dual_rank {
                    "Tank + Backline: choose first or last for that unit. Click art to change order; lethal before the next unit.".to_owned()
                } else if self.damage_amounts.is_some() {
                    "Legal allocation. Tank first, Backline last; lethal before the next unit. Click a number to type, or art to move it last.".to_owned()
                } else {
                    "Suggested amounts follow card order. Tank first, Backline last; lethal before the next unit. Both sides deal damage together.".to_owned()
                };
                self.ui.wrapped_lines(
                    &guidance,
                    1178.0,
                    if wide { 441.0 } else { 417.0 },
                    209.0,
                    11.0,
                    if validation.is_err() { GOLD } else { MUTED },
                    if wide { 3 } else { 4 },
                );
            }
            if self.ui.button(
                Rect::new(1178.0, 493.0, 212.0, 34.0),
                "Confirm damage",
                true,
                true,
            ) {
                self.confirm_damage(lane, total);
            }
            if self.ui.button(
                Rect::new(1178.0, 534.0, 212.0, 27.0),
                "Use suggested allocation",
                false,
                true,
            ) {
                self.damage_order.clear();
                self.damage_amounts = None;
                self.damage_editor = None;
            }
            return Ok(());
        }
        let (title,body)=match &decision{
            Decision::PredictCards{..}|Decision::Payment{..}|Decision::ResourceConversion{..}|Decision::TriggerTargets{..}|Decision::LegendTargets{..}|Decision::CombatDamage{..}|Decision::TokenPlacement{..}|Decision::TokenPlay{..}|Decision::BattlefieldOrder{..}|Decision::RevivePlay{..}|Decision::PendingPlay{..}=>unreachable!(),
            Decision::TrashTarget{..}=>("Choose a trash target","Choose an eligible card below before this ability enters the chain. Opponents may respond before the chosen card is played.".into()),
            Decision::OptionalPlay{card_id,..}=>("Use this triggered ability?",format!("{} offers a play from your trash. Accept and choose the ability's target, or decline now. Choose the card's own targets and costs after the ability resolves.",self.game.catalog[*card_id].name)),
            Decision::ResolutionTargets{..}=>("Choose the location", "Responses separated your targets. Choose which legal group below this spell should affect.".into()),
            Decision::OrderTriggers{..}=>("Order your triggers", "Choose which ability to put on the chain next. The last ability placed will resolve first. Pending triggers cannot be countered yet.".into()),
            Decision::OnPlayTarget{card_id,..}=>("Choose its target",format!("{} has entered play. Choose a highlighted unit for its ability, on the board or in the Board tab.",self.game.catalog[*card_id].name)),
            Decision::DeathReplacement{..}=>("Resolve a death", "Choose a unit and replacement from the cards below. This decides which gear or banishment changes the death.".into()),
            Decision::RevealHand{..}=>("Scuttle Crab's discovery","The opponent's hand is revealed below. You can also inspect their hidden cards in the Hidden tab until this turn ends.".into()),
            Decision::Battlefield{..}=>("Sunken Temple","You conquered with a Mighty unit. Pay 1 energy to draw a card?".into()),
            Decision::ReviveUnit{..}=>("Return to the fight","Choose a unit costing at most 3 energy and 1 power from your trash, or skip. The ability plays it at base when it resolves.".into()),
            Decision::SwapUnit{..}=>("Tideturner","Click another friendly unit at a different location to swap places, or skip. This movement does not exhaust either unit.".into()),
            Decision::EmpowerHere{..}=>("Empower here","Choose a unit at this location or your legend to empower for this turn, or skip.".into()),
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
            return Ok(());
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
                    Decision::Sabotage { .. }
                        | Decision::GrantFlow { .. }
                        | Decision::TrashTarget { .. }
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
                        self.game.can_decline_trash_play()?
                    }
                    Decision::GrantFlow { .. } => !self.game.players[0]
                        .discard
                        .iter()
                        .any(|id| self.game.catalog[*id].kind == CardKind::Spell),
                    Decision::TrashTarget {
                        player,
                        source_card_id,
                        ..
                    } => !(0..self.game.players[player].discard.len())
                        .any(|index| self.game.trash_target_valid(player, source_card_id, index)),
                    _ => true,
                },
            ) {
                if matches!(
                    decision,
                    Decision::Sabotage { .. }
                        | Decision::GrantFlow { .. }
                        | Decision::TrashTarget { .. }
                ) {
                    self.action(Action::SelectCards { indices: vec![] });
                } else if matches!(decision, Decision::SwapUnit { .. }) {
                    self.action(Action::ChooseTarget { target: None });
                } else {
                    self.action(Action::Choose { accept: false });
                }
            }
        }
        Ok(())
    }
    fn selected_card_id(&self) -> Option<usize> {
        match self.selection {
            Some(Selection::Hand(index)) => self.game.players[0].hand.get(index).copied(),
            Some(Selection::Trash(index)) => self.game.players[0].discard.get(index).copied(),
            Some(Selection::Pending(id)) => self
                .game
                .pending_cards()
                .iter()
                .find(|card| card.id == id && card.player == 0)
                .map(|card| card.card_id),
            Some(Selection::Champion) => self.game.players[0].champion,
            Some(Selection::Hidden(id)) => self.game.players[0]
                .hidden
                .iter()
                .find(|card| card.id == id)
                .map(|card| card.card_id),
            _ => None,
        }
    }
    fn toggle_target(&mut self, target: Target) {
        if let Err(error) = self.try_toggle_target(target) {
            self.notify(error);
        }
    }
    fn try_toggle_target(&mut self, target: Target) -> Result<(), String> {
        if let Some(Selection::Ability(source, Some(index))) = self.selection {
            if let Some(position) = self
                .ability_targets
                .iter()
                .position(|chosen| *chosen == target)
            {
                self.ability_targets.remove(position);
            } else if self.ability_target_candidate(target) {
                let maximum = self
                    .ability_rule(source, index)
                    .map_or(0, |(_, rule)| rule.targets.max as usize);
                if self.ability_targets.len() >= maximum {
                    self.ability_targets.clear();
                }
                self.ability_targets.push(target);
                self.toast_until = 0.0;
            } else {
                self.notify("Choose a highlighted unit that meets this ability's requirements.");
            }
            return Ok(());
        }
        if let Some((maximum, _)) = self.game.trigger_target_bounds() {
            if let Some(index) = self.targets.iter().position(|chosen| *chosen == target) {
                // Later slots may have different eligibility, so undo this and later choices.
                self.targets.truncate(index);
            } else if self.targets.len() >= maximum {
                self.notify("All target slots are filled. Click a chosen target to undo it.");
            } else if self.game.valid_trigger_target(target, &self.targets)? {
                self.targets.push(target);
                self.toast_until = 0.0;
            } else {
                self.notify("Choose a highlighted target for this slot. Check any Deflect cost.");
            }
            return Ok(());
        }
        if self.selection == Some(Selection::Legend) && !self.game.legend_target_valid(0, target) {
            self.notify("Choose a highlighted unit that this legend can target.");
            return Ok(());
        }
        let chosen = self.current_targets();
        if let Some(index) = chosen.iter().position(|existing| *existing == target) {
            // Later slots can depend on earlier choices; undo them together.
            self.current_targets_mut().truncate(index);
            return Ok(());
        }
        let maximum = self
            .selected_card_id()
            .map_or(1, |id| local_rift::rules::play_target_spec(id).maximum);
        if self.selected_card_id().is_some() && !self.card_target_candidate(target)? {
            self.notify("This target does not meet the card's requirements.");
            return Ok(());
        }
        if chosen.len() >= maximum {
            if maximum == 1 {
                self.current_targets_mut().clear();
            } else {
                self.notify(format!("This choice allows at most {maximum} target(s). Click a chosen target to undo it."));
                return Ok(());
            }
        }
        self.current_targets_mut().push(target);
        if matches!(target, Target::Battlefield { .. }) {
            self.destination = None;
        }
        self.toast_until = 0.0;
        Ok(())
    }
    fn targeting_battlefield(&self) -> bool {
        self.selected_card_id().is_some_and(|id| {
            local_rift::rules::play_target_spec(id).kind
                == Some(local_rift::rules::PlayTargetKind::Battlefield)
        })
    }
    fn card_target_candidate(&self, target: Target) -> Result<bool, String> {
        let Some((action, execution)) = self.selected_target_query(true)? else {
            return Ok(false);
        };
        Ok(
            optional_query(self.game.play_target_candidates(&action, execution))?
                .is_some_and(|candidates| candidates.contains(&target)),
        )
    }
    fn target_hint(&self, card_id: usize) -> String {
        use local_rift::rules::{Allegiance, PlayTargetKind, play_target_spec, targeting};
        if let Ok(Some((action, execution))) = self.selected_target_query(false) {
            match self.game.play_target_view(&action, execution) {
                Ok(view) => return play_ui::target_view_hint(&self.game, &view),
                Err(error) => return format!("Edit targets: {error}"),
            }
        }
        let spec = play_target_spec(card_id);
        let count = self.current_targets().len();
        match spec.kind {
            Some(PlayTargetKind::Battlefield) => self
                .current_targets()
                .first()
                .map(|target| format!("Target: {}", target_label(&self.game, *target)))
                .unwrap_or_else(|| "Choose either battlefield, even an empty one.".into()),
            Some(PlayTargetKind::Spell) => "Choose a pending spell above.".into(),
            Some(PlayTargetKind::Gear) => {
                format!("Choose gear in Loadout · {count}/{}", spec.maximum)
            }
            Some(PlayTargetKind::UnitOrGear) => {
                if matches!(
                    local_rift::rules::effect(card_id),
                    local_rift::rules::OfficialEffect::KillEnemyDomain { .. }
                ) {
                    "Choose an enemy Chaos unit (base or field), or Chaos gear in Loadout.".into()
                } else {
                    "Choose a battlefield unit, or gear in either Loadout.".into()
                }
            }
            Some(PlayTargetKind::Unit) => {
                let restriction = match targeting(card_id).allegiance {
                    Allegiance::Friendly => "friendly units",
                    Allegiance::Enemy => "enemy units",
                    Allegiance::Mixed => "friendly, then enemy",
                    _ => "units",
                };
                format!(
                    "{}{} {restriction} · {count}/{}",
                    if spec.required == 0 {
                        "Up to "
                    } else {
                        "Choose "
                    },
                    spec.maximum,
                    spec.maximum
                )
            }
            None => "No targets needed.".into(),
        }
    }
    fn targeting_selection(&self) -> bool {
        if self.game.trigger_target_bounds().is_some() {
            return true;
        }
        match self.selection {
            Some(Selection::Ability(source, Some(index))) => self
                .ability_rule(source, index)
                .is_some_and(|(_, rule)| rule.targets.max > 0),
            Some(Selection::Hand(i)) => self.game.players[0].hand.get(i).is_some_and(|id| {
                self.game.catalog[*id].kind != CardKind::Unit
                    && !local_rift::rules::on_play_requires_target(*id)
            }),
            Some(
                Selection::Trash(_)
                | Selection::Pending(_)
                | Selection::Hidden(_)
                | Selection::Gear(_),
            ) => true,
            Some(Selection::Legend) => {
                local_rift::legends::legend_activation(self.game.legend[0].kind)
                    .is_some_and(|ability| ability.targets.max > 0)
            }
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
    fn board_tray(&mut self) -> Result<(), String> {
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
                self.selection == Some(Selection::Unit(unit.id))
                    || self.group.contains(&unit.id)
                    || self.current_targets().contains(&Target::Unit {
                        player: *owner,
                        unit_id: unit.id,
                    }),
            );
            if self.game.trigger_target_bounds().is_some()
                && self.game.valid_trigger_target(
                    Target::Unit {
                        player: *owner,
                        unit_id: unit.id,
                    },
                    &self.targets,
                )?
            {
                self.ui.border(r, TEAL);
                self.ui.pill(r.x + 7.0, r.y + 7.0, "CHOOSE", TEAL);
            }
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
            let might = self.game.unit_might(unit)?;
            let label = format!(
                "{might} might · {}",
                if unit.ready { "ready" } else { "exhausted" }
            );
            if self.ui.width(&label, 9.0) > 122.0 {
                self.ui.text(
                    &format!("MIGHT · {}", if unit.ready { "ready" } else { "exhausted" }),
                    r.x + 9.0,
                    r.y + 154.0,
                    8.0,
                    MUTED,
                );
                self.ui
                    .text(&might.to_string(), r.x + 9.0, r.y + 169.0, 9.0, TEXT);
            } else {
                self.ui.text(&label, r.x + 9.0, r.y + 166.0, 9.0, MUTED);
            }
            if self.ui.hover(r) {
                self.hover_candidate = Some(unit.card_id);
                self.hover_object_candidate = Some((*owner, unit.id));
                if unit.token {
                    self.hover_token_candidate = Some(TokenPreview {
                        name: self.game.unit_name(unit),
                        might: self.game.unit_might(unit)?,
                        gear: false,
                        rules: token_rules(unit.card_id),
                    });
                }
            }
            if self.ui.hover(r) && self.ui.secondary_pressed() && !unit.token {
                self.inspect = Some(unit.card_id);
            }
            let gear_hovered = self.gear_badges(Rect::new(r.x, r.y, r.w, 112.0), *owner, unit.id);
            let source = AbilitySource::Unit {
                player: *owner,
                unit_id: unit.id,
            };
            let ability_button = !self.source_abilities(source).is_empty()
                && self.ui.button(
                    Rect::new(r.x + 67.0, r.y + 7.0, 66.0, 22.0),
                    "Abilities",
                    false,
                    true,
                );
            if ability_button {
                self.open_abilities(source);
            }
            if self.ability_target_candidate(Target::Unit {
                player: *owner,
                unit_id: unit.id,
            }) {
                self.ui.border(r, TEAL);
            }
            if self.ui.clicked(r) && self.game.priority() == 0 && !gear_hovered && !ability_button {
                let target = Target::Unit {
                    player: *owner,
                    unit_id: unit.id,
                };
                if matches!(self.selection, Some(Selection::Ability(_, Some(_))))
                    || matches!(
                        self.game.decision,
                        Some(Decision::TriggerTargets { .. } | Decision::LegendTargets { .. })
                    )
                {
                    self.toggle_target(target);
                } else if self.game.decision.is_some() {
                    self.action(Action::ChooseTarget {
                        target: Some(target),
                    });
                } else if self.targeting_selection() {
                    self.toggle_target(target);
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
        Ok(())
    }
    fn other_tray(&mut self) -> Result<(), String> {
        if self.ability_cost_tray() {
            return Ok(());
        }
        if self.tray == Tray::Board {
            self.board_tray()?;
            return Ok(());
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
            return Ok(());
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
            let champion = self.game.players[owner].champion;
            let legend = self.game.legend_def(owner);
            let id = legend.default_champion;
            let r = self.tile(
                id,
                0,
                "Legend power",
                self.selection == Some(Selection::Legend) && owner == 0,
            );
            if self.ui.clicked(r) && owner == 0 {
                self.remember_resource_draft();
                self.selection = Some(Selection::Legend);
                if !self.game.resource_payment_pending() {
                    self.targets.clear();
                }
            }
            self.ui
                .rect(Rect::new(r.x + 2.0, r.y + 2.0, 136.0, 96.0), BG);
            self.draw_legend_art(
                legend.id,
                Rect::new(r.x + 2.0, r.y + 2.0, 136.0, 96.0),
                true,
                WHITE,
            );
            self.ui.rect(
                Rect::new(r.x + 3.0, r.y + 72.0, r.w - 6.0, 24.0),
                Color::new(0.025, 0.06, 0.08, 0.9),
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
            self.ui
                .rect(Rect::new(r.x + 3.0, r.y + 128.0, r.w - 6.0, 47.0), PANEL);
            self.ui
                .ellipsis(legend.champion, r.x + 9.0, r.y + 143.0, 122.0, 10.0, TEXT);
            self.ui
                .ellipsis(legend.name, r.x + 9.0, r.y + 163.0, 122.0, 9.0, GOLD);
            if self.ui.hover(r) {
                self.legend_hover = Some(legend.id);
                if self.ui.secondary_pressed() {
                    self.lesson = Some(legend.id);
                }
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
                if gear.is_gold() {
                    self.ui
                        .rect(Rect::new(r.x + 2.0, r.y + 2.0, r.w - 4.0, 96.0), BG);
                    self.ui
                        .diamond(vec2(r.x + r.w / 2.0, r.y + 47.0), 26.0, GOLD);
                    self.ui
                        .rect(Rect::new(r.x + 3.0, r.y + 124.0, r.w - 6.0, 50.0), PANEL);
                    self.ui
                        .text("Gear token", r.x + 9.0, r.y + 140.0, 9.0, GOLD);
                    self.ui.text(
                        "Exhaust + kill: 1 Any power",
                        r.x + 9.0,
                        r.y + 161.0,
                        8.5,
                        MUTED,
                    );
                }
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
                if !gear.is_gold() {
                    self.ui
                        .rect(Rect::new(r.x + 3.0, r.y + 127.0, r.w - 6.0, 48.0), PANEL);
                    self.ui.text(
                        if gear.ready { "READY" } else { "EXHAUSTED" },
                        r.x + 9.0,
                        r.y + 144.0,
                        10.0,
                        if gear.ready { TEAL } else { MUTED },
                    );
                    self.ui.text(
                        if self.game.unit_empowered(gear.id) {
                            "EMPOWERED GEAR"
                        } else if self
                            .game
                            .gear_temporary_status(owner, gear.id)
                            .is_some_and(|status| status.active)
                        {
                            "TEMPORARY · next beginning"
                        } else if gear.attached_to.is_some() {
                            "Attached equipment"
                        } else {
                            "In base"
                        },
                        r.x + 9.0,
                        r.y + 165.0,
                        9.0,
                        if self.game.unit_empowered(gear.id) {
                            GOLD
                        } else {
                            MUTED
                        },
                    );
                }
                let target = Target::Gear {
                    player: owner,
                    gear_id: gear.id,
                };
                if self.ability_target_candidate(target)
                    || self.card_target_candidate(target)?
                    || (self.game.trigger_target_bounds().is_some()
                        && (self.targets.contains(&target)
                            || self.game.valid_trigger_target(target, &self.targets)?))
                {
                    self.ui.border(r, TEAL);
                    self.ui.pill(
                        r.x + 8.0,
                        r.y + 37.0,
                        if self.ability_targets.contains(&target) || self.targets.contains(&target)
                        {
                            "TARGETED"
                        } else {
                            "TARGET"
                        },
                        TEAL,
                    );
                }
                let source = AbilitySource::Gear {
                    player: owner,
                    gear_id: gear.id,
                };
                let ability_button = !self.source_abilities(source).is_empty()
                    && self.ui.button(
                        Rect::new(r.x + 67.0, r.y + 7.0, 66.0, 22.0),
                        "Abilities",
                        false,
                        true,
                    );
                if ability_button {
                    self.open_abilities(source);
                }
                if self.ui.clicked(r) && !ability_button {
                    if self.targeting_selection()
                        && !matches!(self.selection, Some(Selection::Gear(_)))
                    {
                        self.toggle_target(Target::Gear {
                            player: owner,
                            gear_id: gear.id,
                        });
                    } else if owner == 0 {
                        self.remember_resource_draft();
                        self.selection = Some(Selection::Gear(gear.id));
                        if !self.game.resource_payment_pending() {
                            self.targets.clear();
                            self.choices.clear();
                        }
                    }
                }
                if self.ui.hover(r) {
                    self.hover_candidate = Some(gear.card_id);
                    self.hover_object_candidate = Some((owner, gear.id));
                    if gear.is_gold() {
                        self.hover_token_candidate = Some(TokenPreview {
                            name: "Gold".into(),
                            might: 0,
                            gear: true,
                            rules: GOLD_RULES.into(),
                        });
                    }
                }
                if self.ui.hover(r) && self.ui.secondary_pressed() {
                    if gear.is_gold() {
                        self.notify(GOLD_RULES);
                    } else {
                        self.inspect = Some(gear.card_id);
                        self.hover_object = Some((owner, gear.id));
                    }
                }
            }
            self.tray_scroll(gears.len(), 4);
        } else {
            if self.ui.button(
                Rect::new(780.0, 648.0, 98.0, 29.0),
                match (self.gear_owner, self.banish_view) {
                    (0, false) => "Your trash >",
                    (0, true) => "Your banish >",
                    (_, false) => "AI trash >",
                    (_, true) => "AI banish >",
                },
                false,
                true,
            ) {
                self.gear_owner = 1 - self.gear_owner;
                self.hand_scroll = 0;
                self.selection = None;
            }
            if self.ui.button(
                Rect::new(884.0, 648.0, 92.0, 29.0),
                "Banish",
                self.banish_view,
                true,
            ) {
                self.banish_view = !self.banish_view;
                self.hand_scroll = 0;
                self.selection = None;
            }
            let owner = self.gear_owner;
            let cards = if self.banish_view {
                self.game.players[owner].banish.clone()
            } else {
                self.game.players[owner].discard.clone()
            };
            self.hand_scroll = self.hand_scroll.min(cards.len().saturating_sub(6));
            for (slot, (index, id)) in cards
                .iter()
                .enumerate()
                .skip(self.hand_scroll)
                .take(6)
                .enumerate()
            {
                let chosen = !self.banish_view
                    && (self.choices.contains(&index)
                        || self.selection == Some(Selection::Trash(index)));
                let r = self.tile(*id, slot, self.game.catalog[*id].name, chosen);
                if self.ui.clicked(r) {
                    if owner == 1 || self.banish_view {
                        self.inspect = Some(*id);
                    } else if matches!(self.selection, Some(Selection::Gear(_))) {
                        if let Some(i) = self.choices.iter().position(|x| *x == index) {
                            self.choices.remove(i);
                        } else if self.choices.len() < 2 {
                            self.choices.push(index);
                        }
                    } else {
                        self.selection = Some(Selection::Trash(index));
                        self.flow_source = None;
                        self.options = PlayOptions::default();
                        self.repeat_targets.clear();
                        self.destination = None;
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
                    match (owner, self.banish_view) {
                        (0, false) => "Your trash is empty.",
                        (0, true) => "You have no banished cards.",
                        (_, false) => "The opponent's trash is empty.",
                        (_, true) => "The opponent has no banished cards.",
                    },
                    270.0,
                    757.0,
                    23.0,
                    MUTED,
                );
            }
        }
        Ok(())
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
    fn choice_tray(&mut self) -> Result<bool, String> {
        let Some(decision) = self.game.decision.clone() else {
            return Ok(false);
        };
        if let Decision::PredictCards { player, cards, .. } = &decision {
            if *player == 0 {
                self.predict_tray(cards);
            }
            return Ok(true);
        }
        if matches!(
            decision,
            Decision::Payment { .. } | Decision::ResourceConversion { .. }
        ) {
            if let Some(card_id) = self
                .game
                .payment_plan_view(0)
                .and_then(|plan| plan.pending_card)
            {
                self.ui
                    .text("PENDING CARD · COST FIXED", 780.0, 671.0, 10.0, GOLD);
                let r = self.tile(card_id, 0, self.game.catalog[card_id].name, true);
                self.ui.pill(r.x + 8.0, r.y + 8.0, "BEING PAID", GOLD);
                if self.ui.hover(r) {
                    self.hover_candidate = Some(card_id);
                }
                if self.ui.hover(r) && self.ui.secondary_pressed() {
                    self.inspect = Some(card_id);
                }
                return Ok(true);
            }
            return Ok(false);
        }
        if matches!(
            decision,
            Decision::TriggerTargets { .. } | Decision::LegendTargets { .. }
        ) {
            if self.tray == Tray::Loadout {
                self.other_tray()?;
            } else {
                self.board_tray()?;
            }
            return Ok(true);
        }
        if let Decision::CombatDamage { lane, total, .. } = decision {
            self.combat_tray(lane, total)?;
            return Ok(true);
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
            return Ok(true);
        }
        if let Decision::OrderTriggers { options, .. } = decision {
            self.ui.text("Choose next trigger", 780.0, 671.0, 9.0, GOLD);
            self.hand_scroll = self.hand_scroll.min(options.len().saturating_sub(6));
            for (slot, option) in options.iter().skip(self.hand_scroll).take(6).enumerate() {
                let expiry = self.game.pending_temporary_source(option.id);
                let name = expiry.map_or(self.game.catalog[option.card_id].name, |(_, name)| name);
                let token = name != self.game.catalog[option.card_id].name;
                let r = self.tile(option.card_id, slot, name, false);
                if token {
                    self.ui
                        .rect(Rect::new(r.x + 2.0, r.y + 2.0, r.w - 4.0, 96.0), BG);
                    self.ui
                        .diamond(vec2(r.x + r.w / 2.0, r.y + 45.0), 22.0, TEAL);
                }
                self.ui
                    .rect(Rect::new(r.x + 3.0, r.y + 128.0, r.w - 6.0, 47.0), PANEL);
                self.ui
                    .wrapped_lines(&option.label, r.x + 9.0, r.y + 141.0, 122.0, 9.0, GOLD, 3);
                if self.ui.hover(r) {
                    self.hover_candidate = Some(option.card_id);
                    if token {
                        self.hover_token_candidate = Some(TokenPreview {
                            name: name.into(),
                            might: local_rift::rules::token_spec(option.card_id)
                                .map_or(0, |spec| u64::from(spec.might)),
                            gear: false,
                            rules: token_rules(option.card_id),
                        });
                    }
                }
                if self.ui.clicked(r) {
                    self.action(Action::OrderTrigger {
                        trigger_id: option.id,
                    });
                }
            }
            self.tray_scroll(options.len(), 6);
            return Ok(true);
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
                self.ui.ellipsis(
                    &format!(
                        "{} · {} might",
                        unit.lane
                            .map_or("Base".into(), |lane| format!("Field {}", lane + 1)),
                        self.game.unit_might(unit)?
                    ),
                    r.x + 9.0,
                    r.y + 174.0,
                    122.0,
                    8.5,
                    TEAL,
                );
                if self.ui.hover(r) {
                    self.hover_candidate = Some(card_id);
                    self.hover_object_candidate = Some((player, option.unit_id));
                }
                if self.ui.clicked(r) {
                    self.action(Action::ChooseDeathReplacement {
                        unit_id: option.unit_id,
                        gear_id: option.gear_id,
                    });
                }
            }
            self.tray_scroll(options.len(), 6);
            return Ok(true);
        }
        if let Decision::PendingPlay {
            pending_id,
            card_id,
            ..
        } = decision
        {
            self.ui
                .text("Finalize the chosen card", 780.0, 671.0, 9.0, GOLD);
            let r = self.tile(card_id, 0, self.game.catalog[card_id].name, true);
            if self.ui.clicked(r) {
                self.selection = Some(Selection::Pending(pending_id));
            }
            if self.ui.hover(r) {
                self.hover_candidate = Some(card_id);
                if self.ui.secondary_pressed() {
                    self.inspect = Some(card_id);
                }
            }
            return Ok(true);
        }
        if let Decision::TrashTarget {
            player,
            source_card_id,
            ..
        } = decision
        {
            let choices: Vec<_> = self.game.players[player]
                .discard
                .iter()
                .copied()
                .enumerate()
                .filter(|(index, _)| self.game.trash_target_valid(player, source_card_id, *index))
                .collect();
            self.ui
                .text("Choose the ability's trash target", 780.0, 671.0, 9.0, GOLD);
            self.hand_scroll = self.hand_scroll.min(choices.len().saturating_sub(6));
            for (slot, (index, card_id)) in
                choices.iter().skip(self.hand_scroll).take(6).enumerate()
            {
                let r = self.tile(*card_id, slot, self.game.catalog[*card_id].name, false);
                if self.ui.clicked(r) {
                    self.action(Action::SelectCards {
                        indices: vec![*index],
                    });
                }
                if self.ui.hover(r) {
                    self.hover_candidate = Some(*card_id);
                    if self.ui.secondary_pressed() {
                        self.inspect = Some(*card_id);
                    }
                }
            }
            if choices.is_empty() {
                self.ui.title(
                    "No eligible cards in your trash.",
                    270.0,
                    757.0,
                    23.0,
                    MUTED,
                );
            }
            self.tray_scroll(choices.len(), 6);
            return Ok(true);
        }
        if matches!(
            decision,
            Decision::ReviveUnit { .. } | Decision::RevivePlay { .. }
        ) {
            let choices: Vec<_> = self.game.players[0]
                .discard
                .iter()
                .copied()
                .enumerate()
                .filter(|(index, id)| match decision {
                    Decision::RevivePlay {
                        trash_index,
                        card_id,
                        ..
                    } => *index == trash_index && *id == card_id,
                    _ => {
                        let card = self.game.catalog[*id];
                        card.kind == CardKind::Unit && card.cost <= 3 && card.power_cost <= 1
                    }
                })
                .collect();
            self.ui.text(
                if matches!(decision, Decision::RevivePlay { .. }) {
                    "Play the chosen unit"
                } else {
                    "Choose a unit to revive"
                },
                780.0,
                671.0,
                9.0,
                GOLD,
            );
            self.hand_scroll = self.hand_scroll.min(choices.len().saturating_sub(6));
            for (slot, (index, id)) in choices.iter().skip(self.hand_scroll).take(6).enumerate() {
                let r = self.tile(
                    *id,
                    slot,
                    self.game.catalog[*id].name,
                    self.selection == Some(Selection::Trash(*index)),
                );
                if self.ui.clicked(r) && self.selection != Some(Selection::Trash(*index)) {
                    self.selection = Some(Selection::Trash(*index));
                    self.options = PlayOptions::default();
                    self.flow_source = None;
                    self.targets.clear();
                    self.repeat_targets.clear();
                    self.destination = None;
                }
                if self.ui.hover(r) {
                    self.hover_candidate = Some(*id);
                    if self.ui.secondary_pressed() {
                        self.inspect = Some(*id);
                    }
                }
            }
            self.tray_scroll(choices.len(), 6);
            if choices.is_empty() {
                self.ui.title(
                    "No eligible units in your trash.",
                    270.0,
                    757.0,
                    23.0,
                    MUTED,
                );
            }
            return Ok(true);
        }
        let cards = match &decision {
            Decision::Discard { .. } => self.game.players[0].hand.clone(),
            Decision::TopThree { cards, .. } | Decision::RevealHand { cards, .. } => cards.clone(),
            Decision::Sabotage { opponent, .. } => self.game.players[*opponent].hand.clone(),
            Decision::ReviveUnit { .. }
            | Decision::GrantFlow { .. }
            | Decision::LastRites { .. }
            | Decision::ReplaySpell { .. } => self.game.players[0].discard.clone(),
            _ => return Ok(false),
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
                        self.flow_source = None;
                        self.options = PlayOptions::default();
                        self.repeat_targets.clear();
                        self.destination = None;
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
        Ok(true)
    }
    fn legend_lesson(&mut self, id: usize) {
        let definition = local_rift::legend_catalog::legend(id).expect("registered legend");
        self.ui.rect(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            Color::new(0.0, 0.015, 0.02, 0.9),
        );
        self.ui.enabled = true;
        self.ui.rect(Rect::new(350.0, 140.0, 840.0, 623.0), PANEL);
        self.ui.border(Rect::new(350.0, 140.0, 840.0, 623.0), LINE);
        self.ui.pill(380.0, 170.0, "OFFICIAL LEGEND RULES", GOLD);
        self.ui
            .ellipsis(&legend_label(definition), 380.0, 237.0, 775.0, 28.0, TEXT);
        self.draw_legend_art(id, Rect::new(380.0, 263.0, 147.0, 206.0), false, WHITE);
        self.ui
            .wrapped(definition.text, 550.0, 283.0, 608.0, 14.0, TEXT);
        self.ui.text("PRACTICE PLAN", 380.0, 510.0, 11.0, TEAL);
        let notes = self
            .game
            .presets
            .iter()
            .find(|preset| preset.legend_id == id)
            .map(|preset| preset.learning_notes)
            .unwrap_or_else(|| legend_practice_tip(id));
        self.ui.wrapped(notes, 380.0, 541.0, 775.0, 13.0, MUTED);
        self.ui.wrapped("Legend rules and training ideas are separate from tournament deck results. Your custom list is editable, and the original four tournament presets keep their sourced lists.",380.0,644.0,770.0,11.0,GOLD);
        if self
            .ui
            .button(Rect::new(380.0, 701.0, 780.0, 38.0), "Back", true, true)
        {
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
        for (index, difficulty) in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ]
        .into_iter()
        .enumerate()
        {
            if self.ui.button(
                Rect::new(1178.0 + index as f32 * 73.0, 294.0, 66.0, 36.0),
                difficulty_label(difficulty),
                self.prefs.difficulty == difficulty,
                true,
            ) {
                self.prefs.difficulty = difficulty;
                self.prefs.save();
            }
        }
        self.ui.wrapped(
            difficulty_description(self.prefs.difficulty),
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
            "Tournament presets or your saved custom lists",
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
            let legend = self.setup_legend(side);
            self.draw_legend_art(
                legend.id,
                Rect::new(x, 693.0, 205.0, 163.0),
                true,
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
            self.ui.title(legend.champion, x + 218.0, 751.0, 26.0, TEXT);
            let deck_name = if self.prefs.use_custom[side] {
                self.custom_decks[side]
                    .as_ref()
                    .map(|d| format!("CUSTOM · {}", d.name))
                    .unwrap_or_else(|| p.name.to_owned())
            } else {
                p.name.to_owned()
            };
            self.ui.ellipsis(
                &deck_name,
                x + 218.0,
                775.0,
                330.0,
                12.0,
                if self.prefs.use_custom[side] {
                    TEAL
                } else {
                    MUTED
                },
            );
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
                self.select_preset(side, i);
            }
            if self
                .ui
                .button(Rect::new(x + 470.0, 796.0, 60.0, 36.0), ">", false, true)
            {
                let i = (deck + 1) % self.game.presets.len();
                self.select_preset(side, i);
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
                self.open_field_picker(FieldPickerTarget::Setup(side), self.fields[side]);
            }
        }
    }
    fn open_field_picker(&mut self, target: FieldPickerTarget, selected: usize) {
        self.field_picker = Some(FieldPicker {
            target,
            selected,
            list_scroll: selected.saturating_sub(10),
            rules_scroll: 0,
        });
        self.hover_card = None;
        self.hover_rect = None;
    }
    fn draw_field_picker(&mut self) {
        let Some(mut picker) = self.field_picker.clone() else {
            return;
        };
        let fields = &local_rift::battlefields::BATTLEFIELDS;
        self.ui.rect(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            Color::new(0.0, 0.02, 0.03, 0.88),
        );
        let r = Rect::new(320.0, 145.0, 800.0, 610.0);
        self.ui.rect(r, PANEL);
        self.ui.border(r, GOLD);
        self.ui
            .title("Choose a battlefield", 344.0, 188.0, 27.0, TEXT);
        self.ui.text(
            match picker.target {
                FieldPickerTarget::Setup(0) => "Your battlefield",
                FieldPickerTarget::Setup(_) => "Opponent's battlefield",
                FieldPickerTarget::Editor => "Custom deck battlefield",
            },
            344.0,
            215.0,
            12.0,
            GOLD,
        );
        self.ui.enabled = true;
        let list = Rect::new(344.0, 236.0, 280.0, 406.0);
        let max_scroll = fields.len().saturating_sub(11);
        if list.contains(self.ui.mouse()) {
            if self.ui.wheel() < 0.0 {
                picker.list_scroll = (picker.list_scroll + 1).min(max_scroll);
            }
            if self.ui.wheel() > 0.0 {
                picker.list_scroll = picker.list_scroll.saturating_sub(1);
            }
        }
        picker.list_scroll = picker.list_scroll.min(max_scroll);
        for (row, field) in fields.iter().skip(picker.list_scroll).take(11).enumerate() {
            if self.ui.button(
                Rect::new(344.0, 240.0 + row as f32 * 36.0, 280.0, 32.0),
                field.name,
                picker.selected == field.id,
                true,
            ) {
                picker.selected = field.id;
                picker.rules_scroll = 0;
            }
        }
        let field = local_rift::battlefields::battlefield(picker.selected);
        self.ui
            .wrapped_lines(field.name, 648.0, 268.0, 440.0, 24.0, TEXT, 2);
        self.ui.text("OFFICIAL RULES", 648.0, 326.0, 10.0, GOLD);
        if let Some(texture) = self.field_art.get(field.id).and_then(Option::as_ref) {
            let height = 150.0 * texture.height() / texture.width();
            self.ui
                .image(texture, Rect::new(648.0, 340.0, 150.0, height), None, WHITE);
        }
        let text_rect = Rect::new(814.0, 340.0, 274.0, 230.0);
        let wheel = self.ui.wheel_override;
        if !text_rect.contains(self.ui.mouse()) {
            self.ui.wheel_override = Some(0.0);
        }
        let overflow = self.ui.scroll_text(
            field.official_text,
            text_rect,
            14.0,
            &mut picker.rules_scroll,
        );
        self.ui.wheel_override = wheel;
        self.ui.wrapped(
            "Both players can contest this location. Its printed effect applies during the match.",
            648.0,
            604.0,
            430.0,
            12.0,
            MUTED,
        );
        if overflow > 0 {
            self.ui.text(
                "Scroll over the rules to read all text.",
                648.0,
                659.0,
                10.0,
                TEAL,
            );
        }
        if max_scroll > 0 {
            self.ui
                .text("Scroll the list for more fields.", 344.0, 659.0, 10.0, TEAL);
        }
        let cancel = self.ui.button(
            Rect::new(344.0, 698.0, 280.0, 34.0),
            "Back · keep current field",
            false,
            true,
        );
        let apply = self.ui.button(
            Rect::new(648.0, 698.0, 440.0, 34.0),
            "Use this battlefield",
            true,
            true,
        );
        if cancel {
            self.field_picker = None;
        } else if apply {
            match picker.target {
                FieldPickerTarget::Setup(owner) => self.fields[owner] = picker.selected,
                FieldPickerTarget::Editor => {
                    if let Some(editor) = &mut self.deck_editor {
                        editor.deck.battlefield = picker.selected;
                    }
                }
            }
            self.field_picker = None;
        } else {
            self.field_picker = Some(picker);
        }
    }
    fn draw_playmat(&mut self, hits: &scene::SceneHits) -> Result<(), String> {
        self.ui.victory_track(
            Rect::new(218.0, 110.0, 23.0, 527.0),
            self.game.players[0].score,
            "YOU",
            TEAL,
        );
        self.ui.victory_track(
            Rect::new(1132.0, 110.0, 23.0, 527.0),
            self.game.players[1].score,
            "AI",
            GOLD,
        );
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
            let target = Target::Battlefield { lane };
            let pending = !self.targeting_battlefield()
                && self
                    .game
                    .chain
                    .iter()
                    .rev()
                    .nth(self.chain_cursor)
                    .is_some_and(|item| chain_field_targets(item).contains(&lane));
            let selected = pending
                || (self.targeting_battlefield() && self.current_targets().contains(&target));
            let candidate = self.targeting_battlefield() && self.card_target_candidate(target)?;
            if self.in_match && (selected || candidate) {
                self.ui.border(
                    Rect::new(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0),
                    if selected { GOLD } else { TEAL },
                );
            }
            self.ui.ellipsis(
                &format!(
                    "{} · {}{}",
                    lane + 1,
                    field.name,
                    if pending {
                        " · PENDING"
                    } else if selected {
                        " · TARGET"
                    } else {
                        ""
                    }
                ),
                r.x + 12.0,
                r.y + 19.0,
                r.w - 88.0,
                13.0,
                if selected { GOLD } else { TEXT },
            );
            self.ui.wrapped_lines(
                field
                    .official_text
                    .split(" (")
                    .next()
                    .unwrap_or(field.official_text),
                r.x + 12.0,
                r.y + 38.0,
                r.w - 24.0,
                12.0,
                TEXT,
                2,
            );
            self.ui.line(
                vec2(r.x + 9.0, r.y + 62.0),
                vec2(r.x + r.w - 9.0, r.y + 62.0),
                1.0,
                LINE,
            );
            self.ui.text("AI", r.x + 11.0, r.y + 103.0, 9.0, GOLD);
            self.ui.text("YOU", r.x + 8.0, r.y + 188.0, 9.0, TEAL);
            self.ui.line(
                vec2(r.x + 9.0, r.y + 146.0),
                vec2(r.x + r.w - 9.0, r.y + 146.0),
                1.0,
                LINE,
            );
            if self
                .ui
                .hover(Rect::new(r.x + r.w - 71.0, r.y + 4.0, 64.0, 25.0))
            {
                self.field_hover = Some(field_id);
            }
            if self.ui.button(
                Rect::new(r.x + r.w - 71.0, r.y + 4.0, 64.0, 25.0),
                "Rules",
                false,
                true,
            ) || (self
                .ui
                .hover(Rect::new(r.x + r.w - 71.0, r.y + 4.0, 64.0, 25.0))
                && self.ui.secondary_pressed())
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
        Ok(())
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
                "Click Rules to keep this open.",
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
        let Some(gear) = gears.first() else {
            return false;
        };
        let chip = Rect::new(r.x, r.y + r.h - 19.0, r.w, 19.0);
        self.ui.rect(chip, Color::new(0.25, 0.20, 0.105, 1.0));
        self.ui.border(chip, GOLD);
        self.ui.diamond(vec2(chip.x + 7.0, chip.y + 9.5), 3.5, GOLD);
        let count_width = if gears.len() > 1 { 44.0 } else { 0.0 };
        self.ui.ellipsis(
            self.game.gear_name(gear),
            chip.x + 14.0,
            chip.y + 13.0,
            chip.w - 18.0 - count_width,
            10.0,
            Color::new(1.0, 0.88, 0.60, 1.0),
        );
        let list = Rect::new(chip.x + chip.w - count_width, chip.y, count_width, chip.h);
        if gears.len() > 1 {
            self.ui.rect(list, Color::new(0.35, 0.27, 0.12, 1.0));
            self.ui.bold(
                &format!("{} gear", gears.len()),
                list.x + 3.0,
                list.y + 13.0,
                10.0,
                TEXT,
            );
        }
        let hovered = self.ui.hover(chip);
        if hovered {
            self.hover_candidate = Some(if gears.len() > 1 && self.ui.hover(list) {
                gears[1].card_id
            } else {
                gear.card_id
            });
            self.hover_token_candidate = None;
            self.hover_object_candidate = Some((
                owner,
                if gears.len() > 1 && self.ui.hover(list) {
                    gears[1].id
                } else {
                    gear.id
                },
            ));
        }
        if self.ui.clicked(chip) || (hovered && self.ui.secondary_pressed()) {
            if gears.len() > 1 && self.ui.hover(list) {
                self.equipment_inspect = Some((owner, unit_id));
                self.equipment_scroll = 0;
                self.hover_card = None;
                self.hover_rect = None;
            } else {
                self.inspect = Some(gear.card_id);
                self.hover_object = Some((owner, gear.id));
            }
        }
        hovered
    }
    fn equipment_help(&mut self) {
        let Some((owner, unit_id)) = self.equipment_inspect else {
            return;
        };
        let Some(unit) = self.game.players[owner]
            .units
            .iter()
            .find(|unit| unit.id == unit_id)
        else {
            self.equipment_inspect = None;
            return;
        };
        let name = self.game.unit_name(unit);
        let gears: Vec<_> = self.game.players[owner]
            .gear
            .iter()
            .filter(|gear| gear.attached_to == Some(unit_id))
            .cloned()
            .collect();
        self.hover_card = None;
        self.hover_rect = None;
        self.ui.rect(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            Color::new(0.0, 0.02, 0.03, 0.82),
        );
        let r = Rect::new(400.0, 170.0, 640.0, 570.0);
        self.ui.rect(r, PANEL);
        self.ui.border(r, GOLD);
        self.ui.text("ATTACHED EQUIPMENT", 426.0, 204.0, 12.0, GOLD);
        self.ui.title(&name, 426.0, 242.0, 24.0, TEXT);
        self.ui.text(
            &format!(
                "{} pieces · abilities apply to this unit while attached",
                gears.len()
            ),
            426.0,
            269.0,
            12.0,
            MUTED,
        );
        let max_scroll = gears.len().saturating_sub(3);
        self.equipment_scroll = self.equipment_scroll.min(max_scroll);
        self.ui.enabled = true;
        for (slot, gear) in gears.iter().skip(self.equipment_scroll).take(3).enumerate() {
            let y = 288.0 + slot as f32 * 118.0;
            self.ui.rect(Rect::new(420.0, y, 600.0, 110.0), BG);
            if let Some(tex) = self.art.get(gear.card_id).and_then(Option::as_ref) {
                self.ui
                    .image(tex, Rect::new(428.0, y + 8.0, 62.0, 87.0), None, WHITE);
            }
            self.ui
                .bold(self.game.gear_name(gear), 505.0, y + 23.0, 14.0, GOLD);
            self.ui.wrapped_lines(
                self.game.catalog[gear.card_id].official_text,
                505.0,
                y + 44.0,
                398.0,
                11.0,
                TEXT,
                4,
            );
            if self.ui.button(
                Rect::new(927.0, y + 68.0, 80.0, 30.0),
                "Full card",
                false,
                true,
            ) {
                self.inspect = Some(gear.card_id);
                self.hover_object = Some((owner, gear.id));
                self.equipment_inspect = None;
            }
        }
        if self.ui.button(
            Rect::new(426.0, 657.0, 72.0, 28.0),
            "<",
            false,
            self.equipment_scroll > 0,
        ) {
            self.equipment_scroll = self.equipment_scroll.saturating_sub(3);
        }
        if self.ui.button(
            Rect::new(942.0, 657.0, 72.0, 28.0),
            ">",
            false,
            self.equipment_scroll < max_scroll,
        ) {
            self.equipment_scroll = (self.equipment_scroll + 3).min(max_scroll);
        }
        self.ui.text(
            &format!(
                "{}–{} of {}",
                self.equipment_scroll + 1,
                (self.equipment_scroll + 3).min(gears.len()),
                gears.len()
            ),
            682.0,
            676.0,
            11.0,
            MUTED,
        );
        if self.ui.button(
            Rect::new(426.0, 698.0, 588.0, 29.0),
            "Back to the mat",
            true,
            true,
        ) {
            self.equipment_inspect = None;
        }
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
        empowered: bool,
        temporary: bool,
        might: Might,
        damage: DamageAmount,
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
            let bounds = Rect::new(
                tex.width() * 0.08,
                tex.height() * 0.12,
                tex.width() * 0.84,
                tex.height() * 0.43,
            );
            // Fill the thumbnail proportionally, staying inside the artwork.
            let scale = (bounds.w / art.w).min(bounds.h / art.h);
            let crop = Rect::new(
                bounds.x + (bounds.w - art.w * scale) / 2.0,
                bounds.y + (bounds.h - art.h * scale) / 2.0,
                art.w * scale,
                art.h * scale,
            );
            self.ui.image(tex, art, Some(crop), WHITE);
        } else {
            self.ui.rect(art, Color::new(0.08, 0.20, 0.23, 1.0));
            self.ui
                .diamond(vec2(r.x + r.w / 2.0, r.y + 30.0), 16.0, color);
            self.ui.text("TOKEN", r.x + 9.0, r.y + 53.0, 8.0, TEXT);
        }
        self.ui
            .rect(Rect::new(r.x + 2.0, r.y + 3.0, r.w - 4.0, 3.0), color);
        let expanded_stats = might > 99 || damage > 99;
        if expanded_stats {
            let line = format!("M {might}");
            self.ui
                .rect(Rect::new(r.x + 3.0, r.y + 8.0, r.w - 20.0, 16.0), BG);
            self.ui
                .ellipsis(&line, r.x + 6.0, r.y + 20.0, r.w - 26.0, 9.0, TEXT);
            if damage > 0 {
                self.ui
                    .rect(Rect::new(r.x + 3.0, r.y + 25.0, r.w - 20.0, 15.0), BG);
                self.ui.ellipsis(
                    &format!("D {damage}"),
                    r.x + 6.0,
                    r.y + 36.0,
                    r.w - 26.0,
                    9.0,
                    Color::new(0.93, 0.43, 0.38, 1.0),
                );
            }
        } else {
            self.ui
                .rect(Rect::new(r.x + 3.0, r.y + 8.0, 22.0, 19.0), BG);
            self.ui
                .bold(&might.to_string(), r.x + 7.0, r.y + 22.0, 13.0, TEXT);
            if damage > 0 {
                self.ui.pill(
                    r.x + r.w - 31.0,
                    r.y + 9.0,
                    &format!("-{damage}"),
                    Color::new(0.93, 0.43, 0.38, 1.0),
                );
            }
        }
        if empowered {
            let (label, width) = if !expanded_stats && r.w >= 108.0 {
                ("EMP", 32.0)
            } else {
                ("E", 14.0)
            };
            let badge = Rect::new(
                if expanded_stats {
                    r.x + r.w - 17.0
                } else {
                    r.x + 29.0
                },
                r.y + if expanded_stats { 27.0 } else { 8.0 },
                width,
                16.0,
            );
            self.ui.rect(badge, BG);
            self.ui.border(badge, GOLD);
            self.ui
                .bold(label, badge.x + 3.0, badge.y + 12.0, 9.0, GOLD);
        }
        if temporary {
            let badge = Rect::new(r.x + r.w - 18.0, r.y + 8.0, 14.0, 14.0);
            self.ui.rect(badge, BG);
            self.ui.border(badge, GOLD);
            self.ui.bold("T", badge.x + 3.0, badge.y + 11.0, 9.0, GOLD);
        }
        self.ui.ellipsis(
            name.split(',').next().unwrap_or(name),
            r.x + 4.0,
            r.y + r.h - 10.0,
            r.w - 8.0,
            10.0,
            TEXT,
        );
        self.ui.rect(
            Rect::new(r.x + 3.0, r.y + r.h - 6.0, r.w - 6.0, 3.0),
            if ready { color } else { LINE },
        );
        if ready && !temporary {
            self.ui
                .diamond(vec2(r.x + r.w - 9.0, r.y + 14.0), 3.5, TEAL);
        } else if !ready {
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
                let spec = local_rift::rules::play_target_spec(*card_id);
                let maximum = spec.maximum;
                let field_target =
                    spec.kind == Some(local_rift::rules::PlayTargetKind::Battlefield);
                let movement = matches!(effect, E::MoveEnemy | E::MoveFriendlyAndReady);
                let mut targets = self.targets.clone();
                match zone {
                    DropZone::Battlefield(lane) if field_target => {
                        targets = vec![Target::Battlefield { lane }];
                    }
                    DropZone::Unit(_, _) if field_target && lane.is_some() => {
                        targets = vec![Target::Battlefield {
                            lane: lane.unwrap(),
                        }];
                    }
                    _ if field_target => {
                        return Err("Choose a battlefield. Your base is not a battlefield.".into());
                    }
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
                    return Err("Drop this spell on an eligible target.".into());
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
                let confirm = field_target
                    || maximum > 1
                    || movement
                    || self.options.repeat
                    || self.play_draft.is_some();
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
                if !self
                    .game
                    .card_target_candidates(0, card_id, lane, &targets[..index])?
                    .contains(target)
                {
                    return Err("This target is not legal for that card.".into());
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
                    if self.targeting_battlefield() {
                        *self.current_targets_mut() = targets;
                    } else {
                        self.targets = targets;
                    }
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
        if self.payment_draft.is_some() {
            return;
        }
        if self.inspect.is_some()
            || self.equipment_inspect.is_some()
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
                || self.hover_token_candidate != self.hover_token
                || self.hover_object_candidate != self.hover_object)
        {
            self.hover_card = self.hover_candidate;
            self.hover_token = self.hover_token_candidate.clone();
            self.hover_object = self.hover_object_candidate;
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
                &if token.gear {
                    "CREATED GEAR · RESOURCE ABILITY".into()
                } else {
                    format!("{} CURRENT MIGHT · CREATED UNIT", token.might)
                },
                r.x + 18.0,
                r.y + 172.0,
                11.0,
                GOLD,
            );
            let live = self
                .hover_object
                .and_then(|(owner, object_id)| {
                    live_object_description(&self.game, owner, object_id, id)
                })
                .unwrap_or_else(|| token.rules.clone());
            self.ui.scroll_text(
                &live,
                Rect::new(r.x + 18.0, r.y + 203.0, r.w - 36.0, 205.0),
                12.0,
                &mut self.hover_scroll,
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
            let art = Rect::new(r.x + 1.0, r.y + 1.0, r.w - 2.0, 111.0);
            let bounds = Rect::new(
                tex.width() * 0.08,
                tex.height() * 0.12,
                tex.width() * 0.84,
                tex.height() * 0.35,
            );
            let scale = (bounds.w / art.w).min(bounds.h / art.h);
            let crop = Rect::new(
                bounds.x + (bounds.w - art.w * scale) / 2.0,
                bounds.y + (bounds.h - art.h * scale) / 2.0,
                art.w * scale,
                art.h * scale,
            );
            self.ui.image(tex, art, Some(crop), WHITE);
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
                    format!(" · {} printed might", card.might)
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
        let printed = if notes.is_empty() {
            card.official_text.to_owned()
        } else {
            format!("{}\n\nPractice note\n{}", card.official_text, notes)
        };
        let text = if let Some(live) = self.hover_object.and_then(|(owner, object_id)| {
            live_object_description(&self.game, owner, object_id, id)
        }) {
            format!(
                "{live}\n\nPRINTED RULES\n{}",
                if printed.is_empty() {
                    "No printed abilities."
                } else {
                    &printed
                }
            )
        } else {
            printed
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
    fn payment_action_name(&self, action: &Action) -> String {
        if let Action::WithPlayChoices { action, .. } | Action::WithPowerPayment { action, .. } =
            action
        {
            return self.payment_action_name(action);
        }
        if matches!(action, Action::CompletePayment)
            && let Some(view) = self.game.payment_plan_view(0)
        {
            if let Some(id) = view.pending_card {
                return self.game.catalog[id].name.to_owned();
            }
            if let Some(original) = view.action {
                return self.payment_action_name(&original);
            }
        }
        if let Action::ActivateAbility {
            source,
            ability_index,
            ..
        }
        | Action::ActivateAbilityWithCosts {
            source,
            ability_index,
            ..
        } = action
            && let Some((id, rule)) = self.ability_rule(*source, *ability_index)
        {
            return format!("{} · {}", self.game.catalog[id].name, rule.label);
        }
        if let Action::ResolveConversion { amount } = action {
            return format!("Convert {amount} power into energy");
        }
        let card_id = match action {
            Action::PlayCard { hand_index, .. }
            | Action::PlayCardOptions { hand_index, .. }
            | Action::PlayCardWithTargets { hand_index, .. } => {
                self.game.players[0].hand.get(*hand_index).copied()
            }
            Action::PlayFromTrash { trash_index, .. }
            | Action::PlayFromTrashWithTargets { trash_index, .. }
            | Action::PlayFromTrashOptions { trash_index, .. } => {
                self.game.players[0].discard.get(*trash_index).copied()
            }
            _ => None,
        };
        card_id
            .map(|id| self.game.catalog[id].name.to_owned())
            .unwrap_or_else(|| "Your selected action".into())
    }
    fn payment_action(draft: &PaymentDraft) -> Action {
        Action::WithPowerPayment {
            action: Box::new(draft.action.clone()),
            payment: draft.chosen.clone(),
        }
    }
    fn confirm_payment(&mut self) {
        let Some(draft) = &self.payment_draft else {
            return;
        };
        let action = Self::payment_action(draft);
        if let Err(error) = self.game.clone().apply(action.clone()) {
            self.notify(error);
            return;
        }
        self.payment_draft = None;
        self.apply_action(action);
    }
    fn payment_counter<T: Copy + Into<u32> + TryFrom<u32>>(
        &self,
        x: f32,
        y: f32,
        label: &str,
        color: Color,
        selected: &mut T,
        available: T,
    ) {
        let value: u32 = (*selected).into();
        let available: u32 = available.into();
        self.ui.text(
            label,
            x,
            y + 20.0,
            11.0,
            if available > 0 { color } else { MUTED },
        );
        if self
            .ui
            .button(Rect::new(x + 166.0, y, 27.0, 28.0), "-", false, value > 0)
        {
            *selected = T::try_from(value.saturating_sub(1)).ok().unwrap();
        }
        self.ui.text(
            &format!("{value} / {available}"),
            x + 202.0,
            y + 20.0,
            if available > 99 { 9.0 } else { 11.0 },
            TEXT,
        );
        if self.ui.button(
            Rect::new(x + 260.0, y, 27.0, 28.0),
            "+",
            false,
            value < available,
        ) {
            *selected = T::try_from(value + 1).ok().unwrap();
        }
    }
    fn payment_editor(&mut self) {
        let Some(mut draft) = self.payment_draft.clone() else {
            return;
        };
        self.ui.rect(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            Color::new(0.0, 0.015, 0.02, 0.86),
        );
        self.ui.enabled = !draft.just_opened;
        draft.just_opened = false;
        let r = Rect::new(360.0, 160.0, 720.0, 590.0);
        self.ui.rect(r, PANEL);
        self.ui.border(r, GOLD);
        self.ui
            .title("Choose power sources", 388.0, 198.0, 25.0, TEXT);
        self.ui.ellipsis(
            &self.payment_action_name(&draft.action),
            388.0,
            223.0,
            640.0,
            13.0,
            GOLD,
        );
        let cost = draft
            .quote
            .costs
            .iter()
            .map(|cost| {
                let domains = if cost.domains.len() == 6 {
                    "any domain".into()
                } else {
                    cost.domains
                        .iter()
                        .map(|d| d.name())
                        .collect::<Vec<_>>()
                        .join(" / ")
                };
                format!(
                    "{} {}{}",
                    cost.amount,
                    domains,
                    if cost.spell_eligible { " (spell)" } else { "" }
                )
            })
            .collect::<Vec<_>>()
            .join(" + ");
        self.ui.wrapped_lines(
            &format!("Power required: {cost}. Energy is paid automatically."),
            388.0,
            247.0,
            660.0,
            11.0,
            MUTED,
            2,
        );
        self.ui.bold("GENERATED POWER", 388.0, 295.0, 10.0, GOLD);
        self.ui.bold("RUNES TO RECYCLE", 738.0, 295.0, 10.0, GOLD);
        for (i, domain) in POWER_DOMAINS.into_iter().enumerate() {
            let y = 309.0 + i as f32 * 36.0;
            self.payment_counter(
                388.0,
                y,
                domain.name(),
                rgb(domain.color()),
                &mut draft.chosen.colored[i],
                draft.quote.available.colored[i],
            );
            self.payment_counter(
                738.0,
                y,
                domain.name(),
                rgb(domain.color()),
                &mut draft.chosen.recycle[i],
                draft.quote.available.recycle[i],
            );
        }
        self.payment_counter(
            388.0,
            525.0,
            "Any power",
            TEXT,
            &mut draft.chosen.any,
            draft.quote.available.any,
        );
        self.payment_counter(
            388.0,
            561.0,
            "Spell-only power",
            TEAL,
            &mut draft.chosen.spell,
            draft.quote.available.spell,
        );
        self.ui.wrapped_lines(
            "Recycled runes return to the rune deck. Exhausted runes are used first.",
            738.0,
            545.0,
            285.0,
            11.0,
            MUTED,
            2,
        );
        let generate = self.ui.button(
            Rect::new(738.0, 581.0, 287.0, 28.0),
            "Generate resources · keep this payment",
            false,
            true,
        );
        let required = draft.quote.suggested.total();
        self.ui.bold(
            &format!("{} / {required} power selected", draft.chosen.total()),
            388.0,
            626.0,
            13.0,
            GOLD,
        );
        self.ui.text(
            "Each count shows selected / available",
            738.0,
            626.0,
            10.0,
            MUTED,
        );
        let validation = self.game.clone().apply(Self::payment_action(&draft));
        self.ui.wrapped_lines(
            validation
                .as_ref()
                .err()
                .map(String::as_str)
                .unwrap_or("Ready. Only these power sources will be spent."),
            388.0,
            650.0,
            660.0,
            11.0,
            if validation.is_ok() { TEAL } else { GOLD },
            2,
        );
        let changed = self
            .payment_draft
            .as_ref()
            .is_some_and(|old| old.chosen != draft.chosen);
        if changed {
            self.toast_until = 0.0;
        }
        self.payment_draft = Some(draft.clone());
        if generate {
            self.suspend_payment();
            return;
        }
        if self.ui.button(
            Rect::new(388.0, 692.0, 184.0, 34.0),
            "Cancel  [ESC]",
            false,
            true,
        ) {
            self.payment_draft = None;
        } else if self.ui.button(
            Rect::new(586.0, 692.0, 220.0, 34.0),
            "Use suggested sources",
            false,
            true,
        ) {
            self.payment_draft.as_mut().unwrap().chosen = draft.quote.suggested;
            self.toast_until = 0.0;
        } else if self.ui.button(
            Rect::new(820.0, 692.0, 232.0, 34.0),
            "Pay & continue",
            true,
            true,
        ) {
            self.confirm_payment();
        }
    }
    fn rune_popover(&mut self) {
        let Some(domain) = self.rune_menu else {
            return;
        };
        let r = Rect::new(self.rune_menu_at.x, self.rune_menu_at.y, 246.0, 220.0);
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
        let i = domain_index(domain);
        let exhausted = self.game.players[0].exhausted_runes[i];
        let ready = self.game.players[0].runes[i].saturating_sub(exhausted);
        self.ui.text(
            &format!("{ready} ready · {exhausted} exhausted"),
            r.x + 16.0,
            r.y + 48.0,
            10.0,
            MUTED,
        );
        for (index, (action, label)) in [
            (
                Action::ActivateRune {
                    domain,
                    recycle: false,
                },
                "Exhaust ready: +1 energy",
            ),
            (
                Action::RecycleRune {
                    domain,
                    exhausted: true,
                },
                "Recycle exhausted: +1 power",
            ),
            (
                Action::RecycleRune {
                    domain,
                    exhausted: false,
                },
                "Recycle ready: +1 power",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let valid =
                self.game.priority() == 0 && self.game.clone().apply(action.clone()).is_ok();
            if self.ui.button(
                Rect::new(r.x + 16.0, r.y + 61.0 + index as f32 * 42.0, 214.0, 34.0),
                label,
                false,
                valid,
            ) {
                self.rune_menu = None;
                self.action(action);
                break;
            }
        }
        self.ui.text(
            "Ready recycling gives no energy.",
            r.x + 16.0,
            r.y + 208.0,
            9.0,
            MUTED,
        );
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
        if self.payment_draft.is_some() {
            return;
        }
        if !self.in_match
            || self.screen != Screen::Practice
            || self.inspect.is_some()
            || self.lesson.is_some()
            || self.show_journal
            || self.confirm_restart
            || self.rune_menu.is_some()
            || self.equipment_inspect.is_some()
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
        self.ui.wrapped("Click a rune to store energy or recycle for matching power. Exhausted runes can still recycle. Choose payment mode below the turn button.",r.x+14.0,r.y+123.0,290.0,10.0,MUTED);
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
    fn match_panel(&mut self) -> Result<(), String> {
        self.ui.rect(Rect::new(1158.0, 108.0, 252.0, 770.0), PANEL);
        self.ui.border(Rect::new(1158.0, 108.0, 252.0, 770.0), LINE);
        self.ui.text(
            &format!(
                "TURN {:02}  ·  {} AI",
                self.game.turn,
                difficulty_label(self.game.difficulty).to_uppercase()
            ),
            1178.0,
            136.0,
            10.0,
            MUTED,
        );
        self.ui.bold("CONQUER · HOLD", 1178.0, 165.0, 13.0, TEXT);
        self.ui.wrapped(
            "Score by taking and holding battlefields. Reach 8 points and lead to win.",
            1178.0,
            187.0,
            212.0,
            11.0,
            MUTED,
        );
        self.ui
            .line(vec2(1178.0, 225.0), vec2(1390.0, 225.0), 1.0, LINE);
        if self.game.in_window() {
            self.response_ribbon()?;
        } else {
            self.ui.text("YOUR LEGEND", 1178.0, 245.0, 9.0, GOLD);
            self.ui.ellipsis(
                &legend_label(self.game.legend_def(0)),
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
            self.decision_panel()?;
        } else {
            self.selection_panel()?;
        }
        if self.game.phase == Phase::Playing
            && !matches!(
                self.game.decision,
                Some(Decision::Payment { .. } | Decision::ResourceConversion { .. })
            )
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
        if self.ui.button(
            Rect::new(1178.0, 623.0, 212.0, 21.0),
            if self.prefs.exact_payment {
                "Power: choose sources  >"
            } else {
                "Power: automatic  >"
            },
            self.prefs.exact_payment,
            true,
        ) {
            self.prefs.exact_payment = !self.prefs.exact_payment;
            self.prefs.save();
        }
        self.ui
            .line(vec2(1178.0, 647.0), vec2(1390.0, 647.0), 1.0, LINE);
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
                "Deck {} · Trash {} · AI hand {}",
                player.deck.len(),
                player.discard.len(),
                self.game.players[1].hand.len()
            ),
            1178.0,
            830.0,
            9.0,
            MUTED,
        );
        for owner in 0..2 {
            self.ui.text(
                &xp_status(&self.game, owner),
                1178.0,
                844.0 + owner as f32 * 13.0,
                9.0,
                if owner == 0 { TEAL } else { GOLD },
            );
        }
        self.ui.text(
            "Drag to play · hover to read · right-click to pin",
            1178.0,
            872.0,
            9.0,
            MUTED,
        );
        Ok(())
    }
    fn selection_panel(&mut self) -> Result<(), String> {
        match self.selection {
            Some(Selection::Ability(source, index)) => self.ability_panel(source, index)?,
            Some(Selection::Unit(id)) => {
                let source = AbilitySource::Unit {
                    player: 0,
                    unit_id: id,
                };
                if self.game.resource_payment_pending() && !self.source_abilities(source).is_empty()
                {
                    self.ability_panel(source, None)?;
                    return Ok(());
                }
                if let Some(unit) = self.game.players[0].units.iter().find(|u| u.id == id) {
                    let card_id = unit.card_id;
                    let ready = unit.ready;
                    let lane = unit.lane;
                    let ganking = self.game.unit_keywords(unit)?.ganking;
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
                            && (lane.is_none() || ganking)
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
                            && (lane.is_none() || ganking)
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
                        let source = AbilitySource::Unit {
                            player: 0,
                            unit_id: id,
                        };
                        let abilities = !self.source_abilities(source).is_empty();
                        if abilities
                            && self.ui.button(
                                Rect::new(1288.0, 533.0, 102.0, 27.0),
                                "Abilities >",
                                false,
                                true,
                            )
                        {
                            self.open_abilities(source);
                        }
                        if self.ui.button(
                            Rect::new(1178.0, 533.0, if abilities { 102.0 } else { 212.0 }, 27.0),
                            &if abilities {
                                "Group units".into()
                            } else if self.group.len() > 1 {
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
            | Some(Selection::Pending(_))
            | Some(Selection::Champion)) => {
                let index = match selection {
                    Some(Selection::Hand(i)) | Some(Selection::Trash(i)) => i,
                    _ => 0,
                };
                let champion = self.selection == Some(Selection::Champion);
                let from_trash = matches!(self.selection, Some(Selection::Trash(_)));
                let pending = if let Some(Selection::Pending(id)) = self.selection {
                    self.game
                        .pending_cards()
                        .into_iter()
                        .find(|card| card.id == id && card.player == 0)
                } else {
                    None
                };
                let card_id = if let Some(pending) = &pending {
                    Some(pending.card_id)
                } else if from_trash {
                    self.game.players[0].discard.get(index).copied()
                } else if champion {
                    self.game.players[0].champion
                } else {
                    self.game.players[0].hand.get(index).copied()
                };
                if let Some(card_id) = card_id {
                    let card = self.game.catalog[card_id];
                    let mut flows = if from_trash && self.game.decision.is_none() {
                        self.game.flow_costs(0, index)
                    } else {
                        Vec::new()
                    };
                    flows.sort_by_key(|flow| match flow.source {
                        FlowSource::Printed => 0,
                        FlowSource::Granted => 1,
                    });
                    if self
                        .flow_source
                        .is_some_and(|source| !flows.iter().any(|flow| flow.source == source))
                    {
                        self.flow_source = None;
                    }
                    if flows.len() == 1 {
                        self.flow_source = Some(flows[0].source);
                    }
                    let selected_flow = flows
                        .iter()
                        .find(|flow| Some(flow.source) == self.flow_source);
                    let selecting_revive =
                        matches!(self.game.decision, Some(Decision::ReviveUnit { .. }));
                    let resolving_revive =
                        matches!(self.game.decision, Some(Decision::RevivePlay { .. }))
                            || pending.as_ref().is_some_and(|card| {
                                card.kind == local_rift::game::InstructedPlayKind::Matriarch
                            });
                    let cost_text = if resolving_revive || selecting_revive {
                        "Base cost waived · extra costs still apply".into()
                    } else if pending
                        .as_ref()
                        .is_some_and(|card| card.kind == local_rift::game::InstructedPlayKind::Fizz)
                    {
                        "Base energy waived · pay power and extras".into()
                    } else if let Some(flow) = selected_flow {
                        let power = if flow.power == 0 {
                            String::new()
                        } else if flow.colored {
                            format!(" + {} matching power", flow.power)
                        } else {
                            format!(" + {} Any power", flow.power)
                        };
                        format!("Flow: {} energy{power}", flow.energy)
                    } else if flows.len() > 1 {
                        "Choose printed or granted Flow".into()
                    } else {
                        format!("{} ENERGY · {} POWER", card.cost, card.power_cost)
                    };
                    let plan = self
                        .play_draft
                        .as_ref()
                        .and_then(|_| self.selected_play_action())
                        .map(|action| {
                            optional_query(self.game.cost_plan(&self.canonical_action(action)))
                        })
                        .transpose()?
                        .flatten();
                    let cost_text = plan
                        .map(|plan| {
                            format!(
                                "TOTAL: {} energy · {} power",
                                plan.energy,
                                plan.power
                                    .iter()
                                    .map(|cost| u32::from(cost.amount))
                                    .sum::<u32>()
                            )
                        })
                        .unwrap_or(cost_text);
                    let discounts = self.game.players[0]
                        .units
                        .iter()
                        .filter(|unit| unit.card_id == 35 && !unit.token)
                        .count()
                        .min(255) as u8;
                    let has_discounts = discounts > 0
                        && (card_id == 29
                            || local_rift::rules::repeat_cost(card_id).is_some()
                            || local_rift::rules::accelerate_cost(card_id).is_some());
                    self.ui
                        .ellipsis(card.name, 1178.0, 359.0, 212.0, 16.0, TEXT);
                    self.ui
                        .text(&cost_text, 1178.0, 383.0, 9.0, rgb(card.domain.color()));
                    if !has_discounts || flows.len() < 2 {
                        self.ui.ellipsis(
                            &card_brief(&self.game, card_id),
                            1178.0,
                            406.0,
                            209.0,
                            10.0,
                            MUTED,
                        );
                    }
                    let entry_hint = entry_ready_hint(&self.game, 0, card_id);
                    self.ui.ellipsis(
                        entry_hint
                            .as_deref()
                            .unwrap_or("Hover the card to read its full text."),
                        1178.0,
                        423.0,
                        212.0,
                        9.0,
                        if entry_hint.is_some() { TEAL } else { MUTED },
                    );
                    if flows.len() > 1 {
                        for (slot, source) in [FlowSource::Printed, FlowSource::Granted]
                            .into_iter()
                            .enumerate()
                        {
                            if self.ui.button(
                                Rect::new(1178.0 + slot as f32 * 110.0, 430.0, 102.0, 20.0),
                                if source == FlowSource::Printed {
                                    "Printed Flow"
                                } else {
                                    "Granted Flow"
                                },
                                self.flow_source == Some(source),
                                true,
                            ) {
                                self.flow_source = Some(source);
                                self.toast_until = 0.0;
                            }
                        }
                    }
                    if has_discounts {
                        let y = if flows.len() > 1 { 394.0 } else { 432.0 };
                        let r = Rect::new(1178.0, y, 212.0, 18.0);
                        let power = self.game.optional_power_discount[0].min(discounts);
                        self.ui.border(r, LINE);
                        self.ui.text(
                            &format!("Discount: {power} power · {} energy  >", discounts - power),
                            1184.0,
                            y + 13.0,
                            9.0,
                            TEAL,
                        );
                        if self.ui.clicked(r) {
                            self.action(Action::SetOptionalDiscount {
                                power: (power + 1) % (discounts + 1),
                            });
                        }
                    }
                    if !self.play_choices_button()? {
                        if local_rift::rules::repeat_cost(card_id).is_some()
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
                            && local_rift::rules::repeat_cost(card_id).is_some()
                            && local_rift::rules::play_target_spec(card_id).maximum > 0
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
                        if !selecting_revive
                            && local_rift::rules::accelerate_cost(card_id).is_some()
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
                        if !from_trash
                            && pending.is_none()
                            && card.official_text.starts_with("[Hidden]")
                        {
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
                    let action = if pending.is_some() {
                        Action::FinalizePendingPlay {
                            lane: self.destination,
                            targets: self.targets.clone(),
                            repeat_targets: self.repeat_targets.clone(),
                            repeat: self.options.repeat,
                            accelerate: self.options.accelerate,
                            bonus: self.options.bonus,
                        }
                    } else if from_trash {
                        Action::PlayFromTrashOptions {
                            trash_index: index,
                            flow: self.flow_source,
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
                    let action = self.canonical_action(action);
                    let validation = self.game.clone().apply(action.clone());
                    let payable_now = validation.is_ok()
                        && self.game.card_supported(card_id)
                        && (flows.len() < 2 || self.flow_source.is_some());
                    let begin_payment = !payable_now
                        && self.game.card_supported(card_id)
                        && (flows.len() < 2 || self.flow_source.is_some())
                        && self.can_begin_payment(&action);
                    let affordable = payable_now || begin_payment;
                    let label = if selecting_revive {
                        "Choose this unit to revive"
                    } else if card.kind == CardKind::Unit {
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
                        if begin_payment {
                            "Begin payment · use resources"
                        } else {
                            label
                        },
                        false,
                        affordable,
                    ) {
                        if begin_payment {
                            self.begin_ui_payment(action);
                        } else {
                            self.action(action);
                        }
                    }
                    if !affordable {
                        let target_help = self.target_hint(card_id);
                        self.ui.wrapped_lines(
                            if !self.game.card_supported(card_id) {
                                "This card's effect is not implemented."
                            } else if flows.len() > 1 && self.flow_source.is_none() {
                                "Choose which Flow cost to pay."
                            } else if self.current_targets().len()
                                < local_rift::rules::play_target_spec(card_id).required
                            {
                                &target_help
                            } else {
                                validation
                                    .as_ref()
                                    .err()
                                    .map(String::as_str)
                                    .unwrap_or("Complete the play choices first.")
                            },
                            1178.0,
                            546.0,
                            212.0,
                            9.0,
                            GOLD,
                            2,
                        );
                    } else if card.kind == CardKind::Unit && !selecting_revive && !resolving_revive
                    {
                        for lane in 0..2 {
                            let action = if pending.is_some() {
                                Action::FinalizePendingPlay {
                                    lane: Some(lane),
                                    targets: self.targets.clone(),
                                    repeat_targets: self.repeat_targets.clone(),
                                    repeat: self.options.repeat,
                                    accelerate: self.options.accelerate,
                                    bonus: self.options.bonus,
                                }
                            } else if champion {
                                Action::PlayChampionOptions {
                                    lane: Some(lane),
                                    target: self.targets.first().copied(),
                                    accelerate: self.options.accelerate,
                                }
                            } else if from_trash {
                                Action::PlayFromTrashOptions {
                                    trash_index: index,
                                    flow: self.flow_source,
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
                        self.ui.wrapped_lines(
                            &self.target_hint(card_id),
                            1178.0,
                            542.0,
                            212.0,
                            10.0,
                            TEAL,
                            2,
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
                    let hint = if local_rift::rules::play_target_spec(card.card_id).maximum > 0 {
                        self.target_hint(card.card_id)
                    } else {
                        self.game.catalog[card.card_id].official_text.to_owned()
                    };
                    self.ui
                        .wrapped_lines(&hint, 1178.0, 391.0, 210.0, 10.0, MUTED, 4);
                    self.play_choices_button()?;
                    let action = self.canonical_action(Action::PlayHidden {
                        hidden_id: id,
                        targets: self.targets.clone(),
                    });
                    let payable = self.game.clone().apply(action.clone()).is_ok();
                    let begin = !payable && self.can_begin_payment(&action);
                    if self.ui.button(
                        Rect::new(1178.0, 516.0, 212.0, 38.0),
                        if begin {
                            "Reveal · begin payment"
                        } else {
                            "Reveal & play"
                        },
                        false,
                        payable || begin,
                    ) {
                        if begin {
                            self.begin_ui_payment(action);
                        } else {
                            self.action(action);
                        }
                    }
                } else {
                    self.selection = None;
                }
            }
            Some(Selection::Legend) => {
                let definition = self.game.legend_def(0);
                self.ui
                    .title(definition.champion, 1178.0, 363.0, 22.0, TEXT);
                self.ui
                    .wrapped_lines(definition.text, 1178.0, 392.0, 210.0, 11.0, MUTED, 6);
                let legend = &self.game.legend[0];
                let status = format!(
                    "{} · {}",
                    if legend.ready { "READY" } else { "EXHAUSTED" },
                    if legend.empowered {
                        "EMPOWERED"
                    } else {
                        "NOT EMPOWERED"
                    }
                );
                self.ui.text(&status, 1178.0, 487.0, 9.0, TEAL);
                if let Some(ability) = local_rift::legends::legend_activation(legend.kind) {
                    let action = Action::ActivateLegend {
                        target: self
                            .targets
                            .first()
                            .copied()
                            .filter(|_| ability.targets.max > 0),
                    };
                    let resources = optional_query(self.game.cost_plan(&action))?
                        .map(|plan| play_ui::total_label(&plan))
                        .unwrap_or_else(|| {
                            format!(
                                "Pay {} energy + {} {} power",
                                ability.energy,
                                ability.power,
                                ability.domain.map_or("Any", |domain| domain.name())
                            )
                        });
                    let mut costs = vec![resources];
                    if ability.exhaust {
                        costs.push("Exhaust".into());
                    }
                    if ability.disempower {
                        costs.push("Disempower".into());
                    }
                    self.ui
                        .wrapped_lines(&costs.join(" · "), 1178.0, 504.0, 211.0, 10.0, GOLD, 2);
                    if ability.targets.max > 0 {
                        let label = self
                            .targets
                            .first()
                            .and_then(|target| match target {
                                Target::Unit { player, unit_id } => self.game.players[*player]
                                    .units
                                    .iter()
                                    .find(|unit| unit.id == *unit_id)
                                    .map(|unit| format!("Target: {}", self.game.unit_name(unit))),
                                _ => None,
                            })
                            .unwrap_or_else(|| "Choose a highlighted unit".into());
                        self.ui.ellipsis(&label, 1178.0, 467.0, 211.0, 11.0, TEXT);
                    } else if ability.effect
                        == local_rift::legends::LegendActivationEffect::LegionEnergy
                    {
                        self.ui.text(
                            if self.game.cards_played_this_turn(0) > 0 {
                                "Legion: adds 1 energy"
                            } else {
                                "No Legion: adds 0 energy"
                            },
                            1178.0,
                            467.0,
                            11.0,
                            TEXT,
                        );
                    } else if legend.kind == local_rift::legends::LegendKind::Ezreal {
                        self.ui.text(
                            &format!(
                                "Enemy targets: {} / 2 this turn",
                                self.game.ezreal_progress(0)
                            ),
                            1178.0,
                            467.0,
                            11.0,
                            TEXT,
                        );
                    }
                }
                if self.ui.button(
                    Rect::new(1178.0, 534.0, 212.0, 28.0),
                    legend_activation_label(self.game.legend_id(0)),
                    false,
                    self.game
                        .clone()
                        .apply(Action::ActivateLegend {
                            target: if self.targeting_selection() {
                                self.targets.first().copied()
                            } else {
                                None
                            },
                        })
                        .is_ok(),
                ) {
                    self.action(Action::ActivateLegend {
                        target: if self.targeting_selection() {
                            self.targets.first().copied()
                        } else {
                            None
                        },
                    });
                }
            }
            Some(Selection::Gear(id)) => {
                let source = AbilitySource::Gear {
                    player: 0,
                    gear_id: id,
                };
                if !self.source_abilities(source).is_empty() {
                    self.ability_panel(source, None)?;
                    return Ok(());
                }
                if let Some(gear) = self.game.players[0]
                    .gear
                    .iter()
                    .find(|g| g.id == id)
                    .cloned()
                {
                    self.ui
                        .ellipsis(self.game.gear_name(&gear), 1178.0, 359.0, 210.0, 16.0, TEXT);
                    self.ui.wrapped_lines(
                        if gear.is_gold() {
                            GOLD_RULES
                        } else {
                            self.game.catalog[gear.card_id].official_text
                        },
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
                    } else if let Some(rule) = local_rift::rules::equipment(gear.card_id) {
                        let action = self.targets.first().copied().map(|target| {
                            if rule.recycle_trash > 0 {
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
                            }
                        });
                        let cost = action
                            .as_ref()
                            .map(|action| optional_query(self.game.cost_plan(action)))
                            .transpose()?
                            .flatten()
                            .map(|plan| play_ui::total_label(&plan))
                            .unwrap_or_else(|| equipment_cost_label(&rule));
                        self.ui.ellipsis(
                            &format!(
                                "{cost}{}",
                                if rule.equip_xp > 0 {
                                    format!(" · XP {} available", self.game.players[0].xp)
                                } else {
                                    String::new()
                                }
                            ),
                            1178.0,
                            473.0,
                            212.0,
                            10.0,
                            GOLD,
                        );
                        self.ui.text(
                            if gear.attached_to.is_some() {
                                "Already attached · Equip is inactive."
                            } else {
                                "Click a friendly unit, then attach."
                            },
                            1178.0,
                            490.0,
                            9.0,
                            TEAL,
                        );
                        if self.ui.button(
                            Rect::new(1178.0, 505.0, 212.0, 34.0),
                            if rule.recycle_trash > 0 {
                                "Attach · recycle selected 2"
                            } else {
                                "Attach to chosen unit"
                            },
                            false,
                            !self.targets.is_empty() && gear.attached_to.is_none(),
                        ) && let Some(target) = self.targets.first().copied()
                        {
                            self.action(if rule.recycle_trash > 0 {
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
                        if rule.recycle_trash > 0 {
                            self.ui.text(
                                &format!(
                                    "Choose {} trash cards in the Trash tab.",
                                    rule.recycle_trash
                                ),
                                1178.0,
                                554.0,
                                9.0,
                                MUTED,
                            );
                        }
                    } else {
                        self.ui.wrapped(
                            if (gear.is_gold() || matches!(local_rift::rules::effect(gear.card_id), local_rift::rules::OfficialEffect::PowerGear)) && !gear.ready {
                                "Exhausted. Its resource ability needs ready gear."
                            } else {
                                "This gear has no Equip ability. Its printed text describes when its ability applies."
                            },
                            1178.0,
                            490.0,
                            210.0,
                            11.0,
                            MUTED,
                        );
                    }
                } else {
                    self.selection = None;
                }
            }
            None => {
                self.ui.title("Make your move.", 1178.0, 365.0, 22.0, TEXT);
                let hint = if self.prefs.coach {
                    if !self.game.chain.is_empty() {
                        "You have priority. Play a Reaction or pass. Two passes resolve the newest spell or ability.".into()
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
        Ok(())
    }
    fn hand(&mut self) -> Result<(), String> {
        if self.draw_play_choices_tray()? {
            return Ok(());
        }
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
                    if matches!(tray, Tray::Loadout | Tray::Board) {
                        self.remember_resource_draft();
                    }
                    self.tray = *tray;
                    if *tray == Tray::Trash {
                        self.banish_view = false;
                    }
                    self.hand_scroll = 0;
                    let selecting_play_target = matches!(tray, Tray::Loadout | Tray::Board)
                        && self.selected_card_id().is_some_and(|card_id| {
                            matches!(
                                local_rift::rules::play_target_spec(card_id).kind,
                                Some(
                                    local_rift::rules::PlayTargetKind::Gear
                                        | local_rift::rules::PlayTargetKind::UnitOrGear
                                )
                            )
                        });
                    if !selecting_play_target
                        && (!matches!(self.selection, Some(Selection::Ability(..)))
                            && (*tray == Tray::Board
                                || (*tray == Tray::Trash && self.gear_owner == 1)
                                || !matches!(self.selection, Some(Selection::Gear(_))))
                            || matches!(self.selection, Some(Selection::Ability(..)))
                                && !matches!(tray, Tray::Board | Tray::Loadout | Tray::Trash))
                    {
                        self.selection = None;
                        if !matches!(
                            self.game.decision,
                            Some(Decision::TriggerTargets { .. } | Decision::LegendTargets { .. })
                        ) {
                            self.targets.clear();
                        }
                    }
                }
            }
            if self.game.decision.is_some()
                && self.game.priority() == 0
                && !(matches!(self.tray, Tray::Loadout | Tray::Board)
                    && self.game.resource_payment_pending())
                && self.choice_tray()?
            {
                return Ok(());
            }
            if self.tray != Tray::Hand {
                self.other_tray()?;
                return Ok(());
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
            let can = self.game.can_afford(0, &c)? && self.game.card_supported(id);
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
                    self.flow_source = None;
                    self.repeat_targets.clear();
                    self.editing_repeat = false;
                }
            }
        }
        if len == 0 {
            let payment_waiting = self.game.resource_payment_pending();
            self.ui.title(
                if matches!(
                    self.game.decision,
                    Some(Decision::TokenPlay { .. } | Decision::TokenPlacement { .. })
                ) {
                    "Finish your token plays."
                } else if payment_waiting {
                    "A resource choice is waiting."
                } else if self.game.in_window() {
                    "A response window is open."
                } else {
                    "An empty hand. A board full of possibility."
                },
                270.0,
                757.0,
                23.0,
                MUTED,
            );
            self.ui.text(
                if matches!(self.game.decision, Some(Decision::TokenPlay { .. } | Decision::TokenPlacement { .. })) {
                    "Choose each token's location and any optional cost in the panel on the right."
                } else if payment_waiting && self.game.priority()==0 {
                    "Use Board or Loadout for Add abilities, then finish the payment or conversion."
                } else if payment_waiting {
                    "Your opponent is choosing resources. Play continues when the payment is finished."
                } else if self.game.in_window() && self.game.priority() == 0 {
                    "Use an available response, or pass priority when you're ready."
                } else if self.game.in_window() {
                    "Your opponent has priority. You can respond when priority returns."
                } else {
                    "Move your ready units, or end your turn to draw."
                },
                270.0,
                789.0,
                12.0,
                MUTED,
            );
        }
        Ok(())
    }
    fn decks(&mut self) {
        self.header(
            "Find your playstyle.",
            "Tournament deck lists, ready for a little practice. Every matchup is yours to try.",
        );
        if self.ui.button(
            Rect::new(1192.0, 25.0, 218.0, 42.0),
            "Build with a legend  >",
            false,
            true,
        ) {
            self.legend_picker = Some(self.game.presets[self.browse_deck].legend_id);
        }
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
            self.lesson = Some(self.game.presets[self.browse_deck].legend_id);
        }
        if self.ui.button(
            Rect::new(265.0, 642.0, 320.0, 30.0),
            "Customize a copy of this list  >",
            false,
            true,
        ) {
            self.open_deck_editor(None);
        }
        for owner in 0..2 {
            if self.ui.button(
                Rect::new(265.0 + owner as f32 * 164.0, 682.0, 156.0, 28.0),
                if owner == 0 {
                    "Edit saved: You"
                } else {
                    "Edit saved: AI"
                },
                self.prefs.use_custom[owner],
                self.custom_decks[owner].is_some(),
            ) {
                self.open_deck_editor(Some(owner));
            }
        }
        if self.ui.button(
            Rect::new(265.0, 728.0, 152.0, 40.0),
            if self.player_deck == self.browse_deck && !self.prefs.use_custom[0] {
                "Your deck"
            } else {
                "Play as this"
            },
            true,
            !self.in_match,
        ) {
            self.select_preset(0, self.browse_deck);
        }
        if self.ui.button(
            Rect::new(429.0, 728.0, 156.0, 40.0),
            if self.opponent_deck == self.browse_deck && !self.prefs.use_custom[1] {
                "Opponent"
            } else {
                "Play against"
            },
            false,
            !self.in_match,
        ) {
            self.select_preset(1, self.browse_deck);
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
        self.ui.wrapped("Spells use a last-in, first-out chain. Reactions can respond to pending spells. Action cards work in an open showdown. Both players must pass. Legends, equipment, Hidden and trash replay are supported. Choose combat damage and spell targets yourself. Choose trigger order, replacements and power sources when they matter; automatic payment remains available.",267.0,709.0,718.0,12.0,MUTED);
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
                    _ => self.prefs.animation = !self.prefs.animation,
                };
                self.prefs.save();
            }
            if i < 2 {
                self.ui
                    .line(vec2(271.0, y + 85.0), vec2(1383.0, y + 85.0), 1.0, LINE);
            }
        }
        self.ui.bold("AI opponent", 271.0, 379.0, 16.0, TEXT);
        self.ui.text(
            "Choose a level for your next match. Your current game keeps its level.",
            271.0,
            407.0,
            12.0,
            MUTED,
        );
        self.ui.text(
            difficulty_description(self.prefs.difficulty),
            271.0,
            434.0,
            11.0,
            TEAL,
        );
        for (index, difficulty) in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ]
        .into_iter()
        .enumerate()
        {
            if self.ui.button(
                Rect::new(1045.0 + index as f32 * 112.0, 363.0, 104.0, 40.0),
                difficulty_label(difficulty),
                self.prefs.difficulty == difficulty,
                true,
            ) {
                self.prefs.difficulty = difficulty;
                self.prefs.save();
            }
        }
        self.ui
            .title("Built for a little practice.", 271.0, 529.0, 27.0, TEXT);
        self.ui.wrapped(&format!("A native Rust client. Your game and {} bundled card images work offline. Preferences and your win/loss record are saved on this device. Matches stay in memory while the app is open.", self.game.catalog.len()),271.0,566.0,700.0,13.0,MUTED);
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
        let printed = if notes.is_empty() {
            card.official_text.to_owned()
        } else {
            format!("{}\n\nPractice note\n{}", card.official_text, notes)
        };
        let text = if let Some(live) = self.hover_object.and_then(|(owner, object_id)| {
            live_object_description(&self.game, owner, object_id, id)
        }) {
            format!(
                "{live}\n\nPRINTED RULES\n{}",
                if printed.is_empty() {
                    "No printed abilities."
                } else {
                    &printed
                }
            )
        } else {
            printed
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
fn chain_field_targets(item: &local_rift::game::ChainItem) -> Vec<usize> {
    item.executions
        .iter()
        .filter_map(|execution| {
            execution.targets.iter().find_map(|target| {
                if let Target::Battlefield { lane } = target {
                    Some(*lane)
                } else {
                    None
                }
            })
        })
        .collect()
}

fn target_label(game: &Game, target: Target) -> String {
    match target {
        Target::Battlefield { lane } => game
            .battlefields
            .get(lane)
            .map(|id| {
                format!(
                    "Field {} · {}",
                    lane + 1,
                    local_rift::battlefields::battlefield(*id).name
                )
            })
            .unwrap_or_else(|| "Unavailable battlefield".into()),
        Target::Unit { player, unit_id } => game
            .players
            .get(player)
            .and_then(|state| state.units.iter().find(|unit| unit.id == unit_id))
            .map(|unit| game.unit_name(unit))
            .unwrap_or_else(|| "Unit left play".into()),
        Target::Gear { player, gear_id } => game
            .players
            .get(player)
            .and_then(|state| state.gear.iter().find(|gear| gear.id == gear_id))
            .map(|gear| game.gear_name(gear).to_owned())
            .unwrap_or_else(|| "Gear left play".into()),
        Target::Spell { chain_id } => game
            .chain
            .iter()
            .find(|item| item.id == chain_id)
            .map(|item| game.chain_name(item))
            .unwrap_or_else(|| "Spell left chain".into()),
    }
}

fn xp_status(game: &Game, owner: usize) -> String {
    format!(
        "{} XP {}{}",
        if owner == 0 { "Your" } else { "Opponent" },
        game.players[owner].xp,
        if game.gained_xp_this_turn(owner) {
            " · + this turn"
        } else {
            ""
        }
    )
}

fn equipment_cost_label(rule: &local_rift::rules::EquipmentRule) -> String {
    let mut costs = Vec::new();
    if rule.equip_energy > 0 {
        costs.push(format!("{} energy", rule.equip_energy));
    }
    if rule.equip_power > 0 {
        costs.push(format!(
            "{} {} power",
            rule.equip_power,
            rule.equip_domain.name()
        ));
    }
    if rule.equip_xp > 0 {
        costs.push(format!("{} XP", rule.equip_xp));
    }
    if rule.recycle_trash > 0 {
        costs.push(format!("recycle {} trash cards", rule.recycle_trash));
    }
    format!(
        "Equip: {}",
        if costs.is_empty() {
            "no resource cost".into()
        } else {
            costs.join(" + ")
        }
    )
}

fn entry_ready_hint(game: &Game, owner: usize, card_id: usize) -> Option<String> {
    use local_rift::rules::EntryReadyRule;
    Some(match local_rift::rules::entry_ready_rule(card_id)? {
        EntryReadyRule::Always => "Printed rule: enters ready".into(),
        EntryReadyRule::ControllerXpAtLeast(threshold) => format!(
            "Level {threshold} entry: {} · XP {}",
            if game.printed_entry_ready(owner, card_id) {
                "ready"
            } else {
                "exhausted"
            },
            game.players[owner].xp
        ),
    })
}

fn card_brief(game: &Game, id: usize) -> String {
    use local_rift::rules::OfficialEffect::*;
    match local_rift::rules::effect(id) {
        KillGear { controller_draw: 0 } => "Kill a gear".into(),
        KillGear { controller_draw } => {
            format!("Kill gear · its controller draws {controller_draw}")
        }
        KillAllGear => "Kill all gear · including attached equipment".into(),
        KillEnemyDomain { domain } => format!("Kill an enemy {} unit or gear", domain.name()),
        TokensAndDraw { count, might, draw } => {
            format!("{count} × {might} Might Mech at base · draw {draw}")
        }
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
        ContextualMight(local_rift::rules::SpellMight::ControllerXp {
            minimum_xp,
            below,
            at_least,
        }) => format!("{below:+} might · Level {minimum_xp}: {at_least:+} instead"),
        Stun => "Stun a unit this turn".into(),
        KillAllUnits => "Kill all units · both players, everywhere".into(),
        Channel {
            count,
            exhausted,
            fallback_draw,
        } => format!(
            "Channel {count}{}{}",
            if exhausted { " exhausted" } else { " ready" },
            if fallback_draw > 0 {
                format!(" · if fewer, draw {fallback_draw}")
            } else {
                String::new()
            }
        ),
        FriendlyMight(amount) => format!("Friendly units: {amount:+} might this turn"),
        ReadyFriendlyAndMight(amount) => format!("Ready your units · {amount:+} might this turn"),
        ExhaustFriendlyAndDamage { amount, .. } => {
            format!("Exhaust yours · deal {amount} to all field units")
        }
        GrantAssault(amount) => format!("Grant Assault {amount} this turn"),
        Kill {
            battlefield_only,
            max_might,
        } => format!(
            "Kill {}{}",
            if battlefield_only {
                "a battlefield unit"
            } else {
                "a unit"
            },
            max_might
                .map(|might| format!(" · might {might} or less"))
                .unwrap_or_default()
        ),
        AreaDamage { amount, selector } => format!(
            "Deal {amount} · {}",
            match selector {
                local_rift::rules::AreaSelector::AllBattlefields => "all units at battlefields",
                local_rift::rules::AreaSelector::AllBases => "all units at bases",
                local_rift::rules::AreaSelector::EnemyAtTargetBattlefield =>
                    "enemies at chosen field",
            }
        ),
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
fn legend_label(legend: &local_rift::legend_catalog::LegendDef) -> String {
    format!("{} · {}", legend.champion, legend.name)
}
fn legend_activation_label(id: usize) -> &'static str {
    match id {
        2 => "Grant Assault 2",
        3 => "Add 1 spell-only power",
        4 => "Exhaust · Legion: +1 energy",
        5 => "Pay 1 energy · create Recruit",
        7 => "Pay & ready a unit",
        9 => "Exhaust · draw 1",
        _ => "Passive / triggered ability",
    }
}
fn legend_practice_tip(id: usize) -> &'static str {
    match id {
        8 => {
            "Every unit play, including a token, triggers Rengar. Choose a unit for +1 Might until this turn ends. Concentrate bonuses where they change a combat."
        }
        9 => {
            "Choose enemy units or gear twice this turn with spells or unit abilities, then exhaust Ezreal to draw. Repeat can supply separate choices. Gear and legend abilities do not count."
        }
        7 => {
            "When you Empower any other object, Ambessa triggers her own Empower. Her ready ability spends Empowered, exhausts her and costs 1 Any power; choose a useful unit before committing."
        }
        4 => {
            "Play a cheap first card to enable Legion. Darius's legend can then add energy for your next play or response; keep track of resources across the whole turn."
        }
        5 => {
            "Viktor can turn one energy into a Recruit during your own open Main Phase. Consider where an extra unit can defend a point before committing it."
        }
        6 => {
            "Lux rewards spells whose printed energy cost is at least five. Early defenders help you reach those plays; a discounted spell can still meet that printed threshold."
        }
        _ => {
            "Use your legend's printed ability to plan your resources and next scoring opportunity."
        }
    }
}
fn difficulty_label(difficulty: Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Easy => "Relaxed",
        Difficulty::Tactical => "Tactical",
        Difficulty::Strategic => "Strategic",
    }
}

fn difficulty_description(difficulty: Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Easy => "Simple, forgiving choices. Perfect for your first few matches.",
        Difficulty::Tactical => "Plans trades and protects points. A little more room to grow.",
        Difficulty::Strategic => "Looks ahead through possible plays using public information.",
    }
}

/// Keep live status tied to the exact object and printed card being inspected.
fn live_object_description(
    game: &Game,
    owner: usize,
    object_id: u32,
    card_id: usize,
) -> Option<String> {
    let player = game.players.get(owner)?;
    if player
        .units
        .iter()
        .any(|unit| unit.id == object_id && unit.card_id == card_id)
    {
        live_unit_description(game, owner, object_id)
    } else if player
        .gear
        .iter()
        .any(|gear| gear.id == object_id && gear.card_id == card_id)
    {
        live_gear_description(game, owner, object_id)
    } else {
        None
    }
}

fn live_gear_description(game: &Game, owner: usize, gear_id: u32) -> Option<String> {
    let gear = game
        .players
        .get(owner)?
        .gear
        .iter()
        .find(|gear| gear.id == gear_id)?;
    let mut lines = vec![
        format!(
            "ON BOARD · {} · {}",
            if owner == 0 {
                "Your gear"
            } else {
                "Opponent's gear"
            },
            if gear.ready { "Ready" } else { "Exhausted" }
        ),
        if game.unit_empowered(gear.id) {
            "EMPOWERED · available for effects or costs that check this state.".into()
        } else {
            "Not Empowered.".into()
        },
    ];
    if !gear.token
        && let Some(rule) = local_rift::rules::equipment(gear.card_id)
    {
        lines.push(format!(
            "{} · attached bearer +{} Might.",
            equipment_cost_label(&rule),
            rule.might
        ));
    }
    if let Some(unit) = gear
        .attached_to
        .and_then(|id| game.players[owner].units.iter().find(|unit| unit.id == id))
    {
        lines.push(format!("Attached to {}. Its printed gear abilities are inactive while attached; its equipment effects apply to the bearer.", game.unit_name(unit)));
        let contributions =
            match game.passive_contributions(local_rift::empower::BoardObject::Unit {
                player: owner,
                unit_id: unit.id,
            }) {
                Ok(contributions) => contributions,
                Err(error) => return Some(format!("Cannot calculate equipped effects: {error}")),
            };
        for contribution in contributions.iter().filter(|contribution| {
            contribution.source
                == local_rift::empower::BoardObject::Gear {
                    player: owner,
                    gear_id,
                }
        }) {
            lines.push(passive_description(game, contribution));
        }
    } else {
        lines.push("Unattached in base.".into());
    }
    if let Some(status) = game.gear_temporary_status(owner, gear_id) {
        if status.active {
            if status.granted {
                lines.push("Granted Temporary remains active even while attached. It expires the gear, not the bearer.".into());
            }
            lines.push("TEMPORARY · At the start of its controller's Beginning Phase, before hold scoring, it triggers an ability to kill this gear. Players can respond before the ability resolves; check the chain for an ability already waiting.".into());
        } else if status.intrinsic {
            lines.push("Printed Temporary is inactive while this gear is attached. A separately granted Temporary ability would remain active.".into());
        }
    }
    if gear.token {
        lines.push(GOLD_RULES.into());
    } else {
        for rule in local_rift::activated::activated_rules(gear.card_id) {
            lines.push(format!(
                "ABILITY · {}\n{} · {}{}",
                rule.label,
                ability_ui::cost_label(rule),
                ability_ui::timing_label(rule),
                if rule.only_if_not_empowered {
                    " · only while not Empowered; no chosen target"
                } else {
                    ""
                }
            ));
            if let Some(level) = ability_ui::level_label(game, owner, rule) {
                lines.push(level);
            }
            if let Some(output) = ability_ui::output_label(rule) {
                lines.push(format!(
                    "{output}. This uses the same physical source as its other abilities."
                ));
            }
        }
    }
    Some(lines.join("\n\n"))
}

fn passive_description(
    game: &Game,
    contribution: &local_rift::continuous::PassiveContribution,
) -> String {
    use local_rift::continuous::{PassiveEffect, PassiveKeyword, Recipients, SourceCondition};
    let effect = match contribution.effect {
        PassiveEffect::Might(amount) => format!("{amount:+} might"),
        PassiveEffect::MightPerFriendlyTemporaryHere { amount } => format!(
            "{:+} might · +{amount} for each friendly Temporary unit at this battlefield",
            contribution.computed_might.unwrap_or(0)
        ),
        PassiveEffect::Keyword(PassiveKeyword::Assault(amount)) => format!("Assault {amount}"),
        PassiveEffect::Keyword(PassiveKeyword::Shield(amount)) => format!("Shield {amount}"),
        PassiveEffect::Keyword(PassiveKeyword::Deflect(amount)) => format!("Deflect {amount}"),
        PassiveEffect::Keyword(PassiveKeyword::Ganking) => "Ganking".into(),
    };
    let requirement = local_rift::continuous::passive_rules(contribution.card_id)
        .get(contribution.rule_index)
        .map(|rule| {
            let controller = &game.players[contribution.source.player()];
            let condition = match rule.condition {
                SourceCondition::AttackingWithAnother => {
                    " · while attacking with another unit".into()
                }
                SourceCondition::CombatAlone => " · while attacking or defending alone".into(),
                SourceCondition::Empowered => " · while source is Empowered".into(),
                SourceCondition::NotEmpowered => " · while source is not Empowered".into(),
                SourceCondition::Buffed => " · while the unit has a Buff counter".into(),
                SourceCondition::OtherFriendlyHere => {
                    " · while another friendly unit is at its location".into()
                }
                SourceCondition::ControllerRunesAtLeast(threshold) => format!(
                    " · {threshold}+ runes in play (currently {}; ready and exhausted both count)",
                    controller
                        .runes
                        .iter()
                        .map(|&count| u16::from(count))
                        .sum::<u16>()
                ),
                SourceCondition::ControllerXpAtLeast(threshold) => format!(
                    " · Level {threshold}: {threshold}+ XP (currently {})",
                    controller.xp
                ),
                SourceCondition::ControllerGainedXpThisTurn => format!(
                    " · controller gained XP this turn: {} (current XP {}; the gain history resets each turn)",
                    if game.gained_xp_this_turn(contribution.source.player()) { "yes" } else { "no" },
                    controller.xp
                ),
                SourceCondition::Always => String::new(),
            };
            let recipients = match rule.recipients {
                Recipients::OtherFriendlyUnitsHere => {
                    " · other friendly units at the source's location"
                }
                Recipients::FriendlyTokenUnits => " · friendly unit tokens, at any location",
                Recipients::AttachedBearer => " · attached bearer only",
                _ => "",
            };
            format!("{condition}{recipients}")
        })
        .unwrap_or_default();
    format!(
        "{} · {}: {}{}{}",
        if contribution.active {
            "ACTIVE"
        } else {
            "INACTIVE"
        },
        game.catalog[contribution.card_id].name,
        effect,
        requirement,
        if contribution.active {
            " (included in current values when applicable)"
        } else {
            " (condition not met)"
        }
    )
}

fn token_identity_description(card_id: usize) -> String {
    let Some(spec) = local_rift::rules::token_spec(card_id) else {
        return "Created unit.".into();
    };
    format!(
        "{} · {} printed Might · domainless{}.",
        spec.name,
        spec.might,
        spec.tag.map_or(String::new(), |tag| format!(" · {tag}"))
    )
}
fn token_rules(card_id: usize) -> String {
    let mut text = token_identity_description(card_id);
    if let Some(spec) = local_rift::rules::token_spec(card_id) {
        if spec.deflect {
            text.push_str(" Deflect: opponents pay 1 extra power to choose this unit with a spell or ability.");
        }
        if spec.temporary {
            text.push_str(" Temporary: at the start of its controller's Beginning Phase, before hold scoring, it triggers an ability to kill this unit.");
        }
    }
    text
}

fn live_unit_description(game: &Game, owner: usize, unit_id: u32) -> Option<String> {
    let unit = game
        .players
        .get(owner)?
        .units
        .iter()
        .find(|unit| unit.id == unit_id)?;
    let stats = match game.try_unit_stats(unit) {
        Ok(stats) => stats,
        Err(error) => return Some(format!("Cannot calculate this unit's stats: {error}")),
    };
    let keywords = stats.keywords;
    let mut lines = vec![
        format!(
            "ON BOARD · {} · {}",
            if owner == 0 {
                "Your unit"
            } else {
                "Opponent's unit"
            },
            if unit.ready { "Ready" } else { "Exhausted" }
        ),
        if stats.might <= 999_999 && unit.damage <= 999_999 {
            format!("{} current might · {} damage", stats.might, unit.damage)
        } else {
            format!(
                "Current might: {}\nMarked damage: {}",
                stats.might, unit.damage
            )
        },
        if game.unit_empowered(unit.id) {
            "EMPOWERED · current might and keywords include active effects.".into()
        } else {
            "Not Empowered.".into()
        },
    ];
    if unit.token {
        lines.push(token_identity_description(unit.card_id));
    }
    if stats.signed_might < 0 {
        lines.push(format!(
            "Calculated might: {}. Rules that reference Might use 0 while it is negative.",
            stats.signed_might
        ));
    }
    if game.buffed_units.contains(&unit_id) {
        lines.push("BUFFED · has a Buff counter. Its +1 might is included above; temporary might and Empowered are separate.".into());
    }
    let contributions = match game.passive_contributions(local_rift::empower::BoardObject::Unit {
        player: owner,
        unit_id,
    }) {
        Ok(contributions) => contributions,
        Err(error) => return Some(format!("Cannot calculate this unit's effects: {error}")),
    };
    for contribution in contributions {
        lines.push(passive_description(game, &contribution));
    }
    if let Some(status) = game.temporary_status(local_rift::empower::BoardObject::Unit {
        player: owner,
        unit_id,
    }) && status.active
    {
        let origin = match (status.intrinsic, status.granted) {
            (true, true) => "printed and granted",
            (true, false) => "printed",
            _ => "granted",
        };
        lines.push(format!("TEMPORARY · {origin}. At the start of its controller's Beginning Phase, before hold scoring, it triggers an ability to kill this unit. Players can respond; check the chain for an ability already waiting. Moving or recalling this unit keeps Temporary."));
    }
    if keywords.hunt > 0 {
        lines.push(format!("Hunt {}: when this unit conquers or holds, one Hunt ability gains {} XP. Printed and equipped Hunt add together.", keywords.hunt, keywords.hunt));
    }
    if unit.stunned {
        lines.push("Stunned: contributes no might to combat damage this turn; still needs lethal damage equal to its full might.".into());
    }
    if unit.protected {
        lines.push("Protected from being chosen by enemy spells and abilities this turn.".into());
    }
    if keywords.assault > 0 {
        lines.push(format!(
            "Assault {}: +{} might while attacking; already included above while this unit is attacking.",
            keywords.assault, keywords.assault
        ));
    }
    if keywords.shield > 0 {
        lines.push(format!(
            "Shield {}: +{} might while defending in combat; already included above while this unit is defending.",
            keywords.shield, keywords.shield
        ));
    }
    if keywords.tank && keywords.backline {
        lines.push("Tank + Backline: the player assigning combat damage chooses whether this unit is treated as first or last, never the middle group.".into());
    } else if keywords.tank {
        lines.push("Tank: must be assigned combat damage before units without Tank.".into());
    }
    if keywords.backline && !keywords.tank {
        lines.push("Backline: assigned combat damage after units without Backline.".into());
    }
    if keywords.deflect > 0 {
        lines.push(format!("Deflect {}: opponents pay {} extra Any power to choose this unit with a spell or ability.", keywords.deflect, keywords.deflect));
    }
    if keywords.ganking {
        lines.push("Ganking: can move from one battlefield to another.".into());
    }
    if keywords.ambush {
        lines.push(
            "Ambush, when played: may play as a Reaction at a battlefield where you have a unit."
                .into(),
        );
    }
    if let Some(cost) = keywords.accelerate {
        lines.push(format!(
            "Accelerate, when played: +{} energy and {} {} power to enter ready.",
            cost.energy,
            cost.power,
            cost.domain.name()
        ));
    }
    if !unit.token {
        if let Some(hint) = entry_ready_hint(game, owner, unit.card_id) {
            lines.push(format!("PRINTED ENTRY RULE · {hint}. Checked when it enters; this does not ready a unit already in play. Optional Accelerate is separate."));
        }
        for rule in local_rift::activated::activated_rules(unit.card_id) {
            lines.push(format!(
                "ABILITY · {}
{} · {}{}",
                rule.label,
                ability_ui::cost_label(rule),
                ability_ui::timing_label(rule),
                if rule.requires_battlefield {
                    " · source must be at a battlefield"
                } else {
                    ""
                }
            ));
            if rule.xp_cost > 0 {
                lines.push(format!("Current XP: {}. XP is spent before responses; losing a Level threshold can change live bonuses immediately.", game.players[owner].xp));
            }
        }
    }
    let gear: Vec<_> = game.players[owner]
        .gear
        .iter()
        .filter(|gear| gear.attached_to == Some(unit_id))
        .collect();
    if !gear.is_empty() {
        lines.push("ATTACHED EQUIPMENT".into());
        for attachment in gear {
            lines.push(format!(
                "{}: {}",
                game.gear_name(attachment),
                game.catalog[attachment.card_id].official_text
            ));
        }
    }
    Some(lines.join("\n"))
}

#[cfg(test)]
mod preference_tests {
    use super::{Difficulty, Preferences};

    #[test]
    fn missing_or_invalid_preferences_have_safe_defaults() {
        let empty = Preferences::from_text("");
        assert!(empty.coach && empty.animation);
        assert!(!empty.advanced && !empty.exact_payment);
        assert_eq!(empty.difficulty, Difficulty::Easy);
        let invalid = Preferences::from_text("wins=oops\nlosses=-1\nunknown=true\n");
        assert_eq!((invalid.wins, invalid.losses), (0, 0));
    }

    #[test]
    fn preferences_round_trip_without_losing_unrelated_settings() {
        let original = Preferences::from_text(
            "coach=false\nadvanced=true\nanimation=false\ntactical=true\nexact_payment=true\ncustom_you=true\ncustom_ai=false\nwins=17\nlosses=9\n",
        );
        let restored = Preferences::from_text(&original.to_text());
        assert!(!restored.coach && restored.advanced && !restored.animation);
        assert!(restored.exact_payment);
        assert_eq!(restored.difficulty, Difficulty::Tactical);
        assert_eq!(restored.use_custom, [true, false]);
        assert_eq!((restored.wins, restored.losses), (17, 9));
    }

    #[test]
    fn old_tactical_setting_migrates_and_new_setting_takes_precedence() {
        assert_eq!(
            Preferences::from_text("tactical=true\n").difficulty,
            Difficulty::Tactical
        );
        assert_eq!(
            Preferences::from_text("tactical=false\n").difficulty,
            Difficulty::Easy
        );
        assert_eq!(
            Preferences::from_text("tactical=true\ndifficulty=easy\n").difficulty,
            Difficulty::Easy
        );
        assert_eq!(
            Preferences::from_text("tactical=false\ndifficulty=strategic\n").difficulty,
            Difficulty::Strategic
        );
    }

    #[test]
    fn every_difficulty_round_trips_with_an_explicit_name() {
        for (key, difficulty) in [
            ("easy", Difficulty::Easy),
            ("tactical", Difficulty::Tactical),
            ("strategic", Difficulty::Strategic),
        ] {
            let prefs = Preferences::from_text(&format!("difficulty={key}\n"));
            assert_eq!(prefs.difficulty, difficulty);
            let saved = prefs.to_text();
            assert!(saved.contains(&format!("difficulty={key}\n")));
            assert!(!saved.lines().any(|line| line.starts_with("tactical=")));
            assert_eq!(Preferences::from_text(&saved).difficulty, difficulty);
        }
    }
}

#[cfg(test)]
mod live_preview_tests {
    use super::*;
    use local_rift::{game::Unit, gear::GearState};
    fn unit(id: u32, card_id: usize) -> Unit {
        Unit {
            id,
            card_id,
            lane: None,
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        }
    }
    fn attached(id: u32, card_id: usize, unit_id: u32) -> GearState {
        GearState {
            id,
            card_id,
            attached_to: Some(unit_id),
            ready: true,
            temporary: false,
            token: false,
        }
    }
    #[test]
    fn sprite_preview_shows_its_identity_and_temporary_without_factory_rules() {
        let mut game = Game::new(3, 3, 7, Difficulty::Easy);
        let mut sprite = unit(100, 355);
        sprite.token = true;
        sprite.buff = 3;
        game.players[0].units = vec![sprite];
        let text = live_unit_description(&game, 0, 100).unwrap();
        assert!(text.contains("Sprite · 3 printed Might · domainless · Fae"));
        assert!(text.contains("TEMPORARY · printed"));
        assert!(text.contains("before hold scoring"));
        assert!(!text.contains("BUFFED"));
        assert!(!text.contains("Play two"));
    }

    #[test]
    fn petal_preview_reports_evaluated_bonus_and_explains_its_battlefield_scope() {
        let mut game = Game::new(3, 3, 7, Difficulty::Easy);
        let mut petal = unit(100, 352);
        petal.lane = Some(0);
        let mut sprite = unit(101, 355);
        sprite.token = true;
        sprite.buff = 3;
        sprite.lane = Some(0);
        game.players[0].units = vec![petal, sprite];
        let text = live_unit_description(&game, 0, 100).unwrap();
        assert!(
            text.contains("+1 might · +1 for each friendly Temporary unit at this battlefield")
        );
        game.players[0].units[0].lane = None;
        let text = live_unit_description(&game, 0, 100).unwrap();
        assert!(
            text.contains("+0 might · +1 for each friendly Temporary unit at this battlefield")
        );
    }

    #[test]
    fn exact_damage_input_accepts_full_integer_range_without_float_rounding() {
        for value in [0, 65_536, 9_007_199_254_740_993, u64::MAX] {
            assert_eq!(parse_damage_amount(&value.to_string()).unwrap(), value);
        }
        for value in [
            "",
            "00",
            "01",
            "-1",
            "+1",
            " 1",
            "1 ",
            "1.0",
            "1e3",
            "18446744073709551616",
        ] {
            assert!(parse_damage_amount(value).is_err(), "accepted {value:?}");
        }
    }
    #[test]
    fn live_preview_preserves_large_might_damage_and_negative_calculation() {
        let mut game = Game::new(3, 3, 7, Difficulty::Easy);
        let exact = 9_007_199_254_740_993;
        let mut large = unit(100, 69);
        large.buff = exact - u64::from(game.catalog[69].might);
        large.damage = exact - 1;
        game.players[0].units = vec![large];
        let text = live_unit_description(&game, 0, 100).unwrap();
        assert!(text.contains("Current might: 9007199254740993"));
        assert!(text.contains("Marked damage: 9007199254740992"));
        game.players[0].units[0].buff = 0;
        game.players[0].units[0].damage = 0;
        game.players[0].units[0].temporary_buff = -11;
        let text = live_unit_description(&game, 0, 100).unwrap();
        assert!(text.contains("0 current might"));
        assert!(text.contains("Calculated might: -3"));
        assert!(text.contains("Rules that reference Might use 0"));
    }
    #[test]
    fn invalid_live_stats_have_error_copy_instead_of_a_fabricated_stat() {
        let mut game = Game::new(3, 3, 7, Difficulty::Easy);
        let mut invalid = unit(100, 69);
        invalid.buff = u64::MAX;
        game.players[0].units = vec![invalid];
        let before = format!("{game:?}");
        let text = live_unit_description(&game, 0, 100).unwrap();
        assert!(text.starts_with("Cannot calculate this unit's stats: engine_capacity:"));
        assert!(!text.contains("0 current might"));
        assert_eq!(format!("{game:?}"), before);
    }
    #[test]
    fn xp_activation_preview_tracks_real_payment_and_level_loss() {
        let mut game = Game::new(1, 3, 7, Difficulty::Easy);
        game.start_match().unwrap();
        game.players[0].xp = 3;
        game.players[0].units = vec![unit(100, 345), unit(101, 338)];
        let action = Action::ActivateAbility {
            source: AbilitySource::Unit {
                player: 0,
                unit_id: 100,
            },
            ability_index: 0,
            targets: vec![],
        };
        let plan = game.cost_plan(&action).unwrap();
        assert_eq!(play_ui::total_label(&plan), "Pay 2 XP");
        assert!(plan.power.is_empty());
        assert!(
            live_unit_description(&game, 0, 100)
                .unwrap()
                .contains("2 XP")
        );
        game.apply(action).unwrap();
        assert_eq!(game.players[0].xp, 1);
        assert!(!game.buffed_units.contains(&100), "cost paid before effect");
        assert!(
            live_unit_description(&game, 0, 101)
                .unwrap()
                .contains("INACTIVE · Gustwalker: Ganking")
        );
        while !game.chain.is_empty() {
            game.apply(Action::Pass).unwrap();
        }
        assert!(game.buffed_units.contains(&100));
    }
    #[test]
    fn honeyfruit_preview_separates_level_output_and_shared_exhaust_cost() {
        let mut game = Game::new(1, 3, 7, Difficulty::Easy);
        game.start_match().unwrap();
        let mut gear = attached(100, 347, 0);
        gear.attached_to = None;
        game.players[0].gear = vec![gear];
        game.players[0].xp = 5;
        let text = live_gear_description(&game, 0, 100).unwrap();
        assert!(text.contains("Level 6 · inactive at XP 5"));
        assert!(text.contains("Add 1 energy + 1 Any power immediately"));
        game.players[0].xp = 6;
        assert!(
            live_gear_description(&game, 0, 100)
                .unwrap()
                .contains("Level 6 · active at XP 6")
        );
        let before = (game.players[0].energy, game.players[0].any_power);
        game.apply(Action::ActivateAbility {
            source: AbilitySource::Gear {
                player: 0,
                gear_id: 100,
            },
            ability_index: 1,
            targets: vec![],
        })
        .unwrap();
        assert_eq!(
            (game.players[0].energy, game.players[0].any_power),
            (before.0 + 1, before.1 + 1)
        );
        assert_eq!(game.players[0].xp, 6);
        assert!(
            live_gear_description(&game, 0, 100)
                .unwrap()
                .contains("Your gear · Exhausted")
        );
    }
    #[test]
    fn heirloom_fixed_payment_is_xp_only_and_full_integer() {
        let mut game = Game::new(2, 3, 7, Difficulty::Easy);
        game.start_match().unwrap();
        game.players[0].units = vec![unit(101, 110)];
        let mut gear = attached(100, 348, 0);
        gear.attached_to = None;
        game.players[0].gear = vec![gear];
        game.players[0].xp = u64::MAX;
        assert_eq!(
            equipment_cost_label(&local_rift::rules::equipment(348).unwrap()),
            "Equip: 1 XP"
        );
        let action = Action::Equip {
            gear_id: 100,
            target: Target::Unit {
                player: 0,
                unit_id: 101,
            },
        };
        assert_eq!(
            play_ui::total_label(&game.cost_plan(&action).unwrap()),
            "Pay 1 XP"
        );
        game.apply(Action::BeginPayment {
            action: Box::new(action),
        })
        .unwrap();
        let plan = game.payment_plan_view(0).unwrap();
        assert_eq!(ability_ui::fixed_payment_label(&plan), "Fixed cost: 1 XP.");
        game.apply(Action::CompletePayment).unwrap();
        assert_eq!(game.players[0].xp, u64::MAX - 1);
    }
    #[test]
    fn xp_status_preserves_full_integer_for_both_players() {
        let mut game = Game::new(1, 3, 7, Difficulty::Easy);
        game.players[0].xp = 9_007_199_254_740_993;
        game.players[1].xp = u64::MAX;
        assert_eq!(xp_status(&game, 0), "Your XP 9007199254740993");
        assert_eq!(xp_status(&game, 1), "Opponent XP 18446744073709551615");
        game.players[1].units = vec![unit(100, 337)];
        assert!(
            live_unit_description(&game, 1, 100)
                .unwrap()
                .contains("currently 18446744073709551615")
        );
    }
    #[test]
    fn conditional_entry_preview_is_not_a_promise_to_ready_an_existing_unit() {
        let mut game = Game::new(1, 3, 7, Difficulty::Easy);
        game.players[0].xp = 2;
        assert_eq!(
            entry_ready_hint(&game, 0, 334).unwrap(),
            "Level 3 entry: exhausted · XP 2"
        );
        let mut soldier = unit(100, 334);
        soldier.ready = false;
        game.players[0].units = vec![soldier];
        game.players[0].xp = 3;
        assert_eq!(
            entry_ready_hint(&game, 0, 334).unwrap(),
            "Level 3 entry: ready · XP 3"
        );
        let text = live_unit_description(&game, 0, 100).unwrap();
        assert!(text.contains("Your unit · Exhausted"));
        assert!(
            text.contains("does not ready a unit already in play")
                && text.contains("Optional Accelerate is separate")
        );
    }
    #[test]
    fn xp_gain_history_preview_requires_an_actual_gain_event() {
        let mut game = Game::new(2, 3, 7, Difficulty::Easy);
        game.start_match().unwrap();
        game.players[0].xp = 255;
        game.players[0].units = vec![unit(100, 342), unit(101, 343)];
        let before = live_unit_description(&game, 0, 100).unwrap();
        assert!(before.contains("gained XP this turn: no (current XP 255"));
        assert!(before.contains("INACTIVE · Wily Newtfish: Ganking"));
        game.apply(Action::Move {
            unit_id: 101,
            lane: Some(0),
        })
        .unwrap();
        while !game.chain.is_empty() {
            game.apply(Action::Pass).unwrap();
        }
        let after = live_unit_description(&game, 0, 100).unwrap();
        assert!(after.contains("gained XP this turn: yes (current XP 257"));
        assert!(after.contains("ACTIVE · Wily Newtfish: Ganking"));
        assert_eq!(xp_status(&game, 0), "Your XP 257 · + this turn");
    }
    #[test]
    fn combat_experience_brief_keeps_the_instead_threshold() {
        let game = Game::new(1, 3, 7, Difficulty::Easy);
        assert_eq!(card_brief(&game, 335), "+1 might · Level 6: +3 instead");
    }
    #[test]
    fn buff_conditions_do_not_confuse_temporary_might_or_empower_with_a_counter() {
        let mut game = Game::new(1, 3, 7, Difficulty::Easy);
        let mut elder = unit(100, 321);
        elder.temporary_buff = 2;
        game.players[0].units = vec![elder];
        game.empowered_units.push(100);
        let before = live_unit_description(&game, 0, 100).unwrap();
        assert!(before.contains("6 current might"));
        assert!(before.contains("INACTIVE · Wizened Elder"));
        assert!(!before.contains("BUFFED ·"));
        game.buffed_units.push(100);
        game.players[0].units[0].buff = 1;
        let after = live_unit_description(&game, 0, 100).unwrap();
        assert!(after.contains("8 current might"));
        assert!(after.contains("BUFFED ·") && after.contains("ACTIVE · Wizened Elder"));
    }
    #[test]
    fn rune_threshold_preview_counts_exhausted_runes_and_updates_on_recycling() {
        let mut game = Game::new(1, 3, 7, Difficulty::Easy);
        game.players[0].units = vec![unit(100, 325)];
        game.players[0].runes = [8, 0, 0, 0, 0, 0];
        game.players[0].exhausted_runes = [1, 0, 0, 0, 0, 0];
        let before = live_unit_description(&game, 0, 100).unwrap();
        assert!(before.contains("8 current might"));
        assert!(before.contains("8+ runes in play (currently 8; ready and exhausted both count)"));
        game.players[0].runes[0] = 7;
        let after = live_unit_description(&game, 0, 100).unwrap();
        assert!(after.contains("4 current might") && after.contains("INACTIVE · Master Yi"));
        assert!(after.contains("currently 7;"));
    }
    #[test]
    fn equipped_level_preview_tracks_each_attachment_and_controller_xp() {
        let mut game = Game::new(1, 3, 7, Difficulty::Easy);
        game.players[1].units = vec![unit(100, 110)];
        game.players[1].gear = vec![attached(200, 330, 100), attached(201, 330, 100)];
        game.players[0].xp = 15;
        game.players[1].xp = 2;
        let before = live_unit_description(&game, 1, 100).unwrap();
        assert!(before.contains("4 current might"));
        assert_eq!(before.matches("INACTIVE · Soul Sword").count(), 2);
        assert!(
            before.contains("Level 3: 3+ XP (currently 2)")
                && before.contains("attached bearer only")
        );
        game.players[1].xp = 3;
        let after = live_unit_description(&game, 1, 100).unwrap();
        assert!(after.contains("6 current might"));
        assert_eq!(after.matches("ACTIVE · Soul Sword").count(), 2);
        assert!(
            live_gear_description(&game, 1, 200)
                .unwrap()
                .contains("ACTIVE · Soul Sword")
        );
        game.players[1].gear[0].attached_to = None;
        assert!(
            live_unit_description(&game, 1, 100)
                .unwrap()
                .contains("4 current might")
        );
    }
    #[test]
    fn hunt_preview_sums_printed_and_attached_hunt_without_separate_trigger_claims() {
        let mut game = Game::new(1, 3, 7, Difficulty::Easy);
        game.players[0].units = vec![unit(100, 22)];
        game.players[0].gear = vec![attached(200, 328, 100), attached(201, 328, 100)];
        let text = live_unit_description(&game, 0, 100).unwrap();
        assert!(text.contains("Hunt 4:") && text.contains("one Hunt ability gains 4 XP"));
        assert!(
            text.contains("ATTACHED EQUIPMENT") && text.matches("Hunter's Machete").count() == 2
        );
        game.players[0].gear.pop();
        assert!(
            live_unit_description(&game, 0, 100)
                .unwrap()
                .contains("Hunt 3:")
        );
    }
    #[test]
    fn token_aura_preview_uses_actual_token_identity_and_never_calls_base_might_a_buff() {
        let mut game = Game::new(3, 1, 7, Difficulty::Easy);
        let mut token = unit(100, 318);
        token.token = true;
        token.buff = 3;
        game.players[0].units = vec![token, unit(101, 324), unit(102, 318)];
        let text = live_unit_description(&game, 0, 100).unwrap();
        assert!(
            text.contains("4 current might")
                && text.contains("friendly unit tokens, at any location")
        );
        assert!(!text.contains("BUFFED ·"));
        assert!(
            !live_unit_description(&game, 0, 102)
                .unwrap()
                .contains("Soul Shepherd")
        );
    }
    #[test]
    fn printed_temporary_preview_uses_canonical_attachment_status() {
        let mut game = Game::new(1, 3, 7, Difficulty::Easy);
        let mut gear = attached(200, 331, 100);
        gear.attached_to = None;
        gear.temporary = true;
        game.players[0].gear = vec![gear];
        let text = live_gear_description(&game, 0, 200).unwrap();
        assert!(
            text.contains("TEMPORARY ·")
                && text.contains("before hold scoring")
                && text.contains("Players can respond")
        );
        game.players[0].gear[0].attached_to = Some(100);
        game.players[0].units = vec![unit(100, 110)];
        let text = live_gear_description(&game, 0, 200).unwrap();
        assert!(text.contains("Printed Temporary is inactive"));
        assert!(!text.contains("TEMPORARY ·"));
    }
    #[test]
    fn granted_temporary_preview_stays_active_after_actual_equip() {
        let mut game = Game::new(1, 3, 7, Difficulty::Easy);
        game.start_match().unwrap();
        game.players[0].units = vec![unit(100, 69)];
        let mut gear = attached(200, 3, 100);
        gear.attached_to = None;
        game.players[0].gear = vec![gear];
        game.players[0].hand = vec![66];
        game.players[0].runes = [0; 6];
        game.players[0].runes[local_rift::game::domain_index(Domain::Mind)] = 6;
        game.players[0].exhausted_runes = [0; 6];
        game.players[0].energy = 6;
        game.players[0].any_power = 2;
        game.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Gear {
                player: 0,
                gear_id: 200,
            }],
        })
        .unwrap();
        while !game.chain.is_empty() {
            game.apply(Action::Pass).unwrap();
        }
        game.apply(Action::Equip {
            gear_id: 200,
            target: Target::Unit {
                player: 0,
                unit_id: 100,
            },
        })
        .unwrap();
        while !game.chain.is_empty() {
            game.apply(Action::Pass).unwrap();
        }
        let text = live_gear_description(&game, 0, 200).unwrap();
        assert!(
            text.contains("TEMPORARY ·")
                && text.contains("Granted Temporary remains active even while attached")
                && text.contains("not the bearer")
        );
        assert!(!text.contains("Printed Temporary is inactive"));
    }
    #[test]
    fn global_spell_briefs_preserve_scope_and_conditional_channel_draw() {
        let game = Game::new(1, 3, 7, Difficulty::Easy);
        assert!(card_brief(&game, 308).contains("both players, everywhere"));
        assert!(card_brief(&game, 310).contains("Channel 2 exhausted · if fewer, draw 1"));
        assert!(card_brief(&game, 311).contains("Friendly units: +5"));
        assert!(card_brief(&game, 312).contains("Ready your units · +3"));
        assert!(card_brief(&game, 313).contains("Exhaust yours · deal 12 to all field units"));
    }
    #[test]
    fn conditional_unit_preview_tracks_source_state_and_aura_removal() {
        let mut game = Game::new(3, 2, 7, Difficulty::Easy);
        game.players[0].units = vec![unit(100, 275)];
        let before = live_unit_description(&game, 0, 100).unwrap();
        assert!(
            before.contains("Not Empowered")
                && before.contains("INACTIVE")
                && !before.contains("Ganking: can move")
        );
        game.empowered_units.push(100);
        let after = live_unit_description(&game, 0, 100).unwrap();
        assert!(
            after.contains("EMPOWERED")
                && after.contains("Ganking: can move")
                && after.contains("ACTIVE · Kinkou")
        );
        game.players[0].gear.push(GearState {
            id: 200,
            card_id: 281,
            attached_to: None,
            ready: true,
            temporary: false,
            token: false,
        });
        let aura = live_unit_description(&game, 0, 100).unwrap();
        assert!(aura.contains("ACTIVE · Rage Amplifier: +1 might"));
        game.empowered_units.push(200);
        let empowered = live_unit_description(&game, 0, 100).unwrap();
        assert!(
            empowered.contains("ACTIVE · Rage Amplifier: +2 might")
                && empowered.contains("INACTIVE · Rage Amplifier: +1 might")
        );
        game.players[0].gear.clear();
        assert!(
            !live_unit_description(&game, 0, 100)
                .unwrap()
                .contains("Rage Amplifier")
        );
    }

    #[test]
    fn live_gear_preview_tracks_instance_state_and_never_leaks_to_another_card() {
        let mut game = Game::new(3, 2, 7, Difficulty::Easy);
        for id in [200, 201] {
            game.players[1].gear.push(GearState {
                id,
                card_id: 250,
                attached_to: None,
                ready: id == 200,
                temporary: false,
                token: false,
            });
        }
        game.empowered_units.push(201);
        let first = live_object_description(&game, 1, 200, 250).unwrap();
        let second = live_object_description(&game, 1, 201, 250).unwrap();
        assert!(first.contains("Opponent's gear · Ready") && first.contains("Not Empowered"));
        assert!(second.contains("Exhausted") && second.contains("EMPOWERED ·"));
        assert!(second.contains("Disempower source · 1 energy · exhaust source"));
        assert!(second.contains("no chosen target"));
        assert!(live_object_description(&game, 1, 201, 251).is_none());
        assert_eq!(
            ability_ui::source_state(
                &game,
                AbilitySource::Gear {
                    player: 1,
                    gear_id: 201
                }
            ),
            "Exhausted · Empowered"
        );
        game.players[1].gear[1].attached_to = Some(100);
        game.players[1].units.push(unit(100, 238));
        assert!(
            live_gear_description(&game, 1, 201)
                .unwrap()
                .contains("printed gear abilities are inactive while attached")
        );
    }

    #[test]
    fn opponent_ability_preview_exposes_both_renata_costs_and_timing() {
        let mut game = Game::new(3, 2, 7, Difficulty::Easy);
        game.players[1].units = vec![unit(100, 239)];
        let text = live_unit_description(&game, 1, 100).unwrap();
        assert!(text.contains("Opponent's unit"));
        assert!(text.contains("ABILITY · Draw 1") && text.contains("1 energy · 1 Mind power"));
        assert!(
            text.contains("ABILITY · Score 1 point")
                && text.contains("4 energy · 4 Mind power · exhaust source")
        );
        assert!(text.contains("source must be at a battlefield"));
        game.players[1].units[0].token = true;
        assert!(
            !live_unit_description(&game, 1, 100)
                .unwrap()
                .contains("ABILITY ·"),
            "tokens do not inherit their source card's activation"
        );
    }

    #[test]
    fn live_preview_distinguishes_instances_and_granted_keywords() {
        let mut game = Game::new(3, 2, 7, Difficulty::Easy);
        game.players[0].units = vec![unit(100, 69), unit(101, 69)];
        game.players[0].units[1].buff = 1;
        game.players[0].units[1].ready = false;
        game.players[0].gear.push(GearState {
            id: 200,
            card_id: 3,
            ready: true,
            attached_to: Some(101),
            temporary: false,
            token: false,
        });
        let first = live_unit_description(&game, 0, 100).unwrap();
        let second = live_unit_description(&game, 0, 101).unwrap();
        assert!(first.contains("8 current might") && first.contains("Ready"));
        assert!(!first.contains("Ganking"));
        assert!(!first.contains("ATTACHED EQUIPMENT"));
        assert!(second.contains("ATTACHED EQUIPMENT") && second.contains("Boots of Swiftness"));
        assert!(
            second.contains("11 current might")
                && second.contains("Exhausted")
                && second.contains("Ganking")
        );
    }
    #[test]
    fn stunned_preview_explains_combat_contribution_without_reducing_lethal() {
        let mut game = Game::new(3, 2, 7, Difficulty::Easy);
        let mut stunned = unit(100, 69);
        stunned.stunned = true;
        game.players[0].units = vec![stunned];
        let text = live_unit_description(&game, 0, 100).unwrap();
        assert!(
            text.contains("8 current might")
                && text.contains("contributes no might to combat damage this turn")
        );
        assert!(text.contains("lethal damage equal to its full might"));
        assert!(!text.contains("next ready"));
    }

    #[test]
    fn keyword_preview_separates_attack_bonus_and_play_cost() {
        let mut game = Game::new(3, 2, 7, Difficulty::Easy);
        game.players[0].units = vec![unit(100, 98)];
        let text = live_unit_description(&game, 0, 100).unwrap();
        assert!(text.contains("Assault 2") && text.contains("while attacking"));
        assert!(text.contains("Deflect 1") && text.contains("Ganking"));
        assert!(text.contains("Accelerate, when played") && text.contains("Fury power"));
    }

    #[test]
    fn battlefield_target_labels_distinguish_duplicate_definitions() {
        let mut game = Game::new(3, 2, 7, Difficulty::Easy);
        game.battlefields = [0, 0];
        let left = target_label(&game, Target::Battlefield { lane: 0 });
        let right = target_label(&game, Target::Battlefield { lane: 1 });
        assert!(left.starts_with("Field 1 · ") && right.starts_with("Field 2 · "));
        assert_ne!(left, right);
        assert_eq!(
            target_label(&game, Target::Battlefield { lane: usize::MAX }),
            "Unavailable battlefield"
        );
    }

    #[test]
    fn local_aura_preview_follows_source_location_and_excludes_self() {
        let mut game = Game::new(3, 2, 7, Difficulty::Easy);
        let mut bearer = unit(100, 110);
        bearer.lane = Some(0);
        let mut taric = unit(101, 307);
        taric.lane = Some(0);
        game.players[0].units = vec![bearer, taric];
        let together = live_unit_description(&game, 0, 100).unwrap();
        assert!(together.contains("ACTIVE · Taric, Protector: Shield 1"));
        assert!(together.contains("source's location"));
        assert!(
            !live_unit_description(&game, 0, 101)
                .unwrap()
                .contains("ACTIVE · Taric")
        );
        game.players[0].units[1].lane = Some(1);
        assert!(
            !live_unit_description(&game, 0, 100)
                .unwrap()
                .contains("ACTIVE · Taric")
        );
    }

    #[test]
    fn conditional_might_preview_names_live_combat_requirement() {
        let mut game = Game::new(3, 2, 7, Difficulty::Easy);
        let mut pigeons = unit(100, 304);
        pigeons.lane = Some(0);
        let mut ally = unit(101, 110);
        ally.lane = Some(0);
        game.players[0].units = vec![pigeons, ally];
        let before = live_unit_description(&game, 0, 100).unwrap();
        assert!(before.contains("INACTIVE · Crimson Pigeons: +2 might"));
        assert!(before.contains("while attacking with another unit"));
        game.showdown = Some(local_rift::game::Showdown {
            lane: 0,
            attacker: 0,
            focus: 0,
            passes: 0,
            combat: true,
        });
        let during = live_unit_description(&game, 0, 100).unwrap();
        assert!(
            during.contains("5 current might")
                && during.contains("ACTIVE · Crimson Pigeons: +2 might")
        );
        game.players[0].units.pop();
        assert!(
            live_unit_description(&game, 0, 100)
                .unwrap()
                .contains("INACTIVE · Crimson Pigeons")
        );
    }

    #[test]
    fn shield_preview_uses_current_defending_might_without_double_counting() {
        let mut game = Game::new(0, 3, 7, Difficulty::Easy);
        let mut poro = unit(100, 110);
        poro.lane = Some(0);
        game.players[0].units = vec![poro];
        let neutral = live_unit_description(&game, 0, 100).unwrap();
        assert!(neutral.contains("2 current might") && neutral.contains("Shield 1"));
        game.showdown = Some(local_rift::game::Showdown {
            lane: 0,
            attacker: 1,
            focus: 1,
            passes: 0,
            combat: false,
        });
        assert!(
            live_unit_description(&game, 0, 100)
                .unwrap()
                .contains("2 current might")
        );
        game.showdown.as_mut().unwrap().combat = true;
        let defending = live_unit_description(&game, 0, 100).unwrap();
        assert!(
            defending.contains("3 current might") && defending.contains("already included above")
        );
        game.showdown.as_mut().unwrap().attacker = 0;
        assert!(
            live_unit_description(&game, 0, 100)
                .unwrap()
                .contains("2 current might")
        );
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
        app.prefs.difficulty = Difficulty::Tactical;
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
                    && player.energy < u16::from(player.total_runes())
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
