//! Irelia's draw/channel and Power-restricted counter through UI/bot actions.
use local_rift::game::{Action, ChainAbility, Difficulty, Game, Target, Unit};

const CENTER: usize = 422;
const HUSH: usize = 423;
fn position(owner: usize) -> Game {
    let mut g = Game::new_for_agents(3, 3, 113421, Difficulty::Tactical);
    g.choose_starting_player(owner).unwrap();
    g.start_match().unwrap();
    g.battlefields = [2, 2];
    g.battlefield_owner = [None; 2];
    for p in &mut g.players {
        p.hand.clear();
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.discard.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 50;
        p.energy = 50;
        p.power_pool = [20; 6];
    }
    g
}
fn unit(g: &mut Game, player: usize, id: u32, card_id: usize, lane: Option<usize>) -> Target {
    g.players[player].units.push(Unit {
        id,
        card_id,
        lane,
        ready: true,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    });
    Target::Unit {
        player,
        unit_id: id,
    }
}
fn cast(g: &mut Game, card: usize, targets: Vec<Target>) {
    let p = g.priority();
    let hand_index = g.players[p].hand.len();
    g.players[p].hand.push(card);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
}
fn resolve(g: &mut Game) {
    g.apply(Action::Pass).unwrap();
    g.apply(Action::Pass).unwrap();
}

#[test]
fn center_discount_uses_opponents_score_in_queries_direct_and_staged_costs() {
    for owner in 0..2 {
        for opponent_score in [0, 4, 5, 7] {
            for staged in [false, true] {
                let mut g = position(owner);
                g.players[owner].score = 7;
                g.players[1 - owner].score = opponent_score;
                g.players[owner].hand = vec![CENTER];
                let action = Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                };
                let cost = if opponent_score >= 5 { 1 } else { 3 };
                assert_eq!(g.play_choices(&action).unwrap().costs.energy, cost);
                if staged {
                    g.apply(Action::BeginPayment {
                        action: Box::new(action),
                    })
                    .unwrap();
                    g.apply(Action::Choose { accept: true }).unwrap();
                } else {
                    g.apply(action).unwrap();
                }
                assert_eq!(g.players[owner].energy, 50 - cost as u16);
                assert_eq!(g.chain.last().unwrap().card_id, CENTER);
            }
        }
    }
}

#[test]
fn center_draws_then_channels_exactly_one_exhausted_rune_without_fallback_draw() {
    for runes in [vec![], vec![local_rift::cards::Domain::Calm]] {
        let mut g = position(0);
        g.players[0].deck = vec![73, 71];
        g.players[0].rune_deck = runes.clone();
        cast(&mut g, CENTER, vec![]);
        resolve(&mut g);
        assert_eq!(g.players[0].hand, vec![71]);
        assert_eq!(g.players[0].deck, vec![73]);
        assert_eq!(g.players[0].total_runes() as usize, runes.len());
        assert_eq!(g.players[0].ready_runes(local_rift::cards::Domain::Calm), 0);
        assert_eq!(g.players[0].energy, 47);
    }
}

#[test]
fn center_counter_and_burnout_victory_do_not_continue_to_channel() {
    let mut g = position(0);
    g.players[0].deck = vec![71];
    g.players[0].rune_deck = vec![local_rift::cards::Domain::Calm];
    cast(&mut g, CENTER, vec![]);
    let id = g.chain[0].id;
    cast(&mut g, 86, vec![Target::Spell { chain_id: id }]);
    resolve(&mut g);
    assert_eq!(g.players[0].deck, vec![71]);
    assert_eq!(g.players[0].total_runes(), 0);
    let mut g = position(0);
    g.players[0].deck.clear();
    g.players[0].discard.clear();
    g.players[1].score = 7;
    g.players[0].rune_deck = vec![local_rift::cards::Domain::Calm];
    cast(&mut g, CENTER, vec![]);
    resolve(&mut g);
    assert_eq!(g.winner, Some(1));
    assert_eq!(g.players[0].total_runes(), 0);
}

#[test]
fn hush_counters_two_power_spell_and_pays_two_calm_power() {
    let mut g = position(0);
    let target = unit(&mut g, 1, 500, 73, None);
    cast(&mut g, 53, vec![target, target]); // Falling Star has 2 Power.
    let id = g.chain[0].id;
    g.apply(Action::Pass).unwrap();
    let energy = g.players[1].energy;
    let power = g.players[1].power_pool[1];
    cast(&mut g, HUSH, vec![Target::Spell { chain_id: id }]);
    assert_eq!(g.players[1].energy, energy - 1);
    assert_eq!(g.players[1].power_pool[1], power - 2);
    resolve(&mut g);
    assert!(g.chain.is_empty());
    assert!(g.players[0].discard.contains(&53));
    assert_eq!(g.players[1].units[0].damage, 0);
}

#[test]
fn hush_uses_printed_power_not_energy_or_deflect_and_never_counters_abilities() {
    let mut g = position(0);
    let target = unit(&mut g, 1, 500, 0, Some(0));
    cast(&mut g, 55, vec![target]); // Deflect payment does not raise printed Power0.
    let id = g.chain.iter().find(|c| c.ability.is_none()).unwrap().id;
    assert!(
        !g.valid_card_target(1, HUSH, None, Target::Spell { chain_id: id }, &[])
            .unwrap()
    );
    let item = g.chain.iter_mut().find(|c| c.id == id).unwrap();
    item.card_id = 53;
    assert!(
        g.valid_card_target(1, HUSH, None, Target::Spell { chain_id: id }, &[])
            .unwrap()
    );
    let item = g.chain.iter_mut().find(|c| c.id == id).unwrap();
    item.ability = Some(ChainAbility::Legend(
        local_rift::legends::LegendAbility::IreliaRefresh,
    ));
    assert!(
        !g.valid_card_target(1, HUSH, None, Target::Spell { chain_id: id }, &[])
            .unwrap()
    );
}

#[test]
fn irelia_priority_spell_draft_finishes_against_each_preset_at_all_levels() {
    let mut seen = std::collections::BTreeSet::new();
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for opponent in 0..local_rift::cards::presets().len() {
            for seat in 0..2 {
                let mut practice = local_rift::deck::CustomDeck::from_preset(0).unwrap();
                // Explicitly authored test variant, never relabel the tournament list.
                practice.name = "Irelia priority spell verification".into();
                for card in [421, CENTER, HUSH] {
                    practice.cards.push((card, 3));
                }
                practice.validate().unwrap();
                let mut presets = [0, opponent];
                let mut custom = [Some(practice), None];
                if seat == 1 {
                    presets.swap(0, 1);
                    custom.swap(0, 1);
                }
                let seed = 423_000 + (level * 100 + opponent * 2 + seat) as u64;
                let mut g =
                    Game::new_with_custom(presets, custom, seed, difficulty, false).unwrap();
                g.choose_starting_player(seat).unwrap();
                g.start_match().unwrap();
                for step in 0..2000 {
                    if g.winner.is_some() {
                        break;
                    }
                    for item in &g.chain {
                        if item.ability.is_none() && [421, CENTER, HUSH].contains(&item.card_id) {
                            seen.insert(item.card_id);
                        }
                    }
                    assert!(g.step_ai().unwrap_or_else(|e| panic!("{difficulty:?}, opponent {opponent}, seat {seat}, seed {seed}, step {step}: {e}")));
                }
                assert!(
                    g.winner.is_some(),
                    "unfinished {difficulty:?} / {opponent} / {seat}"
                );
                let p = &g.players[seat];
                let copies = p
                    .deck
                    .iter()
                    .chain(&p.hand)
                    .chain(&p.discard)
                    .chain(&p.banish)
                    .filter(|&&id| id == CENTER)
                    .count()
                    + g.chain
                        .iter()
                        .filter(|item| {
                            item.player == seat && item.ability.is_none() && item.card_id == CENTER
                        })
                        .count();
                assert_eq!(copies, 3, "Find Your Center copies were lost or duplicated");
            }
        }
    }
    // The sampled draws need not provide a legal high-Power response window.
    // Hush's actual AI use is required in a controlled lethal-spell test below.
    assert!(seen.contains(&421) && seen.contains(&CENTER));
}

#[test]
fn every_ai_level_uses_hush_to_answer_an_eligible_lethal_spell() {
    for difficulty in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        let mut g = position(0);
        g.difficulty = difficulty;
        let target = unit(&mut g, 1, 500, 73, None);
        g.players[1].hand = vec![HUSH];
        cast(&mut g, 53, vec![target, target]);
        g.apply(Action::Pass).unwrap();
        for _ in 0..8 {
            if g.chain
                .iter()
                .any(|item| item.ability.is_none() && item.card_id == HUSH)
            {
                break;
            }
            assert!(g.step_ai().unwrap());
        }
        assert!(
            g.chain
                .iter()
                .any(|item| item.ability.is_none() && item.card_id == HUSH),
            "{difficulty:?} did not play Hush"
        );
    }
}
