//! Eleven source-reviewed battlefield choices; preset defaults remain IDs 0–3.
//!
//! Text is from Riot's official Card Gallery snapshot (2026-10-24). Void Gate
//! uses the corrected Origins erratum, not the older text still shown in art:
//! https://riftbound.leagueoflegends.com/en-us/news/rules-and-releases/riftbound-origins-card-errata/
use crate::activated::AbilitySource;
use crate::cards::CardKind;
use crate::game::play::{CostContext, CostModifier, CostScope, PlayGrantKind};
use crate::game::{ChainAbility, ChainSource, Decision, Game, Phase, Target};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BattlefieldDef {
    pub id: usize,
    pub name: &'static str,
    pub official_text: &'static str,
    pub source_id: &'static str,
    pub source_url: &'static str,
}

pub const BATTLEFIELDS: [BattlefieldDef; 11] = [
    BattlefieldDef {
        id: 0,
        name: "Sunken Temple",
        official_text: "When you conquer here with one or more Mighty units, you may pay 1 energy to draw 1. (A unit is Mighty while it has 5 or more might.)",
        source_id: "sfd-218-221",
        source_url: "https://playriftbound.com/en-us/card-gallery/",
    },
    BattlefieldDef {
        id: 1,
        name: "Seat of Power",
        official_text: "When you conquer here, draw 1 for each other battlefield you or allies control.",
        source_id: "sfd-217-221",
        source_url: "https://playriftbound.com/en-us/card-gallery/",
    },
    BattlefieldDef {
        id: 2,
        name: "Minefield",
        official_text: "When you conquer here, put the top 2 cards of your Main Deck into your trash.",
        source_id: "sfd-212-221",
        source_url: "https://playriftbound.com/en-us/card-gallery/",
    },
    BattlefieldDef {
        id: 3,
        name: "Void Gate",
        official_text: "Spells and abilities deal 1 Bonus Damage to units here. (Each instance of damage the spell deals to a unit here is increased by 1.)",
        source_id: "ogn-296-298",
        source_url: "https://riftbound.leagueoflegends.com/en-us/news/rules-and-releases/riftbound-origins-card-errata/",
    },
    BattlefieldDef {
        id: 4,
        name: "The Academy",
        official_text: "When you hold here, give your next spell this turn [Repeat] equal to its base cost. (You may pay the additional cost to repeat the spell's effect.)",
        source_id: "unl-216-219",
        source_url: "https://playriftbound.com/en-us/card-gallery/",
    },
    BattlefieldDef {
        id: 5,
        name: "Firelights' Hideout",
        official_text: "When you conquer here, the next unit you play this turn has [Accelerate]. (You may pay [1 Energy] and a Power of its Domain as an additional cost to have it enter ready.)",
        source_id: "rad-162-167",
        source_url: "https://playriftbound.com/en-us/card-gallery/",
    },
    BattlefieldDef {
        id: 6,
        name: "Ornn's Forge",
        official_text: "While you control this battlefield, the first friendly non-token gear played each turn costs [1 Energy] less.",
        source_id: "sfd-213-221",
        source_url: "https://playriftbound.com/en-us/card-gallery/",
    },
    BattlefieldDef {
        id: 7,
        name: "Hunters' Circle",
        official_text: "For each player, the first unit or gear they play here on their turn costs [1 Energy] less.",
        source_id: "rad-163-167",
        source_url: "https://playriftbound.com/en-us/card-gallery/",
    },
    BattlefieldDef {
        id: 8,
        name: "Piltovan Forge",
        official_text: "While you control this battlefield, the first friendly gear activated ability played each turn costs [1 Energy] less.",
        source_id: "ven-161-166",
        source_url: "https://playriftbound.com/en-us/card-gallery/",
    },
    BattlefieldDef {
        id: 9,
        name: "Risen Altar",
        official_text: "[Empower] costs of your units here cost [1 Energy] or [Any Power] less.",
        source_id: "ven-163-166",
        source_url: "https://playriftbound.com/en-us/card-gallery/",
    },
    BattlefieldDef {
        id: 10,
        name: "Vaults of Helia",
        official_text: "When you hold here, your non-token units cost [1 Energy] more to play this turn.",
        source_id: "unl-219-219",
        source_url: "https://playriftbound.com/en-us/card-gallery/",
    },
];

pub fn battlefield(id: usize) -> &'static BattlefieldDef {
    &BATTLEFIELDS[id]
}

/// Irelia, Yi, Kennen, and Kai'Sa each default to a field from their source list.
pub fn default_battlefield(deck: usize) -> usize {
    deck.min(3)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattlefieldAbility {
    TempleDraw { lane: usize },
    SeatDraw { lane: usize },
    MinefieldBurn { lane: usize },
    Grant { lane: usize, kind: PlayGrantKind },
}

impl BattlefieldAbility {
    pub fn lane(self) -> usize {
        match self {
            Self::TempleDraw { lane }
            | Self::SeatDraw { lane }
            | Self::MinefieldBurn { lane }
            | Self::Grant { lane, .. } => lane,
        }
    }
}

impl Game {
    pub fn choose_battlefield(&mut self, player: usize, id: usize) -> Result<(), String> {
        if self.phase != Phase::Mulligan {
            return Err("Choose battlefields before the match begins.".into());
        }
        if player >= 2 || id >= BATTLEFIELDS.len() {
            return Err("Choose one of the available battlefields.".into());
        }
        self.battlefields[player] = id;
        Ok(())
    }

    /// The caller invokes this only when the battlefield scores via conquest.
    /// A final-point replacement draw still scores; repeat control changes do not.
    pub(crate) fn battlefield_on_conquer(
        &mut self,
        player: usize,
        lane: usize,
    ) -> Result<(), String> {
        if self.winner.is_some() {
            return Ok(());
        }
        let id = self.battlefields[lane];
        match id {
            0 => {
                let mut mighty = false;
                for unit in &self.players[player].units {
                    if unit.lane == Some(lane) && self.unit_might(unit)? >= 5 {
                        mighty = true;
                        break;
                    }
                }
                if mighty {
                    self.queue_trigger_choice(player, 0, Decision::Battlefield { player, lane });
                }
            }
            1 => self.queue_ability(
                player,
                0,
                Vec::new(),
                ChainAbility::Battlefield(BattlefieldAbility::SeatDraw { lane }),
            ),
            2 => self.queue_ability(
                player,
                0,
                Vec::new(),
                ChainAbility::Battlefield(BattlefieldAbility::MinefieldBurn { lane }),
            ),
            5 => self.queue_battlefield_grant(player, lane, PlayGrantKind::AccelerateNextUnit),
            3 | 4 | 6..=10 => {}
            _ => unreachable!("validated battlefield selection"),
        }

        Ok(())
    }

    /// Holding scores before channel/draw; its ability must remain respondable.
    pub(crate) fn battlefield_on_hold(&mut self, player: usize, lane: usize) {
        if self.winner.is_some() {
            return;
        }
        match self.battlefields[lane] {
            4 => self.queue_battlefield_grant(player, lane, PlayGrantKind::RepeatNextSpell),
            10 => {
                self.queue_battlefield_grant(player, lane, PlayGrantKind::UnitSurchargeThisTurn(1))
            }
            _ => {}
        }
    }

    fn queue_battlefield_grant(&mut self, player: usize, lane: usize, kind: PlayGrantKind) {
        self.queue_ability(
            player,
            0,
            Vec::new(),
            ChainAbility::Battlefield(BattlefieldAbility::Grant { lane, kind }),
        );
    }

    /// The core records completed finalization history, independently of whether
    /// this battlefield was controlled then. A Pending payment keeps its frozen
    /// cost even if another ability finalizes during its resource steps.
    pub(crate) fn battlefield_cost_modifiers(&self, context: &CostContext) -> Vec<CostModifier> {
        let mut modifiers = Vec::new();
        for (lane, &id) in self.battlefields.iter().enumerate() {
            let controlled = self.battlefield_owner[lane] == Some(context.player);
            let applies = match id {
                6 => {
                    controlled
                        && context.ability_source.is_none()
                        && !context.gear_activation
                        && context.card_kind == Some(CardKind::Gear)
                        && !context.token
                        && self.play_history.non_token_gears[context.player] == 0
                }
                7 => {
                    context.player == self.active
                        && context.ability_source.is_none()
                        && !context.gear_activation
                        && context.lane == Some(lane)
                        && matches!(context.card_kind, Some(CardKind::Unit | CardKind::Gear))
                        && self.play_history.units_or_gears_here[context.player][lane] == 0
                }
                8 => {
                    controlled
                        && context.gear_activation
                        && self.play_history.gear_activations[context.player] == 0
                }
                9 => {
                    controlled
                        && context.empower
                        && match context.ability_source {
                            Some(AbilitySource::Unit { player, unit_id })
                                if player == context.player =>
                            {
                                self.players[player]
                                    .units
                                    .iter()
                                    .any(|unit| unit.id == unit_id && unit.lane == Some(lane))
                            }
                            _ => false,
                        }
                }
                _ => false,
            };
            if applies {
                modifiers.push(CostModifier {
                    id: 6_000_000_000 + lane as u64 * 100,
                    label: battlefield(id).name.into(),
                    source: ChainSource::Battlefield {
                        lane,
                        battlefield_id: id,
                    },
                    energy: 1,
                    power: u16::from(id == 9),
                    minimum_energy: 0,
                    choose_energy_or_power: id == 9,
                    increase: false,
                    scope: CostScope::Total,
                });
            }
        }
        modifiers
    }

    pub(crate) fn resolve_battlefield_choice(
        &mut self,
        player: usize,
        lane: usize,
        accept: bool,
    ) -> Result<(), String> {
        if !accept {
            return Ok(());
        }
        if lane >= 2 || self.battlefields[lane] != 0 {
            return Err("That field has no optional Sunken Temple trigger.".into());
        }
        if self.payment_energy_available(player) < 1 {
            return Err("Sunken Temple needs 1 energy to draw a card.".into());
        }
        self.pay_energy(player, 1);
        self.queue_ability(
            player,
            0,
            Vec::new(),
            ChainAbility::Battlefield(BattlefieldAbility::TempleDraw { lane }),
        );
        self.push_log(format!(
            "{} pays 1 energy for Sunken Temple's draw ability.",
            self.players[player].name
        ));
        Ok(())
    }

    pub(crate) fn resolve_battlefield_ability(
        &mut self,
        player: usize,
        ability: BattlefieldAbility,
    ) {
        let lane = ability.lane();
        self.push_log(format!(
            "{} resolves {}'s ability.",
            self.players[player].name,
            battlefield(self.battlefields[lane]).name
        ));
        match ability {
            BattlefieldAbility::TempleDraw { .. } => self.draw(player, 1),
            BattlefieldAbility::SeatDraw { lane } => {
                let count = self
                    .battlefield_owner
                    .iter()
                    .enumerate()
                    .filter(|(other, owner)| *other != lane && **owner == Some(player))
                    .count() as u8;
                self.draw(player, count);
            }
            BattlefieldAbility::MinefieldBurn { .. } => self.burn(player, 2),
            BattlefieldAbility::Grant { lane, kind } => {
                self.grant_play(
                    player,
                    ChainSource::Battlefield {
                        lane,
                        battlefield_id: self.battlefields[lane],
                    },
                    kind,
                );
            }
        }
    }

    /// Use only for damage dealt by a spell or ability, not by a unit as an
    /// instruction's consequence (for example Rampage), and never combat damage.
    pub(crate) fn battlefield_damage(&self, target: Target, amount: u64) -> Result<u64, String> {
        if amount == 0 {
            return Ok(0);
        }
        if self
            .unit_ref(target)
            .and_then(|unit| unit.lane)
            .is_some_and(|lane| self.battlefields[lane] == 3)
        {
            crate::numeric::add_u64(amount, 1, "battlefield damage")
        } else {
            Ok(amount)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Action, Difficulty, Unit};

    fn game() -> Game {
        let mut game = Game::new(3, 3, 83, Difficulty::Easy);
        game.start_match().unwrap();
        game
    }

    fn unit(id: u32, lane: Option<usize>, might: u8) -> Unit {
        Unit {
            id,
            card_id: 25,
            lane,
            ready: true,
            damage: 0,
            buff: u64::from(might),
            temporary_buff: 0,
            token: true,
            protected: false,
            stunned: false,
            banish_on_death: false,
        }
    }

    fn settle(game: &mut Game) {
        game.after_resolution().unwrap();
        for _ in 0..30 {
            if game.decision.is_some() {
                let action = if let Some(Decision::CombatDamage {
                    player,
                    lane,
                    total,
                }) = game.decision
                {
                    Action::AssignDamage {
                        assignments: game
                            .automatic_damage_assignment(player, lane, total)
                            .unwrap(),
                    }
                } else {
                    Action::Choose { accept: false }
                };
                game.apply(action).unwrap();
            } else if game.in_window() {
                game.apply(Action::Pass).unwrap();
            } else {
                return;
            }
        }
        panic!("battlefield window did not finish");
    }

    #[test]
    fn source_defaults_and_setup_choices_are_explicit() {
        for deck in 0..4 {
            let mut game = Game::new(deck, (deck + 1) % 4, 17, Difficulty::Easy);
            assert_eq!(game.battlefields, [deck, (deck + 1) % 4]);
            assert!(!battlefield(deck).name.contains("Arena"));
            game.choose_battlefield(0, 3).unwrap();
            assert_eq!(game.battlefields[0], 3);
            assert!(game.choose_battlefield(0, BATTLEFIELDS.len()).is_err());
            game.start_match().unwrap();
            assert!(game.choose_battlefield(0, 0).is_err());
        }
    }

    #[test]
    fn temple_requires_a_mighty_conqueror_and_draw_waits_for_responses() {
        let mut game = game();
        game.battlefields[0] = 0;
        game.players[0].units.push(unit(100, Some(0), 4));
        game.battlefield_on_conquer(0, 0).unwrap();
        game.after_resolution().unwrap();
        assert!(game.decision.is_none());
        game.players[0].units[0].buff = 5;
        game.battlefield_on_conquer(0, 0).unwrap();
        game.after_resolution().unwrap();
        assert!(matches!(
            game.decision,
            Some(Decision::Battlefield { player: 0, lane: 0 })
        ));
        let hand = game.players[0].hand.len();
        let energy = game.players[0].energy;
        game.apply(Action::Choose { accept: true }).unwrap();
        assert_eq!(game.players[0].energy, energy - 1);
        assert_eq!(game.players[0].hand.len(), hand);
        assert!(game.chain.last().unwrap().ability.is_some());
        settle(&mut game);
        assert_eq!(game.players[0].hand.len(), hand + 1);
    }

    #[test]
    fn temple_decline_preserves_energy_and_hand() {
        let mut game = game();
        game.battlefields[0] = 0;
        let before = game.players[0].clone();
        game.resolve_battlefield_choice(0, 0, false).unwrap();
        assert_eq!(game.players[0], before);
        assert!(game.chain.is_empty());
    }

    #[test]
    fn seat_of_power_counts_other_fields_at_resolution() {
        let mut game = game();
        game.battlefields[0] = 1;
        game.battlefield_owner = [Some(0), Some(0)];
        let hand = game.players[0].hand.len();
        game.battlefield_on_conquer(0, 0).unwrap();
        assert_eq!(game.players[0].hand.len(), hand);
        game.battlefield_owner[1] = None;
        settle(&mut game);
        assert_eq!(game.players[0].hand.len(), hand);
        game.battlefield_owner[1] = Some(0);
        game.battlefield_on_conquer(0, 0).unwrap();
        settle(&mut game);
        assert_eq!(game.players[0].hand.len(), hand + 1);
    }

    #[test]
    fn minefield_does_not_trigger_when_reconquest_cannot_score_again() {
        let mut game = game();
        game.battlefields[0] = 2;
        game.players[0].units.push(unit(100, None, 2));
        let deck = game.players[0].deck.len();
        game.apply(Action::Move {
            unit_id: 100,
            lane: Some(0),
        })
        .unwrap();
        settle(&mut game);
        assert_eq!(game.players[0].score, 1);
        assert_eq!(game.players[0].deck.len(), deck - 2);
        game.players[0].units[0].ready = true;
        game.apply(Action::Move {
            unit_id: 100,
            lane: None,
        })
        .unwrap();
        game.players[0].units[0].ready = true;
        game.apply(Action::Move {
            unit_id: 100,
            lane: Some(0),
        })
        .unwrap();
        settle(&mut game);
        assert_eq!(game.players[0].score, 1);
        assert_eq!(game.players[0].deck.len(), deck - 2);
    }

    #[test]
    fn minefield_triggers_when_final_point_is_replaced_by_a_draw() {
        let mut game = game();
        game.battlefields[0] = 2;
        game.players[0].score = 7;
        game.players[0].units.push(unit(100, None, 2));
        let hand = game.players[0].hand.len();
        let deck = game.players[0].deck.len();
        let trash = game.players[0].discard.len();
        game.apply(Action::Move {
            unit_id: 100,
            lane: Some(0),
        })
        .unwrap();
        settle(&mut game);
        assert_eq!(game.players[0].score, 7);
        assert_eq!(game.players[0].hand.len(), hand + 1);
        assert_eq!(game.players[0].deck.len(), deck - 3);
        assert_eq!(game.players[0].discard.len(), trash + 2);
    }

    #[test]
    fn void_gate_bonus_is_target_local_and_does_not_modify_combat_might() {
        let mut game = game();
        game.battlefields = [3, 2];
        game.players[0].units.push(unit(100, Some(0), 6));
        game.players[1].units.push(unit(101, Some(1), 6));
        game.players[1].units.push(unit(102, None, 6));
        assert_eq!(
            game.battlefield_damage(
                Target::Unit {
                    player: 0,
                    unit_id: 100
                },
                3
            )
            .unwrap(),
            4
        );
        assert_eq!(
            game.battlefield_damage(
                Target::Unit {
                    player: 1,
                    unit_id: 101
                },
                3
            )
            .unwrap(),
            3
        );
        assert_eq!(
            game.battlefield_damage(
                Target::Unit {
                    player: 1,
                    unit_id: 102
                },
                3
            )
            .unwrap(),
            3
        );
        assert_eq!(
            game.battlefield_damage(
                Target::Unit {
                    player: 0,
                    unit_id: 100
                },
                0
            )
            .unwrap(),
            0
        );
        assert_eq!(game.unit_might(&game.players[0].units[0]).unwrap(), 6);
    }

    #[test]
    fn actual_spell_damage_gains_void_gate_bonus_per_packet() {
        let mut game = game();
        game.battlefields = [3, 2];
        game.players[1].units.push(unit(100, Some(0), 10));
        game.players[1].units.push(unit(101, Some(1), 10));
        game.battlefield_owner = [Some(1), Some(1)];
        game.players[0].runes = [2; 6];
        game.players[0].exhausted_runes = [0; 6];
        game.players[0].energy = 12;
        game.players[0].hand = vec![53]; // Two separate packets of 3 damage.
        game.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![
                Target::Unit {
                    player: 1,
                    unit_id: 100,
                },
                Target::Unit {
                    player: 1,
                    unit_id: 101,
                },
            ],
        })
        .unwrap();
        settle(&mut game);
        assert_eq!(game.players[1].units[0].damage, 4);
        assert_eq!(game.players[1].units[1].damage, 3);
    }
}
