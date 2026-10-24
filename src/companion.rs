//! Mote gives contextual coaching from the human's hand and public board only.
//! Advice explains current choices without interrupting play.
//! Basic rules checked against Riot's 2026-10-09 Core Rules, sections 315, 421,
//! and 469-473: https://playriftbound.com/en-us/rules-hub/
use local_rift::cards::CardKind;
use local_rift::game::{Decision, Game, Phase, Target};
use local_rift::legends::LegendTrigger;
use macroquad::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdviceLevel {
    Beginner,
    Advanced,
}

impl AdviceLevel {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Beginner => "Beginner",
            Self::Advanced => "Advanced",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Companion {
    level: AdviceLevel,
    message: String,
    last_observation: Option<u64>,
    tip_index: usize,
}

impl Default for Companion {
    fn default() -> Self {
        Self::new(AdviceLevel::Beginner)
    }
}

impl Companion {
    pub fn new(level: AdviceLevel) -> Self {
        Self {
            level,
            message:
                "I'm Mote, your little practice spirit. We'll find the next good move together."
                    .into(),
            last_observation: None,
            tip_index: 0,
        }
    }

    pub fn level(&self) -> AdviceLevel {
        self.level
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    /// Refreshes only when relevant, observable match state changes. Calling
    /// this each frame never causes a rotating or flickering stream of advice.
    pub fn observe(&mut self, game: &Game) {
        let observation = fingerprint(game);
        if self.last_observation != Some(observation) {
            self.last_observation = Some(observation);
            self.tip_index = 0;
            self.refresh(game);
        }
    }

    pub fn set_level(&mut self, level: AdviceLevel) {
        if self.level != level {
            self.level = level;
            self.tip_index = 0;
            self.last_observation = None;
        }
    }

    /// The user can ask for another relevant thought without taking an action.
    pub fn next_tip(&mut self, game: &Game) {
        self.last_observation = Some(fingerprint(game));
        self.tip_index = self.tip_index.wrapping_add(1);
        self.refresh(game);
    }

    fn refresh(&mut self, game: &Game) {
        let choices = advice(game, self.level);
        self.message = choices[self.tip_index % choices.len()].to_string();
        debug_assert!(self.message.chars().count() <= 180);
    }
}

fn fingerprint(game: &Game) -> u64 {
    let mut hash = DefaultHasher::new();
    game.turn.hash(&mut hash);
    format!("{:?}", game.play_grants).hash(&mut hash);
    format!("{:?}", game.play_history).hash(&mut hash);
    game.active.hash(&mut hash);
    game.priority().hash(&mut hash);
    game.in_window().hash(&mut hash);
    for item in &game.chain {
        item.id.hash(&mut hash);
        item.player.hash(&mut hash);
        item.card_id.hash(&mut hash);
        item.lane.hash(&mut hash);
        match item.target {
            Some(Target::Unit { player, unit_id }) => (0_u8, player, unit_id).hash(&mut hash),
            Some(Target::Spell { chain_id }) => (1_u8, 0_usize, chain_id).hash(&mut hash),
            Some(Target::Gear { player, gear_id }) => (3_u8, player, gear_id).hash(&mut hash),
            Some(Target::Battlefield { lane }) => (4_u8, lane, 0_u32).hash(&mut hash),
            None => (2_u8, 0_usize, 0_u32).hash(&mut hash),
        }
        for target in item.targets.iter().chain(&item.repeat_targets) {
            match target {
                Target::Unit { player, unit_id } => (0_u8, *player, *unit_id).hash(&mut hash),
                Target::Spell { chain_id } => (1_u8, 0_usize, *chain_id).hash(&mut hash),
                Target::Gear { player, gear_id } => (3_u8, *player, *gear_id).hash(&mut hash),
                Target::Battlefield { lane } => (4_u8, *lane, 0_u32).hash(&mut hash),
            }
        }
        format!("{:?}", item.executions).hash(&mut hash);
        format!("{:?}", item.ability).hash(&mut hash);
        item.options.repeat.hash(&mut hash);
        item.options.accelerate.hash(&mut hash);
        item.options.bonus.hash(&mut hash);
        item.from_hidden.hash(&mut hash);
    }
    if let Some(showdown) = &game.showdown {
        showdown.lane.hash(&mut hash);
        showdown.attacker.hash(&mut hash);
        showdown.focus.hash(&mut hash);
        showdown.passes.hash(&mut hash);
        showdown.combat.hash(&mut hash);
    }
    match &game.decision {
        Some(Decision::Predict { player, card_id }) => {
            (0_u8, *player).hash(&mut hash);
            // Predict's looked-at card is private to the predicting player.
            if *player == 0 {
                card_id.hash(&mut hash);
            }
        }
        Some(Decision::PredictCards {
            player,
            choice_id,
            cards,
        }) => {
            (5_u8, *player, *choice_id, cards.len()).hash(&mut hash);
            if *player == 0 {
                cards.hash(&mut hash);
            }
        }
        Some(Decision::CounterTax {
            player,
            chain_id,
            cost,
        }) => {
            (1_u8, *player, *chain_id, *cost).hash(&mut hash);
        }
        Some(Decision::TopThree { player, cards }) => {
            (3_u8, *player).hash(&mut hash);
            if *player == 0 {
                cards.hash(&mut hash);
            }
        }
        Some(Decision::RevealHand {
            player,
            opponent,
            cards,
        }) => {
            (4_u8, *player, *opponent).hash(&mut hash);
            // An explicit reveal makes these cards public for this effect.
            cards.hash(&mut hash);
        }
        Some(decision) => {
            // Other decisions contain only public object IDs and choice types.
            // Sabotage identifies a revealed hand's owner, never its contents.
            format!("{decision:?}").hash(&mut hash);
        }
        None => 2_u8.hash(&mut hash),
    }
    (game.phase as u8).hash(&mut hash);
    game.winner.hash(&mut hash);
    game.battlefield_owner.hash(&mut hash);
    game.battlefields.hash(&mut hash);
    game.scored_this_turn.hash(&mut hash);
    game.empowered_units.hash(&mut hash);
    game.buffed_units.hash(&mut hash);
    for token in game.pending_tokens() {
        (
            token.id,
            token.player,
            token.card_id,
            token.location,
            token.entry_ready,
        )
            .hash(&mut hash);
    }
    for legend in &game.legend {
        (legend.kind as u8).hash(&mut hash);
        legend.ready.hash(&mut hash);
        legend.empowered.hash(&mut hash);
        legend.spell_power.hash(&mut hash);
        legend.assault_grants.hash(&mut hash);
    }
    for (owner, player) in game.players.iter().enumerate() {
        // These fields describe public information. Never read enemy hand,
        // deck, or rune-deck identities, even to compute an observation hash.
        player.name.hash(&mut hash);
        player.score.hash(&mut hash);
        player.energy.hash(&mut hash);
        player.runes.hash(&mut hash);
        player.exhausted_runes.hash(&mut hash);
        player.champion.hash(&mut hash);
        player.power_pool.hash(&mut hash);
        player.any_power.hash(&mut hash);
        player.xp.hash(&mut hash);
        game.gained_xp_this_turn(owner).hash(&mut hash);
        for gear in &player.gear {
            gear.id.hash(&mut hash);
            gear.card_id.hash(&mut hash);
            gear.attached_to.hash(&mut hash);
            gear.ready.hash(&mut hash);
            gear.temporary.hash(&mut hash);
            game.gear_temporary_status(owner, gear.id)
                .map(|status| (status.intrinsic, status.granted, status.active))
                .hash(&mut hash);
            gear.token.hash(&mut hash);
        }
        for hidden in &player.hidden {
            hidden.id.hash(&mut hash);
            hidden.lane.hash(&mut hash);
            hidden.hidden_turn.hash(&mut hash);
            hidden.revealed.hash(&mut hash);
            // Use the same visibility boundary as the UI, including an explicit
            // Scuttle Crab look. Private facedown identities never enter the hash.
            game.hidden_visible_card(0, owner, hidden.id)
                .hash(&mut hash);
        }
        for unit in &player.units {
            unit.id.hash(&mut hash);
            game.temporary_status(local_rift::empower::BoardObject::Unit {
                player: owner,
                unit_id: unit.id,
            })
            .map(|status| (status.intrinsic, status.granted, status.active))
            .hash(&mut hash);
            unit.card_id.hash(&mut hash);
            unit.lane.hash(&mut hash);
            unit.ready.hash(&mut hash);
            unit.damage.hash(&mut hash);
            unit.buff.hash(&mut hash);
            unit.temporary_buff.hash(&mut hash);
            unit.token.hash(&mut hash);
            unit.protected.hash(&mut hash);
            unit.stunned.hash(&mut hash);
            unit.banish_on_death.hash(&mut hash);
        }
    }
    game.players[0].hand.hash(&mut hash);
    // Match journal entries report resolved public actions, not hidden cards.
    for entry in game.log.iter().rev().take(3) {
        entry.hash(&mut hash);
    }
    hash.finish()
}

fn advice(game: &Game, level: AdviceLevel) -> Vec<&'static str> {
    match checked_advice(game, level) {
        Ok(tips) => tips,
        Err(_) => vec![
            "This position reached a numeric limit. I'll pause tactical advice until its stats can be calculated accurately.",
        ],
    }
}

fn checked_advice(game: &Game, level: AdviceLevel) -> Result<Vec<&'static str>, String> {
    game.validate_numeric_state()?;
    let beginner = level == AdviceLevel::Beginner;
    let mut tips = Vec::new();
    let human = &game.players[0];
    let enemy = &game.players[1];

    if let Some(winner) = game.winner {
        tips.push(if winner == 0 {
            "A little victory sparkle! Which move earned you the most breathing room? That's a useful lesson to carry into the next game."
        } else {
            "Every match leaves a clue. Try spotting the first battlefield you could have protected or contested. I'll be here for the next one."
        });
        tips.push("Try this matchup from the other side next. Seeing both hands in separate games can help you recognize each deck's plan.");
        return Ok(tips);
    }

    if game.phase == Phase::Mulligan {
        tips.push(if beginner {
            "Four cards, a fresh start. You may replace up to two. Look for an affordable unit so you can get a board started."
        } else {
            "Plan your first two turns, not just your cheapest play. A useful opening has a deployable unit and a sensible use for your next runes."
        });
        tips.push("Your chosen champion is available in its own zone. You don't need to find that copy in your opening hand.");
        tips.push("Hover over a card to read its full rules. You can keep a preview open with a right-click while planning your first plays.");
        tips.push(archetype_tip(game));
        return Ok(tips);
    }

    if let Some(window_tips) = response_window_advice(game, beginner)? {
        return Ok(window_tips);
    }

    // High stakes scoring and visible threats come before routine suggestions.
    let enemy_has_hold = game.battlefield_owner.contains(&Some(1));
    let human_has_hold = game.battlefield_owner.contains(&Some(0));
    if enemy.score == 7 && enemy_has_hold {
        tips.push(if beginner {
            "Your opponent is one point away and controls a battlefield. See whether you can take it away before their next turn."
        } else {
            "Hold denial is urgent: their next uncontested hold can win. Compare removal and grouped pressure before spending runes elsewhere."
        });
    }
    if human.score == 7 {
        tips.push("At 7 points, a hold can win. A conquest wins only after you've scored every other battlefield that turn; otherwise it draws a card.");
        if human_has_hold && !beginner {
            tips.push("Your held battlefield is a win condition. Compare strengthening it with forcing the opponent to answer a second threat.");
        }
    }

    if human.hand.iter().any(|id| matches!(id, 372 | 373 | 375)) {
        tips.push(if beginner {
            "Return to hand does not kill, so Deathknell does not trigger. Tokens disappear; gear detaches when its unit leaves. Recall instead moves a unit to base."
        } else {
            "Returns buy time, but cards can be played again. Downwell also returns your board; Angler Beast checks current Might when its ability resolves. Compare both sides first."
        });
    }

    let contested = (0..2).find(|&lane| game.contested(lane));
    if let Some(lane) = contested {
        if beginner {
            tips.push("Both sides have units here. Check might totals and legal responses before resolving, then choose your combat damage order.");
        } else if game.lane_might(0, lane)? <= game.lane_might(1, lane)? {
            tips.push("This trade looks costly on raw might. Check your legal responses before committing; printed reactions can change real games.");
        } else {
            tips.push("You lead on raw might. Consider what survives, how damage can be assigned, and whether the survivor can hold the next point.");
        }
    }

    if game.active == 1 {
        tips.push(if beginner {
            "Their turn is a good time to look ahead. Which of your units will be ready next turn, and which battlefield could it reach?"
        } else {
            "Watch public commitments: spent runes and deployed units. Plan a reply from your own hand; an unseen card is still an unknown."
        });
        tips.push("Action and reaction timing matter. You can receive priority during their turn; supported responses appear when a window opens.");
        tips.push(archetype_tip(game));
        return Ok(tips);
    }

    // Brief feedback on a resolved event, always phrased in terms the client
    // actually implements rather than treating omitted abilities as available.
    let recent = game
        .log
        .iter()
        .rev()
        .take(2)
        .map(String::as_str)
        .collect::<Vec<_>>();
    if recent
        .iter()
        .any(|line| line.contains("conquers battlefield"))
    {
        tips.push(if beginner {
            "Ground claimed! Now look ahead: can a unit remain there until your next turn to earn a hold?"
        } else {
            "Conquest gained tempo. Decide whether defending that future hold is worth more than preserving a flexible unit at base."
        });
    } else if recent.iter().any(|line| {
        line.contains(" plays ") && line.contains(" power)") && !line.contains(", 0 power)")
    }) {
        tips.push("Recycling for power spends future access to runes, too. Check generated power before recycling another matching rune.");
    } else if recent.iter().any(|line| line.contains("is defeated")) {
        tips.push(if beginner {
            "A unit was defeated. Take a fresh look at both battlefields; the open route may matter more than the last trade."
        } else {
            "After a trade, reassess who threatens the next hold. Saving a card can be worthwhile if your board already covers that point."
        });
    }

    if recent
        .iter()
        .any(|line| line.contains("combat:") || line.contains("Combat cleanup"))
    {
        tips.push("After combat cleanup, damage is cleared. Recheck the survivors and the next point before spending another card.");
    }
    if human_has_hold {
        tips.push(if beginner {
            "You may deploy a unit exhausted at a battlefield you control, or at base. A held field can be reinforced without moving from base first."
        } else {
            "Deploying into a controlled field reinforces a hold immediately. Deploying at base preserves a choice of destinations next turn."
        });
    }

    let human_stats = human
        .units
        .iter()
        .map(|unit| game.try_unit_stats(unit).map(|stats| (unit, stats)))
        .collect::<Result<Vec<_>, _>>()?;
    let ready_at_base = human.units.iter().any(|u| u.ready && u.lane.is_none());
    if ready_at_base {
        for (lane, field) in game.battlefields.iter().enumerate() {
            if game.battlefield_owner[lane] == Some(0) {
                continue;
            }
            match field {
                0 if human_stats.iter().any(|(unit, stats)| unit.ready && unit.lane.is_none() && stats.might >= 5) => {
                    tips.push("A Mighty unit can unlock Sunken Temple's conquest draw. Keep 1 energy if that extra card is part of your plan.");
                }
                1 if game.battlefield_owner.iter().enumerate().any(|(other, owner)| other != lane && *owner == Some(0)) => {
                    tips.push("You hold another field. Conquering Seat of Power can draw a card while you still control that other location.");
                }
                2 => tips.push("Minefield puts your top two cards into trash when you conquer. That can supply later trash plays, but also brings burnout closer."),
                3 => tips.push("Void Gate adds 1 to each spell or ability damage packet against units there. The bonus can help your removal and threaten your own units."),
                _ => {},
            }
        }
    }
    let empty_field =
        (0..2).any(|lane| game.lane_count(0, lane) == 0 && game.lane_count(1, lane) == 0);
    if ready_at_base && empty_field {
        tips.push(if beginner {
            "A ready unit and an empty battlefield: a lovely starting point. Select the unit, then the field, to try a standard move."
        } else {
            "An empty field is available. A small unit may claim it while a stronger unit stays flexible for the more contested point."
        });
    } else if ready_at_base {
        tips.push(if beginner {
            "Your ready units can make a standard move by exhausting. Look at the destination before moving; an enemy unit means a fight."
        } else {
            "Units at base threaten either field. Compare the point you can gain with the hold you leave exposed before committing them."
        });
    }

    let mut affordable_unit = false;
    let mut power_shortage = false;
    for &id in &human.hand {
        let card = &game.catalog[id];
        let affordable = game.can_afford(0, card)?;
        affordable_unit |= card.kind == CardKind::Unit && affordable;
        power_shortage |= u16::from(card.cost) <= human.energy && !affordable;
    }
    if power_shortage {
        tips.push(if beginner {
            "Energy isn't the whole cost. Power needs matching or flexible power; recycling a matching rune is one way to supply it."
        } else {
            "Power is your bottleneck. Check matching runes and generated power before committing; spare energy cannot replace a power cost."
        });
    }
    if human.hand.iter().any(|&id| {
        local_rift::rules::play_target_spec(id).kind
            == Some(local_rift::rules::PlayTargetKind::Battlefield)
    }) {
        tips.push(if beginner {
            "This spell chooses a battlefield, not its units. Click either field, even an empty one; enemies there when it resolves receive the effect."
        } else {
            "Field targets stay fixed through responses, but their occupants can change. Each Repeat can choose its own field; check where enemies may move."
        });
    }
    if affordable_unit {
        tips.push(if beginner {
            "You can afford a unit. Most units enter exhausted, so think of deploying now as preparing your next turn's moves."
        } else {
            "You have an affordable deployment. Compare its next-turn pressure with preserving runes for an immediate, useful effect."
        });
    }
    if game.play_grants.iter().any(|grant| grant.player == 0) {
        tips.push(if beginner {
            "A next-play effect is waiting. Open Turn effects to see its source and expiry before choosing your next card."
        } else {
            "Next-play effects are committed when that play begins. Check which card benefits before spending a small play just to use spare resources."
        });
        if game.play_grants.iter().any(|grant| {
            grant.player == 0 && grant.kind == local_rift::play::PlayGrantKind::RepeatNextSpell
        }) {
            tips.push(if beginner {
                "Each Repeat is an optional extra cost. Choose its own targets in Play choices; paying twice does not play the spell twice."
            } else {
                "Several Repeat sources are independent. Compare each added execution's targets and cost, including any reduction shared across the total."
            });
        }
        if game.play_grants.iter().any(|grant| {
            grant.player == 0 && grant.kind == local_rift::play::PlayGrantKind::ReadyBuffNextUnit
        }) {
            tips.push("Nami's waiting effect readies and buffs after the unit is played, through a triggered ability. Entering ready happens earlier and is a different event.");
        }
    }
    let empower_gear: Vec<_> = human
        .gear
        .iter()
        .filter(|gear| {
            !gear.token
                && gear.attached_to.is_none()
                && local_rift::activated::activated_rules(gear.card_id)
                    .iter()
                    .any(|rule| rule.only_if_not_empowered)
        })
        .collect();
    if !empower_gear.is_empty() {
        let empowered = empower_gear.iter().any(|gear| game.unit_empowered(gear.id));
        tips.push(if empowered {
            if beginner {
                "Your gear is Empowered. If an ability costs Disempower, that state is spent before responses; countering the ability does not restore it."
            } else {
                "An ability with a Disempower cost spends that state before responses. A counter stops its effect, but does not restore the paid state."
            }
        } else if beginner {
            "Your gear is not Empowered. Its self-Empower ability chooses no target, pays its printed costs, and takes effect after the response window."
        } else {
            "Self-Empower is targetless and respondable. Compare its future payoff with today's energy and ready state before committing to the printed costs."
        });
        if empower_gear
            .iter()
            .any(|gear| !gear.ready && game.unit_empowered(gear.id))
        {
            tips.push("Exhausted and Empowered are separate states. Your gear keeps Empowered while exhausted; an Exhaust cost still needs a ready source.");
        }
    }
    let mut contributions = Vec::new();
    for unit in &human.units {
        contributions.extend(game.passive_contributions(
            local_rift::empower::BoardObject::Unit {
                player: 0,
                unit_id: unit.id,
            },
        )?);
    }
    if !contributions.is_empty() {
        tips.push(if beginner {
            "A unit's live preview shows which conditional bonuses apply now and which source supplies each one. Ready, Empowered and Buffed are separate states."
        } else {
            "Check active conditional bonuses before trading. Removing or Disempowering an aura source can change several units' might immediately."
        });
    }
    let conditions: Vec<_> = contributions
        .iter()
        .filter_map(|source| {
            local_rift::continuous::passive_rules(source.card_id)
                .get(source.rule_index)
                .map(|rule| rule.condition)
        })
        .collect();
    use local_rift::continuous::SourceCondition;
    if conditions.contains(&SourceCondition::ControllerGainedXpThisTurn) {
        tips.push(if beginner {
            "Wily Newtfish checks whether you gained XP this turn, not just your XP total. Its extra might and Ganking last while that public history is true."
        } else {
            "An XP gain can unlock Wily's field-to-field movement this turn. The gain history resets each turn; recheck Ganking before planning a later move."
        });
    }
    if human.hand.iter().any(|&id| {
        matches!(
            local_rift::rules::entry_ready_rule(id),
            Some(local_rift::rules::EntryReadyRule::ControllerXpAtLeast(_))
        )
    }) {
        tips.push(if beginner {
            "A Level entry rule checks your XP when the unit enters. Entering ready is its initial state, not a later Ready effect; Accelerate is a separate option."
        } else {
            "Reach the entry threshold before deploying. Gaining XP after a unit entered exhausted will not retroactively ready it."
        });
    }
    if human
        .hand
        .iter()
        .any(|&id| !local_rift::rules::xp_energy_discount_tiers(id).is_empty())
    {
        tips.push(if beginner {
            "Concentrate uses only its highest reached Level discount. The cost panel shows what you pay; its two printed reductions do not add together."
        } else {
            "Concentrate's cost locks when finalization begins. Use the cost panel to review its current XP discount and any separate Repeat cost."
        });
    }
    if conditions.contains(&SourceCondition::Buffed) {
        tips.push(if beginner {
            "Buffed means having a Buff counter. Temporary might and Empowered do not count; the live preview shows whether this unit's extra ability is active."
        } else {
            "Spending a Buff can remove a conditional bonus as well as the counter's might. Recheck Ganking and lethal damage before using that counter."
        });
    }
    if conditions
        .iter()
        .any(|condition| matches!(condition, SourceCondition::ControllerRunesAtLeast(_)))
    {
        tips.push(if beginner {
            "Rune thresholds count ready and exhausted runes in play. Recycling sends a rune back to the deck, so a live might bonus can disappear."
        } else {
            "Before recycling for power, check your rune threshold. Losing a live might bonus can expose a damaged unit even when the payment itself is affordable."
        });
    }
    if conditions
        .iter()
        .any(|condition| matches!(condition, SourceCondition::ControllerXpAtLeast(_)))
    {
        tips.push(if beginner {
            "Level checks your current XP. Equipment can grant a Level bonus to its bearer; the preview separates that bonus from the equipment's printed might."
        } else {
            "XP can improve several Level units or bearers at once. Inspect their live thresholds before planning the next scoring trade."
        });
    }
    if human_stats.iter().any(|(_, stats)| stats.keywords.hunt > 0) {
        tips.push(if beginner {
            "Hunt gains XP when its unit conquers or holds. Printed Hunt and equipped Hunt add together into one ability; the preview shows the total."
        } else {
            "Stacked Hunt produces one captured XP amount per bearer when it conquers or holds. Protect the scoring unit, and consider which Level thresholds that XP reaches."
        });
    }
    if human.gear.iter().any(|gear| {
        game.gear_temporary_status(0, gear.id)
            .is_some_and(|status| status.active)
    }) {
        tips.push(if beginner {
            "Temporary gear expires through a beginning trigger before hold scoring. Granted Temporary stays active when attached; printed Temporary does not."
        } else {
            "Attachment will not suppress granted Temporary. Count the gear bonuses or protection that will disappear when its beginning trigger resolves before hold."
        });
    }
    if human.units.iter().any(|unit| {
        game.has_temporary(local_rift::empower::BoardObject::Unit {
            player: 0,
            unit_id: unit.id,
        })
    }) {
        tips.push(if beginner {
            "Temporary units trigger a kill ability at your Beginning Phase, before hold scoring. You can respond to that ability; moving or recalling them keeps Temporary."
        } else {
            "Temporary expires before hold scoring. Plan around the coming trigger: a Sprite can conquer now, but keeping it for next turn's hold needs another effect."
        });
    }
    if human.hand.contains(&350) {
        tips.push(if beginner {
            "Last Stand doubles the unit's current Might, including active bonuses, then gives Temporary. Its fixed increase lasts this turn; Temporary lasts until removed."
        } else {
            "Each Last Stand execution takes a fresh Might snapshot. Combat bonuses affect that snapshot, but their later loss still reduces the final total."
        });
    }
    if human.hand.iter().any(|&id| {
        matches!(
            local_rift::rules::effect(id),
            local_rift::rules::OfficialEffect::KillAllUnits
        )
    }) {
        tips.push(if beginner {
            "The Ruination kills units at both bases and both fields, including yours. It chooses no targets; available death replacements are still offered."
        } else {
            "A global kill reaches both bases. Compare your surviving replacement effects and death triggers before spending it; untargeted does not mean enemy-only."
        });
    }
    if !game.activated_options(0).is_empty() {
        tips.push(if beginner {
            "Units and gear can have their own abilities. Select the source, then Abilities to read its timing, costs and legal targets before committing."
        } else {
            "An exhaust cost competes with movement. Compare the ability's payoff with a useful move, and reserve resources for responses after it enters the chain."
        });
    }
    if game.activated_options(0).iter().any(|option| {
        local_rift::activated::activated_rules(option.card_id)[option.ability_index].xp_cost > 0
    }) {
        tips.push(if beginner {
            "An XP cost spends your XP before responses. A lower total can switch off Level bonuses; spending does not erase an XP gain earlier this turn."
        } else {
            "Check live Level thresholds before paying XP. The ability waits for responses after payment; a lost might bonus can matter before the Buff resolves."
        });
    }
    if human.gear.iter().any(|gear| {
        gear.attached_to.is_none()
            && local_rift::activated::activated_rules(gear.card_id)
                .iter()
                .any(|rule| rule.minimum_xp.is_some() && rule.fixed_resource_output().is_some())
    }) {
        tips.push(if beginner {
            "Honeyfruit has two choices at Level 6. Both exhaust the same gear: add 1 Any power, or add 1 energy and 1 Any power. Neither spends XP."
        } else {
            "Honeyfruit's Level ability depends on current XP. At 6 both outputs remain available, but one activation exhausts the shared source."
        });
    }
    if human.gear.iter().any(|gear| {
        local_rift::rules::equipment(gear.card_id).is_some_and(|rule| rule.equip_xp > 0)
    }) {
        tips.push(if beginner {
            "Heirloom gains XP from its own play trigger. Equip then spends XP to attach it; its +2 Might belongs to the bearer, and attaching is not another play."
        } else {
            "Heirloom's play trigger and Equip are separate. Let the XP trigger resolve before using that XP; attaching grants the bearer +2 Might, not another play trigger."
        });
    }
    if human.energy == 0 {
        tips.push(if beginner {
            "No energy left? Ready units may still move. When your useful actions are done, end the turn and let your next turn refresh things."
        } else {
            "Your energy is spent. Check free board actions and future holds, then pass; more actions are useful only when they improve the position."
        });
    }
    if !human.units.is_empty() && human.units.iter().all(|unit| !unit.ready) {
        tips.push("Sleepy units! Exhaustion alone doesn't stop combat might. Stun can; check the unit's state. Units normally ready on their controller's turn.");
    }
    if beginner {
        tips.push("Turn rhythm: ready, beginning and hold scoring, channel runes, then draw. After those steps, plan your actions.");
        tips.push("Ready runes can exhaust for energy. Recycle a rune for matching power, or use generated power. Exhaust first if you want both.");
    } else {
        tips.push("A turn's best trade may be a point, not a unit. Count the next scoring opportunities on both sides before choosing a fight.");
        tips.push("Real matchups depend on printed timing, legends, and keywords. Practice results here are not a ranking of the tournament decks.");
    }
    tips.push(archetype_tip(game));
    Ok(tips)
}
fn response_window_advice(
    game: &Game,
    beginner: bool,
) -> Result<Option<Vec<&'static str>>, String> {
    if let Some(decision) = &game.decision {
        if decision.player() != 0 {
            return Ok(Some(vec![
                "Your opponent is making a choice. I'll watch the public board while we wait, then help when priority returns.",
                "Use this pause to identify the unit or spell that matters most to the next point. A response is useful when it changes that outcome.",
            ]));
        }
        let mut dual_rank = false;
        if let Decision::CombatDamage { player, lane, .. } = decision {
            for unit in game.players[1 - *player]
                .units
                .iter()
                .filter(|unit| unit.lane == Some(*lane))
            {
                let keywords = game.unit_keywords(unit)?;
                dual_rank |= keywords.tank && keywords.backline;
            }
        }
        return Ok(Some(match decision {
            Decision::PredictCards { .. } => vec![
                "Predict looks at cards without drawing them. Choose any to recycle, then set the kept cards in the order you want to draw them.",
                "Keep the next draw that serves your plan. Recycling all is allowed; recycled cards go to the bottom in random order. Confirm only after checking the order.",
            ],
            Decision::Payment { .. }
                if game.payment_plan_view(0).is_some_and(|plan| plan.xp > 0) =>
            {
                vec![
                    "XP is a separate fixed cost. Add abilities make energy or power, not XP; check your XP total before finishing this payment.",
                    if beginner {
                        "Recording the cost does not spend XP yet. Completing payment spends it before responses, so check which Level bonuses will remain."
                    } else {
                        "A failed payment can undo its resource steps. Finishing spends the fixed XP cost and checks lost Level bonuses before the response window."
                    },
                ]
            }
            Decision::Payment { .. } => vec![
                "This action's costs are fixed. Use Add abilities if needed, then finish the payment. An unsuccessful attempt can be undone with its resource steps.",
                "Keep enough matching power and energy for the fixed cost. A conversion can help one resource while spending the other.",
            ],
            Decision::ResourceConversion { .. } => vec![
                "Choose how much to convert. The gear is already exhausted. Zero finishes without adding resources; your original payment stays waiting.",
                "Other Add abilities can supply this payment. Keep the energy and colored power needed for the original action, not just the conversion.",
            ],
            Decision::TriggerTargets { card_id: 374, .. } => vec![
                "Mournful Spirit remembers where it died. Choose a highlighted unit on the Board or gear in a Loadout at that location, then confirm. You may decline.",
                "The target must still be at the death location when the ability resolves. Check unit Deflect costs and death replacements; attached gear is a separate target.",
            ],
            Decision::TriggerTargets {
                card_id: 366..=369, ..
            } => vec![
                "Choose highlighted gear in either Loadout, then confirm. Attached gear is still a separate target; destroying it removes its benefit from the bearer. You may decline.",
                "Check the gear's printed Energy limit and any on-death effect. Noxian Demolitionist checks its current Might again on resolution; Adaptatron buffs only if it actually kills the gear.",
            ],
            Decision::TriggerTargets { .. } => vec![
                "Choose this ability's highlighted targets in order, then confirm. You can decline only when the ability is optional.",
                "The ability joins the chain after you choose targets. Check the next trade and any Deflect costs before confirming.",
            ],
            Decision::OptionalPlay { .. } => vec![
                "Accept or decline this trash-play trigger. If accepted, choose its trash target before the ability joins the chain; finalize the card after it resolves.",
                "Check the trash and your available resources before accepting. Declining here keeps the ability off the chain.",
            ],
            Decision::TokenPlay { card_id, .. } | Decision::TokenPlacement { card_id, .. }
                if local_rift::rules::token_spec(*card_id).is_some_and(|token| token.temporary) =>
            {
                vec![
                    if game
                        .current_pending_token()
                        .is_some_and(|token| token.entry_ready)
                    {
                        "This Sprite has Temporary: its Beginning trigger kills it before hold scoring. It enters ready and can move later this turn."
                    } else {
                        "This Sprite has Temporary: its Beginning trigger kills it before hold scoring. Check its entry state before planning a move."
                    },
                    "A Sprite can help with a conquest or a trade now. A hold next turn needs a plan for its Temporary trigger first.",
                ]
            }
            Decision::TokenPlay { .. } => vec![
                if game
                    .current_pending_token()
                    .is_some_and(|token| token.entry_ready)
                {
                    "This token is instructed to enter ready. That is its initial state, not a later Ready effect; optional costs are separate."
                } else {
                    "This token can enter ready if you pay Accelerate. Choose its allowed location and optional cost before it enters play."
                },
                "Ready entry can enable a later move. Compare the optional cost with the resources you need for responses and later plays.",
            ],
            Decision::TokenPlacement { .. } => vec![
                "Place each unit token at your base or a battlefield you control. A unit at a field can help defend the next hold.",
                "Choose each location separately. Reinforce the field that matters, or keep a unit at base for a later movement.",
            ],
            Decision::ResolutionTargets { .. } => vec![
                "Responses moved your targets apart. Choose the legal location group you want this spell to affect; the other targets will be left alone.",
                "The largest group is not always best. Focus the damage where it changes a trade, conquest or upcoming hold.",
            ],
            Decision::OrderTriggers { .. } => vec![
                "Choose which triggered ability joins the chain next. The last one placed resolves first, so plan the order before adding them.",
                "Pending triggers are not on the chain yet. Once finalized, each ability can be answered in the normal response window.",
            ],
            Decision::OnPlayTarget { .. } => vec![
                "Your permanent has entered play. Choose a legal target now; its triggered ability will then join the chain for responses.",
                "Check which friendly ready, might bonus or attachment helps your next point. Choose an eligible unit on the board or in the Board tab.",
            ],
            Decision::DeathReplacement { .. } => vec![
                "Several effects can change this death. Choose which unit and replacement to use; the others may be checked again afterward.",
                "Compare saving your key unit with keeping equipment for a later fight. A death replacement can change where a unit ends up.",
            ],
            Decision::BattlefieldOrder { combat, .. } => vec![
                if *combat {
                    "Several combats are waiting. The turn player chooses which battlefield opens first; the others stay pending."
                } else {
                    "Resolve uncontested showdowns before combats. When several are waiting, the turn player chooses which battlefield opens first."
                },
                "Consider which field's score or triggers will affect the next fight. Choosing a field opens its normal response window.",
            ],
            Decision::CombatDamage { .. } if dual_rank => {
                vec![
                    "A unit has both Tank and Backline. You choose first or last for that unit, never the middle group. Deal lethal before assigning to the next unit.",
                    "Both players assign before damage is dealt simultaneously. Use exact amounts and click a card's art to change its order.",
                ]
            }
            Decision::CombatDamage { .. } => vec![
                if beginner {
                    "Choose how your combat damage reaches opposing units. Assign Tank first and Backline last; deal lethal before moving to another unit."
                } else {
                    "Allocate for the next point. Within Tank/Backline restrictions, choose which opposing unit must die and whether excess damage changes a replacement."
                },
                "Both players assign before damage is dealt simultaneously. Your units still deal their captured total even if the opposing assignment defeats them.",
                "Use the suggestion or edit exact amounts. Every point must be assigned; lethal comes before the next unit, with Tank first and Backline last.",
            ],
            Decision::Predict { .. } => vec![
                if beginner {
                    "Predict is a little peek: keep this card on top or recycle it. Think about what you'd like to draw next."
                } else {
                    "For Predict, compare the known top card with your next turn's needs. Recycle only when a fresh draw is worth giving up that certainty."
                },
                "Keeping a predicted card leaves it on top; recycling puts it at the bottom. Predict does not itself draw that card.",
            ],
            Decision::CounterTax { .. } => vec![
                if beginner {
                    "A counter asks you to pay extra energy. Paying saves your spell from this counter; declining lets the counter stop it."
                } else {
                    "Price the counter tax against the point at stake. Paying keeps the spell, but those runes won't be available for the next exchange."
                },
                "Check the energy you would have left after paying. Protecting this spell is a choice, not an obligation.",
                "Resource abilities can be used before paying. Board → Abilities shows unit sources; Loadout shows gear and legends.",
            ],
            Decision::Legend {
                trigger: LegendTrigger::IreliaChoose { .. },
                ..
            } => vec![
                if beginner {
                    "Irelia can exhaust her legend and pay 1 power to ready the friendly unit you chose. Accepting puts that ability on the chain."
                } else {
                    "An Irelia ready can enable another move. Compare that point pressure with keeping a rune and preserving the legend for a later choice."
                },
                "Declining an optional legend trigger is fine. The original card still waits on the chain if it has not resolved.",
            ],
            Decision::Legend {
                trigger: LegendTrigger::IreliaConquer,
                ..
            } => vec![
                "Conquest offers Irelia a reset: pay 1 energy to ready her legend. That can prepare another friendly-unit choice later this turn.",
                "Check whether the legend is already ready. Paying is optional; a useful future ready matters more than using every trigger.",
            ],
            Decision::LastRites { .. } => vec![
                "Last Rites lets you play a unit from your trash. Its normal costs still matter; choose an affordable unit or decline.",
                "A recovered unit can reinforce your next point. Compare that plan with saving the resources for a response.",
            ],
            Decision::TrashTarget { .. } => vec![
                "Choose the card in your trash that this ability will play. The ability enters the chain with that target, so opponents can respond first.",
                "Check the chosen card's targets and costs now. You will finalize its own choices after the parent ability finishes resolving.",
            ],
            Decision::PendingPlay { kind, .. } => vec![
                match kind {
                    local_rift::game::InstructedPlayKind::Fizz => {
                        "Finalize the chosen spell: base energy is waived, but power and additional costs still apply. It recycles after resolving."
                    }
                    local_rift::game::InstructedPlayKind::LastRites => {
                        "Finalize the chosen unit, paying its costs. Choose its destination and any optional costs now, after Last Rites has resolved."
                    }
                    local_rift::game::InstructedPlayKind::Matriarch => {
                        "Play the chosen unit at base. Its base cost is waived; optional costs such as Accelerate are chosen and paid now."
                    }
                },
                "This card has left the trash and is pending. Add resources if needed; finish its choices and payment before ordinary priority resumes.",
            ],
            Decision::ReplaySpell { .. } => vec![
                "Fizz offers a spell from your trash with energy cost 3 or less. Ignore its energy cost, pay its power, then recycle the played spell.",
                "Look for a useful legal target before replaying. A free energy cost does not make an effect useful by itself.",
            ],
            Decision::GrantFlow { .. } => vec![
                "Choose a spell in your trash to give Flow this turn. You may later pay its Flow cost to play it from trash, then banish it; this choice doesn't cast it.",
                "Check the spell's legal targets and the resources you will have left before choosing which card gains Flow.",
            ],
            Decision::Battlefield { .. } => vec![
                "Sunken Temple offers a card after conquering with a Mighty unit. Pay 1 energy to put its draw ability on the chain, or decline.",
                "Compare an extra card with keeping energy for the next response. A battlefield's optional trigger is a choice, not an obligation.",
            ],
            Decision::RevivePlay { .. } => vec![
                "Play the selected unit at base. Its base cost is waived; optional costs such as Accelerate are chosen and paid now.",
                "Accelerate can give this revived unit an immediate move. Compare that tempo with keeping the additional energy and power for a response.",
            ],
            Decision::ReviveUnit { .. } => vec![
                "Matriarch may choose a unit costing at most 3 energy and 1 power from your trash. Choose it now; the ability will play it at base when it resolves.",
                "A useful revival improves the board you have now. The largest unit is not always the best next defender or attacker.",
            ],
            Decision::SwapUnit { .. } => vec![
                "This effect offers a friendly unit at another location to swap with. Choose a legal unit, or decline the optional swap.",
                "Check both destinations before swapping: you may strengthen one field while leaving another point exposed.",
            ],
            Decision::EmpowerHere { .. } => vec![
                "Choose a unit at this location to empower, or empower your legend. The choice may name an enemy; check which empowered ability will benefit.",
                "Empower can prepare a later payoff. Compare an immediate combat improvement with a useful future movement or resource effect.",
            ],
            Decision::Discard { .. } => vec![
                "Choose the required discard before drawing. Keep the cards that fit your next turn's resources and scoring plan.",
                "A spare expensive card may be less useful than a timely response. Your next battlefield point can guide the discard.",
            ],
            Decision::TopThree { .. } => vec![
                "Look at these top cards and choose up to one to draw. The others go to trash; you may choose none.",
                "Choose for your next useful play, not just the largest card. You already know what your current hand can accomplish.",
            ],
            Decision::Sabotage { .. } => vec![
                "Sabotage reveals their hand for this choice. Pick a non-unit to recycle, if one is available; the remaining cards stay in their hand.",
                "Use the revealed information to protect your next point. A response that stops your plan may matter more than the most expensive card.",
            ],
            Decision::RevealHand { .. } => vec![
                "Scuttle Crab reveals the opponent's hand and lets you inspect their facedown cards this turn. Read that information before planning your next point.",
                "A revealed answer is useful information. Keep enough resources to respond if it would stop your most important attack or hold.",
            ],
            Decision::BurnForMight { .. } => vec![
                "This optional Burn cost trades cards from the top of your deck for might. Check whether the extra strength changes a fight or a point.",
                "Burning moves cards to trash. Think about future deck access as well as the immediate combat bonus.",
            ],
        }));
    }
    if !game.chain.is_empty() {
        let mut tips = vec![];
        if game.priority() == 0 {
            tips.push(if beginner {
                "You have priority. A spell or ability waits on the chain: play a legal reaction or pass. Two consecutive passes resolve the newest item first."
            } else {
                "You have priority. Spend interaction on the effect that changes the point or trade; saving every spell can empty your resources."
            });
            let friendly_target = game.chain.last().is_some_and(|item| {
                item.player == 1
                    && (matches!(item.target, Some(Target::Unit { player: 0, .. }))
                        || item
                            .targets
                            .iter()
                            .any(|target| matches!(target, Target::Unit { player: 0, .. })))
            });
            if friendly_target {
                tips.push(if beginner {
                    "An opposing effect targets your unit. Check your legal responses and the item on the chain before choosing to pass."
                } else {
                    "A friendly unit is targeted. If it anchors a hold or a winning attack, weigh protection against saving your reaction for the next threat."
                });
            }
        } else {
            tips.push(if beginner {
                "Your opponent has priority on the chain. Wait for your response window; the turn hasn't changed just because priority did."
            } else {
                "Priority is with your opponent. Reassess the chain after their response; its newest item resolves before the older commitments."
            });
        }
        tips.push("The chain is a closed state. Reaction timing is needed to add a response; an Action-only spell waits for an open showdown or your turn.");
        tips.push(if beginner {
            "Passing priority is not ending your turn. It gives the next player a chance to respond before the waiting effect resolves."
        } else {
            "A pass is not a promise to stop interacting forever. Reassess after each play or resolution, and preserve resources for the decisive effect."
        });
        return Ok(Some(tips));
    }
    if let Some(showdown) = &game.showdown {
        let mut tips = vec![];
        if game.priority() == 0 {
            if showdown.passes > 0 {
                tips.push(if beginner {
                    "Your opponent passed focus. If you pass too, the showdown closes and the battlefield proceeds to combat or control."
                } else {
                    "They passed focus. Decide whether extra investment changes the result; passing back advances the showdown without another commitment."
                });
            } else {
                tips.push(if beginner {
                    "The showdown is open and you have focus. Play a legal Action or Reaction, or pass. This response window also happens at an empty field."
                } else {
                    "You have focus. Protect a key attacker when it changes the point; avoid adding cards to a trade your existing board already wins."
                });
            }
        } else {
            tips.push(if beginner {
                "Your opponent has focus in this showdown. You'll get a chance to respond; a unit arriving doesn't settle control immediately."
            } else {
                "The opponent has focus. Track their public spending and your own response options, rather than assuming an unseen answer."
            });
        }
        tips.push("An open showdown allows correctly timed Actions and Reactions. Once a spell is on the chain, only legal Reactions can respond.");
        tips.push("The last ability added to the chain resolves first. When several of your triggers are waiting, choose their order with that in mind.");
        return Ok(Some(tips));
    }
    Ok(None)
}

fn archetype_tip(game: &Game) -> &'static str {
    use local_rift::legends::LegendKind;
    match game.legend[0].kind {
        LegendKind::Ambessa => {
            "When you Empower any other object, Ambessa triggers her own Empower. Spend her state, ready status and 1 Any power to ready a unit during your open Main Phase."
        }
        LegendKind::Irelia => {
            "Irelia can turn choosing a friendly unit into another ready. Plan a useful destination before paying for it, and keep energy for a conquest reset."
        }
        LegendKind::MasterYi => {
            "Yi gives a lone defender +2 might. Adding another friendly unit can remove that bonus, so compare both totals before reinforcing a field."
        }
        LegendKind::Kennen => {
            "Kennen empowers when you play from outside your hand. Spend that empowerment for Assault when the extra attacking might changes the point."
        }
        LegendKind::Darius => {
            "Darius adds energy once Legion is enabled by a card you've played this turn. Keep that resource ability available for a later play or response."
        }
        LegendKind::Viktor => {
            "Viktor trades 1 energy and his ready state for a Recruit in your open Main Phase. Choose a useful location after the ability resolves."
        }
        LegendKind::Lux => {
            "Lux draws after you play a spell with printed energy cost 5 or more. Discounts still help you reach that payoff while keeping resources for responses."
        }
        LegendKind::KaiSa => {
            "Kai'Sa's legend adds flexible power for spells. Use it to preserve matching runes; it cannot pay for units or ability costs."
        }
    }
}

/// Draws a tiny floating fox-light spirit using physical screen coordinates.
/// It intentionally does not switch cameras or capture pointer input.
pub fn draw_mote(center: Vec2, scale: f32, time: f32, accent: Color) {
    let s = scale.max(0.01);
    let c = center + vec2(0., (time * 1.8).sin() * 3.2 * s);
    let alpha = |color: Color, a: f32| Color { a, ..color };
    let glow = Color::new(0.45, 0.94, 0.82, 1.);
    for i in (1..=5).rev() {
        draw_circle(
            c.x,
            c.y + 2. * s,
            (21. + i as f32 * 6.) * s,
            alpha(accent, 0.012),
        );
    }
    draw_ellipse(
        center.x,
        center.y + 31. * s,
        20. * s,
        3.3 * s,
        0.,
        Color::new(0.02, 0.05, 0.07, 0.45),
    );

    // A tapered wispy tail and two oversized ears give the light a fox silhouette.
    draw_triangle(
        c + vec2(5., 17.) * s,
        c + vec2(24., 10.) * s,
        c + vec2(10., 33.) * s,
        alpha(accent, 0.7),
    );
    draw_triangle(
        c + vec2(-18., -9.) * s,
        c + vec2(-16., -30.) * s,
        c + vec2(-3., -16.) * s,
        accent,
    );
    draw_triangle(
        c + vec2(3., -16.) * s,
        c + vec2(16., -30.) * s,
        c + vec2(18., -9.) * s,
        accent,
    );
    draw_triangle(
        c + vec2(-13., -12.) * s,
        c + vec2(-13., -23.) * s,
        c + vec2(-6., -15.) * s,
        glow,
    );
    draw_triangle(
        c + vec2(6., -15.) * s,
        c + vec2(13., -23.) * s,
        c + vec2(13., -12.) * s,
        glow,
    );
    draw_ellipse(c.x, c.y, 22. * s, 22.5 * s, 0., accent);
    draw_ellipse(c.x, c.y + 3.5 * s, 15. * s, 16.5 * s, 0., glow);
    draw_ellipse(c.x - 7.5 * s, c.y - 6. * s, 8. * s, 6. * s, -15., glow);
    draw_ellipse(c.x + 7.5 * s, c.y - 6. * s, 8. * s, 6. * s, 15., glow);

    let ink = Color::new(0.03, 0.20, 0.22, 1.);
    let blink = (time.rem_euclid(6.3) - 5.9).abs() < 0.1;
    for side in [-1., 1.] {
        let eye = c + vec2(side * 7., -1.) * s;
        if blink {
            draw_line(eye.x - 2.4 * s, eye.y, eye.x + 2.4 * s, eye.y, 1.5 * s, ink);
        } else {
            draw_ellipse(eye.x, eye.y, 2.25 * s, 3.3 * s, 0., ink);
            draw_circle(eye.x - 0.5 * s, eye.y - 1.2 * s, 0.8 * s, WHITE);
        }
        draw_ellipse(
            c.x + side * 13. * s,
            c.y + 5.5 * s,
            3.4 * s,
            1.7 * s,
            0.,
            Color::new(1., 0.73, 0.69, 0.55),
        );
    }
    draw_triangle(
        c + vec2(-2., 5.) * s,
        c + vec2(2., 5.) * s,
        c + vec2(0., 7.) * s,
        ink,
    );
    draw_line(c.x, c.y + 7. * s, c.x - 3. * s, c.y + 9. * s, s, ink);
    draw_line(c.x, c.y + 7. * s, c.x + 3. * s, c.y + 9. * s, s, ink);
    let star = c + vec2(0., -12.) * s;
    draw_poly(
        star.x,
        star.y,
        4,
        3.5 * s,
        0.,
        Color::new(1., 0.91, 0.62, 1.),
    );
    for i in 0..3 {
        let a = time * 0.45 + i as f32 * std::f32::consts::TAU / 3.;
        let p = c + vec2(a.cos() * 36., a.sin() * 25. - 4.) * s;
        draw_circle(p.x, p.y, 1.4 * s, alpha(glow, 0.55));
        if i == 0 {
            draw_line(
                p.x - 3. * s,
                p.y,
                p.x + 3. * s,
                p.y,
                0.8 * s,
                alpha(glow, 0.5),
            );
            draw_line(
                p.x,
                p.y - 3. * s,
                p.x,
                p.y + 3. * s,
                0.8 * s,
                alpha(glow, 0.5),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use local_rift::game::{Difficulty, Unit};

    fn game() -> Game {
        Game::new(0, 1, 72, Difficulty::Easy)
    }

    #[test]
    fn sprite_placement_coaching_explains_expiry_and_never_promises_a_hold() {
        let mut game = Game::new(3, 3, 72, Difficulty::Easy);
        game.start_match().unwrap();
        game.players[0].hand = vec![355];
        game.players[0].energy = 10;
        game.players[0].energy_pool = 10;
        game.players[0].runes = [0; 6];
        game.players[0].exhausted_runes = [0; 6];
        game.apply(local_rift::game::Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        })
        .unwrap();
        while game.decision.is_none() {
            game.apply(local_rift::game::Action::Pass).unwrap();
        }
        assert!(matches!(
            game.decision,
            Some(Decision::TokenPlacement { .. } | Decision::TokenPlay { .. })
        ));
        for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
            let tips = advice(&game, level);
            assert!(tips[0].contains("Temporary"));
            assert!(tips[0].contains("before hold scoring"));
            assert!(tips.iter().all(|tip| tip.chars().count() <= 180));
            let mut other = game.clone();
            other.players[1].hand.reverse();
            other.players[1].deck.reverse();
            assert_eq!(tips, advice(&other, level));
            assert_eq!(fingerprint(&game), fingerprint(&other));
        }
    }

    #[test]
    fn invalid_numeric_state_pauses_tactical_advice_without_hidden_information() {
        let mut game = game();
        game.start_match().unwrap();
        game.players[0].units = vec![Unit {
            id: 999,
            card_id: 69,
            lane: None,
            ready: true,
            damage: 0,
            buff: u64::MAX,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        }];
        for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
            let tips = advice(&game, level);
            assert_eq!(tips.len(), 1);
            assert!(tips[0].contains("pause tactical advice"));
            let mut hidden = game.clone();
            hidden.players[1].hand.reverse();
            hidden.players[1].deck.reverse();
            assert_eq!(tips, advice(&hidden, level));
        }
    }
    #[test]
    fn xp_entry_discount_and_history_advice_uses_public_events_and_stays_private() {
        use local_rift::game::Action;
        let mut game = game();
        game.start_match().unwrap();
        game.players[0].hand = vec![334, 336];
        game.players[0].xp = 255;
        game.players[0].units = [342, 343]
            .into_iter()
            .enumerate()
            .map(|(index, card_id)| Unit {
                id: 980 + index as u32,
                card_id,
                lane: None,
                ready: true,
                damage: 0,
                buff: 0,
                temporary_buff: 0,
                token: false,
                protected: false,
                stunned: false,
                banish_on_death: false,
            })
            .collect();
        let beginner = advice(&game, AdviceLevel::Beginner);
        assert!(
            beginner
                .iter()
                .any(|tip| tip.contains("whether you gained XP this turn"))
        );
        assert!(
            beginner
                .iter()
                .any(|tip| tip.contains("initial state, not a later Ready effect"))
        );
        assert!(
            beginner
                .iter()
                .any(|tip| tip.contains("highest reached Level discount"))
        );
        game.apply(Action::Move {
            unit_id: 981,
            lane: Some(0),
        })
        .unwrap();
        while !game.chain.is_empty() {
            game.apply(Action::Pass).unwrap();
        }
        assert_eq!(game.players[0].xp, 257);
        assert!(game.gained_xp_this_turn(0));
        let mut hidden = game.clone();
        hidden.players[1].hand = vec![334, 335, 336];
        hidden.players[1].deck.reverse();
        assert_eq!(fingerprint(&game), fingerprint(&hidden));
        for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
            let tips = advice(&game, level);
            assert_eq!(tips, advice(&hidden, level));
            assert!(tips.iter().all(|tip| tip.chars().count() <= 180));
        }
        assert_ne!(beginner, advice(&game, AdviceLevel::Advanced));
    }

    #[test]
    fn xp_cost_advice_distinguishes_spending_output_and_bearer_effects() {
        use local_rift::gear::GearState;
        let mut game = game();
        game.start_match().unwrap();
        game.players[0].xp = 6;
        game.players[0].units = vec![Unit {
            id: 986,
            card_id: 345,
            lane: None,
            ready: false,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        }];
        game.players[0].gear = [347, 348]
            .into_iter()
            .enumerate()
            .map(|(index, card_id)| GearState {
                id: 987 + index as u32,
                card_id,
                ready: true,
                attached_to: None,
                temporary: false,
                token: false,
            })
            .collect();
        let beginner = advice(&game, AdviceLevel::Beginner);
        assert!(
            beginner
                .iter()
                .any(|tip| tip.contains("XP before responses"))
        );
        assert!(beginner.iter().any(|tip| tip.contains("Neither spends XP")));
        assert!(
            beginner
                .iter()
                .any(|tip| tip.contains("+2 Might belongs to the bearer"))
        );
        let mut hidden = game.clone();
        hidden.players[1].hand = vec![345, 346, 347];
        hidden.players[1].deck.reverse();
        for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
            let tips = advice(&game, level);
            assert_eq!(tips, advice(&hidden, level));
            assert!(tips.iter().all(|tip| tip.chars().count() <= 180));
        }
        assert_ne!(beginner, advice(&game, AdviceLevel::Advanced));
        game.apply(local_rift::game::Action::BeginPayment {
            action: Box::new(local_rift::game::Action::ActivateAbility {
                source: local_rift::activated::AbilitySource::Unit {
                    player: 0,
                    unit_id: 986,
                },
                ability_index: 0,
                targets: vec![],
            }),
        })
        .unwrap();
        assert_eq!(game.players[0].xp, 6);
        for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
            let tips = advice(&game, level);
            assert!(tips[0].contains("not XP"));
            assert!(tips.iter().all(|tip| tip.chars().count() <= 180));
        }
    }

    #[test]
    fn live_hunt_buff_and_level_advice_is_contextual_and_uses_no_hidden_cards() {
        use local_rift::gear::GearState;
        let mut game = game();
        game.start_match().unwrap();
        game.players[0].units = [22, 321, 325, 326]
            .into_iter()
            .enumerate()
            .map(|(index, card_id)| Unit {
                id: 800 + index as u32,
                card_id,
                lane: None,
                ready: true,
                damage: 0,
                buff: 0,
                temporary_buff: 0,
                token: false,
                protected: false,
                stunned: false,
                banish_on_death: false,
            })
            .collect();
        game.players[0].gear = vec![GearState {
            id: 900,
            card_id: 331,
            attached_to: None,
            ready: true,
            temporary: true,
            token: false,
        }];
        game.players[0].hand = vec![308];
        let before = fingerprint(&game);
        game.buffed_units.push(801);
        assert_ne!(
            before,
            fingerprint(&game),
            "a public Buff counter refreshes coaching"
        );
        let beginner = advice(&game, AdviceLevel::Beginner);
        assert!(
            beginner
                .iter()
                .any(|tip| tip.contains("Printed Hunt and equipped Hunt add together"))
        );
        assert!(
            beginner
                .iter()
                .any(|tip| tip.contains("Buffed means having a Buff counter"))
        );
        assert!(
            beginner
                .iter()
                .any(|tip| tip.contains("ready and exhausted runes in play"))
        );
        assert!(
            beginner
                .iter()
                .any(|tip| tip.contains("Level checks your current XP"))
        );
        assert!(beginner.iter().any(|tip| tip.contains("Temporary gear")));
        assert!(beginner.iter().any(|tip| tip.contains("including yours")));
        let mut hidden_changed = game.clone();
        hidden_changed.players[1].hand = vec![308, 310, 313];
        hidden_changed.players[1].deck.reverse();
        assert_eq!(fingerprint(&game), fingerprint(&hidden_changed));
        for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
            let tips = advice(&game, level);
            assert_eq!(tips, advice(&hidden_changed, level));
            assert!(tips.iter().all(|tip| tip.chars().count() <= 180));
        }
        assert_ne!(beginner, advice(&game, AdviceLevel::Advanced));
        game.players[0].gear[0].attached_to = Some(800);
        assert!(
            !advice(&game, AdviceLevel::Beginner)
                .iter()
                .any(|tip| tip.contains("Temporary gear"))
        );
        let before_grant = fingerprint(&game);
        game.players[0].hand = vec![66];
        game.players[0].runes = [0; 6];
        game.players[0].runes[local_rift::game::domain_index(local_rift::cards::Domain::Mind)] = 6;
        game.players[0].exhausted_runes = [0; 6];
        game.players[0].energy = 6;
        game.apply(local_rift::game::Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Gear {
                player: 0,
                gear_id: 900,
            }],
        })
        .unwrap();
        while !game.chain.is_empty() {
            game.apply(local_rift::game::Action::Pass).unwrap();
        }
        assert_ne!(before_grant, fingerprint(&game));
        let tips = advice(&game, AdviceLevel::Beginner);
        assert!(
            tips.iter()
                .any(|tip| tip.contains("Granted Temporary stays active when attached"))
        );
        assert!(
            advice(&game, AdviceLevel::Advanced)
                .iter()
                .any(|tip| tip.contains("Attachment will not suppress granted Temporary"))
        );
        assert!(tips.iter().all(|tip| tip.chars().count() <= 180));
    }

    #[test]
    fn waiting_repeat_and_nami_advice_uses_only_public_grants() {
        use local_rift::{
            game::ChainSource,
            play::{PlayGrant, PlayGrantKind},
        };
        let mut game = game();
        game.start_match().unwrap();
        game.play_grants = vec![
            PlayGrant {
                id: 1,
                player: 0,
                source: ChainSource::Card { card_id: 256 },
                expires_turn: game.turn,
                kind: PlayGrantKind::RepeatNextSpell,
            },
            PlayGrant {
                id: 2,
                player: 0,
                source: ChainSource::Card { card_id: 261 },
                expires_turn: game.turn,
                kind: PlayGrantKind::ReadyBuffNextUnit,
            },
        ];
        let beginner = advice(&game, AdviceLevel::Beginner);
        assert!(
            beginner
                .iter()
                .any(|tip| tip.contains("does not play the spell twice"))
        );
        assert!(
            beginner
                .iter()
                .any(|tip| tip.contains("Entering ready happens earlier"))
        );
        let advanced = advice(&game, AdviceLevel::Advanced);
        assert!(
            advanced
                .iter()
                .any(|tip| tip.contains("Repeat sources are independent"))
        );
        for tip in beginner.iter().chain(&advanced) {
            assert!(tip.chars().count() <= 180);
        }
        let before = fingerprint(&game);
        game.players[1].hand.reverse();
        game.players[1].deck.reverse();
        assert_eq!(fingerprint(&game), before);
        assert_eq!(advice(&game, AdviceLevel::Beginner), beginner);
        game.play_grants.clear();
        assert!(
            !advice(&game, AdviceLevel::Beginner)
                .iter()
                .any(|tip| tip.contains("Nami's waiting effect"))
        );
    }

    #[test]
    fn battlefield_spell_advice_uses_public_targets_without_opponent_hand_access() {
        let mut game = game();
        game.start_match().unwrap();
        game.players[0].hand = vec![299];
        let beginner = advice(&game, AdviceLevel::Beginner);
        let advanced = advice(&game, AdviceLevel::Advanced);
        assert!(
            beginner
                .iter()
                .any(|tip| tip.contains("chooses a battlefield, not its units"))
        );
        assert!(
            advanced
                .iter()
                .any(|tip| tip.contains("Each Repeat can choose its own field"))
        );
        let original = fingerprint(&game);
        game.players[1].hand.fill(299);
        game.players[1].deck.reverse();
        assert_eq!(original, fingerprint(&game));
        assert_eq!(beginner, advice(&game, AdviceLevel::Beginner));
        game.players[0].energy_pool = 20;
        game.players[0].energy = 22;
        game.players[0].any_power = 10;
        game.apply(local_rift::game::Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Battlefield { lane: 0 }],
        })
        .unwrap();
        let first = fingerprint(&game);
        let spell = game
            .chain
            .iter_mut()
            .find(|item| item.card_id == 299 && item.ability.is_none())
            .unwrap();
        spell.target = Some(Target::Battlefield { lane: 1 });
        spell.targets = vec![Target::Battlefield { lane: 1 }];
        assert_ne!(
            first,
            fingerprint(&game),
            "public battlefield target changes refresh advice"
        );
    }

    #[test]
    fn gear_empower_advice_distinguishes_targetless_effect_and_paid_cost() {
        let mut game = game();
        game.start_match().unwrap();
        game.players[0].gear.push(local_rift::gear::GearState {
            id: 800,
            card_id: 250,
            attached_to: None,
            ready: true,
            temporary: false,
            token: false,
        });
        game.players[0].gear.push(local_rift::gear::GearState {
            id: 801,
            card_id: 243,
            attached_to: None,
            ready: false,
            temporary: false,
            token: false,
        });
        game.empowered_units.push(801);
        let beginner = advice(&game, AdviceLevel::Beginner);
        assert!(
            !beginner
                .iter()
                .any(|tip| tip.contains("keeps Empowered while exhausted")),
            "unrelated Empowered gear must not change Tome advice"
        );
        assert!(
            beginner
                .iter()
                .any(|tip| tip.contains("not Empowered") && tip.contains("chooses no target"))
        );
        game.empowered_units.push(800);
        game.players[0].gear[0].ready = false;
        let empowered = advice(&game, AdviceLevel::Beginner);
        let advanced = advice(&game, AdviceLevel::Advanced);
        assert!(
            empowered
                .iter()
                .any(|tip| tip.contains("spent before responses"))
        );
        assert!(
            empowered
                .iter()
                .any(|tip| tip.contains("keeps Empowered while exhausted"))
        );
        assert!(
            advanced
                .iter()
                .any(|tip| tip.contains("counter") && tip.contains("does not restore"))
        );
        assert!(
            beginner
                .iter()
                .chain(&empowered)
                .chain(&advanced)
                .all(|tip| tip.chars().count() <= 180)
        );
    }

    #[test]
    fn renamed_custom_decks_keep_their_legend_advice() {
        use local_rift::deck::CustomDeck;
        for legend in local_rift::legend_catalog::legends() {
            let deck = CustomDeck::from_legend(legend.id).unwrap();
            let original = Game::new_with_custom(
                [0, 1],
                [Some(deck.clone()), None],
                42,
                Difficulty::Easy,
                true,
            )
            .unwrap();
            let expected = archetype_tip(&original);
            assert!(expected.len() <= 180);
            let mut renamed = deck;
            renamed.name = "My quiet experiment".into();
            let custom =
                Game::new_with_custom([0, 1], [Some(renamed), None], 42, Difficulty::Easy, true)
                    .unwrap();
            assert_eq!(archetype_tip(&custom), expected);
            assert_eq!(custom.legend_id(0), legend.id);
            assert!(!custom.players[0].name.starts_with(legend.champion));
        }
    }

    #[test]
    fn levels_offer_distinct_contextual_opening_advice() {
        let game = game();
        let mut beginner = Companion::new(AdviceLevel::Beginner);
        let mut advanced = Companion::new(AdviceLevel::Advanced);
        beginner.observe(&game);
        advanced.observe(&game);
        assert!(beginner.message().contains("replace up to two"));
        assert!(advanced.message().contains("first two turns"));
        assert_ne!(beginner.message(), advanced.message());
        beginner.set_level(AdviceLevel::Advanced);
        beginner.observe(&game);
        assert_eq!(beginner.message(), advanced.message());
    }

    #[test]
    fn enemy_hidden_cards_never_change_advice_or_observation() {
        let mut first = game();
        first.start_match().unwrap();
        let mut second = first.clone();
        second.players[1].hand = vec![65; 30];
        second.players[1].deck.reverse();
        second.players[1].rune_deck.reverse();
        assert_eq!(fingerprint(&first), fingerprint(&second));
        for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
            assert_eq!(advice(&first, level), advice(&second, level));
        }
    }

    #[test]
    fn final_point_threat_takes_priority_and_updates_after_change() {
        let mut game = game();
        game.start_match().unwrap();
        game.players[1].score = 7;
        game.battlefield_owner[1] = Some(1);
        let mut mote = Companion::default();
        mote.observe(&game);
        assert!(mote.message().contains("one point away"));
        game.players[1].score = 4;
        game.battlefield_owner[1] = None;
        mote.observe(&game);
        assert!(!mote.message().contains("one point away"));
    }

    #[test]
    fn manual_tip_does_not_flicker_on_unchanged_frames() {
        let game = game();
        let mut mote = Companion::default();
        mote.observe(&game);
        let first = mote.message().to_owned();
        mote.next_tip(&game);
        let second = mote.message().to_owned();
        assert_ne!(first, second);
        for _ in 0..10 {
            mote.observe(&game);
            assert_eq!(mote.message(), second);
        }
    }

    #[test]
    fn advice_is_short_and_explains_scoring_pressure() {
        for deck in 0..4 {
            let mut game = Game::new(deck, (deck + 1) % 4, 17, Difficulty::Tactical);
            for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
                for tip in advice(&game, level) {
                    assert!(tip.chars().count() <= 180, "Too long: {tip}");
                }
            }
            game.start_match().unwrap();
            game.players[0].hand = (0..game.catalog.len()).collect();
            game.players[0].score = 7;
            game.players[1].score = 7;
            for player in 0..2 {
                game.players[player].units.push(Unit {
                    id: 100 + player as u32,
                    card_id: 0,
                    lane: Some(0),
                    ready: false,
                    damage: 0,
                    buff: 0,
                    temporary_buff: 0,
                    token: false,
                    protected: false,
                    stunned: false,
                    banish_on_death: false,
                });
            }
            for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
                let tips = advice(&game, level);
                assert!(
                    tips.iter().any(|s| s.contains("At 7 points")),
                    "Scoring pressure needs useful advice: {tips:?}"
                );
                for tip in tips {
                    assert!(tip.chars().count() <= 180, "Too long: {tip}");
                }
            }
        }
    }

    #[test]
    fn priority_windows_override_turn_advice_and_track_passes() {
        use local_rift::game::{ChainItem, Showdown};
        let mut game = game();
        game.start_match().unwrap();
        let mut mote = Companion::default();
        game.chain.push(ChainItem {
            id: 77,
            player: 1,
            card_id: 11,
            target: None,
            targets: Vec::new(),
            ability: None,
            recycle_after: false,
            banish_after: false,
            from_hidden: false,
            options: Default::default(),
            repeat_targets: Vec::new(),
            executions: vec![Default::default()],
            play_origin: None,
            bonus_damage: 0,
            lane: None,
        });
        mote.observe(&game);
        assert!(mote.message().contains("You have priority"));
        let turn = game.turn;
        game.chain.clear();
        game.showdown = Some(Showdown {
            combat: false,
            lane: 0,
            attacker: 0,
            focus: 0,
            passes: 0,
        });
        mote.observe(&game);
        assert!(mote.message().contains("showdown is open"));
        game.showdown.as_mut().unwrap().passes = 1;
        mote.observe(&game);
        assert!(mote.message().contains("opponent passed focus"));
        assert_eq!(game.turn, turn);
    }

    #[test]
    fn private_opponent_predict_identity_is_not_observed() {
        let mut first = game();
        first.start_match().unwrap();
        first.decision = Some(Decision::Predict {
            player: 1,
            card_id: 1,
        });
        let mut second = first.clone();
        second.decision = Some(Decision::Predict {
            player: 1,
            card_id: 65,
        });
        assert_eq!(fingerprint(&first), fingerprint(&second));
        for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
            assert_eq!(advice(&first, level), advice(&second, level));
        }
        first.decision = Some(Decision::Predict {
            player: 0,
            card_id: 1,
        });
        assert!(advice(&first, AdviceLevel::Beginner)[0].contains("Predict"));
        first.decision = Some(Decision::CounterTax {
            player: 0,
            chain_id: 77,
            cost: 2,
        });
        assert!(advice(&first, AdviceLevel::Beginner)[0].contains("pay extra energy"));
    }

    #[test]
    fn private_predict_many_keeps_identities_and_order_out_of_advice() {
        let mut first = game();
        first.start_match().unwrap();
        first.decision = Some(Decision::PredictCards {
            player: 1,
            choice_id: 71,
            cards: vec![272, 273],
        });
        let mut second = first.clone();
        second.decision = Some(Decision::PredictCards {
            player: 1,
            choice_id: 71,
            cards: vec![288, 279],
        });
        assert_eq!(fingerprint(&first), fingerprint(&second));
        for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
            assert_eq!(advice(&first, level), advice(&second, level));
        }
        second.decision = Some(Decision::PredictCards {
            player: 0,
            choice_id: 72,
            cards: vec![272, 273],
        });
        assert!(advice(&second, AdviceLevel::Beginner)[0].contains("Predict"));
    }

    #[test]
    fn private_opponent_top_three_choices_are_not_observed() {
        let mut first = game();
        first.start_match().unwrap();
        first.decision = Some(Decision::TopThree {
            player: 1,
            cards: vec![1, 2, 3],
        });
        let mut second = first.clone();
        second.decision = Some(Decision::TopThree {
            player: 1,
            cards: vec![61, 62, 63],
        });
        assert_eq!(fingerprint(&first), fingerprint(&second));
        for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
            assert_eq!(advice(&first, level), advice(&second, level));
        }
    }

    #[test]
    fn enemy_facedown_identity_is_private_until_explicitly_revealed() {
        use local_rift::hidden::HiddenCard;
        let mut first = game();
        first.start_match().unwrap();
        first.players[1].hidden.push(HiddenCard {
            id: 99,
            card_id: 21,
            lane: 0,
            hidden_turn: 0,
            revealed: false,
        });
        let mut second = first.clone();
        second.players[1].hidden[0].card_id = 16;
        assert_eq!(fingerprint(&first), fingerprint(&second));
        for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
            assert_eq!(advice(&first, level), advice(&second, level));
        }
        first.players[1].hidden[0].revealed = true;
        second.players[1].hidden[0].revealed = true;
        assert_ne!(fingerprint(&first), fingerprint(&second));
    }

    #[test]
    fn scuttle_information_is_observed_only_while_its_permission_is_active() {
        use local_rift::hidden::HiddenCard;
        let mut first = game();
        first.start_match().unwrap();
        first.players[1].hidden.push(HiddenCard {
            id: 98,
            card_id: 21,
            lane: 0,
            hidden_turn: 0,
            revealed: false,
        });
        let mut second = first.clone();
        second.players[1].hidden[0].card_id = 16;
        assert_eq!(fingerprint(&first), fingerprint(&second));
        first.hidden_view[0] = true;
        second.hidden_view[0] = true;
        assert_ne!(fingerprint(&first), fingerprint(&second));
        first.hidden_view[0] = false;
        second.hidden_view[0] = false;
        assert_eq!(fingerprint(&first), fingerprint(&second));
        first.decision = Some(Decision::RevealHand {
            player: 0,
            opponent: 1,
            cards: vec![1],
        });
        second.decision = Some(Decision::RevealHand {
            player: 0,
            opponent: 1,
            cards: vec![65],
        });
        assert_ne!(fingerprint(&first), fingerprint(&second));
        assert!(advice(&first, AdviceLevel::Beginner)[0].contains("Scuttle Crab"));
    }

    #[test]
    fn damage_assignment_advice_is_separate_from_chain_priority() {
        let mut game = game();
        game.start_match().unwrap();
        game.decision = Some(Decision::CombatDamage {
            player: 0,
            lane: 0,
            total: 4,
        });
        assert!(advice(&game, AdviceLevel::Beginner)[0].contains("Tank first"));
        assert!(advice(&game, AdviceLevel::Advanced)[0].contains("next point"));
        assert!(
            advice(&game, AdviceLevel::Beginner)
                .iter()
                .any(|tip| tip.contains("simultaneously"))
        );
        game.decision = Some(Decision::CombatDamage {
            player: 1,
            lane: 0,
            total: 4,
        });
        assert!(advice(&game, AdviceLevel::Beginner)[0].contains("opponent is making a choice"));
    }

    #[test]
    fn combined_tank_backline_advice_preserves_both_legal_choices() {
        let mut game = game();
        game.start_match().unwrap();
        game.players[1].units.push(Unit {
            id: 991,
            card_id: 238,
            lane: Some(0),
            ready: true,
            damage: 0,
            buff: 0,
            temporary_buff: 0,
            token: false,
            protected: false,
            stunned: false,
            banish_on_death: false,
        });
        game.players[1].gear.push(local_rift::gear::GearState {
            id: 992,
            card_id: 121,
            ready: true,
            attached_to: Some(991),
            temporary: false,
            token: false,
        });
        game.decision = Some(Decision::CombatDamage {
            player: 0,
            lane: 0,
            total: 8,
        });
        for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
            let tips = advice(&game, level);
            assert!(tips[0].contains("You choose first or last"));
            assert!(tips.iter().all(|tip| tip.chars().count() <= 180));
            assert!(!tips.iter().any(|tip| tip.contains("Assign Tank first")));
        }
    }

    #[test]
    fn every_window_tip_fits_the_companion_panel() {
        use local_rift::game::{ChainItem, Showdown};
        let mut game = game();
        game.start_match().unwrap();
        let mut states = Vec::new();
        for player in 0..2 {
            game.decision = Some(Decision::Predict { player, card_id: 1 });
            states.push(game.clone());
            game.decision = Some(Decision::CounterTax {
                player,
                chain_id: 1,
                cost: 2,
            });
            states.push(game.clone());
            for decision in [
                Decision::PredictCards {
                    player,
                    choice_id: 73,
                    cards: vec![272, 273],
                },
                Decision::Payment { player },
                Decision::ResourceConversion {
                    player,
                    source: local_rift::activated::AbilitySource::Gear {
                        player,
                        gear_id: 90,
                    },
                    card_id: 244,
                    ability_index: 0,
                    direction: local_rift::activated::ConversionDirection::EnergyToPower,
                },
                Decision::TriggerTargets {
                    player,
                    card_id: 188,
                    rule_index: 0,
                    context: local_rift::game::TriggerContext {
                        departure_location: None,
                        movement: None,
                        source: local_rift::game::TriggerSource {
                            player,
                            unit_id: Some(1),
                            gear_id: None,
                        },
                        subject: None,
                    },
                },
                Decision::BattlefieldOrder {
                    player,
                    lanes: vec![0, 1],
                    combat: false,
                },
                Decision::BattlefieldOrder {
                    player,
                    lanes: vec![0, 1],
                    combat: true,
                },
                Decision::CombatDamage {
                    player,
                    lane: 0,
                    total: 4,
                },
                Decision::Legend {
                    player,
                    trigger: LegendTrigger::IreliaChoose { unit_id: 1 },
                },
                Decision::Legend {
                    player,
                    trigger: LegendTrigger::IreliaConquer,
                },
                Decision::LastRites { player, gear_id: 1 },
                Decision::ReplaySpell { player },
                Decision::TrashTarget {
                    player,
                    source_card_id: 36,
                    gear_id: None,
                },
                Decision::PendingPlay {
                    player,
                    pending_id: 1,
                    card_id: 133,
                    kind: local_rift::game::InstructedPlayKind::Fizz,
                },
                Decision::PendingPlay {
                    player,
                    pending_id: 2,
                    card_id: 110,
                    kind: local_rift::game::InstructedPlayKind::LastRites,
                },
                Decision::PendingPlay {
                    player,
                    pending_id: 3,
                    card_id: 96,
                    kind: local_rift::game::InstructedPlayKind::Matriarch,
                },
                Decision::GrantFlow { player },
                Decision::Battlefield { player, lane: 0 },
                Decision::ReviveUnit { player },
                Decision::RevivePlay {
                    player,
                    trash_index: 0,
                    card_id: 96,
                },
                Decision::SwapUnit { player, unit_id: 1 },
                Decision::EmpowerHere {
                    player,
                    lane: Some(0),
                    source_id: None,
                },
                Decision::Discard {
                    player,
                    count: 1,
                    draw_after: 2,
                },
                Decision::TopThree {
                    player,
                    cards: vec![1, 2, 3],
                },
                Decision::Sabotage {
                    player,
                    opponent: 1 - player,
                },
                Decision::RevealHand {
                    player,
                    opponent: 1 - player,
                    cards: vec![1, 2],
                },
                Decision::BurnForMight { player, unit_id: 1 },
            ] {
                game.decision = Some(decision);
                states.push(game.clone());
            }
        }
        game.decision = None;
        game.chain.push(ChainItem {
            id: 1,
            player: 1,
            card_id: 11,
            target: Some(Target::Unit {
                player: 0,
                unit_id: 1,
            }),
            targets: vec![Target::Unit {
                player: 0,
                unit_id: 1,
            }],
            ability: None,
            recycle_after: false,
            banish_after: false,
            from_hidden: false,
            options: Default::default(),
            repeat_targets: Vec::new(),
            executions: vec![Default::default()],
            play_origin: None,
            bonus_damage: 0,
            lane: None,
        });
        states.push(game.clone());
        game.chain.clear();
        for passes in 0..=1 {
            game.showdown = Some(Showdown {
                combat: false,
                lane: 0,
                attacker: 0,
                focus: 0,
                passes,
            });
            states.push(game.clone());
        }
        for state in states {
            for level in [AdviceLevel::Beginner, AdviceLevel::Advanced] {
                for tip in advice(&state, level) {
                    assert!(tip.chars().count() <= 180, "Too long: {tip}");
                }
            }
        }
    }
}
