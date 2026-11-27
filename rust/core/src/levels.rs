use crate::{FruitKind, MonsterKind};

use FruitKind::{BounceMelon, BowlingPumpkin, RedApple};
use MonsterKind::{GreenSlime, IronBoar, Shield, WildHellhound};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelId {
    T1,
    T2,
    Sandbox,
}

#[derive(Debug, Clone, Copy)]
pub struct SpawnGroup {
    pub spawn_x: f32,
    pub first_tick: u32,
    pub interval: u32,
    pub jitter: u32,
    pub count: u32,
    pub pattern: &'static [MonsterKind],
}

#[derive(Debug, Clone, Copy)]
pub struct WaveDef {
    pub new_plants: &'static [FruitKind],
    pub spawns: &'static [SpawnGroup],
}

#[derive(Debug, Clone, Copy)]
pub struct TameFightDef {
    pub climb_speed: f32,
    pub stride_ticks: u32,
    pub pant_ticks: u32,
    pub retreat_x: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct CompanionDef {
    pub patrol_end: f32,
    pub patrol_speed: f32,
    pub charge_range: f32,
    pub charge_speed: f32,
    pub damage: u8,
    pub knockback: f32,
    pub cooldown_ticks: u32,
    pub stamina: u8,
    pub nap_ticks: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct LevelDef {
    pub id: LevelId,
    pub display_name: &'static str,
    pub initial_plants: &'static [(usize, FruitKind)],
    pub available_plants: &'static [FruitKind],
    pub initial_chime_slot: Option<usize>,
    pub companion_enabled: bool,
    pub companion: Option<CompanionDef>,
    pub waves: &'static [WaveDef],
    pub tame_fight: Option<TameFightDef>,
    pub rolling_friction: f32,
    pub knockback_retention: f32,
    pub wind_base: f32,
    pub sandbox_opening: &'static [(MonsterKind, f32)],
    pub sandbox_reinforcements: Option<SpawnGroup>,
}

impl LevelDef {
    pub fn load(id: LevelId) -> &'static Self {
        match id {
            LevelId::T1 => &T1,
            LevelId::T2 => &T2,
            LevelId::Sandbox => &SANDBOX,
        }
    }

    pub fn next_level(&self) -> Option<LevelId> {
        match self.id {
            LevelId::T1 => Some(LevelId::T2),
            LevelId::T2 => Some(LevelId::Sandbox),
            LevelId::Sandbox => None,
        }
    }
}

const fn group(
    first_tick: u32,
    interval: u32,
    jitter: u32,
    count: u32,
    pattern: &'static [MonsterKind],
) -> SpawnGroup {
    SpawnGroup {
        spawn_x: crate::SPAWN_X,
        first_tick,
        interval,
        jitter,
        count,
        pattern,
    }
}

const COMPANION: CompanionDef = CompanionDef {
    patrol_end: 230.,
    patrol_speed: 24.,
    charge_range: 44.,
    charge_speed: 155.,
    damage: 1,
    knockback: 105.,
    cooldown_ticks: 420,
    stamina: 3,
    nap_ticks: 12 * 120,
};

static T1: LevelDef = LevelDef {
    id: LevelId::T1,
    display_name: "初遇黑狼",
    initial_plants: &[(0, RedApple), (2, RedApple), (4, RedApple), (5, RedApple)],
    available_plants: &[RedApple],
    initial_chime_slot: None,
    companion_enabled: false,
    companion: None,
    waves: &[
        WaveDef {
            new_plants: &[],
            spawns: &[group(120, 386, 35, 5, &[GreenSlime])],
        },
        WaveDef {
            new_plants: &[],
            spawns: &[group(100, 305, 48, 7, &[GreenSlime])],
        },
        WaveDef {
            new_plants: &[],
            spawns: &[
                group(100, 270, 30, 4, &[GreenSlime]),
                group(1210, 1, 45, 1, &[WildHellhound]),
            ],
        },
    ],
    tame_fight: Some(TameFightDef {
        climb_speed: 16.,
        stride_ticks: 288,
        pant_ticks: 72,
        retreat_x: 310.,
    }),
    rolling_friction: 1.,
    knockback_retention: 0.976,
    wind_base: 0.8,
    sandbox_opening: &[],
    sandbox_reinforcements: None,
};

static T2: LevelDef = LevelDef {
    id: LevelId::T2,
    display_name: "节奏压测",
    initial_plants: &[
        (0, RedApple),
        (2, BounceMelon),
        (4, BowlingPumpkin),
        (5, RedApple),
    ],
    available_plants: &[RedApple, BounceMelon, BowlingPumpkin],
    initial_chime_slot: None,
    companion_enabled: true,
    companion: Some(COMPANION),
    waves: &[
        WaveDef {
            new_plants: &[],
            spawns: &[
                // A nearby opening pack rewards fast-growing fruit before the main wave arrives.
                SpawnGroup {
                    spawn_x: 185.,
                    ..group(0, 78, 100, 6, &[GreenSlime])
                },
                group(100, 229, 60, 9, &[GreenSlime, IronBoar, Shield, GreenSlime]),
            ],
        },
        WaveDef {
            new_plants: &[BounceMelon],
            spawns: &[group(
                90,
                240,
                72,
                13,
                &[Shield, GreenSlime, IronBoar, Shield, GreenSlime],
            )],
        },
        WaveDef {
            new_plants: &[],
            spawns: &[group(
                70,
                167,
                105,
                22,
                &[IronBoar, Shield, IronBoar, GreenSlime, IronBoar],
            )],
        },
    ],
    tame_fight: None,
    rolling_friction: 1.,
    knockback_retention: 0.976,
    wind_base: 1.05,
    sandbox_opening: &[],
    sandbox_reinforcements: None,
};

static SANDBOX: LevelDef = LevelDef {
    id: LevelId::Sandbox,
    display_name: "满配沙盒",
    initial_plants: &[
        (0, RedApple),
        (1, BounceMelon),
        (2, BowlingPumpkin),
        (4, RedApple),
        (5, BowlingPumpkin),
    ],
    available_plants: &[RedApple, BounceMelon, BowlingPumpkin],
    initial_chime_slot: Some(3),
    companion_enabled: true,
    companion: Some(COMPANION),
    waves: &[],
    tame_fight: None,
    rolling_friction: 1.,
    knockback_retention: 0.976,
    wind_base: 1.,
    sandbox_opening: &[(GreenSlime, 275.), (IronBoar, 350.), (Shield, 423.)],
    sandbox_reinforcements: Some(group(
        600,
        128,
        0,
        u32::MAX,
        &[GreenSlime, IronBoar, Shield],
    )),
};
