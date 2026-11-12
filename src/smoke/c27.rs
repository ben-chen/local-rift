//! C27 controlled positions followed by public actions and actual UI controls.
use super::*;

const SOURCE: u32 = 27001;
const ENEMY: u32 = 27002;
const SECOND: u32 = 27003;
const FRIENDLY: u32 = 27004;
const BASE: u32 = 27005;

fn position(card: usize) -> Game {
    let mut game = Game::new_for_agents(3, 3, 270027, Difficulty::Easy);
    game.start_match().unwrap();
    game.battlefields = vec![0, 0];
    game.battlefield_owner = vec![Some(1), Some(0)];
    for player in &mut game.players {
        player.units.clear();
        player.gear.clear();
        player.hidden.clear();
        player.hand.clear();
        player.discard.clear();
        player.champion = None;
        player.deck = vec![71; 20];
        {
            let setup_runes = [0; 6];
            let setup_exhausted_runes = [0; 6];
            player
                .set_rune_setup(setup_runes, setup_exhausted_runes)
                .unwrap();
        }
        player.energy_pool = 20;
        player.energy = 20;
        player.power_pool = [5; 6];
    }
    game.players[0].units = vec![
        public_unit(SOURCE, card, None),
        public_unit(FRIENDLY, 70, Some(1)),
    ];
    game.players[1].units = vec![
        public_unit(ENEMY, 70, Some(0)),
        public_unit(SECOND, 70, Some(0)),
        public_unit(BASE, 70, None),
    ];
    game
}
fn reset(app: &mut App, card: usize) {
    expansion_fixture(app, card);
    app.game = position(card);
    app.ai_worker.invalidate();
    app.fields = std::array::from_fn(|i| app.game.battlefields[i]);
    app.inspect = None;
    app.field_inspect = None;
    app.field_gear_filter = None;
    app.lesson = None;
    app.rune_menu = None;
    app.chain_cursor = 0;
    app.ui.pointer_override = None;
    app.ui.wheel_override = None;
    app.prefs.coach = true;
    app.prefs.advanced = false;
    app.companion.set_level(AdviceLevel::Beginner);
}
fn source_target() -> Target {
    Target::Unit {
        player: 0,
        unit_id: SOURCE,
    }
}
fn settle(game: &mut Game) {
    for _ in 0..30 {
        if game.chain.is_empty() {
            return;
        }
        assert!(game.decision.is_none());
        game.apply(Action::Pass).unwrap();
    }
    panic!("C27 chain did not settle");
}
fn equip(game: &mut Game, card: usize) {
    game.players[0].gear.push(GearState {
        id: 27006,
        card_id: card,
        lane: None,
        attached_to: None,
        ready: true,
        temporary: false,
        token: false,
    });
    game.apply(Action::Equip {
        gear_id: 27006,
        target: source_target(),
    })
    .unwrap();
    settle(game);
}
fn respond(game: &mut Game, card: usize) {
    if game.priority() != 0 {
        game.apply(Action::Pass).unwrap();
    }
    game.players[0].hand.push(card);
    let quick_draw = game.catalog[card].kind == CardKind::Gear;
    game.apply(Action::PlayCardWithTargets {
        hand_index: game.players[0].hand.len() - 1,
        lane: None,
        targets: if quick_draw {
            vec![]
        } else {
            vec![source_target()]
        },
    })
    .unwrap();
    if quick_draw {
        game.apply(Action::ChooseTarget {
            target: Some(source_target()),
        })
        .unwrap();
    }
    settle(game);
}
fn source_text(game: &Game) -> String {
    crate::live_unit_description(game, 0, SOURCE).unwrap()
}
fn tip(app: &mut App, name: &str) {
    app.companion.observe(&app.game);
    for _ in 0..50 {
        if app.companion.message().contains(name) {
            return;
        }
        app.companion.next_tip(&app.game);
    }
    panic!("C27 advice missing {name}");
}
fn inspect_source(app: &mut App) {
    let rect = unit_rect(app, SOURCE);
    app.ui.input_override = Some((rect.center(), false, true));
}
fn damage(game: &Game, player: usize, id: u32) -> u64 {
    game.players[player]
        .units
        .iter()
        .find(|unit| unit.id == id)
        .unwrap()
        .damage
}

pub(super) fn run_frame(app: &mut App, frame: u32) -> bool {
    match frame {
        0 => {
            reset(app, 414);
            click_unit(app, SOURCE);
        }
        1 => {
            assert!(matches!(app.selection, Some(Selection::Unit(SOURCE))));
            click(app, 1285.0, 420.0);
        }
        2 => {
            assert!(app.game.decision.is_none());
            let item = app.game.chain.last().unwrap();
            assert_eq!(item.card_id, 414);
            assert!(item.targets.is_empty());
            assert!(source_text(&app.game).contains("4 current might"));
            assert!(source_text(&app.game).contains("Combat contribution: 0 damage"));
            assert!(source_text(&app.game).contains("Not Stunned"));
            tip(app, "Graves");
            assert!(!app.companion.message().contains("Ezreal"));
            capture(app, "RIFT_SMOKE_C27_GRAVES_CHAIN_SCREENSHOT");
            capture_unit(app, SOURCE, "RIFT_SMOKE_C27_GRAVES_HOVER_SCREENSHOT");
            inspect_source(app);
        }
        3 => {
            assert_eq!(app.inspect, Some(414));
            capture(app, "RIFT_SMOKE_C27_GRAVES_RULES_SCREENSHOT");
            click(app, 950.0, 704.0);
        }
        4 => {
            assert!(app.inspect.is_none());
            settle(&mut app.game);
            assert_eq!(damage(&app.game, 1, ENEMY), 4);
            assert_eq!(damage(&app.game, 1, SECOND), 4);
            assert_eq!(damage(&app.game, 0, FRIENDLY), 0);
            capture(app, "RIFT_SMOKE_C27_GRAVES_DAMAGE_SCREENSHOT");
            reset(app, 415);
            equip(&mut app.game, 123);
            assert!(source_text(&app.game).contains("Current Assault: 3"));
            assert!(source_text(&app.game).contains("2 current might"));
            capture_unit(app, SOURCE, "RIFT_SMOKE_C27_LUCIAN_BASE_SCREENSHOT");
        }
        5 => click_unit(app, SOURCE),
        6 => click(app, 1285.0, 420.0),
        7 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerTargets { card_id: 415, .. })
            ));
            capture(app, "RIFT_SMOKE_C27_LUCIAN_TARGET_SCREENSHOT");
            click_unit(app, ENEMY);
        }
        8 => {
            assert_eq!(app.targets.len(), 1);
            click(app, 1285.0, 510.0);
        }
        9 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 415);
            capture(app, "RIFT_SMOKE_C27_LUCIAN_CHAIN_SCREENSHOT");
            respond(&mut app.game, 58);
            assert_eq!(damage(&app.game, 1, ENEMY), 3);
            assert!(source_text(&app.game).contains("7 current might"));
            assert!(source_text(&app.game).contains("Current Assault: 3"));
        }
        10 => {
            tip(app, "Lucian");
            capture_unit(app, SOURCE, "RIFT_SMOKE_C27_LUCIAN_RESOLVED_SCREENSHOT");
            reset(app, 416);
            equip(&mut app.game, 124);
            assert!(source_text(&app.game).contains("Current Shield: 5"));
            assert!(source_text(&app.game).contains("4 current might"));
            capture_unit(app, SOURCE, "RIFT_SMOKE_C27_TUSKER_BASE_SCREENSHOT");
            click_unit(app, SOURCE);
        }
        11 => click(app, 1285.0, 420.0),
        12 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::TriggerTargets { card_id: 416, .. })
            ));
            assert_eq!(app.game.trigger_target_bounds(), Some((1, 0)));
            tip(app, "Tusker");
            capture(app, "RIFT_SMOKE_C27_TUSKER_TARGET_SCREENSHOT");
            save_state(app);
            click_unit(app, BASE);
        }
        13 => {
            assert_same_state(app);
            assert!(app.targets.is_empty());
            assert!(app.target_choice_feedback && app.toast_until > get_time());
            assert!(app.toast.contains("Choose a highlighted target"));
            click(app, 1285.0, 547.0);
        }
        14 => {
            assert!(app.game.decision.is_none() && app.game.chain.is_empty());
            assert_eq!(damage(&app.game, 1, ENEMY), 0);
            assert_eq!(app.toast_until, 0.0);
            assert!(!app.target_choice_feedback);
            capture(app, "RIFT_SMOKE_C27_TUSKER_DECLINED_SCREENSHOT");
            reset(app, 416);
            equip(&mut app.game, 124);
            click_unit(app, SOURCE);
        }
        15 => click(app, 1285.0, 420.0),
        16 => {
            assert!(app.game.decision.is_some());
            click_unit(app, FRIENDLY);
        }
        17 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 0,
                    unit_id: FRIENDLY
                }]
            );
            app.notify("Unrelated presentation notice");
            app.toast_until = f64::INFINITY;
            click(app, 1285.0, 510.0);
        }
        18 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 416);
            assert_eq!(app.toast, "Unrelated presentation notice");
            assert_eq!(app.toast_until, f64::INFINITY);
            // End the assertion-only notice before taking the ordinary chain capture.
            app.toast_until = 0.0;
            capture(app, "RIFT_SMOKE_C27_TUSKER_CHAIN_SCREENSHOT");
            // Controlled cross-domain response: no current Body/Mind legend
            // admits Tusker plus Cloth Armor. This is not a legal deck claim.
            respond(&mut app.game, 120);
            assert_eq!(damage(&app.game, 0, FRIENDLY), 7);
            assert!(source_text(&app.game).contains("Current Shield: 7"));
            assert!(source_text(&app.game).contains("4 current might"));
        }
        19 => {
            app.prefs.advanced = true;
            app.companion.set_level(AdviceLevel::Advanced);
            tip(app, "Tusker");
            capture_unit(app, SOURCE, "RIFT_SMOKE_C27_TUSKER_RESOLVED_SCREENSHOT");
            inspect_source(app);
        }
        20 => {
            assert_eq!(app.inspect, Some(416));
            let printed = app.game.catalog[416].official_text;
            assert_eq!(
                app.ui
                    .text_lines(printed, 394.0, 13.0)
                    .join(" ")
                    .split_whitespace()
                    .collect::<Vec<_>>(),
                printed.split_whitespace().collect::<Vec<_>>()
            );
            app.ui.input_override = Some((vec2(950.0, 530.0), false, false));
            app.ui.wheel_override = Some(-100.0);
        }
        21 => {
            assert!(app.inspect_scroll > 0);
            app.ui.wheel_override = Some(0.0);
            capture(app, "RIFT_SMOKE_C27_TUSKER_RULES_SCREENSHOT");
            reset(app, 413);
            app.game
                .apply(Action::Move {
                    unit_id: SOURCE,
                    lane: Some(0),
                })
                .unwrap();
            click_unit(app, ENEMY);
        }
        22 => {
            assert_eq!(app.targets.len(), 1);
            click(app, 1285.0, 510.0);
        }
        23 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 413);
            settle(&mut app.game);
            assert_eq!(damage(&app.game, 1, ENEMY), 6);
            tip(app, "Yasuo");
            capture_unit(app, SOURCE, "RIFT_SMOKE_C27_YASUO_RESOLVED_SCREENSHOT");
        }
        24 => {
            assert!(source_text(&app.game).contains("Current Might: 6"));
            app.ui.input_override = None;
            app.ui.pointer_override = None;
            app.ui.wheel_override = None;
            println!(
                "PASS: 1215 native UI frames including 25 C27 controls and all previous 1190 controls; focused C27 runs only its appended 25 frames."
            );
            return true;
        }
        _ => panic!("C27 smoke exceeded its 25-frame script"),
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_keyword_reference_changes_without_changing_base_might() {
        for (card, spell, label, value, might) in
            [(415, 293, "Assault", 4, 2), (416, 120, "Shield", 5, 4)]
        {
            let mut game = position(card);
            let before = source_text(&game);
            respond(&mut game, spell);
            let after = source_text(&game);
            assert_ne!(before, after);
            assert!(after.contains(&format!("Current {label}: {value}")));
            assert!(after.contains(&format!("{might} current might")));
            assert!(after.contains("Read again on resolution"));
            assert!(after.contains("not full Might"));
        }
    }
    #[test]
    fn graves_description_and_advice_distinguish_all_enemies_from_chosen_targets() {
        let game = position(414);
        let text = source_text(&game);
        assert!(text.contains("Combat contribution: 0 damage") && text.contains("Not Stunned"));
        assert!(text.contains("every enemy") && text.contains("do not choose targets"));
        assert_eq!(text.matches("Current Might: 4").count(), 1);
        for level in [true, false] {
            let tips = crate::combat_ui::card_advice(&game, level).unwrap();
            assert!(tips.iter().any(|tip| tip.contains("Graves")));
            assert!(
                tips.iter()
                    .all(|tip| !tip.contains("Ezreal") && tip.chars().count() <= 180)
            );
        }
    }
    #[test]
    fn all_new_card_advice_fits_sidebar_at_both_levels() {
        for card in 413..=416 {
            for level in [true, false] {
                let tips = crate::combat_ui::card_advice(&position(card), level).unwrap();
                assert!(!tips.is_empty());
                assert!(tips.iter().all(|tip| tip.chars().count() <= 180));
            }
        }
    }
}
