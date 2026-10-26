//! Presentation of independent battlefield gear and pending trigger base costs.
use super::*;

pub(super) fn is_deploy(card_id: usize) -> bool {
    local_rift::rules::static_abilities(card_id).contains(&local_rift::rules::StaticAbility::Deploy)
}

pub(super) fn trigger_payment_source(game: &Game, decision: &Decision) -> Option<String> {
    let Decision::TriggerPayment {
        card_id, context, ..
    } = decision
    else {
        return None;
    };
    let source = context.source;
    let object = if let Some(id) = source.gear_id {
        format!(
            " · gear #{id} · {}",
            gear_location_label(game, source.player, id)
        )
    } else if let Some(id) = source.unit_id {
        format!(" · unit #{id}")
    } else {
        String::new()
    };
    Some(format!("{}{object}", game.catalog[*card_id].name))
}

impl App {
    pub(super) fn trigger_payment_panel(&mut self) -> Result<(), String> {
        let Some(decision @ Decision::TriggerPayment { .. }) = self.game.trigger_payment_context()
        else {
            return Ok(());
        };
        let Decision::TriggerPayment {
            choice_id,
            ref targets,
            ref base_cost,
            deflect,
            ..
        } = decision
        else {
            unreachable!()
        };
        self.ui
            .title("Pay for this trigger?", 1178.0, 366.0, 20.0, TEXT);
        self.ui.wrapped_lines(
            &trigger_payment_source(&self.game, &decision).unwrap(),
            1178.0,
            391.0,
            212.0,
            10.5,
            GOLD,
            2,
        );
        let cost = format!(
            "{}{}",
            play_ui::value_label(base_cost),
            if deflect > 0 {
                format!(" + {deflect} Any power (Deflect)")
            } else {
                String::new()
            }
        );
        self.ui
            .wrapped_lines(&cost, 1178.0, 425.0, 212.0, 11.0, TEXT, 2);
        self.ui.wrapped_lines(
            &if targets.is_empty() {
                "No chosen targets.".into()
            } else {
                targets
                    .iter()
                    .map(|target| target_label(&self.game, *target))
                    .collect::<Vec<_>>()
                    .join(" · ")
            },
            1178.0,
            460.0,
            212.0,
            10.5,
            TEAL,
            2,
        );
        let action = Action::ResolveTriggerPayment {
            choice_id,
            pay: true,
        };
        let payable = self.game.clone().apply(action.clone()).is_ok();
        let can_begin = !payable && self.can_begin_payment(&action);
        if self.ui.button(
            Rect::new(1178.0, 493.0, 212.0, 34.0),
            if can_begin {
                "Begin payment · use resources"
            } else {
                "Pay · finalize ability"
            },
            true,
            payable || can_begin,
        ) {
            if can_begin {
                self.begin_ui_payment(action);
            } else {
                self.action(action);
            }
        }
        if self.ui.button(
            Rect::new(1178.0, 534.0, 212.0, 27.0),
            "Decline this ability",
            false,
            true,
        ) {
            self.action(Action::ResolveTriggerPayment {
                choice_id,
                pay: false,
            });
        }
        self.ui.wrapped_lines("Paid before responses. A later counter gives no refund. Use Add abilities in Board / Loadout before finishing.",
            1178.0, 579.0, 212.0, 10.0, MUTED, 4);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use local_rift::play::PlayChoices;

    fn position() -> Game {
        let mut game = Game::new(3, 3, 2401, Difficulty::Easy);
        game.start_match().unwrap();
        game.players[0].units.clear();
        game.players[1].units.clear();
        game.players[0].gear.clear();
        game.players[1].gear.clear();
        game.players[0].hand = vec![401, 410];
        game.players[0].runes = [0; 6];
        game.players[0].exhausted_runes = [0; 6];
        game.players[0].energy_pool = 20;
        game.players[0].energy = 20;
        game.players[0].power_pool = [20; 6];
        game.battlefield_owner = [None; 2];
        game
    }

    #[test]
    fn deploy_presentation_uses_the_characteristic_not_words_in_rules_text() {
        assert!((401..=408).all(is_deploy));
        assert!(
            !is_deploy(409),
            "Traveler only refers to friendly Deploy gear"
        );
        assert!(!is_deploy(410), "Command only refers to a Deploy target");
    }

    #[test]
    fn deploy_preview_and_scene_follow_the_physical_copy_without_claiming_control() {
        let mut game = position();
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: Some(0),
            target: None,
        })
        .unwrap();
        let id = game.players[0].gear[0].id;
        let text = live_gear_description(&game, 0, id).unwrap();
        assert!(text.contains("Independent gear at battlefield 1"));
        assert!(text.contains("Gear does not control or contest"));
        assert!(text.contains("conquest does not trigger"));
        assert_eq!(game.battlefield_owner, [None; 2]);
        let action = Action::WithPlayChoices {
            action: Box::new(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![
                    Target::Gear {
                        player: 0,
                        gear_id: id,
                    },
                    Target::Battlefield { lane: 1 },
                ],
            }),
            choices: PlayChoices {
                mode: Some(0),
                ..Default::default()
            },
        };
        game.apply(action).unwrap();
        game.apply(Action::Pass).unwrap();
        game.apply(Action::Pass).unwrap();
        assert_eq!(
            scene_gear(&game)
                .iter()
                .map(|g| (g.id, g.owner, g.lane))
                .collect::<Vec<_>>(),
            vec![(id, 0, 1)]
        );
        assert!(
            live_gear_description(&game, 0, id)
                .unwrap()
                .contains("Independent gear at battlefield 2")
        );
        assert_eq!(game.battlefield_owner, [None; 2]);
    }

    #[test]
    fn command_target_hint_tracks_each_canonical_slot_and_mode() {
        let mut game = position();
        game.apply(Action::PlayCard {
            hand_index: 0,
            lane: Some(0),
            target: None,
        })
        .unwrap();
        let id = game.players[0].gear[0].id;
        let mut action = Action::WithPlayChoices {
            action: Box::new(Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            }),
            choices: PlayChoices {
                mode: Some(0),
                ..Default::default()
            },
        };
        let view = game.play_target_view(&action, 0).unwrap();
        assert!(play_ui::target_view_hint(&game, &view).contains("gear on the mat"));
        play_ui::replace_execution_targets(
            &mut action,
            0,
            vec![Target::Gear {
                player: 0,
                gear_id: id,
            }],
        )
        .unwrap();
        let view = game.play_target_view(&action, 0).unwrap();
        assert!(play_ui::target_view_hint(&game, &view).contains("battlefield"));
        assert_eq!(view.candidates, vec![Target::Battlefield { lane: 1 }]);
        if let Action::WithPlayChoices {
            action: inner,
            choices,
        } = &mut action
        {
            choices.mode = Some(1);
            play_ui::replace_execution_targets(inner, 0, vec![]).unwrap();
        }
        assert_eq!(
            play_ui::target_view_hint(&game, &game.play_target_view(&action, 0).unwrap()),
            "This execution needs no targets."
        );
    }
}
