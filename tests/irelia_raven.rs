//! Ravenbloom Prefect costs and exact event-subject identity.
use local_rift::game::{Action, ChainAbility, Decision, Difficulty, Game, Target, Unit};

const PREFECT: usize = 424;
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
fn prefect_cost_is_paid_before_responses_and_gear_is_banished_on_resolution() {
    for owner in 0..2 {
        let mut g = position(1 - owner);
        unit(&mut g, owner, 500, PREFECT, None);
        cast(&mut g, 235, vec![]);
        let Some(Decision::TriggerObjectCost {
            player, context, ..
        }) = g.decision
        else {
            panic!("{:?}", g.decision)
        };
        assert_eq!(player, owner);
        let gear = context.subject.unwrap();
        assert!(matches!(gear, Target::Gear {player,..} if player==1-owner));
        assert!(g.board_target_facts(gear).is_some());
        assert_eq!(g.players[owner].units.len(), 1);
        g.apply(Action::Choose { accept: true }).unwrap();
        assert!(g.players[owner].units.is_empty());
        assert_eq!(g.players[owner].banish, vec![PREFECT]);
        assert!(g.board_target_facts(gear).is_some());
        let item = g.chain.last().unwrap();
        assert!(matches!(
            item.ability,
            Some(ChainAbility::CardTrigger { .. })
        ));
        assert!(item.targets.is_empty());
        resolve(&mut g);
        assert!(g.board_target_facts(gear).is_none());
        assert_eq!(g.players[1 - owner].banish, vec![235]);
    }
}

#[test]
fn decline_retains_prefect_and_gear_without_an_ability() {
    let mut g = position(1);
    unit(&mut g, 0, 500, PREFECT, None);
    cast(&mut g, 235, vec![]);
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(g.players[0].units.len(), 1);
    assert_eq!(g.players[1].gear.len(), 1);
    assert!(g.chain.is_empty());
    assert!(g.decision.is_none());
    assert!(g.players.iter().all(|p| p.banish.is_empty()));
}

#[test]
fn banishing_prefect_does_not_kill_it_or_offer_death_replacement() {
    let mut g = position(1);
    unit(&mut g, 0, 500, PREFECT, None);
    for (id, card, attached) in [(510, 10, Some(500)), (511, 21, None)] {
        g.players[0].gear.push(local_rift::gear::GearState {
            id,
            card_id: card,
            attached_to: attached,
            lane: None,
            ready: true,
            temporary: false,
            token: false,
        });
    }
    cast(&mut g, 235, vec![]);
    g.apply(Action::Choose { accept: true }).unwrap();
    assert!(g.decision.is_none());
    assert_eq!(g.chain.len(), 1);
    assert_eq!(g.players[0].gear.len(), 2);
    assert!(g.players[0].gear.iter().all(|x| x.attached_to.is_none()));
    assert_eq!(g.players[0].banish, vec![PREFECT]);
    assert!(g.players[0].discard.is_empty());
}

#[test]
fn absent_source_cannot_pay_and_accept_rejection_is_atomic() {
    let mut g = position(1);
    unit(&mut g, 0, 500, PREFECT, None);
    cast(&mut g, 235, vec![]);
    // Restored pending occurrence whose physical source is no longer present.
    g.players[0].units.clear();
    let before = format!("{g:?}");
    assert!(g.apply(Action::Choose { accept: true }).is_err());
    assert_eq!(format!("{g:?}"), before);
    assert!(
        !g.legal_actions()
            .unwrap()
            .contains(&Action::Choose { accept: true })
    );
    g.apply(Action::Choose { accept: false }).unwrap();
    assert!(g.decision.is_none());
    assert_eq!(g.players[1].gear.len(), 1);
}

#[test]
fn ability_keeps_original_gear_identity_and_does_not_choose_it() {
    let mut g = position(1);
    unit(&mut g, 0, 500, PREFECT, None);
    cast(&mut g, 235, vec![]);
    g.apply(Action::Choose { accept: true }).unwrap();
    let ability = g.chain.last().unwrap().id;
    // Not So Fast needs an enemy spell/ability choosing a friendly object.
    assert!(
        !g.valid_card_target(1, 12, None, Target::Spell { chain_id: ability }, &[])
            .unwrap()
    );
    let original = g.players[1].gear.pop().unwrap();
    g.players[1].hand.push(original.card_id);
    let mut replacement = original;
    replacement.id += 1000;
    g.players[1].gear.push(replacement);
    resolve(&mut g);
    assert_eq!(g.players[1].gear.len(), 1);
    assert!(g.players[1].banish.is_empty());
    assert_eq!(g.players[0].banish, vec![PREFECT]);
}

#[test]
fn friendly_gear_and_enemy_gear_activation_do_not_trigger_prefect() {
    let mut g = position(0);
    unit(&mut g, 0, 500, PREFECT, None);
    cast(&mut g, 256, vec![]);
    assert!(g.decision.is_none());
    assert!(g.chain.is_empty());
    g.players[0].units[0].card_id = 73;
    unit(&mut g, 1, 501, PREFECT, None);
    let gear_id = g.players[0].gear[0].id;
    g.apply(Action::ActivateAbility {
        source: local_rift::activated::AbilitySource::Gear { player: 0, gear_id },
        ability_index: 0,
        targets: vec![],
    })
    .unwrap();
    resolve(&mut g);
    assert!(g.decision.is_none());
    assert!(g.chain.is_empty());
    assert_eq!(g.players[1].units.len(), 1);
}

#[test]
fn every_ai_level_trades_prefect_for_expensive_gear() {
    for difficulty in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        let mut g = position(1);
        g.difficulty = difficulty;
        unit(&mut g, 0, 500, PREFECT, None);
        cast(&mut g, 235, vec![]);
        assert!(g.step_ai().unwrap());
        assert_eq!(g.players[0].banish, vec![PREFECT], "{difficulty:?}");
        resolve(&mut g);
        assert_eq!(g.players[1].banish, vec![235]);
    }
}

#[test]
fn opponent_gold_play_triggers_prefect_and_spending_it_in_response_keeps_cost_paid() {
    let mut g = position(1);
    unit(&mut g, 0, 500, PREFECT, None);
    let target = unit(&mut g, 0, 501, 71, Some(0));
    cast(&mut g, 219, vec![target]);
    resolve(&mut g);
    assert!(
        matches!(
            g.decision,
            Some(Decision::TriggerObjectCost { player: 0, .. })
        ),
        "{:?}",
        g.decision
    );
    let gold = g.players[1].gear.iter_mut().find(|x| x.token).unwrap();
    // Restored ready Gold permits its ordinary Add activation in the response window.
    gold.ready = true;
    let gold_id = gold.id;
    g.apply(Action::Choose { accept: true }).unwrap();
    if g.priority() != 1 {
        g.apply(Action::Pass).unwrap();
    }
    g.apply(Action::ActivateGear { gear_id: gold_id }).unwrap();
    assert!(g.players[1].gear.is_empty());
    resolve(&mut g);
    assert!(g.players[1].banish.is_empty());
    assert_eq!(g.players[0].banish, vec![PREFECT]);
}

#[test]
fn one_physical_prefect_cannot_pay_two_occurrences() {
    let mut g = position(1);
    unit(&mut g, 0, 500, PREFECT, None);
    cast(&mut g, 235, vec![]);
    let pending = g.decision.clone();
    g.apply(Action::Choose { accept: true }).unwrap();
    // A second recorded occurrence still names the same physical source.
    g.decision = pending;
    assert!(g.apply(Action::Choose { accept: true }).is_err());
    g.apply(Action::Choose { accept: false }).unwrap();
    assert_eq!(g.players[0].banish, vec![PREFECT]);
    assert_eq!(g.chain.len(), 1);
    resolve(&mut g);
    assert_eq!(g.players[1].banish, vec![235]);
}

#[test]
fn ai_keeps_prefect_for_small_gear_and_imminent_scoring() {
    for difficulty in [
        Difficulty::Easy,
        Difficulty::Tactical,
        Difficulty::Strategic,
    ] {
        for (gear, lane, score) in [(10, None, 0), (235, Some(0), 7)] {
            let mut g = position(1);
            g.difficulty = difficulty;
            unit(&mut g, 0, 500, PREFECT, lane);
            g.players[0].score = score;
            if lane.is_some() {
                g.battlefield_owner[0] = Some(0);
            }
            cast(&mut g, gear, vec![]);
            assert!(g.step_ai().unwrap());
            assert!(g.players[0].banish.is_empty(), "{difficulty:?} / {gear}");
            assert_eq!(g.players[0].units.len(), 1);
            assert!(g.decision.is_none());
        }
    }
}

#[test]
fn forty_card_irelia_variant_completes_all_presets_seats_starters_and_levels() {
    let mut observed = std::collections::BTreeSet::new();
    let mut paid = false;
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
                        .extend([(421, 3), (422, 2), (423, 1), (PREFECT, 3)]);
                    assert_eq!(practice.card_count(), 40);
                    practice.validate().unwrap();
                    let mut presets = [0, opponent];
                    let mut custom = [Some(practice), None];
                    if seat == 1 {
                        presets.swap(0, 1);
                        custom.swap(0, 1);
                    }
                    let seed = 424_000 + (level * 100 + opponent * 4 + seat * 2 + starter) as u64;
                    let mut g =
                        Game::new_with_custom(presets, custom, seed, difficulty, false).unwrap();
                    g.choose_starting_player(starter).unwrap();
                    g.start_match().unwrap();
                    for step in 0..2400 {
                        if g.winner.is_some() {
                            break;
                        }
                        for item in &g.chain {
                            if [421, 422, 423, PREFECT].contains(&item.card_id) {
                                observed.insert(item.card_id);
                            }
                        }
                        paid |= g.players[seat].banish.contains(&PREFECT);
                        assert!(g.step_ai().unwrap_or_else(|e|panic!("{difficulty:?} opponent{opponent} seat{seat} starter{starter} step{step}: {e}")));
                    }
                    assert!(
                        g.winner.is_some(),
                        "{difficulty:?}/{opponent}/{seat}/{starter}"
                    );
                    let p = &g.players[seat];
                    for (card, expected) in [(421, 3), (422, 2), (423, 1), (PREFECT, 3)] {
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
    assert!(observed.contains(&421) && observed.contains(&422) && observed.contains(&PREFECT));
    assert!(paid, "No Prefect cost was paid in the match sample");
}
