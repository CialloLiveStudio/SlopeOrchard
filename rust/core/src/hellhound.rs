use std::collections::VecDeque;

use crate::{
    ground, Body, FruitKind, MonsterKind, Orchard, Point, Stage, TameFightDef, VfxEvent, CHIEF_X,
    DT, SLOPE,
};

#[derive(Debug, Clone, Copy)]
pub enum HellhoundState {
    Wild,
    Tamed,
    Patrolling,
    Charging,
    Dizzy { ticks_left: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WildAction {
    Climbing,
    Panting,
    Munching,
    Dizzy,
}

#[derive(Debug, Clone, Copy)]
pub struct Hellhound {
    pub pos: Point,
    pub state: HellhoundState,
    pub tame_progress: u8,
    pub tame_max: u8,
    pub wild_action: WildAction,
    pub wild_pause_ticks: u32,
    wild_stride_ticks: u32,
    pub stamina: u8,
    pub facing_right: bool,
    cooldown: u32,
    charge: Option<ChargeTarget>,
}

#[derive(Debug, Clone, Copy)]
enum ChargeTarget {
    Monster(u32),
    Pumpkin(u32),
}

impl Hellhound {
    pub(crate) fn wild(pos: Point) -> Self {
        Self {
            pos,
            state: HellhoundState::Wild,
            tame_progress: 0,
            tame_max: 100,
            wild_action: WildAction::Climbing,
            wild_pause_ticks: 0,
            wild_stride_ticks: 0,
            stamina: 0,
            facing_right: false,
            cooldown: 0,
            charge: None,
        }
    }

    pub(crate) fn companion(stamina: u8) -> Self {
        Self {
            pos: Self::on_slope(CHIEF_X + 26.),
            state: HellhoundState::Patrolling,
            tame_progress: 0,
            tame_max: 0,
            wild_action: WildAction::Climbing,
            wild_pause_ticks: 0,
            wild_stride_ticks: 0,
            stamina,
            facing_right: true,
            cooldown: 0,
            charge: None,
        }
    }

    pub(crate) fn advance_wild(
        &mut self,
        config: TameFightDef,
        events: &mut VecDeque<VfxEvent>,
    ) -> f32 {
        if self.wild_pause_ticks > 0 {
            self.wild_pause_ticks -= 1;
            return 0.;
        }
        if self.wild_stride_ticks == config.stride_ticks {
            self.wild_stride_ticks = 0;
            self.wild_action = WildAction::Panting;
            self.wild_pause_ticks = config.pant_ticks - 1;
            events.push_back(VfxEvent::WildPant(self.pos));
            return 0.;
        }
        self.wild_action = WildAction::Climbing;
        self.wild_stride_ticks += 1;
        config.climb_speed
    }

    fn on_slope(x: f32) -> Point {
        Point::new(x, ground(x) - 10.)
    }
}

impl Orchard {
    pub(crate) fn tame_hit(&mut self, index: usize, body: &Body) {
        let dog = self.hellhound.as_mut().unwrap();
        let monster = &mut self.monsters[index];
        let pos = monster.pos;
        if body.kind != FruitKind::RedApple && body.stage != Stage::Small {
            return;
        }
        let progress = match (body.stage, body.home_run) {
            (Stage::Small, _) => {
                self.events.push_back(VfxEvent::WildSour(pos));
                3
            }
            (stage, true) => {
                monster.knockback = 205.;
                dog.wild_action = WildAction::Dizzy;
                dog.wild_pause_ticks = 54;
                self.events.push_back(VfxEvent::WildDizzy(pos, stage));
                if stage == Stage::Giant {
                    60
                } else {
                    35
                }
            }
            (stage, false) => {
                dog.wild_action = WildAction::Munching;
                dog.wild_pause_ticks = 132;
                self.events.push_back(VfxEvent::WildMunch(pos, stage));
                if stage == Stage::Giant {
                    monster.knockback = 48.;
                    50
                } else {
                    monster.knockback = 0.;
                    25
                }
            }
        };
        dog.tame_progress = dog.tame_progress.saturating_add(progress).min(dog.tame_max);
        self.events
            .push_back(VfxEvent::TameProgress(pos, dog.tame_progress, dog.tame_max));
        if dog.tame_progress == dog.tame_max {
            dog.state = HellhoundState::Tamed;
            dog.wild_pause_ticks = 0;
            self.companion_unlocked = true;
            self.phase = crate::Phase::Victory;
            self.events.push_back(VfxEvent::Tamed(pos));
        }
    }

    pub(crate) fn feed_companion(&mut self, body: &Body, old: Point) -> bool {
        let Some(dog) = self.hellhound.as_mut() else {
            return false;
        };
        if body.kind != FruitKind::RedApple
            || body.stage == Stage::Small
            || !matches!(dog.state, HellhoundState::Dizzy { .. })
            || !dog.pos.within_segment(old, body.pos, body.radius() + 11.)
        {
            return false;
        }
        dog.stamina = self.level.companion.unwrap().stamina;
        dog.state = HellhoundState::Patrolling;
        dog.cooldown = 0;
        self.events.push_back(VfxEvent::HellhoundRevived(dog.pos));
        true
    }

    pub(crate) fn update_companion(&mut self) {
        let (Some(config), Some(mut dog)) = (self.level.companion, self.hellhound) else {
            return;
        };
        if let HellhoundState::Dizzy { ticks_left } = dog.state {
            dog.state = if ticks_left > 1 {
                HellhoundState::Dizzy {
                    ticks_left: ticks_left - 1,
                }
            } else {
                dog.stamina = config.stamina;
                dog.cooldown = 0;
                HellhoundState::Patrolling
            };
            self.hellhound = Some(dog);
            return;
        }

        dog.cooldown = dog.cooldown.saturating_sub(1);
        let min = CHIEF_X + 26.;
        let max = config.patrol_end;
        let x = dog.pos.x;
        if let Some(boar) = self.monsters.iter_mut().find(|monster| {
            monster.kind == MonsterKind::IronBoar
                && monster.hp > 0
                && !monster.hit_companion
                && monster.knockback < monster.kind.speed()
                && (0. ..=17.).contains(&(monster.pos.x - x))
        }) {
            boar.hit_companion = true;
            dog.stamina -= 1;
            dog.charge = None;
            dog.cooldown = config.cooldown_ticks;
            dog.pos = Hellhound::on_slope((x - 11.).max(min));
            dog.state = if dog.stamina == 0 {
                self.events.push_back(VfxEvent::HellhoundDizzy(dog.pos));
                HellhoundState::Dizzy {
                    ticks_left: config.nap_ticks,
                }
            } else {
                HellhoundState::Patrolling
            };
            self.hellhound = Some(dog);
            return;
        }

        if dog.cooldown == 0 && dog.charge.is_none() {
            let in_range = |target: f32| {
                target >= x - 8. && target <= (x + config.charge_range).min(max + 10.)
            };
            dog.charge = self
                .barricades
                .iter()
                .filter(|barrier| in_range(barrier.body.pos.x))
                .min_by(|a, b| a.body.pos.x.total_cmp(&b.body.pos.x))
                .map(|barrier| ChargeTarget::Pumpkin(barrier.body.id))
                .or_else(|| {
                    self.monsters
                        .iter()
                        .filter(|monster| {
                            monster.hp > 0
                                && in_range(monster.pos.x)
                                && matches!(
                                    monster.kind,
                                    MonsterKind::GreenSlime | MonsterKind::Shield
                                )
                        })
                        .min_by(|a, b| a.pos.x.total_cmp(&b.pos.x))
                        .map(|monster| ChargeTarget::Monster(monster.id))
                });
            if dog.charge.is_some() {
                self.events.push_back(VfxEvent::HellhoundCharge(dog.pos));
            }
        }

        let target = match dog.charge {
            Some(ChargeTarget::Monster(id)) => self
                .monsters
                .iter()
                .find(|m| m.id == id && m.hp > 0)
                .map(|m| m.pos.x),
            Some(ChargeTarget::Pumpkin(id)) => self
                .barricades
                .iter()
                .find(|b| b.body.id == id)
                .map(|b| b.body.pos.x),
            None => None,
        };
        if let Some(target) = target.filter(|target| *target <= max + 10. && *target >= min - 10.) {
            let dx = target - x;
            dog.facing_right = dx >= 0.;
            if dx.abs() > 14. {
                dog.state = HellhoundState::Charging;
                dog.pos = Hellhound::on_slope(
                    (x + dx.signum() * config.charge_speed * DT).clamp(min, max),
                );
                self.hellhound = Some(dog);
                return;
            }
            match dog.charge.take().unwrap() {
                ChargeTarget::Monster(id) => {
                    let monster = self.monsters.iter_mut().find(|m| m.id == id).unwrap();
                    let damage = config.damage.min(monster.hp);
                    monster.hp -= damage;
                    monster.knockback += config.knockback;
                    self.stats.hellhound.record(damage, monster.hp == 0);
                    self.events
                        .push_back(VfxEvent::HellhoundHit(monster.pos, damage));
                    if monster.hp == 0 {
                        self.events.push_back(VfxEvent::MonsterDie(monster.pos));
                    }
                }
                ChargeTarget::Pumpkin(id) => {
                    let index = self
                        .barricades
                        .iter()
                        .position(|b| b.body.id == id)
                        .unwrap();
                    let mut body = self.barricades.remove(index).body;
                    let speed = 125.;
                    body.rolling = true;
                    body.velocity = Point::new(speed, speed * SLOPE);
                    body.hit.clear();
                    self.events.push_back(VfxEvent::PumpkinBowl(body.pos));
                    self.bodies.push(body);
                }
            }
            dog.cooldown = config.cooldown_ticks;
        }
        dog.charge = None;
        dog.state = HellhoundState::Patrolling;
        let direction = if dog.facing_right { 1. } else { -1. };
        let next = (x + direction * config.patrol_speed * DT).clamp(min, max);
        if next == min || next == max {
            dog.facing_right = next == min;
        }
        dog.pos = Hellhound::on_slope(next);
        self.hellhound = Some(dog);
    }
}
