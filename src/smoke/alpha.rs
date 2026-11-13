//! Real drag, ordered targets, allocation edits and rejected confirmation.
use super::*;
const SOURCE: u32 = 44300;
const FIRST: u32 = 44301;
const SECOND: u32 = 44302;
fn target(player: usize, unit_id: u32) -> Target {
    Target::Unit { player, unit_id }
}
fn setup(app: &mut App, might: i64) {
    expansion_fixture(app, 443);
    app.game = Game::new_for_agents(3, 3, 443443, Difficulty::Easy);
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
        p.xp = 0;
        p.set_rune_setup([0; 6], [0; 6]).unwrap();
        p.energy = 10;
        p.energy_pool = 10;
        p.power_pool = [10; 6];
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
    }
    for (player, id, power, lane) in [
        (0, SOURCE, might, None),
        (1, FIRST, 3, Some(0)),
        (1, SECOND, 8, Some(1)),
    ] {
        let mut unit = public_unit(id, 69, lane);
        unit.temporary_buff = power - 8;
        app.game.players[player].units.push(unit);
    }
    app.game.players[0].hand = vec![443];
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
    app.damage_amounts = None;
    app.damage_order.clear();
    app.damage_editor = None;
}
pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app, 5),
        1 => {
            click(app, 315.0, 760.0);
            app.ui.pointer_override = Some((true, false));
        }
        2 | 3 => {
            let r = unit_rect(app, SOURCE);
            app.ui.input_override = Some((vec2(r.x + r.w / 2.0, r.y + r.h / 2.0), false, false));
            app.ui.pointer_override = Some((frame == 2, frame == 3));
        }
        4 => {
            assert_eq!(app.targets, vec![target(0, SOURCE)]);
            assert_eq!(app.game.players[0].energy, 10);
            capture(app, "RIFT_ALPHA_SOURCE");
            click_unit(app, FIRST);
        }
        5 => {
            assert_eq!(app.targets, vec![target(0, SOURCE), target(1, FIRST)]);
            click_unit(app, SECOND);
        }
        6 => {
            assert_eq!(app.targets.len(), 3);
            capture(app, "RIFT_ALPHA_TARGETS");
            click_unit(app, SOURCE);
        }
        7 => {
            assert!(
                app.targets.is_empty(),
                "Removing the source must also clear its recipients"
            );
            assert_eq!(app.game.players[0].hand, vec![443]);
            click_unit(app, SOURCE);
        }
        8 => click_unit(app, FIRST),
        9 => click_unit(app, SECOND),
        10 => {
            assert_eq!(app.targets.len(), 3);
            click(app, 1280.0, 510.0);
        }
        11 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 443);
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[0].energy, 7);
            capture(app, "RIFT_ALPHA_RESPONSES");
            app.action(Action::Pass);
            app.action(Action::Pass);
        }
        12 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::SplitDamage {
                    total: 5,
                    keep: 2,
                    ..
                })
            ));
            assert_eq!(app.damage_amounts, Some(vec![(FIRST, 3), (SECOND, 2)]));
            capture(app, "RIFT_ALPHA_ALLOCATION");
            click(app, 266.0, 845.0);
        }
        13 => {
            assert!(app.damage_validation(usize::MAX, 5).is_err());
            assert_eq!(app.damage_amounts, Some(vec![(FIRST, 2), (SECOND, 2)]));
            capture(app, "RIFT_ALPHA_INVALID");
            click(app, 1285.0, 510.0);
        }
        14 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::SplitDamage { .. })
            ));
            assert!(app.game.players[1].units.iter().all(|u| u.damage == 0));
            assert_eq!(app.damage_amounts, Some(vec![(FIRST, 2), (SECOND, 2)]));
            click(app, 510.0, 845.0); // Second target's plus button.
        }
        15 => {
            assert_eq!(app.damage_amounts, Some(vec![(FIRST, 2), (SECOND, 3)]));
            assert!(app.damage_validation(usize::MAX, 5).is_ok());
            app.companion.set_level(AdviceLevel::Advanced);
            capture(app, "RIFT_ALPHA_ADVANCED");
            click(app, 1285.0, 510.0);
        }
        16 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[1].units[0].damage, 2);
            assert_eq!(app.game.players[1].units[1].damage, 3);
            assert_eq!(app.game.players[0].xp, 0);
            assert!(app.damage_amounts.is_none());
            capture(app, "RIFT_ALPHA_APPLIED");
            app.inspect = Some(443);
        }
        17 => capture(app, "RIFT_ALPHA_INSPECT"),
        18 => {
            setup(app, 3);
            app.action(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![target(0, SOURCE), target(1, FIRST), target(1, SECOND)],
            });
            app.action(Action::Pass);
            app.game.players[1].hand = vec![136];
            app.action(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![target(0, SOURCE)],
            });
            for _ in 0..4 {
                app.action(Action::Pass);
            }
            assert!(matches!(
                app.game.decision,
                Some(Decision::SplitDamage {
                    total: 1,
                    keep: 1,
                    ..
                })
            ));
        }
        19 => {
            assert_eq!(
                app.damage_assignment(usize::MAX, 1).unwrap(),
                vec![(FIRST, 1)]
            );
            capture(app, "RIFT_ALPHA_REDUCED");
            click(app, 266.0, 845.0);
        }
        20 => {
            assert!(app.damage_validation(usize::MAX, 1).is_err());
            click(app, 510.0, 845.0);
        }
        21 => {
            assert_eq!(
                app.damage_assignment(usize::MAX, 1).unwrap(),
                vec![(SECOND, 1)]
            );
            click(app, 1285.0, 510.0);
        }
        22 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.players[1].units[0].damage, 0);
            assert_eq!(app.game.players[1].units[1].damage, 1);
            return true;
        }
        _ => panic!("Alpha smoke exceeded its defined frames"),
    }
    false
}
