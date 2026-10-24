//! Constructed deck validation against the dated official inventory (CR 103).
use crate::battlefields::BATTLEFIELDS;
use crate::cards::{CardId, CardKind, cards, presets};
use crate::game::domain_index;
use crate::legend_catalog::{legend, legends};
use crate::rules::{SupportStatus, support_status};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomDeck {
    pub name: String,
    /// Stable legend registry ID, independent of preset and chosen champion.
    pub legend: usize,
    pub chosen_champion: CardId,
    pub cards: Vec<(CardId, u8)>,
    pub runes: [u8; 6],
    pub battlefield: usize,
}

#[derive(Debug)]
struct Facts {
    domains: Vec<String>,
    tags: Vec<String>,
    /// A legend's champion identity, not every creature/faction tag it carries.
    champion_tag: Option<&'static str>,
    super_types: Vec<String>,
    legal: bool,
    unique: bool,
}

struct Inventory {
    printings: HashMap<String, Facts>,
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn inventory() -> &'static Inventory {
    static INVENTORY: OnceLock<Inventory> = OnceLock::new();
    INVENTORY.get_or_init(|| {
        let data: Value = serde_json::from_str(include_str!("../assets/data/full_catalog.json"))
            .expect("validated bundled official inventory");
        let mut legal = HashMap::new();
        for card in data["cards"].as_array().expect("inventory cards") {
            let unique = strings(&card["keywords_observed"])
                .iter()
                .any(|k| k == "Unique");
            for id in strings(&card["official_ids"]) {
                legal.insert(id, (card["standard_legal"] == true, unique));
            }
        }
        let printings = data["printings"]
            .as_array()
            .expect("inventory printings")
            .iter()
            .map(|p| {
                let id = p["official_id"].as_str().unwrap().to_owned();
                let &(legal, unique) = legal.get(&id).expect("printing identity");
                // A shared faction/creature tag (for example Yordle) does not
                // qualify a different champion for this legend.
                let champion_tag = legends()
                    .iter()
                    .find(|definition| definition.official_id == id)
                    .map(|definition| definition.champion);
                let tags = strings(&p["tags"]);
                if let Some(tag) = champion_tag {
                    assert!(
                        tags.iter().any(|t| t == tag),
                        "Official legend champion tag changed"
                    );
                }
                (
                    id,
                    Facts {
                        domains: strings(&p["domains"]),
                        tags,
                        champion_tag,
                        super_types: strings(&p["super_types"]),
                        legal,
                        unique,
                    },
                )
            })
            .collect();
        Inventory { printings }
    })
}

fn legend_facts(id: usize) -> Option<&'static Facts> {
    inventory().printings.get(legend(id)?.official_id)
}

fn same_champion(card: &Facts, legend: &Facts) -> bool {
    legend
        .champion_tag
        .is_some_and(|tag| card.tags.iter().any(|t| t == tag))
}

fn compatible(card: &Facts, legend: &Facts) -> bool {
    card.legal
        && card
            .domains
            .iter()
            .filter(|domain| domain.as_str() != "colorless")
            .all(|domain| legend.domains.contains(domain))
        && (!card.super_types.iter().any(|s| s == "signature") || same_champion(card, legend))
}

/// Supported legal cards of this legend's complete domain identity. Signature
/// counts and the final deck size still need whole-deck validation.
pub fn eligible_cards(legend: usize) -> Vec<CardId> {
    let Some(legend) = legend_facts(legend).filter(|l| l.legal) else {
        return Vec::new();
    };
    cards()
        .iter()
        .filter(|card| {
            support_status(card.id) != SupportStatus::UnknownCard
                && inventory()
                    .printings
                    .get(card.official_id)
                    .is_some_and(|f| compatible(f, legend))
        })
        .map(|card| card.id)
        .collect()
}

pub fn eligible_champions(legend: usize) -> Vec<CardId> {
    let Some(facts) = legend_facts(legend) else {
        return Vec::new();
    };
    eligible_cards(legend)
        .into_iter()
        .filter(|&id| {
            let catalog = cards();
            let card = &catalog[id];
            let f = &inventory().printings[card.official_id];
            card.kind == CardKind::Unit
                && f.super_types.iter().any(|t| t == "champion")
                && same_champion(f, facts)
        })
        .collect()
}

impl CustomDeck {
    /// The same schema is used by the native saved decks and the bot API.
    pub fn to_json(&self) -> Value {
        json!({"name":self.name,"legend":self.legend,"chosen_champion":self.chosen_champion,
            "cards":self.cards,"runes":self.runes,"battlefield":self.battlefield})
    }

    pub fn from_json(value: &Value) -> Result<Self, String> {
        let object = value
            .as_object()
            .ok_or("A custom deck must be an object.")?;
        let fields = [
            "name",
            "legend",
            "chosen_champion",
            "cards",
            "runes",
            "battlefield",
        ];
        if object.keys().any(|key| !fields.contains(&key.as_str())) {
            return Err("Unknown custom deck field.".into());
        }
        let number = |value: &Value| {
            value
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| "Deck IDs and counts must be nonnegative integers.".to_string())
        };
        let byte = |value: &Value| {
            number(value).and_then(|n| {
                u8::try_from(n).map_err(|_| "A card or rune count is too large.".to_string())
            })
        };
        let rows = value["cards"]
            .as_array()
            .ok_or("Cards must be [card ID, count] pairs.")?;
        if rows.len() > cards().len() {
            return Err("Too many card rows.".into());
        }
        let cards = rows
            .iter()
            .map(|row| {
                let pair = row
                    .as_array()
                    .filter(|r| r.len() == 2)
                    .ok_or("Each card row must contain an ID and count.")?;
                Ok((number(&pair[0])?, byte(&pair[1])?))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let counts = value["runes"]
            .as_array()
            .filter(|r| r.len() == 6)
            .ok_or("Provide six rune counts in Fury, Calm, Mind, Body, Chaos, Order order.")?;
        let mut runes = [0; 6];
        for (slot, count) in runes.iter_mut().zip(counts) {
            *slot = byte(count)?;
        }
        let deck = Self {
            name: value["name"]
                .as_str()
                .ok_or("Deck name must be text.")?
                .to_owned(),
            legend: if let Some(official_id) = value["legend"].as_str() {
                legends()
                    .iter()
                    .find(|definition| definition.official_id == official_id)
                    .map(|definition| definition.id)
                    .ok_or("Choose a supported official legend ID.")?
            } else {
                number(&value["legend"])?
            },
            chosen_champion: number(&value["chosen_champion"])?,
            cards,
            runes,
            battlefield: number(&value["battlefield"])?,
        };
        deck.validate()?;
        Ok(deck)
    }

    pub fn from_preset(index: usize) -> Result<Self, String> {
        let presets = presets();
        let preset = presets
            .get(index)
            .ok_or("Choose a supported legend or preset.")?;
        let mut runes = [0; 6];
        for (domain, count) in preset.domains.into_iter().zip(preset.rune_counts) {
            runes[domain_index(domain)] = count;
        }
        Ok(Self {
            name: format!("Custom {}", preset.name),
            legend: preset.legend_id,
            chosen_champion: preset.chosen_champion,
            cards: preset.cards.clone(),
            runes,
            battlefield: index,
        })
    }

    /// Authored practice lists, not additional tournament/meta snapshots.
    /// The original four start from their existing exact preset lists. New
    /// legends use deliberately fixed recipes so catalog growth never silently
    /// changes an existing training list or a user's saved custom deck.
    pub fn from_legend(id: usize) -> Result<Self, String> {
        let definition = legend(id).ok_or("Choose a supported legend.")?;
        if let Some(index) = presets().iter().position(|preset| preset.legend_id == id) {
            let mut deck = Self::from_preset(index)?;
            deck.name = format!("{} training", definition.champion);
            deck.validate()?;
            return Ok(deck);
        }
        let (cards, split): (&[(CardId, u8)], [u8; 2]) = match id {
            // Cheap first plays enable Legion and Darius's second-card ready.
            4 => (
                &[
                    (56, 3),
                    (59, 3),
                    (96, 3),
                    (99, 3),
                    (101, 3),
                    (102, 3),
                    (143, 3),
                    (179, 3),
                    (183, 3),
                    (74, 3),
                    (85, 3),
                    (224, 2),
                    (58, 3),
                    (203, 2),
                ],
                [7, 5],
            ),
            // Recruit production plus Reactions that work with Innovator.
            5 => (
                &[
                    (175, 3),
                    (176, 3),
                    (68, 3),
                    (57, 3),
                    (147, 3),
                    (170, 3),
                    (177, 2),
                    (63, 3),
                    (52, 3),
                    (85, 3),
                    (87, 2),
                    (138, 3),
                    (126, 2),
                    (129, 2),
                    (120, 2),
                ],
                [6, 6],
            ),
            // Early defenders support the printed five-or-more Energy spells.
            6 => (
                &[
                    (182, 3),
                    (68, 3),
                    (57, 3),
                    (60, 3),
                    (109, 2),
                    (139, 2),
                    (75, 3),
                    (77, 2),
                    (79, 2),
                    (61, 2),
                    (87, 2),
                    (52, 3),
                    (63, 3),
                    (85, 3),
                    (126, 2),
                    (129, 2),
                ],
                [8, 4],
            ),
            _ => return Err("No authored training list is available for this legend.".into()),
        };
        let mut runes = [0; 6];
        for (domain, count) in definition.domains.into_iter().zip(split) {
            runes[domain_index(domain)] = count;
        }
        let facts = legend_facts(id).ok_or("Missing official legend identity.")?;
        let battlefield = BATTLEFIELDS
            .iter()
            .find(|field| {
                inventory()
                    .printings
                    .get(field.source_id)
                    .is_some_and(|field| compatible(field, facts))
            })
            .ok_or("No supported battlefield is legal for this legend.")?
            .id;
        let mut deck = Self {
            name: format!("{} training", definition.champion),
            legend: id,
            chosen_champion: definition.default_champion,
            cards: cards.to_vec(),
            runes,
            battlefield,
        };
        deck.cards.sort_unstable_by_key(|&(id, _)| id);
        deck.validate()?;
        Ok(deck)
    }

    pub fn card_count(&self) -> usize {
        self.cards
            .iter()
            .map(|(_, count)| usize::from(*count))
            .sum()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.name.chars().count() > 100
            || self.name.chars().any(char::is_control)
        {
            return Err("Give your deck a name of 1–100 characters without line breaks.".into());
        }
        let legend = legend_facts(self.legend)
            .filter(|l| l.legal)
            .ok_or("Choose one of the supported Standard-legal legends.")?;
        let catalog = cards();
        let mut ids = HashSet::new();
        let mut names = HashMap::new();
        let mut signatures = 0usize;
        for &(id, count) in &self.cards {
            let card = catalog
                .get(id)
                .ok_or("This deck contains an unavailable card ID.")?;
            if !ids.insert(id) {
                return Err(format!("Combine duplicate rows for {}.", card.name));
            }
            if count == 0 {
                return Err(format!("Remove the empty {} row.", card.name));
            }
            if support_status(id) == SupportStatus::UnknownCard {
                return Err(format!("{} is not implemented.", card.name));
            }
            let facts = inventory()
                .printings
                .get(card.official_id)
                .ok_or("Missing official card identity.")?;
            if !facts.legal {
                return Err(format!("{} is not Standard legal.", card.name));
            }
            if !compatible(facts, legend) {
                return Err(format!(
                    "{} does not match your legend's domains or Signature tag.",
                    card.name
                ));
            }
            let copies = names.entry(card.name).or_insert(0usize);
            *copies += usize::from(count);
            if *copies > if facts.unique { 1 } else { 3 } {
                return Err(format!(
                    "Too many copies of {} (limit {}).",
                    card.name,
                    if facts.unique { 1 } else { 3 }
                ));
            }
            if facts.super_types.iter().any(|t| t == "signature") {
                signatures += usize::from(count);
            }
        }
        if signatures > 3 {
            return Err("A deck may include at most 3 Signature cards in total.".into());
        }
        if self.card_count() < 40 {
            return Err(
                "Your main deck needs at least 40 cards, including the chosen champion.".into(),
            );
        }
        if !eligible_champions(self.legend).contains(&self.chosen_champion) {
            return Err("Choose a supported champion unit with your legend's champion tag.".into());
        }
        if !self
            .cards
            .iter()
            .any(|&(id, count)| id == self.chosen_champion && count > 0)
        {
            return Err("Include your chosen champion in the main deck count.".into());
        }
        if self.runes.iter().map(|&n| usize::from(n)).sum::<usize>() != 12 {
            return Err("The rune deck must contain exactly 12 runes.".into());
        }
        for (domain, count) in crate::payment::POWER_DOMAINS.into_iter().zip(self.runes) {
            if count > 0 && !legend.domains.contains(&domain.name().to_lowercase()) {
                return Err(format!(
                    "{} runes are outside your legend's domain identity.",
                    domain.name()
                ));
            }
        }
        let battlefield = BATTLEFIELDS
            .get(self.battlefield)
            .ok_or("Choose a supported battlefield.")?;
        if !inventory()
            .printings
            .get(battlefield.source_id)
            .is_some_and(|f| compatible(f, legend))
        {
            return Err("That battlefield is not legal for this legend.".into());
        }
        Ok(())
    }
}
