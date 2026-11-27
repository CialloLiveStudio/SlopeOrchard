use godot::prelude::*;
use serde_json::{json, Value};
use slope_orchard_core::{
    Body, FruitKind, HellhoundState, LevelId, Orchard, Point, SlotItem, VfxEvent, DT,
    HOME_RUN_COOLDOWN, SLOTS, TRELLIS_Y,
};

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
            "growth": fruit.growth(),
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
        "rolling": body.rolling, "bounces": body.bounces, "combo_bounces": body.combo_bounces,
        "home_run": body.home_run,
    })
}

fn level_key(id: LevelId) -> &'static str {
    match id {
        LevelId::T1 => "t1",
        LevelId::T2 => "t2",
        LevelId::Sandbox => "sandbox",
    }
}

fn hellhound_snapshot(orchard: &Orchard) -> Value {
    let sandbox_enabled =
        (orchard.level().id == LevelId::Sandbox).then_some(orchard.hellhound.is_some());
    let Some(dog) = orchard.hellhound else {
        return json!({"present": false, "sandbox_enabled": sandbox_enabled});
    };
    let (mode, state, wild_hp, wild_max_hp, nap_ticks_left) = match dog.state {
        HellhoundState::Wild => (
            "wild",
            "wild",
            dog.tame_max - dog.tame_progress,
            dog.tame_max,
            0,
        ),
        HellhoundState::Tamed => ("companion", "tamed", 0, dog.tame_max, 0),
        HellhoundState::Patrolling => ("companion", "patrol", 0, 0, 0),
        HellhoundState::Charging => ("companion", "charge", 0, 0, 0),
        HellhoundState::Dizzy { ticks_left } => ("companion", "nap", 0, 0, ticks_left),
    };
    let config = orchard.level().companion;
    json!({
        "present": true, "sandbox_enabled": sandbox_enabled,
        "mode": mode, "state": state, "pos": point(dog.pos), "facing_right": dog.facing_right,
        "stamina": dog.stamina, "max_stamina": config.map_or(0, |companion| companion.stamina),
        "wild_hp": wild_hp, "wild_max_hp": wild_max_hp, "tame_progress": dog.tame_progress, "tame_max": dog.tame_max,
        "nap_ticks_left": nap_ticks_left, "nap_seconds_left": nap_ticks_left as f32 * DT,
        "nap_seconds_total": config.map_or(0., |companion| companion.nap_ticks as f32 * DT),
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
    fn click_cut(&mut self, x: f32, y: f32) -> bool {
        self.orchard.click_cut(Point::new(x, y))
    }

    #[func]
    fn slash_cut(&mut self, from: Vector2, to: Vector2, vx: f32) -> i32 {
        self.orchard
            .slash_cut(Point::new(from.x, from.y), Point::new(to.x, to.y), vx) as i32
    }

    #[func]
    fn trigger_home_run(&mut self, target_x: f32, target_y: f32) -> bool {
        self.orchard
            .trigger_home_run(Point::new(target_x, target_y))
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
    fn equip_plant(&mut self, slot: i32, kind: i32) -> bool {
        let kind = match kind {
            0 => FruitKind::RedApple,
            1 => FruitKind::BounceMelon,
            2 => FruitKind::BowlingPumpkin,
            _ => return false,
        };
        self.orchard.equip_plant(slot as usize, kind)
    }

    #[func]
    fn equip_chime(&mut self, slot: i32) -> bool {
        self.orchard.equip_chime(slot as usize)
    }

    #[func]
    fn next_level(&mut self) -> bool {
        let advanced = self.orchard.next_level();
        if advanced {
            self.debt = 0.0;
        }
        advanced
    }

    #[func]
    fn start_level(&mut self, id: GString) -> bool {
        let id = match id.to_string().as_str() {
            "t1" => LevelId::T1,
            "t2" => LevelId::T2,
            "sandbox" => LevelId::Sandbox,
            _ => return false,
        };
        self.orchard = Orchard::new(id, 0);
        self.debt = 0.0;
        true
    }

    #[func]
    fn reset_run(&mut self) {
        self.orchard = Orchard::new(self.orchard.level().id, 0);
        self.debt = 0.0;
    }

    #[func]
    fn start_sandbox(&mut self) {
        self.orchard.sandbox();
        self.debt = 0.0;
    }

    #[func]
    fn set_sandbox_companion(&mut self, enabled: bool) -> bool {
        self.orchard.set_sandbox_companion(enabled)
    }

    #[func]
    fn snapshot(&self) -> GString {
        let orchard = &self.orchard;
        let level = orchard.level();
        let state = orchard.snapshot();
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
                    "max_hp": monster.kind.hp(), "lid_hp": monster.lid_hp, "pos": point(monster.pos),
                })
            })
            .collect();
        json!({
            "level_id": level_key(state.level), "level_name": level.display_name,
            "wave": state.wave, "wave_count": level.waves.len(),
            "phase": state.phase as u8, "unlocked_slots": state.unlocked_slots,
            "hellhound": hellhound_snapshot(orchard), "companion_unlocked": state.companion_unlocked,
            "protagonist": {
                "hp": orchard.protagonist.hp, "pos": point(orchard.protagonist.position()),
                "home_run_cooldown": orchard.protagonist.home_run_cooldown,
                "home_run_charge": 1.0 - orchard.protagonist.home_run_cooldown as f32 / HOME_RUN_COOLDOWN as f32,
                "digestion_cooldown": orchard.protagonist.digestion_cooldown,
                "invulnerability_ticks": orchard.protagonist.invulnerability_ticks,
                "hit_flash": orchard.protagonist.hit_flash,
            },
            "slots": state.slots.map(|slot| item(slot.item)),
            "slot_geometry": state.slots.map(|slot| json!({
                "hook": point(slot.hook), "fruit_center": point(slot.fruit_center),
                "sway_angle": slot.sway_angle, "sway_angular_vel": slot.sway_angular_vel,
                "cut_radius": slot.cut_radius,
            })),
            "slot_x": SLOTS, "trellis_y": TRELLIS_Y,
            "reserve": orchard.reserve.iter().map(|&spare| item(Some(spare))).collect::<Vec<_>>(),
            "bodies": bodies, "monsters": monsters, "barricades": barricades, "juice_zones": juice_zones,
        })
        .to_string()
        .as_str()
        .into()
    }

    #[func]
    fn drain_events(&mut self) -> GString {
        self.take_events()
    }

    #[func]
    fn take_events(&mut self) -> GString {
        let events: Vec<_> = self
            .orchard
            .events
            .drain(..)
            .map(|event| match event {
                VfxEvent::FruitCut(pos, kind, vx) => {
                    json!(["cut", point(pos), kind as u8, vx])
                }
                VfxEvent::GreenCut(pos, kind, vx) => {
                    json!(["green_cut", point(pos), kind as u8, vx])
                }
                VfxEvent::FruitShatter(pos, kind, stage) => {
                    json!(["shatter", point(pos), kind as u8, stage as u8])
                }
                VfxEvent::GreenSplat(pos, kind) => {
                    json!(["green_splat", point(pos), kind as u8])
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
                VfxEvent::HomeRunDash { from, target } => {
                    json!(["home_run", point(target), point(from)])
                }
                VfxEvent::ShieldLidDent(pos, hp) => json!(["shield_lid_dent", point(pos), hp]),
                VfxEvent::ShieldLidBreak(pos) => json!(["shield_lid_break", point(pos)]),
                VfxEvent::ShieldRebound(pos) => json!(["shield", point(pos)]),
                VfxEvent::MelonCombo {
                    pos,
                    combo_bounces,
                    monster_hit,
                } => {
                    json!(["combo_bounces", point(pos), combo_bounces, monster_hit])
                }
                VfxEvent::TameProgress(pos, progress, max) => {
                    json!(["tame_progress", point(pos), max - progress, max])
                }
                VfxEvent::WildPant(pos) => json!(["wild_pant", point(pos)]),
                VfxEvent::WildMunch(pos, stage) => json!(["wild_munch", point(pos), stage as u8]),
                VfxEvent::WildDizzy(pos, stage) => json!(["wild_dizzy", point(pos), stage as u8]),
                VfxEvent::WildSour(pos) => json!(["wild_sour", point(pos)]),
                VfxEvent::Tamed(pos) => json!(["tamed", point(pos)]),
                VfxEvent::HellhoundCharge(pos) => json!(["hellhound_charge", point(pos)]),
                VfxEvent::HellhoundHit(pos, damage) => json!(["hellhound_hit", point(pos), damage]),
                VfxEvent::HellhoundDizzy(pos) => json!(["hellhound_nap", point(pos)]),
                VfxEvent::HellhoundRevived(pos) => json!(["hellhound_revived", point(pos)]),
            })
            .collect();
        serde_json::to_string(&events)
            .expect("finite orchard events")
            .as_str()
            .into()
    }
}
