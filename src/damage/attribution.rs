//! Physical Deal and Kill attribution, retained through the parent's cleanup.
use super::*;
use crate::game::{ChainItem, ChainSource, Unit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageSource {
    /// A physical chain item, including an ability distinct from its creator.
    Chain {
        chain_id: u32,
        player: usize,
        card_id: CardId,
        ability: bool,
    },
    Object(ChainSource),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DamageParent {
    Chain {
        source: DamageSource,
        /// A definition-only Card reference is not a physical origin.
        origin: Option<ChainSource>,
    },
    Combat {
        lane: usize,
    },
    /// Direct internal fixtures and actions without a retained physical cause.
    Unattributed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DamagePacket {
    pub player: usize,
    pub target: Target,
    pub amount: DamageAmount,
    pub sources: Vec<DamageSource>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KillKind {
    Instruction,
    Cleanup,
    CombatCleanup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KillCredit {
    pub player: usize,
    pub sources: Vec<DamageSource>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KillRecord {
    pub target: Target,
    pub kind: KillKind,
    pub credits: Vec<KillCredit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DamageLedger {
    pub parent: DamageParent,
    pub damage: Vec<DamagePacket>,
    pub kills: Vec<KillRecord>,
}

impl DamageLedger {
    fn new(parent: DamageParent) -> Self {
        Self {
            parent,
            damage: vec![],
            kills: vec![],
        }
    }

    fn parent_sources(&self) -> Vec<DamageSource> {
        match self.parent {
            DamageParent::Chain { source, origin } => {
                let mut sources = vec![source];
                if let Some(origin) = origin {
                    sources.push(DamageSource::Object(origin));
                }
                sources
            }
            _ => vec![],
        }
    }
}

impl Game {
    pub fn pending_damage_ledger(&self) -> Option<&DamageLedger> {
        self.damage_ledger.as_ref()
    }

    /// The most recently completed physical parent and its cleanup, not a
    /// cumulative history. Clients may collect it alongside their action log.
    pub fn last_damage_ledger(&self) -> Option<&DamageLedger> {
        self.last_damage_ledger.as_ref()
    }

    pub(crate) fn begin_damage_parent(&mut self, item: &ChainItem) {
        let origin = self.chain_source(item);
        self.damage_ledger = Some(DamageLedger::new(DamageParent::Chain {
            source: DamageSource::Chain {
                chain_id: item.id,
                player: item.player,
                card_id: item.card_id,
                ability: item.ability.is_some(),
            },
            origin: (item.ability.is_some() && !matches!(origin, ChainSource::Card { .. }))
                .then_some(origin),
        }));
    }

    pub(crate) fn begin_combat_damage(&mut self, lane: usize) {
        self.damage_ledger = Some(DamageLedger::new(DamageParent::Combat { lane }));
    }

    pub(crate) fn finish_damage_parent(&mut self) {
        if let Some(ledger) = self.damage_ledger.take() {
            self.last_damage_ledger = Some(ledger);
        }
    }

    /// The caller has already calculated bonuses/prevention against the
    /// instruction's pre-damage state. Do not apply those modifiers twice.
    pub(crate) fn mark_dealt_damage(
        &mut self,
        context: DamageContext,
        target: Target,
        amount: DamageAmount,
    ) -> Result<(), String> {
        if amount == 0 || self.unit_ref(target).is_none() {
            return Ok(());
        }
        let (player, sources) = match context.dealer {
            DamageDealer::Unit {
                source: Target::Unit { player, unit_id },
            } => (
                player,
                vec![DamageSource::Object(ChainSource::Unit { player, unit_id })],
            ),
            DamageDealer::Unit { .. } => {
                return Err("A unit Deal requires a physical unit source.".into());
            }
            _ => (
                context.player,
                self.damage_ledger
                    .as_ref()
                    .map_or_else(Vec::new, DamageLedger::parent_sources),
            ),
        };
        self.mark_damage(target, amount)?;
        self.record_damage(DamagePacket {
            player,
            target,
            amount,
            sources,
        });
        Ok(())
    }

    pub(crate) fn mark_combat_damage(
        &mut self,
        player: usize,
        target: Target,
        amount: DamageAmount,
        sources: &[DamageSource],
    ) -> Result<(), String> {
        if amount > 0 && self.unit_ref(target).is_some() {
            self.mark_damage(target, amount)?;
            self.record_damage(DamagePacket {
                player,
                target,
                amount,
                sources: sources.to_vec(),
            });
        }
        Ok(())
    }

    fn record_damage(&mut self, packet: DamagePacket) {
        self.damage_ledger
            .get_or_insert_with(|| DamageLedger::new(DamageParent::Unattributed))
            .damage
            .push(packet);
    }

    /// Called only after replacement choices have removed saved victims.
    pub(crate) fn record_unit_kills(&mut self, dying: &[(usize, Unit)], instruction: bool) {
        for &(player, ref unit) in dying {
            self.record_kill(
                Target::Unit {
                    player,
                    unit_id: unit.id,
                },
                instruction,
            );
        }
    }

    pub(crate) fn record_kill(&mut self, target: Target, instruction: bool) {
        let ledger = self
            .damage_ledger
            .get_or_insert_with(|| DamageLedger::new(DamageParent::Unattributed));
        let combat = matches!(ledger.parent, DamageParent::Combat { .. });
        let kind = if instruction {
            KillKind::Instruction
        } else if combat {
            KillKind::CombatCleanup
        } else {
            KillKind::Cleanup
        };
        let mut credits = vec![];
        if instruction {
            if let DamageParent::Chain {
                source: DamageSource::Chain { player, .. },
                ..
            } = ledger.parent
            {
                credits.push(KillCredit {
                    player,
                    sources: ledger.parent_sources(),
                });
            }
        } else {
            // CR429.5.c: older marked damage is not credited to a later
            // non-damaging effect. Each actual Deal's controller is responsible.
            for packet in ledger
                .damage
                .iter()
                .filter(|packet| packet.target == target)
            {
                let sources = if combat {
                    packet.sources.clone()
                } else {
                    ledger.parent_sources()
                };
                if sources.is_empty() {
                    continue;
                }
                let credit = KillCredit {
                    player: packet.player,
                    sources,
                };
                if !credits.contains(&credit) {
                    credits.push(credit);
                }
            }
        }
        ledger.kills.push(KillRecord {
            target,
            kind,
            credits,
        });
    }
}
