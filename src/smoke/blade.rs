//! Real native Equip choices in controlled positions; not a complete Riven deck.
use super::*;
const GEAR: u32 = 43800;
const FIRST: u32 = 43801;
const SECOND: u32 = 43802;
const SPENT: u32 = 43803;
const ENEMY: u32 = 43804;

fn setup(app: &mut App, innovator: bool) {
    expansion_fixture(app, 437);
    app.game = Game::new_for_agents(3, 3, 438438, Difficulty::Easy);
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
        p.energy = 20;
        p.energy_pool = 20;
        p.power_pool = [0; 6];
        p.any_power = 0;
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
        legend.spell_power = 0;
    }
    app.game.players[0].units = vec![
        public_unit(FIRST, 71, None),
        public_unit(SECOND, if innovator { 429 } else { 71 }, None),
    ];
    app.game.players[0].gear.push(GearState {
        id: GEAR,
        card_id: 438,
        lane: None,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    app.fields = std::array::from_fn(|i| app.game.battlefields[i]);
    app.ai_worker.invalidate();
    app.inspect = None;
    app.field_inspect = None;
    app.lesson = None;
    app.chain_cursor = 0;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.equipment_cost = None;
    app.tray = crate::Tray::Loadout;
    app.gear_owner = 0;
    app.companion.set_level(AdviceLevel::Beginner);
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    let first = Target::Unit {
        player: 0,
        unit_id: FIRST,
    };
    let second = Target::Unit {
        player: 0,
        unit_id: SECOND,
    };
    match frame {
        0 => setup(app, false),
        1 => {
            capture(app, "RIFT_BLADE_LOADOUT");
            click(app, 615.0, 760.0);
        }
        2 => {
            assert!(app.selection == Some(Selection::Gear(GEAR)));
            capture(app, "RIFT_BLADE_PAYMENT_OPTIONS");
            click_unit(app, FIRST);
        }
        3 => {
            assert_eq!(app.targets, vec![first]);
            click(app, 1340.0, 469.0);
        }
        4 => {
            assert!(app.selected_equipment_cost().is_some());
            click_unit(app, SECOND);
        }
        5 => {
            assert_eq!(app.targets, vec![first]);
            assert_eq!(app.selected_equipment_cost().unwrap().unit_id, Some(SECOND));
            assert!(app.game.players[0].units.iter().all(|u| u.ready));
            capture(app, "RIFT_BLADE_EXHAUST_COST");
            click(app, 1280.0, 438.0);
        }
        6 => {
            assert!(!app.selected_equipment_cost().unwrap().choosing_cost);
            click_unit(app, SECOND);
        }
        7 => {
            assert_eq!(app.targets, vec![second]);
            assert_eq!(app.selected_equipment_cost().unwrap().unit_id, Some(SECOND));
            click(app, 1280.0, 499.0);
        }
        8 => click_unit(app, FIRST),
        9 => {
            assert_eq!(app.targets, vec![second]);
            assert_eq!(app.selected_equipment_cost().unwrap().unit_id, Some(FIRST));
            capture(app, "RIFT_BLADE_INDEPENDENT_CHOICES");
            click(app, 1280.0, 548.0);
        }
        10 => {
            assert!(!app.game.players[0].units[0].ready);
            assert!(app.game.players[0].units[1].ready);
            assert_eq!(app.game.players[0].gear[0].attached_to, None);
            assert_eq!(app.game.chain.len(), 1);
            capture(app, "RIFT_BLADE_PENDING_ATTACH");
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        11 => {
            assert_eq!(app.game.players[0].gear[0].attached_to, Some(SECOND));
            assert_eq!(
                app.game.unit_might(&app.game.players[0].units[1]).unwrap(),
                8
            );
            capture_unit(app, SECOND, "RIFT_BLADE_ATTACHED");
        }
        12 => app.inspect = Some(438),
        13 => {
            capture(app, "RIFT_BLADE_RULES");
            setup(app, false);
            app.game.players[0].power_pool[2] = 1;
        }
        14 => click(app, 615.0, 760.0),
        15 => click_unit(app, FIRST),
        16 => {
            assert!(app.selected_equipment_cost().is_none());
            click(app, 1280.0, 548.0);
        }
        17 => {
            assert_eq!(app.game.players[0].power_pool[2], 0);
            assert!(app.game.players[0].units.iter().all(|u| u.ready));
            assert_eq!(app.game.players[0].gear[0].attached_to, None);
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        18 => {
            assert_eq!(app.game.players[0].gear[0].attached_to, Some(FIRST));
            capture(app, "RIFT_BLADE_POWER_PAID");
            setup(app, true);
        }
        19 => click(app, 615.0, 760.0),
        20 => click_unit(app, FIRST),
        21 => click(app, 1340.0, 469.0),
        22 => click_unit(app, SECOND),
        23 => click(app, 1280.0, 548.0),
        24 => {
            assert!(!app.game.players[0].units[1].ready);
            assert_eq!(app.game.players[0].gear[0].attached_to, None);
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerMode { card_id: 429, .. })
            ));
            capture(app, "RIFT_BLADE_INNOVATOR_TRIGGER");
        }
        25 => {
            setup(app, false);
            let mut spent = public_unit(SPENT, 71, None);
            spent.ready = false;
            app.game.players[0].units.push(spent);
            app.game.players[1].units.push(public_unit(ENEMY, 71, None));
        }
        26 => click(app, 615.0, 760.0),
        27 => {
            assert!(app.equipment_cost_candidate(Target::Unit {
                player: 0,
                unit_id: SPENT
            }));
            assert!(!app.equipment_cost_candidate(Target::Unit {
                player: 1,
                unit_id: ENEMY
            }));
            click_unit(app, ENEMY);
        }
        28 => {
            assert!(app.targets.is_empty());
            click_unit(app, SPENT);
        }
        29 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: SPENT
                }]
            );
            click(app, 1340.0, 469.0);
        }
        30 => {
            assert!(!app.equipment_cost_candidate(Target::Unit {
                player: 0,
                unit_id: SPENT
            }));
            assert!(app.equipment_cost_candidate(first));
            click_unit(app, SPENT);
        }
        31 => {
            assert_eq!(app.selected_equipment_cost().unwrap().unit_id, None);
            click_unit(app, ENEMY);
        }
        32 => {
            assert_eq!(app.selected_equipment_cost().unwrap().unit_id, None);
            click_unit(app, FIRST);
        }
        33 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: SPENT
                }]
            );
            assert_eq!(app.selected_equipment_cost().unwrap().unit_id, Some(FIRST));
            capture(app, "RIFT_BLADE_EXHAUSTED_BEARER");
            click(app, 1280.0, 548.0);
        }
        34 => {
            assert!(!app.game.players[0].units[0].ready);
            assert!(!app.game.players[0].units[2].ready);
            assert_eq!(app.game.players[0].gear[0].attached_to, None);
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        35 => {
            assert_eq!(app.game.players[0].gear[0].attached_to, Some(SPENT));
            assert!(!app.game.players[0].units[2].ready);
            capture_unit(app, SPENT, "RIFT_BLADE_EXHAUSTED_ATTACHED");
        }
        36 => return true,
        _ => {}
    }
    false
}
