//! CR832: Show Off is a finalization choice, not a target or a resolution cost.
//! A revealed/picked object's identity is captured before payment. Repeat repeats
//! resolution instructions, not this choice (CR820.1.d.2).
use crate::cards::{CardId, CardKind};
use crate::game::{Action, Game, Target};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShowOffObject {
    Hand { hand_index: usize },
    Gear { gear_id: u32 },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShowOffSelection {
    pub instance: usize,
    pub object: ShowOffObject,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShownObject {
    pub selection: ShowOffSelection,
    pub card_id: CardId,
    pub token: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShowOffOption {
    pub instance: usize,
    pub label: String,
    pub candidates: Vec<ShownObject>,
}

fn source_hand_index(action: &Action) -> Option<usize> {
    match action {
        Action::WithPlayChoices { action, .. }
        | Action::WithPowerPayment { action, .. }
        | Action::BeginPayment { action } => source_hand_index(action),
        Action::PlayCard { hand_index, .. }
        | Action::PlayCardWithTargets { hand_index, .. }
        | Action::PlayCardOptions { hand_index, .. } => Some(*hand_index),
        _ => None,
    }
}

impl Game {
    pub(crate) fn show_off_options(&self, action: &Action, card_id: CardId) -> Vec<ShowOffOption> {
        if card_id != 426 {
            return vec![];
        }
        // Wire hand indices refer to the draft before its source is detached,
        // matching other hand-based play choices. The source itself is excluded.
        let p = &self.players[self.priority()];
        let mut candidates = Vec::new();
        for (hand_index, &card_id) in p.hand.iter().enumerate() {
            if Some(hand_index) != source_hand_index(action)
                && self.catalog[card_id].kind == CardKind::Gear
            {
                candidates.push(ShownObject {
                    selection: ShowOffSelection {
                        instance: 0,
                        object: ShowOffObject::Hand { hand_index },
                    },
                    card_id,
                    token: false,
                });
            }
        }
        for g in &p.gear {
            candidates.push(ShownObject {
                selection: ShowOffSelection {
                    instance: 0,
                    object: ShowOffObject::Gear { gear_id: g.id },
                },
                card_id: g.card_id,
                token: g.token,
            });
        }
        vec![ShowOffOption {
            instance: 0,
            label: "Show Off a gear (optional)".into(),
            candidates,
        }]
    }

    pub(crate) fn capture_show_off(
        &self,
        action: &Action,
        card_id: CardId,
        selections: &[ShowOffSelection],
    ) -> Result<Vec<ShownObject>, String> {
        if card_id != 426 {
            return if selections.is_empty() {
                Ok(vec![])
            } else {
                Err("This card has no Show Off choice.".into())
            };
        }
        if selections.len() > 1 || selections.iter().any(|s| s.instance != 0) {
            return Err("Battle Preparations may show off one gear for instance 0.".into());
        }
        if let Some(snapshot) = self.payment_show_off() {
            if snapshot.iter().map(|s| &s.selection).eq(selections.iter()) {
                return Ok(snapshot.to_vec());
            }
            return Err("Show Off was already fixed before this payment.".into());
        }
        let options = self.show_off_options(action, card_id);
        selections
            .iter()
            .map(|selection| {
                options[0]
                    .candidates
                    .iter()
                    .find(|c| c.selection == *selection)
                    .cloned()
                    .ok_or_else(|| {
                        "Show off a gear in your hand or a friendly gear on the board.".into()
                    })
            })
            .collect()
    }

    pub(crate) fn log_show_off(&mut self, shown: &[ShownObject]) {
        for object in shown {
            let name = if object.token {
                "Gold"
            } else {
                self.catalog[object.card_id].name
            };
            let zone = match object.selection.object {
                ShowOffObject::Hand { .. } => "from hand",
                ShowOffObject::Gear { .. } => "on the board",
            };
            self.push_log(format!(
                "{} shows off {name} {zone}.",
                self.players[self.priority()].name
            ));
        }
    }

    pub(crate) fn validate_preparations_targets(
        &self,
        actor: usize,
        targets: &[Target],
        showed_off: bool,
        complete: bool,
    ) -> Result<(), String> {
        if targets.len() > if showed_off { 3 } else { 1 } {
            return Err(
                "Choose one unit, then up to two different runes if you showed off.".into(),
            );
        }
        if complete && targets.is_empty() {
            return Err("Choose a unit for Battle Preparations.".into());
        }
        for (slot, &t) in targets.iter().enumerate() {
            let valid = if slot == 0 {
                self.can_choose_unit(actor, t)
            } else {
                matches!(t, Target::Rune { player, rune_id } if self.players.get(player)
                    .is_some_and(|p| p.rune_objects().iter().any(|r| r.id == rune_id)))
            };
            if !valid || targets[..slot].contains(&t) {
                return Err("Choose a legal unit followed by different physical runes.".into());
            }
        }
        Ok(())
    }

    pub(crate) fn preparations_candidates(&self, actor: usize, prior: &[Target]) -> Vec<Target> {
        if self
            .validate_preparations_targets(actor, prior, true, false)
            .is_err()
            || prior.len() >= 3
        {
            return vec![];
        }
        if prior.is_empty() {
            self.players
                .iter()
                .enumerate()
                .flat_map(|(player, p)| {
                    p.units.iter().map(move |u| Target::Unit {
                        player,
                        unit_id: u.id,
                    })
                })
                .filter(|&t| self.can_choose_unit(actor, t))
                .collect()
        } else {
            self.players
                .iter()
                .enumerate()
                .flat_map(|(player, p)| {
                    p.rune_objects().iter().map(move |r| Target::Rune {
                        player,
                        rune_id: r.id,
                    })
                })
                .filter(|t| !prior.contains(t))
                .collect()
        }
    }

    pub(crate) fn resolve_preparations(
        &mut self,
        actor: usize,
        targets: &[Target],
        showed_off: bool,
    ) -> Result<(), String> {
        if let Some(&unit) = targets.first()
            && self.can_choose_unit(actor, unit)
        {
            self.modify_might(unit, 2, 0)?;
        }
        // Independent instructions survive the other target becoming illegal.
        if showed_off {
            for &rune in targets.iter().skip(1) {
                if let Target::Rune { player, rune_id } = rune
                    && self
                        .players
                        .get(player)
                        .is_some_and(|p| p.rune_objects().iter().any(|r| r.id == rune_id))
                {
                    self.ready_rune_object(player, rune_id)?;
                }
            }
        }
        Ok(())
    }
}

impl Game {
    pub(super) fn preparations_play_candidates(
        &self,
        action: &Action,
        prepare: bool,
        limit: usize,
    ) -> Result<Vec<Action>, String> {
        use crate::play::{ExecutionChoices, PlayChoices, RepeatSelection};
        let p = self.priority();
        let options = self.show_off_options(action, 426);
        // Prefer information already public. Choosing to reveal is still explicit.
        let shown = options[0]
            .candidates
            .iter()
            .find(|c| matches!(c.selection.object, ShowOffObject::Gear { .. }))
            .or_else(|| options[0].candidates.first())
            .map(|c| c.selection.clone());
        let mut runes: Vec<_> = self.players[p].rune_objects().to_vec();
        runes.sort_by_key(|r| (r.ready, r.domain != crate::cards::Domain::Calm, r.id));
        let runes: Vec<_> = runes
            .into_iter()
            .take(2)
            .map(|r| Target::Rune {
                player: p,
                rune_id: r.id,
            })
            .collect();
        let mut result = Vec::new();
        for unit in self
            .preparations_candidates(p, &[])
            .into_iter()
            .filter(|t| matches!(t, Target::Unit { player, .. } if *player==p))
        {
            for show in [shown.clone(), None] {
                let mut targets = vec![unit];
                if show.is_some() {
                    targets.extend_from_slice(&runes);
                }
                let inner = match action {
                    Action::PlayCard { hand_index, .. } => Action::PlayCardWithTargets {
                        hand_index: *hand_index,
                        lane: None,
                        targets: targets.clone(),
                    },
                    Action::PlayFromTrash { trash_index, .. } => Action::PlayFromTrashWithTargets {
                        trash_index: *trash_index,
                        lane: None,
                        targets: targets.clone(),
                    },
                    Action::FinalizePendingPlay { .. } => Action::FinalizePendingPlay {
                        lane: None,
                        targets: targets.clone(),
                        repeat_targets: vec![],
                        repeat: false,
                        accelerate: false,
                        bonus: false,
                    },
                    _ => continue,
                };
                let base = PlayChoices {
                    show_off: show.into_iter().collect(),
                    ..Default::default()
                };
                let wrapped = Action::WithPlayChoices {
                    action: Box::new(inner.clone()),
                    choices: base.clone(),
                };
                let view = self.play_choices(&wrapped)?;
                let mut choices = vec![base.clone()];
                for repeat in view.repeats.iter().take(3) {
                    let mut repeated = base.clone();
                    repeated.repeats.push(RepeatSelection {
                        option_id: repeat.id,
                        choices: ExecutionChoices {
                            destination: None,
                            targets: targets.clone(),
                            mode: None,
                        },
                    });
                    choices.push(repeated);
                }
                for choice in choices {
                    let candidate = Action::WithPlayChoices {
                        action: Box::new(inner.clone()),
                        choices: choice,
                    };
                    let valid = if prepare {
                        self.clone().begin_payment(candidate.clone())
                    } else {
                        self.probe_action(&candidate)
                    };
                    match valid {
                        Ok(()) => {
                            if !result.contains(&candidate) {
                                result.push(candidate);
                            }
                        }
                        Err(error) if crate::numeric::is_capacity(&error) => return Err(error),
                        Err(_) => {}
                    }
                    if result.len() >= limit {
                        return Ok(result);
                    }
                }
            }
        }
        Ok(result)
    }

    pub(super) fn preparations_score(
        &self,
        targets: &[Target],
        shown: &[ShowOffSelection],
    ) -> i128 {
        let p = self.priority();
        let might = targets
            .first()
            .and_then(|&t| self.unit_ref(t))
            .map_or(0, |u| {
                let fighting = self
                    .showdown
                    .as_ref()
                    .is_some_and(|w| w.combat && u.lane == Some(w.lane));
                if matches!(targets[0],Target::Unit{player,..} if player==p) {
                    if fighting {
                        65
                    } else if u.ready {
                        24
                    } else {
                        4
                    }
                } else {
                    -30
                }
            });
        let runes = targets
            .iter()
            .skip(1)
            .filter(|t| matches!(t,Target::Rune{player,..} if *player==p))
            .count() as i128;
        let reveal = shown
            .iter()
            .any(|s| matches!(s.object, ShowOffObject::Hand { .. }));
        might + 8 * runes - if reveal { 2 } else { 0 }
    }
}
