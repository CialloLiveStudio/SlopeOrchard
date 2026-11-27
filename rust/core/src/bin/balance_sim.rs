use slope_orchard_core::balance::{run, seconds, warnings, Config, Outcome, CONFIGS, SEED_COUNT};
use slope_orchard_core::{balance::Strategy, FruitKind, LevelId, Phase};

fn percent(part: u32, total: u32) -> f32 {
    100. * part as f32 / total.max(1) as f32
}

fn report(config: Config, outcomes: &[Outcome]) -> (f32, usize) {
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
    let totals: [(u32, u32); FruitKind::COUNT] = std::array::from_fn(|source| {
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
    let hybrid = totals[4..]
        .iter()
        .fold((0, 0), |(d, k), (next_d, next_k)| (d + next_d, k + next_k));
    let hybrid = format!(
        "{:.1}/{:.1}",
        percent(hybrid.0, damage_total),
        percent(hybrid.1, kill_total)
    );
    let columns = shares.map(|(damage, kills)| format!("{damage:.1}/{kills:.1}"));
    let assists: [u32; 6] = outcomes.iter().fold([0; 6], |mut sums, outcome| {
        let a = outcome.assists;
        for (sum, n) in sums.iter_mut().zip([
            a.shoves,
            a.pumpkin_headers,
            a.melon_headers,
            a.zoomies,
            a.detonations,
            a.bonus_damage,
        ]) {
            *sum += n;
        }
        sums
    });
    println!(
        "{:?} | {:11} | {:6.1}% | {:11.2} | {:>16} | {:>21} | {:>11} | {:>11} | {:>11} | {:>11} | {:>11} | {:?}",
        config.level,
        config.strategy.name(),
        win_rate,
        hp,
        first_damage_time,
        peak,
        columns[0],
        columns[1],
        columns[2],
        columns[3], hybrid, assists,
    );

    let mut issues = warnings(config, win_rate, hp, shares[..4].try_into().unwrap());
    if matches!(
        config.strategy,
        Strategy::Skilled | Strategy::OrchardMagnet | Strategy::AppleRain | Strategy::BubbleArcade
    ) {
        if let Some(run) = outcomes
            .iter()
            .find(|o| o.first_damage_tick.is_some_and(|t| t < 15 * 120))
        {
            issues.push(format!("Damage before 15s on seed {}", run.seed));
        }
    }
    if outcomes
        .iter()
        .any(|o| !matches!(o.phase, Phase::Victory | Phase::Defeat))
    {
        issues.push("Simulation timed out".into());
    }
    if config.level == LevelId::T5 && (assists[4] == 0 || assists[5] == 0) {
        issues.push("No fragile oil burst contribution".into());
    }
    let examples = outcomes
        .iter()
        .filter(|o| o.phase != Phase::Victory || o.hp <= 1)
        .take(3)
        .map(|o| format!("{} ({:?}, {} HP)", o.seed, o.phase, o.hp))
        .collect::<Vec<_>>();
    if !examples.is_empty() {
        println!("   Seeds {}", examples.join(", "));
    }
    for issue in &issues {
        println!("   [WARN] {issue}");
    }
    (hp, issues.len())
}

fn main() {
    println!(
        "SlopeOrchard balance | {SEED_COUNT} seeds per config (0–{})",
        SEED_COUNT - 1
    );
    println!("Damage/kill columns are percent. First damage is to the protagonist (number of affected runs).");
    println!("Lv | Strategy    | WinRate | AvgRemainHP | AvgFirstDmgTime  | MaxConcurrentMonsters |   Apple D/K |   Melon D/K | Pumpkin D/K |    Berry D/K | Hybrid D/K | Assists [shoves, pumpkin headers, melon headers, zoomies, detonations, bonus HP]");
    let mut mixed_hp = 0.;
    let mut pumpkin_hp = 0.;
    let mut violations = 0;
    for config in CONFIGS {
        let outcomes = (0..SEED_COUNT)
            .map(|seed| run(config, seed))
            .collect::<Vec<_>>();
        let (hp, issues) = report(config, &outcomes);
        violations += issues;
        if config.level == LevelId::T2 {
            match config.strategy {
                Strategy::Skilled => mixed_hp = hp,
                Strategy::CheeseAuditor(FruitKind::BowlingPumpkin) => pumpkin_hp = hp,
                _ => {}
            }
        }
    }
    if mixed_hp - pumpkin_hp < 1.0 {
        println!("   [WARN] T2 All-Pumpkin must finish at least 1 HP below mixed Skilled");
        violations += 1;
    }
    if violations != 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_sim_smoke() {
        assert!(SEED_COUNT >= 128);
        for config in CONFIGS
            .into_iter()
            .filter(|c| c.level == LevelId::T5 || c.strategy == Strategy::Skilled)
        {
            let first = run(config, 7);
            let repeat = run(config, 7);
            assert!(matches!(first.phase, Phase::Victory | Phase::Defeat));
            assert!(first.first_damage_tick.is_none_or(|t| t >= 15 * 120));
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
