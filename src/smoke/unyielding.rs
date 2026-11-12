//! Controlled public position; actual card and turn-effect controls are clicked.
use super::*;

fn setup(app: &mut App) {
    expansion_fixture(app, 441);
    app.game = Game::new_for_agents(3, 3, 441441, Difficulty::Easy);
    app.game.choose_starting_player(1).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![2, 2];
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
    app.game.players[0].hand = vec![441];
    app.game.players[0]
        .units
        .push(public_unit(44100, 69, Some(0)));
    app.game.players[1].hand = vec![67];
    app.game
        .apply(Action::PlayCardWithTargets {
            hand_index: 0,
            lane: None,
            targets: vec![Target::Unit {
                player: 0,
                unit_id: 44100,
            }],
        })
        .unwrap();
    app.game.apply(Action::Pass).unwrap();
    app.fields = [2, 2];
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
fn spirit_tip(app: &mut App, level: AdviceLevel) {
    app.companion.set_level(level);
    app.companion.observe(&app.game);
    for _ in 0..80 {
        if app.companion.message().contains("Unyielding Spirit") {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("Unyielding advice must be available during response and active prevention");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    match frame {
        0 => setup(app),
        1 => {
            spirit_tip(app, AdviceLevel::Beginner);
            capture(app, "RIFT_UNYIELDING_RESPONSE");
            click(app, 315.0, 760.0);
        }
        2 => {
            assert!(app.selection == Some(Selection::Hand(0)));
            capture(app, "RIFT_UNYIELDING_SELECTED");
            click(app, 1280.0, 510.0);
        }
        3 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 441);
            assert!(app.game.chain.last().unwrap().targets.is_empty());
            assert!(!app.game.spell_ability_damage_prevented);
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
        }
        4 => {
            assert!(app.game.spell_ability_damage_prevented);
            spirit_tip(app, AdviceLevel::Beginner);
            capture(app, "RIFT_UNYIELDING_ACTIVE");
            click(app, 1100.0, 48.0);
        }
        5 => {
            assert!(app.show_play_effects);
            capture(app, "RIFT_UNYIELDING_EFFECTS");
            click(app, 700.0, 718.0);
        }
        6 => {
            assert!(!app.show_play_effects);
            app.game.apply(Action::Pass).unwrap();
            app.game.apply(Action::Pass).unwrap();
            assert_eq!(app.game.players[0].units[0].damage, 0);
            assert!(app.game.players[1].discard.contains(&67));
        }
        7 => {
            spirit_tip(app, AdviceLevel::Advanced);
            capture(app, "RIFT_UNYIELDING_ADVANCED");
            app.inspect = Some(441);
        }
        8 => {
            capture(app, "RIFT_UNYIELDING_INSPECT");
            app.inspect = None;
            // Finish the survivor's independent Minefield conquest ability.
            for _ in 0..20 {
                if app.game.chain.is_empty() {
                    break;
                }
                assert!(app.game.decision.is_none());
                app.game.apply(Action::Pass).unwrap();
            }
            assert!(app.game.chain.is_empty());
            app.game.apply(Action::EndTurn).unwrap();
        }
        9 => {
            assert!(!app.game.spell_ability_damage_prevented);
            capture(app, "RIFT_UNYIELDING_EXPIRED");
        }
        10 => return true,
        _ => panic!("Unyielding smoke exceeded its defined frames"),
    }
    false
}
