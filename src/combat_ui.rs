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

/// A restriction on pooled combat damage does not change the unit's Might.
pub fn unit_description(contribution: local_rift::numeric::CombatContribution) -> Vec<String> {
    let mut lines = vec![format!(
        "Combat contribution: {} damage.",
        contribution.damage
    )];
    if contribution.prevents_combat_damage {
        lines.push("NO COMBAT DAMAGE · Printed ability. Current Might still determines lethal damage and the values read by other effects.".into());
        if !contribution.stunned {
            lines.push("Not Stunned: its printed ability prevents combat damage.".into());
        }
    }
    lines
}

pub fn gear_requirement_description(
    game: &Game,
    unit: &local_rift::game::Unit,
) -> Option<&'static str> {
    game.unit_combat_gear_requirement(unit).map(|met| {
        if met {
            "FRIENDLY GEAR HERE · Combat requirement met. Gear attached to any friendly unit here counts, as does unattached gear here. Stun can still prevent combat damage."
        } else {
            "NEEDS FRIENDLY GEAR HERE · Deals no combat damage. Gear at another location or controlled by the opponent does not count. Might and Shield still apply."
        }
    })
}

pub fn unit_badge(
    contribution: local_rift::numeric::CombatContribution,
    ready: bool,
) -> Option<&'static str> {
    if contribution.stunned {
        Some("STUN · 0 COMBAT")
    } else if contribution.prevents_combat_damage {
        Some(if ready { "0 COMBAT" } else { "EXH · 0 COMBAT" })
    } else {
        None
    }
}

/// Live reference values, not a promise of the damage after responses or bonuses.
pub fn source_damage_description(
    game: &Game,
    unit: &local_rift::game::Unit,
) -> Result<Vec<String>, String> {
    use local_rift::trigger_rules::{Effect, SourceDamageRecipients, SourceDamageValue};
    let mut seen = Vec::new();
    let mut lines = Vec::new();
    for rule in local_rift::trigger_rules::trigger_rules(unit.card_id) {
        for effect in rule.effects {
            let Effect::SourceDamage { value, recipients } = effect else {
                continue;
            };
            if seen.contains(&(*value, *recipients)) {
                continue;
            }
            seen.push((*value, *recipients));
            let (label, amount) = match value {
                SourceDamageValue::Might => ("Might", game.unit_might(unit)?),
                SourceDamageValue::Assault => ("Assault", game.unit_keywords(unit)?.assault),
                SourceDamageValue::Shield => ("Shield", game.unit_keywords(unit)?.shield),
            };
            lines.push(format!(
                "ABILITY DAMAGE · Current {label}: {amount}. Read again on resolution, before damage bonuses or prevention."
            ));
            if *value != SourceDamageValue::Might {
                lines.push(format!(
                    "Uses total {label}, not full Might. This value exists even while not {}.",
                    if *value == SourceDamageValue::Assault {
                        "attacking"
                    } else {
                        "defending"
                    }
                ));
            }
            lines.push(match recipients {
                SourceDamageRecipients::ChosenTargets => {
                    "Chosen targets must still satisfy the printed location limits on resolution.".into()
                }
                SourceDamageRecipients::AllEnemiesHere => {
                    "Hits every enemy at this unit's current battlefield. These hits do not choose targets.".into()
                }
            });
        }
    }
    Ok(lines)
}

/// Uses public board facts and the local player's hand, never the opponent's hand.
pub fn card_advice(game: &Game, beginner: bool) -> Result<Vec<&'static str>, String> {
    let mut tips = Vec::new();
    let has_unit = |card| {
        game.players
            .iter()
            .flat_map(|player| &player.units)
            .any(|unit| unit.card_id == card)
    };
    if has_unit(412) {
        tips.push(if beginner {
            "Ezreal's Might still sets his ability damage and the damage needed to kill him. His printed ability adds no combat damage; it does not make him Stunned."
        } else {
            "Ezreal adds no combat damage. His attack or defend ability reads Might on resolution: responses can change its damage or move the target out of his battlefield."
        });
    }
    if has_unit(413) {
        tips.push(if beginner {
            "Yasuo's attack ability reads his current Might when it resolves. It is separate from combat damage; Stunned stops his combat contribution, not this ability."
        } else {
            "Yasuo's damage needs his current Might and a target still at his current battlefield. Responses that change Might or move either unit can change the result."
        });
    }
    if has_unit(414) {
        tips.push(if beginner {
            "Graves deals ability damage to every enemy at his current battlefield. He adds no combat damage, but is not Stunned just because of that printed ability."
        } else {
            "Graves reads current Might and hits all enemies at his current battlefield without choosing targets. Responses can change his Might or move units before resolution."
        });
    }
    if has_unit(415) {
        tips.push(if beginner {
            "Lucian's attack ability uses total Assault, not full Might. Hover over him to see its current value; extra Assault can change both his attack and ability damage."
        } else {
            "Lucian reads total Assault on resolution. Check keyword grants and responses that move him or his chosen enemy; the target must still be at his current battlefield."
        });
    }
    if has_unit(416) {
        tips.push(if beginner {
            "Armored Tusker's move ability is optional. Choose a unit at any battlefield or decline. It reads total Shield on resolution, even while Tusker is not defending."
        } else {
            "Tusker can choose a friendly or enemy battlefield unit after moving. Check current Shield and possible responses before accepting the optional damage."
        });
    }
    if game.players[0].hand.iter().any(|id| {
        local_rift::rules::static_abilities(*id)
            .contains(&local_rift::rules::StaticAbility::MayPlayToAttackedBattlefield)
    }) && game
        .showdown
        .as_ref()
        .is_some_and(|showdown| showdown.combat && showdown.attacker == 0)
    {
        tips.push(if beginner {
            "Rengar can join the battlefield you're attacking, even with no friendly unit left there. Costs, response timing and fixed play destinations still apply."
        } else {
            "Keep power for Rengar's Reaction into the field you're attacking. Assault changes his attacking Might; leaving no other friendly unit there does not remove his permission."
        });
    }
    Ok(tips)
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
