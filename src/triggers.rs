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
pub struct MoveFacts {
    pub origin: Option<usize>,
    pub destination: Option<usize>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerContext {
    pub source: TriggerSource,
    pub subject: Option<Target>,
    pub movement: Option<MoveFacts>,
    pub departure_location: Option<board_targets::BoardLocation>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TriggerState {
    pub draws: [u16; 2],
    pub completed_cards: [u16; 2],
    pub xp_gained: [bool; 2],
    pub first_used: Vec<(TriggerSource, usize)>,
    pub combat_seen: Vec<u32>,
    pub(crate) hidden_play: bool,
}
impl TriggerState {
    pub(super) fn begin_turn(&mut self) {
        self.draws = [0; 2];
        self.completed_cards = [0; 2];
        self.xp_gained = [false; 2];
        self.first_used.clear();
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Occurrence {
    ControllerBeginning,
    GearDeath,
    UnitPlayed,
    GearPlayed,
    SpellPlayed,
    Ready,
    Exhausted,
    Buffed,
    Empowered,
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
    pub gear_id: Option<u32>,
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
            gear_id: None,
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
            gear_id: None,
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
    fn trigger_sources(&self) -> Vec<(CardId, usize, TriggerSource)> {
        let mut sources = Vec::new();
        for player in 0..2 {
            for unit in &self.players[player].units {
                if unit.token {
                    continue;
                }
                for (index, rule) in metadata::trigger_rules(unit.card_id).iter().enumerate() {
                    if rule.text_area == metadata::TriggerTextArea::Rules {
                        sources.push((
                            unit.card_id,
                            index,
                            TriggerSource {
                                player,
                                unit_id: Some(unit.id),
                                gear_id: None,
                            },
                        ));
                    }
                }
            }
            for gear in &self.players[player].gear {
                if gear.token {
                    continue;
                }
                for (index, rule) in metadata::trigger_rules(gear.card_id).iter().enumerate() {
                    let unit_id = match rule.text_area {
                        metadata::TriggerTextArea::Rules if gear.attached_to.is_none() => None,
                        metadata::TriggerTextArea::Equipment if gear.attached_to.is_some() => {
                            gear.attached_to
                        }
                        _ => continue,
                    };
                    sources.push((
                        gear.card_id,
                        index,
                        TriggerSource {
                            player,
                            unit_id,
                            gear_id: Some(gear.id),
                        },
                    ));
                }
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
        source.gear_id.and_then(|gear_id| {
            self.target_location(Target::Gear {
                player: source.player,
                gear_id,
            })
        })
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
            Event::ControllerBeginning => own && event.kind == Occurrence::ControllerBeginning,
            Event::SelfPlayed => {
                (same && event.kind == Occurrence::UnitPlayed)
                    || (event.gear_id.is_some()
                        && event.gear_id == source.gear_id
                        && event.kind == Occurrence::GearPlayed)
            }
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
            Event::SelfEmpowered => same && event.kind == Occurrence::Empowered,
            Event::SelfMoved => same && event.kind == Occurrence::Moved,
            Event::SelfDeath => {
                (same && event.kind == Occurrence::Death)
                    || (event.kind == Occurrence::GearDeath
                        && event.gear_id.is_some()
                        && event.gear_id == source.gear_id
                        && source.unit_id.is_none())
            }
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
            Condition::SourceEmpowered => source
                .unit_id
                .or(source.gear_id)
                .is_some_and(|id| self.unit_empowered(id)),
            Condition::ControllerXpAtLeast(n) => self.players[source.player].xp >= u64::from(n),
            Condition::ControllerGearAtLeast(n) => {
                self.players[source.player].gear.len() >= usize::from(n)
            }
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
        if event.kind == Occurrence::UnitPlayed {
            self.legend_on_unit_completed(event.player);
        }
        if self.trigger_state.completed_cards[event.player] == 1
            && !event.token
            && matches!(
                event.kind,
                Occurrence::UnitPlayed | Occurrence::GearPlayed | Occurrence::SpellPlayed
            )
        {
            let sources: Vec<_> = self.players[event.player]
                .units
                .iter()
                .filter(|u| !u.token && u.card_id == 258 && u.lane.is_some())
                .map(|u| u.id)
                .collect();
            for id in sources {
                self.queue_ability(
                    event.player,
                    258,
                    vec![],
                    ChainAbility::CardEffect { entering: Some(id) },
                );
            }
        }

        if event.kind == Occurrence::SpellPlayed
            && !event.token
            && let Some(card_id) = event.card_id
        {
            self.legend_on_spell_completed(event.player, card_id);
        }
        for (card_id, rule_index, source) in self.trigger_sources() {
            let rule = &metadata::trigger_rules(card_id)[rule_index];
            if rule.event == Event::SelfDeath {
                continue;
            }
            if self.event_matches(source, rule, &event) {
                let subject = event
                    .unit
                    .as_ref()
                    .map(|u| Target::Unit {
                        player: event.subject_player,
                        unit_id: u.id,
                    })
                    .or_else(|| {
                        event.gear_id.map(|gear_id| Target::Gear {
                            player: event.subject_player,
                            gear_id,
                        })
                    });
                self.queue_card_trigger(
                    card_id,
                    rule_index,
                    TriggerContext {
                        source,
                        subject,
                        departure_location: None,
                        movement: (event.kind == Occurrence::Moved).then(|| MoveFacts {
                            origin: event.origin,
                            destination: event.unit.as_ref().and_then(|u| u.lane),
                        }),
                    },
                );
            }
        }
    }
    /// Capture Deathknell while abilities still exist; other death observers
    /// are evaluated only after all simultaneous dying units have left.
    pub(super) fn capture_death_triggers(&mut self, deaths: &[(usize, Unit)]) {
        for (card_id, index, source) in self.trigger_sources() {
            for (player, unit) in deaths {
                let mut event = EventSnapshot::unit(Occurrence::Death, *player, unit.clone());
                event.buffed = self.buffed_units.contains(&unit.id);
                let rule = &metadata::trigger_rules(card_id)[index];
                if rule.event == Event::SelfDeath && self.event_matches(source, rule, &event) {
                    self.queue_card_trigger(
                        card_id,
                        index,
                        TriggerContext {
                            source,
                            movement: None,
                            departure_location: Some(board_targets::BoardLocation::from_lane(
                                *player, unit.lane,
                            )),
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
    pub(crate) fn capture_gear_death(&mut self, player: usize, gear_id: u32) {
        // Capture the source and panel activation before departure; no unit-death observers.
        let Some(gear) = self
            .players
            .get(player)
            .and_then(|p| p.gear.iter().find(|g| g.id == gear_id))
            .cloned()
        else {
            return;
        };
        let mut event = EventSnapshot::simple(Occurrence::GearDeath, player);
        event.gear_id = Some(gear_id);
        event.card_id = Some(gear.card_id);
        event.token = gear.token;
        for (card_id, index, source) in self.trigger_sources() {
            let rule = &metadata::trigger_rules(card_id)[index];
            if rule.event == Event::SelfDeath && self.event_matches(source, rule, &event) {
                self.queue_card_trigger(
                    card_id,
                    index,
                    TriggerContext {
                        source,
                        subject: Some(Target::Gear { player, gear_id }),
                        movement: None,
                        departure_location: self
                            .board_target_facts(Target::Gear { player, gear_id })
                            .map(|facts| facts.location),
                    },
                );
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
    pub(crate) fn gear_play_completed(
        &mut self,
        player: usize,
        card_id: CardId,
        gear_id: u32,
        token: bool,
    ) {
        if !token {
            self.trigger_state.completed_cards[player] += 1;
        }
        let mut event = EventSnapshot::simple(Occurrence::GearPlayed, player);
        event.card_id = Some(card_id);
        event.gear_id = Some(gear_id);
        event.token = token;
        event.hidden = self.trigger_state.hidden_play;
        self.emit_event(event);
        if token {
            self.trigger_batch += 1;
        }
    }
    pub(crate) fn gear_ability_used(&mut self, player: usize) {
        // Each completed use is a distinct chronological event, including
        // nested Add abilities whose observers wait for the outer task.
        self.trigger_batch += 1;
        self.emit_event(EventSnapshot::simple(Occurrence::GearActivated, player));
        self.trigger_batch += 1;
    }
    pub(crate) fn buff_unit(&mut self, target: Target) -> Result<(), String> {
        let Target::Unit { player, .. } = target else {
            return Ok(());
        };
        self.buff_unit_by(player, target)?;

        Ok(())
    }
    pub(crate) fn buff_unit_by(&mut self, actor: usize, target: Target) -> Result<(), String> {
        let Target::Unit { player, unit_id } = target else {
            return Ok(());
        };
        if self.unit_ref(target).is_none() || self.buffed_units.contains(&unit_id) {
            return Ok(());
        }
        self.buffed_units.push(unit_id);
        let unit = self.unit_mut(target).unwrap();
        unit.buff = crate::numeric::add_u64(unit.buff, 1, "Buff counter")?;
        let mut event = EventSnapshot::unit(
            Occurrence::Buffed,
            actor,
            self.unit_ref(target).unwrap().clone(),
        );
        event.subject_player = player;
        self.emit_event(event);

        Ok(())
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
    pub(crate) fn fight_units(&mut self, a: Target, b: Target) -> Result<(), String> {
        let Some((first, second)) = self.unit_ref(a).zip(self.unit_ref(b)) else {
            return Ok(());
        };
        let am = self.unit_might(first)?;
        let bm = self.unit_might(second)?;
        // Snapshot both dealers before marking either packet; cleanup waits for the instruction.
        let ad = crate::numeric::add_u64(first.damage, bm, "Fight damage")?;
        let bd = crate::numeric::add_u64(second.damage, am, "Fight damage")?;
        self.unit_mut(a).unwrap().damage = ad;
        self.unit_mut(b).unwrap().damage = bd;
        Ok(())
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
    ) -> Result<bool, String> {
        Ok({
            let rule = &metadata::trigger_rules(card_id)[index];
            let Some(facts) = self.board_target_facts(target) else {
                return Ok(false);
            };
            let actor = context.source.player;
            let player = facts.controller;
            if !self.board_target_matches_kind(target, rule.targets.kind)
                || !self.can_choose_board_target(actor, target)
            {
                return Ok(false);
            }
            match rule.targets.allegiance {
                Allegiance::Friendly if player != actor => return Ok(false),
                Allegiance::Enemy if player == actor => return Ok(false),
                Allegiance::Mixed if (slot == 0) != (player == actor) => return Ok(false),
                _ => {}
            }
            let lane = facts.location.battlefield();
            if rule.targets.event_locations
                == metadata::EventLocationConstraint::MovedToOrFromBattlefield
                && !context
                    .movement
                    .is_some_and(|m| lane.is_some() && (lane == m.origin || lane == m.destination))
            {
                return Ok(false);
            }
            if rule.targets.relative_might == metadata::RelativeMightConstraint::LessThanSource {
                let Some((unit, source)) = self
                    .unit_ref(target)
                    .zip(Self::source_target(context.source).and_then(|t| self.unit_ref(t)))
                else {
                    return Ok(false);
                };
                if self.unit_might(unit)? >= self.unit_might(source)? {
                    return Ok(false);
                }
            }
            if rule.targets.relative_energy == metadata::RelativeEnergyConstraint::AtMostSourceMight
            {
                let Some(source) =
                    Self::source_target(context.source).and_then(|t| self.unit_ref(t))
                else {
                    return Ok(false);
                };
                if u64::from(facts.printed_energy) > self.unit_might(source)? {
                    return Ok(false);
                }
            }
            let is_source = match target {
                Target::Unit { unit_id, .. } => context.source.unit_id == Some(unit_id),
                Target::Gear { gear_id, .. } => context.source.gear_id == Some(gear_id),
                _ => false,
            };
            if rule.targets.another && is_source {
                return Ok(false);
            }
            if rule.targets.battlefield && lane.is_none() {
                return Ok(false);
            }
            if rule.targets.here
                && self.trigger_here_location(&rule.targets, context) != Some(facts.location)
            {
                return Ok(false);
            }
            if let Some(n) = rule.targets.max_might {
                let Some(unit) = self.unit_ref(target) else {
                    return Ok(false);
                };
                if self.unit_might(unit)? > u64::from(n) {
                    return Ok(false);
                }
            }
            if rule
                .targets
                .max_energy
                .is_some_and(|n| facts.printed_energy > n)
            {
                return Ok(false);
            }
            if rule
                .targets
                .tag
                .is_some_and(|tag| !self.board_target_has_tag(target, tag))
            {
                return Ok(false);
            }
            if rule.event == Event::SelfPlayed
                && let Some(source) = context.source.unit_id
                && let Some(origin) = self.hidden_play_origin(source)
                && (self
                    .unit_ref(Target::Unit {
                        player: actor,
                        unit_id: source,
                    })
                    .is_none()
                    || lane != Some(origin))
            {
                return Ok(false);
            }
            true
        })
    }
    pub fn trigger_target_rule(&self) -> Option<&'static metadata::TargetRule> {
        if matches!(self.decision, Some(Decision::LegendTargets { .. })) {
            const TARGET: metadata::TargetRule = metadata::TargetRule {
                min: 1,
                max: 1,
                allegiance: rules::Allegiance::Any,
                ..metadata::TargetRule::NONE
            };
            return Some(&TARGET);
        }
        let Decision::TriggerTargets {
            card_id,
            rule_index,
            ..
        } = self.decision.as_ref()?
        else {
            return None;
        };
        Some(&metadata::trigger_rules(*card_id).get(*rule_index)?.targets)
    }
    fn trigger_here_location(
        &self,
        rule: &metadata::TargetRule,
        context: TriggerContext,
    ) -> Option<board_targets::BoardLocation> {
        if !rule.here {
            return None;
        }
        match rule.here_basis {
            metadata::HereLocationBasis::SourceDeparture => context.departure_location,
            metadata::HereLocationBasis::LiveSource => self
                .source_location(context.source)
                .map(|(lane, player)| board_targets::BoardLocation::from_lane(player, lane)),
        }
    }
    pub fn trigger_target_location(&self) -> Option<board_targets::BoardLocation> {
        let Decision::TriggerTargets { context, .. } = self.decision.as_ref()? else {
            return None;
        };
        self.trigger_here_location(self.trigger_target_rule()?, *context)
    }
    pub fn trigger_target_bounds(&self) -> Option<(usize, usize)> {
        if matches!(self.decision, Some(Decision::LegendTargets { .. })) {
            return Some((1, 1));
        }
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
    pub fn valid_trigger_target(&self, target: Target, prior: &[Target]) -> Result<bool, String> {
        self.valid_trigger_target_mode(target, prior, true)
    }

    fn valid_trigger_target_mode(
        &self,
        target: Target,
        prior: &[Target],
        costs: bool,
    ) -> Result<bool, String> {
        if let Some(Decision::LegendTargets { player }) = self.decision {
            return Ok(prior.is_empty()
                && self.legend_target_valid(player, target)
                && (!costs
                    || self.available_power(player, None)
                        >= self.deflect_cost(player, &[target])?.into()));
        }
        Ok({
            let Some(Decision::TriggerTargets {
                card_id,
                rule_index,
                context,
                ..
            }) = self.decision.as_ref()
            else {
                return Ok(false);
            };
            let rule = &metadata::trigger_rules(*card_id)[*rule_index];
            if prior.len() >= rule.targets.max as usize
                || (rule.targets.distinct && prior.contains(&target))
            {
                return Ok(false);
            }
            if !self.trigger_rule_target_valid(
                *card_id,
                *rule_index,
                *context,
                target,
                prior.len(),
            )? {
                return Ok(false);
            }
            let mut targets = prior.to_vec();
            targets.push(target);
            !costs
                || self.available_power(context.source.player, None)
                    >= self.deflect_cost(context.source.player, &targets)?.into()
        })
    }
    pub fn trigger_target_candidates(&self) -> Result<Vec<Target>, String> {
        let kind = match self.decision.as_ref() {
            Some(Decision::TriggerTargets {
                card_id,
                rule_index,
                ..
            }) => metadata::trigger_rules(*card_id)[*rule_index].targets.kind,
            _ => rules::PlayTargetKind::Unit,
        };
        Ok(self
            .players
            .iter()
            .enumerate()
            .flat_map(|(player, p)| {
                p.units
                    .iter()
                    .map(move |u| Target::Unit {
                        player,
                        unit_id: u.id,
                    })
                    .chain(p.gear.iter().map(move |g| Target::Gear {
                        player,
                        gear_id: g.id,
                    }))
            })
            .filter(|&target| self.board_target_matches_kind(target, kind))
            .collect())
    }
    pub(super) fn trigger_target_actions(&self) -> Result<Vec<Action>, String> {
        self.trigger_target_actions_mode(true)
    }

    fn trigger_target_actions_mode(&self, costs: bool) -> Result<Vec<Action>, String> {
        Ok({
            let Some((max, min)) = self.trigger_target_bounds() else {
                return Ok(Vec::new());
            };
            let mut sets = vec![Vec::new()];
            let mut actions = Vec::new();
            let candidates = self.trigger_target_candidates()?;
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
                        if self.valid_trigger_target_mode(target, prior, costs)? {
                            let mut v = prior.clone();
                            v.push(target);
                            next.push(v);
                        }
                    }
                }
                sets = next;
            }
            actions
        })
    }
    pub(super) fn choose_trigger_targets(&mut self, targets: Vec<Target>) -> Result<(), String> {
        if let Some(Decision::LegendTargets { player }) = self.decision {
            return self.choose_legend_targets(player, &targets);
        }
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
            if !self.valid_trigger_target(target, &targets[..index])? {
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
    pub(super) fn discard_unfinalizable_trigger(&mut self) -> Result<(), String> {
        if !matches!(
            self.decision,
            Some(Decision::TriggerTargets { .. } | Decision::LegendTargets { .. })
        ) {
            return Ok(());
        }
        let mut payable = false;
        let mut capacity = None;
        for action in self.trigger_target_actions_mode(false)? {
            match self.funded_action_witness(&action) {
                Ok(Some(_)) => {
                    payable = true;
                    break;
                }
                Ok(None) => {}
                Err(e) if crate::numeric::is_capacity(&e) => capacity = Some(e),
                Err(e) => return Err(e),
            }
        }
        if !payable {
            if let Some(e) = capacity {
                return Err(e);
            }
            self.decision = None;
            self.push_log(
                "A triggered ability cannot finalize because no payable legal targets remain."
                    .into(),
            );
        }

        Ok(())
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
    ) -> Result<(), String> {
        let rule = &metadata::trigger_rules(card_id)[index];
        let mut selected = Vec::new();
        for (slot, &target) in targets.iter().enumerate() {
            if self.trigger_rule_target_valid(card_id, index, context, target, slot)? {
                selected.push(target);
            }
        }
        let targets = selected;
        for effect in rule.effects {
            match *effect {
                Effect::KillGear {
                    buff_source_if_killed,
                } => {
                    for &target in &targets {
                        let outcome = self.kill_gear_target(target);
                        if buff_source_if_killed
                            && matches!(
                                outcome,
                                board_targets::GearKillResult::Resolved { killed: true, .. }
                            )
                            && let Some(source) = Self::source_target(context.source)
                        {
                            self.buff_unit_by(player, source)?;
                        }
                    }
                }
                Effect::GiveTemporary(who) => {
                    for target in self.trigger_operands(context, who, &targets) {
                        self.grant_temporary(target);
                    }
                }
                Effect::Xp(amount) => {
                    let count = match amount {
                        Amount::Fixed(n) => u64::from(n),
                        Amount::FriendlyUnits => u64::try_from(self.players[player].units.len()).map_err(|_| "engine_capacity: Unit count exceeds the supported XP representation.")?,
                    };
                    self.gain_xp(player, count)?;
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
                        self.modify_might(target, i128::from(amount), minimum.into())?;
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
                        self.buff_unit_by(player, target)?;
                    }
                }
                Effect::Damage(amount) => {
                    let context = crate::game::damage::DamageContext {
                        player,
                        bonus_damage: 0,
                        dealer: crate::game::damage::DamageDealer::Ability,
                    };
                    let packets: Vec<_> = targets
                        .iter()
                        .map(|&target| {
                            Ok((target, self.damage_amount(context, target, amount.into())?))
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    for (target, amount) in packets {
                        if let Some(unit) = self.unit_mut(target) {
                            unit.damage =
                                crate::numeric::add_u64(unit.damage, amount, "marked damage")?;
                        }
                    }
                }
                Effect::Bounce => {
                    for &target in &targets {
                        self.bounce(target);
                    }
                }
                Effect::ReturnUnitsAtMost(maximum) => {
                    let targets = self.units_at_most(maximum)?;
                    self.return_board_targets(&targets);
                }
                Effect::KillUnitOrGear => {
                    // This descriptor admits exactly one target; unit replacements
                    // and gear Deathknell each retain their canonical kill path.
                    if let Some(&target) = targets.first() {
                        match target {
                            Target::Unit { .. } => self.kill_unit(target)?,
                            Target::Gear { .. } => {
                                self.kill_gear_target(target);
                            }
                            _ => {}
                        }
                    }
                }
                Effect::FightSource => {
                    if let (Some(source), Some(&target)) =
                        (Self::source_target(context.source), targets.first())
                    {
                        self.fight_units(source, target)?;
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
                Effect::PredictMany(n) => self.predict_many(player, n),
                Effect::Channel { count, exhausted } => {
                    self.channel_runes(player, count, exhausted);
                }
                Effect::KillTargets => self.kill_units(&targets)?,
                Effect::DamageIfSourceEmpowered { base, empowered } => {
                    let amount = if Self::source_target(context.source)
                        .and_then(|t| self.unit_ref(t))
                        .is_some_and(|u| self.unit_empowered(u.id))
                    {
                        empowered
                    } else {
                        base
                    };
                    for &target in &targets {
                        self.deal_damage(
                            crate::game::damage::DamageContext {
                                player,
                                bonus_damage: 0,
                                dealer: crate::game::damage::DamageDealer::Ability,
                            },
                            target,
                            amount.into(),
                        )?;
                    }
                }
                Effect::Tokens {
                    count,
                    might,
                    gear,
                    opponent,
                    location,
                    entry_ready,
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
                        self.queue_token_play_with_entry(
                            owner,
                            card_id,
                            might,
                            location,
                            gear,
                            entry_ready,
                        );
                    }
                }
                Effect::Points(n) => {
                    self.players[player].score = self.players[player].score.saturating_add(n);
                }
            }
            self.trigger_batch += 1;
        }
        Ok(())
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
        game.buff_unit_by(0, target).unwrap();
        game.after_resolution().unwrap();
        assert_eq!(game.unit_ref(target).unwrap().buff, 1);
        assert!(!game.unit_ref(target).unwrap().ready);
        assert!(game.chain.is_empty());
    }
}
