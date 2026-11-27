use slope_orchard_core::balance::{run, seconds, Config, Outcome, Strategy, CONFIGS, SEED_COUNT};
use slope_orchard_core::{FruitKind, LevelId, Phase};

fn percent(part: u32, total: u32) -> f32 {
    100. * part as f32 / total.max(1) as f32
}

fn report(config: Config, outcomes: &[Outcome]) {
    let count = outcomes.len() as u32;
    let wins = outcomes
        .iter()
        .filter(|outcome| outcome.phase == Phase::Victory)
        .count() as u32;
    let win_rate = percent(wins, count);
    let hp = outcomes
        .iter()
        .map(|outcome| f32::from(outcome.hp))
        .sum::<f32>()
        / count as f32;
    let first_damage: Vec<_> = outcomes
        .iter()
        .filter_map(|outcome| outcome.first_damage_tick)
        .collect();
    let first_damage_time = if first_damage.is_empty() {
        "—".into()
    } else {
        let sum: f32 = first_damage.iter().copied().map(seconds).sum();
        format!(
            "{:.1}s ({})",
            sum / first_damage.len() as f32,
            first_damage.len()
        )
    };
    let peak = outcomes
        .iter()
        .map(|outcome| outcome.max_concurrent_monsters)
        .max()
        .unwrap_or(0);
    let totals: [(u32, u32); 4] = std::array::from_fn(|source| {
        outcomes
            .iter()
            .map(|outcome| outcome.contributions[source])
            .fold((0, 0), |(damage, kills), next| {
                (damage + next.damage, kills + next.kills)
            })
    });
    let damage_total = totals.iter().map(|(damage, _)| damage).sum();
    let kill_total = totals.iter().map(|(_, kills)| kills).sum();
    let shares =
        totals.map(|(damage, kills)| (percent(damage, damage_total), percent(kills, kill_total)));
    let columns = shares.map(|(damage, kills)| format!("{damage:.1}/{kills:.1}"));
    println!(
        "{:?} | {:11} | {:6.1}% | {:11.2} | {:>16} | {:>21} | {:>11} | {:>11} | {:>11} | {:>11}",
        config.level,
        config.strategy.name(),
        win_rate,
        hp,
        first_damage_time,
        peak,
        columns[0],
        columns[1],
        columns[2],
        columns[3],
    );

    if shares[3].1 > 35. {
        println!(
            "   [WARN] Hellhound kill share {:.1}% exceeds 35%",
            shares[3].1
        );
    }
    if config.level == LevelId::T2 {
        if !matches!(config.strategy, Strategy::CheeseAuditor(_)) {
            for (name, (damage, _)) in ["Apple", "Melon", "Pumpkin"].into_iter().zip(shares) {
                if damage > 70. {
                    println!("   [WARN] T2 mixed {name} damage share {damage:.1}% exceeds 70%");
                }
            }
        }
        if config.strategy == Strategy::CheeseAuditor(FruitKind::BowlingPumpkin) && win_rate > 35. {
            println!("   [WARN] T2 All-Pumpkin win rate {win_rate:.1}% exceeds 35%");
        }
        if config.strategy == Strategy::Casual && win_rate > 85. {
            println!("   [WARN] T2 Casual win rate {win_rate:.1}% exceeds 85%");
        }
    }
    if config.strategy == Strategy::Skilled && win_rate < 85. {
        println!("   [WARN] Skilled win rate {win_rate:.1}% is below 85%");
    }
    let examples = outcomes
        .iter()
        .filter(|outcome| {
            outcome.phase != Phase::Victory
                || outcome.hp <= 1
                || percent(
                    outcome.contributions[3].kills,
                    outcome.contributions.iter().map(|c| c.kills).sum(),
                ) > 35.
        })
        .take(3)
        .map(|outcome| {
            let result = match outcome.phase {
                Phase::Victory => "close win",
                Phase::Defeat => "defeat",
                _ => "timeout",
            };
            let first = outcome
                .first_damage_tick
                .map_or("—".to_owned(), |tick| format!("{:.1}s", seconds(tick)));
            format!(
                "{} ({result}, {} HP, first damage {first}, peak {})",
                outcome.seed, outcome.hp, outcome.max_concurrent_monsters
            )
        })
        .collect::<Vec<_>>();
    if !examples.is_empty() {
        println!("   Seeds {}", examples.join(", "));
    }
}

fn main() {
    println!(
        "SlopeOrchard balance | {SEED_COUNT} seeds per config (0–{})",
        SEED_COUNT - 1
    );
    println!("Damage/kill columns are percent. First damage is to the protagonist (number of affected runs).");
    println!("Lv | Strategy    | WinRate | AvgRemainHP | AvgFirstDmgTime  | MaxConcurrentMonsters |   Apple D/K |   Melon D/K | Pumpkin D/K |    Hound D/K");
    for config in CONFIGS {
        let outcomes = (0..SEED_COUNT)
            .map(|seed| run(config, seed))
            .collect::<Vec<_>>();
        report(config, &outcomes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_sim_smoke() {
        assert!(SEED_COUNT >= 50);
        for level in [LevelId::T1, LevelId::T2] {
            let config = Config {
                level,
                strategy: Strategy::Skilled,
            };
            let first = run(config, 7);
            let repeat = run(config, 7);
            assert!(matches!(first.phase, Phase::Victory | Phase::Defeat));
            assert_eq!(first.phase, repeat.phase);
            assert_eq!(first.hp, repeat.hp);
            assert_eq!(first.first_damage_tick, repeat.first_damage_tick);
            assert_eq!(
                first.max_concurrent_monsters,
                repeat.max_concurrent_monsters
            );
            let contributions =
                |outcome: Outcome| outcome.contributions.map(|c| (c.damage, c.kills));
            assert_eq!(contributions(first), contributions(repeat));
            assert!(first.max_concurrent_monsters > 0);
            report(config, &[first, repeat]);
        }
    }
}
