//! Reviewed, metadata-driven triggered abilities and event snapshots.
//! Source identity is independent of the card that supplied an ability: an
//! attached equipment grants its ability to its bearer, which remains its
//! source even if the equipment subsequently detaches.
use super::*;
use crate::trigger_rules::{
    self as metadata, Amount, Condition, Effect, Event, Operand, TriggerRule,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerSource {
    pub player: usize,
    pub unit_id: Option<u32>,
    pub gear_id: Option<u32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerContext {
    pub source: TriggerSource,
    pub subject: Option<Target>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TriggerState {
    pub draws: [u16; 2],
    pub completed_cards: [u16; 2],
    pub first_used: Vec<(TriggerSource, usize)>,
    pub combat_seen: Vec<u32>,
    pub(crate) hidden_play: bool,
}
impl TriggerState {
    pub(super) fn begin_turn(&mut self) {
        self.draws = [0; 2];
        self.completed_cards = [0; 2];
        self.first_used.clear();
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Occurrence {
    UnitPlayed,
    GearPlayed,
    SpellPlayed,
    Ready,
    Exhausted,
    Buffed,
    Moved,
    Death,
    Attack,
    Defend,
    Conquer,
    Hold,
    Draw,
    GearActivated,
}
#[derive(Clone)]
pub(super) struct EventSnapshot {
    pub kind: Occurrence,
    pub player: usize,
    pub subject_player: usize,
    pub unit: Option<Unit>,
    pub card_id: Option<CardId>,
    pub token: bool,
    pub hidden: bool,
    pub origin: Option<usize>,
    pub buffed: bool,
}
impl EventSnapshot {
    pub(super) fn unit(kind: Occurrence, player: usize, unit: Unit) -> Self {
        Self {
            kind,
            player,
            subject_player: player,
            card_id: Some(unit.card_id),
            token: unit.token,
            unit: Some(unit),
            hidden: false,
            origin: None,
            buffed: false,
        }
    }
    pub(super) fn simple(kind: Occurrence, player: usize) -> Self {
        Self {
            kind,
            player,
            subject_player: player,
            unit: None,
            card_id: None,
            token: false,
            hidden: false,
            origin: None,
            buffed: false,
        }
    }
}

impl Game {
    fn trigger_sources(&self) -> Vec<(CardId, TriggerSource)> {
        let mut sources = Vec::new();
        for player in 0..2 {
            for unit in &self.players[player].units {
                if !unit.token && !metadata::trigger_rules(unit.card_id).is_empty() {
                    sources.push((
                        unit.card_id,
                        TriggerSource {
                            player,
                            unit_id: Some(unit.id),
                            gear_id: None,
                        },
                    ));
                }
            }
            for gear in &self.players[player].gear {
                if gear.token || metadata::trigger_rules(gear.card_id).is_empty() {
                    continue;
                }
                // Equipment Effect Text is active only on an attached bearer.
                if rules::equipment(gear.card_id).is_some() && gear.attached_to.is_none() {
                    continue;
                }
                sources.push((
                    gear.card_id,
                    TriggerSource {
                        player,
                        unit_id: gear.attached_to,
                        gear_id: Some(gear.id),
                    },
                ));
            }
        }
        sources
    }
    fn source_target(source: TriggerSource) -> Option<Target> {
        source.unit_id.map(|unit_id| Target::Unit {
            player: source.player,
            unit_id,
        })
    }
    fn source_location(&self, source: TriggerSource) -> Option<(Option<usize>, usize)> {
        if let Some(target) = Self::source_target(source) {
            return self.target_location(target);
        }
        source
            .gear_id
            .and_then(|id| self.players[source.player].gear.iter().find(|g| g.id == id))
            .map(|_| (None, source.player))
    }
    fn unit_has_tag(&self, unit: &Unit, tag: &str) -> bool {
        if unit.token {
            rules::token_spec(unit.card_id).is_some_and(|spec| spec.tag == Some(tag))
        } else {
            self.catalog[unit.card_id].tags.contains(&tag)
        }
    }
    fn event_matches(
        &self,
        source: TriggerSource,
        rule: &TriggerRule,
        event: &EventSnapshot,
    ) -> bool {
        let own = source.player == event.player;
        let same = event
            .unit
            .as_ref()
            .is_some_and(|u| source.unit_id == Some(u.id));
        let played_card = matches!(
            event.kind,
            Occurrence::UnitPlayed | Occurrence::GearPlayed | Occurrence::SpellPlayed
        ) && !event.token;
        let event_ok = match rule.event {
            Event::SelfPlayed => same && event.kind == Occurrence::UnitPlayed,
            Event::FriendlyUnitPlayed => own && event.kind == Occurrence::UnitPlayed,
            Event::FriendlyGearPlayed => own && event.kind == Occurrence::GearPlayed,
            Event::FriendlySpellPlayed => own && event.kind == Occurrence::SpellPlayed,
            Event::FriendlyCardPlayed => own && played_card,
            Event::FriendlyHiddenPlayed => own && played_card && event.hidden,
            Event::SelfReady => same && event.kind == Occurrence::Ready,
            Event::FriendlyReady => {
                own && event.kind == Occurrence::Ready
                    && event.unit.as_ref().is_some_and(|u| {
                        self.players[event.player]
                            .units
                            .iter()
                            .any(|x| x.id == u.id)
                    })
            }
            Event::SelfExhausted => same && event.kind == Occurrence::Exhausted,
            Event::SelfBuffed => own && same && event.kind == Occurrence::Buffed,
            Event::SelfMoved => same && event.kind == Occurrence::Moved,
            Event::SelfDeath => same && event.kind == Occurrence::Death,
            Event::FriendlyDeath => own && event.kind == Occurrence::Death,
            Event::EnemyDeath => !own && event.kind == Occurrence::Death,
            Event::SelfAttack => same && event.kind == Occurrence::Attack,
            Event::SelfDefend => same && event.kind == Occurrence::Defend,
            Event::FriendlyAttack => own && event.kind == Occurrence::Attack,
            Event::FriendlyDefend => own && event.kind == Occurrence::Defend,
            Event::SelfConquer => same && event.kind == Occurrence::Conquer,
            Event::SelfHold => same && event.kind == Occurrence::Hold,
            Event::FriendlyDraw => own && event.kind == Occurrence::Draw,
            Event::GearActivated => own && event.kind == Occurrence::GearActivated,
        };
        if !event_ok {
            return false;
        }
        rule.conditions.iter().all(|condition| match *condition {
            Condition::Other => !same,
            Condition::FromBattlefield => event.origin.is_some(),
            Condition::ToBattlefield => event.unit.as_ref().is_some_and(|u| u.lane.is_some()),
            Condition::PowerAtLeast(n) => event
                .card_id
                .is_some_and(|id| self.catalog[id].power_cost >= n),
            Condition::EnergyAtLeast(n) => {
                event.card_id.is_some_and(|id| self.catalog[id].cost >= n)
            }
            Condition::SecondCard => self.trigger_state.completed_cards[event.player] == 2,
            Condition::OpponentTurn => event.player != self.active,
            Condition::NonRecruit => event
                .unit
                .as_ref()
                .is_some_and(|u| !self.unit_has_tag(u, "Recruit")),
            Condition::Buffed => event.buffed,
            Condition::StunnedHere => event.unit.as_ref().is_some_and(|u| {
                u.stunned
                    && self.source_location(source)
                        == Some((
                            u.lane,
                            if u.lane.is_none() {
                                event.subject_player
                            } else {
                                0
                            },
                        ))
            }),
            Condition::OtherFriendlyHere(n) => Self::source_target(source)
                .and_then(|t| self.unit_ref(t))
                .is_some_and(|unit| {
                    self.players[source.player]
                        .units
                        .iter()
                        .filter(|u| u.id != unit.id && u.lane == unit.lane)
                        .count()
                        == n as usize
                }),
            Condition::Alone => event.unit.as_ref().is_some_and(|unit| {
                self.players[event.player]
                    .units
                    .iter()
                    .filter(|u| u.lane == unit.lane)
                    .count()
                    == 1
            }),
            Condition::FirstEachTurn => true,
            Condition::SecondDraw => self.trigger_state.draws[event.player] == 2,
        })
    }
    fn queue_card_trigger(&mut self, card_id: CardId, rule_index: usize, context: TriggerContext) {
        let rule = &metadata::trigger_rules(card_id)[rule_index];
        if rule.conditions.contains(&Condition::FirstEachTurn) {
            if self
                .trigger_state
                .first_used
                .contains(&(context.source, rule_index))
            {
                return;
            }
            self.trigger_state
                .first_used
                .push((context.source, rule_index));
        }
        if rule.targets.max > 0 || rule.optional {
            self.queue_trigger_choice(
                context.source.player,
                card_id,
                Decision::TriggerTargets {
                    player: context.source.player,
                    card_id,
                    rule_index,
                    context,
                },
            );
        } else {
            self.queue_ability(
                context.source.player,
                card_id,
                Vec::new(),
                ChainAbility::CardTrigger {
                    rule_index,
                    context,
                },
            );
        }
    }
    pub(super) fn emit_event(&mut self, event: EventSnapshot) {
        for (card_id, source) in self.trigger_sources() {
            for (rule_index, rule) in metadata::trigger_rules(card_id).iter().enumerate() {
                if rule.event == Event::SelfDeath {
                    continue;
                }
                if self.event_matches(source, rule, &event) {
                    let subject = event.unit.as_ref().map(|u| Target::Unit {
                        player: event.subject_player,
                        unit_id: u.id,
                    });
                    self.queue_card_trigger(
                        card_id,
                        rule_index,
                        TriggerContext { source, subject },
                    );
                }
            }
        }
    }
    /// Capture Deathknell while abilities still exist; other death observers
    /// are evaluated only after all simultaneous dying units have left.
    pub(super) fn capture_death_triggers(&mut self, deaths: &[(usize, Unit)]) {
        for (card_id, source) in self.trigger_sources() {
            for (player, unit) in deaths {
                let mut event = EventSnapshot::unit(Occurrence::Death, *player, unit.clone());
                event.buffed = self.buffed_units.contains(&unit.id);
                for (index, rule) in metadata::trigger_rules(card_id).iter().enumerate() {
                    if rule.event == Event::SelfDeath && self.event_matches(source, rule, &event) {
                        self.queue_card_trigger(
                            card_id,
                            index,
                            TriggerContext {
                                source,
                                subject: Some(Target::Unit {
                                    player: *player,
                                    unit_id: unit.id,
                                }),
                            },
                        );
                    }
                }
            }
        }
    }
    pub(super) fn emit_unit_event(&mut self, kind: Occurrence, player: usize, unit_id: u32) {
        if let Some(unit) = self.players[player]
            .units
            .iter()
            .find(|u| u.id == unit_id)
            .cloned()
        {
            let mut event = EventSnapshot::unit(kind, player, unit);
            if kind == Occurrence::UnitPlayed {
                event.hidden = self.trigger_state.hidden_play;
                if !event.token {
                    self.trigger_state.completed_cards[player] += 1;
                }
            }
            self.emit_event(event);
        }
    }
    pub(super) fn emit_play_event(
        &mut self,
        kind: Occurrence,
        player: usize,
        card_id: CardId,
        token: bool,
        hidden: bool,
    ) {
        if !token {
            self.trigger_state.completed_cards[player] += 1;
        }
        let mut event = EventSnapshot::simple(kind, player);
        event.card_id = Some(card_id);
        event.token = token;
        event.hidden = hidden;
        self.emit_event(event);
    }
    pub(crate) fn gear_play_completed(&mut self, player: usize, card_id: CardId, token: bool) {
        self.emit_play_event(
            Occurrence::GearPlayed,
            player,
            card_id,
            token,
            self.trigger_state.hidden_play,
        );
        if token {
            self.trigger_batch += 1;
        }
    }
    pub(crate) fn gear_ability_used(&mut self, player: usize) {
        self.emit_event(EventSnapshot::simple(Occurrence::GearActivated, player));
    }
    pub(crate) fn buff_unit(&mut self, target: Target) {
        let Target::Unit { player, .. } = target else {
            return;
        };
        self.buff_unit_by(player, target);
    }
    pub(crate) fn buff_unit_by(&mut self, actor: usize, target: Target) {
        let Target::Unit { player, unit_id } = target else {
            return;
        };
        if self.unit_ref(target).is_none() || self.buffed_units.contains(&unit_id) {
            return;
        }
        self.buffed_units.push(unit_id);
        self.unit_mut(target).unwrap().buff += 1;
        let mut event = EventSnapshot::unit(
            Occurrence::Buffed,
            actor,
            self.unit_ref(target).unwrap().clone(),
        );
        event.subject_player = player;
        self.emit_event(event);
    }
    pub(crate) fn exhaust_unit(&mut self, player: usize, unit_id: u32) {
        let Some(unit) = self.unit_mut(Target::Unit { player, unit_id }) else {
            return;
        };
        let changed = unit.ready;
        unit.ready = false;
        if changed {
            self.emit_unit_event(Occurrence::Exhausted, player, unit_id);
        }
    }
    pub(crate) fn ready_unit_by(&mut self, actor: usize, owner: usize, unit_id: u32) {
        let Some(unit) = self
            .unit_ref(Target::Unit {
                player: owner,
                unit_id,
            })
            .cloned()
        else {
            return;
        };
        if unit.ready {
            return;
        }
        self.unit_mut(Target::Unit {
            player: owner,
            unit_id,
        })
        .unwrap()
        .ready = true;
        if actor == owner && !unit.token && unit.card_id == 0 {
            self.queue_ability(
                owner,
                0,
                Vec::new(),
                ChainAbility::Triggered(TriggeredAbility::Might { unit_id, amount: 1 }),
            );
        }
        let mut event = EventSnapshot::unit(Occurrence::Ready, actor, unit);
        // The acting player and the subject owner differ for enemy Readies.
        // Source identity remains globally unique, while FriendlyReady checks
        // that the actor actually controls this unit.
        event.unit.as_mut().unwrap().ready = true;
        event.subject_player = owner;
        self.emit_event(event);
    }
    pub(crate) fn fight_units(&mut self, a: Target, b: Target) {
        let Some((am, bm)) = self
            .unit_ref(a)
            .zip(self.unit_ref(b))
            .map(|(a, b)| (self.unit_might(a), self.unit_might(b)))
        else {
            return;
        };
        self.unit_mut(a).unwrap().damage = self.unit_ref(a).unwrap().damage.saturating_add(bm);
        self.unit_mut(b).unwrap().damage = self.unit_ref(b).unwrap().damage.saturating_add(am);
    }
    pub(super) fn sync_combat_triggers(&mut self) {
        let Some(window) = self.showdown.as_ref().filter(|w| w.combat) else {
            return;
        };
        let (lane, attacker) = (window.lane, window.attacker);
        let entrants: Vec<_> = [attacker, 1 - attacker]
            .into_iter()
            .flat_map(|p| {
                self.players[p]
                    .units
                    .iter()
                    .filter(move |u| u.lane == Some(lane))
                    .map(move |u| (p, u.clone()))
            })
            .collect();
        for (player, unit) in entrants {
            if self.trigger_state.combat_seen.contains(&unit.id) {
                continue;
            }
            self.trigger_state.combat_seen.push(unit.id);
            self.emit_event(EventSnapshot::unit(
                if player == attacker {
                    Occurrence::Attack
                } else {
                    Occurrence::Defend
                },
                player,
                unit,
            ));
        }
    }
    fn trigger_rule_target_valid(
        &self,
        card_id: CardId,
        index: usize,
        context: TriggerContext,
        target: Target,
        slot: usize,
    ) -> bool {
        let rule = &metadata::trigger_rules(card_id)[index];
        let Target::Unit { player, unit_id } = target else {
            return false;
        };
        let Some(unit) = self.unit_ref(target) else {
            return false;
        };
        let actor = context.source.player;
        // Shared target prohibitions are independent of the operation itself.
        if !self.official_target_valid(actor, 7, None, target, slot) {
            return false;
        }
        match rule.targets.allegiance {
            Allegiance::Friendly if player != actor => return false,
            Allegiance::Enemy if player == actor => return false,
            Allegiance::Mixed if (slot == 0) != (player == actor) => return false,
            _ => {}
        }
        if rule.targets.another && context.source.unit_id == Some(unit_id) {
            return false;
        }
        if rule.targets.battlefield && unit.lane.is_none() {
            return false;
        }
        if rule.targets.here && self.source_location(context.source) != self.target_location(target)
        {
            return false;
        }
        if rule
            .targets
            .max_might
            .is_some_and(|n| self.unit_might(unit) > n)
        {
            return false;
        }
        if rule
            .targets
            .tag
            .is_some_and(|tag| !self.unit_has_tag(unit, tag))
        {
            return false;
        }
        if rule.event == Event::SelfPlayed
            && let Some(source) = context.source.unit_id
            && let Some(lane) = self.hidden_play_origin(source)
            && (self
                .unit_ref(Target::Unit {
                    player: actor,
                    unit_id: source,
                })
                .is_none()
                || unit.lane != Some(lane))
        {
            return false;
        }
        true
    }
    pub fn trigger_target_bounds(&self) -> Option<(usize, usize)> {
        let Decision::TriggerTargets {
            card_id,
            rule_index,
            ..
        } = self.decision.as_ref()?
        else {
            return None;
        };
        let rule = &metadata::trigger_rules(*card_id)[*rule_index];
        Some((
            rule.targets.max as usize,
            if rule.optional {
                0
            } else {
                rule.targets.min as usize
            },
        ))
    }
    pub fn valid_trigger_target(&self, target: Target, prior: &[Target]) -> bool {
        let Some(Decision::TriggerTargets {
            card_id,
            rule_index,
            context,
            ..
        }) = self.decision.as_ref()
        else {
            return false;
        };
        let rule = &metadata::trigger_rules(*card_id)[*rule_index];
        if prior.len() >= rule.targets.max as usize
            || (rule.targets.distinct && prior.contains(&target))
        {
            return false;
        }
        if !self.trigger_rule_target_valid(*card_id, *rule_index, *context, target, prior.len()) {
            return false;
        }
        let mut targets = prior.to_vec();
        targets.push(target);
        self.available_power(context.source.player, None)
            >= self.deflect_cost(context.source.player, &targets)
    }
    pub fn trigger_target_candidates(&self) -> Vec<Target> {
        self.players
            .iter()
            .enumerate()
            .flat_map(|(player, p)| {
                p.units.iter().map(move |u| Target::Unit {
                    player,
                    unit_id: u.id,
                })
            })
            .collect()
    }
    pub(super) fn trigger_target_actions(&self) -> Vec<Action> {
        let Some((max, min)) = self.trigger_target_bounds() else {
            return Vec::new();
        };
        let mut sets = vec![Vec::new()];
        let mut actions = Vec::new();
        let candidates = self.trigger_target_candidates();
        for count in 0..=max {
            if count >= min {
                actions.extend(
                    sets.iter()
                        .cloned()
                        .map(|targets| Action::ChooseTriggerTargets { targets }),
                );
            }
            let mut next = Vec::new();
            for prior in &sets {
                for &target in &candidates {
                    if self.valid_trigger_target(target, prior) {
                        let mut v = prior.clone();
                        v.push(target);
                        next.push(v);
                    }
                }
            }
            sets = next;
        }
        actions
    }
    pub(super) fn choose_trigger_targets(&mut self, targets: Vec<Target>) -> Result<(), String> {
        let Some(Decision::TriggerTargets {
            player,
            card_id,
            rule_index,
            context,
        }) = self.decision.clone()
        else {
            return Err("No triggered ability is choosing targets.".into());
        };
        let rule = &metadata::trigger_rules(card_id)[rule_index];
        if rule.optional && targets.is_empty() {
            self.decision = None;
            return Ok(());
        }
        if targets.len() < rule.targets.min as usize || targets.len() > rule.targets.max as usize {
            return Err("Choose the required number of targets.".into());
        }
        for (index, &target) in targets.iter().enumerate() {
            if !self.valid_trigger_target(target, &targets[..index]) {
                return Err("That target is not legal or its Deflect cannot be paid.".into());
            }
        }
        self.spend_deflect(player, false, &targets)?;
        self.decision = None;
        self.queue_finalized_ability(
            player,
            card_id,
            targets,
            ChainAbility::CardTrigger {
                rule_index,
                context,
            },
        );
        Ok(())
    }
    pub(super) fn discard_unfinalizable_trigger(&mut self) {
        if !matches!(self.decision, Some(Decision::TriggerTargets { .. })) {
            return;
        }
        let mut probe = self.clone();
        // Additional resource abilities may be used during finalization.
        let player = probe.priority();
        let ids: Vec<_> = probe.players[player]
            .gear
            .iter()
            .filter(|g| probe.gear_can_activate(player, g.id))
            .map(|g| g.id)
            .collect();
        for id in ids {
            let _ = probe.gear_activate(player, id);
        }
        if probe.trigger_target_actions().is_empty() {
            self.decision = None;
            self.push_log(
                "A triggered ability cannot finalize because no payable legal targets remain."
                    .into(),
            );
        }
    }
    pub(super) fn trigger_loot(card_id: CardId, index: usize) -> Option<(u8, u8)> {
        metadata::trigger_rules(card_id)
            .get(index)?
            .effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Loot { discard, draw } => Some((*discard, *draw)),
                _ => None,
            })
    }
    fn trigger_operands(
        &self,
        context: TriggerContext,
        who: Operand,
        targets: &[Target],
    ) -> Vec<Target> {
        match who {
            Operand::Source => Self::source_target(context.source).into_iter().collect(),
            Operand::Subject => context.subject.into_iter().collect(),
            Operand::Targets => targets.to_vec(),
            Operand::AllHere => {
                self.source_location(context.source)
                    .map_or(Vec::new(), |location| {
                        self.players
                            .iter()
                            .enumerate()
                            .flat_map(|(player, p)| {
                                p.units.iter().map(move |u| Target::Unit {
                                    player,
                                    unit_id: u.id,
                                })
                            })
                            .filter(|&target| self.target_location(target) == Some(location))
                            .collect()
                    })
            }
        }
    }
    pub(super) fn resolve_card_trigger(
        &mut self,
        player: usize,
        card_id: CardId,
        index: usize,
        context: TriggerContext,
        targets: &[Target],
    ) {
        let rule = &metadata::trigger_rules(card_id)[index];
        let targets: Vec<_> = targets
            .iter()
            .enumerate()
            .filter_map(|(slot, &target)| {
                self.trigger_rule_target_valid(card_id, index, context, target, slot)
                    .then_some(target)
            })
            .collect();
        for effect in rule.effects {
            match *effect {
                Effect::Xp(amount) => {
                    let count = match amount {
                        Amount::Fixed(n) => n,
                        Amount::FriendlyUnits => self.players[player].units.len().min(255) as u8,
                    };
                    self.players[player].xp = self.players[player].xp.saturating_add(count);
                }
                Effect::Draw(n) => self.draw(player, n),
                Effect::DrawBoth(n) => {
                    self.draw(player, n);
                    self.draw(1 - player, n);
                }
                Effect::Loot { discard, draw } => self.enqueue_decision(Decision::Discard {
                    player,
                    count: discard,
                    draw_after: draw,
                }),
                Effect::Might {
                    who,
                    amount,
                    minimum,
                } => {
                    for target in self.trigger_operands(context, who, &targets) {
                        self.modify_might(target, amount as i16, minimum);
                    }
                }
                Effect::Ready(who) => {
                    for target in self.trigger_operands(context, who, &targets) {
                        if let Target::Unit {
                            player: owner,
                            unit_id,
                        } = target
                        {
                            self.ready_unit_by(player, owner, unit_id);
                        }
                    }
                }
                Effect::Buff(who) => {
                    for target in self.trigger_operands(context, who, &targets) {
                        self.buff_unit_by(player, target);
                    }
                }
                Effect::Damage(amount) => {
                    for &target in &targets {
                        let amount = self.battlefield_damage(target, amount);
                        if let Some(unit) = self.unit_mut(target) {
                            unit.damage = unit.damage.saturating_add(amount);
                        }
                    }
                }
                Effect::Bounce => {
                    for &target in &targets {
                        self.bounce(target);
                    }
                }
                Effect::FightSource => {
                    if let (Some(source), Some(&target)) =
                        (Self::source_target(context.source), targets.first())
                    {
                        self.fight_units(source, target);
                    }
                }
                Effect::Stun => {
                    for &target in &targets {
                        if let Some(unit) = self.unit_mut(target) {
                            unit.stunned = true;
                        }
                    }
                }
                Effect::Predict => self.predict(player),
                Effect::Tokens {
                    count,
                    might,
                    gear,
                    opponent,
                    location,
                } => {
                    let owner = if opponent { 1 - player } else { player };
                    let location = match location {
                        rules::TokenPlacement::Default => None,
                        rules::TokenPlacement::Base => Some(None),
                        rules::TokenPlacement::Here => {
                            let Some((lane, _)) = self.source_location(context.source) else {
                                continue;
                            };
                            Some(lane)
                        }
                    };
                    for _ in 0..count {
                        self.queue_token_play(owner, card_id, might, location, gear);
                    }
                }
                Effect::Points(n) => {
                    self.players[player].score = self.players[player].score.saturating_add(n);
                }
            }
            self.trigger_batch += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opponent_buff_is_not_the_simian_controllers_buff_action() {
        // CR412.4/427.1.c distinguishes "when you buff me" from "become buffed".
        let mut game = Game::new(3, 3, 923, Difficulty::Tactical);
        game.start_match().unwrap();
        let id = game.create_unit(1, 158, false, 0);
        let target = Target::Unit {
            player: 1,
            unit_id: id,
        };
        game.buff_unit_by(0, target);
        game.after_resolution();
        assert_eq!(game.unit_ref(target).unwrap().buff, 1);
        assert!(!game.unit_ref(target).unwrap().ready);
        assert!(game.chain.is_empty());
    }
}
