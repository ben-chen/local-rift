//! Equip alternatives keep the bearer separate from a unit spent as a cost.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct ExhaustDraft {
    pub gear_id: u32,
    pub unit_id: Option<u32>,
    pub choosing_cost: bool,
}

impl App {
    pub(super) fn selected_equipment_cost(&self) -> Option<ExhaustDraft> {
        self.equipment_cost
            .filter(|draft| self.selection == Some(Selection::Gear(draft.gear_id)))
    }

    fn selected_alternate_equipment(&self) -> Option<&local_rift::gear::GearState> {
        let Some(Selection::Gear(id)) = self.selection else {
            return None;
        };
        self.game.players[0].gear.iter().find(|gear| {
            gear.id == id
                && !gear.token
                && local_rift::rules::equipment(gear.card_id)
                    .is_some_and(|rule| rule.equip_exhaust_unit)
        })
    }

    pub(super) fn equipment_cost_candidate(&self, target: Target) -> bool {
        let choosing_cost = self
            .selected_equipment_cost()
            .is_some_and(|draft| draft.choosing_cost);
        self.selected_alternate_equipment()
            .is_some_and(|gear| gear.attached_to.is_none())
            && matches!(target, Target::Unit { player: 0, unit_id }
                if self.game.players[0].units.iter().any(|u|
                    u.id == unit_id && (!choosing_cost || u.ready)))
    }

    pub(super) fn equipment_cost_click(&mut self, target: Target) -> bool {
        let Some(gear) = self.selected_alternate_equipment() else {
            return false;
        };
        if gear.attached_to.is_some() {
            self.notify("Attached equipment cannot equip again.");
            return true;
        }
        let draft = self.selected_equipment_cost();
        if self.equipment_cost_candidate(target) {
            let Target::Unit { unit_id, .. } = target else {
                unreachable!()
            };
            if let Some(draft) = draft.filter(|draft| draft.choosing_cost) {
                self.equipment_cost = Some(ExhaustDraft {
                    unit_id: (draft.unit_id != Some(unit_id)).then_some(unit_id),
                    ..draft
                });
            } else if self.targets == [target] {
                self.targets.clear();
            } else {
                self.targets = vec![target];
            }
            self.toast_until = 0.0;
        } else {
            self.notify(if draft.is_some_and(|draft| draft.choosing_cost) {
                "Choose a ready friendly unit for the exhaust cost."
            } else {
                "Choose a friendly unit to bear this equipment."
            });
        }
        true
    }

    pub(super) fn alternate_equip_panel(
        &mut self,
        gear: &local_rift::gear::GearState,
    ) -> Result<(), String> {
        let id = gear.id;
        let active = gear.attached_to.is_none();
        let target = self.targets.first().copied();
        let draft = self.selected_equipment_cost();
        self.ui
            .ellipsis(self.game.gear_name(gear), 1178.0, 359.0, 212.0, 16.0, TEXT);
        self.ui.wrapped_lines("Attach to give a unit +3 Might and Ganking. Pay 1 Any Power or exhaust a ready friendly unit.",1178.0,388.0,212.0,11.0,MUTED,3);
        let bearer = target
            .map(|t| target_label(&self.game, t))
            .unwrap_or_else(|| "choose a friendly unit".into());
        if self.ui.button(
            Rect::new(1178.0, 426.0, 212.0, 24.0),
            &format!("Bearer: {bearer}"),
            draft.is_none_or(|d| !d.choosing_cost),
            active,
        ) && let Some(d) = &mut self.equipment_cost
        {
            d.choosing_cost = false;
        }
        if self.ui.button(
            Rect::new(1178.0, 455.0, 102.0, 27.0),
            "Pay Power",
            draft.is_none(),
            active,
        ) {
            self.equipment_cost = None;
        }
        if self.ui.button(
            Rect::new(1288.0, 455.0, 102.0, 27.0),
            "Exhaust unit",
            draft.is_some(),
            active && target.is_some(),
        ) {
            self.equipment_cost = Some(ExhaustDraft {
                gear_id: id,
                unit_id: draft.and_then(|d| d.unit_id),
                choosing_cost: true,
            });
        }
        let draft = self.selected_equipment_cost();
        let action = target.and_then(|target| match draft {
            Some(draft) => draft
                .unit_id
                .map(|exhaust_unit_id| Action::EquipExhaustUnit {
                    gear_id: id,
                    target,
                    exhaust_unit_id,
                }),
            None => Some(Action::Equip {
                gear_id: id,
                target,
            }),
        });
        if let Some(draft) = draft {
            let name = draft
                .unit_id
                .map(|unit_id| target_label(&self.game, Target::Unit { player: 0, unit_id }))
                .unwrap_or_else(|| "choose a ready unit".into());
            if self.ui.button(
                Rect::new(1178.0, 487.0, 212.0, 24.0),
                &format!("Exhaust: {name}"),
                draft.choosing_cost,
                active,
            ) {
                self.equipment_cost = Some(ExhaustDraft {
                    choosing_cost: true,
                    ..draft
                });
            }
        } else {
            let label = action
                .as_ref()
                .and_then(|action| self.game.cost_plan(action).ok())
                .map(|p| play_ui::total_label(&p))
                .unwrap_or_else(|| "Pay 1 Any power".into());
            self.ui.ellipsis(&label, 1178.0, 502.0, 212.0, 10.0, GOLD);
        }
        self.ui.ellipsis(
            if !active {
                "Already attached · Equip is inactive."
            } else if draft.is_some_and(|d| d.choosing_cost) {
                "Click a ready unit; the bearer can pay too."
            } else {
                "Click a friendly unit to choose the bearer."
            },
            1178.0,
            525.0,
            212.0,
            9.0,
            TEAL,
        );
        let legal = active
            && action
                .as_ref()
                .is_some_and(|a| self.game.clone().apply(a.clone()).is_ok());
        if self.ui.button(
            Rect::new(1178.0, 535.0, 212.0, 28.0),
            if draft.is_some() {
                "Attach · exhaust unit"
            } else {
                "Attach · pay Power"
            },
            true,
            legal,
        ) && let Some(action) = action
        {
            self.action(action);
        }
        Ok(())
    }
}
