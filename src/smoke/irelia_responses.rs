//! Gust coaching layout, actual advice switches and an unobstructed priority button.
use super::*;

const PYKE: u32 = 64000;

fn setup(app: &mut App) {
    backoff_setup(app); // Reset the existing selection, inspector and payment controls.
    app.game = Game::new_for_agents(0, 1, 64000, Difficulty::Easy);
    app.game.choose_starting_player(1).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![3, 3];
    app.game.battlefield_owner = vec![Some(0), None];
    for player in &mut app.game.players {
        player.units.clear();
        player.gear.clear();
        player.hand.clear();
        player.hidden.clear();
        player.discard.clear();
        player.champion = None;
        player.deck = vec![71; 20];
        player.set_rune_setup([0; 6], [0; 6]).unwrap();
        player.energy = 2;
        player.energy_pool = 2;
        player.power_pool = [0; 6];
        player.any_power = 0;
    }
    app.game.players[0].units = vec![public_unit(PYKE, 13, Some(0))];
    app.game.players[0].hand = vec![8, 1];
    app.game.players[1].hand = vec![11];
    app.game.legend[0].ready = false;
    app.game
        .apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: 0,
                unit_id: PYKE,
            }],
        })
        .unwrap();
    app.game.apply(Action::Pass).unwrap();
    app.player_deck = 0;
    app.opponent_deck = 1;
    app.fields = [3, 3];
    app.inspect = None;
    app.field_inspect = None;
    app.companion.set_level(AdviceLevel::Beginner);
    app.ai_worker.invalidate();
}

pub(super) fn run_frame(app: &mut App, frame: u32) -> bool {
    match frame {
        0 => setup(app),
        1 => {
            assert!(app.companion.message().contains("3 Might"));
            capture(app, "RIFT_GUST_RESPONSE_BEGINNER");
            click(app, 150.0, 600.0);
        }
        2 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            assert!(app.companion.message().contains("Abandon"));
            capture(app, "RIFT_GUST_RESPONSE_ADVANCED");
            // Rules/decision tests exercise the choice; this fixture advances
            // it directly to review the new commentary after the buff resolves.
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![Target::Unit {
                        player: 0,
                        unit_id: PYKE,
                    }],
                })
                .unwrap();
            app.game.apply(Action::Choose { accept: false }).unwrap();
            for _ in 0..8 {
                if app.game.chain.last().is_some_and(|item| item.card_id == 11) {
                    break;
                }
                app.game.apply(Action::Pass).unwrap();
            }
            if app.game.priority() != 0 {
                app.game.apply(Action::Pass).unwrap();
            }
            assert_eq!(app.game.chain.last().unwrap().card_id, 11);
            assert_eq!(
                app.game.unit_might(&app.game.players[0].units[0]).unwrap(),
                5
            );
            click(app, 60.0, 600.0);
        }
        3 => {
            assert_eq!(app.companion.level(), AdviceLevel::Beginner);
            assert!(app.companion.message().contains("can't affect"));
            capture(app, "RIFT_GUST_PROTECTED_BEGINNER");
            click(app, 150.0, 600.0);
        }
        4 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            assert!(app.companion.message().contains("Passing"));
            capture(app, "RIFT_GUST_PROTECTED_ADVANCED");
            click(app, 1280.0, 597.0);
        }
        5 => {
            assert!(app.game.chain.is_empty() || app.game.priority() == 1);
            if !app.game.chain.is_empty() {
                app.game.apply(Action::Pass).unwrap();
            }
            assert!(app.game.chain.is_empty());
            assert_eq!(app.game.players[0].hand, vec![1]);
            assert_eq!(app.game.players[0].energy, 1);
            assert_eq!(app.game.players[0].units[0].id, PYKE);
            return true;
        }
        _ => panic!("Gust coaching smoke exceeded its frames"),
    }
    false
}
