//! Native drafts for public unit and gear activated abilities.
use super::*;
use local_rift::activated::{AbilitySource, AbilityTiming, ActivatedRule, activated_rules};

pub(super) fn cost_label(rule: &ActivatedRule) -> String {
    let mut costs = Vec::new();
    if rule.energy > 0 {
        costs.push(format!("{} energy", rule.energy));
    }
    if rule.power > 0 {
        costs.push(format!(
            "{} {} power",
            rule.power,
            rule.domain.map_or("Any", |domain| domain.name())
        ));
    }
    if rule.exhaust {
        costs.push("exhaust source".to_owned());
    }
    if costs.is_empty() {
        "No resource cost".into()
    } else {
        costs.join(" · ")
    }
}
pub(super) fn timing_label(rule: &ActivatedRule) -> &'static str {
    match rule.timing {
        AbilityTiming::Main => "Your open Main Phase",
        AbilityTiming::Action => "Action · an open window",
        AbilityTiming::Reaction => "Reaction · any priority window",
    }
}
impl App {
    pub(super) fn source_abilities(&self, source: AbilitySource) -> Vec<(usize, usize)> {
        self.game
            .activated_options(source.player())
            .into_iter()
            .filter(|option| option.source == source)
            .map(|option| (option.card_id, option.ability_index))
            .collect()
    }
    pub(super) fn ability_rule(
        &self,
        source: AbilitySource,
        index: usize,
    ) -> Option<(usize, &'static ActivatedRule)> {
        self.source_abilities(source)
            .into_iter()
            .find(|(_, i)| *i == index)
            .and_then(|(id, i)| activated_rules(id).get(i).map(|rule| (id, rule)))
    }
    pub(super) fn open_abilities(&mut self, source: AbilitySource) {
        self.selection = Some(Selection::Ability(source, None));
        self.ability_targets.clear();
        self.group.clear();
        self.drag = None;
        self.hover_card = None;
        self.hover_rect = None;
    }
    pub(super) fn ability_target_candidate(&self, target: Target) -> bool {
        matches!(self.selection, Some(Selection::Ability(source,Some(index)))
            if source.player()==0 && self.game.ability_targets(source,index).contains(&target))
    }
    pub(super) fn ability_panel(&mut self, source: AbilitySource, index: Option<usize>) {
        let options = self.source_abilities(source);
        let Some((card_id, _)) = options.first().copied() else {
            self.selection = None;
            self.ui.wrapped(
                "This ability's source is no longer in play.",
                1178.0,
                375.0,
                212.0,
                13.0,
                MUTED,
            );
            return;
        };
        self.ui.ellipsis(
            self.game.catalog[card_id].name,
            1178.0,
            363.0,
            212.0,
            16.0,
            TEXT,
        );
        if let Some(index) = index {
            let Some((_, rule)) = self.ability_rule(source, index) else {
                self.selection = None;
                return;
            };
            self.ui
                .ellipsis(rule.label, 1178.0, 387.0, 212.0, 12.0, GOLD);
            let deflect: u16 = self
                .ability_targets
                .iter()
                .filter_map(|target| match target {
                    Target::Unit { player, unit_id } if *player != source.player() => {
                        self.game.players[*player]
                            .units
                            .iter()
                            .find(|unit| unit.id == *unit_id)
                            .map(|unit| u16::from(self.game.unit_keywords(unit).deflect))
                    }
                    _ => None,
                })
                .sum();
            let cost = if deflect > 0 {
                format!("{} · +{deflect} Any power (Deflect)", cost_label(rule))
            } else {
                cost_label(rule)
            };
            self.ui
                .wrapped_lines(&cost, 1178.0, 408.0, 212.0, 10.0, TEXT, 2);
            self.ui.text(timing_label(rule), 1178.0, 439.0, 10.0, MUTED);
            let status = if source.player() != 0 {
                "Opponent ability · read only".to_owned()
            } else if self.ability_targets.len() < rule.targets.min as usize {
                format!(
                    "Choose {} highlighted unit{}{}.",
                    rule.targets.min,
                    if rule.targets.min == 1 { "" } else { "s" },
                    if rule.targets.battlefield {
                        " at a battlefield"
                    } else {
                        ""
                    }
                )
            } else {
                self.game
                    .validate_ability(source, index, &self.ability_targets)
                    .err()
                    .unwrap_or_else(|| {
                        if rule.adds_resources() {
                            "Adds resources immediately; no response window.".into()
                        } else {
                            self.ability_targets
                                .first()
                                .and_then(|target| match target {
                                    Target::Unit { player, unit_id } => self.game.players[*player]
                                        .units
                                        .iter()
                                        .find(|unit| unit.id == *unit_id)
                                        .map(|unit| {
                                            format!(
                                                "Target: {}. Responses open after paying.",
                                                self.game.unit_name(unit)
                                            )
                                        }),
                                    _ => None,
                                })
                                .unwrap_or_else(|| {
                                    "Ready. Opponents can respond after you pay.".into()
                                })
                        }
                    })
            };
            self.ui
                .wrapped_lines(&status, 1178.0, 461.0, 212.0, 10.0, TEAL, 2);
            let action = Action::ActivateAbility {
                source,
                ability_index: index,
                targets: self.ability_targets.clone(),
            };
            let valid = source.player() == 0
                && self
                    .game
                    .validate_ability(source, index, &self.ability_targets)
                    .is_ok();
            if self.ui.button(
                Rect::new(1178.0, 493.0, 212.0, 34.0),
                "Pay & use ability",
                true,
                valid,
            ) {
                self.action(action);
            }
            if self.ui.button(
                Rect::new(1178.0, 534.0, 212.0, 27.0),
                "Back · keep costs unspent",
                false,
                true,
            ) {
                self.open_abilities(source);
            }
        } else {
            self.ui.text(
                if source.player() == 0 {
                    "CHOOSE AN ABILITY"
                } else {
                    "OPPONENT ABILITIES · READ ONLY"
                },
                1178.0,
                386.0,
                9.0,
                TEAL,
            );
            for (slot, (card_id, index)) in options.iter().enumerate() {
                let rule = &activated_rules(*card_id)[*index];
                let y = 401.0 + slot as f32 * 64.0;
                if self
                    .ui
                    .button(Rect::new(1178.0, y, 212.0, 32.0), rule.label, false, true)
                {
                    self.selection = Some(Selection::Ability(source, Some(*index)));
                    self.ability_targets.clear();
                }
                self.ui
                    .ellipsis(&cost_label(rule), 1178.0, y + 48.0, 212.0, 10.0, MUTED);
            }
            if self.ui.button(
                Rect::new(1178.0, 534.0, 212.0, 27.0),
                "Back to selected card",
                false,
                true,
            ) {
                self.selection = if source.player() != 0 || self.game.decision.is_some() {
                    None
                } else {
                    Some(match source {
                        AbilitySource::Unit { unit_id, .. } => Selection::Unit(unit_id),
                        AbilitySource::Gear { gear_id, .. } => Selection::Gear(gear_id),
                    })
                };
                self.ability_targets.clear();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn separate_renata_costs_and_dragonsoul_timing_are_explicit() {
        let renata = activated_rules(239);
        assert_eq!(cost_label(&renata[0]), "1 energy · 1 Mind power");
        assert_eq!(
            cost_label(&renata[1]),
            "4 energy · 4 Mind power · exhaust source"
        );
        assert_eq!(timing_label(&renata[0]), "Your open Main Phase");
        let sage = &activated_rules(236)[0];
        assert_eq!(cost_label(sage), "exhaust source");
        assert_eq!(timing_label(sage), "Reaction · any priority window");
    }
}
