use super::*;

impl Body {
    pub(super) fn new(id: u32, kind: FruitKind, stage: Stage, pos: Point, velocity: Point) -> Self {
        Self {
            id,
            kind,
            stage,
            pos,
            velocity,
            angled: velocity.x != 0.,
            energy_ratio: 1.,
            heavy_hits: 0,
            bounces: 0,
            combo_bounces: 0,
            rolling: false,
            home_run: false,
            home_run_lane_distance: 0.,
            passengers: Vec::new(),
            capture_bounces: 0,
            rotation: 0.,
            hit: Vec::new(),
            trellis_contacts: 0,
            trellis_grazes: 0,
        }
    }
}

pub struct Bait {
    pub pos: Point,
    pub stage: Stage,
    pub radius: f32,
    pub bites: u8,
}

pub(super) fn approach_bait(
    monster: &mut Monster,
    baits: &mut [Bait],
    events: &mut VecDeque<VfxEvent>,
) -> bool {
    if monster.hp == 0 || monster.kind == MonsterKind::WildHellhound {
        return false;
    }
    let x = monster.pos.x;
    let Some(bait) = baits
        .iter_mut()
        .filter(|b| b.bites > 0 && (x - b.pos.x).abs() <= b.radius)
        .min_by(|a, b| (a.pos.x - x).abs().total_cmp(&(b.pos.x - x).abs()))
    else {
        return false;
    };
    monster.hungry = true;
    // Stable offsets give each diner a place around the fruit.
    let seat = bait.pos.x + (monster.id % 5) as f32 * 3. - 6.;
    let dx = seat - x;
    monster.knockback *= 0.9;
    let next = x + dx.clamp(-55. * DT, 55. * DT) + monster.knockback * DT;
    monster.pos = Point::new(next, ground(next) - monster.kind.radius());
    if dx.abs() <= 4. && monster.bite_cooldown == 0 {
        monster.bite_cooldown = 90;
        bait.bites -= 1;
        events.push_back(VfxEvent::CandyBite(bait.pos));
    }
    true
}

impl Orchard {
    pub(super) fn slime_pool(&mut self, x: f32, radius: f32) {
        self.juice_zones.push(JuiceZone {
            kind: Puddle::Slime,
            left: x - radius,
            right: x + radius,
            ticks_left: JUICE_TICKS,
        });
    }

    pub(super) fn apple_rain(&mut self, shell: &Body) {
        let count = match shell.stage {
            Stage::Small => 2,
            Stage::Mature => 4 + shell.id % 2,
            Stage::Giant => 7,
        };
        for i in 0..count {
            let fan = i as f32 * 2. / (count - 1) as f32 - 1.;
            let stage = match shell.stage {
                Stage::Small => Stage::Small,
                Stage::Giant if i == 0 => Stage::Giant,
                _ => Stage::Mature,
            };
            let mut pos = shell.pos;
            pos.y -= shell.radius();
            let velocity = Point::new(220. * fan, -250. + fan.abs() * 35.);
            let mut apple = Body::new(self.id(), FruitKind::RedApple, stage, pos, velocity);
            apple.angled = true;
            self.bodies.push(apple);
        }
        self.events.push_back(VfxEvent::AppleRain(shell.pos));
    }

    fn pop_toy(&mut self, body: &mut Body) -> bool {
        let giant = body.stage == Stage::Giant;
        self.slime_pool(body.pos.x, if giant { 58. } else { 40. });
        for (i, mut monster) in body.passengers.drain(..).enumerate() {
            monster.pos.x = body.pos.x.clamp(CHIEF_X + 2., SPAWN_X);
            monster.lift = (ground(monster.pos.x) - monster.kind.radius() - body.pos.y).max(2.);
            monster.pos.y = ground(monster.pos.x) - monster.kind.radius() - monster.lift;
            monster.vertical_speed = 148. + 19. * i as f32;
            monster.knockback = 110. + 23. * i as f32;
            monster.dizzy = 105;
            monster.in_oil = false;
            self.monsters.push(monster);
        }
        self.events.push_back(VfxEvent::Pop(body.pos));
        false
    }

    fn shockwave(&mut self, body: &Body) {
        let (radius, launch, damage) = match body.stage {
            Stage::Small => (18., 80., 1),
            Stage::Mature => (47., 208., 2),
            Stage::Giant => (76., 275., 4),
        };
        self.events.push_back(VfxEvent::Quake(body.pos, radius));
        for m in &mut self.monsters {
            if m.hp == 0
                || m.kind == MonsterKind::WildHellhound
                || body.pos.distance_squared(m.pos) > radius * radius
            {
                continue;
            }
            if m.lid_hp > 0 {
                m.lid_hp = if body.stage == Stage::Giant {
                    0
                } else {
                    m.lid_hp - 1
                };
                self.events.push_back(if m.lid_hp == 0 {
                    VfxEvent::ShieldLidBreak(m.pos)
                } else {
                    VfxEvent::ShieldLidDent(m.pos, m.lid_hp)
                });
            }
            m.vertical_speed = launch;
            m.knockback += 38.;
            m.hit_by_fruit(body, damage, &mut self.stats, &mut self.events);
        }
    }

    fn toy_contact(&self, body: &Body, old: Point, m: &Monster) -> bool {
        m.hp > 0
            && m.kind != MonsterKind::WildHellhound
            && !body.hit.contains(&m.id)
            && m.pos
                .within_segment(old, body.pos, body.radius() + m.kind.radius())
    }

    fn collect_passengers(&mut self, body: &mut Body, old: Point) -> bool {
        use FruitKind::*;
        let giant = body.stage == Stage::Giant;
        if body.kind == Bubble && !body.passengers.is_empty() {
            return true;
        }
        let mut i = 0;
        while i < self.monsters.len() {
            let m = &self.monsters[i];
            if !self.toy_contact(body, old, m) {
                i += 1;
                continue;
            }
            let sticks = m.kind == MonsterKind::GreenSlime;
            let swallows = m.lid_hp == 0 && (m.kind != MonsterKind::IronBoar || giant);
            if body.kind == Katamari && !sticks {
                if giant && m.lid_hp > 0 {
                    self.events.push_back(VfxEvent::ShieldLidBreak(m.pos));
                    self.monsters[i].lid_hp = 0;
                    self.monsters[i].knockback += 140.;
                }
                return self.pop_toy(body);
            }
            if body.kind == Bubble && !swallows {
                i += 1;
                continue;
            }
            body.passengers.push(self.monsters.remove(i));
            if body.kind == Bubble {
                self.launch_melon_rebound(body, if giant { 530. } else { 490. });
                body.capture_bounces = 0;
                self.events.push_back(VfxEvent::MelonBounce(body.pos));
                return true;
            }
            if body.passengers.len() >= if giant { 5 } else { 3 } {
                return self.pop_toy(body);
            }
        }
        true
    }

    fn pinball_hit(&mut self, body: &mut Body, old: Point) {
        if let Some(i) = self
            .monsters
            .iter()
            .position(|m| self.toy_contact(body, old, m))
        {
            let m = &mut self.monsters[i];
            body.hit.push(m.id);
            m.hit_by_fruit(body, body.damage(m.kind), &mut self.stats, &mut self.events);
            body.velocity.y = -375.;
            m.knockback += 22.;
            body.bounces += 1;
            body.combo_bounces += 1;
            body.trellis_contacts = 0;
            self.melon_bounce_event(body, true);
        }
    }

    pub(super) fn move_toy(&mut self, body: &mut Body) -> bool {
        use FruitKind::*;
        let old = body.pos;
        if body.rolling {
            body.velocity.x = (body.velocity.x + 115. * DT).min(body.roll_speed_cap());
            body.velocity.y = body.velocity.x * SLOPE;
            body.pos.x += body.velocity.x * DT;
            body.rotation += body.velocity.x * DT / body.radius();
            body.pos.y = ground(body.pos.x) - body.radius();
        } else {
            let gravity = match body.kind {
                Quake => 1360.,
                Bubble | Pinball => MELON_GRAVITY,
                _ => GRAVITY,
            };
            body.velocity.y += gravity * DT;
            body.pos.x += body.velocity.x * DT;
            body.pos.y += body.velocity.y * DT;
            if body.velocity.y < 0. {
                self.hit_trellis(body, old);
            }
        }
        if body.pos.x > 503. || body.pos.x < -24. {
            if body.kind == Katamari {
                for m in &body.passengers {
                    self.stats.fruits[Katamari as usize].record(m.hp, true);
                    self.events.push_back(VfxEvent::MonsterDie(body.pos));
                }
            } else if !body.passengers.is_empty() {
                body.pos.x = body.pos.x.clamp(CHIEF_X + 2., SPAWN_X);
                return self.pop_toy(body);
            }
            return false;
        }
        if body.stage == Stage::Small {
            if let Some(bonk) = self.protagonist.interact(body, old) {
                self.events.push_back(bonk);
                return self.shatter(body);
            }
            if body.kind == Quake && body.pos.y + body.radius() >= ground(body.pos.x) {
                body.pos.y = ground(body.pos.x) - body.radius();
                self.shockwave(body);
                return self.shatter(body);
            }
            if let Some(i) = self
                .monsters
                .iter()
                .position(|m| self.toy_contact(body, old, m))
            {
                self.monsters[i].hit_by_fruit(body, 1, &mut self.stats, &mut self.events);
                return self.shatter(body);
            }
            if body.pos.y + body.radius() >= ground(body.pos.x) {
                return self.shatter(body);
            }
            return true;
        }

        if (body.kind == Katamari && body.rolling || body.kind == Bubble)
            && !self.collect_passengers(body, old)
        {
            return false;
        }
        if body.kind == Pinball {
            self.pinball_hit(body, old);
        }
        if body.rolling || body.velocity.y < 0. || body.pos.y + body.radius() < ground(body.pos.x) {
            return true;
        }
        body.pos.y = ground(body.pos.x) - body.radius();
        let giant = body.stage == Stage::Giant;
        match body.kind {
            Katamari => {
                body.rolling = true;
                body.velocity.x = (body.velocity.x + body.velocity.y * SLOPE).max(70.);
                body.velocity.y = body.velocity.x * SLOPE;
                self.events.push_back(VfxEvent::PumpkinBowl(body.pos));
            }
            Candy => {
                let radius = if giant { 112. } else { 76. };
                self.slime_pool(body.pos.x, if giant { 32. } else { 22. });
                self.baits.push(Bait {
                    pos: body.pos,
                    stage: body.stage,
                    bites: if giant { 8 } else { 5 },
                    radius,
                });
                return false;
            }
            Quake => {
                self.shockwave(body);
                if body.near_shatter() {
                    return self.shatter(body);
                }
                body.velocity.y =
                    -(body.velocity.y * 0.58).clamp(125., if giant { 370. } else { 305. });
                body.velocity.x = if giant { 94. } else { 71. };
                body.energy_ratio *= 0.55;
                body.bounces += 1;
            }
            Bubble => {
                if !body.passengers.is_empty() {
                    body.capture_bounces += 1;
                    if body.capture_bounces >= if giant { 3 } else { 2 } {
                        return self.pop_toy(body);
                    }
                    self.launch_melon_rebound(body, 490.);
                } else {
                    if body.near_shatter() {
                        return self.pop_toy(body);
                    }
                    let energy = body.energy_ratio * 0.59;
                    self.launch_melon_rebound(body, 190. + 95. * energy);
                    body.energy_ratio = energy;
                }
                self.events.push_back(VfxEvent::MelonBounce(body.pos));
            }
            Pinball => {
                if body.bounces >= if giant { 6 } else { 4 } {
                    return self.shatter(body);
                }
                body.bounces += 1;
                body.combo_bounces += 1;
                if let Some(vx) = self.chime_velocity(body.pos, 475.) {
                    body.velocity = Point::new(vx, -475.);
                } else {
                    body.velocity.y = -385.;
                    if body.velocity.x.abs() < 50. {
                        body.velocity.x = 96.;
                    }
                }
                body.hit.clear();
                body.trellis_contacts = 0;
                self.melon_bounce_event(body, false);
            }
            _ => unreachable!(),
        }
        body.home_run = false;
        true
    }
}
