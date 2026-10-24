//! Public Temporary provenance and intrinsic token identity. Creation readiness
//! and location remain instruction data on each pending token.
use super::*;

pub(super) fn status_to_json(status: crate::temporary::TemporaryStatus) -> Value {
    json!({"intrinsic":status.intrinsic,"granted":status.granted,"active":status.active})
}

pub(super) fn token_identity_to_json(card_id: crate::cards::CardId) -> Value {
    crate::rules::token_spec(card_id).map_or(Value::Null, |spec| {
        json!({"name":spec.name,"might":spec.might,
            "kind":if spec.is_gear {"gear"} else {"unit"},
            "domains":[],"tags":spec.tag.into_iter().collect::<Vec<_>>(),
            "deflect":spec.deflect,"temporary":spec.temporary})
    })
}

pub(super) fn created_token_to_json(card_id: crate::cards::CardId) -> Value {
    let mut identity = token_identity_to_json(card_id);
    if let Some(spec) = crate::rules::token_spec(card_id) {
        identity["count"] = json!(spec.count);
    }
    identity
}

#[cfg(test)]
mod tests;
