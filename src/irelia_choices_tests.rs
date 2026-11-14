//! Authored midgames; actual spell, response, Repeat and activated-ability actions.
//! Expected draws come from Jae's printed instruction, not the AI policy.
use super::*;

const JAE: CardId = 450;

fn position(actor: usize) -> Game {
    let mut game = Game::new_for_agents(0, 0, 450001, Difficulty::Tactical);
    game.choose_starting_player(actor).unwrap();
    game.start_match().unwrap();
    game.battlefields = vec![3, 3];
    game.battlefield_owner = vec![None; 2];
    for state in &mut game.players {
        state.units.clear();
        state.gear.clear();
        state.hidden.clear();
        state.hand.clear();
        state.discard.clear();
        state.banish.clear();
        state.champion = None;
        state.deck = vec![71; 20];
        state.set_rune_setup([0; 6], [0; 6]).unwrap();
        state.energy_pool = 20;
        state.energy = 20;
        state.power_pool = [5; 6];
        state.any_power = 0;
    }
    for legend in &mut game.legend {
        legend.ready = false;
    }
    game
}

fn unit(game: &mut Game, owner: usize, card: CardId, lane: Option<usize>) -> Target {
    let id = game.create_unit(owner, card, false, 0);
    let unit = game.players[owner].units.last_mut().unwrap();
    unit.lane = lane;
    unit.ready = true;
    Target::Unit {
        player: owner,
        unit_id: id,
    }
}

fn choices(game: &mut Game) {
    for _ in 0..20 {
        match game.decision.clone() {
            Some(Decision::OrderTriggers { options, .. }) => {
                game.apply(Action::OrderTrigger {
                    trigger_id: options[0].id,
                })
                .unwrap();
            }
            Some(Decision::Legend { .. }) => {
                game.apply(Action::Choose { accept: false }).unwrap();
            }
            None => return,
            other => panic!("unexpected choice: {other:?}"),
        }
    }
    panic!("choices did not finish")
}

fn settle(game: &mut Game) {
    for _ in 0..40 {
        choices(game);
        if game.chain.is_empty() {
            return;
        }
        game.apply(Action::Pass).unwrap();
    }
    panic!("chain did not finish")
}

fn cast(game: &mut Game, card: CardId, targets: Vec<Target>) {
    let player = game.priority();
    let hand_index = game.players[player].hand.len();
    game.players[player].hand.push(card);
    game.apply(Action::PlayCardWithTargets {
        hand_index,
        lane: None,
        targets,
    })
    .unwrap();
}

fn draws(game: &Game) -> usize {
    game.chain
        .iter()
        .filter(|item| item.card_id == JAE && item.ability.is_some())
        .count()
}

#[test]
fn irelia_jae_draw_is_a_separate_respondable_trigger_for_each_controller() {
    for actor in 0..2 {
        let mut game = position(actor);
        let target = unit(&mut game, actor, JAE, None);
        cast(&mut game, 8, vec![target]);
        choices(&mut game);
        assert_eq!(draws(&game), 1);
        assert!(game.players[actor].hand.is_empty(), "no premature draw");
        settle(&mut game);
        assert_eq!(game.players[actor].hand, vec![71]);
        assert!(game.players[1 - actor].hand.is_empty());
    }
}

#[test]
fn irelia_jae_does_not_draw_for_enemy_spells_or_friendly_abilities() {
    for actor in 0..2 {
        let mut game = position(actor);
        let enemy = unit(&mut game, 1 - actor, JAE, None);
        cast(&mut game, 7, vec![enemy]); // Discipline can choose an enemy unit.
        choices(&mut game);
        assert_eq!(draws(&game), 0);
        settle(&mut game);
        assert!(game.players[1 - actor].hand.is_empty());

        let friendly = unit(&mut game, actor, JAE, None);
        game.players[actor].gear.push(crate::gear::GearState {
            id: 450002,
            card_id: 230,
            lane: None,
            attached_to: None,
            ready: true,
            temporary: false,
            token: false,
        });
        game.apply(Action::ActivateAbility {
            source: crate::activated::AbilitySource::Gear {
                player: actor,
                gear_id: 450002,
            },
            ability_index: 0,
            targets: vec![friendly],
        })
        .unwrap();
        choices(&mut game);
        assert_eq!(draws(&game), 0);
        settle(&mut game);
        assert_eq!(game.players[actor].hand, vec![71]); // Discipline's own draw only.
    }
}

#[test]
fn irelia_jae_failed_payment_never_creates_a_draw_trigger() {
    let mut game = position(0);
    let target = unit(&mut game, 0, JAE, None);
    game.players[0].hand = vec![8];
    game.players[0].energy = 0;
    game.players[0].energy_pool = 0;
    let before = format!("{game:?}");
    assert!(
        game.apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![target],
        })
        .is_err()
    );
    assert_eq!(format!("{game:?}"), before);
}

#[test]
fn irelia_jae_repeat_chooses_each_target_before_any_draw_resolves() {
    for different in [false, true] {
        let mut game = position(0);
        let first = unit(&mut game, 0, JAE, None);
        let second = if different {
            unit(&mut game, 0, JAE, None)
        } else {
            first
        };
        unit(&mut game, 0, 266, None); // Syndra grants Repeat in the showdown.
        unit(&mut game, 1, 71, Some(0));
        game.battlefield_owner[0] = Some(1);
        game.apply(Action::MoveGroup {
            unit_ids: game.players[0].units.iter().map(|unit| unit.id).collect(),
            lane: Some(0),
        })
        .unwrap();
        game.players[0].hand = vec![8];
        let action = Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![first],
        };
        let option_id = game.play_choices(&action).unwrap().repeats[0].id;
        game.apply(Action::WithPlayChoices {
            action: Box::new(action),
            choices: PlayChoices {
                repeats: vec![crate::play::RepeatSelection {
                    option_id,
                    choices: ExecutionChoices {
                        targets: vec![second],
                        ..Default::default()
                    },
                }],
                ..Default::default()
            },
        })
        .unwrap();
        choices(&mut game);
        assert_eq!(draws(&game), 2);
        assert!(game.players[0].hand.is_empty());
        settle(&mut game);
        assert_eq!(game.players[0].hand, vec![71, 71]);
    }
}

#[test]
fn irelia_jae_departure_or_countering_the_spell_does_not_cancel_its_draw() {
    for return_source in [false, true] {
        let mut game = position(0);
        let target = unit(&mut game, 0, JAE, Some(0));
        game.players[0].units[0].temporary_buff = -2;
        game.battlefield_owner[0] = Some(0);
        cast(&mut game, 8, vec![target]);
        choices(&mut game);
        let spell = game.chain.iter().find(|item| item.card_id == 8).unwrap().id;
        if game.priority() == 0 {
            game.apply(Action::Pass).unwrap();
        }
        if return_source {
            cast(&mut game, 11, vec![target]);
        } else {
            cast(&mut game, 6, vec![Target::Spell { chain_id: spell }]);
        }
        settle(&mut game);
        assert_eq!(
            game.players[0].hand.iter().filter(|&&id| id == 71).count(),
            1
        );
        assert_eq!(game.players[0].hand.contains(&JAE), return_source);
        assert_eq!(game.players[0].units.is_empty(), return_source);
    }
}

#[test]
fn irelia_jae_hidden_spell_chooses_normally_but_hiding_is_not_a_spell_play() {
    let mut game = position(0);
    let target = unit(&mut game, 0, JAE, Some(0));
    game.battlefield_owner[0] = Some(0);
    game.players[0].hand = vec![428];
    game.apply(Action::Hide {
        hand_index: 0,
        lane: 0,
    })
    .unwrap();
    assert_eq!(draws(&game), 0);
    assert!(game.players[0].hand.is_empty());
    let hidden_id = game.players[0].hidden[0].id;
    game.apply(Action::EndTurn).unwrap();
    game.apply(Action::EndTurn).unwrap();
    let before = game.players[0].hand.len();
    game.apply(Action::PlayHidden {
        hidden_id,
        targets: vec![target],
    })
    .unwrap();
    choices(&mut game);
    assert_eq!(draws(&game), 1);
    settle(&mut game);
    // Back Off's own draw is Hand-only, so this is exclusively Jae's draw.
    assert_eq!(game.players[0].hand.len(), before + 1);
}
