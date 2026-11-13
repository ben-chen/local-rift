//! Alpha Strike: finalized recipients, resolution-time division and actual deaths.
use super::*;

#[derive(Clone, Debug)]
pub(super) struct AlphaKillWatch {
    player: usize,
    /// Repeat adds instructions to this physical parent (CR820.1.d), rather
    /// than creating another spell with an independent kill attribution.
    chain_id: u32,
    clauses: usize,
    recipients: Vec<Target>,
}

impl Game {
    pub(super) fn alpha_slot_valid(&self, actor: usize, target: Target, slot: usize) -> bool {
        let Target::Unit { player, .. } = target else {
            return false;
        };
        self.can_choose_unit(actor, target)
            && if slot == 0 {
                player == actor
            } else {
                player != actor
                    && self
                        .unit_ref(target)
                        .is_some_and(|unit| unit.lane.is_some())
            }
    }

    pub(super) fn alpha_target_spec(
        &self,
        actor: usize,
        prior: &[Target],
    ) -> Result<rules::PlayTargetSpec, String> {
        let mut maximum_might = 0;
        if let Some(&source) = prior.first() {
            if let Some(unit) = self.unit_ref(source) {
                maximum_might = self.unit_might(unit)?;
            }
        } else {
            for unit in &self.players[actor].units {
                maximum_might = maximum_might.max(self.unit_might(unit)?);
            }
        }
        let available = self.players[1 - actor]
            .units
            .iter()
            .filter(|unit| {
                self.alpha_slot_valid(
                    actor,
                    Target::Unit {
                        player: 1 - actor,
                        unit_id: unit.id,
                    },
                    1,
                )
            })
            .count();
        Ok(rules::PlayTargetSpec {
            kind: Some(rules::PlayTargetKind::Unit),
            maximum: 1 + available.min(usize::try_from(maximum_might).unwrap_or(usize::MAX)),
            required: 2,
        })
    }

    pub(super) fn alpha_candidates(
        &self,
        actor: usize,
        prior: &[Target],
    ) -> Result<Vec<Target>, String> {
        if actor >= 2 {
            return Ok(vec![]);
        }
        if prior.is_empty() {
            if !self.players[1 - actor].units.iter().any(|unit| {
                self.alpha_slot_valid(
                    actor,
                    Target::Unit {
                        player: 1 - actor,
                        unit_id: unit.id,
                    },
                    1,
                )
            }) {
                return Ok(vec![]);
            }
            let mut ranked = Vec::new();
            for unit in &self.players[actor].units {
                let target = Target::Unit {
                    player: actor,
                    unit_id: unit.id,
                };
                let might = self.unit_might(unit)?;
                if might > 0 && self.alpha_slot_valid(actor, target, 0) {
                    ranked.push((might, target));
                }
            }
            ranked.sort_by_key(|(might, _)| std::cmp::Reverse(*might));
            return Ok(ranked.into_iter().map(|(_, target)| target).collect());
        }
        for (slot, &target) in prior.iter().enumerate() {
            if !self.alpha_slot_valid(actor, target, slot) || prior[..slot].contains(&target) {
                return Ok(vec![]);
            }
        }
        let might = self.unit_might(self.unit_ref(prior[0]).unwrap())?;
        if (prior.len() - 1) as u128 >= u128::from(might) {
            return Ok(vec![]);
        }
        let mut ranked = Vec::new();
        for unit in &self.players[1 - actor].units {
            let target = Target::Unit {
                player: 1 - actor,
                unit_id: unit.id,
            };
            if self.alpha_slot_valid(actor, target, 1) && !prior.contains(&target) {
                ranked.push((
                    self.unit_might(unit)?.saturating_sub(unit.damage).max(1),
                    target,
                ));
            }
        }
        ranked.sort_by_key(|(remaining, _)| *remaining);
        Ok(ranked.into_iter().map(|(_, target)| target).collect())
    }

    pub(super) fn prepare_alpha_targets(
        &self,
        actor: usize,
        provided: &[Target],
        automatic: bool,
    ) -> Result<Vec<Target>, String> {
        for (slot, target) in provided.iter().enumerate() {
            if !self
                .alpha_candidates(actor, &provided[..slot])?
                .contains(target)
            {
                return Err("Choose a friendly source, then distinct enemy battlefield units, no more than its current Might.".into());
            }
        }
        let mut targets = provided.to_vec();
        if automatic {
            // Choosing every available enemy can force one point onto too many
            // recipients and prevent an otherwise affordable kill. Consider each
            // growing prefix; explicit player target lists are left untouched.
            let mut best = None;
            let mut best_value = i128::MIN;
            if targets.len() >= 2 {
                best_value = self.alpha_ai_score(actor, &targets)?
                    - i128::from(self.deflect_cost(actor, &targets)?) * 8;
                best = Some(targets.clone());
            }
            while let Some(target) = self.alpha_candidates(actor, &targets)?.first().copied() {
                targets.push(target);
                if targets.len() >= 2 {
                    let value = self.alpha_ai_score(actor, &targets)?
                        - i128::from(self.deflect_cost(actor, &targets)?) * 8;
                    if value > best_value {
                        best_value = value;
                        best = Some(targets.clone());
                    }
                }
            }
            if let Some(preferred) = best {
                targets = preferred;
            }
        }
        if targets.len() < 2 {
            return Err("Alpha Strike needs a friendly unit with damage to divide and at least one enemy battlefield target.".into());
        }
        Ok(targets)
    }

    pub(super) fn begin_alpha_split(
        &mut self,
        player: usize,
        targets: &[Target],
    ) -> Result<(), String> {
        let Some(&source) = targets.first() else {
            return Ok(());
        };
        if !self.alpha_slot_valid(player, source, 0)
            || !self.current_hidden_slot_valid(443, 0, source)
        {
            return Ok(());
        }
        let total = self.unit_might(self.unit_ref(source).unwrap())?;
        let targets: Vec<_> = targets
            .iter()
            .copied()
            .enumerate()
            .skip(1)
            .filter(|&(slot, target)| {
                self.alpha_slot_valid(player, target, slot)
                    && self.current_hidden_slot_valid(443, slot, target)
            })
            .map(|(_, target)| target)
            .collect();
        if targets.is_empty() {
            return Ok(());
        }
        // CR053.2 and429.5.c: "this kills" references the physical spell's
        // attributed kills. Each non-ignored repeated reflexive clause observes
        // that same kill set after the whole parent finishes (CR321,388–389).
        // A source or all recipients mistargeting ignores the linked clause
        // (CR359.3.e.14.a); a legal source with zero Might does not ignore it.
        let chain_id = self.alpha_parent_id()?;
        if let Some(watch) = self
            .alpha_kill_watches
            .iter_mut()
            .find(|w| w.chain_id == chain_id)
        {
            watch.clauses += 1;
        } else {
            self.alpha_kill_watches.push(AlphaKillWatch {
                player,
                chain_id,
                clauses: 1,
                recipients: Vec::new(),
            });
        }
        if total == 0 {
            return Ok(());
        }
        let keep = targets
            .len()
            .min(usize::try_from(total).unwrap_or(usize::MAX));
        self.decision = Some(Decision::SplitDamage {
            player,
            card_id: 443,
            source,
            total,
            targets: targets.clone(),
            keep,
        });
        // A single recipient or exactly one point per recipient has no choice.
        if targets.len() == 1 || u128::from(total) == targets.len() as u128 {
            let allocation = self.automatic_split_assignment(&targets, total, keep)?;
            self.finish_split_assignment(allocation)?;
        }
        Ok(())
    }

    /// The same exact assignment action is shared with combat, with different
    /// validation: a split has no lethal-first or damage-priority restrictions.
    pub(super) fn assign_split_damage(
        &mut self,
        assignments: Vec<(u32, u64)>,
    ) -> Result<(), String> {
        self.finish_split_assignment(assignments)?;
        self.after_resolution()
    }

    fn finish_split_assignment(&mut self, assignments: Vec<(u32, u64)>) -> Result<(), String> {
        let Some(Decision::SplitDamage {
            player,
            total,
            targets,
            keep,
            ..
        }) = self.decision.clone()
        else {
            return Err("No split-damage choice is pending.".into());
        };
        if assignments.len() != keep
            || assignments
                .iter()
                .map(|(_, amount)| u128::from(*amount))
                .sum::<u128>()
                != u128::from(total)
        {
            return Err(format!(
                "Assign all {total} damage to exactly {keep} of the original legal recipients."
            ));
        }
        let mut recipients = Vec::new();
        for (index, &(unit_id, amount)) in assignments.iter().enumerate() {
            if amount == 0 || assignments[..index].iter().any(|(id, _)| *id == unit_id) {
                return Err(
                    "Each retained recipient needs a positive amount; a unit cannot occur twice."
                        .into(),
                );
            }
            let target = targets
                .iter()
                .copied()
                .find(|target| matches!(target, Target::Unit { unit_id: id, .. } if *id == unit_id))
                .ok_or("Allocate only to the original legal recipients.")?;
            recipients.push(target);
        }
        for (&target, &(_, amount)) in recipients.iter().zip(&assignments) {
            self.deal_damage(
                damage::DamageContext {
                    player,
                    bonus_damage: 0,
                    dealer: damage::DamageDealer::Unit,
                },
                target,
                amount,
            )?;
        }
        let chain_id = self.alpha_parent_id()?;
        let watch = self
            .alpha_kill_watches
            .iter_mut()
            .find(|watch| watch.chain_id == chain_id)
            .ok_or("Alpha Strike's physical parent has no kill observer.")?;
        for target in recipients {
            if !watch.recipients.contains(&target) {
                watch.recipients.push(target);
            }
        }
        self.decision = None;
        Ok(())
    }

    fn alpha_parent_id(&self) -> Result<u32, String> {
        self.resolution_frame
            .as_ref()
            .map(|frame| frame.chain_id)
            .ok_or_else(|| "Alpha Strike needs its resolving physical spell.".into())
    }

    /// Public-board heuristic: retain vulnerable recipients, then finish kills.
    /// Returned assignments are suggestions; callers may provide any valid split.
    pub fn automatic_split_assignment(
        &self,
        targets: &[Target],
        total: u64,
        keep: usize,
    ) -> Result<Vec<(u32, u64)>, String> {
        if keep == 0 || keep > targets.len() || keep as u128 > u128::from(total) {
            return Err("The split has no valid recipient count.".into());
        }
        let mut ranked = Vec::new();
        for &target in targets {
            let unit = self
                .unit_ref(target)
                .ok_or("A split recipient is missing.")?;
            ranked.push((
                self.unit_might(unit)?.saturating_sub(unit.damage).max(1),
                unit.id,
            ));
        }
        ranked.sort_unstable();
        ranked.truncate(keep);
        let mut remaining = total - keep as u64;
        let mut assignments = Vec::new();
        for (lethal, id) in ranked {
            let extra = remaining.min(lethal - 1);
            remaining -= extra;
            assignments.push((id, 1 + extra));
        }
        // Over-assignment is allowed for a noncombat split.
        assignments.last_mut().unwrap().1 += remaining;
        Ok(assignments)
    }

    pub(super) fn capture_alpha_kills(&mut self, dying: &[(usize, Unit)]) {
        let mut rewards = Vec::new();
        for watch in &mut self.alpha_kill_watches {
            for &(player, ref unit) in dying {
                let target = Target::Unit {
                    player,
                    unit_id: unit.id,
                };
                if watch.recipients.contains(&target) {
                    watch.recipients.retain(|candidate| *candidate != target);
                    rewards.extend(std::iter::repeat_n(watch.player, watch.clauses));
                }
            }
        }
        for player in rewards {
            self.queue_ability(
                player,
                443,
                vec![],
                ChainAbility::Triggered(TriggeredAbility::Xp(1)),
            );
        }
    }

    pub(super) fn alpha_ai_score(
        &self,
        _player: usize,
        targets: &[Target],
    ) -> Result<i128, String> {
        let Some(source) = targets.first().and_then(|target| self.unit_ref(*target)) else {
            return Ok(0);
        };
        let total = self.unit_might(source)?;
        let keep = targets
            .len()
            .saturating_sub(1)
            .min(usize::try_from(total).unwrap_or(usize::MAX));
        if keep == 0 {
            return Ok(0);
        }
        let allocation = self.automatic_split_assignment(&targets[1..], total, keep)?;
        let mut score = 0;
        for (id, damage) in allocation {
            if let Some(unit) = self
                .players
                .iter()
                .flat_map(|p| &p.units)
                .find(|unit| unit.id == id)
            {
                score += if u128::from(unit.damage) + u128::from(damage)
                    >= u128::from(self.unit_might(unit)?)
                {
                    100
                } else {
                    i128::from(damage) * 3
                };
            }
        }
        Ok(score)
    }
}
