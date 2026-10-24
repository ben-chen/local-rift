//! Deck-to-Banish instructions. Looked cards remain in the deck until the
//! selection; a selected physical card moves before its parent's recycle.
use super::*;

#[derive(Clone, Debug)]
pub(super) struct DeckKillContinuation {
    pub player: usize,
    pub target: Option<Target>,
    pub might: Option<u64>,
    pub killed: bool,
}

impl Game {
    pub(super) fn deck_look_eligible(
        cards: &[CardId],
        kind: InstructedPlayKind,
        killed_might: Option<u64>,
        catalog: &[CardDef],
    ) -> Vec<usize> {
        cards
            .iter()
            .enumerate()
            .filter_map(|(index, &id)| {
                let c = catalog[id];
                let allowed = match kind {
                    InstructedPlayKind::BaitedHook => {
                        c.kind == CardKind::Unit
                            && killed_might
                                .is_some_and(|m| u64::from(c.might).saturating_sub(1) <= m)
                    }
                    InstructedPlayKind::Reinforce => c.kind == CardKind::Unit,
                    InstructedPlayKind::WildClaw => {
                        matches!(c.kind, CardKind::Unit | CardKind::Gear)
                    }
                    _ => false,
                };
                allowed.then_some(index)
            })
            .collect()
    }
    pub(crate) fn look_for_play(
        &mut self,
        player: usize,
        source_card_id: CardId,
        kind: InstructedPlayKind,
        killed_might: Option<u64>,
    ) {
        let cards: Vec<_> = self.players[player]
            .deck
            .iter()
            .rev()
            .take(5)
            .copied()
            .collect();
        let eligible = Self::deck_look_eligible(&cards, kind, killed_might, &self.catalog);
        let choice_id = self.next_chain_id;
        self.next_chain_id += 1;
        self.enqueue_decision(Decision::DeckLookPlay {
            player,
            choice_id,
            source_card_id,
            cards,
            eligible,
            kind,
            killed_might,
        });
    }
    pub(super) fn resolve_deck_look(
        &mut self,
        choice_id: u32,
        index: Option<usize>,
    ) -> Result<(), String> {
        let Some(Decision::DeckLookPlay {
            player,
            choice_id: expected,
            cards,
            eligible,
            kind,
            source_card_id,
            ..
        }) = self.decision.clone()
        else {
            return Err("No deck selection is pending.".into());
        };
        if choice_id != expected {
            return Err("That deck selection is no longer current.".into());
        }
        if index.is_some_and(|i| !eligible.contains(&i)) {
            return Err("That card does not satisfy this instruction.".into());
        }
        if self.players[player]
            .deck
            .iter()
            .rev()
            .take(cards.len())
            .copied()
            .collect::<Vec<_>>()
            != cards
        {
            return Err("The looked-at cards have changed.".into());
        }
        self.decision = None;
        let len = self.players[player].deck.len();
        self.players[player].deck.truncate(len - cards.len());
        // A unique link exists even if selection/play is impossible. It never
        // substitutes an identically named card elsewhere on the board.
        let mut pending_id = self.next_chain_id;
        self.next_chain_id += 1;
        if let Some(i) = index {
            let id = cards[i];
            let position = self.players[player].banish.len();
            self.players[player].banish.push(id);
            self.push_log(format!(
                "{} banishes {} from their deck.",
                self.players[player].name, self.catalog[id].name
            ));
            if !self.silenced[player] {
                pending_id = self.queue_banish_play(player, position, kind, None);
            }
        }
        let mut rest: Vec<_> = cards
            .iter()
            .enumerate()
            .filter(|(i, _)| Some(*i) != index)
            .map(|(_, &id)| id)
            .collect();
        self.rng.shuffle(&mut rest);
        self.players[player].deck.splice(0..0, rest);
        self.push_log(format!(
            "{} recycles {} looked-at cards.",
            self.players[player].name,
            cards.len() - usize::from(index.is_some())
        ));
        if kind == InstructedPlayKind::BaitedHook {
            self.gear_ability_used(player);
            self.push_log(format!(
                "{} finishes Baited Hook.",
                self.players[player].name
            ));
        }
        if kind == InstructedPlayKind::WildClaw {
            self.queue_trigger_choice(
                player,
                source_card_id,
                Decision::EmpowerPlayed {
                    player,
                    source_card_id,
                    pending_id,
                    target: None,
                },
            );
            self.trigger_batch += 1;
        }
        self.after_resolution()
    }
    pub(crate) fn begin_deck_kill(
        &mut self,
        player: usize,
        target: Option<Target>,
    ) -> Result<(), String> {
        self.pending_deck_kill = Some(DeckKillContinuation {
            player,
            target,
            might: None,
            killed: false,
        });
        if let Some(target) = target {
            self.kill_units(&[target])?;
        }
        self.finish_deck_kill();
        Ok(())
    }
    pub(super) fn finish_deck_kill(&mut self) {
        if self.pending_instruction_deaths.is_some() {
            return;
        }
        if let Some(hook) = self.pending_deck_kill.take() {
            self.look_for_play(
                hook.player,
                385,
                InstructedPlayKind::BaitedHook,
                hook.might.filter(|_| hook.killed),
            );
        }
    }
    pub(crate) fn reveal_aurora(&mut self, player: usize) {
        let mut cards = Vec::new();
        while let Some(id) = self.players[player].deck.pop() {
            cards.push(id);
            if self.catalog[id].kind == CardKind::Unit {
                break;
            }
        }
        if cards.is_empty() {
            return;
        }
        let mut rest = cards.clone();
        if rest
            .last()
            .is_some_and(|&id| self.catalog[id].kind == CardKind::Unit)
        {
            let id = rest.pop().unwrap();
            let position = self.players[player].banish.len();
            self.players[player].banish.push(id);
            if !self.silenced[player] {
                self.queue_banish_play(player, position, InstructedPlayKind::DazzlingAurora, None);
            }
        }
        self.rng.shuffle(&mut rest);
        self.players[player].deck.splice(0..0, rest);
        self.push_log(format!(
            "Dazzling Aurora reveals {}.",
            cards
                .iter()
                .map(|&id| self.catalog[id].name)
                .collect::<Vec<_>>()
                .join(", ")
        ));
        self.enqueue_decision(Decision::RevealedDeck {
            player,
            source_card_id: 386,
            cards,
        });
    }
    pub(super) fn link_instructed_entry(
        &mut self,
        pending_id: u32,
        player: usize,
        kind: CardKind,
        object_id: u32,
    ) {
        let target = match kind {
            CardKind::Unit => Some(Target::Unit {
                player,
                unit_id: object_id,
            }),
            CardKind::Gear => Some(Target::Gear {
                player,
                gear_id: object_id,
            }),
            CardKind::Spell => None,
        };
        for trigger in &mut self.pending_triggers {
            if let PendingTriggerEffect::Choice(Decision::EmpowerPlayed {
                pending_id: link,
                target: selected,
                ..
            }) = &mut trigger.effect
                && *link == pending_id
            {
                *selected = target;
            }
        }
    }
    pub(super) fn resolve_linked_empower(&mut self, target: Option<Target>) {
        let Some(target) = target else {
            return;
        };
        match target {
            Target::Unit { player, unit_id } if self.unit_ref(target).is_some() => {
                self.empower_by(player, empower::BoardObject::Unit { player, unit_id });
            }
            Target::Gear { player, gear_id }
                if self.players[player].gear.iter().any(|g| g.id == gear_id) =>
            {
                self.empower_by(player, empower::BoardObject::Gear { player, gear_id });
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "deck_plays/tests.rs"]
mod tests;
