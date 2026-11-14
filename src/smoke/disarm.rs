//! Real drag-to-attack, mandatory target choice, response and advice controls.
use super::*;
const SOURCE: u32 = 45901;
const GRANT: u32 = 45902;
const DEFENDER: u32 = 45903;

fn setup(app: &mut App, siren: bool) {
    let card = if siren { 460 } else { 459 };
    let preset = if siren { 0 } else { 3 };
    expansion_fixture(app, card);
    app.game = Game::new_for_agents(preset, 3, 459459, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![3, 3];
    app.game.battlefield_owner = vec![Some(1), Some(0)];
    for p in &mut app.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 6;
        p.energy = 6;
        p.power_pool = [0; 6];
        p.any_power = 0;
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
    }
    app.game.players[0].hand = vec![if siren { 460 } else { 458 }];
    app.game.players[0].units = vec![public_unit(SOURCE, card, siren.then_some(1))];
    if !siren {
        app.game.players[0]
            .units
            .push(public_unit(GRANT, 459, Some(1)));
    }
    app.game.players[1].units = vec![public_unit(DEFENDER, 110, Some(0))];
    app.player_deck = preset;
    app.opponent_deck = 3;
    app.fields = [3, 3];
    app.clear_game_controls();
    app.lesson = None;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.companion.set_level(AdviceLevel::Beginner);
    app.ai_worker.invalidate();
}

fn advice(app: &mut App) {
    app.companion.observe(&app.game);
    for _ in 0..100 {
        if app.companion.message().contains("Disarm") {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("missing Disarm advice");
}

fn drag_start(app: &mut App) {
    click_unit(app, SOURCE);
    app.ui.pointer_override = Some((true, false));
}
fn drag_to_field(app: &mut App, release: bool) {
    click_field(app, 0);
    let point = app.ui.input_override.unwrap().0;
    app.ui.input_override = Some((point, false, false));
    app.ui.pointer_override = Some((!release, release));
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app, false),
        1 => app.ui.input_override = Some((vec2(315.0, 760.0), false, true)),
        2 => {
            assert_eq!(app.inspect, Some(458));
            capture(app, "RIFT_DISARM_PEDDLER");
            app.inspect = None;
            app.ui.input_override = Some((unit_rect(app, SOURCE).center(), false, false));
        }
        3 => {
            assert_eq!(app.hover_object, Some((0, SOURCE)));
            let text = super::super::live_unit_description(&app.game, 0, SOURCE).unwrap();
            assert!(text.contains("Disarm 2"));
            capture_unit(app, SOURCE, "RIFT_DISARM_SENTRY");
        }
        4 => drag_start(app),
        5 => drag_to_field(app, false),
        6 => drag_to_field(app, true),
        7 => {
            assert!(
                matches!(app.game.decision, Some(Decision::TriggerTargets { context, .. })
                if matches!(context.keyword, Some(local_rift::game::KeywordTrigger::Disarm { value: 2, .. })))
            );
            advice(app);
            capture(app, "RIFT_DISARM_BEGINNER");
            click(app, 150.0, 600.0);
        }
        8 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            advice(app);
            capture(app, "RIFT_DISARM_ADVANCED");
            click_unit(app, DEFENDER);
        }
        9 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: DEFENDER
                }]
            );
            click(app, 1280.0, 510.0);
        }
        10 => {
            assert_eq!(app.game.chain.len(), 1);
            assert_eq!(
                app.game.chain_name(&app.game.chain[0]),
                "Clanging Sentry · Disarm 2"
            );
            capture(app, "RIFT_DISARM_RESPONSE");
            click(app, 1285.0, 595.0);
        }
        11 => {
            assert_eq!(app.game.priority(), 1, "Mote must not block passing");
            app.game.apply(Action::Pass).unwrap();
        }
        12 => {
            assert_eq!(app.game.players[1].units[0].temporary_buff, -2);
            let text = super::super::live_unit_description(&app.game, 1, DEFENDER).unwrap();
            assert!(text.contains("Might -2 this turn"));
            assert!(text.contains("expires at the end of this turn"));
            app.ui.input_override = Some((unit_rect(app, DEFENDER).center(), false, false));
        }
        13 => {
            assert_eq!(app.hover_object, Some((1, DEFENDER)));
            capture_unit(app, DEFENDER, "RIFT_DISARM_REDUCED");
        }
        14 => setup(app, true),
        15 => app.ui.input_override = Some((vec2(315.0, 760.0), false, true)),
        16 => {
            assert_eq!(app.inspect, Some(460));
            capture(app, "RIFT_DISARM_SIREN");
            app.inspect = None;
        }
        17 => drag_start(app),
        18 => drag_to_field(app, false),
        19 => drag_to_field(app, true),
        20 => {
            assert_eq!(
                app.game.players[0].units[0].lane,
                Some(0),
                "Ganking moves between fields"
            );
            assert!(
                matches!(app.game.decision, Some(Decision::TriggerTargets { context, .. })
                if matches!(context.keyword, Some(local_rift::game::KeywordTrigger::Disarm { value: 1, .. })))
            );
            advice(app);
            capture(app, "RIFT_DISARM_GANKING");
            click_unit(app, DEFENDER);
        }
        21 => click(app, 1280.0, 510.0),
        22 => {
            assert_eq!(
                app.game.chain_name(&app.game.chain[0]),
                "Siren of the Shallows · Disarm 1"
            );
            click(app, 1285.0, 595.0);
        }
        23 => {
            app.game.apply(Action::Pass).unwrap();
        }
        24 => assert_eq!(app.game.players[1].units[0].temporary_buff, -1),
        25 => return true,
        _ => panic!("Disarm smoke exceeded its frames"),
    }
    false
}
