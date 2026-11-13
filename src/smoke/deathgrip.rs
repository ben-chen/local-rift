//! Controlled public board; actual drag/drop, confirmation and inspector controls.
use super::*;
const SACRIFICE: u32 = 44701;
const RECIPIENT: u32 = 44702;

fn setup(app: &mut App) {
    expansion_fixture(app, 447);
    app.game = Game::new_for_agents(2, 2, 447447, Difficulty::Easy);
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
    app.game.players[0].hand = vec![447];
    app.game.players[0].units = vec![
        public_unit(SACRIFICE, 69, Some(0)),
        public_unit(RECIPIENT, 69, None),
    ];
    app.player_deck = 2;
    app.opponent_deck = 2;
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
        if app.companion.message().contains("Deathgrip") {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("Deathgrip advice must be available while held");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    let targets = vec![
        Target::Unit {
            player: 0,
            unit_id: SACRIFICE,
        },
        Target::Unit {
            player: 0,
            unit_id: RECIPIENT,
        },
    ];
    match frame {
        0 => setup(app),
        1 => {
            advice(app, AdviceLevel::Beginner);
            capture(app, "RIFT_DEATHGRIP_BEGINNER");
            click(app, 315.0, 760.0);
            app.ui.pointer_override = Some((true, false));
        }
        2 | 3 => {
            let rect = unit_rect(app, SACRIFICE);
            app.ui.input_override = Some((
                vec2(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0),
                false,
                false,
            ));
            app.ui.pointer_override = Some((frame == 2, frame == 3));
        }
        4 => {
            assert_eq!(app.targets, targets[..1]);
            assert_eq!(app.game.players[0].energy, 10);
            assert!(app.target_hint(447).contains("receive its Might"));
            capture(app, "RIFT_DEATHGRIP_RECIPIENT");
            click_unit(app, RECIPIENT);
        }
        5 => {
            assert_eq!(app.targets, targets);
            assert_eq!(app.game.players[0].hand, vec![447]);
            advice(app, AdviceLevel::Advanced);
            capture(app, "RIFT_DEATHGRIP_TARGETS");
            click(app, 1280.0, 510.0);
        }
        6 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 447);
            assert_eq!(app.game.players[0].energy, 8);
            assert_eq!(app.game.players[0].units.len(), 2);
            assert!(app.game.players[0].hand.is_empty());
            capture(app, "RIFT_DEATHGRIP_RESPONSE");
            app.action(Action::Pass);
            app.action(Action::Pass);
        }
        7 => {
            assert!(app.game.chain.is_empty());
            assert_eq!(app.game.players[0].units.len(), 1);
            assert_eq!(app.game.players[0].units[0].id, RECIPIENT);
            assert_eq!(app.game.players[0].units[0].temporary_buff, 8);
            assert_eq!(app.game.players[0].hand, vec![71]);
            capture(app, "RIFT_DEATHGRIP_RESOLVED");
            app.inspect = Some(447);
        }
        8 => capture(app, "RIFT_DEATHGRIP_INSPECT"),
        9 => return true,
        _ => panic!("Deathgrip smoke exceeded its defined frames"),
    }
    false
}
