use std::collections::VecDeque;

pub const DT: f32 = 1.0 / 120.0;
pub const TRELLIS_Y: f32 = 62.0;
pub const SLOPE: f32 = 0.194;
pub const SLOTS: [f32; 6] = [122., 175., 228., 281., 334., 387.];
const GRAVITY: f32 = 610.0;
const SPAWN_X: f32 = 486.0;
const CHIEF_X: f32 = 62.0;
const DOORWAY_REACH_RIGHT: f32 = 115.;
const MAX_HP: u8 = 5;
const BUD_TICKS: u32 = 36;
const CHIME_COOLDOWN: u32 = 120;
pub const HOME_RUN_COOLDOWN: u32 = 7 * 120;
const DIGESTION_COOLDOWN: u32 = 15 * 120;
const HOME_RUN_IFRAMES: u32 = 18;
const HOME_RUN_RADIUS: f32 = 48.;
const HOME_RUN_SPEED: f32 = 440.;
const HOME_RUN_APPROACH_SPEED: f32 = 310.;
const HOME_RUN_MELON_ROLL_OUT: f32 = 120.;
const HOME_RUN_MELON_IMPACT: f32 = 770.;
const MELON_RESTITUTION: f32 = 0.64;
const MELON_MIN_REBOUND: f32 = 115.;
const ROT_TICKS: u32 = 840;
const JUICE_TICKS: u32 = 720;
const JUICE_RADIUS: f32 = 28.0;
const ROLL_THRESHOLD: f32 = 45.0;
const SWIPE_DROP_SPEED: f32 = 90.0;
const MAX_CUT_VX: f32 = 29.24277; // 90 * tan(18 degrees)
const SANDBOX_REINFORCEMENT_DELAY: u32 = 600;

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
            Self::RedApple => 240,
            Self::BounceMelon => 384,
            Self::BowlingPumpkin => 540,
        }
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
        } else if self.age * 2 >= growth {
            Stage::Mature
        } else {
            Stage::Small
        }
    }

    pub fn hanging(self) -> bool {
        self.age >= BUD_TICKS
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SlotItem {
    Fruit(Fruit),
    Chime { cooldown: u32 },
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
    pub rolling: bool,
    pub home_run: bool,
    home_run_lane_distance: f32,
    hit: Vec<u32>,
    trellis_contacts: u8,
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
            (_, Stage::Small, _) => 1,
            (FruitKind::RedApple, Stage::Giant, MonsterKind::IronBoar) => 6,
            (FruitKind::RedApple, Stage::Giant, _) => 4,
            (FruitKind::RedApple, Stage::Mature, _) => 2,
            (FruitKind::BounceMelon, Stage::Giant, _) => 2,
            (FruitKind::BounceMelon, Stage::Mature, _) => 1,
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
    Shield,
}

impl MonsterKind {
    pub fn hp(self) -> u8 {
        match self {
            Self::GreenSlime => 3,
            Self::IronBoar => 9,
            Self::Shield => 6,
        }
    }

    pub fn radius(self) -> f32 {
        match self {
            Self::GreenSlime => 7.,
            Self::IronBoar => 10.,
            Self::Shield => 9.,
        }
    }

    fn speed(self) -> f32 {
        match self {
            Self::GreenSlime => 29.,
            Self::IronBoar => 20.,
            Self::Shield => 24.,
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
    HomeRunDash { from: Point, target: Point },
    ShieldRebound(Point),
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
    tick: u32,
    spawned: u32,
    next_id: u32,
}

impl Default for Orchard {
    fn default() -> Self {
        let mut slots = [None; SLOTS.len()];
        slots[..2].fill(Some(FruitKind::RedApple.into()));
        Self {
            phase: Phase::Wave,
            wave: 1,
            protagonist: Protagonist {
                hp: MAX_HP,
                home_run_cooldown: 0,
                digestion_cooldown: 0,
                invulnerability_ticks: 0,
                hit_flash: 0,
            },
            unlocked_slots: 2,
            slots,
            reserve: Vec::new(),
            bodies: Vec::new(),
            barricades: Vec::new(),
            juice_zones: Vec::new(),
            monsters: Vec::new(),
            events: VecDeque::new(),
            tick: 0,
            spawned: 0,
            next_id: 1,
        }
    }
}

impl Orchard {
    pub fn sandbox(&mut self) {
        use FruitKind::*;
        *self = Self::default();
        self.phase = Phase::Sandbox;
        self.unlocked_slots = SLOTS.len();
        let giant = |kind: FruitKind| {
            SlotItem::Fruit(Fruit {
                kind,
                age: kind.growth_ticks(),
            })
        };
        self.slots = [
            giant(RedApple),
            giant(BounceMelon),
            giant(BowlingPumpkin),
            SlotItem::Chime { cooldown: 0 },
            giant(RedApple),
            giant(BowlingPumpkin),
        ]
        .map(Some);
        self.reserve.push(giant(BounceMelon));
        // The opening shield is positioned so a Giant pumpkin in the last slot can park on impact.
        for (kind, x) in [
            (MonsterKind::GreenSlime, 275.),
            (MonsterKind::IronBoar, 350.),
            (MonsterKind::Shield, 423.),
        ] {
            self.spawn(kind, x);
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
        self.spawned = 0;
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
        let pos = Point::new(SLOTS[slot], TRELLIS_Y);
        let id = self.id();
        self.bodies.push(Body {
            id,
            pos,
            kind: fruit.kind,
            stage: fruit.stage(),
            velocity: Point::new(vx, if vx == 0. { 0. } else { SWIPE_DROP_SPEED }),
            bounces: 0,
            rolling: false,
            home_run: false,
            home_run_lane_distance: 0.,
            hit: Vec::new(),
            trellis_contacts: 0,
        });
        self.slots[slot] = Some(fruit.kind.into());
        self.events
            .push_back(VfxEvent::FruitCut(pos, fruit.kind, vx));
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
                        && fruit.hanging()
                        && pos.within_segment(
                            old,
                            body.pos,
                            body.radius() + fruit.kind.radius() * fruit.stage().scale(),
                        ) =>
                {
                    body.trellis_contacts |= mask;
                    self.drop_fruit(slot, 0.);
                    self.events.push_back(VfxEvent::TrellisKnock(pos));
                }
                _ => {}
            }
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
        self.events.push_back(VfxEvent::MelonBounce(body.pos));
        true
    }

    fn launch_melon_rebound(&self, body: &mut Body, speed: f32) {
        body.bounces += 1;
        body.home_run = false;
        body.velocity.y = -speed;
        body.velocity.x = if body.pos.x < 460. { 13. } else { -13. };
        let rise = body.pos.y - TRELLIS_Y;
        let effective_speed = speed - GRAVITY * DT / 2.;
        let discriminant = effective_speed * effective_speed - 2. * GRAVITY * rise;
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
    }

    fn shatter(&mut self, body: &Body) -> bool {
        self.events
            .push_back(VfxEvent::FruitShatter(body.pos, body.kind, body.stage));
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
                + traction * GRAVITY * SLOPE / (1. + SLOPE * SLOPE) * DT)
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
            body.velocity.y += GRAVITY * DT;
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

        if let Some(event) = self.protagonist.interact(body, old) {
            let bonked = matches!(event, VfxEvent::ChiefBonk(_));
            self.events.push_back(event);
            return bonked && self.shatter(body);
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
            monster.hp = monster.hp.saturating_sub(damage);
            monster.knockback += body.push()
                / if monster.kind == MonsterKind::IronBoar {
                    2.1
                } else {
                    1.
                };
            body.hit.push(monster.id);
            self.events.push_back(VfxEvent::MonsterHit(
                monster.pos,
                damage,
                body.stage == Stage::Giant,
            ));
            if monster.hp == 0 {
                self.events.push_back(VfxEvent::MonsterDie(monster.pos));
            }
            if body.stage == Stage::Small {
                return self.shatter(body);
            }
            if body.home_run
                && monster.kind == MonsterKind::GreenSlime
                && body.kind != FruitKind::BowlingPumpkin
            {
                body.velocity.x *= 0.98;
                body.velocity.y *= 0.98;
                continue;
            }
            match body.kind {
                FruitKind::RedApple => return self.shatter(body),
                FruitKind::BounceMelon => return self.rebound(body),
                FruitKind::BowlingPumpkin => {
                    self.events.push_back(VfxEvent::PumpkinBowl(body.pos));
                    if body.rolling || body.home_run {
                        let retained_speed = match (body.stage, monster.kind) {
                            (Stage::Giant, MonsterKind::GreenSlime) => 0.92,
                            (_, MonsterKind::GreenSlime) => 0.88,
                            (Stage::Giant, _) => 0.60,
                            (_, _) => 0.45,
                        };
                        body.velocity.x *= retained_speed;
                        body.velocity.y *= retained_speed;
                        if body.velocity.x < ROLL_THRESHOLD {
                            break;
                        }
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
        self.monsters.push(Monster {
            id,
            kind,
            hp: kind.hp(),
            pos: Point::new(x, ground(x) - kind.radius()),
            knockback: 0.,
            stride: 0,
            bite_cooldown: 0,
        });
    }

    fn wave_rules(&self) -> (u32, u32, u32) {
        match self.wave {
            1 => (8, 285, 2880),
            2 => (12, 218, 3060),
            3 => (14, 195, 3220),
            _ => (21, 146, 3700),
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
        use FruitKind::*;
        if self.wave == 4 {
            self.phase = Phase::Victory;
            return;
        }
        self.phase = Phase::Intermission;
        self.bodies.clear();
        self.barricades.clear();
        self.juice_zones.clear();
        match self.wave {
            1 => {
                self.unlocked_slots = 3;
                self.grant(BounceMelon.into());
            }
            2 => {
                self.unlocked_slots = 5;
                self.grant(SlotItem::Chime { cooldown: 0 });
                self.grant(BowlingPumpkin.into());
            }
            3 => {
                self.unlocked_slots = 6;
                self.grant(BowlingPumpkin.into());
                self.grant(BounceMelon.into());
            }
            _ => unreachable!(),
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
            let mut x = (old_x + (monster.knockback * slide - monster.kind.speed()) * DT)
                .min(SPAWN_X + 10.);
            monster.knockback *= if slippery { 0.993 } else { 0.976 };
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
        }
        self.barricades.retain(|barrier| barrier.hits_left > 0);
    }

    pub fn step(&mut self) {
        if !self.active() {
            return;
        }
        let (count, interval, duration) = self.wave_rules();
        let sandbox = self.phase == Phase::Sandbox;
        if sandbox
            && self.tick >= SANDBOX_REINFORCEMENT_DELAY
            && (self.tick - SANDBOX_REINFORCEMENT_DELAY) % 128 == 0
            || !sandbox && self.spawned < count && self.tick == 120 + self.spawned * interval
        {
            let kind = match self.spawned % 3 {
                1 if sandbox || self.wave >= 3 => MonsterKind::IronBoar,
                2 if sandbox || self.wave >= 3 => MonsterKind::Shield,
                _ => MonsterKind::GreenSlime,
            };
            self.spawn(kind, SPAWN_X);
            self.spawned += 1;
        }
        self.tick += 1;
        self.protagonist.home_run_cooldown = self.protagonist.home_run_cooldown.saturating_sub(1);
        self.protagonist.digestion_cooldown = self.protagonist.digestion_cooldown.saturating_sub(1);
        self.protagonist.hit_flash = self.protagonist.hit_flash.saturating_sub(1);

        for slot in 0..self.unlocked_slots {
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
        for mut body in std::mem::take(&mut self.bodies) {
            if self.move_body(&mut body) {
                if body.rolling && body.velocity.x < ROLL_THRESHOLD {
                    body.rolling = false;
                    body.home_run = false;
                    body.velocity = Point::default();
                    self.barricades.push(Barricade {
                        body,
                        age: 0,
                        hits_left: 3,
                    });
                } else {
                    self.bodies.push(body);
                }
            }
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
                self.protagonist.hit_flash = 24;
                self.events
                    .push_back(VfxEvent::ChiefHit(self.protagonist.position()));
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
        if self.phase == Phase::Wave && self.tick >= duration && self.monsters.is_empty() {
            self.end_wave();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_fruit(orchard: &mut Orchard, slot: usize, kind: FruitKind, stage: Stage) {
        let age = match stage {
            Stage::Small => BUD_TICKS,
            Stage::Mature => kind.growth_ticks() / 2,
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
        let mut orchard = Orchard::default();
        assert_eq!(orchard.unlocked_slots, 2);
        assert!(orchard.slots[2..].iter().all(Option::is_none));
        orchard.end_wave();
        assert_eq!(orchard.unlocked_slots, 3);
        assert!(!orchard.swap_plants(0, 3));
        assert!(orchard.swap_plants(0, 6));
        assert!(orchard.swap_plants(2, 7));
        assert_eq!(orchard.reserve.len(), 2);
        assert!(!orchard.swap_plants(7, 5));
        assert!(orchard.swap_plants(7, 0));
        assert!(orchard.swap_plants(6, 2));
        assert!(orchard.reserve.is_empty());
        assert!(orchard.next_wave());
        orchard.end_wave();
        assert_eq!(orchard.unlocked_slots, 5);
        assert!(matches!(
            orchard.slots[3],
            Some(SlotItem::Chime { cooldown: 0 })
        ));
        assert!(orchard.next_wave());
        orchard.end_wave();
        assert_eq!(orchard.unlocked_slots, 6);
        assert_eq!(
            orchard.slots.iter().flatten().count() + orchard.reserve.len(),
            7
        );
        assert!(matches!(
            orchard.reserve[0],
            SlotItem::Fruit(Fruit {
                kind: FruitKind::BounceMelon,
                ..
            })
        ));
    }

    #[test]
    fn click_is_vertical_and_swipe_is_capped_at_18_degrees() {
        let mut orchard = Orchard::default();
        for slot in 0..2 {
            set_fruit(&mut orchard, slot, FruitKind::RedApple, Stage::Mature);
        }
        assert!(orchard.cut_fruit(0));
        assert!(!orchard.cut_fruit_angled(1, f32::NAN));
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
            orchard.spawn(MonsterKind::GreenSlime, 180.);
            assert!(!orchard.move_body(&mut body));
            assert_eq!(orchard.monsters[0].hp, 2);
        }
    }

    #[test]
    fn vertically_dropped_melons_decay_to_two_rebounds_across_the_trellis() {
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
                for _ in 0..360 {
                    let before = body.bounces;
                    let impact = body.velocity.y + GRAVITY * DT;
                    if !orchard.move_body(&mut body) {
                        assert!(impact * MELON_RESTITUTION < MELON_MIN_REBOUND);
                        break;
                    }
                    if body.bounces > before {
                        assert!((body.velocity.y + impact * 0.64).abs() < 0.001);
                    }
                    if body.bounces != 0 {
                        let height = &mut apex[(body.bounces - 1) as usize];
                        *height = height.min(body.pos.y);
                    }
                }
                assert!(floor - apex[0] > 30.);
                assert!((0.33..0.46).contains(&((floor - apex[1]) / (floor - apex[0]))));
                assert_eq!(body.bounces, 2);
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
        assert_eq!(orchard.barricades[0].hits_left, 3);
        assert_eq!(orchard.monsters[0].hp, 4);
        orchard
    }

    #[test]
    fn parked_pumpkin_blocks_and_breaks_on_third_bite() {
        let mut orchard = park_pumpkin();
        orchard.tick = 5000;
        orchard.spawned = 8;
        let edge = orchard.barricades[0].body.pos.x + orchard.barricades[0].body.radius() + 10.;
        for remaining in [2, 1] {
            for _ in 0..300 {
                orchard.step();
                if orchard.barricades[0].hits_left == remaining {
                    break;
                }
            }
            assert_eq!(orchard.barricades[0].hits_left, remaining);
            assert_eq!(orchard.monsters[0].pos.x, edge);
        }
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
            3
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
                assert_eq!(orchard.barricades[0].hits_left, 3);
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
        orchard.spawned = 8;
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
        let mut orchard = Orchard::default();
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
        orchard.spawned = 100;
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
        let mut orchard = Orchard::default();
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
        orchard.spawned = 100;
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
        assert_eq!(melon.velocity.y, -HOME_RUN_MELON_IMPACT * 0.64);
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
                let retention = if kind == MonsterKind::GreenSlime {
                    light
                } else {
                    heavy
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
