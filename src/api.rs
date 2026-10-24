//! Deterministic, view-filtered API for local bot runners.
//!
//! `Session` never lends out its authoritative `Game`. Every mutation is tried
//! on a clone and committed only on success. Suggestions are convenient engine
//! candidates, not an exhaustive action space; submit any structured `Action`.
use crate::activated::{AbilitySource, AbilityTiming, ActivatedRule, activated_rules};
use crate::battlefields::{BATTLEFIELDS, BattlefieldAbility};
use crate::cards::{Domain, cards, presets};
use crate::deck::CustomDeck;
use crate::game::{
    Action, ChainAbility, ChainSource, Decision, Difficulty, FlowSource, Game, InstructedPlayKind,
    Phase, Target, TriggerContext, TriggeredAbility,
};
use crate::legend_catalog::legends;
use crate::legends::{LegendAbility, LegendKind, LegendTrigger};
use crate::payment::{PaymentQuote, PowerPayment};
use serde_json::{Map, Value, json};
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_REQUEST_BYTES: usize = 1_048_576;
const MAX_ACTION_ITEMS: usize = 1024;

#[derive(Clone, Debug)]
pub struct ResetOptions {
    pub decks: [usize; 2],
    pub seed: u64,
    pub difficulty: Difficulty,
    pub first_player: usize,
    pub battlefields: Option<[usize; 2]>,
    pub custom_decks: [Option<CustomDeck>; 2],
}

impl Default for ResetOptions {
    fn default() -> Self {
        Self {
            decks: [0, 1],
            seed: 1,
            difficulty: Difficulty::Tactical,
            first_player: 0,
            battlefields: None,
            custom_decks: [None, None],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiError {
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid_request", message)
    }
    fn illegal(message: impl Into<String>) -> Self {
        Self::new("illegal_action", message)
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for ApiError {}

#[derive(Clone, Debug)]
pub struct ActionSuggestion {
    pub id: String,
    pub action: Action,
}

/// Trusted local match coordinator. A player view is private-information safe;
/// the coordinator itself may request either seat and must isolate untrusted bots.
#[derive(Clone)]
pub struct Session {
    game: Game,
    revision: u64,
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("revision", &self.revision)
            .field("summary", &self.summary())
            .finish_non_exhaustive()
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new(ResetOptions::default()).expect("valid default match")
    }
}

impl Session {
    pub fn new(options: ResetOptions) -> Result<Self, ApiError> {
        Ok(Self {
            game: make_game(options)?,
            revision: 0,
        })
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn require_revision(&self, revision: u64) -> Result<(), ApiError> {
        if revision != self.revision {
            Err(ApiError::new(
                "stale_revision",
                format!("Expected revision {}, received {revision}.", self.revision),
            ))
        } else {
            Ok(())
        }
    }
    fn require_actor(&self, player: usize) -> Result<(), ApiError> {
        valid_player(player)?;
        if self.game.phase != Phase::Playing {
            return Err(ApiError::new(
                "wrong_phase",
                "Start the match before taking game actions.",
            ));
        }
        if self.game.priority() != player {
            return Err(ApiError::new(
                "wrong_player",
                "This seat does not have priority.",
            ));
        }
        Ok(())
    }
    fn commit(&mut self, game: Game) -> Result<(), ApiError> {
        let next = self
            .revision
            .checked_add(1)
            .ok_or_else(|| ApiError::new("revision_exhausted", "Create a new session."))?;
        self.game = game;
        self.revision = next;
        Ok(())
    }
    fn transaction(
        &mut self,
        revision: u64,
        f: impl FnOnce(&mut Game) -> Result<(), String>,
    ) -> Result<(), ApiError> {
        self.require_revision(revision)?;
        let mut candidate = self.game.clone();
        match catch_unwind(AssertUnwindSafe(|| f(&mut candidate))) {
            Ok(Ok(())) => self.commit(candidate),
            Ok(Err(error)) => Err(ApiError::illegal(error)),
            Err(_) => Err(ApiError::new(
                "engine_error",
                "The engine rejected this operation; the live match was preserved.",
            )),
        }
    }

    pub fn reset(&mut self, revision: u64, options: ResetOptions) -> Result<(), ApiError> {
        self.require_revision(revision)?;
        self.commit(make_game(options)?)
    }
    pub fn mulligan(
        &mut self,
        player: usize,
        revision: u64,
        indices: &[usize],
    ) -> Result<(), ApiError> {
        valid_player(player)?;
        self.transaction(revision, |game| game.mulligan_player(player, indices))
    }
    /// Beginning a match treats each unused mulligan as keeping that hand.
    pub fn start(&mut self, revision: u64) -> Result<(), ApiError> {
        self.transaction(revision, Game::start_match)
    }
    pub fn apply_action(
        &mut self,
        player: usize,
        revision: u64,
        action: Action,
    ) -> Result<(), ApiError> {
        self.require_revision(revision)?;
        self.require_actor(player)?;
        // Apply the same structural bounds to Rust callers as to JSON clients.
        let checked = parse_action(&action_to_json(&action))?;
        self.transaction(revision, |game| game.apply(checked))
    }
    pub fn validate_action(
        &self,
        player: usize,
        revision: u64,
        action: Action,
    ) -> Result<(), ApiError> {
        self.require_revision(revision)?;
        self.require_actor(player)?;
        let checked = parse_action(&action_to_json(&action))?;
        let mut candidate = self.game.clone();
        match catch_unwind(AssertUnwindSafe(|| candidate.apply(checked))) {
            Ok(result) => result.map_err(ApiError::illegal),
            Err(_) => Err(ApiError::new(
                "engine_error",
                "The engine rejected this operation; the live match was preserved.",
            )),
        }
    }
    pub fn step_ai(&mut self, player: usize, revision: u64) -> Result<(), ApiError> {
        self.require_revision(revision)?;
        self.require_actor(player)?;
        self.transaction(revision, |game| game.step_ai().map(|_| ()))
    }
    pub fn quote_payment(
        &self,
        player: usize,
        revision: u64,
        action: Action,
    ) -> Result<PaymentQuote, ApiError> {
        self.require_revision(revision)?;
        self.require_actor(player)?;
        let action = parse_action(&action_to_json(&action))?;
        match catch_unwind(AssertUnwindSafe(|| self.game.quote_power_payment(&action))) {
            Ok(result) => result.map_err(ApiError::illegal),
            Err(_) => Err(ApiError::new(
                "engine_error",
                "Could not quote this action; the live match was preserved.",
            )),
        }
    }
    /// A non-exhaustive collection of legal convenience actions. In particular,
    /// explicit multi-target, optional-cost and damage choices remain available
    /// through `apply_action`, even if no suggestion describes them.
    pub fn suggestions(&self, player: usize) -> Result<Vec<ActionSuggestion>, ApiError> {
        valid_player(player)?;
        if self.game.phase != Phase::Playing || self.game.priority() != player {
            return Ok(Vec::new());
        }
        let mut actions = Vec::new();
        for action in self.game.legal_actions() {
            if !actions.contains(&action)
                && self
                    .validate_action(player, self.revision, action.clone())
                    .is_ok()
            {
                actions.push(action);
            }
        }
        Ok(actions
            .into_iter()
            .enumerate()
            .map(|(index, action)| ActionSuggestion {
                id: format!("r{}:p{player}:a{index}", self.revision),
                action,
            })
            .collect())
    }
    pub fn apply_suggestion(
        &mut self,
        player: usize,
        revision: u64,
        id: &str,
    ) -> Result<(), ApiError> {
        self.require_revision(revision)?;
        self.require_actor(player)?;
        let action = self
            .suggestions(player)?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or_else(|| {
                ApiError::new(
                    "unknown_action",
                    "This action ID is not a suggestion for this seat and revision.",
                )
            })?
            .action;
        self.apply_action(player, revision, action)
    }

    fn summary(&self) -> Value {
        json!({"phase": phase_name(self.game.phase), "turn": self.game.turn,
            "active": self.game.active, "priority": self.game.priority(), "winner": self.game.winner})
    }

    pub fn observe(&self, viewer: usize) -> Result<Value, ApiError> {
        valid_player(viewer)?;
        let game = &self.game;
        let players: Vec<_> = game.players.iter().enumerate().map(|(owner, state)| {
            let public_hand = match &game.decision {
                Some(Decision::RevealHand { opponent, cards, .. }) if *opponent == owner => Some(cards.clone()),
                Some(Decision::Sabotage { opponent, .. }) if *opponent == owner => Some(state.hand.clone()),
                _ => None,
            };
            let hand = if owner == viewer { Some(state.hand.clone()) } else { public_hand };
            let units: Vec<_> = state.units.iter().map(|unit| {
                let keywords = game.unit_keywords(unit);
                let tags = if unit.token {
                    crate::rules::token_spec(unit.card_id).and_then(|spec|spec.tag).into_iter().collect::<Vec<_>>()
                } else { game.catalog[unit.card_id].tags.to_vec() };
                json!({
                "id":unit.id, "card_id":unit.card_id, "name":game.unit_name(unit), "token":unit.token,
                "tags":tags,
                "lane":unit.lane, "ready":unit.ready, "might":game.unit_might(unit), "damage":unit.damage,
                "buff":unit.buff, "temporary_buff":unit.temporary_buff, "stunned":unit.stunned,
                "protected":unit.protected, "banish_on_death":unit.banish_on_death,
                "empowered":game.unit_empowered(unit.id),"hidden_play_origin":game.hidden_play_origin(unit.id),"movement_locked":game.movement_locked(owner,unit.id),
                "keywords":{"assault":keywords.assault,"shield":keywords.shield,"tank":keywords.tank,"backline":keywords.backline,
                    "deflect":keywords.deflect,"ganking":keywords.ganking,"ambush":keywords.ambush,
                    "accelerate":keywords.accelerate.map(|cost|json!({"energy":cost.energy,"power":cost.power,"domain":cost.domain.name()}))},
                "damage_priority":game.damage_priority(unit)
            })}).collect();
            let gear: Vec<_> = state.gear.iter().map(|gear| json!({
                "id":gear.id, "card_id":gear.card_id, "name":game.gear_name(gear), "token":gear.token,
                "attached_to":gear.attached_to, "lane":game.gear_location(owner,gear.id),
                "ready":gear.ready, "temporary":gear.temporary,"empowered":game.unit_empowered(gear.id)
            })).collect();
            let hidden: Vec<_> = state.hidden.iter().map(|card| json!({
                "id":card.id, "lane":card.lane, "hidden_turn":card.hidden_turn,
                "card_id":game.hidden_visible_card(viewer, owner, card.id)
            })).collect();
            let legend = &game.legend[owner];
            let definition = game.legend_def(owner);
            let legend_observation = json!({"id":definition.id,"official_id":definition.official_id,
                "name":definition.name,"champion":definition.champion,
                "domains":definition.domains.map(|domain|domain.name()),
                "kind":legend_name(legend.kind),"ready":legend.ready,
                "empowered":legend.empowered,"spell_power":legend.spell_power,
                "assault_grants":legend.assault_grants});
            let activated_observation: Vec<_> = game.activated_options(owner).iter().map(|option|json!({
                "source":ability_source_to_json(option.source),"card_id":option.card_id,
                "ability_index":option.ability_index})).collect();
            json!({"player":owner, "name":state.name, "preset":if game.custom_deck(owner).is_none() {Some(game.selected_decks[owner])} else {None},
                "legend_id":game.legend_id(owner), "custom":game.custom_deck(owner).is_some(),
                "custom_deck":if owner==viewer {game.custom_deck(owner).map(CustomDeck::to_json)} else {None},
                "hand":hand, "hand_count":state.hand.len(), "deck_count":state.deck.len(),
                "rune_deck_count":state.rune_deck.len(), "champion":state.champion,
                "discard":state.discard, "banish":state.banish, "score":state.score,
                "energy":state.energy, "energy_pool":state.energy_pool, "runes":state.runes,
                "exhausted_runes":state.exhausted_runes, "power_pool":state.power_pool,
                "any_power":state.any_power, "xp":state.xp, "units":units, "gear":gear, "hidden":hidden,
                "activated_abilities":activated_observation,
                "cards_played_this_turn":game.cards_played_this_turn(owner),"cannot_play_cards":game.cannot_play_cards(owner),
                "cards_completed_this_turn":game.trigger_state.completed_cards[owner],
                "cards_drawn_this_turn":game.trigger_state.draws[owner],
                "optional_power_discount":game.optional_power_discount[owner],
                "granted_flow_indices":game.granted_flow_indices(owner),
                "flow_costs":state.discard.iter().enumerate().map(|(index,_)| json!({"trash_index":index,
                    "costs":game.flow_costs(owner,index).iter().map(|flow|json!({
                        "source":match flow.source {FlowSource::Printed=>"printed",FlowSource::Granted=>"granted"},
                        "energy":flow.energy,"power":flow.power,"colored":flow.colored})).collect::<Vec<_>>()
                })).filter(|row|!row["costs"].as_array().unwrap().is_empty()).collect::<Vec<_>>(),"queued_extra_turns":game.queued_extra_turns(owner),
                "submitted_combat_damage":game.submitted_combat_damage(owner),
                "legend":legend_observation})
        }).collect();
        let chain: Vec<_> = game.chain.iter().map(|item| json!({
            "id":item.id,"player":item.player,"card_id":item.card_id,"name":game.chain_name(item),
            "source":chain_source_to_json(game.chain_source(item)),
            "ability":item.ability.as_ref().map(ability_to_json),"targets":item.targets.iter().map(target_to_json).collect::<Vec<_>>(),
            "lane":item.lane,"from_hidden":item.from_hidden,"recycle_after":item.recycle_after,
            "banish_after":item.banish_after,"repeat":item.options.repeat,"accelerate":item.options.accelerate,
            "bonus":item.options.bonus,"repeat_targets":item.repeat_targets.iter().map(target_to_json).collect::<Vec<_>>()
        })).collect();
        Ok(
            json!({"protocol":PROTOCOL_VERSION,"revision":self.revision,"viewer":viewer,
            "phase":phase_name(game.phase),"turn":game.turn,"active":game.active,
            "first_player":game.first_player,"priority":game.priority(),"winner":game.winner,
            "mulligans_used":game.mulligans_used,"players":players,"battlefields":game.battlefields,
            "battlefield_owner":game.battlefield_owner,"scored_this_turn":game.player_scored_this_turn,
            "chain":chain,"decision":game.decision.as_ref().map(|d|decision_to_json(game,d,viewer)),
            "pending_card_plays":game.pending_cards().iter().map(|card|json!({"id":card.id,"player":card.player,"card_id":card.card_id,"kind":instructed_kind(card.kind)})).collect::<Vec<_>>(),
            "pending_token_plays":game.pending_tokens().iter().map(|token|json!({
                "id":token.id,"player":token.player,"card_id":token.card_id,"might":token.might,"gear":token.gear,
                "destination":match token.location {
                    None => json!({"mode":"choose"}),
                    Some(lane) => json!({"mode":"fixed","lane":lane}),
                }
            })).collect::<Vec<_>>(),
            "showdown":game.showdown.as_ref().map(|w|json!({"lane":w.lane,"attacker":w.attacker,
                "focus":w.focus,"passes":w.passes,"combat":w.combat})),
            "resource_payment_pending":game.resource_payment_pending(),
            "required_trash_play":game.required_trash_play,
            "can_decline_trash_play":game.can_decline_trash_play()}),
        )
    }

    /// One request produces one response. Parse errors also preserve revision.
    pub fn process_line(&mut self, line: &str) -> Value {
        if line.len() > MAX_REQUEST_BYTES {
            return self.error_response(
                ApiError::new("request_too_large", "Maximum request size is 1 MiB."),
                None,
            );
        }
        match serde_json::from_str(line) {
            Ok(request) => self.handle(request),
            Err(_) => self.error_response(
                ApiError::new("invalid_json", "Expected one JSON object per line."),
                None,
            ),
        }
    }
    pub fn handle(&mut self, request: Value) -> Value {
        let request_id = request.get("id").cloned();
        match self.handle_inner(&request) {
            Ok(result) => envelope(true, self.revision, "result", result, request_id),
            Err(error) => self.error_response(error, request_id),
        }
    }
    fn error_response(&self, error: ApiError, id: Option<Value>) -> Value {
        envelope(
            false,
            self.revision,
            "error",
            json!({"code":error.code,"message":error.message}),
            id,
        )
    }
    fn handle_inner(&mut self, request: &Value) -> Result<Value, ApiError> {
        let object = object(request)?;
        let command = string(object, "command")?;
        match command {
            "info" => {
                keys(object, &["command", "id"])?;
                Ok(
                    json!({"protocol":PROTOCOL_VERSION,"name":"Local Rift","summary":self.summary(),
                    "commands":["info","catalog","validate_deck","reset","observe","suggest","quote_payment","validate","apply","mulligan","start","step_ai"],
                    "suggestions_exhaustive":false,"max_request_bytes":MAX_REQUEST_BYTES}),
                )
            }
            "catalog" => {
                keys(object, &["command", "id"])?;
                Ok(catalog())
            }
            "validate_deck" => {
                keys(object, &["command", "id", "deck"])?;
                let deck = CustomDeck::from_json(
                    object
                        .get("deck")
                        .ok_or_else(|| ApiError::invalid("Provide a deck."))?,
                )
                .map_err(ApiError::invalid)?;
                Ok(json!({"legal":true,"card_count":deck.card_count(),"deck":deck.to_json()}))
            }
            "observe" => {
                keys(object, &["command", "id", "player"])?;
                self.observe(player(object)?)
            }
            "suggest" => {
                keys(object, &["command", "id", "player"])?;
                let suggestions = self.suggestions(player(object)?)?;
                Ok(
                    json!({"exhaustive":false,"actions":suggestions.iter().map(|a|json!({"id":a.id,"action":action_to_json(&a.action)})).collect::<Vec<_>>()}),
                )
            }
            "reset" => {
                keys(
                    object,
                    &[
                        "command",
                        "id",
                        "revision",
                        "seed",
                        "decks",
                        "difficulty",
                        "first_player",
                        "battlefields",
                        "custom_decks",
                    ],
                )?;
                let defaults = ResetOptions::default();
                let options = ResetOptions {
                    seed: optional_unsigned(object, "seed")?.unwrap_or(defaults.seed),
                    decks: optional_pair(object, "decks")?.unwrap_or(defaults.decks),
                    difficulty: match optional_string(object, "difficulty")?.unwrap_or("tactical") {
                        "easy" => Difficulty::Easy,
                        "tactical" => Difficulty::Tactical,
                        "strategic" => Difficulty::Strategic,
                        _ => {
                            return Err(ApiError::invalid(
                                "difficulty must be easy, tactical, or strategic.",
                            ));
                        }
                    },
                    first_player: optional_unsigned(object, "first_player")?
                        .map(to_usize)
                        .transpose()?
                        .unwrap_or(0),
                    battlefields: optional_pair(object, "battlefields")?,
                    custom_decks: match object.get("custom_decks") {
                        None => [None, None],
                        Some(value) => {
                            let values = value
                                .as_array()
                                .filter(|values| values.len() == 2)
                                .ok_or_else(|| {
                                    ApiError::invalid(
                                        "custom_decks must contain two decks or nulls.",
                                    )
                                })?;
                            let parse = |value: &Value| {
                                if value.is_null() {
                                    Ok(None)
                                } else {
                                    CustomDeck::from_json(value)
                                        .map(Some)
                                        .map_err(ApiError::invalid)
                                }
                            };
                            [parse(&values[0])?, parse(&values[1])?]
                        }
                    },
                };
                self.reset(unsigned(object, "revision")?, options)?;
                Ok(self.summary())
            }
            "mulligan" => {
                keys(object, &["command", "id", "revision", "player", "indices"])?;
                self.mulligan(
                    player(object)?,
                    unsigned(object, "revision")?,
                    &usize_list(object, "indices")?,
                )?;
                Ok(self.summary())
            }
            "start" => {
                keys(object, &["command", "id", "revision"])?;
                self.start(unsigned(object, "revision")?)?;
                Ok(self.summary())
            }
            "step_ai" => {
                keys(object, &["command", "id", "revision", "player"])?;
                self.step_ai(player(object)?, unsigned(object, "revision")?)?;
                Ok(self.summary())
            }
            "apply" | "validate" | "quote_payment" => {
                keys(
                    object,
                    &["command", "id", "revision", "player", "action", "action_id"],
                )?;
                let player = player(object)?;
                let revision = unsigned(object, "revision")?;
                self.require_revision(revision)?;
                self.require_actor(player)?;
                if object.contains_key("action") == object.contains_key("action_id") {
                    return Err(ApiError::invalid(
                        "Provide exactly one of action or action_id.",
                    ));
                }
                let action = if let Some(value) = object.get("action") {
                    parse_action(value)?
                } else {
                    let id = string(object, "action_id")?;
                    self.suggestions(player)?
                        .into_iter()
                        .find(|a| a.id == id)
                        .ok_or_else(|| {
                            ApiError::new(
                                "unknown_action",
                                "No matching action ID for this seat and revision.",
                            )
                        })?
                        .action
                };
                if command == "quote_payment" {
                    let quote = self.quote_payment(player, revision, action)?;
                    Ok(json!({"suggested":payment_to_json(&quote.suggested),
                        "available":payment_to_json(&quote.available),
                        "domain_order":["fury","calm","mind","body","chaos","order"],
                        "costs":quote.costs.iter().map(|cost| json!({"amount":cost.amount,
                            "domains":cost.domains.iter().map(|d|d.name().to_lowercase()).collect::<Vec<_>>(),
                            "spell_eligible":cost.spell_eligible})).collect::<Vec<_>>() }))
                } else if command == "validate" {
                    self.validate_action(player, revision, action)?;
                    Ok(json!({"legal":true}))
                } else {
                    self.apply_action(player, revision, action)?;
                    Ok(self.summary())
                }
            }
            _ => Err(ApiError::new(
                "unknown_command",
                "Unknown command; use info for the supported commands.",
            )),
        }
    }
}

fn make_game(options: ResetOptions) -> Result<Game, ApiError> {
    valid_player(options.first_player)?;
    if options.decks.iter().any(|&id| id >= presets().len()) {
        return Err(ApiError::invalid("Deck index is outside the catalog."));
    }
    if options
        .battlefields
        .is_some_and(|ids| ids.iter().any(|&id| id >= BATTLEFIELDS.len()))
    {
        return Err(ApiError::invalid(
            "Battlefield index is outside the catalog.",
        ));
    }
    let mut game = Game::new_with_custom(
        options.decks,
        options.custom_decks,
        options.seed,
        options.difficulty,
        false,
    )
    .map_err(ApiError::invalid)?;
    game.choose_starting_player(options.first_player)
        .map_err(ApiError::illegal)?;
    if let Some(fields) = options.battlefields {
        for (player, id) in fields.into_iter().enumerate() {
            game.choose_battlefield(player, id)
                .map_err(ApiError::illegal)?;
        }
    }
    Ok(game)
}

fn envelope(ok: bool, revision: u64, field: &str, payload: Value, id: Option<Value>) -> Value {
    let mut value = json!({"ok":ok,"revision":revision});
    value[field] = payload;
    if let Some(id) = id {
        value["id"] = id;
    }
    value
}
fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Mulligan => "mulligan",
        Phase::Playing => "playing",
        Phase::Finished => "finished",
    }
}
fn legend_name(kind: LegendKind) -> &'static str {
    match kind {
        LegendKind::Irelia => "irelia",
        LegendKind::MasterYi => "master_yi",
        LegendKind::Kennen => "kennen",
        LegendKind::KaiSa => "kaisa",
        LegendKind::Darius => "darius",
        LegendKind::Viktor => "viktor",
        LegendKind::Lux => "lux",
    }
}

fn activated_rule_to_json(index: usize, rule: &ActivatedRule) -> Value {
    use crate::rules::Allegiance;
    json!({"ability_index":index,"label":rule.label,
        "timing":match rule.timing {AbilityTiming::Main=>"main",AbilityTiming::Action=>"action",AbilityTiming::Reaction=>"reaction"},
        "energy":rule.energy,"power":rule.power,"power_domain":rule.domain.map(|domain|domain.name()),
        "exhaust":rule.exhaust,"requires_battlefield":rule.requires_battlefield,
        "adds_resources":rule.adds_resources(),
        "targets":{"min":rule.targets.min,"max":rule.targets.max,
            "allegiance":match rule.targets.allegiance {Allegiance::None=>"none",Allegiance::Any=>"any",
                Allegiance::Friendly=>"friendly",Allegiance::Enemy=>"enemy",Allegiance::Mixed=>"mixed"},
            "battlefield":rule.targets.battlefield,"exhausted":rule.targets.exhausted}})
}

pub fn catalog() -> Value {
    json!({"as_of":crate::cards::DATA_AS_OF,
        "domains":["Fury","Calm","Mind","Body","Chaos","Order"],
        "cards":cards().iter().map(|c|json!({"id":c.id,"official_id":c.official_id,"name":c.name,"energy":c.cost,"power":c.power_cost,
            "might":c.might,"domain":c.domain.name(),"domains":c.domains.iter().map(|d|d.name()).collect::<Vec<_>>(),
            "kind":c.kind.name(),"official_text":c.official_text,"keywords":c.keywords,"tags":c.tags,
            "activated_abilities":activated_rules(c.id).iter().enumerate().map(|(index,rule)|activated_rule_to_json(index,rule)).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "legends":legends().iter().map(|definition|json!({"id":definition.id,
            "official_id":definition.official_id,"name":definition.name,"champion":definition.champion,
            "domains":definition.domains.map(|domain|domain.name()),"official_text":definition.text,
            "default_champion":definition.default_champion,
            "eligible_cards":crate::deck::eligible_cards(definition.id),
            "eligible_champions":crate::deck::eligible_champions(definition.id),
            "training_deck":CustomDeck::from_legend(definition.id).ok().map(|deck|deck.to_json()),
            "training_note":if presets().iter().any(|preset|preset.legend_id==definition.id) {
                "Editable copy of the sourced preset; the original preset remains unchanged."
            } else { "Authored practice list; not a tournament or meta recommendation." }
        })).collect::<Vec<_>>(),
        "presets":presets().iter().enumerate().map(|(id,p)|json!({"id":id,"name":p.name,"legend_id":p.legend_id,
            "legend_name":p.legend_name,"legend_text":p.legend_text,"chosen_champion":p.chosen_champion,
            "eligible_cards":crate::deck::eligible_cards(p.legend_id),"eligible_champions":crate::deck::eligible_champions(p.legend_id),
            "domains":p.domains.map(|d|d.name()),"rune_counts":p.rune_counts,"cards":p.cards,
            "strategy":p.strategy,"learning_notes":p.learning_notes,"source_url":p.source_url})).collect::<Vec<_>>(),
        "battlefields":BATTLEFIELDS.iter().map(|b|json!({"id":b.id,"name":b.name,"official_text":b.official_text})).collect::<Vec<_>>()})
}

pub fn ability_source_to_json(source: AbilitySource) -> Value {
    match source {
        AbilitySource::Unit { player, unit_id } => {
            json!({"type":"unit","player":player,"unit_id":unit_id})
        }
        AbilitySource::Gear { player, gear_id } => {
            json!({"type":"gear","player":player,"gear_id":gear_id})
        }
    }
}

fn chain_source_to_json(source: ChainSource) -> Value {
    match source {
        ChainSource::Card { card_id } => json!({"type":"card","card_id":card_id}),
        ChainSource::Legend { player, legend_id } => {
            json!({"type":"legend","player":player,"legend_id":legend_id})
        }
        ChainSource::Battlefield {
            lane,
            battlefield_id,
        } => json!({"type":"battlefield","lane":lane,"battlefield_id":battlefield_id}),
    }
}

pub fn target_to_json(target: &Target) -> Value {
    match target {
        Target::Unit { player, unit_id } => {
            json!({"type":"unit","player":player,"unit_id":unit_id})
        }
        Target::Gear { player, gear_id } => {
            json!({"type":"gear","player":player,"gear_id":gear_id})
        }
        Target::Spell { chain_id } => json!({"type":"spell","chain_id":chain_id}),
    }
}

pub fn action_to_json(action: &Action) -> Value {
    let ts = |targets: &[Target]| targets.iter().map(target_to_json).collect::<Vec<_>>();
    let t = |target: &Option<Target>| target.as_ref().map(target_to_json);
    match action {
        Action::RecycleRune { domain, exhausted } => {
            json!({"type":"recycle_rune", "domain":domain.name().to_lowercase(), "exhausted":exhausted})
        }
        Action::WithPowerPayment { action, payment } => json!({"type":"with_power_payment",
            "action":if matches!(**action, Action::WithPowerPayment {..}) {json!({"type":"with_power_payment"})} else {action_to_json(action)},
            "payment":payment_to_json(payment)}),
        Action::PlaceToken { lane } => json!({"type":"place_token","lane":lane}),
        Action::ChooseBattlefield { lane } => json!({"type":"choose_battlefield","lane":lane}),
        Action::ChooseResolutionGroup { index } => {
            json!({"type":"choose_resolution_group","index":index})
        }
        Action::SetOptionalDiscount { power } => {
            json!({"type":"set_optional_discount","power":power})
        }
        Action::OrderTrigger { trigger_id } => {
            json!({"type":"order_trigger","trigger_id":trigger_id})
        }
        Action::PlayCard {
            hand_index,
            lane,
            target,
        } => json!({"type":"play_card","hand_index":hand_index,"lane":lane,"target":t(target)}),
        Action::PlayChampion { lane, target } => {
            json!({"type":"play_champion","lane":lane,"target":t(target)})
        }
        Action::PlayCardWithTargets {
            hand_index,
            lane,
            targets,
        } => {
            json!({"type":"play_card_targets","hand_index":hand_index,"lane":lane,"targets":ts(targets)})
        }
        Action::Move { unit_id, lane } => json!({"type":"move","unit_id":unit_id,"lane":lane}),
        Action::MoveGroup { unit_ids, lane } => {
            json!({"type":"move_group","unit_ids":unit_ids,"lane":lane})
        }
        Action::Attack { lane } => json!({"type":"attack","lane":lane}),
        Action::Pass => json!({"type":"pass"}),
        Action::Choose { accept } => json!({"type":"choose","accept":accept}),
        Action::Equip { gear_id, target } => {
            json!({"type":"equip","gear_id":gear_id,"target":target_to_json(target)})
        }
        Action::EquipWithTrash {
            gear_id,
            target,
            trash_indices,
        } => {
            json!({"type":"equip_with_trash","gear_id":gear_id,"target":target_to_json(target),"trash_indices":trash_indices})
        }
        Action::ActivateGear { gear_id } => json!({"type":"activate_gear","gear_id":gear_id}),
        Action::ActivateAbility {
            source,
            ability_index,
            targets,
        } => json!({
            "type":"activate_ability","source":ability_source_to_json(*source),
            "ability_index":ability_index,"targets":ts(targets)}),
        Action::ActivateLegend { target } => json!({"type":"activate_legend","target":t(target)}),
        Action::ActivateRune { domain, recycle } => {
            json!({"type":"activate_rune","domain":domain.name(),"recycle":recycle})
        }
        Action::SelectCards { indices } => json!({"type":"select_cards","indices":indices}),
        Action::PlayFromTrash {
            trash_index,
            lane,
            target,
        } => {
            json!({"type":"play_from_trash","trash_index":trash_index,"lane":lane,"target":t(target)})
        }
        Action::PlayFromTrashWithTargets {
            trash_index,
            lane,
            targets,
        } => {
            json!({"type":"play_from_trash_targets","trash_index":trash_index,"lane":lane,"targets":ts(targets)})
        }
        Action::FinalizePendingPlay {
            lane,
            targets,
            repeat_targets,
            repeat,
            accelerate,
            bonus,
        } => {
            json!({"type":"finalize_pending_play","lane":lane,"targets":ts(targets),"repeat_targets":ts(repeat_targets),"repeat":repeat,"accelerate":accelerate,"bonus":bonus})
        }
        Action::PlayFromTrashOptions {
            trash_index,
            lane,
            targets,
            repeat_targets,
            repeat,
            accelerate,
            bonus,
            flow,
        } => {
            json!({"type":"play_from_trash_options","trash_index":trash_index,"lane":lane,"targets":ts(targets),"repeat_targets":ts(repeat_targets),"repeat":repeat,"accelerate":accelerate,"bonus":bonus,"flow":flow.map(|source| match source {FlowSource::Printed=>"printed",FlowSource::Granted=>"granted"})})
        }
        Action::Hide { hand_index, lane } => {
            json!({"type":"hide","hand_index":hand_index,"lane":lane})
        }
        Action::HideChampion { lane } => json!({"type":"hide_champion","lane":lane}),
        Action::PlayHidden { hidden_id, targets } => {
            json!({"type":"play_hidden","hidden_id":hidden_id,"targets":ts(targets)})
        }
        Action::PlayCardOptions {
            hand_index,
            lane,
            targets,
            repeat_targets,
            repeat,
            accelerate,
            bonus,
        } => {
            json!({"type":"play_card_options","hand_index":hand_index,"lane":lane,"targets":ts(targets),"repeat_targets":ts(repeat_targets),"repeat":repeat,"accelerate":accelerate,"bonus":bonus})
        }
        Action::PlayChampionOptions {
            lane,
            target,
            accelerate,
        } => {
            json!({"type":"play_champion_options","lane":lane,"target":t(target),"accelerate":accelerate})
        }
        Action::EmpowerUnit { unit_id, pay_power } => {
            json!({"type":"empower_unit","unit_id":unit_id,"pay_power":pay_power})
        }
        Action::ChooseTarget { target } => json!({"type":"choose_target","target":t(target)}),
        Action::ChooseTriggerTargets { targets } => {
            json!({"type":"choose_trigger_targets","targets":ts(targets)})
        }
        Action::ChooseDeathReplacement { unit_id, gear_id } => {
            json!({"type":"choose_death_replacement","unit_id":unit_id,"gear_id":gear_id})
        }
        Action::AssignDamage { assignments } => {
            json!({"type":"assign_damage","assignments":assignments})
        }
        Action::EndTurn => json!({"type":"end_turn"}),
    }
}

fn object(value: &Value) -> Result<&Map<String, Value>, ApiError> {
    value
        .as_object()
        .ok_or_else(|| ApiError::invalid("Expected an object."))
}
fn keys(obj: &Map<String, Value>, allowed: &[&str]) -> Result<(), ApiError> {
    if let Some(key) = obj.keys().find(|key| !allowed.contains(&key.as_str())) {
        Err(ApiError::invalid(format!("Unknown field: {key}")))
    } else {
        Ok(())
    }
}
fn required<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<&'a Value, ApiError> {
    obj.get(key)
        .ok_or_else(|| ApiError::invalid(format!("Missing {key}.")))
}
fn string<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<&'a str, ApiError> {
    required(obj, key)?
        .as_str()
        .ok_or_else(|| ApiError::invalid(format!("{key} must be a string.")))
}
fn optional_string<'a>(
    obj: &'a Map<String, Value>,
    key: &str,
) -> Result<Option<&'a str>, ApiError> {
    obj.get(key)
        .map(|v| {
            v.as_str()
                .ok_or_else(|| ApiError::invalid(format!("{key} must be a string.")))
        })
        .transpose()
}
fn unsigned(obj: &Map<String, Value>, key: &str) -> Result<u64, ApiError> {
    required(obj, key)?
        .as_u64()
        .ok_or_else(|| ApiError::invalid(format!("{key} must be a nonnegative integer.")))
}
fn optional_unsigned(obj: &Map<String, Value>, key: &str) -> Result<Option<u64>, ApiError> {
    obj.get(key)
        .map(|v| {
            v.as_u64()
                .ok_or_else(|| ApiError::invalid(format!("{key} must be a nonnegative integer.")))
        })
        .transpose()
}
fn boolean(obj: &Map<String, Value>, key: &str) -> Result<bool, ApiError> {
    required(obj, key)?
        .as_bool()
        .ok_or_else(|| ApiError::invalid(format!("{key} must be true or false.")))
}
fn flag(obj: &Map<String, Value>, key: &str) -> Result<bool, ApiError> {
    if obj.contains_key(key) {
        boolean(obj, key)
    } else {
        Ok(false)
    }
}
fn to_usize(value: u64) -> Result<usize, ApiError> {
    usize::try_from(value)
        .map_err(|_| ApiError::invalid("Integer exceeds this platform's index range."))
}
fn index(obj: &Map<String, Value>, key: &str) -> Result<usize, ApiError> {
    to_usize(unsigned(obj, key)?)
}
fn id32(obj: &Map<String, Value>, key: &str) -> Result<u32, ApiError> {
    u32::try_from(unsigned(obj, key)?).map_err(|_| ApiError::invalid(format!("{key} exceeds u32.")))
}
fn valid_player(player: usize) -> Result<(), ApiError> {
    if player > 1 {
        Err(ApiError::invalid("player must be 0 or 1."))
    } else {
        Ok(())
    }
}
fn player(obj: &Map<String, Value>) -> Result<usize, ApiError> {
    let p = index(obj, "player")?;
    valid_player(p)?;
    Ok(p)
}
fn field(obj: &Map<String, Value>, key: &str) -> Result<usize, ApiError> {
    let value = index(obj, key)?;
    if value >= 2 {
        Err(ApiError::invalid(format!(
            "{key} must be battlefield slot 0 or 1."
        )))
    } else {
        Ok(value)
    }
}
fn lane(obj: &Map<String, Value>) -> Result<Option<usize>, ApiError> {
    if obj.get("lane").is_none_or(Value::is_null) {
        Ok(None)
    } else {
        field(obj, "lane").map(Some)
    }
}
fn array<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<&'a Vec<Value>, ApiError> {
    let values = required(obj, key)?
        .as_array()
        .ok_or_else(|| ApiError::invalid(format!("{key} must be an array.")))?;
    if values.len() > MAX_ACTION_ITEMS {
        return Err(ApiError::invalid("Action array exceeds 1024 entries."));
    }
    Ok(values)
}
fn usize_list(obj: &Map<String, Value>, key: &str) -> Result<Vec<usize>, ApiError> {
    array(obj, key)?
        .iter()
        .map(|v| {
            v.as_u64()
                .ok_or_else(|| {
                    ApiError::invalid(format!("{key} entries must be nonnegative integers."))
                })
                .and_then(to_usize)
        })
        .collect()
}
fn optional_pair(obj: &Map<String, Value>, key: &str) -> Result<Option<[usize; 2]>, ApiError> {
    if !obj.contains_key(key) {
        return Ok(None);
    }
    let values = usize_list(obj, key)?;
    Ok(Some(values.try_into().map_err(|_| {
        ApiError::invalid(format!("{key} must contain two indices."))
    })?))
}
fn parse_domain(value: &str) -> Result<Domain, ApiError> {
    match value.to_ascii_lowercase().as_str() {
        "fury" => Ok(Domain::Fury),
        "calm" => Ok(Domain::Calm),
        "mind" => Ok(Domain::Mind),
        "body" => Ok(Domain::Body),
        "chaos" => Ok(Domain::Chaos),
        "order" => Ok(Domain::Order),
        _ => Err(ApiError::invalid("Unknown domain.")),
    }
}
pub fn parse_ability_source(value: &Value) -> Result<AbilitySource, ApiError> {
    let obj = object(value)?;
    match string(obj, "type")? {
        "unit" => {
            keys(obj, &["type", "player", "unit_id"])?;
            Ok(AbilitySource::Unit {
                player: player(obj)?,
                unit_id: id32(obj, "unit_id")?,
            })
        }
        "gear" => {
            keys(obj, &["type", "player", "gear_id"])?;
            Ok(AbilitySource::Gear {
                player: player(obj)?,
                gear_id: id32(obj, "gear_id")?,
            })
        }
        _ => Err(ApiError::invalid(
            "Ability source must be a unit or gear object.",
        )),
    }
}

pub fn parse_target(value: &Value) -> Result<Target, ApiError> {
    let obj = object(value)?;
    match string(obj, "type")? {
        "unit" => {
            keys(obj, &["type", "player", "unit_id"])?;
            Ok(Target::Unit {
                player: player(obj)?,
                unit_id: id32(obj, "unit_id")?,
            })
        }
        "gear" => {
            keys(obj, &["type", "player", "gear_id"])?;
            Ok(Target::Gear {
                player: player(obj)?,
                gear_id: id32(obj, "gear_id")?,
            })
        }
        "spell" => {
            keys(obj, &["type", "chain_id"])?;
            Ok(Target::Spell {
                chain_id: id32(obj, "chain_id")?,
            })
        }
        _ => Err(ApiError::invalid(
            "Target type must be unit, gear, or spell.",
        )),
    }
}
fn target(obj: &Map<String, Value>) -> Result<Option<Target>, ApiError> {
    obj.get("target")
        .filter(|v| !v.is_null())
        .map(parse_target)
        .transpose()
}
fn targets(obj: &Map<String, Value>, key: &str) -> Result<Vec<Target>, ApiError> {
    if obj.contains_key(key) {
        array(obj, key)?.iter().map(parse_target).collect()
    } else {
        Ok(Vec::new())
    }
}

fn payment_to_json(payment: &PowerPayment) -> Value {
    json!({"colored":payment.colored,"any":payment.any,"spell":payment.spell,"recycle":payment.recycle})
}

fn parse_payment(value: &Value) -> Result<PowerPayment, ApiError> {
    let o = object(value)?;
    keys(o, &["colored", "any", "spell", "recycle"])?;
    let amounts = |key| -> Result<[u8; 6], ApiError> {
        let values = usize_list(o, key)?;
        let array: [usize; 6] = values
            .try_into()
            .map_err(|_| ApiError::invalid("Power arrays need exactly six domain counts."))?;
        let mut out = [0; 6];
        for (dest, amount) in out.iter_mut().zip(array) {
            *dest = u8::try_from(amount)
                .map_err(|_| ApiError::invalid("Power counts must be between 0 and 255."))?;
        }
        Ok(out)
    };
    Ok(PowerPayment {
        colored: amounts("colored")?,
        recycle: amounts("recycle")?,
        any: u8::try_from(unsigned(o, "any")?)
            .map_err(|_| ApiError::invalid("any exceeds 255."))?,
        spell: u8::try_from(unsigned(o, "spell")?)
            .map_err(|_| ApiError::invalid("spell exceeds 255."))?,
    })
}

pub fn parse_action(value: &Value) -> Result<Action, ApiError> {
    let o = object(value)?;
    let kind = string(o, "type")?;
    macro_rules! fields {($($name:literal),* $(,)?)=>{keys(o,&["type",$($name),*])?};}
    Ok(match kind {
        "recycle_rune" => {
            fields!("domain", "exhausted");
            Action::RecycleRune {
                domain: parse_domain(string(o, "domain")?)?,
                exhausted: o
                    .get("exhausted")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| ApiError::invalid("exhausted must specify true or false."))?,
            }
        }
        "with_power_payment" => {
            fields!("action", "payment");
            let inner = o
                .get("action")
                .ok_or_else(|| ApiError::invalid("Provide the action to pay for."))?;
            if inner.get("type").and_then(Value::as_str) == Some("with_power_payment") {
                return Err(ApiError::invalid("Payment choices cannot be nested."));
            }
            Action::WithPowerPayment {
                action: Box::new(parse_action(inner)?),
                payment: parse_payment(
                    o.get("payment")
                        .ok_or_else(|| ApiError::invalid("Provide a payment choice."))?,
                )?,
            }
        }
        "place_token" => {
            fields!("lane");
            Action::PlaceToken { lane: lane(o)? }
        }
        "choose_battlefield" => {
            fields!("lane");
            Action::ChooseBattlefield {
                lane: index(o, "lane")?,
            }
        }
        "choose_resolution_group" => {
            fields!("index");
            Action::ChooseResolutionGroup {
                index: index(o, "index")?,
            }
        }
        "set_optional_discount" => {
            fields!("power");
            Action::SetOptionalDiscount {
                power: u8::try_from(unsigned(o, "power")?)
                    .map_err(|_| ApiError::invalid("power exceeds u8."))?,
            }
        }
        "order_trigger" => {
            fields!("trigger_id");
            Action::OrderTrigger {
                trigger_id: id32(o, "trigger_id")?,
            }
        }
        "play_card" => {
            fields!("hand_index", "lane", "target");
            Action::PlayCard {
                hand_index: index(o, "hand_index")?,
                lane: lane(o)?,
                target: target(o)?,
            }
        }
        "play_champion" => {
            fields!("lane", "target");
            Action::PlayChampion {
                lane: lane(o)?,
                target: target(o)?,
            }
        }
        "play_card_targets" => {
            fields!("hand_index", "lane", "targets");
            Action::PlayCardWithTargets {
                hand_index: index(o, "hand_index")?,
                lane: lane(o)?,
                targets: targets(o, "targets")?,
            }
        }
        "move" => {
            fields!("unit_id", "lane");
            Action::Move {
                unit_id: id32(o, "unit_id")?,
                lane: lane(o)?,
            }
        }
        "move_group" => {
            fields!("unit_ids", "lane");
            Action::MoveGroup {
                unit_ids: usize_list(o, "unit_ids")?
                    .into_iter()
                    .map(|v| {
                        u32::try_from(v).map_err(|_| ApiError::invalid("Unit ID exceeds u32."))
                    })
                    .collect::<Result<_, _>>()?,
                lane: lane(o)?,
            }
        }
        "attack" => {
            fields!("lane");
            Action::Attack {
                lane: field(o, "lane")?,
            }
        }
        "pass" => {
            fields!();
            Action::Pass
        }
        "choose" => {
            fields!("accept");
            Action::Choose {
                accept: boolean(o, "accept")?,
            }
        }
        "equip" => {
            fields!("gear_id", "target");
            Action::Equip {
                gear_id: id32(o, "gear_id")?,
                target: parse_target(required(o, "target")?)?,
            }
        }
        "equip_with_trash" => {
            fields!("gear_id", "target", "trash_indices");
            Action::EquipWithTrash {
                gear_id: id32(o, "gear_id")?,
                target: parse_target(required(o, "target")?)?,
                trash_indices: usize_list(o, "trash_indices")?,
            }
        }
        "activate_gear" => {
            fields!("gear_id");
            Action::ActivateGear {
                gear_id: id32(o, "gear_id")?,
            }
        }
        "activate_ability" => {
            fields!("source", "ability_index", "targets");
            Action::ActivateAbility {
                source: parse_ability_source(required(o, "source")?)?,
                ability_index: index(o, "ability_index")?,
                targets: targets(o, "targets")?,
            }
        }
        "activate_legend" => {
            fields!("target");
            Action::ActivateLegend { target: target(o)? }
        }
        "activate_rune" => {
            fields!("domain", "recycle");
            Action::ActivateRune {
                domain: parse_domain(string(o, "domain")?)?,
                recycle: boolean(o, "recycle")?,
            }
        }
        "select_cards" => {
            fields!("indices");
            Action::SelectCards {
                indices: usize_list(o, "indices")?,
            }
        }
        "play_from_trash" => {
            fields!("trash_index", "lane", "target");
            Action::PlayFromTrash {
                trash_index: index(o, "trash_index")?,
                lane: lane(o)?,
                target: target(o)?,
            }
        }
        "play_from_trash_targets" => {
            fields!("trash_index", "lane", "targets");
            Action::PlayFromTrashWithTargets {
                trash_index: index(o, "trash_index")?,
                lane: lane(o)?,
                targets: targets(o, "targets")?,
            }
        }
        "finalize_pending_play" => {
            fields!(
                "lane",
                "targets",
                "repeat_targets",
                "repeat",
                "accelerate",
                "bonus"
            );
            Action::FinalizePendingPlay {
                lane: lane(o)?,
                targets: targets(o, "targets")?,
                repeat_targets: targets(o, "repeat_targets")?,
                repeat: flag(o, "repeat")?,
                accelerate: flag(o, "accelerate")?,
                bonus: flag(o, "bonus")?,
            }
        }
        "play_from_trash_options" => {
            fields!(
                "trash_index",
                "lane",
                "targets",
                "repeat_targets",
                "repeat",
                "accelerate",
                "bonus",
                "flow"
            );
            Action::PlayFromTrashOptions {
                trash_index: index(o, "trash_index")?,
                lane: lane(o)?,
                targets: targets(o, "targets")?,
                repeat_targets: targets(o, "repeat_targets")?,
                repeat: flag(o, "repeat")?,
                accelerate: flag(o, "accelerate")?,
                bonus: flag(o, "bonus")?,
                flow: match o.get("flow") {
                    None | Some(Value::Null) => None,
                    Some(Value::String(value)) if value == "printed" => Some(FlowSource::Printed),
                    Some(Value::String(value)) if value == "granted" => Some(FlowSource::Granted),
                    _ => return Err(ApiError::invalid("flow must be printed, granted, or null.")),
                },
            }
        }
        "hide" => {
            fields!("hand_index", "lane");
            Action::Hide {
                hand_index: index(o, "hand_index")?,
                lane: field(o, "lane")?,
            }
        }
        "hide_champion" => {
            fields!("lane");
            Action::HideChampion {
                lane: field(o, "lane")?,
            }
        }
        "play_hidden" => {
            fields!("hidden_id", "targets");
            Action::PlayHidden {
                hidden_id: id32(o, "hidden_id")?,
                targets: targets(o, "targets")?,
            }
        }
        "play_card_options" => {
            fields!(
                "hand_index",
                "lane",
                "targets",
                "repeat_targets",
                "repeat",
                "accelerate",
                "bonus"
            );
            Action::PlayCardOptions {
                hand_index: index(o, "hand_index")?,
                lane: lane(o)?,
                targets: targets(o, "targets")?,
                repeat_targets: targets(o, "repeat_targets")?,
                repeat: flag(o, "repeat")?,
                accelerate: flag(o, "accelerate")?,
                bonus: flag(o, "bonus")?,
            }
        }
        "play_champion_options" => {
            fields!("lane", "target", "accelerate");
            Action::PlayChampionOptions {
                lane: lane(o)?,
                target: target(o)?,
                accelerate: flag(o, "accelerate")?,
            }
        }
        "empower_unit" => {
            fields!("unit_id", "pay_power");
            Action::EmpowerUnit {
                unit_id: id32(o, "unit_id")?,
                pay_power: flag(o, "pay_power")?,
            }
        }
        "choose_target" => {
            fields!("target");
            Action::ChooseTarget { target: target(o)? }
        }
        "choose_trigger_targets" => {
            fields!("targets");
            Action::ChooseTriggerTargets {
                targets: targets(o, "targets")?,
            }
        }
        "choose_death_replacement" => {
            fields!("unit_id", "gear_id");
            Action::ChooseDeathReplacement {
                unit_id: id32(o, "unit_id")?,
                gear_id: if o.get("gear_id").is_none_or(Value::is_null) {
                    None
                } else {
                    Some(id32(o, "gear_id")?)
                },
            }
        }
        "assign_damage" => {
            fields!("assignments");
            let assignments = array(o, "assignments")?
                .iter()
                .map(|v| {
                    let pair = v.as_array().filter(|p| p.len() == 2).ok_or_else(|| {
                        ApiError::invalid("Each assignment must be [unit_id, damage].")
                    })?;
                    let id = pair[0]
                        .as_u64()
                        .and_then(|v| u32::try_from(v).ok())
                        .ok_or_else(|| ApiError::invalid("Invalid assignment unit ID."))?;
                    let amount = pair[1]
                        .as_u64()
                        .and_then(|v| u16::try_from(v).ok())
                        .ok_or_else(|| ApiError::invalid("Invalid damage amount."))?;
                    Ok((id, amount))
                })
                .collect::<Result<_, ApiError>>()?;
            Action::AssignDamage { assignments }
        }
        "end_turn" => {
            fields!();
            Action::EndTurn
        }
        _ => return Err(ApiError::invalid("Unknown action type.")),
    })
}

fn legend_trigger(trigger: LegendTrigger) -> Value {
    match trigger {
        LegendTrigger::IreliaChoose { unit_id } => {
            json!({"type":"irelia_choose","unit_id":unit_id})
        }
        LegendTrigger::IreliaConquer => json!({"type":"irelia_conquer"}),
    }
}

fn instructed_kind(kind: InstructedPlayKind) -> &'static str {
    match kind {
        InstructedPlayKind::Fizz => "fizz",
        InstructedPlayKind::LastRites => "last_rites",
        InstructedPlayKind::Matriarch => "matriarch",
    }
}

fn decision_to_json(game: &Game, decision: &Decision, viewer: usize) -> Value {
    let player = decision.player();
    let mut out = match decision {
        Decision::TriggerTargets {
            card_id,
            rule_index,
            context,
            ..
        } => {
            let (maximum, required) = game.trigger_target_bounds().unwrap_or((0, 0));
            json!({"type":"trigger_targets","card_id":card_id,"rule_index":rule_index,
                "context":trigger_context_to_json(context),"maximum":maximum,"required":required,
                "candidates":game.trigger_target_candidates().iter().map(target_to_json).collect::<Vec<_>>()})
        }
        Decision::TrashTarget {
            source_card_id,
            gear_id,
            ..
        } => json!({"type":"trash_target","source_card_id":source_card_id,"gear_id":gear_id}),
        Decision::PendingPlay {
            pending_id,
            card_id,
            kind,
            ..
        } => {
            json!({"type":"pending_play","pending_id":pending_id,"card_id":card_id,"kind":instructed_kind(*kind)})
        }
        Decision::BattlefieldOrder { lanes, combat, .. } => {
            json!({"type":"battlefield_order","lanes":lanes,"combat":combat})
        }
        Decision::OptionalPlay {
            card_id, gear_id, ..
        } => json!({"type":"optional_play","card_id":card_id,"gear_id":gear_id}),
        Decision::TokenPlacement {
            card_id, remaining, ..
        } => json!({"type":"token_placement","card_id":card_id,"remaining":remaining}),
        Decision::ResolutionTargets {
            card_id,
            groups,
            repeated,
            ..
        } => {
            json!({"type":"resolution_targets","card_id":card_id,"groups":groups.iter().map(|group|group.iter().map(target_to_json).collect::<Vec<_>>()).collect::<Vec<_>>(),"repeated":repeated})
        }
        Decision::OrderTriggers { options, .. } => {
            json!({"type":"order_triggers","options":options.iter().map(|option|json!({"id":option.id,"card_id":option.card_id,"label":option.label})).collect::<Vec<_>>()})
        }
        Decision::OnPlayTarget {
            card_id,
            source_id,
            lane,
            ..
        } => json!({"type":"on_play_target","card_id":card_id,"source_id":source_id,"lane":lane}),
        Decision::DeathReplacement { options, .. } => {
            json!({"type":"death_replacement","options":options.iter().map(|option|json!({"unit_id":option.unit_id,"gear_id":option.gear_id})).collect::<Vec<_>>()})
        }
        Decision::CombatDamage { lane, total, .. } => {
            json!({"type":"combat_damage","lane":lane,"total":total})
        }
        Decision::RevealHand {
            opponent, cards, ..
        } => json!({"type":"reveal_hand","opponent":opponent,"cards":cards}),
        Decision::Battlefield { lane, .. } => json!({"type":"battlefield","lane":lane}),
        Decision::ReviveUnit { .. } => json!({"type":"revive_unit"}),
        Decision::RevivePlay {
            trash_index,
            card_id,
            ..
        } => json!({"type":"revive_play","trash_index":trash_index,"card_id":card_id}),
        Decision::SwapUnit { unit_id, .. } => json!({"type":"swap_unit","unit_id":unit_id}),
        Decision::EmpowerHere {
            lane, source_id, ..
        } => json!({"type":"empower_here","lane":lane,"source_id":source_id}),
        Decision::Legend { trigger, .. } => {
            json!({"type":"legend","trigger":legend_trigger(*trigger)})
        }
        Decision::LastRites { gear_id, .. } => json!({"type":"last_rites","gear_id":gear_id}),
        Decision::ReplaySpell { .. } => json!({"type":"replay_spell"}),
        Decision::GrantFlow { .. } => json!({"type":"grant_flow"}),
        Decision::Discard {
            count, draw_after, ..
        } => json!({"type":"discard","count":count,"draw_after":draw_after}),
        Decision::TopThree { cards, .. } => {
            if player == viewer {
                json!({"type":"top_three","cards":cards})
            } else {
                json!({"type":"top_three"})
            }
        }
        Decision::Sabotage { opponent, .. } => {
            json!({"type":"sabotage","opponent":opponent,"cards":game.players[*opponent].hand})
        }
        Decision::BurnForMight { unit_id, .. } => {
            json!({"type":"burn_for_might","unit_id":unit_id})
        }
        Decision::Predict { card_id, .. } => {
            if player == viewer {
                json!({"type":"predict","card_id":card_id})
            } else {
                json!({"type":"predict"})
            }
        }
        Decision::CounterTax { chain_id, cost, .. } => {
            json!({"type":"counter_tax","chain_id":chain_id,"cost":cost})
        }
    };
    out["player"] = json!(player);
    out
}

fn ability_to_json(ability: &ChainAbility) -> Value {
    match ability {
        ChainAbility::Activated {
            source,
            card_id,
            ability_index,
        } => json!({
            "type":"activated","source":ability_source_to_json(*source),
            "card_id":card_id,"ability_index":ability_index}),
        ChainAbility::CardTrigger {
            rule_index,
            context,
        } => {
            json!({"type":"card_trigger","rule_index":rule_index,"context":trigger_context_to_json(context)})
        }
        ChainAbility::PlayChosen {
            trash_index,
            card_id,
            kind,
        } => {
            json!({"type":"play_chosen","trash_index":if *trash_index==usize::MAX {None} else {Some(*trash_index)},"card_id":card_id,"kind":instructed_kind(*kind)})
        }
        ChainAbility::Disempower { player, object_id } => {
            json!({"type":"disempower","player":player,"object_id":object_id})
        }
        ChainAbility::OptionalPlay { gear_id } => json!({"type":"optional_play","gear_id":gear_id}),
        ChainAbility::GrantFlowChosen {
            trash_index,
            card_id,
        } => {
            json!({"type":"grant_flow_chosen","trash_index":if *trash_index==usize::MAX {None} else {Some(*trash_index)},"card_id":card_id})
        }
        ChainAbility::ReviveChosen {
            trash_index,
            card_id,
        } => {
            json!({"type":"revive_chosen","trash_index":if *trash_index==usize::MAX {None} else {Some(*trash_index)},"card_id":card_id})
        }
        ChainAbility::EmpowerObject {
            player,
            object_id,
            lane,
            source_id,
        } => {
            json!({"type":"empower_object","player":player,"object_id":object_id,"lane":lane,"source_id":source_id})
        }
        ChainAbility::Equip { gear_id, target } => {
            json!({"type":"equip","gear_id":gear_id,"target":target_to_json(target)})
        }
        ChainAbility::QuickDrawAttach { gear_id, target } => {
            json!({"type":"quick_draw_attach","gear_id":gear_id,"target":target_to_json(target)})
        }
        ChainAbility::Empower { unit_id } => json!({"type":"empower","unit_id":unit_id}),
        ChainAbility::Swap { unit_id, target } => {
            json!({"type":"swap","unit_id":unit_id,"target":target_to_json(target)})
        }
        ChainAbility::CardEffect { entering } => json!({"type":"card_effect","entering":entering}),
        ChainAbility::Legend(ability) => match ability {
            LegendAbility::IreliaReady { unit_id } => {
                json!({"type":"irelia_ready","unit_id":unit_id})
            }
            LegendAbility::IreliaRefresh => json!({"type":"irelia_refresh"}),
            LegendAbility::KennenEmpower => json!({"type":"kennen_empower"}),
            LegendAbility::KennenAssault { target } => {
                json!({"type":"kennen_assault","target":target_to_json(target)})
            }
            LegendAbility::ViktorRecruit => json!({"type":"viktor_recruit"}),
            LegendAbility::LuxDraw => json!({"type":"lux_draw"}),
        },
        ChainAbility::Battlefield(ability) => json!({"type":match ability{
        BattlefieldAbility::TempleDraw{..}=>"temple_draw",BattlefieldAbility::SeatDraw{..}=>"seat_draw",BattlefieldAbility::MinefieldBurn{..}=>"minefield_burn"
    },"lane":ability.lane()}),
        ChainAbility::Triggered(ability) => match ability {
            TriggeredAbility::DestroyGear { gear_id } => {
                json!({"type":"destroy_gear","gear_id":gear_id})
            }
            TriggeredAbility::Draw(n) => json!({"type":"draw","count":n}),
            TriggeredAbility::Might { unit_id, amount } => {
                json!({"type":"might","unit_id":unit_id,"amount":amount})
            }
            TriggeredAbility::Stun { player, unit_id } => {
                json!({"type":"stun","player":player,"unit_id":unit_id})
            }
            TriggeredAbility::Gold => json!({"type":"gold"}),
            TriggeredAbility::Tokens { count, might } => {
                json!({"type":"tokens","count":count,"might":might})
            }
            TriggeredAbility::Xp(n) => json!({"type":"xp","amount":n}),
            TriggeredAbility::RevealHandXp => json!({"type":"reveal_hand_xp"}),
            TriggeredAbility::GrantFlow => json!({"type":"grant_flow"}),
            TriggeredAbility::Loot { count, draw } => {
                json!({"type":"loot","discard":count,"draw":draw})
            }
            TriggeredAbility::BurnForMight { unit_id } => {
                json!({"type":"burn_for_might","unit_id":unit_id})
            }
            TriggeredAbility::LastRites { gear_id } => {
                json!({"type":"last_rites","gear_id":gear_id})
            }
            TriggeredAbility::SwapChoice { unit_id } => {
                json!({"type":"swap_choice","unit_id":unit_id})
            }
            TriggeredAbility::EmpowerHere { lane, source_id } => {
                json!({"type":"empower_here","lane":lane,"source_id":source_id})
            }
        },
    }
}

fn trigger_context_to_json(context: &TriggerContext) -> Value {
    json!({"source":{"player":context.source.player,"unit_id":context.source.unit_id,
        "gear_id":context.source.gear_id},"subject":context.subject.as_ref().map(target_to_json)})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Unit, domain_index};
    use crate::hidden::HiddenCard;

    fn playing() -> Session {
        let mut session = Session::new(ResetOptions {
            decks: [3, 3],
            ..ResetOptions::default()
        })
        .unwrap();
        session.start(0).unwrap();
        session
    }
    fn unit(id: u32, lane: Option<usize>, might: u8) -> Unit {
        Unit {
            id,
            card_id: 25,
            lane,
            ready: true,
            damage: 0,
            buff: might,
            temporary_buff: 0,
            token: true,
            protected: false,
            stunned: false,
            banish_on_death: false,
        }
    }
    fn submit(session: &mut Session, action: Action) {
        let response = session.handle(json!({"command":"apply","revision":session.revision(),
            "player":session.game.priority(),"action":action_to_json(&action)}));
        assert_eq!(response["ok"], true, "{response}");
    }
    fn pass_twice(session: &mut Session) {
        submit(session, Action::Pass);
        submit(session, Action::Pass);
    }

    #[test]
    fn attachment_observations_distinguish_activated_equip_from_quick_draw() {
        let target = Target::Unit {
            player: 0,
            unit_id: 12,
        };
        let equip = ability_to_json(&ChainAbility::Equip { gear_id: 7, target });
        let triggered = ability_to_json(&ChainAbility::QuickDrawAttach { gear_id: 7, target });
        assert_eq!(equip["type"], "equip");
        assert_eq!(triggered["type"], "quick_draw_attach");
        assert_eq!(equip["gear_id"], triggered["gear_id"]);
        assert_eq!(equip["target"], triggered["target"]);
    }

    #[test]
    fn all_structured_actions_round_trip_without_losing_choices() {
        let unit = Target::Unit {
            player: 1,
            unit_id: 12,
        };
        let gear = Target::Gear {
            player: 0,
            gear_id: 6,
        };
        let spell = Target::Spell { chain_id: 3 };
        let actions = vec![
            Action::PlayCard {
                hand_index: 2,
                lane: Some(1),
                target: Some(unit),
            },
            Action::PlayChampion {
                lane: None,
                target: None,
            },
            Action::PlayCardWithTargets {
                hand_index: 1,
                lane: None,
                targets: vec![unit, gear, spell],
            },
            Action::Move {
                unit_id: 4,
                lane: None,
            },
            Action::MoveGroup {
                unit_ids: vec![1, 4],
                lane: Some(0),
            },
            Action::Attack { lane: 1 },
            Action::Pass,
            Action::Choose { accept: false },
            Action::Equip {
                gear_id: 9,
                target: unit,
            },
            Action::EquipWithTrash {
                gear_id: 9,
                target: unit,
                trash_indices: vec![4, 2],
            },
            Action::ActivateGear { gear_id: 3 },
            Action::ActivateAbility {
                source: AbilitySource::Unit {
                    player: 0,
                    unit_id: 19,
                },
                ability_index: 1,
                targets: vec![],
            },
            Action::ActivateAbility {
                source: AbilitySource::Gear {
                    player: 1,
                    gear_id: 20,
                },
                ability_index: 0,
                targets: vec![unit],
            },
            Action::ActivateLegend { target: Some(unit) },
            Action::ActivateRune {
                domain: Domain::Chaos,
                recycle: true,
            },
            Action::RecycleRune {
                domain: Domain::Chaos,
                exhausted: false,
            },
            Action::RecycleRune {
                domain: Domain::Chaos,
                exhausted: true,
            },
            Action::SelectCards {
                indices: vec![4, 1],
            },
            Action::PlayFromTrash {
                trash_index: 3,
                lane: Some(1),
                target: None,
            },
            Action::PlayFromTrashWithTargets {
                trash_index: 3,
                lane: None,
                targets: vec![unit, unit],
            },
            Action::Hide {
                hand_index: 0,
                lane: 1,
            },
            Action::HideChampion { lane: 0 },
            Action::PlayHidden {
                hidden_id: 8,
                targets: vec![unit],
            },
            Action::PlayFromTrashOptions {
                flow: None,
                trash_index: 2,
                lane: Some(1),
                targets: vec![unit],
                repeat_targets: vec![gear],
                repeat: true,
                accelerate: false,
                bonus: false,
            },
            Action::PlayCardOptions {
                hand_index: 1,
                lane: None,
                targets: vec![unit],
                repeat_targets: vec![gear],
                repeat: true,
                accelerate: false,
                bonus: false,
            },
            Action::PlayChampionOptions {
                lane: Some(1),
                target: Some(unit),
                accelerate: true,
            },
            Action::EmpowerUnit {
                unit_id: 8,
                pay_power: true,
            },
            Action::ChooseTarget { target: None },
            Action::ChooseTriggerTargets { targets: vec![] },
            Action::ChooseTriggerTargets {
                targets: vec![unit, gear],
            },
            Action::ChooseDeathReplacement {
                unit_id: 2,
                gear_id: Some(5),
            },
            Action::ChooseDeathReplacement {
                unit_id: 2,
                gear_id: None,
            },
            Action::OrderTrigger { trigger_id: 8 },
            Action::ChooseResolutionGroup { index: 1 },
            Action::ChooseBattlefield { lane: 1 },
            Action::PlaceToken { lane: Some(1) },
            Action::PlaceToken { lane: None },
            Action::SetOptionalDiscount { power: 2 },
            Action::AssignDamage {
                assignments: vec![(9, 2), (3, 4)],
            },
            Action::EndTurn,
            Action::WithPowerPayment {
                action: Box::new(Action::Hide {
                    hand_index: 2,
                    lane: 1,
                }),
                payment: PowerPayment {
                    colored: [1, 0, 2, 0, 0, 0],
                    recycle: [0, 1, 0, 0, 0, 0],
                    any: 3,
                    spell: 2,
                },
            },
        ];
        for action in actions {
            assert_eq!(parse_action(&action_to_json(&action)).unwrap(), action);
        }
    }

    #[test]
    fn activated_catalog_exposes_printed_costs_timing_and_target_restrictions() {
        let catalog = catalog();
        let renata = &catalog["cards"][239]["activated_abilities"];
        assert_eq!(renata.as_array().unwrap().len(), 2);
        assert_eq!(renata[0]["ability_index"], 0);
        assert_eq!(renata[0]["energy"], 1);
        assert_eq!(renata[0]["power"], 1);
        assert_eq!(renata[0]["power_domain"], "Mind");
        assert_eq!(renata[0]["exhaust"], false);
        assert_eq!(renata[1]["ability_index"], 1);
        assert_eq!(renata[1]["energy"], 4);
        assert_eq!(renata[1]["power"], 4);
        assert_eq!(renata[1]["exhaust"], true);
        assert_eq!(renata[1]["requires_battlefield"], true);
        let sage = &catalog["cards"][236]["activated_abilities"][0];
        assert_eq!(sage["timing"], "reaction");
        assert_eq!(sage["adds_resources"], true);
        let bar = &catalog["cards"][234]["activated_abilities"][0];
        assert_eq!(bar["targets"]["allegiance"], "friendly");
        assert_eq!(bar["targets"]["exhausted"], true);
        let ballista = &catalog["cards"][232]["activated_abilities"][0];
        assert_eq!(ballista["timing"], "main");
        assert_eq!(ballista["targets"]["battlefield"], true);
    }

    #[test]
    fn activated_api_distinguishes_source_copies_and_ability_indices() {
        let mut session = playing();
        session.game.battlefields = [0, 0];
        session.game.players[0].units = [700, 701]
            .into_iter()
            .map(|id| Unit {
                card_id: 239,
                token: false,
                buff: 0,
                ..unit(id, Some(0), 0)
            })
            .collect();
        session.game.battlefield_owner[0] = Some(0);
        session.game.players[0].energy_pool = 10;
        session.game.players[0].energy = 10;
        session.game.players[0].runes = [0; 6];
        session.game.players[0].exhausted_runes = [0; 6];
        session.game.players[0].power_pool = [0, 0, 8, 0, 0, 0];
        let first = session.observe(0).unwrap();
        let second = session.observe(1).unwrap();
        assert_eq!(
            first["players"][0]["activated_abilities"],
            second["players"][0]["activated_abilities"]
        );
        assert_eq!(
            first["players"][0]["activated_abilities"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
        assert!(second["players"][0]["hand"].is_null());

        let source = AbilitySource::Unit {
            player: 0,
            unit_id: 700,
        };
        let draw = Action::ActivateAbility {
            source,
            ability_index: 0,
            targets: vec![],
        };
        let revision = session.revision();
        let quoted = session.handle(json!({"command":"quote_payment","revision":revision,
            "player":0,"action":action_to_json(&draw)}));
        assert_eq!(quoted["ok"], true, "{quoted}");
        assert_eq!(session.revision(), revision);
        assert_eq!(quoted["result"]["suggested"]["colored"][2], 1);
        let hand_count = session.game.players[0].hand.len();
        submit(&mut session, draw);
        let pending = session.observe(1).unwrap();
        assert_eq!(
            pending["chain"][0]["source"],
            json!({"type":"card","card_id":239})
        );
        assert_eq!(
            pending["chain"][0]["ability"],
            json!({"type":"activated",
            "source":{"type":"unit","player":0,"unit_id":700},"card_id":239,"ability_index":0})
        );
        assert_eq!(session.game.players[0].hand.len(), hand_count);
        pass_twice(&mut session);
        assert_eq!(session.game.players[0].hand.len(), hand_count + 1);
        assert!(session.game.players[0].units[0].ready);

        let score = Action::ActivateAbility {
            source,
            ability_index: 1,
            targets: vec![],
        };
        submit(
            &mut session,
            Action::WithPowerPayment {
                action: Box::new(score),
                payment: PowerPayment {
                    colored: [0, 0, 4, 0, 0, 0],
                    ..PowerPayment::default()
                },
            },
        );
        assert_eq!(
            session.observe(0).unwrap()["chain"][0]["ability"]["ability_index"],
            1
        );
        assert!(!session.game.players[0].units[0].ready);
        assert!(session.game.players[0].units[1].ready);
        assert_eq!(session.game.players[0].score, 0);
        pass_twice(&mut session);
        assert_eq!(session.game.players[0].score, 1);
        assert_eq!(session.game.player_scored_this_turn[0], [false; 2]);
    }

    #[test]
    fn malformed_and_invalid_activated_commands_preserve_match_and_revision() {
        let mut session = playing();
        let good = json!({"type":"activate_ability","source":{"type":"unit","player":0,"unit_id":8},
            "ability_index":0,"targets":[]});
        let mut cases = vec![good.clone()]; // A valid-shaped but absent source.
        for source in [
            json!({"type":"unit","player":2,"unit_id":8}),
            json!({"type":"unit","player":0,"unit_id":-1}),
            json!({"type":"unit","player":0,"unit_id":4294967296u64}),
            json!({"type":"gear","player":0,"gear_id":1.5}),
            json!({"type":"unit","player":0,"unit_id":8,"gear_id":8}),
            json!({"type":"legend","player":0,"legend_id":3}),
            json!({"type":"unit","unit_id":8}),
        ] {
            let mut value = good.clone();
            value["source"] = source;
            cases.push(value);
        }
        for index in [json!(-1), json!(0.5), json!(true), json!("0")] {
            let mut value = good.clone();
            value["ability_index"] = index;
            cases.push(value);
        }
        let before = format!("{:?}", session.game);
        let revision = session.revision();
        for action in cases {
            let response = session
                .handle(json!({"command":"apply","revision":revision,"player":0,"action":action}));
            assert_eq!(response["ok"], false, "{response}");
            assert_eq!(session.revision(), revision);
            assert_eq!(format!("{:?}", session.game), before);
        }
    }

    #[test]
    fn payment_quotes_and_explicit_sources_are_revision_checked_and_atomic() {
        let mut session = playing();
        session.game.players[0].hand = vec![21];
        session.game.players[0].units.push(unit(100, Some(0), 3));
        session.game.players[0].power_pool[4] = 1;
        session.game.players[0].any_power = 1;
        session.game.battlefield_owner[0] = Some(0);
        let revision = session.revision();
        let action = json!({"type":"hide","hand_index":0,"lane":0});
        let before = format!("{:?}", session.game);
        let response = session.handle(
            json!({"command":"quote_payment","revision":revision,"player":0,"action":action}),
        );
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["result"]["suggested"]["colored"][4], 1);
        assert_eq!(response["result"]["available"]["any"], 1);
        assert_eq!(format!("{:?}", session.game), before);
        assert_eq!(session.revision(), revision);
        for request in [
            json!({"command":"quote_payment","revision":revision-1,"player":0,"action":action}),
            json!({"command":"quote_payment","revision":revision,"player":1,"action":action}),
            json!({"command":"apply","revision":revision,"player":0,"action":{"type":"with_power_payment","action":action,"payment":{"colored":[0,0,0,0,0,0],"recycle":[0,0,0,0,0,0],"any":2,"spell":0}}}),
            json!({"command":"apply","revision":revision,"player":0,"action":{"type":"with_power_payment","action":{"type":"with_power_payment","action":action},"payment":{}}}),
            json!({"command":"apply","revision":revision,"player":0,"action":{"type":"with_power_payment","action":action,"payment":{"colored":[0,0,0],"recycle":[0,0,0,0,0,0],"any":1,"spell":0}}}),
        ] {
            let response = session.handle(request);
            assert_eq!(response["ok"], false, "{response}");
            assert_eq!(session.revision(), revision);
            assert_eq!(format!("{:?}", session.game), before);
        }
        let response = session.handle(json!({"command":"apply","revision":revision,"player":0,"action":{"type":"with_power_payment","action":action,"payment":{"colored":[0,0,0,0,0,0],"recycle":[0,0,0,0,0,0],"any":1,"spell":0}}}));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(session.revision(), revision + 1);
        assert_eq!(session.game.players[0].any_power, 0);
        assert_eq!(session.game.players[0].power_pool[4], 1);
        assert_eq!(session.game.players[0].hidden.len(), 1);
    }

    #[test]
    fn malformed_stale_illegal_and_wrong_seat_commands_are_atomic() {
        let mut session = playing();
        let before = format!("{:?}", session.game);
        let revision = session.revision();
        let requests = vec![
            json!({"command":"apply","revision":revision-1,"player":0,"action":{"type":"end_turn"}}),
            json!({"command":"apply","revision":revision,"player":1,"action":{"type":"end_turn"}}),
            json!({"command":"apply","revision":revision,"player":0,"action":{"type":"play_card","hand_index":99999}}),
            json!({"command":"apply","revision":revision,"player":0,"action":{"type":"move","unit_id":888,"lane":0}}),
            json!({"command":"apply","revision":revision,"player":0,"action":{"type":"play_card","hand_index":0,"target":{"type":"unit","player":2,"unit_id":1}}}),
            json!({"command":"apply","revision":revision,"player":0,"action":{"type":"attack","lane":99}}),
            json!({"command":"apply","revision":revision,"player":0,"action":{"type":"choose","accept":"false"}}),
            json!({"command":"apply","revision":revision,"player":0,"action":{"type":"end_turn","typo":1}}),
            json!({"command":"reset","revision":revision,"decks":[0,99]}),
            json!({"command":"reset","revision":revision,"first_player":2}),
            json!({"command":"unknown"}),
            json!([]),
        ];
        for request in requests {
            let result = session.handle(request);
            assert_eq!(result["ok"], false, "{result}");
            assert_eq!(session.revision(), revision);
            assert_eq!(format!("{:?}", session.game), before);
        }
        assert_eq!(
            session.process_line("{broken")["error"]["code"],
            "invalid_json"
        );
        assert_eq!(format!("{:?}", session.game), before);
    }

    #[test]
    fn revision_bound_suggestions_expire_and_validation_never_advances() {
        let mut session = playing();
        let action = session
            .suggestions(0)
            .unwrap()
            .into_iter()
            .find(|s| s.action == Action::EndTurn)
            .unwrap();
        let revision = session.revision();
        session
            .validate_action(0, revision, action.action.clone())
            .unwrap();
        assert_eq!(session.revision(), revision);
        session.apply_suggestion(0, revision, &action.id).unwrap();
        assert_eq!(session.revision(), revision + 1);
        assert_eq!(
            session
                .apply_suggestion(0, revision, &action.id)
                .unwrap_err()
                .code,
            "stale_revision"
        );
        assert_eq!(
            session
                .apply_suggestion(1, session.revision(), &action.id)
                .unwrap_err()
                .code,
            "unknown_action"
        );
    }

    #[test]
    fn both_agents_choose_their_own_mulligan_and_setup_is_validated() {
        let mut session = Session::new(ResetOptions {
            first_player: 1,
            battlefields: Some([3, 2]),
            ..ResetOptions::default()
        })
        .unwrap();
        let original = session.game.players[0].hand.clone();
        session.mulligan(1, 0, &[0, 2]).unwrap();
        assert_eq!(session.game.players[0].hand, original);
        assert_eq!(session.game.players[1].hand.len(), 4);
        assert_eq!(session.game.mulligans_used, [false, true]);
        assert!(session.mulligan(1, 1, &[]).is_err());
        assert_eq!(session.revision(), 1);
        session.mulligan(0, 1, &[]).unwrap();
        session.start(2).unwrap();
        assert_eq!(session.game.active, 1);
        assert_eq!(session.game.players[1].total_runes(), 2);
        assert_eq!(session.game.battlefields, [3, 2]);
    }

    #[test]
    fn observations_hide_hands_future_decks_and_private_decisions() {
        let mut first = playing();
        first.game.players[1].hand = vec![65, 65];
        first.game.players[1].hidden.push(HiddenCard {
            id: 100,
            card_id: 46,
            lane: 1,
            hidden_turn: 1,
            revealed: false,
        });
        let mut second = first.clone();
        second.game.players[1].hand = vec![1, 1];
        second.game.players[0].deck.reverse();
        second.game.players[1].deck.reverse();
        second.game.players[0].rune_deck.reverse();
        second.game.players[1].rune_deck.reverse();
        second.game.players[1].hidden[0].card_id = 16;
        second
            .game
            .log
            .push("private diagnostic that must never be exported".into());
        assert_eq!(format!("{first:?}"), format!("{second:?}"));
        assert_eq!(first.observe(0).unwrap(), second.observe(0).unwrap());
        assert_ne!(first.observe(1).unwrap(), second.observe(1).unwrap());
        first.game.decision = Some(Decision::Predict {
            player: 1,
            card_id: 65,
        });
        second.game.decision = Some(Decision::Predict {
            player: 1,
            card_id: 1,
        });
        assert_eq!(first.observe(0).unwrap(), second.observe(0).unwrap());
        assert!(first.suggestions(0).unwrap().is_empty());
        first.game.decision = Some(Decision::TopThree {
            player: 1,
            cards: vec![65, 65],
        });
        second.game.decision = Some(Decision::TopThree {
            player: 1,
            cards: vec![1, 1],
        });
        assert_eq!(first.observe(0).unwrap(), second.observe(0).unwrap());
        assert_eq!(first.observe(0).unwrap()["players"][1]["hand"], Value::Null);
        assert_eq!(
            first.observe(0).unwrap()["players"][1]["hidden"][0]["card_id"],
            Value::Null
        );
    }

    #[test]
    fn legitimate_reveal_and_temporary_hidden_permission_are_visible_only_as_allowed() {
        let mut session = playing();
        session.game.players[1].hand = vec![65, 1];
        session.game.players[1].hidden.push(HiddenCard {
            id: 100,
            card_id: 46,
            lane: 1,
            hidden_turn: 1,
            revealed: false,
        });
        session.game.decision = Some(Decision::RevealHand {
            player: 0,
            opponent: 1,
            cards: vec![65, 1],
        });
        session.game.hidden_view[0] = true;
        let observed = session.observe(0).unwrap();
        assert_eq!(observed["players"][1]["hand"], json!([65, 1]));
        assert_eq!(observed["players"][1]["hidden"][0]["card_id"], 46);
        session.game.hidden_view[0] = false;
        session.game.decision = None;
        let observed = session.observe(0).unwrap();
        assert!(observed["players"][1]["hand"].is_null());
        assert!(observed["players"][1]["hidden"][0]["card_id"].is_null());
    }

    #[test]
    fn both_views_show_attachment_identity_location_and_symmetric_resources() {
        let mut session = playing();
        session.game.players[0].units = vec![unit(700, Some(1), 2)];
        session.game.players[0].gear.push(crate::gear::GearState {
            id: 701,
            card_id: 3,
            attached_to: Some(700),
            ready: true,
            temporary: false,
            token: false,
        });
        session.game.players[1].energy_pool = 2;
        session.game.players[1].energy = 2;
        session.game.players[1].power_pool = [0, 1, 0, 0, 0, 0];
        session.game.legend[1].spell_power = 1;
        for viewer in 0..2 {
            let observed = session.observe(viewer).unwrap();
            let gear = &observed["players"][0]["gear"][0];
            assert_eq!(gear["id"], 701);
            assert_eq!(gear["card_id"], 3);
            assert_eq!(gear["attached_to"], 700);
            assert_eq!(gear["lane"], 1);
            assert_eq!(observed["players"][1]["energy_pool"], 2);
            assert_eq!(
                observed["players"][1]["power_pool"],
                json!([0, 1, 0, 0, 0, 0])
            );
            assert_eq!(observed["players"][1]["legend"]["spell_power"], 1);
        }
        session.game.players[0].units[0].lane = None;
        let observed = session.observe(1).unwrap();
        assert_eq!(observed["players"][0]["gear"][0]["attached_to"], 700);
        assert!(observed["players"][0]["gear"][0]["lane"].is_null());
    }

    #[test]
    fn public_unit_keywords_follow_equipment_grants_level_and_token_identity() {
        let mut session = playing();
        let mut yi = unit(700, None, 0);
        yi.card_id = 22;
        yi.token = false;
        let mut assault = unit(701, None, 0);
        assault.card_id = 97; // Sharkling: Assault4 and Accelerate.
        assault.token = false;
        let mut bird = unit(702, None, 1);
        bird.card_id = 34;
        session.game.players[0].units = vec![yi, assault, bird];
        session.game.players[0].gear.push(crate::gear::GearState {
            id: 703,
            card_id: 3,
            attached_to: Some(701),
            ready: true,
            temporary: false,
            token: false,
        });
        session.game.legend[0].assault_grants.push((701, 2));
        for viewer in 0..2 {
            let observed = session.observe(viewer).unwrap();
            let units = &observed["players"][0]["units"];
            assert_eq!(units[0]["keywords"]["deflect"], 0);
            assert_eq!(units[0]["keywords"]["ganking"], false);
            assert_eq!(units[1]["keywords"]["assault"], 6);
            assert_eq!(units[1]["keywords"]["ganking"], true);
            assert!(units[1]["keywords"]["accelerate"].is_object());
            assert_eq!(units[1]["might"], 3); // 1 printed + Boots2; no attack bonus at base.
            assert_eq!(units[2]["keywords"]["deflect"], 1);
            assert!(units[2]["keywords"]["accelerate"].is_null());
            assert_eq!(units[2]["damage_priority"], 1);
        }
        session.game.players[0].xp = 6;
        session.game.players[0].gear[0].attached_to = None;
        session.game.legend[0].assault_grants.clear();
        let observed = session.observe(1).unwrap();
        let units = &observed["players"][0]["units"];
        assert_eq!(units[0]["keywords"]["deflect"], 1);
        assert_eq!(units[0]["keywords"]["ganking"], true);
        assert_eq!(units[1]["keywords"]["assault"], 4);
        assert_eq!(units[1]["keywords"]["ganking"], false);
    }

    #[test]
    fn revive_play_observation_is_public_fixed_and_not_declineable() {
        let mut session = playing();
        session.game.players[0].discard = vec![96];
        session.game.decision = Some(Decision::RevivePlay {
            player: 0,
            trash_index: 0,
            card_id: 96,
        });
        for viewer in 0..2 {
            let view = session.observe(viewer).unwrap();
            assert_eq!(
                view["decision"],
                json!({"type":"revive_play","player":0,"trash_index":0,"card_id":96})
            );
            assert_eq!(view["can_decline_trash_play"], false);
        }
        let before = session.observe(0).unwrap();
        let result = session.handle(
            json!({"command":"apply","revision":session.revision(),"player":0,
            "action":{"type":"choose","accept":false}}),
        );
        assert_eq!(result["ok"], false);
        assert_eq!(session.observe(0).unwrap(), before);
        submit(
            &mut session,
            Action::PlayFromTrashOptions {
                trash_index: 0,
                lane: None,
                targets: vec![],
                repeat_targets: vec![],
                repeat: false,
                accelerate: false,
                bonus: false,
                flow: None,
            },
        );
        assert!(session.game.decision.is_none());
        assert!(session.game.players[0].discard.is_empty());
        assert_eq!(session.game.players[0].units[0].card_id, 96);
        assert!(!session.game.players[0].units[0].ready);
    }

    #[test]
    fn token_observation_tracks_pending_plays_and_each_finalized_unit() {
        let mut session = playing();
        session.game.players[0].hand = vec![87];
        session.game.players[0].energy_pool = 6;
        session.game.players[0].energy = 6;
        session.game.players[0].runes = [0; 6];
        session.game.players[0].exhausted_runes = [0; 6];
        submit(
            &mut session,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            },
        );
        pass_twice(&mut session);
        let mut previous_id = 0;
        for entered in 0..4 {
            let own = session.observe(0).unwrap();
            let other = session.observe(1).unwrap();
            assert_eq!(own["pending_token_plays"], other["pending_token_plays"]);
            let pending = own["pending_token_plays"].as_array().unwrap();
            assert_eq!(pending.len(), 4 - entered);
            assert_eq!(
                own["players"][0]["units"].as_array().unwrap().len(),
                entered
            );
            assert_eq!(own["chain"], json!([]));
            assert_eq!(own["players"][0]["discard"], json!([87]));
            let id = pending[0]["id"].as_u64().unwrap();
            assert!(id > previous_id);
            previous_id = id;
            assert_eq!(pending[0]["card_id"], 87);
            assert_eq!(pending[0]["might"], 1);
            assert_eq!(pending[0]["gear"], false);
            assert_eq!(pending[0]["destination"], json!({"mode":"choose"}));
            submit(&mut session, Action::PlaceToken { lane: None });
        }
        let view = session.observe(1).unwrap();
        assert_eq!(view["pending_token_plays"], json!([]));
        assert_eq!(view["players"][0]["units"].as_array().unwrap().len(), 4);
        assert!(view["decision"].is_null());
    }

    #[test]
    fn pending_real_card_and_shield_observations_are_public_and_round_trip() {
        let mut session = playing();
        let player = &mut session.game.players[0];
        player.hand = vec![36];
        player.discard = vec![7];
        player.units.clear();
        player.runes = [6; 6];
        player.exhausted_runes = [0; 6];
        player.energy = 36;
        submit(
            &mut session,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            },
        );
        submit(&mut session, Action::Choose { accept: true });
        assert_eq!(
            session.observe(1).unwrap()["decision"]["type"],
            "trash_target"
        );
        submit(&mut session, Action::SelectCards { indices: vec![0] });
        assert_eq!(
            session.observe(1).unwrap()["chain"][0]["ability"]["type"],
            "play_chosen"
        );
        pass_twice(&mut session);
        let own = session.observe(0).unwrap();
        let other = session.observe(1).unwrap();
        assert_eq!(own["pending_card_plays"], other["pending_card_plays"]);
        assert_eq!(own["pending_card_plays"][0]["card_id"], 7);
        assert_eq!(own["pending_card_plays"][0]["kind"], "fizz");
        assert_eq!(own["players"][0]["discard"], json!([]));
        let id = session.game.players[0].units[0].id;
        let action = Action::FinalizePendingPlay {
            lane: None,
            targets: vec![Target::Unit {
                player: 0,
                unit_id: id,
            }],
            repeat_targets: vec![],
            repeat: false,
            accelerate: false,
            bonus: false,
        };
        let encoded = action_to_json(&action);
        assert_eq!(parse_action(&encoded).unwrap(), action);
        submit(&mut session, action);
        assert_eq!(session.observe(1).unwrap()["pending_card_plays"], json!([]));
        let mut shield = unit(900, None, 0);
        shield.token = false;
        shield.card_id = 107;
        session.game.players[0].units.push(shield);
        assert_eq!(
            session.observe(1).unwrap()["players"][0]["units"][1]["keywords"]["shield"],
            2
        );
    }

    #[test]
    fn bots_can_submit_explicit_targets_and_grouped_destinations() {
        let mut session = playing();
        session.game.battlefields = [0, 1];
        session.game.players[0].hand = vec![53];
        session.game.players[0].runes = [0; 6];
        session.game.players[0].runes[domain_index(Domain::Fury)] = 5;
        session.game.players[0].exhausted_runes = [0; 6];
        session.game.players[0].energy = 5;
        session.game.players[1].units = vec![unit(100, None, 4), unit(101, None, 4)];
        let targets = vec![
            Target::Unit {
                player: 1,
                unit_id: 101,
            },
            Target::Unit {
                player: 1,
                unit_id: 100,
            },
        ];
        submit(
            &mut session,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: targets.clone(),
            },
        );
        assert_eq!(session.game.chain.last().unwrap().targets, targets);
        pass_twice(&mut session);
        assert_eq!(
            session.game.players[1]
                .units
                .iter()
                .map(|u| u.damage)
                .collect::<Vec<_>>(),
            vec![3, 3]
        );
        session.game.players[0].units = vec![unit(200, None, 2), unit(201, None, 2)];
        submit(
            &mut session,
            Action::MoveGroup {
                unit_ids: vec![201, 200],
                lane: Some(1),
            },
        );
        assert!(
            session.game.players[0]
                .units
                .iter()
                .all(|u| u.lane == Some(1) && !u.ready)
        );
        assert_eq!(session.game.showdown.as_ref().unwrap().lane, 1);
    }

    #[test]
    fn fizz_replay_options_pay_repeat_only_and_preserve_explicit_counter_targets() {
        let mut session = playing();
        session.game.players[0].hand = vec![48];
        session.game.players[0].discard = vec![37];
        session.game.players[0].runes = [0; 6];
        session.game.players[0].runes[domain_index(Domain::Chaos)] = 4;
        session.game.players[0].exhausted_runes = [0; 6];
        session.game.players[0].energy = 4;
        session.game.players[0].energy_pool = 0;
        submit(
            &mut session,
            Action::PlayCard {
                hand_index: 0,
                lane: None,
                target: None,
            },
        );
        let chain_id = session.game.chain.last().unwrap().id;
        // A resolved Fizz trigger can permit a counterspell while another spell
        // remains on the chain. The spell's base energy is waived, not Repeat.
        session.game.decision = Some(Decision::ReplaySpell { player: 0 });
        let action = json!({"type":"play_from_trash_options","trash_index":0,
            "lane":null,"targets":[{"type":"spell","chain_id":chain_id}],
            "repeat_targets":[{"type":"spell","chain_id":chain_id}],
            "repeat":true,"accelerate":false,"bonus":false});
        let before = session.observe(0).unwrap();
        let rejected = session.handle(json!({"command":"apply","revision":session.revision(),
            "player":0,"action":action}));
        assert_eq!(rejected["error"]["code"], "illegal_action");
        assert_eq!(session.observe(0).unwrap(), before);

        session.game.players[0].energy_pool = 2;
        session.game.players[0].energy += 2;
        let accepted = session.handle(json!({"command":"apply","revision":session.revision(),
            "player":0,"action":action}));
        assert_eq!(accepted["ok"], true, "{accepted}");
        assert_eq!(session.game.players[0].energy, 1);
        let observation = session.observe(0).unwrap();
        let cast = observation["chain"].as_array().unwrap().last().unwrap();
        assert_eq!(cast["card_id"], 37);
        assert_eq!(cast["repeat"], true);
        assert_eq!(cast["recycle_after"], true);
        assert_eq!(cast["targets"], action["targets"]);
        assert_eq!(cast["repeat_targets"], action["repeat_targets"]);
    }

    #[test]
    fn bots_can_choose_combat_order_and_invalid_allocation_is_atomic() {
        let mut session = playing();
        session.game.battlefields = [0, 1];
        session.game.players[0].units = vec![unit(100, None, 4)];
        session.game.players[1].units = vec![unit(200, Some(0), 2), unit(201, Some(0), 2)];
        session.game.battlefield_owner[0] = Some(1);
        submit(
            &mut session,
            Action::Move {
                unit_id: 100,
                lane: Some(0),
            },
        );
        pass_twice(&mut session);
        assert!(matches!(
            session.game.decision,
            Some(Decision::CombatDamage {
                player: 0,
                total: 4,
                ..
            })
        ));
        let before = session.observe(0).unwrap();
        assert!(
            session
                .apply_action(
                    0,
                    session.revision(),
                    Action::AssignDamage {
                        assignments: vec![(201, 1), (200, 3)]
                    }
                )
                .is_err()
        );
        assert_eq!(session.observe(0).unwrap(), before);
        submit(
            &mut session,
            Action::AssignDamage {
                assignments: vec![(201, 2), (200, 2)],
            },
        );
        assert_eq!(
            session.game.submitted_combat_damage(0),
            Some([(201, 2), (200, 2)].as_slice())
        );
        assert_eq!(
            session.observe(1).unwrap()["players"][0]["submitted_combat_damage"],
            json!([[201, 2], [200, 2]])
        );
    }

    #[test]
    fn deterministic_structured_replay_runs_a_complete_agent_match() {
        let mut first = playing();
        let mut second = first.clone();
        let mut random = 918273u64;
        for _ in 0..5000 {
            assert_eq!(first.observe(0).unwrap(), second.observe(0).unwrap());
            assert_eq!(first.observe(1).unwrap(), second.observe(1).unwrap());
            if first.game.phase == Phase::Finished {
                return;
            }
            let player = first.game.priority();
            let suggestions = first.suggestions(player).unwrap();
            assert!(!suggestions.is_empty());
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            let action = &suggestions[random as usize % suggestions.len()].action;
            let request = json!({"command":"apply","revision":first.revision(),"player":player,"action":action_to_json(action)});
            assert_eq!(first.handle(request.clone()), second.handle(request));
        }
        panic!("custom legal-action bot match did not terminate");
    }

    #[test]
    fn api_builtin_steps_finish_every_preset_pairing() {
        for own in 0..4 {
            for opposing in 0..4 {
                let mut session = Session::new(ResetOptions {
                    decks: [own, opposing],
                    seed: 72 + own as u64,
                    first_player: (own + opposing) % 2,
                    ..ResetOptions::default()
                })
                .unwrap();
                session.start(0).unwrap();
                for _ in 0..5000 {
                    if session.game.phase == Phase::Finished {
                        break;
                    }
                    session
                        .step_ai(session.game.priority(), session.revision())
                        .unwrap();
                }
                assert_eq!(session.game.phase, Phase::Finished, "pair {own}/{opposing}");
            }
        }
    }
    #[test]
    fn public_flow_instances_and_exact_quotes_match_the_selected_source() {
        let mut session = playing();
        session.game.players[0].discard = vec![39];
        session.game.players[0].energy = 5;
        session.game.players[0].energy_pool = 5;
        session.game.players[0].runes = [0; 6];
        session.game.players[0].exhausted_runes = [0; 6];
        session.game.players[0].power_pool = [1, 0, 0, 0, 0, 0];
        for viewer in 0..2 {
            let observed = session.observe(viewer).unwrap();
            assert_eq!(
                observed["players"][0]["flow_costs"],
                json!([{
                "trash_index":0,"costs":[{"source":"printed","energy":2,"power":1,"colored":false}]}])
            );
        }
        let request = json!({"command":"quote_payment","player":0,"revision":session.revision(),
            "action":{"type":"play_from_trash_options","trash_index":0,"flow":"printed"}});
        let before = session.observe(0).unwrap();
        let quote = session.handle(request);
        assert_eq!(quote["ok"], true, "{quote}");
        assert_eq!(
            quote["result"]["suggested"]["colored"],
            json!([1, 0, 0, 0, 0, 0])
        );
        assert_eq!(session.observe(0).unwrap(), before);
    }

    #[test]
    fn generic_trigger_api_preserves_target_order_and_bearer_context() {
        let mut session = playing();
        session.game.players[0].units = vec![unit(100, None, 5), unit(101, None, 3)];
        session.game.players[1].units = vec![unit(102, None, 3)];
        let context = TriggerContext {
            source: crate::game::TriggerSource {
                player: 0,
                unit_id: Some(100),
                gear_id: None,
            },
            subject: Some(Target::Unit {
                player: 0,
                unit_id: 100,
            }),
        };
        session.game.decision = Some(Decision::TriggerTargets {
            player: 0,
            card_id: 190,
            rule_index: 0,
            context,
        });
        for viewer in 0..2 {
            let observed = session.observe(viewer).unwrap();
            let choice = &observed["decision"];
            assert_eq!(choice["type"], "trigger_targets");
            assert_eq!(choice["maximum"], 2);
            assert_eq!(choice["required"], 2);
            assert_eq!(choice["context"]["source"]["unit_id"], 100);
        }
        let own = Target::Unit {
            player: 0,
            unit_id: 101,
        };
        let enemy = Target::Unit {
            player: 1,
            unit_id: 102,
        };
        let before = session.observe(0).unwrap();
        for targets in [vec![], vec![enemy, own], vec![own, own]] {
            assert!(
                session
                    .apply_action(
                        0,
                        session.revision(),
                        Action::ChooseTriggerTargets { targets }
                    )
                    .is_err()
            );
            assert_eq!(session.observe(0).unwrap(), before);
        }
        submit(
            &mut session,
            Action::ChooseTriggerTargets {
                targets: vec![own, enemy],
            },
        );
        let observed = session.observe(0).unwrap();
        let item = observed["chain"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["card_id"] == 190)
            .unwrap();
        assert_eq!(
            item["targets"],
            json!([target_to_json(&own), target_to_json(&enemy)])
        );
        assert_eq!(item["ability"]["type"], "card_trigger");
        assert_eq!(
            item["ability"]["context"],
            trigger_context_to_json(&context)
        );
    }

    #[test]
    fn optional_trigger_api_reports_zero_required_and_can_decline() {
        let mut session = playing();
        session.game.players[0].units = vec![unit(100, None, 5)];
        session.game.decision = Some(Decision::TriggerTargets {
            player: 0,
            card_id: 188,
            rule_index: 0,
            context: TriggerContext {
                source: crate::game::TriggerSource {
                    player: 0,
                    unit_id: Some(100),
                    gear_id: None,
                },
                subject: None,
            },
        });
        let observed = session.observe(0).unwrap();
        assert_eq!(observed["decision"]["maximum"], 1);
        assert_eq!(observed["decision"]["required"], 0);
        submit(
            &mut session,
            Action::ChooseTriggerTargets { targets: vec![] },
        );
        assert!(session.game.decision.is_none());
        assert!(session.game.chain.is_empty());
    }
}
