//! Drag a Reaction to a unit, choose its linked chain item, undo and confirm.
use super::*;
const UNIT: u32 = 44200;

fn setup(app: &mut App) {
    expansion_fixture(app, 442);
    app.game = Game::new_for_agents(3, 3, 442442, Difficulty::Easy);
    app.game.choose_starting_player(1).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![9, 9];
    app.game.battlefield_owner = vec![None; 2];
    for p in &mut app.game.players {
        p.units.clear();
        p.gear.clear();
        p.hand.clear();
        p.hidden.clear();
        p.discard.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy = 10;
        p.energy_pool = 10;
        p.power_pool = [10; 6];
    }
    app.game.players[0].hand = vec![442];
    app.game.players[0]
        .units
        .push(public_unit(UNIT, 69, Some(0)));
    app.game.players[1].hand = vec![53];
    let unit = Target::Unit {
        player: 0,
        unit_id: UNIT,
    };
    app.game
        .apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![unit, unit],
        })
        .unwrap();
    app.game.apply(Action::Pass).unwrap();
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
    app.companion.set_level(AdviceLevel::Beginner);
}

fn tip(app: &mut App, level: AdviceLevel) {
    app.companion.set_level(level);
    app.companion.observe(&app.game);
    for _ in 0..80 {
        if app.companion.message().contains("Repulse") {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("Repulse advice must be available during the response window");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    let target = Target::Unit {
        player: 0,
        unit_id: UNIT,
    };
    match frame {
        0 => setup(app),
        1 => {
            tip(app, AdviceLevel::Beginner);
            capture(app, "RIFT_REPULSE_BEGINNER");
            click(app, 315.0, 760.0);
            app.ui.pointer_override = Some((true, false));
        }
        2 | 3 => {
            let rect = app
                .drop_zones
                .iter()
                .find(|(zone, _)| matches!(zone, crate::DropZone::Unit(0, UNIT)))
                .unwrap()
                .1;
            app.ui.input_override = Some((
                vec2(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0),
                false,
                false,
            ));
            app.ui.pointer_override = Some((frame == 2, frame == 3));
        }
        4 => {
            assert_eq!(app.targets, vec![target]);
            assert_eq!(app.game.players[0].energy, 10);
            assert_eq!(app.game.players[0].hand, vec![442]);
            capture(app, "RIFT_REPULSE_UNIT");
            click(app, 1300.0, 320.0);
        }
        5 => {
            assert_eq!(
                app.targets,
                vec![
                    target,
                    Target::Spell {
                        chain_id: app.game.chain[0].id
                    }
                ]
            );
            capture(app, "RIFT_REPULSE_LINKED");
            click_unit(app, UNIT);
        }
        6 => {
            assert!(
                app.targets.is_empty(),
                "Undoing the unit must clear its dependent chain choice"
            );
            assert_eq!(app.game.players[0].energy, 10);
            click_unit(app, UNIT);
        }
        7 => {
            assert_eq!(app.targets, vec![target]);
            click(app, 1300.0, 320.0);
        }
        8 => {
            assert_eq!(app.targets.len(), 2);
            click(app, 1280.0, 510.0);
        }
        9 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 442);
            assert_eq!(app.game.players[0].energy, 9);
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Body)],
                9
            );
            tip(app, AdviceLevel::Advanced);
            capture(app, "RIFT_REPULSE_ADVANCED");
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        10 => {
            assert!(app.game.chain.is_empty());
            assert_eq!(app.game.players[0].units[0].damage, 0);
            assert!(app.game.players[1].discard.contains(&53));
            capture(app, "RIFT_REPULSE_COUNTERED");
            app.inspect = Some(442);
        }
        11 => capture(app, "RIFT_REPULSE_INSPECT"),
        12 => return true,
        _ => panic!("Repulse smoke exceeded its defined frames"),
    }
    false
}
