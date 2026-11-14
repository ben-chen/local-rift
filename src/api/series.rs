//! Revision-checked match coordination; player observations never expose rivals' pools.
use super::*;
use crate::match_ai::{MatchPolicy, Scouting, recommend};
use crate::match_series::{MatchSeries, Registration, SeriesPhase, card_rows};

#[derive(Clone, Debug)]
pub(super) struct SeriesRun {
    pub state: MatchSeries,
    pub seed: u64,
    pub difficulty: Difficulty,
    pub scouting: [Scouting; 2],
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
            scouting: std::array::from_fn(|_| Scouting::default()),
        });
        self.format_date = None;
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
        if old_phase == SeriesPhase::Game
            && (series.state.phase() != SeriesPhase::Game
                || old_number != series.state.game_number())
        {
            for (p, scout) in series.scouting.iter_mut().enumerate() {
                scout
                    .observe_game_end(&self.game, p)
                    .map_err(ApiError::illegal)?;
            }
        }
        let game = if series.state.phase() == SeriesPhase::Game
            && (old_phase != SeriesPhase::Game || old_number != series.state.game_number())
        {
            let mut game = series
                .state
                .new_game(
                    series.seed.wrapping_add(series.state.game_number() - 1),
                    series.difficulty,
                )
                .map_err(ApiError::illegal)?;
            game.choose_format_date(self.format_date.as_deref())
                .map_err(ApiError::illegal)?;
            Some(game)
        } else {
            None
        };
        let next = self
            .revision
            .checked_add(1)
            .ok_or_else(|| ApiError::new("revision_exhausted", "Create a new session."))?;
        if let Some(game) = game {
            for (p, scout) in series.scouting.iter_mut().enumerate() {
                scout.observe_game(&game, p).map_err(ApiError::illegal)?;
            }
            self.game = game;
        }
        self.series = Some(series);
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
            "series_suggest" | "series_step_ai" => {
                if command == "series_suggest" {
                    keys(o, &["command", "id", "player", "policy"])?;
                } else {
                    keys(o, &["command", "id", "revision", "player", "policy"])?;
                    self.require_revision(unsigned(o, "revision")?)?;
                }
                let p = player(o)?;
                let policy = match optional_string(o, "policy")?.unwrap_or("adaptive") {
                    "adaptive" => MatchPolicy::Adaptive,
                    "keep" => MatchPolicy::Keep,
                    _ => return Err(ApiError::invalid("policy must be adaptive or keep.")),
                };
                let run = self.series.as_ref().ok_or_else(|| {
                    ApiError::new("wrong_phase", "Begin a registered match first.")
                })?;
                let view = run.state.observe(p).map_err(ApiError::illegal)?;
                let plan = recommend(&view, p, &run.scouting[p], run.difficulty, policy)
                    .map_err(ApiError::illegal)?;
                if command == "series_suggest" {
                    return Ok(json!({"plan":plan.to_json(),"scouting":run.scouting[p].to_json()}));
                }
                let revision = unsigned(o, "revision")?;
                match run.state.phase() {
                    SeriesPhase::Sideboarding => self.series_sideboard(
                        p,
                        revision,
                        plan.deck.cards,
                        plan.deck.chosen_champion,
                    )?,
                    SeriesPhase::Battlefields => self.series_battlefield(
                        p,
                        revision,
                        plan.battlefield
                            .ok_or_else(|| ApiError::illegal("No unused battlefield."))?,
                    )?,
                    SeriesPhase::PlayOrder => {
                        self.series_play_order(p, revision, plan.first_player)?
                    }
                    _ => {
                        return Err(ApiError::new(
                            "wrong_phase",
                            "Match AI setup is available during sideboarding, battlefield selection and play-order choice.",
                        ));
                    }
                }
            }
            "validate_registration" => {
                keys(o, &["command", "id", "registration", "format_date"])?;
                let r = Registration::from_json(required(o, "registration")?)
                    .map_err(ApiError::invalid)?;
                let mut result = json!({"legal":true,"registration":r.to_json()});
                if let Some(date) = optional_string(o, "format_date")? {
                    result["format"] = crate::legality::validate_registration_at(&r, date)
                        .map_err(ApiError::invalid)?
                        .to_json();
                }
                return Ok(result);
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
                        "format_date",
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
                let registrations = [parse(&regs[0])?, parse(&regs[1])?];
                let format_date = optional_string(o, "format_date")?;
                if let Some(date) = format_date {
                    for registration in &registrations {
                        crate::legality::validate_registration_at(registration, date)
                            .map_err(ApiError::invalid)?;
                    }
                }
                self.begin_series(
                    unsigned(o, "revision")?,
                    registrations,
                    optional_unsigned(o, "seed")?.unwrap_or(1),
                    difficulty,
                    index(o, "designated_player")?,
                )?;
                self.format_date = format_date.map(str::to_owned);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hidden::HiddenCard;

    #[test]
    fn game_end_reveals_enter_scouting_only_after_a_successful_concession_or_draw() {
        for ending in ["game", "match", "draw"] {
            let mut session = Session::new(Default::default()).unwrap();
            let r = Registration::from_preset(5).unwrap();
            session
                .begin_series(0, [r.clone(), r], 1, Difficulty::Tactical, 0)
                .unwrap();
            session
                .series_battlefield(0, session.revision(), 13)
                .unwrap();
            session
                .series_battlefield(1, session.revision(), 13)
                .unwrap();
            session.series_play_order(0, session.revision(), 0).unwrap();
            session.game.players[1].hidden.push(HiddenCard {
                id: 100,
                card_id: 432,
                lane: 0,
                hidden_turn: 1,
                revealed: false,
            });
            let known = |s: &Session| s.series.as_ref().unwrap().scouting[0].to_json();
            assert_eq!(known(&session)["seen_cards"], json!([0]));
            assert!(
                session
                    .series_concede(0, session.revision() - 1, false)
                    .is_err()
            );
            assert_eq!(known(&session)["seen_cards"], json!([0]));
            if ending == "draw" {
                session
                    .series_draw(0, session.revision(), false, true)
                    .unwrap();
                assert_eq!(known(&session)["seen_cards"], json!([0]));
                session
                    .series_draw(1, session.revision(), false, true)
                    .unwrap();
                assert_eq!(session.series.as_ref().unwrap().state.game_number(), 2);
            } else {
                session
                    .series_concede(0, session.revision(), ending == "match")
                    .unwrap();
            }
            assert_eq!(known(&session)["seen_cards"], json!([0, 432]));
        }
    }
}
