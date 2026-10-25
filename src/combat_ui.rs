//! Public combat progress shared by the board and nonblocking companion.
use local_rift::game::{CombatResolution, CombatResult, CombatStep, Game};

pub fn stage_label(step: CombatStep) -> &'static str {
    match step {
        CombatStep::Cleanup => "COMBAT CLEANUP",
        CombatStep::DetermineResult => "COMBAT RESULT PENDING",
        CombatStep::EstablishControl => "ESTABLISHING CONTROL",
        CombatStep::End => "FINISHING COMBAT",
    }
}

pub fn stage_description(status: CombatResolution) -> &'static str {
    match status.step {
        CombatStep::Cleanup => {
            "Finish death replacements and cleanup before determining the result. Combat bonuses still apply."
        }
        CombatStep::DetermineResult => {
            "Resolve damage and cleanup abilities before determining the result. Combat bonuses still apply."
        }
        CombatStep::EstablishControl => {
            "The result is recorded. Resolve its abilities before establishing control."
        }
        CombatStep::End => {
            "Finish Conquer abilities and responses before combat bonuses expire. The combat result is already recorded."
        }
    }
}

pub fn result_label(status: CombatResolution) -> &'static str {
    match status.result {
        None => "Result pending",
        Some(CombatResult::Winner { player: 0 }) => "Result: you won combat",
        Some(CombatResult::Winner { .. }) => "Result: opponent won combat",
        Some(CombatResult::NoResult) => "Result: no winner",
    }
}

/// A completion checkpoint has no new focus action. It advances automatically
/// when there is no pending decision or chain to finish.
pub fn automatic_checkpoint(game: &Game) -> bool {
    game.combat_resolution().is_some() && game.chain.is_empty() && game.decision.is_none()
}

pub fn waiting_copy(game: &Game) -> Option<(&'static str, &'static str)> {
    let status = game.combat_resolution()?;
    if !game.chain.is_empty() {
        return None;
    }
    Some((
        "Combat is resolving.",
        if game.decision.is_some() {
            "Finish the pending choice on the right. Combat continues after its effects resolve."
        } else {
            stage_description(status)
        },
    ))
}

pub fn advice(status: CombatResolution, beginner: bool) -> &'static str {
    if beginner {
        stage_description(status)
    } else {
        match status.step {
            CombatStep::Cleanup | CombatStep::DetermineResult => {
                "Damage and cleanup effects can change the survivors before the combat result is recorded. Check those effects before planning a conquest."
            }
            CombatStep::EstablishControl => {
                "The combat result is fixed. Control is established after result effects finish; a later board change does not rewrite that result."
            }
            CombatStep::End => {
                "Conquer responses still happen during combat. Assault and Shield remain active until these effects finish; check what survives when those bonuses expire."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use local_rift::game::{Action, Decision};

    #[test]
    fn deathknell_choice_is_closed_combat_and_result_is_not_declared_early() {
        for attacker in 0..2 {
            let mut game = crate::smoke::c23_deathknell_fixture(attacker, false);
            assert!(matches!(
                game.decision,
                Some(Decision::TriggerTargets { .. })
            ));
            let status = game.combat_resolution().unwrap();
            assert_eq!(result_label(status), "Result pending");
            assert_eq!(waiting_copy(&game).unwrap().0, "Combat is resolving.");
            assert!(!game.showdown_focus_open());
            assert!(!automatic_checkpoint(&game));
            game.apply(Action::ChooseTriggerTargets { targets: vec![] })
                .unwrap();
            assert!(waiting_copy(&game).is_none());
            assert!(game.combat_resolution().is_none());
        }
    }

    #[test]
    fn conquer_chain_keeps_captured_result_and_combat_bonuses() {
        for attacker in 0..2 {
            let mut game = crate::smoke::c23_deathknell_fixture(attacker, true);
            game.apply(Action::ChooseTriggerTargets { targets: vec![] })
                .unwrap();
            let status = game.combat_resolution().unwrap();
            assert_eq!(status.step, CombatStep::End);
            assert_eq!(
                status.result,
                Some(CombatResult::Winner { player: attacker })
            );
            assert!(result_label(status).contains("won combat"));
            assert!(stage_description(status).contains("already recorded"));
            assert!(game.showdown.is_some());
            assert!(!game.showdown_focus_open());
            assert!(!game.chain.is_empty());
            assert!(waiting_copy(&game).is_none());
            assert_eq!(
                game.unit_might(&game.players[attacker].units[0]).unwrap(),
                4
            );
        }
    }

    #[test]
    fn every_stage_has_short_advice_and_never_offers_an_open_showdown() {
        for step in [
            CombatStep::Cleanup,
            CombatStep::DetermineResult,
            CombatStep::EstablishControl,
            CombatStep::End,
        ] {
            let status = CombatResolution {
                lane: 0,
                attacker: 0,
                step,
                attackers_recalled: false,
                result: None,
            };
            for beginner in [true, false] {
                let text = advice(status, beginner);
                assert!(text.chars().count() <= 180);
                assert!(!text.contains("open showdown"));
            }
        }
    }
}
