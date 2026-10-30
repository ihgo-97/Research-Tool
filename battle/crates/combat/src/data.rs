//! Data definitions loaded from RON files. Every balance number lives in `data/`.
//!
//! Files are parsed into `*Def` structs, then validated and resolved into
//! [`GameData`], which refers to types, moves and creatures by index.

use crate::math::Vec2;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

pub type TypeId = usize;
pub type MoveId = usize;
pub type CreatureId = usize;

#[derive(Debug)]
pub struct DataError(pub String);

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "data error: {}", self.0)
    }
}

impl std::error::Error for DataError {}

fn err<T>(msg: impl Into<String>) -> Result<T, DataError> {
    Err(DataError(msg.into()))
}

// ---------------------------------------------------------------- types.ron

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum Effect {
    Strong,
    Weak,
    Immune,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EffectMultipliers {
    pub strong: f32,
    pub weak: f32,
    /// "Immune" is softened to a multiplier so it is never a hard counter.
    pub immune: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TypeChartDef {
    pub types: Vec<String>,
    pub multipliers: EffectMultipliers,
    /// (attacking type, defending type, effect)
    pub matchups: Vec<(String, String, Effect)>,
}

#[derive(Debug, Clone)]
pub struct TypeChart {
    pub names: Vec<String>,
    pub multipliers: EffectMultipliers,
    /// `table[attacker][defender]`
    table: Vec<Vec<f32>>,
}

impl TypeChart {
    pub fn from_def(def: &TypeChartDef) -> Result<Self, DataError> {
        let n = def.types.len();
        let mut table = vec![vec![1.0; n]; n];
        let index = |name: &str| -> Result<usize, DataError> {
            def.types
                .iter()
                .position(|t| t == name)
                .ok_or_else(|| DataError(format!("type chart: unknown type '{name}'")))
        };
        for (a, d, e) in &def.matchups {
            let m = match e {
                Effect::Strong => def.multipliers.strong,
                Effect::Weak => def.multipliers.weak,
                Effect::Immune => def.multipliers.immune,
            };
            table[index(a)?][index(d)?] = m;
        }
        Ok(Self {
            names: def.types.clone(),
            multipliers: def.multipliers.clone(),
            table,
        })
    }

    pub fn id(&self, name: &str) -> Option<TypeId> {
        self.names.iter().position(|t| t == name)
    }

    pub fn single(&self, attacker: TypeId, defender: TypeId) -> f32 {
        self.table[attacker][defender]
    }

    /// Multiplier of an attacking type against a (possibly dual-typed) defender.
    pub fn multiplier(&self, attacker: TypeId, defender: &[TypeId]) -> f32 {
        defender.iter().map(|&d| self.single(attacker, d)).product()
    }
}

// ---------------------------------------------------------------- moves.ron

#[derive(Debug, Clone, Deserialize)]
pub enum MoveShape {
    /// Travels in a straight line; `count` > 1 fans out over `spread_deg`.
    Projectile {
        speed: f32,
        radius: f32,
        range: f32,
        count: u32,
        spread_deg: f32,
        pierce: bool,
    },
    /// Arc in front of the user that follows them while active.
    Melee {
        range: f32,
        arc_deg: f32,
        active: f32,
        /// Forward impulse applied to the user when the swing starts.
        lunge: f32,
    },
    /// Circle placed at the aim point (clamped to `max_distance`; 0 = on self),
    /// telegraphed for `delay` seconds before it becomes active.
    Burst {
        radius: f32,
        delay: f32,
        active: f32,
        max_distance: f32,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct MoveDefRaw {
    pub id: String,
    pub asset: String,
    #[serde(rename = "type")]
    pub element: String,
    pub power: f32,
    pub cooldown: f32,
    pub windup: f32,
    pub recovery: f32,
    pub knockback: f32,
    pub hitstun: f32,
    pub shape: MoveShape,
}

#[derive(Debug, Clone)]
pub struct MoveDef {
    pub id: String,
    pub asset: String,
    pub element: TypeId,
    pub power: f32,
    pub cooldown: f32,
    pub windup: f32,
    pub recovery: f32,
    pub knockback: f32,
    pub hitstun: f32,
    pub shape: MoveShape,
}

impl MoveDef {
    /// Distance from the user's centre at which this move can connect.
    pub fn reach(&self) -> f32 {
        match &self.shape {
            MoveShape::Projectile { range, .. } => *range,
            MoveShape::Melee { range, .. } => *range,
            MoveShape::Burst {
                radius,
                max_distance,
                ..
            } => max_distance + radius,
        }
    }
}

// ------------------------------------------------------------ creatures.ron

#[derive(Debug, Clone, Deserialize)]
pub struct CreatureDefRaw {
    pub id: String,
    pub asset: String,
    pub types: Vec<String>,
    pub hp: f32,
    pub attack: f32,
    pub defense: f32,
    pub speed: f32,
    pub radius: f32,
    pub moves: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CreatureDef {
    pub id: String,
    pub asset: String,
    pub types: Vec<TypeId>,
    pub hp: f32,
    pub attack: f32,
    pub defense: f32,
    pub speed: f32,
    pub radius: f32,
    pub moves: Vec<MoveId>,
}

// -------------------------------------------------------------- balance.ron

#[derive(Debug, Clone, Deserialize)]
pub struct DashDef {
    /// Multiplier on the creature's move speed.
    pub speed: f32,
    pub duration: f32,
    pub cooldown: f32,
    /// Seconds of invulnerability from the start of the dash.
    pub iframes: f32,
    pub cancels_recovery: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AiProfile {
    /// Seconds between decisions; also the reaction delay to new threats.
    pub reaction_time: f32,
    /// Probability of reacting to a detected incoming hit.
    pub dodge_chance: f32,
    /// How far ahead (seconds) incoming projectiles are considered threats.
    pub threat_lookahead: f32,
    /// Probability per decision of attacking when a move is usable.
    pub aggression: f32,
    /// Aim error (degrees, uniformly +/-).
    pub aim_error_deg: f32,
    /// 0 = aim at current position, 1 = full linear prediction.
    pub lead: f32,
    /// Tolerance around the preferred range before approaching/retreating.
    pub range_band: f32,
    /// Seconds between strafe-direction flips (randomised +/-50%).
    pub strafe_switch: f32,
    /// Fraction of max speed spent strafing at preferred range.
    pub strafe_amount: f32,
    /// Distance kept while every move is on cooldown.
    pub kite_range: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Balance {
    pub tick_rate: u32,
    pub max_battle_seconds: f32,
    pub damage_scale: f32,
    pub damage_variance: f32,
    /// Same-type attack bonus.
    pub stab: f32,
    /// Movement speed multiplier during windup/recovery.
    pub attack_move_mult: f32,
    /// How quickly velocity reaches its target (1/s).
    pub acceleration: f32,
    /// Exponential knockback decay rate (1/s).
    pub knockback_decay: f32,
    pub dash: DashDef,
    pub ai_profiles: HashMap<String, AiProfile>,
}

// ---------------------------------------------------------------- arena.ron

#[derive(Debug, Clone, Deserialize)]
pub struct Pillar {
    pub pos: Vec2,
    pub radius: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArenaDef {
    pub width: f32,
    pub height: f32,
    pub spawns: Vec<Vec2>,
    pub pillars: Vec<Pillar>,
}

// ------------------------------------------------------------- aggregation

#[derive(Debug, Clone)]
pub struct GameData {
    pub types: TypeChart,
    pub moves: Vec<MoveDef>,
    pub creatures: Vec<CreatureDef>,
    pub balance: Balance,
    pub arena: ArenaDef,
}

/// Raw file contents, before cross-reference resolution.
pub struct RawData {
    pub types: TypeChartDef,
    pub moves: Vec<MoveDefRaw>,
    pub creatures: Vec<CreatureDefRaw>,
    pub balance: Balance,
    pub arena: ArenaDef,
}

fn parse<T: serde::de::DeserializeOwned>(name: &str, text: &str) -> Result<T, DataError> {
    ron::from_str(text).map_err(|e| DataError(format!("{name}: {e}")))
}

fn read(dir: &Path, name: &str) -> Result<String, DataError> {
    let p = dir.join(name);
    std::fs::read_to_string(&p).map_err(|e| DataError(format!("{}: {e}", p.display())))
}

impl GameData {
    pub fn load_dir(dir: impl AsRef<Path>) -> Result<Self, DataError> {
        let dir = dir.as_ref();
        Self::from_strs(
            &read(dir, "types.ron")?,
            &read(dir, "moves.ron")?,
            &read(dir, "creatures.ron")?,
            &read(dir, "balance.ron")?,
            &read(dir, "arena.ron")?,
        )
    }

    pub fn from_strs(
        types: &str,
        moves: &str,
        creatures: &str,
        balance: &str,
        arena: &str,
    ) -> Result<Self, DataError> {
        Self::resolve(RawData {
            types: parse("types.ron", types)?,
            moves: parse("moves.ron", moves)?,
            creatures: parse("creatures.ron", creatures)?,
            balance: parse("balance.ron", balance)?,
            arena: parse("arena.ron", arena)?,
        })
    }

    pub fn resolve(raw: RawData) -> Result<Self, DataError> {
        let types = TypeChart::from_def(&raw.types)?;
        let ty = |name: &str, ctx: &str| {
            types
                .id(name)
                .ok_or_else(|| DataError(format!("{ctx}: unknown type '{name}'")))
        };

        let mut moves = Vec::new();
        for m in raw.moves {
            if moves.iter().any(|x: &MoveDef| x.id == m.id) {
                return err(format!("duplicate move id '{}'", m.id));
            }
            if m.cooldown <= 0.0 || m.power < 0.0 || m.windup < 0.0 || m.recovery < 0.0 {
                return err(format!("move '{}': non-positive timing/power", m.id));
            }
            if let MoveShape::Projectile { count, speed, .. } = &m.shape {
                if *count == 0 || *speed <= 0.0 {
                    return err(format!("move '{}': projectile needs count>0, speed>0", m.id));
                }
            }
            moves.push(MoveDef {
                element: ty(&m.element, &format!("move '{}'", m.id))?,
                id: m.id,
                asset: m.asset,
                power: m.power,
                cooldown: m.cooldown,
                windup: m.windup,
                recovery: m.recovery,
                knockback: m.knockback,
                hitstun: m.hitstun,
                shape: m.shape,
            });
        }

        let mut creatures = Vec::new();
        for c in raw.creatures {
            let ctx = format!("creature '{}'", c.id);
            if creatures.iter().any(|x: &CreatureDef| x.id == c.id) {
                return err(format!("duplicate creature id '{}'", c.id));
            }
            if c.types.is_empty() || c.types.len() > 2 {
                return err(format!("{ctx}: needs 1 or 2 types"));
            }
            if c.moves.is_empty() || c.moves.len() > 4 {
                return err(format!("{ctx}: needs 1-4 moves"));
            }
            if c.hp <= 0.0 || c.defense <= 0.0 || c.radius <= 0.0 {
                return err(format!("{ctx}: hp/defense/radius must be > 0"));
            }
            let mut ts = Vec::new();
            for t in &c.types {
                ts.push(ty(t, &ctx)?);
            }
            let mut ms = Vec::new();
            for m in &c.moves {
                ms.push(
                    moves
                        .iter()
                        .position(|x| &x.id == m)
                        .ok_or_else(|| DataError(format!("{ctx}: unknown move '{m}'")))?,
                );
            }
            creatures.push(CreatureDef {
                id: c.id,
                asset: c.asset,
                types: ts,
                hp: c.hp,
                attack: c.attack,
                defense: c.defense,
                speed: c.speed,
                radius: c.radius,
                moves: ms,
            });
        }

        if raw.arena.spawns.len() < 2 {
            return err("arena: needs at least 2 spawn points");
        }
        if raw.balance.tick_rate == 0 {
            return err("balance: tick_rate must be > 0");
        }
        if raw.balance.ai_profiles.is_empty() {
            return err("balance: needs at least one AI profile");
        }

        Ok(Self {
            types,
            moves,
            creatures,
            balance: raw.balance,
            arena: raw.arena,
        })
    }

    pub fn creature_id(&self, id: &str) -> Option<CreatureId> {
        self.creatures.iter().position(|c| c.id == id)
    }

    pub fn move_id(&self, id: &str) -> Option<MoveId> {
        self.moves.iter().position(|m| m.id == id)
    }

    pub fn ai_profile(&self, name: &str) -> Option<&AiProfile> {
        self.balance.ai_profiles.get(name)
    }

    pub fn dt(&self) -> f32 {
        1.0 / self.balance.tick_rate as f32
    }
}
