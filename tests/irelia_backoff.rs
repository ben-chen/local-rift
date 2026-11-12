//! Authored 40-card Irelia practice across every preset, seat, starter and AI level.
use local_rift::game::{Difficulty, Game};
const BACK_OFF: usize = 428;
#[test]
fn backoff_forty_card_irelia_finishes_all_presets_seats_starters_and_levels() {
    let mut observed = std::collections::BTreeSet::new();
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
                for starter in 0..2 {
                    let mut practice = local_rift::deck::CustomDeck::from_preset(0).unwrap();
                    practice.name = "Irelia 40-card priority verification".into();
                    // Authored sideboard-style substitutions; the stored event list is unchanged.
                    for (card, count) in [(6, 3), (17, 3), (19, 1), (11, 1), (1, 1)] {
                        let entry = practice.cards.iter_mut().find(|x| x.0 == card).unwrap();
                        assert_eq!(entry.1, count);
                        entry.1 = 0;
                    }
                    practice.cards.retain(|x| x.1 > 0);
                    practice
                        .cards
                        .extend([(421, 3), (422, 2), (423, 1), (BACK_OFF, 3)]);
                    assert_eq!(practice.card_count(), 40);
                    practice.validate().unwrap();
                    let mut presets = [0, opponent];
                    let mut custom = [Some(practice), None];
                    if seat == 1 {
                        presets.swap(0, 1);
                        custom.swap(0, 1);
                    }
                    let seed = 428_000 + (level * 100 + opponent * 4 + seat * 2 + starter) as u64;
                    let mut g =
                        Game::new_with_custom(presets, custom, seed, difficulty, false).unwrap();
                    g.choose_starting_player(starter).unwrap();
                    g.start_match().unwrap();
                    for step in 0..2400 {
                        if g.winner.is_some() {
                            break;
                        }
                        for item in &g.chain {
                            if [421, 422, 423, BACK_OFF].contains(&item.card_id) {
                                observed.insert(item.card_id);
                            }
                        }
                        assert!(g.step_ai().unwrap_or_else(|e|panic!("{difficulty:?} opponent{opponent} seat{seat} starter{starter} step{step}: {e}")));
                    }
                    assert!(
                        g.winner.is_some(),
                        "{difficulty:?}/{opponent}/{seat}/{starter}"
                    );
                    let p = &g.players[seat];
                    for (card, expected) in [(421, 3), (422, 2), (423, 1), (BACK_OFF, 3)] {
                        let copies = p
                            .deck
                            .iter()
                            .chain(&p.hand)
                            .chain(&p.discard)
                            .chain(&p.banish)
                            .filter(|&&id| id == card)
                            .count()
                            + p.units
                                .iter()
                                .filter(|u| !u.token && u.card_id == card)
                                .count()
                            + p.hidden.iter().filter(|h| h.card_id == card).count()
                            + g.chain
                                .iter()
                                .filter(|item| {
                                    item.player == seat
                                        && item.ability.is_none()
                                        && item.card_id == card
                                })
                                .count();
                        assert_eq!(
                            copies, expected,
                            "card{card}, {difficulty:?}/{opponent}/{seat}/{starter}"
                        );
                    }
                }
            }
        }
    }
    assert!(observed.contains(&421) && observed.contains(&422) && observed.contains(&BACK_OFF));
}
