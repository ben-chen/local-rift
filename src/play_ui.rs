//! Native drafts for independently chosen spell executions and cost reductions.
use super::*;
use local_rift::{game::ChainSource, play::*};

#[derive(Clone)]
pub(super) struct PlayDraft {
    pub key: (Selection, usize),
    pub choices: PlayChoices,
    pub execution: usize,
    pub open: bool,
    pub scroll: usize,
}
#[derive(Clone)]
pub(super) struct CostEditor {
    action: Action,
    choices: PlayChoices,
    commit: bool,
    scroll: usize,
    allocation: Option<u64>,
    last_plan: Option<CostPlanView>,
    component_scroll: usize,
    allocation_scroll: usize,
}

pub(super) fn source_label(game: &Game, source: &ChainSource) -> String {
    match *source {
        ChainSource::Card { card_id } => game.catalog[card_id].name.into(),
        ChainSource::Unit { player, unit_id } => game
            .players
            .get(player)
            .and_then(|state| state.units.iter().find(|unit| unit.id == unit_id))
            .map(|unit| game.unit_name(unit))
            .unwrap_or_else(|| "Unit that left play".into()),
        ChainSource::Gear { player, gear_id } => game
            .players
            .get(player)
            .and_then(|state| state.gear.iter().find(|gear| gear.id == gear_id))
            .map(|gear| game.gear_name(gear).to_owned())
            .unwrap_or_else(|| "Gear that left play".into()),
        ChainSource::Legend { legend_id, .. } => local_rift::legend_catalog::legend(legend_id)
            .map_or("Legend", |legend| legend.name)
            .into(),
        ChainSource::Battlefield { battlefield_id, .. } => {
            local_rift::battlefields::battlefield(battlefield_id)
                .name
                .into()
        }
    }
}
pub(super) fn token_placement_description(
    game: &Game,
    token: &local_rift::game::PendingToken,
) -> String {
    let spec = local_rift::rules::token_spec(token.card_id);
    let name = spec.map_or("Unit", |spec| spec.name);
    let location = match token.location {
        Some(Some(lane)) => format!(
            "Fixed at {}",
            local_rift::battlefields::battlefield(game.battlefields[lane]).name
        ),
        Some(None) => "Fixed at your base".into(),
        None => "Choose base or a field you control".into(),
    };
    format!(
        "{} Might {name} · {}{}. {location}.",
        token.might,
        if token.entry_ready {
            "enters ready"
        } else {
            "enters exhausted"
        },
        if spec.is_some_and(|spec| spec.temporary) {
            " · Temporary"
        } else {
            ""
        }
    )
}
fn value_label(value: &CostValue) -> String {
    let domains = if value.domains.is_empty() || value.domains.len() == 6 {
        "Any".into()
    } else {
        value
            .domains
            .iter()
            .map(|domain| domain.name())
            .collect::<Vec<_>>()
            .join(" / ")
    };
    let mut parts = Vec::new();
    if value.energy > 0 {
        parts.push(format!("{} energy", value.energy));
    }
    if value.power > 0 {
        parts.push(format!("{} {domains} power", value.power));
    }
    if value.xp > 0 {
        parts.push(format!("{} XP", value.xp));
    }
    if parts.is_empty() {
        "No added cost".into()
    } else {
        parts.join(" + ")
    }
}
pub(super) fn total_label(plan: &CostPlanView) -> String {
    let mut parts = Vec::new();
    if plan.energy > 0 || plan.xp == 0 {
        parts.push(format!("{} energy", plan.energy));
    }
    for cost in &plan.power {
        let domains = if cost.domains.is_empty() || cost.domains.len() == 6 {
            "Any".into()
        } else {
            cost.domains
                .iter()
                .map(|domain| domain.name())
                .collect::<Vec<_>>()
                .join(" / ")
        };
        parts.push(format!("{} {domains} power", cost.amount));
    }
    if plan.xp > 0 {
        parts.push(format!("{} XP", plan.xp));
    }
    format!("Pay {}", parts.join(" + "))
}
fn strip_options(action: Action) -> Action {
    match action {
        Action::PlayCardOptions {
            hand_index,
            lane,
            targets,
            ..
        } => Action::PlayCardOptions {
            hand_index,
            lane,
            targets,
            repeat_targets: vec![],
            repeat: false,
            accelerate: false,
            bonus: false,
        },
        Action::PlayFromTrashOptions {
            trash_index,
            lane,
            flow,
            targets,
            ..
        } => Action::PlayFromTrashOptions {
            trash_index,
            lane,
            flow,
            targets,
            repeat_targets: vec![],
            repeat: false,
            accelerate: false,
            bonus: false,
        },
        Action::FinalizePendingPlay { lane, targets, .. } => Action::FinalizePendingPlay {
            lane,
            targets,
            repeat_targets: vec![],
            repeat: false,
            accelerate: false,
            bonus: false,
        },
        Action::PlayChampionOptions { lane, target, .. } => Action::PlayChampionOptions {
            lane,
            target,
            accelerate: false,
        },
        Action::FinalizeTokenPlay { lane, .. } => Action::FinalizeTokenPlay {
            lane,
            accelerate: false,
        },
        other => other,
    }
}
pub(super) fn core_action(action: &Action) -> &Action {
    match action {
        Action::WithPowerPayment { action, .. } | Action::WithPlayChoices { action, .. } => {
            core_action(action)
        }
        other => other,
    }
}
/// Change one execution in a cloned query draft, never the player's selection.
pub(super) fn replace_execution_targets(
    action: &mut Action,
    execution: usize,
    targets: Vec<Target>,
) -> Result<(), String> {
    match action {
        Action::WithPlayChoices { action, choices } => {
            if execution == 0 {
                replace_execution_targets(action, 0, targets)
            } else {
                let repeat = choices
                    .repeats
                    .get_mut(execution - 1)
                    .ok_or("This Repeat execution is no longer selected.")?;
                repeat.choices.targets = targets;
                Ok(())
            }
        }
        Action::WithPowerPayment { action, .. } | Action::BeginPayment { action } => {
            replace_execution_targets(action, execution, targets)
        }
        Action::PlayCardOptions {
            targets: original,
            repeat_targets,
            ..
        }
        | Action::PlayFromTrashOptions {
            targets: original,
            repeat_targets,
            ..
        }
        | Action::FinalizePendingPlay {
            targets: original,
            repeat_targets,
            ..
        } if execution <= 1 => {
            *if execution == 0 {
                original
            } else {
                repeat_targets
            } = targets;
            Ok(())
        }
        Action::PlayCardWithTargets {
            targets: original, ..
        }
        | Action::PlayFromTrashWithTargets {
            targets: original, ..
        }
        | Action::PlayHidden {
            targets: original, ..
        } if execution == 0 => {
            *original = targets;
            Ok(())
        }
        Action::PlayChampionOptions { target, .. } if execution == 0 && targets.len() <= 1 => {
            *target = targets.first().copied();
            Ok(())
        }
        _ => Err("This action has no editable target execution.".into()),
    }
}

pub(super) fn target_view_hint(game: &Game, view: &PlayTargetView) -> String {
    let Some(slot) = &view.next_slot else {
        return if view.spec.maximum == 0 {
            "This execution needs no targets.".into()
        } else {
            format!(
                "{} · Click a chosen target to edit its slot.",
                view.targets
                    .iter()
                    .map(|target| target_label(game, *target))
                    .collect::<Vec<_>>()
                    .join(" · ")
            )
        };
    };
    let noun = match view.spec.kind {
        Some(local_rift::rules::PlayTargetKind::UnitOrGear)
            if matches!(
                local_rift::rules::effect(view.card_id),
                local_rift::rules::OfficialEffect::KillEnemyDomain { .. }
            ) =>
        {
            "an enemy Chaos unit (base or field), or gear in Loadout"
        }
        Some(local_rift::rules::PlayTargetKind::UnitOrGear) if view.card_id == 379 => {
            "a unit with 3 Might or less (base or field), or gear in Loadout"
        }
        Some(local_rift::rules::PlayTargetKind::Unit) if view.card_id == 383 && slot.index == 1 => {
            "an enemy unit at a battlefield for 3 damage"
        }
        Some(local_rift::rules::PlayTargetKind::UnitOrGear) => {
            "a battlefield unit, or gear in Loadout"
        }
        Some(local_rift::rules::PlayTargetKind::Gear) => "gear in either player's Loadout",
        Some(local_rift::rules::PlayTargetKind::Battlefield) => "either battlefield, even if empty",
        Some(local_rift::rules::PlayTargetKind::Spell) => "a pending spell above",
        _ => match local_rift::rules::targeting(view.card_id).allegiance {
            local_rift::rules::Allegiance::Friendly => "a highlighted friendly unit",
            local_rift::rules::Allegiance::Enemy => "a highlighted enemy unit",
            local_rift::rules::Allegiance::Mixed if slot.index == 0 => {
                "a highlighted friendly unit"
            }
            local_rift::rules::Allegiance::Mixed => "a highlighted enemy unit",
            _ => "a highlighted unit",
        },
    };
    let mut text = format!("{} · Choose {noun}", slot.index + 1);
    if let Some(restriction) = slot.hidden {
        let (lane, relation) = match restriction {
            HiddenTargetRestriction::AtBattlefield { lane } => (lane, "at"),
            HiddenTargetRestriction::OutsideBattlefield { lane } => (lane, "outside"),
        };
        let name = local_rift::battlefields::battlefield(game.battlefields[lane]).name;
        text.push_str(&format!(" {relation} field {} · {name}", lane + 1));
    }
    if let Some(prior) = slot.different_location_from {
        text.push_str(&format!(
            " at a different location from target {}",
            prior + 1
        ));
    }
    text
}
fn has_play_choices(action: &Action) -> bool {
    matches!(
        action,
        Action::WithPlayChoices { .. }
            | Action::WithPowerPayment { .. }
            | Action::BeginPayment { .. }
            | Action::CompletePayment
    )
}
impl App {
    pub(super) fn selected_target_query(
        &self,
        replace_single: bool,
    ) -> Result<Option<(Action, usize)>, String> {
        let Some(action) = self.selected_play_action() else {
            return Ok(None);
        };
        let mut action = self.canonical_action(action);
        let execution = self
            .play_draft
            .as_ref()
            .map_or(usize::from(self.editing_repeat), |draft| draft.execution);
        if replace_single
            && self
                .selected_card_id()
                .is_some_and(|id| local_rift::rules::play_target_spec(id).maximum == 1)
        {
            replace_execution_targets(&mut action, execution, vec![])?;
        }
        Ok(Some((action, execution)))
    }
    pub(super) fn sync_token_draft(&mut self) {
        let id = self.game.current_pending_token().map(|token| token.id);
        if self.token_draft_id != id {
            if id.is_some() {
                self.options = PlayOptions::default();
                self.destination = None;
            }
            self.token_draft_id = id;
        }
    }
    pub(super) fn review_action_costs(&mut self, action: Action) {
        self.cost_editor = Some(CostEditor {
            action,
            choices: PlayChoices::default(),
            commit: true,
            scroll: 0,
            allocation: None,
            last_plan: None,
            component_scroll: 0,
            allocation_scroll: 0,
        });
        self.hover_card = None;
        self.hover_rect = None;
    }
    pub(super) fn selected_play_action(&self) -> Option<Action> {
        let lane = self.destination;
        let targets = self.targets.clone();
        let repeat_targets = self.repeat_targets.clone();
        let PlayOptions {
            repeat,
            accelerate,
            bonus,
        } = self.options;
        Some(match self.selection? {
            Selection::Hand(hand_index) => Action::PlayCardOptions {
                hand_index,
                lane,
                targets,
                repeat_targets,
                repeat,
                accelerate,
                bonus,
            },
            Selection::Trash(trash_index) => Action::PlayFromTrashOptions {
                trash_index,
                flow: self.flow_source,
                lane,
                targets,
                repeat_targets,
                repeat,
                accelerate,
                bonus,
            },
            Selection::Pending(_) => Action::FinalizePendingPlay {
                lane,
                targets,
                repeat_targets,
                repeat,
                accelerate,
                bonus,
            },
            Selection::Champion => Action::PlayChampionOptions {
                lane,
                target: targets.first().copied(),
                accelerate,
            },
            Selection::Hidden(hidden_id) => Action::PlayHidden { hidden_id, targets },
            _ => return None,
        })
    }
    fn extended_choices(&self, view: &PlayChoiceView) -> bool {
        view.repeats.len() > usize::from(local_rift::rules::repeat_cost(view.card_id).is_some())
            || view.repeats.iter().any(|option| {
                option.source
                    != (ChainSource::Card {
                        card_id: view.card_id,
                    })
            })
            || (view.accelerate.is_some()
                && local_rift::rules::accelerate_cost(view.card_id).is_none())
            || (view.bonus.is_some() && view.card_id != 29)
            || view
                .costs
                .modifiers
                .iter()
                .any(|modifier| !matches!(modifier.source, ChainSource::Card { card_id: 35 }))
    }
    pub(super) fn sync_play_draft(&mut self) -> Result<(), String> {
        let key = self.selection.zip(self.selected_card_id());
        if self
            .play_draft
            .as_ref()
            .is_some_and(|draft| Some(draft.key) != key)
        {
            self.play_draft = None;
        }
        if self.play_draft.is_none()
            && let (Some(key), Some(action)) = (key, self.selected_play_action())
            && let Some(view) = super::optional_query(self.game.play_choices(&action))?
            && self.extended_choices(&view)
        {
            self.start_play_draft(key, &view);
        }
        Ok(())
    }
    fn start_play_draft(&mut self, key: (Selection, usize), view: &PlayChoiceView) {
        let mut choices = PlayChoices {
            accelerate: self.options.accelerate,
            bonus: self.options.bonus,
            ..Default::default()
        };
        if self.options.repeat
            && let Some(option) = view
                .repeats
                .iter()
                .find(|option| option.source == (ChainSource::Card { card_id: key.1 }))
        {
            choices.repeats.push(RepeatSelection {
                option_id: option.id,
                choices: ExecutionChoices {
                    targets: self.repeat_targets.clone(),
                    mode: None,
                },
            });
        }
        self.play_draft = Some(PlayDraft {
            key,
            choices,
            execution: 0,
            open: false,
            scroll: 0,
        });
        self.options = PlayOptions::default();
        self.editing_repeat = false;
    }
    pub(super) fn canonical_action(&self, action: Action) -> Action {
        if has_play_choices(&action) {
            return action;
        }
        if let Some(draft) = &self.play_draft
            && self.selection == Some(draft.key.0)
            && self.selected_card_id() == Some(draft.key.1)
            && matches!(
                action,
                Action::PlayCardOptions { .. }
                    | Action::PlayFromTrashOptions { .. }
                    | Action::FinalizePendingPlay { .. }
                    | Action::PlayChampionOptions { .. }
                    | Action::PlayHidden { .. }
            )
        {
            return Action::WithPlayChoices {
                action: Box::new(strip_options(action)),
                choices: draft.choices.clone(),
            };
        }
        action
    }
    pub(super) fn current_targets(&self) -> &[Target] {
        if let Some(draft) = &self.play_draft
            && draft.execution > 0
            && let Some(repeat) = draft.choices.repeats.get(draft.execution - 1)
        {
            return &repeat.choices.targets;
        }
        if self.editing_repeat {
            &self.repeat_targets
        } else {
            &self.targets
        }
    }
    pub(super) fn current_targets_mut(&mut self) -> &mut Vec<Target> {
        if let Some(draft) = &mut self.play_draft {
            draft.choices.cost_order.clear();
        }
        if let Some(draft) = &mut self.play_draft
            && draft.execution > 0
            && let Some(repeat) = draft.choices.repeats.get_mut(draft.execution - 1)
        {
            return &mut repeat.choices.targets;
        }
        if self.editing_repeat {
            &mut self.repeat_targets
        } else {
            &mut self.targets
        }
    }
    pub(super) fn play_choices_button(&mut self) -> Result<bool, String> {
        self.sync_play_draft()?;
        if let Some(draft) = &mut self.play_draft {
            let count = draft.choices.repeats.len();
            if self.ui.button(
                Rect::new(1178.0, 454.0, 212.0, 28.0),
                &if count == 0 {
                    "Play choices & cost".into()
                } else {
                    format!(
                        "Play choices · {count} Repeat{}",
                        if count == 1 { "" } else { "s" }
                    )
                },
                draft.open,
                true,
            ) {
                draft.open = !draft.open;
                self.hover_card = None;
                self.hover_rect = None;
            }
            return Ok(true);
        }
        Ok(false)
    }
    pub(super) fn draw_play_choices_tray(&mut self) -> Result<bool, String> {
        let Some(mut draft) = self.play_draft.clone().filter(|draft| draft.open) else {
            return Ok(false);
        };
        let Some(action) = self.selected_play_action() else {
            return Ok(false);
        };
        let action = Action::WithPlayChoices {
            action: Box::new(strip_options(action)),
            choices: draft.choices.clone(),
        };
        let Some(view) = super::optional_query(self.game.play_choices(&action))? else {
            return Ok(false);
        };
        self.ui.rect(Rect::new(245.0, 687.0, 883.0, 202.0), PANEL);
        self.ui.border(Rect::new(245.0, 687.0, 883.0, 202.0), LINE);
        let list = Rect::new(255.0, 716.0, 425.0, 155.0);
        if self.ui.hover(list) {
            let wheel = self.ui.wheel();
            if wheel < 0.0 {
                draft.scroll = draft
                    .scroll
                    .saturating_add(1)
                    .min(view.repeats.len().saturating_sub(3));
            }
            if wheel > 0.0 {
                draft.scroll = draft.scroll.saturating_sub(1);
            }
        }
        self.ui.text(
            "OPTIONAL REPEATS · select independently",
            260.0,
            708.0,
            10.0,
            GOLD,
        );
        for (slot, option) in view.repeats.iter().skip(draft.scroll).take(3).enumerate() {
            let y = 720.0 + slot as f32 * 50.0;
            let position = draft
                .choices
                .repeats
                .iter()
                .position(|selected| selected.option_id == option.id);
            let rect = Rect::new(258.0, y, 412.0, 44.0);
            self.ui.rect(
                rect,
                if position.is_some() {
                    Color::new(0.08, 0.20, 0.20, 1.0)
                } else {
                    BG
                },
            );
            self.ui
                .border(rect, if position.is_some() { TEAL } else { LINE });
            self.ui.ellipsis(
                &format!(
                    "{} {}",
                    if position.is_some() { "+" } else { "○" },
                    if option.source
                        == (ChainSource::Card {
                            card_id: view.card_id
                        })
                    {
                        option.label.clone()
                    } else {
                        format!("{} · Repeat", source_label(&self.game, &option.source))
                    }
                ),
                268.0,
                y + 17.0,
                388.0,
                11.5,
                TEXT,
            );
            self.ui.ellipsis(
                &value_label(&option.cost),
                268.0,
                y + 35.0,
                388.0,
                10.0,
                TEAL,
            );
            if self.ui.clicked(rect) {
                if let Some(index) = position {
                    draft.choices.cost_order.clear();
                    draft.choices.repeats.remove(index);
                    draft.execution = 0;
                } else {
                    draft.choices.cost_order.clear();
                    draft.choices.repeats.push(RepeatSelection {
                        option_id: option.id,
                        choices: ExecutionChoices::default(),
                    });
                    draft.execution = draft.choices.repeats.len();
                }
            }
        }
        if view.repeats.is_empty() {
            self.ui
                .text("This play has no Repeat costs.", 260.0, 746.0, 12.0, MUTED);
        }
        self.ui
            .text("EDIT TARGETS FOR EACH EXECUTION", 695.0, 708.0, 10.0, GOLD);
        if self.ui.button(
            Rect::new(980.0, 690.0, 128.0, 22.0),
            "Review costs",
            false,
            true,
        ) {
            self.cost_editor = Some(CostEditor {
                action: strip_options(self.selected_play_action().unwrap()),
                choices: draft.choices.clone(),
                commit: false,
                scroll: 0,
                allocation: None,
                last_plan: None,
                component_scroll: 0,
                allocation_scroll: 0,
            });
        }
        let count = draft.choices.repeats.len() + 1;
        let first = (draft.execution / 3) * 3;
        for execution in first..count.min(first + 3) {
            let mut label = if execution == 0 {
                "Original".into()
            } else {
                format!("Repeat {execution}")
            };
            if local_rift::rules::play_target_spec(view.card_id).kind
                == Some(local_rift::rules::PlayTargetKind::Battlefield)
            {
                let targets = if execution == 0 {
                    &self.targets
                } else {
                    &draft.choices.repeats[execution - 1].choices.targets
                };
                label.push_str(&match targets.first() {
                    Some(Target::Battlefield { lane }) => format!(" · F{}", lane + 1),
                    _ => " · ?".into(),
                });
            }
            if self.ui.button(
                Rect::new(
                    695.0 + (execution - first) as f32 * 104.0,
                    719.0,
                    98.0,
                    27.0,
                ),
                &label,
                draft.execution == execution,
                true,
            ) {
                draft.execution = execution;
            }
        }
        if self
            .ui
            .button(Rect::new(1010.0, 719.0, 43.0, 27.0), "<", false, first > 0)
        {
            draft.execution = first - 1;
        }
        if self.ui.button(
            Rect::new(1062.0, 719.0, 43.0, 27.0),
            ">",
            false,
            first + 3 < count,
        ) {
            draft.execution = first + 3;
        }
        let targets = if draft.execution == 0 {
            &self.targets
        } else {
            &draft.choices.repeats[draft.execution - 1].choices.targets
        };
        let hint = if targets.is_empty() {
            match local_rift::rules::play_target_spec(view.card_id).kind {
                Some(local_rift::rules::PlayTargetKind::Battlefield) => {
                    "Choose either battlefield on the mat, even if empty.".into()
                }
                Some(local_rift::rules::PlayTargetKind::Spell) => {
                    "Choose a pending spell above the board.".into()
                }
                Some(local_rift::rules::PlayTargetKind::Gear) => {
                    "Choose gear in either player's Loadout.".into()
                }
                Some(local_rift::rules::PlayTargetKind::UnitOrGear) => {
                    "Choose a unit at a battlefield or gear in either Loadout.".into()
                }
                Some(local_rift::rules::PlayTargetKind::Unit) => {
                    "Choose eligible units on the mat.".into()
                }
                None => "This execution needs no targets.".into(),
            }
        } else {
            targets
                .iter()
                .map(|target| target_label(&self.game, *target))
                .collect::<Vec<_>>()
                .join(" · ")
        };
        self.ui
            .wrapped_lines(&hint, 695.0, 766.0, 414.0, 11.0, TEXT, 2);
        if draft.execution > 0
            && self.ui.button(
                Rect::new(695.0, 802.0, 198.0, 25.0),
                "Copy original targets",
                false,
                true,
            )
        {
            draft.choices.cost_order.clear();
            draft.choices.repeats[draft.execution - 1].choices.targets = self.targets.clone();
        }
        if draft.execution > 1
            && self.ui.button(
                Rect::new(903.0, 802.0, 205.0, 25.0),
                "Move this Repeat earlier",
                false,
                true,
            )
        {
            draft
                .choices
                .repeats
                .swap(draft.execution - 1, draft.execution - 2);
            draft.execution -= 1;
        }
        let mut x = 695.0;
        if let Some(optional) = &view.accelerate {
            if self.ui.button(
                Rect::new(x, 836.0, 198.0, 29.0),
                "Accelerate · enter ready",
                draft.choices.accelerate,
                true,
            ) {
                draft.choices.cost_order.clear();
                draft.choices.accelerate = !draft.choices.accelerate;
            }
            if self.ui.hover(Rect::new(x, 836.0, 198.0, 29.0)) {
                self.ui
                    .text(&value_label(&optional.cost), x, 881.0, 9.0, TEAL);
            }
            x += 208.0;
        }
        if let Some(optional) = &view.bonus {
            if self.ui.button(
                Rect::new(x, 836.0, 198.0, 29.0),
                if view.card_id == 261 {
                    "Extra cost: stun on play"
                } else {
                    &optional.label
                },
                draft.choices.bonus,
                true,
            ) {
                draft.choices.cost_order.clear();
                draft.choices.bonus = !draft.choices.bonus;
            }
            if self.ui.hover(Rect::new(x, 836.0, 198.0, 29.0)) {
                self.ui
                    .text(&value_label(&optional.cost), x, 881.0, 9.0, TEAL);
            }
        }
        if view.accelerate.is_none()
            && view.bonus.is_none()
            && self.ui.button(
                Rect::new(695.0, 836.0, 413.0, 29.0),
                "Review cost reductions and order",
                false,
                true,
            )
        {
            self.cost_editor = Some(CostEditor {
                action: strip_options(self.selected_play_action().unwrap()),
                choices: draft.choices.clone(),
                commit: false,
                scroll: 0,
                allocation: None,
                last_plan: None,
                component_scroll: 0,
                allocation_scroll: 0,
            });
        }
        self.play_draft = Some(draft);
        Ok(true)
    }
    pub(super) fn maybe_review_cost(&mut self, action: &Action) -> Result<bool, String> {
        let (base, choices) = match action {
            Action::WithPlayChoices { action, choices } if choices.cost_order.is_empty() => {
                (action.as_ref().clone(), choices.clone())
            }
            action if !has_play_choices(action) => {
                let (repeat, accelerate, bonus, targets) = match action {
                    Action::PlayCardOptions {
                        repeat,
                        accelerate,
                        bonus,
                        repeat_targets,
                        ..
                    }
                    | Action::PlayFromTrashOptions {
                        repeat,
                        accelerate,
                        bonus,
                        repeat_targets,
                        ..
                    }
                    | Action::FinalizePendingPlay {
                        repeat,
                        accelerate,
                        bonus,
                        repeat_targets,
                        ..
                    } => (*repeat, *accelerate, *bonus, repeat_targets.clone()),
                    Action::PlayChampionOptions { accelerate, .. }
                    | Action::FinalizeTokenPlay { accelerate, .. } => {
                        (false, *accelerate, false, vec![])
                    }
                    _ => (false, false, false, vec![]),
                };
                let mut choices = PlayChoices {
                    accelerate,
                    bonus,
                    ..Default::default()
                };
                if repeat
                    && let Some(view) = super::optional_query(self.game.play_choices(action))?
                    && let Some(option) = view.repeats.iter().find(|option| {
                        option.source
                            == (ChainSource::Card {
                                card_id: view.card_id,
                            })
                    })
                {
                    choices.repeats.push(RepeatSelection {
                        option_id: option.id,
                        choices: ExecutionChoices {
                            targets,
                            mode: None,
                        },
                    });
                }
                (strip_options(action.clone()), choices)
            }
            _ => return Ok(false),
        };
        if let Some(plan) = super::optional_query(self.game.cost_plan(action))?
            && (plan
                .modifiers
                .iter()
                .filter(|modifier| {
                    !modifier.increase && modifier.source != (ChainSource::Card { card_id: 35 })
                })
                .count()
                > 1
                || plan.modifiers.iter().any(|modifier| {
                    !modifier.increase
                        && (matches!(action, Action::WithPlayChoices { .. })
                            || modifier.source != (ChainSource::Card { card_id: 35 }))
                        && (modifier.choose_energy_or_power
                            || (modifier.power > 0
                                && matches!(modifier.scope, CostScope::Total)
                                && plan
                                    .components
                                    .iter()
                                    .filter(|component| component.cost.power > 0)
                                    .count()
                                    > 1))
                }))
        {
            self.cost_editor = Some(CostEditor {
                action: base,
                choices,
                commit: true,
                scroll: 0,
                allocation: None,
                last_plan: None,
                component_scroll: 0,
                allocation_scroll: 0,
            });
            self.hover_card = None;
            self.hover_rect = None;
            return Ok(true);
        }
        Ok(false)
    }
    pub(super) fn draw_cost_editor(&mut self) {
        let Some(mut editor) = self.cost_editor.take() else {
            return;
        };
        let wrapped = Action::WithPlayChoices {
            action: Box::new(editor.action.clone()),
            choices: editor.choices.clone(),
        };
        let result = self.game.cost_plan(&wrapped);
        let validation_error = result.as_ref().err().cloned();
        let plan = match result {
            Ok(plan) => {
                editor.last_plan = Some(plan.clone());
                plan
            }
            Err(error) => {
                if let Some(plan) = editor.last_plan.clone() {
                    plan
                } else {
                    self.notify(error);
                    return;
                }
            }
        };
        if editor.choices.cost_order.is_empty() {
            editor.choices.cost_order = plan.applied_order.clone();
        }
        self.ui.rect(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            Color::new(0.0, 0.02, 0.03, 0.85),
        );
        self.ui.rect(Rect::new(300.0, 136.0, 840.0, 650.0), PANEL);
        self.ui.border(Rect::new(300.0, 136.0, 840.0, 650.0), GOLD);
        self.ui.enabled = editor.allocation.is_none();
        self.ui
            .title("Choose how costs are reduced", 325.0, 179.0, 25.0, TEXT);
        self.ui.text("Energy / Power choices and order are fixed before paying. Resources are chosen afterward.",325.0,205.0,11.0,MUTED);
        self.ui.text("COST COMPONENTS", 325.0, 240.0, 10.0, GOLD);
        if self.ui.hover(Rect::new(325.0, 250.0, 295.0, 375.0)) {
            if self.ui.wheel() < 0.0 {
                editor.component_scroll = editor
                    .component_scroll
                    .saturating_add(1)
                    .min(plan.components.len().saturating_sub(8));
            }
            if self.ui.wheel() > 0.0 {
                editor.component_scroll = editor.component_scroll.saturating_sub(1);
            }
        }
        for (row, component) in plan
            .components
            .iter()
            .skip(editor.component_scroll)
            .take(8)
            .enumerate()
        {
            let y = 266.0 + row as f32 * 46.0;
            self.ui
                .ellipsis(&component.label, 325.0, y, 295.0, 11.0, TEXT);
            self.ui.ellipsis(
                &value_label(&component.cost),
                325.0,
                y + 18.0,
                295.0,
                10.0,
                MUTED,
            );
        }
        self.ui.bold(
            &format!(
                "Pay {} energy + {} power",
                plan.energy,
                plan.power
                    .iter()
                    .map(|cost| u32::from(cost.amount))
                    .sum::<u32>()
            ),
            325.0,
            670.0,
            16.0,
            TEAL,
        );
        self.ui.wrapped_lines("The total includes chosen optional costs and reductions. Matching power requirements still apply.",325.0,696.0,295.0,11.0,MUTED,2);
        let modifiers: Vec<_> = plan
            .applied_order
            .iter()
            .filter_map(|selection| {
                plan.modifiers
                    .iter()
                    .find(|modifier| modifier.id == selection.modifier_id)
                    .map(|modifier| (selection.clone(), modifier.clone()))
            })
            .collect();
        let list = Rect::new(650.0, 248.0, 464.0, 460.0);
        if self.ui.hover(list) {
            if self.ui.wheel() < 0.0 {
                editor.scroll = editor
                    .scroll
                    .saturating_add(1)
                    .min(modifiers.len().saturating_sub(4));
            }
            if self.ui.wheel() > 0.0 {
                editor.scroll = editor.scroll.saturating_sub(1);
            }
        }
        self.ui.text(
            "APPLICATION ORDER · scroll for more",
            650.0,
            240.0,
            10.0,
            GOLD,
        );
        for (slot, (selection, modifier)) in
            modifiers.iter().enumerate().skip(editor.scroll).take(4)
        {
            let y = 254.0 + (slot - editor.scroll) as f32 * 110.0;
            self.ui.rect(Rect::new(650.0, y, 464.0, 101.0), BG);
            self.ui.ellipsis(
                &format!(
                    "{}. {}",
                    slot + 1,
                    source_label(&self.game, &modifier.source)
                ),
                660.0,
                y + 19.0,
                355.0,
                12.0,
                TEXT,
            );
            self.ui
                .ellipsis(&modifier.label, 660.0, y + 36.0, 355.0, 9.0, MUTED);
            let scope = match modifier.scope {
                CostScope::Total => "total".into(),
                CostScope::Component { component_id } => plan
                    .components
                    .iter()
                    .find(|part| part.id == component_id)
                    .map_or("component".into(), |part| part.label.clone()),
            };
            let description = if modifier.increase {
                format!("Adds {} energy / {} power", modifier.energy, modifier.power)
            } else if modifier.choose_energy_or_power {
                format!(
                    "Reduce {} energy OR {} power · {scope}",
                    modifier.energy, modifier.power
                )
            } else {
                format!(
                    "Reduce {} energy + {} power · min {} energy · {scope}",
                    modifier.energy, modifier.power, modifier.minimum_energy
                )
            };
            self.ui
                .ellipsis(&description, 660.0, y + 53.0, 440.0, 10.0, TEAL);
            if modifier.choose_energy_or_power {
                for (index, power) in [false, true].into_iter().enumerate() {
                    if self.ui.button(
                        Rect::new(660.0 + index as f32 * 100.0, y + 64.0, 94.0, 27.0),
                        if power { "Power" } else { "Energy" },
                        selection.use_power == power,
                        true,
                    ) && let Some(choice) = editor
                        .choices
                        .cost_order
                        .iter_mut()
                        .find(|choice| choice.modifier_id == modifier.id)
                    {
                        choice.use_power = power;
                        choice.power_allocations.clear();
                    }
                }
            }
            if !modifier.increase
                && modifier.power > 0
                && (!modifier.choose_energy_or_power || selection.use_power)
                && self.ui.button(
                    Rect::new(866.0, y + 64.0, 143.0, 27.0),
                    "Power allocation",
                    editor.allocation == Some(modifier.id),
                    true,
                )
            {
                editor.allocation = Some(modifier.id);
            }
            if slot > 0
                && !modifiers[slot - 1].1.increase
                && matches!(
                    (&modifiers[slot - 1].1.scope, &modifier.scope),
                    (CostScope::Total, CostScope::Total)
                        | (CostScope::Component { .. }, CostScope::Component { .. })
                )
                && !modifier.increase
                && self.ui.button(
                    Rect::new(1023.0, y + 10.0, 80.0, 27.0),
                    "Earlier",
                    false,
                    true,
                )
                && let Some(index) = editor
                    .choices
                    .cost_order
                    .iter()
                    .position(|choice| choice.modifier_id == modifier.id)
                && index > 0
            {
                editor.choices.cost_order.swap(index, index - 1);
                for choice in &mut editor.choices.cost_order {
                    choice.power_allocations.clear();
                }
            }
        }
        if modifiers.is_empty() {
            self.ui.text(
                "No reductions apply to this action.",
                650.0,
                276.0,
                12.0,
                MUTED,
            );
        }
        if let Some(error) = &validation_error {
            self.ui
                .wrapped_lines(error, 325.0, 718.0, 780.0, 10.0, GOLD, 1);
        }
        if let Some(id) = editor.allocation {
            self.ui.enabled = true;
            self.draw_power_allocation(&mut editor, &plan, id);
        } else {
            if self
                .ui
                .button(Rect::new(325.0, 736.0, 190.0, 32.0), "Cancel", false, true)
            {
                return;
            }
            if self.ui.button(
                Rect::new(530.0, 736.0, 190.0, 32.0),
                "Suggested order",
                false,
                true,
            ) {
                editor.choices.cost_order.clear();
            }
            if self.ui.button(
                Rect::new(735.0, 736.0, 379.0, 32.0),
                if editor.commit {
                    "Use these costs and continue"
                } else {
                    "Save choices · return to mat"
                },
                true,
                validation_error.is_none(),
            ) {
                if editor.commit {
                    let action = Action::WithPlayChoices {
                        action: Box::new(editor.action),
                        choices: editor.choices,
                    };
                    if self.game.clone().apply(action.clone()).is_err()
                        && self.can_begin_payment(&action)
                    {
                        self.begin_ui_payment(action);
                    } else {
                        self.action(action);
                    }
                } else if let Some(draft) = &mut self.play_draft {
                    draft.choices = editor.choices;
                }
                return;
            }
        }
        self.cost_editor = Some(editor);
    }
    fn draw_power_allocation(&self, editor: &mut CostEditor, plan: &CostPlanView, id: u64) {
        let Some(modifier) = plan.modifiers.iter().find(|modifier| modifier.id == id) else {
            editor.allocation = None;
            return;
        };
        let Some(selection) = editor
            .choices
            .cost_order
            .iter_mut()
            .find(|choice| choice.modifier_id == id)
        else {
            editor.allocation = None;
            return;
        };
        self.ui.rect(Rect::new(510.0, 236.0, 610.0, 483.0), PANEL);
        self.ui.border(Rect::new(510.0, 236.0, 610.0, 483.0), TEAL);
        self.ui
            .title("Allocate the Power reduction", 532.0, 272.0, 20.0, TEXT);
        self.ui.wrapped_lines("Choose which cost components lose generic Power. Empty allocation uses the suggested distribution.",532.0,299.0,560.0,11.0,MUTED,2);
        let total: u16 = selection
            .power_allocations
            .iter()
            .map(|part| part.amount)
            .sum();
        self.ui.text(
            &format!("Assigned {total} / {} reduction", modifier.power),
            532.0,
            345.0,
            12.0,
            TEAL,
        );
        if self.ui.hover(Rect::new(532.0, 355.0, 560.0, 270.0)) {
            if self.ui.wheel() < 0.0 {
                editor.allocation_scroll = editor
                    .allocation_scroll
                    .saturating_add(1)
                    .min(plan.components.len().saturating_sub(6));
            }
            if self.ui.wheel() > 0.0 {
                editor.allocation_scroll = editor.allocation_scroll.saturating_sub(1);
            }
        }
        for (row, component) in plan
            .components
            .iter()
            .skip(editor.allocation_scroll)
            .take(6)
            .enumerate()
        {
            let y = 365.0 + row as f32 * 43.0;
            self.ui
                .ellipsis(&component.label, 532.0, y + 19.0, 330.0, 11.0, TEXT);
            let amount = selection
                .power_allocations
                .iter()
                .find(|part| part.component_id == component.id)
                .map_or(0, |part| part.amount);
            self.ui
                .text(&amount.to_string(), 974.0, y + 20.0, 13.0, TEXT);
            for (x, add) in [(927.0, false), (1020.0, true)] {
                if self.ui.button(
                    Rect::new(x, y, 34.0, 28.0),
                    if add { "+" } else { "-" },
                    false,
                    if add {
                        total < modifier.power
                            && amount < component.cost.power
                            && match modifier.scope {
                                CostScope::Total => true,
                                CostScope::Component { component_id } => {
                                    component_id == component.id
                                }
                            }
                    } else {
                        amount > 0
                    },
                ) {
                    if let Some(part) = selection
                        .power_allocations
                        .iter_mut()
                        .find(|part| part.component_id == component.id)
                    {
                        part.amount = if add {
                            part.amount + 1
                        } else {
                            part.amount - 1
                        };
                    } else if add {
                        selection.power_allocations.push(PowerReduction {
                            component_id: component.id,
                            amount: 1,
                        });
                    }
                    selection.power_allocations.retain(|part| part.amount > 0);
                }
            }
        }
        if self.ui.button(
            Rect::new(532.0, 666.0, 267.0, 31.0),
            "Suggested allocation",
            false,
            true,
        ) {
            selection.power_allocations.clear();
        }
        if self
            .ui
            .button(Rect::new(817.0, 666.0, 281.0, 31.0), "Done", true, true)
        {
            editor.allocation = None;
        }
    }
    pub(super) fn token_play_panel(
        &mut self,
        card_id: usize,
        remaining: u8,
        fixed_lane: Option<Option<usize>>,
    ) -> Result<(), String> {
        let name = local_rift::rules::token_spec(card_id).map_or("token", |spec| spec.name);
        let entry_ready = self
            .game
            .current_pending_token()
            .is_some_and(|token| token.entry_ready);
        let base = Action::FinalizeTokenPlay {
            lane: fixed_lane.flatten(),
            accelerate: self.options.accelerate,
        };
        let accelerate =
            super::optional_query(self.game.play_choices(&base))?.and_then(|view| view.accelerate);
        let location = match fixed_lane {
            Some(Some(lane)) => format!("Fixed at field {}.", lane + 1),
            Some(None) => "Fixed at your base.".into(),
            None => "Choose base or a field you control.".into(),
        };
        self.ui
            .title(&format!("Play a {name}"), 1178.0, 366.0, 21.0, TEXT);
        self.ui.wrapped_lines(
            &format!(
                "{remaining} remaining. {location} {}",
                if entry_ready {
                    "Enters ready by instruction."
                } else if accelerate.is_some() {
                    "Accelerate can make it enter ready."
                } else {
                    "Normal entry rules apply."
                }
            ),
            1178.0,
            397.0,
            209.0,
            11.0,
            MUTED,
            3,
        );
        let label = accelerate
            .as_ref()
            .map_or("No optional Accelerate".into(), |option| {
                format!("Accelerate · {}", value_label(&option.cost))
            });
        if self.ui.button(
            Rect::new(1178.0, 443.0, 212.0, 29.0),
            &label,
            self.options.accelerate,
            accelerate.is_some(),
        ) {
            self.options.accelerate = !self.options.accelerate;
        }
        for (index, lane) in [None, Some(0), Some(1)].into_iter().enumerate() {
            let allowed = fixed_lane.map_or_else(
                || lane.is_none_or(|lane| self.game.battlefield_owner[lane] == Some(0)),
                |fixed| fixed == lane,
            );
            if self.ui.button(
                Rect::new(1178.0, 480.0 + index as f32 * 29.0, 212.0, 25.0),
                &lane.map_or("Play at base".into(), |lane| {
                    format!("Play at field {}", lane + 1)
                }),
                false,
                allowed,
            ) {
                let action = Action::FinalizeTokenPlay {
                    lane,
                    accelerate: self.options.accelerate,
                };
                if self.game.clone().apply(action.clone()).is_ok() {
                    self.action(action);
                } else if self.can_begin_payment(&action) {
                    self.begin_ui_payment(action);
                } else {
                    self.action(action);
                }
            }
        }
        Ok(())
    }
    pub(super) fn draw_play_effects(&mut self) {
        if !self.show_play_effects {
            return;
        }
        self.ui.rect(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            Color::new(0.0, 0.02, 0.03, 0.82),
        );
        self.ui.rect(Rect::new(330.0, 140.0, 780.0, 615.0), PANEL);
        self.ui.border(Rect::new(330.0, 140.0, 780.0, 615.0), GOLD);
        self.ui.enabled = true;
        self.ui
            .title("Effects available this turn", 354.0, 183.0, 25.0, TEXT);
        let mut lines = Vec::new();
        for grant in &self.game.play_grants {
            let effect=match grant.kind {
                PlayGrantKind::ReadyNextUnit=>"Your next unit enters ready.".into(),
                PlayGrantKind::AccelerateNextUnit=>"Your next unit gains optional Accelerate for ready entry.".into(),
                PlayGrantKind::BonusNextSpell(amount)=>format!("Your next spell deals {amount} Bonus Damage."),
                PlayGrantKind::RepeatNextSpell=>"Your next spell gains a separate optional Repeat equal to its printed base cost.".into(),
                PlayGrantKind::DiscountNextSpell{energy,power}=>format!("Your next spell costs {energy} energy and {power} power less."),
                PlayGrantKind::DiscountNextCard{energy,power}=>format!("Your next card costs {energy} energy and {power} power less."),
                PlayGrantKind::ReadyUnitsThisTurn=>"Your units enter ready this turn.".into(),
                PlayGrantKind::ReadyBuffNextUnit=>"After your next unit is played, a triggered ability will ready it and give it a buff.".into(),
                PlayGrantKind::UnitSurchargeThisTurn(amount)=>format!("Your non-token units cost {amount} more energy this turn."),
            };
            lines.push(format!(
                "{} · {}\n{}",
                if grant.player == 0 { "YOU" } else { "OPPONENT" },
                source_label(&self.game, &grant.source),
                effect
            ));
        }
        if lines.is_empty() {
            lines.push("No waiting next-play effects. Printed abilities on units, gear and locations still apply.".into());
        }
        for player in 0..2 {
            lines.push(format!("{} · PUBLIC TURN HISTORY\n{} non-token gear played · {} gear abilities played\nUnits or gear played here: field 1 = {}, field 2 = {}",if player==0 {"YOU"} else {"OPPONENT"},self.game.play_history.non_token_gears[player],self.game.play_history.gear_activations[player],self.game.play_history.units_or_gears_here[player][0],self.game.play_history.units_or_gears_here[player][1]));
        }
        self.ui.scroll_text(
            &lines.join("\n\n"),
            Rect::new(354.0, 219.0, 732.0, 440.0),
            14.0,
            &mut self.play_effects_scroll,
        );
        self.ui.text(
            "Scroll for all effects. Next-play effects are committed when that play begins.",
            354.0,
            685.0,
            11.0,
            MUTED,
        );
        if self.ui.button(
            Rect::new(354.0, 704.0, 732.0, 31.0),
            "Back to the mat",
            true,
            true,
        ) {
            self.show_play_effects = false;
        }
    }
}
