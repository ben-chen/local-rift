//! Exercise actual hand, token placement, optional payment and inspector controls.
use super::*;

fn setup(app: &mut App) {
    expansion_fixture(app, 448);
    app.game = Game::new_for_agents(3, 3, 448448, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![9, 9];
    app.game.battlefield_owner = vec![None; 2];
    for player in &mut app.game.players {
        player.units.clear();
        player.gear.clear();
        player.hand.clear();
        player.hidden.clear();
        player.discard.clear();
        player.champion = None;
        player.deck = vec![71; 20];
        player.set_rune_setup([0; 6], [0; 6]).unwrap();
        player.energy = 10;
        player.energy_pool = 10;
        player.power_pool = [10; 6];
    }
    app.game.players[0].hand = vec![448];
    app.player_deck = 3;
    app.opponent_deck = 3;
    app.fields = [9, 9];
    app.ai_worker.invalidate();
    app.selection = None;
    app.targets.clear();
    app.inspect = None;
    app.field_inspect = None;
    app.lesson = None;
    app.show_play_effects = false;
    app.chain_cursor = 0;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
}

fn advice(app: &mut App, level: AdviceLevel) {
    app.companion.set_level(level);
    app.companion.observe(&app.game);
    for _ in 0..80 {
        if app.companion.message().contains("Guards!") {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("Guards! advice must be available while held or paying");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 | 9 => setup(app),
        1 => {
            advice(app, AdviceLevel::Beginner);
            capture(app, "RIFT_GUARDS_BEGINNER");
            click(app, 315.0, 760.0);
        }
        2 | 11 => click(app, 1280.0, 510.0),
        3 | 12 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 448);
            assert_eq!(app.game.players[0].energy, 7);
            assert!(app.game.players[0].units.is_empty());
            app.action(Action::Pass);
            app.action(Action::Pass);
        }
        4 | 13 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TokenPlacement { .. })
            ));
            if frame == 4 {
                capture(app, "RIFT_GUARDS_PLACEMENT");
            }
            click(app, 1280.0, 470.0);
        }
        5 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerPayment { card_id: 448, .. })
            ));
            assert_eq!(app.game.players[0].units.len(), 1);
            assert!(!app.game.players[0].units[0].ready);
            advice(app, AdviceLevel::Advanced);
            capture(app, "RIFT_GUARDS_PAYMENT");
            click(app, 1280.0, 510.0);
        }
        6 => {
            assert!(app.game.decision.is_none());
            assert!(app.game.chain.last().unwrap().ability.is_some());
            assert!(app.game.chain.last().unwrap().targets.is_empty());
            assert!(!app.game.players[0].units[0].ready);
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Order)],
                9
            );
            capture(app, "RIFT_GUARDS_RESPONSE");
            app.action(Action::Pass);
            app.action(Action::Pass);
        }
        7 => {
            assert!(app.game.chain.is_empty());
            assert!(app.game.players[0].units[0].ready);
            assert!(app.game.players[0].units[0].token);
            capture(app, "RIFT_GUARDS_READY");
            app.inspect = Some(448);
        }
        8 => capture(app, "RIFT_GUARDS_INSPECT"),
        10 => click(app, 315.0, 760.0),
        14 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerPayment { card_id: 448, .. })
            ));
            advice(app, AdviceLevel::Beginner);
            capture(app, "RIFT_GUARDS_DECLINE_PROMPT");
            click(app, 1280.0, 547.0);
        }
        15 => {
            assert!(app.game.decision.is_none());
            assert!(app.game.chain.is_empty());
            assert!(!app.game.players[0].units[0].ready);
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Order)],
                10
            );
            capture(app, "RIFT_GUARDS_DECLINED");
        }
        16 => return true,
        _ => panic!("Guards smoke exceeded its defined frames"),
    }
    false
}
