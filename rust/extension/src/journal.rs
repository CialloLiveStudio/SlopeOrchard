use godot::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use slope_orchard_core::FruitKind;
use std::collections::{BTreeMap, BTreeSet};

const STAGES: [&str; 3] = ["t1", "t2", "t5"];
const NOTES: [&str; 16] = [
    "直落砸击",
    "反弹撞击果架",
    "沿坡滚动阻敌",
    "落地留下粘液",
    "一刀落下双果",
    "高弹保留更多动能",
    "重壳多次撞击",
    "铺开双倍粘液",
    "粘住史莱姆滚下坡",
    "爆开扇形苹果雨",
    "震起周围怪物",
    "吞怪弹起再破裂",
    "分裂多颗高速弹珠",
    "落地引怪聚餐",
    "冲刺推怪顶瓜",
    "粘液炼油附加易碎",
];

fn catalog() -> Vec<Value> {
    let mut fruits = FruitKind::SEEDS.to_vec();
    for a in FruitKind::SEEDS {
        for b in FruitKind::SEEDS {
            fruits.extend(a.combine(b));
        }
    }
    fruits.sort_by_key(|k| *k as u8);
    fruits.dedup();
    let mut entries: Vec<_> = fruits
        .into_iter()
        .map(|k| {
            let id = k as usize;
            json!({"id":id, "name":k.name(), "note":NOTES[id],
               "recipe":k.parents().map(|(a,b)| [a as u8,b as u8])})
        })
        .collect();
    for (id, name) in [(14, "小黑"), (15, "露露西")] {
        entries.push(json!({"id":id,"name":name,"note":NOTES[id],"recipe":null}));
    }
    entries
}

// An absent stage record means uncleared. A recorded value is its best surviving HP.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Save {
    version: u8,
    revision: u32,
    started: bool,
    muted: bool,
    clears: BTreeMap<String, u8>,
    fruits: BTreeSet<u8>,
    companions: BTreeSet<u8>,
}

impl Default for Save {
    fn default() -> Self {
        Self {
            version: 1,
            revision: 0,
            started: false,
            muted: false,
            clears: BTreeMap::new(),
            fruits: (0..4).collect(),
            companions: BTreeSet::new(),
        }
    }
}

impl Save {
    fn decode(raw: &str) -> Option<Self> {
        let save: Self = serde_json::from_str(raw).ok()?;
        (save.version == 1
            && (0..4).all(|i| save.fruits.contains(&i))
            && save.fruits.iter().all(|&i| i < 14)
            && save.companions.iter().all(|i| [14, 15].contains(i))
            && save.clears.iter().all(|(k, hp)| {
                STAGES.contains(&k.as_str()) && (1..=5).contains(hp) && save.available(k)
            }))
        .then_some(save)
    }

    fn available(&self, stage: &str) -> bool {
        match stage {
            "t1" => true,
            "t2" => self.clears.contains_key("t1"),
            "t5" => self.clears.contains_key("t2"),
            _ => false,
        }
    }

    fn clear(&mut self, stage: &str, hp: u8) -> bool {
        if !self.available(stage) || !(1..=5).contains(&hp) {
            return false;
        }
        let best = self.clears.entry(stage.to_owned()).or_default();
        let improved = hp > *best;
        *best = (*best).max(hp);
        let recruited = match stage {
            "t1" => self.companions.insert(14),
            "t5" => self.companions.insert(15),
            _ => false,
        };
        improved || recruited
    }
}

#[derive(GodotClass, Default)]
#[class(base=RefCounted, init)]
struct OrchardJournal {
    state: Save,
}

#[godot_api]
impl OrchardJournal {
    #[func]
    fn entries(&self) -> GString {
        json!(catalog()).to_string().as_str().into()
    }

    #[func]
    fn encode(&self) -> GString {
        serde_json::to_string(&self.state).unwrap().as_str().into()
    }

    #[func]
    fn restore(&mut self, raw: GString) -> bool {
        let Some(candidate) = Save::decode(&raw.to_string()) else {
            return false;
        };
        if candidate.revision < self.state.revision {
            return false;
        }
        self.state = candidate;
        true
    }

    #[func]
    fn stage_open(&self, stage: GString) -> bool {
        self.state.available(&stage.to_string())
    }

    #[func]
    fn record_clear(&mut self, stage: GString, hp: u8) -> bool {
        let changed = self.state.clear(&stage.to_string(), hp);
        self.changed(changed)
    }

    #[func]
    fn discover(&mut self, id: u8) -> bool {
        let changed = match id {
            4..=13 => self.state.fruits.insert(id),
            14..=15 => self.state.companions.insert(id),
            _ => false,
        };
        self.changed(changed)
    }

    #[func]
    fn begin_campaign(&mut self) -> bool {
        let changed = !self.state.started;
        self.state.started = true;
        self.changed(changed)
    }

    #[func]
    fn set_muted(&mut self, muted: bool) -> bool {
        let changed = self.state.muted != muted;
        self.state.muted = muted;
        self.changed(changed)
    }
}

impl OrchardJournal {
    fn changed(&mut self, changed: bool) -> bool {
        if changed {
            self.state.revision = self.state.revision.saturating_add(1);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_matches_sprite_ids_and_recipes() {
        let entries = catalog();
        assert_eq!(entries.len(), 16);
        for (i, entry) in entries.iter().enumerate() {
            assert_eq!(entry["id"], i);
            assert!(entry["note"].as_str().unwrap().chars().count() <= 14);
            if (4..14).contains(&i) {
                let parents = entry["recipe"].as_array().unwrap();
                let [a, b] =
                    [0, 1].map(|j| FruitKind::SEEDS[parents[j].as_u64().unwrap() as usize]);
                assert_eq!(a.combine(b).unwrap() as usize, i);
            }
        }
    }

    #[test]
    fn campaign_keeps_best_and_requires_predecessor() {
        let mut save = Save::default();
        assert!(!save.clear("t2", 4));
        assert!(save.clear("t1", 3));
        assert!(!save.clear("t1", 2));
        assert!(save.clear("t1", 5));
        assert!(save.clear("t2", 1) && save.available("t5"));
        assert_eq!(save.clears["t1"], 5);
        assert!(save.companions.contains(&14));
    }

    #[test]
    fn profile_round_trip_and_reject_invalid_data() {
        let mut save = Save::default();
        assert_eq!(save.fruits, (0..4).collect());
        save.muted = true;
        save.fruits.insert(11);
        save.clear("t1", 4);
        let raw = serde_json::to_string(&save).unwrap();
        assert_eq!(Save::decode(&raw), Some(save));
        assert!(Save::decode(&raw.replace("\"version\":1", "\"version\":2")).is_none());
        assert!(Save::decode(&raw.replace("\"t1\":4", "\"t5\":4")).is_none());
        assert!(Save::decode("{").is_none());
    }
}
