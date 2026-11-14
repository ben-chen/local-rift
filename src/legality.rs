//! Date-specific validation against the accepted catalog and announced global bans.
//! Dates are explicit: deterministic replays must never change with the host clock.
use crate::{
    battlefields::battlefield, cards::cards, deck::CustomDeck, match_series::Registration,
};
use serde_json::{Value, json};
use std::sync::OnceLock;

fn schedule() -> &'static Value {
    static SCHEDULE: OnceLock<Value> = OnceLock::new();
    SCHEDULE.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/data/format_updates.json"))
            .expect("reviewed format schedule must be valid JSON")
    })
}

/// Validate a full Gregorian date before lexical ISO-date comparisons.
fn date(value: &str) -> Result<&str, String> {
    let invalid = || "Use a valid format date in YYYY-MM-DD form.".to_owned();
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(i, b)| i != 4 && i != 7 && !b.is_ascii_digit())
    {
        return Err(invalid());
    }
    let year: u32 = value[..4].parse().map_err(|_| invalid())?;
    let month: usize = value[5..7].parse().map_err(|_| invalid())?;
    let day: u32 = value[8..].parse().map_err(|_| invalid())?;
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if year == 0 || !(1..=12).contains(&month) || day == 0 || day > days[month - 1] {
        return Err(invalid());
    }
    Ok(value)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledRestriction {
    pub section: &'static str,
    pub identity: String,
    pub name: String,
    pub effective_on: String,
    pub source_url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatValidation {
    pub as_of: String,
    pub upcoming: Vec<ScheduledRestriction>,
}

impl FormatValidation {
    pub fn to_json(&self) -> Value {
        json!({
            "scope": schedule()["scope"], "catalog_as_of": schedule()["catalog_as_of"],
            "reviewed_on": schedule()["reviewed_on"], "as_of": self.as_of,
            "upcoming": self.upcoming.iter().map(|change| json!({
                "section":change.section,"identity":change.identity,"name":change.name,
                "effective_on":change.effective_on,"source_url":change.source_url,
            })).collect::<Vec<_>>(),
        })
    }
}

fn validate_objects(
    as_of: &str,
    objects: Vec<(&'static str, &'static str)>,
) -> Result<FormatValidation, String> {
    let as_of = date(as_of)?;
    if as_of < schedule()["catalog_as_of"].as_str().expect("catalog date") {
        return Err("This catalog cannot validate formats before 2026-10-24.".into());
    }
    let mut upcoming = Vec::new();
    let mut banned = Vec::new();
    for update in schedule()["updates"].as_array().expect("format updates") {
        let effective = date(update["effective_on"].as_str().expect("effective date"))?;
        for card in update["banned"].as_array().expect("banned identities") {
            for &(section, printing) in &objects {
                if !card["official_ids"]
                    .as_array()
                    .expect("printing cohort")
                    .iter()
                    .any(|id| id.as_str() == Some(printing))
                {
                    continue;
                }
                let change = ScheduledRestriction {
                    section,
                    identity: card["identity"].as_str().expect("identity").into(),
                    name: card["name"].as_str().expect("card name").into(),
                    effective_on: effective.into(),
                    source_url: update["source_url"].as_str().expect("source URL").into(),
                };
                if as_of >= effective {
                    banned.push(format!(
                        "{} ({section}, banned from {effective})",
                        change.name
                    ));
                } else if !upcoming.contains(&change) {
                    upcoming.push(change);
                }
            }
        }
    }
    if !banned.is_empty() {
        return Err(format!(
            "Not legal in global Standard on {as_of}: {}.",
            banned.join("; ")
        ));
    }
    Ok(FormatValidation {
        as_of: as_of.into(),
        upcoming,
    })
}

fn main_objects(deck: &CustomDeck) -> Vec<(&'static str, &'static str)> {
    let mut objects: Vec<_> = deck
        .cards
        .iter()
        .map(|&(id, _)| ("main", cards()[id].official_id))
        .collect();
    objects.push((
        "legend",
        crate::legend_catalog::legends()[deck.legend].official_id,
    ));
    objects
}

/// Validate the supported list and selected battlefield on an explicit global date.
/// Regional release/ban schedules are not inferred from the global announcement.
pub fn validate_deck_at(deck: &CustomDeck, as_of: &str) -> Result<FormatValidation, String> {
    deck.validate()?;
    let mut objects = main_objects(deck);
    objects.push(("battlefield", battlefield(deck.battlefield).source_id));
    validate_objects(as_of, objects)
}

/// Include all registered fields and sideboard cards, not just the current main.
pub fn validate_registration_at(
    registration: &Registration,
    as_of: &str,
) -> Result<FormatValidation, String> {
    registration.validate()?;
    let mut objects = main_objects(&registration.deck);
    objects.extend(
        registration
            .sideboard
            .iter()
            .map(|&(id, _)| ("sideboard", cards()[id].official_id)),
    );
    objects.extend(
        registration
            .battlefields
            .iter()
            .map(|&id| ("battlefield", battlefield(id).source_id)),
    );
    validate_objects(as_of, objects)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_dates_reject_invalid_days_and_unreviewed_history() {
        for invalid in [
            "2026-2-01",
            "2026-02-29",
            "2026-04-31",
            "2026-00-01",
            "2026-13-01",
            "0000-01-01",
            "20é6-01-01",
        ] {
            assert!(date(invalid).is_err(), "{invalid}");
        }
        assert_eq!(date("2028-02-29").unwrap(), "2028-02-29");
        assert!(validate_deck_at(&CustomDeck::from_preset(0).unwrap(), "2026-10-23").is_err());
    }

    #[test]
    fn announced_bans_begin_on_effective_date_and_never_rewrite_a_preset() {
        for preset in [0, 5] {
            let deck = CustomDeck::from_preset(preset).unwrap();
            let before = deck.to_json();
            for day in ["2026-11-13", "2026-11-22"] {
                let report = validate_deck_at(&deck, day).unwrap();
                assert!(report.upcoming.iter().any(|c| c.identity == "sfd-048-221"));
            }
            for day in ["2026-11-23", "2026-12-01"] {
                let error = validate_deck_at(&deck, day).unwrap_err();
                assert!(error.contains("Stellacorn Herder") && error.contains("2026-11-23"));
            }
            assert_eq!(deck.to_json(), before);
        }
    }

    #[test]
    fn registration_checks_sideboard_and_every_field() {
        let registration = Registration::from_json(&json!({"preset":4})).unwrap();
        let report = validate_registration_at(&registration, "2026-11-22").unwrap();
        assert!(
            report
                .upcoming
                .iter()
                .any(|c| c.name == "Brynhir Thundersong" && c.section == "sideboard")
        );
        assert_eq!(
            report
                .upcoming
                .iter()
                .filter(|c| c.section == "battlefield")
                .count(),
            2
        );
        let error = validate_registration_at(&registration, "2026-11-23").unwrap_err();
        assert!(
            error.contains("Brynhir Thundersong")
                && error.contains("Emperor's Dais")
                && error.contains("Star Spring")
        );
    }

    #[test]
    fn restrictions_follow_printing_cohorts_without_banning_other_cards() {
        let card = &schedule()["updates"][0]["banned"][0];
        for id in card["official_ids"].as_array().unwrap() {
            assert!(validate_objects("2026-11-23", vec![("main", id.as_str().unwrap())]).is_err());
        }
        assert!(
            validate_objects("2026-11-23", vec![("main", "sfd-154-221")])
                .unwrap()
                .upcoming
                .is_empty()
        );
    }
}
