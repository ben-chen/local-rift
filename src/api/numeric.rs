//! Exact dynamic numbers for clients whose JSON numbers use binary64.
use super::*;

const MAX_EXACT_JSON_INTEGER: u64 = (1_u64 << 53) - 1;

pub(super) fn parse_damage(value: &Value) -> Result<u64, ApiError> {
    if let Some(amount) = value.as_u64() {
        return Ok(amount);
    }
    if let Some(text) = value.as_str()
        && !text.is_empty()
        && (text == "0" || !text.starts_with('0'))
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && let Ok(amount) = text.parse::<u64>()
    {
        return Ok(amount);
    }
    Err(ApiError::invalid(
        "Damage must be an unsigned 64-bit integer or canonical decimal string.",
    ))
}

pub(super) fn assignment_action(assignments: &[(u32, u64)]) -> Value {
    json!({"type":"assign_damage","assignments":assignments.iter().map(|&(id,amount)| {
        let amount = if amount > MAX_EXACT_JSON_INTEGER {
            json!(amount.to_string())
        } else {
            json!(amount)
        };
        json!([id,amount])
    }).collect::<Vec<_>>()})
}

pub(super) fn exact_assignments(assignments: Option<&[(u32, u64)]>) -> Value {
    json!(assignments.map(|values| {
        values
            .iter()
            .map(|&(id, amount)| json!([id, amount.to_string()]))
            .collect::<Vec<_>>()
    }))
}

#[cfg(test)]
mod tests;
