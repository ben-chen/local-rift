//! Small public-board tactics for the two heuristic opponent levels.
//! These prefer an immediately available final conquest. They do not search
//! hidden draws or predict an opponent's discretionary responses.
use super::*;

impl Game {
    pub(super) fn irelia_winning_route(&self) -> Result<Option<Action>, String> {
        let player = self.priority();
        if self.legend[player].kind != crate::legends::LegendKind::Irelia
            || self.active != player
            || self.in_window()
            || self.decision.is_some()
            || self.players[player].score < VICTORY_SCORE - 1
            || self.players[player].score < self.players[1 - player].score
        {
            return Ok(None);
        }
        let lanes: Vec<_> = (0..self.battlefield_count())
            .filter(|&lane| {
                !self.player_scored_this_turn[player][lane]
                    && self.battlefield_owner[lane] != Some(player)
                    && self.lane_count(1 - player, lane) == 0
                    && (0..self.battlefield_count())
                        .filter(|&other| other != lane)
                        .all(|other| self.player_scored_this_turn[player][other])
            })
            .collect();
        let mut runners = Vec::new();
        for unit in &self.players[player].units {
            if self.movement_locked(player, unit.id) {
                continue;
            }
            for &lane in &lanes {
                if !self.standard_move_route_allowed(unit, Some(lane))? {
                    continue;
                }
                if unit.ready {
                    let action = Action::Move {
                        unit_id: unit.id,
                        lane: Some(lane),
                    };
                    if self.clone().apply(action.clone()).is_ok() {
                        return Ok(Some(action));
                    }
                } else {
                    runners.push(unit.id);
                }
            }
        }
        if !self.legend[player].ready || runners.is_empty() {
            return Ok(None);
        }
        // Positive Might spells can choose an exhausted runner without removing
        // it. Keep exact targeting/payment in the engine, including field costs
        // and the separate Power cost for the subsequent Irelia ready.
        let mut spells: Vec<_> = self.players[player].hand.iter().enumerate()
            .filter(|(_, id)| matches!(rules::effect(**id), OfficialEffect::ModifyMight { amount, .. } if amount > 0))
            .map(|(index, id)| (self.catalog[*id].cost, index)).collect();
        spells.sort_unstable();
        let mut attempts = 0;
        for (_, hand_index) in spells {
            for &unit_id in &runners {
                if attempts == 12 {
                    return Ok(None);
                }
                attempts += 1;
                let action = Action::PlayCardWithTargets {
                    hand_index,
                    lane: None,
                    targets: vec![Target::Unit { player, unit_id }],
                };
                let mut probe = self.clone();
                if probe.apply(action.clone()).is_err() {
                    continue;
                }
                // Fervent can also trigger when chosen. Select only legal
                // ordering steps; never resolve the spell or inspect its draw.
                for _ in 0..8 {
                    if !matches!(probe.decision, Some(Decision::OrderTriggers { .. })) {
                        break;
                    }
                    let Some(order) = probe.legal_actions()?.into_iter().next() else {
                        break;
                    };
                    probe.apply(order)?;
                }
                if matches!(probe.decision, Some(Decision::Legend {
                    player: owner, trigger: LegendTrigger::IreliaChoose { unit_id: target },
                }) if owner == player && target == unit_id)
                    && probe.apply(Action::Choose { accept: true }).is_ok()
                {
                    return Ok(Some(action));
                }
            }
        }
        Ok(None)
    }
}
