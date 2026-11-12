//! Public classification of finalization and resolving-instruction refusal.
use super::*;

pub(super) fn kind_to_json(kind: crate::payment_plan::PaymentRefusalKind) -> Value {
    use crate::payment_plan::PaymentRefusalKind;
    json!(match kind {
        PaymentRefusalKind::Ordinary => "ordinary",
        PaymentRefusalKind::PendingCard => "pending_card",
        PaymentRefusalKind::PendingToken => "pending_token",
        PaymentRefusalKind::Trigger => "trigger",
        PaymentRefusalKind::Instruction => "instruction",
    })
}

#[cfg(test)]
mod tests;
