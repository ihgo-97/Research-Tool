//! Generic real-time combat core. No window, no input devices, no assets:
//! data in (RON), intents in, state and events out.

pub mod ai;
pub mod damage;
pub mod data;
pub mod hitbox;
pub mod math;
pub mod rng;
pub mod sim;

pub use data::GameData;
pub use math::Vec2;
pub use sim::{Battle, Event, Intent, Outcome};

use data::CreatureId;
use sim::FighterStats;
use std::sync::Arc;

/// Result of one AI-vs-AI battle.
#[derive(Debug, Clone)]
pub struct BattleSummary {
    pub creatures: [CreatureId; 2],
    pub outcome: Outcome,
    pub seconds: f32,
    pub hp_left: [f32; 2],
    pub stats: [FighterStats; 2],
    /// Per move id: (times launched, hits, damage dealt).
    pub move_stats: Vec<(u32, u32, f32)>,
}

/// Run a 1v1 battle with both sides AI-controlled until it ends.
pub fn run_ai_battle(data: &Arc<GameData>, a: CreatureId, b: CreatureId, profile: &str, seed: u64) -> BattleSummary {
    run_ai_battle_with(data, a, b, [profile, profile], 0, seed)
}

/// Result of one AI-vs-AI team battle.
#[derive(Debug, Clone)]
pub struct TeamSummary {
    pub rosters: [Vec<CreatureId>; 2],
    pub outcome: Outcome,
    pub seconds: f32,
    /// Voluntary switches per side.
    pub switches: [u32; 2],
    /// Fainted creatures per side.
    pub fainted: [usize; 2],
}

/// Run an AI-vs-AI team battle (rosters of any size) until it ends.
pub fn run_ai_team_battle(data: &Arc<GameData>, rosters: [Vec<CreatureId>; 2], profiles: [&str; 2], arena: usize, seed: u64) -> TeamSummary {
    let prof = |name: &str| {
        data.ai_profile(name)
            .unwrap_or_else(|| panic!("unknown AI profile '{name}'"))
            .clone()
    };
    let mut battle = Battle::with_arena(data.clone(), &rosters, arena, seed);
    let mut ais = [
        ai::AiController::new(prof(profiles[0]), seed.wrapping_mul(31).wrapping_add(1)),
        ai::AiController::new(prof(profiles[1]), seed.wrapping_mul(31).wrapping_add(2)),
    ];
    let cap = ((battle.time_limit() + 5.0) * data.balance.tick_rate as f32) as u64;
    while battle.outcome().is_none() && battle.tick() < cap {
        let intents = [ais[0].update(&battle, 0), ais[1].update(&battle, 1)];
        battle.step(&intents);
    }
    let fainted = |side: usize| {
        battle.teams()[side]
            .members
            .iter()
            .filter(|&&m| !battle.fighter(m).fighter.alive())
            .count()
    };
    TeamSummary {
        outcome: battle.outcome().unwrap_or(Outcome::Timeout { team: None }),
        seconds: battle.time(),
        switches: [battle.teams()[0].switches, battle.teams()[1].switches],
        fainted: [fainted(0), fainted(1)],
        rosters,
    }
}

/// Like [`run_ai_battle`] but each side can use a different AI profile.
pub fn run_ai_battle_with(data: &Arc<GameData>, a: CreatureId, b: CreatureId, profiles: [&str; 2], arena: usize, seed: u64) -> BattleSummary {
    let prof = |name: &str| {
        data.ai_profile(name)
            .unwrap_or_else(|| panic!("unknown AI profile '{name}'"))
            .clone()
    };
    let mut battle = Battle::with_arena(data.clone(), &[vec![a], vec![b]], arena, seed);
    let mut ais = [
        ai::AiController::new(prof(profiles[0]), seed.wrapping_mul(31).wrapping_add(1)),
        ai::AiController::new(prof(profiles[1]), seed.wrapping_mul(31).wrapping_add(2)),
    ];
    // Hard cap in ticks in case the outcome logic ever fails.
    let cap = ((data.balance.max_battle_seconds + 5.0) * data.balance.tick_rate as f32) as u64;
    let mut move_stats = vec![(0u32, 0u32, 0.0f32); data.moves.len()];
    while battle.outcome().is_none() && battle.tick() < cap {
        let intents = [ais[0].update(&battle, 0), ais[1].update(&battle, 1)];
        battle.step(&intents);
        for e in battle.events() {
            match *e {
                Event::MoveLaunched { mv, .. } => move_stats[mv].0 += 1,
                Event::Hit { mv, damage, .. } => {
                    move_stats[mv].1 += 1;
                    move_stats[mv].2 += damage;
                }
                _ => {}
            }
        }
    }
    let f0 = battle.fighter(0).fighter;
    let f1 = battle.fighter(1).fighter;
    BattleSummary {
        creatures: [a, b],
        outcome: battle.outcome().unwrap_or(Outcome::Timeout { team: None }),
        seconds: battle.time(),
        hp_left: [f0.hp / f0.max_hp, f1.hp / f1.max_hp],
        stats: [f0.stats, f1.stats],
        move_stats,
    }
}
