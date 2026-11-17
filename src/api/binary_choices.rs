use super::*;

impl Session {
    /// Both answers to a binary question, including unavailable answers and
    /// their reasons. Only the acting seat may inspect a private decision.
    pub fn binary_choices(&self, player: usize, revision: u64) -> Result<Value, ApiError> {
        self.require_revision(revision)?;
        self.require_actor(player)?;
        let view = self
            .game
            .binary_choices()
            .map_err(ApiError::illegal)?
            .ok_or_else(|| ApiError::illegal("This decision does not use a binary choice."))?;
        Ok(json!({
            "player":view.player,
            "scope":"choose",
            "exhaustive":true,
            "options":view.options.iter().map(|option| json!({
                "action":action_to_json(&option.action),
                "label":option.label,
                "enabled":option.enabled(),
                "unavailable_reason":option.unavailable_reason
            })).collect::<Vec<_>>()
        }))
    }
}

#[cfg(test)]
mod tests;
