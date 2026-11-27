use std::collections::VecDeque;

pub const DT: f32 = 1.0 / 120.0;
pub const TRELLIS_Y: f32 = 62.0;
pub const SLOPE: f32 = 0.194;
pub const SLOTS: [f32; 6] = [110., 165., 220., 275., 330., 385.];
pub const MOUNTS: [f32; 5] = [137.5, 192.5, 247.5, 302.5, 357.5];
const GRAVITY: f32 = 610.0;
const SPAWN_X: f32 = 486.0;
const DOOR_X: f32 = 51.0;
const BUD_TICKS: u32 = 36;

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

#[derive(Debug)]
pub struct Body {
    pub id: u32,
    pub kind: FruitKind,
    pub stage: Stage,
    pub pos: Point,
    pub velocity: Point,
    pub bounces: u8,
    pub rolling: bool,
    hit: Vec<u32>,
    trellis_contacts: u16,
}

impl Body {
    fn radius(&self) -> f32 {
        self.kind.radius() * self.stage.scale()
    }

    fn push(&self) -> f32 {
        let mass = match self.kind {
            FruitKind::RedApple => 1.0,
            FruitKind::BounceMelon => 1.4,
            FruitKind::BowlingPumpkin => 4.0,
        };
        mass * self.stage.scale() * 26.0
    }

    fn damage(&self, kind: MonsterKind) -> u8 {
        match (self.kind, self.stage, kind) {
            (FruitKind::RedApple, Stage::Giant, MonsterKind::IronBoar) => 6,
            (FruitKind::RedApple, Stage::Giant, _) => 4,
            (FruitKind::RedApple, Stage::Mature, _) => 2,
            (FruitKind::RedApple, Stage::Small, _) => 1,
            (FruitKind::BounceMelon, Stage::Giant, _) => 3,
            (FruitKind::BounceMelon, _, _) => 2,
            (FruitKind::BowlingPumpkin, Stage::Giant, _) => 7,
            (FruitKind::BowlingPumpkin, Stage::Mature, _) => 5,
            (FruitKind::BowlingPumpkin, Stage::Small, _) => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterKind {
    GreenSlime,
    IronBoar,
}

impl MonsterKind {
    pub fn hp(self) -> u8 {
        match self {
            Self::GreenSlime => 3,
            Self::IronBoar => 9,
        }
    }

    pub fn radius(self) -> f32 {
        match self {
            Self::GreenSlime => 7.,
            Self::IronBoar => 10.,
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
    FruitCut(Point, FruitKind),
    FruitShatter(Point, FruitKind),
    MonsterHit(Point, u8, bool),
    MonsterDie(Point),
    MelonBounce(Point),
    TrellisKnock(Point),
    ChimeRing(Point),
    PumpkinBowl(Point),
    CabinHit(Point),
}

pub struct Orchard {
    pub phase: Phase,
    pub wave: usize,
    pub cabin_hp: u8,
    pub slots: [Option<Fruit>; 6],
    pub reserve: Option<Fruit>,
    pub chimes: [bool; 5],
    pub bodies: Vec<Body>,
    pub monsters: Vec<Monster>,
    pub events: VecDeque<VfxEvent>,
    tick: u32,
    spawned: u32,
    next_id: u32,
}

impl Default for Orchard {
    fn default() -> Self {
        let mut slots = [None; 6];
        slots[0] = Some(Fruit::new(FruitKind::RedApple));
        slots[1] = slots[0];
        Self {
            phase: Phase::Wave,
            wave: 1,
            cabin_hp: 5,
            slots,
            reserve: None,
            chimes: [false; 5],
            bodies: Vec::new(),
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
        self.slots = [
            RedApple,
            BounceMelon,
            BowlingPumpkin,
            BounceMelon,
            RedApple,
            BowlingPumpkin,
        ]
        .map(|kind| {
            Some(Fruit {
                kind,
                age: kind.growth_ticks(),
            })
        });
        self.chimes[0] = true;
        self.chimes[2] = true;
    }

    fn active(&self) -> bool {
        matches!(self.phase, Phase::Wave | Phase::Sandbox)
    }

    pub fn swap_plants(&mut self, from: usize, to: usize) -> bool {
        if !matches!(self.phase, Phase::Intermission | Phase::Sandbox) || from > 6 || to > 6 {
            return false;
        }
        match (from, to) {
            (6, 6) => return false,
            (6, slot) | (slot, 6) if self.reserve.is_some() && self.slots[slot].is_some() => {
                std::mem::swap(&mut self.reserve, &mut self.slots[slot]);
            }
            (6, _) | (_, 6) => return false,
            _ => self.slots.swap(from, to),
        }
        true
    }

    pub fn move_chime(&mut self, from: usize, to: usize) -> bool {
        if !matches!(self.phase, Phase::Intermission | Phase::Sandbox)
            || from >= MOUNTS.len()
            || to >= MOUNTS.len()
            || !self.chimes[from]
        {
            return false;
        }
        self.chimes.swap(from, to);
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
        self.active() && self.drop_fruit(slot)
    }

    fn drop_fruit(&mut self, slot: usize) -> bool {
        let Some(Some(fruit)) = self.slots.get(slot).copied() else {
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
            velocity: Point::default(),
            bounces: 0,
            rolling: false,
            hit: Vec::new(),
            trellis_contacts: 0,
        });
        self.slots[slot] = Some(Fruit::new(fruit.kind));
        self.events.push_back(VfxEvent::FruitCut(pos, fruit.kind));
        true
    }

    fn ring(&mut self, mount: usize) {
        self.events
            .push_back(VfxEvent::ChimeRing(Point::new(MOUNTS[mount], TRELLIS_Y)));
        for slot in [mount, mount + 1] {
            if let Some(fruit) = &mut self.slots[slot] {
                if fruit.stage() == Stage::Giant {
                    self.drop_fruit(slot);
                } else {
                    fruit.age = fruit.kind.growth_ticks();
                }
            }
        }
    }

    fn rebound(&mut self, body: &mut Body) -> bool {
        if body.bounces == 3 {
            return self.shatter(body);
        }
        body.bounces += 1;
        // The 263-pixel rise clears the trellis from the far right edge.
        body.velocity.y = -568.;
        let mut closest = f32::INFINITY;
        let mut target = body.pos.x;
        for (x, mounted) in MOUNTS.into_iter().zip(self.chimes) {
            let distance = (x - body.pos.x).abs();
            if mounted && distance <= 35.0 && distance < closest {
                closest = distance;
                target = x;
            }
        }
        // Adjacent chimes catch the rebound. Otherwise preserve a mild downhill deflection.
        if closest.is_infinite() {
            target += if body.pos.x < 460.0 { 7.0 } else { -7.0 };
        }
        let dy = (body.pos.y - TRELLIS_Y).max(0.);
        let travel = (2.0 * dy / (568.0 + (568.0_f32.powi(2) - 2.0 * GRAVITY * dy).max(0.).sqrt()))
            .max(0.10);
        body.velocity.x = ((target - body.pos.x) / travel).clamp(-106., 106.);
        body.hit.clear();
        body.trellis_contacts = 0;
        self.events.push_back(VfxEvent::MelonBounce(body.pos));
        true
    }

    fn shatter(&mut self, body: &Body) -> bool {
        self.events
            .push_back(VfxEvent::FruitShatter(body.pos, body.kind));
        false
    }

    fn move_body(&mut self, body: &mut Body) -> bool {
        let old = body.pos;
        let radius = body.radius();
        if body.rolling {
            body.velocity.x += GRAVITY * SLOPE / (1. + SLOPE * SLOPE) * DT;
            body.pos.x += body.velocity.x * DT;
            body.pos.y = ground(body.pos.x) - radius;
        } else {
            if body.kind == FruitKind::BounceMelon && body.pos.y <= TRELLIS_Y - 10.0 {
                body.velocity.x *= 0.86;
            }
            body.velocity.y += GRAVITY * DT;
            body.pos.x += body.velocity.x * DT;
            body.pos.y += body.velocity.y * DT;
        }
        if body.pos.x > 503. || body.pos.x < -24. {
            return false;
        }

        if body.kind == FruitKind::BounceMelon && body.velocity.y < 0. {
            for (mount, x) in MOUNTS.into_iter().enumerate() {
                let mask = 1 << (6 + mount);
                if self.chimes[mount]
                    && body.trellis_contacts & mask == 0
                    && Point::new(x, TRELLIS_Y).within_segment(old, body.pos, radius + 3.)
                {
                    body.trellis_contacts |= mask;
                    self.ring(mount);
                }
            }
            for (slot, x) in SLOTS.into_iter().enumerate() {
                let mask = 1 << slot;
                if let Some(fruit) = self.slots[slot].filter(|fruit| fruit.hanging()) {
                    let pos = Point::new(x, TRELLIS_Y);
                    if body.trellis_contacts & mask == 0
                        && pos.within_segment(
                            old,
                            body.pos,
                            radius + fruit.kind.radius() * fruit.stage().scale(),
                        )
                    {
                        body.trellis_contacts |= mask;
                        self.drop_fruit(slot);
                        self.events.push_back(VfxEvent::TrellisKnock(pos));
                    }
                }
            }
        }

        for monster in &mut self.monsters {
            if monster.hp == 0
                || body.hit.contains(&monster.id)
                || (body.kind == FruitKind::BounceMelon && body.velocity.y < 0.)
                || !monster
                    .pos
                    .within_segment(old, body.pos, radius + monster.kind.radius())
            {
                continue;
            }
            let damage = body.damage(monster.kind);
            monster.hp = monster.hp.saturating_sub(damage);
            monster.knockback += body.push()
                / if monster.kind == MonsterKind::IronBoar {
                    2.1
                } else {
                    1.0
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
            match body.kind {
                FruitKind::RedApple => return self.shatter(body),
                FruitKind::BounceMelon => return self.rebound(body),
                FruitKind::BowlingPumpkin => self.events.push_back(VfxEvent::PumpkinBowl(body.pos)),
            }
        }

        if !body.rolling && body.pos.y + radius >= ground(body.pos.x) {
            body.pos.y = ground(body.pos.x) - radius;
            match body.kind {
                FruitKind::RedApple => return self.shatter(body),
                FruitKind::BounceMelon => return self.rebound(body),
                FruitKind::BowlingPumpkin => {
                    body.rolling = true;
                    body.velocity.x = (body.velocity.y * SLOPE / (1. + SLOPE * SLOPE)).max(10.);
                    body.velocity.y = body.velocity.x * SLOPE;
                    self.events.push_back(VfxEvent::PumpkinBowl(body.pos));
                }
            }
        }
        true
    }

    fn spawn(&mut self, kind: MonsterKind) {
        let id = self.id();
        self.monsters.push(Monster {
            id,
            kind,
            hp: kind.hp(),
            pos: Point::new(SPAWN_X, ground(SPAWN_X) - kind.radius()),
            knockback: 0.,
            stride: 0,
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

    fn grant(&mut self, kind: FruitKind) {
        if let Some(slot) = self.slots.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(Fruit::new(kind));
        } else {
            self.reserve = Some(Fruit::new(kind));
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
        match self.wave {
            1 => {
                self.grant(BounceMelon);
                self.chimes[2] = true;
            }
            2 => {
                self.grant(BowlingPumpkin);
                self.grant(RedApple);
            }
            3 => {
                self.grant(BounceMelon);
                self.grant(BowlingPumpkin);
                if let Some(chime) = self.chimes.iter_mut().find(|chime| !**chime) {
                    *chime = true;
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn step(&mut self) {
        if !self.active() {
            return;
        }
        let (count, interval, duration) = self.wave_rules();
        let sandbox = self.phase == Phase::Sandbox;
        if sandbox && self.tick % 128 == 0
            || !sandbox && self.spawned < count && self.tick == 120 + self.spawned * interval
        {
            let boar = (sandbox || self.wave >= 3) && self.spawned % 3 == 1;
            self.spawn(if boar {
                MonsterKind::IronBoar
            } else {
                MonsterKind::GreenSlime
            });
            self.spawned += 1;
        }
        self.tick += 1;

        for slot in 0..self.slots.len() {
            if let Some(fruit) = &mut self.slots[slot] {
                fruit.age += 1;
                if fruit.age > fruit.kind.growth_ticks() + 420 {
                    self.drop_fruit(slot);
                }
            }
        }

        for monster in &mut self.monsters {
            let slime = monster.kind == MonsterKind::GreenSlime;
            monster.stride += 1;
            monster.pos.x += (monster.knockback - if slime { 29. } else { 20. }) * DT;
            monster.pos.x = monster.pos.x.min(SPAWN_X + 10.);
            monster.knockback *= 0.976;
            let arc = (monster.stride % 78) as f32 / 78.0;
            let lift = if slime { 22. * arc * (1. - arc) } else { 0. };
            monster.pos.y = ground(monster.pos.x) - monster.kind.radius() - lift;
        }

        let mut previous = std::mem::take(&mut self.bodies);
        previous.retain_mut(|body| self.move_body(body));
        self.bodies.append(&mut previous);

        self.monsters.retain(|monster| {
            if monster.hp == 0 {
                return false;
            }
            if monster.pos.x <= DOOR_X {
                self.cabin_hp = self.cabin_hp.saturating_sub(1);
                self.events
                    .push_back(VfxEvent::CabinHit(Point::new(DOOR_X, ground(DOOR_X) - 10.)));
                return false;
            }
            true
        });

        if self.cabin_hp == 0 && !sandbox {
            self.phase = Phase::Defeat;
        }
        if sandbox && self.cabin_hp == 0 {
            self.cabin_hp = 5;
        }
        if self.phase == Phase::Wave && self.tick >= duration && self.monsters.is_empty() {
            self.end_wave();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hanging(kind: FruitKind) -> Option<Fruit> {
        Some(Fruit {
            kind,
            age: BUD_TICKS,
        })
    }

    #[test]
    fn pumpkin_accelerates_downhill() {
        let mut orchard = Orchard::default();
        orchard.slots[0] = hanging(FruitKind::BowlingPumpkin);
        assert!(orchard.cut_fruit(0));
        while !orchard.bodies[0].rolling {
            orchard.step();
        }
        let (x, vx) = (orchard.bodies[0].pos.x, orchard.bodies[0].velocity.x);
        for _ in 0..35 {
            orchard.step();
        }
        assert!(orchard.bodies[0].pos.x > x && orchard.bodies[0].velocity.x > vx);
    }

    #[test]
    fn melon_physically_knocks_a_fruit_off_the_trellis() {
        let mut orchard = Orchard::default();
        orchard.slots[0] = hanging(FruitKind::BounceMelon);
        orchard.cut_fruit(0);
        let body = orchard.bodies[0].id;
        for _ in 0..125 {
            orchard.step();
        }
        assert!(orchard
            .bodies
            .iter()
            .any(|fruit| fruit.id == body && fruit.pos.y <= TRELLIS_Y));
        assert!(orchard
            .events
            .iter()
            .any(|event| matches!(event, VfxEvent::TrellisKnock(_))));
        assert!(orchard.bodies.iter().any(|fruit| fruit.id != body));
    }

    #[test]
    fn rising_melon_rings_chime_to_ripen_and_drop() {
        let mut orchard = Orchard::default();
        orchard.chimes[0] = true;
        orchard.slots[1].as_mut().unwrap().age = FruitKind::RedApple.growth_ticks();
        orchard.slots[2] = hanging(FruitKind::BounceMelon);
        orchard.cut_fruit(2);
        orchard.bodies[0].pos = Point::new(MOUNTS[0], TRELLIS_Y + 7.);
        orchard.bodies[0].velocity.y = -568.;
        orchard.step();
        assert_eq!(orchard.slots[0].unwrap().stage(), Stage::Giant);
        assert_eq!(orchard.slots[1].unwrap().stage(), Stage::Small);
        assert!(orchard
            .bodies
            .iter()
            .any(|body| body.kind == FruitKind::RedApple));
        assert!(orchard
            .events
            .iter()
            .any(|event| matches!(event, VfxEvent::ChimeRing(_))));
    }

    #[test]
    fn max_slope_rebound_clears_trellis() {
        let mut orchard = Orchard::default();
        orchard.slots[0] = hanging(FruitKind::BounceMelon);
        orchard.cut_fruit(0);
        let mut body = orchard.bodies.pop().unwrap();
        body.pos = Point::new(492., ground(492.) - body.radius());
        assert!(orchard.rebound(&mut body));
        for _ in 0..110 {
            assert!(orchard.move_body(&mut body));
        }
        assert!(body.pos.y < TRELLIS_Y);
    }
}
