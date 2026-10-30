//! Generates all placeholder art in `assets/`: creature sprite sheets, arena
//! ground tiles and obstacle rocks. Run: `cargo run --release --bin gen_sprites`.
//!
//! Creatures are 3D primitive models rendered to pixel art (see `sdf.rs`):
//! 64px frames, 6 animation frames per row, and rows grouped per animation as
//! [down, up, side] facings (idle 0-2, walk 3-5, attack 6-8, hurt 9-11).
//! This must match `sheet` in assets.ron. Any sheet can be replaced by real art
//! with the same layout without touching code.

mod models;
mod sdf;

use macroquad::texture::Image;
use models::Pose;
use sdf::{Camera, Facing};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const FRAME: usize = 64;
const COLS: usize = 6;
const ANIMS: usize = 4;
const FACINGS: [Facing; 3] = [Facing::Down, Facing::Up, Facing::Side];
/// Game units per sprite pixel; must match `pixel_scale` in assets.ron.
const PIXEL_SCALE: f32 = 2.0;
const ELEVATION: f32 = 38.0;
const ANCHOR: (f32, f32) = (32.0, 50.0);

#[derive(Deserialize)]
struct CreatureEntry {
    sprite: Option<String>,
}

#[derive(Deserialize)]
struct ArenaArt {
    ground: String,
    obstacle: String,
    obstacle_radius: f32,
    snow: bool,
}

#[derive(Deserialize)]
struct Assets {
    creatures: HashMap<String, CreatureEntry>,
    arena_art: HashMap<String, ArenaArt>,
}

#[derive(Deserialize)]
struct CreatureData {
    asset: String,
    radius: f32,
}


fn root() -> PathBuf {
    let local = PathBuf::from("assets");
    if local.is_dir() {
        PathBuf::from(".")
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }
}

/// Sample a keyframe track at `u` in 0..1 (linear interpolation).
fn track(keys: &[f32], u: f32) -> f32 {
    let x = u * (keys.len() - 1) as f32;
    let i = (x as usize).min(keys.len() - 2);
    keys[i] + (keys[i + 1] - keys[i]) * (x - i as f32)
}

fn pose(anim: usize, frame: usize) -> Pose {
    use std::f32::consts::TAU;
    let ph = frame as f32 / COLS as f32;
    let w = ph * TAU;
    // Progress through one-shot animations (attack, hurt), 0..1 inclusive.
    let u = frame as f32 / (COLS - 1) as f32;
    let base = Pose {
        squash: 1.0,
        phase: ph,
        ..Default::default()
    };
    match anim {
        // idle: breathing bob, blink on the last frame
        0 => Pose {
            bob: 0.5 * w.sin(),
            squash: 1.0 + 0.02 * w.sin(),
            eyes_closed: frame == COLS - 1,
            ..base
        },
        // walk: trot cycle, two bounces per stride
        1 => Pose {
            stride: w.cos(),
            bob: w.sin().abs(),
            ..base
        },
        // attack: wind back, lunge, hold, recover
        2 => Pose {
            lean: track(&[-2.0, -1.0, 3.0, 3.5, 2.0, 1.0], u),
            squash: track(&[1.05, 1.03, 0.95, 0.94, 0.97, 0.99], u),
            eyes_closed: frame == 0,
            mouth_open: frame > 0,
            ..base
        },
        // hurt: frame 0 becomes the white flash; then recoil and settle
        _ => Pose {
            lean: track(&[0.0, -2.6, -2.2, -1.6, -1.0, -0.5], u),
            squash: track(&[1.0, 0.88, 0.91, 0.94, 0.97, 0.99], u),
            eyes_closed: true,
            mouth_open: true,
            ..base
        },
    }
}

fn blit(sheet: &mut [u8], sheet_w: usize, px: &[[u8; 4]], x0: usize, y0: usize, size: usize) {
    for y in 0..size {
        for x in 0..size {
            let i = ((y0 + y) * sheet_w + x0 + x) * 4;
            sheet[i..i + 4].copy_from_slice(&px[y * size + x]);
        }
    }
}

/// macroquad's `export_png` flips rows (it expects GL screen data), so flip first.
fn save(bytes: Vec<u8>, w: usize, h: usize, path: &Path) {
    let mut flipped = vec![0u8; bytes.len()];
    for y in 0..h {
        flipped[y * w * 4..(y + 1) * w * 4].copy_from_slice(&bytes[(h - 1 - y) * w * 4..(h - y) * w * 4]);
    }
    let img = Image {
        bytes: flipped,
        width: w as u16,
        height: h as u16,
    };
    std::fs::create_dir_all(path.parent().unwrap()).expect("create output dir");
    img.export_png(path.to_str().unwrap());
    println!("wrote {}", path.display());
}

fn creature_sheet(id: &str, radius: f32, out: &Path) {
    // Model unit scale so the body roughly matches the hurtbox at PIXEL_SCALE.
    let s = radius / PIXEL_SCALE / 8.0 * 1.2;
    let cam = Camera {
        size: FRAME,
        anchor: ANCHOR,
        elevation_deg: ELEVATION,
    };
    let w = FRAME * COLS;
    let h = FRAME * ANIMS * FACINGS.len();
    let mut sheet = vec![0u8; w * h * 4];
    for anim in 0..ANIMS {
        for (fi, &facing) in FACINGS.iter().enumerate() {
            for frame in 0..COLS {
                let model = models::build(id, s, &pose(anim, frame));
                let mut px = sdf::render(&model, &cam, facing);
                if anim == 3 && frame == 0 {
                    for p in &mut px {
                        if p[3] > 0 {
                            *p = [255, 255, 255, 255];
                        }
                    }
                }
                let row = anim * FACINGS.len() + fi;
                blit(&mut sheet, w, &px, frame * FRAME, row * FRAME, FRAME);
            }
        }
    }
    save(sheet, w, h, out);
}

fn rock_strip(radius: f32, snow: bool, out: &Path) {
    let r = radius / PIXEL_SCALE;
    let cam = Camera {
        size: FRAME,
        anchor: (32.0, 46.0),
        elevation_deg: ELEVATION,
    };
    let variants = 3;
    let w = FRAME * variants;
    let mut sheet = vec![0u8; w * FRAME * 4];
    for v in 0..variants {
        let px = sdf::render(&models::rock(r * 1.08, v as u32 + 7, snow), &cam, Facing::Side);
        blit(&mut sheet, w, &px, v * FRAME, 0, FRAME);
    }
    save(sheet, w, FRAME, out);
}

/// Seamless 64x64 ground tile with speckles, drawn on a torus so it tiles.
fn ground_tile(kind: &str, out: &Path) {
    const T: usize = 64;
    let hash = |x: i32, y: i32, k: u32| -> f32 {
        let (x, y) = (x.rem_euclid(T as i32) as u32, y.rem_euclid(T as i32) as u32);
        let mut h = x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263) ^ k.wrapping_mul(2246822519);
        h = (h ^ (h >> 13)).wrapping_mul(1274126177);
        ((h ^ (h >> 16)) % 10000) as f32 / 10000.0
    };
    let (base, alt, dark, light): ([u8; 4], [u8; 4], [u8; 4], [u8; 4]) = match kind {
        "grass" => ([112, 184, 88, 255], [100, 170, 80, 255], [70, 130, 64, 255], [150, 212, 110, 255]),
        _ => ([228, 234, 244, 255], [220, 228, 240, 255], [184, 196, 218, 255], [248, 251, 255, 255]),
    };
    let mut px = vec![base; T * T];
    for y in 0..T as i32 {
        for x in 0..T as i32 {
            // soft 2-tone patches from smoothed noise
            let mut n = 0.0;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1), (-1, 0), (0, -1)] {
                n += hash((x + dx) / 4, (y + dy) / 4, 1);
            }
            let i = y as usize * T + x as usize;
            if n / 6.0 > 0.62 {
                px[i] = alt;
            }
            let r = hash(x, y, 2);
            if r > 0.993 {
                px[i] = dark;
            } else if r > 0.98 {
                px[i] = light;
            }
        }
    }
    // little tufts: grass blades or snow drifts
    for k in 0..10 {
        let cx = (hash(k, 0, 3) * T as f32) as i32;
        let cy = (hash(k, 0, 4) * T as f32) as i32;
        let pts: &[(i32, i32, bool)] = if kind == "grass" {
            &[(0, 0, true), (0, -1, true), (2, 0, true), (2, -1, true), (2, -2, false), (-2, 0, true), (-2, -1, false), (1, 1, false), (-1, 1, false)]
        } else {
            &[(-2, 0, false), (-1, 0, true), (0, 0, true), (1, 0, true), (2, 0, false), (0, -1, true), (-1, 1, false), (0, 1, false), (1, 1, false)]
        };
        for &(dx, dy, hi) in pts {
            let (x, y) = ((cx + dx).rem_euclid(T as i32), (cy + dy).rem_euclid(T as i32));
            px[y as usize * T + x as usize] = if hi { light } else { dark };
        }
    }
    let bytes: Vec<u8> = px.into_iter().flatten().collect();
    save(bytes, T, T, out);
}

fn main() {
    let root = root();
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap_or_else(|e| panic!("{p}: {e}"));
    let assets: Assets = ron::from_str(&read("assets/assets.ron")).expect("parse assets.ron");
    let creatures: Vec<CreatureData> = ron::from_str(&read("data/creatures.ron")).expect("parse creatures.ron");

    for c in &creatures {
        let Some(Some(rel)) = assets.creatures.get(&c.asset).map(|e| e.sprite.as_ref()) else {
            continue;
        };
        creature_sheet(&c.asset, c.radius, &root.join("assets").join(rel));
    }
    let mut themes: Vec<_> = assets.arena_art.iter().collect();
    themes.sort_by_key(|(k, _)| k.as_str());
    for (name, art) in themes {
        let kind = if art.snow { "snow" } else { "grass" };
        let _ = name;
        ground_tile(kind, &root.join("assets").join(&art.ground));
        rock_strip(art.obstacle_radius, art.snow, &root.join("assets").join(&art.obstacle));
    }
}
