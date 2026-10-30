//! Sprite sheets and textures: loading, validation, facing and animation-frame
//! selection.

use crate::assets::{AssetDb, SheetLayout};
use combat::sim::Action;
use combat::Vec2;
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

/// Facing row within an animation; `Side` faces right, mirrored for left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Down,
    Up,
    Side,
}

/// Facing row and horizontal flip for an on-screen direction.
pub fn pick_dir(facing: Vec2) -> (Dir, bool) {
    // Slight bias toward side views, which read best in motion.
    if facing.x.abs() >= facing.y.abs() * 0.8 {
        (Dir::Side, facing.x < 0.0)
    } else if facing.y > 0.0 {
        (Dir::Down, false)
    } else {
        (Dir::Up, false)
    }
}

fn read_image(path: &Path) -> Result<Image, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Image::from_file_with_format(&bytes, None).map_err(|e| format!("{}: {e}", path.display()))
}

/// Decode a sheet and check it matches the layout. Works without a window.
pub fn load_sheet_image(path: &Path, layout: &SheetLayout) -> Result<Image, String> {
    let img = read_image(path)?;
    let max_row = [layout.rows.idle, layout.rows.walk, layout.rows.attack, layout.rows.hurt]
        .into_iter()
        .max()
        .unwrap_or(0);
    let need_w = layout.frame_size * layout.columns;
    let need_h = layout.frame_size * (max_row + layout.facings.max(1));
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

fn texture(img: &Image) -> Texture2D {
    let tex = Texture2D::from_image(img);
    tex.set_filter(FilterMode::Nearest);
    tex
}

pub struct ArenaTextures {
    pub ground: Texture2D,
    pub obstacles: Texture2D,
}

pub struct Sprites {
    sheets: HashMap<String, Texture2D>,
    arenas: HashMap<String, ArenaTextures>,
    pub layout: SheetLayout,
}

impl Sprites {
    /// Loads every sheet; failures become warnings and fall back to shapes.
    pub fn load(dir: &Path, assets: &AssetDb) -> (Self, Vec<String>) {
        let mut sheets = HashMap::new();
        let mut arenas = HashMap::new();
        let mut warnings = Vec::new();
        for (id, c) in &assets.creatures {
            let Some(rel) = &c.sprite else { continue };
            match load_sheet_image(&dir.join(rel), &assets.sheet) {
                Ok(img) => {
                    sheets.insert(id.clone(), texture(&img));
                }
                Err(e) => warnings.push(e),
            }
        }
        for (id, a) in &assets.arena_art {
            match (read_image(&dir.join(&a.ground)), read_image(&dir.join(&a.obstacle))) {
                (Ok(g), Ok(o)) => {
                    arenas.insert(
                        id.clone(),
                        ArenaTextures {
                            ground: texture(&g),
                            obstacles: texture(&o),
                        },
                    );
                }
                (Err(e), _) | (_, Err(e)) => warnings.push(e),
            }
        }
        (
            Self {
                sheets,
                arenas,
                layout: assets.sheet.clone(),
            },
            warnings,
        )
    }

    pub fn sheet(&self, creature_asset: &str) -> Option<&Texture2D> {
        self.sheets.get(creature_asset)
    }

    pub fn arena(&self, art: &str) -> Option<&ArenaTextures> {
        self.arenas.get(art)
    }

    pub fn source_rect(&self, anim: Anim, dir: Dir, frame: u32) -> Rect {
        let l = &self.layout;
        let base = match anim {
            Anim::Idle => l.rows.idle,
            Anim::Walk => l.rows.walk,
            Anim::Attack => l.rows.attack,
            Anim::Hurt => l.rows.hurt,
        };
        let row = if l.facings >= 3 {
            base + match dir {
                Dir::Down => 0,
                Dir::Up => 1,
                Dir::Side => 2,
            }
        } else {
            base
        };
        let s = l.frame_size as f32;
        Rect::new(frame as f32 * s, row as f32 * s, s, s)
    }

    pub fn portrait_rect(&self) -> Rect {
        let p = self.layout.portrait;
        let base = self.source_rect(Anim::Idle, Dir::Down, 0);
        Rect::new(base.x + p.x, base.y + p.y, p.w, p.h)
    }

    /// Draw one frame with its anchor at screen point `at`.
    pub fn draw_frame(&self, tex: &Texture2D, src: Rect, at: macroquad::math::Vec2, flip: bool, tint: Color) {
        let sc = self.layout.pixel_scale;
        let (ax, ay) = self.layout.anchor;
        // Mirroring flips the anchor horizontally as well.
        let ax = if flip { src.w - ax } else { ax };
        draw_texture_ex(
            tex,
            (at.x - ax * sc).round(),
            (at.y - ay * sc).round(),
            tint,
            DrawTextureParams {
                dest_size: Some(vec2(src.w * sc, src.h * sc)),
                source: Some(src),
                flip_x: flip,
                ..Default::default()
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{PixelRect, SheetRows};

    fn layout() -> SheetLayout {
        SheetLayout {
            frame_size: 64,
            columns: 4,
            rows: SheetRows { idle: 0, walk: 3, attack: 6, hurt: 9 },
            facings: 3,
            fps: 8.0,
            pixel_scale: 2.0,
            anchor: (32.0, 46.0),
            portrait: PixelRect { x: 10.0, y: 4.0, w: 44.0, h: 36.0 },
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
    fn facing_picks_row_and_mirror() {
        assert_eq!(pick_dir(Vec2::new(1.0, 0.0)), (Dir::Side, false));
        assert_eq!(pick_dir(Vec2::new(-1.0, 0.2)), (Dir::Side, true));
        assert_eq!(pick_dir(Vec2::new(0.1, 1.0)), (Dir::Down, false));
        assert_eq!(pick_dir(Vec2::new(-0.1, -1.0)), (Dir::Up, false));
    }

    #[test]
    fn shipped_sheets_match_layout() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let assets = AssetDb::load(&dir).expect("assets.ron");
        for (id, c) in &assets.creatures {
            let rel = c.sprite.as_ref().unwrap_or_else(|| panic!("{id} has no sprite"));
            load_sheet_image(&dir.join(rel), &assets.sheet).unwrap();
        }
        for a in assets.arena_art.values() {
            read_image(&dir.join(&a.ground)).unwrap();
            read_image(&dir.join(&a.obstacle)).unwrap();
        }
    }
}
