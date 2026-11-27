use std::collections::VecDeque;

mod hellhound;
mod levels;
pub use hellhound::{Hellhound, HellhoundState};
pub use levels::{CompanionDef, LevelDef, LevelId, SpawnGroup, TameFightDef, WaveDef};
pub mod balance;

pub const DT: f32 = 1.0 / 120.0;
pub const TRELLIS_Y: f32 = 62.0;
pub const SLOPE: f32 = 0.194;
pub const SLOTS: [f32; 6] = [122., 175., 228., 281., 334., 387.];
const GRAVITY: f32 = 610.0;
const SPAWN_X: f32 = 486.0;
const CHIEF_X: f32 = 62.0;
const DOORWAY_REACH_RIGHT: f32 = 115.;
const MAX_HP: u8 = 5;
pub const GREEN_CUT_RADIUS: f32 = 22.;
const VINE_LENGTH: f32 = 18.;
const MAX_SWAY: f32 = 18. * std::f32::consts::PI / 180.;
const CHIME_COOLDOWN: u32 = 120;
pub const HOME_RUN_COOLDOWN: u32 = 7 * 120;
const DIGESTION_COOLDOWN: u32 = 15 * 120;
const HOME_RUN_IFRAMES: u32 = 18;
const HOME_RUN_RADIUS: f32 = 48.;
const HOME_RUN_SPEED: f32 = 440.;
const HOME_RUN_APPROACH_SPEED: f32 = 310.;
const HOME_RUN_MELON_ROLL_OUT: f32 = 120.;
const HOME_RUN_MELON_IMPACT: f32 = 770.;
const MELON_GRAVITY: f32 = 685.;
const MELON_RESTITUTION: f32 = 0.78;
const MELON_MONSTER_RESTITUTION: f32 = 0.88;
const MELON_MIN_REBOUND: f32 = 115.;
const ROT_TICKS: u32 = 840;
const JUICE_TICKS: u32 = 720;
const JUICE_RADIUS: f32 = 28.0;
const ROLL_THRESHOLD: f32 = 45.0;
const SWIPE_DROP_SPEED: f32 = 90.0;
const MAX_CUT_VX: f32 = 29.24277; // 90 * tan(18 degrees)

pub fn ground(x: f32) -> f32 {
    155.0 + SLOPE * (x - 70.0)
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    fn distance_squared(self, other: Self) -> f32 {
        (self.x - other.x).powi(2) + (self.y - other.y).powi(2)
    }

    fn within_segment(self, a: Self, b: Self, radius: f32) -> bool {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let length_squared = dx * dx + dy * dy;
        let t = if length_squared > 0.0 {
            (((self.x - a.x) * dx + (self.y - a.y) * dy) / length_squared).clamp(0., 1.)
        } else {
            0.0
        };
        (self.x - a.x - dx * t).powi(2) + (self.y - a.y - dy * t).powi(2) <= radius.powi(2)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FruitKind {
    RedApple,
    BounceMelon,
    BowlingPumpkin,
}

impl FruitKind {
    pub fn growth_ticks(self) -> u32 {
        match self {
            Self::RedApple => 504,
            Self::BounceMelon => 744,
            Self::BowlingPumpkin => 1260,
        }
    }

    pub fn mature_ticks(self) -> u32 {
        match self {
            Self::RedApple => 240,
            Self::BounceMelon => 384,
            Self::BowlingPumpkin => 660,
        }
    }

    pub fn hanging_ticks(self) -> u32 {
        (self.mature_ticks() * 3).div_ceil(10)
    }

    fn radius(self) -> f32 {
        match self {
            Self::RedApple => 6.5,
            Self::BounceMelon => 8.0,
            Self::BowlingPumpkin => 9.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Small,
    Mature,
    Giant,
}

impl Stage {
    pub fn scale(self) -> f32 {
        match self {
            Self::Small => 0.5,
            Self::Mature => 0.85,
            Self::Giant => 1.2,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Fruit {
    pub kind: FruitKind,
    pub age: u32,
}

impl Fruit {
    pub fn new(kind: FruitKind) -> Self {
        Self { kind, age: 0 }
    }

    pub fn stage(self) -> Stage {
        let growth = self.kind.growth_ticks();
        if self.age >= growth {
            Stage::Giant
        } else if self.age >= self.kind.mature_ticks() {
            Stage::Mature
        } else {
            Stage::Small
        }
    }

    pub fn hanging(self) -> bool {
        self.age >= self.kind.hanging_ticks()
    }

    pub fn growth(self) -> f32 {
        let mature = self.kind.mature_ticks();
        if self.age < mature {
            0.5 * self.age as f32 / mature as f32
        } else {
            0.5 + 0.5 * (self.age - mature) as f32 / (self.kind.growth_ticks() - mature) as f32
        }
        .min(1.)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SlotItem {
    Fruit(Fruit),
    Chime { cooldown: u32 },
}

#[derive(Debug, Default, Clone, Copy)]
pub struct VineSway {
    /// Angle from vertical, positive toward the downhill side, in radians.
    pub sway_angle: f32,
    /// Angular velocity in radians per second.
    pub sway_angular_vel: f32,
}

impl VineSway {
    fn impulse(&mut self, impulse: f32) {
        self.sway_angular_vel = (self.sway_angular_vel + impulse).clamp(-2.4, 2.4);
    }

    fn advance(&mut self, breeze: f32) {
        self.sway_angular_vel +=
            (breeze - 17. * self.sway_angle - 3.4 * self.sway_angular_vel) * DT;
        let angle = self.sway_angle + self.sway_angular_vel * DT;
        self.sway_angle = angle.clamp(-MAX_SWAY, MAX_SWAY);
        if angle != self.sway_angle {
            self.sway_angular_vel = 0.;
        }
    }

    fn offset(self) -> Point {
        Point::new(
            VINE_LENGTH * self.sway_angle.sin(),
            VINE_LENGTH * (self.sway_angle.cos() - 1.),
        )
    }

    fn tangent(self) -> Point {
        let speed = VINE_LENGTH * self.sway_angular_vel;
        Point::new(
            speed * self.sway_angle.cos(),
            -speed * self.sway_angle.sin(),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SlotSnapshot {
    pub hook: Point,
    pub fruit_center: Point,
    pub item: Option<SlotItem>,
    pub sway_angle: f32,
    pub sway_angular_vel: f32,
    pub cut_radius: Option<f32>,
}

#[derive(Debug, Clone, Copy)]
pub struct OrchardSnapshot {
    pub level: LevelId,
    pub phase: Phase,
    pub wave: usize,
    pub unlocked_slots: usize,
    pub slots: [SlotSnapshot; SLOTS.len()],
    pub hellhound: Option<Hellhound>,
    pub companion_unlocked: bool,
}

impl From<FruitKind> for SlotItem {
    fn from(kind: FruitKind) -> Self {
        Self::Fruit(Fruit::new(kind))
    }
}

#[derive(Debug)]
pub struct Body {
    pub id: u32,
    pub kind: FruitKind,
    pub stage: Stage,
    pub pos: Point,
    pub velocity: Point,
    pub bounces: u32,
    pub combo_bounces: u32,
    pub rolling: bool,
    pub home_run: bool,
    home_run_lane_distance: f32,
    hit: Vec<u32>,
    trellis_contacts: u8,
    trellis_grazes: u8,
}

impl Body {
    fn radius(&self) -> f32 {
        self.kind.radius() * self.stage.scale()
    }

    fn push(&self) -> f32 {
        if self.stage == Stage::Small {
            return 5.0;
        }
        let mass = match self.kind {
            FruitKind::RedApple => 1.0,
            FruitKind::BounceMelon => 0.42,
            FruitKind::BowlingPumpkin => 4.0,
        };
        mass * self.stage.scale() * 26.0 + if self.home_run { 180. } else { 0. }
    }

    fn damage(&self, kind: MonsterKind) -> u8 {
        let base = match (self.kind, self.stage, kind) {
            (_, Stage::Small, _) | (FruitKind::BowlingPumpkin, _, MonsterKind::Shield) => return 1,
            (FruitKind::BowlingPumpkin, Stage::Giant, MonsterKind::IronBoar) if self.rolling => 5,
            (FruitKind::BowlingPumpkin, Stage::Mature, MonsterKind::IronBoar) if self.rolling => 3,
            (FruitKind::RedApple, Stage::Giant, MonsterKind::IronBoar) => 6,
            (FruitKind::RedApple, Stage::Giant, _) => 4,
            (FruitKind::RedApple, Stage::Mature, MonsterKind::IronBoar) => 3,
            (FruitKind::RedApple, Stage::Mature, _) => 2,
            (FruitKind::BounceMelon, Stage::Giant, MonsterKind::IronBoar) => 2,
            (FruitKind::BounceMelon, Stage::Giant, MonsterKind::GreenSlime) => 2,
            (FruitKind::BounceMelon, Stage::Mature, MonsterKind::IronBoar) => 1,
            (FruitKind::BounceMelon, Stage::Giant, _) => 3,
            (FruitKind::BounceMelon, Stage::Mature, _) => 2,
            (FruitKind::BowlingPumpkin, Stage::Giant, _) => 7,
            (FruitKind::BowlingPumpkin, Stage::Mature, _) => 5,
        };
        base + if self.home_run { 2 } else { 0 }
    }

    fn roll_speed_cap(&self) -> f32 {
        match (self.home_run, self.stage) {
            (true, _) => HOME_RUN_SPEED,
            (false, Stage::Giant) => 170.,
            (false, _) => 145.,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterKind {
    GreenSlime,
    IronBoar,
    /// The ElasticShield enemy, with a springy cap and exposed sides.
    Shield,
    WildHellhound,
}

impl MonsterKind {
    pub fn hp(self) -> u8 {
        match self {
            Self::GreenSlime => 3,
            Self::IronBoar => 9,
            Self::Shield => 6,
            Self::WildHellhound => 1,
        }
    }

    pub fn radius(self) -> f32 {
        match self {
            Self::GreenSlime => 7.,
            Self::IronBoar => 10.,
            Self::Shield => 9.,
            Self::WildHellhound => 12.,
        }
    }

    pub fn speed(self) -> f32 {
        match self {
            Self::GreenSlime => 29.,
            Self::IronBoar => 20.,
            Self::Shield => 24.,
            Self::WildHellhound => 48.,
        }
    }
}

#[derive(Debug)]
pub struct Monster {
    pub id: u32,
    pub kind: MonsterKind,
    pub hp: u8,
    pub pos: Point,
    knockback: f32,
    stride: u32,
    bite_cooldown: u32,
    hit_companion: bool,
}

impl Monster {
    pub fn horizontal_speed(&self) -> f32 {
        self.knockback - self.kind.speed()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Wave,
    Intermission,
    Victory,
    Defeat,
    Sandbox,
}

#[derive(Debug)]
pub enum VfxEvent {
    FruitCut(Point, FruitKind, f32),
    FruitShatter(Point, FruitKind, Stage),
    MonsterHit(Point, u8, bool),
    MonsterDie(Point),
    MelonBounce(Point),
    TrellisKnock(Point),
    ChimeRing(Point),
    PumpkinBowl(Point),
    BarricadeBite(Point),
    PumpkinRot(Point),
    ChiefHit(Point),
    ChiefHeal(Point),
    ChiefBonk(Point),
    HomeRunDash {
        from: Point,
        target: Point,
    },
    ShieldRebound(Point),
    GreenCut(Point, FruitKind, f32),
    GreenSplat(Point, FruitKind),
    MelonCombo {
        pos: Point,
        combo_bounces: u32,
        monster_hit: bool,
    },
    TameProgress(Point, u8, u8),
    Tamed(Point),
    HellhoundCharge(Point),
    HellhoundHit(Point, u8),
    HellhoundDizzy(Point),
    HellhoundRevived(Point),
}

pub struct Protagonist {
    pub hp: u8,
    pub home_run_cooldown: u32,
    pub digestion_cooldown: u32,
    pub invulnerability_ticks: u32,
    pub hit_flash: u32,
}

impl Protagonist {
    pub fn position(&self) -> Point {
        let x = CHIEF_X - 6. * self.hit_flash as f32 / 24.;
        Point::new(x, ground(x) - 11.)
    }

    fn interact(&mut self, body: &Body, old: Point) -> Option<VfxEvent> {
        let y = ground(CHIEF_X) - 22. - body.radius();
        if body.rolling || body.home_run || body.velocity.y <= 0. || old.y > y || body.pos.y < y {
            return None;
        }
        let t = (y - old.y) / (body.pos.y - old.y);
        let x = old.x + (body.pos.x - old.x) * t;
        // The doorway reach catches a left swipe from slot zero, whose straight drop stays clear.
        if !(CHIEF_X - 10. ..=DOORWAY_REACH_RIGHT).contains(&x) {
            return None;
        }
        let contact = Point::new(x, y);
        if body.stage == Stage::Small {
            return Some(VfxEvent::ChiefBonk(contact));
        }
        if body.kind == FruitKind::RedApple
            && body.stage == Stage::Giant
            && self.hp < MAX_HP
            && self.digestion_cooldown == 0
        {
            self.hp += 1;
            self.digestion_cooldown = DIGESTION_COOLDOWN;
            return Some(VfxEvent::ChiefHeal(contact));
        }
        None
    }
}

pub struct Barricade {
    body: Body,
    pub hits_left: u8,
    pub age: u32,
}

impl Barricade {
    pub fn body(&self) -> &Body {
        &self.body
    }

    pub fn rotting(&self) -> bool {
        self.age >= ROT_TICKS - 240
    }
}

pub struct JuiceZone {
    pub left: f32,
    pub right: f32,
    pub ticks_left: u32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CombatContribution {
    pub damage: u32,
    pub kills: u32,
}

impl CombatContribution {
    fn record(&mut self, damage: u8, killed: bool) {
        self.damage += u32::from(damage);
        self.kills += u32::from(killed);
    }
}

#[derive(Debug, Default)]
pub struct RunStats {
    /// Damage actually removed from enemy HP, excluding overkill and taming.
    pub fruits: [CombatContribution; 3],
    pub hellhound: CombatContribution,
    pub first_damage_tick: Option<u32>,
    pub max_concurrent_monsters: usize,
}

pub struct Orchard {
    pub phase: Phase,
    pub wave: usize,
    pub protagonist: Protagonist,
    pub unlocked_slots: usize,
    pub slots: [Option<SlotItem>; SLOTS.len()],
    pub reserve: Vec<SlotItem>,
    pub bodies: Vec<Body>,
    pub barricades: Vec<Barricade>,
    pub juice_zones: Vec<JuiceZone>,
    pub monsters: Vec<Monster>,
    pub events: VecDeque<VfxEvent>,
    pub hellhound: Option<Hellhound>,
    pub companion_unlocked: bool,
    pub stats: RunStats,
    level: &'static LevelDef,
    vines: [VineSway; SLOTS.len()],
    seed: u64,
    elapsed: u32,
    tick: u32,
    spawned: Vec<u32>,
    next_id: u32,
}

impl Default for Orchard {
    fn default() -> Self {
        Self::new(LevelId::T1, 0)
    }
}

impl Orchard {
    pub fn new(id: LevelId, seed: u64) -> Self {
        let level = LevelDef::load(id);
        let sandbox = id == LevelId::Sandbox;
        let mut slots = [None; SLOTS.len()];
        let fruit_slots = slots
            .iter_mut()
            .enumerate()
            .filter(|(index, _)| Some(*index) != level.initial_chime_slot);
        for ((_, slot), &kind) in fruit_slots.zip(level.initial_plants) {
            *slot = Some(SlotItem::Fruit(Fruit {
                kind,
                age: if sandbox { kind.growth_ticks() } else { 0 },
            }));
        }
        if let Some(slot) = level.initial_chime_slot {
            slots[slot] = Some(SlotItem::Chime { cooldown: 0 });
        }
        let mut orchard = Self {
            phase: if sandbox { Phase::Sandbox } else { Phase::Wave },
            wave: 1,
            protagonist: Protagonist {
                hp: MAX_HP,
                home_run_cooldown: 0,
                digestion_cooldown: 0,
                invulnerability_ticks: 0,
                hit_flash: 0,
            },
            unlocked_slots: level.initial_unlocked_slots,
            slots,
            reserve: Vec::new(),
            bodies: Vec::new(),
            barricades: Vec::new(),
            juice_zones: Vec::new(),
            monsters: Vec::new(),
            events: VecDeque::new(),
            level,
            hellhound: level
                .companion
                .filter(|_| level.companion_enabled)
                .map(|config| Hellhound::companion(config.stamina)),
            companion_unlocked: level.companion_enabled,
            stats: RunStats::default(),
            vines: [VineSway::default(); SLOTS.len()],
            seed,
            elapsed: 0,
            tick: 0,
            spawned: vec![0; level.waves.first().map_or(1, |wave| wave.spawns.len())],
            next_id: 1,
        };
        if sandbox {
            orchard.reserve.push(SlotItem::Fruit(Fruit {
                kind: FruitKind::BounceMelon,
                age: FruitKind::BounceMelon.growth_ticks(),
            }));
            for &(kind, x) in level.sandbox_opening {
                orchard.spawn(kind, x);
            }
        }
        orchard
    }

    pub fn sandbox(&mut self) {
        *self = Self::new(LevelId::Sandbox, self.seed);
    }

    pub fn set_sandbox_companion(&mut self, enabled: bool) -> bool {
        if self.phase != Phase::Sandbox {
            return false;
        }
        if enabled {
            let config = self.level.companion.unwrap();
            self.hellhound
                .get_or_insert_with(|| Hellhound::companion(config.stamina));
        } else {
            self.hellhound = None;
        }
        true
    }

    pub fn level(&self) -> &'static LevelDef {
        self.level
    }

    pub fn next_level(&mut self) -> bool {
        if self.phase != Phase::Victory {
            return false;
        }
        let Some(next) = self.level.next_level() else {
            return false;
        };
        *self = Self::new(next, self.seed);
        true
    }

    fn can_equip(&self, slot: usize) -> bool {
        slot < self.unlocked_slots
            && (matches!(self.phase, Phase::Intermission | Phase::Sandbox)
                || self.phase == Phase::Wave && self.elapsed == 0)
    }

    pub fn equip_plant(&mut self, slot: usize, kind: FruitKind) -> bool {
        if !self.can_equip(slot) || !self.level.available_plants.contains(&kind) {
            return false;
        }
        if !matches!(self.slots[slot], Some(SlotItem::Fruit(fruit)) if fruit.kind == kind) {
            self.slots[slot] = Some(kind.into());
        }
        true
    }

    pub fn equip_chime(&mut self, slot: usize) -> bool {
        if !self.can_equip(slot) || self.level.initial_chime_slot.is_none() {
            return false;
        }
        if let Some(from) = self
            .slots
            .iter()
            .position(|item| matches!(item, Some(SlotItem::Chime { .. })))
        {
            self.slots.swap(from, slot);
        } else if let Some(index) = self
            .reserve
            .iter()
            .position(|item| matches!(item, SlotItem::Chime { .. }))
        {
            let chime = self.reserve.remove(index);
            if let Some(plant) = self.slots[slot].replace(chime) {
                self.reserve.push(plant);
            }
        } else {
            self.slots[slot] = Some(SlotItem::Chime { cooldown: 0 });
        }
        true
    }

    pub fn slot_snapshot(&self, slot: usize) -> SlotSnapshot {
        let sway = self.vines[slot];
        let hook = Point::new(SLOTS[slot], TRELLIS_Y - VINE_LENGTH);
        let offset = sway.offset();
        let item = self.slots[slot];
        SlotSnapshot {
            hook,
            fruit_center: Point::new(hook.x + offset.x, TRELLIS_Y + offset.y),
            item,
            sway_angle: sway.sway_angle,
            sway_angular_vel: sway.sway_angular_vel,
            cut_radius: match item {
                Some(SlotItem::Fruit(fruit)) if slot < self.unlocked_slots && fruit.hanging() => {
                    Some(if fruit.stage() == Stage::Small {
                        GREEN_CUT_RADIUS
                    } else {
                        19.
                    })
                }
                _ => None,
            },
        }
    }

    pub fn snapshot(&self) -> OrchardSnapshot {
        OrchardSnapshot {
            level: self.level.id,
            phase: self.phase,
            wave: self.wave,
            unlocked_slots: self.unlocked_slots,
            slots: std::array::from_fn(|slot| self.slot_snapshot(slot)),
            hellhound: self.hellhound,
            companion_unlocked: self.companion_unlocked,
        }
    }

    fn active(&self) -> bool {
        matches!(self.phase, Phase::Wave | Phase::Sandbox)
    }

    pub fn swap_plants(&mut self, from: usize, to: usize) -> bool {
        if !matches!(self.phase, Phase::Intermission | Phase::Sandbox)
            || from == to
            || (self.unlocked_slots..SLOTS.len()).contains(&from)
            || (self.unlocked_slots..SLOTS.len()).contains(&to)
        {
            return false;
        }
        let tray = SLOTS.len();
        match (from < tray, to < tray) {
            (true, true) if self.slots[from].is_some() => self.slots.swap(from, to),
            (true, false) if to - tray <= self.reserve.len() => {
                let Some(item) = self.slots[from].take() else {
                    return false;
                };
                if let Some(spare) = self.reserve.get_mut(to - tray) {
                    self.slots[from] = Some(std::mem::replace(spare, item));
                } else {
                    self.reserve.push(item);
                }
            }
            (false, true) if from - tray < self.reserve.len() => {
                let index = from - tray;
                if let Some(item) = self.slots[to].as_mut() {
                    std::mem::swap(&mut self.reserve[index], item);
                } else {
                    self.slots[to] = Some(self.reserve.remove(index));
                }
            }
            (false, false)
                if from - tray < self.reserve.len() && to - tray < self.reserve.len() =>
            {
                self.reserve.swap(from - tray, to - tray);
            }
            _ => return false,
        }
        true
    }

    pub fn next_wave(&mut self) -> bool {
        if self.phase != Phase::Intermission {
            return false;
        }
        self.phase = Phase::Wave;
        self.wave += 1;
        self.tick = 0;
        self.spawned = vec![0; self.level.waves[self.wave - 1].spawns.len()];
        true
    }

    fn id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn cut_fruit(&mut self, slot: usize) -> bool {
        self.cut_fruit_angled(slot, 0.)
    }

    pub fn cut_fruit_angled(&mut self, slot: usize, vx: f32) -> bool {
        if !self.active() || !vx.is_finite() {
            return false;
        }
        self.drop_fruit(slot, vx.clamp(-MAX_CUT_VX, MAX_CUT_VX))
    }

    pub fn click_cut(&mut self, point: Point) -> bool {
        let slot = (0..self.unlocked_slots)
            .filter_map(|slot| {
                let view = self.slot_snapshot(slot);
                let distance = point.distance_squared(view.fruit_center);
                (distance <= view.cut_radius?.powi(2)).then_some((slot, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        slot.is_some_and(|(slot, _)| self.cut_fruit(slot))
    }

    pub fn slash_cut(&mut self, from: Point, to: Point, vx: f32) -> usize {
        if !self.active() || !vx.is_finite() {
            return 0;
        }
        let targets: [bool; SLOTS.len()] = std::array::from_fn(|slot| {
            let view = self.slot_snapshot(slot);
            view.cut_radius
                .is_some_and(|radius| view.fruit_center.within_segment(from, to, radius))
        });
        targets
            .into_iter()
            .enumerate()
            .filter(|&(slot, hit)| hit && self.cut_fruit_angled(slot, vx))
            .count()
    }

    fn drop_fruit(&mut self, slot: usize, vx: f32) -> bool {
        if slot >= self.unlocked_slots {
            return false;
        }
        let Some(SlotItem::Fruit(fruit)) = self.slots[slot] else {
            return false;
        };
        if !fruit.hanging() {
            return false;
        }
        let pos = self.slot_snapshot(slot).fruit_center;
        let tangent = self.vines[slot].tangent();
        let velocity = Point::new(
            tangent.x + vx,
            tangent.y + if vx == 0. { 0. } else { SWIPE_DROP_SPEED },
        );
        let id = self.id();
        self.bodies.push(Body {
            id,
            pos,
            kind: fruit.kind,
            stage: fruit.stage(),
            velocity,
            bounces: 0,
            combo_bounces: 0,
            rolling: false,
            home_run: false,
            home_run_lane_distance: 0.,
            hit: Vec::new(),
            trellis_contacts: 0,
            trellis_grazes: 0,
        });
        self.slots[slot] = Some(fruit.kind.into());
        for neighbor in [slot.wrapping_sub(1), slot + 1] {
            if neighbor < self.unlocked_slots {
                self.vines[neighbor].impulse(if neighbor < slot { -1.05 } else { 1.05 });
            }
        }
        self.events.push_back(if fruit.stage() == Stage::Small {
            VfxEvent::GreenCut(pos, fruit.kind, velocity.x)
        } else {
            VfxEvent::FruitCut(pos, fruit.kind, velocity.x)
        });
        true
    }

    pub fn trigger_home_run(&mut self, cursor: Point) -> bool {
        if !self.active() || self.protagonist.home_run_cooldown != 0 {
            return false;
        }
        let airborne = self
            .bodies
            .iter()
            .enumerate()
            .filter(|(_, body)| !body.rolling && body.velocity.y >= 0.)
            .map(|(index, body)| (false, index, body));
        let parked = self
            .barricades
            .iter()
            .enumerate()
            .map(|(index, barrier)| (true, index, &barrier.body));
        let target = airborne
            .chain(parked)
            .filter(|(_, _, body)| body.stage != Stage::Small)
            .map(|(parked, index, body)| (parked, index, body.pos.distance_squared(cursor)))
            .filter(|(_, _, distance)| *distance <= HOME_RUN_RADIUS.powi(2))
            .min_by(|a, b| a.2.total_cmp(&b.2));
        let Some((parked, index, _)) = target else {
            return false;
        };

        let body = if parked {
            let mut body = self.barricades.remove(index).body;
            body.rolling = true;
            self.bodies.push(body);
            self.bodies.last_mut().unwrap()
        } else {
            &mut self.bodies[index]
        };
        body.home_run = true;
        body.velocity = Point::new(HOME_RUN_SPEED, HOME_RUN_SPEED * SLOPE);
        body.home_run_lane_distance = 0.;
        body.hit.clear();
        self.protagonist.home_run_cooldown = HOME_RUN_COOLDOWN;
        self.protagonist.invulnerability_ticks = HOME_RUN_IFRAMES;
        self.events.push_back(VfxEvent::HomeRunDash {
            from: self.protagonist.position(),
            target: body.pos,
        });
        true
    }

    fn ring(&mut self, slot: usize) {
        self.slots[slot] = Some(SlotItem::Chime {
            cooldown: CHIME_COOLDOWN,
        });
        self.events
            .push_back(VfxEvent::ChimeRing(Point::new(SLOTS[slot], TRELLIS_Y)));
        for (index, vine) in self.vines[..self.unlocked_slots].iter_mut().enumerate() {
            let distance = index.abs_diff(slot);
            if distance > 0 {
                let direction = if index < slot { -1. } else { 1. };
                vine.impulse(direction * 2.2 / distance as f32);
            }
        }
        for neighbor in [slot.wrapping_sub(1), slot + 1] {
            if neighbor >= self.unlocked_slots {
                continue;
            }
            if let Some(SlotItem::Fruit(fruit)) = &mut self.slots[neighbor] {
                if fruit.stage() == Stage::Giant {
                    self.drop_fruit(neighbor, 0.);
                } else {
                    fruit.age = fruit.kind.growth_ticks();
                }
            }
        }
    }

    fn hit_trellis(&mut self, body: &mut Body, old: Point) {
        for (slot, x) in SLOTS.into_iter().take(self.unlocked_slots).enumerate() {
            let pos = Point::new(x, TRELLIS_Y);
            let fruit_center = self.slot_snapshot(slot).fruit_center;
            let mask = 1 << slot;
            if body.trellis_contacts & mask != 0 {
                continue;
            }
            match self.slots[slot] {
                Some(SlotItem::Chime { cooldown: 0 })
                    if pos.within_segment(old, body.pos, body.radius() + 5.) =>
                {
                    body.trellis_contacts |= mask;
                    self.ring(slot);
                }
                Some(SlotItem::Fruit(fruit))
                    if body.kind == FruitKind::BounceMelon
                        && body.stage != Stage::Small
                        && body.bounces > 0
                        && fruit.hanging()
                        && fruit_center.within_segment(
                            old,
                            body.pos,
                            body.radius() + fruit.kind.radius() * fruit.stage().scale(),
                        ) =>
                {
                    body.trellis_contacts |= mask;
                    self.vines[slot].impulse(self.graze_impulse(slot, body));
                    self.drop_fruit(slot, 0.);
                    self.events.push_back(VfxEvent::TrellisKnock(fruit_center));
                }
                Some(SlotItem::Fruit(_))
                    if body.kind == FruitKind::BounceMelon
                        && body.stage != Stage::Small
                        && body.bounces > 0
                        && body.trellis_grazes & mask == 0
                        && Point::new(fruit_center.x, fruit_center.y + 13.).within_segment(
                            old,
                            body.pos,
                            body.radius() + 6.,
                        ) =>
                {
                    body.trellis_grazes |= mask;
                    let impulse = self.graze_impulse(slot, body);
                    self.vines[slot].impulse(impulse);
                }
                _ => {}
            }
        }
    }

    fn graze_impulse(&self, slot: usize, body: &Body) -> f32 {
        if body.pos.x < SLOTS[slot] {
            1.45
        } else {
            -1.45
        }
    }

    fn rebound(&mut self, body: &mut Body) -> bool {
        let impact = if body.home_run {
            HOME_RUN_MELON_IMPACT
        } else {
            body.velocity.y
        };
        let speed = impact * MELON_RESTITUTION;
        if speed < MELON_MIN_REBOUND {
            return self.shatter(body);
        }
        self.launch_melon_rebound(body, speed);
        self.melon_bounce_event(body, false);
        true
    }

    fn melon_bounce_event(&mut self, body: &Body, monster_hit: bool) {
        self.events.push_back(VfxEvent::MelonBounce(body.pos));
        self.events.push_back(VfxEvent::MelonCombo {
            pos: body.pos,
            combo_bounces: body.combo_bounces,
            monster_hit,
        });
    }

    fn launch_melon_rebound(&self, body: &mut Body, speed: f32) {
        body.bounces += 1;
        body.combo_bounces += 1;
        body.home_run = false;
        body.velocity.y = -speed;
        body.velocity.x = if body.pos.x < 460. { 34. } else { -24. };
        let rise = body.pos.y - TRELLIS_Y;
        let effective_speed = speed - MELON_GRAVITY * DT / 2.;
        let discriminant = effective_speed * effective_speed - 2. * MELON_GRAVITY * rise;
        if rise > 0. && discriminant >= 0. {
            let target = SLOTS
                .iter()
                .zip(self.slots)
                .filter(|(x, item)| {
                    matches!(item, Some(SlotItem::Chime { cooldown: 0 }))
                        && (**x - body.pos.x).abs() <= 60.
                })
                .map(|(x, _)| *x - body.pos.x)
                .min_by(|a, b| a.abs().total_cmp(&b.abs()));
            if let Some(dx) = target {
                let travel = 2. * rise / (effective_speed + discriminant.sqrt());
                body.velocity.x = (dx / travel).clamp(-185., 185.);
            }
        }
        body.hit.clear();
        body.trellis_contacts = 0;
        body.trellis_grazes = 0;
    }

    fn shatter(&mut self, body: &Body) -> bool {
        self.events.push_back(if body.stage == Stage::Small {
            VfxEvent::GreenSplat(body.pos, body.kind)
        } else {
            VfxEvent::FruitShatter(body.pos, body.kind, body.stage)
        });
        false
    }

    fn slippery(zones: &[JuiceZone], x: f32) -> bool {
        zones
            .iter()
            .any(|zone| (zone.left..=zone.right).contains(&x))
    }

    fn try_barricade_hit(&mut self, body: &mut Body, old: Point) -> Option<bool> {
        if body.rolling || body.home_run || body.velocity.y <= 0. {
            return None;
        }
        let index = self.barricades.iter().position(|barrier| {
            let top = barrier.body.pos.y - barrier.body.radius() - body.radius();
            old.y <= top
                && body.pos.y >= top
                && (barrier.body.pos.x - body.pos.x).abs()
                    <= barrier.body.radius() + body.radius() * 0.5
        })?;
        if body.stage == Stage::Small {
            return Some(self.shatter(body));
        }
        if body.kind == FruitKind::BounceMelon {
            body.pos.y = self.barricades[index].body.pos.y
                - self.barricades[index].body.radius()
                - body.radius();
            return Some(self.rebound(body));
        }
        let mut parked = self.barricades.remove(index).body;
        let speed = (body.velocity.y * 0.38 + body.velocity.x.max(0.) * 0.4)
            .clamp(105., parked.roll_speed_cap());
        parked.velocity = Point::new(speed, speed * SLOPE);
        parked.rolling = true;
        parked.hit.clear();
        self.events.push_back(VfxEvent::PumpkinBowl(parked.pos));
        self.bodies.push(parked);
        Some(self.shatter(body))
    }

    fn shield_contact(body: &Body, old: Point, monster: &Monster) -> Option<Point> {
        if body.rolling
            || body.home_run
            || body.stage == Stage::Small
            || body.velocity.y <= 0.
            || monster.kind != MonsterKind::Shield
        {
            return None;
        }
        let y = monster.pos.y - 8. - body.radius();
        if old.y > y || body.pos.y < y {
            return None;
        }
        let x = old.x + (body.pos.x - old.x) * ((y - old.y) / (body.pos.y - old.y));
        ((x - monster.pos.x).abs() <= 10.).then_some(Point::new(x, y))
    }

    fn move_body(&mut self, body: &mut Body) -> bool {
        let old = body.pos;
        let radius = body.radius();
        if body.rolling {
            let traction = if Self::slippery(&self.juice_zones, old.x) {
                2.8
            } else {
                1.
            };
            body.velocity.x = (body.velocity.x
                + traction * self.level.rolling_friction * GRAVITY * SLOPE / (1. + SLOPE * SLOPE)
                    * DT)
                .min(body.roll_speed_cap());
            body.velocity.y = body.velocity.x * SLOPE;
            body.pos.x += body.velocity.x * DT;
            body.pos.y = ground(body.pos.x) - radius;
        } else if body.home_run {
            // Airborne targets converge on the slope, then keep a constant clearance.
            let dx = body.velocity.x * DT;
            let clearance = (ground(old.x) - radius - old.y).max(0.);
            let descent = (HOME_RUN_APPROACH_SPEED * DT).min(clearance);
            body.pos.x += dx;
            body.velocity.y = body.velocity.x * SLOPE + descent / DT;
            body.pos.y = ground(body.pos.x) - radius - (clearance - descent);
            if clearance <= descent {
                body.home_run_lane_distance += dx;
                if body.kind == FruitKind::BowlingPumpkin {
                    body.rolling = true;
                    self.events.push_back(VfxEvent::PumpkinBowl(body.pos));
                }
            }
        } else {
            body.velocity.y += if body.kind == FruitKind::BounceMelon {
                MELON_GRAVITY
            } else {
                GRAVITY
            } * DT;
            body.pos.x += body.velocity.x * DT;
            body.pos.y += body.velocity.y * DT;
        }
        if body.pos.x > 503. || body.pos.x < -24. {
            return false;
        }
        if body.velocity.y < 0. && !body.home_run {
            self.hit_trellis(body, old);
            return true;
        }

        if self.feed_companion(body, old) {
            return false;
        }
        if let Some(event) = self.protagonist.interact(body, old) {
            if matches!(event, VfxEvent::ChiefBonk(_)) {
                self.events.push_back(event);
                return self.shatter(body);
            }
            self.events.push_back(event);
            return false;
        }
        if let Some(survives) = self.try_barricade_hit(body, old) {
            return survives;
        }

        for index in 0..self.monsters.len() {
            let monster = &mut self.monsters[index];
            if monster.hp == 0 || body.hit.contains(&monster.id) {
                continue;
            }
            if let Some(contact) = Self::shield_contact(body, old, monster) {
                body.pos = contact;
                if body.kind == FruitKind::BounceMelon {
                    let speed = body.velocity.y * 1.5;
                    self.launch_melon_rebound(body, speed);
                    self.melon_bounce_event(body, true);
                } else {
                    body.velocity.x *= 1.5;
                    body.velocity.y *= -1.5;
                    body.hit.clear();
                    body.trellis_contacts = 0;
                }
                self.events.push_back(VfxEvent::ShieldRebound(contact));
                return true;
            }
            // The shield cap sits above a narrower body, leaving its sides exposed.
            let (center, hit_radius) = if monster.kind == MonsterKind::Shield {
                (Point::new(monster.pos.x, monster.pos.y + 4.), 7.)
            } else {
                (monster.pos, monster.kind.radius())
            };
            if !center.within_segment(old, body.pos, radius + hit_radius) {
                continue;
            }
            let damage = body.damage(monster.kind);
            let (id, kind, pos) = (monster.id, monster.kind, monster.pos);
            let combo_push = if body.kind == FruitKind::BounceMelon {
                1. + 0.10 * body.combo_bounces.min(6) as f32
            } else {
                1.
            };
            monster.knockback += body.push() * combo_push
                / match kind {
                    MonsterKind::Shield if body.kind == FruitKind::BowlingPumpkin => 2.8,
                    MonsterKind::IronBoar => 2.1,
                    MonsterKind::WildHellhound => 1.6,
                    _ => 1.,
                };
            body.hit.push(id);
            if kind == MonsterKind::WildHellhound {
                self.tame_hit(body, pos, damage);
            } else {
                let actual_damage = damage.min(monster.hp);
                monster.hp -= actual_damage;
                self.stats.fruits[body.kind as usize].record(actual_damage, monster.hp == 0);
                self.events.push_back(VfxEvent::MonsterHit(
                    pos,
                    actual_damage,
                    body.stage == Stage::Giant,
                ));
                if monster.hp == 0 {
                    self.events.push_back(VfxEvent::MonsterDie(pos));
                }
            }
            if body.stage == Stage::Small {
                return self.shatter(body);
            }
            if body.home_run && kind == MonsterKind::GreenSlime && body.kind == FruitKind::RedApple
            {
                body.velocity.x *= 0.98;
                body.velocity.y *= 0.98;
                continue;
            }
            match body.kind {
                FruitKind::RedApple => return self.shatter(body),
                FruitKind::BounceMelon => {
                    let impact = if body.home_run {
                        HOME_RUN_MELON_IMPACT
                    } else {
                        body.velocity.y
                    };
                    self.launch_melon_rebound(body, impact * MELON_MONSTER_RESTITUTION);
                    body.velocity.x = -24.;
                    body.hit.push(id);
                    self.melon_bounce_event(body, true);
                    return true;
                }
                FruitKind::BowlingPumpkin => {
                    self.events.push_back(VfxEvent::PumpkinBowl(body.pos));
                    if body.rolling || body.home_run {
                        let retained_speed = match (body.stage, kind) {
                            (Stage::Giant, MonsterKind::GreenSlime) => 0.92,
                            (_, MonsterKind::GreenSlime) => 0.88,
                            (_, MonsterKind::Shield) => 0.08,
                            (Stage::Giant, _) => 0.60,
                            (_, _) => 0.45,
                        };
                        body.velocity.x *= retained_speed;
                        body.velocity.y *= retained_speed;
                        if body.velocity.x < ROLL_THRESHOLD {
                            break;
                        }
                    } else {
                        body.pos.y = ground(body.pos.x) - radius;
                        break;
                    }
                }
            }
        }

        if body.home_run {
            if body.kind == FruitKind::BounceMelon
                && body.home_run_lane_distance >= HOME_RUN_MELON_ROLL_OUT
            {
                return self.rebound(body);
            }
        } else if !body.rolling && body.pos.y + radius >= ground(body.pos.x) {
            body.pos.y = ground(body.pos.x) - radius;
            if body.stage == Stage::Small {
                return self.shatter(body);
            }
            match body.kind {
                FruitKind::RedApple => return self.shatter(body),
                FruitKind::BounceMelon => return self.rebound(body),
                FruitKind::BowlingPumpkin => {
                    body.rolling = true;
                    body.velocity.x = ((body.velocity.x + body.velocity.y * SLOPE)
                        / (1. + SLOPE * SLOPE))
                        .clamp(ROLL_THRESHOLD + 5., body.roll_speed_cap());
                    body.velocity.y = body.velocity.x * SLOPE;
                    self.events.push_back(VfxEvent::PumpkinBowl(body.pos));
                }
            }
        }
        true
    }

    fn spawn(&mut self, kind: MonsterKind, x: f32) {
        let id = self.id();
        let pos = Point::new(x, ground(x) - kind.radius());
        if kind == MonsterKind::WildHellhound {
            self.hellhound = self
                .level
                .tame_fight
                .map(|fight| Hellhound::wild(pos, fight.resolve));
        }
        self.monsters.push(Monster {
            id,
            kind,
            hp: kind.hp(),
            pos,
            knockback: 0.,
            stride: 0,
            bite_cooldown: 0,
            hit_companion: false,
        });
        self.stats.max_concurrent_monsters =
            self.stats.max_concurrent_monsters.max(self.monsters.len());
    }

    fn spawn_due(&mut self, groups: &[SpawnGroup]) {
        for (index, group) in groups.iter().enumerate() {
            let ordinal = self.spawned[index];
            if ordinal >= group.count {
                continue;
            }
            // Independent, stable jitter keeps the same level seed across every strategy.
            let mut key = self
                .seed
                .wrapping_add((self.wave as u64) << 40)
                .wrapping_add((index as u64) << 32)
                .wrapping_add(u64::from(ordinal))
                .wrapping_add(0x9e3779b97f4a7c15);
            key = (key ^ (key >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            key = (key ^ (key >> 27)).wrapping_mul(0x94d049bb133111eb);
            let jitter = (key ^ (key >> 31)) % (u64::from(group.jitter) + 1);
            let due = u64::from(group.first_tick)
                + u64::from(ordinal) * u64::from(group.interval)
                + jitter;
            if u64::from(self.tick) >= due {
                let kind = group.pattern[ordinal as usize % group.pattern.len()];
                self.spawn(kind, SPAWN_X);
                self.spawned[index] += 1;
            }
        }
    }

    fn grant(&mut self, item: SlotItem) {
        if let Some(slot) = self.slots[..self.unlocked_slots]
            .iter_mut()
            .find(|slot| slot.is_none())
        {
            *slot = Some(item);
        } else {
            self.reserve.push(item);
        }
    }

    fn end_wave(&mut self) {
        let Some(next) = self.level.waves.get(self.wave) else {
            self.phase = Phase::Victory;
            return;
        };
        self.phase = Phase::Intermission;
        self.bodies.clear();
        self.barricades.clear();
        self.juice_zones.clear();
        self.unlocked_slots = next.unlocked_slots;
        for &kind in next.new_plants {
            self.grant(kind.into());
        }
    }

    fn update_barricades(&mut self) {
        self.juice_zones.retain_mut(|zone| {
            zone.ticks_left -= 1;
            zone.ticks_left > 0
        });
        self.barricades.retain_mut(|barrier| {
            barrier.age += 1;
            if barrier.age <= ROT_TICKS {
                return true;
            }
            let pos = barrier.body.pos;
            self.juice_zones.push(JuiceZone {
                left: pos.x - JUICE_RADIUS,
                right: pos.x + JUICE_RADIUS,
                ticks_left: JUICE_TICKS,
            });
            self.events.push_back(VfxEvent::PumpkinRot(pos));
            false
        });
    }

    fn move_monsters(&mut self) {
        for monster in &mut self.monsters {
            monster.stride += 1;
            monster.bite_cooldown = monster.bite_cooldown.saturating_sub(1);
            let old_x = monster.pos.x;
            let slippery = Self::slippery(&self.juice_zones, old_x);
            let slide = if slippery { 1.65 } else { 1. };
            let speed = if monster.kind == MonsterKind::WildHellhound {
                self.level.tame_fight.unwrap().charge_speed
            } else {
                monster.kind.speed()
            };
            let mut x = (old_x + (monster.knockback * slide - speed) * DT).min(SPAWN_X + 10.);
            monster.knockback *= if slippery {
                0.993
            } else {
                self.level.knockback_retention
            };
            let mut blocked = false;
            if x < old_x {
                let contact = self
                    .barricades
                    .iter_mut()
                    .filter_map(|barrier| {
                        let edge =
                            barrier.body.pos.x + barrier.body.radius() + monster.kind.radius();
                        (barrier.hits_left > 0 && old_x >= barrier.body.pos.x && x <= edge)
                            .then_some((edge, barrier))
                    })
                    .max_by(|(a, _), (b, _)| a.total_cmp(b));
                if let Some((edge, barrier)) = contact {
                    x = edge;
                    blocked = true;
                    if monster.bite_cooldown == 0 {
                        monster.bite_cooldown = 84;
                        barrier.hits_left -= 1;
                        self.events
                            .push_back(VfxEvent::BarricadeBite(barrier.body.pos));
                        if barrier.hits_left == 0 {
                            self.events.push_back(VfxEvent::FruitShatter(
                                barrier.body.pos,
                                barrier.body.kind,
                                barrier.body.stage,
                            ));
                        }
                    }
                }
            }
            let arc = (monster.stride % 78) as f32 / 78.;
            let lift = if monster.kind == MonsterKind::GreenSlime && !blocked {
                22. * arc * (1. - arc)
            } else {
                0.
            };
            monster.pos = Point::new(x, ground(x) - monster.kind.radius() - lift);
            if monster.kind == MonsterKind::WildHellhound {
                if let Some(dog) = &mut self.hellhound {
                    dog.pos = monster.pos;
                }
            }
        }
        self.barricades.retain(|barrier| barrier.hits_left > 0);
    }

    pub fn step(&mut self) {
        if !self.active() {
            return;
        }
        let sandbox = self.phase == Phase::Sandbox;
        let level = self.level;
        let groups = if sandbox {
            level.sandbox_reinforcements.as_slice()
        } else {
            level.waves[self.wave - 1].spawns
        };
        self.spawn_due(groups);
        self.tick += 1;
        self.elapsed += 1;
        self.protagonist.home_run_cooldown = self.protagonist.home_run_cooldown.saturating_sub(1);
        self.protagonist.digestion_cooldown = self.protagonist.digestion_cooldown.saturating_sub(1);
        self.protagonist.hit_flash = self.protagonist.hit_flash.saturating_sub(1);

        for slot in 0..self.unlocked_slots {
            let t = self.elapsed as f32 * DT;
            let phase = (self.seed % 101) as f32 * 0.16 + slot as f32 * 1.2;
            let breeze =
                self.level.wind_base * ((2.3 * t + phase).sin() + 0.4 * (3.7 * t + phase).sin());
            self.vines[slot].advance(breeze);
            match &mut self.slots[slot] {
                Some(SlotItem::Fruit(fruit)) => {
                    fruit.age += 1;
                    if fruit.age > fruit.kind.growth_ticks() + 420 {
                        self.drop_fruit(slot, 0.);
                    }
                }
                Some(SlotItem::Chime { cooldown }) => *cooldown = cooldown.saturating_sub(1),
                None => {}
            }
        }
        for item in &mut self.reserve {
            if let SlotItem::Chime { cooldown } = item {
                *cooldown = cooldown.saturating_sub(1);
            }
        }

        self.update_barricades();
        self.move_monsters();
        self.update_companion();
        for mut body in std::mem::take(&mut self.bodies) {
            if self.move_body(&mut body) {
                if body.rolling && body.velocity.x < ROLL_THRESHOLD {
                    body.rolling = false;
                    body.home_run = false;
                    body.velocity = Point::default();
                    self.barricades.push(Barricade {
                        body,
                        age: 0,
                        hits_left: 2,
                    });
                } else {
                    self.bodies.push(body);
                }
            }
        }

        let tamed = self
            .hellhound
            .is_some_and(|dog| matches!(dog.state, HellhoundState::Tamed));
        if tamed {
            self.monsters
                .retain(|monster| monster.kind != MonsterKind::WildHellhound);
            self.phase = Phase::Victory;
            return;
        }
        self.monsters.retain_mut(|monster| {
            if monster.hp == 0 {
                return false;
            }
            if monster.pos.x <= CHIEF_X {
                if self.protagonist.invulnerability_ticks > 0 {
                    monster.pos.x = CHIEF_X;
                    return true;
                }
                self.protagonist.hp = self.protagonist.hp.saturating_sub(1);
                self.stats.first_damage_tick.get_or_insert(self.elapsed);
                self.protagonist.hit_flash = 24;
                self.events
                    .push_back(VfxEvent::ChiefHit(self.protagonist.position()));
                if monster.kind == MonsterKind::WildHellhound {
                    let x = level.tame_fight.unwrap().retreat_x;
                    monster.pos = Point::new(x, ground(x) - monster.kind.radius());
                    monster.knockback = 0.;
                    if let Some(dog) = &mut self.hellhound {
                        dog.pos = monster.pos;
                    }
                    return true;
                }
                return false;
            }
            true
        });
        self.protagonist.invulnerability_ticks =
            self.protagonist.invulnerability_ticks.saturating_sub(1);

        if self.protagonist.hp == 0 {
            if sandbox {
                self.protagonist.hp = MAX_HP;
            } else {
                self.phase = Phase::Defeat;
            }
        }
        if self.phase == Phase::Wave
            && self.monsters.is_empty()
            && groups
                .iter()
                .zip(&self.spawned)
                .all(|(group, &count)| count == group.count)
        {
            self.end_wave();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn isolated_sandbox() -> Orchard {
        let mut orchard = Orchard::new(LevelId::Sandbox, 0);
        orchard.set_sandbox_companion(false);
        orchard.monsters.clear();
        orchard.spawned[0] = u32::MAX;
        orchard
    }

    fn set_fruit(orchard: &mut Orchard, slot: usize, kind: FruitKind, stage: Stage) {
        let age = match stage {
            Stage::Small => kind.hanging_ticks(),
            Stage::Mature => kind.mature_ticks(),
            Stage::Giant => kind.growth_ticks(),
        };
        orchard.slots[slot] = Some(SlotItem::Fruit(Fruit { kind, age }));
    }

    fn stage_at(orchard: &Orchard, slot: usize) -> Stage {
        match orchard.slots[slot] {
            Some(SlotItem::Fruit(fruit)) => fruit.stage(),
            _ => panic!("missing fruit in slot {slot}"),
        }
    }

    #[test]
    fn growth_thresholds_match_each_crop_and_green_hitboxes_work_for_both_inputs() {
        for (kind, mature, giant) in [
            (FruitKind::RedApple, 240, 504),
            (FruitKind::BounceMelon, 384, 744),
            (FruitKind::BowlingPumpkin, 660, 1260),
        ] {
            let fruit = |age| Fruit { kind, age };
            assert_eq!((kind.mature_ticks(), kind.growth_ticks()), (mature, giant));
            assert_eq!(fruit(mature - 1).stage(), Stage::Small);
            assert_eq!(fruit(mature).stage(), Stage::Mature);
            assert_eq!(fruit(mature).growth(), 0.5);
            assert_eq!(fruit(giant - 1).stage(), Stage::Mature);
            assert_eq!(fruit(giant).stage(), Stage::Giant);
            assert_eq!(fruit(giant).growth(), 1.);
            let mut orchard = Orchard::default();
            let first = kind.hanging_ticks();
            orchard.slots[0] = Some(SlotItem::Fruit(fruit(first - 1)));
            assert!(fruit(first - 1).growth() < 0.15);
            assert!(!orchard.click_cut(Point::new(SLOTS[0], TRELLIS_Y)));
            assert!(fruit(first).growth() >= 0.15);
            for slash in [false, true] {
                orchard.slots[0] = Some(SlotItem::Fruit(fruit(first)));
                let view = orchard.snapshot().slots[0];
                assert!(view.cut_radius.unwrap() >= 22.);
                let y = view.fruit_center.y + 22.;
                let x = view.fruit_center.x;
                if slash {
                    assert_eq!(
                        orchard.slash_cut(Point::new(x - 7., y), Point::new(x + 7., y), 15.),
                        1
                    );
                } else {
                    assert!(orchard.click_cut(Point::new(x, y)));
                }
                assert_eq!(orchard.bodies.last().unwrap().stage, Stage::Small);
                assert!(
                    matches!(orchard.events.back(), Some(VfxEvent::GreenCut(_, event_kind, _)) if *event_kind == kind)
                );
            }
        }
    }

    #[test]
    fn vine_wind_neighbor_cut_chime_and_graze_excite_sway_and_drops_inherit_tangent() {
        let mut orchard = isolated_sandbox();
        let mut peak = 0_f32;
        for _ in 0..120 {
            orchard.step();
            peak = peak.max(orchard.snapshot().slots[0].sway_angle.abs());
        }
        assert!((0.01..MAX_SWAY).contains(&peak));
        let before = orchard.vines[1].sway_angular_vel;
        assert!(orchard.cut_fruit(0));
        assert!(orchard.vines[1].sway_angular_vel > before);
        let before = orchard.vines[2].sway_angular_vel;
        orchard.ring(3);
        assert!(orchard.vines[2].sway_angular_vel < before);
        let mut peak: f32 = 0.;
        for _ in 0..100 {
            orchard.step();
            peak = peak.max(orchard.vines[2].sway_angle.abs());
        }
        assert!((8_f32.to_radians()..=MAX_SWAY).contains(&peak));

        let center = orchard.slot_snapshot(1).fruit_center;
        let mut melon = body_at(
            &mut orchard,
            FruitKind::BounceMelon,
            Stage::Giant,
            Point::new(center.x, center.y + 17.),
            Point::new(0., -195.),
        );
        melon.bounces = 1;
        let before = orchard.vines[1].sway_angular_vel;
        assert!(orchard.move_body(&mut melon));
        assert_ne!(orchard.vines[1].sway_angular_vel, before);
        set_fruit(&mut orchard, 1, FruitKind::RedApple, Stage::Mature);
        let sway = orchard.vines[1];
        let tangent = sway.tangent();
        let center = orchard.slot_snapshot(1).fruit_center;
        assert!(orchard.cut_fruit_angled(1, 15.));
        let body = orchard.bodies.last().unwrap();
        assert_eq!((body.pos.x, body.pos.y), (center.x, center.y));
        assert_eq!(body.velocity.x, tangent.x + 15.);
        assert_eq!(body.velocity.y, tangent.y + SWIPE_DROP_SPEED);
    }

    #[test]
    fn melon_monster_rebound_biases_uphill_and_increments_combo_knockback() {
        let mut pushes = Vec::new();
        for previous_combo in [0, 3] {
            let mut orchard = Orchard::default();
            orchard.spawn(MonsterKind::GreenSlime, 212.);
            let pos = orchard.monsters[0].pos;
            let mut body = body_at(
                &mut orchard,
                FruitKind::BounceMelon,
                Stage::Mature,
                Point::new(pos.x, pos.y - 9.),
                Point::new(34., 250.),
            );
            body.combo_bounces = previous_combo;
            assert!(orchard.move_body(&mut body));
            assert!(body.velocity.x < 0.);
            assert_eq!(
                body.velocity.y,
                -(250. + MELON_GRAVITY * DT) * MELON_MONSTER_RESTITUTION
            );
            assert_eq!(body.combo_bounces, previous_combo + 1);
            assert!(
                matches!(orchard.events.back(), Some(VfxEvent::MelonCombo {combo_bounces, monster_hit: true, ..})
                if *combo_bounces == previous_combo + 1)
            );
            pushes.push(orchard.monsters[0].knockback);
        }
        assert!(pushes[1] > pushes[0]);
    }

    #[test]
    fn level_definitions_load_expected_content_and_allow_duplicate_plants() {
        let mut t1 = Orchard::new(LevelId::T1, 42);
        assert_eq!(t1.level().display_name, "初遇黑狼");
        assert_eq!(t1.snapshot().unlocked_slots, 2);
        assert!(!t1.equip_plant(0, FruitKind::BowlingPumpkin));
        assert!(!t1.equip_chime(0));
        assert!(t1.hellhound.is_none());
        let t2 = Orchard::new(LevelId::T2, 42);
        assert_eq!(t2.level().display_name, "节奏压测");
        assert!(t2.hellhound.is_some());
        assert!(t2
            .level()
            .waves
            .iter()
            .flat_map(|wave| wave.spawns)
            .any(|group| group.pattern.contains(&MonsterKind::Shield)));
        let mut sandbox = Orchard::new(LevelId::Sandbox, 42);
        assert_eq!(sandbox.level().display_name, "满配沙盒");
        assert_eq!(sandbox.unlocked_slots, 6);
        assert!(matches!(sandbox.slots[3], Some(SlotItem::Chime { .. })));
        assert!(sandbox.set_sandbox_companion(false));
        assert!(sandbox.snapshot().hellhound.is_none());
        assert!(sandbox.set_sandbox_companion(true));
        assert!(sandbox.snapshot().hellhound.is_some());
        for slot in 0..sandbox.unlocked_slots {
            assert!(sandbox.equip_plant(slot, FruitKind::BowlingPumpkin));
        }
        assert!(sandbox.slots.iter().all(|slot| matches!(
            slot,
            Some(SlotItem::Fruit(Fruit {
                kind: FruitKind::BowlingPumpkin,
                ..
            }))
        )));
        assert!(sandbox.equip_chime(3));
    }

    #[test]
    fn wild_hellhound_spawns_in_final_wave_and_tames_into_victory() {
        let mut orchard = Orchard::new(LevelId::T1, 7);
        for _ in 0..2 {
            orchard.end_wave();
            assert!(orchard.next_wave());
        }
        for _ in 0..1300 {
            orchard.step();
        }
        let config = orchard.level().tame_fight.unwrap();
        let dog = orchard.snapshot().hellhound.unwrap();
        assert!(matches!(dog.state, HellhoundState::Wild { remaining, max }
            if remaining == config.resolve && max == config.resolve));
        assert!(dog.pos.x < SPAWN_X);
        let hit = |orchard: &mut Orchard, kind: FruitKind| {
            let dog = orchard.hellhound.unwrap();
            let mut body = body_at(orchard, kind, Stage::Giant, dog.pos, Point::new(0., 220.));
            orchard.move_body(&mut body);
        };
        hit(&mut orchard, FruitKind::RedApple);
        assert!(
            matches!(orchard.hellhound.unwrap().state, HellhoundState::Wild { remaining, .. }
            if remaining == config.resolve - 4 - config.apple_bonus)
        );
        for _ in 0..2 {
            hit(&mut orchard, FruitKind::RedApple);
        }
        orchard.step();
        assert_eq!(orchard.phase, Phase::Victory);
        assert!(matches!(
            orchard.hellhound.unwrap().state,
            HellhoundState::Tamed
        ));
        assert!(orchard
            .events
            .iter()
            .any(|event| matches!(event, VfxEvent::Tamed(_))));
        assert!(!orchard
            .events
            .iter()
            .any(|event| matches!(event, VfxEvent::MonsterDie(_))));
        assert!(orchard.next_level());
        assert_eq!(orchard.level().id, LevelId::T2);
        assert!(matches!(
            orchard.hellhound.unwrap().state,
            HellhoundState::Patrolling
        ));
    }

    #[test]
    fn companion_patrols_charges_and_restarts_parked_pumpkins() {
        let mut orchard = Orchard::new(LevelId::T2, 0);
        let config = orchard.level().companion.unwrap();
        let start = orchard.hellhound.unwrap().pos.x;
        for _ in 0..1450 {
            orchard.update_companion();
            assert!((start..=config.patrol_end).contains(&orchard.hellhound.unwrap().pos.x));
        }
        let x = orchard.hellhound.unwrap().pos.x + 29.;
        orchard.spawn(MonsterKind::GreenSlime, x);
        for _ in 0..30 {
            orchard.update_companion();
        }
        assert_eq!(
            orchard.monsters[0].hp,
            MonsterKind::GreenSlime.hp() - config.damage
        );
        assert_eq!(orchard.monsters[0].knockback, config.knockback);
        assert_eq!(orchard.stats.hellhound.damage, u32::from(config.damage));
        orchard.monsters.clear();
        let x = orchard.hellhound.unwrap().pos.x;
        let kind = FruitKind::BowlingPumpkin;
        let body = body_at(
            &mut orchard,
            kind,
            Stage::Mature,
            Point::new(x, ground(x) - 8.),
            Point::default(),
        );
        let id = body.id;
        orchard.barricades.push(Barricade {
            body,
            age: 0,
            hits_left: 2,
        });
        for _ in 0..config.cooldown_ticks + 1200 {
            orchard.update_companion();
            if orchard.barricades.is_empty() {
                break;
            }
        }
        let restarted = orchard.bodies.iter().find(|body| body.id == id).unwrap();
        assert!(restarted.rolling && restarted.velocity.x > 100.);
    }

    #[test]
    fn three_boars_trigger_twelve_second_nap_and_a_ripe_apple_revives() {
        let mut orchard = Orchard::new(LevelId::T2, 0);
        for expected_stamina in [2, 1, 0] {
            let x = orchard.hellhound.unwrap().pos.x;
            orchard.spawn(MonsterKind::IronBoar, x + 10.);
            orchard.update_companion();
            orchard.update_companion();
            assert_eq!(orchard.hellhound.unwrap().stamina, expected_stamina);
            orchard.monsters.clear();
        }
        assert!(matches!(
            orchard.hellhound.unwrap().state,
            HellhoundState::Dizzy { ticks_left: 1439 }
        ));
        assert!(orchard
            .events
            .iter()
            .any(|event| matches!(event, VfxEvent::HellhoundDizzy(_))));
        let pos = orchard.hellhound.unwrap().pos;
        let mut apple = body_at(
            &mut orchard,
            FruitKind::RedApple,
            Stage::Small,
            pos,
            Point::new(0., 40.),
        );
        orchard.move_body(&mut apple);
        assert_eq!(orchard.hellhound.unwrap().stamina, 0);
        let mut apple = body_at(
            &mut orchard,
            FruitKind::RedApple,
            Stage::Mature,
            pos,
            Point::new(0., 40.),
        );
        assert!(!orchard.move_body(&mut apple));
        assert_eq!(orchard.hellhound.unwrap().stamina, 3);
        assert!(matches!(
            orchard.hellhound.unwrap().state,
            HellhoundState::Patrolling
        ));
        assert!(matches!(
            orchard.events.back(),
            Some(VfxEvent::HellhoundRevived(_))
        ));
    }

    fn body_at(
        orchard: &mut Orchard,
        kind: FruitKind,
        stage: Stage,
        pos: Point,
        velocity: Point,
    ) -> Body {
        set_fruit(orchard, 0, kind, stage);
        assert!(orchard.cut_fruit(0));
        let mut body = orchard.bodies.pop().unwrap();
        body.pos = pos;
        body.velocity = velocity;
        body
    }

    #[test]
    fn rising_body_rings_slot_chime_and_cooldown_lasts_120_ticks() {
        let mut orchard = Orchard {
            unlocked_slots: 4,
            ..Orchard::default()
        };
        orchard.slots[1] = Some(SlotItem::Chime { cooldown: 0 });
        set_fruit(&mut orchard, 0, FruitKind::RedApple, Stage::Mature);
        set_fruit(&mut orchard, 2, FruitKind::BowlingPumpkin, Stage::Giant);
        set_fruit(&mut orchard, 3, FruitKind::RedApple, Stage::Small);
        assert!(!orchard.cut_fruit(1));
        let mut rising = body_at(
            &mut orchard,
            FruitKind::RedApple,
            Stage::Mature,
            Point::new(SLOTS[1], TRELLIS_Y + 11.),
            Point::new(0., -560.),
        );
        set_fruit(&mut orchard, 0, FruitKind::RedApple, Stage::Mature);
        assert!(orchard.move_body(&mut rising));
        assert!(matches!(
            orchard.slots[1],
            Some(SlotItem::Chime { cooldown: 120 })
        ));
        assert_eq!(stage_at(&orchard, 0), Stage::Giant);
        assert_eq!(stage_at(&orchard, 2), Stage::Small);
        assert_eq!(stage_at(&orchard, 3), Stage::Small);
        assert_eq!(orchard.bodies[0].kind, FruitKind::BowlingPumpkin);
        assert_eq!(orchard.bodies[0].stage, Stage::Giant);
        for _ in 0..119 {
            orchard.step();
        }
        assert!(matches!(
            orchard.slots[1],
            Some(SlotItem::Chime { cooldown: 1 })
        ));
        rising.pos.y = TRELLIS_Y + 11.;
        rising.trellis_contacts = 0;
        assert!(orchard.move_body(&mut rising));
        assert_eq!(
            orchard
                .events
                .iter()
                .filter(|e| matches!(e, VfxEvent::ChimeRing(_)))
                .count(),
            1
        );
        orchard.step();
        assert!(matches!(
            orchard.slots[1],
            Some(SlotItem::Chime { cooldown: 0 })
        ));
    }

    #[test]
    fn unlocks_reject_locked_placements_and_tray_holds_multiple_items() {
        let mut orchard = Orchard::new(LevelId::T2, 0);
        assert_eq!(orchard.unlocked_slots, 3);
        assert!(orchard.slots[3..].iter().all(Option::is_none));
        assert!(!orchard.swap_plants(0, 3));
        orchard.end_wave();
        assert_eq!(orchard.unlocked_slots, 4);
        assert_eq!(stage_at(&orchard, 3), Stage::Small);
        assert!(!orchard.swap_plants(0, 4));
        assert!(orchard.swap_plants(0, 6));
        assert!(orchard.swap_plants(2, 7));
        assert_eq!(orchard.reserve.len(), 2);
        assert!(!orchard.swap_plants(7, 5));
        assert!(orchard.swap_plants(7, 0));
        assert!(orchard.swap_plants(6, 2));
        assert!(orchard.reserve.is_empty());
        assert!(orchard.next_wave());
        assert_eq!(orchard.wave, 2);
        assert!(!orchard.swap_plants(0, 1));
    }

    #[test]
    fn click_is_vertical_and_swipe_is_capped_at_18_degrees() {
        let mut orchard = Orchard::default();
        for slot in 0..2 {
            set_fruit(&mut orchard, slot, FruitKind::RedApple, Stage::Mature);
        }
        assert!(orchard.cut_fruit(0));
        assert!(!orchard.cut_fruit_angled(1, f32::NAN));
        orchard.vines[1] = VineSway::default();
        assert!(orchard.cut_fruit_angled(1, -500.));
        assert_eq!(orchard.bodies[0].velocity.x, 0.);
        let velocity = orchard.bodies[1].velocity;
        assert!((velocity.x.atan2(velocity.y).to_degrees() + 18.).abs() < 0.01);
    }

    #[test]
    fn small_green_fruit_shatters_on_ground_or_on_contact_for_one_damage() {
        for kind in [
            FruitKind::BounceMelon,
            FruitKind::BowlingPumpkin,
            FruitKind::RedApple,
        ] {
            let mut orchard = Orchard::default();
            let pos = Point::new(
                180.,
                ground(180.) - kind.radius() * Stage::Small.scale() - 1.,
            );
            let mut body = body_at(&mut orchard, kind, Stage::Small, pos, Point::new(0., 300.));
            assert!(!orchard.move_body(&mut body));
            assert_eq!(body.bounces, 0);
            assert!(!body.rolling);
            assert!(
                matches!(orchard.events.back(), Some(VfxEvent::GreenSplat(_, crop)) if *crop == kind)
            );
            orchard.spawn(MonsterKind::GreenSlime, 180.);
            assert!(!orchard.move_body(&mut body));
            assert_eq!(orchard.monsters[0].hp, 2);
            assert!(
                matches!(orchard.events.back(), Some(VfxEvent::GreenSplat(_, crop)) if *crop == kind)
            );
        }
    }

    #[test]
    fn floor_rebounds_retain_78_percent_then_shatter() {
        for (slot, x) in SLOTS.into_iter().enumerate() {
            for stage in [Stage::Mature, Stage::Giant] {
                let mut orchard = Orchard {
                    unlocked_slots: SLOTS.len(),
                    ..Orchard::default()
                };
                let kind = FruitKind::BounceMelon;
                set_fruit(&mut orchard, slot, kind, stage);
                assert!(orchard.cut_fruit(slot));
                let mut body = orchard.bodies.pop().unwrap();
                let floor = ground(x) - body.radius();
                let mut apex = [floor; 2];
                for _ in 0..900 {
                    let before = body.bounces;
                    let impact = body.velocity.y + MELON_GRAVITY * DT;
                    if !orchard.move_body(&mut body) {
                        assert!(impact * MELON_RESTITUTION < MELON_MIN_REBOUND);
                        break;
                    }
                    if body.bounces > before {
                        assert!((body.velocity.y + impact * MELON_RESTITUTION).abs() < 0.001);
                        assert_eq!(body.combo_bounces, body.bounces);
                    }
                    if (1..=2).contains(&body.bounces) {
                        let height = &mut apex[(body.bounces - 1) as usize];
                        *height = height.min(body.pos.y);
                    }
                }
                assert!(floor - apex[0] > 30.);
                assert!(apex[1] > apex[0]);
                assert!(body.bounces >= 2);
                assert!(matches!(
                    orchard.events.back(),
                    Some(VfxEvent::FruitShatter(..))
                ));
            }
        }
    }

    #[test]
    fn shield_boosted_melon_rings_a_chime_and_outlasts_three_bounces() {
        let mut orchard = Orchard::default();
        let x = SLOTS[1];
        orchard.slots[1] = Some(SlotItem::Chime { cooldown: 0 });
        orchard.spawn(MonsterKind::Shield, x);
        let mut melon = body_at(
            &mut orchard,
            FruitKind::BounceMelon,
            Stage::Giant,
            Point::new(x, TRELLIS_Y),
            Point::default(),
        );
        for _ in 0..100 {
            assert!(orchard.move_body(&mut melon));
            if melon.bounces > 0 {
                break;
            }
        }
        assert_eq!(melon.bounces, 1);
        assert_eq!(orchard.monsters[0].hp, MonsterKind::Shield.hp());
        orchard.monsters.clear();
        for _ in 0..1800 {
            if !orchard.move_body(&mut melon) {
                break;
            }
        }
        assert!(melon.bounces > 3);
        assert!(orchard
            .events
            .iter()
            .any(|e| matches!(e, VfxEvent::ChimeRing(_))));
        assert!(matches!(
            orchard.events.back(),
            Some(VfxEvent::FruitShatter(..))
        ));
    }

    fn park_pumpkin() -> Orchard {
        let mut orchard = Orchard::default();
        let kind = FruitKind::BowlingPumpkin;
        let pos = Point::new(203., ground(203.) - kind.radius() * Stage::Mature.scale());
        let mut body = body_at(&mut orchard, kind, Stage::Mature, pos, Point::new(80., 0.));
        body.rolling = true;
        orchard.bodies.push(body);
        orchard.spawn(MonsterKind::IronBoar, 219.);
        orchard.step();
        assert!(orchard.bodies.is_empty());
        assert_eq!(orchard.barricades[0].hits_left, 2);
        assert_eq!(orchard.monsters[0].hp, 6);
        orchard
    }

    #[test]
    fn parked_pumpkin_blocks_and_breaks_on_second_bite() {
        let mut orchard = park_pumpkin();
        orchard.tick = 5000;
        orchard.spawned[0] = orchard.level.waves[0].spawns[0].count;
        let edge = orchard.barricades[0].body.pos.x + orchard.barricades[0].body.radius() + 10.;
        for _ in 0..300 {
            orchard.step();
            if orchard.barricades[0].hits_left == 1 {
                break;
            }
        }
        assert_eq!(orchard.barricades[0].hits_left, 1);
        assert_eq!(orchard.monsters[0].pos.x, edge);
        for _ in 0..84 {
            orchard.step();
        }
        assert!(orchard.barricades.is_empty());
        assert!(orchard.juice_zones.is_empty());
        assert_eq!(
            orchard
                .events
                .iter()
                .filter(|e| matches!(e, VfxEvent::BarricadeBite(_)))
                .count(),
            2
        );
    }

    #[test]
    fn parked_pumpkin_bounces_melon_and_restarts_under_heavy_fruit() {
        let mut orchard = park_pumpkin();
        let parked = &orchard.barricades[0].body;
        let top = Point::new(parked.pos.x, parked.pos.y - parked.radius());
        let id = parked.id;
        for kind in [FruitKind::BounceMelon, FruitKind::BowlingPumpkin] {
            let pos = Point::new(top.x, top.y - kind.radius() * Stage::Giant.scale() - 1.);
            let mut hitter = body_at(&mut orchard, kind, Stage::Giant, pos, Point::new(0., 420.));
            let survived = orchard.move_body(&mut hitter);
            if kind == FruitKind::BounceMelon {
                assert!(survived && hitter.velocity.y < 0.);
                assert_eq!(orchard.barricades[0].hits_left, 2);
            } else {
                assert!(!survived);
                assert!(orchard.barricades.is_empty());
                let restarted = &orchard.bodies[0];
                assert_eq!(restarted.id, id);
                assert!(restarted.rolling && restarted.velocity.x > 100.);
            }
        }
    }

    #[test]
    fn parked_pumpkin_rots_then_juice_accelerates_bodies_and_expires() {
        let mut orchard = park_pumpkin();
        orchard.monsters.clear();
        orchard.tick = 200;
        orchard.spawned[0] = orchard.level.waves[0].spawns[0].count;
        let x = orchard.barricades[0].body.pos.x;
        for _ in 0..599 {
            orchard.update_barricades();
        }
        assert!(!orchard.barricades[0].rotting());
        orchard.update_barricades();
        assert!(orchard.barricades[0].rotting());
        for _ in 600..840 {
            orchard.update_barricades();
        }
        assert_eq!(orchard.barricades[0].age, 840);
        orchard.update_barricades();
        assert!(orchard.barricades.is_empty());
        let zone = &orchard.juice_zones[0];
        assert_eq!(
            (zone.left, zone.right, zone.ticks_left),
            (x - 28., x + 28., 720)
        );
        let mut dry = Orchard::default();
        for game in [&mut orchard, &mut dry] {
            let mut body = body_at(
                game,
                FruitKind::BowlingPumpkin,
                Stage::Mature,
                Point::new(x, ground(x) - 8.),
                Point::new(120., 0.),
            );
            body.rolling = true;
            game.bodies.push(body);
            game.spawn(MonsterKind::GreenSlime, x + 16.);
            game.monsters[0].knockback = 70.;
            game.step();
        }
        assert!(orchard.bodies[0].velocity.x > dry.bodies[0].velocity.x + 1.);
        assert!(orchard.monsters[0].pos.x > dry.monsters[0].pos.x);
        for _ in 1..719 {
            orchard.update_barricades();
        }
        assert_eq!(orchard.juice_zones[0].ticks_left, 1);
        orchard.update_barricades();
        assert!(orchard.juice_zones.is_empty());
    }

    #[test]
    fn slot_zero_vertical_drop_hits_the_slope_without_touching_the_protagonist() {
        for stage in [Stage::Small, Stage::Mature, Stage::Giant] {
            let mut orchard = Orchard::default();
            orchard.protagonist.hp = 4;
            orchard.spawn(MonsterKind::GreenSlime, SLOTS[0] + 14.);
            set_fruit(&mut orchard, 0, FruitKind::RedApple, stage);
            assert!(orchard.cut_fruit(0));
            for _ in 0..82 {
                orchard.step();
            }
            assert_eq!(orchard.protagonist.hp, 4);
            assert!(orchard
                .events
                .iter()
                .any(|e| matches!(e, VfxEvent::MonsterHit(..))));
            assert!(!orchard.events.iter().any(|e| matches!(
                e,
                VfxEvent::ChiefHeal(_) | VfxEvent::ChiefBonk(_) | VfxEvent::HomeRunDash { .. }
            )));
        }
    }

    #[test]
    fn protagonist_bonks_small_fruit_and_digests_only_giant_apples_for_fifteen_seconds() {
        let mut orchard = isolated_sandbox();
        orchard.protagonist.hp = 3;
        for stage in [Stage::Small, Stage::Mature] {
            set_fruit(&mut orchard, 0, FruitKind::RedApple, stage);
            assert!(orchard.cut_fruit_angled(0, -MAX_CUT_VX));
            for _ in 0..64 {
                orchard.step();
            }
            assert_eq!(orchard.protagonist.hp, 3);
        }
        assert!(orchard
            .events
            .iter()
            .any(|e| matches!(e, VfxEvent::ChiefBonk(_))));
        let drop_at_door = |stage, orchard: &mut Orchard| {
            let mut body = body_at(
                orchard,
                FruitKind::RedApple,
                stage,
                Point::new(CHIEF_X, TRELLIS_Y),
                Point::default(),
            );
            while orchard.move_body(&mut body) {}
        };

        set_fruit(&mut orchard, 0, FruitKind::RedApple, Stage::Giant);
        assert!(orchard.cut_fruit_angled(0, -MAX_CUT_VX));
        for _ in 0..60 {
            orchard.step();
            if orchard.protagonist.hp == 4 {
                break;
            }
        }
        assert_eq!(orchard.protagonist.hp, 4);
        assert_eq!(orchard.protagonist.digestion_cooldown, 1800);
        for _ in 0..1799 {
            orchard.step();
        }
        assert_eq!(orchard.protagonist.digestion_cooldown, 1);
        drop_at_door(Stage::Giant, &mut orchard);
        assert_eq!(orchard.protagonist.hp, 4);
        orchard.step();
        assert_eq!(orchard.protagonist.digestion_cooldown, 0);
        drop_at_door(Stage::Giant, &mut orchard);
        assert_eq!(orchard.protagonist.hp, 5);
        orchard.protagonist.digestion_cooldown = 0;
        drop_at_door(Stage::Giant, &mut orchard);
        assert_eq!(orchard.protagonist.hp, 5);
        assert_eq!(orchard.protagonist.digestion_cooldown, 0);
    }

    #[test]
    fn flash_step_snaps_to_nearest_valid_fruit_and_recharges_after_seven_seconds() {
        let mut orchard = isolated_sandbox();
        let cursor = Point::new(200., 80.);
        for (stage, x, vy) in [
            (Stage::Mature, 248., 25.),
            (Stage::Giant, 150., 25.),
            (Stage::Small, 200., 25.),
            (Stage::Giant, 210., -25.),
        ] {
            let body = body_at(
                &mut orchard,
                FruitKind::RedApple,
                stage,
                Point::new(x, 80.),
                Point::new(0., vy),
            );
            orchard.bodies.push(body);
        }
        assert!(!orchard.trigger_home_run(Point::new(200., 129.)));
        assert_eq!(orchard.protagonist.home_run_cooldown, 0);
        assert!(orchard.trigger_home_run(cursor));
        assert!(orchard.bodies[0].home_run);
        assert!(orchard.bodies[1..].iter().all(|body| !body.home_run));
        assert!(matches!(orchard.events.back(),
            Some(VfxEvent::HomeRunDash { from, target }) if from.x == CHIEF_X && target.x == 248.
        ));
        assert_eq!(orchard.protagonist.home_run_cooldown, 840);
        orchard.spawn(MonsterKind::GreenSlime, CHIEF_X);
        for _ in 0..HOME_RUN_IFRAMES {
            orchard.step();
            assert_eq!(orchard.protagonist.hp, 5);
        }
        orchard.step();
        assert_eq!(orchard.protagonist.hp, 4);
        for _ in (HOME_RUN_IFRAMES + 1)..839 {
            orchard.step();
        }
        assert_eq!(orchard.protagonist.home_run_cooldown, 1);
        set_fruit(&mut orchard, 0, FruitKind::RedApple, Stage::Mature);
        assert!(orchard.cut_fruit(0));
        let closest = orchard.bodies.last().unwrap();
        let (closest_id, next) = (closest.id, closest.pos);
        let farther = body_at(
            &mut orchard,
            FruitKind::BounceMelon,
            Stage::Giant,
            Point::new(next.x + 30., next.y),
            Point::default(),
        );
        orchard.bodies.push(farther);
        assert!(!orchard.trigger_home_run(next));
        orchard.step();
        assert_eq!(orchard.protagonist.home_run_cooldown, 0);
        assert!(orchard.trigger_home_run(next));
        assert_eq!(
            orchard.bodies.iter().find(|body| body.home_run).unwrap().id,
            closest_id
        );
    }

    #[test]
    fn flat_home_run_pierces_slimes_then_hits_the_shield_body_for_bonus_damage() {
        let mut orchard = Orchard::default();
        orchard.spawn(MonsterKind::GreenSlime, 184.);
        orchard.spawn(MonsterKind::GreenSlime, 223.);
        orchard.spawn(MonsterKind::Shield, 270.);
        let kind = FruitKind::RedApple;
        let stage = Stage::Mature;
        let pos = Point::new(160., ground(160.) - kind.radius() * stage.scale() - 1.);
        let apple = body_at(&mut orchard, kind, stage, pos, Point::new(0., 10.));
        orchard.bodies.push(apple);
        assert!(orchard.trigger_home_run(pos));
        let mut apple = orchard.bodies.pop().unwrap();
        while orchard.move_body(&mut apple) {
            assert!((apple.velocity.y / apple.velocity.x - SLOPE).abs() < 0.35);
            assert!(apple.velocity.x > 420.);
        }
        assert_eq!(orchard.monsters[0].hp, 0);
        assert_eq!(orchard.monsters[1].hp, 0);
        assert_eq!(orchard.monsters[2].hp, 2);
        assert!(orchard.monsters[2].knockback > 180.);
        assert!(!orchard
            .events
            .iter()
            .any(|e| matches!(e, VfxEvent::ShieldRebound(_))));
    }

    #[test]
    fn home_run_refills_melon_for_a_high_rebound_and_restarts_parked_pumpkin() {
        let mut orchard = Orchard::default();
        let x = SLOTS[1];
        orchard.slots[1] = Some(SlotItem::Chime { cooldown: 0 });
        orchard.spawn(MonsterKind::IronBoar, x + 22.);
        let kind = FruitKind::BounceMelon;
        let pos = Point::new(
            x - 20.,
            ground(x - 20.) - kind.radius() * Stage::Giant.scale(),
        );
        let melon = body_at(&mut orchard, kind, Stage::Giant, pos, Point::new(0., 10.));
        orchard.bodies.push(melon);
        assert!(orchard.trigger_home_run(pos));
        let mut melon = orchard.bodies.pop().unwrap();
        for _ in 0..40 {
            assert!(orchard.move_body(&mut melon));
            if melon.bounces > 0 {
                break;
            }
        }
        assert_eq!(orchard.monsters[0].hp, 5);
        assert!(orchard.monsters[0].knockback > 80.);
        assert_eq!(
            melon.velocity.y,
            -HOME_RUN_MELON_IMPACT * MELON_MONSTER_RESTITUTION
        );
        orchard.monsters.clear();
        for _ in 0..700 {
            if !orchard.move_body(&mut melon) {
                break;
            }
        }
        assert!(melon.bounces > 3);
        assert!(orchard
            .events
            .iter()
            .any(|e| matches!(e, VfxEvent::ChimeRing(_))));

        let mut orchard = park_pumpkin();
        let parked = orchard.barricades[0].body();
        let (id, pos) = (parked.id, parked.pos);
        assert!(orchard.trigger_home_run(Point::new(pos.x, pos.y + 48.)));
        assert!(orchard.barricades.is_empty());
        let pumpkin = &orchard.bodies[0];
        assert_eq!(pumpkin.id, id);
        assert!(pumpkin.rolling && pumpkin.home_run && pumpkin.velocity.x == HOME_RUN_SPEED);
    }

    #[test]
    fn rolling_pumpkins_regain_speed_between_hits_and_lose_speed_by_monster_weight() {
        for (stage, light, heavy) in [(Stage::Mature, 0.88, 0.45), (Stage::Giant, 0.92, 0.60)] {
            let mut orchard = Orchard::default();
            let kind = FruitKind::BowlingPumpkin;
            let mut pumpkin = body_at(
                &mut orchard,
                kind,
                stage,
                Point::new(100., ground(100.) - kind.radius() * stage.scale()),
                Point::new(80., 0.),
            );
            pumpkin.rolling = true;
            let acceleration = GRAVITY * SLOPE / (1. + SLOPE * SLOPE) * DT;
            for kind in [
                MonsterKind::GreenSlime,
                MonsterKind::GreenSlime,
                MonsterKind::GreenSlime,
                MonsterKind::GreenSlime,
                MonsterKind::GreenSlime,
                MonsterKind::IronBoar,
                MonsterKind::Shield,
            ] {
                let offset = pumpkin.radius() + kind.radius() - 6.;
                orchard.spawn(kind, pumpkin.pos.x + offset);
                let next = (pumpkin.velocity.x + acceleration).min(pumpkin.roll_speed_cap());
                assert!(orchard.move_body(&mut pumpkin));
                let retention = match kind {
                    MonsterKind::GreenSlime => light,
                    MonsterKind::Shield => 0.08,
                    _ => heavy,
                };
                assert!(
                    (pumpkin.velocity.x - next * retention).abs() < 0.001,
                    "{stage:?} vs {kind:?}"
                );
                let enemy = orchard.monsters.last().unwrap();
                assert!(enemy.hp < kind.hp() && enemy.knockback > 0.);
                if kind == MonsterKind::GreenSlime {
                    assert!(pumpkin.velocity.x > ROLL_THRESHOLD);
                    let before = pumpkin.velocity.x;
                    for _ in 0..16 {
                        assert!(orchard.move_body(&mut pumpkin));
                    }
                    assert!(pumpkin.velocity.x > before);
                }
            }
            assert!(pumpkin.velocity.x < ROLL_THRESHOLD);
        }

        let mut orchard = Orchard::default();
        let mut pumpkin = body_at(
            &mut orchard,
            FruitKind::BowlingPumpkin,
            Stage::Giant,
            Point::new(80., ground(80.) - 11.4),
            Point::new(100., 0.),
        );
        pumpkin.rolling = true;
        for _ in 0..160 {
            assert!(orchard.move_body(&mut pumpkin));
        }
        assert_eq!(pumpkin.velocity.x, pumpkin.roll_speed_cap());
    }

    #[test]
    fn shield_top_rebounds_at_one_and_a_half_times_speed_and_side_takes_damage() {
        let mut orchard = Orchard::default();
        orchard.spawn(MonsterKind::Shield, 179.);
        let center = orchard.monsters[0].pos;
        let kind = FruitKind::RedApple;
        let stage = Stage::Mature;
        let radius = kind.radius() * stage.scale();
        let mut top = body_at(
            &mut orchard,
            kind,
            stage,
            Point::new(center.x, center.y - 8. - radius - 1.),
            Point::new(0., 320.),
        );
        assert!(orchard.move_body(&mut top));
        assert_eq!(orchard.monsters[0].hp, 6);
        assert!((top.velocity.y + 1.5 * (320. + GRAVITY * DT)).abs() < 0.001);
        let mut side = body_at(
            &mut orchard,
            kind,
            stage,
            Point::new(center.x + 12., center.y + 2.),
            Point::new(-27., 320.),
        );
        assert!(!orchard.move_body(&mut side));
        assert_eq!(orchard.monsters[0].hp, 4);
        assert!(orchard.monsters[0].knockback > 0.);
    }
}
