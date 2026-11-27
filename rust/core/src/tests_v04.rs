use super::tests::{body_at, isolated_sandbox};
use super::*;
use FruitKind::{BounceMelon, BowlingPumpkin, RedApple, SlimeBerry};

const CROPS: [FruitKind; 4] = [RedApple, BounceMelon, BowlingPumpkin, SlimeBerry];

#[test]
fn prep_battle_result_and_retries_enforce_command_permissions() {
    for direct in [false, true] {
        let mut game = Orchard::new(LevelId::T5, 19);
        for _ in 0..500 {
            game.step();
        }
        assert_eq!(
            (game.phase, game.elapsed, game.monsters.len()),
            (Phase::Prep, 0, 0)
        );
        assert!(!game.cut_fruit(1));
        assert!(game.graft_plants(1, 2));
        assert!(game.set_hellhound(false));
        assert!(game.set_lulucy(false));
        assert!(game.swap_plants(2, 1));
        assert!(game.start_battle());
        assert!(!game.start_battle() && !game.retry(true));
        assert!(!game.set_hellhound(true) && !game.set_lulucy(true));
        assert!(!game.swap_plants(1, 0) && !game.swap_plants(1, 6));
        assert!(!game.graft_plants(0, 2) && !game.equip_plant(0, SlimeBerry));
        assert!(!game.equip_chime(0) && !game.remove_plant(0));
        game.protagonist.hp = 1;
        game.place_monster(MonsterKind::GreenSlime, CHIEF_X);
        game.step();
        assert_eq!(game.phase, Phase::Defeat);
        assert!(!game.next_level() && !game.start_battle());
        assert!(!game.cut_fruit(1) && !game.equip_plant(1, RedApple));
        assert!(game.retry(direct));
        assert_eq!(game.phase, if direct { Phase::Battle } else { Phase::Prep });
        assert_eq!(
            (game.elapsed, game.wave, game.protagonist.hp),
            (0, 1, MAX_HP)
        );
        assert!(matches!(
            game.slots[1],
            Some(SlotItem::Fruit(Fruit {
                kind: BounceMelon,
                graft: Some(SlimeBerry),
                age: 0
            }))
        ));
        assert!(game.hellhound.is_none() && !game.lulucy && game.monsters.is_empty());
    }
    let mut game = Orchard::default();
    for id in [LevelId::T1, LevelId::T2, LevelId::T5] {
        assert_eq!((game.level.id, game.phase), (id, Phase::Prep));
        assert!(game.start_battle());
        game.wave = game.level.waves.len();
        game.end_wave();
        assert_eq!(game.phase, Phase::Victory);
        assert_eq!(game.next_level(), id != LevelId::T5);
    }
}

#[test]
fn ground_monsters_share_one_entrance_and_climb() {
    let mut game = Orchard::default();
    for kind in [
        MonsterKind::GreenSlime,
        MonsterKind::IronBoar,
        MonsterKind::Shield,
        MonsterKind::WildHellhound,
    ] {
        game.spawn(kind);
        let monster = game.monsters.last().unwrap();
        assert_eq!(
            (monster.pos.x, monster.pos.y),
            (SPAWN_X, ground(SPAWN_X) - kind.radius())
        );
        game.move_monsters();
        assert!(game.monsters.last().unwrap().pos.x < SPAWN_X);
    }
    let sandbox = Orchard::new(LevelId::Sandbox, 0);
    assert!(sandbox.monsters.iter().all(|m| m.pos.x == SPAWN_X));
}

#[test]
fn all_sixteen_ordered_grafts_survive_harvest_and_refuse_a_third_parent() {
    for base in CROPS {
        for donor in CROPS {
            let mut game = Orchard::new(LevelId::T5, 3);
            assert!(game.equip_plant(0, donor) && game.equip_plant(1, base));
            assert!(game.graft_plants(0, 1));
            assert!(game.slots[0].is_none());
            assert!(!game.graft_plants(2, 1) && !game.graft_plants(1, 2));
            let Some(SlotItem::Fruit(ref mut fruit)) = game.slots[1] else {
                panic!()
            };
            assert_eq!((fruit.kind, fruit.graft), (base, Some(donor)));
            for (actual, first, second) in [
                (
                    fruit.growth_ticks(),
                    base.growth_ticks(),
                    donor.growth_ticks(),
                ),
                (
                    fruit.mature_ticks(),
                    base.mature_ticks(),
                    donor.mature_ticks(),
                ),
            ] {
                let expected = if donor == RedApple && base != RedApple {
                    (first as f32 * 0.7).round() as u32
                } else {
                    first.max(second)
                };
                assert_eq!(actual, expected);
            }
            fruit.age = fruit.growth_ticks();
            assert!(game.start_battle() && game.cut_fruit(1));
            assert_eq!(
                game.bodies.len(),
                if base == RedApple && base == donor {
                    2
                } else {
                    1
                }
            );
            assert!(game
                .bodies
                .iter()
                .all(|b| b.kind == base && b.graft == Some(donor) && b.stage == Stage::Giant));
            assert!(
                matches!(game.slots[1], Some(SlotItem::Fruit(f)) if f.graft == Some(donor) && f.age == 0)
            );
        }
    }
}

#[test]
fn berry_land_splash_slows_without_ticks_of_damage_and_big_puddle_spreads() {
    assert_eq!(
        (SlimeBerry.mature_ticks(), SlimeBerry.growth_ticks()),
        (312, 624)
    );
    for graft in [None, Some(SlimeBerry)] {
        let mut game = isolated_sandbox();
        game.slots.fill(None);
        let x = 250.;
        game.place_monster(MonsterKind::IronBoar, x + 21.);
        let mut berry = body_at(
            &mut game,
            SlimeBerry,
            Stage::Mature,
            Point::new(x, ground(x) - 6.),
            Point::new(0., 300.),
        );
        berry.graft = graft;
        assert!(!game.move_body(&mut berry));
        let zone = &game.juice_zones[0];
        assert_eq!(zone.kind, Puddle::Slime);
        assert_eq!(
            zone.right - zone.left,
            if graft.is_some() { 94. } else { 52. }
        );
        let start = game.monsters[0].pos.x;
        for _ in 0..120 {
            game.move_monsters();
        }
        let monster = &game.monsters[0];
        assert!((start - monster.pos.x - monster.kind.speed() * 0.35).abs() < 0.01);
        assert_eq!(monster.hp, MonsterKind::IronBoar.hp() - 1);
        for _ in 0..JUICE_TICKS {
            game.update_barricades();
        }
        assert!(game.juice_zones.is_empty());
    }
}

#[test]
fn every_fruit_even_green_detonates_oil_once_and_bonus_belongs_to_fruit() {
    for kind in CROPS {
        for stage in [Stage::Small, Stage::Mature] {
            let mut game = isolated_sandbox();
            game.slots.fill(None);
            game.place_monster(MonsterKind::IronBoar, 250.);
            game.juice_zones.push(JuiceZone {
                kind: Puddle::Juice,
                left: 200.,
                right: 300.,
                ticks_left: 600,
            });
            game.elapsed = 299;
            game.step();
            assert_eq!(game.juice_zones[0].kind, Puddle::Oil);
            assert!(game
                .events
                .iter()
                .any(|e| matches!(e, VfxEvent::OilCast(_))));
            for _ in 0..120 {
                game.step();
            }
            assert_eq!(game.monsters[0].hp, MonsterKind::IronBoar.hp());
            assert!(game.monsters[0].fragile);
            let pos = game.monsters[0].pos;
            let mut fruit = body_at(&mut game, kind, stage, pos, Point::new(0., 300.));
            let damage = fruit.damage(MonsterKind::IronBoar);
            game.move_body(&mut fruit);
            assert_eq!(game.monsters[0].hp, MonsterKind::IronBoar.hp() - 2 * damage);
            game.move_monsters();
            assert!(!game.monsters[0].fragile);
            assert_eq!(
                (
                    game.stats.assists.detonations,
                    game.stats.assists.bonus_damage
                ),
                (1, u32::from(damage))
            );
            assert_eq!(
                game.stats.fruits[kind as usize].damage,
                u32::from(2 * damage)
            );
            assert!(game
                .events
                .iter()
                .any(|e| matches!(e, VfxEvent::FragileBurst(_))));
        }
    }
}

#[test]
fn super_bounce_armor_and_grafted_landings_modify_base_physics() {
    let mut game = isolated_sandbox();
    let mut melon = body_at(
        &mut game,
        BounceMelon,
        Stage::Giant,
        Point::new(240., ground(240.) - 10.),
        Point::new(0., 300.),
    );
    let mut enhanced = melon.clone();
    enhanced.graft = Some(BounceMelon);
    melon.graft = Some(SlimeBerry);
    assert!(game.rebound(&mut melon) && game.rebound(&mut enhanced));
    assert!(enhanced.velocity.y < melon.velocity.y && enhanced.energy_ratio > melon.energy_ratio);
    assert_eq!(game.juice_zones[0].kind, Puddle::Slime);
    for (graft, bites) in [(BowlingPumpkin, 4), (RedApple, 1)] {
        let mut game = isolated_sandbox();
        game.place_monster(MonsterKind::IronBoar, 200.);
        let pos = game.monsters[0].pos;
        let mut pumpkin = body_at(
            &mut game,
            BowlingPumpkin,
            Stage::Mature,
            pos,
            Point::new(70., 0.),
        );
        pumpkin.graft = Some(graft);
        pumpkin.rolling = true;
        game.bodies.push(pumpkin);
        game.step();
        assert_eq!(game.barricades[0].hits_left, bites);
        if graft == RedApple {
            assert_eq!(game.juice_zones[0].kind, Puddle::Juice);
        }
    }
}

#[test]
fn hound_rushes_to_a_spent_melon_and_refills_its_energy() {
    let mut game = isolated_sandbox();
    game.set_hellhound(true);
    let start = game.hellhound.unwrap().pos.x;
    let x = start + 22.;
    let mut melon = body_at(
        &mut game,
        BounceMelon,
        Stage::Mature,
        Point::new(x, ground(x) - 26.),
        Point::new(0., 40.),
    );
    melon.energy_ratio = 0.10;
    melon.bounces = 3;
    game.bodies.push(melon);
    for _ in 0..40 {
        game.step();
        if game.stats.assists.melon_headers > 0 {
            break;
        }
    }
    assert!(game.hellhound.unwrap().pos.x > start);
    let melon = &game.bodies[0];
    assert_eq!(melon.energy_ratio, 1.);
    assert!(melon.velocity.y < -450.);
    assert_eq!(game.stats.assists.melon_headers, 1);
    for _ in 0..50 {
        game.step();
    }
    assert!(game.bodies[0].pos.y < TRELLIS_Y);
}

#[test]
fn awake_angled_apple_starts_one_swept_zero_damage_zoom_with_eight_second_cooldown() {
    let mut game = isolated_sandbox();
    game.set_hellhound(true);
    let mouth = game.hellhound.unwrap().pos;
    let mut apple = body_at(
        &mut game,
        RedApple,
        Stage::Giant,
        mouth,
        Point::new(20., 120.),
    );
    assert!(!game.feed_companion(&apple, mouth));
    apple.angled = true;
    for (i, kind) in [
        MonsterKind::GreenSlime,
        MonsterKind::Shield,
        MonsterKind::IronBoar,
    ]
    .into_iter()
    .enumerate()
    {
        game.place_monster(kind, mouth.x + 40. + 75. * i as f32);
    }
    for x in [mouth.x + 60., mouth.x + 220.] {
        let body = body_at(
            &mut game,
            BowlingPumpkin,
            Stage::Mature,
            Point::new(x, ground(x) - 8.),
            Point::default(),
        );
        game.barricades.push(Barricade {
            body,
            hits_left: 2,
            age: 0,
        });
    }
    assert!(game.feed_companion(&apple, mouth));
    assert_eq!(game.hellhound.unwrap().zoom_cooldown, 960);
    for _ in 0..54 {
        game.update_companion();
    }
    assert!(game.hellhound.unwrap().pos.x - mouth.x > 285.);
    assert_eq!(game.stats.assists.zoomies, 1);
    assert_eq!(game.stats.assists.shoves, 3);
    assert_eq!(game.stats.assists.pumpkin_headers, 2);
    assert!(game.barricades.is_empty());
    assert!(game
        .monsters
        .iter()
        .all(|m| m.hp == m.kind.hp() && m.knockback > 250.));
    for _ in 54..440 {
        game.update_companion();
    }
    apple.pos = game.hellhound.unwrap().pos;
    assert!(!game.feed_companion(&apple, apple.pos));
    for _ in 440..960 {
        game.update_companion();
    }
    apple.pos = game.hellhound.unwrap().pos;
    assert!(game.feed_companion(&apple, apple.pos));
}

#[test]
fn berry_with_apple_trait_leaves_both_slime_and_juice() {
    let mut game = isolated_sandbox();
    let mut berry = body_at(
        &mut game,
        SlimeBerry,
        Stage::Giant,
        Point::new(300., ground(300.) - 7.),
        Point::new(0., 160.),
    );
    berry.graft = Some(RedApple);
    assert!(!game.move_body(&mut berry));
    assert!(game.juice_zones.iter().any(|z| z.kind == Puddle::Juice));
    assert!(game.juice_zones.iter().any(|z| z.kind == Puddle::Slime));
}

#[test]
fn slow_giant_rolls_until_its_third_heavy() {
    let mut game = isolated_sandbox();
    let x = 240.;
    let mut pumpkin = body_at(
        &mut game,
        BowlingPumpkin,
        Stage::Giant,
        Point::new(x, ground(x) - 11.),
        Point::new(48., 0.),
    );
    pumpkin.rolling = true;
    game.bodies.push(pumpkin);
    for i in 1..=3 {
        let x = game.bodies[0].pos.x;
        game.place_monster(MonsterKind::IronBoar, x + 7.);
        game.step();
        assert_eq!(game.barricades.len(), usize::from(i == 3));
    }
}
