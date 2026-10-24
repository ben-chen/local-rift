//! Canonical play choices, cost plans, and delayed play grants.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayOrigin {
    Hand,
    Champion,
    Trash,
    Banish,
    Hidden { lane: usize },
    Token,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiddenTargetRestriction {
    AtBattlefield { lane: usize },
    OutsideBattlefield { lane: usize },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayTargetSlot {
    pub index: usize,
    pub hidden: Option<HiddenTargetRestriction>,
    pub different_location_from: Option<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayTargetView {
    pub card_id: CardId,
    pub origin: PlayOrigin,
    pub execution: usize,
    pub targets: Vec<Target>,
    pub spec: rules::PlayTargetSpec,
    pub next_slot: Option<PlayTargetSlot>,
    pub candidates: Vec<Target>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExecutionChoices {
    pub targets: Vec<Target>,
    pub mode: Option<usize>,
}
// No current spell has a mode choice: current accepted mode is None.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepeatSelection {
    pub option_id: u64,
    pub choices: ExecutionChoices,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PowerReduction {
    pub component_id: u64,
    pub amount: u16,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostSelection {
    pub modifier_id: u64,
    pub use_power: bool,
    pub power_allocations: Vec<PowerReduction>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayChoices {
    pub repeats: Vec<RepeatSelection>, // ordered extra executions; each option at most once
    pub accelerate: bool,
    pub bonus: bool, // existing Rampage option / Nami optional cost; descriptor labels below
    pub cost_order: Vec<CostSelection>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CostValue {
    pub xp: crate::xp::XpAmount,
    pub energy: u16,
    pub power: u16,
    pub domains: Vec<Domain>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepeatOption {
    pub id: u64,
    pub label: String,
    pub source: ChainSource,
    pub cost: CostValue,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionalCostOption {
    pub label: String,
    pub cost: CostValue,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CostScope {
    Total,
    Component { component_id: u64 },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostComponent {
    pub id: u64,
    pub label: String,
    pub cost: CostValue,
    pub optional: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostModifier {
    pub id: u64,
    pub label: String,
    pub source: ChainSource,
    pub energy: u64,
    pub power: u16,
    pub minimum_energy: u16,
    pub choose_energy_or_power: bool,
    pub increase: bool,
    pub scope: CostScope,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CostPlanView {
    pub xp: crate::xp::XpAmount,
    pub components: Vec<CostComponent>,
    pub modifiers: Vec<CostModifier>,
    pub energy: u32,
    pub power: Vec<crate::payment::PowerCost>,
    pub applied_order: Vec<CostSelection>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayChoiceView {
    pub card_id: CardId,
    pub origin: PlayOrigin,
    pub repeats: Vec<RepeatOption>,
    pub accelerate: Option<OptionalCostOption>,
    pub bonus: Option<OptionalCostOption>,
    pub costs: CostPlanView,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayGrantKind {
    ReadyNextUnit,
    AccelerateNextUnit,
    BonusNextSpell(u8),
    RepeatNextSpell,
    DiscountNextSpell { energy: u16, power: u16 },
    DiscountNextCard { energy: u16, power: u16 },
    ReadyUnitsThisTurn,
    ReadyBuffNextUnit,
    UnitSurchargeThisTurn(u16),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayGrant {
    pub id: u32,
    pub player: usize,
    pub source: ChainSource,
    pub expires_turn: u32,
    pub kind: PlayGrantKind,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayHistory {
    /// Ezreal's qualifying-target progress, capped at its printed threshold 2.
    pub ezreal_progress: [u8; 2],
    pub non_token_gears: [u16; 2],
    pub gear_activations: [u16; 2],
    pub units_or_gears_here: [[u16; 2]; 2], // [player][physical lane]
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolutionProgress {
    pub chain_id: u32,
    pub execution: usize,
    pub executions: usize,
}

pub struct CostContext {
    pub player: usize,
    pub card_id: Option<CardId>,
    pub card_kind: Option<CardKind>,
    pub token: bool,
    pub lane: Option<usize>,
    pub origin: Option<PlayOrigin>,
    pub ability_source: Option<AbilitySource>,
    pub empower: bool,
    pub gear_activation: bool,
    pub targets: Vec<Target>,
}

#[derive(Clone, Debug)]
pub(crate) struct ActivePlay {
    pub player: usize,
    pub card_id: CardId,
    pub origin: PlayOrigin,
    pub choices: PlayChoices,
    pub costs: CostPlanView,
    pub grants: Vec<PlayGrant>,
    pub paid: bool,
}

#[derive(Clone, Debug)]
struct CardIntent {
    id: CardId,
    origin: PlayOrigin,
    lane: Option<usize>,
    targets: Vec<Target>,
    base: CostValue,
    legacy: PlayChoices,
    token: bool,
    has_flow: bool,
}

impl Game {
    pub(crate) fn grant_play(
        &mut self,
        player: usize,
        source: ChainSource,
        kind: PlayGrantKind,
    ) -> u32 {
        let id = self.next_chain_id;
        self.next_chain_id += 1;
        self.play_grants.push(PlayGrant {
            id,
            player,
            source,
            expires_turn: self.turn,
            kind,
        });
        id
    }

    pub(crate) fn current_damage_context(&self) -> Option<crate::game::damage::DamageContext> {
        self.damage_context
    }

    fn applicable_grants(&self, player: usize, kind: CardKind, token: bool) -> Vec<PlayGrant> {
        self.play_grants
            .iter()
            .filter(|g| {
                g.player == player
                    && g.expires_turn == self.turn
                    && match g.kind {
                        PlayGrantKind::ReadyNextUnit
                        | PlayGrantKind::AccelerateNextUnit
                        | PlayGrantKind::ReadyBuffNextUnit
                        | PlayGrantKind::ReadyUnitsThisTurn
                        | PlayGrantKind::UnitSurchargeThisTurn(_) => kind == CardKind::Unit,
                        PlayGrantKind::BonusNextSpell(_)
                        | PlayGrantKind::RepeatNextSpell
                        | PlayGrantKind::DiscountNextSpell { .. } => kind == CardKind::Spell,
                        PlayGrantKind::DiscountNextCard { .. } => !token,
                    }
            })
            .cloned()
            .collect()
    }

    fn intent_grants(&self, intent: &CardIntent) -> Vec<PlayGrant> {
        if let Some(active) = self
            .active_play
            .as_ref()
            .filter(|a| a.card_id == intent.id && a.origin == intent.origin)
        {
            return active.grants.clone();
        }
        let reserved = match intent.origin {
            PlayOrigin::Token => self.pending_token.as_ref().map(|p| p.id),
            PlayOrigin::Trash | PlayOrigin::Banish => self.pending_card.as_ref().map(|p| p.id),
            _ => None,
        };
        if let Some(id) = reserved
            && let Some((_, grants)) = self.reserved_play_grants.iter().find(|(key, _)| *key == id)
        {
            return grants.clone();
        }
        if self.payment_frozen_source().is_some() {
            return self.payment_play_grants.clone();
        }
        self.applicable_grants(
            self.priority(),
            if intent.token && rules::token_is_gear(intent.id) {
                CardKind::Gear
            } else if intent.token {
                CardKind::Unit
            } else {
                self.catalog[intent.id].kind
            },
            intent.token,
        )
    }

    pub(crate) fn reserve_play_grants(
        &mut self,
        player: usize,
        kind: CardKind,
        key: u32,
        token: bool,
    ) {
        let grants = self.applicable_grants(player, kind, token);
        self.consume_grants(&grants);
        self.reserved_play_grants.push((key, grants));
    }

    fn consume_grants(&mut self, grants: &[PlayGrant]) {
        self.play_grants.retain(|g| {
            !grants.iter().any(|taken| taken.id == g.id)
                || matches!(
                    g.kind,
                    PlayGrantKind::ReadyUnitsThisTurn | PlayGrantKind::UnitSurchargeThisTurn(_)
                )
        });
    }

    fn card_intent(&self, action: &Action) -> Result<CardIntent, String> {
        let p = self.priority();
        let mut legacy = PlayChoices::default();
        let (id, origin, lane, targets) = match action {
            Action::WithPlayChoices { action, choices } => {
                let mut i = self.card_intent(action)?;
                i.legacy = choices.clone();
                return Ok(i);
            }
            Action::WithPowerPayment { action, .. } | Action::BeginPayment { action } => {
                return self.card_intent(action);
            }
            Action::PlayCard {
                hand_index,
                lane,
                target,
            } => (
                *self.players[p]
                    .hand
                    .get(*hand_index)
                    .ok_or("Choose a hand card.")?,
                PlayOrigin::Hand,
                *lane,
                target.iter().copied().collect(),
            ),
            Action::PlayCardWithTargets {
                hand_index,
                lane,
                targets,
            } => (
                *self.players[p]
                    .hand
                    .get(*hand_index)
                    .ok_or("Choose a hand card.")?,
                PlayOrigin::Hand,
                *lane,
                targets.clone(),
            ),
            Action::PlayCardOptions {
                hand_index,
                lane,
                targets,
                repeat_targets,
                repeat,
                accelerate,
                bonus,
            } => {
                let id = *self.players[p]
                    .hand
                    .get(*hand_index)
                    .ok_or("Choose a hand card.")?;
                legacy = self.legacy_choices(id, *repeat, *accelerate, *bonus, repeat_targets);
                (id, PlayOrigin::Hand, *lane, targets.clone())
            }
            Action::PlayChampion { lane, target } => (
                self.players[p]
                    .champion
                    .ok_or("No chosen champion remains.")?,
                PlayOrigin::Champion,
                *lane,
                target.iter().copied().collect(),
            ),
            Action::PlayChampionOptions {
                lane,
                target,
                accelerate,
            } => {
                let id = self.players[p]
                    .champion
                    .ok_or("No chosen champion remains.")?;
                legacy.accelerate = *accelerate;
                (
                    id,
                    PlayOrigin::Champion,
                    *lane,
                    target.iter().copied().collect(),
                )
            }
            Action::PlayHidden { hidden_id, targets } => {
                let h = self.players[p]
                    .hidden
                    .iter()
                    .find(|h| h.id == *hidden_id)
                    .ok_or("No facedown card exists.")?;
                (
                    h.card_id,
                    PlayOrigin::Hidden { lane: h.lane },
                    Some(h.lane),
                    targets.clone(),
                )
            }
            Action::PlayFromTrash {
                trash_index,
                lane,
                target,
            } => (
                *self.players[p]
                    .discard
                    .get(*trash_index)
                    .ok_or("Choose a trash card.")?,
                PlayOrigin::Trash,
                *lane,
                target.iter().copied().collect(),
            ),
            Action::PlayFromTrashWithTargets {
                trash_index,
                lane,
                targets,
            } => (
                *self.players[p]
                    .discard
                    .get(*trash_index)
                    .ok_or("Choose a trash card.")?,
                PlayOrigin::Trash,
                *lane,
                targets.clone(),
            ),
            Action::PlayFromTrashOptions {
                trash_index,
                lane,
                targets,
                repeat_targets,
                repeat,
                accelerate,
                bonus,
                ..
            } => {
                let id = *self.players[p]
                    .discard
                    .get(*trash_index)
                    .ok_or("Choose a trash card.")?;
                legacy = self.legacy_choices(id, *repeat, *accelerate, *bonus, repeat_targets);
                (id, PlayOrigin::Trash, *lane, targets.clone())
            }
            Action::FinalizePendingPlay {
                lane,
                targets,
                repeat_targets,
                repeat,
                accelerate,
                bonus,
            } => {
                let pending = self.pending_card.as_ref().ok_or("No pending card.")?;
                let id = pending.card_id;
                legacy = self.legacy_choices(id, *repeat, *accelerate, *bonus, repeat_targets);
                (id, pending.play_origin(), *lane, targets.clone())
            }
            Action::PlaceToken { lane } | Action::FinalizeTokenPlay { lane, .. } => {
                let t = self.pending_token.as_ref().ok_or("No pending token.")?;
                if let Action::FinalizeTokenPlay { accelerate, .. } = action {
                    legacy.accelerate = *accelerate;
                }
                return Ok(CardIntent {
                    id: t.card_id,
                    origin: PlayOrigin::Token,
                    lane: *lane,
                    targets: vec![],
                    base: CostValue::default(),
                    legacy,
                    token: true,
                    has_flow: false,
                });
            }
            _ => return Err("This action is not a card or token play.".into()),
        };
        let c = self.catalog.get(id).ok_or("Unknown card.")?;
        let mut base = CostValue {
            xp: 0,
            energy: c.cost.into(),
            power: c.power_cost.into(),
            domains: c.domains.to_vec(),
        };
        if matches!(origin, PlayOrigin::Hidden { .. }) {
            base.energy = 0;
            base.power = 0;
        }
        if matches!(origin, PlayOrigin::Trash | PlayOrigin::Banish) {
            if let Some(instruction) = self.current_play_instruction() {
                match instruction.cost {
                    BaseCostWaiver::None => {}
                    BaseCostWaiver::Energy => base.energy = 0,
                    BaseCostWaiver::All => {
                        base.energy = 0;
                        base.power = 0;
                    }
                }
            } else if self.decision.is_none() {
                let (index, source) = match action {
                    Action::PlayFromTrashOptions {
                        trash_index, flow, ..
                    } => (*trash_index, *flow),
                    Action::PlayFromTrash { trash_index, .. }
                    | Action::PlayFromTrashWithTargets { trash_index, .. } => (*trash_index, None),
                    _ => (usize::MAX, None),
                };
                let flows = self.flow_costs(p, index);
                if let Some(f) = flows.iter().find(|f| source.is_none_or(|s| s == f.source)) {
                    base.energy = f.energy.into();
                    base.power = f.power.into();
                    if !f.colored {
                        base.domains = ALL_DOMAINS.to_vec();
                    }
                } else if source.is_some() {
                    return Err("That Flow instance is not available on this trash card.".into());
                }
            }
        }
        let has_flow = rules::flow_cost(id).is_some()
            || self
                .pending_card
                .as_ref()
                .is_some_and(|p| p.card_id == id && p.had_granted_flow())
            || match action {
                Action::PlayFromTrash { trash_index, .. }
                | Action::PlayFromTrashWithTargets { trash_index, .. }
                | Action::PlayFromTrashOptions { trash_index, .. } => {
                    self.temporary_flow[p].contains(trash_index)
                }
                _ => false,
            };
        Ok(CardIntent {
            id,
            origin,
            lane,
            targets,
            base,
            legacy,
            token: false,
            has_flow,
        })
    }

    fn legacy_choices(
        &self,
        _id: CardId,
        repeat: bool,
        accelerate: bool,
        bonus: bool,
        targets: &[Target],
    ) -> PlayChoices {
        PlayChoices {
            repeats: if repeat {
                vec![RepeatSelection {
                    option_id: 1,
                    choices: ExecutionChoices {
                        targets: targets.to_vec(),
                        mode: None,
                    },
                }]
            } else {
                vec![]
            },
            accelerate,
            bonus,
            ..Default::default()
        }
    }

    fn repeat_options(&self, intent: &CardIntent, grants: &[PlayGrant]) -> Vec<RepeatOption> {
        if intent.token || self.catalog[intent.id].kind != CardKind::Spell {
            return vec![];
        }
        let mut result = Vec::new();
        if let Some(c) = rules::repeat_cost(intent.id) {
            result.push(RepeatOption {
                id: 1,
                label: "Printed Repeat".into(),
                source: ChainSource::Card { card_id: intent.id },
                cost: CostValue {
                    xp: 0,
                    energy: c.energy.into(),
                    power: c.power.into(),
                    domains: c.domain.map_or_else(|| ALL_DOMAINS.to_vec(), |d| vec![d]),
                },
            });
        }
        for g in grants {
            if g.kind == PlayGrantKind::RepeatNextSpell {
                let c = self.catalog[intent.id];
                result.push(RepeatOption {
                    id: 1_000_000 + u64::from(g.id),
                    label: "Granted Repeat: printed cost".into(),
                    source: g.source,
                    cost: CostValue {
                        xp: 0,
                        energy: c.cost.into(),
                        power: c.power_cost.into(),
                        domains: c.domains.to_vec(),
                    },
                });
            }
        }
        let p = self.priority();
        for u in &self.players[p].units {
            if !u.token
                && u.card_id == 266
                && self
                    .showdown
                    .as_ref()
                    .is_some_and(|w| u.lane == Some(w.lane))
            {
                result.push(RepeatOption {
                    id: 2_000_000 + u64::from(u.id),
                    label: "Syndra Repeat".into(),
                    source: ChainSource::Card { card_id: 266 },
                    cost: CostValue {
                        xp: 0,
                        energy: 2,
                        power: 1,
                        domains: vec![Domain::Chaos],
                    },
                });
            }
        }
        result
    }

    fn accelerate_option(
        &self,
        intent: &CardIntent,
        grants: &[PlayGrant],
    ) -> Option<OptionalCostOption> {
        if !intent.token && self.catalog[intent.id].kind != CardKind::Unit {
            return None;
        }
        let granted = grants
            .iter()
            .any(|g| g.kind == PlayGrantKind::AccelerateNextUnit)
            || (intent.origin != PlayOrigin::Hand
                && self.players[self.priority()]
                    .units
                    .iter()
                    .any(|u| !u.token && u.card_id == 265));
        if let Some(c) = (!intent.token)
            .then(|| rules::accelerate_cost(intent.id))
            .flatten()
        {
            return Some(OptionalCostOption {
                label: "Accelerate: enter ready".into(),
                cost: CostValue {
                    xp: 0,
                    energy: c.energy.into(),
                    power: c.power.into(),
                    domains: vec![c.domain],
                },
            });
        }
        granted.then(|| OptionalCostOption {
            label: "Granted Accelerate: enter ready".into(),
            cost: CostValue {
                xp: 0,
                energy: 1,
                power: 1,
                domains: if intent.token || self.catalog[intent.id].domains.len() != 1 {
                    ALL_DOMAINS.to_vec()
                } else {
                    self.catalog[intent.id].domains.to_vec()
                },
            },
        })
    }

    // Cost facts are determined after a physical source leaves its origin.
    // Queries and actual finalization use the same snapshot, including replayed
    // payments whose frozen snapshot has already removed the selected source.
    fn play_cost_snapshot(&self, action: &Action) -> Self {
        if let Some(frozen) = self.payment_frozen_source() {
            return frozen.clone();
        }
        match action {
            Action::WithPlayChoices { action, .. }
            | Action::WithPowerPayment { action, .. }
            | Action::BeginPayment { action } => self.play_cost_snapshot(action),
            Action::PlayFromTrash { trash_index, .. }
            | Action::PlayFromTrashWithTargets { trash_index, .. }
            | Action::PlayFromTrashOptions { trash_index, .. }
                if self.pending_card.is_none() =>
            {
                let mut state = self.clone();
                if *trash_index < state.players[state.priority()].discard.len() {
                    state.remove_from_trash(state.priority(), *trash_index);
                }
                state
            }
            _ => self.clone(),
        }
    }

    pub fn play_target_candidates(
        &self,
        action: &Action,
        execution: usize,
    ) -> Result<Vec<Target>, String> {
        Ok(self.play_target_view(action, execution)?.candidates)
    }
    pub fn play_target_view(
        &self,
        action: &Action,
        execution: usize,
    ) -> Result<PlayTargetView, String> {
        if let Some(state) = self.payment_target_query_state()? {
            if matches!(action, Action::CompletePayment) {
                let intent = self
                    .payment_plan_view(self.priority())
                    .and_then(|view| view.action)
                    .ok_or("The payment's play intent is unavailable.")?;
                return state.play_target_view_inner(&intent, execution);
            }
            return state.play_target_view_inner(action, execution);
        }
        self.play_target_view_inner(action, execution)
    }
    fn play_target_view_inner(
        &self,
        action: &Action,
        execution: usize,
    ) -> Result<PlayTargetView, String> {
        self.play_choices(action)?;
        let intent = self.card_intent(action)?;
        let targets = if execution == 0 {
            intent.targets.clone()
        } else {
            intent
                .legacy
                .repeats
                .get(execution - 1)
                .ok_or("That execution is not selected.")?
                .choices
                .targets
                .clone()
        };
        let spec = rules::play_target_spec(intent.id);
        let next_slot = (targets.len() < spec.maximum).then(|| {
            let hidden = match intent.origin {
                PlayOrigin::Hidden { lane } => match rules::hidden_target_policy(intent.id) {
                    rules::HiddenTargetPolicy::Here => {
                        Some(HiddenTargetRestriction::AtBattlefield { lane })
                    }
                    rules::HiddenTargetPolicy::Elsewhere => {
                        Some(HiddenTargetRestriction::OutsideBattlefield { lane })
                    }
                    rules::HiddenTargetPolicy::FirstHere if targets.is_empty() => {
                        Some(HiddenTargetRestriction::AtBattlefield { lane })
                    }
                    _ => None,
                },
                _ => None,
            };
            PlayTargetSlot {
                index: targets.len(),
                hidden,
                different_location_from: (rules::targeting(intent.id).different_locations
                    && !targets.is_empty())
                .then_some(0),
            }
        });
        let mut candidates = Vec::new();
        if next_slot.is_some() {
            for target in
                self.card_target_candidates(self.priority(), intent.id, intent.lane, &targets)?
            {
                if !matches!(intent.origin,PlayOrigin::Hidden {lane} if !self.hidden_target_allowed_at(intent.id,lane,targets.len(),target))
                {
                    candidates.push(target);
                }
            }
        }
        Ok(PlayTargetView {
            card_id: intent.id,
            origin: intent.origin,
            execution,
            targets,
            spec,
            next_slot,
            candidates,
        })
    }

    pub fn play_choices(&self, action: &Action) -> Result<PlayChoiceView, String> {
        Self::validate_choice_nesting(action)?;
        let intent = self.card_intent(action)?;
        // Previews accept incomplete drafts, not malformed choices. Keep this on
        // the live board: only cost facts use the frozen/post-departure snapshot.
        if intent.lane.is_some_and(|lane| lane >= BATTLEFIELDS) {
            return Err("Choose a battlefield on this board.".into());
        }
        for group in std::iter::once(&intent.targets)
            .chain(intent.legacy.repeats.iter().map(|r| &r.choices.targets))
        {
            for (slot, &target) in group.iter().enumerate() {
                if !self.valid_card_target(
                    self.priority(),
                    intent.id,
                    intent.lane,
                    target,
                    &group[..slot],
                )? || matches!(intent.origin, PlayOrigin::Hidden { lane } if !self.hidden_target_allowed_at(intent.id, lane, slot, target))
                {
                    return Err("A supplied target does not meet this play's restrictions.".into());
                }
            }
        }
        let grants = self.intent_grants(&intent);
        let repeats = self.repeat_options(&intent, &grants);
        let accelerate = self.accelerate_option(&intent, &grants);
        let bonus = match intent.id {
            29 => Some(OptionalCostOption {
                label: "Rampage: +2 Might".into(),
                cost: CostValue {
                    xp: 0,
                    energy: 0,
                    power: 1,
                    domains: vec![Domain::Body],
                },
            }),
            261 => Some(OptionalCostOption {
                label: "Nami: stun an enemy when played".into(),
                cost: CostValue {
                    xp: 0,
                    energy: 0,
                    power: 1,
                    domains: vec![Domain::Calm],
                },
            }),
            _ => None,
        };
        let costs = self.play_cost_snapshot(action).card_cost_plan(
            &intent,
            &intent.legacy,
            &grants,
            &repeats,
            accelerate.as_ref(),
            bonus.as_ref(),
        )?;
        Ok(PlayChoiceView {
            card_id: intent.id,
            origin: intent.origin,
            repeats,
            accelerate,
            bonus,
            costs,
        })
    }

    pub fn cost_plan(&self, action: &Action) -> Result<CostPlanView, String> {
        Self::validate_choice_nesting(action)?;
        if let Action::WithPowerPayment { action, .. } | Action::BeginPayment { action } = action {
            return self.cost_plan(action);
        }
        if let Action::WithPlayChoices { action, choices } = action
            && self.card_intent(action).is_err()
        {
            if !choices.repeats.is_empty() || choices.accelerate || choices.bonus {
                return Err("Card additional-cost choices do not apply to this ability.".into());
            }
            return self.ability_plan(action, &choices.cost_order);
        }
        self.play_choices(action)
            .map(|v| v.costs)
            .or_else(|_| self.ability_plan(action, &[]))
    }

    fn card_cost_plan(
        &self,
        intent: &CardIntent,
        choices: &PlayChoices,
        grants: &[PlayGrant],
        repeats: &[RepeatOption],
        accelerate: Option<&OptionalCostOption>,
        bonus: Option<&OptionalCostOption>,
    ) -> Result<CostPlanView, String> {
        let p = self.priority();
        let c = self.catalog[intent.id];
        let mut components = vec![CostComponent {
            id: 1,
            label: "Base cost".into(),
            cost: intent.base.clone(),
            optional: false,
        }];
        let mut seen = Vec::new();
        for selected in &choices.repeats {
            if selected.choices.mode.is_some() {
                return Err("This spell has no mode selection.".into());
            }
            if seen.contains(&selected.option_id) {
                return Err("Choose each Repeat instance at most once.".into());
            }
            seen.push(selected.option_id);
            let r = repeats
                .iter()
                .find(|r| r.id == selected.option_id)
                .ok_or("That Repeat instance is not available.")?;
            components.push(CostComponent {
                id: 100 + selected.option_id,
                label: r.label.clone(),
                cost: r.cost.clone(),
                optional: true,
            });
        }
        if choices.accelerate {
            components.push(CostComponent {
                id: 2,
                label: "Accelerate".into(),
                cost: accelerate
                    .ok_or("This play has no Accelerate instance.")?
                    .cost
                    .clone(),
                optional: true,
            });
        }
        if choices.bonus {
            components.push(CostComponent {
                id: 3,
                label: "Optional additional cost".into(),
                cost: bonus
                    .ok_or("This card has no optional additional cost.")?
                    .cost
                    .clone(),
                optional: true,
            });
        }
        let mut all_targets = intent.targets.clone();
        for r in &choices.repeats {
            all_targets.extend(&r.choices.targets);
        }
        let deflect = self.deflect_cost(p, &all_targets)?;
        if deflect > 0 {
            components.push(CostComponent {
                id: 4,
                label: "Deflect".into(),
                cost: CostValue {
                    xp: 0,
                    energy: 0,
                    power: deflect.into(),
                    domains: ALL_DOMAINS.to_vec(),
                },
                optional: false,
            });
        }
        let source = ChainSource::Card { card_id: intent.id };
        let mut modifiers = Vec::new();
        if intent.origin == PlayOrigin::Banish
            && let Some(instruction) = self.current_play_instruction()
            && instruction.energy_discount > 0
        {
            let source_card = self.pending_card.as_ref().map_or(intent.id, |c| {
                if c.kind == InstructedPlayKind::Reinforce {
                    387
                } else {
                    388
                }
            });
            modifiers.push(modifier(
                9_000_000,
                "Instructed play Energy discount",
                ChainSource::Card {
                    card_id: source_card,
                },
                instruction.energy_discount.into(),
                0,
                0,
                false,
                false,
            ));
        }
        if !intent.token {
            for (i, a) in rules::static_abilities(intent.id).iter().enumerate() {
                let (energy, minimum) = match *a {
                    StaticAbility::ControlsTagEnergyDiscount { tag, energy }
                        if self.players[p].units.iter().any(|u| {
                            self.board_target_has_tag(
                                Target::Unit {
                                    player: p,
                                    unit_id: u.id,
                                },
                                tag,
                            )
                        }) || self.players[p].gear.iter().any(|g| {
                            self.board_target_has_tag(
                                Target::Gear {
                                    player: p,
                                    gear_id: g.id,
                                },
                                tag,
                            )
                        }) =>
                    {
                        (u64::from(energy), 0)
                    }
                    StaticAbility::HighestMightEnergyDiscount => (
                        self.players[p]
                            .units
                            .iter()
                            .map(|u| self.unit_might(u))
                            .collect::<Result<Vec<_>, String>>()?
                            .into_iter()
                            .max()
                            .unwrap_or(0),
                        0,
                    ),
                    StaticAbility::EmpoweredEnergyDiscount(n) if self.controls_empowered(p) => {
                        (n.into(), 0)
                    }
                    StaticAbility::TrashEnergyDiscount => (self.players[p].discard.len() as u64, 0),
                    StaticAbility::LegionEnergyDiscount(n) if self.cards_played[p] > 0 => {
                        (n.into(), 0)
                    }
                    StaticAbility::XpEnergyDiscount { tiers } => {
                        let Some(tier) = tiers
                            .iter()
                            .rev()
                            .find(|t| self.players[p].xp >= u64::from(t.minimum_xp))
                        else {
                            continue;
                        };
                        (u64::from(tier.energy), 0)
                    }
                    StaticAbility::PerCardEnergyDiscount { amount, minimum } => (
                        u64::from(self.cards_played[p]) * u64::from(amount),
                        minimum.into(),
                    ),
                    _ => continue,
                };
                modifiers.push(modifier(
                    10_000 + i as u64,
                    "Printed discount",
                    source,
                    energy,
                    0,
                    minimum,
                    false,
                    false,
                ));
            }
        }
        for g in grants {
            let (energy, power, increase) = match g.kind {
                PlayGrantKind::DiscountNextSpell { energy, power }
                | PlayGrantKind::DiscountNextCard { energy, power } => (energy, power, false),
                PlayGrantKind::UnitSurchargeThisTurn(n) if !intent.token => (n, 0, true),
                _ => continue,
            };
            modifiers.push(modifier(
                3_000_000 + u64::from(g.id),
                "Delayed play modifier",
                g.source,
                energy.into(),
                power,
                0,
                false,
                increase,
            ));
        }
        let spell = !intent.token && c.kind == CardKind::Spell;
        for owner in 0..2 {
            for u in &self.players[owner].units {
                if u.token {
                    continue;
                }
                let (energy, power, min, choose, increase) = match u.card_id {
                    267 if owner == p && spell && self.unit_empowered(u.id) => {
                        (1, 1, 1, false, false)
                    }
                    269 if owner == p
                        && spell
                        && intent.origin == PlayOrigin::Trash
                        && intent.has_flow =>
                    {
                        (2, 0, 1, false, false)
                    }
                    270 if spell
                        && self
                            .showdown
                            .as_ref()
                            .is_some_and(|w| w.combat && u.lane == Some(w.lane)) =>
                    {
                        (1, 1, if owner == p { 1 } else { 0 }, false, owner != p)
                    }
                    271 if owner == p
                        && spell
                        && all_targets.contains(&Target::Unit {
                            player: p,
                            unit_id: u.id,
                        }) =>
                    {
                        (1, 1, 0, true, false)
                    }
                    _ => continue,
                };
                modifiers.push(modifier(
                    4_000_000 + u64::from(u.id),
                    "Unit cost modifier",
                    ChainSource::Card { card_id: u.card_id },
                    energy,
                    power,
                    min,
                    choose,
                    increase,
                ));
            }
            for g in &self.players[owner].gear {
                if owner != p && !g.token && g.card_id == 268 && g.attached_to.is_none() && spell {
                    modifiers.push(modifier(
                        5_000_000 + u64::from(g.id),
                        "Helm of Suppression",
                        ChainSource::Card { card_id: 268 },
                        1,
                        u16::from(self.unit_empowered(g.id)),
                        0,
                        false,
                        true,
                    ));
                }
            }
        }
        for u in self.players[p]
            .units
            .iter()
            .filter(|u| !u.token && u.card_id == 35)
        {
            for (component_index, component) in
                components.iter().enumerate().filter(|(_, c)| c.optional)
            {
                let mut m = modifier(
                    (1u64 << 63) | (u64::from(u.id) << 31) | component_index as u64,
                    "Ezreal optional cost discount",
                    ChainSource::Card { card_id: 35 },
                    1,
                    1,
                    0,
                    true,
                    false,
                );
                m.scope = CostScope::Component {
                    component_id: component.id,
                };
                modifiers.push(m);
            }
        }
        modifiers.extend(self.battlefield_cost_modifiers(&CostContext {
            player: p,
            card_id: Some(intent.id),
            card_kind: Some(if intent.token {
                if rules::token_is_gear(intent.id) {
                    CardKind::Gear
                } else {
                    CardKind::Unit
                }
            } else {
                c.kind
            }),
            token: intent.token,
            lane: intent.lane,
            origin: Some(intent.origin),
            ability_source: None,
            empower: false,
            gear_activation: false,
            targets: all_targets,
        }));
        solve_cost_plan(
            components,
            modifiers,
            &choices.cost_order,
            spell,
            self.optional_power_discount[p],
        )
    }

    pub(crate) fn pay_active_play(
        &mut self,
        player: usize,
        id: CardId,
    ) -> Option<Result<(), String>> {
        let active = self
            .active_play
            .as_ref()
            .filter(|a| a.player == player && a.card_id == id)?;
        if active.paid {
            return Some(Ok(()));
        }
        let costs = active.costs.clone();
        if costs.energy > u32::from(self.payment_energy_available(player)) {
            return Some(Err("Not enough energy for the selected cost plan.".into()));
        }
        // Preserve the existing automatic card-payment policy: spend stored
        // energy, then tap matching runes before recycling them for power.
        // Explicit prior rune actions and exact power-source choices still win.
        let preferred = if active.origin == PlayOrigin::Token {
            &[][..]
        } else {
            self.catalog[id].domains
        };
        self.pay_energy_preferring(player, costs.energy as u16, preferred);
        for cost in &costs.power {
            if let Err(e) =
                self.consume_power(player, &cost.domains, cost.amount, cost.spell_eligible)
            {
                return Some(Err(e));
            }
        }
        self.active_play.as_mut().unwrap().paid = true;
        Some(Ok(()))
    }

    pub(crate) fn apply_play_choices(
        &mut self,
        action: Action,
        explicit: Option<PlayChoices>,
    ) -> Result<(), String> {
        if self.card_intent(&action).is_err() {
            return self.apply_cost_choices(action, explicit.unwrap_or_default());
        }
        let mut probe = self.clone();
        let mut intent = probe.card_intent(&action)?;
        let automatic = matches!(
            action,
            Action::PlayCard { .. } | Action::PlayChampion { .. } | Action::PlayFromTrash { .. }
        );
        if automatic {
            intent.targets =
                probe.prepare_targets(probe.priority(), intent.id, intent.lane, &intent.targets)?;
        }
        let wrapped = explicit.is_some();
        let choices = explicit.unwrap_or_else(|| intent.legacy.clone());
        if wrapped && !intent.legacy.repeats.is_empty() {
            return Err("Do not combine legacy Repeat with independent Repeat selections.".into());
        }
        let grants = probe.intent_grants(&intent);
        let repeats = probe.repeat_options(&intent, &grants);
        let accel = probe.accelerate_option(&intent, &grants);
        let bonus = match intent.id {
            29 => Some(OptionalCostOption {
                label: String::new(),
                cost: CostValue {
                    xp: 0,
                    power: 1,
                    domains: vec![Domain::Body],
                    ..Default::default()
                },
            }),
            261 => Some(OptionalCostOption {
                label: String::new(),
                cost: CostValue {
                    xp: 0,
                    power: 1,
                    domains: vec![Domain::Calm],
                    ..Default::default()
                },
            }),
            _ => None,
        };
        for r in &choices.repeats {
            if r.choices.mode.is_some() {
                return Err("This spell has no mode selection.".into());
            }
            probe.prepare_targets_explicit(
                probe.priority(),
                intent.id,
                intent.lane,
                &r.choices.targets,
            )?;
            if let PlayOrigin::Hidden { lane } = intent.origin
                && r.choices
                    .targets
                    .iter()
                    .enumerate()
                    .any(|(slot, &t)| !probe.hidden_target_allowed_at(intent.id, lane, slot, t))
            {
                return Err("Hidden Repeat targets must be at the hidden battlefield.".into());
            }
        }
        let cost_snapshot = probe.play_cost_snapshot(&action);
        let costs = cost_snapshot.card_cost_plan(
            &intent,
            &choices,
            &grants,
            &repeats,
            accel.as_ref(),
            bonus.as_ref(),
        )?;
        probe.consume_grants(&grants);
        probe.active_play = Some(ActivePlay {
            player: probe.priority(),
            card_id: intent.id,
            origin: intent.origin,
            choices: choices.clone(),
            costs,
            grants,
            paid: false,
        });
        let inner = without_legacy_options(action);
        probe.apply(inner)?;
        probe.active_play = None;
        probe.payment_play_grants.clear();
        *self = probe;
        Ok(())
    }

    pub(crate) fn is_card_play_action(action: &Action) -> bool {
        matches!(
            action,
            Action::PlayCard { .. }
                | Action::PlayCardWithTargets { .. }
                | Action::PlayCardOptions { .. }
                | Action::PlayChampion { .. }
                | Action::PlayChampionOptions { .. }
                | Action::PlayHidden { .. }
                | Action::PlayFromTrash { .. }
                | Action::PlayFromTrashWithTargets { .. }
                | Action::PlayFromTrashOptions { .. }
                | Action::FinalizePendingPlay { .. }
                | Action::PlaceToken { .. }
                | Action::FinalizeTokenPlay { .. }
        )
    }

    pub fn resolution_progress(&self) -> Option<ResolutionProgress> {
        self.resolution_frame.as_ref().map(|r| ResolutionProgress {
            chain_id: r.chain_id,
            execution: r.execution,
            executions: r.executions,
        })
    }
}

#[allow(clippy::too_many_arguments)] // Mirrors the named descriptor fields at each explicit card mapping.
fn modifier(
    id: u64,
    label: &str,
    source: ChainSource,
    energy: u64,
    power: u16,
    minimum_energy: u16,
    choose: bool,
    increase: bool,
) -> CostModifier {
    CostModifier {
        id,
        label: label.into(),
        source,
        energy,
        power,
        minimum_energy,
        choose_energy_or_power: choose,
        increase,
        scope: CostScope::Total,
    }
}

fn without_legacy_options(action: Action) -> Action {
    match action {
        Action::PlayCardOptions {
            hand_index,
            lane,
            targets,
            ..
        } => Action::PlayCardWithTargets {
            hand_index,
            lane,
            targets,
        },
        Action::PlayChampionOptions { lane, target, .. } => Action::PlayChampion { lane, target },
        Action::PlayFromTrashOptions {
            trash_index,
            lane,
            targets,
            flow,
            ..
        } => Action::PlayFromTrashOptions {
            trash_index,
            lane,
            targets,
            flow,
            repeat_targets: vec![],
            repeat: false,
            accelerate: false,
            bonus: false,
        },
        Action::FinalizePendingPlay { lane, targets, .. } => Action::FinalizePendingPlay {
            lane,
            targets,
            repeat_targets: vec![],
            repeat: false,
            accelerate: false,
            bonus: false,
        },
        a => a,
    }
}

pub(crate) fn solve_cost_plan(
    components: Vec<CostComponent>,
    modifiers: Vec<CostModifier>,
    selected: &[CostSelection],
    spell: bool,
    legacy_power: u8,
) -> Result<CostPlanView, String> {
    let mut work = components.clone();
    let mut order = Vec::new();
    let mut used = Vec::new();
    for s in selected {
        if used.contains(&s.modifier_id) || !modifiers.iter().any(|m| m.id == s.modifier_id) {
            return Err("Choose each existing cost modifier once.".into());
        }
        used.push(s.modifier_id);
    }
    let mut total_seen = false;
    for choice in selected {
        let m = modifiers
            .iter()
            .find(|m| m.id == choice.modifier_id)
            .unwrap();
        if m.increase && (choice.use_power || !choice.power_allocations.is_empty()) {
            return Err("Cost increases do not accept discount choices or allocations.".into());
        }
        if !m.increase {
            match m.scope {
                CostScope::Total => total_seen = true,
                CostScope::Component { .. } if total_seen => {
                    return Err("Component discounts must precede total-cost discounts.".into());
                }
                _ => {}
            }
        }
    }
    let mut sequence: Vec<_> = modifiers
        .iter()
        .filter(|m| m.increase)
        .map(|m| (m, None))
        .collect();
    for component_stage in [true, false] {
        sequence.extend(selected.iter().filter_map(|s| {
            modifiers
                .iter()
                .find(|m| {
                    m.id == s.modifier_id
                        && !m.increase
                        && matches!(m.scope, CostScope::Component { .. }) == component_stage
                })
                .map(|m| (m, Some(s)))
        }));
        sequence.extend(
            modifiers
                .iter()
                .filter(|m| {
                    !m.increase
                        && !used.contains(&m.id)
                        && matches!(m.scope, CostScope::Component { .. }) == component_stage
                })
                .map(|m| (m, None)),
        );
    }
    let mut optional_power_used: Vec<(u64, u8)> = Vec::new();
    for (m, choice) in sequence {
        let use_power = choice.map_or_else(
            || {
                if let CostScope::Component { component_id } = m.scope {
                    let count = optional_power_used
                        .iter()
                        .find(|(id, _)| *id == component_id)
                        .map_or(0, |(_, n)| *n);
                    if m.choose_energy_or_power && count < legacy_power {
                        if let Some((_, n)) = optional_power_used
                            .iter_mut()
                            .find(|(id, _)| *id == component_id)
                        {
                            *n += 1;
                        } else {
                            optional_power_used.push((component_id, 1));
                        }
                        return true;
                    }
                }
                false
            },
            |s| s.use_power,
        );
        if use_power && !m.choose_energy_or_power {
            return Err("This modifier has no Energy-or-Power choice.".into());
        }
        let indices: Vec<_> = work
            .iter()
            .enumerate()
            .filter(|(_, c)| match m.scope {
                CostScope::Total => true,
                CostScope::Component { component_id } => c.id == component_id,
            })
            .map(|(i, _)| i)
            .collect();
        let energy = if m.choose_energy_or_power && use_power {
            0
        } else {
            m.energy
        };
        let power = if m.choose_energy_or_power && !use_power {
            0
        } else {
            m.power
        };
        if m.increase {
            work.push(CostComponent {
                id: m.id,
                label: m.label.clone(),
                optional: false,
                cost: CostValue {
                    xp: 0,
                    energy: u16::try_from(energy)
                        .map_err(|_| crate::numeric::capacity("increased component Energy cost"))?,
                    power,
                    domains: ALL_DOMAINS.to_vec(),
                },
            });
            order.push(CostSelection {
                modifier_id: m.id,
                use_power: false,
                power_allocations: vec![],
            });
            continue;
        }
        let total: u32 = indices
            .iter()
            .map(|&i| u32::from(work[i].cost.energy))
            .sum();
        let mut remaining =
            energy.min(u64::from(total.saturating_sub(u32::from(m.minimum_energy))));
        for &i in &indices {
            let n = remaining.min(u64::from(work[i].cost.energy));
            work[i].cost.energy -= n as u16;
            remaining -= n;
        }
        let capacity: u32 = indices.iter().map(|&i| u32::from(work[i].cost.power)).sum();
        let required = u32::from(power).min(capacity);
        let mut allocations = Vec::new();
        if let Some(s) = choice.filter(|s| !s.power_allocations.is_empty()) {
            let mut ids = Vec::new();
            let mut total = 0u32;
            for a in &s.power_allocations {
                if ids.contains(&a.component_id) {
                    return Err("Do not duplicate a component allocation.".into());
                }
                ids.push(a.component_id);
                let i = *indices
                    .iter()
                    .find(|&&i| work[i].id == a.component_id)
                    .ok_or("Power reduction selects an ineligible component.")?;
                if a.amount > work[i].cost.power {
                    return Err("Power reduction exceeds the component's remaining cost.".into());
                }
                work[i].cost.power -= a.amount;
                total += u32::from(a.amount);
                allocations.push(a.clone());
            }
            if total != required {
                return Err("Allocate the entire usable Power reduction.".into());
            }
        } else {
            let mut left = required;
            for &i in &indices {
                let n = left.min(u32::from(work[i].cost.power)) as u16;
                if n > 0 {
                    work[i].cost.power -= n;
                    left -= u32::from(n);
                    allocations.push(PowerReduction {
                        component_id: work[i].id,
                        amount: n,
                    });
                }
            }
        }
        order.push(CostSelection {
            modifier_id: m.id,
            use_power,
            power_allocations: allocations,
        });
    }
    let xp = work
        .iter()
        .try_fold(0u64, |sum, c| sum.checked_add(c.cost.xp))
        .ok_or("engine_capacity: Total XP cost exceeds the supported integer representation.")?;
    let energy = work.iter().map(|c| u32::from(c.cost.energy)).sum();
    let mut power: Vec<_> = work
        .into_iter()
        .filter(|c| c.cost.power > 0)
        .map(|c| crate::payment::PowerCost {
            domains: if c.cost.domains.is_empty() {
                ALL_DOMAINS.to_vec()
            } else {
                c.cost.domains
            },
            amount: c.cost.power,
            spell_eligible: spell,
        })
        .collect();
    power.sort_by_key(|c| c.domains.len());
    Ok(CostPlanView {
        xp,
        components,
        modifiers,
        energy,
        power,
        applied_order: order,
    })
}

#[derive(Clone, Debug)]
pub(crate) struct ResolutionFrame {
    pub chain_id: u32,
    pub execution: usize,
    pub executions: usize,
    pub started: bool,
}

impl Game {
    pub(crate) fn play_enters_ready(&self, player: usize, card_id: CardId, token: bool) -> bool {
        self.players[player]
            .units
            .iter()
            .any(|u| !u.token && u.card_id == 264)
            || self.play_grants.iter().any(|g| {
                g.player == player
                    && g.expires_turn == self.turn
                    && g.kind == PlayGrantKind::ReadyUnitsThisTurn
            })
            || self.active_play.as_ref().is_some_and(|a| {
                a.player == player
                    && a.card_id == card_id
                    && (a.origin == PlayOrigin::Token) == token
                    && (a.choices.accelerate
                        || a.grants.iter().any(|g| {
                            matches!(
                                g.kind,
                                PlayGrantKind::ReadyNextUnit | PlayGrantKind::ReadyUnitsThisTurn
                            )
                        }))
            })
    }

    pub(crate) fn resolve_spell_executions(&mut self) -> Result<(), String> {
        if self.resolution_frame.is_none() {
            let item = self.chain.last_mut().ok_or("No spell to resolve.")?;
            if item.executions.is_empty() {
                item.executions.push(ExecutionChoices {
                    targets: item.targets.clone(),
                    mode: None,
                });
                if item.options.repeat {
                    item.executions.push(ExecutionChoices {
                        targets: item.repeat_targets.clone(),
                        mode: None,
                    });
                }
            }
            self.resolution_frame = Some(ResolutionFrame {
                chain_id: item.id,
                execution: 0,
                executions: item.executions.len(),
                started: false,
            });
        }
        loop {
            if self.decision.is_some() {
                return Ok(());
            }
            if let Some(choice) = self.pending_decisions.pop_front() {
                self.decision = Some(choice);
                return Ok(());
            }
            let frame = self.resolution_frame.clone().unwrap();
            if frame.started {
                self.resolution_frame.as_mut().unwrap().execution += 1;
                self.resolution_frame.as_mut().unwrap().started = false;
                continue;
            }
            let index = self
                .chain
                .iter()
                .position(|i| i.id == frame.chain_id)
                .ok_or("Resolving spell left the chain unexpectedly.")?;
            if frame.execution >= frame.executions {
                let item = self.chain.remove(index);
                self.resolution_frame = None;
                self.damage_context = None;
                if matches!(rules::effect(item.card_id), OfficialEffect::ExtraTurn)
                    || item.banish_after
                {
                    self.players[item.player].banish.push(item.card_id);
                } else if item.recycle_after {
                    self.players[item.player].deck.insert(0, item.card_id);
                } else {
                    self.players[item.player].discard.push(item.card_id);
                }
                self.after_spell_played(
                    item.player,
                    item.card_id,
                    item.from_hidden || item.recycle_after || item.banish_after,
                    item.from_hidden,
                );
                self.after_resolution()?;
                return Ok(());
            }
            let mut item = self.chain[index].clone();
            item.targets = item.executions[frame.execution].targets.clone();
            item.target = item.targets.first().copied();
            let groups =
                self.resolution_groups(item.player, item.card_id, item.lane, &item.targets)?;
            if rules::targeting(item.card_id).same_location && groups.len() > 1 {
                self.decision = Some(Decision::ResolutionTargets {
                    player: item.player,
                    card_id: item.card_id,
                    groups,
                    repeated: frame.execution != 0,
                    execution: frame.execution,
                });
                return Ok(());
            }
            self.resolution_frame.as_mut().unwrap().started = true;
            self.damage_context = Some(damage::DamageContext {
                player: item.player,
                bonus_damage: item.bonus_damage,
                dealer: damage::DamageDealer::Spell {
                    card_id: item.card_id,
                },
            });
            if self.is_counter(item.card_id) {
                self.resolve_counter(&item);
            } else {
                if item.options.bonus
                    && item.card_id == 29
                    && let Some(&target) = item.targets.first()
                {
                    self.modify_might(target, 2, 0)?;
                }
                self.resolve_operation(item.player, item.card_id, item.lane, item.targets, None)?;
            }
            // CR359.3.e.15: Arcane Shift's final instruction banishes the
            // physical resolving spell. It cannot execute any later Repeat.
            if rules::effect(item.card_id) == OfficialEffect::ArcaneShift {
                let index = self
                    .chain
                    .iter()
                    .position(|i| i.id == frame.chain_id)
                    .ok_or("Resolving spell is missing.")?;
                let item = self.chain.remove(index);
                self.players[item.player].banish.push(item.card_id);
                self.resolution_frame = None;
                self.damage_context = None;
                self.after_spell_played(
                    item.player,
                    item.card_id,
                    item.from_hidden || item.recycle_after || item.banish_after,
                    item.from_hidden,
                );
                self.after_resolution()?;
                return Ok(());
            }
            self.damage_context = None;
        }
    }
}

impl Game {
    pub(crate) fn capture_payment_grants(&mut self, action: &Action) {
        if let Ok(intent) = self.card_intent(action) {
            let grants = self.intent_grants(&intent);
            self.consume_grants(&grants);
            self.payment_play_grants = grants;
        }
    }
}

impl Game {
    fn ability_plan(
        &self,
        action: &Action,
        selection: &[CostSelection],
    ) -> Result<CostPlanView, String> {
        if let Action::ActivateLegend { target } = action {
            let player = self.priority();
            let activation = crate::legends::legend_activation(self.legend[player].kind)
                .ok_or("This legend has no activated ability.")?;
            let mut components = vec![CostComponent {
                id: 1,
                label: "Legend ability".into(),
                cost: CostValue {
                    xp: 0,
                    energy: activation.energy.into(),
                    power: activation.power.into(),
                    domains: activation
                        .domain
                        .map_or_else(|| ALL_DOMAINS.to_vec(), |d| vec![d]),
                },
                optional: false,
            }];
            let deflect = self.deflect_cost(player, &target.iter().copied().collect::<Vec<_>>())?;
            if deflect > 0 {
                components.push(CostComponent {
                    id: 4,
                    label: "Deflect".into(),
                    cost: CostValue {
                        xp: 0,
                        energy: 0,
                        power: deflect.into(),
                        domains: ALL_DOMAINS.to_vec(),
                    },
                    optional: false,
                });
            }
            return solve_cost_plan(components, vec![], selection, false, 0);
        }
        let (player, source, card_id, energy, power, xp, domains, empower, targets) = match action {
            Action::ActivateAbility {
                source,
                ability_index,
                targets,
            }
            | Action::ActivateAbilityWithCosts {
                source,
                ability_index,
                targets,
                ..
            } => {
                let id = match *source {
                    AbilitySource::Unit { player, unit_id } => self
                        .unit_ref(Target::Unit { player, unit_id })
                        .filter(|u| !u.token)
                        .map(|u| u.card_id),
                    AbilitySource::Gear { player, gear_id } => self
                        .players
                        .get(player)
                        .and_then(|p| p.gear.iter().find(|g| g.id == gear_id && !g.token))
                        .map(|g| g.card_id),
                }
                .ok_or("That ability source is missing.")?;
                let r = crate::activated::activated_rules(id)
                    .get(*ability_index)
                    .ok_or("No such ability.")?;
                if r.minimum_xp
                    .is_some_and(|n| self.players[source.player()].xp < u64::from(n))
                {
                    return Err("This dependent ability requires more XP.".into());
                }
                (
                    source.player(),
                    *source,
                    id,
                    r.energy.into(),
                    r.power.into(),
                    r.xp_cost,
                    r.domain.map_or_else(|| ALL_DOMAINS.to_vec(), |d| vec![d]),
                    r.is_empower(),
                    targets.clone(),
                )
            }
            Action::Equip { gear_id, target }
            | Action::EquipWithTrash {
                gear_id, target, ..
            } => {
                let p = self.priority();
                let g = self.players[p]
                    .gear
                    .iter()
                    .find(|g| g.id == *gear_id && !g.token)
                    .ok_or("No such equipment.")?;
                let r = rules::equipment(g.card_id).ok_or("No Equip ability.")?;
                (
                    p,
                    AbilitySource::Gear {
                        player: p,
                        gear_id: *gear_id,
                    },
                    g.card_id,
                    r.equip_energy.into(),
                    r.equip_power.into(),
                    r.equip_xp,
                    vec![r.equip_domain],
                    false,
                    vec![*target],
                )
            }
            Action::EmpowerUnit { unit_id, pay_power } => {
                let p = self.priority();
                let u = self
                    .unit_ref(Target::Unit {
                        player: p,
                        unit_id: *unit_id,
                    })
                    .ok_or("No such unit.")?;
                let (e, power, d) = match u.card_id {
                    24 => {
                        if *pay_power {
                            (0, 1, Domain::Body)
                        } else {
                            (1, 0, Domain::Body)
                        }
                    }
                    45 => (2, 1, Domain::Chaos),
                    _ => return Err("No such Empower ability.".into()),
                };
                (
                    p,
                    AbilitySource::Unit {
                        player: p,
                        unit_id: *unit_id,
                    },
                    u.card_id,
                    e,
                    power,
                    0,
                    vec![d],
                    true,
                    vec![],
                )
            }
            _ => return Err("This action has no selectable cost plan.".into()),
        };
        let mut components = vec![CostComponent {
            id: 1,
            label: "Ability cost".into(),
            cost: CostValue {
                xp,
                energy,
                power,
                domains,
            },
            optional: false,
        }];
        let deflect = self.deflect_cost(player, &targets)?;
        if deflect > 0 {
            components.push(CostComponent {
                id: 4,
                label: "Deflect".into(),
                cost: CostValue {
                    xp: 0,
                    energy: 0,
                    power: deflect.into(),
                    domains: ALL_DOMAINS.to_vec(),
                },
                optional: false,
            });
        }
        let mut modifiers = self.battlefield_cost_modifiers(&CostContext {
            player,
            card_id: Some(card_id),
            card_kind: None,
            token: false,
            lane: match source {
                AbilitySource::Unit { player, unit_id } => self
                    .unit_ref(Target::Unit { player, unit_id })
                    .and_then(|u| u.lane),
                AbilitySource::Gear { player, gear_id } => self.gear_location(player, gear_id),
            },
            origin: None,
            ability_source: Some(source),
            empower,
            gear_activation: matches!(source, AbilitySource::Gear { .. }),
            targets,
        });
        if let Action::ActivateAbility { ability_index, .. }
        | Action::ActivateAbilityWithCosts { ability_index, .. } = action
            && let Some(rule) = crate::activated::activated_rules(card_id).get(*ability_index)
        {
            let runes: u16 = self.players[player]
                .runes
                .iter()
                .map(|&n| u16::from(n))
                .sum();
            for (index, discount) in rule.cost_modifiers.iter().enumerate() {
                let energy = match *discount {
                    crate::activated::AbilityCostModifier::EnergyPerControlledRune { amount } => {
                        amount * runes
                    }
                    crate::activated::AbilityCostModifier::EnergyIfRunesAtMost {
                        maximum,
                        amount,
                    } => {
                        if runes <= maximum.into() {
                            amount
                        } else {
                            0
                        }
                    }
                };
                let object = match source {
                    AbilitySource::Unit { unit_id, .. } => unit_id,
                    AbilitySource::Gear { gear_id, .. } => gear_id,
                };
                modifiers.push(modifier(
                    (1u64 << 62)
                        | (u64::from(object) << 16)
                        | ((*ability_index as u64) << 8)
                        | index as u64,
                    "Printed Empower discount",
                    ChainSource::Card { card_id },
                    energy.into(),
                    0,
                    0,
                    false,
                    false,
                ));
            }
        }
        solve_cost_plan(components, modifiers, selection, false, 0)
    }

    pub(crate) fn pay_action_cost(&mut self, action: &Action) -> Result<(), String> {
        let mut probe = self.clone();
        probe.pay_action_cost_inner(action)?;
        *self = probe;
        Ok(())
    }
    fn pay_action_cost_inner(&mut self, action: &Action) -> Result<(), String> {
        let costs = self
            .payment_frozen_source()
            .unwrap_or(self)
            .ability_plan(action, &self.selected_cost_order)?;
        let p = self.priority();
        if costs.energy > u32::from(self.payment_energy_available(p)) {
            return Err("Not enough energy for this cost plan.".into());
        }
        self.spend_xp(p, costs.xp)?;
        self.pay_energy(p, costs.energy as u16);
        for c in costs.power {
            self.consume_power(p, &c.domains, c.amount, c.spell_eligible)?;
        }
        Ok(())
    }

    fn apply_cost_choices(&mut self, action: Action, choices: PlayChoices) -> Result<(), String> {
        if !choices.repeats.is_empty() || choices.accelerate || choices.bonus {
            return Err("Card additional-cost choices do not apply to this ability.".into());
        }
        self.ability_plan(&action, &choices.cost_order)?;
        let mut probe = self.clone();
        let prior = std::mem::replace(&mut probe.selected_cost_order, choices.cost_order);
        probe.apply(action)?;
        probe.selected_cost_order = prior;
        *self = probe;
        Ok(())
    }
}

impl Game {
    /// Bounded positive policy candidates; this is not a rules-legality oracle.
    pub(crate) fn canonical_play_candidates(&self, limit: usize) -> Result<Vec<Action>, String> {
        Ok({
            if self.decision.is_some() {
                return Ok(vec![]);
            }
            let p = self.priority();
            let mut intents = Vec::new();
            for hand_index in 0..self.players[p].hand.len() {
                intents.push(Action::PlayCard {
                    hand_index,
                    lane: None,
                    target: None,
                });
            }
            if self.players[p].champion.is_some() {
                intents.push(Action::PlayChampion {
                    lane: None,
                    target: None,
                });
            }
            for trash_index in 0..self.players[p].discard.len() {
                if !self.flow_costs(p, trash_index).is_empty() {
                    intents.push(Action::PlayFromTrash {
                        trash_index,
                        lane: None,
                        target: None,
                    });
                }
            }
            let intents: Vec<_> = intents
                .into_iter()
                .flat_map(|action| {
                    if self.card_intent(&action).ok().is_some_and(|intent| {
                        rules::play_target_spec(intent.id).kind
                            == Some(rules::PlayTargetKind::Battlefield)
                    }) {
                        (0..BATTLEFIELDS)
                            .map(|lane| {
                                let mut targeted = action.clone();
                                match &mut targeted {
                                    Action::PlayCard { target, .. }
                                    | Action::PlayChampion { target, .. }
                                    | Action::PlayFromTrash { target, .. } => {
                                        *target = Some(Target::Battlefield { lane })
                                    }
                                    _ => {}
                                }
                                targeted
                            })
                            .collect::<Vec<_>>()
                    } else {
                        vec![action]
                    }
                })
                .collect();
            let mut result = Vec::new();
            for action in intents {
                let Ok(view) = self.play_choices(&action) else {
                    continue;
                };
                let initial = self
                    .card_intent(&action)
                    .map(|intent| intent.targets)
                    .unwrap_or_default();
                let Ok(targets) = self.prepare_targets(p, view.card_id, None, &initial) else {
                    continue;
                };
                let repeat_targets: Vec<Vec<Target>> = if rules::play_target_spec(view.card_id).kind
                    == Some(rules::PlayTargetKind::Battlefield)
                {
                    self.card_target_candidates(p, view.card_id, None, &[])?
                        .into_iter()
                        .map(|target| vec![target])
                        .collect()
                } else {
                    vec![targets.clone()]
                };
                let mut choices = vec![PlayChoices::default()];
                if view.accelerate.is_some() {
                    choices.push(PlayChoices {
                        accelerate: true,
                        ..Default::default()
                    });
                }
                if view.bonus.is_some() {
                    choices.push(PlayChoices {
                        bonus: true,
                        ..Default::default()
                    });
                }
                for r in view.repeats.iter().take(3) {
                    for selected in &repeat_targets {
                        choices.push(PlayChoices {
                            repeats: vec![RepeatSelection {
                                option_id: r.id,
                                choices: ExecutionChoices {
                                    targets: selected.clone(),
                                    mode: None,
                                },
                            }],
                            ..Default::default()
                        });
                    }
                }
                if view.repeats.len() > 1 {
                    choices.push(PlayChoices {
                        repeats: view
                            .repeats
                            .iter()
                            .take(4)
                            .map(|r| RepeatSelection {
                                option_id: r.id,
                                choices: ExecutionChoices {
                                    targets: targets.clone(),
                                    mode: None,
                                },
                            })
                            .collect(),
                        ..Default::default()
                    });
                }
                for choice in choices {
                    let candidate = Action::WithPlayChoices {
                        action: Box::new(action.clone()),
                        choices: choice,
                    };
                    if self.clone().apply(candidate.clone()).is_ok() {
                        result.push(candidate);
                        if result.len() >= limit {
                            return Ok(result);
                        }
                    }
                }
            }
            result
        })
    }
    pub(crate) fn canonical_action_score(&self, action: &Action) -> Result<i128, String> {
        Ok({
            let Ok(intent) = self.card_intent(action) else {
                return Ok(0);
            };
            let extras = intent.legacy.repeats.len() as i128 * 5
                + i128::from(intent.legacy.accelerate) * 15
                + i128::from(intent.legacy.bonus) * 5;
            let score = if matches!(rules::effect(intent.id), OfficialEffect::AreaDamage { .. }) {
                let base = self.area_damage_score(self.priority(), intent.id, &intent.targets)?;
                base + intent
                    .legacy
                    .repeats
                    .iter()
                    .map(|r| self.area_damage_score(self.priority(), intent.id, &r.choices.targets))
                    .collect::<Result<Vec<_>, String>>()?
                    .into_iter()
                    .sum::<i128>()
            } else {
                self.ai_card_score(intent.id)?
            };
            score + extras
        })
    }
}

impl Game {
    pub(crate) fn validate_choice_nesting(action: &Action) -> Result<(), String> {
        match action {
            Action::WithPowerPayment { action, .. } | Action::BeginPayment { action } => {
                Self::validate_choice_nesting(action)
            }
            Action::WithPlayChoices { action, .. }
                if matches!(
                    **action,
                    Action::WithPlayChoices { .. }
                        | Action::WithPowerPayment { .. }
                        | Action::BeginPayment { .. }
                ) =>
            {
                Err("Put payment wrappers outside one canonical play-choice wrapper.".into())
            }
            _ => Ok(()),
        }
    }
    pub(crate) fn restore_reserved_grants(&mut self, key: u32) {
        if let Some(i) = self
            .reserved_play_grants
            .iter()
            .position(|(id, _)| *id == key)
        {
            let (_, grants) = self.reserved_play_grants.remove(i);
            for g in grants {
                if !self.play_grants.iter().any(|existing| existing.id == g.id) {
                    self.play_grants.push(g);
                }
            }
        }
    }
}
