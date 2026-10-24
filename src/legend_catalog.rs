//! Reviewed legend identities, independent of the sourced tournament presets.
//! Text and artwork originate in the official card gallery; runtime behavior lives
//! in legends.rs. Registry IDs remain stable when additional legends are reviewed.

use crate::cards::{CardId, Domain};

#[derive(Clone, Copy, Debug)]
pub struct LegendDef {
    pub id: usize,
    pub official_id: &'static str,
    pub name: &'static str,
    pub champion: &'static str,
    pub domains: [Domain; 2],
    pub text: &'static str,
    pub default_champion: CardId,
    pub accent: [u8; 3],
    pub art_url: &'static str,
}

pub fn legends() -> &'static [LegendDef] {
    LEGENDS
}

pub fn legend(id: usize) -> Option<&'static LegendDef> {
    LEGENDS.get(id).filter(|entry| entry.id == id)
}

const LEGENDS: &[LegendDef] = &[
    LegendDef {
        id: 0,
        official_id: "sfd-195-221",
        name: "Blade Dancer",
        champion: "Irelia",
        domains: [Domain::Calm, Domain::Chaos],
        text: "When you choose a friendly unit, you may exhaust me and pay [Any Power] to ready it.\nWhen you conquer, you may pay [1 Energy] to ready me.",
        default_champion: 0,
        accent: [105, 226, 203],
        art_url: "https://cmsassets.rgpub.io/sanity/images/dsfx7636/game_data_live/656ef2d1724b818e9e737ec5dcce923de067a316-744x1039.png?accountingTag=RB",
    },
    LegendDef {
        id: 1,
        official_id: "ogs-019-024",
        name: "Wuju Bladesman, Starter",
        champion: "Master Yi",
        domains: [Domain::Calm, Domain::Body],
        text: "While a friendly unit defends alone, it gets +2 Might.",
        default_champion: 22,
        accent: [148, 209, 115],
        art_url: "https://cmsassets.rgpub.io/sanity/images/dsfx7636/game_data_live/8231ced23eaf22ca3bf62ec8cb86b83a3e222da6-744x1039.png?accountingTag=RB",
    },
    LegendDef {
        id: 2,
        official_id: "ven-155-166",
        name: "Heart of the Tempest",
        champion: "Kennen",
        domains: [Domain::Chaos, Domain::Order],
        text: "When you play a card from anywhere other than your hand, empower me.\n[Action][>] Disempower me, [Exhaust]: Give a unit [Assault 2] this turn. (+2 Might while it's an attacker.)",
        default_champion: 33,
        accent: [179, 144, 255],
        art_url: "https://cmsassets.rgpub.io/sanity/images/dsfx7636/game_data_live/0eab83392b310417d2630d50a3bfee3dd02b31c4-744x1039.png?accountingTag=RB",
    },
    LegendDef {
        id: 3,
        official_id: "ogn-247-298",
        name: "Daughter of the Void",
        champion: "Kai'Sa",
        domains: [Domain::Fury, Domain::Mind],
        text: "[Exhaust]: [Reaction] — [Add] [Any Power]. Use only to play spells. (Abilities that add resources can't be reacted to.)",
        default_champion: 49,
        accent: [111, 177, 255],
        art_url: "https://cmsassets.rgpub.io/sanity/images/dsfx7636/game_data_live/3ea826277086945a2cdfda4e41b771075aa8b15b-744x1039.png?accountingTag=RB",
    },
    LegendDef {
        id: 4,
        official_id: "ogn-253-298",
        name: "Hand of Noxus",
        champion: "Darius",
        domains: [Domain::Fury, Domain::Order],
        text: "[Exhaust]: [Reaction], [Legion] — [Add] [1 Energy]. (Abilities that add resources can't be reacted to. Get the effect if you've played a card this turn.)",
        default_champion: 183,
        accent: [224, 104, 99],
        art_url: "https://cmsassets.rgpub.io/sanity/images/dsfx7636/game_data_live/5b9a904048b7fb936377844ec2d31564b7fedf81-744x1039.png?accountingTag=RB",
    },
    LegendDef {
        id: 5,
        official_id: "ogn-265-298",
        name: "Herald of the Arcane",
        champion: "Viktor",
        domains: [Domain::Mind, Domain::Order],
        text: "[1 Energy], [Exhaust]: Play a 1 Might Recruit unit token.",
        default_champion: 175,
        accent: [113, 193, 182],
        art_url: "https://cmsassets.rgpub.io/sanity/images/dsfx7636/game_data_live/913236dabf1e4f71650bbba46870fff8163e2eff-744x1039.png?accountingTag=RB",
    },
    LegendDef {
        id: 6,
        official_id: "ogs-021-024",
        name: "Lady of Luminosity, Starter",
        champion: "Lux",
        domains: [Domain::Mind, Domain::Order],
        text: "When you play a spell that costs [5 Energy] or more, draw 1.",
        default_champion: 182,
        accent: [235, 214, 132],
        art_url: "https://cmsassets.rgpub.io/sanity/images/dsfx7636/game_data_live/44885d811b70621b188d9813b2b10b5cff1b81e6-744x1039.png?accountingTag=RB",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{cards, presets};
    use serde_json::Value;
    use std::collections::BTreeSet;

    #[test]
    fn registry_matches_independent_manifest_and_official_source() {
        let manifest: Value =
            serde_json::from_str(include_str!("../assets/data/legends.json")).unwrap();
        let catalog: Value =
            serde_json::from_str(include_str!("../assets/data/full_catalog.json")).unwrap();
        let main_cards = cards();
        assert_eq!(legends().len(), 7);
        assert_eq!(
            manifest["legends"].as_array().unwrap().len(),
            legends().len()
        );
        let mut official_ids = BTreeSet::new();
        for (id, def) in legends().iter().enumerate() {
            assert_eq!(def.id, id);
            assert!(official_ids.insert(def.official_id));
            assert_eq!(legend(id).unwrap().official_id, def.official_id);
            let record = &manifest["legends"][id];
            assert_eq!(record["official_id"], def.official_id);
            assert_eq!(record["name"], def.name);
            assert_eq!(record["champion"], def.champion);
            assert_eq!(record["text"], def.text);
            assert_eq!(record["art_url"], def.art_url);
            assert_eq!(record["accent"], serde_json::json!(def.accent));
            assert_eq!(record["default_champion"], def.default_champion);
            assert_eq!(
                record["default_champion_official_id"],
                main_cards[def.default_champion].official_id
            );
            let official = catalog["printings"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["official_id"] == def.official_id)
                .unwrap();
            assert_eq!(official["name"], def.name);
            assert_eq!(official["rules_text"], def.text);
            assert_eq!(official["image_url"], def.art_url);
            let domains: BTreeSet<_> = def
                .domains
                .iter()
                .map(|d| d.name().to_lowercase())
                .collect();
            let printed: BTreeSet<_> = official["domains"]
                .as_array()
                .unwrap()
                .iter()
                .map(|d| d.as_str().unwrap().to_owned())
                .collect();
            assert_eq!(domains, printed);
            assert!(
                official["tags"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v == def.champion)
            );
            let identity = catalog["cards"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| {
                    c["official_ids"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|v| v == def.official_id)
                })
                .unwrap();
            assert_eq!(identity["standard_legal"], true);
            assert_eq!(identity["implementation"]["runtime_id"], id);
            assert_eq!(identity["implementation"]["playable_in_client"], true);
        }
        assert!(legend(legends().len()).is_none());
        assert!(legend(usize::MAX).is_none());
    }

    #[test]
    fn original_preset_ids_and_choices_are_preserved() {
        let presets = presets();
        assert_eq!(presets.len(), 4);
        for (index, preset) in presets.iter().enumerate() {
            let def = legend(preset.legend_id).unwrap();
            assert_eq!(preset.legend_id, index);
            assert_eq!(def.domains, preset.domains);
            assert_eq!(def.accent, preset.accent);
            assert_eq!(def.default_champion, preset.chosen_champion);
        }
        assert_eq!(
            [
                legend(4).unwrap().default_champion,
                legend(5).unwrap().default_champion,
                legend(6).unwrap().default_champion
            ],
            [183, 175, 182]
        );
    }
}
