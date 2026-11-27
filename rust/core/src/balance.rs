use crate::{
    ground, CombatContribution, Fruit, FruitKind, LevelId, Monster, MonsterKind, Orchard, Phase,
    Point, SlotItem, Stage, CHIEF_X, DT, GRAVITY, MAX_CUT_VX, MELON_GRAVITY, SLOTS, SPAWN_X,
    SWIPE_DROP_SPEED, TRELLIS_Y,
};
use FruitKind::{BounceMelon, BowlingPumpkin, RedApple};
use MonsterKind::{GreenSlime, IronBoar, Shield, WildHellhound};

pub const SEED_COUNT: u64 = 128;
const TICK_LIMIT: u32 = 120 * 240;
const ACTION_PERIOD: u32 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    Casual,
    Greedy,
    Skilled,
    CheeseAuditor(FruitKind),
}

impl Strategy {
    pub fn name(self) -> &'static str {
        match self {
            Self::Casual => "Casual",
            Self::Greedy => "Greedy",
            Self::Skilled => "Skilled",
            Self::CheeseAuditor(RedApple) => "All-Apple",
            Self::CheeseAuditor(BounceMelon) => "All-Melon",
            Self::CheeseAuditor(BowlingPumpkin) => "All-Pumpkin",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub level: LevelId,
    pub strategy: Strategy,
}

pub const CONFIGS: [Config; 9] = [
    Config {
        level: LevelId::T1,
        strategy: Strategy::Casual,
    },
    Config {
        level: LevelId::T1,
        strategy: Strategy::Greedy,
    },
    Config {
        level: LevelId::T1,
        strategy: Strategy::Skilled,
    },
    Config {
        level: LevelId::T2,
        strategy: Strategy::Casual,
    },
    Config {
        level: LevelId::T2,
        strategy: Strategy::Greedy,
    },
    Config {
        level: LevelId::T2,
        strategy: Strategy::Skilled,
    },
    Config {
        level: LevelId::T2,
        strategy: Strategy::CheeseAuditor(BowlingPumpkin),
    },
    Config {
        level: LevelId::T2,
        strategy: Strategy::CheeseAuditor(RedApple),
    },
    Config {
        level: LevelId::T2,
        strategy: Strategy::CheeseAuditor(BounceMelon),
    },
];

#[derive(Debug, Clone, Copy)]
pub struct Outcome {
    pub seed: u64,
    pub phase: Phase,
    pub hp: u8,
    pub first_damage_tick: Option<u32>,
    pub max_concurrent_monsters: usize,
    pub contributions: [CombatContribution; 4],
}

/// Guardrails use the unrounded percentages and mean printed in the report.
pub fn warnings(config: Config, win_rate: f32, hp: f32, shares: [(f32, f32); 4]) -> Vec<String> {
    let mut warnings = Vec::new();
    match (config.level, config.strategy) {
        (LevelId::T1, Strategy::Casual) if win_rate < 92. => {
            warnings.push(format!("T1 Casual win rate {win_rate:.1}% is below 92%"));
        }
        (LevelId::T1, Strategy::Skilled) => {
            if win_rate < 100. {
                warnings.push(format!("T1 Skilled win rate {win_rate:.1}% is below 100%"));
            }
            if hp < 4. {
                warnings.push(format!("T1 Skilled avg remaining HP {hp:.2} is below 4.0"));
            }
        }
        (LevelId::T2, Strategy::Casual) if !(45. ..=70.).contains(&win_rate) => {
            warnings.push(format!(
                "T2 Casual win rate {win_rate:.1}% is outside 45% to 70%"
            ));
        }
        (LevelId::T2, Strategy::Skilled) if win_rate < 90. => {
            warnings.push(format!("T2 Skilled win rate {win_rate:.1}% is below 90%"));
        }
        (LevelId::T2, Strategy::CheeseAuditor(BowlingPumpkin)) if win_rate > 40. => {
            warnings.push(format!(
                "T2 All-Pumpkin win rate {win_rate:.1}% exceeds 40%"
            ));
        }
        _ => {}
    }
    if config.level == LevelId::T2 {
        if shares[3].1 > 35. {
            warnings.push(format!(
                "T2 Hellhound kill share {:.1}% exceeds 35%",
                shares[3].1
            ));
        }
        if !matches!(config.strategy, Strategy::CheeseAuditor(_)) {
            for (name, (damage, _)) in ["Apple", "Melon", "Pumpkin"].into_iter().zip(shares) {
                if damage > 70. {
                    warnings.push(format!(
                        "T2 mixed {name} damage share {damage:.1}% exceeds 70%"
                    ));
                }
            }
        }
    }
    warnings
}

struct Player {
    strategy: Strategy,
    random: u64,
    casual_delay: [Option<u32>; crate::SLOTS.len()],
}

impl Player {
    fn delay(&mut self) -> u32 {
        self.random = self
            .random
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        72 + ((self.random >> 32) % 109) as u32
    }

    fn apply_loadout(&self, orchard: &mut Orchard) {
        if let Strategy::CheeseAuditor(kind) = self.strategy {
            for slot in 0..SLOTS.len() {
                if orchard.slots[slot].is_some() {
                    assert!(orchard.equip_plant(slot, kind));
                }
            }
        }
    }

    fn act(&mut self, orchard: &mut Orchard) {
        match self.strategy {
            Strategy::Casual => self.casual(orchard),
            Strategy::Greedy => self.greedy(orchard),
            Strategy::Skilled | Strategy::CheeseAuditor(_) => self.skilled(orchard),
        }
    }

    fn casual(&mut self, orchard: &mut Orchard) {
        for slot in 0..SLOTS.len() {
            let Some(SlotItem::Fruit(fruit)) = orchard.slots[slot] else {
                continue;
            };
            if fruit.stage() == Stage::Small {
                self.casual_delay[slot] = None;
                continue;
            }
            let delay = match self.casual_delay[slot] {
                Some(delay) => delay,
                None => {
                    let delay = self.delay();
                    self.casual_delay[slot] = Some(delay);
                    delay
                }
            };
            let ready = fruit.age >= fruit.kind.mature_ticks() + delay;
            let nearby_threat = fruit.age >= fruit.kind.mature_ticks() + delay / 5
                && orchard
                    .monsters
                    .iter()
                    .any(|monster| (6. ..=28.).contains(&(monster.pos.x - SLOTS[slot])));
            if (ready || nearby_threat) && orchard.cut_fruit(slot) {
                self.casual_delay[slot] = None;
            }
        }
        self.try_home_run(orchard, |monster| {
            monster.pos.x <= CHIEF_X + 0.30 * (SPAWN_X - CHIEF_X)
        });
    }

    fn greedy(&self, orchard: &mut Orchard) {
        self.try_home_run(orchard, |monster| monster.pos.x <= SLOTS[1]);
        for slot in 0..SLOTS.len() {
            let Some(SlotItem::Fruit(fruit)) = orchard.slots[slot] else {
                continue;
            };
            if fruit.stage() != Stage::Giant {
                continue;
            }
            if slot == 0
                && fruit.kind == RedApple
                && orchard.protagonist.hp <= 3
                && orchard.protagonist.digestion_cooldown == 0
            {
                orchard.cut_fruit_angled(slot, -MAX_CUT_VX);
            } else {
                if shot_value(
                    orchard,
                    fruit,
                    Point::new(SLOTS[slot], TRELLIS_Y),
                    Point::default(),
                ) >= 0.9
                {
                    orchard.cut_fruit(slot);
                }
            }
        }
    }

    fn try_home_run(&self, orchard: &mut Orchard, target: impl Fn(&Monster) -> bool) -> bool {
        if orchard.protagonist.home_run_cooldown > 0 {
            return false;
        }
        let bodies = orchard
            .bodies
            .iter()
            .filter(|body| !body.rolling && body.velocity.y >= 0.)
            .chain(orchard.barricades.iter().map(|barrier| barrier.body()))
            .filter(|body| body.stage != Stage::Small);
        let mut best: Option<(f32, Point)> = None;
        for body in bodies {
            let clearance = ground(body.pos.x) - body.pos.y - body.radius();
            let descent_dx =
                clearance.max(0.) * crate::HOME_RUN_SPEED / crate::HOME_RUN_APPROACH_SPEED;
            let score = orchard
                .monsters
                .iter()
                .filter(|monster| target(monster))
                .map(|monster| {
                    let gap = monster.pos.x - body.pos.x;
                    if gap < descent_dx - 18. || gap > 225. {
                        return 0.;
                    }
                    let weight = match (body.kind, monster.kind) {
                        (RedApple, IronBoar) if body.stage == Stage::Giant => 3.3,
                        (BowlingPumpkin, IronBoar | Shield) => 2.6,
                        (BounceMelon, IronBoar | Shield) => 2.4,
                        (_, Shield | IronBoar | WildHellhound) => 2.1,
                        _ => 1.,
                    };
                    weight * (1.2 - gap.max(0.) / 400.)
                })
                .sum::<f32>();
            if score > 0. && best.is_none_or(|(rating, _)| score > rating) {
                best = Some((score, body.pos));
            }
        }
        best.is_some_and(|(_, pos)| orchard.trigger_home_run(pos))
    }

    fn skilled(&self, orchard: &mut Orchard) {
        self.try_home_run(orchard, |monster| {
            matches!(monster.kind, IronBoar | Shield | WildHellhound) || monster.pos.x <= SLOTS[1]
        });
        if orchard.monsters.is_empty() {
            return;
        }
        for slot in 0..SLOTS.len() {
            let Some(SlotItem::Fruit(fruit)) = orchard.slots[slot] else {
                continue;
            };
            if fruit.stage() == Stage::Small {
                continue;
            }
            let origin = Point::new(SLOTS[slot], TRELLIS_Y);
            if slot == 0
                && fruit.kind == RedApple
                && fruit.stage() == Stage::Giant
                && orchard.protagonist.hp <= 3
                && orchard.protagonist.digestion_cooldown == 0
            {
                orchard.cut_fruit_angled(slot, -MAX_CUT_VX);
                continue;
            }
            let mut best = (0., 0.);
            for slash in [0., -MAX_CUT_VX, -15., 15., MAX_CUT_VX] {
                let velocity = Point::new(slash, if slash == 0. { 0. } else { SWIPE_DROP_SPEED });
                let score = shot_value(orchard, fruit, origin, velocity);
                if score > best.0 {
                    best = (score, slash);
                }
            }
            if best.0 >= 1. {
                orchard.cut_fruit_angled(slot, best.1);
            }
        }
    }
}

fn forecast_x(orchard: &Orchard, monster: &Monster, time: f32) -> f32 {
    let climb_time = if monster.kind == WildHellhound {
        let paused = orchard.hellhound.unwrap().wild_pause_ticks;
        let moving_time = (time - seconds(paused)).max(0.);
        moving_time * orchard.level().tame_fight.unwrap().climb_speed
    } else {
        time * monster.kind.speed()
    };
    let push = (monster.horizontal_speed() + monster.kind.speed()) * time.min(0.26);
    let x = monster.pos.x - climb_time + push;
    orchard.barricades.iter().fold(x, |x, barrier| {
        let edge = barrier.body().pos.x + barrier.body().radius() + monster.kind.radius();
        if monster.pos.x >= edge {
            x.max(edge)
        } else {
            x
        }
    })
}

fn shot_value(orchard: &Orchard, fruit: Fruit, origin: Point, velocity: Point) -> f32 {
    let gravity = if fruit.kind == BounceMelon {
        MELON_GRAVITY
    } else {
        GRAVITY
    };
    let radius = fruit.kind.radius() * fruit.stage().scale();
    let speed = velocity.y - velocity.x * crate::SLOPE;
    let time_to =
        |y: f32| ((speed * speed + 2. * gravity * (y - origin.y).max(0.)).sqrt() - speed) / gravity;
    let floor_time = time_to(ground(origin.x) - radius);
    let landing = origin.x + velocity.x * floor_time;
    let mut direct: f32 = 0.;
    let mut rolling = 0.;
    let mut crowded = 0.;
    for monster in &orchard.monsters {
        let time = time_to(monster.pos.y - radius);
        let x = forecast_x(orchard, monster, time);
        let fruit_x = origin.x + velocity.x * time;
        let distance = (fruit_x - x).abs();
        let radius_sum = radius + monster.kind.radius();
        let incoming = monster.pos.x > CHIEF_X + 12.;
        if !incoming {
            continue;
        }
        if (x - landing).abs() <= 45. {
            crowded += 1.;
        }
        if distance < radius_sum {
            let precision = 1. - 0.45 * distance / radius_sum;
            let value = match (fruit.kind, fruit.stage(), monster.kind) {
                (RedApple, Stage::Giant, IronBoar | WildHellhound) => 3.1,
                (RedApple, Stage::Mature, Shield) if monster.lid_hp > 0 && distance <= 8. => 1.15,
                (RedApple, _, Shield) => 2.1,
                (RedApple, Stage::Mature, GreenSlime) if monster.hp > 2 => 1.4,
                (BounceMelon, _, Shield) => 2.1,
                (BounceMelon, _, _) => 1.9,
                (BowlingPumpkin, _, GreenSlime) => 1.6,
                _ => 2.35,
            };
            direct = direct.max(precision * value);
        }
        if fruit.kind == BowlingPumpkin {
            let gap = forecast_x(orchard, monster, floor_time) - landing;
            if (-3. ..=98.).contains(&gap) {
                rolling += match monster.kind {
                    GreenSlime => 0.9,
                    Shield | IronBoar if fruit.stage() == Stage::Giant => 1.05,
                    _ => 0.65,
                };
            }
        }
    }
    if fruit.kind == BounceMelon && crowded > 0. {
        direct += 0.40 * crowded;
        if orchard.protagonist.home_run_cooldown == 0
            && orchard.monsters.iter().any(|monster| {
                matches!(monster.kind, IronBoar | Shield)
                    && (20. ..=130.).contains(&(monster.pos.x - origin.x))
            })
        {
            direct += 0.65;
        }
    }
    if fruit.kind == RedApple
        && orchard.barricades.iter().any(|barrier| {
            (barrier.body().pos.x - landing).abs() < 9.
                && orchard
                    .monsters
                    .iter()
                    .any(|monster| (22. ..=75.).contains(&(monster.pos.x - landing)))
        })
    {
        direct = direct.max(1.2);
    }
    direct + rolling
}

pub fn run(config: Config, seed: u64) -> Outcome {
    let mut orchard = Orchard::new(config.level, seed);
    let mut player = Player {
        strategy: config.strategy,
        random: seed ^ 0x6cedf7,
        casual_delay: [None; crate::SLOTS.len()],
    };
    player.apply_loadout(&mut orchard);
    for tick in 0..TICK_LIMIT {
        match orchard.phase {
            Phase::Intermission => {
                player.apply_loadout(&mut orchard);
                orchard.next_wave();
            }
            Phase::Victory | Phase::Defeat => break,
            _ => {}
        }
        if tick % ACTION_PERIOD == 0 {
            player.act(&mut orchard);
        }
        orchard.step();
        orchard.events.clear();
    }
    let [apple, melon, pumpkin] = orchard.stats.fruits;
    Outcome {
        seed,
        phase: orchard.phase,
        hp: orchard.protagonist.hp,
        first_damage_tick: orchard.stats.first_damage_tick,
        max_concurrent_monsters: orchard.stats.max_concurrent_monsters,
        contributions: [apple, melon, pumpkin, orchard.stats.hellhound],
    }
}

pub fn seconds(tick: u32) -> f32 {
    tick as f32 * DT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comfort_guardrails_check_each_boundary_before_rounding() {
        let check = |level, strategy, wins, hp, shares| {
            warnings(Config { level, strategy }, wins, hp, shares).len()
        };
        let empty = [(0., 0.); 4];
        assert_eq!(check(LevelId::T1, Strategy::Casual, 92., 0., empty), 0);
        assert_eq!(check(LevelId::T1, Strategy::Casual, 91.99, 0., empty), 1);
        assert_eq!(check(LevelId::T1, Strategy::Skilled, 100., 4., empty), 0);
        assert_eq!(check(LevelId::T1, Strategy::Skilled, 99.99, 3.99, empty), 2);

        for (wins, count) in [(44.99, 1), (45., 0), (70., 0), (70.01, 1)] {
            assert_eq!(check(LevelId::T2, Strategy::Casual, wins, 0., empty), count);
        }
        assert_eq!(check(LevelId::T2, Strategy::Skilled, 90., 0., empty), 0);
        assert_eq!(check(LevelId::T2, Strategy::Skilled, 89.99, 0., empty), 1);

        let pumpkin = Strategy::CheeseAuditor(BowlingPumpkin);
        let mut shares = empty;
        shares[2].0 = 70.;
        shares[3].1 = 35.;
        assert_eq!(check(LevelId::T2, pumpkin, 40., 0., shares), 0);
        assert_eq!(check(LevelId::T2, pumpkin, 40.01, 0., shares), 1);
        assert_eq!(check(LevelId::T2, Strategy::Greedy, 100., 0., shares), 0);
        shares[2].0 = 70.01;
        shares[3].1 = 35.01;
        assert_eq!(check(LevelId::T2, Strategy::Greedy, 100., 0., shares), 2);
        assert_eq!(check(LevelId::T2, pumpkin, 40., 0., shares), 1);
        assert_eq!(check(LevelId::T1, Strategy::Greedy, 100., 0., shares), 0);
    }
}
