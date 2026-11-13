//! Public physical causes only: no private deck, hand or facedown identities.
use super::*;
use crate::game::damage::{DamageLedger, DamageParent, DamageSource, KillKind};

fn source_to_json(source: &DamageSource) -> Value {
    match *source {
        DamageSource::Chain {
            chain_id,
            player,
            card_id,
            ability,
        } => json!({
            "type": if ability { "ability" } else { "spell" },
            "chain_id":chain_id,"player":player,"card_id":card_id
        }),
        DamageSource::Object(source) => chain_source_to_json(source),
    }
}

pub(super) fn to_json(ledger: &DamageLedger) -> Value {
    let parent = match &ledger.parent {
        DamageParent::Chain { source, origin } => json!({"type":"chain",
            "source":source_to_json(source),"origin":origin.map(chain_source_to_json)}),
        DamageParent::Combat { lane } => json!({"type":"combat","lane":lane}),
        DamageParent::Unattributed => json!({"type":"unattributed"}),
    };
    json!({"parent":parent,
        "damage":ledger.damage.iter().map(|packet| json!({
            "player":packet.player,"target":target_to_json(&packet.target),
            "amount":packet.amount,"amount_exact":packet.amount.to_string(),
            "sources":packet.sources.iter().map(source_to_json).collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "kills":ledger.kills.iter().map(|kill| json!({
            "target":target_to_json(&kill.target),
            "kind": match kill.kind {
                KillKind::Instruction => "instruction",
                KillKind::Cleanup => "cleanup",
                KillKind::CombatCleanup => "combat_cleanup",
            },
            "credits":kill.credits.iter().map(|credit| json!({"player":credit.player,
                "sources":credit.sources.iter().map(source_to_json).collect::<Vec<_>>()
            })).collect::<Vec<_>>()
        })).collect::<Vec<_>>()
    })
}
