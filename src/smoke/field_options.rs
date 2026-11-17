//! Native controls for open-field units, a gear exhaust trigger, and target costs.
use super::*;
use local_rift::battlefields::SANDSWEPT_TOMB_ID;

const FRIEND: u32 = 47201;
const ENEMY: u32 = 47202;
const DRAGON: u32 = 47203;

fn setup(app: &mut App, card: usize) {
    expansion_fixture(app, card);
    app.game = Game::new_for_agents(0, 0, 472472, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![SANDSWEPT_TOMB_ID, 3];
    app.game.battlefield_owner = vec![None, Some(1)];
    for p in &mut app.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.set_rune_setup([0, 3, 0, 0, 3, 0], [0; 6]).unwrap();
        p.rune_deck = vec![Domain::Calm; 6];
        p.energy_pool = 6;
        p.energy = 12;
        p.power_pool = [5; 6];
        p.any_power = 0;
    }
    for legend in &mut app.game.legend {
        legend.ready = false;
    }
    app.game.players[0].hand = vec![card];
    app.game.players[0].units = vec![public_unit(FRIEND, 193, None)];
    app.game.players[1].units = vec![
        public_unit(ENEMY, 193, Some(1)),
        public_unit(DRAGON, 471, None),
    ];
    app.player_deck = 0;
    app.opponent_deck = 0;
    app.fields = [SANDSWEPT_TOMB_ID, 3];
    app.clear_game_controls();
    app.inspect = None;
    app.field_inspect = None;
    app.lesson = None;
    app.prefs.coach = true;
    app.prefs.exact_payment = false;
    app.companion.set_level(AdviceLevel::Beginner);
    app.ai_worker.invalidate();
}

fn advice(app: &mut App, name: &str) {
    app.companion.observe(&app.game);
    for _ in 0..100 {
        if app.companion.message().contains(name) {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("missing {name} advice");
}

/// Supply only unopposed responses and unrelated optional Irelia choices.
/// All new-card targets are selected through the visible controls below.
fn advance(app: &mut App, target_card: Option<usize>) {
    for _ in 0..80 {
        if matches!(app.game.decision, Some(Decision::TriggerTargets { card_id, .. })
            if target_card == Some(card_id))
        {
            return;
        }
        if app.game.decision.is_none() && app.game.chain.is_empty() && app.game.showdown.is_none() {
            assert!(target_card.is_none(), "missing target choice");
            return;
        }
        let action = match &app.game.decision {
            Some(Decision::OrderTriggers { options, .. }) => Action::OrderTrigger {
                trigger_id: options[0].id,
            },
            Some(Decision::Legend { .. }) => Action::Choose { accept: false },
            None => Action::Pass,
            other => panic!("unexpected field-options choice: {other:?}"),
        };
        app.game.apply(action).unwrap();
    }
    panic!("field-options chain did not finish");
}

fn chosen_cost(app: &App) -> local_rift::play::CostPlanView {
    let action = app.canonical_action(app.selected_play_action().unwrap());
    app.game.cost_plan(&action).unwrap()
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100., -100.), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.);
    match frame {
        0 => {
            setup(app, 470);
            // A gear-only battlefield is still open for a unit to enter.
            app.game.players[1].gear.push(GearState {
                id: 47204,
                card_id: 401,
                lane: Some(0),
                attached_to: None,
                ready: true,
                temporary: false,
                token: false,
            });
        }
        1 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        2 => {
            assert_eq!(app.inspect, Some(470));
            capture(app, "RIFT_FIELD_OPTIONS_DECKHAND_INSPECT");
            app.inspect = None;
            click(app, 315., 760.);
        }
        3 => {
            advice(app, "Deckhand");
            capture(app, "RIFT_FIELD_OPTIONS_DECKHAND_BEGINNER");
            // Enemy-controlled field is disabled; the hand and costs stay put.
            click(app, 1340., 548.);
        }
        4 => {
            assert_eq!(app.game.players[0].hand, vec![470]);
            assert_eq!(app.game.players[0].energy, 12);
            click(app, 150., 600.);
        }
        5 => {
            advice(app, "Deckhand");
            capture(app, "RIFT_FIELD_OPTIONS_DECKHAND_ADVANCED");
            click(app, 1220., 548.);
        }
        6 => {
            advance(app, None);
            let unit = app.game.players[0]
                .units
                .iter()
                .find(|u| u.card_id == 470)
                .unwrap();
            assert_eq!((unit.lane, unit.ready), (Some(0), false));
            assert_eq!(app.game.players[0].energy, 9);
            capture_unit(app, unit.id, "RIFT_FIELD_OPTIONS_DECKHAND_PLAYED");
        }
        7 => setup(app, 471),
        8 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        9 => {
            assert_eq!(app.inspect, Some(471));
            capture(app, "RIFT_FIELD_OPTIONS_DRAKE_INSPECT");
            app.inspect = None;
            click(app, 315., 760.);
        }
        10 => {
            advice(app, "Ocean Drake");
            capture(app, "RIFT_FIELD_OPTIONS_DRAKE_BEGINNER");
            click(app, 150., 600.);
        }
        11 => {
            advice(app, "Ocean Drake");
            capture(app, "RIFT_FIELD_OPTIONS_DRAKE_ADVANCED");
            click(app, 1220., 548.);
        }
        12 => {
            advance(app, Some(471));
            capture(app, "RIFT_FIELD_OPTIONS_DRAKE_TARGETS");
            click_unit(app, DRAGON);
        }
        13 => {
            assert!(app.targets.is_empty(), "a Dragon must not be selectable");
            click_unit(app, ENEMY);
        }
        14 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: ENEMY
                }]
            );
            capture(app, "RIFT_FIELD_OPTIONS_DRAKE_CHOSEN");
            click(app, 1280., 510.);
        }
        15 => {
            advance(app, None);
            assert_eq!(app.game.players[1].hand, vec![193]);
            assert!(app.game.players[1].units.iter().any(|u| u.id == DRAGON));
            capture(app, "RIFT_FIELD_OPTIONS_DRAKE_RETURNED");
        }
        16 => setup(app, 471),
        17 => click(app, 315., 760.),
        18 => click(app, 1280., 510.),
        19 => {
            advance(app, Some(471));
            capture(app, "RIFT_FIELD_OPTIONS_DRAKE_OPTIONAL");
            click(app, 1280., 548.);
        }
        20 => {
            advance(app, None);
            assert!(app.game.players[1].hand.is_empty());
            assert_eq!(app.game.players[1].units.len(), 2);
        }
        21 => setup(app, 472),
        22 => app.ui.input_override = Some((vec2(315., 760.), false, true)),
        23 => {
            assert_eq!(app.inspect, Some(472));
            capture(app, "RIFT_FIELD_OPTIONS_ROSTRUM_INSPECT");
            app.inspect = None;
            click(app, 315., 760.);
        }
        24 => {
            advice(app, "Rostrum");
            capture(app, "RIFT_FIELD_OPTIONS_ROSTRUM_BEGINNER");
            click(app, 150., 600.);
        }
        25 => {
            advice(app, "Rostrum");
            capture(app, "RIFT_FIELD_OPTIONS_ROSTRUM_ADVANCED");
            click(app, 1280., 510.);
        }
        26 => {
            advance(app, None);
            assert!(app.game.players[0].gear[0].ready);
            click(app, 400., 663.); // Loadout.
        }
        27 => click(app, 615., 760.),
        28 => {
            assert!(matches!(app.selection, Some(Selection::Gear(_))));
            click(app, 1280., 417.);
        }
        29 => {
            capture(app, "RIFT_FIELD_OPTIONS_ROSTRUM_ACTIVATE");
            click(app, 1280., 510.);
        }
        30 => {
            assert!(!app.game.players[0].gear[0].ready);
            assert!(app.game.players[0].hand.is_empty());
            assert_eq!(app.game.players[0].total_runes(), 6);
            advance(app, Some(472));
            capture(app, "RIFT_FIELD_OPTIONS_ROSTRUM_TARGET");
            click_unit(app, ENEMY);
        }
        31 => {
            assert!(
                app.targets.is_empty(),
                "Rostrum only boosts a friendly unit"
            );
            click_unit(app, FRIEND);
        }
        32 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: FRIEND
                }]
            );
            click(app, 1280., 510.);
        }
        33 => {
            advance(app, None);
            assert_eq!(app.game.players[0].hand, vec![71]);
            assert_eq!(app.game.players[0].total_runes(), 7);
            // Paying the gear's 8 Energy exhausted two Calm runes earlier.
            // The newly channeled rune is exhausted too, leaving one ready.
            assert_eq!(app.game.players[0].ready_runes(Domain::Calm), 1);
            assert_eq!(
                app.game.players[0].exhausted_runes[domain_index(Domain::Calm)],
                3
            );
            assert!(!app.game.players[0].gear[0].ready);
            assert_eq!(app.game.players[0].units[0].temporary_buff, 3);
            capture_unit(app, FRIEND, "RIFT_FIELD_OPTIONS_ROSTRUM_RESOLVED");
        }
        34 => {
            setup(app, 28); // Punch First: 1 Energy, 2 Body Power.
            app.game.players[0].units[0].lane = Some(0);
            app.game.battlefield_owner[0] = Some(0);
            app.game.players[0].power_pool = [0, 0, 0, 1, 0, 0];
            app.field_inspect = Some(SANDSWEPT_TOMB_ID);
        }
        35 => {
            capture(app, "RIFT_FIELD_OPTIONS_TOMB_INSPECT");
            click(app, 650., 505.);
        }
        36 => click(app, 315., 760.),
        37 => {
            click_unit(app, ENEMY);
        }
        38 => {
            let plan = chosen_cost(app);
            assert_eq!(
                plan.power.iter().map(|p| u32::from(p.amount)).sum::<u32>(),
                2
            );
            capture(app, "RIFT_FIELD_OPTIONS_TOMB_ENEMY_COST");
            click_unit(app, ENEMY);
        }
        39 => click_unit(app, FRIEND),
        40 => {
            let plan = chosen_cost(app);
            assert_eq!(plan.energy, 1);
            assert_eq!(
                plan.power.iter().map(|p| u32::from(p.amount)).sum::<u32>(),
                1
            );
            advice(app, "Sandswept");
            capture(app, "RIFT_FIELD_OPTIONS_TOMB_DISCOUNT");
            click(app, 1280., 467.); // Open play choices before reviewing costs.
        }
        41 => {
            assert!(app.play_draft.as_ref().unwrap().open);
            click(app, 1040., 700.);
        }
        42 => {
            assert!(app.cost_editor.is_some());
            capture(app, "RIFT_FIELD_OPTIONS_TOMB_COST_REVIEW");
            click(app, 900., 750.);
        }
        43 => click(app, 1280., 510.),
        44 => {
            advance(app, None);
            assert_eq!(app.game.players[0].energy, 11);
            assert_eq!(app.game.players[0].power_pool, [0; 6]);
            assert_eq!(
                app.game.unit_might(&app.game.players[0].units[0]).unwrap(),
                7
            );
            capture_unit(app, FRIEND, "RIFT_FIELD_OPTIONS_TOMB_RESOLVED");
        }
        45 => click(app, 150., 600.),
        46 => {
            advice(app, "Sandswept");
            capture(app, "RIFT_FIELD_OPTIONS_TOMB_ADVANCED");
            setup(app, 470);
        }
        47 => {
            click(app, 315., 760.);
            app.ui.pointer_override = Some((true, false));
        }
        48 | 49 => {
            click_field(app, 0);
            let pointer = app.ui.input_override.unwrap().0;
            app.ui.input_override = Some((pointer, false, false));
            app.ui.pointer_override = Some((frame == 48, frame == 49));
        }
        50 => {
            advance(app, None);
            assert!(app.game.players[0].hand.is_empty());
            let unit = app.game.players[0]
                .units
                .iter()
                .find(|u| u.card_id == 470)
                .unwrap();
            assert_eq!(unit.lane, Some(0));
            capture_unit(app, unit.id, "RIFT_FIELD_OPTIONS_OPEN_FIELD_DRAG");
        }
        51 => return true,
        _ => panic!("field-options smoke exceeded script"),
    }
    false
}
