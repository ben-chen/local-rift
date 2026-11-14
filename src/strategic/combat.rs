//! Bounded alternatives to the UI's deterministic combat allocation.
//! Only public units are ranked. Real action validation and resolution remain
//! authoritative; the cheap removal estimate is only a search ordering policy.
use super::*;

const DAMAGE_BEAM: usize = 24;

struct Victim {
    id: u32,
    lethal: u64,
    first: u8,
    last: u8,
    value: i128,
}

#[derive(Clone)]
struct Allocation {
    assignments: Vec<(u32, u64)>,
    remaining: u64,
    priority: u8,
    value: i128,
}

impl Game {
    pub(super) fn strategic_damage_candidates(&self) -> Result<Vec<Action>, String> {
        let Some(Decision::CombatDamage {
            player,
            lane,
            total,
        }) = self.decision
        else {
            return Ok(Vec::new());
        };
        let victims = self.players[1 - player]
            .units
            .iter()
            .filter(|unit| unit.lane == Some(lane))
            .map(|unit| {
                let keywords = self.unit_keywords(unit)?;
                let mut lasting = unit.clone();
                lasting.temporary_buff = 0;
                Ok(Victim {
                    id: unit.id,
                    lethal: self.unit_might(unit)?.saturating_sub(unit.damage).max(1),
                    first: self.damage_priority(unit)?,
                    // CR467.2.c.8: Tank + Backline can be assigned first OR last.
                    last: if keywords.backline {
                        2
                    } else {
                        self.damage_priority(unit)?
                    },
                    value: 34
                        + i128::from(self.unit_might(&lasting)?) * 14
                        + i128::from(unit.ready) * 12,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mut frontier = vec![Allocation {
            assignments: Vec::new(),
            remaining: total,
            priority: 0,
            value: 0,
        }];
        let mut complete = Vec::new();
        // Expand at most DAMAGE_BEAM prefixes per depth, rather than all n!
        // permutations. Finished allocations compete across different depths,
        // so killing several small units can beat killing one expensive unit.
        for _ in 0..victims.len() {
            let mut next = Vec::new();
            for prefix in frontier {
                let remaining: Vec<_> = victims
                    .iter()
                    .filter(|victim| !prefix.assignments.iter().any(|&(id, _)| id == victim.id))
                    .collect();
                for victim in &remaining {
                    let priority = if victim.first < prefix.priority {
                        victim.last
                    } else {
                        victim.first
                    };
                    if priority < prefix.priority
                        || remaining
                            .iter()
                            .any(|other| other.id != victim.id && other.last < priority)
                    {
                        continue;
                    }
                    let amount = if remaining.len() == 1 {
                        prefix.remaining
                    } else {
                        prefix.remaining.min(victim.lethal)
                    };
                    if amount == 0 {
                        continue;
                    }
                    let mut candidate = prefix.clone();
                    candidate.assignments.push((victim.id, amount));
                    candidate.remaining -= amount;
                    candidate.priority = priority;
                    if amount >= victim.lethal {
                        candidate.value += victim.value;
                    }
                    // Nonlethal damage heals in combat cleanup, so it does not
                    // earn a lasting board-value bonus in this ordering policy.
                    if candidate.remaining == 0 {
                        complete.push(candidate);
                    } else {
                        next.push(candidate);
                    }
                }
            }
            rank(&mut complete);
            rank(&mut next);
            frontier = next;
            if frontier.is_empty() {
                break;
            }
        }
        // Check the real action path, including future assignment restrictions.
        // Keeping the UI's default separately gives root search a legal fallback.
        Ok(complete
            .into_iter()
            .filter_map(|allocation| {
                let action = Action::AssignDamage {
                    assignments: allocation.assignments,
                };
                self.clone().apply(action.clone()).is_ok().then_some(action)
            })
            .collect())
    }
}

fn rank(allocations: &mut Vec<Allocation>) {
    allocations.sort_by(|a, b| {
        b.value
            .cmp(&a.value)
            .then_with(|| a.assignments.cmp(&b.assignments))
    });
    allocations.truncate(DAMAGE_BEAM);
}
