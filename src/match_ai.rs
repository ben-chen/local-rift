//! Between-game heuristics. The planner receives one seat's filtered match view,
//! never the opposing registration. Scouting reads only explicitly visible zones.
use crate::{
    cards::{CardDef, CardId, CardKind, Domain, cards},
    deck::CustomDeck,
    game::{Decision, Difficulty, Game},
    match_series::Registration,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scouting {
    seen: BTreeSet<CardId>,
    small_field_peak: usize,
    unit_peak: usize,
    gear_peak: usize,
}
impl Scouting {
    /// Repeated snapshots are idempotent. Tokens do not disclose their creator
    /// as a card in the opponent's deck. Facedown identities become visible at
    /// game end under CR 422.4, through the same visibility gate as the inspector.
    pub fn observe_game(&mut self, game: &Game, viewer: usize) -> Result<(), String> {
        if viewer > 1 {
            return Err("Choose one of the two players.".into());
        }
        let enemy = 1 - viewer;
        let state = &game.players[enemy];
        let small = state
            .units
            .iter()
            .filter(|u| u.lane.is_some())
            .map(|u| game.unit_might(u).map(|m| usize::from(m <= 3)))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .sum();
        // Collect before changing memory so failed numeric evaluation is atomic.
        self.small_field_peak = self.small_field_peak.max(small);
        self.unit_peak = self.unit_peak.max(state.units.len());
        self.gear_peak = self
            .gear_peak
            .max(state.gear.iter().filter(|g| !g.token).count());
        self.seen.extend(state.champion);
        self.seen
            .extend(state.discard.iter().chain(&state.banish).copied());
        self.seen
            .extend(state.units.iter().filter(|u| !u.token).map(|u| u.card_id));
        self.seen
            .extend(state.gear.iter().filter(|g| !g.token).map(|g| g.card_id));
        self.seen.extend(
            state
                .hidden
                .iter()
                .filter_map(|h| game.hidden_visible_card(viewer, enemy, h.id)),
        );
        self.seen.extend(
            game.chain
                .iter()
                .filter(|c| c.player == enemy && c.ability.is_none())
                .map(|c| c.card_id),
        );
        self.seen.extend(
            game.pending_cards()
                .iter()
                .filter(|c| c.player == enemy)
                .map(|c| c.card_id),
        );
        match &game.decision {
            Some(Decision::RevealHand {
                opponent, cards, ..
            }) if *opponent == enemy => self.seen.extend(cards),
            Some(
                Decision::Sabotage { opponent, .. }
                | Decision::RecycleRevealedHand { opponent, .. },
            ) if *opponent == enemy => self.seen.extend(&state.hand),
            Some(Decision::RevealedCard {
                player, card_id, ..
            }) if *player == enemy => {
                self.seen.insert(*card_id);
            }
            Some(Decision::RevealedDeck { player, cards, .. }) if *player == enemy => {
                self.seen.extend(cards)
            }
            _ => {}
        }
        Ok(())
    }
    /// A coordinator calls this only after a successful concession or agreed
    /// draw ends the game without the scoring engine entering Finished.
    pub fn observe_game_end(&mut self, game: &Game, viewer: usize) -> Result<(), String> {
        self.observe_game(game, viewer)?;
        self.seen
            .extend(game.players[1 - viewer].hidden.iter().map(|h| h.card_id));
        Ok(())
    }
    pub fn to_json(&self) -> Value {
        json!({"seen_cards":self.seen,"small_field_peak":self.small_field_peak,
            "unit_peak":self.unit_peak,"gear_peak":self.gear_peak})
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchPolicy {
    Keep,
    Adaptive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideboardSwap {
    pub out: CardId,
    pub inside: CardId,
    pub reason: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchPlan {
    pub deck: CustomDeck,
    pub sideboard: Vec<(CardId, u8)>,
    pub battlefield: Option<usize>,
    pub first_player: usize,
    pub swaps: Vec<SideboardSwap>,
    pub reasons: Vec<String>,
}
impl MatchPlan {
    pub fn to_json(&self) -> Value {
        json!({"cards":self.deck.cards,"chosen_champion":self.deck.chosen_champion,
            "sideboard":self.sideboard,"battlefield":self.battlefield,"first_player":self.first_player,
            "swaps":self.swaps.iter().map(|s|json!({"out":s.out,"in":s.inside,"reason":s.reason})).collect::<Vec<_>>(),
            "reasons":self.reasons})
    }
}

struct Evidence {
    gears: usize,
    spells: usize,
    damage_spells: usize,
    unit_interactions: usize,
    fury: usize,
    mind: usize,
    empower_units: usize,
    small: usize,
    units: usize,
}
impl Evidence {
    fn new(scout: &Scouting, catalog: &[CardDef]) -> Self {
        use crate::rules::OfficialEffect as Effect;
        let known: Vec<_> = scout
            .seen
            .iter()
            .filter_map(|id| catalog.get(*id))
            .collect();
        Self {
            gears: known
                .iter()
                .filter(|c| c.kind == CardKind::Gear)
                .count()
                .max(scout.gear_peak),
            spells: known.iter().filter(|c| c.kind == CardKind::Spell).count(),
            // Classify reviewed spell instructions, not text substrings or
            // unseen deck contents. Alpha and Fight explicitly deal unit damage.
            damage_spells: known
                .iter()
                .filter(|c| {
                    c.kind == CardKind::Spell
                        && matches!(
                            crate::rules::effect(c.id),
                            Effect::Damage { .. }
                                | Effect::AreaDamage { .. }
                                | Effect::ExhaustFriendlyAndDamage { .. }
                                | Effect::DamageAndGold { .. }
                                | Effect::ContextualDamage(_)
                                | Effect::ArcaneShift
                        )
                })
                .count(),
            unit_interactions: known
                .iter()
                .filter(|c| {
                    c.kind == CardKind::Spell
                        && matches!(
                            crate::rules::effect(c.id),
                            Effect::Damage { .. }
                                | Effect::Kill { .. }
                                | Effect::DisempowerThenKill { .. }
                                | Effect::Stun
                                | Effect::StunAndDrawFromHand
                                | Effect::Bounce { .. }
                                | Effect::BounceBoth
                                | Effect::MoveEnemy
                                | Effect::SwapMight
                                | Effect::Fight { .. }
                                | Effect::EmpoweredFight { .. }
                                | Effect::DamageAndGold { .. }
                                | Effect::ContextualDamage(_)
                                | Effect::AlphaStrike
                                | Effect::BanishSmallOrReturn { .. }
                        )
                })
                .count(),
            fury: known
                .iter()
                .filter(|c| {
                    c.domains.contains(&Domain::Fury)
                        && matches!(c.kind, CardKind::Unit | CardKind::Spell)
                })
                .count(),
            small: scout.small_field_peak,
            mind: known
                .iter()
                .filter(|c| c.domains.contains(&Domain::Mind))
                .count(),
            empower_units: known
                .iter()
                .filter(|c| {
                    c.kind == CardKind::Unit
                        && crate::rules::requirements(c.id)
                            .contains(&crate::rules::Mechanic::Empower)
                })
                .count(),
            units: scout.unit_peak,
        }
    }
    /// Reviewed roles in the sourced sideboards; unknown sideboard cards are kept
    /// neutral rather than assigned capabilities by a text substring guess.
    fn role(&self, id: CardId) -> Option<(i32, &'static str)> {
        match id {
            443 if self.small >= 2 => Some((
                45 + 5 * self.small.min(4) as i32,
                "Several small battlefield units were visible. Alpha Strike can divide a friendly unit's Might among them; keep a source and compare current Might before casting.",
            )),
            441 if self.damage_spells > 0 => Some((
                40 + 10 * self.damage_spells.min(3) as i32,
                "Spell damage was revealed. Unyielding Spirit can prevent spell and ability damage for the turn for both players; it does not stop unit-dealt damage or other instructions.",
            )),
            442 if self.unit_interactions > 0 => Some((
                40 + 8 * self.unit_interactions.min(3) as i32,
                "Unit-targeting interaction was revealed. Repulse needs your unit at a battlefield and an enemy spell or ability choosing it without choosing another friendly unit.",
            )),
            366 if self.gears > 0 => Some((
                35 + 10 * self.gears.min(3) as i32,
                "Non-token gear was revealed. Disarming Rake adds a unit and an optional play ability that can kill a gear; the ability allows responses.",
            )),
            433 if self.mind > 0 => Some((
                40 + 10 * self.mind.min(3) as i32,
                "Revealed Mind cards give Decree of Strength possible hand targets; it cannot recycle cards of other domains.",
            )),
            434 if self.small > 0 || self.empower_units > 0 => Some((
                40 + 10 * (self.small + self.empower_units).min(3) as i32,
                "Small units or units with Empower were revealed. Lacerate removes Empowered, then checks current Might; Flow offers another use.",
            )),
            424 | 369 if self.gears > 0 => Some((
                35 + 10 * self.gears.min(3) as i32,
                "Revealed non-token gear gives this gear answer a target.",
            )),
            20 if self.units >= 3 => Some((
                40 + 5 * self.units.min(5) as i32,
                "Several enemy units were visible; Vex can stun later unit plays while at a battlefield.",
            )),
            421 if self.fury > 0 => Some((
                35 + 10 * self.fury.min(3) as i32,
                "Revealed Fury units or spells can enable Decree of Focus's conditional protection.",
            )),
            11 if self.small > 0 => Some((
                40 + 10 * self.small.min(3) as i32,
                "Enemy units with at most 3 Might were seen at battlefields, where Gust can return them.",
            )),
            1 if self.spells >= 3 => Some((
                40 + 5 * self.spells.min(5) as i32,
                "Several spell identities were revealed; Abandon can return a spell on the chain to hand.",
            )),
            _ => None,
        }
    }
}

#[derive(Default)]
struct Shape {
    units: u32,
    early: u32,
    gear: u32,
    energy: u32,
    spells: u32,
}
fn shape(deck: &CustomDeck, catalog: &[CardDef]) -> Shape {
    let mut s = Shape::default();
    for &(id, n) in &deck.cards {
        let c = &catalog[id];
        let n = u32::from(n);
        s.energy += u32::from(c.cost) * n;
        match c.kind {
            CardKind::Unit => {
                s.units += n;
                if c.cost <= 3 {
                    s.early += n;
                }
            }
            CardKind::Gear => s.gear += n,
            CardKind::Spell => s.spells += n,
        }
    }
    s
}
fn value(id: CardId, count: u8, evidence: &Evidence, catalog: &[CardDef]) -> i32 {
    let c = &catalog[id];
    let base = match c.kind {
        CardKind::Unit => 25 + if c.cost <= 3 { 10 } else { 0 },
        CardKind::Gear => 30,
        CardKind::Spell => 25,
    };
    if let Some((role, _)) = evidence.role(id) {
        // Diminishing returns apply to narrow answers, not a deck's broad
        // three-copy staples. Multiple copies are not evidence of redundancy.
        role - i32::from(count.saturating_sub(1)) * 10
    } else if matches!(id, 1 | 11 | 421 | 433 | 434 | 441..=443) {
        15
    } else {
        base
    }
}

fn copies(deck: &CustomDeck, id: CardId) -> u8 {
    deck.cards
        .iter()
        .find(|(card, _)| *card == id)
        .map_or(0, |(_, n)| *n)
}
fn preserves_plan(original: &CustomDeck, next: &CustomDeck, limit: usize) -> bool {
    let distance: usize = next
        .cards
        .iter()
        .map(|(id, n)| usize::from(n.saturating_sub(copies(original, *id))))
        .sum();
    if distance > limit {
        return false;
    }
    // Reviewed Irelia engine: targeting/readying, movement, card draw, cheap
    // protection and the Orianna equipment payoff. These are AI preferences;
    // manual sideboarding is unrestricted beyond normal match rules.
    original.legend != 0
        || [5, 6, 7, 14, 17, 18, 425, 430]
            .iter()
            .all(|id| copies(next, *id) >= copies(original, *id))
}

/// `view` must be `MatchSeries::observe(viewer)`. The planner never receives the
/// coordinator or a Game: it cannot read an enemy pool, opening hand, or shuffle.
pub fn recommend(
    view: &Value,
    viewer: usize,
    scout: &Scouting,
    difficulty: Difficulty,
    policy: MatchPolicy,
) -> Result<MatchPlan, String> {
    if viewer > 1 {
        return Err("Choose one of the two players.".into());
    }
    let registration = Registration::from_json(&view["registration"])?;
    let current = CustomDeck::from_json(&view["deck"])?;
    let (mut deck, mut sideboard) =
        registration.configure(current.cards, current.chosen_champion)?;
    let catalog = cards();
    let evidence = Evidence::new(scout, &catalog);
    let original = shape(&registration.deck, &catalog);
    let mut swaps = Vec::new();
    let adaptive = policy == MatchPolicy::Adaptive && difficulty != Difficulty::Easy;
    let limit = if difficulty == Difficulty::Strategic {
        4
    } else {
        2
    };
    // Only enter sideboarding after a real match transition. First-game lists
    // and submitted choices are never changed by a recommendation.
    if adaptive && view["phase"] == "sideboarding" && view["sideboard_ready"][viewer] == false {
        for _ in 0..limit {
            let mut best = None;
            let mut gain = 10;
            for &(inside, _) in &sideboard {
                let Some((_, reason)) = evidence.role(inside) else {
                    continue;
                };
                let copies_in = deck
                    .cards
                    .iter()
                    .find(|(id, _)| *id == inside)
                    .map_or(0, |(_, n)| *n);
                for &(out, copies_out) in &deck.cards {
                    if out == inside || out == deck.chosen_champion {
                        continue;
                    }
                    let improvement = value(inside, copies_in + 1, &evidence, &catalog)
                        - value(out, copies_out, &evidence, &catalog);
                    if improvement <= gain {
                        continue;
                    }
                    let mut rows: BTreeMap<_, _> = deck.cards.iter().copied().collect();
                    *rows.get_mut(&out).expect("main row") -= 1;
                    *rows.entry(inside).or_default() += 1;
                    let proposed = rows.into_iter().filter(|(_, n)| *n > 0).collect();
                    let Ok((candidate, side)) =
                        registration.configure(proposed, deck.chosen_champion)
                    else {
                        continue;
                    };
                    let next = shape(&candidate, &catalog);
                    if next.units < original.units
                        || next.early < original.early
                        || next.gear < original.gear
                        || next.energy > original.energy + 4
                        || !preserves_plan(&registration.deck, &candidate, limit)
                    {
                        continue;
                    }
                    gain = improvement;
                    best = Some((
                        candidate,
                        side,
                        SideboardSwap {
                            out,
                            inside,
                            reason: reason.into(),
                        },
                    ));
                }
            }
            let Some((next, side, swap)) = best else {
                break;
            };
            deck = next;
            sideboard = side;
            swaps.push(swap);
        }
    }
    let fields = view["available_battlefields"]
        .as_array()
        .ok_or("Missing available battlefields.")?;
    let own = shape(&deck, &catalog);
    let mut battlefield = None;
    let mut field_score = i32::MIN;
    for field in fields {
        let id = field
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .ok_or("Invalid battlefield.")?;
        if !registration.battlefields.contains(&id) {
            return Err("Unregistered battlefield.".into());
        }
        // Only the own registered deck is used. Scores are heuristics, not rules.
        let score = if adaptive {
            match id {
                13 => own.spells as i32 * 3,
                14 => 25,
                0 => {
                    let printed: i32 = deck
                        .cards
                        .iter()
                        .filter(|(id, _)| catalog[*id].might >= 5)
                        .map(|(_, n)| i32::from(*n) * 4)
                        .sum();
                    // Sunken Temple checks live Might. Irelia can reach five
                    // through choosing/readying and equipment, not just print.
                    let irelia = if copies(&deck, 0) > 0 {
                        deck.cards
                            .iter()
                            .filter(|(id, _)| catalog[*id].tags.contains(&"Equipment"))
                            .map(|(_, n)| i32::from(*n) * 3)
                            .sum::<i32>()
                            + [5, 7, 8, 19, 425]
                                .iter()
                                .map(|id| i32::from(copies(&deck, *id)) * 2)
                                .sum::<i32>()
                    } else {
                        0
                    };
                    printed + irelia
                }
                6 | 8 => own.gear as i32 * 4,
                7 => (own.units + own.gear) as i32,
                4 => own.spells as i32,
                5 => own.units as i32,
                _ => 0,
            }
        } else {
            0
        };
        if score > field_score {
            field_score = score;
            battlefield = Some(id);
        }
    }
    // A deliberately explicit baseline until play/draw strength is measured.
    let first_player = viewer;
    let mut reasons = Vec::new();
    if swaps.is_empty() {
        reasons.push("Keep the current main deck: no supported swap clears the evidence and deck-balance checks.".into());
    } else {
        reasons.push("Preserve the registered unit, early-unit and gear counts while adding answers to revealed threats.".into());
    }
    reasons.push(
        if adaptive {
            "Prefer an unused battlefield that rewards the current deck's spells, units or gear."
        } else {
            "Use the next unused registered battlefield."
        }
        .into(),
    );
    reasons.push("Choose first for initiative; this policy does not yet evaluate matchup-specific play/draw advantages.".into());
    Ok(MatchPlan {
        deck,
        sideboard,
        battlefield,
        first_player,
        swaps,
        reasons,
    })
}
