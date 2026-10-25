//! A Predict choice suspends an instruction sequence, not its parent execution.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PredictContinuationView {
    pub chain_id: u32,
    pub player: usize,
    pub source_card_id: CardId,
    pub remaining_draw: u8,
    pub remaining_xp: u64,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum PredictCompletion {
    SpellExecution,
    Activated {
        source: AbilitySource,
        ability_index: usize,
    },
}
#[derive(Clone, Copy, Debug)]
pub(super) struct PredictContinuation {
    pub view: PredictContinuationView,
    completion: PredictCompletion,
}

impl Game {
    pub fn predict_continuation(&self) -> Option<PredictContinuationView> {
        self.predict_tail.map(|tail| tail.view)
    }

    pub(super) fn start_predict_sequence(
        &mut self,
        view: PredictContinuationView,
        completion: PredictCompletion,
        count: u8,
    ) -> Result<(), String> {
        if self.predict_tail.is_some() {
            return Err("A Predict instruction sequence is already resolving.".into());
        }
        if !self.chain.iter().any(|item| item.id == view.chain_id) {
            return Err("The Predict source is no longer resolving.".into());
        }
        self.predict_tail = Some(PredictContinuation { view, completion });
        self.predict_many(view.player, count);
        if self.decision.is_none() && self.pending_decisions.is_empty() {
            // Empty Predict is still followed by its independent Draw/XP.
            self.resume_predict_sequence()?;
        }
        Ok(())
    }

    pub(super) fn resume_predict_sequence(&mut self) -> Result<(), String> {
        let tail = self
            .predict_tail
            .ok_or("No Predict sequence is suspended.")?;
        let index = self
            .chain
            .iter()
            .position(|item| item.id == tail.view.chain_id)
            .ok_or("The suspended Predict source is missing.")?;
        // Keep the frame present until ALL instructions finish. Events raised
        // by Draw and XP cannot finalize between the instructions (Core321).
        self.draw(tail.view.player, tail.view.remaining_draw)?;
        if self.winner.is_none() && tail.view.remaining_xp != 0 {
            self.gain_xp(tail.view.player, tail.view.remaining_xp)?;
        }
        if let PredictCompletion::Activated {
            source,
            ability_index,
        } = tail.completion
        {
            self.chain.remove(index);
            self.finish_activated(source, tail.view.source_card_id, ability_index);
        }
        self.predict_tail = None;
        Ok(())
    }
}

#[cfg(test)]
#[path = "prediction/tests.rs"]
mod tests;
