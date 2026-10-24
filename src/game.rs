//! Deterministic, renderer-independent Riftbound practice rules.
//!
//! The core turn, rune, movement, combat, and scoring rules are modelled here.
//! Printed operations use the official-card metadata in `rules.rs`. Unsupported
//! interactions are reported explicitly through `card_support`.

use crate::battlefields::{BattlefieldAbility, default_battlefield};
use crate::cards::{CardDef, CardId, CardKind, DeckPreset, Domain, cards, presets};
use crate::deck::CustomDeck;
use crate::gear::GearState;
use crate::hidden::HiddenCard;
use crate::legends::{LegendAbility, LegendState, LegendTrigger};
use crate::rules::{self, Allegiance, CounterKind, OfficialEffect, StaticAbility};
use std::collections::VecDeque;

#[path = "scalar_spells.rs"]
mod scalar_spells;
#[path = "strategic.rs"]
mod strategic;
#[path = "triggers.rs"]
pub mod triggers;
use triggers::{EventSnapshot, Occurrence};
pub use triggers::{TriggerContext, TriggerSource, TriggerState};

pub const VICTORY_SCORE: u8 = 8;
pub const BATTLEFIELDS: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Tactical,
    Strategic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Mulligan,
    Playing,
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Unit { player: usize, unit_id: u32 },
    Spell { chain_id: u32 },
    Gear { player: usize, gear_id: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    ChooseTriggerTargets {
        targets: Vec<Target>,
    },
    FinalizePendingPlay {
        lane: Option<usize>,
        targets: Vec<Target>,
        repeat_targets: Vec<Target>,
        repeat: bool,
        accelerate: bool,
        bonus: bool,
    },
    WithPowerPayment {
        action: Box<Action>,
        payment: crate::payment::PowerPayment,
    },
    PlayFromTrashOptions {
        trash_index: usize,
        lane: Option<usize>,
        targets: Vec<Target>,
        repeat_targets: Vec<Target>,
        repeat: bool,
        accelerate: bool,
        bonus: bool,
        flow: Option<FlowSource>,
    },
    PlaceToken {
        lane: Option<usize>,
    },
    ChooseResolutionGroup {
        index: usize,
    },
    ChooseBattlefield {
        lane: usize,
    },
    SetOptionalDiscount {
        power: u8,
    },
    OrderTrigger {
        trigger_id: u32,
    },
    ChooseDeathReplacement {
        unit_id: u32,
        gear_id: Option<u32>,
    },
    /// Exhaust into stored energy, or recycle into matching colored power.
    ActivateRune {
        domain: Domain,
        recycle: bool,
    },
    /// Recycle an explicitly ready or exhausted rune, without first exhausting it.
    RecycleRune {
        domain: Domain,
        exhausted: bool,
    },
    PlayCard {
        hand_index: usize,
        lane: Option<usize>,
        target: Option<Target>,
    },
    PlayChampion {
        lane: Option<usize>,
        target: Option<Target>,
    },
    PlayCardWithTargets {
        hand_index: usize,
        targets: Vec<Target>,
        lane: Option<usize>,
    },
    Move {
        unit_id: u32,
        lane: Option<usize>,
    },
    MoveGroup {
        unit_ids: Vec<u32>,
        lane: Option<usize>,
    },
    Attack {
        lane: usize,
    },
    Pass,
    Choose {
        accept: bool,
    },
    Equip {
        gear_id: u32,
        target: Target,
    },
    EquipWithTrash {
        gear_id: u32,
        target: Target,
        trash_indices: Vec<usize>,
    },
    ActivateGear {
        gear_id: u32,
    },
    ActivateLegend {
        target: Option<Target>,
    },
    SelectCards {
        indices: Vec<usize>,
    },
    PlayFromTrashWithTargets {
        trash_index: usize,
        lane: Option<usize>,
        targets: Vec<Target>,
    },
    PlayFromTrash {
        trash_index: usize,
        lane: Option<usize>,
        target: Option<Target>,
    },
    Hide {
        hand_index: usize,
        lane: usize,
    },
    HideChampion {
        lane: usize,
    },
    PlayHidden {
        hidden_id: u32,
        targets: Vec<Target>,
    },
    PlayCardOptions {
        hand_index: usize,
        lane: Option<usize>,
        targets: Vec<Target>,
        repeat_targets: Vec<Target>,
        repeat: bool,
        accelerate: bool,
        bonus: bool,
    },
    PlayChampionOptions {
        lane: Option<usize>,
        target: Option<Target>,
        accelerate: bool,
    },
    EmpowerUnit {
        unit_id: u32,
        pay_power: bool,
    },
    ChooseTarget {
        target: Option<Target>,
    },
    AssignDamage {
        assignments: Vec<(u32, u16)>,
    },
    EndTurn,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChainAbility {
    CardTrigger {
        rule_index: usize,
        context: TriggerContext,
    },
    PlayChosen {
        trash_index: usize,
        card_id: CardId,
        kind: InstructedPlayKind,
    },
    GrantFlowChosen {
        trash_index: usize,
        card_id: CardId,
    },
    Disempower {
        player: usize,
        object_id: Option<u32>,
    },
    OptionalPlay {
        gear_id: Option<u32>,
    },
    ReviveChosen {
        trash_index: usize,
        card_id: CardId,
    },
    EmpowerObject {
        player: usize,
        object_id: Option<u32>,
        lane: Option<usize>,
        source_id: Option<u32>,
    },
    Equip {
        gear_id: u32,
        target: Target,
    },
    Legend(LegendAbility),
    Battlefield(BattlefieldAbility),
    Empower {
        unit_id: u32,
    },
    Swap {
        unit_id: u32,
        target: Target,
    },
    CardEffect {
        entering: Option<u32>,
    },
    Triggered(TriggeredAbility),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TriggeredAbility {
    DestroyGear {
        gear_id: u32,
    },
    Draw(u8),
    Might {
        unit_id: u32,
        amount: i16,
    },
    Stun {
        player: usize,
        unit_id: u32,
    },
    Gold,
    Tokens {
        might: u8,
        count: u8,
    },
    Xp(u8),
    RevealHandXp,
    GrantFlow,
    Loot {
        count: u8,
        draw: u8,
    },
    BurnForMight {
        unit_id: u32,
    },
    LastRites {
        gear_id: u32,
    },
    SwapChoice {
        unit_id: u32,
    },
    EmpowerHere {
        lane: Option<usize>,
        source_id: Option<u32>,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayOptions {
    pub repeat: bool,
    pub accelerate: bool,
    pub bonus: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowSource {
    Printed,
    Granted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowOption {
    pub source: FlowSource,
    pub energy: u8,
    pub power: u8,
    pub colored: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct TrashOptions {
    play: PlayOptions,
    flow: Option<FlowSource>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainItem {
    pub id: u32,
    pub player: usize,
    pub card_id: CardId,
    pub target: Option<Target>,
    pub targets: Vec<Target>,
    pub lane: Option<usize>,
    /// None is a physical spell; abilities reference a card in another zone.
    pub ability: Option<ChainAbility>,
    pub recycle_after: bool,
    pub banish_after: bool,
    pub from_hidden: bool,
    pub options: PlayOptions,
    pub repeat_targets: Vec<Target>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Showdown {
    pub lane: usize,
    pub attacker: usize,
    pub focus: usize,
    pub passes: u8,
    pub combat: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    TriggerTargets {
        player: usize,
        card_id: CardId,
        rule_index: usize,
        context: TriggerContext,
    },
    TrashTarget {
        player: usize,
        source_card_id: CardId,
        gear_id: Option<u32>,
    },
    PendingPlay {
        player: usize,
        pending_id: u32,
        card_id: CardId,
        kind: InstructedPlayKind,
    },
    BattlefieldOrder {
        player: usize,
        lanes: Vec<usize>,
        combat: bool,
    },
    OptionalPlay {
        player: usize,
        card_id: CardId,
        gear_id: Option<u32>,
    },
    TokenPlacement {
        player: usize,
        card_id: CardId,
        remaining: u8,
    },
    ResolutionTargets {
        player: usize,
        card_id: CardId,
        groups: Vec<Vec<Target>>,
        repeated: bool,
    },
    OrderTriggers {
        player: usize,
        options: Vec<TriggerOption>,
    },
    OnPlayTarget {
        player: usize,
        card_id: CardId,
        source_id: u32,
        lane: Option<usize>,
    },
    DeathReplacement {
        player: usize,
        options: Vec<DeathReplacement>,
    },
    CombatDamage {
        player: usize,
        lane: usize,
        total: u16,
    },
    RevealHand {
        player: usize,
        opponent: usize,
        cards: Vec<CardId>,
    },
    Battlefield {
        player: usize,
        lane: usize,
    },
    ReviveUnit {
        player: usize,
    },
    /// Matriarch's selected unit is being played now, with optional costs available.
    RevivePlay {
        player: usize,
        trash_index: usize,
        card_id: CardId,
    },
    SwapUnit {
        player: usize,
        unit_id: u32,
    },
    EmpowerHere {
        player: usize,
        lane: Option<usize>,
        source_id: Option<u32>,
    },
    Legend {
        player: usize,
        trigger: LegendTrigger,
    },
    LastRites {
        player: usize,
        gear_id: u32,
    },
    ReplaySpell {
        player: usize,
    },
    GrantFlow {
        player: usize,
    },
    Discard {
        player: usize,
        count: u8,
        draw_after: u8,
    },
    TopThree {
        player: usize,
        cards: Vec<CardId>,
    },
    Sabotage {
        player: usize,
        opponent: usize,
    },
    BurnForMight {
        player: usize,
        unit_id: u32,
    },
    Predict {
        player: usize,
        card_id: CardId,
    },
    CounterTax {
        player: usize,
        chain_id: u32,
        cost: u8,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TriggerOption {
    pub id: u32,
    pub card_id: CardId,
    pub label: String,
}

#[derive(Clone, Debug)]
enum PendingTriggerEffect {
    Ability {
        targets: Vec<Target>,
        ability: ChainAbility,
        lane: Option<usize>,
    },
    Choice(Decision),
    Token(PendingToken),
    Card(PendingCard),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstructedPlayKind {
    Fizz,
    LastRites,
    Matriarch,
}

/// A real card moved to the chain by an instruction, awaiting finalization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingCard {
    pub id: u32,
    pub player: usize,
    pub card_id: CardId,
    pub kind: InstructedPlayKind,
    origin_index: usize,
    origin_flow: bool,
    origin_targets: Vec<u32>,
}

/// Tokens created on the chain await finalization after their parent effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingToken {
    pub id: u32,
    pub player: usize,
    pub card_id: CardId,
    pub might: u8,
    pub gear: bool,
    /// None permits the normal location choice; Some(None) mandates base.
    pub location: Option<Option<usize>>,
}
#[derive(Clone, Debug)]
struct PendingTrigger {
    batch: u64,
    option: TriggerOption,
    player: usize,
    effect: PendingTriggerEffect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeathReplacement {
    pub unit_id: u32,
    /// None is Smite's banishment; Some is the gear sacrificed instead.
    pub gear_id: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardSupport {
    Exact,
    Partial,
    Unsupported,
}

impl Decision {
    pub fn player(&self) -> usize {
        match self {
            Self::TriggerTargets { player, .. }
            | Self::BattlefieldOrder { player, .. }
            | Self::TrashTarget { player, .. }
            | Self::PendingPlay { player, .. }
            | Self::OptionalPlay { player, .. }
            | Self::TokenPlacement { player, .. }
            | Self::ResolutionTargets { player, .. }
            | Self::OrderTriggers { player, .. }
            | Self::OnPlayTarget { player, .. }
            | Self::DeathReplacement { player, .. }
            | Self::CombatDamage { player, .. }
            | Self::RevealHand { player, .. }
            | Self::Battlefield { player, .. }
            | Self::ReviveUnit { player }
            | Self::RevivePlay { player, .. }
            | Self::SwapUnit { player, .. }
            | Self::EmpowerHere { player, .. }
            | Self::Predict { player, .. }
            | Self::CounterTax { player, .. }
            | Self::Legend { player, .. }
            | Self::LastRites { player, .. }
            | Self::ReplaySpell { player }
            | Self::GrantFlow { player }
            | Self::Discard { player, .. }
            | Self::TopThree { player, .. }
            | Self::Sabotage { player, .. }
            | Self::BurnForMight { player, .. } => *player,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unit {
    pub id: u32,
    pub card_id: CardId,
    /// `None` is this unit's owner's base; `Some(0..2)` is a battlefield.
    pub lane: Option<usize>,
    pub ready: bool,
    pub damage: u8,
    /// Permanent extra might; for a token, this is its base might as well.
    pub buff: u8,
    pub temporary_buff: i16,
    pub token: bool,
    pub protected: bool,
    pub stunned: bool,
    pub banish_on_death: bool,
}

/// Current keyword capabilities, including tokens, equipment and granted abilities.
/// Assault is the keyword's value; it increases Might only while attacking.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnitKeywords {
    pub assault: u8,
    pub shield: u8,
    pub tank: bool,
    pub backline: bool,
    pub deflect: u8,
    pub ganking: bool,
    pub ambush: bool,
    pub accelerate: Option<rules::AccelerateCost>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerState {
    pub name: String,
    pub deck: Vec<CardId>,
    pub hand: Vec<CardId>,
    pub discard: Vec<CardId>,
    pub banish: Vec<CardId>,
    pub champion: Option<CardId>,
    pub units: Vec<Unit>,
    pub gear: Vec<GearState>,
    pub hidden: Vec<HiddenCard>,
    pub power_pool: [u8; 6],
    pub any_power: u8,
    pub score: u8,
    /// Total available energy: stored energy plus energy obtainable from ready runes.
    pub energy: u8,
    /// Energy already generated in the rune pool; expires when pools empty.
    pub energy_pool: u8,
    /// All channelled runes by domain, including exhausted runes.
    pub runes: [u8; 6],
    pub exhausted_runes: [u8; 6],
    /// The top of the rune deck is the end of this vector.
    pub rune_deck: Vec<Domain>,
    pub xp: u8,
}

impl PlayerState {
    pub fn total_runes(&self) -> u8 {
        self.runes.iter().sum()
    }
    pub fn ready_runes(&self, domain: Domain) -> u8 {
        let d = domain_index(domain);
        self.runes[d].saturating_sub(self.exhausted_runes[d])
    }
    pub fn domain_power(&self, domain: Domain) -> u8 {
        self.runes[domain_index(domain)] + self.power_pool[domain_index(domain)] + self.any_power
    }
}

#[derive(Clone, Copy, Debug)]
enum StartStage {
    Beginning,
    Holding,
    Channel,
    Draw,
    Main,
}

#[derive(Clone, Debug)]
struct PendingCombat {
    lane: usize,
    attacker: usize,
    powers: [u16; 2],
    assignments: [Option<Vec<(u32, u16)>>; 2],
}

#[derive(Clone, Debug)]
pub struct Game {
    pub trigger_state: TriggerState,
    pub catalog: Vec<CardDef>,
    pub presets: Vec<DeckPreset>,
    pub players: [PlayerState; 2],
    pub active: usize,
    pub first_player: usize,
    /// Counts individual turns, with the starting player's opening turn equal to 1.
    pub turn: u32,
    pub phase: Phase,
    pub winner: Option<usize>,
    pub difficulty: Difficulty,
    pub log: Vec<String>,
    pub battlefield_owner: [Option<usize>; BATTLEFIELDS],
    /// A battlefield scores at most once for the active player per turn,
    /// whether that score came from holding or conquering.
    pub scored_this_turn: [bool; BATTLEFIELDS],
    pub player_scored_this_turn: [[bool; BATTLEFIELDS]; 2],
    pub mulligan_used: bool,
    pub mulligans_used: [bool; 2],
    pub chain: Vec<ChainItem>,
    pub showdown: Option<Showdown>,
    pub decision: Option<Decision>,
    pub legend: [LegendState; 2],
    pub battlefields: [usize; 2],
    pub empowered_units: Vec<u32>,
    pub buffed_units: Vec<u32>,
    pub optional_power_discount: [u8; 2],
    pub required_trash_play: bool,
    pub hidden_view: [bool; 2],
    pub(crate) payment: Option<crate::payment::PaymentContext>,
    temporary_empower: Vec<(usize, usize, Option<u32>)>,
    ending_turn: bool,
    pending_decisions: VecDeque<Decision>,
    pending_triggers: Vec<PendingTrigger>,
    trigger_batch: u64,
    pending_token: Option<PendingToken>,
    pending_card: Option<PendingCard>,
    hidden_play_origins: Vec<(u32, usize)>,
    /// A discard/draw instruction retains its resolving item until its choice
    /// finishes, so the spell cannot enter its own burnout shuffle.
    pending_loot: Option<u32>,
    pending_entries: Vec<(usize, usize)>,
    movement_locks: Vec<(usize, u32)>,
    pending_combat: Option<PendingCombat>,
    pending_combat_cleanup: Option<(usize, usize)>,
    pending_start: Option<StartStage>,
    priority_player: usize,
    chain_passes: u8,
    chain_initiator: Option<usize>,
    next_chain_id: u32,
    extra_turns: [u8; 2],
    temporary_flow: [Vec<usize>; 2],
    pyke_triggered: Vec<u32>,
    silenced: [bool; 2],
    cards_played: [u16; 2],
    pub(crate) selected_decks: [usize; 2],
    custom_decks: [Option<CustomDeck>; 2],
    ai_mulligan: bool,
    rng: Rng,
    pub(crate) next_unit_id: u32,
    turns_taken: [u32; 2],
    ai_steps_this_turn: u16,
}

/// Stable domain indices are intentionally independent of enum discriminants.
pub fn domain_index(domain: Domain) -> usize {
    match domain {
        Domain::Fury => 0,
        Domain::Calm => 1,
        Domain::Mind => 2,
        Domain::Body => 3,
        Domain::Chaos => 4,
        Domain::Order => 5,
    }
}

const ALL_DOMAINS: [Domain; 6] = [
    Domain::Fury,
    Domain::Calm,
    Domain::Mind,
    Domain::Body,
    Domain::Chaos,
    Domain::Order,
];

#[derive(Clone, Debug)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x9e3779b97f4a7c15 } else { seed })
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            values.swap(i, self.next() as usize % (i + 1));
        }
    }
}

impl Game {
    pub fn new(player_deck: usize, ai_deck: usize, seed: u64, difficulty: Difficulty) -> Self {
        let mut game = Self::new_for_agents(player_deck, ai_deck, seed, difficulty);
        let choices: Vec<_> = game.players[1]
            .hand
            .iter()
            .enumerate()
            .filter(|(_, id)| game.catalog[**id].cost > 3)
            .map(|(i, _)| i)
            .take(2)
            .collect();
        game.mulligan_player(1, &choices)
            .expect("valid AI opening hand");
        game.ai_mulligan = true;
        game
    }

    /// Leaves both opening hands untouched for external agents.
    pub fn new_for_agents(
        player_deck: usize,
        ai_deck: usize,
        seed: u64,
        difficulty: Difficulty,
    ) -> Self {
        let last = presets().len() - 1;
        Self::new_with_custom(
            [player_deck.min(last), ai_deck.min(last)],
            [None, None],
            seed,
            difficulty,
            false,
        )
        .expect("valid bundled presets")
    }

    /// A validated constructed deck may replace either seat's preset. Keeping
    /// `ai_mulligan` false leaves both opening hands untouched for external bots.
    pub fn new_with_custom(
        preset_indices: [usize; 2],
        custom_decks: [Option<CustomDeck>; 2],
        seed: u64,
        difficulty: Difficulty,
        ai_mulligan: bool,
    ) -> Result<Self, String> {
        let catalog = cards();
        let presets = presets();
        if preset_indices.iter().any(|&index| index >= presets.len()) {
            return Err("Choose an available preset.".into());
        }
        for deck in custom_decks.iter().flatten() {
            deck.validate()?;
        }
        let selected_decks = std::array::from_fn(|player| {
            custom_decks[player]
                .as_ref()
                .map_or(preset_indices[player], |deck| deck.legend)
        });
        let mut rng = Rng::new(seed);
        let players = std::array::from_fn(|player| {
            let preset = &presets[selected_decks[player]];
            let custom = custom_decks[player].as_ref();
            let chosen_champion =
                custom.map_or(preset.chosen_champion, |deck| deck.chosen_champion);
            let mut deck = Vec::new();
            for &(id, count) in custom.map_or(&preset.cards, |deck| &deck.cards) {
                assert!(id < catalog.len(), "preset references an unknown card");
                deck.extend(std::iter::repeat_n(id, count as usize));
            }
            // The main-deck count includes the chosen champion, which begins
            // in its own public zone.
            let champion = deck
                .iter()
                .position(|&id| id == chosen_champion)
                .map(|index| deck.remove(index));
            rng.shuffle(&mut deck);
            let mut rune_deck = Vec::with_capacity(12);
            if let Some(custom) = custom {
                for (domain, count) in ALL_DOMAINS.into_iter().zip(custom.runes) {
                    rune_deck.extend(std::iter::repeat_n(domain, usize::from(count)));
                }
            } else {
                // Preserve the original preset shuffle ordering for seeded replays.
                for (domain, count) in preset.domains.into_iter().zip(preset.rune_counts) {
                    rune_deck.extend(std::iter::repeat_n(domain, count as usize));
                }
            }
            rng.shuffle(&mut rune_deck);
            let mut hand = Vec::with_capacity(4);
            for _ in 0..4 {
                if let Some(id) = deck.pop() {
                    hand.push(id);
                }
            }
            PlayerState {
                name: custom.map_or_else(|| preset.champion.to_string(), |deck| deck.name.clone()),
                deck,
                hand,
                discard: Vec::new(),
                banish: Vec::new(),
                champion,
                units: Vec::new(),
                gear: Vec::new(),
                hidden: Vec::new(),
                power_pool: [0; 6],
                any_power: 0,
                score: 0,
                energy: 0,
                energy_pool: 0,
                runes: [0; 6],
                exhausted_runes: [0; 6],
                rune_deck,
                xp: 0,
            }
        });
        let legend = selected_decks.map(|i| LegendState::new(presets[i].chosen_champion));
        let mut game = Self {
            catalog,
            presets,
            players,
            active: 0,
            first_player:0,
            turn: 0,
            phase: Phase::Mulligan,
            winner: None,
            difficulty,
            log: vec![
                "Welcome. Keep your four cards, or replace up to two before the match.".into(),
                "Inspect a card for its official rules and the implementation status of every interaction.".into(),
            ],
            battlefield_owner: [None; 2],
            scored_this_turn: [false; 2],
            player_scored_this_turn: [[false; 2]; 2],
            mulligan_used: false,
            mulligans_used: [false;2],
            chain: Vec::new(),
            showdown: None,
            decision: None,
            legend,
            battlefields: std::array::from_fn(|player|custom_decks[player].as_ref()
                .map_or_else(||default_battlefield(selected_decks[player]),|deck|deck.battlefield)),
            empowered_units: Vec::new(),
            buffed_units: Vec::new(),
            optional_power_discount: [0;2],
            required_trash_play:false,
            hidden_view:[false;2],
            payment: None,
            temporary_empower: Vec::new(),
            ending_turn: false,
            pending_decisions: VecDeque::new(),
            pending_triggers: Vec::new(),
            trigger_state: TriggerState::default(),
            trigger_batch: 0,
            pending_token: None,
            pending_card: None,
            hidden_play_origins: Vec::new(),
            pending_loot: None,
            pending_entries: Vec::new(),
            movement_locks: Vec::new(),
            pending_combat:None,
            pending_combat_cleanup:None,
            pending_start:None,
            priority_player: 0,
            chain_passes: 0,
            chain_initiator: None,
            next_chain_id: 1,
            extra_turns: [0; 2],
            temporary_flow: [Vec::new(), Vec::new()],
            pyke_triggered: Vec::new(),
            silenced: [false; 2],
            cards_played: [0; 2],
            selected_decks,
            custom_decks,
            ai_mulligan,
            rng,
            next_unit_id: 1,
            turns_taken: [0; 2],
            ai_steps_this_turn: 0,
        };
        if ai_mulligan {
            let choices: Vec<_> = game.players[1]
                .hand
                .iter()
                .enumerate()
                .filter(|(_, id)| game.catalog[**id].cost > 3)
                .map(|(i, _)| i)
                .take(2)
                .collect();
            game.mulligan_player(1, &choices)?;
        }
        Ok(game)
    }

    pub fn custom_deck(&self, player: usize) -> Option<&CustomDeck> {
        self.custom_decks.get(player).and_then(Option::as_ref)
    }

    pub fn rematch(&mut self, seed: u64) {
        let first_player = self.first_player;
        let battlefields = self.battlefields;
        *self = Self::new_with_custom(
            self.selected_decks,
            self.custom_decks.clone(),
            seed,
            self.difficulty,
            self.ai_mulligan,
        )
        .expect("previously validated decks");
        self.battlefields = battlefields;
        self.choose_starting_player(first_player)
            .expect("previous starting player is valid");
    }

    /// The player currently allowed to act. `active` remains the turn owner.
    pub fn priority(&self) -> usize {
        self.decision
            .as_ref()
            .map_or(self.priority_player, Decision::player)
    }

    pub fn in_window(&self) -> bool {
        !self.chain.is_empty() || self.showdown.is_some() || self.decision.is_some()
    }

    pub fn chain_name(&self, item: &ChainItem) -> String {
        match &item.ability {
            Some(ChainAbility::Disempower { .. }) => {
                "Tornado Warrior: disempower at end of turn".into()
            }
            Some(ChainAbility::Battlefield(ability)) => format!(
                "{} ability",
                crate::battlefields::battlefield(self.battlefields[ability.lane()]).name
            ),
            Some(ChainAbility::Legend(_)) => format!(
                "{} legend",
                self.presets[self.selected_decks[item.player]].legend_name
            ),
            Some(_) => format!("{} ability", self.catalog[item.card_id].name),
            None => self.catalog[item.card_id].name.into(),
        }
    }

    pub fn card_support(&self, id: CardId) -> CardSupport {
        let Some(card) = self.catalog.get(id) else {
            return CardSupport::Unsupported;
        };
        if matches!(rules::effect(id), OfficialEffect::Unsupported) {
            return CardSupport::Unsupported;
        }
        if card.kind == CardKind::Unit || card.kind == CardKind::Gear || id == 50 {
            CardSupport::Partial
        } else {
            CardSupport::Exact
        }
    }

    pub fn card_supported(&self, id: CardId) -> bool {
        self.card_support(id) != CardSupport::Unsupported
    }

    /// Specific outstanding behavior differences; official text is shown separately.
    pub fn effect_description(&self, id: CardId) -> String {
        if self.catalog.get(id).is_none() || !self.card_supported(id) {
            return "This card has no executable rules mapping and cannot be played.".into();
        }
        String::new()
    }

    fn effective_cost(&self, player: usize, card: &CardDef) -> u8 {
        self.discounted_energy(player, card.id, card.cost)
    }

    // Base replacements and additional costs have already been applied (CR356).
    fn discounted_energy(&self, player: usize, card_id: CardId, mut cost: u8) -> u8 {
        for ability in rules::static_abilities(card_id) {
            match *ability {
                StaticAbility::HighestMightEnergyDiscount => {
                    let highest = self.players[player]
                        .units
                        .iter()
                        .map(|unit| self.unit_might(unit))
                        .max()
                        .unwrap_or(0);
                    cost = cost.saturating_sub(highest);
                }
                StaticAbility::EmpoweredEnergyDiscount(amount)
                    if self.controls_empowered(player) =>
                {
                    cost = cost.saturating_sub(amount);
                }
                StaticAbility::TrashEnergyDiscount => {
                    cost = cost.saturating_sub(self.players[player].discard.len().min(255) as u8);
                }
                StaticAbility::LegionEnergyDiscount(amount) if self.cards_played[player] > 0 => {
                    cost = cost.saturating_sub(amount);
                }
                StaticAbility::PerCardEnergyDiscount { amount, minimum } => {
                    let discount = self.cards_played[player]
                        .saturating_mul(amount as u16)
                        .min(255) as u8;
                    cost = cost.saturating_sub(discount).max(minimum.min(cost));
                }
                _ => {}
            }
        }
        cost
    }

    pub fn movement_locked(&self, player: usize, unit_id: u32) -> bool {
        self.movement_locks.contains(&(player, unit_id))
    }
    pub fn cards_played_this_turn(&self, player: usize) -> u16 {
        self.cards_played[player]
    }
    pub fn cannot_play_cards(&self, player: usize) -> bool {
        self.silenced[player]
    }
    pub fn granted_flow_indices(&self, player: usize) -> &[usize] {
        &self.temporary_flow[player]
    }
    pub fn queued_extra_turns(&self, player: usize) -> u8 {
        self.extra_turns[player]
    }
    pub fn submitted_combat_damage(&self, player: usize) -> Option<&[(u32, u16)]> {
        self.pending_combat.as_ref()?.assignments[player].as_deref()
    }

    pub fn deck_count(&self, player: usize) -> usize {
        self.players[player].deck.len()
    }

    /// Replaces zero to two cards once. `start_match` keeps the resulting hand.
    pub fn mulligan(&mut self, indices: &[usize]) -> Result<(), String> {
        self.mulligan_player(0, indices)
    }

    pub fn mulligan_player(&mut self, player: usize, indices: &[usize]) -> Result<(), String> {
        if player > 1 {
            return Err("Choose one of the two players.".into());
        }
        if self.phase != Phase::Mulligan || self.mulligans_used[player] {
            return Err("The opening hand can only be changed once.".into());
        }
        if indices.len() > 2 {
            return Err("Choose at most two cards to replace.".into());
        }
        let mut sorted = indices.to_vec();
        sorted.sort_unstable();
        if sorted.windows(2).any(|pair| pair[0] == pair[1])
            || sorted.iter().any(|&i| i >= self.players[player].hand.len())
        {
            return Err("Choose different cards from your opening hand.".into());
        }
        replace_opening_cards(&mut self.players[player], &sorted, &mut self.rng);
        self.mulligans_used[player] = true;
        self.mulligan_used = self.mulligans_used[0];
        self.push_log(format!(
            "{} replaced {} opening card(s).",
            self.players[player].name,
            sorted.len()
        ));
        Ok(())
    }

    pub fn choose_starting_player(&mut self, player: usize) -> Result<(), String> {
        if self.phase != Phase::Mulligan {
            return Err("Choose the starting player before the match begins.".into());
        }
        if player > 1 {
            return Err("Choose one of the two players.".into());
        }
        self.first_player = player;
        self.active = player;
        self.priority_player = player;
        Ok(())
    }

    pub fn start_match(&mut self) -> Result<(), String> {
        if self.phase != Phase::Mulligan {
            return Err("The match has already begun.".into());
        }
        self.mulligan_used = true;
        self.mulligans_used = [true; 2];
        self.phase = Phase::Playing;
        self.start_turn();
        Ok(())
    }

    pub fn unit_might(&self, unit: &Unit) -> u8 {
        self.actual_unit_might(unit).clamp(0, 255) as u8
    }

    pub fn unit_keywords(&self, unit: &Unit) -> UnitKeywords {
        let mut keywords = UnitKeywords::default();
        let owner = self
            .players
            .iter()
            .position(|p| p.units.iter().any(|u| u.id == unit.id));
        if unit.token {
            keywords.deflect = rules::token_spec(unit.card_id).is_some_and(|s| s.deflect) as u8;
        } else {
            for ability in rules::static_abilities(unit.card_id) {
                match *ability {
                    StaticAbility::Assault(value) => {
                        keywords.assault = keywords.assault.saturating_add(value)
                    }
                    StaticAbility::Shield(value) => {
                        keywords.shield = keywords.shield.saturating_add(value)
                    }
                    StaticAbility::Deflect(value) => {
                        keywords.deflect = keywords.deflect.saturating_add(value)
                    }
                    StaticAbility::Tank => keywords.tank = true,
                    StaticAbility::Backline => keywords.backline = true,
                    StaticAbility::Ganking => keywords.ganking = true,
                    StaticAbility::Ambush => keywords.ambush = true,
                    StaticAbility::LevelSixDeflectAndGanking
                        if owner.is_some_and(|p| self.players[p].xp >= 6) =>
                    {
                        keywords.deflect = keywords.deflect.saturating_add(1);
                        keywords.ganking = true;
                    }
                    _ => {}
                }
            }
            keywords.accelerate = rules::accelerate_cost(unit.card_id);
        }
        let gear = self.gear_keywords(unit.id);
        keywords.assault = keywords.assault.saturating_add(gear.assault);
        keywords.shield = keywords.shield.saturating_add(gear.shield);
        keywords.deflect = keywords.deflect.saturating_add(gear.deflect);
        keywords.tank |= gear.tank;
        keywords.backline |= gear.backline;
        keywords.ganking |= gear.ganking;
        for (_, value) in self
            .legend
            .iter()
            .flat_map(|l| &l.assault_grants)
            .filter(|(id, _)| *id == unit.id)
        {
            keywords.assault = keywords.assault.saturating_add(*value);
        }
        keywords
    }

    /// The printed/granted assignment ordering shared by rules and the client.
    pub fn damage_priority(&self, unit: &Unit) -> u8 {
        let keywords = self.unit_keywords(unit);
        if keywords.tank {
            0
        } else if keywords.backline {
            2
        } else {
            1
        }
    }

    // CR143.2.c.1: references treat negative Might as zero, but calculations
    // of increases/decreases (including Swap) use the actual signed value.
    fn actual_unit_might(&self, unit: &Unit) -> i16 {
        let base = if unit.token {
            0
        } else {
            self.catalog[unit.card_id].might
        };
        let assault = if self.showdown.as_ref().is_some_and(|w| {
            unit.lane == Some(w.lane)
                && w.combat
                && self.players[w.attacker]
                    .units
                    .iter()
                    .any(|u| u.id == unit.id)
        }) {
            self.unit_keywords(unit).assault as i16
        } else {
            0
        };
        let shield = if self.showdown.as_ref().is_some_and(|w| {
            unit.lane == Some(w.lane)
                && w.combat
                && self.players[1 - w.attacker]
                    .units
                    .iter()
                    .any(|u| u.id == unit.id)
        }) {
            self.unit_keywords(unit).shield as i16
        } else {
            0
        };
        base as i16
            + unit.buff as i16
            + unit.temporary_buff
            + assault
            + shield
            + if unit.card_id == 24 && self.empowered_units.contains(&unit.id) {
                1
            } else {
                0
            }
            + self.gear_might_bonus(unit.id) as i16
            + self.legend_might_bonus(unit) as i16
    }

    pub fn lane_might(&self, player: usize, lane: usize) -> u16 {
        self.players[player]
            .units
            .iter()
            .filter(|u| u.lane == Some(lane) && !u.stunned)
            .map(|u| self.unit_might(u) as u16)
            .sum()
    }

    pub fn lane_count(&self, player: usize, lane: usize) -> usize {
        self.players[player]
            .units
            .iter()
            .filter(|u| u.lane == Some(lane))
            .count()
    }

    pub fn in_combat(&self, lane: usize) -> bool {
        self.showdown
            .as_ref()
            .is_some_and(|w| w.lane == lane && w.combat)
    }

    pub fn contested(&self, lane: usize) -> bool {
        lane < BATTLEFIELDS && self.lane_count(0, lane) > 0 && self.lane_count(1, lane) > 0
    }

    pub fn can_afford(&self, player: usize, card: &CardDef) -> bool {
        self.players[player].energy >= self.effective_cost(player, card)
            && self.power_available(player, card.domains, card.kind == CardKind::Spell)
                >= card.power_cost
    }

    /// All action validation happens before any mutation, so rejected actions
    /// never spend resources or consume cards.
    pub fn apply(&mut self, action: Action) -> Result<(), String> {
        if let Action::WithPowerPayment { action, payment } = action {
            return self.apply_power_payment(*action, payment);
        }
        if self.phase != Phase::Playing {
            return Err("Start a match before taking actions.".into());
        }
        if self.decision.is_some()
            && !matches!(
                action,
                Action::ChooseTriggerTargets { .. }
                    | Action::Choose { .. }
                    | Action::SelectCards { .. }
                    | Action::PlayFromTrash { .. }
                    | Action::ChooseTarget { .. }
                    | Action::PlayFromTrashWithTargets { .. }
                    | Action::PlayFromTrashOptions { .. }
                    | Action::AssignDamage { .. }
                    | Action::ChooseDeathReplacement { .. }
                    | Action::OrderTrigger { .. }
                    | Action::ChooseResolutionGroup { .. }
                    | Action::ChooseBattlefield { .. }
                    | Action::PlaceToken { .. }
                    | Action::FinalizePendingPlay { .. }
            )
            && !(self.resource_payment_pending()
                && matches!(
                    action,
                    Action::ActivateGear { .. }
                        | Action::ActivateLegend { .. }
                        | Action::ActivateRune { .. }
                        | Action::RecycleRune { .. }
                        | Action::SetOptionalDiscount { .. }
                ))
        {
            return Err("Resolve the pending choice before taking another action.".into());
        }
        let player = self.priority();
        let cleanup = !matches!(
            action,
            Action::Pass
                | Action::SetOptionalDiscount { .. }
                | Action::ChooseResolutionGroup { .. }
                | Action::PlaceToken { .. }
                | Action::ActivateRune { .. }
                | Action::RecycleRune { .. }
                | Action::ActivateGear { .. }
                | Action::ActivateLegend { .. }
        );
        match action {
            Action::ChooseTriggerTargets { targets } => self.choose_trigger_targets(targets)?,
            Action::FinalizePendingPlay {
                lane,
                targets,
                repeat_targets,
                repeat,
                accelerate,
                bonus,
            } => {
                if !matches!(self.decision, Some(Decision::PendingPlay { .. })) {
                    return Err("There is no pending card to finalize.".into());
                }
                self.play_from_trash_mode(
                    usize::MAX,
                    lane,
                    targets,
                    false,
                    TrashOptions {
                        play: PlayOptions {
                            repeat,
                            accelerate,
                            bonus,
                        },
                        flow: None,
                    },
                    repeat_targets,
                )?;
            }
            Action::WithPowerPayment { .. } => unreachable!("payment wrapper handled above"),
            Action::PlayFromTrashOptions {
                trash_index,
                lane,
                targets,
                repeat_targets,
                repeat,
                accelerate,
                bonus,
                flow,
            } => self.play_from_trash_mode(
                trash_index,
                lane,
                targets,
                false,
                TrashOptions {
                    play: PlayOptions {
                        repeat,
                        accelerate,
                        bonus,
                    },
                    flow,
                },
                repeat_targets,
            )?,
            Action::PlaceToken { lane } => self.place_token(lane)?,
            Action::ChooseResolutionGroup { index } => self.choose_resolution_group(index)?,
            Action::ChooseBattlefield { lane } => self.choose_battlefield_order(lane)?,
            Action::SetOptionalDiscount { power } => {
                let count = self.players[player]
                    .units
                    .iter()
                    .filter(|u| !u.token && u.card_id == 35)
                    .count() as u8;
                if power > count {
                    return Err("Choose no more power discounts than your Ezreals supply.".into());
                }
                self.optional_power_discount[player] = power;
            }
            Action::OrderTrigger { trigger_id } => self.order_trigger(trigger_id)?,
            Action::ChooseDeathReplacement { unit_id, gear_id } => {
                self.choose_death_replacement(unit_id, gear_id)?
            }
            Action::ActivateRune { domain, recycle } => {
                self.activate_rune(player, domain, recycle)?
            }
            Action::RecycleRune { domain, exhausted } => {
                self.recycle_rune(player, domain, exhausted)?
            }
            Action::PlayCard {
                hand_index,
                lane,
                target,
            } => {
                let id = *self.players[player]
                    .hand
                    .get(hand_index)
                    .ok_or("That card is no longer in your hand.")?;
                let resolved = self.validate_play(id, lane, target)?;
                let targets = self.prepare_targets(
                    player,
                    id,
                    lane,
                    &resolved.into_iter().collect::<Vec<_>>(),
                )?;
                self.validate_play_cost(player, id, &targets)?;
                self.spend_play_cost(player, id, &targets)?;
                self.players[player].hand.remove(hand_index);
                self.play_or_queue_targets(player, id, lane, targets);
            }
            Action::PlayCardWithTargets {
                hand_index,
                targets,
                lane,
            } => {
                let id = *self.players[player]
                    .hand
                    .get(hand_index)
                    .ok_or("That card is no longer in your hand.")?;
                self.validate_play_structure(id, lane)?;
                let targets = self.prepare_targets_explicit(player, id, lane, &targets)?;
                self.validate_play_cost(player, id, &targets)?;
                self.spend_play_cost(player, id, &targets)?;
                self.players[player].hand.remove(hand_index);
                self.play_or_queue_targets(player, id, lane, targets);
            }
            Action::PlayChampion { lane, target } => {
                let id = self.players[player]
                    .champion
                    .ok_or("Your chosen champion has already been played.")?;
                let resolved = self.validate_play(id, lane, target)?;
                let targets = self.prepare_targets(
                    player,
                    id,
                    lane,
                    &resolved.into_iter().collect::<Vec<_>>(),
                )?;
                self.validate_play_cost(player, id, &targets)?;
                self.spend_play_cost(player, id, &targets)?;
                self.players[player].champion = None;
                self.play_or_queue_targets(player, id, lane, targets);
                if let Some(ability) = self.legend_on_offhand_play(player) {
                    self.queue_ability(player, id, Vec::new(), ChainAbility::Legend(ability));
                }
            }
            Action::Move { unit_id, lane } => {
                if self.in_window() {
                    return Err("Standard movement is only allowed in an open main phase. Move your attacking group together.".into());
                }
                self.move_group(&[unit_id], lane)?;
            }
            Action::MoveGroup { unit_ids, lane } => {
                if self.in_window() {
                    return Err(
                        "Standard movement is not allowed during a chain or showdown.".into(),
                    );
                }
                self.move_group(&unit_ids, lane)?;
            }
            Action::Attack { lane } => {
                if let Some(showdown) = &self.showdown {
                    if showdown.lane != lane {
                        return Err("Finish the current showdown first.".into());
                    }
                    self.pass()?;
                } else if self.contested(lane) {
                    self.resolve_combat(lane, self.active);
                } else {
                    return Err("There is no showdown at that battlefield.".into());
                }
            }
            Action::Hide { hand_index, lane } => self.hide_card(player, hand_index, lane)?,
            Action::HideChampion { lane } => self.hide_champion(player, lane)?,
            Action::PlayHidden { hidden_id, targets } => {
                self.play_hidden(player, hidden_id, targets)?
            }
            Action::PlayCardOptions {
                hand_index,
                lane,
                targets,
                repeat_targets,
                repeat,
                accelerate,
                bonus,
            } => self.play_options(
                Some(hand_index),
                lane,
                targets,
                repeat_targets,
                PlayOptions {
                    repeat,
                    accelerate,
                    bonus,
                },
            )?,
            Action::PlayChampionOptions {
                lane,
                target,
                accelerate,
            } => self.play_options(
                None,
                lane,
                target.into_iter().collect(),
                Vec::new(),
                PlayOptions {
                    accelerate,
                    ..PlayOptions::default()
                },
            )?,
            Action::EmpowerUnit { unit_id, pay_power } => self.empower_unit(unit_id, pay_power)?,
            Action::ChooseTarget { target } => self.choose_target(target)?,
            Action::Equip { gear_id, target } => self.gear_equip(player, gear_id, target)?,
            Action::EquipWithTrash {
                gear_id,
                target,
                trash_indices,
            } => self.gear_equip_with_trash(player, gear_id, target, &trash_indices)?,
            Action::ActivateGear { gear_id } => self.gear_activate(player, gear_id)?,
            Action::ActivateLegend { target } => self.activate_legend(target)?,
            Action::SelectCards { indices } => self.select_cards(&indices)?,
            Action::PlayFromTrashWithTargets {
                trash_index,
                lane,
                targets,
            } => self.play_from_trash_targets(trash_index, lane, targets)?,
            Action::PlayFromTrash {
                trash_index,
                lane,
                target,
            } => self.play_from_trash(trash_index, lane, target)?,
            Action::AssignDamage { assignments } => self.assign_combat_damage(assignments)?,
            Action::Pass => self.pass()?,
            Action::Choose { accept } => self.choose(accept)?,
            Action::EndTurn => {
                if self.in_window() {
                    return Err(
                        "Pass priority to finish the chain or showdown before ending the turn."
                            .into(),
                    );
                }
                // The explicit combat button teaches grouping; ending the turn
                // also resolves any staged battles, so the game cannot stall.
                for lane in 0..BATTLEFIELDS {
                    if self.contested(lane) {
                        self.resolve_combat(lane, self.active);
                        return Ok(());
                    }
                    if self.phase == Phase::Finished {
                        return Ok(());
                    }
                }
                self.begin_ending();
            }
        }
        if cleanup {
            self.after_resolution();
        } else {
            self.advance_triggers();
        }
        Ok(())
    }

    fn move_group(&mut self, unit_ids: &[u32], lane: Option<usize>) -> Result<(), String> {
        let player = self.active;
        if unit_ids.is_empty() {
            return Err("Select at least one unit to move.".into());
        }
        if lane.is_some_and(|l| l >= BATTLEFIELDS) {
            return Err("Choose one of the two battlefields.".into());
        }
        let mut indices = Vec::with_capacity(unit_ids.len());
        for &id in unit_ids {
            let index = self.players[player]
                .units
                .iter()
                .position(|u| u.id == id)
                .ok_or("Choose only units you control.")?;
            if indices.contains(&index) {
                return Err("A group cannot contain the same unit twice.".into());
            }
            let unit = &self.players[player].units[index];
            if !unit.ready {
                return Err("Every selected unit must be ready before moving.".into());
            }
            if self.movement_locks.contains(&(player, unit.id)) {
                return Err("Vex prevents you from moving this unit this turn.".into());
            }
            if unit.lane == lane {
                return Err("A selected unit is already at the destination.".into());
            }
            if unit.lane.is_some() && lane.is_some() && !self.unit_keywords(unit).ganking {
                return Err("Only a unit with Ganking may move between battlefields.".into());
            }
            indices.push(index);
        }
        let mut origins = Vec::new();
        let mut moved_ids = Vec::new();
        for &index in &indices {
            self.exhaust_unit(player, self.players[player].units[index].id);
        }
        self.trigger_batch += 1;
        for index in indices {
            let unit = &self.players[player].units[index];
            let name = self.unit_name(unit);
            moved_ids.push((unit.id, unit.lane));
            if let Some(old) = unit.lane
                && !origins.contains(&old)
            {
                origins.push(old);
            }
            self.players[player].units[index].lane = lane;
            self.players[player].units[index].ready = false;
            let destination =
                lane.map_or_else(|| "base".into(), |l| format!("battlefield {}", l + 1));
            self.push_log(format!(
                "{} moves {name} to {destination}.",
                self.players[player].name
            ));
        }
        for (id, origin) in moved_ids {
            self.on_move(player, id, origin, lane);
        }
        self.trigger_batch += 1;
        for old in origins {
            self.refresh_control(old);
        }
        if let Some(destination) = lane
            && (self.battlefield_owner[destination] != Some(player) || self.contested(destination))
        {
            // Move triggers finalize and resolve before a staged showdown can
            // open. Tokens they play can change the eventual combat group.
            self.stage_entry(destination, player);
        }
        Ok(())
    }

    fn play_or_queue_targets(
        &mut self,
        player: usize,
        id: CardId,
        lane: Option<usize>,
        targets: Vec<Target>,
    ) {
        let target = targets.first().copied();
        let chosen_targets = targets.clone();
        self.cards_played[player] += 1;
        if self.catalog[id].kind == CardKind::Spell {
            if self.chain.is_empty() {
                self.chain_initiator = Some(player);
            }
            let chain_id = self.next_chain_id;
            self.next_chain_id += 1;
            self.chain.push(ChainItem {
                id: chain_id,
                player,
                card_id: id,
                target,
                targets,
                lane,
                ability: None,
                recycle_after: false,
                banish_after: false,
                from_hidden: false,
                options: PlayOptions::default(),
                repeat_targets: Vec::new(),
            });
            self.chain_passes = 0;
            self.priority_player = player;
            if let Some(showdown) = &mut self.showdown {
                showdown.passes = 0;
            }
            self.push_log(format!(
                "{} casts {}. Reactions may be added before it resolves.",
                self.players[player].name, self.catalog[id].name
            ));
        } else {
            let opened = self.chain.is_empty();
            if opened {
                self.chain_initiator = Some(player);
            }
            // Permanents finalize and resolve immediately, but playing one is
            // still an intervening chain item: previous passes no longer count.
            self.chain_passes = 0;
            self.resolve_play(player, id, lane, targets);
        }
        if self.catalog[id].kind == CardKind::Spell {
            self.trigger_chosen(player, &chosen_targets);
        }
    }

    /// Add-resource abilities are legal while an instructed payment is pending.
    pub fn resource_payment_pending(&self) -> bool {
        matches!(
            self.decision,
            Some(
                Decision::ReplaySpell { .. }
                    | Decision::PendingPlay { .. }
                    | Decision::LastRites { .. }
                    | Decision::ReviveUnit { .. }
                    | Decision::RevivePlay { .. }
                    | Decision::Legend { .. }
                    | Decision::Battlefield { .. }
                    | Decision::CounterTax { .. }
                    | Decision::OnPlayTarget { .. }
                    | Decision::TriggerTargets { .. }
            )
        )
    }

    fn after_spell_played(&mut self, player: usize, id: CardId, offhand: bool, hidden: bool) {
        self.emit_play_event(Occurrence::SpellPlayed, player, id, false, hidden);
        // Core 420.4: played triggers occur only after successful resolution.
        let ravens: Vec<_> = self.players[player]
            .units
            .iter()
            .filter(|u| !u.token && u.card_id == 60)
            .map(|u| u.id)
            .collect();
        for unit_id in ravens {
            self.queue_ability(
                player,
                60,
                Vec::new(),
                ChainAbility::Triggered(TriggeredAbility::Might { unit_id, amount: 1 }),
            );
        }
        if offhand && let Some(ability) = self.legend_on_offhand_play(player) {
            self.queue_ability(player, id, Vec::new(), ChainAbility::Legend(ability));
        }
        // A completed spell play is a later event than the actions it executed.
        self.trigger_batch += 1;
    }

    fn on_unit_played(&mut self, player: usize, unit_id: u32) {
        if let Some(lane) = self.players[player]
            .units
            .iter()
            .find(|unit| unit.id == unit_id)
            .and_then(|unit| unit.lane)
        {
            self.stage_entry(lane, player);
        }
        self.emit_unit_event(Occurrence::UnitPlayed, player, unit_id);
        // Tokens are played units, although they are not played cards.
        let vexes = self.players[1 - player]
            .units
            .iter()
            .filter(|u| !u.token && u.card_id == 20 && u.lane.is_some())
            .count();
        for _ in 0..vexes {
            self.queue_ability(
                1 - player,
                20,
                Vec::new(),
                ChainAbility::Triggered(TriggeredAbility::Stun { player, unit_id }),
            );
        }
        if self.players[player]
            .units
            .iter()
            .any(|u| u.id == unit_id && u.token)
        {
            // Each token is a distinct Play action (CR303/350). Keep all
            // observers of this token together, but do not merge later tokens
            // or the enclosing spell's completed-play event into this batch.
            // Real-card offhand observers are collected by the play caller.
            self.trigger_batch += 1;
        }
    }

    fn open_showdown(&mut self, lane: usize, attacker: usize) {
        self.pending_entries.retain(|&(field, _)| field != lane);
        self.trigger_state.combat_seen.clear();
        self.showdown = Some(Showdown {
            lane,
            attacker,
            focus: attacker,
            passes: 0,
            combat: self.contested(lane),
        });
        self.sync_combat_triggers();
        self.advance_triggers();
        self.priority_player = self.chain.last().map_or(attacker, |item| item.player);
        self.push_log(format!("Battlefield {} showdown opens. Action and Reaction cards are legal; both players must pass to continue.", lane + 1));
    }

    // Contested belongs to the first entering player, independently of the
    // chain's initiator and the turn player (CR190.3.a.1, 345, 466.2.c.1).
    fn stage_entry(&mut self, lane: usize, contender: usize) {
        if self.battlefield_owner[lane] != Some(contender)
            && !self
                .showdown
                .as_ref()
                .is_some_and(|window| window.lane == lane)
            && !self.pending_entries.iter().any(|&(field, _)| field == lane)
        {
            self.pending_entries.push((lane, contender));
        }
    }

    fn reconcile_staged_battlefields(&mut self) {
        for (lane, contender) in self.pending_entries.clone() {
            if self.lane_count(contender, lane) == 0 {
                self.pending_entries.retain(|&(field, _)| field != lane);
                let other = 1 - contender;
                if self.lane_count(other, lane) > 0 {
                    self.stage_entry(lane, other);
                }
            }
        }
    }

    fn open_staged_battlefield(&mut self) -> bool {
        self.reconcile_staged_battlefields();
        // A non-combat showdown is selected before any staged combat. Within
        // either class the turn player chooses, not the Contested applier.
        for combat in [false, true] {
            let mut lanes: Vec<_> = self
                .pending_entries
                .iter()
                .filter_map(|&(lane, _)| (self.contested(lane) == combat).then_some(lane))
                .collect();
            lanes.sort_unstable();
            if lanes.len() > 1 {
                self.decision = Some(Decision::BattlefieldOrder {
                    player: self.active,
                    lanes,
                    combat,
                });
                return true;
            }
            if let Some(&lane) = lanes.first() {
                let (_, contender) = self
                    .pending_entries
                    .iter()
                    .find(|&&(field, _)| field == lane)
                    .copied()
                    .unwrap();
                self.open_showdown(lane, contender);
                return true;
            }
        }
        false
    }

    fn choose_battlefield_order(&mut self, lane: usize) -> Result<(), String> {
        let Some(Decision::BattlefieldOrder { lanes, combat, .. }) = &self.decision else {
            return Err("There is no pending battlefield order choice.".into());
        };
        if !lanes.contains(&lane) || self.contested(lane) != *combat {
            return Err("Choose one of the staged battlefields shown.".into());
        }
        let contender = self
            .pending_entries
            .iter()
            .find_map(|&(field, player)| (field == lane).then_some(player))
            .ok_or("That battlefield is no longer staged.")?;
        self.decision = None;
        self.open_showdown(lane, contender);
        Ok(())
    }

    fn pass(&mut self) -> Result<(), String> {
        if self.decision.is_some() {
            return Err("Resolve the pending choice first.".into());
        }
        if self.chain.is_empty() && self.showdown.is_none() {
            return Err("There is no response window. End your turn when finished.".into());
        }
        let actor = self.priority();
        self.push_log(format!(
            "{} passes {}.",
            self.players[actor].name,
            if self.chain.is_empty() {
                "focus"
            } else {
                "priority"
            }
        ));
        if !self.chain.is_empty() {
            self.chain_passes += 1;
            if self.chain_passes < 2 {
                self.priority_player = 1 - actor;
                return Ok(());
            }
            self.chain_passes = 0;
            return self.resolve_top_chain();
        }
        if let Some(showdown) = &mut self.showdown {
            showdown.passes += 1;
            if showdown.passes < 2 {
                showdown.focus = 1 - showdown.focus;
                self.priority_player = showdown.focus;
                return Ok(());
            }
            let window = self.showdown.take().unwrap();
            if window.combat || self.contested(window.lane) {
                self.resolve_combat(window.lane, window.attacker);
                return Ok(());
            } else {
                self.refresh_control(window.lane);
            }
            self.priority_player = self.active;
            self.push_log("The showdown closes; the turn player's main phase resumes.".into());
            self.after_resolution();
            return Ok(());
        }
        Err("There is no response window. End your turn when finished.".into())
    }

    fn resolution_groups(
        &self,
        player: usize,
        card_id: CardId,
        lane: Option<usize>,
        targets: &[Target],
    ) -> Vec<Vec<Target>> {
        let mut groups: Vec<Vec<Target>> = Vec::new();
        for (slot, &target) in targets.iter().enumerate() {
            if !self.official_target_valid(player, card_id, lane, target, slot) {
                continue;
            }
            let location = self.target_location(target);
            if let Some(group) = groups
                .iter_mut()
                .find(|group| self.target_location(group[0]) == location)
            {
                group.push(target);
            } else {
                groups.push(vec![target]);
            }
        }
        groups
    }

    fn choose_resolution_group(&mut self, index: usize) -> Result<(), String> {
        let Some(Decision::ResolutionTargets {
            ref groups,
            repeated,
            ..
        }) = self.decision
        else {
            return Err("No resolution target choice is pending.".into());
        };
        let targets = groups
            .get(index)
            .ok_or("Choose one of these target groups.")?
            .clone();
        let item = self
            .chain
            .last_mut()
            .ok_or("The resolving spell is missing.")?;
        if repeated {
            item.repeat_targets = targets;
        } else {
            item.targets = targets;
            item.target = item.targets.first().copied();
        }
        self.decision = None;
        self.resolve_top_chain()
    }

    fn place_token(&mut self, lane: Option<usize>) -> Result<(), String> {
        let Some(Decision::TokenPlacement { player, .. }) = self.decision else {
            return Err("No token placement is pending.".into());
        };
        if lane.is_some_and(|lane| {
            lane >= BATTLEFIELDS || self.battlefield_owner[lane] != Some(player)
        }) {
            return Err("Play the token at your base or a battlefield you control.".into());
        }
        let token = self
            .pending_token
            .take()
            .ok_or("The pending token is missing.")?;
        self.decision = None;
        self.finish_token_play(token, lane);
        self.after_resolution();
        Ok(())
    }

    fn resolve_top_chain(&mut self) -> Result<(), String> {
        let item = self.chain.last().expect("resolving chain");
        if let Some(ChainAbility::CardTrigger { rule_index, .. }) = &item.ability
            && let Some((discard, draw)) = Self::trigger_loot(item.card_id, *rule_index)
            && self.pending_loot != Some(item.id)
        {
            self.pending_loot = Some(item.id);
            self.decision = Some(Decision::Discard {
                player: item.player,
                count: discard,
                draw_after: draw,
            });
            return Ok(());
        }
        if let OfficialEffect::Loot { discard, draw } = rules::effect(item.card_id)
            && (item.ability.is_none()
                || matches!(item.ability, Some(ChainAbility::CardEffect { .. })))
            && self.pending_loot != Some(item.id)
        {
            self.pending_loot = Some(item.id);
            self.decision = Some(Decision::Discard {
                player: item.player,
                count: discard,
                draw_after: draw,
            });
            return Ok(());
        }
        if item.card_id == 50 && item.ability.is_none() {
            for repeated in [false, true] {
                if repeated && !item.options.repeat {
                    continue;
                }
                let targets = if repeated {
                    &item.repeat_targets
                } else {
                    &item.targets
                };
                let groups = self.resolution_groups(item.player, item.card_id, item.lane, targets);
                if groups.len() > 1 {
                    self.decision = Some(Decision::ResolutionTargets {
                        player: item.player,
                        card_id: item.card_id,
                        groups,
                        repeated,
                    });
                    return Ok(());
                }
            }
        }
        let item = self.chain.pop().unwrap();
        let loot_finished = self.pending_loot == Some(item.id);
        if loot_finished {
            self.pending_loot = None;
        }
        let physical_spell = item.ability.is_none();
        if let Some(ability) = item.ability.clone() {
            match ability {
                ChainAbility::CardTrigger {
                    rule_index,
                    context,
                } => {
                    if !loot_finished {
                        self.resolve_card_trigger(
                            item.player,
                            item.card_id,
                            rule_index,
                            context,
                            &item.targets,
                        );
                    }
                }
                ChainAbility::PlayChosen {
                    trash_index,
                    card_id,
                    kind,
                } => {
                    if self.players[item.player].discard.get(trash_index) == Some(&card_id) {
                        self.queue_card_play(item.player, trash_index, kind);
                    }
                }
                ChainAbility::GrantFlowChosen {
                    trash_index,
                    card_id,
                } => {
                    if self.players[item.player].discard.get(trash_index) == Some(&card_id)
                        && !self.temporary_flow[item.player].contains(&trash_index)
                    {
                        self.temporary_flow[item.player].push(trash_index);
                    }
                }
                ChainAbility::Disempower { player, object_id } => {
                    if let Some(id) = object_id {
                        self.empowered_units.retain(|&unit| unit != id);
                    } else {
                        self.legend[player].empowered = false;
                    }
                }
                ChainAbility::OptionalPlay { gear_id } => {
                    self.required_trash_play = true;
                    self.enqueue_decision(if let Some(gear_id) = gear_id {
                        Decision::LastRites {
                            player: item.player,
                            gear_id,
                        }
                    } else {
                        Decision::ReplaySpell {
                            player: item.player,
                        }
                    });
                }
                ChainAbility::ReviveChosen {
                    trash_index,
                    card_id,
                } => {
                    if self.players[item.player].discard.get(trash_index) == Some(&card_id) {
                        self.queue_card_play(
                            item.player,
                            trash_index,
                            InstructedPlayKind::Matriarch,
                        );
                    }
                }
                ChainAbility::EmpowerObject {
                    player,
                    object_id,
                    lane,
                    source_id,
                } => {
                    if source_id.is_some_and(|id| {
                        self.unit_ref(Target::Unit {
                            player: item.player,
                            unit_id: id,
                        })
                        .is_none()
                    }) {
                        self.after_resolution();
                        return Ok(());
                    }
                    let valid = if let Some(id) = object_id {
                        let target = Target::Unit {
                            player,
                            unit_id: id,
                        };
                        self.unit_ref(target).is_some_and(|unit| {
                            unit.lane == lane
                                && self.official_target_valid(item.player, 7, None, target, 0)
                        }) || self.players[player]
                            .gear
                            .iter()
                            .any(|gear| gear.id == id && self.gear_location(player, id) == lane)
                    } else {
                        lane.is_none()
                    };
                    if valid {
                        self.empower_now(player, object_id, Some(item.player));
                    }
                }
                ChainAbility::Equip { gear_id, target } => {
                    self.gear_resolve_equip(item.player, gear_id, target)
                }
                ChainAbility::Battlefield(ability) => {
                    self.resolve_battlefield_ability(item.player, ability)
                }
                ChainAbility::Empower { unit_id } => {
                    self.empower_now(item.player, Some(unit_id), None)
                }
                ChainAbility::Swap { unit_id, target } => {
                    self.resolve_swap(item.player, unit_id, target)
                }
                ChainAbility::Legend(ability) => self.resolve_legend_ability(item.player, ability),
                ChainAbility::CardEffect { entering } => {
                    // "Here" is the source's current location, not the location
                    // where its trigger happened (CR359.3.e.12 / 359.3.f.2).
                    let source = entering.and_then(|unit_id| {
                        self.unit_ref(Target::Unit {
                            player: item.player,
                            unit_id,
                        })
                    });
                    let here = rules::token_placement(item.card_id) == rules::TokenPlacement::Here;
                    if !loot_finished && (!here || source.is_some()) {
                        let lane = if here {
                            source.unwrap().lane
                        } else {
                            item.lane
                        };
                        let targets = if let Some(origin) =
                            entering.and_then(|id| self.hidden_play_origin(id))
                        {
                            item.targets
                                .iter()
                                .copied()
                                .filter(|&target| {
                                    source.is_some()
                                        && self.hidden_target_allowed(item.card_id, origin, target)
                                })
                                .collect()
                        } else {
                            item.targets.clone()
                        };
                        self.resolve_operation(item.player, item.card_id, lane, targets, entering);
                    }
                }
                ChainAbility::Triggered(ability) => {
                    self.resolve_trigger(item.player, item.card_id, ability)
                }
            }
        } else if self.is_counter(item.card_id) {
            self.resolve_counter(&item);
            if item.options.repeat {
                let mut second = item.clone();
                second.target = item.repeat_targets.first().copied();
                second.targets = item.repeat_targets.clone();
                second.options.repeat = false;
                self.resolve_counter(&second);
            }
            self.players[item.player].discard.push(item.card_id);
        } else {
            let targets = if item.from_hidden {
                item.targets
                    .iter()
                    .copied()
                    .filter(|&target| {
                        item.lane.is_some_and(|lane| {
                            self.hidden_target_allowed(item.card_id, lane, target)
                        })
                    })
                    .collect()
            } else {
                item.targets.clone()
            };
            if item.options.bonus
                && item.card_id == 29
                && let Some(&target) = targets.first()
            {
                self.modify_might(target, 2, 0);
            }
            if !loot_finished {
                self.push_log(format!(
                    "{} resolves {}.",
                    self.players[item.player].name, self.catalog[item.card_id].name
                ));
                self.resolve_operation(item.player, item.card_id, item.lane, targets, None);
                if item.options.repeat {
                    self.resolve_operation(
                        item.player,
                        item.card_id,
                        item.lane,
                        item.repeat_targets.clone(),
                        None,
                    );
                }
            }
            // A physical spell stays out of trash until every repeated
            // instruction finishes, including draws that may cause burnout.
            if matches!(rules::effect(item.card_id), OfficialEffect::ExtraTurn) {
                self.players[item.player].banish.push(item.card_id);
            } else {
                self.players[item.player].discard.push(item.card_id);
            }
        }
        if physical_spell {
            self.after_spell_played(
                item.player,
                item.card_id,
                item.from_hidden || item.recycle_after || item.banish_after,
                item.from_hidden,
            );
        }
        if (item.recycle_after || item.banish_after)
            && let Some(index) = self.players[item.player]
                .discard
                .iter()
                .rposition(|&id| id == item.card_id)
        {
            self.remove_from_trash(item.player, index);
            if item.banish_after {
                self.players[item.player].banish.push(item.card_id);
            } else {
                self.players[item.player].deck.insert(0, item.card_id);
            }
        }
        self.after_resolution();
        Ok(())
    }

    pub(crate) fn after_resolution(&mut self) {
        for player in [self.active, 1 - self.active] {
            self.check_winner(player);
        }
        if self.winner.is_some() {
            return;
        }
        self.remove_defeated();
        if matches!(self.decision, Some(Decision::DeathReplacement { .. })) {
            return;
        }
        self.sync_combat_triggers();
        self.reconcile_staged_battlefields();
        if self.pending_combat_cleanup.is_some() {
            self.finish_combat_cleanup();
            return;
        }
        if self.decision.is_none() {
            while let Some(decision) = self.pending_decisions.pop_front() {
                if matches!(&decision,Decision::CounterTax{chain_id,..} if !self.chain.iter().any(|item|item.id==*chain_id))
                {
                    continue;
                }
                self.decision = Some(decision);
                break;
            }
        }
        if self.decision.is_some() {
            return;
        }
        self.advance_triggers();
        if self.decision.is_some() {
            return;
        }
        self.chain_passes = 0;
        if let Some(item) = self.chain.last() {
            self.priority_player = item.player;
            return;
        }
        // CR323.5–6: staged fields do not retain an absent controller in an
        // open state. Only an ongoing showdown/combat preserves that control.
        for lane in 0..BATTLEFIELDS {
            if !self
                .showdown
                .as_ref()
                .is_some_and(|window| window.lane == lane)
                && self.battlefield_owner[lane]
                    .is_some_and(|owner| self.lane_count(owner, lane) == 0)
            {
                self.battlefield_owner[lane] = None;
            }
        }
        self.cleanup_lost_hidden();
        let initiator = self.chain_initiator.take();
        // CR323.13: a non-combat showdown becomes combat only in Showdown
        // Open State, after the chain and pending finalizations finish.
        if let Some(lane) = self.showdown.as_ref().map(|window| window.lane)
            && self.contested(lane)
        {
            self.showdown.as_mut().unwrap().combat = true;
            self.sync_combat_triggers();
            if !self.pending_triggers.is_empty() {
                self.after_resolution();
                return;
            }
        }
        if let Some(showdown) = &mut self.showdown {
            if let Some(player) = initiator {
                showdown.focus = 1 - player;
            }
            showdown.passes = 0;
            self.priority_player = showdown.focus;
        } else {
            self.priority_player = self.active;
            // Reconcile externally restored board positions too; real plays
            // record the Contested applier when each unit enters the field.
            for lane in 0..BATTLEFIELDS {
                if self.contested(lane)
                    && !self.pending_entries.iter().any(|&(field, _)| field == lane)
                {
                    let contender =
                        self.battlefield_owner[lane].map_or(self.active, |owner| 1 - owner);
                    self.stage_entry(lane, contender);
                }
            }
            if self.open_staged_battlefield() {
                return;
            }
            for lane in 0..BATTLEFIELDS {
                self.refresh_control(lane);
            }
            self.advance_triggers();
            if self.chain.is_empty() && self.decision.is_none() && self.winner.is_none() {
                if self.ending_turn {
                    self.finish_ending();
                } else if self.pending_start.is_some() {
                    self.advance_start_turn();
                }
            }
        }
    }

    fn choose(&mut self, accept: bool) -> Result<(), String> {
        let decision = self.decision.clone().ok_or("There is no pending choice.")?;
        if matches!(decision, Decision::TriggerTargets { .. }) {
            if accept {
                return Err("Choose the required targets before finalizing this ability.".into());
            }
            self.choose_trigger_targets(Vec::new())?;
            self.after_resolution();
            return Ok(());
        }
        if matches!(decision, Decision::PendingPlay { .. }) {
            if accept || !self.can_decline_trash_play() {
                return Err("Finalize the pending card with legal choices and payment.".into());
            }
            let card = self
                .pending_card
                .take()
                .ok_or("The pending card is missing.")?;
            // CR358.5 undoes a play that cannot finalize, including its move
            // out of the originating trash. No completed-play trigger occurs.
            self.restore_failed_card(card);
            self.decision = None;
            self.after_resolution();
            return Ok(());
        }
        if let Decision::CounterTax { player, cost, .. } = decision
            && accept
            && self.players[player].energy < cost
        {
            return Err(format!(
                "You need {cost} energy to pay this counterspell's additional cost."
            ));
        }
        if matches!(
            decision,
            Decision::BattlefieldOrder { .. }
                | Decision::TrashTarget { .. }
                | Decision::TokenPlacement { .. }
                | Decision::ResolutionTargets { .. }
                | Decision::OrderTriggers { .. }
                | Decision::OnPlayTarget { .. }
                | Decision::RevivePlay { .. }
                | Decision::DeathReplacement { .. }
                | Decision::CombatDamage { .. }
                | Decision::Discard { .. }
                | Decision::Sabotage { .. }
                | Decision::GrantFlow { .. }
        ) {
            return Err("Select the required cards to continue.".into());
        }
        if matches!(
            decision,
            Decision::LastRites { .. } | Decision::ReplaySpell { .. } | Decision::ReviveUnit { .. }
        ) && accept
        {
            return Err("Choose a legal card in the trash to play, or decline.".into());
        }
        if matches!(decision, Decision::TopThree { .. }) {
            if accept {
                return Err("Choose a revealed card, or decline to draw one.".into());
            }
            return self.select_cards(&[]);
        }
        if let Decision::Legend { player, trigger } = decision.clone() {
            let mut probe = self.clone();
            probe.decision = None;
            probe.resolve_legend_choice(player, trigger, accept)?;
        }
        if matches!(
            decision,
            Decision::SwapUnit { .. } | Decision::EmpowerHere { .. }
        ) && accept
        {
            return Err("Choose a target on the board to accept this ability.".into());
        }
        if let Decision::Battlefield { player, lane } = decision.clone() {
            let mut probe = self.clone();
            probe.decision = None;
            probe.resolve_battlefield_choice(player, lane, accept)?;
        }
        if !accept
            && matches!(
                decision,
                Decision::ReplaySpell { .. } | Decision::LastRites { .. }
            )
            && !self.can_decline_trash_play()
        {
            return Err("This accepted trigger must play an eligible card if possible.".into());
        }
        if matches!(
            decision,
            Decision::ReplaySpell { .. } | Decision::LastRites { .. }
        ) {
            self.required_trash_play = false;
        }
        self.decision = None;
        match decision {
            Decision::OptionalPlay {
                player,
                card_id,
                gear_id,
            } => {
                if accept {
                    self.decision = Some(Decision::TrashTarget {
                        player,
                        source_card_id: card_id,
                        gear_id,
                    });
                }
            }
            Decision::TriggerTargets { .. }
            | Decision::BattlefieldOrder { .. }
            | Decision::TrashTarget { .. }
            | Decision::PendingPlay { .. }
            | Decision::TokenPlacement { .. }
            | Decision::ResolutionTargets { .. }
            | Decision::OrderTriggers { .. }
            | Decision::OnPlayTarget { .. }
            | Decision::RevivePlay { .. }
            | Decision::DeathReplacement { .. } => {
                return Err("Choose a replacement effect.".into());
            }
            Decision::CombatDamage { .. } => unreachable!(),
            Decision::RevealHand { .. } => {}
            Decision::Battlefield { player, lane } => {
                self.resolve_battlefield_choice(player, lane, accept)?
            }
            Decision::ReviveUnit { .. }
            | Decision::SwapUnit { .. }
            | Decision::EmpowerHere { .. } => {}
            Decision::Legend { player, trigger } => {
                self.resolve_legend_choice(player, trigger, accept)?
            }
            Decision::LastRites { .. } | Decision::ReplaySpell { .. } => {}
            Decision::BurnForMight { player, unit_id } => {
                if accept {
                    self.burn(player, 1);
                    if self.winner.is_none() {
                        self.queue_finalized_ability(
                            player,
                            43,
                            Vec::new(),
                            ChainAbility::Triggered(TriggeredAbility::Might { unit_id, amount: 1 }),
                        );
                    }
                }
            }
            Decision::Discard { .. }
            | Decision::Sabotage { .. }
            | Decision::TopThree { .. }
            | Decision::GrantFlow { .. } => unreachable!(),
            Decision::Predict { player, card_id } => {
                if accept && self.players[player].deck.last() == Some(&card_id) {
                    self.players[player].deck.pop();
                    self.players[player].deck.insert(0, card_id);
                }
                self.push_log(format!(
                    "{} predicts and {} the top card.",
                    self.players[player].name,
                    if accept { "recycles" } else { "keeps" }
                ));
            }
            Decision::CounterTax {
                player,
                chain_id,
                cost,
            } => {
                if accept {
                    self.pay_energy(player, cost);
                    self.push_log(format!(
                        "{} pays {cost} energy. The spell stays on the chain.",
                        self.players[player].name
                    ));
                } else {
                    self.counter_spell(chain_id, false);
                }
            }
        }
        self.after_resolution();
        Ok(())
    }

    fn is_counter(&self, id: CardId) -> bool {
        matches!(rules::effect(id), OfficialEffect::Counter(_))
    }

    fn counter_target_valid(&self, actor: usize, card: CardId, chain_id: u32) -> bool {
        let Some(item) = self.chain.iter().find(|item| item.id == chain_id) else {
            return false;
        };
        let target = self.catalog[item.card_id];
        let OfficialEffect::Counter(kind) = rules::effect(card) else {
            return false;
        };
        if item.ability.is_some() && kind != CounterKind::NotSoFast {
            return false;
        }
        match kind {
            CounterKind::Abandon | CounterKind::HardBargain | CounterKind::Unconditional => true,
            CounterKind::Defy => target.cost <= 4 && target.power_cost <= 1,
            CounterKind::NotSoFast => {
                item.player != actor
                    && item.targets.iter().any(
                        |target| matches!(target, Target::Unit { player, .. } | Target::Gear {player,..} if *player == actor),
                    )
            }
        }
    }

    fn default_counter_target(&self, actor: usize, card: CardId) -> Option<Target> {
        self.chain
            .iter()
            .rev()
            .filter(|item| self.counter_target_valid(actor, card, item.id))
            .find(|item| item.player != actor)
            .or_else(|| {
                self.chain
                    .iter()
                    .rev()
                    .find(|item| self.counter_target_valid(actor, card, item.id))
            })
            .map(|item| Target::Spell { chain_id: item.id })
    }

    fn resolve_counter(&mut self, item: &ChainItem) {
        let Some(Target::Spell { chain_id }) = item.target else {
            return;
        };
        if !self.counter_target_valid(item.player, item.card_id, chain_id) {
            self.push_log(format!(
                "{} has no legal counter target.",
                self.catalog[item.card_id].name
            ));
            // Abandon's Predict is independent of the counter instruction.
            // Mistargeting only negates the impossible instruction (359.3.e.5).
            if rules::effect(item.card_id) == OfficialEffect::Counter(CounterKind::Abandon) {
                self.predict(item.player);
            }
            return;
        }
        match rules::effect(item.card_id) {
            OfficialEffect::Counter(CounterKind::Abandon) => {
                self.counter_spell(chain_id, true);
                self.predict(item.player);
            }
            OfficialEffect::Counter(
                CounterKind::Defy | CounterKind::NotSoFast | CounterKind::Unconditional,
            ) => self.counter_spell(chain_id, false),
            OfficialEffect::Counter(CounterKind::HardBargain) => {
                let target_player = self
                    .chain
                    .iter()
                    .find(|target| target.id == chain_id)
                    .unwrap()
                    .player;
                if self.players[target_player].energy >= 2 {
                    self.enqueue_decision(Decision::CounterTax {
                        player: target_player,
                        chain_id,
                        cost: 2,
                    });
                    self.push_log(format!(
                        "{} may pay 2 energy to prevent Hard Bargain countering their spell.",
                        self.players[target_player].name
                    ));
                } else {
                    self.counter_spell(chain_id, false);
                }
            }
            _ => {}
        }
    }

    fn counter_spell(&mut self, chain_id: u32, to_hand: bool) {
        if let Some(index) = self.chain.iter().position(|item| item.id == chain_id) {
            let item = self.chain.remove(index);
            if item.ability.is_some() {
                self.push_log("The targeted ability is countered.".into());
                return;
            }
            let destination = if item.banish_after {
                self.players[item.player].banish.push(item.card_id);
                "banished by Flow's delayed replacement"
            } else if item.recycle_after {
                self.players[item.player].deck.insert(0, item.card_id);
                "recycled by the replay instruction"
            } else if to_hand {
                self.players[item.player].hand.push(item.card_id);
                "returned to its owner's hand"
            } else {
                self.players[item.player].discard.push(item.card_id);
                "put into its owner's trash"
            };
            self.push_log(format!(
                "{} is countered and {destination}.",
                self.catalog[item.card_id].name
            ));
        }
    }

    fn optional_cost(
        &self,
        player: usize,
        id: CardId,
        options: PlayOptions,
    ) -> Result<(u8, u8), String> {
        let mut energy: u8 = 0;
        let mut power: u8 = 0;
        if options.repeat {
            let cost = rules::repeat_cost(id).ok_or("This card has no Repeat cost.")?;
            energy += cost.energy;
            power += cost.power;
        }
        if options.accelerate {
            let cost = rules::accelerate_cost(id).ok_or("This unit has no Accelerate ability.")?;
            energy += cost.energy;
            power += cost.power;
        }
        if options.bonus {
            if id != 29 {
                return Err("This card has no optional might bonus cost.".into());
            }
            power += 1;
        }
        let ezreals = self.players[player]
            .units
            .iter()
            .filter(|u| !u.token && u.card_id == 35)
            .count() as u8;
        if options.repeat || options.accelerate || options.bonus {
            let power_reductions = self.optional_power_discount[player].min(ezreals);
            power = power.saturating_sub(power_reductions);
            energy = energy.saturating_sub(ezreals - power_reductions);
        }
        Ok((energy, power))
    }

    fn play_options(
        &mut self,
        hand_index: Option<usize>,
        lane: Option<usize>,
        provided: Vec<Target>,
        repeat_provided: Vec<Target>,
        options: PlayOptions,
    ) -> Result<(), String> {
        let player = self.priority();
        let id = if let Some(index) = hand_index {
            *self.players[player]
                .hand
                .get(index)
                .ok_or("Choose a card in your hand.")?
        } else {
            self.players[player]
                .champion
                .ok_or("Your chosen champion has already been played.")?
        };
        self.validate_play_structure(id, lane)?;
        let targets = self.prepare_targets_explicit(player, id, lane, &provided)?;
        let repeated = if options.repeat {
            self.prepare_targets_explicit(player, id, lane, &repeat_provided)?
        } else {
            if !repeat_provided.is_empty() {
                return Err("Enable Repeat before choosing its targets.".into());
            }
            Vec::new()
        };
        let (extra_energy, extra_power) = self.optional_cost(player, id, options)?;
        let mut probe = self.clone();
        probe.catalog[id].cost = probe.catalog[id].cost.saturating_add(extra_energy);
        probe.catalog[id].power_cost = probe.catalog[id].power_cost.saturating_add(extra_power);
        let mut all_targets = targets.clone();
        all_targets.extend(&repeated);
        probe.validate_play_cost(player, id, &all_targets)?;
        let original = self.catalog[id];
        self.catalog[id] = probe.catalog[id];
        let payment_result = self.spend_play_cost(player, id, &all_targets);
        self.catalog[id] = original;
        payment_result?;
        if let Some(index) = hand_index {
            self.players[player].hand.remove(index);
        } else {
            self.players[player].champion = None;
        }
        let entering_id = self.next_unit_id;
        self.play_or_queue_targets(player, id, lane, targets);
        if original.kind == CardKind::Spell {
            if let Some(item) = self
                .chain
                .iter_mut()
                .rev()
                .find(|i| i.player == player && i.card_id == id && i.ability.is_none())
            {
                item.options = options;
                item.repeat_targets = repeated.clone();
            }
        } else if options.accelerate
            && let Some(unit) = self.players[player]
                .units
                .iter_mut()
                .find(|u| u.id == entering_id)
        {
            unit.ready = true;
        }
        if options.repeat {
            self.trigger_chosen(player, &repeated);
        }
        if hand_index.is_none()
            && let Some(ability) = self.legend_on_offhand_play(player)
        {
            self.queue_ability(player, id, Vec::new(), ChainAbility::Legend(ability));
        }
        Ok(())
    }

    pub fn unit_empowered(&self, unit_id: u32) -> bool {
        self.empowered_units.contains(&unit_id)
    }

    fn empower_unit(&mut self, unit_id: u32, pay_power: bool) -> Result<(), String> {
        if self.in_window() || self.priority() != self.active {
            return Err("Activate Empower in your open main phase.".into());
        }
        let player = self.priority();
        let unit = self.players[player]
            .units
            .iter()
            .find(|u| u.id == unit_id)
            .ok_or("Choose one of your units.")?;
        let id = unit.card_id;
        if unit.token || !matches!(id, 24 | 45) {
            return Err("This unit has no activated Empower ability.".into());
        }
        if self.unit_empowered(unit_id) {
            return Err("This unit is already empowered.".into());
        }
        let (energy, power, domain) = if id == 24 {
            if pay_power {
                (0, 1, Domain::Body)
            } else {
                (1, 0, Domain::Body)
            }
        } else {
            (2, 1, Domain::Chaos)
        };
        if self.players[player].energy < energy
            || self.available_power(player, Some(domain)) < power
        {
            return Err("Not enough energy and power to Empower this unit.".into());
        }
        self.pay_energy(player, energy);
        self.spend_power(player, Some(domain), power)?;
        self.queue_ability(player, id, Vec::new(), ChainAbility::Empower { unit_id });
        Ok(())
    }

    fn empower_now(
        &mut self,
        player: usize,
        unit: Option<u32>,
        temporary_controller: Option<usize>,
    ) {
        if let Some(id) = unit {
            let Some(source) = self.players[player]
                .units
                .iter()
                .find(|u| u.id == id)
                .map(|u| u.card_id)
                .or_else(|| {
                    self.players[player]
                        .gear
                        .iter()
                        .find(|g| g.id == id)
                        .map(|g| g.card_id)
                })
            else {
                return;
            };
            if self.unit_empowered(id) {
                if let Some(controller) = temporary_controller
                    && !self.ending_turn
                {
                    self.temporary_empower.push((controller, player, unit));
                }
                return;
            }
            self.empowered_units.push(id);
            if source == 45 {
                self.queue_trigger_choice(player, 45, Decision::ReviveUnit { player });
            }
        } else {
            self.legend[player].empowered = true;
        }
        if let Some(controller) = temporary_controller
            && !self.ending_turn
        {
            self.temporary_empower.push((controller, player, unit));
        }
    }

    pub fn on_play_target_valid(
        &self,
        player: usize,
        card_id: CardId,
        source_id: u32,
        lane: Option<usize>,
        target: Target,
    ) -> bool {
        if let Some(origin) = self.hidden_play_origin(source_id)
            && (self
                .unit_ref(Target::Unit {
                    player,
                    unit_id: source_id,
                })
                .is_none()
                || !self.hidden_target_allowed(card_id, origin, target))
        {
            return false;
        }
        if rules::equipment(card_id).is_some_and(|gear| gear.quick_draw) {
            matches!(target,Target::Unit{player:owner,..} if owner==player)
                && self.unit_ref(target).is_some()
        } else {
            self.unit_ref(target)
                .is_some_and(|u| !matches!(card_id, 23 | 27) || u.id != source_id)
                && self.official_target_valid(player, card_id, lane, target, 0)
        }
    }

    pub(crate) fn request_on_play_target(
        &mut self,
        player: usize,
        card_id: CardId,
        source_id: u32,
        lane: Option<usize>,
    ) {
        let possible_power = self.available_power(player, None) + self.gear_available_power(player);
        let exists = self.players.iter().enumerate().any(|(owner, state)| {
            state.units.iter().any(|unit| {
                let target = Target::Unit {
                    player: owner,
                    unit_id: unit.id,
                };
                self.on_play_target_valid(player, card_id, source_id, lane, target)
                    && self.deflect_cost(player, &[target]) <= possible_power
            })
        });
        if exists {
            self.queue_trigger_choice(
                player,
                card_id,
                Decision::OnPlayTarget {
                    player,
                    card_id,
                    source_id,
                    lane,
                },
            );
        }
    }

    fn choose_target(&mut self, target: Option<Target>) -> Result<(), String> {
        let decision = self
            .decision
            .clone()
            .ok_or("There is no target choice pending.")?;
        match decision {
            Decision::OnPlayTarget {
                player,
                card_id,
                source_id,
                lane,
            } => {
                let target = target.ok_or("Choose a target for this triggered ability.")?;
                if !self.on_play_target_valid(player, card_id, source_id, lane, target) {
                    return Err("Choose an eligible unit for this triggered ability.".into());
                }
                self.spend_deflect(player, false, &[target])?;
                self.decision = None;
                let ability = if rules::equipment(card_id).is_some_and(|gear| gear.quick_draw) {
                    ChainAbility::Equip {
                        gear_id: source_id,
                        target,
                    }
                } else {
                    ChainAbility::CardEffect {
                        entering: Some(source_id),
                    }
                };
                self.queue_finalized_ability(player, card_id, vec![target], ability);
                self.chain.last_mut().unwrap().lane = lane;
            }
            Decision::SwapUnit { player, unit_id } => {
                if let Some(target) = target {
                    let Some(source) = self.unit_ref(Target::Unit { player, unit_id }) else {
                        self.decision = None;
                        self.after_resolution();
                        return Ok(());
                    };
                    let source_lane = source.lane;
                    if !matches!(target,Target::Unit{player:owner,..} if owner==player)
                        || self.unit_ref(target).is_none_or(|u| u.lane == source_lane)
                    {
                        return Err("Choose a friendly unit at another location.".into());
                    }
                    self.decision = None;
                    self.queue_ability(
                        player,
                        18,
                        vec![target],
                        ChainAbility::Swap { unit_id, target },
                    );
                } else {
                    self.decision = None;
                }
            }
            Decision::EmpowerHere {
                player,
                lane,
                source_id,
            } => {
                if source_id.is_some_and(|id| {
                    self.unit_ref(Target::Unit {
                        player,
                        unit_id: id,
                    })
                    .is_none()
                }) {
                    self.decision = None;
                    self.after_resolution();
                    return Ok(());
                }
                let (owner, unit) = if let Some(target) = target {
                    match target {
                        Target::Unit {
                            player: owner,
                            unit_id,
                        } if self.unit_ref(target).is_some_and(|u| {
                            u.lane == lane && (lane.is_some() || owner == player)
                        }) =>
                        {
                            (owner, Some(unit_id))
                        }
                        Target::Gear {
                            player: owner,
                            gear_id,
                        } if self
                            .players
                            .get(owner)
                            .is_some_and(|p| p.gear.iter().any(|g| g.id == gear_id))
                            && self.gear_location(owner, gear_id) == lane
                            && (lane.is_some() || owner == player) =>
                        {
                            (owner, Some(gear_id))
                        }
                        _ => return Err("Choose a unit or gear at this location.".into()),
                    }
                } else {
                    if lane.is_some() {
                        return Err(
                            "Your legend is at your base; choose an object at this battlefield."
                                .into(),
                        );
                    }
                    (player, None)
                };
                if let Some(target) = target {
                    if matches!(target, Target::Unit { .. })
                        && !self.official_target_valid(player, 7, None, target, 0)
                    {
                        return Err("That unit cannot be chosen by this ability.".into());
                    }
                    self.spend_deflect(player, false, &[target])?;
                }
                self.decision = None;
                self.queue_finalized_ability(
                    player,
                    46,
                    target.into_iter().collect(),
                    ChainAbility::EmpowerObject {
                        player: owner,
                        object_id: unit,
                        lane,
                        source_id,
                    },
                );
            }
            _ => return Err("This choice does not choose a board target.".into()),
        }
        self.after_resolution();
        Ok(())
    }

    fn resolve_swap(&mut self, player: usize, unit_id: u32, target: Target) {
        let source = Target::Unit { player, unit_id };
        let Some((a, b)) = self
            .unit_ref(source)
            .zip(self.unit_ref(target))
            .map(|(a, b)| (a.lane, b.lane))
        else {
            return;
        };
        let Target::Unit {
            player: owner,
            unit_id: other,
        } = target
        else {
            return;
        };
        if a == b
            || owner != player
            || self.movement_locks.contains(&(player, unit_id))
            || self.movement_locks.contains(&(player, other))
        {
            return;
        }
        self.unit_mut(source).unwrap().lane = b;
        self.unit_mut(target).unwrap().lane = a;
        self.on_move(player, unit_id, a, b);
        self.on_move(player, other, b, a);
        self.trigger_batch += 1;
        for lane in [a, b].into_iter().flatten() {
            self.stage_entry(lane, player);
        }
        for lane in [a, b].into_iter().flatten() {
            self.refresh_control(lane);
        }
    }

    pub(crate) fn spend_deflect(
        &mut self,
        player: usize,
        is_spell: bool,
        targets: &[Target],
    ) -> Result<(), String> {
        let tax = self.deflect_cost(player, targets);
        self.consume_power(player, &ALL_DOMAINS, tax, is_spell)
    }

    pub(crate) fn validate_hidden_targets(
        &self,
        player: usize,
        id: CardId,
        lane: usize,
        targets: &[Target],
    ) -> Result<Vec<Target>, String> {
        if self.silenced[player] {
            return Err("You cannot play cards for the rest of this turn.".into());
        }
        let targets = self.prepare_targets(player, id, None, targets)?;
        let tax = self.deflect_cost(player, &targets);
        if self.power_available(
            player,
            &ALL_DOMAINS,
            self.catalog[id].kind == CardKind::Spell,
        ) < tax
        {
            return Err("Not enough power to pay Deflect.".into());
        }
        if lane >= BATTLEFIELDS {
            return Err("Choose a battlefield.".into());
        }
        Ok(targets)
    }

    pub(crate) fn queue_hidden_card(
        &mut self,
        player: usize,
        id: CardId,
        lane: usize,
        targets: Vec<Target>,
    ) {
        let source_id = self.next_unit_id;
        if self.catalog[id].kind == CardKind::Unit {
            self.hidden_play_origins.push((self.next_unit_id, lane));
        }
        self.trigger_state.hidden_play = true;
        self.play_or_queue_targets(player, id, Some(lane), targets);
        self.trigger_state.hidden_play = false;
        if self.catalog[id].kind == CardKind::Spell {
            if let Some(item) = self
                .chain
                .iter_mut()
                .rev()
                .find(|i| i.player == player && i.card_id == id && i.ability.is_none())
            {
                item.from_hidden = true;
            }
        } else if id == 46 {
            self.queue_trigger_choice(
                player,
                id,
                Decision::EmpowerHere {
                    player,
                    lane: Some(lane),
                    source_id: Some(source_id),
                },
            );
        }
        if self.catalog[id].kind != CardKind::Spell
            && let Some(ability) = self.legend_on_offhand_play(player)
        {
            self.queue_ability(player, id, Vec::new(), ChainAbility::Legend(ability));
        }
        self.after_resolution();
    }

    pub fn hidden_play_origin(&self, unit_id: u32) -> Option<usize> {
        self.hidden_play_origins
            .iter()
            .find_map(|&(id, lane)| (id == unit_id).then_some(lane))
    }

    pub(crate) fn deflect_cost(&self, actor: usize, targets: &[Target]) -> u8 {
        let mut cost: u8 = 0;
        for &target in targets {
            if let Target::Unit { player, .. } = target
                && player != actor
                && let Some(unit) = self.unit_ref(target)
            {
                cost = cost.saturating_add(self.unit_keywords(unit).deflect);
            }
        }
        cost
    }

    fn validate_play_cost(
        &self,
        player: usize,
        id: CardId,
        targets: &[Target],
    ) -> Result<(), String> {
        if !self.can_afford(player, &self.catalog[id]) {
            return Err("Not enough energy or matching power.".into());
        }
        let mut probe = self.clone();
        probe.spend_play_cost(player, id, targets)
    }

    fn spend_play_cost(
        &mut self,
        player: usize,
        id: CardId,
        targets: &[Target],
    ) -> Result<(), String> {
        self.spend(player, id)?;
        self.spend_deflect(player, self.catalog[id].kind == CardKind::Spell, targets)
    }

    pub(crate) fn available_power(&self, player: usize, domain: Option<Domain>) -> u8 {
        let domains: Vec<_> = domain.map_or_else(|| ALL_DOMAINS.to_vec(), |d| vec![d]);
        self.power_available(player, &domains, false)
    }

    pub(crate) fn spend_power(
        &mut self,
        player: usize,
        domain: Option<Domain>,
        amount: u8,
    ) -> Result<(), String> {
        let domains: Vec<_> = domain.map_or_else(|| ALL_DOMAINS.to_vec(), |d| vec![d]);
        self.consume_power(player, &domains, amount, false)
    }

    fn activate_rune(
        &mut self,
        player: usize,
        domain: Domain,
        recycle: bool,
    ) -> Result<(), String> {
        if self.decision.is_some() && !self.resource_payment_pending() {
            return Err("Finish the pending choice before activating a rune.".into());
        }
        let state = &mut self.players[player];
        let d = domain_index(domain);
        if recycle {
            if state.runes[d] == 0 {
                return Err("No rune of that domain is channelled.".into());
            }
            state.runes[d] -= 1;
            if state.exhausted_runes[d] > 0 {
                state.exhausted_runes[d] -= 1;
            }
            state.rune_deck.insert(0, domain);
            state.power_pool[d] += 1;
        } else {
            if state.runes[d] == state.exhausted_runes[d] {
                return Err("No ready rune of that domain is available.".into());
            }
            state.exhausted_runes[d] += 1;
            state.energy_pool += 1;
        }
        state.energy = state.energy_pool
            + state
                .runes
                .iter()
                .zip(state.exhausted_runes)
                .map(|(all, used)| all - used)
                .sum::<u8>();
        self.push_log(format!(
            "{} {} a {} rune, adding {}.",
            self.players[player].name,
            if recycle { "recycles" } else { "exhausts" },
            domain.name(),
            if recycle { "power" } else { "energy" }
        ));
        Ok(())
    }

    fn remove_from_trash(&mut self, player: usize, index: usize) -> CardId {
        for item in &mut self.chain {
            if item.player == player
                && let Some(
                    ChainAbility::ReviveChosen { trash_index, .. }
                    | ChainAbility::PlayChosen { trash_index, .. }
                    | ChainAbility::GrantFlowChosen { trash_index, .. },
                ) = &mut item.ability
            {
                if *trash_index == index {
                    *trash_index = usize::MAX;
                } else if *trash_index > index && *trash_index != usize::MAX {
                    *trash_index -= 1;
                }
            }
        }
        self.temporary_flow[player].retain(|&i| i != index);
        for i in &mut self.temporary_flow[player] {
            if *i > index {
                *i -= 1;
            }
        }
        self.players[player].discard.remove(index)
    }

    pub(crate) fn recycle_cards(&mut self, player: usize, indices: &[usize]) {
        let mut indices = indices.to_vec();
        indices.sort_unstable();
        indices.dedup();
        let mut cards = Vec::new();
        for index in indices.into_iter().rev() {
            if index < self.players[player].discard.len() {
                cards.push(self.remove_from_trash(player, index));
            }
        }
        self.rng.shuffle(&mut cards);
        self.players[player].deck.splice(0..0, cards);
    }

    pub(crate) fn recall_unit(&mut self, player: usize, unit_id: u32, exhaust: bool, heal: bool) {
        if exhaust {
            self.exhaust_unit(player, unit_id);
        }
        if let Some(unit) = self.players[player]
            .units
            .iter_mut()
            .find(|u| u.id == unit_id)
        {
            unit.lane = None;
            if exhaust {
                unit.ready = false;
            }
            if heal {
                unit.damage = 0;
            }
        }
    }

    pub(crate) fn enqueue_decision(&mut self, decision: Decision) {
        if self.decision.is_none() {
            self.decision = Some(decision);
        } else {
            self.pending_decisions.push_back(decision);
        }
    }

    /// Public pending chain objects; their board-entry choices have not all finalized.
    pub fn pending_tokens(&self) -> Vec<PendingToken> {
        self.pending_token
            .iter()
            .cloned()
            .chain(self.pending_triggers.iter().filter_map(|p| {
                if let PendingTriggerEffect::Token(token) = &p.effect {
                    Some(token.clone())
                } else {
                    None
                }
            }))
            .collect()
    }

    pub fn pending_cards(&self) -> Vec<PendingCard> {
        self.pending_card
            .iter()
            .cloned()
            .chain(self.pending_triggers.iter().filter_map(|trigger| {
                if let PendingTriggerEffect::Card(card) = &trigger.effect {
                    Some(card.clone())
                } else {
                    None
                }
            }))
            .collect()
    }

    fn queue_card_play(&mut self, player: usize, trash_index: usize, kind: InstructedPlayKind) {
        // CR354.3: the instruction moves the selected physical card now, but
        // no child choices, payments, or entry occur inside the parent effect.
        if self.silenced[player] {
            return;
        }
        let origin_flow = self.temporary_flow[player].contains(&trash_index);
        let origin_targets = self
            .chain
            .iter()
            .filter_map(|item| {
                if item.player != player {
                    return None;
                }
                let selected = match &item.ability {
                    Some(
                        ChainAbility::PlayChosen { trash_index, .. }
                        | ChainAbility::ReviveChosen { trash_index, .. }
                        | ChainAbility::GrantFlowChosen { trash_index, .. },
                    ) => *trash_index,
                    _ => return None,
                };
                (selected == trash_index).then_some(item.id)
            })
            .collect();
        let card_id = self.remove_from_trash(player, trash_index);
        self.trigger_batch += 1;
        let id = self.next_chain_id;
        self.next_chain_id += 1;
        let card = PendingCard {
            id,
            player,
            card_id,
            kind,
            origin_index: trash_index,
            origin_flow,
            origin_targets,
        };
        self.pending_triggers.push(PendingTrigger {
            batch: self.trigger_batch,
            option: TriggerOption {
                id,
                card_id,
                label: "Pending instructed card play".into(),
            },
            player,
            effect: PendingTriggerEffect::Card(card),
        });
        self.trigger_batch += 1;
    }

    fn restore_failed_card(&mut self, card: PendingCard) {
        let index = card
            .origin_index
            .min(self.players[card.player].discard.len());
        for item in &mut self.chain {
            if item.player == card.player
                && let Some(
                    ChainAbility::PlayChosen { trash_index, .. }
                    | ChainAbility::ReviveChosen { trash_index, .. }
                    | ChainAbility::GrantFlowChosen { trash_index, .. },
                ) = &mut item.ability
            {
                if card.origin_targets.contains(&item.id) {
                    *trash_index = index;
                } else if *trash_index != usize::MAX && *trash_index >= index {
                    *trash_index += 1;
                }
            }
        }
        for selected in &mut self.temporary_flow[card.player] {
            if *selected >= index {
                *selected += 1;
            }
        }
        if card.origin_flow {
            self.temporary_flow[card.player].push(index);
        }
        self.players[card.player]
            .discard
            .insert(index, card.card_id);
    }

    fn queue_token_play(
        &mut self,
        player: usize,
        card_id: CardId,
        might: u8,
        location: Option<Option<usize>>,
        gear: bool,
    ) {
        // CR354.3: a play instructed by a resolving item first becomes Pending.
        // It cannot make choices, enter, or trigger observers until that item
        // finishes and older pending items have finalized (CR337.1.b).
        self.trigger_batch += 1;
        let id = self.next_chain_id;
        self.next_chain_id += 1;
        let token = PendingToken {
            id,
            player,
            card_id,
            might,
            gear,
            location,
        };
        self.pending_triggers.push(PendingTrigger {
            batch: self.trigger_batch,
            option: TriggerOption {
                id,
                card_id,
                label: "Pending token play".into(),
            },
            player,
            effect: PendingTriggerEffect::Token(token),
        });
        self.trigger_batch += 1;
    }

    fn finish_token_play(&mut self, token: PendingToken, lane: Option<usize>) {
        if token.gear {
            self.gear_gold(token.player, true);
        } else {
            let entering = self.create_unit(token.player, token.card_id, true, token.might);
            self.players[token.player].units.last_mut().unwrap().lane = lane;
            self.on_unit_played(token.player, entering);
        }
    }

    pub(crate) fn queue_trigger_choice(
        &mut self,
        player: usize,
        card_id: CardId,
        choice: Decision,
    ) {
        let id = self.next_chain_id;
        self.next_chain_id += 1;
        let label = match &choice {
            Decision::Battlefield { lane, .. } => format!(
                "{} optional trigger",
                crate::battlefields::battlefield(self.battlefields[*lane]).name
            ),
            Decision::Legend { .. } => format!(
                "{} legend",
                self.presets[self.selected_decks[player]].legend_name
            ),
            _ => self.catalog[card_id].name.into(),
        };
        self.pending_triggers.push(PendingTrigger {
            batch: self.trigger_batch,
            option: TriggerOption { id, card_id, label },
            player,
            effect: PendingTriggerEffect::Choice(choice),
        });
    }

    fn advance_triggers(&mut self) {
        while self.decision.is_none() && !self.pending_triggers.is_empty() {
            // Finalize the oldest pending group before triggers generated
            // while finalizing that group. APNAP orders only simultaneous
            // triggers; it cannot move a newer item ahead of an older one.
            let batch = self.pending_triggers.iter().map(|t| t.batch).min().unwrap();
            let player = if self
                .pending_triggers
                .iter()
                .any(|t| t.batch == batch && t.player == self.active)
            {
                self.active
            } else {
                1 - self.active
            };
            let options: Vec<_> = self
                .pending_triggers
                .iter()
                .filter(|t| t.batch == batch && t.player == player)
                .map(|t| t.option.clone())
                .collect();
            if options.len() > 1 {
                self.decision = Some(Decision::OrderTriggers { player, options });
                return;
            }
            self.finalize_trigger(options[0].id);
        }
    }

    fn finalize_trigger(&mut self, trigger_id: u32) {
        let index = self
            .pending_triggers
            .iter()
            .position(|t| t.option.id == trigger_id)
            .expect("offered trigger");
        let trigger = self.pending_triggers.remove(index);
        self.trigger_batch += 1;
        match trigger.effect {
            PendingTriggerEffect::Card(card) => {
                self.decision = Some(Decision::PendingPlay {
                    player: card.player,
                    pending_id: card.id,
                    card_id: card.card_id,
                    kind: card.kind,
                });
                self.pending_card = Some(card);
            }
            PendingTriggerEffect::Choice(choice) => {
                self.enqueue_decision(choice);
                self.discard_unfinalizable_trigger();
            }
            PendingTriggerEffect::Token(token) => {
                if let Some(lane) = token.location {
                    self.finish_token_play(token, lane);
                } else {
                    let remaining = 1 + self.pending_triggers.iter().filter(|pending| matches!(&pending.effect, PendingTriggerEffect::Token(other) if other.player == token.player && other.card_id == token.card_id)).count() as u8;
                    self.decision = Some(Decision::TokenPlacement {
                        player: token.player,
                        card_id: token.card_id,
                        remaining,
                    });
                    self.pending_token = Some(token);
                }
            }
            PendingTriggerEffect::Ability {
                targets,
                ability,
                lane,
            } => {
                self.queue_finalized_ability(
                    trigger.player,
                    trigger.option.card_id,
                    targets,
                    ability,
                );
                self.chain.last_mut().unwrap().lane = lane;
            }
        }
    }

    fn order_trigger(&mut self, trigger_id: u32) -> Result<(), String> {
        let Some(Decision::OrderTriggers { ref options, .. }) = self.decision else {
            return Err("No trigger order is pending.".into());
        };
        if !options.iter().any(|o| o.id == trigger_id) {
            return Err("Choose one of these pending triggers.".into());
        }
        self.decision = None;
        self.finalize_trigger(trigger_id);
        self.advance_triggers();
        Ok(())
    }

    pub(crate) fn queue_ability(
        &mut self,
        player: usize,
        card_id: CardId,
        targets: Vec<Target>,
        ability: ChainAbility,
    ) {
        if matches!(
            ability,
            ChainAbility::Triggered(TriggeredAbility::GrantFlow)
        ) {
            self.queue_trigger_choice(player, card_id, Decision::GrantFlow { player });
            return;
        }
        let optional = match ability {
            ChainAbility::CardEffect { .. } if card_id == 36 => Some(None),
            ChainAbility::Triggered(TriggeredAbility::LastRites { gear_id }) => Some(Some(gear_id)),
            _ => None,
        };
        if let Some(gear_id) = optional {
            self.queue_trigger_choice(
                player,
                card_id,
                Decision::OptionalPlay {
                    player,
                    card_id,
                    gear_id,
                },
            );
            return;
        }
        let triggered = matches!(
            ability,
            ChainAbility::CardTrigger { .. }
                | ChainAbility::Triggered(_)
                | ChainAbility::Disempower { .. }
                | ChainAbility::CardEffect { .. }
                | ChainAbility::Legend(LegendAbility::KennenEmpower)
                | ChainAbility::Battlefield(
                    BattlefieldAbility::SeatDraw { .. } | BattlefieldAbility::MinefieldBurn { .. }
                )
        );
        if !triggered {
            self.queue_finalized_ability(player, card_id, targets, ability);
            return;
        }
        let id = self.next_chain_id;
        self.next_chain_id += 1;
        let label = match &ability {
            ChainAbility::Battlefield(a) => format!(
                "{} trigger",
                crate::battlefields::battlefield(self.battlefields[a.lane()]).name
            ),
            _ => self.catalog[card_id].name.into(),
        };
        self.pending_triggers.push(PendingTrigger {
            batch: self.trigger_batch,
            option: TriggerOption { id, card_id, label },
            player,
            effect: PendingTriggerEffect::Ability {
                targets,
                ability,
                lane: None,
            },
        });
    }

    fn queue_finalized_ability(
        &mut self,
        player: usize,
        card_id: CardId,
        targets: Vec<Target>,
        ability: ChainAbility,
    ) {
        if self.chain.is_empty() {
            self.chain_initiator = Some(player);
        }
        let id = self.next_chain_id;
        self.next_chain_id += 1;
        self.chain.push(ChainItem {
            id,
            player,
            card_id,
            target: targets.first().copied(),
            targets: targets.clone(),
            lane: None,
            ability: Some(ability),
            recycle_after: false,
            banish_after: false,
            from_hidden: false,
            options: PlayOptions::default(),
            repeat_targets: Vec::new(),
        });
        self.chain_passes = 0;
        self.priority_player = player;
        if let Some(showdown) = &mut self.showdown {
            showdown.passes = 0;
        }
        self.trigger_chosen(player, &targets);
        self.push_log(format!(
            "{} adds an ability to the chain.",
            self.players[player].name
        ));
    }

    fn trigger_chosen(&mut self, player: usize, targets: &[Target]) {
        for &target in targets {
            if let Target::Unit {
                player: owner,
                unit_id,
            } = target
                && owner == player
                && self
                    .unit_ref(target)
                    .is_some_and(|u| !u.token && u.card_id == 0)
            {
                self.queue_ability(
                    player,
                    0,
                    Vec::new(),
                    ChainAbility::Triggered(TriggeredAbility::Might { unit_id, amount: 1 }),
                );
            }
        }
        for trigger in self.legend_on_choose(player, targets) {
            self.queue_trigger_choice(player, 0, Decision::Legend { player, trigger });
        }
    }

    pub(crate) fn ready_unit(&mut self, player: usize, unit_id: u32) {
        self.ready_unit_by(player, player, unit_id);
    }

    fn target_location(&self, target: Target) -> Option<(Option<usize>, usize)> {
        let Target::Unit { player, .. } = target else {
            return None;
        };
        let unit = self.unit_ref(target)?;
        Some((unit.lane, if unit.lane.is_none() { player } else { 0 }))
    }

    pub fn trash_target_valid(
        &self,
        player: usize,
        source_card_id: CardId,
        trash_index: usize,
    ) -> bool {
        let Some(&id) = self
            .players
            .get(player)
            .and_then(|p| p.discard.get(trash_index))
        else {
            return false;
        };
        let card = self.catalog[id];
        match source_card_id {
            36 => card.kind == CardKind::Spell && card.cost <= 3,
            38 => card.kind == CardKind::Unit,
            45 => card.kind == CardKind::Unit && card.cost <= 3 && card.power_cost <= 1,
            _ => false,
        }
    }

    fn select_cards(&mut self, indices: &[usize]) -> Result<(), String> {
        let decision = self
            .decision
            .clone()
            .ok_or("There is no card choice to resolve.")?;
        let mut sorted = indices.to_vec();
        sorted.sort_unstable();
        if sorted.windows(2).any(|p| p[0] == p[1]) {
            return Err("Choose different cards.".into());
        }
        match decision {
            Decision::TrashTarget {
                player,
                source_card_id,
                gear_id,
            } => {
                let available = (0..self.players[player].discard.len())
                    .any(|i| self.trash_target_valid(player, source_card_id, i));
                if (available && indices.len() != 1)
                    || (!available && !indices.is_empty())
                    || indices
                        .first()
                        .is_some_and(|&i| !self.trash_target_valid(player, source_card_id, i))
                {
                    return Err(
                        "Choose one eligible card in your trash for the triggered ability.".into(),
                    );
                }
                self.decision = None;
                if let Some(&trash_index) = indices.first() {
                    self.queue_finalized_ability(
                        player,
                        source_card_id,
                        Vec::new(),
                        ChainAbility::PlayChosen {
                            trash_index,
                            card_id: self.players[player].discard[trash_index],
                            kind: if gear_id.is_some() {
                                InstructedPlayKind::LastRites
                            } else {
                                InstructedPlayKind::Fizz
                            },
                        },
                    );
                }
            }
            Decision::GrantFlow { player } => {
                let available = self.players[player]
                    .discard
                    .iter()
                    .any(|&id| self.catalog[id].kind == CardKind::Spell);
                if (available && indices.len() != 1)
                    || (!available && !indices.is_empty())
                    || indices.first().is_some_and(|&i| {
                        i >= self.players[player].discard.len()
                            || self.catalog[self.players[player].discard[i]].kind != CardKind::Spell
                    })
                {
                    return Err("Choose a spell in your trash to grant Flow this turn.".into());
                }
                self.decision = None;
                if let Some(&trash_index) = indices.first() {
                    let card_id = self.players[player].discard[trash_index];
                    self.queue_finalized_ability(
                        player,
                        33,
                        Vec::new(),
                        ChainAbility::GrantFlowChosen {
                            trash_index,
                            card_id,
                        },
                    );
                }
            }
            Decision::Discard {
                player,
                count,
                draw_after,
            } => {
                let required = (count as usize).min(self.players[player].hand.len());
                if sorted.len() != required
                    || sorted.iter().any(|&i| i >= self.players[player].hand.len())
                {
                    return Err(format!("Choose {required} card(s) from your hand."));
                }
                self.decision = None;
                for index in sorted.into_iter().rev() {
                    let card = self.players[player].hand.remove(index);
                    self.players[player].discard.push(card);
                }
                self.push_log(format!(
                    "{} discards {required} and draws {draw_after}.",
                    self.players[player].name
                ));
                self.draw(player, draw_after);
                if self.pending_loot.is_some() {
                    return self.resolve_top_chain();
                }
            }
            Decision::TopThree { player, cards } => {
                if indices.len() > 1 || indices.first().is_some_and(|&i| i >= cards.len()) {
                    return Err("Choose up to one of the revealed cards.".into());
                }
                self.decision = None;
                // Looking does not move cards or burn out. Moving only the cards
                // actually seen cannot overdraw an exhausted deck.
                for (index, &card) in cards.iter().enumerate() {
                    if self.players[player].deck.pop() != Some(card) {
                        return Err("The inspected cards changed unexpectedly.".into());
                    }
                    if indices.first() == Some(&index) {
                        self.players[player].hand.push(card);
                    } else {
                        self.players[player].discard.push(card);
                    }
                }
                self.push_log(format!(
                    "{} finishes looking at the top cards.",
                    self.players[player].name
                ));
            }
            Decision::Sabotage { player, opponent } => {
                let available = self.players[opponent]
                    .hand
                    .iter()
                    .any(|&id| self.catalog[id].kind != CardKind::Unit);
                if (available && indices.len() != 1)
                    || (!available && !indices.is_empty())
                    || indices.first().is_some_and(|&i| {
                        i >= self.players[opponent].hand.len()
                            || self.catalog[self.players[opponent].hand[i]].kind == CardKind::Unit
                    })
                {
                    return Err("Choose one revealed non-unit card, if available.".into());
                }
                self.decision = None;
                if let Some(&index) = indices.first() {
                    let card = self.players[opponent].hand.remove(index);
                    self.players[opponent].deck.insert(0, card);
                    self.push_log(format!(
                        "{} recycles the revealed {}.",
                        self.players[player].name, self.catalog[card].name
                    ));
                }
            }
            _ => return Err("This choice does not select cards.".into()),
        }
        self.after_resolution();
        Ok(())
    }

    // Cancellation depends on existence of a legal explicit choice, never the
    // heuristic preferred target (which may have unaffordable Deflect).
    fn pending_play_actions(&self, limit: usize) -> Vec<Action> {
        let Some(Decision::PendingPlay {
            player, card_id, ..
        }) = self.decision
        else {
            return Vec::new();
        };
        let card = self.catalog[card_id];
        let (_, mut required) = rules::target_bounds(card_id);
        if card.kind != CardKind::Spell {
            required = 0;
        }
        if self.is_counter(card_id)
            || matches!(rules::effect(card_id), OfficialEffect::TemporaryGear)
        {
            required = 1;
        }
        let mut actions = Vec::new();
        for lane in [None, Some(0), Some(1)] {
            let mut groups = vec![Vec::new()];
            for _ in 0..required {
                let mut next = Vec::new();
                for group in groups {
                    let choices: Vec<Target> = if self.is_counter(card_id) {
                        self.chain
                            .iter()
                            .filter(|item| self.counter_target_valid(player, card_id, item.id))
                            .map(|item| Target::Spell { chain_id: item.id })
                            .collect()
                    } else if matches!(rules::effect(card_id), OfficialEffect::TemporaryGear) {
                        self.players
                            .iter()
                            .enumerate()
                            .flat_map(|(player, state)| {
                                state.gear.iter().map(move |gear| Target::Gear {
                                    player,
                                    gear_id: gear.id,
                                })
                            })
                            .collect()
                    } else {
                        self.players
                            .iter()
                            .enumerate()
                            .flat_map(|(player, state)| {
                                state.units.iter().map(move |unit| Target::Unit {
                                    player,
                                    unit_id: unit.id,
                                })
                            })
                            .filter(|&target| {
                                self.valid_card_target(player, card_id, lane, target, &group)
                            })
                            .collect()
                    };
                    for target in choices {
                        let mut chosen = group.clone();
                        chosen.push(target);
                        next.push(chosen);
                    }
                }
                groups = next;
            }
            if let Ok(preferred) = self.prepare_targets(player, card_id, lane, &[])
                && !groups.contains(&preferred)
            {
                groups.insert(0, preferred);
            }
            for targets in groups {
                for options in [
                    PlayOptions::default(),
                    PlayOptions {
                        accelerate: true,
                        ..Default::default()
                    },
                    PlayOptions {
                        repeat: true,
                        ..Default::default()
                    },
                    PlayOptions {
                        bonus: true,
                        ..Default::default()
                    },
                ] {
                    let action = Action::FinalizePendingPlay {
                        lane,
                        repeat_targets: if options.repeat {
                            targets.clone()
                        } else {
                            Vec::new()
                        },
                        targets: targets.clone(),
                        repeat: options.repeat,
                        accelerate: options.accelerate,
                        bonus: options.bonus,
                    };
                    if self.clone().apply(action.clone()).is_ok() {
                        actions.push(action);
                        if actions.len() >= limit {
                            return actions;
                        }
                    }
                }
            }
        }
        actions
    }

    pub fn can_decline_trash_play(&self) -> bool {
        if matches!(self.decision, Some(Decision::PendingPlay { .. })) {
            // Add-resource activations remain available during payment. Test
            // the fully generated resources as well as the current pools.
            let mut probe = self.clone();
            let player = probe.priority();
            let gear_ids: Vec<_> = probe.players[player].gear.iter().map(|g| g.id).collect();
            for gear_id in gear_ids {
                let _ = probe.apply(Action::ActivateGear { gear_id });
            }
            let _ = probe.apply(Action::ActivateLegend { target: None });
            return probe.pending_play_actions(1).is_empty();
        }
        if matches!(self.decision, Some(Decision::RevivePlay { .. })) {
            return false;
        }
        if !self.required_trash_play
            || !matches!(
                self.decision,
                Some(Decision::ReplaySpell { .. } | Decision::LastRites { .. })
            )
        {
            return true;
        }
        for index in 0..self.players[self.priority()].discard.len() {
            for lane in [None, Some(0), Some(1)] {
                if self.clone().play_from_trash(index, lane, None).is_ok() {
                    return false;
                }
            }
        }
        true
    }

    fn play_from_trash(
        &mut self,
        trash_index: usize,
        lane: Option<usize>,
        target: Option<Target>,
    ) -> Result<(), String> {
        self.play_from_trash_mode(
            trash_index,
            lane,
            target.into_iter().collect(),
            true,
            TrashOptions::default(),
            Vec::new(),
        )
    }

    fn play_from_trash_targets(
        &mut self,
        trash_index: usize,
        lane: Option<usize>,
        provided: Vec<Target>,
    ) -> Result<(), String> {
        self.play_from_trash_mode(
            trash_index,
            lane,
            provided,
            false,
            TrashOptions::default(),
            Vec::new(),
        )
    }

    /// The actual Flow instances on this trash object. Listing an instance does
    /// not waive normal timing, targets, or any additional costs.
    pub fn flow_costs(&self, player: usize, trash_index: usize) -> Vec<FlowOption> {
        let Some(state) = self.players.get(player) else {
            return Vec::new();
        };
        let Some(&id) = state.discard.get(trash_index) else {
            return Vec::new();
        };
        let card = self.catalog[id];
        let mut choices = Vec::new();
        if self.temporary_flow[player].contains(&trash_index) {
            choices.push(FlowOption {
                source: FlowSource::Granted,
                energy: card.cost,
                power: card.power_cost,
                colored: true,
            });
        }
        if let Some(cost) = rules::flow_cost(id) {
            choices.push(FlowOption {
                source: FlowSource::Printed,
                energy: cost.energy,
                power: cost.any_power,
                colored: cost.colored,
            });
        }
        choices
    }

    fn pay_trash_cost(
        &mut self,
        player: usize,
        id: CardId,
        base: (u8, u8, bool),
        extra: (u8, u8),
        targets: &[Target],
    ) -> Result<(), String> {
        let card = self.catalog[id];
        let energy = self.discounted_energy(player, id, base.0.saturating_add(extra.0));
        if self.players[player].energy < energy {
            return Err("Not enough energy for the base and additional costs.".into());
        }
        self.pay_energy(player, energy);
        // Satisfy restricted colored obligations before flexible obligations.
        for (amount, colored) in [
            (extra.1, true),
            (base.1, base.2),
            (self.deflect_cost(player, targets), false),
        ] {
            let domains = if colored { card.domains } else { &ALL_DOMAINS };
            self.consume_power(player, domains, amount, card.kind == CardKind::Spell)?;
        }
        Ok(())
    }

    fn play_from_trash_mode(
        &mut self,
        trash_index: usize,
        lane: Option<usize>,
        provided: Vec<Target>,
        automatic: bool,
        options: TrashOptions,
        repeat_provided: Vec<Target>,
    ) -> Result<(), String> {
        let flow_source = options.flow;
        let options = options.play;
        let player = self.priority();
        let pending = if matches!(self.decision, Some(Decision::PendingPlay { .. })) {
            if trash_index != usize::MAX {
                return Err("Finalize the already selected pending card.".into());
            }
            Some(
                self.pending_card
                    .clone()
                    .ok_or("The pending card is missing.")?,
            )
        } else {
            None
        };
        let id = if let Some(card) = &pending {
            card.card_id
        } else {
            *self.players[player]
                .discard
                .get(trash_index)
                .ok_or("Choose a card in your trash.")?
        };
        let card = self.catalog[id];
        let decision = pending.as_ref().map_or_else(
            || self.decision.clone(),
            |card| {
                Some(match card.kind {
                    InstructedPlayKind::Fizz => Decision::ReplaySpell { player },
                    InstructedPlayKind::LastRites => Decision::LastRites { player, gear_id: 0 },
                    InstructedPlayKind::Matriarch => Decision::RevivePlay {
                        player,
                        trash_index,
                        card_id: id,
                    },
                })
            },
        );
        let free_energy = matches!(decision, Some(Decision::ReplaySpell { .. }));
        let selecting_revive = matches!(decision, Some(Decision::ReviveUnit { .. }));
        let free_all = selecting_revive || matches!(decision, Some(Decision::RevivePlay { .. }));
        if selecting_revive && options != PlayOptions::default() {
            return Err("Choose Matriarch's unit now; its optional play costs are chosen when the ability resolves.".into());
        }
        if free_all && lane.is_some() {
            return Err("Matriarch plays the unit at your base.".into());
        }
        let flows = self.flow_costs(player, trash_index);
        if flow_source.is_some() && decision.is_some() {
            return Err("Choose a Flow cost only when playing from trash using Flow.".into());
        }
        let chosen_flow = if decision.is_none() {
            if let Some(source) = flow_source {
                Some(
                    *flows
                        .iter()
                        .find(|f| f.source == source)
                        .ok_or("That Flow instance is not available on this trash card.")?,
                )
            } else {
                flows.first().copied()
            }
        } else {
            None
        };
        let flow = chosen_flow.map(|f| (f.energy, f.power, f.colored));
        match decision {
            Some(Decision::LastRites { .. }) if card.kind == CardKind::Unit => {}
            Some(Decision::ReplaySpell { .. })
                if card.kind == CardKind::Spell && card.cost <= 3 => {}
            Some(Decision::ReviveUnit { .. })
                if card.kind == CardKind::Unit && card.cost <= 3 && card.power_cost <= 1 => {}
            Some(Decision::RevivePlay {
                trash_index: selected,
                card_id,
                ..
            }) if selected == trash_index && card_id == id => {}
            None if flow.is_some() => {}
            _ => return Err("This card cannot currently be played from the trash.".into()),
        }
        if selecting_revive {
            if !provided.is_empty() || !repeat_provided.is_empty() {
                return Err("Choose only the trash unit now; its play choices occur after the trigger resolves.".into());
            }
            self.decision = None;
            self.queue_finalized_ability(
                player,
                45,
                Vec::new(),
                ChainAbility::ReviveChosen {
                    trash_index,
                    card_id: id,
                },
            );
            self.after_resolution();
            return Ok(());
        }
        let mut probe = self.clone();
        if pending.is_none() {
            probe.remove_from_trash(player, trash_index);
        }
        probe.decision = None;
        probe.pending_decisions.clear();
        let original_chain = probe.chain.clone();
        if decision.is_some() {
            probe.chain.clear();
            probe.showdown = None;
            probe.active = player;
            probe.priority_player = player;
        }
        probe.catalog[id].cost = 0;
        probe.catalog[id].power_cost = 0;
        probe.validate_play_structure(id, lane)?;
        // Bypassing timing must not erase the spells that a replayed counter can target.
        probe.chain = original_chain;
        let targets = probe.prepare_targets_mode(player, id, lane, &provided, automatic)?;
        let repeated = if options.repeat {
            probe.prepare_targets_explicit(player, id, lane, &repeat_provided)?
        } else {
            if !repeat_provided.is_empty() {
                return Err("Enable Repeat before choosing additional targets.".into());
            }
            Vec::new()
        };
        let extra = self.optional_cost(player, id, options)?;
        let mut base = flow.unwrap_or((
            if free_energy || free_all {
                0
            } else {
                card.cost
            },
            if free_all { 0 } else { card.power_cost },
            true,
        ));
        let mut all_targets = targets.clone();
        all_targets.extend(&repeated);
        // Convenience callers still get a legal available instance if the
        // preferred granted cost is unpayable. An explicit choice never falls back.
        if flow_source.is_none() && flow.is_some() {
            for choice in &flows {
                let cost = (choice.energy, choice.power, choice.colored);
                let mut probe = self.clone();
                if pending.is_none() {
                    probe.remove_from_trash(player, trash_index);
                }
                if probe
                    .pay_trash_cost(player, id, cost, extra, &all_targets)
                    .is_ok()
                {
                    base = cost;
                    break;
                }
            }
        }
        let mut cost_probe = self.clone();
        if pending.is_none() {
            cost_probe.remove_from_trash(player, trash_index);
        }
        cost_probe.pay_trash_cost(player, id, base, extra, &all_targets)?;
        self.required_trash_play = false;
        self.decision = None;
        if pending.is_some() {
            self.pending_card = None;
        } else {
            self.remove_from_trash(player, trash_index);
        }
        self.pay_trash_cost(player, id, base, extra, &all_targets)
            .expect("validated trash costs");
        let entering = self.next_unit_id;
        self.play_or_queue_targets(player, id, lane, targets);
        if card.kind == CardKind::Spell {
            if let Some(item) =
                self.chain.iter_mut().rev().find(|item| {
                    item.player == player && item.card_id == id && item.ability.is_none()
                })
            {
                item.recycle_after = free_energy;
                item.banish_after = flow.is_some();
                item.options = options;
                item.repeat_targets = repeated.clone();
            }
            if options.repeat {
                self.trigger_chosen(player, &repeated);
            }
        } else {
            if options.accelerate
                && let Some(unit) = self.players[player]
                    .units
                    .iter_mut()
                    .find(|u| u.id == entering)
            {
                unit.ready = true;
            }
            if let Some(ability) = self.legend_on_offhand_play(player) {
                self.queue_ability(player, id, Vec::new(), ChainAbility::Legend(ability));
            }
        }
        self.after_resolution();
        Ok(())
    }

    fn predict(&mut self, player: usize) {
        // Core 432.1.c: merely looking at an empty deck does not burn out.
        if self.winner.is_some() {
            return;
        }
        if let Some(&card_id) = self.players[player].deck.last() {
            self.enqueue_decision(Decision::Predict { player, card_id });
        }
    }

    pub(crate) fn pay_energy(&mut self, player: usize, mut amount: u8) {
        let state = &mut self.players[player];
        let stored = amount.min(state.energy_pool);
        state.energy_pool -= stored;
        amount -= stored;
        for domain in 0..6 {
            let paid = (state.runes[domain] - state.exhausted_runes[domain]).min(amount);
            state.exhausted_runes[domain] += paid;
            amount -= paid;
        }
        state.energy = state.energy_pool
            + state
                .runes
                .iter()
                .zip(state.exhausted_runes)
                .map(|(all, spent)| all - spent)
                .sum::<u8>();
    }

    fn rune_actions(&self, player: usize) -> Vec<Action> {
        if player != self.priority() {
            return Vec::new();
        }
        ALL_DOMAINS
            .iter()
            .flat_map(|&domain| {
                [false, true].map(move |recycle| Action::ActivateRune { domain, recycle })
            })
            .filter(|action| self.clone().apply(action.clone()).is_ok())
            .collect()
    }

    pub fn legal_actions(&self) -> Vec<Action> {
        if self.phase != Phase::Playing {
            return Vec::new();
        }
        if let Some(decision) = &self.decision {
            let player = decision.player();
            let mut actions = match decision {
                Decision::TriggerTargets { .. } => self.trigger_target_actions(),
                Decision::TrashTarget {
                    player,
                    source_card_id,
                    ..
                } => {
                    let mut choices: Vec<_> = (0..self.players[*player].discard.len())
                        .filter(|&i| self.trash_target_valid(*player, *source_card_id, i))
                        .map(|i| Action::SelectCards { indices: vec![i] })
                        .collect();
                    if choices.is_empty() {
                        choices.push(Action::SelectCards {
                            indices: Vec::new(),
                        });
                    }
                    choices
                }
                Decision::PendingPlay { .. } => {
                    let mut choices = self.pending_play_actions(32);
                    choices.push(Action::Choose { accept: false });
                    choices
                }
                Decision::BattlefieldOrder { lanes, .. } => lanes
                    .iter()
                    .map(|&lane| Action::ChooseBattlefield { lane })
                    .collect(),
                Decision::TokenPlacement { player, .. } => {
                    std::iter::once(Action::PlaceToken { lane: None })
                        .chain(
                            (0..BATTLEFIELDS)
                                .filter(|&lane| self.battlefield_owner[lane] == Some(*player))
                                .map(|lane| Action::PlaceToken { lane: Some(lane) }),
                        )
                        .collect()
                }
                Decision::ResolutionTargets { groups, .. } => (0..groups.len())
                    .map(|index| Action::ChooseResolutionGroup { index })
                    .collect(),
                Decision::OrderTriggers { options, .. } => options
                    .iter()
                    .map(|o| Action::OrderTrigger { trigger_id: o.id })
                    .collect(),
                Decision::OnPlayTarget { .. } => self
                    .players
                    .iter()
                    .enumerate()
                    .flat_map(|(owner, state)| {
                        state.units.iter().map(move |u| Action::ChooseTarget {
                            target: Some(Target::Unit {
                                player: owner,
                                unit_id: u.id,
                            }),
                        })
                    })
                    .collect(),
                Decision::DeathReplacement { options, .. } => options
                    .iter()
                    .map(|choice| Action::ChooseDeathReplacement {
                        unit_id: choice.unit_id,
                        gear_id: choice.gear_id,
                    })
                    .collect(),
                Decision::CombatDamage {
                    player,
                    lane,
                    total,
                } => vec![Action::AssignDamage {
                    assignments: self.automatic_damage_assignment(*player, *lane, *total),
                }],
                Decision::Discard { count, .. } => {
                    let needed = (*count as usize).min(self.players[player].hand.len());
                    if needed == 1 {
                        (0..self.players[player].hand.len())
                            .map(|i| Action::SelectCards { indices: vec![i] })
                            .collect()
                    } else {
                        vec![Action::SelectCards {
                            indices: (0..needed).collect(),
                        }]
                    }
                }
                Decision::TopThree { cards, .. } => (0..cards.len())
                    .map(|i| Action::SelectCards { indices: vec![i] })
                    .chain(std::iter::once(Action::SelectCards {
                        indices: Vec::new(),
                    }))
                    .collect(),
                Decision::Sabotage { opponent, .. } => (0..self.players[*opponent].hand.len())
                    .map(|i| Action::SelectCards { indices: vec![i] })
                    .chain(std::iter::once(Action::SelectCards {
                        indices: Vec::new(),
                    }))
                    .collect(),
                Decision::GrantFlow { .. } => (0..self.players[player].discard.len())
                    .map(|i| Action::SelectCards { indices: vec![i] })
                    .chain(std::iter::once(Action::SelectCards {
                        indices: Vec::new(),
                    }))
                    .collect(),
                Decision::LastRites { .. }
                | Decision::ReplaySpell { .. }
                | Decision::ReviveUnit { .. } => (0..self.players[player].discard.len())
                    .flat_map(|trash_index| {
                        [None, Some(0), Some(1)].map(move |lane| Action::PlayFromTrash {
                            trash_index,
                            lane,
                            target: None,
                        })
                    })
                    .chain(std::iter::once(Action::Choose { accept: false }))
                    .collect(),
                Decision::RevivePlay { trash_index, .. } => [false, true]
                    .into_iter()
                    .map(|accelerate| Action::PlayFromTrashOptions {
                        trash_index: *trash_index,
                        lane: None,
                        targets: Vec::new(),
                        repeat_targets: Vec::new(),
                        repeat: false,
                        accelerate,
                        bonus: false,
                        flow: None,
                    })
                    .collect(),
                Decision::SwapUnit { .. } | Decision::EmpowerHere { .. } => self.players[player]
                    .units
                    .iter()
                    .map(|u| Action::ChooseTarget {
                        target: Some(Target::Unit {
                            player,
                            unit_id: u.id,
                        }),
                    })
                    .chain(std::iter::once(Action::ChooseTarget { target: None }))
                    .chain(std::iter::once(Action::Choose { accept: false }))
                    .collect(),
                _ => vec![
                    Action::Choose { accept: false },
                    Action::Choose { accept: true },
                ],
            };
            if self.resource_payment_pending() {
                actions.extend(self.gear_actions(player));
                actions.extend(self.legend_actions(player));
                actions.extend(self.rune_actions(player));
            }
            actions.retain(|action| self.clone().apply(action.clone()).is_ok());
            return actions;
        }
        let mut actions = self.rune_actions(self.priority());
        for (hand_index, &id) in self.players[self.priority()].hand.iter().enumerate() {
            if self.validate_play(id, None, None).is_ok() {
                actions.push(Action::PlayCard {
                    hand_index,
                    lane: None,
                    target: None,
                });
            }
        }
        if self.players[self.priority()]
            .champion
            .is_some_and(|id| self.validate_play(id, None, None).is_ok())
        {
            actions.push(Action::PlayChampion {
                lane: None,
                target: None,
            });
        }
        actions.extend(self.gear_actions(self.priority()));
        actions.extend(self.hidden_actions(self.priority()));
        for unit in &self.players[self.priority()].units {
            if matches!(unit.card_id, 24 | 45) && !unit.token {
                for pay_power in [false, true] {
                    let action = Action::EmpowerUnit {
                        unit_id: unit.id,
                        pay_power,
                    };
                    if self.clone().apply(action.clone()).is_ok() {
                        actions.push(action);
                    }
                }
            }
        }
        actions.extend(self.legend_actions(self.priority()));
        for trash_index in 0..self.players[self.priority()].discard.len() {
            let action = Action::PlayFromTrash {
                trash_index,
                lane: None,
                target: None,
            };
            if self.clone().apply(action.clone()).is_ok() {
                actions.push(action);
            }
        }
        if self.in_window() {
            actions.push(Action::Pass);
            return actions;
        }
        for unit in &self.players[self.priority()].units {
            if !unit.ready || self.movement_locks.contains(&(self.priority(), unit.id)) {
                continue;
            }
            if unit.lane.is_none() {
                for lane in 0..BATTLEFIELDS {
                    actions.push(Action::Move {
                        unit_id: unit.id,
                        lane: Some(lane),
                    });
                }
            } else {
                actions.push(Action::Move {
                    unit_id: unit.id,
                    lane: None,
                });
                if self.unit_keywords(unit).ganking {
                    for lane in 0..BATTLEFIELDS {
                        if unit.lane != Some(lane) {
                            actions.push(Action::Move {
                                unit_id: unit.id,
                                lane: Some(lane),
                            });
                        }
                    }
                }
            }
        }
        for lane in 0..BATTLEFIELDS {
            if self.contested(lane) {
                actions.push(Action::Attack { lane });
            }
        }
        actions.push(Action::EndTurn);
        actions
    }

    /// Executes exactly one action for the active player. The renderer owns the
    /// pacing and can pause between calls. This also supports headless AI-v-AI.
    pub fn step_ai(&mut self) -> Result<bool, String> {
        if self.phase != Phase::Playing {
            return Ok(false);
        }
        let action = self.choose_ai_action();
        self.ai_steps_this_turn += 1;
        self.apply(action)?;
        Ok(true)
    }

    /// Convenience helper for command-line simulations, never required by UI.
    pub fn run_ai_turn(&mut self) -> Result<(), String> {
        let starting_turn = self.turn;
        while self.phase == Phase::Playing && self.turn == starting_turn {
            self.step_ai()?;
        }
        Ok(())
    }

    pub fn action_hint(&self) -> String {
        if self.phase == Phase::Mulligan {
            return "Choose up to two cards to replace, or keep your opening hand.".into();
        }
        if let Some(winner) = self.winner {
            return format!(
                "{} reached eight points. Start a rematch to practice again.",
                self.players[winner].name
            );
        }
        if self.decision.is_some() {
            return "Resolve the card choice before the chain continues.".into();
        }
        if !self.chain.is_empty() {
            return "Play a Reaction, or pass priority. Two passes resolve the newest spell or ability.".into();
        }
        if self.showdown.is_some() {
            return "Play an Action or Reaction, or pass focus. Two passes finish the showdown."
                .into();
        }
        if self.priority() == 1 {
            return "Watch your opponent's turn, or pause to inspect the board.".into();
        }
        if (0..BATTLEFIELDS).any(|l| self.contested(l)) {
            return "Resolve this battlefield showdown before continuing.".into();
        }
        if self.players[0]
            .units
            .iter()
            .any(|u| u.ready && u.lane.is_none())
        {
            return "Select a ready unit at base, then choose a battlefield to move and conquer."
                .into();
        }
        if self.players[0]
            .hand
            .iter()
            .any(|&id| self.validate_play(id, None, None).is_ok())
        {
            return "Play a card from your hand. Units normally enter exhausted unless their text says otherwise."
                .into();
        }
        "End your turn when finished. Holding a battlefield at your next turn scores again.".into()
    }

    pub fn unit_name(&self, unit: &Unit) -> String {
        if unit.token {
            format!(
                "{} token",
                rules::token_spec(unit.card_id).map_or("Recruit", |spec| spec.name)
            )
        } else {
            self.catalog[unit.card_id].name.to_string()
        }
    }

    fn validate_play(
        &self,
        id: CardId,
        lane: Option<usize>,
        target: Option<Target>,
    ) -> Result<Option<Target>, String> {
        self.validate_play_structure(id, lane)?;
        let actor = self.priority();
        if self.is_counter(id) {
            let selected = target.or_else(|| self.default_counter_target(actor, id));
            if let Some(Target::Spell { chain_id }) = selected
                && self.counter_target_valid(actor, id, chain_id)
            {
                return Ok(selected);
            }
            return Err("Choose an eligible spell on the chain for this counterspell.".into());
        }
        let targets =
            self.prepare_targets(actor, id, lane, &target.into_iter().collect::<Vec<_>>())?;
        self.validate_play_cost(actor, id, &targets)?;
        Ok(targets.first().copied())
    }

    fn validate_play_structure(&self, id: CardId, lane: Option<usize>) -> Result<(), String> {
        let card = self.catalog.get(id).ok_or("Unknown card.")?;
        let actor = self.priority();
        if !self.card_supported(id) {
            return Err(self.effect_description(id));
        }
        if self.silenced[actor] {
            return Err(
                "An effect prevents you from playing cards for the rest of this turn.".into(),
            );
        }
        let abilities = rules::static_abilities(id);
        let ambush = abilities.contains(&StaticAbility::Ambush);
        let enemy_field = abilities.contains(&StaticAbility::MayDeployToEnemyBattlefield);
        let reaction = card
            .official_text
            .lines()
            .any(|line| line.starts_with("[Reaction]") || line.starts_with("[Quick-Draw]"))
            || (ambush
                && lane.is_some_and(|l| {
                    l < BATTLEFIELDS
                        && (self.lane_count(actor, l) > 0
                            || (enemy_field && self.lane_count(1 - actor, l) > 0))
                }));
        let action = card
            .official_text
            .lines()
            .any(|line| line.starts_with("[Action]"));
        if !self.chain.is_empty() && !reaction {
            return Err("Only Reaction cards may be added to an existing chain.".into());
        }
        if self.chain.is_empty() && self.showdown.is_some() && !reaction && !action {
            return Err("Only Action or Reaction cards may be played during a showdown.".into());
        }
        if !self.in_window() && actor != self.active {
            return Err("Wait for a response window or your own main phase.".into());
        }
        if let Some(lane) = lane {
            let movement_spell = matches!(
                rules::effect(id),
                OfficialEffect::MoveEnemy | OfficialEffect::MoveFriendlyAndReady
            );
            let legal_unit_location = lane < BATTLEFIELDS
                && card.kind == CardKind::Unit
                && (self.battlefield_owner[lane] == Some(actor)
                    || (ambush && self.lane_count(actor, lane) > 0)
                    || (enemy_field && self.lane_count(1 - actor, lane) > 0));
            if lane >= BATTLEFIELDS || (!movement_spell && !legal_unit_location) {
                return Err("Deploy a unit at your base or a battlefield you control.".into());
            }
        }
        if !self.can_afford(self.priority(), card) {
            return Err(format!(
                "Need {} energy and {} {:?} power. Power recycles a matching rune.",
                card.cost, card.power_cost, card.domain
            ));
        }
        Ok(())
    }

    fn prepare_targets(
        &self,
        actor: usize,
        id: CardId,
        lane: Option<usize>,
        provided: &[Target],
    ) -> Result<Vec<Target>, String> {
        self.prepare_targets_mode(actor, id, lane, provided, true)
    }

    fn prepare_targets_explicit(
        &self,
        actor: usize,
        id: CardId,
        lane: Option<usize>,
        provided: &[Target],
    ) -> Result<Vec<Target>, String> {
        self.prepare_targets_mode(actor, id, lane, provided, false)
    }

    fn prepare_targets_mode(
        &self,
        actor: usize,
        id: CardId,
        lane: Option<usize>,
        provided: &[Target],
        automatic: bool,
    ) -> Result<Vec<Target>, String> {
        // Targeted play triggers choose after the permanent is on the board.
        if rules::on_play_requires_target(id)
            || rules::equipment(id).is_some_and(|gear| gear.quick_draw)
        {
            if !automatic && !provided.is_empty() {
                return Err(
                    "Choose this permanent's triggered-ability target after it enters.".into(),
                );
            }
            return Ok(Vec::new());
        }
        if self.is_counter(id) {
            if provided.len() > 1 {
                return Err("A counterspell chooses exactly one chain item.".into());
            }
            let target = provided.first().copied().or_else(|| {
                automatic
                    .then(|| self.default_counter_target(actor, id))
                    .flatten()
            });
            return match target {
                Some(Target::Spell { chain_id })
                    if self.counter_target_valid(actor, id, chain_id) =>
                {
                    Ok(vec![Target::Spell { chain_id }])
                }
                _ => Err("Choose an eligible spell on the chain.".into()),
            };
        }
        let effect = rules::effect(id);
        if matches!(
            effect,
            OfficialEffect::DestroyGearAndDraw | OfficialEffect::TemporaryGear
        ) {
            if provided.len() > 1 {
                return Err("Choose at most one gear.".into());
            }
            let target = provided.first().copied().or_else(|| {
                if !automatic {
                    return None;
                }
                self.players
                    .iter()
                    .enumerate()
                    .flat_map(|(player, p)| {
                        p.gear.iter().map(move |g| Target::Gear {
                            player,
                            gear_id: g.id,
                        })
                    })
                    .find(|t| matches!(t, Target::Gear { player, .. } if *player != actor))
            });
            if let Some(Target::Gear { player, gear_id }) = target
                && player < 2
                && self.players[player].gear.iter().any(|g| g.id == gear_id)
            {
                return Ok(vec![Target::Gear { player, gear_id }]);
            }
            if id == 41 && provided.is_empty() {
                return Ok(Vec::new());
            }
            return Err("Choose a gear on the board.".into());
        }
        if self.catalog[id].kind == CardKind::Gear {
            if let Some(&target) = provided.first() {
                if !matches!(target, Target::Unit { player, .. } if player == actor)
                    || self.unit_ref(target).is_none()
                {
                    return Err("Choose a friendly unit for this equipment.".into());
                }
                return Ok(vec![target]);
            }
            return Ok(Vec::new());
        }
        let (maximum, required) = rules::target_bounds(id);
        if maximum == 0 {
            if !automatic && !provided.is_empty() {
                return Err("This card does not choose targets.".into());
            }
            return Ok(Vec::new());
        }
        if provided.len() > maximum {
            return Err(format!("Choose at most {maximum} targets."));
        }
        let mut targets = Vec::new();
        for &target in provided {
            if !self.official_target_valid(actor, id, lane, target, targets.len()) {
                return Err(
                    "This unit does not meet the card's printed targeting restrictions.".into(),
                );
            }
            if rules::targeting(id).distinct && targets.contains(&target) {
                return Err("Choose different units for this card.".into());
            }
            if rules::targeting(id).same_location
                && targets.first().is_some_and(|&first| {
                    self.target_location(first) != self.target_location(target)
                })
            {
                return Err("These targets must be at the same location.".into());
            }
            targets.push(target);
        }
        let desired = if !automatic {
            provided.len()
        } else if provided.is_empty() {
            maximum
        } else {
            required.max(provided.len())
        };
        while targets.len() < desired {
            let mut candidates = Vec::new();
            for owner in 0..2 {
                for unit in &self.players[owner].units {
                    let target = Target::Unit {
                        player: owner,
                        unit_id: unit.id,
                    };
                    if !self.official_target_valid(actor, id, lane, target, targets.len()) {
                        continue;
                    }
                    if rules::targeting(id).distinct && targets.contains(&target) {
                        continue;
                    }
                    if rules::targeting(id).same_location
                        && targets.first().is_some_and(|&first| {
                            self.target_location(first) != self.target_location(target)
                        })
                    {
                        continue;
                    }
                    let hostile = matches!(
                        effect,
                        OfficialEffect::Damage { .. }
                            | OfficialEffect::ContextualDamage(_)
                            | OfficialEffect::DamageAndGold { .. }
                            | OfficialEffect::Bounce { .. }
                            | OfficialEffect::MoveEnemy
                    ) || matches!(effect, OfficialEffect::ModifyMight { amount, .. } if amount < 0)
                        || matches!(effect, OfficialEffect::ContextualMight(rules::SpellMight::OwnExhausted { per_unit }) if per_unit < 0)
                        || (effect == OfficialEffect::DualMight && targets.len() == 1);
                    if hostile && owner == actor && targets.len() >= required {
                        continue;
                    }
                    let preferred = if hostile {
                        owner != actor
                    } else {
                        owner == actor
                    };
                    let rank = if preferred { 1000 } else { 0 }
                        + if effect == OfficialEffect::ReadyAnother && !unit.ready {
                            200
                        } else {
                            0
                        }
                        + if unit.lane.is_some() { 50 } else { 0 }
                        + self.unit_might(unit) as i32;
                    candidates.push((rank, target));
                }
            }
            let Some((_, target)) = candidates.into_iter().max_by_key(|(rank, _)| *rank) else {
                break;
            };
            targets.push(target);
        }
        if targets.len() < required && self.catalog[id].kind != CardKind::Unit {
            return Err("There are not enough eligible targets for this card.".into());
        }
        Ok(targets)
    }

    fn target_lane(&self, target: Target) -> Option<usize> {
        let Target::Unit { player, unit_id } = target else {
            return None;
        };
        self.players
            .get(player)?
            .units
            .iter()
            .find(|u| u.id == unit_id)?
            .lane
    }

    pub fn valid_card_target(
        &self,
        player: usize,
        card_id: CardId,
        lane: Option<usize>,
        target: Target,
        prior: &[Target],
    ) -> bool {
        if card_id >= self.catalog.len() || player > 1 {
            return false;
        }
        self.official_target_valid(player, card_id, lane, target, prior.len())
            && (!rules::targeting(card_id).distinct || !prior.contains(&target))
            && (!rules::targeting(card_id).same_location
                || prior.first().is_none_or(|&first| {
                    self.target_location(first) == self.target_location(target)
                }))
    }

    fn official_target_valid(
        &self,
        actor: usize,
        id: CardId,
        destination: Option<usize>,
        target: Target,
        slot: usize,
    ) -> bool {
        let Target::Unit { player, unit_id } = target else {
            return false;
        };
        let Some(unit) = self
            .players
            .get(player)
            .and_then(|p| p.units.iter().find(|u| u.id == unit_id))
        else {
            return false;
        };
        if player != actor
            && (unit.protected
                || (!unit.token && unit.card_id == 31)
                || (!unit.token
                    && unit.card_id == 2
                    && !self
                        .showdown
                        .as_ref()
                        .is_some_and(|w| unit.lane == Some(w.lane) && w.combat)))
        {
            return false;
        }
        match rules::targeting(id).allegiance {
            Allegiance::Friendly if player != actor => return false,
            Allegiance::Enemy if player == actor => return false,
            Allegiance::Mixed if (slot == 0) != (player == actor) => return false,
            _ => {}
        }
        match rules::effect(id) {
            OfficialEffect::ContextualDamage(_) | OfficialEffect::DamageAndGold { .. } => {
                unit.lane.is_some()
            }
            OfficialEffect::ContextualMight(rules::SpellMight::EnemiesHere { .. }) => {
                unit.lane.is_some()
            }
            OfficialEffect::Fight { .. } if id == 217 && slot == 1 => unit.lane.is_some(),
            OfficialEffect::Damage {
                battlefield_only, ..
            } => !battlefield_only || unit.lane.is_some(),
            OfficialEffect::Bounce {
                max_might,
                battlefield_only,
            } => {
                (!battlefield_only || unit.lane.is_some())
                    && max_might.is_none_or(|max| self.unit_might(unit) <= max)
            }
            OfficialEffect::MoveFriendlyToBase { .. } => unit.lane.is_some(),
            OfficialEffect::MoveEnemy | OfficialEffect::MoveFriendlyAndReady => {
                unit.lane != destination && destination.is_none_or(|l| l < BATTLEFIELDS)
            }
            OfficialEffect::SwapMight => unit.lane.is_some(),
            OfficialEffect::ReadyAnother => true,
            OfficialEffect::BuffAnother => player == actor,
            _ => true,
        }
    }

    pub(crate) fn spend(&mut self, player: usize, id: CardId) -> Result<(), String> {
        let card = self.catalog[id];
        let energy = self.effective_cost(player, &card);
        let mut remaining = energy;
        let stored = remaining.min(self.players[player].energy_pool);
        self.players[player].energy_pool -= stored;
        remaining -= stored;
        for &domain in card.domains {
            let d = domain_index(domain);
            let ready = self.players[player].runes[d] - self.players[player].exhausted_runes[d];
            let paid = remaining.min(ready);
            self.players[player].exhausted_runes[d] += paid;
            remaining -= paid;
        }
        self.pay_energy(player, remaining);
        self.consume_power(
            player,
            card.domains,
            card.power_cost,
            card.kind == CardKind::Spell,
        )
    }

    pub(crate) fn unit_ref(&self, target: Target) -> Option<&Unit> {
        let Target::Unit { player, unit_id } = target else {
            return None;
        };
        self.players
            .get(player)?
            .units
            .iter()
            .find(|unit| unit.id == unit_id)
    }

    pub(crate) fn unit_mut(&mut self, target: Target) -> Option<&mut Unit> {
        let Target::Unit { player, unit_id } = target else {
            return None;
        };
        self.players
            .get_mut(player)?
            .units
            .iter_mut()
            .find(|unit| unit.id == unit_id)
    }

    fn modify_might(&mut self, target: Target, amount: i16, minimum: u8) {
        let Some(unit) = self.unit_ref(target) else {
            return;
        };
        let before = self.unit_might(unit) as i16;
        let delta = if amount < 0 && minimum > 0 {
            amount.max(minimum.min(before as u8) as i16 - before)
        } else {
            amount
        };
        if let Some(unit) = self.unit_mut(target) {
            unit.temporary_buff += delta;
        }
    }

    fn move_effect(
        &mut self,
        actor: usize,
        target: Target,
        destination: Option<usize>,
        ready: bool,
    ) {
        let Target::Unit { player, unit_id } = target else {
            return;
        };
        let Some(index) = self.players[player]
            .units
            .iter()
            .position(|u| u.id == unit_id)
        else {
            return;
        };
        let old = self.players[player].units[index].lane;
        if actor == player && self.movement_locks.contains(&(player, unit_id)) {
            if ready {
                self.ready_unit(player, unit_id);
            }
            return;
        }
        if old == destination {
            return;
        }
        self.players[player].units[index].lane = destination;
        self.on_move(player, unit_id, old, destination);
        self.trigger_batch += 1;
        if ready {
            self.ready_unit_by(actor, player, unit_id);
        }
        if let Some(lane) = destination {
            self.stage_entry(lane, player);
        }
        if let Some(old) = old {
            self.refresh_control(old);
        }
    }

    fn on_move(
        &mut self,
        player: usize,
        unit_id: u32,
        origin: Option<usize>,
        destination: Option<usize>,
    ) {
        if let Some(unit) = self.players[player]
            .units
            .iter()
            .find(|u| u.id == unit_id)
            .cloned()
        {
            let mut event = EventSnapshot::unit(Occurrence::Moved, player, unit);
            event.origin = origin;
            self.emit_event(event);
        }
        let Some(unit) = self.players[player].units.iter().find(|u| u.id == unit_id) else {
            return;
        };
        if unit.token {
            return;
        }
        let source = unit.card_id;
        let ability = match source {
            2 if destination.is_some() => Some(TriggeredAbility::Might { unit_id, amount: 2 }),
            17 => Some(TriggeredAbility::Draw(1)),
            43 => {
                self.queue_trigger_choice(player, 43, Decision::BurnForMight { player, unit_id });
                None
            }
            47 => Some(TriggeredAbility::Loot { count: 1, draw: 1 }),
            _ => None,
        };
        if let Some(ability) = ability {
            self.queue_ability(player, source, Vec::new(), ChainAbility::Triggered(ability));
        }
    }

    fn bounce(&mut self, target: Target) {
        let Target::Unit { player, unit_id } = target else {
            return;
        };
        if let Some(index) = self.players[player]
            .units
            .iter()
            .position(|u| u.id == unit_id)
        {
            self.empowered_units.retain(|&id| id != unit_id);
            self.gear_unit_left_board(player, unit_id);
            let unit = self.players[player].units.remove(index);
            self.push_log(format!(
                "{} returns to its owner's hand.",
                self.unit_name(&unit)
            ));
            if !unit.token {
                self.players[player].hand.push(unit.card_id);
            }
        }
    }

    fn resolve_play(
        &mut self,
        player: usize,
        id: CardId,
        lane: Option<usize>,
        targets: Vec<Target>,
    ) {
        let card = self.catalog[id];
        if card.kind == CardKind::Gear {
            self.gear_play(player, id, targets.first().copied())
                .expect("validated gear play");
            return;
        }
        self.push_log(format!(
            "{} resolves {}.",
            self.players[player].name, card.name
        ));
        if card.kind == CardKind::Unit {
            let entering = self.create_unit(player, id, false, 0);
            self.players[player].units.last_mut().unwrap().lane = lane;
            let legion_active = !rules::static_abilities(id).contains(&StaticAbility::LegionOnPlay)
                || self.cards_played[player] >= 2;
            if legion_active && rules::on_play_requires_target(id) {
                self.request_on_play_target(player, id, entering, lane);
            } else if legion_active
                && rules::effect(id) != OfficialEffect::None
                && rules::effect(id) != OfficialEffect::Unsupported
            {
                self.queue_ability(
                    player,
                    id,
                    targets,
                    ChainAbility::CardEffect {
                        entering: Some(entering),
                    },
                );
                if let Some(PendingTrigger {
                    effect:
                        PendingTriggerEffect::Ability {
                            lane: target_lane, ..
                        },
                    ..
                }) = self.pending_triggers.last_mut()
                {
                    *target_lane = lane;
                }
            }
            if id == 18 {
                self.queue_trigger_choice(
                    player,
                    id,
                    Decision::SwapUnit {
                        player,
                        unit_id: entering,
                    },
                );
            }
            self.on_unit_played(player, entering);
            return;
        }
        self.resolve_operation(player, id, lane, targets, None);
        if matches!(rules::effect(id), OfficialEffect::ExtraTurn) {
            self.players[player].banish.push(id);
        } else {
            self.players[player].discard.push(id);
        }
    }

    fn resolve_operation(
        &mut self,
        player: usize,
        id: CardId,
        lane: Option<usize>,
        targets: Vec<Target>,
        entering: Option<u32>,
    ) {
        let mut valid: Vec<Target> = targets
            .iter()
            .enumerate()
            .filter_map(|(slot, &target)| {
                self.official_target_valid(player, id, lane, target, slot)
                    .then_some(target)
            })
            .collect();
        if rules::targeting(id).same_location && valid.len() > 1 {
            let location = self.target_location(valid[0]);
            valid.retain(|&target| self.target_location(target) == location);
        }
        match rules::effect(id) {
            effect @ (OfficialEffect::EmpoweredFight { .. }
            | OfficialEffect::DamageAndGold { .. }
            | OfficialEffect::ContextualMight(_)
            | OfficialEffect::ContextualDamage(_)
            | OfficialEffect::DrawForBattlefields { .. }
            | OfficialEffect::DrawForMighty) => {
                self.resolve_scalar_spell(player, id, effect, &valid);
            }
            OfficialEffect::DisruptHand => {
                self.enqueue_decision(Decision::Sabotage {
                    player,
                    opponent: 1 - player,
                });
                self.push_log(format!(
                    "{} reveals their hand for Sabotage.",
                    self.players[1 - player].name
                ));
            }
            OfficialEffect::Loot { discard, draw } => self.enqueue_decision(Decision::Discard {
                player,
                count: discard,
                draw_after: draw,
            }),
            OfficialEffect::ChooseTopThreeBurnRest => {
                let cards = self.players[player]
                    .deck
                    .iter()
                    .rev()
                    .take(3)
                    .copied()
                    .collect();
                self.enqueue_decision(Decision::TopThree { player, cards });
            }
            OfficialEffect::ReplayCheapSpell => {
                self.enqueue_decision(Decision::ReplaySpell { player })
            }
            OfficialEffect::DestroyGearAndDraw => {
                if let Some(&Target::Gear {
                    player: owner,
                    gear_id,
                }) = targets.first()
                {
                    self.gear_destroy(owner, gear_id);
                }
                self.draw(player, 1);
            }
            OfficialEffect::TemporaryGear => {
                if let Some(&target) = targets.first() {
                    self.gear_make_temporary(player, id, target);
                }
            }
            OfficialEffect::None | OfficialEffect::Unsupported => {}
            OfficialEffect::Counter(_) => {} // Executed by the chain resolver.
            OfficialEffect::Draw(count) => self.draw(player, count),
            OfficialEffect::Damage {
                amount,
                draw,
                banish_on_death,
                ..
            } => {
                for target in valid {
                    let damage_amount = self.battlefield_damage(target, amount);
                    if let Some(unit) = self.unit_mut(target) {
                        unit.damage = unit.damage.saturating_add(damage_amount);
                        unit.banish_on_death |= banish_on_death;
                    }
                }
                if draw > 0 {
                    self.draw(player, draw);
                }
            }
            OfficialEffect::ModifyMight {
                amount,
                minimum,
                draw,
                alone_bonus,
            } => {
                for target in valid {
                    let bonus = if alone_bonus != 0 {
                        if let Target::Unit {
                            player: owner,
                            unit_id,
                        } = target
                        {
                            let location = self.players[owner]
                                .units
                                .iter()
                                .find(|u| u.id == unit_id)
                                .unwrap()
                                .lane;
                            if self.players[owner]
                                .units
                                .iter()
                                .filter(|u| u.lane == location)
                                .count()
                                == 1
                            {
                                alone_bonus
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    } else {
                        0
                    };
                    self.modify_might(target, amount as i16 + bonus as i16, minimum);
                    if id == 19
                        && let Some(unit) = self.unit_mut(target)
                    {
                        unit.protected = true;
                    }
                }
                if draw > 0 {
                    self.draw(player, draw);
                }
                if id == 52 {
                    self.predict(player);
                }
            }
            OfficialEffect::DualMight => {
                for (slot, &target) in targets.iter().enumerate() {
                    if self.official_target_valid(player, id, lane, target, slot) {
                        self.modify_might(target, if slot == 0 { 2 } else { -2 }, 0);
                    }
                }
            }
            OfficialEffect::Bounce { .. } | OfficialEffect::BounceBoth => {
                for target in valid {
                    self.bounce(target);
                }
            }
            OfficialEffect::MoveFriendlyToBase { .. } => {
                for target in valid {
                    self.move_effect(player, target, None, false);
                }
            }
            OfficialEffect::MoveEnemy => {
                for target in valid {
                    self.move_effect(player, target, lane, false);
                }
            }
            OfficialEffect::MoveFriendlyAndReady => {
                for target in valid {
                    self.move_effect(player, target, lane, true);
                }
            }
            OfficialEffect::SwapMight => {
                if valid.len() == 2 && self.target_lane(valid[0]) == self.target_lane(valid[1]) {
                    let first = self.actual_unit_might(self.unit_ref(valid[0]).unwrap());
                    let second = self.actual_unit_might(self.unit_ref(valid[1]).unwrap());
                    self.modify_might(valid[0], second - first, 0);
                    self.modify_might(valid[1], first - second, 0);
                }
            }
            OfficialEffect::Fight { .. } => {
                if valid.len() == 2 {
                    let first = self.unit_might(self.unit_ref(valid[0]).unwrap());
                    let second = self.unit_might(self.unit_ref(valid[1]).unwrap());
                    self.unit_mut(valid[0]).unwrap().damage = self
                        .unit_ref(valid[0])
                        .unwrap()
                        .damage
                        .saturating_add(second);
                    self.unit_mut(valid[1]).unwrap().damage = self
                        .unit_ref(valid[1])
                        .unwrap()
                        .damage
                        .saturating_add(first);
                }
            }
            OfficialEffect::ReadyAnother => {
                if let Some(&Target::Unit {
                    player: owner,
                    unit_id,
                }) = valid.first()
                    && Some(unit_id) != entering
                {
                    self.ready_unit_by(player, owner, unit_id);
                }
            }
            OfficialEffect::BuffAnother => {
                if let Some(&target) = valid.first()
                    && let Some(unit) = self.unit_ref(target)
                    && Some(unit.id) != entering
                    && !self.buffed_units.contains(&unit.id)
                {
                    let id = unit.id;
                    let _ = id;
                    self.buff_unit(target);
                }
            }
            OfficialEffect::BuffSelf => {
                if let Some(unit_id) = entering
                    && self.unit_ref(Target::Unit { player, unit_id }).is_some()
                    && !self.buffed_units.contains(&unit_id)
                {
                    self.buff_unit(Target::Unit { player, unit_id });
                }
            }
            OfficialEffect::Tokens { count, might } => {
                for _ in 0..count {
                    self.queue_token_play(
                        player,
                        id,
                        might,
                        match rules::token_placement(id) {
                            rules::TokenPlacement::Default => None,
                            rules::TokenPlacement::Base => Some(None),
                            rules::TokenPlacement::Here => Some(lane),
                        },
                        rules::token_is_gear(id),
                    );
                }
            }
            OfficialEffect::Burn(count) => self.burn(player, count),
            OfficialEffect::WeakenEnemies { amount, minimum } => {
                let enemy_targets: Vec<_> = self.players[1 - player]
                    .units
                    .iter()
                    .map(|u| Target::Unit {
                        player: 1 - player,
                        unit_id: u.id,
                    })
                    .collect();
                for target in enemy_targets {
                    self.modify_might(target, -(amount as i16), minimum);
                }
            }
            OfficialEffect::SilenceOpponentsForTurn => self.silenced[1 - player] = true,
            OfficialEffect::ExtraTurn => {
                self.extra_turns[player] = self.extra_turns[player].saturating_add(1);
                self.push_log(format!(
                    "{} will take an extra turn after this turn.",
                    self.players[player].name
                ));
            }
            // Permanent gear operations are handled by the gear lifecycle.
            OfficialEffect::Equipment
            | OfficialEffect::PowerGear
            | OfficialEffect::PreventNextDeathRecall => {}
        }
        // Cleanup waits until the whole chain item, including Repeat, finishes.
    }

    fn resolve_trigger(&mut self, player: usize, source: CardId, ability: TriggeredAbility) {
        match ability {
            TriggeredAbility::DestroyGear { gear_id } => {
                self.gear_destroy(player, gear_id);
            }
            TriggeredAbility::SwapChoice { unit_id } => {
                self.enqueue_decision(Decision::SwapUnit { player, unit_id })
            }
            TriggeredAbility::EmpowerHere { lane, source_id } => {
                self.enqueue_decision(Decision::EmpowerHere {
                    player,
                    lane,
                    source_id,
                })
            }
            TriggeredAbility::RevealHandXp => {
                self.players[player].xp = self.players[player].xp.saturating_add(1);
                self.hidden_view[player] = true;
                self.enqueue_decision(Decision::RevealHand {
                    player,
                    opponent: 1 - player,
                    cards: self.players[1 - player].hand.clone(),
                });
                self.push_log(format!(
                    "{} sees the opponent's hand and may inspect their hidden cards this turn.",
                    self.players[player].name
                ));
            }
            TriggeredAbility::Draw(count) => self.draw(player, count),
            TriggeredAbility::Might { unit_id, amount } => {
                self.modify_might(Target::Unit { player, unit_id }, amount, 0)
            }
            TriggeredAbility::Stun {
                player: owner,
                unit_id,
            } => {
                if let Some(unit) = self.unit_mut(Target::Unit {
                    player: owner,
                    unit_id,
                }) {
                    unit.stunned = true;
                    if !self.movement_locks.contains(&(owner, unit_id)) {
                        self.movement_locks.push((owner, unit_id));
                    }
                }
            }
            TriggeredAbility::Gold => {
                self.queue_token_play(player, source, 0, Some(None), true);
            }
            TriggeredAbility::Tokens { might, count } => {
                for _ in 0..count {
                    self.queue_token_play(player, source, might, Some(None), false);
                }
            }
            TriggeredAbility::Xp(count) => {
                self.players[player].xp = self.players[player].xp.saturating_add(count)
            }
            TriggeredAbility::GrantFlow => self.enqueue_decision(Decision::GrantFlow { player }),
            TriggeredAbility::Loot { count, draw } => self.enqueue_decision(Decision::Discard {
                player,
                count,
                draw_after: draw,
            }),
            TriggeredAbility::BurnForMight { unit_id } => {
                self.enqueue_decision(Decision::BurnForMight { player, unit_id })
            }
            TriggeredAbility::LastRites { gear_id } => {
                self.enqueue_decision(Decision::LastRites { player, gear_id })
            }
        }
        self.remove_defeated();
    }

    pub(crate) fn burn(&mut self, player: usize, count: u8) {
        self.move_top_cards(player, count, false);
    }

    fn create_unit(&mut self, player: usize, card_id: CardId, token: bool, buff: u8) -> u32 {
        let id = self.next_unit_id;
        self.next_unit_id += 1;
        self.players[player].units.push(Unit {
            id,
            card_id,
            lane: None,
            ready: !token && rules::static_abilities(card_id).contains(&StaticAbility::EntersReady),
            damage: 0,
            buff,
            temporary_buff: 0,
            token,
            protected: false,
            stunned: false,
            banish_on_death: false,
        });
        id
    }

    fn begin_ending(&mut self) {
        self.ending_turn = true;
        for (controller, player, object_id) in std::mem::take(&mut self.temporary_empower) {
            self.queue_ability(
                controller,
                46,
                Vec::new(),
                ChainAbility::Disempower { player, object_id },
            );
        }
    }

    fn finish_ending(&mut self) {
        // CR317.2: the ending triggers finish before the expiration cleanup.
        // Heal first, then end this-turn effects together, then clear pools.
        for state in &mut self.players {
            for unit in &mut state.units {
                unit.damage = 0;
                unit.temporary_buff = 0;
                unit.protected = false;
                unit.stunned = false;
                unit.banish_on_death = false;
            }
            state.power_pool = [0; 6];
            state.any_power = 0;
            state.energy_pool = 0;
            state.energy = state
                .runes
                .iter()
                .zip(state.exhausted_runes)
                .map(|(all, used)| all - used)
                .sum();
        }
        self.silenced = [false; 2];
        self.movement_locks.clear();
        self.legend_end_turn();
        self.hidden_view = [false; 2];
        self.temporary_flow = [Vec::new(), Vec::new()];
        self.ending_turn = false;
        if self.extra_turns[self.active] > 0 {
            self.extra_turns[self.active] -= 1;
        } else {
            self.active = 1 - self.active;
        }
        self.start_turn();
    }

    fn start_turn(&mut self) {
        if self.winner.is_some() {
            return;
        }
        self.priority_player = self.active;
        self.turn += 1;
        self.turns_taken[self.active] += 1;
        self.ai_steps_this_turn = 0;
        self.cards_played = [0; 2];
        self.trigger_state.begin_turn();
        self.pyke_triggered.clear();
        self.scored_this_turn = [false; 2];
        self.player_scored_this_turn = [[false; 2]; 2];
        let player = self.active;
        self.push_log(format!(
            "Turn {} · {}",
            self.turn, self.players[player].name
        ));
        self.players[player].exhausted_runes = [0; 6];
        self.players[player].energy =
            self.players[player].total_runes() + self.players[player].energy_pool;
        self.gear_awaken(player);
        self.legend_begin_turn(player);
        self.pending_start = Some(StartStage::Beginning);
        let ready_ids: Vec<_> = self.players[player].units.iter().map(|u| u.id).collect();
        for id in ready_ids {
            self.ready_unit(player, id);
        }
        // Awaken triggers finish before Beginning, Hold, Channel and Draw.
        self.after_resolution();
    }

    fn advance_start_turn(&mut self) {
        let Some(stage) = self.pending_start.take() else {
            return;
        };
        let player = self.active;
        match stage {
            StartStage::Beginning => {
                self.pending_start = Some(StartStage::Holding);
                self.gear_beginning(player);
            }
            StartStage::Holding => {
                self.pending_start = Some(StartStage::Channel);
                for lane in 0..BATTLEFIELDS {
                    if self.battlefield_owner[lane] == Some(player)
                        && self.lane_count(player, lane) > 0
                        && !self.contested(lane)
                    {
                        self.score_battlefield(player, lane, true);
                        self.score_triggers(player, lane, true);
                    }
                }
            }
            StartStage::Channel => {
                self.pending_start = Some(StartStage::Draw);
                let count = if player != self.first_player && self.turns_taken[player] == 1 {
                    3
                } else {
                    2
                };
                for _ in 0..count {
                    if let Some(domain) = self.players[player].rune_deck.pop() {
                        self.players[player].runes[domain_index(domain)] += 1;
                    }
                }
                self.players[player].energy = self.players[player].energy_pool
                    + self.players[player]
                        .runes
                        .iter()
                        .zip(&self.players[player].exhausted_runes)
                        .map(|(all, spent)| all - spent)
                        .sum::<u8>();
            }
            StartStage::Draw => {
                self.pending_start = Some(StartStage::Main);
                self.draw(player, 1);
            }
            StartStage::Main => {
                for state in &mut self.players {
                    state.power_pool = [0; 6];
                    state.any_power = 0;
                    state.energy_pool = 0;
                    state.energy = state
                        .runes
                        .iter()
                        .zip(state.exhausted_runes)
                        .map(|(all, used)| all - used)
                        .sum();
                }
                for legend in &mut self.legend {
                    legend.spell_power = 0;
                }
            }
        }
        if self.winner.is_none() {
            self.after_resolution();
        }
    }

    pub(crate) fn draw(&mut self, player: usize, count: u8) {
        self.move_top_cards(player, count, true);
    }

    fn move_top_cards(&mut self, player: usize, count: u8, drawing: bool) {
        for _ in 0..count {
            let mut consecutive_burnouts = 0;
            while self.players[player].deck.is_empty() && self.winner.is_none() {
                self.push_log(format!(
                    "{} burns out: recycle the trash; the opponent scores 1.",
                    self.players[player].name
                ));
                self.temporary_flow[player].clear();
                for item in &mut self.chain {
                    if item.player == player
                        && let Some(
                            ChainAbility::ReviveChosen { trash_index, .. }
                            | ChainAbility::PlayChosen { trash_index, .. }
                            | ChainAbility::GrantFlowChosen { trash_index, .. },
                        ) = &mut item.ability
                    {
                        *trash_index = usize::MAX;
                    }
                }
                self.players[player].deck = std::mem::take(&mut self.players[player].discard);
                self.rng.shuffle(&mut self.players[player].deck);
                self.players[1 - player].score += 1;
                consecutive_burnouts += 1;
                // Only repeated empty-deck burnouts bypass cleanup victory.
                // The first must complete the original instruction (CR432.2.d).
                if consecutive_burnouts > 1 {
                    self.check_winner(1 - player);
                }
                if self.winner.is_some() {
                    return;
                }
            }
            if self.winner.is_some() {
                return;
            }
            if let Some(id) = self.players[player].deck.pop() {
                if drawing {
                    self.players[player].hand.push(id);
                    self.trigger_state.draws[player] += 1;
                    self.emit_event(EventSnapshot::simple(Occurrence::Draw, player));
                    self.trigger_batch += 1;
                } else {
                    self.players[player].discard.push(id);
                }
            }
        }
    }

    /// A legal deterministic allocation; humans may submit another legal order.
    pub fn automatic_damage_assignment(
        &self,
        player: usize,
        lane: usize,
        total: u16,
    ) -> Vec<(u32, u16)> {
        if player > 1 || lane >= BATTLEFIELDS {
            return Vec::new();
        }
        let mut order: Vec<_> = self.players[1 - player]
            .units
            .iter()
            .filter(|u| u.lane == Some(lane))
            .map(|u| {
                (
                    u.id,
                    self.unit_might(u).saturating_sub(u.damage).max(1) as u16,
                    self.damage_priority(u),
                )
            })
            .collect();
        order.sort_unstable_by_key(|&(id, lethal, priority)| (priority, lethal, id));
        let mut remaining = total;
        let mut result = Vec::new();
        let count = order.len();
        for (index, (id, lethal, _)) in order.into_iter().enumerate() {
            if remaining == 0 {
                break;
            }
            let amount = if index + 1 == count {
                remaining
            } else {
                remaining.min(lethal)
            };
            result.push((id, amount));
            remaining -= amount;
        }
        result
    }

    fn resolve_combat(&mut self, lane: usize, attacker: usize) {
        self.showdown = Some(Showdown {
            lane,
            attacker,
            focus: attacker,
            passes: 2,
            combat: true,
        });
        let powers = [self.lane_might(0, lane), self.lane_might(1, lane)];
        self.push_log(format!(
            "Battlefield {} combat: {} might vs {} might. Assign damage before either side is hit.",
            lane + 1,
            powers[attacker],
            powers[1 - attacker]
        ));
        self.pending_combat = Some(PendingCombat {
            lane,
            attacker,
            powers,
            assignments: [None, None],
        });
        if !self.contested(lane) {
            self.pending_combat.as_mut().unwrap().assignments =
                [Some(Vec::new()), Some(Vec::new())];
        }
        self.advance_combat_assignment();
    }

    fn advance_combat_assignment(&mut self) {
        let combat = self.pending_combat.as_mut().expect("active combat");
        for player in [combat.attacker, 1 - combat.attacker] {
            if combat.assignments[player].is_none() {
                if combat.powers[player] == 0 {
                    combat.assignments[player] = Some(Vec::new());
                    continue;
                }
                self.decision = Some(Decision::CombatDamage {
                    player,
                    lane: combat.lane,
                    total: combat.powers[player],
                });
                return;
            }
        }
        self.finish_combat();
    }

    fn assign_combat_damage(&mut self, assignments: Vec<(u32, u16)>) -> Result<(), String> {
        let Some(Decision::CombatDamage {
            player,
            lane,
            total,
        }) = self.decision
        else {
            return Err("Combat damage is not awaiting assignment.".into());
        };
        let mut remaining: Vec<_> = self.players[1 - player]
            .units
            .iter()
            .filter(|u| u.lane == Some(lane))
            .collect();
        let sum: u32 = assignments.iter().map(|(_, amount)| *amount as u32).sum();
        if sum != total as u32 {
            return Err(format!("Assign all {total} damage exactly."));
        }
        for (index, &(id, amount)) in assignments.iter().enumerate() {
            if amount == 0 {
                return Err("Each selected unit must receive positive damage.".into());
            }
            let Some(position) = remaining.iter().position(|u| u.id == id) else {
                return Err(
                    "Assign damage only once to each opposing unit at this battlefield.".into(),
                );
            };
            let unit = remaining[position];
            let minimum_priority = remaining
                .iter()
                .map(|u| self.damage_priority(u))
                .min()
                .unwrap();
            if self.damage_priority(unit) != minimum_priority {
                return Err("Assign to Tank units first and Backline units last.".into());
            }
            let lethal = self.unit_might(unit).saturating_sub(unit.damage).max(1) as u16;
            if amount < lethal && index + 1 < assignments.len() {
                return Err(
                    "Assign lethal damage to a unit before assigning damage to the next.".into(),
                );
            }
            if amount > lethal && remaining.len() > 1 {
                return Err("Do not over-assign lethal damage while another unit remains.".into());
            }
            remaining.remove(position);
        }
        self.pending_combat
            .as_mut()
            .ok_or("No combat is active.")?
            .assignments[player] = Some(assignments);
        self.decision = None;
        self.advance_combat_assignment();
        Ok(())
    }

    fn finish_combat(&mut self) {
        let combat = self.pending_combat.take().expect("completed assignments");
        for player in 0..2 {
            for &(id, amount) in combat.assignments[player]
                .as_ref()
                .expect("both sides assigned")
            {
                if let Some(unit) = self.players[1 - player]
                    .units
                    .iter_mut()
                    .find(|u| u.id == id)
                {
                    unit.damage = unit.damage.saturating_add(amount.min(u8::MAX as u16) as u8);
                }
            }
        }
        self.pending_combat_cleanup = Some((combat.lane, combat.attacker));
        self.after_resolution();
    }

    fn finish_combat_cleanup(&mut self) {
        let (lane, attacker) = self
            .pending_combat_cleanup
            .take()
            .expect("combat cleanup pending");
        // Combat cleanup heals all surviving units, then recalls attackers.
        for state in &mut self.players {
            for unit in &mut state.units {
                unit.damage = 0;
            }
        }
        if self.lane_count(attacker, lane) > 0 && self.lane_count(1 - attacker, lane) > 0 {
            for unit in &mut self.players[attacker].units {
                if unit.lane == Some(lane) {
                    unit.lane = None;
                }
            }
            self.push_log("The defense holds. Surviving attackers are recalled to base.".into());
        }
        self.showdown = None;
        self.priority_player = self.active;
        self.refresh_control(lane);
        self.push_log("The showdown closes; the turn player's main phase resumes.".into());
        self.after_resolution();
    }

    fn death_replacements(&self, player: usize) -> Vec<DeathReplacement> {
        let mut choices = Vec::new();
        for unit in self.players[player]
            .units
            .iter()
            .filter(|u| u.damage > 0 && u.damage >= self.unit_might(u))
        {
            for gear in &self.players[player].gear {
                if !gear.token
                    && ((gear.card_id == 10 && gear.attached_to == Some(unit.id))
                        || (gear.card_id == 21 && gear.attached_to.is_none()))
                {
                    choices.push(DeathReplacement {
                        unit_id: unit.id,
                        gear_id: Some(gear.id),
                    });
                }
            }
            if unit.banish_on_death {
                choices.push(DeathReplacement {
                    unit_id: unit.id,
                    gear_id: None,
                });
            }
        }
        choices
    }

    fn apply_death_replacement(&mut self, player: usize, choice: DeathReplacement) {
        if let Some(gear_id) = choice.gear_id {
            self.gear_replace_death(player, choice.unit_id, gear_id);
        } else if let Some(index) = self.players[player]
            .units
            .iter()
            .position(|u| u.id == choice.unit_id)
        {
            self.gear_unit_left_board(player, choice.unit_id);
            self.empowered_units.retain(|&id| id != choice.unit_id);
            let unit = self.players[player].units.remove(index);
            self.push_log(format!(
                "{} is banished instead of dying.",
                self.unit_name(&unit)
            ));
            if !unit.token {
                self.players[player].banish.push(unit.card_id);
            }
        }
    }

    fn choose_death_replacement(
        &mut self,
        unit_id: u32,
        gear_id: Option<u32>,
    ) -> Result<(), String> {
        let Some(Decision::DeathReplacement {
            player,
            ref options,
        }) = self.decision
        else {
            return Err("No death replacement is pending.".into());
        };
        let choice = DeathReplacement { unit_id, gear_id };
        if !options.contains(&choice) {
            return Err("Choose one of the offered death replacements.".into());
        }
        self.decision = None;
        self.apply_death_replacement(player, choice);
        self.after_resolution();
        Ok(())
    }

    pub(crate) fn remove_defeated(&mut self) {
        if matches!(self.decision, Some(Decision::DeathReplacement { .. })) {
            return;
        }
        // CR373: perform replacements before simultaneous unmodified deaths,
        // in turn order; the controller chooses both effect and affected event.
        for player in [self.active, 1 - self.active] {
            loop {
                let options = self.death_replacements(player);
                if options.len() > 1 {
                    if let Some(old) = self.decision.take() {
                        self.pending_decisions.push_front(old);
                    }
                    self.decision = Some(Decision::DeathReplacement { player, options });
                    return;
                }
                if let Some(&choice) = options.first() {
                    self.apply_death_replacement(player, choice);
                } else {
                    break;
                }
            }
        }
        let dying: Vec<_> = self
            .players
            .iter()
            .enumerate()
            .flat_map(|(player, state)| {
                state
                    .units
                    .iter()
                    .filter(|u| {
                        u.damage > 0 && u.damage >= self.unit_might(u) && !u.banish_on_death
                    })
                    .cloned()
                    .map(move |u| (player, u))
            })
            .collect();
        let death_events: Vec<_> = dying
            .iter()
            .map(|(player, unit)| {
                let mut event = EventSnapshot::unit(Occurrence::Death, *player, unit.clone());
                event.buffed = self.buffed_units.contains(&unit.id);
                event
            })
            .collect();
        self.capture_death_triggers(&dying);
        let mut deaths = Vec::new();
        let pykes: [Vec<u32>; 2] = [0, 1].map(|p| {
            self.players[p]
                .units
                .iter()
                .filter(|u| !u.token && u.card_id == 13 && u.lane.is_some())
                .map(|u| u.id)
                .collect()
        });
        let mut real_deaths = [false; 2];
        for (player, has_real_death) in real_deaths.iter_mut().enumerate() {
            let mut defeated: Vec<(usize, bool)> = self.players[player]
                .units
                .iter()
                .enumerate()
                .filter(|(_, u)| u.damage > 0 && u.damage >= self.unit_might(u))
                .map(|(i, u)| {
                    (
                        i,
                        self.players[player]
                            .units
                            .iter()
                            .filter(|other| other.lane == u.lane)
                            .count()
                            == 1,
                    )
                })
                .collect();
            defeated.reverse();
            for (index, alone) in defeated {
                let unit_id = self.players[player].units[index].id;
                if !self.players[player].units[index].banish_on_death {
                    *has_real_death = true;
                }
                self.empowered_units.retain(|&id| id != unit_id);
                self.gear_unit_left_board(player, unit_id);
                let unit = self.players[player].units.remove(index);
                self.push_log(format!(
                    "{} is {}.",
                    self.unit_name(&unit),
                    if unit.banish_on_death {
                        "banished instead of dying"
                    } else {
                        "defeated"
                    }
                ));
                if !unit.token {
                    if unit.banish_on_death {
                        self.players[player].banish.push(unit.card_id);
                    } else {
                        self.players[player].discard.push(unit.card_id);
                        deaths.push((player, unit.card_id, alone));
                    }
                }
            }
        }
        for event in death_events {
            self.emit_event(event);
        }
        for player in 0..2 {
            if real_deaths[1 - player] {
                for &id in &pykes[player] {
                    // CR383.2.c.2: a source that dies in this same action cannot
                    // see another unit die. Its own Deathknell is the exception.
                    if self.players[player].units.iter().any(|unit| unit.id == id)
                        && !self.pyke_triggered.contains(&id)
                    {
                        self.pyke_triggered.push(id);
                        self.queue_ability(
                            player,
                            13,
                            Vec::new(),
                            ChainAbility::Triggered(TriggeredAbility::Gold),
                        );
                    }
                }
            }
        }
        for (player, card, alone) in deaths {
            let effect = match card {
                25 if alone => Some(TriggeredAbility::Draw(1)),
                68 => Some(TriggeredAbility::Draw(1)),
                34 => Some(TriggeredAbility::Tokens { count: 1, might: 1 }),
                54 => Some(TriggeredAbility::Tokens { count: 2, might: 3 }),
                149 => Some(TriggeredAbility::Gold),
                150 => Some(TriggeredAbility::Tokens { count: 1, might: 1 }),
                15 => Some(TriggeredAbility::RevealHandXp),
                _ => None,
            };
            if let Some(effect) = effect {
                self.queue_ability(player, card, Vec::new(), ChainAbility::Triggered(effect));
            }
        }
    }

    pub(crate) fn refresh_control(&mut self, lane: usize) {
        if self.showdown.as_ref().is_some_and(|w| w.lane == lane)
            || !self.chain.is_empty()
            || self
                .pending_entries
                .iter()
                .any(|(pending, _)| *pending == lane)
        {
            return;
        }
        match (self.lane_count(0, lane) > 0, self.lane_count(1, lane) > 0) {
            (true, false) => self.claim_lane(0, lane),
            (false, true) => self.claim_lane(1, lane),
            (false, false) => {
                self.battlefield_owner[lane] = None;
                self.cleanup_lost_hidden();
            }
            (true, true) => {} // Defender keeps control until the battle resolves.
        }
    }

    fn claim_lane(&mut self, player: usize, lane: usize) {
        if self.battlefield_owner[lane] == Some(player) {
            return;
        }
        self.battlefield_owner[lane] = Some(player);
        let scoring = !self.player_scored_this_turn[player][lane];
        self.score_battlefield(player, lane, false);
        if scoring && self.winner.is_none() {
            self.score_triggers(player, lane, false);
            self.battlefield_on_conquer(player, lane);
        }
        self.cleanup_lost_hidden();
    }

    fn score_battlefield(&mut self, player: usize, lane: usize, holding: bool) {
        if self.player_scored_this_turn[player][lane] || self.winner.is_some() {
            return;
        }
        let all_other_scored = (0..BATTLEFIELDS)
            .filter(|&l| l != lane)
            .all(|l| self.player_scored_this_turn[player][l]);
        self.player_scored_this_turn[player][lane] = true;
        if player == self.active {
            self.scored_this_turn[lane] = true;
        }
        if !holding && self.players[player].score >= VICTORY_SCORE - 1 && !all_other_scored {
            self.push_log(format!("Final-point rule: {} draws instead. Hold a battlefield or score the other battlefield this turn to win.", self.players[player].name));
            self.draw(player, 1);
            return;
        }
        self.players[player].score += 1;
        self.push_log(format!(
            "{} {} battlefield {}: {} / {} points.",
            self.players[player].name,
            if holding { "holds" } else { "conquers" },
            lane + 1,
            self.players[player].score,
            VICTORY_SCORE
        ));
        // CR315.2.b.2: Hold every controlled battlefield as one outstanding
        // task. Its cleanup checks victory after all points and Hold triggers
        // have been generated, before those triggers finalize (CR323.1/334).
        if !holding {
            self.check_winner(player);
        }
    }

    fn score_triggers(&mut self, player: usize, lane: usize, holding: bool) {
        let cards: Vec<_> = self.players[player]
            .units
            .iter()
            .filter(|u| u.lane == Some(lane) && !u.token)
            .map(|u| u.card_id)
            .collect();
        let ids: Vec<_> = self.players[player]
            .units
            .iter()
            .filter(|u| u.lane == Some(lane))
            .map(|u| u.id)
            .collect();
        for id in ids {
            self.emit_unit_event(
                if holding {
                    Occurrence::Hold
                } else {
                    Occurrence::Conquer
                },
                player,
                id,
            );
            self.gear_conquer_hold(player, id);
        }
        if !holding && let Some(trigger) = self.legend_on_conquer(player) {
            self.queue_trigger_choice(player, 0, Decision::Legend { player, trigger });
        }
        for card in cards {
            let effect = match card {
                49 if !holding => Some(TriggeredAbility::Draw(1)),
                33 if !holding => Some(TriggeredAbility::GrantFlow),
                22 => Some(TriggeredAbility::Xp(2)),
                _ => None,
            };
            if let Some(effect) = effect {
                self.queue_ability(player, card, Vec::new(), ChainAbility::Triggered(effect));
            }
        }
    }

    fn check_winner(&mut self, player: usize) {
        if self.players[player].score >= VICTORY_SCORE
            && self.players[player].score > self.players[1 - player].score
            && self.winner.is_none()
        {
            self.winner = Some(player);
            self.phase = Phase::Finished;
            self.push_log(format!("{} wins!", self.players[player].name));
        }
    }

    pub(crate) fn push_log(&mut self, message: String) {
        self.log.push(message);
        if self.log.len() > 2048 {
            self.log.drain(..self.log.len() - 2048);
        }
    }

    fn choose_ai_action(&self) -> Action {
        if self.difficulty == Difficulty::Strategic {
            self.choose_strategic_action()
        } else {
            self.choose_heuristic_action()
        }
    }

    fn choose_heuristic_action(&self) -> Action {
        let player = self.priority();
        let enemy = 1 - player;
        if let Some(decision) = &self.decision {
            let actions = self.legal_actions();
            let preferred = match decision {
                Decision::Predict { player, card_id } => Some(Action::Choose {
                    accept: self.catalog[*card_id].cost > self.players[*player].total_runes() + 2,
                }),
                Decision::CounterTax { player, cost, .. } => Some(Action::Choose {
                    accept: self.players[*player].energy >= *cost,
                }),
                Decision::OptionalPlay { .. }
                | Decision::Legend { .. }
                | Decision::BurnForMight { .. } => Some(Action::Choose { accept: true }),
                _ => None,
            };
            if let Some(action) = preferred
                && actions.contains(&action)
            {
                return action;
            }
            return actions
                .first()
                .cloned()
                .unwrap_or(Action::Choose { accept: false });
        }
        if self.ai_steps_this_turn >= 160 && !self.in_window() {
            return Action::EndTurn;
        }

        // Only the active player's hand is read. The opponent contributes its
        // public units and score, never its hand or deck order.
        let mut plays: Vec<(i32, Action)> = Vec::new();
        for action in self.hidden_actions(player) {
            match &action {
                Action::PlayHidden { .. } => {
                    plays.push((if self.showdown.is_some() { 95 } else { 25 }, action))
                }
                Action::Hide { hand_index, .. } => {
                    let id = self.players[player].hand[*hand_index];
                    if self.catalog[id].cost >= 3
                        || self.players[player].energy < self.catalog[id].cost
                    {
                        plays.push((85, action));
                    }
                }
                _ => {}
            }
        }
        for unit in &self.players[player].units {
            if matches!(unit.card_id, 24 | 45) && !unit.token {
                let action = Action::EmpowerUnit {
                    unit_id: unit.id,
                    pay_power: false,
                };
                if self.clone().apply(action.clone()).is_ok() {
                    plays.push((40, action));
                }
            }
        }
        for action in self.legend_actions(player) {
            plays.push((35, action));
        }
        for action in self.gear_actions(player) {
            plays.push((25, action));
        }
        for trash_index in 0..self.players[player].discard.len() {
            let action = Action::PlayFromTrash {
                trash_index,
                lane: None,
                target: None,
            };
            if self.clone().apply(action.clone()).is_ok() {
                plays.push((
                    self.ai_card_score(self.players[player].discard[trash_index]),
                    action,
                ));
            }
        }
        for (hand_index, &id) in self.players[player].hand.iter().enumerate() {
            if rules::accelerate_cost(id).is_some() || rules::repeat_cost(id).is_some() || id == 29
            {
                let action = Action::PlayCardOptions {
                    hand_index,
                    lane: None,
                    targets: Vec::new(),
                    repeat_targets: Vec::new(),
                    repeat: rules::repeat_cost(id).is_some(),
                    accelerate: rules::accelerate_cost(id).is_some(),
                    bonus: id == 29,
                };
                if self.clone().apply(action.clone()).is_ok() {
                    let score = self.ai_card_score(id);
                    if score > 0 {
                        plays.push((score + 12, action));
                    }
                }
            }
            if self.validate_play(id, None, None).is_ok() {
                let score = self.ai_card_score(id);
                if score > 0 {
                    plays.push((
                        score,
                        Action::PlayCard {
                            hand_index,
                            lane: None,
                            target: None,
                        },
                    ));
                }
            }
        }
        if let Some(id) = self.players[player].champion
            && self.validate_play(id, None, None).is_ok()
        {
            plays.push((
                self.ai_card_score(id) + 1,
                Action::PlayChampion {
                    lane: None,
                    target: None,
                },
            ));
        }
        if let Some((_, action)) = plays.into_iter().max_by_key(|(score, _)| *score) {
            return action;
        }

        if self.in_window() {
            return Action::Pass;
        }
        let ready_base: Vec<&Unit> = self.players[player]
            .units
            .iter()
            .filter(|u| {
                u.ready && !self.movement_locks.contains(&(player, u.id)) && u.lane.is_none()
            })
            .collect();
        let base_power: u16 = ready_base.iter().map(|u| self.unit_might(u) as u16).sum();
        if !ready_base.is_empty() {
            let mut targets: Vec<(i32, usize)> = (0..BATTLEFIELDS)
                .filter_map(|lane| {
                    let friendly = self.lane_might(player, lane);
                    let opposing = self.lane_might(enemy, lane);
                    let ours = self.lane_count(player, lane);
                    let theirs = self.lane_count(enemy, lane);
                    let can_win = theirs == 0 || friendly + base_power > opposing;
                    if self.difficulty != Difficulty::Easy && !can_win {
                        return None;
                    }
                    let score = if theirs == 0 && ours == 0 {
                        120
                    } else if theirs > 0 && can_win {
                        100 + opposing as i32
                    } else if theirs > 0 {
                        10 - opposing as i32
                    } else {
                        20 - friendly as i32
                    };
                    Some((score, lane))
                })
                .collect();
            targets.sort_by_key(|&(score, lane)| (score, std::cmp::Reverse(lane)));
            if let Some(&(_, lane)) = targets.last() {
                // A small unit claims an empty field; strong units lead battles.
                let unit =
                    if self.lane_count(enemy, lane) == 0 && self.lane_count(player, lane) == 0 {
                        ready_base
                            .iter()
                            .min_by_key(|u| self.unit_might(u))
                            .unwrap()
                    } else {
                        ready_base
                            .iter()
                            .max_by_key(|u| self.unit_might(u))
                            .unwrap()
                    };
                if self.lane_count(enemy, lane) > 0 {
                    return Action::MoveGroup {
                        unit_ids: ready_base.iter().map(|u| u.id).collect(),
                        lane: Some(lane),
                    };
                }
                return Action::Move {
                    unit_id: unit.id,
                    lane: Some(lane),
                };
            }
        }
        for lane in 0..BATTLEFIELDS {
            if self.contested(lane) {
                return Action::Attack { lane };
            }
        }
        Action::EndTurn
    }

    fn ai_card_score(&self, id: CardId) -> i32 {
        let card = self.catalog[id];
        let tactical = self.difficulty != Difficulty::Easy;
        if self.is_counter(id) {
            return if self.chain.iter().any(|item| {
                item.player != self.priority()
                    && self.counter_target_valid(self.priority(), id, item.id)
            }) {
                105
            } else {
                0
            };
        }
        if card.kind == CardKind::Unit {
            return 50 + card.might as i32 * 4
                - if tactical {
                    card.power_cost as i32 * 3
                } else {
                    card.cost as i32
                };
        }
        if card.kind == CardKind::Gear {
            return 35;
        }
        let actor = self.priority();
        let in_combat = self
            .showdown
            .as_ref()
            .is_some_and(|w| self.contested(w.lane));
        let targets = self
            .prepare_targets(actor, id, None, &[])
            .unwrap_or_default();
        match rules::effect(id) {
            effect @ (OfficialEffect::Damage { .. }
            | OfficialEffect::ContextualDamage(_)
            | OfficialEffect::DamageAndGold { .. }) => {
                let lethal = targets
                    .iter()
                    .filter(|&&target| {
                        let amount = match effect {
                            OfficialEffect::Damage { amount, .. }
                            | OfficialEffect::DamageAndGold { amount } => amount,
                            OfficialEffect::ContextualDamage(condition) => {
                                self.contextual_damage(actor, target, condition)
                            }
                            _ => unreachable!(),
                        };
                        self.unit_ref(target).is_some_and(|unit| {
                            unit.damage
                                .saturating_add(self.battlefield_damage(target, amount))
                                >= self.unit_might(unit)
                        })
                    })
                    .count();
                if lethal > 0 {
                    100 + lethal as i32 * 5
                } else if in_combat || !tactical {
                    20
                } else {
                    0
                }
            }
            OfficialEffect::ModifyMight { amount, .. } if amount < 0 => {
                if in_combat {
                    85
                } else {
                    0
                }
            }
            OfficialEffect::ContextualMight(rules::SpellMight::OwnExhausted { .. })
                if self.players[actor].units.iter().all(|unit| unit.ready) =>
            {
                0
            }
            OfficialEffect::ModifyMight { .. }
            | OfficialEffect::ContextualMight(_)
            | OfficialEffect::DualMight
            | OfficialEffect::SwapMight => {
                if in_combat {
                    80
                } else {
                    0
                }
            }
            OfficialEffect::Bounce { .. } | OfficialEffect::BounceBoth => {
                if in_combat {
                    90
                } else if self.players[1 - actor]
                    .units
                    .iter()
                    .any(|u| u.lane.is_some())
                {
                    55
                } else {
                    0
                }
            }
            OfficialEffect::MoveEnemy => {
                if in_combat {
                    95
                } else {
                    0
                }
            }
            OfficialEffect::MoveFriendlyAndReady => {
                if in_combat {
                    30
                } else {
                    0
                }
            }
            OfficialEffect::MoveFriendlyToBase { .. } => {
                if in_combat {
                    10
                } else {
                    0
                }
            }
            effect @ (OfficialEffect::Fight { .. } | OfficialEffect::EmpoweredFight { .. }) => {
                let bonus = match effect {
                    OfficialEffect::EmpoweredFight { amount } => amount.max(0) as u8,
                    _ => 0,
                };
                if targets.len() == 2
                    && self
                        .unit_ref(targets[0])
                        .zip(self.unit_ref(targets[1]))
                        .is_some_and(|(a, b)| {
                            self.unit_might(a).saturating_add(bonus) > self.unit_might(b)
                        })
                {
                    70
                } else {
                    0
                }
            }
            OfficialEffect::Tokens { count, might } => 45 + count as i32 * might as i32 * 4,
            OfficialEffect::ReadyAnother => {
                if targets.iter().any(|&target| {
                    matches!(target, Target::Unit { player, .. } if player == actor)
                        && self.unit_ref(target).is_some_and(|unit| !unit.ready)
                }) {
                    35
                } else {
                    0
                }
            }
            OfficialEffect::DrawForMighty
                if self.players[actor]
                    .units
                    .iter()
                    .all(|unit| self.unit_might(unit) < 5) =>
            {
                0
            }
            OfficialEffect::Draw(_)
            | OfficialEffect::ChooseTopThreeBurnRest
            | OfficialEffect::DrawForBattlefields { .. }
            | OfficialEffect::DrawForMighty => {
                if self.players[actor].hand.len() < 5 {
                    15
                } else {
                    0
                }
            }
            OfficialEffect::DisruptHand => {
                if !self.players[1 - actor].hand.is_empty() {
                    30
                } else {
                    0
                }
            }
            OfficialEffect::DestroyGearAndDraw | OfficialEffect::TemporaryGear => {
                if self.players[1 - actor].gear.is_empty() {
                    0
                } else {
                    40
                }
            }
            OfficialEffect::ExtraTurn => 65,
            _ => 0,
        }
    }
}

fn replace_opening_cards(player: &mut PlayerState, indices: &[usize], rng: &mut Rng) {
    let mut bottom = Vec::with_capacity(indices.len());
    for &index in indices {
        if let Some(replacement) = player.deck.pop() {
            bottom.push(std::mem::replace(&mut player.hand[index], replacement));
        }
    }
    rng.shuffle(&mut bottom);
    player.deck.splice(0..0, bottom);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        let mut game = Game::new(0, 1, 1234, Difficulty::Tactical);
        game.start_match().unwrap();
        game.legend = [LegendState::new(49), LegendState::new(49)];
        game
    }

    fn finish_windows(game: &mut Game) {
        for _ in 0..100 {
            if !game.in_window() || game.winner.is_some() {
                return;
            }
            if game.decision.is_some() {
                let action = if let Some(Decision::CombatDamage {
                    player,
                    lane,
                    total,
                }) = game.decision
                {
                    Action::AssignDamage {
                        assignments: game.automatic_damage_assignment(player, lane, total),
                    }
                } else {
                    let choices = game.legal_actions();
                    choices
                        .iter()
                        .find(|a| **a == Action::Choose { accept: false })
                        .cloned()
                        .unwrap_or_else(|| choices[0].clone())
                };
                game.apply(action).unwrap();
            } else {
                game.apply(Action::Pass).unwrap();
            }
        }
        panic!("window failed to close");
    }

    fn unit(game: &mut Game, player: usize, might: u8, lane: Option<usize>, ready: bool) -> u32 {
        let id = game.create_unit(player, 0, true, might);
        let unit = game.players[player].units.last_mut().unwrap();
        unit.lane = lane;
        unit.ready = ready;
        id
    }

    fn give_runes(game: &mut Game, player: usize, domain: Domain, number: u8) {
        game.players[player].runes = [0; 6];
        game.players[player].runes[domain_index(domain)] = number;
        game.players[player].exhausted_runes = [0; 6];
        game.players[player].energy = number;
        game.players[player].rune_deck = vec![domain; (12 - number) as usize];
    }

    #[test]
    fn seed_reproduces_decks_and_four_card_opening() {
        let first = Game::new(0, 1, 42, Difficulty::Easy);
        let second = Game::new(0, 1, 42, Difficulty::Easy);
        assert_eq!(first.players, second.players);
        for player in &first.players {
            assert_eq!(player.hand.len(), 4);
            assert_eq!(player.deck.len(), 35);
            assert!(player.champion.is_some());
            assert_eq!(player.rune_deck.len(), 12);
        }
    }

    #[test]
    fn mulligan_is_once_up_to_two_and_preserves_card_count() {
        let mut game = Game::new(0, 1, 29, Difficulty::Easy);
        let before = game.players[0].clone();
        assert!(game.mulligan(&[0, 0]).is_err());
        assert!(game.mulligan(&[0, 1, 2]).is_err());
        assert_eq!(game.players[0], before);
        let expected = [
            before.deck[before.deck.len() - 1],
            before.deck[before.deck.len() - 2],
        ];
        game.mulligan(&[0, 2]).unwrap();
        assert_eq!([game.players[0].hand[0], game.players[0].hand[2]], expected);
        assert_eq!(game.players[0].deck.len(), 35);
        assert!(game.mulligan(&[]).is_err());
    }

    #[test]
    fn turn_channel_and_second_player_bonus_are_correct() {
        let mut game = game();
        assert_eq!(game.players[0].total_runes(), 2);
        assert_eq!(game.players[0].hand.len(), 5);
        game.apply(Action::EndTurn).unwrap();
        assert_eq!(game.players[1].total_runes(), 3);
        assert_eq!(game.players[1].hand.len(), 5);
        game.apply(Action::EndTurn).unwrap();
        assert_eq!(game.players[0].total_runes(), 4);
    }

    #[test]
    fn invalid_actions_do_not_spend_cards_or_resources() {
        let mut game = game();
        let before = game.players.clone();
        assert!(
            game.apply(Action::PlayCard {
                hand_index: 999,
                lane: None,
                target: None
            })
            .is_err()
        );
        assert!(
            game.apply(Action::Move {
                unit_id: 999,
                lane: Some(0)
            })
            .is_err()
        );
        assert!(game.apply(Action::Attack { lane: 100 }).is_err());
        assert!(
            game.apply(Action::PlayChampion {
                lane: Some(0),
                target: None
            })
            .is_err()
        );
        assert_eq!(game.players, before);
    }

    #[test]
    fn energy_then_recycled_power_supports_combined_cost_above_twelve() {
        let mut game = game();
        let id = 0;
        let mut card = game.catalog[0];
        card.id = id;
        card.cost = 10;
        card.power_cost = 4;
        card.kind = CardKind::Unit;
        game.catalog[id] = card;
        game.players[0].hand = vec![id];
        give_runes(&mut game, 0, card.domain, 12);
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: None,
        })
        .unwrap();
        assert_eq!(game.players[0].total_runes(), 8);
        assert_eq!(game.players[0].energy, 2);
        assert_eq!(game.players[0].rune_deck.len(), 4);
        assert!(!game.players[0].units[0].ready);
        assert_eq!(game.players[0].units[0].lane, None);
    }

    #[test]
    fn power_requires_matching_domain_and_rejection_is_atomic() {
        let mut game = game();
        let mut card = game.catalog[0];
        card.power_cost = 1;
        card.cost = 1;
        card.kind = CardKind::Unit;
        game.catalog[0] = card;
        game.players[0].hand = vec![0];
        let other = ALL_DOMAINS
            .iter()
            .copied()
            .find(|&domain| domain != card.domain)
            .unwrap();
        give_runes(&mut game, 0, other, 12);
        let before = game.players.clone();
        assert!(
            game.apply(Action::PlayCard {
                hand_index: 0,
                lane: None,
                target: None
            })
            .is_err()
        );
        assert_eq!(game.players, before);
    }

    #[test]
    fn movement_exhausts_and_cannot_jump_between_battlefields() {
        let mut game = game();
        let id = unit(&mut game, 0, 3, None, true);
        game.apply(Action::Move {
            unit_id: id,
            lane: Some(0),
        })
        .unwrap();
        finish_windows(&mut game);
        assert_eq!(game.players[0].score, 1);
        assert!(!game.players[0].units[0].ready);
        game.players[0].units[0].ready = true;
        let before = game.players.clone();
        assert!(
            game.apply(Action::Move {
                unit_id: id,
                lane: Some(1)
            })
            .is_err()
        );
        assert_eq!(game.players, before);
    }

    #[test]
    fn equal_might_combat_is_simultaneous_and_destroys_both_sides() {
        let mut game = game();
        unit(&mut game, 0, 4, Some(0), false);
        unit(&mut game, 1, 4, Some(0), false);
        game.battlefield_owner[0] = Some(1);
        game.apply(Action::Attack { lane: 0 }).unwrap();
        finish_windows(&mut game);
        assert!(game.players[0].units.is_empty());
        assert!(game.players[1].units.is_empty());
        assert_eq!(game.battlefield_owner[0], None);
        assert_eq!(game.players[0].score, 0);
    }

    #[test]
    fn combat_cleanup_heals_survivors_at_every_location() {
        let mut game = game();
        unit(&mut game, 0, 5, Some(0), false);
        unit(&mut game, 1, 2, Some(0), false);
        unit(&mut game, 0, 4, None, false);
        unit(&mut game, 1, 4, Some(1), false);
        game.players[0].units[1].damage = 1;
        game.players[1].units[1].damage = 2;
        game.apply(Action::Attack { lane: 0 }).unwrap();
        finish_windows(&mut game);
        assert!(
            game.players
                .iter()
                .flat_map(|p| &p.units)
                .all(|u| u.damage == 0)
        );
    }

    #[test]
    fn grouped_movement_validates_all_units_before_exhausting_any() {
        let mut game = game();
        let first = unit(&mut game, 0, 3, None, true);
        let second = unit(&mut game, 0, 3, None, false);
        let before = game.players.clone();
        assert!(
            game.apply(Action::MoveGroup {
                unit_ids: vec![first, second],
                lane: Some(0)
            })
            .is_err()
        );
        assert_eq!(game.players, before);
        assert!(
            game.apply(Action::MoveGroup {
                unit_ids: vec![first, first],
                lane: Some(0)
            })
            .is_err()
        );
        assert_eq!(game.players, before);
        game.players[0].units[1].ready = true;
        game.apply(Action::MoveGroup {
            unit_ids: vec![first, second],
            lane: Some(0),
        })
        .unwrap();
        finish_windows(&mut game);
        assert_eq!(game.lane_count(0, 0), 2);
        assert_eq!(game.players[0].score, 1);
        assert!(game.players[0].units.iter().all(|u| !u.ready));
    }

    #[test]
    fn units_may_deploy_exhausted_to_a_controlled_battlefield() {
        let mut game = game();
        let id = game
            .catalog
            .iter()
            .position(|c| c.kind == CardKind::Unit && rules::effect(c.id) == OfficialEffect::None)
            .unwrap();
        let card = game.catalog[id];
        game.players[0].hand = vec![id];
        give_runes(&mut game, 0, card.domain, 12);
        unit(&mut game, 0, 1, Some(1), false);
        game.battlefield_owner[1] = Some(0);
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: Some(1),
            target: None,
        })
        .unwrap();
        assert_eq!(game.players[0].units.last().unwrap().lane, Some(1));
        assert!(!game.players[0].units.last().unwrap().ready);
    }

    #[test]
    fn conquest_history_is_per_player_even_on_opponents_turn() {
        let mut game = game();
        game.claim_lane(0, 0);
        game.claim_lane(1, 0);
        assert_eq!(game.players[0].score, 1);
        assert_eq!(game.players[1].score, 1);
        game.claim_lane(0, 0);
        game.claim_lane(1, 0);
        assert_eq!(game.players[0].score, 1);
        assert_eq!(game.players[1].score, 1);
    }

    #[test]
    fn tied_scores_above_victory_threshold_do_not_end_game() {
        let mut game = game();
        game.players[0].score = 8;
        game.players[1].score = 8;
        game.check_winner(0);
        assert_eq!(game.winner, None);
        game.players[1].score = 9;
        game.check_winner(1);
        assert_eq!(game.winner, Some(1));
    }

    #[test]
    fn grouped_attack_assigns_damage_and_winner_conquers() {
        let mut game = game();
        unit(&mut game, 0, 3, Some(0), false);
        unit(&mut game, 0, 5, Some(0), false);
        unit(&mut game, 1, 6, Some(0), false);
        game.battlefield_owner[0] = Some(1);
        game.apply(Action::Attack { lane: 0 }).unwrap();
        finish_windows(&mut game);
        assert_eq!(game.players[0].units.len(), 1);
        assert_eq!(game.players[0].units[0].damage, 0);
        assert!(game.players[1].units.is_empty());
        assert_eq!(game.players[0].score, 1);
        assert_eq!(game.battlefield_owner[0], Some(0));
    }

    #[test]
    fn final_conquest_draws_until_every_battlefield_scored() {
        let mut game = game();
        game.players[0].score = 7;
        let first = unit(&mut game, 0, 0, None, true);
        let second = unit(&mut game, 0, 2, None, true);
        let hand_size = game.players[0].hand.len();
        game.apply(Action::Move {
            unit_id: first,
            lane: Some(0),
        })
        .unwrap();
        finish_windows(&mut game);
        assert_eq!(game.players[0].score, 7);
        assert_eq!(game.players[0].hand.len(), hand_size + 1);
        assert!(game.scored_this_turn[0]);
        game.apply(Action::Move {
            unit_id: second,
            lane: Some(1),
        })
        .unwrap();
        finish_windows(&mut game);
        assert_eq!(game.winner, Some(0));
    }

    #[test]
    fn holding_can_score_final_point_and_zero_might_can_hold() {
        let mut game = game();
        game.players[0].score = 7;
        unit(&mut game, 0, 0, Some(0), false);
        game.battlefield_owner[0] = Some(0);
        game.apply(Action::EndTurn).unwrap();
        game.apply(Action::EndTurn).unwrap();
        assert_eq!(game.winner, Some(0));
    }

    #[test]
    fn a_battlefield_cannot_score_twice_in_the_same_turn() {
        let mut game = game();
        let id = unit(&mut game, 0, 2, None, true);
        game.apply(Action::Move {
            unit_id: id,
            lane: Some(0),
        })
        .unwrap();
        finish_windows(&mut game);
        game.players[0].units[0].ready = true;
        game.apply(Action::Move {
            unit_id: id,
            lane: None,
        })
        .unwrap();
        finish_windows(&mut game);
        game.players[0].units[0].ready = true;
        game.apply(Action::Move {
            unit_id: id,
            lane: Some(0),
        })
        .unwrap();
        finish_windows(&mut game);
        assert_eq!(game.players[0].score, 1);
    }

    #[test]
    fn burn_out_recycles_discard_and_scores_for_opponent() {
        let mut game = game();
        game.players[0].deck.clear();
        game.players[0].discard = vec![0, 1];
        let hand_size = game.players[0].hand.len();
        game.draw(0, 1);
        assert_eq!(game.players[1].score, 1);
        assert_eq!(game.players[0].hand.len(), hand_size + 1);
        assert_eq!(game.players[0].deck.len(), 1);
        assert!(game.players[0].discard.is_empty());
    }

    #[test]
    fn empty_deck_and_trash_burn_out_terminates() {
        let mut game = game();
        game.players[0].deck.clear();
        game.players[0].discard.clear();
        game.draw(0, 1);
        assert_eq!(game.winner, Some(1));
    }

    #[test]
    fn ai_choice_does_not_depend_on_opponents_hidden_cards() {
        let first = game();
        let mut second = first.clone();
        second.players[1].hand = vec![0; 50];
        second.players[1].deck.reverse();
        assert_eq!(first.choose_ai_action(), second.choose_ai_action());
    }

    #[test]
    fn empty_field_conquest_waits_for_both_players_to_pass() {
        let mut game = game();
        let id = unit(&mut game, 0, 2, None, true);
        game.apply(Action::Move {
            unit_id: id,
            lane: Some(0),
        })
        .unwrap();
        assert_eq!(game.players[0].score, 0);
        assert_eq!(game.battlefield_owner[0], None);
        assert_eq!(game.priority(), 0);
        assert!(game.apply(Action::EndTurn).is_err());
        game.apply(Action::Pass).unwrap();
        assert_eq!(game.priority(), 1);
        assert_eq!(game.active, 0);
        assert_eq!(game.players[0].score, 0);
        game.apply(Action::Pass).unwrap();
        assert_eq!(game.players[0].score, 1);
        assert_eq!(game.priority(), 0);
        assert!(game.showdown.is_none());
    }

    #[test]
    fn reaction_counter_on_opponents_turn_removes_original_before_resolution() {
        let mut game = game();
        let friendly = unit(&mut game, 0, 3, None, false);
        give_runes(&mut game, 0, Domain::Calm, 12);
        give_runes(&mut game, 1, Domain::Calm, 12);
        game.players[0].hand = vec![7]; // Discipline
        game.players[1].hand = vec![6]; // Defy
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: Some(Target::Unit {
                player: 0,
                unit_id: friendly,
            }),
        })
        .unwrap();
        assert_eq!(game.chain.len(), 1);
        assert_eq!(game.players[0].units[0].temporary_buff, 0);
        let original = game.chain[0].id;
        game.apply(Action::Pass).unwrap();
        assert_eq!(game.priority(), 1);
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: Some(Target::Spell { chain_id: original }),
        })
        .unwrap();
        assert_eq!(game.chain.len(), 2);
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert!(game.chain.is_empty());
        assert_eq!(game.players[0].units[0].temporary_buff, 0);
        assert!(game.players[0].discard.contains(&7));
        assert!(game.players[1].discard.contains(&6));
        assert_eq!(game.active, 0);
        assert_eq!(game.priority(), 0);
    }

    #[test]
    fn reaction_buff_resolves_before_damage_and_changes_survival() {
        let mut game = game();
        let target = unit(&mut game, 1, 3, Some(0), false);
        give_runes(&mut game, 0, Domain::Fury, 12);
        give_runes(&mut game, 1, Domain::Calm, 12);
        game.players[0].hand = vec![55]; // Hextech Ray
        game.players[1].hand = vec![7]; // Discipline
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: Some(Target::Unit {
                player: 1,
                unit_id: target,
            }),
        })
        .unwrap();
        game.apply(Action::Pass).unwrap();
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: Some(Target::Unit {
                player: 1,
                unit_id: target,
            }),
        })
        .unwrap();
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert_eq!(game.chain.len(), 1);
        assert_eq!(game.players[1].units[0].temporary_buff, 2);
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert_eq!(game.players[1].units.len(), 1);
        assert_eq!(game.players[1].units[0].damage, 3);
    }

    #[test]
    fn only_reactions_are_legal_on_a_closed_chain() {
        let mut game = game();
        let target = unit(&mut game, 0, 3, None, true);
        give_runes(&mut game, 0, Domain::Body, 12);
        game.players[0].hand = vec![7, 28]; // Discipline then Punch First (Action)
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: Some(Target::Unit {
                player: 0,
                unit_id: target,
            }),
        })
        .unwrap();
        let before = game.players.clone();
        assert!(
            game.apply(Action::PlayCard {
                hand_index: 0,
                lane: None,
                target: Some(Target::Unit {
                    player: 0,
                    unit_id: target
                })
            })
            .is_err()
        );
        assert!(
            game.apply(Action::Move {
                unit_id: target,
                lane: Some(0)
            })
            .is_err()
        );
        assert_eq!(game.players, before);
    }

    #[test]
    fn abandon_returns_the_spell_and_predict_is_a_real_private_choice() {
        let mut game = game();
        let target = unit(&mut game, 0, 3, None, false);
        give_runes(&mut game, 0, Domain::Calm, 12);
        give_runes(&mut game, 1, Domain::Chaos, 12);
        game.players[0].hand = vec![7];
        game.players[1].hand = vec![1];
        let predicted = *game.players[1].deck.last().unwrap();
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: Some(Target::Unit {
                player: 0,
                unit_id: target,
            }),
        })
        .unwrap();
        game.apply(Action::Pass).unwrap();
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: None,
        })
        .unwrap();
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert_eq!(game.players[0].hand, vec![7]);
        assert_eq!(game.players[1].hand.len(), 0);
        assert_eq!(
            game.decision,
            Some(Decision::Predict {
                player: 1,
                card_id: predicted
            })
        );
        assert_eq!(game.priority(), 1);
        game.apply(Action::Choose { accept: true }).unwrap();
        assert_eq!(game.players[1].deck.first(), Some(&predicted));
        assert!(game.decision.is_none());
        assert_eq!(game.priority(), 0);
    }

    #[test]
    fn hard_bargain_lets_controller_pay_or_accept_counter() {
        for pay in [false, true] {
            let mut game = game();
            let target = unit(&mut game, 0, 3, None, false);
            give_runes(&mut game, 0, Domain::Calm, 12);
            give_runes(&mut game, 1, Domain::Order, 12);
            game.players[0].hand = vec![7];
            game.players[1].hand = vec![37];
            game.apply(Action::PlayCard {
                hand_index: 0,
                lane: None,
                target: Some(Target::Unit {
                    player: 0,
                    unit_id: target,
                }),
            })
            .unwrap();
            game.apply(Action::Pass).unwrap();
            game.apply(Action::PlayCard {
                hand_index: 0,
                lane: None,
                target: None,
            })
            .unwrap();
            game.apply(Action::Pass).unwrap();
            game.apply(Action::Pass).unwrap();
            assert!(matches!(
                game.decision,
                Some(Decision::CounterTax {
                    player: 0,
                    cost: 2,
                    ..
                })
            ));
            let energy = game.players[0].energy;
            game.apply(Action::Choose { accept: pay }).unwrap();
            assert_eq!(game.players[0].energy, energy - if pay { 2 } else { 0 });
            assert_eq!(game.chain.len(), usize::from(pay));
            finish_windows(&mut game);
            assert_eq!(
                game.players[0].units[0].temporary_buff,
                if pay { 2 } else { 0 }
            );
        }
    }

    #[test]
    fn every_preset_and_both_difficulties_complete_ai_matches() {
        let count = presets().len();
        for difficulty in [Difficulty::Easy, Difficulty::Tactical] {
            for first in 0..count {
                for second in 0..count {
                    let mut game = Game::new(
                        first,
                        second,
                        0xabc + (first * count + second) as u64,
                        difficulty,
                    );
                    game.start_match().unwrap();
                    for _ in 0..3000 {
                        if game.winner.is_some() {
                            break;
                        }
                        game.step_ai().unwrap();
                        for player in &game.players {
                            assert_eq!(player.total_runes() as usize + player.rune_deck.len(), 12);
                            assert_eq!(
                                player.energy,
                                player.energy_pool
                                    + player
                                        .runes
                                        .iter()
                                        .zip(player.exhausted_runes)
                                        .map(|(&all, exhausted)| all - exhausted)
                                        .sum::<u8>()
                            );
                        }
                    }
                    assert!(
                        game.winner.is_some(),
                        "AI failed to finish {first} vs {second} on {difficulty:?}, turn {}",
                        game.turn
                    );
                }
            }
        }
    }
    #[test]
    fn ride_the_wind_into_empty_field_opens_showdown_before_conquest() {
        let mut game = game();
        let id = unit(&mut game, 0, 3, None, false);
        game.players[0].hand = vec![14];
        let domain = game.catalog[14].domain;
        give_runes(&mut game, 0, domain, 12);
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: Some(0),
            target: Some(Target::Unit {
                player: 0,
                unit_id: id,
            }),
        })
        .unwrap();
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert!(game.showdown.is_some());
        assert!(!game.showdown.as_ref().unwrap().combat);
        assert_eq!(game.players[0].score, 0);
        assert!(game.players[0].units[0].ready);
        finish_windows(&mut game);
        assert_eq!(game.players[0].score, 1);
    }

    #[test]
    fn negative_might_modifiers_are_preserved_below_zero() {
        let mut game = game();
        let id = unit(&mut game, 0, 2, None, false);
        let target = Target::Unit {
            player: 0,
            unit_id: id,
        };
        game.modify_might(target, -4, 0);
        assert_eq!(game.unit_might(game.unit_ref(target).unwrap()), 0);
        game.modify_might(target, 3, 0);
        assert_eq!(game.unit_might(game.unit_ref(target).unwrap()), 1);
    }

    #[test]
    fn bellows_cannot_choose_units_in_different_players_bases() {
        let mut game = game();
        let a = unit(&mut game, 0, 4, None, false);
        let b = unit(&mut game, 1, 4, None, false);
        game.players[0].hand = vec![50];
        let domain = game.catalog[50].domain;
        give_runes(&mut game, 0, domain, 12);
        let before = game.players.clone();
        assert!(
            game.apply(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![
                    Target::Unit {
                        player: 0,
                        unit_id: a
                    },
                    Target::Unit {
                        player: 1,
                        unit_id: b
                    }
                ]
            })
            .is_err()
        );
        assert_eq!(game.players, before);
    }

    #[test]
    fn bellows_rechecks_same_location_after_a_response_moves_a_target() {
        let mut game = game();
        let a = unit(&mut game, 1, 4, Some(0), false);
        let b = unit(&mut game, 1, 4, Some(0), false);
        let c = unit(&mut game, 1, 4, Some(0), false);
        game.resolve_operation(
            0,
            50,
            None,
            vec![
                Target::Unit {
                    player: 1,
                    unit_id: a,
                },
                Target::Unit {
                    player: 1,
                    unit_id: b,
                },
                Target::Unit {
                    player: 1,
                    unit_id: c,
                },
            ],
            None,
        );
        assert!(game.players[1].units.iter().all(|u| u.damage == 1));
        for u in &mut game.players[1].units {
            u.damage = 0;
        }
        game.players[1].units[0].lane = None;
        let all = vec![
            Target::Unit {
                player: 1,
                unit_id: a,
            },
            Target::Unit {
                player: 1,
                unit_id: b,
            },
            Target::Unit {
                player: 1,
                unit_id: c,
            },
        ];
        let groups = game.resolution_groups(0, 50, None, &all);
        assert_eq!(groups.len(), 2);
        let selected = groups.into_iter().find(|group| group.len() == 2).unwrap();
        game.resolve_operation(0, 50, None, selected, None);
        assert_eq!(game.players[1].units[0].damage, 0);
        assert_eq!(game.players[1].units[1].damage, 1);
        assert_eq!(game.players[1].units[2].damage, 1);
    }

    #[test]
    fn deflect_is_paid_for_each_choice_and_rejection_is_atomic() {
        let mut game = game();
        let victim = game.create_unit(1, 20, false, 0);
        game.players[1].units.last_mut().unwrap().lane = Some(0);
        game.players[0].hand = vec![55];
        let domain = game.catalog[55].domain;
        give_runes(&mut game, 0, domain, 3);
        game.players[0].rune_deck.clear();
        let target = Target::Unit {
            player: 1,
            unit_id: victim,
        };
        assert_eq!(game.deflect_cost(0, &[target, target]), 2);
        let required = game.catalog[55].power_cost;
        game.players[0].runes = [0; 6];
        game.players[0].exhausted_runes = [0; 6];
        game.players[0].runes[domain_index(domain)] = required;
        game.players[0].energy = required;
        let before = game.players.clone();
        assert!(
            game.apply(Action::PlayCard {
                hand_index: 0,
                lane: None,
                target: Some(target)
            })
            .is_err()
        );
        assert_eq!(game.players, before);
        give_runes(&mut game, 0, domain, 12);
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: Some(target),
        })
        .unwrap();
        assert_eq!(game.players[0].total_runes(), 12 - required - 1);
    }

    #[test]
    fn flow_uses_alternate_cost_and_banishes_the_physical_spell() {
        let mut game = game();
        game.players[0].discard = vec![48];
        give_runes(&mut game, 0, Domain::Chaos, 3);
        game.apply(Action::PlayFromTrash {
            trash_index: 0,
            lane: None,
            target: None,
        })
        .unwrap();
        assert!(game.players[0].discard.is_empty());
        assert_eq!(game.players[0].energy, 0);
        finish_windows(&mut game);
        assert_eq!(game.players[0].units.len(), 2);
        assert!(
            game.players[0]
                .units
                .iter()
                .all(|u| u.token && game.unit_might(u) == 1)
        );
        assert_eq!(game.players[0].banish, vec![48]);
    }

    #[test]
    fn ezreal_onplay_waits_on_chain_then_discards_before_drawing() {
        let mut game = game();
        game.players[0].hand = vec![35, 25];
        let domain = game.catalog[35].domain;
        give_runes(&mut game, 0, domain, 12);
        let deck = game.players[0].deck.len();
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: None,
        })
        .unwrap();
        assert_eq!(game.players[0].units.len(), 1);
        assert_eq!(game.players[0].hand, vec![25]);
        assert!(game.chain.last().unwrap().ability.is_some());
        assert!(game.decision.is_none());
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert!(matches!(
            game.decision,
            Some(Decision::Discard {
                count: 1,
                draw_after: 2,
                ..
            })
        ));
        let before = game.players.clone();
        assert!(
            game.apply(Action::SelectCards { indices: vec![9] })
                .is_err()
        );
        assert_eq!(game.players, before);
        game.apply(Action::SelectCards { indices: vec![0] })
            .unwrap();
        assert_eq!(game.players[0].hand.len(), 2);
        assert_eq!(game.players[0].deck.len(), deck - 2);
        assert!(game.players[0].discard.contains(&25));
    }

    #[test]
    fn top_three_looks_without_burning_out_and_only_moves_seen_cards() {
        let mut game = game();
        game.players[0].deck = vec![25, 27];
        let score = game.players[1].score;
        game.resolve_operation(0, 39, None, Vec::new(), None);
        assert!(
            matches!(&game.decision,Some(Decision::TopThree{cards,..}) if cards == &vec![27,25])
        );
        game.apply(Action::SelectCards { indices: vec![1] })
            .unwrap();
        assert!(game.players[0].deck.is_empty());
        assert_eq!(game.players[1].score, score);
        assert_eq!(game.players[0].hand.last(), Some(&25));
        assert_eq!(game.players[0].discard.last(), Some(&27));
    }

    #[test]
    fn sabotage_recycles_the_selected_revealed_nonunit() {
        let mut game = game();
        game.players[1].hand = vec![25, 7, 8];
        game.resolve_operation(0, 32, None, Vec::new(), None);
        let before = game.players.clone();
        assert!(
            game.apply(Action::SelectCards { indices: vec![0] })
                .is_err()
        );
        assert_eq!(game.players, before);
        game.apply(Action::SelectCards { indices: vec![2] })
            .unwrap();
        assert_eq!(game.players[1].hand, vec![25, 7]);
        assert_eq!(game.players[1].deck.first(), Some(&8));
    }

    #[test]
    fn combat_cleanup_still_heals_when_reaction_removed_last_defender() {
        let mut game = game();
        let id = game.create_unit(0, 56, false, 0);
        game.players[0].units[0].lane = Some(0);
        let enemy = unit(&mut game, 1, 1, Some(0), false);
        game.open_showdown(0, 0);
        assert!(game.in_combat(0));
        game.unit_mut(Target::Unit {
            player: 0,
            unit_id: id,
        })
        .unwrap()
        .damage = 2;
        game.bounce(Target::Unit {
            player: 1,
            unit_id: enemy,
        });
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        let survivor = game
            .unit_ref(Target::Unit {
                player: 0,
                unit_id: id,
            })
            .unwrap();
        assert_eq!(survivor.damage, 0);
        assert_eq!(game.unit_might(survivor), 1);
    }
    #[test]
    fn accelerate_pays_actual_additional_cost_and_unit_enters_ready() {
        let mut game = game();
        game.players[0].hand = vec![49];
        give_runes(&mut game, 0, Domain::Fury, 12);
        game.apply(Action::PlayCardOptions {
            hand_index: 0,
            lane: None,
            targets: Vec::new(),
            repeat_targets: Vec::new(),
            repeat: false,
            accelerate: true,
            bonus: false,
        })
        .unwrap();
        assert!(game.players[0].units[0].ready);
        assert_eq!(game.players[0].energy, 7);
        assert_eq!(game.players[0].total_runes(), 11);
    }

    #[test]
    fn invalid_optional_cost_flag_cannot_spend_a_card_or_runes() {
        let mut game = game();
        game.players[0].hand = vec![25];
        give_runes(&mut game, 0, Domain::Calm, 12);
        let before = game.players.clone();
        assert!(
            game.apply(Action::PlayCardOptions {
                hand_index: 0,
                lane: None,
                targets: Vec::new(),
                repeat_targets: Vec::new(),
                repeat: true,
                accelerate: false,
                bonus: false
            })
            .is_err()
        );
        assert_eq!(game.players, before);
    }

    #[test]
    fn repeat_bellows_resolves_both_selected_target_groups() {
        let mut game = game();
        let a = unit(&mut game, 1, 5, None, false);
        let b = unit(&mut game, 1, 5, None, false);
        game.players[0].hand = vec![50];
        give_runes(&mut game, 0, Domain::Mind, 12);
        let card = game.catalog[50];
        game.apply(Action::PlayCardOptions {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: 1,
                unit_id: a,
            }],
            repeat_targets: vec![Target::Unit {
                player: 1,
                unit_id: b,
            }],
            repeat: true,
            accelerate: false,
            bonus: false,
        })
        .unwrap();
        assert_eq!(game.players[0].total_runes(), 12 - card.power_cost - 1);
        finish_windows(&mut game);
        assert_eq!(
            game.unit_ref(Target::Unit {
                player: 1,
                unit_id: a
            })
            .unwrap()
            .damage,
            1
        );
        assert_eq!(
            game.unit_ref(Target::Unit {
                player: 1,
                unit_id: b
            })
            .unwrap()
            .damage,
            1
        );
        assert_eq!(
            game.players[0]
                .discard
                .iter()
                .filter(|&&id| id == 50)
                .count(),
            1
        );
    }

    #[test]
    fn marauder_empower_is_a_paid_respondable_persistent_ability() {
        let mut game = game();
        let id = game.create_unit(0, 24, false, 0);
        give_runes(&mut game, 0, Domain::Body, 3);
        game.apply(Action::EmpowerUnit {
            unit_id: id,
            pay_power: false,
        })
        .unwrap();
        assert!(!game.unit_empowered(id));
        assert_eq!(game.players[0].energy, 2);
        finish_windows(&mut game);
        assert!(game.unit_empowered(id));
        assert_eq!(game.unit_might(&game.players[0].units[0]), 3);
        let before = game.players.clone();
        assert!(
            game.apply(Action::EmpowerUnit {
                unit_id: id,
                pay_power: true
            })
            .is_err()
        );
        assert_eq!(game.players, before);
    }

    #[test]
    fn matriarch_revives_a_cheap_unit_without_paying_its_printed_power() {
        let mut game = game();
        let id = game.create_unit(0, 45, false, 0);
        give_runes(&mut game, 0, Domain::Chaos, 2);
        game.players[0].discard = vec![35];
        game.apply(Action::EmpowerUnit {
            unit_id: id,
            pay_power: false,
        })
        .unwrap();
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert!(matches!(game.decision, Some(Decision::ReviveUnit { .. })));
        assert_eq!(game.players[0].energy, 0);
        game.apply(Action::PlayFromTrash {
            trash_index: 0,
            lane: None,
            target: None,
        })
        .unwrap();
        assert!(
            !game.players[0].units.iter().any(|u| u.card_id == 35),
            "revival waits for responses"
        );
        finish_windows(&mut game);
        assert!(game.players[0].units.iter().any(|u| u.card_id == 35));
        assert!(!game.players[0].discard.contains(&35));
    }

    #[test]
    fn tideturner_swap_preserves_control_and_hidden_card_during_atomic_exchange() {
        let mut game = game();
        let defender = unit(&mut game, 0, 2, Some(0), false);
        let tidal = game.create_unit(0, 18, false, 0);
        game.battlefield_owner[0] = Some(0);
        game.players[0].hidden.push(HiddenCard {
            id: 900,
            card_id: 44,
            lane: 0,
            hidden_turn: 0,
            revealed: false,
        });
        game.decision = Some(Decision::SwapUnit {
            player: 0,
            unit_id: tidal,
        });
        game.apply(Action::ChooseTarget {
            target: Some(Target::Unit {
                player: 0,
                unit_id: defender,
            }),
        })
        .unwrap();
        finish_windows(&mut game);
        assert_eq!(
            game.unit_ref(Target::Unit {
                player: 0,
                unit_id: tidal
            })
            .unwrap()
            .lane,
            Some(0)
        );
        assert_eq!(
            game.unit_ref(Target::Unit {
                player: 0,
                unit_id: defender
            })
            .unwrap()
            .lane,
            None
        );
        assert_eq!(game.players[0].hidden.len(), 1);
        assert_eq!(game.battlefield_owner[0], Some(0));
    }

    #[test]
    fn tornado_can_empower_an_enemy_here_and_disempowers_at_end_of_turn() {
        let mut game = game();
        let enemy = game.create_unit(1, 24, false, 0);
        game.players[1].units.last_mut().unwrap().lane = Some(0);
        game.decision = Some(Decision::EmpowerHere {
            player: 0,
            lane: Some(0),
            source_id: None,
        });
        game.apply(Action::ChooseTarget {
            target: Some(Target::Unit {
                player: 1,
                unit_id: enemy,
            }),
        })
        .unwrap();
        assert!(!game.unit_empowered(enemy), "Empower waits for responses");
        finish_windows(&mut game);
        assert!(game.unit_empowered(enemy));
        game.apply(Action::EndTurn).unwrap();
        assert!(
            game.unit_empowered(enemy),
            "delayed Disempower waits for responses"
        );
        finish_windows(&mut game);
        assert!(!game.unit_empowered(enemy));
    }

    #[test]
    fn scuttle_death_reveals_hand_and_grants_temporary_hidden_inspection() {
        let mut game = game();
        let id = game.create_unit(0, 15, false, 0);
        game.players[0].units.last_mut().unwrap().damage = 1;
        let opponent_hand = game.players[1].hand.clone();
        game.remove_defeated();
        game.after_resolution();
        assert!(
            game.unit_ref(Target::Unit {
                player: 0,
                unit_id: id
            })
            .is_none()
        );
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert!(game.hidden_view[0]);
        assert_eq!(game.players[0].xp, 1);
        assert!(
            matches!(&game.decision,Some(Decision::RevealHand{cards,..})if cards==&opponent_hand)
        );
        game.apply(Action::Choose { accept: false }).unwrap();
        game.apply(Action::EndTurn).unwrap();
        assert!(!game.hidden_view[0]);
    }

    #[test]
    fn countering_flow_banishes_it_even_when_abandon_would_return_it() {
        for to_hand in [false, true] {
            let mut game = game();
            game.players[0].discard = vec![48];
            give_runes(&mut game, 0, Domain::Chaos, 3);
            game.apply(Action::PlayFromTrash {
                trash_index: 0,
                lane: None,
                target: None,
            })
            .unwrap();
            let chain_id = game
                .chain
                .iter()
                .find(|item| item.ability.is_none())
                .unwrap()
                .id;
            game.counter_spell(chain_id, to_hand);
            assert_eq!(game.players[0].banish, vec![48]);
            assert!(game.players[0].discard.is_empty());
            assert!(!game.players[0].hand.contains(&48));
        }
    }

    #[test]
    fn countering_fizz_replay_recycles_it_even_when_abandon_would_return_it() {
        for to_hand in [false, true] {
            let mut game = game();
            game.players[0].discard = vec![48];
            game.decision = Some(Decision::ReplaySpell { player: 0 });
            game.apply(Action::PlayFromTrash {
                trash_index: 0,
                lane: None,
                target: None,
            })
            .unwrap();
            let chain_id = game
                .chain
                .iter()
                .find(|item| item.ability.is_none())
                .unwrap()
                .id;
            game.counter_spell(chain_id, to_hand);
            assert_eq!(game.players[0].deck.first(), Some(&48));
            assert!(game.players[0].discard.is_empty());
            assert!(!game.players[0].hand.contains(&48));
        }
    }

    #[test]
    fn first_mate_may_ready_an_enemy_without_triggering_enemy_irelia() {
        let mut game = game();
        let enemy = game.create_unit(1, 0, false, 0);
        game.players[0].hand = vec![23];
        give_runes(&mut game, 0, Domain::Body, 12);
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: Some(Target::Unit {
                player: 1,
                unit_id: enemy,
            }),
        })
        .unwrap();
        finish_windows(&mut game);
        let unit = game
            .unit_ref(Target::Unit {
                player: 1,
                unit_id: enemy,
            })
            .unwrap();
        assert!(unit.ready);
        assert_eq!(unit.temporary_buff, 0);
    }

    #[test]
    fn combat_damage_waits_for_both_assignments_and_preserves_human_order() {
        let mut game = game();
        let attacker = unit(&mut game, 0, 5, Some(0), false);
        let first = unit(&mut game, 1, 3, Some(0), false);
        let second = unit(&mut game, 1, 3, Some(0), false);
        game.apply(Action::Attack { lane: 0 }).unwrap();
        assert_eq!(
            game.decision,
            Some(Decision::CombatDamage {
                player: 0,
                lane: 0,
                total: 5
            })
        );
        game.apply(Action::AssignDamage {
            assignments: vec![(second, 3), (first, 2)],
        })
        .unwrap();
        assert!(
            game.players
                .iter()
                .flat_map(|p| &p.units)
                .all(|u| u.damage == 0)
        );
        assert_eq!(game.priority(), 1);
        game.apply(Action::AssignDamage {
            assignments: vec![(attacker, 6)],
        })
        .unwrap();
        assert!(game.players[0].units.is_empty());
        assert_eq!(game.players[1].units.len(), 1);
        assert_eq!(game.players[1].units[0].id, first);
        assert_eq!(game.players[1].units[0].damage, 0);
    }

    #[test]
    fn combat_assignment_enforces_lethal_and_rejects_without_mutation() {
        let mut game = game();
        unit(&mut game, 0, 5, Some(0), false);
        let a = unit(&mut game, 1, 3, Some(0), false);
        let b = unit(&mut game, 1, 3, Some(0), false);
        game.apply(Action::Attack { lane: 0 }).unwrap();
        let before = game.players.clone();
        let decision = game.decision.clone();
        for assignments in [
            vec![(a, 2), (b, 3)],
            vec![(a, 4), (b, 1)],
            vec![(a, 3), (a, 2)],
            vec![(a, 3)],
            vec![(999, 5)],
        ] {
            assert!(game.apply(Action::AssignDamage { assignments }).is_err());
            assert_eq!(game.players, before);
            assert_eq!(game.decision, decision);
        }
        assert!(game.apply(Action::Choose { accept: false }).is_err());
        assert!(game.apply(Action::Pass).is_err());
    }

    #[test]
    fn combat_tank_and_backline_priorities_are_validated() {
        let mut game = game();
        unit(&mut game, 0, 20, Some(0), false);
        let tank = game.create_unit(1, 57, false, 0);
        let normal = unit(&mut game, 1, 2, Some(0), false);
        let back = game.create_unit(1, 13, false, 0);
        for u in &mut game.players[1].units {
            u.lane = Some(0);
        }
        game.apply(Action::Attack { lane: 0 }).unwrap();
        assert!(
            game.apply(Action::AssignDamage {
                assignments: vec![(normal, 2), (tank, 4), (back, 14)]
            })
            .is_err()
        );
        let assignments = game.automatic_damage_assignment(0, 0, 20);
        assert_eq!(
            assignments.iter().map(|a| a.0).collect::<Vec<_>>(),
            vec![tank, normal, back]
        );
        assert_eq!(assignments.iter().map(|a| a.1).sum::<u16>(), 20);
        game.apply(Action::AssignDamage { assignments }).unwrap();
    }

    #[test]
    fn countered_spells_do_not_trigger_ravenbloom_or_offhand_kennen() {
        let mut game = game();
        game.legend[0] = LegendState::new(33);
        game.create_unit(0, 60, false, 0);
        game.players[0].discard = vec![48];
        game.decision = Some(Decision::ReplaySpell { player: 0 });
        game.apply(Action::PlayFromTrash {
            trash_index: 0,
            lane: None,
            target: None,
        })
        .unwrap();
        assert_eq!(game.chain.len(), 1);
        assert!(game.chain[0].ability.is_none());
        let id = game.chain[0].id;
        game.counter_spell(id, false);
        game.after_resolution();
        assert!(game.chain.is_empty());
        assert!(!game.legend[0].empowered);
        assert_eq!(game.players[0].units[0].temporary_buff, 0);
    }

    #[test]
    fn successful_spells_trigger_ravenbloom_only_after_resolution() {
        let mut game = game();
        game.create_unit(0, 60, false, 0);
        game.players[0].hand = vec![48];
        give_runes(&mut game, 0, Domain::Chaos, 3);
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: None,
        })
        .unwrap();
        assert_eq!(game.chain.len(), 1);
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert!(matches!(
            game.decision,
            Some(Decision::TokenPlacement { remaining: 2, .. })
        ));
        game.apply(Action::PlaceToken { lane: None }).unwrap();
        game.apply(Action::PlaceToken { lane: None }).unwrap();
        assert_eq!(game.players[0].units[0].temporary_buff, 0);
        assert!(matches!(
            game.chain.last().unwrap().ability,
            Some(ChainAbility::Triggered(TriggeredAbility::Might {
                amount: 1,
                ..
            }))
        ));
        finish_windows(&mut game);
        assert_eq!(game.players[0].units[0].temporary_buff, 1);
    }

    #[test]
    fn vex_locks_owner_effect_movement_but_enemy_movement_can_still_move_unit() {
        let mut game = game();
        let id = unit(&mut game, 0, 4, Some(0), true);
        game.resolve_trigger(
            1,
            20,
            TriggeredAbility::Stun {
                player: 0,
                unit_id: id,
            },
        );
        let target = Target::Unit {
            player: 0,
            unit_id: id,
        };
        game.move_effect(0, target, None, false);
        assert_eq!(game.unit_ref(target).unwrap().lane, Some(0));
        assert!(
            game.apply(Action::Move {
                unit_id: id,
                lane: None
            })
            .is_err()
        );
        game.move_effect(1, target, Some(1), false);
        assert_eq!(game.unit_ref(target).unwrap().lane, Some(1));
    }

    #[test]
    fn played_unit_tokens_trigger_vex_but_not_kennen_card_trigger() {
        let mut game = game();
        game.legend[0] = LegendState::new(33);
        game.create_unit(1, 20, false, 0);
        game.players[1].units[0].lane = Some(0);
        game.resolve_trigger(0, 34, TriggeredAbility::Tokens { might: 1, count: 1 });
        game.after_resolution();
        assert_eq!(game.chain.len(), 1);
        assert!(matches!(
            game.chain[0].ability,
            Some(ChainAbility::Triggered(TriggeredAbility::Stun { .. }))
        ));
        assert!(!game.legend[0].empowered);
        finish_windows(&mut game);
        assert!(game.players[0].units[0].stunned);
    }

    #[test]
    fn beginning_triggers_resolve_before_holding_and_holding_before_channel_draw() {
        let mut game = game();
        game.players[0].units.clear();
        let irelia = game.create_unit(0, 0, false, 0);
        let yi = game.create_unit(0, 22, false, 0);
        for u in &mut game.players[0].units {
            u.lane = Some(0);
            u.ready = false;
        }
        game.battlefield_owner[0] = Some(0);
        give_runes(&mut game, 0, Domain::Calm, 2);
        let hand = game.players[0].hand.len();
        game.start_turn();
        assert_eq!(game.players[0].score, 0);
        assert_eq!(game.players[0].total_runes(), 2);
        assert_eq!(game.players[0].hand.len(), hand);
        assert!(
            game.unit_ref(Target::Unit {
                player: 0,
                unit_id: irelia
            })
            .unwrap()
            .ready
        );
        assert!(matches!(
            game.chain.last().unwrap().ability,
            Some(ChainAbility::Triggered(TriggeredAbility::Might { .. }))
        ));
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert_eq!(game.players[0].score, 1);
        assert_eq!(game.players[0].total_runes(), 2);
        assert_eq!(game.players[0].hand.len(), hand);
        assert!(matches!(
            game.chain.last().unwrap().ability,
            Some(ChainAbility::Triggered(TriggeredAbility::Xp(2)))
        ));
        finish_windows(&mut game);
        assert_eq!(game.players[0].total_runes(), 4);
        assert_eq!(game.players[0].hand.len(), hand + 1);
        assert!(
            game.unit_ref(Target::Unit {
                player: 0,
                unit_id: yi
            })
            .is_some()
        );
        assert!(game.pending_start.is_none());
    }

    #[test]
    fn channel_does_not_refresh_runes_spent_during_hold_payment() {
        let mut game = game();
        game.players[0].units.clear();
        let yi = game.create_unit(0, 22, false, 0);
        game.players[0].units[0].lane = Some(0);
        game.battlefield_owner[0] = Some(0);
        give_runes(&mut game, 0, Domain::Fury, 4);
        game.start_turn();
        assert!(!game.chain.is_empty());
        game.pay_energy(0, 2);
        finish_windows(&mut game);
        assert_eq!(game.players[0].total_runes(), 6);
        assert_eq!(game.players[0].energy, 4);
        assert!(
            game.unit_ref(Target::Unit {
                player: 0,
                unit_id: yi
            })
            .is_some()
        );
    }

    #[test]
    fn pure_stun_does_not_prevent_standard_movement() {
        let mut game = game();
        let id = unit(&mut game, 0, 2, None, true);
        game.players[0].units[0].stunned = true;
        game.apply(Action::Move {
            unit_id: id,
            lane: Some(0),
        })
        .unwrap();
        finish_windows(&mut game);
        assert_eq!(game.players[0].score, 1);
        assert_eq!(game.lane_might(0, 0), 0);
    }

    #[test]
    fn combat_recall_preserves_a_ready_survivors_state() {
        let mut game = game();
        let id = unit(&mut game, 0, 0, Some(0), true);
        unit(&mut game, 1, 0, Some(0), false);
        game.apply(Action::Attack { lane: 0 }).unwrap();
        let survivor = game
            .unit_ref(Target::Unit {
                player: 0,
                unit_id: id,
            })
            .unwrap();
        assert_eq!(survivor.lane, None);
        assert!(survivor.ready);
    }

    #[test]
    fn vex_movement_lock_does_not_prevent_separate_ready_instruction() {
        let mut game = game();
        let id = game.create_unit(0, 0, false, 0);
        game.players[0].units[0].lane = Some(0);
        game.resolve_trigger(
            1,
            20,
            TriggeredAbility::Stun {
                player: 0,
                unit_id: id,
            },
        );
        game.move_effect(
            0,
            Target::Unit {
                player: 0,
                unit_id: id,
            },
            Some(1),
            true,
        );
        let unit = game
            .unit_ref(Target::Unit {
                player: 0,
                unit_id: id,
            })
            .unwrap();
        assert_eq!(unit.lane, Some(0));
        assert!(unit.ready);
        game.after_resolution();
        assert!(matches!(
            game.chain.last().unwrap().ability,
            Some(ChainAbility::Triggered(TriggeredAbility::Might { .. }))
        ));
    }

    #[test]
    fn repeated_damage_finishes_before_guardian_death_replacement_cleanup() {
        let mut game = game();
        let id = unit(&mut game, 1, 1, None, false);
        game.players[1].gear.push(GearState {
            id: 900,
            card_id: 10,
            attached_to: Some(id),
            ready: true,
            temporary: false,
            token: false,
        });
        game.players[0].hand = vec![50];
        give_runes(&mut game, 0, Domain::Mind, 4);
        let target = Target::Unit {
            player: 1,
            unit_id: id,
        };
        game.apply(Action::PlayCardOptions {
            hand_index: 0,
            lane: None,
            targets: vec![target],
            repeat_targets: vec![target],
            repeat: true,
            accelerate: false,
            bonus: false,
        })
        .unwrap();
        finish_windows(&mut game);
        let survivor = game
            .unit_ref(target)
            .expect("Guardian saves the unit after both repeated damage packets");
        assert_eq!(survivor.damage, 0);
        assert_eq!(survivor.lane, None);
        assert!(game.players[1].gear.is_empty());
    }

    #[test]
    fn opponent_can_start_and_second_player_receives_opening_rune_bonus() {
        let mut game = Game::new(0, 1, 700, Difficulty::Easy);
        game.choose_starting_player(1).unwrap();
        game.start_match().unwrap();
        assert_eq!(game.active, 1);
        assert_eq!(game.priority(), 1);
        assert_eq!(game.players[1].total_runes(), 2);
        assert_eq!(game.players[0].total_runes(), 0);
        assert_eq!(game.players[1].hand.len(), 5);
        game.run_ai_turn().unwrap();
        assert_eq!(game.active, 0);
        assert_eq!(game.players[0].total_runes(), 3);
        assert_eq!(game.players[0].hand.len(), 5);
        let before = game.players.clone();
        assert!(game.choose_starting_player(0).is_err());
        assert_eq!(game.players, before);
        game.rematch(701);
        assert_eq!(game.first_player, 1);
        assert_eq!(game.active, 1);
        assert_eq!(game.phase, Phase::Mulligan);
    }

    #[test]
    fn invalid_starting_player_is_rejected_without_mutation() {
        let mut game = Game::new(0, 1, 700, Difficulty::Easy);
        assert!(game.choose_starting_player(2).is_err());
        assert_eq!(game.active, 0);
        assert_eq!(game.first_player, 0);
    }
    #[test]
    fn rune_activation_preserves_floating_energy_when_recycled() {
        let mut g = game();
        give_runes(&mut g, 0, Domain::Calm, 2);
        g.apply(Action::ActivateRune {
            domain: Domain::Calm,
            recycle: false,
        })
        .unwrap();
        assert_eq!((g.players[0].energy, g.players[0].energy_pool), (2, 1));
        g.apply(Action::ActivateRune {
            domain: Domain::Calm,
            recycle: true,
        })
        .unwrap();
        assert_eq!(
            (
                g.players[0].energy,
                g.players[0].energy_pool,
                g.players[0].total_runes()
            ),
            (2, 1, 1)
        );
        assert_eq!(g.players[0].power_pool[domain_index(Domain::Calm)], 1);
        g.pay_energy(0, 2);
        assert_eq!((g.players[0].energy, g.players[0].energy_pool), (0, 0));
        assert_eq!(g.players[0].exhausted_runes[domain_index(Domain::Calm)], 1);
    }

    #[test]
    fn automatic_power_payment_floats_surplus_energy_until_pool_emptying() {
        let mut g = game();
        give_runes(&mut g, 0, Domain::Chaos, 2);
        g.spend_power(0, Some(Domain::Chaos), 1).unwrap();
        assert_eq!((g.players[0].energy, g.players[0].energy_pool), (2, 1));
        g.apply(Action::EndTurn).unwrap();
        assert_eq!((g.players[0].energy, g.players[0].energy_pool), (1, 0));
    }

    #[test]
    fn both_agents_receive_unchanged_hands_and_independent_mulligans() {
        let mut a = Game::new_for_agents(0, 1, 99, Difficulty::Easy);
        let mut b = a.clone();
        let first = a.players[0].clone();
        let second = a.players[1].clone();
        a.mulligan_player(1, &[0, 1]).unwrap();
        assert_eq!(a.players[0], first);
        assert_ne!(a.players[1].hand, second.hand);
        assert_eq!(a.mulligans_used, [false, true]);
        assert!(a.mulligan_player(1, &[]).is_err());
        a.mulligan_player(0, &[2]).unwrap();
        b.mulligan_player(1, &[0, 1]).unwrap();
        b.mulligan_player(0, &[2]).unwrap();
        assert_eq!(a.players, b.players);
    }

    #[test]
    fn controller_chooses_which_simultaneous_unit_hourglass_saves() {
        let mut g = game();
        let a = unit(&mut g, 0, 2, None, false);
        let b = unit(&mut g, 0, 3, None, false);
        g.gear_play(0, 21, None).unwrap();
        let gear = g.players[0].gear[0].id;
        for u in &mut g.players[0].units {
            u.damage = 9;
        }
        g.after_resolution();
        assert!(
            matches!(g.decision,Some(Decision::DeathReplacement{ref options,..}) if options.len()==2)
        );
        let before = g.players.clone();
        assert!(
            g.apply(Action::ChooseDeathReplacement {
                unit_id: 9999,
                gear_id: Some(gear)
            })
            .is_err()
        );
        assert_eq!(g.players, before);
        g.apply(Action::ChooseDeathReplacement {
            unit_id: a,
            gear_id: Some(gear),
        })
        .unwrap();
        assert!(
            g.players[0]
                .units
                .iter()
                .any(|u| u.id == a && u.damage == 0)
        );
        assert!(!g.players[0].units.iter().any(|u| u.id == b));
    }

    #[test]
    fn smite_and_guardian_are_ordered_by_affected_units_controller() {
        let mut g = game();
        let id = unit(&mut g, 0, 2, None, false);
        g.gear_play(0, 10, None).unwrap();
        let gear = g.players[0].gear[0].id;
        g.gear_resolve_equip(
            0,
            gear,
            Target::Unit {
                player: 0,
                unit_id: id,
            },
        );
        g.players[0].units[0].damage = 9;
        g.players[0].units[0].banish_on_death = true;
        g.after_resolution();
        assert!(
            matches!(g.decision,Some(Decision::DeathReplacement{ref options,..}) if options.len()==2)
        );
        g.apply(Action::ChooseDeathReplacement {
            unit_id: id,
            gear_id: Some(gear),
        })
        .unwrap();
        assert_eq!(g.players[0].units.len(), 1);
        assert_eq!(g.players[0].units[0].damage, 0);
        assert!(g.players[0].banish.is_empty());
    }

    #[test]
    fn token_base_might_does_not_count_as_an_existing_buff() {
        let mut g = game();
        let token = g.create_unit(0, 48, true, 1);
        let target = Target::Unit {
            player: 0,
            unit_id: token,
        };
        g.resolve_operation(0, 27, None, vec![target], Some(999));
        assert_eq!(g.unit_might(g.unit_ref(target).unwrap()), 2);
        g.resolve_operation(0, 27, None, vec![target], Some(998));
        assert_eq!(g.unit_might(g.unit_ref(target).unwrap()), 2);
    }

    #[test]
    fn simultaneous_triggers_allow_owner_order_and_then_opponent_finalization() {
        let mut g = game();
        g.queue_ability(
            1,
            68,
            vec![],
            ChainAbility::Triggered(TriggeredAbility::Draw(1)),
        );
        g.queue_ability(
            0,
            15,
            vec![],
            ChainAbility::Triggered(TriggeredAbility::Draw(1)),
        );
        g.queue_ability(
            0,
            25,
            vec![],
            ChainAbility::Triggered(TriggeredAbility::Draw(1)),
        );
        assert!(
            g.chain.is_empty(),
            "pending triggers are not finalized counter targets"
        );
        g.after_resolution();
        let Some(Decision::OrderTriggers {
            player,
            ref options,
        }) = g.decision
        else {
            panic!("owner must choose order")
        };
        assert_eq!(player, 0);
        let chosen = options.iter().find(|o| o.card_id == 25).unwrap().id;
        g.apply(Action::OrderTrigger { trigger_id: chosen })
            .unwrap();
        assert_eq!(
            g.chain
                .iter()
                .map(|i| (i.player, i.card_id))
                .collect::<Vec<_>>(),
            vec![(0, 25), (0, 15), (1, 68)]
        );
        assert_eq!(g.priority(), 1);
    }

    #[test]
    fn bellows_controller_can_keep_smaller_legal_target_group() {
        let mut g = game();
        g.battlefields = [0, 1];
        let a = unit(&mut g, 1, 4, Some(0), false);
        let b = unit(&mut g, 1, 4, Some(0), false);
        let c = unit(&mut g, 1, 4, Some(0), false);
        g.players[0].hand = vec![50];
        give_runes(&mut g, 0, Domain::Mind, 6);
        let targets = vec![a, b, c]
            .into_iter()
            .map(|unit_id| Target::Unit { player: 1, unit_id })
            .collect();
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets,
        })
        .unwrap();
        g.players[1].units[1].lane = None;
        g.players[1].units[2].lane = None;
        g.apply(Action::Pass).unwrap();
        g.apply(Action::Pass).unwrap();
        let Some(Decision::ResolutionTargets { ref groups, .. }) = g.decision else {
            panic!("split targets need a choice")
        };
        let index = groups.iter().position(|group| group.len() == 1).unwrap();
        g.apply(Action::ChooseResolutionGroup { index }).unwrap();
        assert_eq!(g.players[1].units[0].damage, 1);
        assert_eq!(g.players[1].units[1].damage, 0);
        assert_eq!(g.players[1].units[2].damage, 0);
    }

    #[test]
    fn ezreal_controller_allocates_optional_discount_to_power() {
        let mut g = game();
        g.create_unit(0, 35, false, 0);
        let options = PlayOptions {
            repeat: true,
            ..PlayOptions::default()
        };
        assert_eq!(g.optional_cost(0, 50, options).unwrap(), (0, 1));
        g.apply(Action::SetOptionalDiscount { power: 1 }).unwrap();
        assert_eq!(g.optional_cost(0, 50, options).unwrap(), (1, 0));
        assert!(g.apply(Action::SetOptionalDiscount { power: 2 }).is_err());
        assert_eq!(g.optional_power_discount[0], 1);
    }
    #[test]
    fn tentacles_can_be_placed_independently_without_intermediate_responses() {
        let mut g = game();
        g.battlefields = [0, 1];
        unit(&mut g, 0, 3, Some(0), false);
        g.battlefield_owner[0] = Some(0);
        g.players[0].hand = vec![48];
        give_runes(&mut g, 0, Domain::Chaos, 6);
        g.apply(Action::PlayCard {
            hand_index: 0,
            lane: None,
            target: None,
        })
        .unwrap();
        g.apply(Action::Pass).unwrap();
        g.apply(Action::Pass).unwrap();
        assert!(matches!(
            g.decision,
            Some(Decision::TokenPlacement { remaining: 2, .. })
        ));
        let chain = g.chain.clone();
        assert!(g.apply(Action::PlaceToken { lane: Some(1) }).is_err());
        assert_eq!(g.chain, chain);
        g.apply(Action::PlaceToken { lane: Some(0) }).unwrap();
        assert!(matches!(
            g.decision,
            Some(Decision::TokenPlacement { remaining: 1, .. })
        ));
        assert_eq!(g.players[0].units.len(), 2);
        assert!(g.players[0].discard.contains(&48));
        g.apply(Action::PlaceToken { lane: None }).unwrap();
        assert_eq!(
            g.players[0]
                .units
                .iter()
                .filter(|u| u.token && u.card_id == 48 && u.lane == Some(0))
                .count(),
            1
        );
        assert_eq!(
            g.players[0]
                .units
                .iter()
                .filter(|u| u.token && u.card_id == 48 && u.lane.is_none())
                .count(),
            1
        );
    }

    #[test]
    fn winning_burnout_recycles_trash_before_awarding_the_point() {
        let mut g = game();
        g.players[0].deck.clear();
        g.players[0].discard = vec![1, 2, 3];
        g.players[1].score = 7;
        let hand = g.players[0].hand.clone();
        g.draw(0, 1);
        assert_eq!(g.winner, None);
        assert!(g.players[0].discard.is_empty());
        assert_eq!(g.players[0].deck.len(), 2);
        assert_eq!(g.players[0].hand.len(), hand.len() + 1);
        g.after_resolution();
        assert_eq!(g.winner, Some(1));
    }

    #[test]
    fn matriarch_tracks_chosen_trash_object_and_fizzles_after_departure() {
        let mut g = game();
        g.players[0].discard = vec![7, 25, 25];
        g.decision = Some(Decision::ReviveUnit { player: 0 });
        g.apply(Action::PlayFromTrash {
            trash_index: 1,
            lane: None,
            target: None,
        })
        .unwrap();
        assert!(g.players[0].units.is_empty());
        g.remove_from_trash(0, 0);
        assert!(matches!(
            g.chain.last().unwrap().ability,
            Some(ChainAbility::ReviveChosen {
                trash_index: 0,
                card_id: 25
            })
        ));
        g.remove_from_trash(0, 0);
        finish_windows(&mut g);
        assert!(
            g.players[0].units.is_empty(),
            "another identical card is not the chosen object"
        );
        assert_eq!(g.players[0].discard, vec![25]);
    }
    #[test]
    fn shadow_disciple_pays_burn_cost_before_trigger_resolves() {
        let mut g = game();
        let id = g.create_unit(0, 43, false, 0);
        g.players[0].units[0].ready = true;
        let deck = g.players[0].deck.len();
        g.apply(Action::Move {
            unit_id: id,
            lane: Some(0),
        })
        .unwrap();
        assert!(matches!(g.decision, Some(Decision::BurnForMight { .. })));
        g.apply(Action::Choose { accept: true }).unwrap();
        assert_eq!(g.players[0].deck.len(), deck - 1);
        assert_eq!(g.players[0].units[0].temporary_buff, 0);
        assert!(
            matches!(g.chain.last().unwrap().ability,Some(ChainAbility::Triggered(TriggeredAbility::Might{unit_id,..})) if unit_id==id)
        );
        g.apply(Action::Pass).unwrap();
        g.apply(Action::Pass).unwrap();
        assert_eq!(g.players[0].units[0].temporary_buff, 1);
    }
    #[test]
    fn granted_flow_can_pay_repeat_and_banishes_after_both_executions() {
        let mut g = game();
        let victim = unit(&mut g, 1, 5, None, false);
        give_runes(&mut g, 0, Domain::Mind, 12);
        g.players[0].discard = vec![50];
        g.temporary_flow[0] = vec![0];
        let target = Target::Unit {
            player: 1,
            unit_id: victim,
        };
        let card = g.catalog[50];
        g.apply(Action::PlayFromTrashOptions {
            flow: None,
            trash_index: 0,
            lane: None,
            targets: vec![target],
            repeat_targets: vec![target],
            repeat: true,
            accelerate: false,
            bonus: false,
        })
        .unwrap();
        assert_eq!(g.players[0].energy, 12 - card.cost - 1);
        assert_eq!(g.players[0].total_runes(), 12 - card.power_cost - 1);
        assert!(g.chain.last().unwrap().options.repeat && g.chain.last().unwrap().banish_after);
        finish_windows(&mut g);
        assert_eq!(g.unit_ref(target).unwrap().damage, 2);
        assert_eq!(g.players[0].banish, vec![50]);
    }

    #[test]
    fn last_rites_can_accelerate_its_paid_unit_play() {
        let mut g = game();
        give_runes(&mut g, 0, Domain::Fury, 12);
        g.players[0].discard = vec![49];
        g.decision = Some(Decision::LastRites {
            player: 0,
            gear_id: 999,
        });
        let card = g.catalog[49];
        g.apply(Action::PlayFromTrashOptions {
            flow: None,
            trash_index: 0,
            lane: None,
            targets: vec![],
            repeat_targets: vec![],
            repeat: false,
            accelerate: true,
            bonus: false,
        })
        .unwrap();
        assert!(g.players[0].units[0].ready);
        assert_eq!(g.players[0].energy, 12 - card.cost - 1);
        assert_eq!(g.players[0].total_runes(), 12 - card.power_cost - 1);
    }

    #[test]
    fn fizz_may_is_decided_before_responses_and_commits_to_a_legal_play() {
        let mut g = game();
        give_runes(&mut g, 0, Domain::Chaos, 6);
        g.players[0].discard = vec![48];
        g.queue_ability(0, 36, vec![], ChainAbility::CardEffect { entering: None });
        g.after_resolution();
        assert!(g.chain.is_empty());
        assert!(matches!(
            g.decision,
            Some(Decision::OptionalPlay { card_id: 36, .. })
        ));
        let mut declined = g.clone();
        declined.apply(Action::Choose { accept: false }).unwrap();
        assert!(declined.chain.is_empty());
        g.apply(Action::Choose { accept: true }).unwrap();
        assert!(matches!(g.decision, Some(Decision::TrashTarget { .. })));
        g.apply(Action::SelectCards { indices: vec![0] }).unwrap();
        assert!(matches!(
            g.chain.last().unwrap().ability,
            Some(ChainAbility::PlayChosen {
                kind: InstructedPlayKind::Fizz,
                card_id: 48,
                ..
            })
        ));
        assert!(g.decision.is_none());
        g.apply(Action::Pass).unwrap();
        g.apply(Action::Pass).unwrap();
        assert!(matches!(g.decision, Some(Decision::PendingPlay { .. })));
        assert!(!g.can_decline_trash_play());
        assert!(g.apply(Action::Choose { accept: false }).is_err());
        g.apply(Action::FinalizePendingPlay {
            lane: None,
            targets: vec![],
            repeat_targets: vec![],
            repeat: false,
            accelerate: false,
            bonus: false,
        })
        .unwrap();
        assert!(!g.required_trash_play);
    }

    #[test]
    fn smite_replacement_persists_after_guardian_heals_the_unit() {
        let mut g = game();
        let id = g.create_unit(0, 25, false, 0);
        g.gear_play(0, 10, None).unwrap();
        let gear = g.players[0].gear[0].id;
        let target = Target::Unit {
            player: 0,
            unit_id: id,
        };
        g.gear_resolve_equip(0, gear, target);
        g.unit_mut(target).unwrap().damage = 20;
        g.unit_mut(target).unwrap().banish_on_death = true;
        g.after_resolution();
        g.apply(Action::ChooseDeathReplacement {
            unit_id: id,
            gear_id: Some(gear),
        })
        .unwrap();
        assert!(g.unit_ref(target).unwrap().banish_on_death);
        g.unit_mut(target).unwrap().damage = 20;
        g.after_resolution();
        assert!(g.unit_ref(target).is_none());
        assert_eq!(g.players[0].banish, vec![25]);
    }
    #[test]
    fn accepted_replay_ai_can_choose_a_battlefield_destination() {
        let mut game = Game::new(0, 2, 29, Difficulty::Easy);
        game.choose_starting_player(0).unwrap();
        game.start_match().unwrap();
        for step in 0..5000 {
            if game.phase == Phase::Finished {
                return;
            }
            game.step_ai().unwrap_or_else(|error| {
                panic!(
                    "step {step}, turn {}, decision {:?}: {error}",
                    game.turn, game.decision
                )
            });
        }
        panic!("seed 29 did not finish");
    }
}
