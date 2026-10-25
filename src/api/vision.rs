//! Public Vision characteristics and provenance contain no looked-at cards.
use super::*;
use crate::empower::BoardObject;
use crate::game::VisionOrigin;

pub(super) fn origin_to_json(origin: VisionOrigin) -> Value {
    match origin {
        VisionOrigin::Printed => json!({"type":"printed"}),
        VisionOrigin::Granted {
            source,
            card_id,
            rule_index,
        } => json!({"type":"granted","source":empowered::board_object_to_json(source),
            "card_id":card_id,"rule_index":rule_index}),
    }
}

pub(super) fn instances_to_json(game: &Game, source: BoardObject) -> Result<Value, ApiError> {
    let instances = game.vision_instances(source).map_err(ApiError::illegal)?;
    Ok(json!(
        instances
            .iter()
            .map(|instance| json!({
                "source":empowered::board_object_to_json(instance.source),
                "origin":origin_to_json(instance.origin),"active":instance.active,
            }))
            .collect::<Vec<_>>()
    ))
}

#[cfg(test)]
#[path = "vision/tests.rs"]
mod tests;
