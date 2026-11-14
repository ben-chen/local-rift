//! Actual target and resolution controls, with both unobtrusive advice levels.
use super::*;

fn setup(app: &mut App) {
    expansion_fixture(app, 455);
    app.game = Game::new_for_agents(0, 1, 455455, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![3, 3];
    app.game.battlefield_owner = vec![None; 2];
    for p in &mut app.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.set_rune_setup([0, 6, 0, 0, 0, 0], [0; 6]).unwrap();
        p.rune_deck = vec![Domain::Calm; 6];
        p.energy = 12;
        p.energy_pool = 6;
        p.power_pool = [0; 6];
        p.any_power = 0;
    }
    app.game.legend[0].ready = false;
    app.game.legend[1].ready = true;
    app.game.players[0].hand = vec![455];
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
}

fn advice(app: &mut App, level: AdviceLevel) {
    app.companion.set_level(level);
    app.companion.observe(&app.game);
    let resolving = matches!(app.game.decision, Some(Decision::LegendReadiness { .. }));
    let expected = match (resolving, level) {
        (false, AdviceLevel::Beginner) => "Royal Entourage chooses",
        (false, AdviceLevel::Advanced) => "Ready Irelia",
        (true, AdviceLevel::Beginner) => "Responses have finished",
        (true, AdviceLevel::Advanced) => "already paid",
    };
    for _ in 0..80 {
        if app.companion.message().contains(expected) {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("missing legend advice");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app),
        1 => app.ui.input_override = Some((vec2(315.0, 760.0), false, true)),
        2 => {
            assert_eq!(app.inspect, Some(455));
            capture(app, "RIFT_ENTOURAGE_INSPECT");
        }
        3 => {
            app.inspect = None;
            click(app, 315.0, 760.0);
        }
        4 => click(app, 1285.0, 510.0),
        5 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerTargets { card_id: 455, .. })
            ));
            advice(app, AdviceLevel::Beginner);
            capture(app, "RIFT_ENTOURAGE_TARGET_BEGINNER");
            click(app, 150.0, 600.0);
        }
        6 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            advice(app, AdviceLevel::Advanced);
            capture(app, "RIFT_ENTOURAGE_TARGET_ADVANCED");
            click(app, 1285.0, 490.0);
        }
        7 => {
            assert!(app.game.decision.is_none());
            assert!(!app.game.legend[0].ready);
            assert_eq!(app.game.chain.len(), 1);
            capture(app, "RIFT_ENTOURAGE_RESPONSE");
            click(app, 1285.0, 595.0);
        }
        8 => {
            assert_eq!(app.game.priority(), 1, "Mote must not intercept passing");
            app.game.apply(Action::Pass).unwrap();
        }
        9 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::LegendReadiness { .. })
            ));
            advice(app, AdviceLevel::Advanced);
            capture(app, "RIFT_ENTOURAGE_RESOLUTION");
            click(app, 1285.0, 525.0);
        }
        10 => {
            assert!(app.game.legend[0].ready);
            assert!(app.game.chain.is_empty());
            capture(app, "RIFT_ENTOURAGE_READY");
        }
        11 => {
            setup(app);
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![],
                })
                .unwrap();
            click(app, 1285.0, 534.0);
        }
        12 => {
            assert_eq!(
                app.game.chain[0].targets,
                vec![Target::Legend {
                    player: 1,
                    legend_id: app.game.legend[1].kind.id()
                }]
            );
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            advice(app, AdviceLevel::Beginner);
            capture(app, "RIFT_ENTOURAGE_OPPONENT");
            click(app, 1285.0, 563.0);
        }
        13 => {
            assert!(!app.game.legend[1].ready);
            assert!(app.game.chain.is_empty());
            capture(app, "RIFT_ENTOURAGE_EXHAUSTED");
        }
        14 => return true,
        _ => panic!("Entourage scenario exceeded its frames"),
    }
    false
}
