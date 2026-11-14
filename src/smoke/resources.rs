//! Real controls for optional resource payment and physical unit-cost choices.
use super::*;

fn setup(app: &mut App, card: usize) {
    expansion_fixture(app, card);
    app.game = Game::new_for_agents(0, 1, 451451, Difficulty::Easy);
    app.game.choose_starting_player(0).unwrap();
    app.game.start_match().unwrap();
    app.game.battlefields = vec![3, 3];
    app.game.battlefield_owner = vec![None; 2];
    for p in &mut app.game.players {
        p.units.clear();
        p.gear.clear();
        p.hidden.clear();
        p.hand.clear();
        p.discard.clear();
        p.champion = None;
        p.deck = vec![71; 20];
        p.set_rune_setup([0, 3, 0, 0, 3, 0], [0; 6]).unwrap();
        p.rune_deck = vec![Domain::Calm; 3];
        p.energy = 12;
        p.energy_pool = 6;
        p.power_pool = [0; 6];
        p.any_power = 0;
    }
    app.game.players[0].hand = vec![card];
    if card != 451 {
        app.game.players[0].deck.push(450);
        fixture_unit(app, 0, 451001, 450);
        app.game.players[0].units[0].ready = true;
    }
    app.player_deck = 0;
    app.opponent_deck = 1;
    app.fields = [3, 3];
    app.inspect = None;
    app.field_inspect = None;
    app.lesson = None;
    app.rune_menu = None;
    app.chain_cursor = 0;
    app.prefs.coach = true;
    app.ai_worker.invalidate();
    app.companion.set_level(AdviceLevel::Beginner);
}

fn tip(app: &mut App, name: &str, level: AdviceLevel) {
    app.companion.set_level(level);
    app.companion.observe(&app.game);
    for _ in 0..100 {
        if app.companion.message().contains(name) {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("missing {name} advice");
}

pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    // The hand receives input after the sidebar draws. Give the next frame
    // time to construct and display the selected card's cost controls.
    let frame = match frame {
        12 | 21 => return false,
        13..=20 => frame - 1,
        22.. => frame - 2,
        _ => frame,
    };
    match frame {
        0 => setup(app, 451),
        1 => click(app, 315.0, 760.0),
        2 => click(app, 1285.0, 467.0),
        3 => {
            assert!(app.play_draft.as_ref().unwrap().open);
            click(app, 790.0, 850.0);
        }
        4 => {
            assert!(app.play_draft.as_ref().unwrap().choices.bonus);
            assert_eq!(app.game.players[0].hand, vec![451]);
            tip(app, "Keeper", AdviceLevel::Beginner);
            capture(app, "RIFT_KEEPER_COST");
        }
        5 => click(app, 1285.0, 510.0),
        6 => {
            assert_eq!(app.game.players[0].units[0].card_id, 451);
            assert_eq!(app.game.players[0].runes[domain_index(Domain::Calm)], 2);
            assert!(app.game.players[0].hand.is_empty());
            assert_eq!(app.game.chain.len(), 1);
            capture(app, "RIFT_KEEPER_TRIGGER");
            click(app, 1285.0, 595.0);
        }
        7 => {
            assert_eq!(app.game.priority(), 1);
            app.game.apply(Action::Pass).unwrap();
        }
        8 => {
            assert_eq!(app.game.players[0].hand, vec![71]);
            let rect = unit_rect(app, app.game.players[0].units[0].id);
            app.ui.input_override = Some((rect.center(), false, true));
        }
        9 => {
            assert_eq!(app.inspect, Some(451));
            capture(app, "RIFT_KEEPER_INSPECT");
        }
        10 => setup(app, 452),
        11 => click(app, 315.0, 760.0),
        12 => {
            assert!(app.play_draft.as_ref().unwrap().open);
            click(app, 460.0, 768.0);
        }
        13 => {
            let draft = app.play_draft.as_ref().unwrap();
            assert_eq!(draft.choices.exhaust_unit, Some(451001));
            assert!(draft.choices.bonus);
            assert!(app.game.players[0].units[0].ready);
            tip(app, "Meditation", AdviceLevel::Beginner);
            capture(app, "RIFT_MEDITATION_COST");
        }
        14 => click(app, 1285.0, 510.0),
        15 => {
            assert!(!app.game.players[0].units[0].ready);
            assert!(app.game.players[0].hand.is_empty());
            assert_eq!(app.game.chain.len(), 1);
            assert!(
                app.game.decision.is_none(),
                "cost must not offer Irelia's ready ability"
            );
            assert!(app.game.legend[0].ready);
            tip(app, "Meditation", AdviceLevel::Advanced);
            capture(app, "RIFT_MEDITATION_PAID");
            click(app, 1285.0, 595.0);
        }
        16 => {
            assert_eq!(app.game.priority(), 1);
            app.game.apply(Action::Pass).unwrap();
        }
        17 => {
            assert_eq!(app.game.players[0].hand, vec![71, 71]);
            capture(app, "RIFT_MEDITATION_DRAW");
        }
        18 => setup(app, 453),
        19 => click(app, 315.0, 760.0),
        20 => {
            assert!(app.play_draft.as_ref().unwrap().open);
            click(app, 460.0, 768.0);
        }
        21 => {
            assert_eq!(
                app.play_draft.as_ref().unwrap().choices.exhaust_unit,
                Some(451001)
            );
            assert!(!app.play_draft.as_ref().unwrap().choices.bonus);
            tip(app, "Life from Stone", AdviceLevel::Beginner);
            capture(app, "RIFT_LIFE_COST");
        }
        22 => click(app, 1285.0, 510.0),
        23 => {
            assert!(!app.game.players[0].units[0].ready);
            assert!(app.game.players[0].hand.is_empty());
            assert_eq!(app.game.players[0].total_runes(), 6);
            assert_eq!(app.game.chain.len(), 1);
            assert!(app.game.decision.is_none());
            assert!(app.game.legend[0].ready);
            tip(app, "Life from Stone", AdviceLevel::Advanced);
            capture(app, "RIFT_LIFE_PAID");
            click(app, 1285.0, 595.0);
        }
        24 => {
            assert_eq!(app.game.priority(), 1);
            app.game.apply(Action::Pass).unwrap();
        }
        25 => {
            assert_eq!(app.game.players[0].hand, vec![71]);
            assert_eq!(app.game.players[0].total_runes(), 7);
            assert_eq!(
                app.game.players[0].exhausted_runes[domain_index(Domain::Calm)],
                1
            );
            capture(app, "RIFT_LIFE_CHANNEL");
        }
        26 => return true,
        _ => panic!("resource-card scenario exceeded its frames"),
    }
    false
}
