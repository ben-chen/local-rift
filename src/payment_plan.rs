//! Suspended finalization with costs fixed before in-payment resource abilities.
//!
//! Core 356.4.g fixes variable discounts before payment; 358 checks legality
//! afterward and rolls back the attempted finalization when it cannot finish.
//! The normal action implementation remains the sole validator and executor.
use crate::cards::{CardId, Domain};
use crate::game::{Action, Decision, Game, PendingCard};
use crate::hidden::HiddenCard;
use crate::payment::PowerCost;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Recording,
    Pending,
    Paying,
    Replaying,
}

#[derive(Clone, Debug)]
enum Obligation {
    Xp(crate::xp::XpAmount),
    Energy(u16),
    Power(PowerCost),
}

#[derive(Clone, Debug)]
pub(crate) struct PaymentPlan {
    player: usize,
    action: Action,
    original: Box<Game>,
    frozen: Box<Game>,
    pending_card: Option<PaymentCard>,
    obligations: Vec<Obligation>,
    show_off: Vec<crate::show_off::ShownObject>,
    rune_targets: Vec<crate::game::Target>,
    mode: Mode,
}

#[derive(Clone, Debug)]
enum PaymentOrigin {
    Hand(usize),
    Champion,
    Hidden { index: usize, card: HiddenCard },
    Trash(PendingCard),
}
#[derive(Clone, Debug)]
struct PaymentCard {
    player: usize,
    card_id: CardId,
    origin: PaymentOrigin,
}

/// The result of refusing a supported frozen cost. Finalization refusal rolls
/// back the play; instruction refusal skips its effect and retains Add actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaymentRefusalKind {
    Ordinary,
    PendingCard,
    PendingToken,
    Trigger,
    /// Skip a dependent effect of an already resolving ability; keep Add actions.
    Instruction,
}

/// The selected card is publicly announced and physically in the pending zone.
/// Private intent details (such as unrelated cost-selection indices) stay private.
#[derive(Clone, Debug)]
pub struct PaymentPlanView {
    pub xp: crate::xp::XpAmount,
    pub player: usize,
    pub energy: u32,
    pub power: Vec<PowerCost>,
    pub action: Option<Action>,
    pub pending_card: Option<CardId>,
    pub show_off: Vec<crate::show_off::ShownObject>,
    pub refusal_kind: Option<PaymentRefusalKind>,
}

impl PaymentPlan {
    fn refusal_kind(&self) -> Option<PaymentRefusalKind> {
        // Dais always incurs its return-unit base cost even if its Energy
        // component is reduced to zero; Core405.2 is not resource-only.
        let mut field_action = &self.action;
        while let Action::WithPlayChoices { action, .. } = field_action {
            field_action = action;
        }
        if matches!(
            field_action,
            Action::ResolveBattlefieldPayment { pay: true, .. }
        ) && matches!(
            self.original.decision,
            Some(Decision::BattlefieldPayment { .. })
        ) {
            return Some(PaymentRefusalKind::Trigger);
        }
        if matches!(field_action, Action::ResolveTriggerTrashPayment { .. }) {
            return Some(PaymentRefusalKind::Trigger);
        }
        if !self.obligations.iter().any(|cost| match cost {
            Obligation::Energy(amount) => *amount > 0,
            Obligation::Power(cost) => cost.amount > 0,
            Obligation::Xp(_) => false,
        }) {
            return None;
        }
        let mut action = &self.action;
        while let Action::WithPlayChoices { action: inner, .. } = action {
            action = inner;
        }
        use PaymentRefusalKind::*;
        match action {
            Action::ResolveTriggerPayment { pay: true, .. }
                if self.original.resolving_trigger_payment().is_some() =>
            {
                Some(Instruction)
            }
            Action::FinalizePendingPlay { .. } => Some(PendingCard),
            Action::FinalizeTokenPlay { .. } | Action::PlaceToken { .. } => Some(PendingToken),
            Action::PlayFromTrash { .. }
            | Action::PlayFromTrashWithTargets { .. }
            | Action::PlayFromTrashOptions { .. } => match self.original.decision {
                None => Some(Ordinary),
                Some(
                    Decision::PendingPlay { .. }
                    | Decision::LastRites { .. }
                    | Decision::ReplaySpell { .. }
                    | Decision::RevivePlay { .. },
                ) => Some(PendingCard),
                _ => None,
            },
            Action::PlayCard { .. }
            | Action::PlayCardWithTargets { .. }
            | Action::PlayCardOptions { .. }
            | Action::PlayChampion { .. }
            | Action::PlayChampionOptions { .. }
            | Action::PlayHidden { .. }
            | Action::ActivateAbility { .. }
            | Action::ActivateAbilityWithCosts { .. }
            | Action::ActivateLegend { .. }
            | Action::Equip { .. }
            | Action::EquipWithTrash { .. }
            | Action::EmpowerUnit { .. } => Some(Ordinary),
            Action::ResolveTriggerPayment { pay: true, .. }
            | Action::ChooseTriggerTargets { .. }
            | Action::ChooseTarget { .. }
                if pending_trigger_finalization(&self.original) =>
            {
                Some(Trigger)
            }
            // These legacy choices pay BEFORE queuing the responding ability.
            // CounterTax instead belongs to an already resolving parent.
            Action::Choose { accept: true }
                if matches!(
                    self.original.decision,
                    Some(Decision::Legend { .. } | Decision::Battlefield { .. })
                ) && pending_trigger_finalization(&self.original) =>
            {
                Some(Trigger)
            }
            _ => None,
        }
    }
}

fn pending_trigger_finalization(game: &Game) -> bool {
    match game.decision {
        Some(
            Decision::BattlefieldPayment { .. }
            | Decision::TriggerPayment {
                resolving_chain_id: None,
                ..
            }
            | Decision::TriggerTargets { .. }
            | Decision::LegendTargets { .. }
            | Decision::OnPlayTarget { .. }
            | Decision::EmpowerHere { .. },
        ) => true,
        Some(Decision::Legend { trigger, .. }) => matches!(
            trigger,
            crate::legends::LegendTrigger::IreliaChoose { .. }
                | crate::legends::LegendTrigger::IreliaConquer
        ),
        Some(Decision::Battlefield { lane, .. }) => game.battlefields.get(lane) == Some(&0),
        _ => false,
    }
}

impl Game {
    /// Keep an announced physical cost card pinned while Add actions alter trash.
    /// None means the entire trash leaves (burnout). Rollback snapshots stay intact.
    pub(crate) fn update_pending_trash_cost(&mut self, owner: usize, removed: Option<usize>) {
        let Some(plan) = self
            .payment_plan
            .as_mut()
            .filter(|p| p.mode == Mode::Pending)
        else {
            return;
        };
        let mut action = &mut plan.action;
        while let Action::WithPlayChoices { action: inner, .. } = action {
            action = inner;
        }
        if let Action::ResolveTriggerTrashPayment {
            trash_player,
            trash_index,
            ..
        } = action
            && *trash_player == owner
            && *trash_index != usize::MAX
        {
            match removed {
                None => *trash_index = usize::MAX,
                Some(index) if index == *trash_index => *trash_index = usize::MAX,
                Some(index) if index < *trash_index => *trash_index -= 1,
                _ => {}
            }
        }
    }

    pub(crate) fn checkpoint_finalization_payment_add(&mut self, action: &Action) {
        let eligible = matches!(
            self.decision,
            Some(
                Decision::PendingPlay { .. }
                    | Decision::TokenPlay { .. }
                    | Decision::LastRites { .. }
                    | Decision::ReplaySpell { .. }
                    | Decision::RevivePlay { .. }
            )
        ) || (pending_trigger_finalization(self)
            && !matches!(self.decision, Some(Decision::TriggerPayment { .. })));
        if self.finalization_payment_origin.is_none()
            && self.payment_plan.is_none()
            && self.resource_frames.is_empty()
            && eligible
            && matches!(
                action,
                Action::ActivateGear { .. }
                    | Action::ActivateAbility { .. }
                    | Action::ActivateAbilityWithCosts { .. }
                    | Action::ActivateLegend { .. }
                    | Action::ActivateRuneObject { .. }
                    | Action::ActivateRune { .. }
                    | Action::RecycleRune { .. }
            )
        {
            // This snapshot is after the parent resolved and before only the
            // first Add; it contains no recursively captured copy of itself.
            let mut origin = self.clone();
            // Exact-payment ledgers belong to the current command, never to a
            // persistent rollback point. Rollback preserves its caller's ledger.
            origin.payment = None;
            self.finalization_payment_origin = Some(Box::new(origin));
        }
    }

    pub(crate) fn clear_completed_finalization_payment_origin(&mut self) {
        let Some(origin) = &self.finalization_payment_origin else {
            return;
        };
        if self.payment_plan.is_some() || !self.resource_frames.is_empty() {
            return;
        }
        let same_trigger = matches!((&origin.decision, &self.decision),
            (Some(Decision::TriggerTargets { player:a, card_id:b, rule_index:c, context:d }),
             Some(Decision::TriggerPayment { player:e, card_id:f, rule_index:g, context:h, .. }))
                if (a,b,c,d)==(e,f,g,h));
        let same_token = origin.current_pending_token().map(|token| token.id)
            == self.current_pending_token().map(|token| token.id);
        let same_occurrence = origin.pending_occurrence_id == self.pending_occurrence_id;
        if !same_occurrence || !same_token || (origin.decision != self.decision && !same_trigger) {
            self.finalization_payment_origin = None;
        }
    }

    pub(crate) fn rollback_finalization_payment_add(&mut self) {
        if let Some(origin) = self.finalization_payment_origin.take() {
            let command_payment = self.payment.take();
            *self = *origin;
            self.payment = command_payment;
        }
    }

    pub(crate) fn checkpoint_trigger_payment_add(&mut self, action: &Action) {
        if self.trigger_payment_origin.is_none()
            && self.payment_plan.is_none()
            && matches!(
                self.decision,
                Some(Decision::TriggerPayment {
                    resolving_chain_id: None,
                    ..
                })
            )
            && matches!(
                action,
                Action::ActivateGear { .. }
                    | Action::ActivateAbility { .. }
                    | Action::ActivateAbilityWithCosts { .. }
                    | Action::ActivateLegend { .. }
                    | Action::ActivateRuneObject { .. }
                    | Action::ActivateRune { .. }
                    | Action::RecycleRune { .. }
            )
        {
            // The checkpoint already contains the played source and pinned
            // trigger targets. Only subsequent payment-resource actions undo.
            let mut origin = self.clone();
            origin.payment = None;
            self.trigger_payment_origin = Some(Box::new(origin));
        }
    }

    /// Only the public pinned trigger payment choice is exposed, including
    /// while Add instructions or the exact-payment plan temporarily suspend it.
    pub fn trigger_payment_context(&self) -> Option<Decision> {
        self.decision
            .as_ref()
            .filter(|d| matches!(d, Decision::TriggerPayment { .. }))
            .cloned()
            .or_else(|| {
                self.resource_frames
                    .iter()
                    .rev()
                    .filter_map(|frame| frame.suspended.as_ref())
                    .find(|d| matches!(d, Decision::TriggerPayment { .. }))
                    .cloned()
            })
            .or_else(|| {
                self.payment_plan
                    .as_ref()
                    .and_then(|plan| plan.original.trigger_payment_context())
            })
    }
    pub(crate) fn resolving_trigger_payment(&self) -> Option<u32> {
        match self.trigger_payment_context() {
            Some(Decision::TriggerPayment {
                resolving_chain_id, ..
            }) => resolving_chain_id,
            _ => None,
        }
    }
    pub fn battlefield_payment_context(&self) -> Option<Decision> {
        self.decision
            .as_ref()
            .filter(|d| matches!(d, Decision::BattlefieldPayment { .. }))
            .cloned()
            .or_else(|| {
                self.resource_frames
                    .iter()
                    .rev()
                    .filter_map(|frame| frame.suspended.as_ref())
                    .find(|d| matches!(d, Decision::BattlefieldPayment { .. }))
                    .cloned()
            })
            .or_else(|| {
                self.payment_plan
                    .as_ref()
                    .and_then(|plan| plan.original.battlefield_payment_context())
            })
    }
    pub(crate) fn payment_target_query_state(&self) -> Result<Option<Game>, String> {
        let Some(plan) = self.payment_plan.as_ref() else {
            return Ok(None);
        };
        let mut state = self.clone();
        state.decision = plan.original.decision.clone();
        if let Some(card) = plan.pending_card.clone() {
            state.restore_payment_card(card)?;
        }
        state.payment_plan.as_mut().unwrap().mode = Mode::Replaying;
        Ok(Some(state))
    }
    pub fn payment_plan_view(&self, viewer: usize) -> Option<PaymentPlanView> {
        let plan = self.payment_plan.as_ref()?;
        Some(PaymentPlanView {
            xp: plan
                .obligations
                .iter()
                .filter_map(|cost| match cost {
                    Obligation::Xp(n) => Some(*n),
                    _ => None,
                })
                .sum(),
            player: plan.player,
            energy: plan
                .obligations
                .iter()
                .filter_map(|cost| match cost {
                    Obligation::Energy(n) => Some(u32::from(*n)),
                    Obligation::Power(_) | Obligation::Xp(_) => None,
                })
                .sum(),
            power: plan
                .obligations
                .iter()
                .filter_map(|cost| match cost {
                    Obligation::Power(cost) => Some(cost.clone()),
                    Obligation::Energy(_) | Obligation::Xp(_) => None,
                })
                .collect(),
            action: (viewer == plan.player).then(|| plan.action.clone()),
            pending_card: plan.pending_card.as_ref().map(|card| card.card_id),
            show_off: plan.show_off.clone(),
            refusal_kind: plan.refusal_kind(),
        })
    }

    pub(crate) fn payment_bypasses_cost_checks(&self, player: usize) -> bool {
        self.payment_plan.as_ref().is_some_and(|plan| {
            plan.player == player && matches!(plan.mode, Mode::Recording | Mode::Replaying)
        })
    }

    pub(crate) fn payment_energy_available(&self, player: usize) -> u16 {
        if self.payment_bypasses_cost_checks(player) {
            u16::MAX
        } else {
            self.players[player].energy
        }
    }

    pub(crate) fn payment_record_xp(
        &mut self,
        player: usize,
        amount: crate::xp::XpAmount,
    ) -> Result<bool, String> {
        let Some(plan) = self
            .payment_plan
            .as_mut()
            .filter(|plan| plan.player == player)
        else {
            return Ok(false);
        };
        match plan.mode {
            Mode::Recording => {
                // Keep the public aggregate exact even for host-capacity fixtures.
                plan.obligations.iter().try_fold(amount, |sum, cost| {
                    sum.checked_add(if let Obligation::Xp(n) = cost { *n } else { 0 })
                }).ok_or("engine_capacity: Total XP cost exceeds the supported integer representation.")?;
                if amount > 0 {
                    plan.obligations.push(Obligation::Xp(amount));
                }
                Ok(true)
            }
            Mode::Replaying => Ok(true),
            _ => Ok(false),
        }
    }

    pub(crate) fn payment_record_energy(&mut self, player: usize, amount: u16) -> bool {
        let Some(plan) = self
            .payment_plan
            .as_mut()
            .filter(|plan| plan.player == player)
        else {
            return false;
        };
        match plan.mode {
            Mode::Recording => {
                if amount > 0 {
                    plan.obligations.push(Obligation::Energy(amount));
                }
                true
            }
            Mode::Replaying => true,
            _ => false,
        }
    }

    pub(crate) fn payment_record_power(
        &mut self,
        player: usize,
        domains: &[Domain],
        amount: u16,
        spell: bool,
    ) -> bool {
        let Some(plan) = self
            .payment_plan
            .as_mut()
            .filter(|plan| plan.player == player)
        else {
            return false;
        };
        match plan.mode {
            Mode::Recording => {
                if amount > 0 {
                    plan.obligations.push(Obligation::Power(PowerCost {
                        domains: domains.to_vec(),
                        amount,
                        spell_eligible: spell,
                    }));
                }
                true
            }
            Mode::Replaying => true,
            _ => false,
        }
    }

    pub(crate) fn reserved_rune_targets(&self) -> Vec<crate::game::Target> {
        self.active_play
            .as_ref()
            .map(|a| a.rune_targets.as_slice())
            .into_iter()
            .chain(
                self.payment_plan
                    .as_ref()
                    .map(|p| p.rune_targets.as_slice()),
            )
            .flatten()
            .copied()
            .collect()
    }

    pub(crate) fn payment_show_off(&self) -> Option<&[crate::show_off::ShownObject]> {
        self.payment_plan
            .as_ref()
            .filter(|plan| matches!(plan.mode, Mode::Recording | Mode::Replaying))
            .map(|plan| plan.show_off.as_slice())
    }

    pub(crate) fn payment_frozen_source(&self) -> Option<&Game> {
        self.payment_plan
            .as_ref()
            .filter(|plan| matches!(plan.mode, Mode::Recording | Mode::Replaying))
            .map(|plan| plan.frozen.as_ref())
    }

    pub(crate) fn payment_blocks_cleanup(&self) -> bool {
        self.payment_plan.is_some()
    }

    /// Start a single suspended finalization; nested Add abilities use their own
    /// resource-continuation stack, never a nested payment plan.
    pub(crate) fn begin_payment(&mut self, action: Action) -> Result<(), String> {
        if self.payment_plan.is_some() || !self.resource_frames.is_empty() || self.payment.is_some()
        {
            return Err("Finish the current payment before beginning another.".into());
        }
        if !payment_intent(&action)
            || (matches!(action, Action::Choose { .. })
                && !matches!(
                    self.decision,
                    Some(
                        Decision::CounterTax { .. }
                            | Decision::Legend { .. }
                            | Decision::Battlefield { .. }
                    )
                ))
            || (matches!(
                action,
                Action::PlayFromTrash { .. }
                    | Action::PlayFromTrashWithTargets { .. }
                    | Action::PlayFromTrashOptions { .. }
            ) && matches!(self.decision, Some(Decision::ReviveUnit { .. })))
        {
            return Err("This action does not start a card or ability payment.".into());
        }
        let player = self.priority();
        let original = self.clone();
        let mut probe = self.clone();
        probe.payment_plan = Some(Box::new(PaymentPlan {
            player,
            action: action.clone(),
            original: Box::new(original.clone()),
            frozen: Box::new(original.clone()),
            pending_card: None,
            obligations: Vec::new(),
            show_off: Vec::new(),
            rune_targets: Vec::new(),
            mode: Mode::Recording,
        }));
        // Normalize convenience target choices before resources change the board.
        let action = probe.normalize_payment_intent(action)?;
        let shown = self.capture_action_show_off(&action)?;
        let mut staged = self.clone();
        staged.capture_payment_grants(&action);
        let pending_card = staged.detach_payment_card(&action)?;
        // The source has already left its zone when cost facts are determined.
        // Restore it only inside the disposable executor probe; cost queries use
        // this staged snapshot, and no external observer sees the restoration.
        probe = staged.clone();
        probe.payment_plan = Some(Box::new(PaymentPlan {
            player,
            action: action.clone(),
            original: Box::new(original),
            frozen: Box::new(staged.clone()),
            pending_card: pending_card.clone(),
            obligations: Vec::new(),
            show_off: shown.clone(),
            rune_targets: self.capture_action_rune_targets(&action),
            mode: Mode::Recording,
        }));
        if let Some(card) = pending_card {
            probe.restore_payment_card(card)?;
        }
        probe.apply(action)?;
        let mut plan = probe
            .payment_plan
            .take()
            .ok_or("Payment recording was interrupted.")?;
        plan.mode = Mode::Pending;
        staged.payment_plan = Some(plan);
        staged.log_show_off(&shown);
        *self = staged;
        self.decision = Some(Decision::Payment { player });
        Ok(())
    }

    fn detach_payment_card(&mut self, action: &Action) -> Result<Option<PaymentCard>, String> {
        let player = self.priority();
        let (card_id, origin) = match action {
            Action::WithPlayChoices { action, .. } => return self.detach_payment_card(action),
            Action::PlayCard { hand_index, .. }
            | Action::PlayCardWithTargets { hand_index, .. }
            | Action::PlayCardOptions { hand_index, .. } => {
                if *hand_index >= self.players[player].hand.len() {
                    return Err("Choose a card in your hand.".into());
                }
                (
                    self.players[player].hand.remove(*hand_index),
                    PaymentOrigin::Hand(*hand_index),
                )
            }
            Action::PlayChampion { .. } | Action::PlayChampionOptions { .. } => (
                self.players[player]
                    .champion
                    .take()
                    .ok_or("Your chosen champion has already been played.")?,
                PaymentOrigin::Champion,
            ),
            Action::PlayFromTrash { trash_index, .. }
            | Action::PlayFromTrashWithTargets { trash_index, .. }
            | Action::PlayFromTrashOptions { trash_index, .. } => {
                let card = self.payment_detach_trash(player, *trash_index)?;
                (card.card_id, PaymentOrigin::Trash(card))
            }
            Action::PlayHidden { hidden_id, .. } => {
                let index = self.players[player]
                    .hidden
                    .iter()
                    .position(|card| card.id == *hidden_id)
                    .ok_or("That facedown card is no longer available.")?;
                let card = self.players[player].hidden.remove(index);
                (card.card_id, PaymentOrigin::Hidden { index, card })
            }
            // An instructed PendingPlay is already detached by its parent ability.
            _ => return Ok(None),
        };
        Ok(Some(PaymentCard {
            player,
            card_id,
            origin,
        }))
    }

    fn restore_payment_card(&mut self, card: PaymentCard) -> Result<(), String> {
        match card.origin {
            PaymentOrigin::Hand(index) => {
                if index > self.players[card.player].hand.len() {
                    return Err("The staged hand position changed.".into());
                }
                self.players[card.player].hand.insert(index, card.card_id);
            }
            PaymentOrigin::Champion => {
                if self.players[card.player].champion.is_some() {
                    return Err("The chosen-champion position changed.".into());
                }
                self.players[card.player].champion = Some(card.card_id);
            }
            PaymentOrigin::Hidden {
                index,
                card: hidden,
            } => {
                if index > self.players[card.player].hidden.len() {
                    return Err("The staged facedown position changed.".into());
                }
                self.players[card.player].hidden.insert(index, hidden);
            }
            PaymentOrigin::Trash(pending) => self.payment_restore_trash(pending),
        }
        Ok(())
    }

    fn normalize_payment_intent(&self, action: Action) -> Result<Action, String> {
        let player = self.priority();
        match action {
            Action::WithPlayChoices { action, choices } => Ok(Action::WithPlayChoices {
                action: Box::new(self.normalize_payment_intent(*action)?),
                choices,
            }),
            Action::PlayCard {
                hand_index,
                lane,
                target,
            } => {
                let id = *self.players[player]
                    .hand
                    .get(hand_index)
                    .ok_or("Choose a card in your hand.")?;
                let targets = self.prepare_targets(
                    player,
                    id,
                    lane,
                    &target.into_iter().collect::<Vec<_>>(),
                )?;
                Ok(Action::PlayCardWithTargets {
                    hand_index,
                    lane,
                    targets,
                })
            }
            Action::PlayChampion { lane, target } => {
                let id = self.players[player]
                    .champion
                    .ok_or("Your chosen champion has already been played.")?;
                let targets = self.prepare_targets(
                    player,
                    id,
                    lane,
                    &target.into_iter().collect::<Vec<_>>(),
                )?;
                Ok(Action::PlayChampion {
                    lane,
                    target: targets.first().copied(),
                })
            }
            Action::PlayFromTrash {
                trash_index,
                lane,
                target,
            } => {
                let id = *self.players[player]
                    .discard
                    .get(trash_index)
                    .ok_or("Choose a card in your trash.")?;
                let targets = self.prepare_targets(
                    player,
                    id,
                    lane,
                    &target.into_iter().collect::<Vec<_>>(),
                )?;
                Ok(Action::PlayFromTrashWithTargets {
                    trash_index,
                    lane,
                    targets,
                })
            }
            // PlayHidden's legacy empty-target convenience must not retarget
            // after resources change. Its validator supplies the explicit pair.
            Action::PlayHidden { hidden_id, targets } if targets.is_empty() => {
                let hidden = self.players[player]
                    .hidden
                    .iter()
                    .find(|c| c.id == hidden_id)
                    .ok_or("That facedown card is no longer available.")?;
                let targets =
                    self.prepare_targets(player, hidden.card_id, Some(hidden.lane), &[])?;
                Ok(Action::PlayHidden { hidden_id, targets })
            }
            other => Ok(other),
        }
    }

    pub(crate) fn complete_payment(&mut self) -> Result<(), String> {
        let plan = self
            .payment_plan
            .as_ref()
            .ok_or("There is no payment to complete.")?;
        if plan.mode != Mode::Pending
            || !matches!(self.decision, Some(Decision::Payment { player }) if player == plan.player)
            || !self.resource_frames.is_empty()
        {
            return Err("Finish the nested resource instruction first.".into());
        }
        let player = plan.player;
        let costs = plan.obligations.clone();
        let intent = plan.action.clone();
        let original_decision = plan.original.decision.clone();
        let pending_card = plan.pending_card.clone();
        let mut candidate = self.clone();
        candidate.payment_plan.as_mut().unwrap().mode = Mode::Paying;
        for cost in costs {
            match cost {
                Obligation::Xp(amount) => candidate.spend_xp(player, amount)?,
                Obligation::Energy(amount) => {
                    if candidate.players[player].energy < amount {
                        return Err(format!("The frozen payment still needs {amount} energy."));
                    }
                    candidate.pay_energy(player, amount);
                }
                Obligation::Power(cost) => candidate.consume_power(
                    player,
                    &cost.domains,
                    cost.amount,
                    cost.spell_eligible,
                )?,
            }
        }
        candidate.payment_plan.as_mut().unwrap().mode = Mode::Replaying;
        candidate.decision = original_decision;
        if let Some(card) = pending_card {
            candidate.restore_payment_card(card)?;
        }
        candidate.apply(intent)?;
        candidate.payment_plan = None;
        candidate.remove_defeated()?;
        candidate.after_resolution()?;
        candidate.validate_numeric_state()?;
        *self = candidate;
        Ok(())
    }

    /// A positive finalization cost or unsuccessful attempt may be undone,
    /// restoring its Add actions and original choice rather than skipping it.
    pub fn can_cancel_payment(&self) -> Result<bool, String> {
        self.validate_numeric_state()?;
        if !self
            .payment_plan
            .as_ref()
            .is_some_and(|p| p.mode == Mode::Pending)
            || !self.resource_frames.is_empty()
        {
            return Ok(false);
        }
        let plan = self.payment_plan.as_ref().unwrap();
        // Entering the payment UI does not Pay. Core445.2 allows refusing
        // positive card/ability finalization resources even when fully funded.
        // Undo restores the choice; Refuse additionally consumes pending items.
        if plan.refusal_kind().is_some() {
            return Ok(true);
        }
        Ok(!crate::numeric::legal_result(
            self.clone().complete_payment(),
        )?)
    }

    pub fn can_refuse_payment(&self) -> Result<bool, String> {
        self.validate_numeric_state()?;
        Ok(self.payment_plan.as_ref().is_some_and(|plan|
            plan.mode == Mode::Pending
                && matches!(self.decision, Some(Decision::Payment { player }) if player == plan.player)
                && self.resource_frames.is_empty()
                && plan.refusal_kind().is_some()))
    }

    pub(crate) fn refuse_payment(&mut self) -> Result<(), String> {
        if !self.can_refuse_payment()? {
            return Err(
                "Only a supported pending cost can be refused; finish nested instructions first."
                    .into(),
            );
        }
        let kind = self.payment_plan.as_ref().unwrap().refusal_kind().unwrap();
        if kind == PaymentRefusalKind::Instruction {
            // Unlike Undo, refusal of a resolving instruction preserves Add
            // actions and their occurrences (Core445.2.b). No cost was paid yet.
            let plan = self.payment_plan.take().unwrap();
            self.decision = plan.original.decision.clone();
            let Some(Decision::TriggerPayment { choice_id, .. }) = self.decision else {
                return Err("The instruction payment choice is missing.".into());
            };
            return self.resolve_trigger_payment(choice_id, false);
        }
        self.cancel_payment()?;
        if kind == PaymentRefusalKind::Ordinary {
            return Ok(());
        }
        // A prior direct Add belongs to this pending child/trigger, not its
        // already completed parent. The staged Undo boundary stays distinct.
        self.rollback_finalization_payment_add();
        match kind {
            PaymentRefusalKind::PendingCard | PaymentRefusalKind::PendingToken => {
                self.drop_refused_pending_play()
            }
            PaymentRefusalKind::Trigger => {
                if let Some(Decision::TriggerPayment { choice_id, .. }) = self.decision {
                    self.resolve_trigger_payment(choice_id, false)?;
                } else {
                    self.decision = None;
                    self.trigger_payment_origin = None;
                }
                self.after_resolution()
            }
            PaymentRefusalKind::Ordinary | PaymentRefusalKind::Instruction => unreachable!(),
        }
    }

    pub(crate) fn cancel_payment(&mut self) -> Result<(), String> {
        if !self.can_cancel_payment()? {
            return Err(
                "This payment can complete; complete it or finish the nested resource instruction."
                    .into(),
            );
        }
        let plan = self.payment_plan.take().unwrap();
        // The exact-payment wrapper is command-local, not part of the attempted
        // finalization. Preserve its empty ledger so quotes succeed and an
        // overpayment is rejected atomically by the outer wrapper.
        let command_payment = self.payment.take();
        *self = *plan.original;
        self.payment = command_payment;
        Ok(())
    }
}

fn payment_intent(action: &Action) -> bool {
    if let Action::WithPlayChoices { action, .. } = action {
        return payment_intent(action);
    }
    matches!(
        action,
        Action::PlayCard { .. }
            | Action::FinalizeTokenPlay { .. }
            | Action::PlaceToken { .. }
            | Action::PlayCardWithTargets { .. }
            | Action::PlayCardOptions { .. }
            | Action::PlayChampion { .. }
            | Action::PlayChampionOptions { .. }
            | Action::PlayFromTrash { .. }
            | Action::PlayFromTrashWithTargets { .. }
            | Action::PlayFromTrashOptions { .. }
            | Action::FinalizePendingPlay { .. }
            | Action::PlayHidden { .. }
            | Action::Hide { .. }
            | Action::HideChampion { .. }
            | Action::ActivateAbility { .. }
            | Action::ActivateAbilityWithCosts { .. }
            | Action::ActivateLegend { .. }
            | Action::Equip { .. }
            | Action::EquipWithTrash { .. }
            | Action::EmpowerUnit { .. }
            | Action::ResolveBattlefieldPayment { pay: true, .. }
            | Action::ResolveTriggerPayment { pay: true, .. }
            | Action::ResolveTriggerTrashPayment { .. }
            | Action::ChooseTriggerTargets { .. }
            | Action::ChooseTarget { .. }
            | Action::Choose { accept: true }
    )
}
