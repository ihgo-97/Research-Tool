//! Sound playback behind one small interface. Without the `audio` cargo
//! feature this is a no-op, so the default build needs no audio libraries.

use crate::assets::AssetDb;
use std::path::Path;

#[cfg(feature = "audio")]
use std::collections::HashMap;

pub struct Sfx {
    #[cfg(feature = "audio")]
    sounds: HashMap<String, macroquad::audio::Sound>,
}

impl Sfx {
    /// Loads every sound referenced by `assets.sounds`; returns load warnings.
    #[cfg(feature = "audio")]
    pub async fn load(dir: &Path, assets: &AssetDb) -> (Self, Vec<String>) {
        let mut sounds = HashMap::new();
        let mut warnings = Vec::new();
        for (key, path) in &assets.sounds {
            let Some(rel) = path else { continue };
            let full = dir.join(rel);
            match macroquad::audio::load_sound(&full.to_string_lossy()).await {
                Ok(s) => {
                    sounds.insert(key.clone(), s);
                }
                Err(e) => warnings.push(format!("{}: {e}", full.display())),
            }
        }
        (Self { sounds }, warnings)
    }

    #[cfg(not(feature = "audio"))]
    pub async fn load(_dir: &Path, _assets: &AssetDb) -> (Self, Vec<String>) {
        (Self {}, Vec::new())
    }

    /// Play each queued cue once (duplicates within a frame are merged).
    pub fn play_cues(&self, cues: &mut Vec<&'static str>) {
        cues.sort_unstable();
        cues.dedup();
        #[cfg(feature = "audio")]
        for c in cues.iter() {
            if let Some(s) = self.sounds.get(*c) {
                macroquad::audio::play_sound_once(s);
            }
        }
        cues.clear();
    }
}
