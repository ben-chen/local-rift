//! Authored Irelia equipment substitutions; sourced tournament lists stay intact.
use local_rift::{
    cards::presets,
    deck::CustomDeck,
    game::{Difficulty, Game},
};

#[test]
fn conditional_equipment_finishes_irelia_games_at_every_level_and_seat() {
    let mut attached = [[[0usize; 2]; 2]; 3];
    let mut active = [[[0usize; 2]; 2]; 3];
    for (level, difficulty) in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ]
    .into_iter()
    .enumerate()
    {
        for opponent in [0, 5] {
            for seat in 0..2 {
                for starter in 0..2 {
                    for sample in 0..6 {
                        let mut deck = CustomDeck::from_preset(0).unwrap();
                        deck.name = "Irelia / conditional equipment verification".into();
                        for (old, new) in [(3, 444), (21, 445)] {
                            let row = deck.cards.iter_mut().find(|(id, _)| *id == old).unwrap();
                            assert_eq!(row.1, 2);
                            row.0 = new;
                        }
                        deck.validate().unwrap();
                        assert_eq!(deck.card_count(), 40);
                        let mut lists = [0, opponent];
                        let mut custom = [Some(deck), None];
                        if seat == 1 {
                            lists.swap(0, 1);
                            custom.swap(0, 1);
                        }
                        let seed = 444445
                            + (sample * 4096 + level * 100 + opponent * 10 + seat * 2 + starter)
                                as u64;
                        let mut g =
                            Game::new_with_custom(lists, custom, seed, difficulty, false).unwrap();
                        g.choose_starting_player(starter).unwrap();
                        g.start_match().unwrap();
                        let mut seen = std::collections::HashSet::new();
                        for step in 0..2400 {
                            if g.winner.is_some() {
                                break;
                            }
                            assert!(g.step_ai().unwrap_or_else(|e|panic!(
                                "{difficulty:?} vs{opponent} seat{seat} start{starter} sample{sample} step{step}: {e}")));
                            g.validate_numeric_state().unwrap();
                            let p = &g.players[seat];
                            for (index, card) in [444, 445].into_iter().enumerate() {
                                let copies = p
                                    .deck
                                    .iter()
                                    .chain(&p.hand)
                                    .chain(&p.discard)
                                    .chain(&p.banish)
                                    .filter(|&&id| id == card)
                                    .count()
                                    + p.gear
                                        .iter()
                                        .filter(|e| !e.token && e.card_id == card)
                                        .count()
                                    + g.chain
                                        .iter()
                                        .filter(|s| {
                                            s.player == seat
                                                && s.card_id == card
                                                && s.ability.is_none()
                                        })
                                        .count();
                                assert_eq!(
                                    copies, 2,
                                    "Equipment physical copies must be conserved"
                                );
                                for gear in p.gear.iter().filter(|e| e.card_id == card) {
                                    if let Some(unit_id) = gear.attached_to {
                                        if seen.insert((gear.id, unit_id)) {
                                            attached[level][seat][index] += 1;
                                        }
                                        let contributions = g
                                            .passive_contributions(
                                                local_rift::empower::BoardObject::Unit {
                                                    player: seat,
                                                    unit_id,
                                                },
                                            )
                                            .unwrap();
                                        if contributions
                                            .iter()
                                            .any(|c| c.card_id == card && c.active)
                                        {
                                            active[level][seat][index] += 1;
                                        }
                                    }
                                }
                            }
                        }
                        assert!(
                            g.winner.is_some(),
                            "{difficulty:?} vs{opponent} seat{seat} start{starter} sample{sample}"
                        );
                    }
                }
            }
        }
    }
    eprintln!(
        "Equipment attachments by level/seat/card: {attached:?}; active conditions: {active:?}"
    );
    assert!(attached.iter().flatten().flatten().all(|&count| count > 0));
    assert!(active.iter().flatten().flatten().all(|&count| count > 0));
    assert!(presets()[0].cards.contains(&(3, 2)));
    assert!(presets()[0].cards.contains(&(21, 2)));
    assert!(
        !presets()[0]
            .cards
            .iter()
            .any(|&(id, _)| id == 444 || id == 445)
    );
}
