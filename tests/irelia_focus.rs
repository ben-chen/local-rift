//! Irelia's anti-Fury sideboard spell through the same actions used by UI/bots.
use local_rift::game::{Action, ChainAbility, Difficulty, Game, Showdown, Target, Unit};

const DECREE: usize = 421;
fn position() -> Game {
    let mut g = Game::new_for_agents(3, 3, 113421, Difficulty::Tactical);
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
fn combat(g: &mut Game, lane: usize, focus: usize) {
    g.showdown = Some(Showdown {
        lane,
        attacker: 0,
        focus,
        passes: 0,
        combat: true,
    });
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
fn eligible(g: &Game, player: usize) -> Vec<Target> {
    g.card_target_candidates(player, DECREE, None, &[]).unwrap()
}

#[test]
fn combat_checks_enemy_domain_location_and_actual_combat_for_both_seats() {
    for player in 0..2 {
        let mut g = position();
        let friendly = unit(&mut g, player, 100, 73, Some(0));
        let other = unit(&mut g, player, 101, 73, Some(1));
        let base = unit(&mut g, player, 102, 73, None);
        unit(&mut g, 1 - player, 200, 183, Some(0));
        combat(&mut g, 0, player);
        assert_eq!(eligible(&g, player), vec![friendly]);
        assert!(!eligible(&g, player).contains(&other));
        assert!(!eligible(&g, player).contains(&base));
        g.showdown.as_mut().unwrap().combat = false;
        assert!(eligible(&g, player).is_empty());
        g.showdown.as_mut().unwrap().combat = true;
        g.players[1 - player].units[0].token = true;
        assert!(
            eligible(&g, player).is_empty(),
            "factory card does not color a token"
        );
        g.players[1 - player].units[0].token = false;
        g.players[1 - player].units[0].card_id = 73;
        assert!(eligible(&g, player).is_empty());
    }
}

#[test]
fn enemy_fury_spell_targets_including_base_and_multiple_choices_are_eligible() {
    let mut g = position();
    let base = unit(&mut g, 1, 100, 73, None);
    let field = unit(&mut g, 1, 101, 73, Some(1));
    let unchosen = unit(&mut g, 1, 102, 73, None);
    cast(&mut g, 53, vec![base, field]);
    assert_eq!(eligible(&g, 1), vec![base, field]);
    assert!(!eligible(&g, 1).contains(&unchosen));
    g.apply(Action::Pass).unwrap();
    cast(&mut g, DECREE, vec![base]);
    let before = g.players[1].units[0].temporary_buff;
    resolve(&mut g);
    assert_eq!(g.players[1].units[0].temporary_buff, before + 4);
    assert_eq!(
        g.chain.len(),
        1,
        "damage remains respondable beneath Decree"
    );
}

#[test]
fn friendly_spells_other_domains_abilities_and_unchosen_area_damage_do_not_qualify() {
    let mut g = position();
    let friend = unit(&mut g, 0, 100, 73, None);
    let enemy = unit(&mut g, 1, 101, 73, None);
    cast(&mut g, 53, vec![friend, enemy]);
    assert!(eligible(&g, 0).is_empty());
    assert_eq!(eligible(&g, 1), vec![enemy]);
    g.chain[0].card_id = 7;
    assert!(eligible(&g, 1).is_empty());
    g.chain[0].card_id = 53;
    g.chain[0].ability = Some(ChainAbility::Legend(
        local_rift::legends::LegendAbility::IreliaRefresh,
    ));
    assert!(eligible(&g, 1).is_empty());
    g.chain.clear();
    // Unchosen recipients are deliberately absent from a spell's target list.
    cast(&mut g, 299, vec![Target::Battlefield { lane: 0 }]);
    assert!(eligible(&g, 1).is_empty());
}

#[test]
fn repeat_execution_target_is_still_chosen_by_the_pending_spell() {
    let mut g = position();
    let a = unit(&mut g, 1, 100, 73, None);
    let b = unit(&mut g, 1, 101, 73, None);
    g.players[0].hand.push(216); // Upstage Comedy, Fury with printed Repeat.
    g.apply(Action::PlayCardOptions {
        hand_index: 0,
        lane: None,
        targets: vec![a],
        repeat_targets: vec![b],
        repeat: true,
        accelerate: false,
        bonus: false,
    })
    .unwrap();
    assert_eq!(eligible(&g, 1), vec![a, b]);
}

#[test]
fn removing_the_threat_above_decree_makes_its_target_illegal_on_resolution() {
    let mut g = position();
    let target = unit(&mut g, 1, 100, 73, None);
    cast(&mut g, 53, vec![target, target]);
    let damage_id = g.chain[0].id;
    g.apply(Action::Pass).unwrap();
    cast(&mut g, DECREE, vec![target]);
    cast(
        &mut g,
        86,
        vec![Target::Spell {
            chain_id: damage_id,
        }],
    );
    resolve(&mut g); // Wind Wall counters the Fury spell below Decree.
    assert_eq!(g.chain.len(), 1);
    resolve(&mut g);
    assert_eq!(g.players[1].units[0].temporary_buff, 0);
    assert!(g.players[1].discard.contains(&DECREE));
}

#[test]
fn leaving_combat_in_response_invalidates_decree_and_bad_casts_are_atomic() {
    let mut g = position();
    let target = unit(&mut g, 0, 100, 73, Some(0));
    unit(&mut g, 1, 200, 183, Some(0));
    combat(&mut g, 0, 0);
    cast(&mut g, DECREE, vec![target]);
    cast(&mut g, 9, vec![target]); // Flash returns the chosen unit to base.
    resolve(&mut g);
    assert_eq!(g.players[0].units[0].lane, None);
    resolve(&mut g);
    assert_eq!(g.players[0].units[0].temporary_buff, 0);
    g.players[0].hand.push(DECREE);
    let hand_index = g.players[0].hand.len() - 1;
    let before = format!("{g:?}");
    assert!(
        g.apply(Action::PlayCardWithTargets {
            hand_index,
            lane: None,
            targets: vec![target]
        })
        .is_err()
    );
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn catalogue_exposes_restriction_and_irelia_custom_decks_accept_the_card() {
    let catalog = local_rift::api::catalog();
    assert_eq!(
        catalog["cards"][DECREE]["play_targets"]["condition"]["domain"],
        "Fury"
    );
    assert_eq!(
        catalog["cards"][DECREE]["play_targets"]["condition"]["rechecked_on_resolution"],
        true
    );
    assert!(
        catalog["cards"][7]["play_targets"]
            .get("condition")
            .is_none()
    );
    assert!(local_rift::deck::eligible_cards(0).contains(&DECREE));
    let card = local_rift::cards::cards()[DECREE];
    assert_eq!(
        (card.official_id, card.cost, card.power_cost),
        ("ven-040-166", 1, 0)
    );
}

#[test]
fn choosing_irelia_adds_her_growth_above_decree_even_if_decree_later_mistargets() {
    let mut g = position();
    let target = unit(&mut g, 0, 100, 0, Some(0));
    unit(&mut g, 1, 200, 183, Some(0));
    combat(&mut g, 0, 0);
    cast(&mut g, DECREE, vec![target]);
    assert_eq!(g.chain.len(), 2);
    assert_eq!(g.chain[0].card_id, DECREE);
    assert_eq!(g.chain[1].card_id, 0);
    resolve(&mut g);
    assert_eq!(g.players[0].units[0].temporary_buff, 1);
    cast(&mut g, 9, vec![target]);
    // Flash chooses Irelia too: resolve that growth, then Flash, then Decree.
    resolve(&mut g);
    resolve(&mut g);
    assert_eq!(g.players[0].units[0].lane, None);
    resolve(&mut g);
    assert_eq!(g.players[0].units[0].temporary_buff, 2);
}

#[test]
fn irelia_with_decree_completes_games_against_every_preset_at_each_ai_level() {
    let mut cast_count = 0;
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
                practice.name = "Irelia Decree verification".into();
                practice.cards.push((DECREE, 3));
                practice.validate().unwrap();
                let mut presets = [0, opponent];
                let mut custom = [Some(practice), None];
                if seat == 1 {
                    presets.swap(0, 1);
                    custom.swap(0, 1);
                }
                let seed = 421_000 + (level * 100 + opponent * 2 + seat) as u64;
                let mut g =
                    Game::new_with_custom(presets, custom, seed, difficulty, false).unwrap();
                g.choose_starting_player(seat).unwrap();
                g.start_match().unwrap();
                for step in 0..2000 {
                    if g.winner.is_some() {
                        break;
                    }
                    cast_count += g
                        .chain
                        .iter()
                        .filter(|item| item.ability.is_none() && item.card_id == DECREE)
                        .count();
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
                    .filter(|&&id| id == DECREE)
                    .count()
                    + g.chain
                        .iter()
                        .filter(|item| {
                            item.player == seat && item.ability.is_none() && item.card_id == DECREE
                        })
                        .count();
                assert_eq!(copies, 3, "Decree copies were lost or duplicated");
            }
        }
    }
    assert!(
        cast_count > 0,
        "the new card must actually be played in the matrix"
    );
}
