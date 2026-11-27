use godot::prelude::*;
use serde_json::{json, Value};
use slope_orchard_core::{Body, Orchard, Point, SlotItem, VfxEvent};

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

fn item(value: Option<SlotItem>) -> Value {
    match value {
        Some(SlotItem::Fruit(fruit)) => json!({
            "type": "fruit",
            "kind": fruit.kind as u8, "stage": fruit.stage() as u8, "scale": fruit.stage().scale(),
            "age": fruit.age, "hanging": fruit.hanging(),
            "giant_age": fruit.age.saturating_sub(fruit.kind.growth_ticks()),
        }),
        Some(SlotItem::Chime { cooldown }) => json!({ "type": "chime", "cooldown": cooldown }),
        None => Value::Null,
    }
}

fn body_snapshot(body: &Body) -> Value {
    json!({
        "id": body.id, "kind": body.kind as u8, "stage": body.stage as u8,
        "scale": body.stage.scale(), "pos": point(body.pos), "velocity": point(body.velocity),
        "rolling": body.rolling, "bounces": body.bounces, "home_run": body.home_run,
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
    fn cut_fruit_angled(&mut self, slot: i32, vx: f32) -> bool {
        self.orchard.cut_fruit_angled(slot as usize, vx)
    }

    #[func]
    fn swap_plants(&mut self, from: i32, to: i32) -> bool {
        self.orchard.swap_plants(from as usize, to as usize)
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
        let bodies: Vec<_> = orchard.bodies.iter().map(body_snapshot).collect();
        let barricades: Vec<_> = orchard
            .barricades
            .iter()
            .map(|barrier| {
                json!({
                    "body": body_snapshot(barrier.body()), "hits_left": barrier.hits_left,
                    "age": barrier.age, "rotting": barrier.rotting(),
                })
            })
            .collect();
        let juice_zones: Vec<_> = orchard
            .juice_zones
            .iter()
            .map(|zone| {
                json!({
                    "left": zone.left, "right": zone.right, "ticks_left": zone.ticks_left,
                })
            })
            .collect();
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
            "wave": orchard.wave, "phase": orchard.phase as u8, "unlocked_slots": orchard.unlocked_slots,
            "protagonist": {
                "hp": orchard.protagonist.hp, "pos": point(orchard.protagonist.position()),
                "swing_cooldown": orchard.protagonist.swing_cooldown, "hit_flash": orchard.protagonist.hit_flash,
            },
            "slots": orchard.slots.map(item),
            "reserve": orchard.reserve.iter().map(|&spare| item(Some(spare))).collect::<Vec<_>>(),
            "bodies": bodies, "monsters": monsters, "barricades": barricades, "juice_zones": juice_zones,
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
                VfxEvent::FruitCut(pos, kind, vx) => json!(["cut", point(pos), kind as u8, vx]),
                VfxEvent::FruitShatter(pos, kind, stage) => {
                    json!(["shatter", point(pos), kind as u8, stage as u8])
                }
                VfxEvent::MonsterHit(pos, damage, giant) => {
                    json!(["hit", point(pos), damage, giant])
                }
                VfxEvent::MonsterDie(pos) => json!(["die", point(pos)]),
                VfxEvent::MelonBounce(pos) => json!(["bounce", point(pos)]),
                VfxEvent::TrellisKnock(pos) => json!(["knock", point(pos)]),
                VfxEvent::ChimeRing(pos) => json!(["ring", point(pos)]),
                VfxEvent::PumpkinBowl(pos) => json!(["bowl", point(pos)]),
                VfxEvent::BarricadeBite(pos) => json!(["bite", point(pos)]),
                VfxEvent::PumpkinRot(pos) => json!(["rot", point(pos)]),
                VfxEvent::ChiefHit(pos) => json!(["chief_hit", point(pos)]),
                VfxEvent::ChiefHeal(pos) => json!(["heal", point(pos)]),
                VfxEvent::ChiefBonk(pos) => json!(["bonk", point(pos)]),
                VfxEvent::HomeRun(pos) => json!(["home_run", point(pos)]),
                VfxEvent::ShieldRebound(pos) => json!(["shield", point(pos)]),
            })
            .collect();
        serde_json::to_string(&events)
            .expect("finite orchard events")
            .as_str()
            .into()
    }
}
