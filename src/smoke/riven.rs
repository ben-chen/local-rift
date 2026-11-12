//! Controlled positions exercise real field/unit clicks and late return choices.
//! Fixture construction is not a legal deck or a complete Riven match.
use super::*;
const ENEMY: u32 = 43702;
const ELSEWHERE: u32 = 43703;

fn field_click(app: &mut App, lane: usize) {
    let rect = app
        .drop_zones
        .iter()
        .find_map(|(zone, rect)| {
            matches!(zone, crate::DropZone::Battlefield(index) if *index == lane).then_some(*rect)
        })
        .expect("field surface");
    // The shared 80%-width click can hit an expanded unit card. Use the empty
    // left gutter so this test exercises the battlefield surface explicitly.
    click(app, rect.x + 12.0, rect.y + rect.h * 0.78);
}

fn setup(app: &mut App, crowded: bool, hidden: bool) {
    expansion_fixture(app, 437);
    app.game = Game::new_for_agents(3, 3, 437437, Difficulty::Easy);
    app.game.start_match().unwrap();
    app.game.battlefields = vec![9, 9];
    app.game.battlefield_owner = vec![Some(0), None];
    for p in &mut app.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy = 20;
        p.energy_pool = 20;
        p.power_pool = [10; 6];
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
    }
    app.game.players[0]
        .units
        .push(public_unit(43700, 71, Some(0)));
    if crowded {
        app.game.players[0]
            .units
            .push(public_unit(43701, 71, Some(0)));
    }
    app.game.players[1].units = vec![
        public_unit(ENEMY, 71, Some(0)),
        public_unit(ELSEWHERE, 71, Some(1)),
    ];
    if hidden {
        app.game.players[0]
            .hidden
            .push(local_rift::hidden::HiddenCard {
                id: 43799,
                card_id: 437,
                lane: 0,
                hidden_turn: 0,
                revealed: false,
            });
        app.game
            .apply(Action::PlayHidden {
                hidden_id: 43799,
                targets: vec![],
            })
            .unwrap();
    } else {
        app.game.players[0].hand = vec![437];
    }
    app.fields = std::array::from_fn(|i| app.game.battlefields[i]);
    app.ai_worker.invalidate();
    app.inspect = None;
    app.field_inspect = None;
    app.lesson = None;
    app.chain_cursor = 0;
    app.prefs.coach = true;
    app.companion.set_level(AdviceLevel::Beginner);
}

fn enemy_present(app: &App) -> bool {
    app.game.players[1].units.iter().any(|u| u.id == ENEMY)
}

fn fixture_play(app: &mut App) {
    app.game
        .apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: Some(0),
            targets: vec![],
        })
        .unwrap();
}

fn resolve(app: &mut App) {
    assert!(app.game.decision.is_none());
    app.game.apply(Action::Pass).unwrap();
    app.game.apply(Action::Pass).unwrap();
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    let field = Target::Battlefield { lane: 0 };
    let enemy = Target::Unit {
        player: 1,
        unit_id: ENEMY,
    };
    match frame {
        0 => setup(app, true, false),
        1 => {
            capture(app, "RIFT_RIVEN_STRANGER_HAND");
            click(app, 315.0, 760.0);
        }
        2 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            field_click(app, 0);
        }
        3 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerTargets { card_id: 437, .. })
            ));
            capture(app, "RIFT_RIVEN_STRANGER_FIELD");
            field_click(app, 1);
        }
        4 => {
            assert_eq!(app.targets, vec![Target::Battlefield { lane: 1 }]);
            click_unit(app, ELSEWHERE);
        }
        5 => {
            assert_eq!(app.targets.len(), 2);
            field_click(app, 1);
        }
        6 => {
            assert!(
                app.targets.is_empty(),
                "Clicking the chosen field must undo both dependent targets"
            );
            field_click(app, 0);
        }
        7 => {
            assert_eq!(app.targets, vec![field]);
            capture(app, "RIFT_RIVEN_STRANGER_ENEMY");
            click_unit(app, ENEMY);
        }
        8 => {
            assert_eq!(app.targets, vec![field, enemy]);
            capture(app, "RIFT_RIVEN_STRANGER_TARGETS");
            click_unit(app, ENEMY);
        }
        9 => {
            assert_eq!(app.targets, vec![field]);
            click_unit(app, ENEMY);
        }
        10 => {
            assert_eq!(app.targets, vec![field, enemy]);
            click(app, 1280.0, 545.0);
        }
        11 => {
            assert!(app.targets.is_empty());
            field_click(app, 0);
        }
        12 => click_unit(app, ENEMY),
        13 => {
            assert_eq!(app.targets, vec![field, enemy]);
            click(app, 1280.0, 510.0);
        }
        14 => {
            assert!(app.game.decision.is_none());
            assert!(enemy_present(app));
            assert_eq!(
                crate::chain_field_targets(app.game.chain.last().unwrap()),
                vec![0]
            );
            capture(app, "RIFT_RIVEN_STRANGER_RESPONSES");
        }
        15 => {
            resolve(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerReturn { .. })
            ));
        }
        16 => {
            // Capture the settled frame after the new prompt's font glyphs load.
            capture(app, "RIFT_RIVEN_STRANGER_RETURN");
            click(app, 1280.0, 545.0);
        }
        17 => {
            assert!(enemy_present(app));
            assert!(app.game.decision.is_none() && app.game.chain.is_empty());
            assert_eq!(app.game.turn, 1);
            capture(app, "RIFT_RIVEN_STRANGER_DECLINED");
            setup(app, true, false);
            fixture_play(app);
        }
        18 => field_click(app, 0),
        19 => click_unit(app, ENEMY),
        20 => click(app, 1280.0, 510.0),
        21 => {
            resolve(app);
            app.companion.set_level(AdviceLevel::Advanced);
            capture(app, "RIFT_RIVEN_STRANGER_ADVANCED");
        }
        22 => click(app, 1280.0, 505.0),
        23 => {
            assert!(!enemy_present(app));
            assert_eq!(app.game.players[1].hand, vec![71]);
            assert!(app.game.decision.is_none() && app.game.chain.is_empty());
            assert_eq!(app.game.turn, 1);
            capture(app, "RIFT_RIVEN_STRANGER_RETURNED");
            app.inspect = Some(437);
        }
        24 => {
            capture(app, "RIFT_RIVEN_STRANGER_RULES");
            setup(app, true, true);
        }
        25 => {
            capture(app, "RIFT_RIVEN_STRANGER_HIDDEN");
            field_click(app, 1);
        }
        26 => {
            assert!(app.targets.is_empty());
            field_click(app, 0);
        }
        27 => {
            assert_eq!(app.targets, vec![field]);
            click_unit(app, ELSEWHERE);
        }
        28 => {
            assert_eq!(app.targets, vec![field]);
            click_unit(app, ENEMY);
        }
        29 => {
            assert_eq!(app.targets, vec![field, enemy]);
            click(app, 1280.0, 510.0);
        }
        30 => {
            resolve(app);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerReturn { .. })
            ));
            click(app, 1280.0, 505.0);
        }
        31 => {
            assert!(!enemy_present(app));
            setup(app, false, false);
            fixture_play(app);
        }
        32 => field_click(app, 0),
        33 => click_unit(app, ENEMY),
        34 => click(app, 1280.0, 510.0),
        35 => {
            resolve(app);
            assert!(app.game.decision.is_none() && app.game.chain.is_empty());
            assert!(enemy_present(app));
            capture(app, "RIFT_RIVEN_STRANGER_UNCROWDED");
        }
        36 => return true,
        _ => {}
    }
    false
}
