//! Physical board runes. Aggregate arrays are compatibility mirrors, not identity.
use crate::cards::Domain;
use crate::game::{Game, PlayerState, domain_index};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuneObject {
    pub id: u32,
    pub domain: Domain,
    pub ready: bool,
}

impl PlayerState {
    pub fn rune_objects(&self) -> &[RuneObject] {
        &self.rune_objects
    }

    /// Explicit controlled-position setup. Replaces all board runes with NEW
    /// identities; never used by gameplay, observation or legacy action adapters.
    /// Existing resource pools and the private Rune Deck are left to the caller.
    pub fn set_rune_setup(&mut self, counts: [u8; 6], exhausted: [u8; 6]) -> Result<(), String> {
        if exhausted.iter().zip(counts).any(|(&used, all)| used > all) {
            return Err("Exhausted rune count exceeds its total.".into());
        }
        let total: u32 = counts.iter().map(|&n| u32::from(n)).sum();
        self.next_rune_id
            .checked_add(total)
            .ok_or_else(|| crate::numeric::capacity("rune identity"))?;
        self.rune_objects.clear();
        self.runes = [0; 6];
        self.exhausted_runes = [0; 6];
        for (d, domain) in crate::payment::POWER_DOMAINS.into_iter().enumerate() {
            for n in 0..counts[d] {
                self.add_rune_object(domain, n >= exhausted[d])?;
            }
        }
        Ok(())
    }

    pub(crate) fn validate_rune_objects(&self) -> Result<(), String> {
        let mut totals = [0u16; 6];
        let mut used = [0u16; 6];
        let mut ids = std::collections::HashSet::new();
        for rune in &self.rune_objects {
            let d = domain_index(rune.domain);
            totals[d] += 1;
            used[d] += u16::from(!rune.ready);
            if rune.id == 0 || rune.id >= self.next_rune_id || !ids.insert(rune.id) {
                return Err("Invalid physical rune identity.".into());
            }
        }
        if totals != self.runes.map(u16::from) || used != self.exhausted_runes.map(u16::from) {
            return Err(
                "Rune object/count mismatch; controlled setups must use set_rune_setup.".into(),
            );
        }
        Ok(())
    }

    pub(crate) fn add_rune_object(&mut self, domain: Domain, ready: bool) -> Result<u32, String> {
        let d = domain_index(domain);
        let next = self
            .next_rune_id
            .checked_add(1)
            .ok_or_else(|| crate::numeric::capacity("rune identity"))?;
        let count = self.runes[d]
            .checked_add(1)
            .ok_or_else(|| crate::numeric::capacity("rune count"))?;
        let id = self.next_rune_id;
        self.next_rune_id = next;
        self.runes[d] = count;
        self.exhausted_runes[d] += u8::from(!ready);
        self.rune_objects.push(RuneObject { id, domain, ready });
        Ok(id)
    }

    pub(crate) fn exhaust_rune_objects(&mut self, domain: Domain, count: u8) {
        for rune in self
            .rune_objects
            .iter_mut()
            .filter(|r| r.domain == domain && r.ready)
            .take(usize::from(count))
        {
            rune.ready = false;
        }
        self.exhausted_runes[domain_index(domain)] += count;
    }

    pub(crate) fn remove_rune_object(&mut self, id: u32) -> Result<RuneObject, String> {
        let index = self
            .rune_objects
            .iter()
            .position(|r| r.id == id)
            .ok_or("That rune is no longer on the board.")?;
        let rune = self.rune_objects.remove(index);
        let d = domain_index(rune.domain);
        self.runes[d] -= 1;
        self.exhausted_runes[d] -= u8::from(!rune.ready);
        self.rune_deck.insert(0, rune.domain);
        Ok(rune)
    }

    pub(crate) fn rune_for_domain(&self, domain: Domain, ready: bool) -> Option<u32> {
        self.rune_objects
            .iter()
            .find(|r| r.domain == domain && r.ready == ready)
            .map(|r| r.id)
    }

    pub(crate) fn awaken_rune_objects(&mut self) {
        for rune in &mut self.rune_objects {
            rune.ready = true;
        }
        self.exhausted_runes = [0; 6];
    }

    pub(crate) fn refresh_rune_energy(&mut self) -> Result<(), String> {
        let ready = self
            .runes
            .iter()
            .zip(self.exhausted_runes)
            .map(|(&all, used)| u16::from(all - used))
            .sum::<u16>();
        self.energy = self
            .energy_pool
            .checked_add(ready)
            .ok_or_else(|| crate::numeric::capacity("available Energy"))?;
        Ok(())
    }
}

impl Game {
    pub(crate) fn activate_rune_object(
        &mut self,
        player: usize,
        rune_id: u32,
        recycle: bool,
    ) -> Result<(), String> {
        if self.decision.is_some() && !self.resource_payment_pending() {
            return Err("Finish the pending choice before activating a rune.".into());
        }
        let rune = *self.players[player]
            .rune_objects()
            .iter()
            .find(|r| r.id == rune_id)
            .ok_or("That rune is no longer on the board.")?;
        if !recycle && !rune.ready {
            return Err("That rune is already exhausted.".into());
        }
        let state = &mut self.players[player];
        let d = domain_index(rune.domain);
        if recycle {
            state.remove_rune_object(rune_id)?;
            state.power_pool[d] = state.power_pool[d]
                .checked_add(1)
                .ok_or_else(|| crate::numeric::capacity("rune Power"))?;
        } else {
            state
                .rune_objects
                .iter_mut()
                .find(|r| r.id == rune_id)
                .unwrap()
                .ready = false;
            state.exhausted_runes[d] += 1;
            state.energy_pool = state
                .energy_pool
                .checked_add(1)
                .ok_or_else(|| crate::numeric::capacity("rune Energy"))?;
        }
        state.refresh_rune_energy()?;
        self.push_log(format!(
            "{} {} a {} rune, adding {}.",
            self.players[player].name,
            if recycle { "recycles" } else { "exhausts" },
            rune.domain.name(),
            if recycle { "power" } else { "energy" }
        ));
        Ok(())
    }

    pub(crate) fn ready_rune_object(&mut self, player: usize, rune_id: u32) -> Result<(), String> {
        let Some(state) = self.players.get_mut(player) else {
            return Ok(());
        };
        let Some(rune) = state.rune_objects.iter_mut().find(|r| r.id == rune_id) else {
            return Ok(());
        };
        if !rune.ready {
            rune.ready = true;
            state.exhausted_runes[domain_index(rune.domain)] -= 1;
            state.refresh_rune_energy()?;
        }
        Ok(())
    }
}
