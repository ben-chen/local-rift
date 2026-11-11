//! Irelia's optional object cost through real native accept/decline controls.
use super::*;
const PREFECT: u32 = 42400;
fn setup(app: &mut App) {
    expansion_fixture(app, 70);
    app.game = Game::new_for_agents(3, 3, 424424, Difficulty::Easy);
    app.game.choose_starting_player(1).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = [2, 2];
    app.game.battlefield_owner = [None; 2];
    for p in &mut app.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 20;
        p.energy = 20;
        p.power_pool = [10; 6];
    }
    app.game.players[0]
        .units
        .push(public_unit(PREFECT, 424, None));
    app.game.players[1].hand = vec![235];
    app.game
        .apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![],
        })
        .unwrap();
    assert!(matches!(
        app.game.decision,
        Some(Decision::TriggerObjectCost { player: 0, .. })
    ));
    app.fields = app.game.battlefields;
    app.ai_worker.invalidate();
    app.inspect = None;
    app.field_inspect = None;
    app.lesson = None;
    app.chain_cursor = 0;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.companion.set_level(AdviceLevel::Beginner);
}
pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app),
        1 => {
            // Allow the first render to populate the font atlas before capture.
            capture(app, "RIFT_IRELIA_COST");
            click(app, 1280.0, 505.0);
        }
        2 => {
            assert!(app.game.players[0].units.is_empty());
            assert_eq!(app.game.players[0].banish, vec![424]);
            assert_eq!(app.game.players[1].gear.len(), 1);
            assert_eq!(app.game.chain.len(), 1);
            capture(app, "RIFT_IRELIA_PAID");
        }
        3 => {
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            assert_eq!(app.game.players[1].banish, vec![235]);
            capture(app, "RIFT_IRELIA_RESOLVED");
        }
        4 => {
            setup(app);
            app.companion.set_level(AdviceLevel::Advanced);
            capture(app, "RIFT_IRELIA_ADVANCED");
        }
        5 => click(app, 1280.0, 545.0),
        6 => {
            assert!(app.game.decision.is_none());
            assert!(app.game.chain.is_empty());
            assert_eq!(app.game.players[0].units.len(), 1);
            assert_eq!(app.game.players[1].gear.len(), 1);
            capture(app, "RIFT_IRELIA_DECLINED");
        }
        7 => {
            setup(app);
            app.game.players[0].units.clear();
            capture(app, "RIFT_IRELIA_UNAVAILABLE");
        }
        8 => click(app, 1280.0, 505.0),
        9 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerObjectCost { .. })
            ));
            click(app, 1280.0, 545.0);
        }
        10 => {
            assert!(app.game.decision.is_none());
            setup(app);
            app.inspect = Some(424);
            capture(app, "RIFT_IRELIA_PREFECT");
        }
        11 => click(app, 940.0, 700.0),
        12 => {
            assert!(app.inspect.is_none());
            return true;
        }
        _ => {}
    }
    false
}
