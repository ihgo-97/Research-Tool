//! Presentation tuning ("game feel") loaded from `data/feel.ron`. These values
//! change how a fight looks and responds, never the simulation's numbers.

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct HitStop {
    /// Hits dealing less than this cause no freeze.
    pub min_damage: f32,
    pub per_damage: f32,
    pub max: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Shake {
    pub player_hit: f32,
    pub ko: f32,
    pub decay: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ParticleCounts {
    pub hit: u32,
    pub dodge: u32,
    pub dash: u32,
    pub ko: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Feel {
    pub hitstop: HitStop,
    pub shake: Shake,
    pub flash_seconds: f32,
    pub particles: ParticleCounts,
    /// How long an early dash press is remembered.
    pub dash_buffer: f32,
}

impl Feel {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let p = dir.join("feel.ron");
        let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        ron::from_str(&text).map_err(|e| format!("{}: {e}", p.display()))
    }

    /// Freeze duration for a hit of `damage`.
    pub fn hitstop_for(&self, damage: f32) -> f32 {
        let h = &self.hitstop;
        if damage < h.min_damage {
            0.0
        } else {
            (damage * h.per_damage).min(h.max)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_feel_loads_and_hitstop_is_bounded() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let f = Feel::load(&dir).unwrap();
        assert_eq!(f.hitstop_for(0.0), 0.0);
        assert!(f.hitstop_for(1000.0) <= f.hitstop.max);
        assert!(f.hitstop_for(f.hitstop.min_damage + 10.0) > 0.0);
    }
}
