//! Creature sprite sheets: loading, validation and animation-frame selection.

use crate::assets::{AssetDb, SheetLayout};
use combat::sim::Action;
use macroquad::prelude::*;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anim {
    Idle,
    Walk,
    Attack,
    Hurt,
}

/// Decode a sheet and check it matches the layout. Works without a window.
pub fn load_sheet_image(path: &Path, layout: &SheetLayout) -> Result<Image, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let img = Image::from_file_with_format(&bytes, None).map_err(|e| format!("{}: {e}", path.display()))?;
    let max_row = [layout.rows.idle, layout.rows.walk, layout.rows.attack, layout.rows.hurt]
        .into_iter()
        .max()
        .unwrap_or(0);
    let need_w = layout.frame_size * layout.columns;
    let need_h = layout.frame_size * (max_row + 1);
    if (img.width as u32) < need_w || (img.height as u32) < need_h {
        return Err(format!(
            "{}: {}x{} is smaller than the {}x{} the sheet layout needs",
            path.display(),
            img.width,
            img.height,
            need_w,
            need_h
        ));
    }
    Ok(img)
}

/// Which animation and frame to show. Pure, so it can be unit tested.
pub fn pick_frame(action: &Action, speed: f32, flash: bool, time: f32, layout: &SheetLayout) -> (Anim, u32) {
    let cols = layout.columns.max(1);
    let cycle = |rate: f32| ((time * layout.fps * rate) as u32) % cols;
    if flash {
        return (Anim::Hurt, 0); // frame 0 of the hurt row is the white flash
    }
    match *action {
        Action::Fainted => (Anim::Hurt, cols - 1),
        Action::Hitstun { .. } => (Anim::Hurt, 1 + cycle(1.0) % (cols - 1).max(1)),
        Action::Windup { .. } => (Anim::Attack, 0),
        Action::Recovery { .. } => (Anim::Attack, (1 + cycle(1.5) % (cols - 1).max(1)).min(cols - 1)),
        Action::Dash { .. } => (Anim::Walk, cycle(2.0)),
        Action::Idle if speed > 25.0 => (Anim::Walk, cycle(1.0)),
        Action::Idle => (Anim::Idle, cycle(0.6)),
    }
}

pub struct Sprites {
    sheets: HashMap<String, Texture2D>,
    pub layout: SheetLayout,
}

impl Sprites {
    /// Loads every creature sheet; failures become warnings and fall back to shapes.
    pub fn load(dir: &Path, assets: &AssetDb) -> (Self, Vec<String>) {
        let mut sheets = HashMap::new();
        let mut warnings = Vec::new();
        for (id, c) in &assets.creatures {
            let Some(rel) = &c.sprite else { continue };
            match load_sheet_image(&dir.join(rel), &assets.sheet) {
                Ok(img) => {
                    let tex = Texture2D::from_image(&img);
                    tex.set_filter(FilterMode::Nearest);
                    sheets.insert(id.clone(), tex);
                }
                Err(e) => warnings.push(e),
            }
        }
        (
            Self {
                sheets,
                layout: assets.sheet.clone(),
            },
            warnings,
        )
    }

    pub fn sheet(&self, creature_asset: &str) -> Option<&Texture2D> {
        self.sheets.get(creature_asset)
    }

    pub fn source_rect(&self, anim: Anim, frame: u32) -> Rect {
        let l = &self.layout;
        let row = match anim {
            Anim::Idle => l.rows.idle,
            Anim::Walk => l.rows.walk,
            Anim::Attack => l.rows.attack,
            Anim::Hurt => l.rows.hurt,
        };
        let s = l.frame_size as f32;
        Rect::new(frame as f32 * s, row as f32 * s, s, s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::SheetRows;
    use combat::Vec2;

    fn layout() -> SheetLayout {
        SheetLayout {
            frame_size: 48,
            columns: 4,
            rows: SheetRows { idle: 0, walk: 1, attack: 2, hurt: 3 },
            fps: 8.0,
            body_fraction: 0.56,
        }
    }

    #[test]
    fn animation_follows_fighter_state() {
        let l = layout();
        assert_eq!(pick_frame(&Action::Idle, 0.0, false, 0.0, &l).0, Anim::Idle);
        assert_eq!(pick_frame(&Action::Idle, 200.0, false, 0.0, &l).0, Anim::Walk);
        let w = Action::Windup { slot: 0, t: 0.1, aim: Vec2::ZERO };
        assert_eq!(pick_frame(&w, 0.0, false, 0.0, &l), (Anim::Attack, 0));
        assert_eq!(pick_frame(&Action::Hitstun { t: 0.2 }, 0.0, false, 0.0, &l).0, Anim::Hurt);
        assert_eq!(pick_frame(&Action::Fainted, 0.0, false, 0.0, &l), (Anim::Hurt, 3));
        // The hit flash overrides everything and uses the white frame.
        assert_eq!(pick_frame(&w, 0.0, true, 0.0, &l), (Anim::Hurt, 0));
    }

    #[test]
    fn frames_stay_in_range_over_time() {
        let l = layout();
        let actions = [
            Action::Idle,
            Action::Recovery { t: 0.1 },
            Action::Hitstun { t: 0.1 },
            Action::Dash { t: 0.1, dir: Vec2::X },
        ];
        for a in actions {
            for k in 0..200 {
                let (anim, f) = pick_frame(&a, 100.0, false, k as f32 * 0.037, &l);
                assert!(f < l.columns, "{a:?} frame {f}");
                if anim == Anim::Hurt {
                    assert!(f >= 1, "hurt animation must not reuse the flash frame");
                }
            }
        }
    }

    #[test]
    fn shipped_sheets_match_layout() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let assets = AssetDb::load(&dir).expect("assets.ron");
        for (id, c) in &assets.creatures {
            let rel = c.sprite.as_ref().unwrap_or_else(|| panic!("{id} has no sprite"));
            load_sheet_image(&dir.join(rel), &assets.sheet).unwrap();
        }
    }
}
