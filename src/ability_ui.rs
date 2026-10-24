//! Native drafts for public unit and gear activated abilities.
use super::*;
use local_rift::activated::{
    AbilitySource, AbilityTargetKind, AbilityTiming, ActivatedRule, ConversionDirection,
    activated_rules,
};

/// User selections survive resource visits; these are not game-state snapshots.
#[derive(Clone)]
pub(super) struct UiDraft {
    selection: Option<Selection>,
    targets: Vec<Target>,
    ability_targets: Vec<Target>,
    ability_costs: Vec<usize>,
    repeat_targets: Vec<Target>,
    editing_repeat: bool,
    destination: Option<usize>,
    choices: Vec<usize>,
    options: PlayOptions,
    play_draft: Option<play_ui::PlayDraft>,
    flow_source: Option<FlowSource>,
    tray: Tray,
    conversion_amount: u16,
    conversion_editor: Option<String>,
    conversion_key: Option<String>,
}
#[derive(Clone)]
pub(super) struct SuspendedPayment {
    draft: PaymentDraft,
    ui: UiDraft,
    decision: String,
    owns_plan: bool,
}
impl App {
    fn decision_key(&self) -> String {
        format!("{:?}", self.game.decision)
    }
    fn ui_draft(&self) -> UiDraft {
        UiDraft {
            selection: self.selection,
            targets: self.targets.clone(),
            ability_targets: self.ability_targets.clone(),
            ability_costs: self.ability_costs.clone(),
            repeat_targets: self.repeat_targets.clone(),
            editing_repeat: self.editing_repeat,
            destination: self.destination,
            choices: self.choices.clone(),
            options: self.options,
            play_draft: self.play_draft.clone(),
            flow_source: self.flow_source,
            tray: self.tray,
            conversion_amount: self.conversion_amount,
            conversion_editor: self.conversion_editor.clone(),
            conversion_key: self.conversion_key.clone(),
        }
    }
    pub(super) fn restore_ui_draft(&mut self, draft: UiDraft) {
        self.selection = draft.selection;
        self.targets = draft.targets;
        self.ability_targets = draft.ability_targets;
        self.ability_costs = draft.ability_costs;
        self.repeat_targets = draft.repeat_targets;
        self.editing_repeat = draft.editing_repeat;
        self.destination = draft.destination;
        self.choices = draft.choices;
        self.options = draft.options;
        self.play_draft = draft.play_draft;
        self.flow_source = draft.flow_source;
        self.tray = draft.tray;
        self.conversion_amount = draft.conversion_amount;
        self.conversion_editor = draft.conversion_editor;
        self.conversion_key = draft.conversion_key;
        self.trigger_choice_key = self
            .game
            .decision
            .as_ref()
            .filter(|decision| matches!(decision, Decision::TriggerTargets { .. }))
            .map(|decision| format!("{decision:?}"));
    }
    pub(super) fn remember_resource_draft(&mut self) {
        if self.game.resource_payment_pending() {
            let key = self.decision_key();
            if self
                .resource_drafts
                .last()
                .is_none_or(|(saved, _)| *saved != key)
            {
                let mut draft = self.ui_draft();
                if matches!(
                    draft.selection,
                    Some(
                        Selection::Ability(..)
                            | Selection::Gear(_)
                            | Selection::Unit(_)
                            | Selection::Legend
                    )
                ) {
                    draft.selection = None;
                }
                self.resource_drafts.push((key, draft));
                self.conversion_editor = None;
            }
        }
    }
    pub(super) fn restore_resource_draft(&mut self) -> bool {
        let key = self.decision_key();
        if self
            .resource_drafts
            .last()
            .is_some_and(|(saved, _)| *saved == key)
        {
            let (_, draft) = self.resource_drafts.pop().unwrap();
            self.restore_ui_draft(draft);
            return true;
        }
        false
    }
    pub(super) fn resource_visit_action_allowed(&self, action: &Action) -> Result<bool, String> {
        Ok(match action {
            Action::WithPowerPayment { action, .. } | Action::WithPlayChoices { action, .. } => {
                self.resource_visit_action_allowed(action)?
            }
            Action::ActivateRune { .. }
            | Action::RecycleRune { .. }
            | Action::ActivateGear { .. } => true,
            Action::ActivateLegend { .. } => matches!(
                self.game.legend[0].kind,
                local_rift::legends::LegendKind::KaiSa | local_rift::legends::LegendKind::Darius
            ),
            Action::ActivateAbility {
                source,
                ability_index,
                ..
            }
            | Action::ActivateAbilityWithCosts {
                source,
                ability_index,
                ..
            } => self
                .ability_rule(*source, *ability_index)
                .is_some_and(|(_, rule)| rule.adds_resources()),
            Action::ResolveConversion { .. } | Action::CompletePayment => true,
            Action::Choose { accept: false }
                if matches!(self.game.decision, Some(Decision::Payment { .. })) =>
            {
                self.game.can_cancel_payment()?
            }
            _ => self
                .suspended_payments
                .last()
                .is_some_and(|saved| saved.decision != self.decision_key()),
        })
    }
    pub(super) fn can_begin_payment(&self, action: &Action) -> bool {
        self.game.payment_plan_view(0).is_none()
            && self
                .game
                .clone()
                .apply(Action::BeginPayment {
                    action: Box::new(action.clone()),
                })
                .is_ok()
    }
    pub(super) fn begin_ui_payment(&mut self, action: Action) -> bool {
        let action = self.canonical_action(action);
        match self.maybe_review_cost(&action) {
            Ok(true) => return false,
            Ok(false) => {}
            Err(error) => {
                self.notify(error);
                return false;
            }
        }
        let original = self.ui_draft();
        match self.game.apply(Action::BeginPayment {
            action: Box::new(action),
        }) {
            Ok(()) => {
                self.payment_origin = Some(original);
                self.selection = None;
                self.tray = Tray::Loadout;
                self.gear_owner = 0;
                self.hand_scroll = 0;
                true
            }
            Err(error) => {
                self.notify(error);
                false
            }
        }
    }
    pub(super) fn suspend_payment(&mut self) {
        let Some(draft) = self.payment_draft.clone() else {
            return;
        };
        let ui = self.ui_draft();
        let owns_plan = self.game.payment_plan_view(0).is_none()
            && !matches!(
                self.game.decision,
                Some(Decision::ResourceConversion { .. })
            );
        if owns_plan && !self.begin_ui_payment(draft.action.clone()) {
            return;
        }
        self.payment_draft = None;
        self.suspended_payments.push(SuspendedPayment {
            draft,
            ui,
            decision: self.decision_key(),
            owns_plan,
        });
        self.selection = None;
        self.tray = Tray::Loadout;
        self.gear_owner = 0;
        self.hand_scroll = 0;
        self.hover_card = None;
        self.hover_rect = None;
    }
    fn resume_payment(&mut self) {
        let Some(saved) = self.suspended_payments.last().cloned() else {
            return;
        };
        if saved.decision != self.decision_key() {
            self.notify("Finish the current resource conversion first.");
            return;
        }
        let action = if saved.owns_plan {
            Action::CompletePayment
        } else {
            saved.draft.action.clone()
        };
        let quote = match self.game.quote_power_payment(&action) {
            Ok(quote) => quote,
            Err(error) => {
                self.notify(error);
                return;
            }
        };
        self.suspended_payments.pop();
        self.restore_ui_draft(saved.ui);
        if saved.owns_plan {
            self.selection = None;
        }
        self.payment_draft = Some(PaymentDraft {
            quote,
            action,
            just_opened: true,
            ..saved.draft
        });
        self.hover_card = None;
        self.hover_rect = None;
    }
    pub(super) fn suspended_payment_banner(&mut self) -> Result<(), String> {
        let Some(saved) = self.suspended_payments.last().cloned() else {
            return Ok(());
        };
        if self.payment_draft.is_some() || self.screen != Screen::Practice {
            return Ok(());
        }
        let ready = saved.decision == self.decision_key();
        self.ui.rect(Rect::new(244.0, 86.0, 886.0, 40.0), PANEL);
        self.ui.border(Rect::new(244.0, 86.0, 886.0, 40.0), GOLD);
        self.ui.ellipsis(
            &format!(
                "Payment saved · {}",
                self.payment_action_name(if saved.owns_plan {
                    &Action::CompletePayment
                } else {
                    &saved.draft.action
                })
            ),
            256.0,
            111.0,
            526.0,
            11.0,
            GOLD,
        );
        if self.ui.button(
            Rect::new(800.0, 92.0, 192.0, 28.0),
            "Resume payment",
            true,
            ready,
        ) {
            self.resume_payment();
        }
        if self.ui.button(
            Rect::new(1002.0, 92.0, 116.0, 28.0),
            if saved.owns_plan {
                "Undo attempt"
            } else {
                "Cancel draft"
            },
            false,
            ready && (!saved.owns_plan || self.game.can_cancel_payment()?),
        ) {
            if saved.owns_plan {
                self.apply_action(Action::Choose { accept: false });
            } else {
                self.suspended_payments.pop();
                self.restore_ui_draft(saved.ui);
            }
        }
        Ok(())
    }
}

pub(super) fn cost_label(rule: &ActivatedRule) -> String {
    let mut costs = Vec::new();
    if rule.xp_cost > 0 {
        costs.push(format!("{} XP", rule.xp_cost));
    }
    if rule.disempower_source {
        costs.push("Disempower source".to_owned());
    }
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
    if rule.recycle_count > 0 {
        costs.push(format!(
            "recycle {} {} from trash",
            rule.recycle_count,
            if rule.recycle_unit_only {
                "unit"
            } else {
                "cards"
            }
        ));
    }
    if rule.exhaust {
        costs.push("exhaust source".to_owned());
    }
    if rule.kill_source {
        costs.push("kill source".into());
    }
    if costs.is_empty() {
        "No resource cost".into()
    } else {
        costs.join(" · ")
    }
}
pub(super) fn level_label(game: &Game, player: usize, rule: &ActivatedRule) -> Option<String> {
    rule.minimum_xp.map(|threshold| {
        format!(
            "Level {threshold} · {} at XP {}",
            if game.players[player].xp >= u64::from(threshold) {
                "active"
            } else {
                "inactive"
            },
            game.players[player].xp
        )
    })
}
pub(super) fn output_label(rule: &ActivatedRule) -> Option<String> {
    rule.fixed_resource_output().map(|output| {
        let mut parts = Vec::new();
        if output.energy > 0 {
            parts.push(format!("{} energy", output.energy));
        }
        if output.power > 0 {
            parts.push(format!(
                "{} {} power",
                output.power,
                output.domain.map_or("Any", |domain| domain.name())
            ));
        }
        format!("Add {} immediately", parts.join(" + "))
    })
}
pub(super) fn fixed_payment_label(plan: &local_rift::payment_plan::PaymentPlanView) -> String {
    let mut costs = Vec::new();
    if plan.energy > 0 {
        costs.push(format!("{} energy", plan.energy));
    }
    costs.extend(plan.power.iter().map(|cost| {
        let domain = if cost.domains.len() == 6 {
            "Any".into()
        } else {
            cost.domains
                .iter()
                .map(|domain| domain.name())
                .collect::<Vec<_>>()
                .join("/")
        };
        format!("{} {domain} power", cost.amount)
    }));
    if plan.xp > 0 {
        costs.push(format!("{} XP", plan.xp));
    }
    format!(
        "Fixed cost: {}.",
        if costs.is_empty() {
            "none".into()
        } else {
            costs.join(" + ")
        }
    )
}
pub(super) fn source_state(game: &Game, source: AbilitySource) -> String {
    let (id, ready, attached) = match source {
        AbilitySource::Unit { player, unit_id } => {
            let Some(unit) = game.players[player]
                .units
                .iter()
                .find(|unit| unit.id == unit_id)
            else {
                return "Source left play".into();
            };
            (unit.id, unit.ready, false)
        }
        AbilitySource::Gear { player, gear_id } => {
            let Some(gear) = game.players[player]
                .gear
                .iter()
                .find(|gear| gear.id == gear_id)
            else {
                return "Source left play".into();
            };
            (gear.id, gear.ready, gear.attached_to.is_some())
        }
    };
    format!(
        "{} · {}{}",
        if ready { "Ready" } else { "Exhausted" },
        if game.unit_empowered(id) {
            "Empowered"
        } else {
            "Not Empowered"
        },
        if attached { " · attached" } else { "" }
    )
}

fn effective_cost_label(rule: &ActivatedRule, plan: &local_rift::play::CostPlanView) -> String {
    let mut parts = vec![play_ui::total_label(plan)];
    if rule.disempower_source {
        parts.push("Disempower source".into());
    }
    if rule.recycle_count > 0 {
        parts.push(format!(
            "recycle {} {} from trash",
            rule.recycle_count,
            if rule.recycle_unit_only {
                "units"
            } else {
                "cards"
            }
        ));
    }
    if rule.exhaust {
        parts.push("exhaust source".into());
    }
    if rule.kill_source {
        parts.push("kill source".into());
    }
    parts.join(" · ")
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
        self.remember_resource_draft();
        self.selection = Some(Selection::Ability(source, None));
        self.ability_targets.clear();
        self.ability_costs.clear();
        self.group.clear();
        self.drag = None;
        self.hover_card = None;
        self.hover_rect = None;
    }
    pub(super) fn ability_target_candidate(&self, target: Target) -> bool {
        if self.selection == Some(Selection::Legend) && self.targeting_selection() {
            return self.game.legend_target_valid(0, target);
        }
        matches!(self.selection, Some(Selection::Ability(source,Some(index)))
            if source.player()==0 && self.game.ability_targets(source,index).contains(&target))
    }
    pub(super) fn ability_cost_tray(&mut self) -> bool {
        let Some(Selection::Ability(source, Some(index))) = self.selection else {
            return false;
        };
        let Some((_, rule)) = self.ability_rule(source, index) else {
            return false;
        };
        if rule.recycle_count == 0 || self.tray != Tray::Trash || source.player() != 0 {
            return false;
        }
        let candidates = self.game.ability_cost_candidates(source, index);
        let cards = self.game.players[0].discard.clone();
        self.ui.text(
            &format!(
                "COST · {} / {} to recycle",
                self.ability_costs.len(),
                rule.recycle_count
            ),
            780.0,
            671.0,
            10.0,
            GOLD,
        );
        self.hand_scroll = self.hand_scroll.min(cards.len().saturating_sub(6));
        for (slot, (index, id)) in cards
            .iter()
            .enumerate()
            .skip(self.hand_scroll)
            .take(6)
            .enumerate()
        {
            let selected = self.ability_costs.contains(&index);
            let valid = candidates.contains(&index);
            let r = self.tile(*id, slot, self.game.catalog[*id].name, selected);
            self.ui.pill(
                r.x + 8.0,
                r.y + 8.0,
                if selected {
                    "PAY COST"
                } else if valid {
                    "COST OPTION"
                } else {
                    "INELIGIBLE"
                },
                if valid { GOLD } else { MUTED },
            );
            if valid && self.ui.clicked(r) {
                if let Some(position) = self.ability_costs.iter().position(|&i| i == index) {
                    self.ability_costs.remove(position);
                } else if self.ability_costs.len() < rule.recycle_count as usize {
                    self.ability_costs.push(index);
                } else {
                    self.notify("Deselect a cost card before choosing another.");
                }
            }
            if self.ui.hover(r) {
                self.hover_candidate = Some(*id);
            }
            if self.ui.hover(r) && self.ui.secondary_pressed() {
                self.inspect = Some(*id);
            }
        }
        self.tray_scroll(cards.len(), 6);
        if cards.is_empty() {
            self.ui.title(
                "No cards in your trash to pay this cost.",
                270.0,
                757.0,
                22.0,
                MUTED,
            );
        }
        true
    }
    pub(super) fn locked_payment_panel(&mut self) -> Result<(), String> {
        let Some(plan) = self.game.payment_plan_view(0) else {
            return Ok(());
        };
        self.ui.title("Finish payment", 1178.0, 366.0, 22.0, TEXT);
        let name = self.payment_action_name(&Action::CompletePayment);
        self.ui
            .wrapped_lines(&name, 1178.0, 393.0, 212.0, 11.0, GOLD, 2);
        self.ui.wrapped_lines(
            &fixed_payment_label(&plan),
            1178.0,
            430.0,
            212.0,
            11.0,
            TEXT,
            3,
        );
        if self.ui.button(
            Rect::new(1178.0, 493.0, 212.0, 34.0),
            "Pay & finish action",
            true,
            true,
        ) {
            self.action(Action::CompletePayment);
        }
        if self.ui.button(
            Rect::new(1178.0, 534.0, 212.0, 27.0),
            "Undo unsuccessful attempt",
            false,
            self.game.can_cancel_payment()?,
        ) {
            self.action(Action::Choose { accept: false });
        }
        self.ui.wrapped_lines("Board / Loadout: use Add abilities. Costs stay fixed. Undo restores resource steps only if this attempt cannot complete.",1178.0,580.0,212.0,10.0,MUTED,4);
        Ok(())
    }
    pub(super) fn commit_conversion_editor(&mut self) -> Result<(), String> {
        let Some(value) = self.conversion_editor.as_ref() else {
            return Ok(());
        };
        let amount = value
            .parse::<u16>()
            .map_err(|_| "Enter a whole amount from 0 to the available resources.".to_owned())?;
        if amount > self.game.conversion_available().unwrap_or(0) {
            return Err("That amount exceeds the resources currently available.".into());
        }
        self.conversion_amount = amount;
        self.conversion_editor = None;
        Ok(())
    }
    pub(super) fn conversion_panel(&mut self, card_id: usize, direction: ConversionDirection) {
        let key = self.decision_key();
        if self.conversion_key.as_ref() != Some(&key) {
            self.conversion_key = Some(key);
            self.conversion_amount = 0;
            self.conversion_editor = None;
        }
        let available = self.game.conversion_available().unwrap_or(0);
        let (pay, receive) = match direction {
            ConversionDirection::EnergyToPower => ("energy", "Any power"),
            ConversionDirection::PowerToEnergy => ("power", "energy"),
        };
        self.ui.ellipsis(
            self.game.catalog[card_id].name,
            1178.0,
            366.0,
            212.0,
            20.0,
            TEXT,
        );
        self.ui
            .text("CONVERSION · SOURCE EXHAUSTED", 1178.0, 389.0, 9.0, GOLD);
        self.ui.wrapped_lines(
            &format!("Pay {pay}; gain the same amount of {receive}. {available} available now."),
            1178.0,
            410.0,
            212.0,
            11.0,
            TEXT,
            2,
        );
        let step = if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
            10
        } else {
            1
        };
        if self.ui.button(
            Rect::new(1178.0, 444.0, 34.0, 30.0),
            "-",
            false,
            self.conversion_amount > 0,
        ) {
            self.conversion_amount = self.conversion_amount.saturating_sub(step);
        }
        let amount_rect = Rect::new(1217.0, 444.0, 56.0, 30.0);
        self.ui.border(
            amount_rect,
            if self.conversion_editor.is_some() {
                GOLD
            } else {
                LINE
            },
        );
        self.ui.ellipsis(
            self.conversion_editor
                .as_deref()
                .unwrap_or(&self.conversion_amount.to_string()),
            1224.0,
            466.0,
            45.0,
            17.0,
            GOLD,
        );
        if self.ui.clicked(amount_rect) {
            self.conversion_editor = Some(String::new());
        }
        self.ui.text(
            if self.conversion_editor.is_some() {
                "Type amount · Enter confirms"
            } else {
                "Click number to type · Shift +/- = 10"
            },
            1178.0,
            487.0,
            9.0,
            MUTED,
        );
        if self.ui.button(
            Rect::new(1282.0, 444.0, 34.0, 30.0),
            "+",
            false,
            self.conversion_amount < available,
        ) {
            self.conversion_amount = self.conversion_amount.saturating_add(step).min(available);
        }
        if self
            .ui
            .button(Rect::new(1325.0, 444.0, 65.0, 30.0), "Max", false, true)
        {
            self.conversion_amount = available;
        }
        if self.ui.button(
            Rect::new(1178.0, 493.0, 212.0, 34.0),
            "Pay amount & convert",
            true,
            true,
        ) {
            if let Err(error) = self.commit_conversion_editor() {
                self.notify(error);
            } else {
                self.action(Action::ResolveConversion {
                    amount: self.conversion_amount,
                });
            }
        }
        if self.ui.button(
            Rect::new(1178.0, 534.0, 212.0, 27.0),
            "Convert zero · stay exhausted",
            false,
            true,
        ) {
            self.action(Action::ResolveConversion { amount: 0 });
        }
        self.ui.wrapped_lines(
            "Board / Loadout: use other Add abilities while this conversion waits.",
            1178.0,
            581.0,
            212.0,
            10.0,
            MUTED,
            3,
        );
        let waiting = self
            .game
            .resource_continuations()
            .iter()
            .map(|entry| match entry.task {
                "ResourceConversion" => "resource conversion",
                "CounterTax" => "counter payment",
                "PendingPlay" | "LastRites" | "ReplaySpell" | "RevivePlay" => "pending card",
                "TriggerTargets" | "OnPlayTarget" => "ability targets",
                "Legend" => "legend choice",
                "Battlefield" => "battlefield choice",
                _ => "original payment",
            })
            .collect::<Vec<_>>()
            .join(" → ");
        if !waiting.is_empty() {
            self.ui
                .ellipsis(&format!("Then: {waiting}"), 1178.0, 629.0, 212.0, 9.0, GOLD);
        }
    }
    pub(super) fn ability_panel(
        &mut self,
        source: AbilitySource,
        index: Option<usize>,
    ) -> Result<(), String> {
        self.ui.ellipsis(
            &source_state(&self.game, source),
            1178.0,
            343.0,
            212.0,
            9.0,
            GOLD,
        );
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
            return Ok(());
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
                return Ok(());
            };
            self.ui
                .wrapped_lines(rule.label, 1178.0, 385.0, 212.0, 11.0, GOLD, 2);
            let action = if rule.recycle_count > 0 {
                Action::ActivateAbilityWithCosts {
                    source,
                    ability_index: index,
                    targets: self.ability_targets.clone(),
                    trash_indices: self.ability_costs.clone(),
                }
            } else {
                Action::ActivateAbility {
                    source,
                    ability_index: index,
                    targets: self.ability_targets.clone(),
                }
            };
            let cost_plan = super::optional_query(self.game.cost_plan(&action))?;
            let mut deflect = 0_u16;
            for target in &self.ability_targets {
                if let Target::Unit { player, unit_id } = target
                    && *player != source.player()
                    && let Some(unit) = self.game.players[*player]
                        .units
                        .iter()
                        .find(|unit| unit.id == *unit_id)
                {
                    deflect = deflect
                        .checked_add(u16::from(self.game.unit_keywords(unit)?.deflect))
                        .ok_or_else(|| {
                            "engine_capacity: Deflect total exceeds supported power.".to_owned()
                        })?;
                }
            }
            let cost = if let Some(plan) = &cost_plan {
                effective_cost_label(rule, plan)
            } else if deflect > 0 {
                format!("{} · +{deflect} Any power (Deflect)", cost_label(rule))
            } else {
                cost_label(rule)
            };
            self.ui.wrapped_lines(
                &cost,
                1178.0,
                418.0,
                212.0,
                if cost.len() < 40 { 12.0 } else { 10.0 },
                TEXT,
                2,
            );
            let timing = level_label(&self.game, source.player(), rule)
                .unwrap_or_else(|| timing_label(rule).into());
            self.ui.ellipsis(&timing, 1178.0, 449.0, 212.0, 9.0, MUTED);
            let status = if source.player() != 0 {
                "Opponent ability · read only".to_owned()
            } else if self.ability_costs.len() != rule.recycle_count as usize {
                format!(
                    "COST: select {} {} in your trash ({} selected).",
                    rule.recycle_count,
                    if rule.recycle_unit_only {
                        "unit"
                    } else {
                        "cards"
                    },
                    self.ability_costs.len()
                )
            } else if self.ability_targets.len() < rule.targets.min as usize {
                format!(
                    "Choose {} highlighted {}{}{}.",
                    rule.targets.min,
                    if rule.targets.kind == AbilityTargetKind::Gear {
                        "gear"
                    } else {
                        "unit"
                    },
                    if rule.targets.min == 1 { "" } else { "s" },
                    if rule.targets.kind == AbilityTargetKind::Gear {
                        " in either Loadout"
                    } else if rule.targets.battlefield {
                        " at a battlefield"
                    } else {
                        ""
                    }
                )
            } else {
                self.game
                    .validate_ability_with_costs(
                        source,
                        index,
                        &self.ability_targets,
                        &self.ability_costs,
                    )
                    .err()
                    .unwrap_or_else(|| {
                        if rule.only_if_not_empowered {
                            "Empowers this source on resolution. No target is chosen; opponents can respond.".into()
                        } else if rule.disempower_source {
                            "Disempower is paid before responses, together with the other costs.".into()
                        } else if rule.conversion_direction().is_some() {
                            "Exhaust first; choose the conversion amount next. Zero is allowed."
                                .into()
                        } else if let Some(output) = output_label(rule) {
                            format!("{output}. No response window.")
                        } else if rule.adds_resources() {
                            "Adds resources immediately; no response window.".into()
                        } else if rule.xp_cost > 0 {
                            "XP is spent before responses. Lower XP may switch off Level bonuses immediately.".into()
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
                                    Target::Gear { player, gear_id } => self.game.players[*player]
                                        .gear
                                        .iter()
                                        .find(|gear| gear.id == *gear_id)
                                        .map(|gear| {
                                            format!(
                                                "Target: {}. Empowers this gear, not its bearer.",
                                                self.game.gear_name(gear)
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
                .wrapped_lines(&status, 1178.0, 468.0, 212.0, 9.5, TEAL, 2);
            let valid = source.player() == 0
                && self
                    .game
                    .validate_ability_with_costs(
                        source,
                        index,
                        &self.ability_targets,
                        &self.ability_costs,
                    )
                    .is_ok();
            let begin_payment = source.player() == 0 && !valid && self.can_begin_payment(&action);
            if self.ui.button(
                Rect::new(1178.0, 493.0, 212.0, 34.0),
                if begin_payment {
                    "Begin payment · use resources"
                } else if rule.conversion_direction().is_some() {
                    "Exhaust · choose amount"
                } else {
                    "Pay & use ability"
                },
                true,
                valid || begin_payment,
            ) {
                if begin_payment {
                    self.begin_ui_payment(action.clone());
                } else {
                    self.action(action.clone());
                }
            }
            let cost_details = source.player() == 0
                && cost_plan
                    .as_ref()
                    .is_some_and(|plan| !plan.modifiers.is_empty());
            if self.ui.button(
                Rect::new(
                    1178.0,
                    534.0,
                    if cost_details { 133.0 } else { 212.0 },
                    27.0,
                ),
                if cost_details {
                    "Back · unspent"
                } else {
                    "Back · keep costs unspent"
                },
                false,
                true,
            ) {
                self.open_abilities(source);
            }
            if cost_details
                && self
                    .ui
                    .button(Rect::new(1319.0, 534.0, 71.0, 27.0), "Costs", false, true)
            {
                self.review_action_costs(action);
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
                    .ability_button(Rect::new(1178.0, y, 212.0, 32.0), rule.label)
                {
                    self.selection = Some(Selection::Ability(source, Some(*index)));
                    self.ability_targets.clear();
                    self.ability_costs.clear();
                    if source.player() == 0 && rule.recycle_count > 0 {
                        self.tray = Tray::Trash;
                        self.gear_owner = 0;
                        self.hand_scroll = 0;
                    } else if rule.targets.kind == AbilityTargetKind::Gear {
                        self.tray = Tray::Loadout;
                    }
                }
                let mut detail = cost_label(rule);
                if let Some(level) = level_label(&self.game, source.player(), rule) {
                    detail.push_str(&format!(" · {level}"));
                }
                self.ui
                    .wrapped_lines(&detail, 1178.0, y + 45.0, 212.0, 9.5, MUTED, 2);
            }
            if options.len() < 2
                && let Some(rule) = activated_rules(card_id).iter().find(|rule| {
                    rule.minimum_xp.is_some_and(|minimum| {
                        self.game.players[source.player()].xp < u64::from(minimum)
                    })
                })
            {
                let text = format!(
                    "{}: {}",
                    level_label(&self.game, source.player(), rule).unwrap(),
                    rule.label
                );
                self.ui
                    .wrapped_lines(&text, 1178.0, 478.0, 212.0, 10.0, MUTED, 3);
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn object_costs_are_distinct_from_conversion_instructions() {
        assert_eq!(
            cost_label(&activated_rules(240)[0]),
            "1 energy · recycle 3 cards from trash · exhaust source"
        );
        assert!(cost_label(&activated_rules(241)[0]).contains("recycle 1 unit from trash"));
        assert!(cost_label(&activated_rules(242)[0]).contains("kill source"));
        for id in [244, 245] {
            assert_eq!(cost_label(&activated_rules(id)[0]), "exhaust source");
        }
    }
    #[test]
    fn empower_costs_remain_distinct_from_effects_and_timing() {
        for id in [250, 251] {
            assert_eq!(cost_label(&activated_rules(id)[0]), "exhaust source");
            assert_eq!(
                cost_label(&activated_rules(id)[1]),
                "Disempower source · 1 energy · exhaust source"
            );
        }
        assert_eq!(cost_label(&activated_rules(253)[0]), "2 energy");
        assert_eq!(
            cost_label(&activated_rules(252)[0]),
            "1 energy · exhaust source"
        );
        assert_eq!(
            timing_label(&activated_rules(252)[0]),
            "Your open Main Phase"
        );
        assert_eq!(
            timing_label(&activated_rules(252)[1]),
            "Reaction · any priority window"
        );
        assert_eq!(
            cost_label(&activated_rules(246)[0]),
            "1 energy · 1 Calm power · exhaust source · kill source"
        );
    }

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
