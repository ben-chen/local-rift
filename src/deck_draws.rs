//! Inspected occurrences remain in their deck until commitment. Revealing a
//! selected card is public; rejected private cards never enter public records.
use super::*;

impl Game {
    pub(super) fn draw_look_eligible(
        cards: &[CardId],
        kind: DrawLookKind,
        catalog: &[CardDef],
    ) -> Vec<usize> {
        cards
            .iter()
            .enumerate()
            .filter_map(|(index, &id)| {
                let card = catalog[id];
                let eligible = match kind {
                    DrawLookKind::DoubleTrouble => card.kind == CardKind::Unit,
                    DrawLookKind::FateWeaver => card.kind == CardKind::Spell && card.cost >= 4,
                    DrawLookKind::TrifarianCaptain => {
                        matches!(card.kind, CardKind::Unit | CardKind::Gear)
                            && card.cost <= 3
                            && card.power_cost <= 1
                    }
                };
                eligible.then_some(index)
            })
            .collect()
    }

    pub(crate) fn look_to_draw(
        &mut self,
        player: usize,
        source_card_id: CardId,
        kind: DrawLookKind,
    ) {
        let count = if kind == DrawLookKind::DoubleTrouble {
            3
        } else {
            4
        };
        let cards: Vec<_> = self.players[player]
            .deck
            .iter()
            .rev()
            .take(count)
            .copied()
            .collect();
        let eligible = Self::draw_look_eligible(&cards, kind, &self.catalog);
        let choice_id = self.next_chain_id;
        self.next_chain_id += 1;
        self.enqueue_decision(Decision::DrawLook {
            player,
            choice_id,
            source_card_id,
            cards,
            eligible,
            kind,
        });
    }

    pub(super) fn resolve_draw_look(
        &mut self,
        choice_id: u32,
        index: Option<usize>,
    ) -> Result<(), String> {
        let Some(Decision::DrawLook {
            player,
            choice_id: expected,
            source_card_id,
            cards,
            eligible,
            ..
        }) = self.decision.clone()
        else {
            return Err("No draw selection is pending.".into());
        };
        if choice_id != expected {
            return Err("That draw selection is no longer current.".into());
        }
        if index.is_some_and(|i| !eligible.contains(&i)) {
            return Err("Choose an eligible inspected occurrence, or recycle all.".into());
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
            return Err("The inspected cards have changed.".into());
        }
        self.decision = None;
        let len = self.players[player].deck.len();
        if let Some(index) = index {
            let card = cards[index];
            self.log_draw_reveal(player, source_card_id, card);
            // The specified occurrence is Drawn while the inspected remainder
            // is still in its actual zone; observers see that event in order.
            self.players[player].deck.remove(len - 1 - index);
            self.draw_exact_card(player, card);
        }
        let mut rest: Vec<_> = cards
            .iter()
            .enumerate()
            .filter(|(i, _)| Some(*i) != index)
            .map(|(_, &id)| id)
            .collect();
        self.players[player].deck.truncate(len - cards.len());
        self.rng.shuffle(&mut rest);
        self.players[player].deck.splice(0..0, rest);
        self.push_log(format!(
            "{} recycles {} inspected cards.",
            self.players[player].name,
            cards.len() - usize::from(index.is_some())
        ));
        if let Some(index) = index {
            self.enqueue_decision(Decision::RevealedCard {
                player,
                source_card_id,
                card_id: cards[index],
                outcome: RevealOutcome::Draw,
            });
        }
        self.after_resolution()
    }

    fn log_draw_reveal(&mut self, player: usize, source: CardId, card: CardId) {
        self.push_log(format!(
            "{} reveals {} for {}.",
            self.players[player].name, self.catalog[card].name, self.catalog[source].name
        ));
    }

    pub(crate) fn reveal_moved_top(
        &mut self,
        player: usize,
        source_card_id: CardId,
        source: Option<Target>,
    ) -> Result<(), String> {
        // Core432.1.c.1 negates subsequent instructions on an unavailable
        // revealed card, including Pakaa's otherwise branch. No Draw/Burn Out.
        let Some(&card) = self.players[player].deck.last() else {
            return Ok(());
        };
        self.log_draw_reveal(player, source_card_id, card);
        self.players[player].deck.pop();
        let printed = self.catalog[card];
        let draws = match source_card_id {
            392 => printed.kind == CardKind::Gear,
            393 => printed.kind == CardKind::Unit && printed.might >= 5,
            394 => printed.kind == CardKind::Unit,
            _ => unreachable!("Only reviewed Move/reveal sources reach this effect"),
        };
        let outcome = if draws {
            self.draw_exact_card(player, card);
            RevealOutcome::Draw
        } else if source_card_id == 394 {
            self.players[player].discard.push(card);
            if let Some(source) = source {
                self.modify_might(source, 2, 0)?;
            }
            RevealOutcome::Trash
        } else {
            self.players[player].deck.insert(0, card);
            RevealOutcome::Recycle
        };
        self.enqueue_decision(Decision::RevealedCard {
            player,
            source_card_id,
            card_id: card,
            outcome,
        });
        Ok(())
    }
}

#[cfg(test)]
#[path = "deck_draws/tests.rs"]
mod tests;
