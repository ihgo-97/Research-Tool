//! Presentation assets (names, colours, future sprite/sound paths), keyed by
//! the `asset` ids used in `data/`. Swapping art never touches code or balance.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

pub type Rgb = (u8, u8, u8);

#[derive(Debug, Clone, Deserialize)]
pub struct TypeAsset {
    pub name: String,
    pub color: Rgb,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatureAsset {
    pub name: String,
    pub color: Rgb,
    /// Not drawn yet: placeholder shapes are used until real art lands.
    #[allow(dead_code)]
    #[serde(default)]
    pub sprite: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MoveAsset {
    pub name: String,
    /// Not played yet: audio is disabled until sounds exist.
    #[allow(dead_code)]
    #[serde(default)]
    pub sound: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetDb {
    pub types: HashMap<String, TypeAsset>,
    pub creatures: HashMap<String, CreatureAsset>,
    pub moves: HashMap<String, MoveAsset>,
}

const FALLBACK: Rgb = (160, 160, 160);

impl AssetDb {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let p = dir.join("assets.ron");
        let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        ron::from_str(&text).map_err(|e| format!("{}: {e}", p.display()))
    }

    /// Ids referenced by game data that have no asset entry.
    pub fn missing(&self, data: &combat::GameData) -> Vec<String> {
        let mut out = Vec::new();
        for t in &data.types.names {
            if !self.types.contains_key(t) {
                out.push(format!("type '{t}'"));
            }
        }
        for c in &data.creatures {
            if !self.creatures.contains_key(&c.asset) {
                out.push(format!("creature '{}'", c.asset));
            }
        }
        for m in &data.moves {
            if !self.moves.contains_key(&m.asset) {
                out.push(format!("move '{}'", m.asset));
            }
        }
        out
    }

    pub fn type_name<'a>(&'a self, id: &'a str) -> &'a str {
        self.types.get(id).map(|t| t.name.as_str()).unwrap_or(id)
    }

    pub fn type_color(&self, id: &str) -> Rgb {
        self.types.get(id).map(|t| t.color).unwrap_or(FALLBACK)
    }

    pub fn creature_name<'a>(&'a self, id: &'a str) -> &'a str {
        self.creatures.get(id).map(|c| c.name.as_str()).unwrap_or(id)
    }

    pub fn creature_color(&self, id: &str) -> Rgb {
        self.creatures.get(id).map(|c| c.color).unwrap_or(FALLBACK)
    }

    pub fn move_name<'a>(&'a self, id: &'a str) -> &'a str {
        self.moves.get(id).map(|m| m.name.as_str()).unwrap_or(id)
    }
}
