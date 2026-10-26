//! Finite resource reachability. Every producer exhausts or leaves the board;
//! every rune advances ready -> exhausted -> recycled. No search-depth cutoff
//! is used to decide that a mandatory payment is impossible.
use crate::activated::{ConversionDirection, activated_rules};
use crate::game::{Action, Decision, Game};
use crate::legends::LegendKind;
use crate::payment::{POWER_DOMAINS, PowerPayment};
use std::collections::HashSet;

impl Game {
    fn resource_state_key(&self, player: usize) -> String {
        let p = &self.players[player];
        format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
            (
                self.players.each_ref().map(|p| p.xp),
                self.trigger_state.xp_gained,
                &self.play_history
            ),
            self.play_grants,
            p.runes,
            p.exhausted_runes,
            p.power_pool,
            p.energy_pool,
            p.any_power,
            p.units,
            p.gear,
            self.legend[player],
            self.decision,
            self.resource_frames,
            p.units
                .iter()
                .map(|u| (u.id, self.unit_empowered(u.id)))
                .chain(p.gear.iter().map(|g| (g.id, self.unit_empowered(g.id))))
                .collect::<Vec<_>>()
        )
    }

    /// All immediate Add operations. Ordinary actions and response windows are
    /// excluded: these cannot be used to satisfy the interrupted payment.
    pub(crate) fn resource_actions(&self, player: usize) -> Vec<Action> {
        if player >= self.players.len() || player != self.priority() {
            return Vec::new();
        }
        let mut result = Vec::new();
        for option in self.activated_options(player) {
            if activated_rules(option.card_id)[option.ability_index].adds_resources() {
                let action = Action::ActivateAbility {
                    source: option.source,
                    ability_index: option.ability_index,
                    targets: Vec::new(),
                };
                if match self.clone().apply(action.clone()) {
                    Ok(()) => true,
                    Err(e) => crate::numeric::is_capacity(&e),
                } {
                    result.push(action);
                }
            }
        }
        for gear in &self.players[player].gear {
            if self.gear_can_activate(player, gear.id) {
                result.push(Action::ActivateGear { gear_id: gear.id });
            }
        }
        if matches!(
            self.legend[player].kind,
            LegendKind::KaiSa | LegendKind::Darius
        ) {
            result.extend(self.legend_actions(player));
        }
        for domain in POWER_DOMAINS {
            for action in [
                Action::ActivateRune {
                    domain,
                    recycle: false,
                },
                Action::RecycleRune {
                    domain,
                    exhausted: true,
                },
                Action::RecycleRune {
                    domain,
                    exhausted: false,
                },
            ] {
                if match self.clone().apply(action.clone()) {
                    Ok(()) => true,
                    Err(e) => crate::numeric::is_capacity(&e),
                } {
                    result.push(action);
                }
            }
        }
        if let Some(Decision::ResourceConversion { direction, .. }) = self.decision {
            let maximum = self.conversion_available().unwrap_or(0);
            // Prefer useful amounts, retaining every amount (including zero).
            for amount in (0..=maximum).rev() {
                let action = Action::ResolveConversion { amount };
                if direction == ConversionDirection::EnergyToPower || amount == 0 {
                    result.push(action);
                } else {
                    // Generic power can consume different colors/runes. Keep
                    // every allocation, since later colored obligations differ.
                    let sources = self.power_sources(player);
                    let limits: Vec<u16> = sources
                        .colored
                        .iter()
                        .copied()
                        .chain(sources.recycle.iter().map(|&n| u16::from(n)))
                        .chain(std::iter::once(sources.any))
                        .collect();
                    fn payments(
                        limits: &[u16],
                        left: u16,
                        current: &mut Vec<u16>,
                        out: &mut Vec<Vec<u16>>,
                    ) {
                        if limits.is_empty() {
                            if left == 0 {
                                out.push(current.clone());
                            }
                            return;
                        }
                        let remaining_capacity = limits[1..]
                            .iter()
                            .map(|&n| u32::from(n))
                            .sum::<u32>()
                            .min(u32::from(left))
                            as u16;
                        for used in left.saturating_sub(remaining_capacity)..=left.min(limits[0]) {
                            current.push(used);
                            payments(&limits[1..], left - used, current, out);
                            current.pop();
                        }
                    }
                    let mut combinations = Vec::new();
                    payments(&limits, amount, &mut Vec::new(), &mut combinations);
                    for values in combinations {
                        let mut payment = PowerPayment {
                            any: values[12],
                            ..Default::default()
                        };
                        payment.colored.copy_from_slice(&values[..6]);
                        for d in 0..6 {
                            payment.recycle[d] = values[6 + d] as u8;
                        }
                        result.push(Action::WithPowerPayment {
                            action: Box::new(action.clone()),
                            payment,
                        });
                    }
                }
            }
        }
        result
    }

    /// Freeze the chosen intent before generating resources, so consuming an
    /// Empowered Gold or a Might source cannot recalculate a variable discount.
    pub(crate) fn funded_action_witness(
        &self,
        action: &Action,
    ) -> Result<Option<Vec<Action>>, String> {
        self.validate_numeric_state()?;
        let mut direct = self.clone();
        let initial = crate::numeric::legal_result(
            direct
                .apply(action.clone())
                .and_then(|()| direct.validate_numeric_state()),
        );
        if initial.as_ref().is_ok_and(|v| *v) {
            return Ok(Some(vec![action.clone()]));
        }
        let mut prepared = self.clone();
        match prepared.begin_payment(action.clone()) {
            Ok(()) => {}
            Err(e) if crate::numeric::is_capacity(&e) => return Err(e),
            Err(_) => return initial.map(|_| None),
        }
        let player = prepared.priority();
        let Some(path) = prepared.resource_witness(player, |state| {
            crate::numeric::legal_result(state.clone().complete_payment())
        })?
        else {
            return initial.map(|_| None);
        };
        let mut witness = vec![Action::BeginPayment {
            action: Box::new(action.clone()),
        }];
        witness.extend(path);
        witness.push(Action::CompletePayment);
        Ok(Some(witness))
    }

    // Positive-only AI suggestions. Exhaustion of this budget means Unknown,
    // never an impossibility result. Mandatory checks use resource_witness.
    fn resource_suggestion_actions(&self, player: usize) -> Vec<Action> {
        const EDGE_LIMIT: usize = 24;
        let mut actions = Vec::new();
        if let Some(Decision::ResourceConversion { direction, .. }) = self.decision {
            let maximum = self.conversion_available().unwrap_or(0);
            let preferred = self
                .payment_plan_view(player)
                .map(|plan| match direction {
                    ConversionDirection::PowerToEnergy => {
                        plan.energy
                            .saturating_sub(u32::from(self.players[player].energy))
                            .min(u32::from(maximum)) as u16
                    }
                    ConversionDirection::EnergyToPower => {
                        plan.power
                            .iter()
                            .map(|cost| u32::from(cost.amount))
                            .sum::<u32>()
                            .saturating_sub(self.power_sources(player).total())
                            .min(u32::from(maximum)) as u16
                    }
                })
                .unwrap_or(1)
                .min(maximum);
            let mut amounts = vec![preferred, 0, 1.min(maximum), maximum];
            amounts.dedup();
            for amount in amounts {
                if actions.len() >= EDGE_LIMIT {
                    break;
                }
                let action = Action::ResolveConversion { amount };
                if direction == ConversionDirection::EnergyToPower || amount == 0 {
                    actions.push(action);
                    continue;
                }
                let sources = self.power_sources(player);
                let limits: Vec<u16> = sources
                    .colored
                    .iter()
                    .copied()
                    .chain(sources.recycle.iter().map(|&n| u16::from(n)))
                    .chain(std::iter::once(sources.any))
                    .collect();
                fn allocations(
                    limits: &[u16],
                    left: u16,
                    path: &mut Vec<u16>,
                    out: &mut Vec<Vec<u16>>,
                    limit: usize,
                ) {
                    if out.len() >= limit {
                        return;
                    }
                    if limits.is_empty() {
                        if left == 0 {
                            out.push(path.clone());
                        }
                        return;
                    }
                    let capacity = limits[1..]
                        .iter()
                        .map(|&n| u32::from(n))
                        .sum::<u32>()
                        .min(u32::from(left)) as u16;
                    for n in left.saturating_sub(capacity)..=left.min(limits[0]) {
                        path.push(n);
                        allocations(&limits[1..], left - n, path, out, limit);
                        path.pop();
                        if out.len() >= limit {
                            break;
                        }
                    }
                }
                let mut choices = Vec::new();
                allocations(
                    &limits,
                    amount,
                    &mut Vec::new(),
                    &mut choices,
                    8.min(EDGE_LIMIT - actions.len()),
                );
                for choice in choices {
                    let mut payment = PowerPayment {
                        any: choice[12],
                        ..Default::default()
                    };
                    payment.colored.copy_from_slice(&choice[..6]);
                    for d in 0..6 {
                        payment.recycle[d] = choice[d + 6] as u8;
                    }
                    actions.push(Action::WithPowerPayment {
                        action: Box::new(action.clone()),
                        payment,
                    });
                }
            }
        }
        // Cost payment itself can tap/recycle runes. The suggestion search does
        // not enumerate equivalent explicit rune activation permutations.
        for option in self.activated_options(player) {
            if actions.len() >= EDGE_LIMIT {
                break;
            }
            if activated_rules(option.card_id)[option.ability_index].adds_resources() {
                let action = Action::ActivateAbility {
                    source: option.source,
                    ability_index: option.ability_index,
                    targets: Vec::new(),
                };
                if self.clone().apply(action.clone()).is_ok() {
                    actions.push(action);
                }
            }
        }
        for gear in &self.players[player].gear {
            if actions.len() >= EDGE_LIMIT {
                break;
            }
            if self.gear_can_activate(player, gear.id) {
                actions.push(Action::ActivateGear { gear_id: gear.id });
            }
        }
        if actions.len() < EDGE_LIMIT
            && matches!(
                self.legend[player].kind,
                LegendKind::KaiSa | LegendKind::Darius
            )
        {
            actions.extend(
                self.legend_actions(player)
                    .into_iter()
                    .take(EDGE_LIMIT - actions.len()),
            );
        }
        actions
    }

    fn resource_suggestion_witness(
        &self,
        player: usize,
        goal: impl Fn(&Game) -> bool,
        budget: usize,
    ) -> Option<Vec<Action>> {
        fn visit(
            state: &Game,
            player: usize,
            goal: &impl Fn(&Game) -> bool,
            remaining: &mut usize,
            seen: &mut HashSet<String>,
            path: &mut Vec<Action>,
        ) -> bool {
            if *remaining == 0 {
                return false;
            }
            *remaining -= 1;
            if goal(state) {
                return true;
            }
            if !seen.insert(state.resource_state_key(player)) {
                return false;
            }
            for action in state.resource_suggestion_actions(player) {
                if *remaining == 0 {
                    break;
                }
                let mut next = state.clone();
                if next.apply(action.clone()).is_err() {
                    continue;
                }
                path.push(action);
                if visit(&next, player, goal, remaining, seen, path) {
                    return true;
                }
                path.pop();
            }
            false
        }
        let mut path = Vec::new();
        let mut remaining = budget;
        visit(
            self,
            player,
            &goal,
            &mut remaining,
            &mut HashSet::new(),
            &mut path,
        )
        .then_some(path)
    }

    pub(crate) fn funded_suggestion_witness(
        &self,
        action: &Action,
        budget: usize,
    ) -> Option<Vec<Action>> {
        if budget == 0 {
            return None;
        }
        if self.clone().apply(action.clone()).is_ok() {
            return Some(vec![action.clone()]);
        }
        let mut prepared = self.clone();
        prepared.begin_payment(action.clone()).ok()?;
        let mut witness = vec![Action::BeginPayment {
            action: Box::new(action.clone()),
        }];
        witness.extend(prepared.resource_suggestion_witness(
            prepared.priority(),
            |state| state.clone().complete_payment().is_ok(),
            budget,
        )?);
        witness.push(Action::CompletePayment);
        Some(witness)
    }

    pub(crate) fn conversion_play_actions(&self) -> Vec<Action> {
        let player = self.priority();
        if self.decision.is_some()
            || !self.activated_options(player).iter().any(|option| {
                activated_rules(option.card_id)[option.ability_index]
                    .conversion_direction()
                    .is_some()
                    && self
                        .validate_ability(option.source, option.ability_index, &[])
                        .is_ok()
            })
        {
            return Vec::new();
        }
        let mut result = Vec::new();
        let mut candidates: Vec<_> = (0..self.players[player].hand.len()).collect();
        candidates.sort_by_key(|&index| self.catalog[self.players[player].hand[index]].cost);
        let mut attempted = 0;
        for hand_index in candidates {
            let intent = Action::PlayCard {
                hand_index,
                lane: None,
                target: None,
            };
            if self.clone().apply(intent.clone()).is_ok() {
                continue;
            }
            if attempted >= 2 {
                break;
            }
            attempted += 1;
            if let Some(witness) = self.funded_suggestion_witness(&intent, 48)
                && let Some(first) = witness.first()
            {
                result.push(first.clone());
            }
        }
        result
    }

    pub(crate) fn resource_completion_step(&self) -> Result<Option<Action>, String> {
        self.validate_numeric_state()?;
        let player = self.priority();
        let payment = self.payment_plan.is_some();
        let tax = self
            .resource_frames
            .iter()
            .filter_map(|f| f.suspended.as_ref())
            .find_map(|d| {
                if let Decision::CounterTax { player, cost, .. } = d {
                    Some((*player, *cost))
                } else {
                    None
                }
            });
        let goal = |state: &Game| -> Result<bool, String> {
            if payment {
                crate::numeric::legal_result(state.clone().complete_payment())
            } else if let Some((owner, cost)) = tax {
                Ok(matches!(state.decision, Some(Decision::CounterTax { .. }))
                    && state.players[owner].energy >= u16::from(cost))
            } else {
                Ok(!matches!(
                    state.decision,
                    Some(Decision::ResourceConversion { .. })
                ))
            }
        };
        if goal(self).unwrap_or(false) {
            return Ok(payment.then_some(Action::CompletePayment));
        }
        // Bounded positive search may skip a capacity-limited hypothetical branch.
        // An absent witness is Unknown and always falls through to the exact proof.
        if let Some(path) =
            self.resource_suggestion_witness(player, |s| goal(s).unwrap_or(false), 48)
        {
            return Ok(path.into_iter().next());
        }
        Ok(self
            .resource_witness(player, goal)?
            .and_then(|path| path.into_iter().next()))
    }

    /// None is a complete negative proof. A capacity-limited branch makes the
    /// negative outcome unresolved; another executable positive witness may win.
    pub(crate) fn resource_witness(
        &self,
        player: usize,
        goal: impl Fn(&Game) -> Result<bool, String>,
    ) -> Result<Option<Vec<Action>>, String> {
        fn visit(
            game: &Game,
            player: usize,
            goal: &impl Fn(&Game) -> Result<bool, String>,
            seen: &mut HashSet<String>,
            path: &mut Vec<Action>,
            capacity: &mut Option<String>,
        ) -> Result<bool, String> {
            match goal(game) {
                Ok(true) => return Ok(true),
                Ok(false) => {}
                Err(e) if crate::numeric::is_capacity(&e) => *capacity = Some(e),
                Err(e) => return Err(e),
            }
            if !seen.insert(game.resource_state_key(player)) {
                return Ok(false);
            }
            for action in game.resource_actions(player) {
                let mut next = game.clone();
                match next
                    .apply(action.clone())
                    .and_then(|()| next.validate_numeric_state())
                {
                    Ok(()) => {}
                    Err(e) if crate::numeric::is_capacity(&e) => {
                        *capacity = Some(e);
                        continue;
                    }
                    Err(_) => continue,
                }
                path.push(action);
                if visit(&next, player, goal, seen, path, capacity)? {
                    return Ok(true);
                }
                path.pop();
            }
            Ok(false)
        }
        if player >= self.players.len() {
            return Ok(None);
        }
        self.validate_numeric_state()?;
        let mut path = Vec::new();
        let mut capacity = None;
        if visit(
            self,
            player,
            &goal,
            &mut HashSet::new(),
            &mut path,
            &mut capacity,
        )? {
            Ok(Some(path))
        } else if let Some(e) = capacity {
            Err(e)
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Difficulty, Target, Unit};
    use crate::gear::GearState;

    fn fixture() -> Game {
        let mut g = Game::new_for_agents(1, 1, 4452, Difficulty::Tactical);
        g.start_match().unwrap();
        for p in &mut g.players {
            p.hand.clear();
            p.units.clear();
            p.gear.clear();
            p.discard.clear();
            p.champion = None;
            p.runes = [0; 6];
            p.exhausted_runes = [0; 6];
            p.energy = 0;
            p.energy_pool = 0;
            p.power_pool = [0; 6];
            p.any_power = 0;
        }
        g.legend[0].ready = false;
        g.legend[1].ready = false;
        g
    }
    fn add_gear(g: &mut Game, id: u32, card_id: usize) {
        g.players[0].gear.push(GearState {
            id,
            card_id,
            lane: None,
            attached_to: None,
            ready: true,
            temporary: false,
            token: false,
        });
    }
    fn heart() -> Action {
        Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        }
    }
    #[test]
    fn coupled_energy_and_power_have_an_exact_negative_and_positive_proof() {
        let mut g = fixture();
        add_gear(&mut g, 801, 244);
        g.players[0].hand = vec![230];
        g.players[0].energy = 3;
        g.players[0].energy_pool = 3;
        // Heart needs3energy+1power. Henge cannot spend the same energy twice.
        assert!(g.funded_action_witness(&heart()).unwrap().is_none());
        g.players[0].energy = 4;
        g.players[0].energy_pool = 4;
        let witness = g
            .funded_action_witness(&heart())
            .unwrap()
            .expect("Convert one and retain three");
        for step in witness {
            g.apply(step).unwrap();
        }
        assert!(g.players[0].gear.iter().any(|x| x.card_id == 230));
        assert_eq!(g.players[0].energy, 0);
        assert_eq!(g.players[0].any_power, 0);
    }
    #[test]
    fn search_can_generate_energy_inside_a_conversion_without_losing_payment() {
        let mut g = fixture();
        add_gear(&mut g, 802, 244);
        g.players[0].hand = vec![230];
        g.players[0].energy = 3;
        g.players[0].energy_pool = 3;
        g.players[0].units.push(Unit {
            id: 803,
            card_id: 236,
            lane: None,
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        });
        let witness = g
            .funded_action_witness(&heart())
            .unwrap()
            .expect("Sage supplies the fourth combined resource");
        for step in witness {
            g.apply(step).unwrap();
        }
        assert!(g.players[0].gear.iter().any(|x| x.card_id == 230));
        assert!(!g.players[0].units[0].ready);
    }
    #[test]
    fn anomaly_search_preserves_required_color_instead_of_using_default_order() {
        let mut g = fixture();
        add_gear(&mut g, 804, 245);
        // A source-agnostic resource fixture: exact source allocations matter
        // independently of whether a particular legend can include both cards.
        g.players[0].units.push(Unit {
            id: 805,
            card_id: 237,
            lane: Some(0),
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        });
        g.players[1].units.push(Unit {
            id: 806,
            card_id: 69,
            lane: None,
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        });
        g.players[0].power_pool[0] = 1;
        g.players[0].power_pool[1] = 3;
        // Frozen1Fury ability by itself needs no conversion. Freeze a 3-energy
        // plus1Fury card instead: Rengar costs4E1Fury, use1stored energy.
        g.players[0].hand = vec![98];
        g.players[0].energy = 1;
        g.players[0].energy_pool = 1;
        let witness = g
            .funded_action_witness(&heart())
            .unwrap()
            .expect("Spend Calm for energy, keep Fury");
        assert!(witness.iter().any(|a|matches!(a,Action::WithPowerPayment{payment,..} if payment.colored[1]>0 && payment.colored[0]==0)));
        for step in witness {
            g.apply(step).unwrap();
        }
        assert!(g.players[0].units.iter().any(|u| u.card_id == 98));
    }
    #[test]
    fn zero_input_converters_terminate_and_cannot_manufacture_resources() {
        let mut g = fixture();
        add_gear(&mut g, 807, 244);
        add_gear(&mut g, 808, 245);
        g.players[0].hand = vec![230];
        assert!(g.funded_action_witness(&heart()).unwrap().is_none());
    }
    #[test]
    fn triggered_deflect_feasibility_can_use_sage_then_henge() {
        let mut g = fixture();
        add_gear(&mut g, 809, 244);
        g.players[0].units.push(Unit {
            id: 810,
            card_id: 236,
            lane: None,
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        });
        g.players[1].units.push(Unit {
            id: 811,
            card_id: 0,
            lane: None,
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        });
        g.decision = Some(Decision::OnPlayTarget {
            player: 0,
            card_id: 23,
            source_id: 812,
            lane: None,
        });
        let action = Action::ChooseTarget {
            target: Some(Target::Unit {
                player: 1,
                unit_id: 811,
            }),
        };
        let witness = g.funded_action_witness(&action).unwrap().unwrap();
        for step in witness {
            g.apply(step).unwrap();
        }
        assert!(g.chain.iter().any(|i| i.card_id == 23));
    }
    #[test]
    fn suggestion_budget_exhaustion_does_not_change_mandatory_feasibility() {
        let mut g = fixture();
        add_gear(&mut g, 814, 244);
        g.players[0].hand = vec![230];
        g.players[0].energy = 4;
        g.players[0].energy_pool = 4;
        assert!(g.funded_suggestion_witness(&heart(), 0).is_none());
        assert!(g.funded_suggestion_witness(&heart(), 1).is_none());
        let path = g.funded_action_witness(&heart()).unwrap().unwrap();
        for action in path {
            g.apply(action).unwrap();
        }
        assert!(g.players[0].gear.iter().any(|gear| gear.card_id == 230));
    }

    #[test]
    fn converter_suggestions_bound_generation_even_with_large_mixed_pools() {
        let mut g = fixture();
        add_gear(&mut g, 815, 245);
        g.players[0].power_pool = [65_000; 6];
        g.apply(Action::ActivateAbility {
            source: crate::activated::AbilitySource::Gear {
                player: 0,
                gear_id: 815,
            },
            ability_index: 0,
            targets: Vec::new(),
        })
        .unwrap();
        let actions = g.resource_suggestion_actions(0);
        assert!(!actions.is_empty());
        assert!(actions.len() <= 24);
        for action in actions {
            g.clone().apply(action).unwrap();
        }
    }

    #[test]
    fn every_ai_level_executes_a_positive_converter_payment_witness() {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let mut g = fixture();
            g.difficulty = difficulty;
            add_gear(&mut g, 816, 244);
            g.players[0].hand = vec![98];
            g.players[0].energy = 5;
            g.players[0].energy_pool = 5;
            for _ in 0..5 {
                if g.players[0].units.iter().any(|unit| unit.card_id == 98) {
                    break;
                }
                g.step_ai().unwrap();
            }
            assert!(
                g.players[0].units.iter().any(|unit| unit.card_id == 98),
                "{difficulty:?} did not complete the witnessed conversion"
            );
            assert!(
                !g.players[0]
                    .gear
                    .iter()
                    .find(|gear| gear.id == 816)
                    .unwrap()
                    .ready
            );
            assert_eq!(g.players[0].energy, 0);
            assert_eq!(g.players[0].any_power, 0);
        }
    }

    #[test]
    fn external_begin_payment_uses_bounded_positive_continuation_without_a_cache() {
        let mut g = fixture();
        add_gear(&mut g, 818, 244);
        g.players[0].hand = vec![230];
        g.players[0].energy = 4;
        g.players[0].energy_pool = 4;
        g.apply(Action::BeginPayment {
            action: Box::new(heart()),
        })
        .unwrap();
        for _ in 0..3 {
            g.step_ai().unwrap();
        }
        assert!(g.players[0].gear.iter().any(|gear| gear.card_id == 230));
        assert!(g.payment_plan_view(0).is_none());
    }

    #[test]
    fn manual_payment_rollback_does_not_replay_stale_ai_resource_actions() {
        let mut g = fixture();
        add_gear(&mut g, 817, 244);
        g.players[0].hand = vec![230];
        g.players[0].energy = 4;
        g.players[0].energy_pool = 4;
        g.step_ai().unwrap();
        assert!(matches!(g.decision, Some(Decision::Payment { .. })));
        // No power has yet been generated, so the selected finalization cannot
        // currently finish and CR358.5 permits undo of the payment process.
        g.apply(Action::Choose { accept: false }).unwrap();
        assert!(g.decision.is_none());
        // The human changes the intended play after undo; no valid follow-up
        // can justify exhausting Henge for the abandoned transaction.
        g.players[0].hand.clear();
        g.step_ai().unwrap();
        assert!(
            g.players[0]
                .gear
                .iter()
                .find(|gear| gear.id == 817)
                .unwrap()
                .ready
        );
        assert!(!matches!(
            g.decision,
            Some(Decision::ResourceConversion { .. })
        ));
    }

    #[test]
    fn empowered_egg_resource_witness_uses_exact_yield_and_not_ordinary_empower() {
        for empowered in [false, true] {
            let mut g = fixture();
            add_gear(&mut g, 819, 252);
            add_gear(&mut g, 820, 244);
            if empowered {
                g.empowered_units.push(819);
            }
            g.players[0].hand = vec![230];
            g.players[0].energy = 2;
            g.players[0].energy_pool = 2;
            let witness = g.funded_action_witness(&heart()).unwrap();
            assert_eq!(
                witness.is_some(),
                empowered,
                "two stored + Egg yield must cover 3E + 1P"
            );
            if let Some(witness) = witness {
                for action in witness {
                    g.apply(action).unwrap();
                }
                assert!(g.players[0].gear.iter().any(|gear| gear.card_id == 230));
            }
        }
    }

    #[test]
    fn resource_proof_does_not_double_spend_eggs_exhaust_for_self_empower() {
        let mut g = fixture();
        add_gear(&mut g, 821, 252);
        let action = Action::ActivateAbility {
            source: crate::activated::AbilitySource::Gear {
                player: 0,
                gear_id: 821,
            },
            ability_index: 0,
            targets: vec![],
        };
        assert!(g.funded_action_witness(&action).unwrap().is_none());
        g.players[0].units.push(Unit {
            id: 822,
            card_id: 236,
            lane: None,
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        });
        let path = g.funded_action_witness(&action).unwrap().unwrap();
        for action in path {
            g.apply(action).unwrap();
        }
        assert!(!g.players[0].gear[0].ready);
        assert!(!g.unit_empowered(821), "status is the respondable effect");
        g.apply(Action::Pass).unwrap();
        g.apply(Action::Pass).unwrap();
        assert!(g.unit_empowered(821));
    }

    #[test]
    fn strategic_payment_continuation_ignores_opponent_private_zones() {
        let mut g = fixture();
        g.difficulty = Difficulty::Strategic;
        add_gear(&mut g, 813, 244);
        g.players[0].hand = vec![230];
        g.players[0].energy = 4;
        g.players[0].energy_pool = 4;
        let mut other = g.clone();
        other.players[1].hand = vec![7, 7, 7];
        other.players[1].deck = vec![98; 30];
        g.begin_payment(heart()).unwrap();
        other.begin_payment(heart()).unwrap();
        g.step_ai().unwrap();
        other.step_ai().unwrap();
        assert_eq!(g.decision, other.decision);
        assert_eq!(g.players[0].energy, other.players[0].energy);
        assert_eq!(g.players[0].gear, other.players[0].gear);
    }
    #[test]
    fn exact_resource_proof_preserves_capacity_without_hiding_a_positive_alternative() {
        let mut game = fixture();
        let before = format!("{game:?}");
        assert!(game.resource_witness(0, |_| Ok(false)).unwrap().is_none());
        let error = game
            .resource_witness(0, |_| Err(crate::numeric::capacity("probe")))
            .unwrap_err();
        assert!(error.starts_with("engine_capacity:"));
        assert_eq!(format!("{game:?}"), before);
        // A bounded suggestion can decline a hypothetical branch, but only an
        // exact search establishes impossibility. A legal Add can still yield
        // a complete executable witness after the initial goal is unresolved.
        game.players[0].runes[0] = 1;
        let path = game
            .resource_witness(0, |state| {
                if state.players[0].energy > 0 {
                    Ok(true)
                } else {
                    Err(crate::numeric::capacity("unresolved completion"))
                }
            })
            .unwrap()
            .expect("the rune is a positive witness");
        for action in path {
            game.apply(action).unwrap();
        }
        assert_eq!(game.players[0].energy, 1);
    }
}
