//! Desktop clock and presentation boundary; the rules engine only receives a fixed date.
use crate::*;
use local_rift::{
    deck::CustomDeck,
    legality::{self, ScheduledRestriction},
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum FormatChoice {
    #[default]
    Today,
    Snapshot,
    On(String),
}

impl FormatChoice {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "today" => Ok(Self::Today),
            "snapshot" => Ok(Self::Snapshot),
            date => {
                legality::validate_format_date(date)?;
                Ok(Self::On(date.into()))
            }
        }
    }
    pub fn key(&self) -> &str {
        match self {
            Self::Today => "today",
            Self::Snapshot => "snapshot",
            Self::On(date) => date,
        }
    }
    fn label(&self) -> String {
        match self {
            Self::Today => "Standard · today".into(),
            Self::Snapshot => "Historical snapshot".into(),
            Self::On(date) => format!("Standard · {date}"),
        }
    }
    fn resolve_on(&self, today: &str) -> Result<Option<String>, String> {
        let date = match self {
            Self::Today => today,
            Self::Snapshot => return Ok(None),
            Self::On(date) => date,
        };
        legality::validate_format_date(date)?;
        Ok(Some(date.into()))
    }
    pub fn resolve(&self) -> Result<Option<String>, String> {
        // Use the player's local calendar, not UTC midnight. Resolve once when
        // accepting a game/registration so subsequent turns do not read a clock.
        self.resolve_on(&chrono::Local::now().date_naive().to_string())
    }
}

pub(super) fn game_format_label(date: Option<&str>) -> String {
    date.map_or_else(
        || {
            format!(
                "Historical practice · {} snapshot",
                legality::format_calendar().catalog_as_of
            )
        },
        |date| format!("Global Standard · {date}"),
    )
}

pub(super) struct FormatPicker {
    registered: bool,
    scroll: [usize; 2],
}

impl App {
    pub(super) fn open_format_picker(&mut self, registered: bool) {
        self.format_picker = Some(FormatPicker {
            registered,
            scroll: [0; 2],
        });
    }
    pub(super) fn format_button(&mut self, rect: Rect, registered: bool) {
        if self.ui.button(
            rect,
            &format!("{}  >", self.prefs.format.label()),
            false,
            true,
        ) {
            self.open_format_picker(registered);
        }
    }
    /// Inspect the actual chosen list/field or full registration, including its
    /// unused sideboard and fields. This same validator supplies rejection text.
    pub(super) fn setup_format_check(
        &self,
        seat: usize,
        registered: bool,
    ) -> Result<Vec<ScheduledRestriction>, String> {
        let date = self.prefs.format.resolve()?;
        if registered {
            let reg = self.match_registration(seat)?;
            match date {
                Some(date) => legality::validate_registration_at(&reg, &date).map(|v| v.upcoming),
                None => reg.validate().map(|_| Vec::new()),
            }
        } else {
            let mut deck = if self.prefs.use_custom[seat] {
                self.custom_decks[seat]
                    .clone()
                    .ok_or("Choose a saved custom list.")?
            } else {
                CustomDeck::from_preset(if seat == 0 {
                    self.player_deck
                } else {
                    self.opponent_deck
                })?
            };
            deck.battlefield = self.fields[seat];
            match date {
                Some(date) => legality::validate_deck_at(&deck, &date).map(|v| v.upcoming),
                None => deck.validate().map(|_| Vec::new()),
            }
        }
    }
    pub(super) fn draw_format_picker(&mut self) {
        let Some(mut picker) = self.format_picker.take() else {
            return;
        };
        self.ui.rect(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            Color::new(0.0, 0.02, 0.03, 0.92),
        );
        self.ui.rect(Rect::new(290.0, 145.0, 860.0, 610.0), PANEL);
        self.ui.border(Rect::new(290.0, 145.0, 860.0, 610.0), GOLD);
        // The background was drawn with input disabled; only this dialog now
        // consumes clicks. It never advances or replaces an existing game.
        self.ui.enabled = true;
        self.ui
            .title("Choose your practice format", 318.0, 185.0, 26.0, TEXT);
        let calendar = legality::format_calendar();
        self.ui.wrapped(
            &format!(
                "Global Standard 1v1 · sources reviewed {}. Regional schedules may differ.",
                calendar.reviewed_on
            ),
            318.0,
            215.0,
            800.0,
            12.0,
            MUTED,
        );
        let choices = [FormatChoice::Today, FormatChoice::Snapshot];
        for (i, choice) in choices.into_iter().enumerate() {
            if self.ui.button(
                Rect::new(318.0 + i as f32 * 268.0, 244.0, 254.0, 36.0),
                &choice.label(),
                self.prefs.format == choice,
                true,
            ) {
                self.prefs.format = choice;
                self.prefs.save();
                self.toast.clear();
                self.toast_until = 0.0;
                picker.scroll = [0; 2];
            }
        }
        if !calendar.effective_dates.is_empty() {
            let next = match &self.prefs.format {
                FormatChoice::On(date) => calendar
                    .effective_dates
                    .iter()
                    .position(|d| *d == date)
                    .map_or(0, |i| (i + 1) % calendar.effective_dates.len()),
                _ => 0,
            };
            if self.ui.button(
                Rect::new(854.0, 244.0, 268.0, 36.0),
                &format!("Announced: {}  >", calendar.effective_dates[next]),
                false,
                true,
            ) {
                self.prefs.format = FormatChoice::On(calendar.effective_dates[next].into());
                self.prefs.save();
                self.toast.clear();
                self.toast_until = 0.0;
                picker.scroll = [0; 2];
            }
        }
        let summary = match self.prefs.format.resolve() {
            Ok(Some(date)) => format!(
                "Selected: global Standard on {date}. Uses the bans in this source bundle; future dates preview announced changes. The date stays fixed for the entire match."
            ),
            Ok(None) => format!(
                "Selected: historical practice using the {} catalog. Later bans do not apply. This preserves the original tournament lists; it is not current Standard validation.",
                calendar.catalog_as_of
            ),
            Err(error) => error,
        };
        self.ui.wrapped(&summary, 318.0, 310.0, 800.0, 13.0, TEXT);
        for seat in 0..2 {
            let x = 318.0 + seat as f32 * 410.0;
            let result = self.setup_format_check(seat, picker.registered);
            self.ui.text(
                if seat == 0 {
                    "YOUR LIST"
                } else {
                    "OPPONENT LIST"
                },
                x,
                389.0,
                11.0,
                GOLD,
            );
            self.ui.text(
                if result.is_ok() {
                    "Eligible for selected format"
                } else {
                    "Changes needed before starting"
                },
                x,
                415.0,
                14.0,
                if result.is_ok() { TEAL } else { GOLD },
            );
            let detail = match result {
                Ok(changes) if !changes.is_empty() => format!(
                    "Legal now. Upcoming restrictions:\n{}",
                    changes
                        .iter()
                        .map(|c| format!("{} · {} · from {}", c.name, c.section, c.effective_on))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
                Ok(_) => {
                    if picker.registered {
                        "Main deck, sideboard, runes and all three battlefields pass the selected checks.".into()
                    } else {
                        "Main deck, runes and the selected battlefield pass the selected checks."
                            .into()
                    }
                }
                Err(error) => error,
            };
            let rect = Rect::new(x, 438.0, 386.0, 180.0);
            let wheel = self.ui.wheel_override;
            if !rect.contains(self.ui.mouse()) {
                self.ui.wheel_override = Some(0.0);
            }
            let overflow = self
                .ui
                .scroll_text(&detail, rect, 13.0, &mut picker.scroll[seat]);
            self.ui.wheel_override = wheel;
            if overflow > 0 {
                self.ui.text(
                    "Scroll here to read all restrictions.",
                    x,
                    640.0,
                    10.0,
                    MUTED,
                );
            }
        }
        self.ui.text("Your deck lists are kept as saved. Choose historical practice or edit a list to resolve restrictions.",
            318.0, 674.0, 11.0, MUTED);
        if !self.ui.button(
            Rect::new(318.0, 699.0, 804.0, 34.0),
            "Done · return to setup",
            true,
            true,
        ) {
            self.format_picker = Some(picker);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_preferences_choose_today_and_explicit_formats_roundtrip() {
        assert_eq!(
            Preferences::from_text("coach=false\n").format,
            FormatChoice::Today
        );
        for invalid in ["", "2026-02-30", "2025-01-01", "nonsense"] {
            assert_eq!(
                Preferences::from_text(&format!("format={invalid}\n")).format,
                FormatChoice::Today
            );
        }
        for key in ["today", "snapshot", "2026-11-22", "2026-11-23"] {
            let original =
                Preferences::from_text(&format!("format={key}\nadvanced=true\nwins=7\n"));
            let restored = Preferences::from_text(&original.to_text());
            assert_eq!(restored.format.key(), key);
            assert!(restored.advanced);
            assert_eq!(restored.wins, 7);
        }
    }

    #[test]
    fn today_resolves_at_acceptance_while_historical_choices_are_clock_independent() {
        assert_eq!(
            FormatChoice::Today
                .resolve_on("2026-11-22")
                .unwrap()
                .as_deref(),
            Some("2026-11-22")
        );
        assert_eq!(
            FormatChoice::Today
                .resolve_on("2026-11-23")
                .unwrap()
                .as_deref(),
            Some("2026-11-23")
        );
        assert_eq!(
            FormatChoice::Snapshot.resolve_on("bad clock").unwrap(),
            None
        );
        assert_eq!(
            FormatChoice::On("2026-11-22".into())
                .resolve_on("2026-11-23")
                .unwrap()
                .as_deref(),
            Some("2026-11-22")
        );
        assert!(FormatChoice::Today.resolve_on("bad clock").is_err());
    }
}
