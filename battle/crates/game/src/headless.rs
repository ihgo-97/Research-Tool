//! `--headless`: run many AI-vs-AI battles without a window and print stats.

use combat::data::CreatureId;
use combat::rng::Rng;
use combat::{BattleSummary, GameData, Outcome, TeamSummary};
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;

use crate::assets::AssetDb;

pub struct HeadlessOpts {
    pub per_pair: usize,
    pub matchup: Option<(CreatureId, CreatureId)>,
    /// Creatures per side; above 1 runs random team battles instead of 1v1 pairs.
    pub team_size: usize,
    pub ai: String,
    /// AI profile for the right-hand side (defaults to `ai`).
    pub ai_b: Option<String>,
    pub seed: u64,
    /// Also print per-move usage and hit rates.
    pub moves: bool,
}

struct Job {
    a: CreatureId,
    b: CreatureId,
    seed: u64,
}

fn workers() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
}

/// Run `f` over `jobs` on all cores, isolating panics: each job yields its
/// result or a crash message.
fn run_parallel<J: Sync, T: Send>(
    jobs: &[J],
    f: impl Fn(&J) -> T + Sync,
    describe: impl Fn(&J) -> String + Sync,
) -> Vec<Result<T, String>> {
    // Silence the default panic printer; crashes are counted and reported.
    let prev_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let threads = workers();
    let results = std::thread::scope(|s| {
        let handles: Vec<_> = jobs
            .chunks(jobs.len().div_ceil(threads).max(1))
            .map(|chunk| {
                let (f, describe) = (&f, &describe);
                s.spawn(move || {
                    chunk
                        .iter()
                        .map(|j| {
                            panic::catch_unwind(AssertUnwindSafe(|| f(j))).map_err(|e| {
                                let msg = e
                                    .downcast_ref::<String>()
                                    .cloned()
                                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                                    .unwrap_or_else(|| "unknown panic".into());
                                format!("{}: {msg}", describe(j))
                            })
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().expect("worker thread")).collect()
    });
    panic::set_hook(prev_hook);
    results
}

struct TeamJob {
    rosters: [Vec<CreatureId>; 2],
    seed: u64,
}

/// Random team battles; per-creature stats are "win rate of teams it was on".
fn run_teams(data: Arc<GameData>, assets: &AssetDb, opts: &HeadlessOpts) -> bool {
    let n = data.creatures.len();
    let k = opts.team_size.min(n);
    let total = opts.per_pair * n * n;
    let mut rng = Rng::new(opts.seed);
    let mut pick = || {
        let mut pool: Vec<CreatureId> = (0..n).collect();
        let mut team = Vec::new();
        for _ in 0..k {
            team.push(pool.swap_remove(rng.below(pool.len())));
        }
        team
    };
    let jobs: Vec<TeamJob> = (0..total)
        .map(|i| TeamJob {
            rosters: [pick(), pick()],
            seed: opts.seed ^ (i as u64).wrapping_mul(0x9E37_79B9),
        })
        .collect();
    let started = std::time::Instant::now();
    let ai = opts.ai.clone();
    let ai_b = opts.ai_b.clone().unwrap_or_else(|| ai.clone());
    let results: Vec<Result<TeamSummary, String>> = run_parallel(
        &jobs,
        |j| combat::run_ai_team_battle(&data, j.rosters.clone(), [&ai, &ai_b], j.seed),
        |j| format!("{:?} vs {:?} seed {}", j.rosters[0], j.rosters[1], j.seed),
    );
    let elapsed = started.elapsed().as_secs_f32();

    let name = |c: CreatureId| assets.creature_name(&data.creatures[c].asset).to_string();
    let crashes: Vec<&String> = results.iter().filter_map(|r| r.as_ref().err()).collect();
    let ok: Vec<&TeamSummary> = results.iter().filter_map(|r| r.as_ref().ok()).collect();
    let mut games = vec![0usize; n];
    let mut wins = vec![0usize; n];
    let (mut ko, mut timeouts, mut draws) = (0, 0, 0);
    let mut side_wins = [0usize; 2];
    let mut switches = 0u32;
    let mut lengths: Vec<f32> = Vec::new();
    let mut winner_lost = 0usize;
    for s in &ok {
        lengths.push(s.seconds);
        match s.outcome {
            Outcome::Victory { .. } => ko += 1,
            Outcome::Timeout { .. } => timeouts += 1,
        }
        let w = s.outcome.winner();
        if let Some(w) = w {
            side_wins[w as usize] += 1;
            winner_lost += s.fainted[w as usize];
        } else {
            draws += 1;
        }
        switches += s.switches[0] + s.switches[1];
        for side in 0..2 {
            for &c in &s.rosters[side] {
                games[c] += 1;
                if w == Some(side as u8) {
                    wins[c] += 1;
                }
            }
        }
    }
    lengths.sort_by(f32::total_cmp);
    let pct = |a: usize, b: usize| if b == 0 { 0.0 } else { 100.0 * a as f32 / b as f32 };
    let ai_desc = match &opts.ai_b {
        Some(b) => format!("left '{}' vs right '{b}'", opts.ai),
        None => format!("'{}'", opts.ai),
    };
    println!(
        "== headless teams: {} battles of {k}v{k}, AI {ai_desc}, {} threads, {:.1}s wall",
        results.len(),
        workers(),
        elapsed
    );
    println!("crashes: {}   KOs: {}   timeouts: {}   draws: {}", crashes.len(), ko, timeouts, draws);
    if !lengths.is_empty() {
        println!(
            "fight length: avg {:.1}s  min {:.1}s  median {:.1}s  max {:.1}s",
            lengths.iter().sum::<f32>() / lengths.len() as f32,
            lengths[0],
            lengths[lengths.len() / 2],
            lengths[lengths.len() - 1]
        );
    }
    let games_n = ok.len().max(1) as f32;
    println!(
        "side bias: left {:.1}%  right {:.1}%   switches/battle {:.2}   winner's fainted {:.2}",
        pct(side_wins[0], ok.len()),
        pct(side_wins[1], ok.len()),
        switches as f32 / games_n,
        winner_lost as f32 / games_n
    );
    println!();
    println!("{:<10} {:>6} {:>12}", "creature", "games", "team win%");
    let mut order: Vec<usize> = (0..n).filter(|&c| games[c] > 0).collect();
    order.sort_by(|&a, &b| pct(wins[b], games[b]).total_cmp(&pct(wins[a], games[a])));
    for c in order {
        println!("{:<10} {:>6} {:>11.1}%", name(c), games[c], pct(wins[c], games[c]));
    }
    for c in crashes.iter().take(10) {
        println!("CRASH: {c}");
    }
    crashes.is_empty()
}

/// Returns false if any battle crashed.
pub fn run(data: Arc<GameData>, assets: &AssetDb, opts: &HeadlessOpts) -> bool {
    if opts.team_size > 1 {
        return run_teams(data, assets, opts);
    }
    let n = data.creatures.len();
    let pairs: Vec<(CreatureId, CreatureId)> = match opts.matchup {
        Some(p) => vec![p],
        None => (0..n).flat_map(|a| (0..n).map(move |b| (a, b))).collect(),
    };
    let mut jobs = Vec::new();
    for &(a, b) in &pairs {
        for k in 0..opts.per_pair {
            let seed = opts.seed ^ ((a as u64) << 40) ^ ((b as u64) << 32) ^ k as u64;
            jobs.push(Job { a, b, seed });
        }
    }

    let started = std::time::Instant::now();
    let threads = workers();
    let ai = opts.ai.clone();
    let ai_b = opts.ai_b.clone().unwrap_or_else(|| ai.clone());
    let results: Vec<Result<BattleSummary, String>> = run_parallel(
        &jobs,
        |j| combat::run_ai_battle_with(&data, j.a, j.b, [&ai, &ai_b], j.seed),
        |j| format!("{} vs {} seed {}", j.a, j.b, j.seed),
    );
    let elapsed = started.elapsed().as_secs_f32();

    let name = |c: CreatureId| assets.creature_name(&data.creatures[c].asset).to_string();
    let crashes: Vec<&String> = results.iter().filter_map(|r| r.as_ref().err()).collect();
    let ok: Vec<&BattleSummary> = results.iter().filter_map(|r| r.as_ref().ok()).collect();

    let mut wins = vec![0usize; n];
    let mut games = vec![0usize; n];
    let mut hp_in_wins = vec![0.0f32; n];
    let mut dmg = vec![0.0f32; n];
    let mut secs = vec![0.0f32; n];
    let mut dodges = vec![0u32; n];
    let mut matrix = vec![vec![(0usize, 0usize); n]; n]; // (wins of row, games)
    let (mut ko, mut timeouts, mut draws) = (0, 0, 0);
    let mut side_wins = [0usize; 2];
    let mut lengths: Vec<f32> = Vec::new();

    for s in &ok {
        lengths.push(s.seconds);
        match s.outcome {
            Outcome::Victory { .. } => ko += 1,
            Outcome::Timeout { .. } => timeouts += 1,
        }
        let winner = s.outcome.winner();
        if winner.is_none() {
            draws += 1;
        }
        for side in 0..2 {
            let c = s.creatures[side];
            games[c] += 1;
            dmg[c] += s.stats[side].damage_dealt;
            secs[c] += s.seconds;
            dodges[c] += s.stats[side].dodges;
            let other = s.creatures[1 - side];
            matrix[c][other].1 += 1;
            if winner == Some(side as u8) {
                wins[c] += 1;
                hp_in_wins[c] += s.hp_left[side];
                matrix[c][other].0 += 1;
            }
        }
        if let Some(w) = winner {
            side_wins[w as usize] += 1;
        }
    }

    lengths.sort_by(f32::total_cmp);
    let avg_len = lengths.iter().sum::<f32>() / lengths.len().max(1) as f32;
    let pct = |a: usize, b: usize| if b == 0 { 0.0 } else { 100.0 * a as f32 / b as f32 };

    let ai_desc = match &opts.ai_b {
        Some(b) => format!("left '{}' vs right '{b}'", opts.ai),
        None => format!("'{}'", opts.ai),
    };
    println!("== headless: {} battles, AI {ai_desc}, {} threads, {:.1}s wall", results.len(), threads, elapsed);
    println!(
        "crashes: {}   KOs: {}   timeouts: {}   draws: {}",
        crashes.len(),
        ko,
        timeouts,
        draws
    );
    if !lengths.is_empty() {
        println!(
            "fight length: avg {:.1}s  min {:.1}s  median {:.1}s  max {:.1}s",
            avg_len,
            lengths[0],
            lengths[lengths.len() / 2],
            lengths[lengths.len() - 1]
        );
    }
    println!(
        "side bias: left {:.1}%  right {:.1}%",
        pct(side_wins[0], ok.len()),
        pct(side_wins[1], ok.len())
    );
    println!();
    println!(
        "{:<10} {:>6} {:>7} {:>9} {:>8} {:>8}",
        "creature", "games", "win%", "hp@win", "dps", "dodge/g"
    );
    let mut order: Vec<usize> = (0..n).filter(|&c| games[c] > 0).collect();
    order.sort_by(|&a, &b| pct(wins[b], games[b]).total_cmp(&pct(wins[a], games[a])));
    for &c in &order {
        println!(
            "{:<10} {:>6} {:>6.1}% {:>8.0}% {:>8.2} {:>8.2}",
            name(c),
            games[c],
            pct(wins[c], games[c]),
            if wins[c] > 0 { 100.0 * hp_in_wins[c] / wins[c] as f32 } else { 0.0 },
            dmg[c] / secs[c].max(1e-3),
            dodges[c] as f32 / games[c] as f32,
        );
    }
    if opts.matchup.is_none() {
        println!();
        println!("matchups (row win% vs column, mirrors included):");
        print!("{:<10}", "");
        for c in 0..n {
            print!(" {:>8.8}", name(c));
        }
        println!();
        for (r, row) in matrix.iter().enumerate() {
            print!("{:<10}", name(r));
            for &(w, g) in row {
                print!(" {:>7.0}%", pct(w, g));
            }
            println!();
        }
    }
    if opts.moves {
        let mut ms = vec![(0u32, 0u32, 0.0f32); data.moves.len()];
        for s in &ok {
            for (i, m) in s.move_stats.iter().enumerate() {
                ms[i].0 += m.0;
                ms[i].1 += m.1;
                ms[i].2 += m.2;
            }
        }
        println!();
        println!("{:<13} {:>7} {:>6} {:>8} {:>9}", "move", "uses", "hit%", "dmg/use", "dmg/hit");
        for (i, m) in data.moves.iter().enumerate() {
            let (u, h, d) = ms[i];
            if u == 0 {
                continue;
            }
            // Multi-projectile moves can exceed 100% (several hits per use).
            println!(
                "{:<13} {:>7} {:>5.0}% {:>8.1} {:>9.1}",
                assets.move_name(&m.asset),
                u,
                100.0 * h as f32 / u as f32,
                d / u as f32,
                if h > 0 { d / h as f32 } else { 0.0 }
            );
        }
    }
    for c in crashes.iter().take(10) {
        println!("CRASH: {c}");
    }
    crashes.is_empty()
}
