//! Revision-checked match coordination; player observations never expose rivals' pools.
use super::*;
use crate::match_series::{MatchSeries, Registration, SeriesPhase, card_rows};

#[derive(Clone, Debug)]
pub(super) struct SeriesRun {
    pub state: MatchSeries,
    pub seed: u64,
    pub difficulty: Difficulty,
}
impl Session {
    pub(super) fn require_series_game(&self) -> Result<(), ApiError> {
        if self
            .series
            .as_ref()
            .is_some_and(|s| s.state.phase() != SeriesPhase::Game)
        {
            Err(ApiError::new(
                "wrong_phase",
                "Finish the match setup choices before taking game actions.",
            ))
        } else {
            Ok(())
        }
    }
    pub fn begin_series(
        &mut self,
        revision: u64,
        registrations: [Registration; 2],
        seed: u64,
        difficulty: Difficulty,
        designated_player: usize,
    ) -> Result<(), ApiError> {
        self.require_revision(revision)?;
        let state =
            MatchSeries::new(registrations, designated_player).map_err(ApiError::invalid)?;
        let next = self
            .revision
            .checked_add(1)
            .ok_or_else(|| ApiError::new("revision_exhausted", "Create a new session."))?;
        self.series = Some(SeriesRun {
            state,
            seed,
            difficulty,
        });
        self.revision = next;
        Ok(())
    }
    fn series_transition(
        &mut self,
        revision: u64,
        f: impl FnOnce(&mut MatchSeries) -> Result<(), String>,
    ) -> Result<(), ApiError> {
        self.require_revision(revision)?;
        let mut series = self
            .series
            .clone()
            .ok_or_else(|| ApiError::new("wrong_phase", "Begin a registered match first."))?;
        let old_phase = series.state.phase();
        let old_number = series.state.game_number();
        f(&mut series.state).map_err(ApiError::illegal)?;
        let game = if series.state.phase() == SeriesPhase::Game
            && (old_phase != SeriesPhase::Game || old_number != series.state.game_number())
        {
            Some(
                series
                    .state
                    .new_game(
                        series.seed.wrapping_add(series.state.game_number() - 1),
                        series.difficulty,
                    )
                    .map_err(ApiError::illegal)?,
            )
        } else {
            None
        };
        let next = self
            .revision
            .checked_add(1)
            .ok_or_else(|| ApiError::new("revision_exhausted", "Create a new session."))?;
        self.series = Some(series);
        if let Some(game) = game {
            self.game = game;
        }
        self.revision = next;
        Ok(())
    }
    pub fn series_sideboard(
        &mut self,
        player: usize,
        revision: u64,
        cards: Vec<(usize, u8)>,
        chosen_champion: usize,
    ) -> Result<(), ApiError> {
        self.series_transition(revision, |s| {
            s.submit_sideboard(player, cards, chosen_champion)
        })
    }
    pub fn series_battlefield(
        &mut self,
        player: usize,
        revision: u64,
        battlefield: usize,
    ) -> Result<(), ApiError> {
        self.series_transition(revision, |s| s.choose_battlefield(player, battlefield))
    }
    pub fn series_play_order(
        &mut self,
        player: usize,
        revision: u64,
        first_player: usize,
    ) -> Result<(), ApiError> {
        self.series_transition(revision, |s| s.choose_play_order(player, first_player))
    }
    pub fn series_concede(
        &mut self,
        player: usize,
        revision: u64,
        whole_match: bool,
    ) -> Result<(), ApiError> {
        self.series_transition(revision, |s| {
            if whole_match {
                s.concede_match(player)
            } else {
                s.concede_game(player)
            }
        })
    }
    pub fn series_draw(
        &mut self,
        player: usize,
        revision: u64,
        whole_match: bool,
        accept: bool,
    ) -> Result<(), ApiError> {
        self.series_transition(revision, |s| s.offer_draw(player, whole_match, accept))
    }
    pub(super) fn series_request(
        &mut self,
        command: &str,
        o: &Map<String, Value>,
    ) -> Result<Value, ApiError> {
        match command {
            "validate_registration" => {
                keys(o, &["command", "id", "registration"])?;
                let r = Registration::from_json(required(o, "registration")?)
                    .map_err(ApiError::invalid)?;
                return Ok(json!({"legal":true,"registration":r.to_json()}));
            }
            "series_begin" => {
                keys(
                    o,
                    &[
                        "command",
                        "id",
                        "revision",
                        "registrations",
                        "seed",
                        "difficulty",
                        "designated_player",
                    ],
                )?;
                let regs = required(o, "registrations")?
                    .as_array()
                    .filter(|a| a.len() == 2)
                    .ok_or_else(|| ApiError::invalid("Provide two match registrations."))?;
                let parse = |v| Registration::from_json(v).map_err(ApiError::invalid);
                let difficulty = match optional_string(o, "difficulty")?.unwrap_or("tactical") {
                    "easy" => Difficulty::Easy,
                    "tactical" => Difficulty::Tactical,
                    "strategic" => Difficulty::Strategic,
                    _ => {
                        return Err(ApiError::invalid(
                            "difficulty must be easy, tactical, or strategic.",
                        ));
                    }
                };
                self.begin_series(
                    unsigned(o, "revision")?,
                    [parse(&regs[0])?, parse(&regs[1])?],
                    optional_unsigned(o, "seed")?.unwrap_or(1),
                    difficulty,
                    index(o, "designated_player")?,
                )?;
            }
            "series_sideboard" => {
                keys(
                    o,
                    &[
                        "command",
                        "id",
                        "revision",
                        "player",
                        "cards",
                        "chosen_champion",
                    ],
                )?;
                self.series_sideboard(
                    player(o)?,
                    unsigned(o, "revision")?,
                    card_rows(required(o, "cards")?).map_err(ApiError::invalid)?,
                    index(o, "chosen_champion")?,
                )?;
            }
            "series_battlefield" => {
                keys(o, &["command", "id", "revision", "player", "battlefield"])?;
                self.series_battlefield(
                    player(o)?,
                    unsigned(o, "revision")?,
                    index(o, "battlefield")?,
                )?;
            }
            "series_play_order" => {
                keys(o, &["command", "id", "revision", "player", "first_player"])?;
                self.series_play_order(
                    player(o)?,
                    unsigned(o, "revision")?,
                    index(o, "first_player")?,
                )?;
            }
            "series_concede" | "series_draw" => {
                let allowed = if command == "series_draw" {
                    vec!["command", "id", "revision", "player", "scope", "accept"]
                } else {
                    vec!["command", "id", "revision", "player", "scope"]
                };
                keys(o, &allowed)?;
                let whole = match string(o, "scope")? {
                    "game" => false,
                    "match" => true,
                    _ => return Err(ApiError::invalid("scope must be game or match.")),
                };
                if command == "series_draw" {
                    let accept = required(o, "accept")?
                        .as_bool()
                        .ok_or_else(|| ApiError::invalid("accept must be true or false."))?;
                    self.series_draw(player(o)?, unsigned(o, "revision")?, whole, accept)?;
                } else {
                    self.series_concede(player(o)?, unsigned(o, "revision")?, whole)?;
                }
            }
            _ => return Err(ApiError::invalid("Unknown series command.")),
        }
        Ok(self.summary())
    }
}
