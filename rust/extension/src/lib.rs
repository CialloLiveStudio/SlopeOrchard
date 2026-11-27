use godot::prelude::*;
use serde_json::{json, Value};
use slope_orchard_core::{Fruit, Orchard, Point, VfxEvent};

struct SlopeOrchardExtension;

#[gdextension]
unsafe impl ExtensionLibrary for SlopeOrchardExtension {}

#[derive(GodotClass)]
#[class(base=RefCounted, init)]
struct OrchardBridge {
    orchard: Orchard,
    debt: f64,
}

fn point(pos: Point) -> Value {
    json!([pos.x, pos.y])
}

fn fruit(value: Option<Fruit>) -> Value {
    value.map_or(Value::Null, |fruit| {
        json!({
            "kind": fruit.kind as u8, "stage": fruit.stage() as u8, "scale": fruit.stage().scale(),
            "age": fruit.age, "hanging": fruit.hanging(),
            "giant_age": fruit.age.saturating_sub(fruit.kind.growth_ticks()),
        })
    })
}

#[godot_api]
impl OrchardBridge {
    #[func]
    fn advance(&mut self, delta: f64) {
        if !delta.is_finite() || delta <= 0.0 {
            return;
        }
        const STEP: f64 = 1.0 / 120.0;
        self.debt += delta.min(0.25);
        while self.debt + 1e-10 >= STEP {
            self.orchard.step();
            self.debt -= STEP;
        }
    }

    #[func]
    fn cut_fruit(&mut self, slot: i32) -> bool {
        self.orchard.cut_fruit(slot as usize)
    }

    #[func]
    fn swap_plants(&mut self, from: i32, to: i32) -> bool {
        self.orchard.swap_plants(from as usize, to as usize)
    }

    #[func]
    fn move_chime(&mut self, from: i32, to: i32) -> bool {
        self.orchard.move_chime(from as usize, to as usize)
    }

    #[func]
    fn next_wave(&mut self) -> bool {
        self.orchard.next_wave()
    }

    #[func]
    fn reset_run(&mut self) {
        self.orchard = Orchard::default();
        self.debt = 0.0;
    }

    #[func]
    fn start_sandbox(&mut self) {
        self.orchard.sandbox();
        self.debt = 0.0;
    }

    #[func]
    fn snapshot(&self) -> GString {
        let orchard = &self.orchard;
        let bodies: Vec<_> = orchard.bodies.iter().map(|body| json!({
            "id": body.id, "kind": body.kind as u8, "stage": body.stage as u8,
            "scale": body.stage.scale(), "pos": point(body.pos), "velocity": point(body.velocity),
            "rolling": body.rolling, "bounces": body.bounces,
        })).collect();
        let monsters: Vec<_> = orchard
            .monsters
            .iter()
            .map(|monster| {
                json!({
                    "id": monster.id, "kind": monster.kind as u8, "hp": monster.hp,
                    "max_hp": monster.kind.hp(), "pos": point(monster.pos),
                })
            })
            .collect();
        json!({
            "wave": orchard.wave, "phase": orchard.phase as u8, "hp": orchard.cabin_hp,
            "slots": orchard.slots.map(fruit), "reserve": fruit(orchard.reserve),
            "chimes": orchard.chimes, "bodies": bodies, "monsters": monsters,
        })
        .to_string()
        .as_str()
        .into()
    }

    #[func]
    fn drain_events(&mut self) -> GString {
        let events: Vec<_> = self
            .orchard
            .events
            .drain(..)
            .map(|event| match event {
                VfxEvent::FruitCut(pos, kind) => json!(["cut", point(pos), kind as u8]),
                VfxEvent::FruitShatter(pos, kind) => json!(["shatter", point(pos), kind as u8]),
                VfxEvent::MonsterHit(pos, damage, giant) => {
                    json!(["hit", point(pos), damage, giant])
                }
                VfxEvent::MonsterDie(pos) => json!(["die", point(pos)]),
                VfxEvent::MelonBounce(pos) => json!(["bounce", point(pos)]),
                VfxEvent::TrellisKnock(pos) => json!(["knock", point(pos)]),
                VfxEvent::ChimeRing(pos) => json!(["ring", point(pos)]),
                VfxEvent::PumpkinBowl(pos) => json!(["bowl", point(pos)]),
                VfxEvent::CabinHit(pos) => json!(["cabin", point(pos)]),
            })
            .collect();
        serde_json::to_string(&events)
            .expect("finite orchard events")
            .as_str()
            .into()
    }
}
