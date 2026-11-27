use FruitKind::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FruitKind {
    RedApple,
    BounceMelon,
    BowlingPumpkin,
    SlimeBerry,
    TwinApple,
    CrownMelon,
    ArmoredPumpkin,
    MegaBerry,
    Katamari,
    Pinata,
    Quake,
    Bubble,
    Pinball,
    Candy,
}

// Table order is also the stable sprite order in the extension snapshot.
const RECIPES: [(FruitKind, FruitKind, FruitKind, &str); 10] = [
    (RedApple, RedApple, TwinApple, "双生红苹果"),
    (BounceMelon, BounceMelon, CrownMelon, "王冠甜瓜"),
    (BowlingPumpkin, BowlingPumpkin, ArmoredPumpkin, "重甲巨南瓜"),
    (SlimeBerry, SlimeBerry, MegaBerry, "巨泡粘液莓"),
    (BowlingPumpkin, SlimeBerry, Katamari, "块魂滚怪球"),
    (RedApple, BowlingPumpkin, Pinata, "套娃皮纳塔瓜"),
    (BounceMelon, BowlingPumpkin, Quake, "打桩震地瓜"),
    (BounceMelon, SlimeBerry, Bubble, "吞怪大泡泡"),
    (RedApple, BounceMelon, Pinball, "三连弹珠串"),
    (RedApple, SlimeBerry, Candy, "糖渍诱饵果"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImpactClass {
    Apple,
    Melon,
    Pumpkin,
    Berry,
}

impl FruitKind {
    pub const COUNT: usize = 14;
    pub const SEEDS: [Self; 4] = [RedApple, BounceMelon, BowlingPumpkin, SlimeBerry];

    pub fn combine(self, other: Self) -> Option<Self> {
        RECIPES
            .iter()
            .find(|&&(a, b, _, _)| (a, b) == (self, other) || (b, a) == (self, other))
            .map(|r| r.2)
    }

    pub fn parents(self) -> Option<(Self, Self)> {
        RECIPES.iter().find(|r| r.2 == self).map(|r| (r.0, r.1))
    }

    pub fn name(self) -> &'static str {
        match self {
            RedApple => "红苹果",
            BounceMelon => "甜瓜",
            BowlingPumpkin => "南瓜",
            SlimeBerry => "粘液莓",
            _ => RECIPES.iter().find(|r| r.2 == self).unwrap().3,
        }
    }

    pub fn growth_ticks(self) -> u32 {
        match self {
            RedApple | TwinApple => 504,
            BounceMelon | CrownMelon => 744,
            BowlingPumpkin | ArmoredPumpkin => 1104,
            SlimeBerry | MegaBerry => 624,
            Katamari | Pinata | Quake => 840,
            Bubble | Pinball | Candy => 660,
        }
    }

    pub fn mature_ticks(self) -> u32 {
        match self {
            RedApple | TwinApple => 240,
            BounceMelon | CrownMelon => 384,
            BowlingPumpkin | ArmoredPumpkin => 576,
            SlimeBerry | MegaBerry => 312,
            Katamari | Pinata | Quake => 420,
            Bubble | Pinball | Candy => 330,
        }
    }

    pub fn hanging_ticks(self) -> u32 {
        (self.mature_ticks() * 3).div_ceil(10)
    }

    pub(crate) fn radius(self) -> f32 {
        match self {
            RedApple | TwinApple | Candy => 6.5,
            BounceMelon | CrownMelon => 8.,
            BowlingPumpkin | ArmoredPumpkin | Pinata | Quake | Katamari => 9.5,
            SlimeBerry => 7.,
            MegaBerry | Bubble => 11.,
            Pinball => 4.,
        }
    }

    pub(crate) fn melon_motion(self) -> bool {
        matches!(self, BounceMelon | CrownMelon)
    }
    pub(crate) fn can_header(self) -> bool {
        self.melon_motion() || matches!(self, Bubble | Quake)
    }

    // Shared contact damage and impulse profile. Toy trajectories live in toys.rs.
    pub(crate) fn impact_class(self) -> ImpactClass {
        match self {
            RedApple | TwinApple => ImpactClass::Apple,
            BounceMelon | CrownMelon | Pinball | Bubble => ImpactClass::Melon,
            BowlingPumpkin | ArmoredPumpkin | Pinata | Quake | Katamari => ImpactClass::Pumpkin,
            SlimeBerry | Candy | MegaBerry => ImpactClass::Berry,
        }
    }
}
