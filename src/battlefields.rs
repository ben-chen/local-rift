//! Four legal battlefield choices drawn from the included published presets.
//!
//! Text is from Riot's official Card Gallery snapshot (2026-10-24). Void Gate
//! uses the corrected Origins erratum, not the older text still shown in art:
//! https://riftbound.leagueoflegends.com/en-us/news/rules-and-releases/riftbound-origins-card-errata/
use crate::game::{ChainAbility, Decision, Game, Phase, Target};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BattlefieldDef {
    pub id: usize,
    pub name: &'static str,
    pub official_text: &'static str,
    pub source_id: &'static str,
    pub source_url: &'static str,
}

pub const BATTLEFIELDS: [BattlefieldDef; 4] = [
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
];

pub fn battlefield(id: usize) -> &'static BattlefieldDef {
    &BATTLEFIELDS[id]
}

/// Irelia, Yi, Kennen, and Kai'Sa each default to a field from their source list.
pub fn default_battlefield(deck: usize) -> usize {
    deck.min(BATTLEFIELDS.len() - 1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattlefieldAbility {
    TempleDraw { lane: usize },
    SeatDraw { lane: usize },
    MinefieldBurn { lane: usize },
}

impl BattlefieldAbility {
    pub fn lane(self) -> usize {
        match self {
            Self::TempleDraw { lane } | Self::SeatDraw { lane } | Self::MinefieldBurn { lane } => {
                lane
            }
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
    pub(crate) fn battlefield_on_conquer(&mut self, player: usize, lane: usize) {
        if self.winner.is_some() {
            return;
        }
        let id = self.battlefields[lane];
        match id {
            0 => {
                let mighty = self.players[player]
                    .units
                    .iter()
                    .any(|unit| unit.lane == Some(lane) && self.unit_might(unit) >= 5);
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
            3 => {}
            _ => unreachable!("validated battlefield selection"),
        }
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
        if self.players[player].energy < 1 {
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
        }
    }

    /// Use only for damage dealt by a spell or ability, not by a unit as an
    /// instruction's consequence (for example Rampage), and never combat damage.
    pub(crate) fn battlefield_damage(&self, target: Target, amount: u8) -> u8 {
        if amount == 0 {
            return 0;
        }
        if self
            .unit_ref(target)
            .and_then(|unit| unit.lane)
            .is_some_and(|lane| self.battlefields[lane] == 3)
        {
            amount.saturating_add(1)
        } else {
            amount
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
            buff: might,
            temporary_buff: 0,
            token: true,
            protected: false,
            stunned: false,
            banish_on_death: false,
        }
    }

    fn settle(game: &mut Game) {
        game.after_resolution();
        for _ in 0..30 {
            if game.decision.is_some() {
                let action = if let Some(Decision::CombatDamage {
                    player,
                    lane,
                    total,
                }) = game.decision
                {
                    Action::AssignDamage {
                        assignments: game.automatic_damage_assignment(player, lane, total),
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
            assert!(game.choose_battlefield(0, 4).is_err());
            game.start_match().unwrap();
            assert!(game.choose_battlefield(0, 0).is_err());
        }
    }

    #[test]
    fn temple_requires_a_mighty_conqueror_and_draw_waits_for_responses() {
        let mut game = game();
        game.battlefields[0] = 0;
        game.players[0].units.push(unit(100, Some(0), 4));
        game.battlefield_on_conquer(0, 0);
        game.after_resolution();
        assert!(game.decision.is_none());
        game.players[0].units[0].buff = 5;
        game.battlefield_on_conquer(0, 0);
        game.after_resolution();
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
        game.battlefield_on_conquer(0, 0);
        assert_eq!(game.players[0].hand.len(), hand);
        game.battlefield_owner[1] = None;
        settle(&mut game);
        assert_eq!(game.players[0].hand.len(), hand);
        game.battlefield_owner[1] = Some(0);
        game.battlefield_on_conquer(0, 0);
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
            ),
            4
        );
        assert_eq!(
            game.battlefield_damage(
                Target::Unit {
                    player: 1,
                    unit_id: 101
                },
                3
            ),
            3
        );
        assert_eq!(
            game.battlefield_damage(
                Target::Unit {
                    player: 1,
                    unit_id: 102
                },
                3
            ),
            3
        );
        assert_eq!(
            game.battlefield_damage(
                Target::Unit {
                    player: 0,
                    unit_id: 100
                },
                0
            ),
            0
        );
        assert_eq!(game.unit_might(&game.players[0].units[0]), 6);
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
