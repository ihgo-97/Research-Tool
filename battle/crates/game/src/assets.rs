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
    /// Sprite sheet path relative to the assets folder (layout: [`SheetLayout`]).
    #[serde(default)]
    pub sprite: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MoveAsset {
    pub name: String,
    /// Visual effect id (crates/game/src/vfx.rs); empty = generic.
    #[serde(default)]
    pub vfx: String,
    /// Played when the move launches (audio builds only).
    #[cfg_attr(not(feature = "audio"), allow(dead_code))]
    #[serde(default)]
    pub sound: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatusAsset {
    pub badge: String,
    pub inflicted: String,
    pub color: Rgb,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct SheetRows {
    pub idle: u32,
    pub walk: u32,
    pub attack: u32,
    pub hurt: u32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PixelRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SheetLayout {
    pub frame_size: u32,
    pub columns: u32,
    /// First row of each animation.
    pub rows: SheetRows,
    /// Rows per animation: 1 (single facing) or 3 (down, up, side).
    pub facings: u32,
    pub fps: f32,
    /// Game units per sprite pixel (integer keeps pixels crisp).
    pub pixel_scale: f32,
    /// Frame pixel drawn on the creature's position.
    pub anchor: (f32, f32),
    /// HUD portrait crop of the first idle (down) frame.
    pub portrait: PixelRect,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArenaArt {
    pub ground: String,
    pub obstacle: String,
    /// Pillar radius (game units) the obstacle art is drawn for.
    pub obstacle_radius: f32,
    /// Used by the sprite generator only.
    #[allow(dead_code)]
    pub snow: bool,
    pub border: Rgb,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetDb {
    pub sheet: SheetLayout,
    /// Battle-event sound ids (see `audio.rs`); None = silent.
    #[cfg_attr(not(feature = "audio"), allow(dead_code))]
    #[serde(default)]
    pub sounds: HashMap<String, Option<String>>,
    #[serde(default)]
    pub arena_art: HashMap<String, ArenaArt>,
    #[serde(default)]
    pub statuses: HashMap<String, StatusAsset>,
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
        for st in &data.statuses {
            if !self.statuses.contains_key(&st.id) {
                out.push(format!("status '{}'", st.id));
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

    /// (badge, inflicted text, colour) for a status id.
    pub fn status<'a>(&'a self, id: &'a str) -> (&'a str, &'a str, Rgb) {
        match self.statuses.get(id) {
            Some(s) => (&s.badge, &s.inflicted, s.color),
            None => (id, id, FALLBACK),
        }
    }

    pub fn move_vfx(&self, id: &str) -> &str {
        self.moves.get(id).map(|m| m.vfx.as_str()).unwrap_or("")
    }

    pub fn move_name<'a>(&'a self, id: &'a str) -> &'a str {
        self.moves.get(id).map(|m| m.name.as_str()).unwrap_or(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_wav(bytes: &[u8]) -> bool {
        bytes.len() > 44 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE"
    }

    #[test]
    fn every_referenced_sound_exists_and_is_wav() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let db = AssetDb::load(&dir).expect("assets.ron");
        let paths = db.sounds.values().flatten().chain(db.moves.values().filter_map(|m| m.sound.as_ref()));
        let mut n = 0;
        for rel in paths {
            let bytes = std::fs::read(dir.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
            assert!(is_wav(&bytes), "{rel} is not a WAV file");
            n += 1;
        }
        assert!(n > 0);
    }
}
