//! Collective target restrictions (CR355.11). Never enumerate all subsets.
use super::*;

impl Game {
    pub fn targets_fit_might_budget(
        &self,
        targets: &[Target],
        maximum: u64,
    ) -> Result<bool, String> {
        let mut remaining = maximum;
        for &target in targets {
            let Some(unit) = self.unit_ref(target) else {
                return Ok(false);
            };
            let Some(next) = remaining.checked_sub(self.unit_might(unit)?) else {
                return Ok(false);
            };
            remaining = next;
        }
        Ok(true)
    }

    pub(super) fn target_group_fits(
        &self,
        card: CardId,
        targets: &[Target],
    ) -> Result<bool, String> {
        match rules::targeting(card).maximum_total_might {
            Some(maximum) => self.targets_fit_might_budget(targets, maximum),
            None => Ok(true),
        }
    }

    pub(super) fn begin_target_subset(
        &mut self,
        item: &ChainItem,
        execution: usize,
    ) -> Result<bool, String> {
        let Some(maximum) = rules::targeting(item.card_id).maximum_total_might else {
            return Ok(false);
        };
        let mut candidates = Vec::new();
        for (slot, &target) in item.targets.iter().enumerate() {
            if self.official_target_valid(item.player, item.card_id, item.lane, target, slot)?
                && self.current_hidden_slot_valid(item.card_id, slot, target)
            {
                candidates.push(target);
            }
        }
        if self.targets_fit_might_budget(&candidates, maximum)? {
            return Ok(false);
        }
        self.decision = Some(Decision::ResolutionSubset {
            player: item.player,
            card_id: item.card_id,
            candidates,
            maximum,
            execution,
        });
        Ok(true)
    }

    /// A subset is maximal when no other original eligible target can be added.
    /// A smaller cardinality subset can be valid; zero-Might targets must remain.
    pub fn validate_resolution_subset(&self, targets: &[Target]) -> Result<(), String> {
        let Some(Decision::ResolutionSubset {
            candidates,
            maximum,
            ..
        }) = &self.decision
        else {
            return Err("No target subset choice is pending.".into());
        };
        for (index, target) in targets.iter().enumerate() {
            if !candidates.contains(target) || targets[..index].contains(target) {
                return Err("Choose each eligible original target at most once.".into());
            }
        }
        if !self.targets_fit_might_budget(targets, *maximum)? {
            return Err(format!(
                "The chosen units must total at most {maximum} Might."
            ));
        }
        let mut extended = targets.to_vec();
        for target in candidates.iter().filter(|target| !targets.contains(target)) {
            extended.push(*target);
            if self.targets_fit_might_budget(&extended, *maximum)? {
                return Err("Another original target still fits. Include it or choose a different legal subset.".into());
            }
            extended.pop();
        }
        Ok(())
    }

    pub(super) fn choose_resolution_subset(&mut self, targets: Vec<Target>) -> Result<(), String> {
        self.validate_resolution_subset(&targets)?;
        let Some(Decision::ResolutionSubset { execution, .. }) = self.decision else {
            unreachable!()
        };
        let item = self
            .chain
            .last_mut()
            .ok_or("The resolving spell is missing.")?;
        let choices = item
            .executions
            .get_mut(execution)
            .ok_or("The resolving execution is missing.")?;
        choices.targets = targets.clone();
        if execution == 0 {
            item.target = targets.first().copied();
            item.targets = targets;
        } else if execution == 1 {
            item.repeat_targets = targets;
        }
        self.decision = None;
        self.resolve_top_chain()
    }

    /// Bounded suggestion set for bots; explicit actions accept every legal subset.
    pub(super) fn resolution_subset_suggestions(
        &self,
        player: usize,
        candidates: &[Target],
        maximum: u64,
    ) -> Result<Vec<Action>, String> {
        let mut ranked = candidates
            .iter()
            .map(|&target| Ok((self.return_target_score(player, target)?, target)))
            .collect::<Result<Vec<_>, String>>()?;
        ranked.sort_by_key(|entry| std::cmp::Reverse(entry.0));
        let mut groups = Vec::new();
        // Try value-first and Might-first paths, without an exponential powerset.
        for by_might in [false, true] {
            let mut order = ranked.clone();
            if by_might {
                let mut weighted = order
                    .iter()
                    .map(|&(score, target)| {
                        Ok((
                            self.unit_might(
                                self.unit_ref(target).ok_or("Missing subset target.")?,
                            )?,
                            score,
                            target,
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                weighted.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
                order = weighted
                    .into_iter()
                    .map(|(_, score, target)| (score, target))
                    .collect();
            }
            let mut selected = Vec::new();
            for (_, target) in order {
                selected.push(target);
                if !self.targets_fit_might_budget(&selected, maximum)? {
                    selected.pop();
                }
            }
            self.validate_resolution_subset(&selected)?;
            let action = Action::ChooseResolutionSubset { targets: selected };
            if !groups.contains(&action) {
                groups.push(action);
            }
        }
        Ok(groups)
    }
}
