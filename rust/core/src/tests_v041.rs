use super::tests::isolated_sandbox;
use super::*;
use FruitKind::*;
use MonsterKind::*;

fn toy(kind: FruitKind, stage: Stage, x: f32) -> Body {
    Body::new(
        900,
        kind,
        stage,
        Point::new(x, ground(x) - kind.radius() * stage.scale()),
        Point::new(10., 240.),
    )
}

#[test]
fn all_ten_recipes_preview_merge_regrow_split_and_reject_a_third_parent() {
    for level in [LevelId::T5, LevelId::Sandbox] {
        let mut seen = Vec::new();
        for a in FruitKind::SEEDS {
            for b in FruitKind::SEEDS {
                let recipe = a.combine(b).unwrap();
                assert_eq!(b.combine(a), Some(recipe));
                if !seen.contains(&recipe) {
                    seen.push(recipe);
                }
                let mut game = Orchard::new(level, 2);
                game.equip_plant(0, a);
                game.equip_plant(1, b);
                assert_eq!(game.preview_graft(PlantSource::Slot(0), 1), Some(recipe));
                assert!(game.graft_plants(0, 1));
                assert!(game.slots[0].is_none());
                assert!(matches!(game.events.back(), Some(VfxEvent::Synthesis(..))));
                assert!(!game.graft_plants(1, 2) && !game.merge(PlantSource::Seed(a), 1));
                assert!(game.split_or_clear(1));
                assert!(game.graft_plants(0, 1));
                let Some(SlotItem::Fruit(f)) = &mut game.slots[1] else {
                    panic!()
                };
                assert_eq!(f.kind, recipe);
                f.age = f.growth_ticks();
                if level == LevelId::T5 {
                    game.start_battle();
                    assert!(!game.split_or_clear(1));
                }
                assert!(game.cut_fruit(1));
                assert!(
                    matches!(game.slots[1],Some(SlotItem::Fruit(f)) if f.kind == recipe && f.age == 0)
                );
            }
        }
        assert_eq!(seen.len(), 10);
    }
}

#[test]
fn twin_cut_and_pinball_fan_spawn_independent_projectiles_immediately() {
    for (kind, stage, count) in [
        (TwinApple, Stage::Mature, 2),
        (Pinball, Stage::Mature, 3),
        (Pinball, Stage::Giant, 5),
    ] {
        let mut g = isolated_sandbox();
        g.slots[0] = Some(SlotItem::Fruit(Fruit {
            kind,
            age: if stage == Stage::Giant {
                kind.growth_ticks()
            } else {
                kind.mature_ticks()
            },
        }));
        assert!(g.cut_fruit(0));
        assert_eq!(g.bodies.len(), count);
        assert!(g
            .bodies
            .iter()
            .all(|b| b.stage == stage
                && b.kind == if kind == TwinApple { RedApple } else { Pinball }));
        let mut speeds: Vec<_> = g.bodies.iter().map(|b| b.velocity.x).collect();
        speeds.sort_by(f32::total_cmp);
        speeds.dedup();
        assert_eq!(speeds.len(), count);
        if kind == Pinball {
            assert!(speeds[0] < -150. && speeds[count - 1] > 150.);
        }
    }
}

#[test]
fn crowned_melon_retains_more_energy_and_armored_giant_parks_at_five() {
    let mut g = isolated_sandbox();
    let mut plain = toy(BounceMelon, Stage::Mature, 230.);
    let mut crown = toy(CrownMelon, Stage::Mature, 230.);
    assert!(g.rebound(&mut plain) && g.rebound(&mut crown));
    assert!(crown.velocity.y < plain.velocity.y && crown.energy_ratio > plain.energy_ratio);
    let mut shell = toy(ArmoredPumpkin, Stage::Giant, 220.);
    shell.rolling = true;
    shell.velocity.x = 65.;
    g.bodies.push(shell);
    for i in 1..=5 {
        g.place_monster(IronBoar, g.bodies[0].pos.x + 4.);
        g.step();
        assert_eq!(g.barricades.len(), usize::from(i == 5));
    }
}

#[test]
fn katamari_preserves_passenger_ids_rotates_and_pops_at_each_cap() {
    for (stage, cap) in [(Stage::Mature, 3), (Stage::Giant, 5)] {
        let mut g = isolated_sandbox();
        let mut ball = toy(Katamari, stage, 230.);
        assert!(g.move_body(&mut ball));
        for n in 1..=cap {
            g.place_monster(GreenSlime, ball.pos.x + 1.);
            let id = g.monsters.last().unwrap().id;
            assert_eq!(g.move_body(&mut ball), n < cap);
            if n < cap {
                assert_eq!(ball.passengers.last().unwrap().id, id);
                assert_eq!(ball.passengers.len(), n);
                assert!(g.monsters.is_empty() && ball.rotation > 0.);
            }
        }
        assert_eq!(g.monsters.len(), cap);
        assert!(g
            .monsters
            .iter()
            .all(|m| m.hp == m.kind.hp() && m.dizzy > 0 && m.vertical_speed > 0.));
        assert_eq!(g.juice_zones[0].kind, Puddle::Slime);
    }
}

#[test]
fn katamari_bottom_clears_cargo_but_heavy_contact_releases_it() {
    for off_edge in [true, false] {
        let mut g = isolated_sandbox();
        let mut ball = toy(Katamari, Stage::Giant, 444.);
        ball.rolling = true;
        g.place_monster(GreenSlime, 444.);
        assert!(g.move_body(&mut ball));
        if off_edge {
            ball.pos.x = 504.;
            assert!(!g.move_body(&mut ball));
            assert_eq!(g.stats.fruits[Katamari as usize].kills, 1);
            assert!(g.monsters.is_empty());
        } else {
            g.place_monster(Shield, ball.pos.x);
            assert!(!g.move_body(&mut ball));
            assert_eq!(g.monsters[0].lid_hp, 0);
            assert_eq!(g.monsters.len(), 2);
        }
    }
}

#[test]
fn pinata_parking_shattering_and_home_run_scatter_edible_apples() {
    for stage in [Stage::Mature, Stage::Giant] {
        for cause in 0..3 {
            let mut g = isolated_sandbox();
            let mut shell = toy(Pinata, stage, 140.);
            match cause {
                0 => {
                    shell.rolling = true;
                    g.bodies.push(shell);
                    let pos = g.bodies[0].pos;
                    assert!(g.trigger_home_run(pos));
                }
                1 => {
                    g.shatter(&shell);
                }
                _ => {
                    shell.rolling = true;
                    let hits = if stage == Stage::Giant { 3 } else { 1 };
                    for _ in 0..hits {
                        g.place_monster(IronBoar, shell.pos.x);
                    }
                    g.bodies.push(shell);
                    g.step();
                }
            }
            assert!(g.barricades.is_empty());
            assert!(g
                .bodies
                .iter()
                .all(|b| b.kind == RedApple && b.angled && b.velocity.y < 0.));
            assert!((if stage == Stage::Giant { 6..=7 } else { 4..=5 }).contains(&g.bodies.len()));
            assert!(
                g.bodies.first().unwrap().velocity.x < 0.
                    && g.bodies.last().unwrap().velocity.x > 0.
            );
            g.set_hellhound(true);
            let mouth = g.hellhound.unwrap().pos;
            let mut apple = g.bodies.pop().unwrap();
            apple.pos = mouth;
            assert!(g.feed_companion(&apple, mouth));
            assert_eq!(g.stats.assists.zoomies, 1);
            if stage == Stage::Giant {
                let mut healing = g.bodies.remove(0);
                let y = ground(CHIEF_X) - 22. - healing.radius();
                healing.pos = Point::new(100., y + 1.);
                healing.velocity.y = 50.;
                g.protagonist.hp = 3;
                assert!(matches!(
                    g.protagonist.interact(&healing, Point::new(100., y - 1.)),
                    Some(VfxEvent::ChiefHeal(_))
                ));
            }
        }
    }
}

#[test]
fn quake_impacts_launch_nearby_monsters_and_dent_or_remove_lids() {
    for (stage, lid) in [(Stage::Mature, 2), (Stage::Giant, 0)] {
        let mut g = isolated_sandbox();
        g.place_monster(Shield, 245.);
        g.place_monster(IronBoar, 365.);
        let mut hammer = toy(Quake, stage, 218.);
        assert!(g.move_body(&mut hammer));
        assert!(hammer.velocity.y < 0. && !hammer.rolling);
        assert_eq!(g.monsters[0].lid_hp, lid);
        g.move_monsters();
        assert!(g.monsters[0].lift > 0. && g.monsters[1].lift == 0.);
        for _ in 0..180 {
            if !g.move_body(&mut hammer) {
                break;
            }
        }
        assert!(
            g.events
                .iter()
                .filter(|e| matches!(e, VfxEvent::Quake(..)))
                .count()
                >= 2
        );
    }
}

#[test]
fn bubble_carries_monster_to_chime_then_flings_it_after_two_or_three_landings() {
    for (stage, kind, landings) in [(Stage::Mature, GreenSlime, 2), (Stage::Giant, IronBoar, 3)] {
        let mut g = isolated_sandbox();
        g.slots.fill(None);
        g.slots[2] = Some(SlotItem::Chime { cooldown: 0 });
        g.place_monster(kind, SLOTS[2]);
        let mut bubble = toy(Bubble, stage, SLOTS[2]);
        assert!(g.move_body(&mut bubble));
        assert_eq!(bubble.passengers[0].kind, kind);
        assert!(g.monsters.is_empty() && bubble.velocity.y < -450.);
        let mut popped = false;
        for _ in 0..800 {
            if !g.move_body(&mut bubble) {
                popped = true;
                break;
            }
        }
        assert!(popped);
        assert!(g.events.iter().any(|e| matches!(e, VfxEvent::ChimeRing(_))));
        assert_eq!(bubble.capture_bounces, landings);
        assert_eq!(g.monsters[0].kind, kind);
        assert!(g.monsters[0].dizzy > 0 && g.monsters[0].knockback > 0.);
        assert_eq!(g.juice_zones[0].kind, Puddle::Slime);
    }
}

#[test]
fn candy_lands_groups_both_sides_and_runs_out_of_bites_without_doing_damage() {
    for (stage, bites, radius) in [(Stage::Mature, 5, 76.), (Stage::Giant, 8, 112.)] {
        let mut g = isolated_sandbox();
        let mut candy = toy(Candy, stage, 250.);
        assert!(!g.move_body(&mut candy));
        assert_eq!((g.baits[0].bites, g.baits[0].radius), (bites, radius));
        g.place_monster(IronBoar, 215.);
        g.place_monster(Shield, 285.);
        for _ in 0..105 {
            g.move_monsters();
        }
        assert!(g
            .monsters
            .iter()
            .all(|m| m.hungry && (m.pos.x - 250.).abs() < 10.));
        for _ in 0..400 {
            g.move_monsters();
        }
        assert!(g.baits.is_empty());
        assert_eq!(
            g.events
                .iter()
                .filter(|e| matches!(e, VfxEvent::CandyBite(_)))
                .count(),
            usize::from(bites)
        );
        assert!(g.monsters.iter().all(|m| m.hp == m.kind.hp() && !m.hungry));
    }
}

#[test]
fn header_recovers_both_spent_hybrids_without_hitting_monsters() {
    for kind in [Quake, Bubble] {
        let mut g = isolated_sandbox();
        g.set_hellhound(true);
        let x = g.hellhound.unwrap().pos.x + 8.;
        let mut b = toy(kind, Stage::Mature, x);
        b.energy_ratio = 0.1;
        g.bodies.push(b);
        g.place_monster(GreenSlime, x + 31.);
        g.update_companion();
        assert_eq!(g.stats.assists.melon_headers, 1);
        assert!(g.bodies[0].velocity.y < -450.);
        assert_eq!(g.monsters[0].hp, GreenSlime.hp());
    }
}

#[test]
fn hybrid_pools_become_oil_and_multiball_detonates_for_double_fruit_damage() {
    for kind in [Katamari, Bubble, Candy] {
        let mut g = isolated_sandbox();
        g.slots.fill(None);
        let mut b = toy(kind, Stage::Mature, 260.);
        match kind {
            Candy => {
                assert!(!g.move_body(&mut b));
            }
            Bubble => {
                b.energy_ratio = 0.1;
                assert!(!g.move_body(&mut b));
            }
            Katamari => {
                b.rolling = true;
                g.place_monster(IronBoar, 260.);
                assert!(!g.move_body(&mut b));
                g.monsters.clear();
            }
            _ => unreachable!(),
        }
        g.place_monster(IronBoar, 260.);
        g.elapsed = 299;
        g.step();
        assert_eq!(g.juice_zones[0].kind, Puddle::Oil);
        assert_eq!(g.monsters[0].hp, IronBoar.hp());
        let mut marble = toy(Pinball, Stage::Mature, 260.);
        g.move_body(&mut marble);
        assert_eq!(g.monsters[0].hp, IronBoar.hp() - 4);
        assert_eq!(g.stats.fruits[Pinball as usize].damage, 4);
        assert_eq!(g.stats.assists.bonus_damage, 2);
    }
}

#[test]
fn cut_pinball_cluster_can_ring_a_neighboring_chime() {
    let mut g = isolated_sandbox();
    g.slots.fill(None);
    g.slots[2] = Some(SlotItem::Fruit(Fruit {
        kind: Pinball,
        age: Pinball.mature_ticks(),
    }));
    g.slots[3] = Some(SlotItem::Chime { cooldown: 0 });
    assert!(g.cut_fruit(2));
    for _ in 0..170 {
        g.step();
    }
    assert!(g.events.iter().any(|e| matches!(e, VfxEvent::ChimeRing(_))));
}

#[test]
fn a_giant_pinata_apple_can_reach_the_porch_under_its_own_trajectory() {
    let mut g = isolated_sandbox();
    g.slots.fill(None);
    g.protagonist.hp = 3;
    let shell = toy(Pinata, Stage::Giant, 185.);
    g.apple_rain(&shell);
    for _ in 0..160 {
        g.step();
    }
    assert_eq!(g.protagonist.hp, 4);
    assert!(g.events.iter().any(|e| matches!(e, VfxEvent::ChiefHeal(_))));
}
