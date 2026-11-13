//! Native match practice shares registration and transitions with the bot API.
use super::*;
use local_rift::{
    cards::cards,
    deck::CustomDeck,
    match_ai::{MatchPolicy, Scouting, recommend},
    match_series::{MatchSeries, Registration, SeriesPhase},
};

#[derive(Clone)]
pub(super) struct SideboardDraft {
    pub deck: CustomDeck,
    pub side: Vec<(usize, u8)>,
}
impl SideboardDraft {
    fn from_state(state: &MatchSeries, player: usize) -> Result<Self, String> {
        let view = state.observe(player)?;
        Ok(Self {
            deck: CustomDeck::from_json(&view["deck"])?,
            side: local_rift::match_series::card_rows(&view["sideboard"])?,
        })
    }
    pub(super) fn transfer(&mut self, id: usize, into_main: bool) -> Result<(), String> {
        let (from, to) = if into_main {
            (&mut self.side, &mut self.deck.cards)
        } else {
            (&mut self.deck.cards, &mut self.side)
        };
        let row = from
            .iter_mut()
            .find(|(card, n)| *card == id && *n > 0)
            .ok_or("That card is not in this pile.")?;
        row.1 -= 1;
        from.retain(|(_, n)| *n > 0);
        if let Some(row) = to.iter_mut().find(|(card, _)| *card == id) {
            row.1 += 1;
        } else {
            to.push((id, 1));
        }
        from.sort_unstable();
        to.sort_unstable();
        Ok(())
    }
    fn validate(&self, state: &MatchSeries) -> Result<(), String> {
        let mut preview = state.clone();
        preview.submit_sideboard(0, self.deck.cards.clone(), self.deck.chosen_champion)
    }
}

pub(super) struct NativeMatch {
    pub state: MatchSeries,
    seed: u64,
    difficulty: Difficulty,
    pub result_pending: bool,
    pub draft: Option<SideboardDraft>,
    pub(super) pages: [usize; 2],
    pub field_choice: Option<usize>,
    scouting: [Scouting; 2],
    sideboard_hint: Option<String>,
}
impl NativeMatch {
    fn new(
        regs: [Registration; 2],
        seed: u64,
        difficulty: Difficulty,
        chooser: usize,
    ) -> Result<Self, String> {
        let mut result = Self {
            state: MatchSeries::new(regs, chooser)?,
            seed,
            difficulty,
            result_pending: false,
            draft: None,
            pages: [0; 2],
            field_choice: None,
            scouting: std::array::from_fn(|_| Scouting::default()),
            sideboard_hint: None,
        };
        result.prepare_opponent()?;
        Ok(result)
    }
    /// The shared planner receives this seat's own registration and scouting.
    fn prepare_opponent(&mut self) -> Result<(), String> {
        let view = self.state.observe(1)?;
        if self.state.phase() == SeriesPhase::Sideboarding && view["sideboard_ready"][1] == false {
            let plan = recommend(
                &view,
                1,
                &self.scouting[1],
                self.difficulty,
                MatchPolicy::Adaptive,
            )?;
            self.state
                .submit_sideboard(1, plan.deck.cards, plan.deck.chosen_champion)?;
        }
        let view = self.state.observe(1)?;
        if self.state.phase() == SeriesPhase::Battlefields
            && view["battlefield_committed"][1] == false
        {
            let field = recommend(
                &view,
                1,
                &self.scouting[1],
                self.difficulty,
                MatchPolicy::Adaptive,
            )?
            .battlefield
            .ok_or("No available battlefield.")?;
            self.state.choose_battlefield(1, field)?;
        }
        Ok(())
    }
    fn start_game(&mut self, first: usize) -> Result<Game, String> {
        let mut candidate = self.state.clone();
        let chooser = candidate.summary()["designated_player"].as_u64().unwrap() as usize;
        if chooser == 1 && first != 1 {
            return Err("The opponent chooses to play first.".into());
        }
        candidate.choose_play_order(chooser, first)?;
        let game = candidate.new_game(
            self.seed.wrapping_add(candidate.game_number() - 1),
            self.difficulty,
        )?;
        self.state = candidate;
        self.draft = None;
        self.field_choice = None;
        Ok(game)
    }
    fn finish_game(&mut self, game: &Game) -> Result<(), String> {
        let mut candidate = self.state.clone();
        candidate.finish_completed_game(game)?;
        self.observe_game(game)?;
        self.state = candidate;
        self.result_pending = true;
        Ok(())
    }
    pub(super) fn observe_game(&mut self, game: &Game) -> Result<(), String> {
        let mut scouts = self.scouting.clone();
        for (p, scout) in scouts.iter_mut().enumerate() {
            scout.observe_game(game, p)?;
        }
        self.scouting = scouts;
        Ok(())
    }
    fn review_sideboard(&mut self) -> Result<(), String> {
        let draft = self.draft.as_ref().ok_or("No sideboard draft.")?;
        draft.validate(&self.state)?;
        let mut view = self.state.observe(0)?;
        view["deck"] = draft.deck.to_json();
        let plan = recommend(
            &view,
            0,
            &self.scouting[0],
            Difficulty::Strategic,
            MatchPolicy::Adaptive,
        )?;
        let catalog = cards();
        self.sideboard_hint = Some(if let Some(swap) = plan.swaps.first() {
            format!(
                "Consider {} for {}. {}",
                catalog[swap.inside].name, catalog[swap.out].name, swap.reason
            )
        } else {
            "No clear swap from the revealed cards. Keep your plan intact; unseen cards may still change the matchup.".into()
        });
        Ok(())
    }
    fn concede_game(&mut self, game: &Game, player: usize) -> Result<(), String> {
        let mut candidate = self.state.clone();
        candidate.concede_game(player)?;
        let mut scouts = self.scouting.clone();
        for (p, scout) in scouts.iter_mut().enumerate() {
            scout.observe_game_end(game, p)?;
        }
        self.state = candidate;
        self.scouting = scouts;
        self.result_pending = true;
        Ok(())
    }
}

impl App {
    pub(super) fn native_match_pending(&self) -> bool {
        self.series_lobby
            || self
                .native_match
                .as_ref()
                .is_some_and(|m| m.result_pending || m.state.phase() != SeriesPhase::Game)
    }
    fn match_registration(&self, seat: usize) -> Result<Registration, String> {
        if self.prefs.use_custom[seat] {
            return Err("This saved list has no registered sideboard and three-field set. Choose a complete preset below.".into());
        }
        Registration::from_preset(if seat == 0 {
            self.player_deck
        } else {
            self.opponent_deck
        })
    }
    pub(super) fn begin_native_match(
        &mut self,
        match_seed: u64,
        chooser: usize,
    ) -> Result<(), String> {
        let regs = [self.match_registration(0)?, self.match_registration(1)?];
        let series = NativeMatch::new(regs, match_seed, self.prefs.difficulty, chooser)?;
        self.clear_game_controls();
        self.native_match = Some(series);
        self.series_lobby = false;
        self.in_match = true;
        self.screen = Screen::Practice;
        self.recorded = false;
        Ok(())
    }
    pub(super) fn update_native_match(&mut self) -> Result<(), String> {
        let Some(series) = self.native_match.as_mut() else {
            return Ok(());
        };
        if series.state.phase() == SeriesPhase::Game {
            series.observe_game(&self.game)?;
        }
        if series.state.phase() == SeriesPhase::Game && self.game.winner.is_some() {
            series.finish_game(&self.game)?;
            self.ai_worker.invalidate();
            self.clear_game_controls();
            if !self.recorded {
                if self.game.winner == Some(0) {
                    self.prefs.wins += 1;
                } else {
                    self.prefs.losses += 1;
                }
                self.prefs.save();
                self.recorded = true;
            }
        } else if series.state.phase() == SeriesPhase::Game
            && self.game.phase == Phase::Mulligan
            && self.game.first_player == 1
            && !self.game.mulligans_used[1]
        {
            Self::opponent_match_mulligan(&mut self.game)?;
        }
        Ok(())
    }
    fn opponent_match_mulligan(game: &mut Game) -> Result<(), String> {
        let indices = game.players[1]
            .hand
            .iter()
            .enumerate()
            .filter(|(_, id)| game.catalog[**id].cost > 3)
            .map(|(i, _)| i)
            .take(2)
            .collect::<Vec<_>>();
        game.mulligan_player(1, &indices)
    }
    pub(super) fn finish_opening_hand(&mut self, indices: &[usize]) -> Result<(), String> {
        let mut candidate = self.game.clone();
        if self.native_match.is_some()
            && candidate.first_player == 1
            && !candidate.mulligans_used[1]
        {
            Self::opponent_match_mulligan(&mut candidate)?;
        }
        candidate.mulligan(indices)?;
        if self.native_match.is_some() && !candidate.mulligans_used[1] {
            Self::opponent_match_mulligan(&mut candidate)?;
        }
        candidate.start_match()?;
        self.ai_worker.invalidate();
        self.game = candidate;
        self.mulligan.clear();
        Ok(())
    }
    fn launch_series_game(&mut self, first: usize) -> Result<(), String> {
        let game = self
            .native_match
            .as_mut()
            .ok_or("No active match.")?
            .start_game(first)?;
        self.install_game(game);
        self.update_native_match()
    }
    fn continue_series_result(&mut self) -> Result<(), String> {
        let series = self.native_match.as_mut().ok_or("No active match.")?;
        if series.state.phase() != SeriesPhase::Sideboarding {
            return Err("The match has finished.".into());
        }
        series.prepare_opponent()?;
        series.draft = Some(SideboardDraft::from_state(&series.state, 0)?);
        series.sideboard_hint = None;
        series.pages = [0; 2];
        series.result_pending = false;
        Ok(())
    }
    fn submit_series_sideboard(&mut self) -> Result<(), String> {
        let series = self.native_match.as_mut().ok_or("No active match.")?;
        let draft = series.draft.as_ref().ok_or("No sideboard draft.")?;
        draft.validate(&series.state)?;
        series
            .state
            .submit_sideboard(0, draft.deck.cards.clone(), draft.deck.chosen_champion)?;
        series.draft = None;
        series.field_choice = None;
        series.prepare_opponent()
    }
    pub(super) fn leave_native_match(&mut self) {
        self.native_match = None;
        self.series_lobby = false;
        self.series_concede = false;
        self.in_match = false;
        self.clear_game_controls();
        self.game = Game::new(
            self.player_deck,
            self.opponent_deck,
            seed(),
            self.prefs.difficulty,
        );
    }
    pub(super) fn series_coach_tip(&self) -> Option<&str> {
        let advanced = self.companion.level() == AdviceLevel::Advanced;
        if self.series_lobby {
            return Some(if advanced {
                "Plan your sideboard around specific cards to remove as well as cards to add. A good swap should improve a matchup without weakening your main plan."
            } else {
                "A match is first to two game wins. You register 40 main cards, up to 10 sideboard cards and three battlefields before playing."
            });
        }
        let series = self.native_match.as_ref()?;
        if series.state.phase() == SeriesPhase::Sideboarding
            && !series.result_pending
            && let Some(hint) = &series.sideboard_hint
        {
            return Some(hint);
        }
        if series.result_pending || series.state.phase() == SeriesPhase::Complete {
            return Some(if advanced {
                "Review what decided the last game: early board access, interaction, or a resource bottleneck. Make the next swap address that specific problem."
            } else {
                "Each game has its own point score. Winning a game earns one match win. You win the match when you have two game wins."
            });
        }
        match (series.state.phase(), advanced) {
            (SeriesPhase::Sideboarding, false) => Some(
                "Move one card out for each card you add. Keep exactly 40 in the main deck, including your chosen champion. Your legend and runes stay fixed.",
            ),
            (SeriesPhase::Sideboarding, true) => Some(
                "Keep enough units to contest fields. More answers can help a matchup, but too many situational spells can leave you without a board.",
            ),
            (SeriesPhase::Battlefields, false) => Some(
                "Choose a battlefield privately. Both choices reveal together. After a decisive game, neither player can use their chosen field again this match.",
            ),
            (SeriesPhase::Battlefields, true) => Some(
                "Choose a field that suits your current deck and plan. Save a different useful field for the next game; you cannot reuse one after a decisive result.",
            ),
            (SeriesPhase::PlayOrder, false) => Some(
                "The designated player chooses who goes first before hands are drawn. After a game, the player who lost gets this choice.",
            ),
            (SeriesPhase::PlayOrder, true) => Some(
                "Going first gives initiative; going second channels an extra rune on your first turn. Weigh your curve and field plan before choosing.",
            ),
            _ => None,
        }
    }
    pub(super) fn draw_series_lobby(&mut self) {
        self.header(
            "A match, from start to finish.",
            "UNTIMED BEST OF THREE · Register your deck, then win two games.",
        );
        if self.ui.button(
            Rect::new(1246.0, 25.0, 164.0, 42.0),
            "Single game",
            false,
            true,
        ) {
            self.series_lobby = false;
        }
        self.ui.wrapped("Start with a complete registered preset. Its main deck, sideboard, runes and three battlefields stay together for this match.", 260.0,137.0,1100.0,15.0,TEXT);
        for seat in 0..2 {
            let x = 260.0 + seat as f32 * 578.0;
            self.ui.rect(Rect::new(x, 186.0, 552.0, 372.0), PANEL);
            self.ui.text(
                if seat == 0 {
                    "YOUR REGISTRATION"
                } else {
                    "OPPONENT REGISTRATION"
                },
                x + 20.0,
                214.0,
                11.0,
                if seat == 0 { TEAL } else { GOLD },
            );
            self.draw_legend_art(
                self.setup_legend(seat).id,
                Rect::new(x + 20.0, 238.0, 110.0, 155.0),
                false,
                WHITE,
            );
            let selected = if seat == 0 {
                self.player_deck
            } else {
                self.opponent_deck
            };
            self.ui.wrapped(
                if self.prefs.use_custom[seat] {
                    self.custom_decks[seat]
                        .as_ref()
                        .map_or("Saved custom list", |d| d.name.as_str())
                } else {
                    self.game.presets[selected].name
                },
                x + 150.0,
                264.0,
                375.0,
                20.0,
                TEXT,
            );
            match self.match_registration(seat) {
                Ok(reg) => {
                    self.ui.text(
                        &format!(
                            "40 main · {} sideboard · 12 runes",
                            reg.sideboard
                                .iter()
                                .map(|(_, n)| usize::from(*n))
                                .sum::<usize>()
                        ),
                        x + 150.0,
                        340.0,
                        13.0,
                        TEAL,
                    );
                    for (i, field) in reg.battlefields.into_iter().enumerate() {
                        let def = local_rift::battlefields::battlefield(field);
                        if self.ui.button(
                            Rect::new(x + 20.0, 411.0 + i as f32 * 38.0, 512.0, 30.0),
                            def.name,
                            false,
                            true,
                        ) {
                            self.field_inspect = Some(field);
                        }
                    }
                }
                Err(error) => {
                    self.ui.wrapped(&error, x + 150.0, 335.0, 375.0, 12.0, GOLD);
                }
            }
            let available: Vec<_> = self
                .game
                .presets
                .iter()
                .enumerate()
                .filter(|(_, preset)| preset.registration.is_some())
                .map(|(id, _)| id)
                .collect();
            if self.ui.button(
                Rect::new(x + 20.0, 522.0, 512.0, 27.0),
                "Next registered deck  >",
                false,
                !available.is_empty(),
            ) {
                let next = available
                    .iter()
                    .position(|&id| id == selected)
                    .map_or(0, |index| (index + 1) % available.len());
                self.select_preset(seat, available[next]);
            }
        }
        self.ui.text("OPPONENT", 260.0, 599.0, 11.0, MUTED);
        for (i, level) in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ]
        .into_iter()
        .enumerate()
        {
            if self.ui.button(
                Rect::new(260.0 + i as f32 * 120.0, 615.0, 110.0, 36.0),
                difficulty_label(level),
                self.prefs.difficulty == level,
                true,
            ) {
                self.prefs.difficulty = level;
                self.prefs.save();
            }
        }
        self.ui.wrapped("Tactical and Strategic opponents use revealed cards to consider sideboard swaps and choose fields for their deck. Easy keeps its list. All levels choose first when designated.",260.0,680.0,540.0,12.0,MUTED);
        self.ui
            .text("WHO GETS THE OPENING CHOICE?", 838.0, 599.0, 11.0, MUTED);
        if self.ui.button(
            Rect::new(838.0, 615.0, 552.0, 36.0),
            ["You (practice)", "Opponent (practice)", "Coin toss"][self.series_chooser],
            false,
            true,
        ) {
            self.series_chooser = (self.series_chooser + 1) % 3;
        }
        self.ui.wrapped("That player chooses first or second after fields reveal. The previous loser gets the choice in later games.",838.0,680.0,530.0,12.0,MUTED);
        let legal = self.match_registration(0).is_ok() && self.match_registration(1).is_ok();
        if self.ui.button(
            Rect::new(1030.0, 779.0, 360.0, 48.0),
            "Register & choose battlefield  >",
            true,
            legal,
        ) {
            let match_seed = seed();
            rand::srand(match_seed);
            let chooser = if self.series_chooser == 2 {
                rand::gen_range(0, 2)
            } else {
                self.series_chooser
            };
            if let Err(error) = self.begin_native_match(match_seed, chooser) {
                self.notify(error);
            }
        }
        self.ui.wrapped("Complete registrations: Xeno Irelia, Arito Irelia, Master Yi, Kennen, Rengar and Sonecon Riven. Other presets and custom lists remain available for single games.",260.0,801.0,730.0,12.0,MUTED);
    }
    pub(super) fn draw_native_match(&mut self) -> Result<(), String> {
        let series = self.native_match.as_ref().ok_or("No active match.")?;
        let view = series.state.observe(0)?;
        let result = series.result_pending || series.state.phase() == SeriesPhase::Complete;
        let title = if result {
            "Every game teaches something."
        } else {
            match series.state.phase() {
                SeriesPhase::Sideboarding => "Refine your plan.",
                SeriesPhase::Battlefields => "Choose your ground.",
                _ => "The fields are revealed.",
            }
        };
        self.header(
            title,
            &format!(
                "BEST OF THREE · YOU {} — {} OPPONENT · GAME {}",
                view["wins"][0], view["wins"][1], view["game_number"]
            ),
        );
        if self.ui.button(
            Rect::new(1246.0, 25.0, 164.0, 42.0),
            if view["complete"] == true {
                "Back to setup"
            } else {
                "Leave match"
            },
            false,
            true,
        ) {
            if view["complete"] == true {
                self.leave_native_match();
            } else {
                self.confirm_restart = true;
            }
            return Ok(());
        }
        if result {
            self.draw_series_result(&view);
            return Ok(());
        }
        match self.native_match.as_ref().unwrap().state.phase() {
            SeriesPhase::Sideboarding => self.draw_series_sideboard()?,
            SeriesPhase::Battlefields => self.draw_series_fields(&view)?,
            SeriesPhase::PlayOrder => self.draw_series_order(&view),
            _ => {}
        }
        Ok(())
    }
    fn draw_series_result(&mut self, view: &serde_json::Value) {
        let complete = view["complete"] == true;
        self.ui.title(
            if complete {
                if view["winner"] == 0 {
                    "Match won."
                } else {
                    "Match complete."
                }
            } else {
                "Game complete."
            },
            275.0,
            160.0,
            36.0,
            TEXT,
        );
        for p in 0..2 {
            let x = 275.0 + p as f32 * 565.0;
            self.ui.rect(Rect::new(x, 192.0, 540.0, 130.0), PANEL);
            self.ui.text(
                if p == 0 {
                    "YOUR GAME WINS"
                } else {
                    "OPPONENT GAME WINS"
                },
                x + 25.0,
                225.0,
                12.0,
                MUTED,
            );
            for pip in 0..2 {
                self.ui.diamond(
                    vec2(x + 45.0 + pip as f32 * 54.0, 276.0),
                    17.0,
                    if view["wins"][p].as_u64().unwrap() > pip {
                        if p == 0 { TEAL } else { GOLD }
                    } else {
                        LINE
                    },
                );
            }
        }
        self.ui.text("MATCH HISTORY", 275.0, 370.0, 12.0, GOLD);
        for (i, record) in view["history"]
            .as_array()
            .unwrap()
            .iter()
            .rev()
            .take(4)
            .enumerate()
        {
            let y = 410.0 + i as f32 * 66.0;
            let who = if record["winner"] == 0 {
                "You won"
            } else if record["winner"] == 1 {
                "Opponent won"
            } else {
                "Draw"
            };
            let scores = record["scores"]
                .as_array()
                .map(|s| format!("{} — {}", s[0], s[1]))
                .unwrap_or_else(|| "Concession / agreed result".into());
            self.ui.bold(
                &format!("Game {} · {who} · {scores}", record["number"]),
                275.0,
                y,
                16.0,
                TEXT,
            );
            let names = record["battlefields"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| {
                    v.as_u64()
                        .map(|id| local_rift::battlefields::battlefield(id as usize).name)
                        .unwrap_or("Not revealed")
                })
                .collect::<Vec<_>>();
            self.ui
                .text(&names.join("  /  "), 275.0, y + 23.0, 12.0, MUTED);
        }
        self.ui.wrapped(if complete {"The match is finished. A new match restores the original registered lists and all three battlefields."}else{"Next: exchange sideboard cards, choose an unused battlefield, then let the previous loser choose who starts."},275.0,731.0,1070.0,15.0,TEXT);
        if self.ui.button(
            Rect::new(1030.0, 795.0, 350.0, 48.0),
            if complete {
                "Back to match setup"
            } else {
                "Continue to sideboarding  >"
            },
            true,
            true,
        ) {
            if complete {
                self.leave_native_match();
                self.series_lobby = true;
            } else if let Err(error) = self.continue_series_result() {
                self.notify(error);
            }
        }
    }
    fn draw_series_fields(&mut self, view: &serde_json::Value) -> Result<(), String> {
        self.ui.wrapped("Choose one of your unused registered battlefields. Your opponent has committed privately; neither choice reveals until you confirm.",260.0,132.0,1120.0,15.0,TEXT);
        let fields = view["registration"]["battlefields"].as_array().unwrap();
        let available = view["available_battlefields"].as_array().unwrap();
        for (i, id) in fields.iter().enumerate() {
            let id = id.as_u64().unwrap() as usize;
            let field = local_rift::battlefields::battlefield(id);
            let legal = available.iter().any(|v| v.as_u64() == Some(id as u64));
            let selected = self.native_match.as_ref().unwrap().field_choice == Some(id);
            let x = 260.0 + i as f32 * 382.0;
            let rect = Rect::new(x, 191.0, 362.0, 473.0);
            self.ui.rect(rect, PANEL);
            self.ui.border(rect, if selected { TEAL } else { LINE });
            if let Some(art) = self.field_art.get(&id) {
                self.ui.image(
                    art,
                    Rect::new(x + 12.0, 203.0, 338.0, 175.0),
                    None,
                    if legal { WHITE } else { MUTED },
                );
            }
            self.ui.wrapped(
                field.name,
                x + 20.0,
                414.0,
                322.0,
                23.0,
                if legal { TEXT } else { MUTED },
            );
            self.ui
                .wrapped_lines(field.official_text, x + 20.0, 463.0, 322.0, 12.0, MUTED, 5);
            if self.ui.button(
                Rect::new(x + 20.0, 582.0, 150.0, 32.0),
                "Read full rules",
                false,
                true,
            ) {
                self.field_inspect = Some(id);
            }
            if self.ui.button(
                Rect::new(x + 190.0, 582.0, 152.0, 32.0),
                if !legal {
                    "Already used"
                } else if selected {
                    "Selected"
                } else {
                    "Select field"
                },
                selected,
                legal,
            ) {
                self.native_match.as_mut().unwrap().field_choice = Some(id);
            }
        }
        let chosen = self.native_match.as_ref().unwrap().field_choice;
        self.ui.wrapped("After a decisive game, both players retire their selected field for the rest of the match. Read the effect before committing.",260.0,711.0,1090.0,14.0,MUTED);
        if self.ui.button(
            Rect::new(1030.0, 790.0, 355.0, 48.0),
            "Confirm & reveal both fields  >",
            true,
            chosen.is_some(),
        ) {
            self.native_match
                .as_mut()
                .unwrap()
                .state
                .choose_battlefield(0, chosen.unwrap())?;
        }
        Ok(())
    }
    fn draw_series_order(&mut self, view: &serde_json::Value) {
        for p in 0..2 {
            let id = view["players"][p]["selected_battlefield"].as_u64().unwrap() as usize;
            let def = local_rift::battlefields::battlefield(id);
            let x = 270.0 + p as f32 * 570.0;
            self.ui.rect(Rect::new(x, 140.0, 540.0, 346.0), PANEL);
            self.ui.text(
                if p == 0 {
                    "YOUR FIELD"
                } else {
                    "OPPONENT FIELD"
                },
                x + 20.0,
                170.0,
                12.0,
                if p == 0 { TEAL } else { GOLD },
            );
            self.ui.title(def.name, x + 20.0, 208.0, 23.0, TEXT);
            self.ui
                .wrapped_lines(def.official_text, x + 20.0, 253.0, 490.0, 14.0, TEXT, 7);
            let champion = view["players"][p]["chosen_champion"].as_u64().unwrap() as usize;
            self.ui.text(
                &format!("Chosen champion: {}", self.game.catalog[champion].name),
                x + 20.0,
                420.0,
                12.0,
                GOLD,
            );
            if self.ui.button(
                Rect::new(x + 20.0, 439.0, 500.0, 30.0),
                "Read full battlefield rules",
                false,
                true,
            ) {
                self.field_inspect = Some(id);
            }
        }
        let yours = view["designated_player"] == 0;
        self.ui.title(
            if yours {
                "You choose who starts."
            } else {
                "Your opponent chooses to start."
            },
            275.0,
            563.0,
            30.0,
            TEXT,
        );
        self.ui.wrapped("Opening hands are drawn after this choice. The second player channels an extra rune on their first turn. Mulligans happen in turn order.",275.0,610.0,1080.0,16.0,MUTED);
        if yours {
            for (first, label) in [(0, "Play first"), (1, "Play second")] {
                if self.ui.button(
                    Rect::new(275.0 + first as f32 * 565.0, 728.0, 540.0, 53.0),
                    label,
                    first == 0,
                    true,
                ) {
                    if let Err(error) = self.launch_series_game(first) {
                        self.notify(error);
                    }
                    return;
                }
            }
        } else if self.ui.button(
            Rect::new(840.0, 728.0, 540.0, 53.0),
            "Draw opening hand  >",
            true,
            true,
        ) && let Err(error) = self.launch_series_game(1)
        {
            self.notify(error);
        }
    }
    fn draw_series_sideboard(&mut self) -> Result<(), String> {
        let series = self.native_match.as_ref().ok_or("No active match.")?;
        let draft = series.draft.as_ref().ok_or("No sideboard draft.")?.clone();
        let validity = draft.validate(&series.state);
        self.ui.wrapped("Move cards between your main deck and sideboard. Keep exactly 40 main cards, including your chosen champion. Your opponent's sideboard stays private.",260.0,132.0,1110.0,14.0,TEXT);
        let champions = local_rift::deck::eligible_champions(draft.deck.legend);
        for pile in 0..2 {
            let x = 260.0 + pile as f32 * 577.0;
            let rows = if pile == 0 {
                &draft.deck.cards
            } else {
                &draft.side
            };
            let count: usize = rows.iter().map(|(_, n)| usize::from(*n)).sum();
            self.ui.rect(Rect::new(x, 182.0, 555.0, 524.0), PANEL);
            self.ui.bold(
                &format!(
                    "{} · {count}",
                    if pile == 0 { "MAIN DECK" } else { "SIDEBOARD" }
                ),
                x + 15.0,
                211.0,
                15.0,
                if pile == 0 && count != 40 { GOLD } else { TEAL },
            );
            let page = self.native_match.as_ref().unwrap().pages[pile]
                .min(rows.len().saturating_sub(1) / 10);
            self.native_match.as_mut().unwrap().pages[pile] = page;
            for (row, &(id, n)) in rows.iter().skip(page * 10).take(10).enumerate() {
                let y = 229.0 + row as f32 * 42.0;
                let card = self.game.catalog[id];
                if let Some(Some(art)) = self.art.get(id) {
                    self.ui
                        .image(art, Rect::new(x + 12.0, y, 35.0, 38.0), None, WHITE);
                }
                let name_rect = Rect::new(x + 52.0, y, 278.0, 38.0);
                self.ui
                    .ellipsis(card.name, x + 54.0, y + 16.0, 268.0, 12.0, TEXT);
                self.ui.text(
                    &format!(
                        "×{n} · {}",
                        if draft.deck.chosen_champion == id {
                            "Chosen champion"
                        } else {
                            card.kind.name()
                        }
                    ),
                    x + 54.0,
                    y + 33.0,
                    10.0,
                    if draft.deck.chosen_champion == id {
                        GOLD
                    } else {
                        MUTED
                    },
                );
                if self.ui.hover(name_rect) {
                    self.hover_candidate = Some(id);
                }
                if self.ui.clicked(name_rect) {
                    self.inspect_printed_card(id);
                }
                if pile == 0
                    && champions.contains(&id)
                    && self.ui.button(
                        Rect::new(x + 337.0, y + 3.0, 112.0, 31.0),
                        if draft.deck.chosen_champion == id {
                            "Chosen"
                        } else {
                            "Choose"
                        },
                        draft.deck.chosen_champion == id,
                        true,
                    )
                {
                    self.native_match
                        .as_mut()
                        .unwrap()
                        .draft
                        .as_mut()
                        .unwrap()
                        .deck
                        .chosen_champion = id;
                    self.native_match.as_mut().unwrap().sideboard_hint = None;
                }
                if self.ui.button(
                    Rect::new(x + 460.0, y + 3.0, 80.0, 31.0),
                    if pile == 0 { "Out >" } else { "< In" },
                    false,
                    true,
                ) {
                    self.native_match
                        .as_mut()
                        .unwrap()
                        .draft
                        .as_mut()
                        .unwrap()
                        .transfer(id, pile == 1)?;
                    self.native_match.as_mut().unwrap().sideboard_hint = None;
                }
            }
            for (i, label) in ["<", ">"].into_iter().enumerate() {
                if self.ui.button(
                    Rect::new(x + 385.0 + i as f32 * 80.0, 662.0, 65.0, 30.0),
                    label,
                    false,
                    if i == 0 {
                        page > 0
                    } else {
                        (page + 1) * 10 < rows.len()
                    },
                ) {
                    self.native_match.as_mut().unwrap().pages[pile] =
                        if i == 0 { page - 1 } else { page + 1 };
                }
            }
            self.ui.text(
                &format!(
                    "Page {} / {} · click a card for full rules",
                    page + 1,
                    rows.len().max(1).div_ceil(10)
                ),
                x + 15.0,
                682.0,
                11.0,
                MUTED,
            );
        }
        self.ui.wrapped(validity.as_ref().err().map_or("Ready: your registered pool is intact, with a legal main deck and chosen champion.",String::as_str),260.0,739.0,1110.0,13.0,if validity.is_ok(){TEAL}else{GOLD});
        if self.ui.button(
            Rect::new(260.0, 793.0, 270.0, 46.0),
            "Undo this game's edits",
            false,
            true,
        ) {
            let series = self.native_match.as_mut().unwrap();
            series.draft = Some(SideboardDraft::from_state(&series.state, 0)?);
            series.sideboard_hint = None;
        }
        if self.ui.button(
            Rect::new(550.0, 793.0, 450.0, 46.0),
            if self.prefs.coach {
                "Mote: review my sideboard"
            } else {
                "Wake Mote & review sideboard"
            },
            false,
            validity.is_ok(),
        ) {
            self.native_match.as_mut().unwrap().review_sideboard()?;
            if !self.prefs.coach {
                self.prefs.coach = true;
                self.prefs.save();
            }
        }
        if self.ui.button(
            Rect::new(1030.0, 793.0, 357.0, 46.0),
            "Confirm sideboard & continue  >",
            true,
            validity.is_ok(),
        ) {
            self.submit_series_sideboard()?;
        }
        Ok(())
    }
    pub(super) fn draw_series_concession(&mut self) {
        if !self.series_concede {
            return;
        }
        self.ui.rect(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            Color::new(0.0, 0.015, 0.02, 0.88),
        );
        self.ui.enabled = true;
        self.ui.rect(Rect::new(480.0, 310.0, 540.0, 250.0), PANEL);
        self.ui
            .title("Concede this game?", 505.0, 356.0, 28.0, TEXT);
        self.ui.wrapped("Your opponent earns one game win. If the match continues, you can sideboard and choose who starts the next game.",505.0,397.0,486.0,14.0,MUTED);
        if self.ui.button(
            Rect::new(505.0, 484.0, 225.0, 44.0),
            "Keep playing",
            false,
            true,
        ) {
            self.series_concede = false;
        }
        if self.ui.button(
            Rect::new(750.0, 484.0, 240.0, 44.0),
            "Concede game",
            true,
            true,
        ) {
            let series = self.native_match.as_mut().unwrap();
            match series.concede_game(&self.game, 0) {
                Ok(()) => {
                    series.result_pending = true;
                    self.series_concede = false;
                    self.ai_worker.invalidate();
                    self.clear_game_controls();
                    self.recorded = true;
                    self.prefs.losses += 1;
                    self.prefs.save();
                }
                Err(error) => self.notify(error),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    fn pool(d: &SideboardDraft) -> BTreeMap<usize, u8> {
        let mut total = BTreeMap::new();
        for &(id, n) in d.deck.cards.iter().chain(&d.side) {
            *total.entry(id).or_default() += n;
        }
        total
    }
    #[test]
    fn sideboard_draft_preserves_pool_and_requires_eligible_chosen_champion() {
        let mut r = Registration::from_preset(5).unwrap();
        // Explicit authored variant solely to test a different eligible champion.
        r.sideboard.retain(|(id, _)| *id != 1);
        r.sideboard.push((271, 1));
        let mut native = NativeMatch::new([r.clone(), r], 32, Difficulty::Easy, 0).unwrap();
        native.state.concede_game(1).unwrap();
        native.prepare_opponent().unwrap();
        let mut d = SideboardDraft::from_state(&native.state, 0).unwrap();
        let original = pool(&d);
        d.transfer(432, false).unwrap();
        assert_eq!(d.deck.card_count(), 39);
        assert!(d.validate(&native.state).is_err());
        d.transfer(424, true).unwrap();
        assert_eq!(pool(&d), original);
        d.validate(&native.state).unwrap();
        d.transfer(0, false).unwrap();
        d.transfer(271, true).unwrap();
        assert!(d.validate(&native.state).is_err());
        d.deck.chosen_champion = 271;
        d.validate(&native.state).unwrap();
        assert_eq!(pool(&d), original);
        let before = pool(&d);
        assert!(d.transfer(9999, true).is_err());
        assert_eq!(pool(&d), before);
        // Validation previews never submit the live choice.
        assert_eq!(native.state.summary()["sideboard_ready"][0], false);
    }
    #[test]
    fn native_setup_keeps_fields_private_and_respects_opponent_play_order() {
        let r = Registration::from_preset(5).unwrap();
        let mut native = NativeMatch::new([r.clone(), r], 77, Difficulty::Tactical, 1).unwrap();
        let human = native.state.observe(0).unwrap();
        assert!(human["players"][1]["selected_battlefield"].is_null());
        assert_eq!(human["battlefield_committed"][1], true);
        native.state.choose_battlefield(0, 0).unwrap();
        let before = native.state.observe(0).unwrap();
        assert!(native.start_game(0).is_err());
        assert_eq!(before, native.state.observe(0).unwrap());
        let game = native.start_game(1).unwrap();
        assert_eq!(game.first_player, 1);
        assert_eq!(game.mulligans_used, [false; 2]);
        assert_eq!(game.battlefields, [0, 13]);
        assert_eq!(game.players[0].hand.len(), 4);
    }
    #[test]
    fn native_controller_completes_real_matches_at_every_level() {
        let mut games = 0;
        for (level, difficulty) in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ]
        .into_iter()
        .enumerate()
        {
            for chooser in 0..2 {
                let r = Registration::from_preset(5).unwrap();
                let mut native = NativeMatch::new(
                    [r.clone(), r],
                    910000 + level as u64 * 100 + chooser as u64,
                    difficulty,
                    chooser,
                )
                .unwrap();
                while native.state.phase() != SeriesPhase::Complete {
                    if native.state.phase() == SeriesPhase::Sideboarding {
                        native.prepare_opponent().unwrap();
                        let mut d = SideboardDraft::from_state(&native.state, 0).unwrap();
                        if native.state.game_number() == 2 {
                            d.transfer(432, false).unwrap();
                            d.transfer(424, true).unwrap();
                        }
                        native
                            .state
                            .submit_sideboard(0, d.deck.cards, d.deck.chosen_champion)
                            .unwrap();
                        native.prepare_opponent().unwrap();
                    }
                    let view = native.state.observe(0).unwrap();
                    let field = view["available_battlefields"][0].as_u64().unwrap() as usize;
                    native.state.choose_battlefield(0, field).unwrap();
                    let chooser = native.state.summary()["designated_player"]
                        .as_u64()
                        .unwrap() as usize;
                    let mut game = native.start_game(chooser).unwrap();
                    for p in [game.first_player, 1 - game.first_player] {
                        if p == 1 {
                            App::opponent_match_mulligan(&mut game).unwrap();
                        } else {
                            game.mulligan_player(0, &[]).unwrap();
                        }
                    }
                    game.start_match().unwrap();
                    for _ in 0..6000 {
                        if game.winner.is_some() {
                            break;
                        }
                        assert!(game.step_ai().unwrap());
                        native.observe_game(&game).unwrap();
                    }
                    assert!(game.winner.is_some());
                    native.finish_game(&game).unwrap();
                    games += 1;
                }
                let v = native.state.summary();
                assert_eq!(v["wins"][v["winner"].as_u64().unwrap() as usize], 2);
            }
        }
        println!("Native controller: 6 complete matches, {games} actual games.");
    }
}
