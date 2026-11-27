use crate::{FruitKind, Hellhound, LevelId, Orchard, Phase, SlotItem, SLOTS};

impl Orchard {
    pub fn can_configure(&self) -> bool {
        matches!(self.phase, Phase::Prep | Phase::Sandbox)
    }

    pub fn grafting_unlocked(&self) -> bool {
        matches!(self.level.id, LevelId::T5 | LevelId::Sandbox)
    }

    pub fn start_battle(&mut self) -> bool {
        if self.phase != Phase::Prep {
            return false;
        }
        self.phase = Phase::Battle;
        true
    }

    pub fn retry(&mut self, direct: bool) -> bool {
        if self.phase != Phase::Defeat {
            return false;
        }
        let mut fresh = Self::new(self.level.id, self.seed);
        fresh.slots = self.slots.map(|item| {
            item.map(|mut item| {
                if let SlotItem::Fruit(ref mut fruit) = item {
                    fruit.age = 0;
                }
                item
            })
        });
        fresh.reinforced = self.reinforced;
        fresh.set_hellhound(self.hellhound.is_some());
        fresh.lulucy = self.lulucy;
        if direct {
            fresh.start_battle();
        }
        *self = fresh;
        true
    }

    pub fn set_hellhound(&mut self, enabled: bool) -> bool {
        let Some(config) = self.level.companion else {
            return false;
        };
        if !self.can_configure() {
            return false;
        }
        if enabled {
            self.hellhound
                .get_or_insert_with(|| Hellhound::companion(config.stamina));
        } else {
            self.hellhound = None;
        }
        true
    }

    pub fn set_lulucy(&mut self, enabled: bool) -> bool {
        if !self.can_configure() || !self.grafting_unlocked() {
            return false;
        }
        self.lulucy = enabled;
        true
    }

    pub fn plant_limit(&self) -> usize {
        if self.level.id == LevelId::Sandbox {
            SLOTS.len()
        } else {
            self.level.initial_plants.len()
                + usize::from(self.reinforced || self.phase == Phase::Reinforcement)
        }
    }

    pub fn equip_plant(&mut self, slot: usize, kind: FruitKind) -> bool {
        if slot >= SLOTS.len() || !self.level.available_plants.contains(&kind) {
            return false;
        }
        if self.phase == Phase::Reinforcement {
            if self.slots[slot].is_some() {
                return false;
            }
            self.slots[slot] = Some(kind.into());
            self.reinforced = true;
            self.advance_wave();
            return true;
        }
        if !self.can_configure()
            || (self.slots[slot].is_none()
                && self.slots.iter().flatten().count() >= self.plant_limit())
        {
            return false;
        }
        self.slots[slot] = Some(kind.into());
        true
    }

    pub fn remove_plant(&mut self, slot: usize) -> bool {
        if !self.can_configure() || !matches!(self.slots.get(slot), Some(Some(SlotItem::Fruit(_))))
        {
            return false;
        }
        self.slots[slot] = None;
        true
    }

    pub fn graft_plants(&mut self, from: usize, to: usize) -> bool {
        if !self.can_configure() || !self.grafting_unlocked() || from == to || to >= SLOTS.len() {
            return false;
        }
        let donor = if from < SLOTS.len() {
            self.slots[from]
        } else if self.phase == Phase::Sandbox {
            self.reserve.get(from - SLOTS.len()).copied()
        } else {
            None
        };
        let Some(SlotItem::Fruit(donor)) = donor else {
            return false;
        };
        let Some(SlotItem::Fruit(base)) = &mut self.slots[to] else {
            return false;
        };
        if donor.graft.is_some() || base.graft.is_some() {
            return false;
        }
        base.graft = Some(donor.kind);
        base.age = 0;
        if from < SLOTS.len() {
            self.slots[from] = None;
        } else {
            self.reserve.remove(from - SLOTS.len());
        }
        true
    }
}
