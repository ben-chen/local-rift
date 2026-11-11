//! C26 public actions through existing controls in declared controlled positions.
use super::*;
const EZ: u32 = 26003;
const ALLY: u32 = 26001;
const ENEMY: u32 = 26002;

fn position(card: usize) -> Game {
    let mut game = Game::new_for_agents(3, 3, 260026, Difficulty::Easy);
    game.start_match().unwrap();
    game.battlefields = [0, 0];
    game.battlefield_owner = [Some(1), None];
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
    game.players[0].hand = vec![card];
    game
}
fn reset(app: &mut App, card: usize) {
    expansion_fixture(app, card);
    app.game = position(card);
    app.fields = app.game.battlefields;
    app.inspect = None;
    app.field_inspect = None;
    app.field_gear_filter = None;
    app.lesson = None;
    app.rune_menu = None;
    app.chain_cursor = 0;
    app.ui.pointer_override = None;
    app.prefs.coach = true;
    app.prefs.advanced = false;
    app.companion.set_level(AdviceLevel::Beginner);
}
fn pair(game: &mut Game) {
    game.apply(Action::Pass).unwrap();
    game.apply(Action::Pass).unwrap();
}
fn rengar_position() -> Game {
    let mut game = position(411);
    game.players[0].hand.push(9);
    game.players[0].units = vec![public_unit(ALLY, 71, None)];
    game.players[1].units = vec![public_unit(ENEMY, 70, Some(0))];
    game.apply(Action::Move {
        unit_id: ALLY,
        lane: Some(0),
    })
    .unwrap();
    game.apply(Action::PlayCardWithTargets {
        hand_index: 1,
        lane: None,
        targets: vec![Target::Unit {
            player: 0,
            unit_id: ALLY,
        }],
    })
    .unwrap();
    pair(&mut game);
    game.apply(Action::Pass).unwrap();
    assert_eq!(game.priority(), 0);
    assert_eq!(game.lane_count(0, 0), 0);
    assert_eq!(game.battlefield_owner[0], Some(1));
    game
}
fn rengar(app: &mut App) {
    reset(app, 411);
    app.game = rengar_position();
}
fn assert_rengar(app: &App) -> u32 {
    let unit = app.game.players[0]
        .units
        .iter()
        .find(|u| u.card_id == 411)
        .unwrap();
    assert_eq!(unit.lane, Some(0));
    assert!(!unit.ready);
    assert_eq!(app.game.unit_combat_contribution(unit).unwrap().damage, 5);
    unit.id
}
fn ezreal(app: &mut App, defending: bool, combat: bool) {
    reset(app, 412);
    app.game.players[0].hand.clear();
    app.game.players[0].units = vec![public_unit(EZ, 412, defending.then_some(0))];
    app.game.players[1].units = vec![public_unit(ENEMY, 70, (!defending).then_some(0))];
    if combat {
        if defending {
            app.game.battlefield_owner[0] = Some(0);
            app.game.apply(Action::EndTurn).unwrap();
            app.game
                .apply(Action::Move {
                    unit_id: ENEMY,
                    lane: Some(0),
                })
                .unwrap();
        } else {
            app.game.players[0].units.push(public_unit(ALLY, 71, None));
            app.game
                .apply(Action::MoveGroup {
                    unit_ids: vec![EZ, ALLY],
                    lane: Some(0),
                })
                .unwrap();
        }
        assert!(matches!(
            app.game.decision,
            Some(Decision::TriggerTargets { card_id: 412, .. })
        ));
    } else {
        app.game.players[0].units[0].lane = Some(0);
    }
}
fn assert_ezreal(game: &Game) {
    let unit = game.players[0].units.iter().find(|u| u.id == EZ).unwrap();
    let c = game.unit_combat_contribution(unit).unwrap();
    assert_eq!(
        (c.might, c.damage, c.stunned, c.prevents_combat_damage),
        (3, 0, false, true)
    );
    let text = crate::live_unit_description(game, 0, EZ).unwrap();
    assert!(text.contains("3 current might"));
    assert!(text.contains("Combat contribution: 0 damage"));
    assert!(text.contains("Not Stunned"));
}

pub(super) fn run_frame(app: &mut App, frame: u32) -> bool {
    match frame {
        0 => {
            rengar(app);
            click(app, 315.0, 760.0);
        }
        1 => {
            assert!(matches!(app.selection, Some(Selection::Hand(0))));
            capture(app, "RIFT_SMOKE_C26_RENGAR_DESTINATION_SCREENSHOT");
            assert!(
                app.drop_action(
                    &crate::DragSource::Hand {
                        index: 0,
                        card_id: 411
                    },
                    crate::DropZone::Battlefield(1)
                )
                .is_err()
            );
            save_state(app);
            click(app, 1339.0, 546.0);
        }
        2 => {
            assert_same_state(app);
            click(app, 1229.0, 546.0);
        }
        3 => {
            let id = assert_rengar(app);
            capture_unit(app, id, "RIFT_SMOKE_C26_RENGAR_JOINED_SCREENSHOT");
            rengar(app);
            click(app, 315.0, 760.0);
            app.ui.pointer_override = Some((true, false));
        }
        4 | 5 => {
            let rect = unit_rect(app, ENEMY);
            app.ui.input_override = Some((rect.center(), false, false));
            app.ui.pointer_override = Some((frame == 4, frame == 5));
        }
        6 => {
            assert_rengar(app);
            app.ui.pointer_override = None;
            capture(app, "RIFT_SMOKE_C26_RENGAR_DRAG_SCREENSHOT");
            ezreal(app, false, false);
        }
        7 => {
            assert_ezreal(&app.game);
            capture_unit(app, EZ, "RIFT_SMOKE_C26_EZREAL_HOVER_SCREENSHOT");
            let rect = unit_rect(app, EZ);
            app.ui.input_override = Some((rect.center(), false, true));
        }
        8 => {
            assert_eq!(app.inspect, Some(412));
            capture(app, "RIFT_SMOKE_C26_EZREAL_INSPECTOR_SCREENSHOT");
            app.inspect = None;
            app.game.players[0].units[0].stunned = true;
        }
        9 => {
            let c = app
                .game
                .unit_combat_contribution(&app.game.players[0].units[0])
                .unwrap();
            assert!(c.stunned && c.prevents_combat_damage);
            capture_unit(app, EZ, "RIFT_SMOKE_C26_EZREAL_STUNNED_SCREENSHOT");
            ezreal(app, false, true);
            click_unit(app, ENEMY);
        }
        10 => {
            assert_eq!(
                app.targets,
                vec![Target::Unit {
                    player: 1,
                    unit_id: ENEMY
                }]
            );
            capture(app, "RIFT_SMOKE_C26_EZREAL_ATTACK_TARGET_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        11 => {
            assert!(app.game.decision.is_none());
            assert_eq!(app.game.chain.last().unwrap().card_id, 412);
            assert_eq!(app.game.players[1].units[0].damage, 0);
            capture(app, "RIFT_SMOKE_C26_EZREAL_ATTACK_RESPONSE_SCREENSHOT");
            pair(&mut app.game);
        }
        12 => {
            assert_ezreal(&app.game);
            assert_eq!(app.game.players[1].units[0].damage, 3);
            assert_eq!(app.game.lane_might(0, 0).unwrap(), 5);
            assert_eq!(app.game.lane_might(1, 0).unwrap(), 10);
            capture(app, "RIFT_SMOKE_C26_COMBAT_PREVIEW_SCREENSHOT");
            pair(&mut app.game);
        }
        13 => {
            assert!(matches!(
                app.game.decision,
                Some(Decision::CombatDamage {
                    player: 0,
                    total: 5,
                    ..
                })
            ));
            capture(app, "RIFT_SMOKE_C26_COMBAT_ASSIGNMENT_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        14 => {
            let Some(Decision::CombatDamage {
                player: 1,
                lane,
                total: 10,
            }) = app.game.decision
            else {
                panic!("opponent assignment: {:?}", app.game.decision)
            };
            let assignments = app.game.automatic_damage_assignment(1, lane, 10).unwrap();
            app.game
                .apply(Action::AssignDamage { assignments })
                .unwrap();
            assert!(app.game.players[0].units.is_empty());
            reset(app, 412);
            app.game.players[0].hand.clear();
            app.game.players[0].units = vec![public_unit(EZ, 412, Some(0))];
            app.game.players[0].units[0].ready = false;
            app.game.battlefield_owner[0] = Some(0);
            click_unit(app, EZ);
        }
        15 => {
            click(app, 1339.0, 546.0);
        }
        16 => {
            assert!(matches!(app.selection, Some(Selection::Ability(_, None))));
            click(app, 1285.0, 417.0);
        }
        17 => {
            assert!(matches!(
                app.selection,
                Some(Selection::Ability(_, Some(0)))
            ));
            capture(app, "RIFT_SMOKE_C26_EZREAL_MOVE_COST_SCREENSHOT");
            click(app, 1285.0, 510.0);
        }
        18 => {
            assert_eq!(app.game.chain.len(), 1);
            assert_eq!(app.game.players[0].units[0].lane, Some(0));
            assert_eq!(
                app.game.players[0].power_pool[domain_index(Domain::Mind)],
                4
            );
            capture(app, "RIFT_SMOKE_C26_EZREAL_MOVE_RESPONSE_SCREENSHOT");
            pair(&mut app.game);
        }
        19 => {
            assert_eq!(app.game.players[0].units[0].lane, None);
            assert!(!app.game.players[0].units[0].ready);
            assert_ezreal(&app.game);
            capture_unit(app, EZ, "RIFT_SMOKE_C26_EZREAL_BASE_SCREENSHOT");
            click_unit(app, EZ);
        }
        20 => {
            click(app, 1339.0, 546.0);
        }
        21 => {
            click(app, 1285.0, 417.0);
        }
        22 => {
            save_state(app);
            click(app, 1285.0, 510.0);
        }
        23 => {
            assert_same_state(app);
            capture(app, "RIFT_SMOKE_C26_EZREAL_INVALID_BASE_MOVE_SCREENSHOT");
            ezreal(app, true, true);
            click_unit(app, ENEMY);
        }
        24 => {
            click(app, 1285.0, 510.0);
        }
        25 => {
            assert_eq!(app.game.chain.last().unwrap().card_id, 412);
            capture(app, "RIFT_SMOKE_C26_EZREAL_DEFEND_RESPONSE_SCREENSHOT");
            pair(&mut app.game);
        }
        26 => {
            assert_eq!(app.game.players[1].units[0].damage, 3);
            assert_ezreal(&app.game);
            app.prefs.advanced = true;
            app.companion.set_level(AdviceLevel::Advanced);
            app.companion.observe(&app.game);
            for _ in 0..40 {
                if app.companion.message().contains("Ezreal") {
                    break;
                }
                app.companion.next_tip(&app.game);
            }
            assert!(app.companion.message().contains("Ezreal"));
            capture_unit(app, EZ, "RIFT_SMOKE_C26_EZREAL_DEFEND_HOVER_SCREENSHOT");
            let rect = unit_rect(app, EZ);
            app.ui.input_override = Some((rect.center(), false, true));
        }
        27 => {
            assert_eq!(app.inspect, Some(412));
            let printed = app.game.catalog[412].official_text;
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
        28 => {
            assert!(app.inspect_scroll > 0);
            app.ui.wheel_override = Some(0.0);
            capture(app, "RIFT_SMOKE_C26_EZREAL_FULL_RULES_SCREENSHOT");
            app.ui.input_override = None;
            app.ui.wheel_override = None;
            println!(
                "PASS: 1190 native UI frames including 29 C26 controls and all previous 1161 controls; focused C26 runs only its appended 29 frames."
            );
            return true;
        }
        _ => panic!("C26 smoke exceeded its 29-frame script"),
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn c26_live_inspection_distinguishes_printed_restriction_from_actual_stun() {
        let mut game = position(412);
        game.players[0].units = vec![public_unit(EZ, 412, Some(0))];
        assert_ezreal(&game);
        let c = game
            .unit_combat_contribution(&game.players[0].units[0])
            .unwrap();
        assert_eq!(crate::combat_ui::unit_badge(c, true), Some("0 COMBAT"));
        game.players[0].units[0].stunned = true;
        let c = game
            .unit_combat_contribution(&game.players[0].units[0])
            .unwrap();
        assert_eq!(
            crate::combat_ui::unit_badge(c, true),
            Some("STUN · 0 COMBAT")
        );
        assert!(
            !crate::live_unit_description(&game, 0, EZ)
                .unwrap()
                .contains("Not Stunned")
        );
    }
    #[test]
    fn c26_destination_and_advice_follow_retained_attacker_permission() {
        let game = rengar_position();
        for lane in [None, Some(0), Some(1)] {
            let legal = game
                .clone()
                .apply(Action::PlayCardWithTargets {
                    hand_index: 0,
                    lane,
                    targets: vec![],
                })
                .is_ok();
            assert_eq!(legal, lane != Some(1));
        }
        for beginner in [true, false] {
            let tips = crate::combat_ui::card_advice(&game, beginner).unwrap();
            assert!(tips.iter().any(|tip| tip.contains("Rengar")));
            assert!(tips.iter().all(|tip| tip.chars().count() <= 180));
        }
    }
}
