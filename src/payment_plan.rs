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

/// The selected card is publicly announced and physically in the pending zone.
/// Private intent details (such as unrelated cost-selection indices) stay private.
#[derive(Clone, Debug)]
pub struct PaymentPlanView {
    pub player: usize,
    pub energy: u32,
    pub power: Vec<PowerCost>,
    pub action: Option<Action>,
    pub pending_card: Option<CardId>,
}

impl Game {
    pub fn payment_plan_view(&self, viewer: usize) -> Option<PaymentPlanView> {
        let plan = self.payment_plan.as_ref()?;
        Some(PaymentPlanView {
            player: plan.player,
            energy: plan
                .obligations
                .iter()
                .filter_map(|cost| match cost {
                    Obligation::Energy(n) => Some(u32::from(*n)),
                    Obligation::Power(_) => None,
                })
                .sum(),
            power: plan
                .obligations
                .iter()
                .filter_map(|cost| match cost {
                    Obligation::Power(cost) => Some(cost.clone()),
                    Obligation::Energy(_) => None,
                })
                .collect(),
            action: (viewer == plan.player).then(|| plan.action.clone()),
            pending_card: plan.pending_card.as_ref().map(|card| card.card_id),
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
            mode: Mode::Recording,
        }));
        // Normalize convenience target choices before resources change the board.
        let action = probe.normalize_payment_intent(action)?;
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
        candidate.remove_defeated();
        candidate.after_resolution();
        *self = candidate;
        Ok(())
    }

    /// A failed attempt may be undone, restoring even the Add abilities used
    /// during it. A mandatory original choice is restored, never skipped.
    pub fn can_cancel_payment(&self) -> bool {
        self.payment_plan
            .as_ref()
            .is_some_and(|plan| plan.mode == Mode::Pending)
            && self.resource_frames.is_empty()
            && self.clone().complete_payment().is_err()
    }

    pub(crate) fn cancel_payment(&mut self) -> Result<(), String> {
        if !self.can_cancel_payment() {
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
            | Action::ChooseTriggerTargets { .. }
            | Action::ChooseTarget { .. }
            | Action::Choose { accept: true }
    )
}
