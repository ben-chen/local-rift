//! Exact power-source choices, shared by the native client and bot interface.
use crate::cards::Domain;
use crate::game::{Action, Game, domain_index};

pub const POWER_DOMAINS: [Domain; 6] = [
    Domain::Fury,
    Domain::Calm,
    Domain::Mind,
    Domain::Body,
    Domain::Chaos,
    Domain::Order,
];

/// Counts in Fury, Calm, Mind, Body, Chaos, Order order. `recycle` activates
/// that many channelled runes; the other entries spend already generated power.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PowerPayment {
    pub colored: [u8; 6],
    pub any: u8,
    pub spell: u8,
    pub recycle: [u8; 6],
}

impl PowerPayment {
    pub fn total(&self) -> u16 {
        self.colored
            .iter()
            .chain(&self.recycle)
            .map(|&n| u16::from(n))
            .sum::<u16>()
            + u16::from(self.any)
            + u16::from(self.spell)
    }
    fn contains(&self, other: &Self) -> bool {
        self.any >= other.any
            && self.spell >= other.spell
            && (0..6)
                .all(|d| self.colored[d] >= other.colored[d] && self.recycle[d] >= other.recycle[d])
    }
    fn subtract(&mut self, other: &Self) {
        self.any -= other.any;
        self.spell -= other.spell;
        for d in 0..6 {
            self.colored[d] -= other.colored[d];
            self.recycle[d] -= other.recycle[d];
        }
    }
    fn add(&mut self, other: &Self) {
        self.any += other.any;
        self.spell += other.spell;
        for d in 0..6 {
            self.colored[d] += other.colored[d];
            self.recycle[d] += other.recycle[d];
        }
    }
    fn available(&self, domains: &[Domain], spell: bool) -> u8 {
        domains.iter().fold(
            self.any.saturating_add(if spell { self.spell } else { 0 }),
            |n, &d| {
                let d = domain_index(d);
                n.saturating_add(self.colored[d])
                    .saturating_add(self.recycle[d])
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PowerCost {
    pub domains: Vec<Domain>,
    pub amount: u8,
    pub spell_eligible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentQuote {
    pub suggested: PowerPayment,
    pub available: PowerPayment,
    pub costs: Vec<PowerCost>,
}

#[derive(Clone, Debug)]
pub(crate) struct PaymentContext {
    player: usize,
    remaining: Option<PowerPayment>,
    spent: PowerPayment,
    costs: Vec<PowerCost>,
}

impl Game {
    pub(crate) fn recycle_rune(
        &mut self,
        player: usize,
        domain: Domain,
        exhausted: bool,
    ) -> Result<(), String> {
        if self.decision.is_some() && !self.resource_payment_pending() {
            return Err("Finish the pending choice before activating a rune.".into());
        }
        let state = &mut self.players[player];
        let d = domain_index(domain);
        let available = if exhausted {
            state.exhausted_runes[d]
        } else {
            state.runes[d] - state.exhausted_runes[d]
        };
        if available == 0 {
            return Err(format!(
                "No {} {} rune is available.",
                if exhausted { "exhausted" } else { "ready" },
                domain.name()
            ));
        }
        state.runes[d] -= 1;
        if exhausted {
            state.exhausted_runes[d] -= 1;
        }
        state.rune_deck.insert(0, domain);
        state.power_pool[d] += 1;
        state.energy = state.energy_pool
            + state
                .runes
                .iter()
                .zip(state.exhausted_runes)
                .map(|(all, used)| all - used)
                .sum::<u8>();
        self.push_log(format!(
            "{} recycles a {} {} rune for power.",
            self.players[player].name,
            if exhausted { "exhausted" } else { "ready" },
            domain.name()
        ));
        Ok(())
    }

    pub fn power_sources(&self, player: usize) -> PowerPayment {
        let state = &self.players[player];
        PowerPayment {
            colored: state.power_pool,
            any: state.any_power,
            spell: self.legend[player].spell_power,
            recycle: state.runes,
        }
    }

    /// Executes only on a disposable copy. Targets, optional costs, and Deflect
    /// are included, so the quote cannot drift from the action's actual cost.
    pub fn quote_power_payment(&self, action: &Action) -> Result<PaymentQuote, String> {
        if matches!(action, Action::WithPowerPayment { .. }) || self.payment.is_some() {
            return Err("Quote the action before adding a payment choice.".into());
        }
        let player = self.priority();
        let mut probe = self.clone();
        probe.payment = Some(PaymentContext {
            player,
            remaining: None,
            spent: PowerPayment::default(),
            costs: Vec::new(),
        });
        probe.apply(action.clone())?;
        let context = probe.payment.take().expect("payment recording context");
        Ok(PaymentQuote {
            suggested: context.spent,
            available: self.power_sources(player),
            costs: context.costs,
        })
    }

    pub(crate) fn apply_power_payment(
        &mut self,
        action: Action,
        payment: PowerPayment,
    ) -> Result<(), String> {
        if matches!(action, Action::WithPowerPayment { .. }) || self.payment.is_some() {
            return Err("Payment choices cannot be nested.".into());
        }
        let player = self.priority();
        if !self.power_sources(player).contains(&payment) {
            return Err("The chosen power sources are not available.".into());
        }
        let mut candidate = self.clone();
        candidate.payment = Some(PaymentContext {
            player,
            remaining: Some(payment),
            spent: PowerPayment::default(),
            costs: Vec::new(),
        });
        candidate.apply(action)?;
        let context = candidate.payment.take().expect("exact payment context");
        if context.remaining.expect("exact payment budget").total() != 0 {
            return Err(
                "Choose exactly the power required by this action; extra sources were selected."
                    .into(),
            );
        }
        *self = candidate;
        Ok(())
    }

    fn permitted_sources(&self, player: usize) -> PowerPayment {
        self.payment
            .as_ref()
            .filter(|p| p.player == player)
            .and_then(|p| p.remaining.clone())
            .unwrap_or_else(|| self.power_sources(player))
    }

    pub(crate) fn power_available(&self, player: usize, domains: &[Domain], spell: bool) -> u8 {
        self.permitted_sources(player).available(domains, spell)
    }

    /// Every power cost goes through this function, including abilities, Hidden,
    /// optional/replay costs, and Deflect. Unselected pools remain visible and
    /// unchanged; only the sources used to pay are restricted.
    pub(crate) fn consume_power(
        &mut self,
        player: usize,
        domains: &[Domain],
        amount: u8,
        spell: bool,
    ) -> Result<(), String> {
        let sources = self.permitted_sources(player);
        if sources.available(domains, spell) < amount {
            return Err(
                "The selected sources do not provide enough matching power for this cost.".into(),
            );
        }
        if amount == 0 {
            return Ok(());
        }
        let mut used = PowerPayment::default();
        let mut remaining = amount;
        if spell {
            used.spell = remaining.min(sources.spell);
            remaining -= used.spell;
        }
        for &domain in domains {
            let d = domain_index(domain);
            used.colored[d] = remaining.min(sources.colored[d]);
            remaining -= used.colored[d];
        }
        used.any = remaining.min(sources.any);
        remaining -= used.any;
        // Match the normal payment behavior: exhausted runes before ready ones.
        for exhausted in [true, false] {
            for &domain in domains {
                let d = domain_index(domain);
                let count = if exhausted {
                    sources.recycle[d].min(self.players[player].exhausted_runes[d])
                } else {
                    sources.recycle[d] - used.recycle[d]
                };
                let paid = remaining.min(count);
                used.recycle[d] += paid;
                remaining -= paid;
            }
        }
        debug_assert_eq!(remaining, 0);
        if !self.power_sources(player).contains(&used) {
            return Err("A selected power source is no longer available.".into());
        }
        let state = &mut self.players[player];
        state.any_power -= used.any;
        self.legend[player].spell_power -= used.spell;
        for (d, domain) in POWER_DOMAINS.into_iter().enumerate() {
            state.power_pool[d] -= used.colored[d];
            let exhausted = used.recycle[d].min(state.exhausted_runes[d]);
            state.exhausted_runes[d] -= exhausted;
            state.runes[d] -= used.recycle[d];
            // A ready rune may exhaust for energy before it is recycled.
            state.energy_pool += used.recycle[d] - exhausted;
            for _ in 0..used.recycle[d] {
                state.rune_deck.insert(0, domain);
            }
        }
        state.energy = state.energy_pool
            + state
                .runes
                .iter()
                .zip(state.exhausted_runes)
                .map(|(all, used)| all - used)
                .sum::<u8>();
        if let Some(context) = self.payment.as_mut().filter(|p| p.player == player) {
            if let Some(budget) = &mut context.remaining {
                budget.subtract(&used);
            }
            context.spent.add(&used);
            context.costs.push(PowerCost {
                domains: domains.to_vec(),
                amount,
                spell_eligible: spell,
            });
        }
        Ok(())
    }
}
