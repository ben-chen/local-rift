//! Bounded, deterministic planning from the acting player's information.
//!
//! Rollouts settle the current chain/combat assuming no further discretionary
//! responses. Unknown draws are sampled from public legend legality, never from
//! the real opponent's list, hand, facedown cards, or either deck's order.
//! This is a practice opponent, not a claim of tournament-level strength.
use super::*;

const ROOT_LIMIT: usize = 48;
const FOLLOWUP_LIMIT: usize = 10;
const SETTLE_LIMIT: usize = 48;

impl Game {
    pub(super) fn choose_strategic_action(&self) -> Result<Action, String> {
        Ok({
            // A suspended transaction retains its rollback snapshot. Follow the
            // public-resource witness rather than resampling only half that state.
            if self.payment_plan.is_some() || !self.resource_frames.is_empty() {
                return self.choose_heuristic_action();
            }
            if self.ai_steps_this_turn >= 160 && !self.in_window() && self.decision.is_none() {
                return Ok(Action::EndTurn);
            }
            let actor = self.priority();
            let belief = self.strategic_belief(actor);
            let mut choices: Vec<_> = belief
                .strategic_candidates(ROOT_LIMIT)?
                .into_iter()
                .filter_map(|action| {
                    let mut result = belief.clone();
                    result.apply(action.clone()).ok()?;
                    result.strategic_settle(belief.turn).ok()?;
                    // Compare EndTurn at the same horizon as other actions. Letting
                    // only this candidate include the opponent's next Hold would
                    // reward pointless actions merely for postponing those points.
                    let score = if action == Action::EndTurn {
                        belief.strategic_value(actor).ok()?
                    } else {
                        result.strategic_value(actor).ok()?
                    } - action_penalty(&action);
                    Some((score, action, result))
                })
                .collect();
            choices.sort_by_key(|choice| std::cmp::Reverse(choice.0));
            // A second own action catches sequences such as ready-then-attack or
            // resource-ability-then-play. The small beam bounds UI work per choice.
            for choice in choices.iter_mut().take(2) {
                let state = &choice.2;
                if state.winner.is_none()
                    && state.turn == belief.turn
                    && state.priority() == actor
                    && state.decision.is_none()
                    && !state.in_window()
                {
                    let mut best = choice.0;
                    for next in state.strategic_candidates(FOLLOWUP_LIMIT)? {
                        let mut result = state.clone();
                        if result.apply(next.clone()).is_ok() {
                            if result.strategic_settle(belief.turn).is_err() {
                                continue;
                            }
                            best = best.max(
                                (if next == Action::EndTurn {
                                    state.strategic_value(actor)?
                                } else {
                                    match result.strategic_value(actor) {
                                        Ok(value) => value,
                                        Err(_) => continue,
                                    }
                                }) - action_penalty(&choice.1)
                                    - action_penalty(&next),
                            );
                        }
                    }
                    choice.0 = (choice.0 * 2 + best) / 3;
                }
            }
            choices.sort_by_key(|choice| std::cmp::Reverse(choice.0));
            choices
                .into_iter()
                .map(|(_, action, _)| action)
                // The belief state is hypothetical; the final action still goes
                // through the exact engine's legality checks on the live position.
                .find(|action| self.clone().apply(action.clone()).is_ok())
                .map(Ok)
                .unwrap_or_else(|| self.choose_heuristic_action())?
        })
    }

    fn strategic_belief(&self, actor: usize) -> Self {
        let mut state = self.clone();
        let mut seed = 0x6a09e667f3bcc909u64 ^ self.turn as u64 ^ ((actor as u64) << 32);
        for &id in &self.players[actor].hand {
            seed = seed.rotate_left(7).wrapping_add(id as u64 + 1);
        }
        for player in &self.players {
            seed = seed.rotate_left(9).wrapping_add(player.score as u64);
            for unit in &player.units {
                seed = seed.rotate_left(5).wrapping_add(unit.id as u64);
            }
        }
        let mut rng = Rng::new(seed);
        for owner in 0..2 {
            let domains = self.legend_def(owner).domains;
            let pool: Vec<_> = crate::deck::eligible_cards(self.legend_id(owner))
                .into_iter()
                .filter(|&id| self.card_supported(id))
                .collect();
            let sample = |rng: &mut Rng| pool[rng.next() as usize % pool.len()];
            for id in &mut state.players[owner].deck {
                *id = sample(&mut rng);
            }
            for domain in &mut state.players[owner].rune_deck {
                *domain = domains[rng.next() as usize % domains.len()];
            }
            let hand_visible = owner == actor
                || matches!(self.decision,
                Some(Decision::Sabotage { opponent, .. } | Decision::RevealHand { opponent, .. })
                    if opponent == owner);
            if !hand_visible {
                for id in &mut state.players[owner].hand {
                    *id = sample(&mut rng);
                }
            }
            let hidden_pool: Vec<_> = pool
                .iter()
                .copied()
                .filter(|&id| self.catalog[id].keywords.contains(&"Hidden"))
                .collect();
            for hidden in &mut state.players[owner].hidden {
                if self.hidden_visible_card(actor, owner, hidden.id).is_none() {
                    let pool = if hidden_pool.is_empty() {
                        &pool
                    } else {
                        &hidden_pool
                    };
                    hidden.card_id = pool[rng.next() as usize % pool.len()];
                }
            }
        }
        // These instructions explicitly expose the top cards to their owner.
        match &self.decision {
            Some(Decision::Predict { player, card_id }) if *player == actor => {
                if let Some(top) = state.players[actor].deck.last_mut() {
                    *top = *card_id;
                }
            }
            Some(
                Decision::TopThree { player, cards }
                | Decision::PredictCards { player, cards, .. }
                | Decision::DeckLookPlay { player, cards, .. },
            ) if *player == actor => {
                for (slot, &id) in state.players[actor].deck.iter_mut().rev().zip(cards) {
                    *slot = id;
                }
            }
            _ => {}
        }
        // Queued look choices have not yet been shown to the acting player.
        // Keep them consistent with the sampled world instead of retaining a
        // private identity that a later rollout could consult.
        for decision in &mut state.pending_decisions {
            scrub_queued_look(decision, &state.players, &state.catalog);
        }
        for trigger in &mut state.pending_triggers {
            if let PendingTriggerEffect::Choice(decision) = &mut trigger.effect {
                scrub_queued_look(decision, &state.players, &state.catalog);
            }
        }
        state.rng = rng;
        state.log.clear();
        state
    }

    fn strategic_candidates(&self, limit: usize) -> Result<Vec<Action>, String> {
        Ok({
            let actor = self.priority();
            let mut candidates = vec![self.choose_heuristic_action()?];
            candidates.extend(self.legal_actions()?.into_iter().filter(|action| {
            !matches!(
                action,
                Action::ActivateRune { .. } | Action::RecycleRune { .. }
            ) && !matches!(action,Action::ActivateLegend{target} if self.legend_ai_score(actor,*target).is_none())
        }));
            if self.decision.is_none() {
                for (hand_index, &id) in self.players[actor].hand.iter().enumerate() {
                    for lane in [None, Some(0), Some(1)] {
                        if self.validate_play_structure(id, lane).is_err() {
                            continue;
                        }
                        candidates.push(Action::PlayCard {
                            hand_index,
                            lane,
                            target: None,
                        });
                        let (_, required) = rules::target_bounds(id);
                        let valid: Vec<_> = self
                            .card_target_candidates(actor, id, lane, &[])?
                            .into_iter()
                            .take(8)
                            .collect();
                        let mut target_sets = Vec::new();
                        if let Ok(default) = self.prepare_targets(actor, id, lane, &[]) {
                            target_sets.push(default);
                        }
                        for &target in &valid {
                            if required <= 1 {
                                target_sets.push(vec![target]);
                            } else {
                                for other in
                                    self.card_target_candidates(actor, id, lane, &[target])?
                                {
                                    target_sets.push(vec![target, other]);
                                }
                            }
                        }
                        target_sets.dedup();
                        target_sets.truncate(12);
                        for selected in &target_sets {
                            candidates.push(Action::PlayCardWithTargets {
                                hand_index,
                                lane,
                                targets: selected.clone(),
                            });
                            if rules::accelerate_cost(id).is_some()
                                || rules::repeat_cost(id).is_some()
                                || id == 29
                            {
                                let repeated = if rules::repeat_cost(id).is_some() {
                                    // Repeating on the same object can flip a fight;
                                    // separate targets can save or remove two units.
                                    std::iter::once(selected.clone())
                                        .chain(
                                            target_sets
                                                .iter()
                                                .filter(|other| *other != selected)
                                                .take(2)
                                                .cloned(),
                                        )
                                        .collect::<Vec<_>>()
                                } else {
                                    vec![Vec::new()]
                                };
                                for repeat_targets in repeated {
                                    candidates.push(Action::PlayCardOptions {
                                        hand_index,
                                        lane,
                                        targets: selected.clone(),
                                        repeat_targets,
                                        repeat: rules::repeat_cost(id).is_some(),
                                        accelerate: rules::accelerate_cost(id).is_some(),
                                        bonus: id == 29,
                                    });
                                }
                            }
                        }
                    }
                }
                if !self.in_window() {
                    let mut ready: Vec<_> = self.players[actor]
                        .units
                        .iter()
                        .filter(|unit| {
                            unit.ready
                                && unit.lane.is_none()
                                && !self.movement_locks.contains(&(actor, unit.id))
                        })
                        .collect();
                    let mut keyed = ready
                        .iter()
                        .map(|&u| Ok((self.unit_might(u)?, u.id, u)))
                        .collect::<Result<Vec<_>, String>>()?;
                    keyed.sort_by_key(|&(might, id, _)| (might, id));
                    ready = keyed.into_iter().map(|(_, _, u)| u).collect();
                    for lane in 0..BATTLEFIELDS {
                        for count in 2..=ready.len().min(8) {
                            for group in [&ready[..count], &ready[ready.len() - count..]] {
                                candidates.push(Action::MoveGroup {
                                    unit_ids: group.iter().map(|unit| unit.id).collect(),
                                    lane: Some(lane),
                                });
                            }
                        }
                    }
                }
            }
            candidates.extend(self.canonical_play_candidates(32)?);
            let mut unique = Vec::new();
            for action in candidates {
                if !unique.contains(&action) && self.clone().apply(action.clone()).is_ok() {
                    unique.push(action);
                }
            }
            // Rank cheaply before deeper resolution, retaining pass/end-turn as
            // meaningful choices even when many spell target combinations exist.
            let mut ranked = unique
                .into_iter()
                .map(|action| {
                    let score = match &action {
                        Action::WithPlayChoices { .. } => self.canonical_action_score(&action)?,
                        Action::Move { .. } | Action::MoveGroup { .. } => 150,
                        Action::EndTurn | Action::Pass => 145,
                        Action::PlayCard { hand_index, .. }
                        | Action::PlayCardWithTargets { hand_index, .. }
                        | Action::PlayCardOptions { hand_index, .. } => {
                            self.ai_card_score(self.players[actor].hand[*hand_index])?
                        }
                        _ => 110,
                    };
                    Ok((score, action))
                })
                .collect::<Result<Vec<_>, String>>()?;
            ranked.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
            let mut unique: Vec<_> = ranked.into_iter().map(|(_, a)| a).collect();
            unique.truncate(limit);
            unique
        })
    }

    fn strategic_settle(&mut self, starting_turn: u32) -> Result<(), String> {
        for _ in 0..SETTLE_LIMIT {
            if self.winner.is_some() || self.turn != starting_turn {
                break;
            }
            let action = if self.decision.is_some() {
                self.choose_heuristic_action()?
            } else if self.in_window() {
                Action::Pass
            } else {
                break;
            };
            if let Err(error) = self.apply(action) {
                // This is a disposable rollout, but a capacity-limited branch
                // must be discarded rather than scored as a settled position.
                if crate::numeric::is_capacity(&error) {
                    return Err(error);
                }
                break;
            }
        }

        Ok(())
    }

    fn strategic_value(&self, actor: usize) -> Result<i128, String> {
        Ok({
            if let Some(winner) = self.winner {
                return Ok(if winner == actor {
                    1_000_000
                } else {
                    -1_000_000
                });
            }
            let side = |owner: usize| -> Result<i128, String> {
                let player = &self.players[owner];
                let mut value = player.score as i128 * 1200
                    + player.hand.len().min(9) as i128 * 30
                    + player.total_runes() as i128 * 24
                    + player.energy as i128 * 2;
                for unit in &player.units {
                    let might = self.unit_might(unit)? as i128;
                    value +=
                        34 + might * 14 + i128::from(unit.ready) * 12 - unit.damage as i128 * 3;
                    // Temporary Might only earns its full value when it changes a
                    // resolved fight; otherwise it will expire without lasting gain.
                    let mut lasting = unit.clone();
                    lasting.temporary_buff = 0;
                    value -= (might - self.unit_might(&lasting)? as i128) * 13;
                }
                for gear in &player.gear {
                    value += 22
                        + self.catalog[gear.card_id].cost as i128 * 5
                        + i128::from(gear.attached_to.is_some()) * 8;
                }
                for lane in 0..BATTLEFIELDS {
                    if self.battlefield_owner[lane] == Some(owner)
                        && self.lane_count(owner, lane) > 0
                    {
                        let defending = self.lane_might(owner, lane)? as i128;
                        let mut threat = 0_i128;
                        for unit in &self.players[1 - owner].units {
                            if unit.lane.is_none() || self.unit_keywords(unit)?.ganking {
                                threat += i128::from(self.unit_might(unit)?);
                            }
                        }
                        value += 260 + (defending - threat).clamp(-10, 10) * 8;
                    }
                }
                Ok(value)
            };
            side(actor)? - side(1 - actor)?
        })
    }
}

fn scrub_queued_look(decision: &mut Decision, players: &[PlayerState; 2], catalog: &[CardDef]) {
    match decision {
        Decision::DeckLookPlay {
            player,
            cards,
            eligible,
            kind,
            killed_might,
            ..
        } => {
            *cards = players[*player]
                .deck
                .iter()
                .rev()
                .take(cards.len())
                .copied()
                .collect();
            *eligible = Game::deck_look_eligible(cards, *kind, *killed_might, catalog);
        }
        Decision::Predict { player, card_id } => {
            if let Some(&id) = players[*player].deck.last() {
                *card_id = id;
            }
        }
        Decision::PredictCards { player, cards, .. } => {
            *cards = players[*player]
                .deck
                .iter()
                .rev()
                .take(cards.len())
                .copied()
                .collect();
        }
        Decision::TopThree { player, cards } => {
            *cards = players[*player]
                .deck
                .iter()
                .rev()
                .take(3)
                .copied()
                .collect();
        }
        Decision::RevealHand {
            opponent, cards, ..
        } => {
            *cards = players[*opponent].hand.clone();
        }
        _ => {}
    }
}

fn action_penalty(action: &Action) -> i128 {
    match action {
        Action::Pass | Action::EndTurn => 0,
        Action::ActivateGear { .. }
        | Action::ActivateLegend { .. }
        | Action::ActivateAbility { .. }
        | Action::ActivateAbilityWithCosts { .. } => 5,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Game {
        let mut game = Game::new_for_agents(3, 3, 42, Difficulty::Strategic);
        game.start_match().unwrap();
        for player in &mut game.players {
            player.hand.clear();
            player.units.clear();
            player.champion = None;
            player.runes = [0; 6];
            player.exhausted_runes = [0; 6];
            player.energy = 0;
        }
        game
    }

    #[test]
    fn ignores_secret_identities_orders_and_engine_rng() {
        let first = Game::new_for_agents(0, 1, 99, Difficulty::Strategic);
        let mut first = first;
        first.start_match().unwrap();
        let mut second = first.clone();
        second.players[1].hand.fill(52);
        second.players[0].deck.fill(52);
        second.players[1].deck.fill(52);
        second.players[0].rune_deck.reverse();
        second.players[1].rune_deck.reverse();
        second.rng = Rng::new(987654);
        assert_eq!(
            first.choose_strategic_action().unwrap(),
            second.choose_strategic_action().unwrap()
        );
    }

    #[test]
    fn takes_winning_conquest_before_spending_cards() {
        let mut game = fixture();
        game.players[0].score = 7;
        game.player_scored_this_turn[0][1] = true;
        game.scored_this_turn[1] = true;
        let id = game.create_unit(0, 2, false, 0);
        game.players[0].units[0].ready = true;
        let action = game.choose_strategic_action().unwrap();
        assert_eq!(
            action,
            Action::Move {
                unit_id: id,
                lane: Some(0)
            }
        );
        game.apply(action).unwrap();
        game.strategic_settle(game.turn).unwrap();
        assert_eq!(game.winner, Some(0));
    }

    #[test]
    fn does_not_send_a_unit_into_a_losing_fight() {
        let mut game = fixture();
        game.create_unit(0, 2, false, 0);
        game.players[0].units[0].ready = true;
        for lane in 0..2 {
            game.create_unit(1, 2, true, 20);
            game.players[1].units[lane].lane = Some(lane);
            game.battlefield_owner[lane] = Some(1);
        }
        assert_eq!(game.choose_strategic_action().unwrap(), Action::EndTurn);
    }

    #[test]
    fn saves_temporary_buffs_without_a_useful_fight() {
        let mut game = fixture();
        game.players[0].hand = vec![135, 136];
        game.players[0].energy = 8;
        game.players[0].energy_pool = 8;
        game.create_unit(0, 72, false, 0);
        game.create_unit(1, 72, false, 0);
        assert_eq!(game.choose_strategic_action().unwrap(), Action::EndTurn);
    }

    #[test]
    fn uses_a_reaction_to_win_a_public_combat() {
        let mut game = fixture();
        game.players[0].hand = vec![135];
        game.players[0].energy = 2;
        game.players[0].energy_pool = 2;
        let attacker = game.create_unit(0, 72, false, 0);
        game.players[0].units[0].ready = true;
        game.create_unit(1, 72, true, 4);
        game.players[1].units[0].lane = Some(0);
        game.battlefield_owner[0] = Some(1);
        game.apply(Action::Move {
            unit_id: attacker,
            lane: Some(0),
        })
        .unwrap();
        let action = game.choose_strategic_action().unwrap();
        assert!(
            matches!(
                action,
                Action::PlayCard { .. } | Action::PlayCardWithTargets { .. }
            ),
            "{action:?}"
        );
        game.apply(action).unwrap();
        game.strategic_settle(game.turn).unwrap();
        assert!(game.players[1].units.is_empty());
        assert_eq!(game.players[0].units.len(), 1);
        assert_eq!(game.battlefield_owner[0], Some(0));
    }

    #[test]
    fn pays_repeat_when_a_single_buff_still_loses_combat() {
        let mut game = fixture();
        game.players[0].hand = vec![135];
        game.players[0].energy = 4;
        game.players[0].energy_pool = 4;
        let attacker = game.create_unit(0, 72, false, 0);
        game.players[0].units[0].ready = true;
        game.create_unit(1, 72, true, 6);
        game.players[1].units[0].lane = Some(0);
        game.battlefield_owner[0] = Some(1);
        game.apply(Action::Move {
            unit_id: attacker,
            lane: Some(0),
        })
        .unwrap();
        let action = game.choose_strategic_action().unwrap();
        assert!(
            matches!(&action, Action::PlayCardOptions { repeat: true, .. })
                || matches!(&action, Action::WithPlayChoices { choices, .. } if !choices.repeats.is_empty()),
            "{action:?}"
        );
        game.apply(action).unwrap();
        game.strategic_settle(game.turn).unwrap();
        assert!(game.players[1].units.is_empty());
        assert_eq!(game.players[0].units.len(), 1);
        assert_eq!(game.battlefield_owner[0], Some(0));
    }

    #[test]
    fn queued_private_looks_are_replaced_by_sampled_information() {
        let mut first = fixture();
        first.pending_decisions.push_back(Decision::Predict {
            player: 1,
            card_id: 72,
        });
        first.pending_decisions.push_back(Decision::TopThree {
            player: 1,
            cards: vec![72, 73, 74],
        });
        first.pending_decisions.push_back(Decision::PredictCards {
            player: 1,
            choice_id: 91,
            cards: vec![72, 73],
        });
        first.queue_trigger_choice(
            1,
            283,
            Decision::PredictCards {
                player: 1,
                choice_id: 92,
                cards: vec![72, 73],
            },
        );
        let mut second = first.clone();
        second.pending_decisions = VecDeque::from([
            Decision::Predict {
                player: 1,
                card_id: 52,
            },
            Decision::TopThree {
                player: 1,
                cards: vec![52, 53, 54],
            },
            Decision::PredictCards {
                player: 1,
                choice_id: 91,
                cards: vec![52, 53],
            },
        ]);
        for trigger in &mut second.pending_triggers {
            if let PendingTriggerEffect::Choice(Decision::PredictCards { cards, .. }) =
                &mut trigger.effect
            {
                *cards = vec![52, 53];
            }
        }
        let a = first.strategic_belief(0);
        let b = second.strategic_belief(0);
        assert_eq!(a.pending_decisions, b.pending_decisions);
        assert_eq!(
            format!("{:?}", a.pending_triggers),
            format!("{:?}", b.pending_triggers)
        );
        assert_eq!(
            first.choose_strategic_action().unwrap(),
            second.choose_strategic_action().unwrap()
        );
    }

    #[test]
    fn current_predict_two_preserves_the_actors_known_top_cards_in_sampled_worlds() {
        let mut game = fixture();
        game.players[0].deck = vec![69, 70, 71, 72];
        game.decision = Some(Decision::PredictCards {
            player: 0,
            choice_id: 91,
            cards: vec![72, 71],
        });
        let belief = game.strategic_belief(0);
        assert_eq!(
            belief.players[0]
                .deck
                .iter()
                .rev()
                .take(2)
                .copied()
                .collect::<Vec<_>>(),
            vec![72, 71]
        );
        assert_eq!(belief.decision, game.decision);
    }

    #[test]
    fn registry_legends_sample_only_publicly_legal_cards_and_rune_domains() {
        for legend in 0..crate::legend_catalog::legends().len() {
            let custom = crate::deck::CustomDeck::from_legend(legend).unwrap();
            let mut game = Game::new_with_custom(
                [0, 0],
                [Some(custom.clone()), Some(custom)],
                811,
                Difficulty::Strategic,
                false,
            )
            .unwrap();
            game.start_match().unwrap();
            // A wide deterministic sample exercises the public pool; these
            // secret placeholders must never be used as the source of draws.
            game.players[0].deck = vec![69; 1_000];
            game.players[1].deck = vec![69; 1_000];
            game.players[1].hand = vec![69; 100];
            let belief = game.strategic_belief(0);
            let eligible = crate::deck::eligible_cards(legend);
            for owner in 0..2 {
                assert!(
                    belief.players[owner]
                        .deck
                        .iter()
                        .all(|id| eligible.contains(id))
                );
                assert!(
                    belief.players[owner]
                        .rune_deck
                        .iter()
                        .all(|domain| { game.legend_def(owner).domains.contains(domain) })
                );
            }
            assert!(
                belief.players[1]
                    .hand
                    .iter()
                    .all(|id| eligible.contains(id))
            );
            // Currently supported Signatures have unique full domain pairs;
            // eligibility still applies their champion restriction explicitly.
            for (owner, signature) in [(0, 5), (2, 39)] {
                if legend != owner {
                    assert!(belief.players[1].deck.iter().all(|&id| id != signature));
                }
            }
        }
    }

    #[test]
    fn new_legend_beliefs_do_not_read_the_opponents_saved_training_recipe() {
        let mut game = Game::new_with_custom(
            [0, 0],
            [
                Some(crate::deck::CustomDeck::from_legend(4).unwrap()),
                Some(crate::deck::CustomDeck::from_legend(5).unwrap()),
            ],
            812,
            Difficulty::Strategic,
            false,
        )
        .unwrap();
        game.start_match().unwrap();
        let first = game.strategic_belief(0);
        // Preserve public identity and zone sizes while changing private data.
        let private = game.custom_decks[1].as_mut().unwrap();
        private.name = "A different private list".into();
        private.cards = vec![(69, 40)];
        game.players[1].hand.fill(69);
        game.players[1].deck.fill(70);
        game.players[0].deck.fill(77);
        game.rng = Rng::new(123456);
        let second = game.strategic_belief(0);
        for owner in 0..2 {
            assert_eq!(first.players[owner].deck, second.players[owner].deck);
            assert_eq!(
                first.players[owner].rune_deck,
                second.players[owner].rune_deck
            );
        }
        assert_eq!(first.players[1].hand, second.players[1].hand);
    }

    #[test]
    fn deterministic_complete_strategic_matches() {
        for pair in [(0, 1), (1, 2), (2, 3), (3, 0)] {
            let mut game = Game::new(pair.0, pair.1, 17, Difficulty::Strategic);
            game.start_match().unwrap();
            for _ in 0..1500 {
                if game.winner.is_some() {
                    break;
                }
                game.step_ai().unwrap();
            }
            assert!(game.winner.is_some(), "Strategic match stalled: {pair:?}");
        }
    }
}
