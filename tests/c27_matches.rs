//! Separate legal-40-card match matrix; all earlier matrices/seeds stay unchanged.
use local_rift::cards::CardId;
use local_rift::deck::{CustomDeck, eligible_cards};
use local_rift::game::{Action, ChainAbility, Decision, Difficulty, Game, Phase, domain_index};
use std::collections::{BTreeMap, BTreeSet};

fn source_damage_deck(legend: usize, card: CardId) -> CustomDeck {
    let mut deck = CustomDeck::from_legend(legend).unwrap();
    assert!(eligible_cards(legend).contains(&card));
    deck.name = format!("Source damage verification {legend}/{card}");
    if let Some((_, count)) = deck.cards.iter_mut().find(|(id, _)| *id == card) {
        *count = 3;
    } else {
        deck.cards.push((card, 3));
    }
    let mut excess = deck
        .cards
        .iter()
        .map(|(_, n)| usize::from(*n))
        .sum::<usize>()
        - 40;
    for (id, count) in &mut deck.cards {
        if *id == card {
            continue;
        }
        let min = u8::from(*id == deck.chosen_champion);
        let remove = usize::from(*count - min).min(excess);
        *count -= u8::try_from(remove).unwrap();
        excess -= remove;
    }
    assert_eq!(excess, 0);
    deck.cards.retain(|(_, count)| *count > 0);
    deck.validate().unwrap();
    deck
}

fn multiset(cards: impl IntoIterator<Item = CardId>) -> BTreeMap<CardId, usize> {
    let mut out = BTreeMap::new();
    for card in cards {
        *out.entry(card).or_default() += 1;
    }
    out
}

fn conserved(game: &Game, decks: &[CustomDeck; 2]) {
    let mut objects = BTreeSet::new();
    for (owner, spec) in decks.iter().enumerate() {
        let player = &game.players[owner];
        let expected = multiset(
            spec.cards
                .iter()
                .flat_map(|&(id, copies)| std::iter::repeat_n(id, usize::from(copies))),
        );
        let actual = multiset(
            player
                .deck
                .iter()
                .chain(&player.hand)
                .chain(&player.discard)
                .chain(&player.banish)
                .copied()
                .chain(player.champion)
                .chain(game.payment_plan_view(owner).and_then(|plan| {
                    (plan.player == owner)
                        .then_some(plan.pending_card)
                        .flatten()
                }))
                .chain(
                    player
                        .units
                        .iter()
                        .filter(|unit| !unit.token)
                        .map(|unit| unit.card_id),
                )
                .chain(
                    player
                        .gear
                        .iter()
                        .filter(|gear| !gear.token)
                        .map(|gear| gear.card_id),
                )
                .chain(player.hidden.iter().map(|card| card.card_id))
                .chain(
                    game.pending_cards()
                        .iter()
                        .filter(|card| card.player == owner)
                        .map(|card| card.card_id),
                )
                .chain(
                    game.chain
                        .iter()
                        .filter(|item| item.player == owner && item.ability.is_none())
                        .map(|item| item.card_id),
                ),
        );
        assert_eq!(
            actual, expected,
            "physical cards changed: turn {} seat {owner}",
            game.turn
        );
        let mut rune_totals = player.runes.map(u16::from);
        for &domain in &player.rune_deck {
            rune_totals[domain_index(domain)] += 1;
        }
        assert_eq!(rune_totals, spec.runes.map(u16::from));
        assert!(
            player
                .runes
                .iter()
                .zip(player.exhausted_runes)
                .all(|(&all, used)| used <= all)
        );
        let ready_energy: u16 = player
            .runes
            .iter()
            .zip(player.exhausted_runes)
            .map(|(&all, used)| u16::from(all - used))
            .sum();
        assert_eq!(player.energy, ready_energy + player.energy_pool);
        for unit in &player.units {
            assert!(objects.insert(unit.id), "duplicate object ID {}", unit.id);
            assert!(unit.lane.is_none_or(|lane| lane < 2));
        }
        for gear in &player.gear {
            assert!(objects.insert(gear.id), "duplicate object ID {}", gear.id);
            assert!(
                gear.attached_to
                    .is_none_or(|id| player.units.iter().any(|unit| unit.id == id))
            );
        }
    }
}

/// Canonical choices must preserve the pending physical source and chosen targets.
/// Probe worlds are discarded; these checks are not counted as played triggers.
fn audit_pending_choices(game: &Game, probes: &mut [[usize; 4]; 2]) {
    let Some(Decision::TriggerTargets {
        player,
        card_id,
        rule_index,
        context,
    }) = game.decision.clone()
    else {
        return;
    };
    if !(413..=416).contains(&card_id) {
        return;
    }
    assert_ne!(card_id, 414, "Graves must never ask for a target");
    assert_eq!(context.source.player, player);
    assert!(context.source.unit_id.is_some());
    let ids: BTreeSet<_> = game.chain.iter().map(|item| item.id).collect();
    let mut offered_decline = false;
    for action in game.legal_actions().unwrap() {
        let Action::ChooseTriggerTargets { ref targets } = action else {
            continue;
        };
        let mut probe = game.clone();
        probe.apply(action.clone()).unwrap();
        let created: Vec<_> = probe
            .chain
            .iter()
            .filter(|item| !ids.contains(&item.id) && item.card_id == card_id)
            .collect();
        if targets.is_empty() {
            assert_eq!(card_id, 416);
            assert!(
                created.is_empty(),
                "declining Tusker must not finalize its ability"
            );
            offered_decline = true;
        } else {
            assert_eq!(created.len(), 1);
            let item = created[0];
            assert_eq!(item.player, player);
            assert_eq!(item.targets, *targets);
            assert!(
                matches!(&item.ability, Some(ChainAbility::CardTrigger { rule_index: actual_rule, context: actual_context }) if *actual_rule == rule_index && *actual_context == context)
            );
        }
        probes[player][card_id - 413] += 1;
    }
    if card_id == 416 {
        assert!(offered_decline);
    }
}

#[test]
fn source_damage_cards_complete_exact_forty_card_games_in_all_eleven_legend_homes() {
    let mut seen: [BTreeSet<CardId>; 2] = std::array::from_fn(|_| BTreeSet::new());
    let mut triggers = [[0usize; 4]; 2];
    let mut entries = [[0usize; 4]; 2];
    let mut choice_steps = [[0usize; 4]; 2];
    let mut choice_probes = [[0usize; 4]; 2];
    // Per-step transition observations, not inferred unique event counts.
    let mut tusker_choice_outcomes = [[0usize; 3]; 2]; // accepted, declined, continued
    let mut matches = 0;
    let mut actions = 0;
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for starting in 0..2 {
            for (home_index, (card, legend)) in [
                (413, 0),
                (413, 1),
                (414, 1),
                (414, 7),
                (414, 8),
                (415, 3),
                (415, 4),
                (415, 8),
                (416, 1),
                (416, 7),
                (416, 8),
            ]
            .into_iter()
            .enumerate()
            {
                for seat in 0..2 {
                    let home = source_damage_deck(legend, card);
                    let opponent =
                        CustomDeck::from_legend(if card == 415 { 7 } else { 4 }).unwrap();
                    let decks = if seat == 0 {
                        [home, opponent]
                    } else {
                        [opponent, home]
                    };
                    let seed =
                        427000 + (level * 10000 + starting * 1000 + home_index * 10 + seat) as u64;
                    let mut game = Game::new_with_custom(
                        [0, 3],
                        [Some(decks[0].clone()), Some(decks[1].clone())],
                        seed,
                        difficulty,
                        false,
                    )
                    .unwrap();
                    game.choose_starting_player(starting).unwrap();
                    game.start_match().unwrap();
                    let mut seen_chain = BTreeSet::new();
                    let mut seen_units = BTreeSet::new();
                    for step in 0..6000 {
                        conserved(&game, &decks);
                        for (owner, p) in game.players.iter().enumerate() {
                            seen[owner].extend(
                                p.units
                                    .iter()
                                    .filter(|u| !u.token && (413..=416).contains(&u.card_id))
                                    .map(|u| u.card_id),
                            );
                            for u in &p.units {
                                if !u.token
                                    && (413..=416).contains(&u.card_id)
                                    && seen_units.insert(u.id)
                                {
                                    entries[owner][u.card_id - 413] += 1;
                                }
                                let c = game.unit_combat_contribution(u).unwrap();
                                if u.card_id == 414 && !u.token {
                                    assert_eq!(c.damage, 0);
                                    assert!(c.prevents_combat_damage);
                                }
                            }
                        }
                        for item in &game.chain {
                            if !seen_chain.insert(item.id) {
                                continue;
                            }
                            if (413..=416).contains(&item.card_id)
                                && matches!(item.ability, Some(ChainAbility::CardTrigger { .. }))
                            {
                                triggers[item.player][item.card_id - 413] += 1;
                            }
                        }
                        if game.winner.is_some() {
                            break;
                        }
                        audit_pending_choices(&game, &mut choice_probes);
                        let pending_choice = game.decision.clone().filter(|decision| {
                            matches!(
                                decision,
                                Decision::TriggerTargets {
                                    card_id: 413..=416,
                                    ..
                                }
                            )
                        });
                        if let Some(Decision::TriggerTargets {
                            player, card_id, ..
                        }) = &pending_choice
                        {
                            choice_steps[*player][card_id - 413] += 1;
                        }
                        let prior_chain: BTreeSet<_> =
                            game.chain.iter().map(|item| item.id).collect();
                        assert!(game.step_ai().unwrap_or_else(|error|panic!("card{card}/legend{legend} seat{seat} {difficulty:?} starter{starting} seed{seed} step{step}: {error}; decision {:?}; log {:?}",game.decision,game.log.iter().rev().take(8).collect::<Vec<_>>())),"AI stopped before winner");
                        if let Some(Decision::TriggerTargets {
                            player,
                            card_id: 416,
                            context,
                            ..
                        }) = &pending_choice
                        {
                            let accepted = game.chain.iter().any(|item| !prior_chain.contains(&item.id)
                                && matches!(&item.ability, Some(ChainAbility::CardTrigger { context: actual, .. }) if actual == context));
                            let consumed = game.decision != pending_choice
                                && game.payment_plan_view(*player).is_none()
                                && !game.resource_payment_pending();
                            tusker_choice_outcomes[*player][if accepted {
                                0
                            } else if consumed {
                                1
                            } else {
                                2
                            }] += 1;
                        }
                        actions += 1;
                    }
                    assert_eq!(
                        game.phase,
                        Phase::Finished,
                        "card{card}/legend{legend} seat{seat} {difficulty:?} starter{starting} seed{seed} stalled"
                    );
                    assert!(game.players[game.winner.unwrap()].score >= 8);
                    conserved(&game, &decks);
                    matches += 1;
                }
            }
        }
    }
    assert_eq!(matches, 132);
    for cards in &seen {
        assert_eq!(*cards, [413, 414, 415, 416].into_iter().collect());
    }
    println!(
        "PASS: {matches} legal40 games / {actions} actions; played {seen:?}; Yasuo/Graves/Lucian/Tusker board entries {entries:?}; finalized triggers {triggers:?}; pending choice steps {choice_steps:?}; legal choice probes {choice_probes:?}; Tusker accepted/declined/continued choice transitions {tusker_choice_outcomes:?}"
    );
}
