//! Physical card selection and a real opposing response before subset selection.
use super::*;

const TWO: u32 = 46501;
const THREE: u32 = 46502;
const ZERO: u32 = 46503;
fn target(id: u32) -> Target {
    Target::Unit {
        player: 1,
        unit_id: id,
    }
}

fn setup(app: &mut App) {
    expansion_fixture(app, 465);
    app.game = Game::new_for_agents(0, 3, 465465, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![3, 3];
    app.game.battlefield_owner = vec![Some(1), None];
    for p in &mut app.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy_pool = 12;
        p.energy = 12;
        p.power_pool = [5; 6];
        p.any_power = 0;
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
    }
    app.game.players[0].hand = vec![465];
    app.game.players[1].hand = vec![7];
    let mut zero = public_unit(ZERO, 88, Some(0));
    zero.temporary_buff = -2;
    app.game.players[1].units = vec![
        public_unit(TWO, 88, Some(0)),
        public_unit(THREE, 89, Some(0)),
        zero,
    ];
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
        if app.companion.message().contains("Decree") {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("missing Decree advice");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100., -100.), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.);
    match frame {
        0 => setup(app),
        1 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        2 => {
            assert_eq!(app.inspect, Some(465));
            capture(app, "RIFT_DISCORD_INSPECT");
            app.inspect = None;
        }
        3 => click(app, 315., 760.),
        4 => {
            assert!(matches!(app.selection, Some(Selection::Hand(0))));
            click_unit(app, TWO);
        }
        5 => {
            assert_eq!(app.targets, vec![target(TWO)]);
            click_unit(app, THREE);
        }
        6 => {
            assert_eq!(app.targets.len(), 2);
            click_unit(app, ZERO);
        }
        7 => {
            assert_eq!(app.targets.len(), 3);
            capture(app, "RIFT_DISCORD_TARGETS");
            click(app, 1280., 510.);
        }
        8 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 465);
            advice(app);
            capture(app, "RIFT_DISCORD_BEGINNER");
            click(app, 150., 600.);
        }
        9 => {
            assert_eq!(app.companion.level(), AdviceLevel::Advanced);
            advice(app);
            capture(app, "RIFT_DISCORD_ADVANCED");
            if app.game.priority() == 0 {
                app.game.apply(Action::Pass).unwrap();
            }
            assert_eq!(app.game.priority(), 1);
            app.game
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane: None,
                    targets: vec![target(TWO)],
                })
                .unwrap();
            finish_chain_until_choice(app);
        }
        10 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::ResolutionSubset { .. })
            ));
            assert!(app.targets.is_empty());
            capture(app, "RIFT_DISCORD_SUBSET");
            click_unit(app, TWO);
        }
        11 => {
            assert_eq!(app.targets, vec![target(TWO)]);
            assert!(app.game.validate_resolution_subset(&app.targets).is_err());
            capture(app, "RIFT_DISCORD_INCLUDE_ZERO");
            click(app, 1280., 532.);
        }
        12 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::ResolutionSubset { .. })
            ));
            click_unit(app, ZERO);
        }
        13 => {
            assert_eq!(app.targets, vec![target(TWO), target(ZERO)]);
            assert!(app.game.validate_resolution_subset(&app.targets).is_ok());
            capture(app, "RIFT_DISCORD_FOUR_MIGHT");
            click_unit(app, TWO);
        }
        14 => {
            assert_eq!(app.targets, vec![target(ZERO)]);
            click_unit(app, THREE);
        }
        15 => {
            assert_eq!(app.targets, vec![target(ZERO), target(THREE)]);
            assert!(app.game.validate_resolution_subset(&app.targets).is_ok());
            capture(app, "RIFT_DISCORD_THREE_MIGHT");
            click(app, 1280., 532.);
        }
        16 => {
            assert!(app.game.chain.is_empty() && app.game.decision.is_none());
            assert_eq!(app.game.players[1].units.len(), 1);
            assert_eq!(app.game.players[1].units[0].id, TWO);
            assert!(app.game.players[0].discard.contains(&465));
            capture(app, "RIFT_DISCORD_RESOLVED");
        }
        17 => return true,
        _ => panic!("Discord smoke exceeded script"),
    }
    false
}
