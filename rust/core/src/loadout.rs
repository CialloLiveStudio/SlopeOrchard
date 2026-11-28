use crate::{FruitKind, Hellhound, LevelId, Orchard, Phase, SlotItem, SLOTS};

#[derive(Clone, Copy)]
pub enum PlantSource {
    Slot(usize),
    Seed(FruitKind),
}

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
        if let Some(chime @ SlotItem::Chime { .. }) = self.slots[slot] {
            self.reserve.push(chime);
        }
        self.slots[slot] = Some(kind.into());
        true
    }

    pub fn drop_seed(&mut self, kind: FruitKind, slot: usize) -> bool {
        if self.grafting_unlocked()
            && matches!(self.slots.get(slot), Some(Some(SlotItem::Fruit(_))))
        {
            self.merge(PlantSource::Seed(kind), slot)
        } else {
            self.equip_plant(slot, kind)
        }
    }

    pub fn remove_plant(&mut self, slot: usize) -> bool {
        if !self.can_configure() {
            return false;
        }
        let Some(item) = self.slots.get_mut(slot).and_then(Option::take) else {
            return false;
        };
        if let SlotItem::Chime { .. } = item {
            self.reserve.push(item);
        }
        true
    }

    pub fn split_or_clear(&mut self, slot: usize) -> bool {
        if !self.can_configure() {
            return false;
        }
        let Some(Some(SlotItem::Fruit(fruit))) = self.slots.get(slot) else {
            return self.remove_plant(slot);
        };
        let Some((a, b)) = fruit.kind.parents() else {
            return self.remove_plant(slot);
        };
        self.slots[slot] = Some(a.into());
        if let Some(empty) = self.slots.iter_mut().find(|s| s.is_none()) {
            *empty = Some(b.into());
        }
        // A full rack already has an unlimited supply of the second seed in the tray.
        true
    }

    pub fn preview_graft(&self, source: PlantSource, to: usize) -> Option<FruitKind> {
        if !self.can_configure() || !self.grafting_unlocked() {
            return None;
        }
        let Some(SlotItem::Fruit(target)) = *self.slots.get(to)? else {
            return None;
        };
        let donor = match source {
            PlantSource::Seed(k) if self.level.available_plants.contains(&k) => k,
            PlantSource::Slot(from) if from != to => {
                let item = if from < SLOTS.len() {
                    self.slots[from]?
                } else if self.phase == Phase::Sandbox {
                    *self.reserve.get(from - SLOTS.len())?
                } else {
                    return None;
                };
                let SlotItem::Fruit(f) = item else {
                    return None;
                };
                f.kind
            }
            _ => return None,
        };
        target.kind.combine(donor)
    }

    pub fn merge(&mut self, source: PlantSource, to: usize) -> bool {
        let Some(recipe) = self.preview_graft(source, to) else {
            return false;
        };
        if let PlantSource::Slot(from) = source {
            if from < SLOTS.len() {
                self.slots[from] = None;
            } else {
                self.reserve.remove(from - SLOTS.len());
            }
        }
        self.slots[to] = Some(recipe.into());
        self.vines[to].impulse(2.);
        self.events.push_back(crate::VfxEvent::Synthesis(
            self.slot_snapshot(to).fruit_center,
            recipe,
        ));
        true
    }

    pub fn graft_plants(&mut self, from: usize, to: usize) -> bool {
        self.merge(PlantSource::Slot(from), to)
    }
}
