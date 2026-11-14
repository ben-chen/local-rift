//! A reflexive ability references the physical token from its own instruction.
//! The token is initially only a pending play (CR354.3); its eventual unit ID
//! must be linked without choosing a different same-name token (CR355.10.d).
use super::*;

#[derive(Clone, Debug)]
pub(super) struct LinkedTokenReady {
    pub trigger_id: u32,
    pub token_id: u32,
    pub unit: Option<Target>,
}

impl Game {
    pub(super) fn queue_guards(&mut self, player: usize) {
        let token_id = self.next_chain_id;
        self.queue_token_play(
            player,
            448,
            2,
            self.current_hidden_origin().map(Some),
            false,
        );
        let trigger_id = self.next_chain_id;
        self.linked_token_readies.push(LinkedTokenReady {
            trigger_id,
            token_id,
            unit: None,
        });
        self.queue_trigger_choice(
            player,
            448,
            Decision::TriggerTargets {
                player,
                card_id: 448,
                rule_index: 0,
                context: TriggerContext {
                    keyword: None,
                    source: triggers::TriggerSource {
                        player,
                        unit_id: None,
                        gear_id: None,
                    },
                    subject: None,
                    mode: None,
                    movement: None,
                    departure_location: None,
                },
            },
        );
        self.trigger_batch += 1;
    }

    pub(super) fn link_created_token(&mut self, token: &PendingToken, unit_id: u32) {
        for link in &mut self.linked_token_readies {
            if link.token_id == token.id {
                link.unit = Some(Target::Unit {
                    player: token.player,
                    unit_id,
                });
            }
        }
    }

    pub(super) fn finalize_linked_token(&mut self, trigger: &mut PendingTrigger) {
        if let Some(index) = self
            .linked_token_readies
            .iter()
            .position(|link| link.trigger_id == trigger.option.id)
        {
            let link = self.linked_token_readies.remove(index);
            let PendingTriggerEffect::Choice(Decision::TriggerTargets { context, .. }) =
                &mut trigger.effect
            else {
                unreachable!("linked token ready must retain its pending trigger")
            };
            // A refused token leaves None. A departed token keeps its old ID;
            // Ready will do nothing, and cannot find a later replacement copy.
            context.subject = link.unit;
        }
    }
}
