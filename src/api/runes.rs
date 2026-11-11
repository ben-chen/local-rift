//! Public physical board rune identities; aggregate fields remain compatible.
use super::*;

pub(super) fn object_to_json(rune: &crate::runes::RuneObject) -> Value {
    json!({"id":rune.id,"domain":rune.domain.name(),"ready":rune.ready})
}

#[cfg(test)]
mod tests;
