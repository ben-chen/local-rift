//! Separate legal-40-card match matrix; all earlier matrices/seeds stay unchanged.
use local_rift::cards::CardId;
use local_rift::deck::{CustomDeck, eligible_cards};
use local_rift::game::{ChainAbility, Decision, Difficulty, Game, Phase, domain_index};
use std::collections::{BTreeMap, BTreeSet};

fn cohort_deck(legend: usize, card: CardId) -> CustomDeck {
    let mut deck = CustomDeck::from_legend(legend).unwrap();
    assert!(eligible_cards(legend).contains(&card));
    deck.name = format!("C28 unit and field verification {legend}/{card}");
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
        let mut rune_ids = BTreeSet::new();
        let mut physical = [0u8; 6];
        let mut exhausted = [0u8; 6];
        for rune in player.rune_objects() {
            assert!(rune_ids.insert(rune.id), "duplicate physical rune identity");
            let domain = domain_index(rune.domain);
            physical[domain] += 1;
            exhausted[domain] += u8::from(!rune.ready);
        }
        assert_eq!(physical, player.runes);
        assert_eq!(exhausted, player.exhausted_runes);
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

#[test]
fn four_units_and_six_fields_complete_144_legal_forty_card_games() {
    let mut entries = [[0usize; 4]; 2];
    let mut triggers = [[0usize; 4]; 2];
    let mut unit_choice_steps = [[0usize; 4]; 2];
    let mut faefolk_outcomes = [[0usize; 3]; 2]; // accepted, declined, continued
    let mut field_choices = [[0usize; 6]; 2]; // unique choice identities
    let mut field_triggers = [[0usize; 6]; 2]; // distinct finalized chain identities
    let mut field_choice_steps = [[0usize; 6]; 2];
    let mut field_outcomes = [[0usize; 3]; 2]; // finalized, declined, continued
    let mut delayed = [0usize; 2];
    let mut field_setups = [[0usize; 6]; 2]; // physical lane, field
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
                (417, 1),
                (417, 7),
                (417, 8),
                (418, 1),
                (418, 7),
                (418, 8),
                (419, 1),
                (419, 7),
                (419, 8),
                (420, 3),
                (420, 4),
                (420, 8),
            ]
            .into_iter()
            .enumerate()
            {
                for seat in 0..2 {
                    let home = cohort_deck(legend, card);
                    let opponent =
                        CustomDeck::from_legend(if card == 420 { 7 } else { 4 }).unwrap();
                    let mut decks = if seat == 0 {
                        [home, opponent]
                    } else {
                        [opponent, home]
                    };
                    for (lane, deck) in decks.iter_mut().enumerate() {
                        let offset = (home_index + level + starting + seat + lane * 3) % 6;
                        deck.battlefield = 11 + offset;
                        deck.validate().unwrap();
                        field_setups[lane][offset] += 1;
                    }
                    let seed =
                        428000 + (level * 10000 + starting * 1000 + home_index * 10 + seat) as u64;
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
                    let mut seen_units = BTreeSet::new();
                    let mut seen_chain = BTreeSet::new();
                    let mut seen_choices = BTreeSet::new();
                    let mut seen_delayed = BTreeSet::new();
                    for step in 0..6000 {
                        conserved(&game, &decks);
                        for (owner, state) in game.players.iter().enumerate() {
                            for unit in &state.units {
                                if !unit.token
                                    && (417..=420).contains(&unit.card_id)
                                    && seen_units.insert(unit.id)
                                {
                                    entries[owner][unit.card_id - 417] += 1;
                                }
                            }
                        }
                        for item in &game.chain {
                            if !seen_chain.insert(item.id) {
                                continue;
                            }
                            if (417..=420).contains(&item.card_id)
                                && matches!(item.ability, Some(ChainAbility::CardTrigger { .. }))
                            {
                                triggers[item.player][item.card_id - 417] += 1;
                            }
                            if let Some(ChainAbility::Battlefield(ability)) = &item.ability {
                                let field = game.battlefields[ability.lane()];
                                if (11..=16).contains(&field) {
                                    field_triggers[item.player][field - 11] += 1;
                                }
                            }
                        }
                        for occurrence in game.delayed_battlefield_triggers() {
                            if seen_delayed.insert(occurrence.id) {
                                delayed[occurrence.player] += 1;
                            }
                        }
                        if game.winner.is_some() {
                            break;
                        }
                        let decision = game.decision.clone();
                        let field_choice = match decision {
                            Some(Decision::BattlefieldTargets {
                                player,
                                choice_id,
                                source,
                                ..
                            })
                            | Some(Decision::BattlefieldCost {
                                player,
                                choice_id,
                                source,
                            })
                            | Some(Decision::BattlefieldPayment {
                                player,
                                choice_id,
                                source,
                                ..
                            }) => Some((player, choice_id, source)),
                            _ => None,
                        };
                        if let Some((player, id, source)) = field_choice {
                            assert!((11..=16).contains(&source.battlefield_id));
                            assert_eq!(game.battlefields[source.lane], source.battlefield_id);
                            field_choice_steps[player][source.battlefield_id - 11] += 1;
                            if seen_choices.insert((player, id)) {
                                field_choices[player][source.battlefield_id - 11] += 1;
                            }
                        }
                        if let Some(Decision::TriggerTargets {
                            player, card_id, ..
                        }) = &decision
                            && (417..=420).contains(card_id)
                        {
                            unit_choice_steps[*player][card_id - 417] += 1;
                        }
                        let prior: BTreeSet<_> = game.chain.iter().map(|i| i.id).collect();
                        assert!(game.step_ai().unwrap_or_else(|error| panic!(
                            "card{card}/legend{legend} seat{seat} {difficulty:?} starter{starting} seed{seed} step{step}: {error}; decision {:?}; last {:?}",game.decision,game.log.iter().rev().take(8).collect::<Vec<_>>())), "AI stopped before winner");
                        if let Some(Decision::TriggerTargets {
                            player,
                            card_id: 417,
                            context,
                            ..
                        }) = &decision
                        {
                            let accepted = game.chain.iter().any(|i| !prior.contains(&i.id) &&
                                matches!(&i.ability,Some(ChainAbility::CardTrigger {context:actual,..}) if actual==context));
                            let consumed = game.decision != decision
                                && game.payment_plan_view(*player).is_none()
                                && !game.resource_payment_pending();
                            faefolk_outcomes[*player][if accepted {
                                0
                            } else if consumed {
                                1
                            } else {
                                2
                            }] += 1;
                        }
                        if let Some((player, _, source)) = field_choice {
                            let accepted = game.chain.iter().any(|i| !prior.contains(&i.id) && i.player==player &&
                                matches!(&i.ability,Some(ChainAbility::Battlefield(a)) if a.lane()==source.lane));
                            let consumed = game.decision.is_none()
                                && game.payment_plan_view(player).is_none()
                                && !game.resource_payment_pending();
                            field_outcomes[player][if accepted {
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
                    println!(
                        "C28 game{matches} card{card}/legend{legend} seat{seat} {difficulty:?} starter{starting} seed{seed} finished turn{}",
                        game.turn
                    );
                }
            }
        }
    }
    assert_eq!(matches, 144);
    assert!(field_setups.iter().flatten().all(|n| *n > 0));
    // Usage is measured, never inferred from match completion or required to be
    // nonzero stochastically. Focused actual-card cases cover declined/unseen paths.
    println!(
        "PASS: {matches} legal40 games / {actions} actions; Faefolk/Kinkou/Nidalee/Pyke entries {entries:?}; finalized unit triggers {triggers:?}; unit choice steps {unit_choice_steps:?}; Faefolk accepted/declined/continued {faefolk_outcomes:?}; field setups {field_setups:?}; unique field choices {field_choices:?}; field choice steps {field_choice_steps:?}; finalized field triggers {field_triggers:?}; field finalized/declined/continued transitions {field_outcomes:?}; delayed Peak occurrences {delayed:?}"
    );
}
