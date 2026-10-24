//! Independent examples from Riot's October 9, 2026 Core Rules and card text.
//! These exercise the public API rather than individual implementation helpers.
use local_rift::game::{Action, Difficulty, Game, Target, Unit};
use local_rift::gear::GearState;

#[test]
fn runtime_card_facts_match_the_sourced_catalog_including_errata_and_domains() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../assets/data/decks.json")).unwrap();
    let sourced = manifest["cards"].as_array().unwrap();
    let catalog = local_rift::cards::cards();
    assert_eq!(catalog.len(), sourced.len());
    for card in catalog {
        let source = &sourced[card.id];
        assert_eq!(source["id"], card.id);
        assert_eq!(source["name"], card.name);
        assert_eq!(source["energy"], card.cost);
        assert_eq!(source["power"], card.power_cost);
        assert_eq!(
            source["official_text"], card.official_text,
            "{} has stale rules text",
            card.name
        );
        let domains: Vec<_> = card.domains.iter().map(|domain| domain.name()).collect();
        assert_eq!(
            source["domains"],
            serde_json::json!(domains),
            "{} has missing domains",
            card.name
        );
    }
}

fn position() -> Game {
    let mut game = Game::new(3, 3, 17, Difficulty::Tactical);
    game.start_match().unwrap();
    for player in &mut game.players {
        player.hand.clear();
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.runes = [6, 0, 6, 0, 0, 0];
        player.exhausted_runes = [0; 6];
        player.energy = 12;
    }
    game
}

fn unit(id: u32, lane: Option<usize>) -> Unit {
    Unit {
        id,
        card_id: 59, // Noxus Hopeful, without a triggered ability.
        lane,
        ready: true,
        damage: 0,
        buff: 0,
        temporary_buff: 0,
        token: false,
        protected: false,
        stunned: false,
        banish_on_death: false,
    }
}

#[test]
fn defiant_dance_accepts_chaos_for_its_multidomain_power_cost() {
    // CR 135.2.e.6.c uses Defiant Dance as its explicit example.
    let mut game = position();
    game.players[0].hand = vec![5];
    game.players[0].runes = [0, 0, 0, 0, 2, 0];
    game.players[0].energy = 2;
    game.players[0].units.push(unit(100, None));
    game.players[1].units.push(unit(101, None));
    let result = game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![
            Target::Unit {
                player: 0,
                unit_id: 100,
            },
            Target::Unit {
                player: 1,
                unit_id: 101,
            },
        ],
    });
    assert!(
        result.is_ok(),
        "Calm OR Chaos must pay this cost: {result:?}"
    );
    assert_eq!(game.players[0].runes[4], 1);
}

#[test]
fn void_seeker_still_draws_when_its_target_moves_to_base() {
    // CR 359.3.e.5 names this exact Void Seeker / movement example.
    let mut game = position();
    game.players[0].hand = vec![67];
    game.players[1].hand = vec![9];
    game.players[1].units.push(unit(101, Some(0)));
    game.battlefield_owner[0] = Some(1);
    let target = Target::Unit {
        player: 1,
        unit_id: 101,
    };
    let before_draw = game.players[0].deck.len();
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![target],
    })
    .unwrap();
    game.apply(Action::Pass).unwrap();
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![target],
    })
    .unwrap();
    for _ in 0..8 {
        if game.chain.is_empty() {
            break;
        }
        game.apply(Action::Pass).unwrap();
    }
    assert!(game.chain.is_empty());
    assert_eq!(game.players[1].units[0].lane, None);
    assert_eq!(game.players[1].units[0].damage, 0);
    assert_eq!(game.players[0].hand.len(), 1);
    assert_eq!(game.players[0].deck.len(), before_draw - 1);
}

#[test]
fn singularity_can_explicitly_choose_zero_units() {
    // "Deal 6 to each of up to two units" permits zero, even with legal targets.
    let mut game = position();
    game.players[0].hand = vec![61];
    game.players[1].units.push(unit(101, None));
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    })
    .unwrap();
    assert!(
        game.chain[0].targets.is_empty(),
        "An explicit empty target set must remain empty."
    );
}

#[test]
fn flash_can_explicitly_choose_zero_units() {
    let mut game = position();
    game.players[0].hand = vec![9];
    game.players[0].units.push(unit(100, Some(0)));
    game.battlefield_owner[0] = Some(0);
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    })
    .unwrap();
    assert!(game.chain[0].targets.is_empty());
}

#[test]
fn salvage_may_decline_to_kill_an_existing_enemy_gear() {
    // October 21, 2025 erratum: "You may kill up to one gear. Draw 1."
    let mut game = position();
    game.players[0].runes = [0, 0, 0, 0, 0, 4];
    game.players[0].energy = 4;
    game.players[0].hand = vec![41];
    game.players[1].gear.push(GearState {
        id: 102,
        card_id: 42,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![],
    })
    .unwrap();
    assert!(game.chain[0].targets.is_empty());
}

#[test]
fn rengar_can_ambush_an_enemy_only_battlefield_in_response() {
    // The April 3, 2026 Unleashed erratum explicitly extends Rengar's Ambush.
    let mut game = position();
    game.players[0].hand = vec![7];
    game.players[0].units.push(unit(100, Some(0)));
    game.battlefield_owner[0] = Some(0);
    game.players[1].hand = vec![30];
    game.players[1].runes = [0, 0, 0, 6, 0, 0];
    game.players[1].energy = 6;
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Unit {
            player: 0,
            unit_id: 100,
        }],
    })
    .unwrap();
    game.apply(Action::Pass).unwrap();
    let result = game.apply(Action::PlayCard {
        hand_index: 0,
        lane: Some(0),
        target: None,
    });
    assert!(
        result.is_ok(),
        "Rengar's Ambush permits this response: {result:?}"
    );
    assert!(
        game.players[1]
            .units
            .iter()
            .any(|u| u.card_id == 30 && u.lane == Some(0))
    );
}

#[test]
fn not_so_fast_can_protect_friendly_gear() {
    // Printed wording covers "a friendly unit or gear", not only units.
    let mut game = position();
    game.apply(Action::EndTurn).unwrap();
    game.players[0].runes = [0, 6, 0, 0, 0, 0];
    game.players[0].exhausted_runes = [0; 6];
    game.players[0].energy = 6;
    game.players[0].hand = vec![12];
    game.players[1].hand = vec![66];
    game.players[0].gear.push(GearState {
        id: 102,
        card_id: 42,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![Target::Gear {
            player: 0,
            gear_id: 102,
        }],
    })
    .unwrap();
    let chain_id = game.chain[0].id;
    game.apply(Action::Pass).unwrap();
    let result = game.apply(Action::PlayCard {
        hand_index: 0,
        lane: None,
        target: Some(Target::Spell { chain_id }),
    });
    assert!(
        result.is_ok(),
        "Friendly gear must be protected too: {result:?}"
    );
}

#[test]
fn mandatory_on_play_target_does_not_stall_when_deflect_cannot_be_paid() {
    let mut game = position();
    game.players[0].hand = vec![23]; // First Mate must choose another unit.
    game.players[0].runes = [0; 6];
    game.players[0].exhausted_runes = [0; 6];
    game.players[0].energy_pool = 3;
    game.players[0].energy = 3;
    let mut enemy = unit(101, None);
    enemy.card_id = 0; // Irelia has Deflect, but no power remains to pay it.
    game.players[1].units.push(enemy);
    game.apply(Action::PlayCard {
        hand_index: 0,
        lane: None,
        target: None,
    })
    .unwrap();
    assert!(
        !game.legal_actions().is_empty(),
        "An unpayable mandatory trigger must not leave the match without a legal action."
    );
}

#[test]
fn falling_star_pays_deflect_for_each_time_it_chooses_the_same_unit() {
    // CR 809.1.c / Spiritforged FAQ: two choices incur two Deflect costs.
    let mut game = position();
    game.players[0].hand = vec![53];
    game.players[0].runes = [6, 0, 0, 0, 0, 0];
    game.players[0].energy = 6;
    let mut enemy = unit(101, None);
    enemy.card_id = 0;
    game.players[1].units.push(enemy);
    let target = Target::Unit {
        player: 1,
        unit_id: 101,
    };
    game.apply(Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets: vec![target, target],
    })
    .unwrap();
    assert_eq!(
        game.players[0].total_runes(),
        2,
        "Pay two printed Fury power and two separate Deflect power."
    );
}
