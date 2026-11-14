//! Controlled positions exercised through real drag, inspect and response controls.
use super::*;

const CARD: usize = 449;
const SOURCE: u32 = 44901;
const FRIENDLY: u32 = 44902;
const ENEMY: u32 = 44903;

fn reset(app: &mut App, ambush: bool) {
    expansion_fixture(app, CARD);
    app.game = Game::new_for_agents(0, 1, 449449, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![3, 3];
    app.game.battlefield_owner = vec![if ambush { Some(0) } else { Some(1) }, None];
    for player in &mut app.game.players {
        player.units.clear();
        player.gear.clear();
        player.hand.clear();
        player.hidden.clear();
        player.discard.clear();
        player.champion = None;
        player.deck = vec![71; 20];
        player.set_rune_setup([0, 3, 0, 0, 3, 0], [0; 6]).unwrap();
        player.rune_deck = vec![
            Domain::Calm,
            Domain::Chaos,
            Domain::Calm,
            Domain::Chaos,
            Domain::Calm,
            Domain::Chaos,
        ];
        player.energy_pool = 6;
        player.energy = 12;
        player.power_pool = [0; 6];
        player.power_pool[domain_index(Domain::Chaos)] = 2;
        player.any_power = 0;
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
    }
    if ambush {
        app.game.players[0].units = vec![public_unit(FRIENDLY, 59, Some(0))];
        app.game.players[0].hand = vec![CARD, 7];
        app.game
            .apply(Action::PlayCardWithTargets {
                hand_index: 1,
                lane: None,
                targets: vec![Target::Unit {
                    player: 0,
                    unit_id: FRIENDLY,
                }],
            })
            .unwrap();
        // Decline Irelia's separate optional trigger so the actual Discipline
        // remains on the chain while the hand card is dragged as a Reaction.
        assert!(matches!(
            app.game.decision,
            Some(Decision::Legend {
                player: 0,
                trigger: local_rift::legends::LegendTrigger::IreliaChoose { .. }
            })
        ));
        app.game.apply(Action::Choose { accept: false }).unwrap();
        assert!(app.game.decision.is_none());
        // Discipline costs two Energy; declining Irelia spends nothing more.
        assert_eq!(app.game.players[0].energy, 10);
    } else {
        app.game.players[0].units = vec![public_unit(SOURCE, CARD, None)];
        app.game.players[1].units = vec![public_unit(ENEMY, 59, Some(0))];
    }
    app.player_deck = 0;
    app.opponent_deck = 1;
    app.fields = [3, 3];
    app.ai_worker.invalidate();
    app.inspect = None;
    app.field_inspect = None;
    app.lesson = None;
    app.rune_menu = None;
    app.chain_cursor = 0;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.companion.set_level(AdviceLevel::Beginner);
}

fn advice(app: &mut App, level: AdviceLevel) {
    app.companion.set_level(level);
    app.companion.observe(&app.game);
    for _ in 0..80 {
        if app.companion.message().contains("Kha'Zix") {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("Kha'Zix advice must be available while held or in play");
}

fn source(app: &App) -> u32 {
    app.game.players[0]
        .units
        .iter()
        .find(|unit| unit.card_id == CARD)
        .unwrap()
        .id
}

fn drag_to(app: &mut App, field: bool, release: bool) {
    assert!(app.drag.is_some(), "the real press must arm a drag");
    let rect = app
        .drop_zones
        .iter()
        .find_map(|(zone, rect)| {
            let matches = if field {
                matches!(zone, crate::DropZone::Battlefield(0))
            } else {
                matches!(zone, crate::DropZone::Base)
            };
            matches.then_some(*rect)
        })
        .unwrap();
    app.ui.input_override = Some((
        vec2(rect.x + rect.w * 0.8, rect.y + rect.h * 0.8),
        false,
        false,
    ));
    app.ui.pointer_override = Some((!release, release));
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => reset(app, true),
        1 | 4 => {
            if frame == 4 {
                assert_eq!(app.game.players[0].hand, vec![CARD]);
                assert_eq!(app.game.players[0].energy, 10);
                assert_eq!(
                    app.game.players[0].power_pool[domain_index(Domain::Chaos)],
                    2
                );
                assert_eq!(app.game.players[0].units.len(), 1);
            }
            advice(app, AdviceLevel::Beginner);
            click(app, 315.0, 760.0);
            app.ui.pointer_override = Some((true, false));
        }
        2 | 3 => drag_to(app, false, frame == 3),
        5 | 6 => drag_to(app, true, frame == 6),
        7 => {
            assert!(app.game.players[0].hand.is_empty());
            let unit = app.game.players[0]
                .units
                .iter()
                .find(|unit| unit.card_id == CARD)
                .unwrap();
            assert_eq!(unit.lane, Some(0));
            assert!(!unit.ready);
            assert_eq!(app.game.players[0].energy, 6);
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Chaos)],
                1
            );
            assert_eq!(app.game.chain.last().unwrap().card_id, 7);
            advice(app, AdviceLevel::Beginner);
            capture(app, "RIFT_KHAZIX_AMBUSH");
        }
        8 => {
            let id = source(app);
            let rect = unit_rect(app, id);
            app.ui.input_override = Some((rect.center(), false, true));
        }
        9 => {
            assert_eq!(app.inspect, Some(CARD));
            capture(app, "RIFT_KHAZIX_INSPECT");
            app.ui.input_override = Some((vec2(950.0, 530.0), false, false));
            app.ui.wheel_override = Some(-100.0);
        }
        10 => {
            assert!(app.inspect_scroll > 0);
            capture(app, "RIFT_KHAZIX_RULES");
            click(app, 940.0, 700.0);
        }
        11 => {
            assert!(app.inspect.is_none());
            reset(app, false);
            click_unit(app, SOURCE);
            app.ui.pointer_override = Some((true, false));
        }
        12 | 13 => drag_to(app, true, frame == 13),
        14 => {
            assert_eq!(app.game.players[0].units[0].lane, Some(0));
            assert_eq!(app.game.chain.last().unwrap().card_id, CARD);
            assert_eq!(app.game.players[0].xp, 0);
            assert_eq!(
                app.game.unit_might(&app.game.players[0].units[0]).unwrap(),
                4
            );
            advice(app, AdviceLevel::Beginner);
            capture(app, "RIFT_KHAZIX_TRIGGER");
            app.action(Action::Pass);
            app.action(Action::Pass);
        }
        15 => {
            assert!(app.game.chain.is_empty());
            assert_eq!(app.game.players[0].xp, 2);
            assert_eq!(
                app.game.unit_might(&app.game.players[0].units[0]).unwrap(),
                6
            );
            advice(app, AdviceLevel::Advanced);
            capture_unit(app, SOURCE, "RIFT_KHAZIX_RESOLVED");
        }
        16 => return true,
        _ => panic!("Kha'Zix smoke exceeded its defined frames"),
    }
    false
}
