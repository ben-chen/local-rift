//! Untimed Standard best-of-three coordination (CR 488; TR 403, 404, 406, 407, 601).
//! The registered pool is immutable. Hidden setup choices are released together.
use crate::{
    battlefields::BATTLEFIELDS,
    cards::{CardId, presets},
    deck::CustomDeck,
    game::{Difficulty, Game},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Registration {
    pub deck: CustomDeck,
    pub sideboard: Vec<(CardId, u8)>,
    pub battlefields: [usize; 3],
}
impl Registration {
    pub fn from_preset(id: usize) -> Result<Self, String> {
        let list = presets();
        let source = list.get(id).and_then(|p| p.registration).ok_or("This preset has no complete match registration. Provide its sideboard and three battlefields explicitly.")?;
        let registration = Self {
            deck: CustomDeck::from_preset(id)?,
            sideboard: source.sideboard.to_vec(),
            battlefields: source
                .battlefields
                .try_into()
                .map_err(|_| "Register exactly three battlefields.")?,
        };
        registration.validate()?;
        Ok(registration)
    }
    pub fn validate(&self) -> Result<(), String> {
        self.deck.validate_sideboard(&self.sideboard)?;
        let mut names = HashSet::new();
        for &id in &self.battlefields {
            let field = BATTLEFIELDS
                .get(id)
                .ok_or("Choose a supported battlefield.")?;
            if !names.insert(field.name) {
                return Err("Register three differently named battlefields.".into());
            }
            let mut deck = self.deck.clone();
            deck.battlefield = id;
            deck.validate()?;
        }
        if !self.battlefields.contains(&self.deck.battlefield) {
            return Err("The deck's default battlefield must be in its registration.".into());
        }
        Ok(())
    }
    pub fn to_json(&self) -> Value {
        json!({"deck": self.deck.to_json(), "sideboard": self.sideboard, "battlefields": self.battlefields})
    }
    pub fn from_json(value: &Value) -> Result<Self, String> {
        let o = value
            .as_object()
            .ok_or("A registration must be an object.")?;
        if o.contains_key("preset") {
            if o.len() != 1 {
                return Err("A preset registration cannot be overridden; provide an explicit registration instead.".into());
            }
            return Self::from_preset(number(&o["preset"])?);
        }
        if o.len() != 3
            || !["deck", "sideboard", "battlefields"]
                .iter()
                .all(|k| o.contains_key(*k))
        {
            return Err("Provide only deck, sideboard and battlefields.".into());
        }
        let fields = o["battlefields"]
            .as_array()
            .filter(|v| v.len() == 3)
            .ok_or("Register exactly three battlefields.")?;
        let result = Self {
            deck: CustomDeck::from_json(&o["deck"])?,
            sideboard: card_rows(&o["sideboard"])?,
            battlefields: [
                number(&fields[0])?,
                number(&fields[1])?,
                number(&fields[2])?,
            ],
        };
        result.validate()?;
        Ok(result)
    }
    fn pool(&self) -> BTreeMap<CardId, u16> {
        let mut result = BTreeMap::new();
        for &(id, n) in self.deck.cards.iter().chain(&self.sideboard) {
            *result.entry(id).or_insert(0) += u16::from(n);
        }
        result
    }
    /// Validate a proposed main deck against this immutable registered pool.
    pub fn configure(
        &self,
        cards: Vec<(CardId, u8)>,
        chosen: CardId,
    ) -> Result<(CustomDeck, Vec<(CardId, u8)>), String> {
        let mut deck = self.deck.clone();
        deck.cards = cards;
        deck.chosen_champion = chosen;
        deck.validate()?;
        if deck.card_count() != 40 {
            return Err(
                "A match main deck must remain exactly 40 cards, including the chosen champion."
                    .into(),
            );
        }
        let mut remaining = self.pool();
        for &(id, count) in &deck.cards {
            let copies = remaining
                .get_mut(&id)
                .ok_or("Sideboarding cannot add cards outside the registered pool.")?;
            *copies = copies
                .checked_sub(u16::from(count))
                .ok_or("Sideboarding cannot add copies outside the registered pool.")?;
        }
        let sideboard: Vec<(CardId, u8)> = remaining
            .into_iter()
            .filter(|(_, n)| *n > 0)
            .map(|(id, n)| (id, u8::try_from(n).expect("validated copy limit")))
            .collect();
        deck.validate_sideboard(&sideboard)?;
        Ok((deck, sideboard))
    }
}
fn number(v: &Value) -> Result<usize, String> {
    v.as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| "IDs must be nonnegative integers.".into())
}
pub fn card_rows(v: &Value) -> Result<Vec<(CardId, u8)>, String> {
    let rows = v
        .as_array()
        .filter(|r| r.len() <= 50)
        .ok_or("Provide at most 50 card rows.")?;
    rows.iter()
        .map(|row| {
            let pair = row
                .as_array()
                .filter(|r| r.len() == 2)
                .ok_or("Each row must be [card ID, count].")?;
            Ok((
                number(&pair[0])?,
                u8::try_from(number(&pair[1])?).map_err(|_| "Card count is too large.")?,
            ))
        })
        .collect()
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeriesPhase {
    Sideboarding,
    Battlefields,
    PlayOrder,
    Game,
    Complete,
}
impl SeriesPhase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Sideboarding => "sideboarding",
            Self::Battlefields => "battlefields",
            Self::PlayOrder => "play_order",
            Self::Game => "game",
            Self::Complete => "complete",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameRecord {
    pub number: u64,
    pub winner: Option<usize>,
    pub battlefields: [Option<usize>; 2],
    pub first_player: Option<usize>,
    pub scores: Option<[u32; 2]>,
}
#[derive(Clone, Debug)]
pub struct MatchSeries {
    registrations: [Registration; 2],
    decks: [CustomDeck; 2],
    sideboards: [Vec<(CardId, u8)>; 2],
    phase: SeriesPhase,
    wins: [u8; 2],
    winner: Option<usize>,
    history: Vec<GameRecord>,
    game_number: u64,
    chooser: usize,
    first_player: Option<usize>,
    public_champions: [CardId; 2],
    ready: [bool; 2],
    fields: [Option<usize>; 2],
    used: [Vec<usize>; 2],
    draw_game: [bool; 2],
    draw_match: [bool; 2],
}
fn seat(p: usize) -> Result<(), String> {
    if p < 2 {
        Ok(())
    } else {
        Err("Choose one of the two players.".into())
    }
}
impl MatchSeries {
    pub fn new(registrations: [Registration; 2], designated_player: usize) -> Result<Self, String> {
        seat(designated_player)?;
        for r in &registrations {
            r.validate()?;
        }
        Ok(Self {
            decks: registrations.clone().map(|r| r.deck),
            sideboards: registrations.clone().map(|r| r.sideboard),
            public_champions: registrations.clone().map(|r| r.deck.chosen_champion),
            registrations,
            phase: SeriesPhase::Battlefields,
            wins: [0; 2],
            winner: None,
            history: vec![],
            game_number: 1,
            chooser: designated_player,
            first_player: None,
            ready: [false; 2],
            fields: [None; 2],
            used: [vec![], vec![]],
            draw_game: [false; 2],
            draw_match: [false; 2],
        })
    }
    pub fn phase(&self) -> SeriesPhase {
        self.phase
    }
    pub fn game_number(&self) -> u64 {
        self.game_number
    }
    pub fn clear_draw_offers(&mut self) {
        self.draw_game = [false; 2];
        self.draw_match = [false; 2];
    }
    fn require(&self, phase: SeriesPhase) -> Result<(), String> {
        if self.phase == phase {
            Ok(())
        } else {
            Err(format!("This choice requires the {} phase.", phase.name()))
        }
    }
    pub fn submit_sideboard(
        &mut self,
        player: usize,
        cards: Vec<(CardId, u8)>,
        chosen: CardId,
    ) -> Result<(), String> {
        seat(player)?;
        self.require(SeriesPhase::Sideboarding)?;
        if self.ready[player] {
            return Err("Your sideboard is already submitted.".into());
        }
        let (deck, side) = self.registrations[player].configure(cards, chosen)?;
        self.decks[player] = deck;
        self.sideboards[player] = side;
        self.ready[player] = true;
        self.clear_draw_offers();
        if self.ready == [true; 2] {
            self.public_champions = self.decks.clone().map(|d| d.chosen_champion);
            self.phase = SeriesPhase::Battlefields;
            self.ready = [false; 2];
        }
        Ok(())
    }
    pub fn choose_battlefield(&mut self, player: usize, field: usize) -> Result<(), String> {
        seat(player)?;
        self.require(SeriesPhase::Battlefields)?;
        if self.fields[player].is_some() {
            return Err("Your battlefield is already committed.".into());
        }
        if !self.registrations[player].battlefields.contains(&field)
            || self.used[player].contains(&field)
        {
            return Err("Choose an unused registered battlefield.".into());
        }
        self.fields[player] = Some(field);
        self.clear_draw_offers();
        if self.fields.iter().all(Option::is_some) {
            self.phase = SeriesPhase::PlayOrder;
        }
        Ok(())
    }
    pub fn choose_play_order(&mut self, player: usize, first_player: usize) -> Result<(), String> {
        seat(player)?;
        seat(first_player)?;
        self.require(SeriesPhase::PlayOrder)?;
        if player != self.chooser {
            return Err("Only the designated player may choose who plays first.".into());
        }
        self.first_player = Some(first_player);
        self.phase = SeriesPhase::Game;
        self.clear_draw_offers();
        Ok(())
    }
    pub fn new_game(&self, seed: u64, difficulty: Difficulty) -> Result<Game, String> {
        self.require(SeriesPhase::Game)?;
        let mut decks = self.decks.clone();
        for (i, d) in decks.iter_mut().enumerate() {
            d.battlefield = self.fields[i].ok_or("Choose both battlefields first.")?;
        }
        let mut game = Game::new_with_custom([0, 1], decks.map(Some), seed, difficulty, false)?;
        game.choose_starting_player(self.first_player.ok_or("Choose play order first.")?)?;
        Ok(game)
    }
    pub fn finish_completed_game(&mut self, game: &Game) -> Result<(), String> {
        self.require(SeriesPhase::Game)?;
        let winner = game.winner.ok_or("The game has not finished.")?;
        if game
            .battlefields
            .get(..2)
            .is_none_or(|fields| fields.iter().copied().map(Some).ne(self.fields))
            || Some(game.first_player) != self.first_player
        {
            return Err("The completed game does not match this setup.".into());
        }
        self.finish(
            Some(winner),
            Some(game.players.each_ref().map(|p| u32::from(p.score))),
        )
    }
    fn finish(&mut self, winner: Option<usize>, scores: Option<[u32; 2]>) -> Result<(), String> {
        if self.phase == SeriesPhase::Complete {
            return Err("The match is complete.".into());
        }
        if let Some(p) = winner {
            seat(p)?;
        }
        let next = self
            .game_number
            .checked_add(1)
            .ok_or("Match game counter exhausted.")?;
        let revealed = matches!(self.phase, SeriesPhase::PlayOrder | SeriesPhase::Game);
        self.history.push(GameRecord {
            number: self.game_number,
            winner,
            battlefields: if revealed { self.fields } else { [None; 2] },
            first_player: self.first_player,
            scores,
        });
        self.clear_draw_offers();
        if let Some(p) = winner {
            self.wins[p] += 1;
            if revealed {
                for i in 0..2 {
                    self.used[i].push(self.fields[i].expect("revealed field"));
                }
            }
            if self.wins[p] == 2 {
                self.winner = Some(p);
                self.phase = SeriesPhase::Complete;
                return Ok(());
            }
            self.chooser = 1 - p;
            self.first_player = None;
            self.fields = [None; 2];
            self.ready = [false; 2];
            self.phase = SeriesPhase::Sideboarding;
        }
        self.game_number = next;
        Ok(())
    }
    pub fn concede_game(&mut self, player: usize) -> Result<(), String> {
        seat(player)?;
        self.finish(Some(1 - player), None)
    }
    pub fn concede_match(&mut self, player: usize) -> Result<(), String> {
        seat(player)?;
        if self.phase == SeriesPhase::Complete {
            return Err("The match is complete.".into());
        }
        self.winner = Some(1 - player);
        self.phase = SeriesPhase::Complete;
        self.clear_draw_offers();
        Ok(())
    }
    pub fn offer_draw(
        &mut self,
        player: usize,
        whole_match: bool,
        accept: bool,
    ) -> Result<(), String> {
        seat(player)?;
        if self.phase == SeriesPhase::Complete {
            return Err("The match is complete.".into());
        }
        if !whole_match {
            self.require(SeriesPhase::Game)?;
        }
        let offers = if whole_match {
            &mut self.draw_match
        } else {
            &mut self.draw_game
        };
        offers[player] = accept;
        if *offers == [true; 2] {
            if whole_match {
                self.winner = None;
                self.phase = SeriesPhase::Complete;
                self.clear_draw_offers();
            } else {
                self.finish(None, None)?;
            }
        }
        Ok(())
    }
    pub fn summary(&self) -> Value {
        json!({"phase":self.phase.name(),"format":"best_of_three","timed":false,"game_number":self.game_number,"wins":self.wins,"winner":self.winner,"complete":self.phase==SeriesPhase::Complete,"designated_player":self.chooser,"first_player":self.first_player,"sideboard_ready":self.ready,"battlefield_committed":self.fields.map(|f|f.is_some()),"draw_game":self.draw_game,"draw_match":self.draw_match,
        "history":self.history.iter().map(|g|json!({"number":g.number,"winner":g.winner,"battlefields":g.battlefields,"first_player":g.first_player,"scores":g.scores})).collect::<Vec<_>>()})
    }
    pub fn observe(&self, viewer: usize) -> Result<Value, String> {
        seat(viewer)?;
        let revealed = self.fields.iter().all(Option::is_some)
            && matches!(
                self.phase,
                SeriesPhase::PlayOrder | SeriesPhase::Game | SeriesPhase::Complete
            );
        let mut view = self.summary();
        view["players"]=json!((0..2).map(|p|json!({"legend":self.decks[p].legend,"chosen_champion":if p==viewer{self.decks[p].chosen_champion}else{self.public_champions[p]},"sideboard_count":self.sideboards[p].iter().map(|(_,n)|usize::from(*n)).sum::<usize>(),"selected_battlefield":if p==viewer||revealed{self.fields[p]}else{None},"used_battlefields":self.used[p]})).collect::<Vec<_>>());
        view["registration"] = self.registrations[viewer].to_json();
        view["deck"] = self.decks[viewer].to_json();
        view["sideboard"] = json!(self.sideboards[viewer]);
        view["available_battlefields"] = json!(
            self.registrations[viewer]
                .battlefields
                .iter()
                .filter(|id| !self.used[viewer].contains(id))
                .collect::<Vec<_>>()
        );
        Ok(view)
    }
}
