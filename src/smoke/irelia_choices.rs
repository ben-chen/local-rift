//! Play Jae, inspect his face, choose him with a spell, and pass on his draw.
use super::*;

const JAE: usize = 450;

fn setup(app: &mut App) {
    expansion_fixture(app, JAE);
    app.game = Game::new_for_agents(0, 1, 450450, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![3, 3];
    app.game.battlefield_owner = vec![None; 2];
    for player in &mut app.game.players {
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.hand.clear();
        player.discard.clear();
        player.champion = None;
        player.deck = vec![71; 20];
        player.set_rune_setup([0, 3, 0, 0, 3, 0], [0; 6]).unwrap();
        player.rune_deck = vec![Domain::Calm; 3];
        player.rune_deck.extend([Domain::Chaos; 3]);
        player.energy = 12;
        player.energy_pool = 6;
        player.power_pool = [0, 0, 0, 0, 2, 0];
        player.any_power = 0;
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
    }
    app.game.players[0].hand = vec![JAE, 8];
    app.player_deck = 0;
    app.opponent_deck = 1;
    app.fields = [3, 3];
    app.inspect = None;
    app.field_inspect = None;
    app.lesson = None;
    app.rune_menu = None;
    app.chain_cursor = 0;
    app.prefs.coach = true;
    app.ai_worker.invalidate();
    app.companion.set_level(AdviceLevel::Beginner);
}

fn source(app: &App) -> u32 {
    app.game.players[0].units[0].id
}

fn advice(app: &mut App, level: AdviceLevel) {
    app.companion.set_level(level);
    app.companion.observe(&app.game);
    for _ in 0..80 {
        if app.companion.message().contains("Jae") {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("Jae advice is missing");
}

pub(super) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app),
        1 => click(app, 315.0, 760.0),
        2 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            click(app, 1285.0, 510.0);
        }
        3 => {
            assert_eq!(app.game.players[0].units[0].card_id, JAE);
            assert!(!app.game.players[0].units[0].ready);
            assert_eq!(app.game.players[0].energy, 7);
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Chaos)],
                0
            );
            assert_eq!(app.game.players[0].hand, vec![8]);
            let r = unit_rect(app, source(app));
            app.ui.input_override = Some((r.center(), false, true));
        }
        4 => {
            assert_eq!(app.inspect, Some(JAE));
            capture(app, "RIFT_JAE_INSPECT");
            app.inspect = None;
        }
        5 => {
            advice(app, AdviceLevel::Beginner);
            capture(app, "RIFT_JAE_BEGINNER");
            click(app, 150.0, 600.0);
        }
        6 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            advice(app, AdviceLevel::Advanced);
            capture(app, "RIFT_JAE_ADVANCED");
            click(app, 315.0, 760.0);
        }
        7 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            click_unit(app, source(app));
        }
        8 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: source(app)
                }]
            );
            click(app, 1285.0, 510.0);
        }
        9 => {
            assert!(app.game.chain.iter().any(|item| item.card_id == 8));
            // Irelia's independent optional ability is covered by the earlier
            // native sequence; decline it here to isolate Jae's response window.
            for _ in 0..8 {
                match app.game.decision.clone() {
                    Some(Decision::Legend { .. }) => {
                        app.game.apply(Action::Choose { accept: false }).unwrap();
                    }
                    Some(Decision::OrderTriggers { options, .. }) => {
                        app.game
                            .apply(Action::OrderTrigger {
                                trigger_id: options[0].id,
                            })
                            .unwrap();
                    }
                    None => break,
                    other => panic!("unexpected Jae choice: {other:?}"),
                }
            }
        }
        10 => {
            assert_eq!(app.game.chain.len(), 2);
            assert_eq!(app.game.chain.last().unwrap().card_id, JAE);
            assert!(app.game.players[0].hand.is_empty());
            assert_eq!(app.game.priority(), 0);
            advice(app, AdviceLevel::Beginner);
            capture(app, "RIFT_JAE_TRIGGER");
            click(app, 1285.0, 595.0);
        }
        11 => {
            assert_eq!(
                app.game.priority(),
                1,
                "Mote must not intercept the pass click"
            );
            assert!(app.game.players[0].hand.is_empty());
            app.game.apply(Action::Pass).unwrap();
        }
        12 => {
            assert_eq!(app.game.players[0].hand, vec![71]);
            assert_eq!(app.game.chain.len(), 1);
            assert_eq!(app.game.chain[0].card_id, 8);
            assert_eq!(
                app.game.unit_might(&app.game.players[0].units[0]).unwrap(),
                5
            );
            capture(app, "RIFT_JAE_DRAW");
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        13 => {
            assert!(app.game.chain.is_empty());
            assert_eq!(
                app.game.unit_might(&app.game.players[0].units[0]).unwrap(),
                7
            );
            assert_eq!(app.game.players[0].hand, vec![71]);
            return true;
        }
        _ => panic!("Jae native scenario exceeded its frames"),
    }
    false
}
