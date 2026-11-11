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
            expansion_fixture(app, 425);
            app.game = Game::new_for_agents(3, 3, 425425, Difficulty::Easy);
            app.game.start_match().unwrap();
            app.game.battlefields = [2, 2];
            app.game.battlefield_owner = [None; 2];
            for p in &mut app.game.players {
                p.units.clear();
                p.gear.clear();
                p.hidden.clear();
                p.hand.clear();
                p.champion = None;
                p.energy_pool = 20;
                p.energy = 20;
                p.power_pool = [10; 6];
                p.set_rune_setup([0; 6], [0; 6]).unwrap();
            }
            let mut unit = public_unit(42500, 73, None);
            unit.ready = false;
            app.game.players[0].units = vec![unit];
            app.game.players[0].hand = vec![425];
            app.game.players[0].deck = vec![71; 10];
            app.fields = app.game.battlefields;
            app.ai_worker.invalidate();
            app.prefs.exact_payment = false;
            app.companion.set_level(AdviceLevel::Beginner);
        }
        13 => {
            capture(app, "RIFT_IRELIA_HUNKER_HAND");
            click(app, 315.0, 760.0);
        }
        14 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            click_unit(app, 42500);
        }
        15 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: 42500
                }]
            );
            click(app, 1285.0, 510.0);
        }
        16 => {
            assert!(app.game.chain.iter().any(|item| item.card_id == 425));
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            assert!(app.game.players[0].units[0].ready);
            assert!(app.game.movement_prevented(1, 0, 42500));
            assert_eq!(app.game.players[0].hand, vec![71]);
            assert!(!app.game.action_hint().contains("ready unit at base"));
            capture(app, "RIFT_IRELIA_HUNKER_LOCKED");
            click_unit(app, 42500);
        }
        17 => {
            assert!(app.selection == Some(Selection::Unit(42500)));
            // The selected-unit movement button is disabled, not a failed action.
            app.toast_until = 0.0;
            click(app, 1285.0, 420.0);
        }
        18 => {
            assert_eq!(app.game.players[0].units[0].lane, None);
            assert_eq!(app.toast_until, 0.0);
            assert!(
                super::super::live_unit_description(&app.game, 0, 42500)
                    .unwrap()
                    .contains("either player's spells")
            );
            capture_unit(app, 42500, "RIFT_IRELIA_HUNKER_HOVER");
        }
        19 => {
            app.inspect = Some(425);
            app.companion.set_level(AdviceLevel::Advanced);
            capture(app, "RIFT_IRELIA_HUNKER_INSPECT");
            click(app, 940.0, 700.0);
        }
        20 => {
            assert!(app.inspect.is_none());
        }
        21 => {
            expansion_fixture(app, 426);
            app.game = Game::new_for_agents(3, 3, 426426, Difficulty::Easy);
            app.game.start_match().unwrap();
            app.game.battlefields = [2, 2];
            app.game.battlefield_owner = [None; 2];
            for p in &mut app.game.players {
                p.units.clear();
                p.gear.clear();
                p.hidden.clear();
                p.hand.clear();
                p.champion = None;
                p.set_rune_setup([0; 6], [0; 6]).unwrap();
                p.energy_pool = 0;
                p.energy = 0;
                p.power_pool = [0; 6];
            }
            app.game.players[0].units = vec![public_unit(42600, 73, None)];
            app.game.players[0].hand = vec![426, 58];
            app.game.players[0]
                .set_rune_setup([0, 2, 0, 0, 0, 0], [0; 6])
                .unwrap();
            app.game.players[0].energy = 2;
            app.fields = app.game.battlefields;
            app.ai_worker.invalidate();
            app.prefs.exact_payment = false;
            app.companion.set_level(AdviceLevel::Beginner);
        }
        22 => {
            capture(app, "RIFT_IRELIA_PREP_HAND");
            click(app, 315.0, 760.0);
        }
        23 => {}
        24 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            assert!(app.play_draft.as_ref().unwrap().open);
            capture(app, "RIFT_IRELIA_PREP_SHOW");
            click(app, 450.0, 770.0);
        }
        25 => {
            assert_eq!(app.play_draft.as_ref().unwrap().choices.show_off.len(), 1);
            click_unit(app, 42600);
        }
        26 => {
            assert_eq!(app.targets.len(), 1);
            click(app, 430.0, 606.0);
        }
        27 => {
            assert_eq!(app.targets.len(), 2);
            click(app, 463.0, 606.0);
        }
        28 => {
            assert_eq!(app.targets.len(), 3);
            assert!(app.rune_menu.is_none());
            capture(app, "RIFT_IRELIA_PREP_TARGETS");
            click(app, 1285.0, 510.0);
        }
        29 => {
            let spell = app.game.chain.iter().find(|c| c.card_id == 426).unwrap();
            assert_eq!(spell.show_off.len(), 1);
            assert_eq!(spell.targets.len(), 3);
            assert!(app.game.players[0].rune_objects().iter().all(|r| !r.ready));
            capture(app, "RIFT_IRELIA_PREP_CHAIN");
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        30 => {
            assert_eq!(app.game.players[0].units[0].temporary_buff, 2);
            assert!(app.game.players[0].rune_objects().iter().all(|r| r.ready));
            capture(app, "RIFT_IRELIA_PREP_READY");
            app.inspect = Some(426);
            app.companion.set_level(AdviceLevel::Advanced);
        }
        31 => {
            capture(app, "RIFT_IRELIA_PREP_INSPECT");
            click(app, 940.0, 700.0);
        }
        32 => {
            assert!(app.inspect.is_none());
            return true;
        }

        _ => {}
    }
    false
}
