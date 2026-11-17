//! Shared resource instructions resolve in turn order (CR303.2.a).
//! Instructed token plays wait for their parent spell to finish (CR354.3).
use super::*;

#[cfg(test)]
#[path = "resource_spells_tests.rs"]
mod tests;

impl Game {
    pub(super) fn each_player_discard_then_draw(&mut self, count: u8) -> Result<(), String> {
        // Finish the first instruction for both players before starting draws.
        for player in [self.active, 1 - self.active] {
            let state = &mut self.players[player];
            let discarded = state.hand.len();
            state.discard.append(&mut state.hand);
            self.push_log(format!(
                "{} discards {discarded} cards.",
                self.players[player].name
            ));
        }
        for player in [self.active, 1 - self.active] {
            if self.winner.is_some() {
                break;
            }
            self.draw(player, count)?;
        }
        Ok(())
    }

    pub(super) fn play_conditional_gold(&mut self, player: usize, card_id: CardId) {
        let enhanced = self.active == player
            && self.resolution_frame.as_ref().is_some_and(|frame| {
                self.chain.iter().any(|item| {
                    item.id == frame.chain_id
                        && item.player == player
                        && item.card_id == card_id
                        && item.ability.is_none()
                        && item.from_hidden
                })
            });
        for _ in 0..if enhanced { 3 } else { 1 } {
            self.queue_token_play_with_entry(player, card_id, 0, Some(None), true, enhanced);
        }
    }

    pub(super) fn resolve_shared_resources(
        &mut self,
        chooser: usize,
        cards: bool,
    ) -> Result<(), String> {
        self.push_log(format!(
            "{} chooses {} for Party Favors.",
            self.players[chooser].name,
            if cards { "Cards" } else { "Runes" }
        ));
        for player in [self.active, 1 - self.active] {
            if self.winner.is_some() {
                break;
            }
            if cards {
                self.draw(player, 1)?;
            } else {
                self.channel_runes(player, 1, true)?;
            }
        }
        Ok(())
    }

    pub(super) fn shared_resources_prefers_cards(&self, player: usize) -> bool {
        let own = &self.players[player];
        let other = &self.players[1 - player];
        // Only public counts and the chooser's information influence this
        // modest heuristic. Hidden card identities and deck order do not.
        let draw_value = |hand: usize| if hand <= 2 { 5 } else { 2 };
        let rune_value = |state: &PlayerState| {
            if state.rune_deck.is_empty() {
                0
            } else if state.total_runes() < 6 {
                4
            } else {
                1
            }
        };
        let burnout = if own.deck.is_empty() && other.score >= 7 {
            10
        } else {
            0
        };
        draw_value(own.hand.len()) - draw_value(other.hand.len()) - burnout
            >= rune_value(own) - rune_value(other)
    }
}
