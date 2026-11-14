//! Controlled board/resource fixtures followed by real card and choice actions.
//! The fixtures are not claimed to be complete registered tournament decks.
use super::*;
const GUARDS: CardId = 448;

fn position(actor: usize) -> Game {
    let mut game = Game::new_for_agents(3, 3, 448154, Difficulty::Tactical);
    game.choose_starting_player(actor).unwrap();
    game.start_match().unwrap();
    game.battlefields = vec![9, 9];
    game.battlefield_owner = vec![None; 2];
    for player in &mut game.players {
        player.units.clear();
        player.gear.clear();
        player.hand.clear();
        player.hidden.clear();
        player.discard.clear();
        player.banish.clear();
        player.champion = None;
        player.deck = vec![71; 20];
        player.set_rune_setup([0; 6], [0; 6]).unwrap();
        player.energy = 100;
        player.energy_pool = 100;
        player.power_pool = [20; 6];
    }
    for legend in &mut game.legend {
        legend.ready = false;
    }
    game
}

fn unit(game: &mut Game, player: usize, lane: Option<usize>) -> Target {
    let unit_id = game.create_unit(player, 69, false, 0);
    let unit = game.players[player].units.last_mut().unwrap();
    unit.lane = lane;
    unit.ready = true;
    Target::Unit { player, unit_id }
}

fn pair(game: &mut Game) {
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
}

fn drain(game: &mut Game) {
    for _ in 0..30 {
        assert!(game.decision.is_none(), "{:?}", game.decision);
        if game.chain.is_empty() {
            return;
        }
        game.apply(Action::Pass).unwrap();
    }
    panic!("Chain did not finish");
}

fn cast(g: &mut Game) {
    let actor = g.priority();
    let hand_index = g.players[actor].hand.len();
    g.players[actor].hand.push(GUARDS);
    g.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets: vec![],
    })
    .unwrap();
}

fn choose_token(g: &mut Game) -> (u32, Target) {
    assert!(matches!(g.decision, Some(Decision::TokenPlacement { .. })));
    g.apply(Action::PlaceToken { lane: None }).unwrap();
    let Some(Decision::TriggerPayment {
        choice_id,
        context,
        targets,
        base_cost,
        resolving_chain_id,
        ..
    }) = &g.decision
    else {
        panic!("{:?}", g.decision)
    };
    assert!(
        targets.is_empty(),
        "Referencing it must not choose the token"
    );
    assert!(resolving_chain_id.is_none());
    assert_eq!(base_cost.energy, 0);
    assert_eq!(base_cost.power, 1);
    assert_eq!(base_cost.domains, vec![Domain::Order]);
    (*choice_id, context.subject.unwrap())
}

#[test]
fn guards_token_enters_before_separate_optional_payment_and_ready_response_window() {
    for actor in 0..2 {
        for pay in [false, true] {
            let mut g = position(actor);
            cast(&mut g);
            assert!(g.players[actor].units.is_empty());
            assert_eq!(g.players[actor].energy, 97);
            pair(&mut g);
            let (choice_id, target) = choose_token(&mut g);
            assert!(!g.unit_ref(target).unwrap().ready);
            assert_eq!(g.unit_might(g.unit_ref(target).unwrap()).unwrap(), 2);
            assert!(g.unit_ref(target).unwrap().token);
            assert!(g.linked_token_readies.is_empty());
            let power = g.players[actor].power_pool[domain_index(Domain::Order)];
            g.apply(Action::ResolveTriggerPayment { choice_id, pay })
                .unwrap();
            assert_eq!(
                g.players[actor].power_pool[domain_index(Domain::Order)],
                power - u16::from(pay)
            );
            assert!(!g.unit_ref(target).unwrap().ready);
            if pay {
                assert!(g.chain.last().unwrap().ability.is_some());
                assert!(g.chain.last().unwrap().targets.is_empty());
            }
            drain(&mut g);
            assert_eq!(g.unit_ref(target).unwrap().ready, pay);
        }
    }
}

#[test]
fn spell_only_and_targeted_counters_reject_the_untargeted_ready_ability() {
    for actor in 0..2 {
        let mut g = position(actor);
        cast(&mut g);
        pair(&mut g);
        let (choice_id, target) = choose_token(&mut g);
        g.apply(Action::ResolveTriggerPayment {
            choice_id,
            pay: true,
        })
        .unwrap();
        let chain_id = g.chain.last().unwrap().id;
        g.apply(Action::Pass).unwrap();
        let enemy = 1 - actor;
        for card in [1, 6, 12, 37, 86, 423] {
            g.players[enemy].hand = vec![card];
            let before = format!("{g:?}");
            assert!(
                g.apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![Target::Spell { chain_id }]
                })
                .is_err()
            );
            assert_eq!(format!("{g:?}"), before);
        }
        drain(&mut g);
        assert!(g.unit_ref(target).unwrap().ready);
    }
}

#[test]
fn departed_created_token_does_not_redirect_ready_or_refund_paid_power() {
    for actor in 0..2 {
        let mut g = position(actor);
        let old = g.create_unit(actor, 220, true, 2);
        cast(&mut g);
        pair(&mut g);
        let (choice_id, target) = choose_token(&mut g);
        g.apply(Action::ResolveTriggerPayment {
            choice_id,
            pay: true,
        })
        .unwrap();
        assert_eq!(g.players[actor].power_pool[domain_index(Domain::Order)], 19);
        g.players[actor].hand.push(377);
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        })
        .unwrap();
        let power = g.players[actor].power_pool;
        drain(&mut g);
        assert!(g.unit_ref(target).is_none());
        assert!(
            !g.unit_ref(Target::Unit {
                player: actor,
                unit_id: old
            })
            .unwrap()
            .ready
        );
        assert_eq!(g.players[actor].power_pool, power);
    }
}

#[test]
fn hidden_waives_spell_cost_but_not_separate_ready_payment_and_fixes_token_field() {
    for actor in 0..2 {
        let mut g = position(actor);
        unit(&mut g, actor, Some(0));
        g.battlefield_owner[0] = Some(actor);
        g.players[actor].hidden.push(HiddenCard {
            id: 44800,
            card_id: GUARDS,
            lane: 0,
            hidden_turn: 0,
            revealed: false,
        });
        g.players[actor].energy = 0;
        g.players[actor].energy_pool = 0;
        g.players[actor].power_pool = [0; 6];
        g.apply(Action::PlayHidden {
            hidden_id: 44800,
            targets: vec![],
        })
        .unwrap();
        pair(&mut g);
        let Some(Decision::TriggerPayment {
            choice_id, context, ..
        }) = g.decision
        else {
            panic!("{:?}", g.decision)
        };
        let token = context.subject.unwrap();
        assert_eq!(g.unit_ref(token).unwrap().lane, Some(0));
        assert!(!g.unit_ref(token).unwrap().ready);
        let before = format!("{g:?}");
        assert!(
            g.apply(Action::ResolveTriggerPayment {
                choice_id,
                pay: true
            })
            .is_err()
        );
        assert_eq!(format!("{g:?}"), before);
        g.apply(Action::ResolveTriggerPayment {
            choice_id,
            pay: false,
        })
        .unwrap();
        drain(&mut g);
        assert!(!g.unit_ref(token).unwrap().ready);
        assert_eq!(g.players[actor].energy, 0);
    }
}

#[test]
fn repeat_keeps_token_links_separate_from_each_other_and_an_existing_soldier() {
    use crate::play::{PlayChoices, RepeatSelection};
    for actor in 0..2 {
        let mut g = position(actor);
        let old = g.create_unit(actor, 220, true, 2);
        g.play_grants.push(PlayGrant {
            id: 44899,
            player: actor,
            source: ChainSource::Card { card_id: 443 },
            expires_turn: g.turn,
            kind: PlayGrantKind::RepeatNextSpell,
        });
        g.players[actor].hand.push(GUARDS);
        let action = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        };
        let option_id = g.play_choices(&action).unwrap().repeats[0].id;
        g.apply(Action::WithPlayChoices {
            action: Box::new(action),
            choices: PlayChoices {
                repeats: vec![RepeatSelection {
                    option_id,
                    choices: Default::default(),
                }],
                ..Default::default()
            },
        })
        .unwrap();
        pair(&mut g);
        let (first_choice, first) = choose_token(&mut g);
        g.apply(Action::ResolveTriggerPayment {
            choice_id: first_choice,
            pay: true,
        })
        .unwrap();
        let (second_choice, second) = choose_token(&mut g);
        assert_ne!(first, second);
        g.apply(Action::ResolveTriggerPayment {
            choice_id: second_choice,
            pay: false,
        })
        .unwrap();
        drain(&mut g);
        assert!(g.unit_ref(first).unwrap().ready);
        assert!(!g.unit_ref(second).unwrap().ready);
        assert!(
            !g.unit_ref(Target::Unit {
                player: actor,
                unit_id: old
            })
            .unwrap()
            .ready
        );
        assert!(g.linked_token_readies.is_empty());
    }
}

#[test]
fn refused_pending_token_cannot_ready_an_existing_sand_soldier() {
    for actor in 0..2 {
        let mut g = position(actor);
        let old = g.create_unit(actor, 220, true, 2);
        g.create_unit(actor, 265, false, 0); // Offers optional Accelerate to tokens.
        cast(&mut g);
        pair(&mut g);
        assert!(matches!(g.decision, Some(Decision::TokenPlay { .. })));
        g.apply(Action::BeginPayment {
            action: Box::new(Action::FinalizeTokenPlay {
                lane: None,
                accelerate: true,
            }),
        })
        .unwrap();
        g.apply(Action::RefusePayment).unwrap();
        let Some(Decision::TriggerPayment {
            choice_id, context, ..
        }) = g.decision
        else {
            panic!("{:?}", g.decision)
        };
        assert!(context.subject.is_none());
        g.apply(Action::ResolveTriggerPayment {
            choice_id,
            pay: true,
        })
        .unwrap();
        drain(&mut g);
        assert!(
            !g.unit_ref(Target::Unit {
                player: actor,
                unit_id: old
            })
            .unwrap()
            .ready
        );
        assert_eq!(g.players[actor].units.iter().filter(|u| u.token).count(), 1);
        assert!(g.linked_token_readies.is_empty());
    }
}

#[test]
fn guards_ai_pays_only_for_an_exhausted_created_token_and_ignores_private_cards() {
    for actor in 0..2 {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            for state in 0..3 {
                let mut g = position(actor);
                g.difficulty = difficulty;
                cast(&mut g);
                pair(&mut g);
                let (choice_id, target) = choose_token(&mut g);
                if state == 1 {
                    g.unit_mut(target).unwrap().ready = true;
                } else if state == 2 {
                    g.players[actor].units.clear();
                }
                let expected = Action::ResolveTriggerPayment {
                    choice_id,
                    pay: state == 0,
                };
                assert_eq!(g.choose_ai_action().unwrap(), expected);
                g.players[1 - actor].hand = vec![1, 86, 446];
                g.players[1 - actor].deck = vec![448; 20];
                assert_eq!(g.choose_ai_action().unwrap(), expected);
            }
        }
    }
}

#[test]
fn countered_guards_creates_neither_token_nor_reflexive_ability() {
    for actor in 0..2 {
        let mut g = position(actor);
        cast(&mut g);
        let chain_id = g.chain.last().unwrap().id;
        g.apply(Action::Pass).unwrap();
        g.players[1 - actor].hand.push(86);
        g.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Spell { chain_id }],
        })
        .unwrap();
        drain(&mut g);
        assert!(g.players[actor].units.is_empty());
        assert!(g.linked_token_readies.is_empty());
        assert!(g.pending_triggers.is_empty());
    }
}
