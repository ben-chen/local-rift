//! Actual legal games for the Deploy package, both seats/starters and all AI levels.
use local_rift::cards::CardId;
use local_rift::deck::{CustomDeck, eligible_cards};
use local_rift::game::{Decision, Difficulty, Game, Phase, domain_index};
use std::collections::{BTreeMap, BTreeSet};

fn deploy_deck(legend: usize) -> CustomDeck {
    let mut deck = CustomDeck::from_legend(legend).unwrap();
    deck.name = format!("Deploy verification {legend}");
    let eligible = eligible_cards(legend);
    for id in 401..=410 {
        if !eligible.contains(&id) {
            continue;
        }
        if let Some((_, count)) = deck.cards.iter_mut().find(|(old, _)| *old == id) {
            *count = 3;
        } else {
            deck.cards.push((id, 3));
        }
    }
    let mut excess = deck
        .cards
        .iter()
        .map(|(_, n)| usize::from(*n))
        .sum::<usize>()
        - 40;
    for (id, count) in &mut deck.cards {
        if (401..=410).contains(id) {
            continue;
        }
        let minimum = u8::from(*id == deck.chosen_champion);
        let remove = usize::from(*count - minimum).min(excess);
        *count -= u8::try_from(remove).unwrap();
        excess -= remove;
    }
    assert_eq!(excess, 0);
    deck.cards.retain(|(_, count)| *count > 0);
    assert_eq!(
        deck.cards
            .iter()
            .map(|(_, n)| usize::from(*n))
            .sum::<usize>(),
        40
    );
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

#[test]
fn deploy_completes_exact_forty_card_games_across_every_legend_and_ai_level() {
    let mut seen: [BTreeSet<CardId>; 2] = std::array::from_fn(|_| BTreeSet::new());
    let mut payment_choices: [BTreeSet<CardId>; 2] = std::array::from_fn(|_| BTreeSet::new());
    let mut command_modes: [BTreeSet<usize>; 2] = std::array::from_fn(|_| BTreeSet::new());
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
            for home in 0..10 {
                for anchor in [4, 7] {
                    for seat in 0..2 {
                        let mut ids = [home, anchor];
                        if seat == 1 {
                            ids.swap(0, 1);
                        }
                        let decks = ids.map(deploy_deck);
                        let seed = 341000
                            + (level * 10000 + starting * 1000 + home * 40 + anchor * 2 + seat)
                                as u64;
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
                        for step in 0..6000 {
                            conserved(&game, &decks);
                            for (owner, player) in game.players.iter().enumerate() {
                                seen[owner].extend(
                                    player
                                        .units
                                        .iter()
                                        .filter(|u| !u.token && (401..=410).contains(&u.card_id))
                                        .map(|u| u.card_id),
                                );
                                seen[owner].extend(
                                    player
                                        .gear
                                        .iter()
                                        .filter(|g| !g.token && (401..=410).contains(&g.card_id))
                                        .map(|g| g.card_id),
                                );
                            }
                            for item in &game.chain {
                                if item.card_id == 410 && item.ability.is_none() {
                                    seen[item.player].insert(410);
                                    command_modes[item.player]
                                        .extend(item.executions.iter().filter_map(|e| e.mode));
                                }
                            }
                            if let Some(Decision::TriggerPayment {
                                player, card_id, ..
                            }) = game.decision
                            {
                                payment_choices[player].insert(card_id);
                            }
                            if game.winner.is_some() {
                                break;
                            }
                            assert!(game.step_ai().unwrap_or_else(|error| panic!("home{home}/anchor{anchor} seat{seat} {difficulty:?} starter{starting} seed{seed} step{step}: {error}; decision {:?}; recent {:?}",game.decision,game.log.iter().rev().take(8).collect::<Vec<_>>())), "AI stopped in unfinished match");
                            actions += 1;
                        }
                        assert_eq!(
                            game.phase,
                            Phase::Finished,
                            "home{home}/anchor{anchor} seat{seat} {difficulty:?} starter{starting} seed{seed} stalled"
                        );
                        let winner = game.winner.expect("finished game needs a winner");
                        assert!(game.players[winner].score >= 8);
                        conserved(&game, &decks);
                        matches += 1;
                    }
                }
            }
        }
    }
    assert_eq!(matches, 240);
    for seat in 0..2 {
        assert_eq!(
            seen[seat],
            (401..=410).collect(),
            "new card play coverage missing in seat {seat}"
        );
        // The ordinary AI often plays Portal before moving/exhausting units,
        // so its optional ready cost is covered in the actual-action focused
        // regressions rather than required to occur in this bounded seed set.
        assert!(
            payment_choices[seat].contains(&403),
            "Poster pre-chain cost missing in seat {seat}"
        );
        assert!(
            command_modes[seat].contains(&1),
            "Command draw mode missing in seat {seat}"
        );
    }
    println!(
        "PASS: {matches} exact-40-card games / {actions} actions; played cards {seen:?}; pre-chain payment decisions {payment_choices:?}; Command modes {command_modes:?}"
    );
}
