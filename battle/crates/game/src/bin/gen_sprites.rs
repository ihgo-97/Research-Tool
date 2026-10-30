//! Generates the original placeholder creature sprite sheets in
//! `assets/sprites/`. Run: `cargo run --release --bin gen_sprites`.
//!
//! Sheet layout (must match `sheet` in assets.ron): 48x48 frames, 4 columns
//! (animation frames) x 4 rows (idle, walk, attack, hurt). Creatures face right.
//! Real art can replace any sheet with no code change as long as it keeps the layout.

use macroquad::texture::Image;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const F: usize = 48;
const COLS: usize = 4;
const ROWS: usize = 4;

type Px = [u8; 4];
const CLEAR: Px = [0, 0, 0, 0];
const INK: Px = [28, 24, 34, 255];
const WHITE: Px = [255, 255, 255, 255];

fn rgb(c: (u8, u8, u8)) -> Px {
    [c.0, c.1, c.2, 255]
}

fn tint(p: Px, k: f32) -> Px {
    let f = |v: u8| ((v as f32 * k).clamp(0.0, 255.0)) as u8;
    [f(p[0]), f(p[1]), f(p[2]), p[3]]
}

fn mix(a: Px, b: Px, t: f32) -> Px {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2]), 255]
}

struct Canvas {
    px: Vec<Px>,
}

impl Canvas {
    fn new() -> Self {
        Self { px: vec![CLEAR; F * F] }
    }

    fn set(&mut self, x: i32, y: i32, c: Px) {
        if x >= 0 && y >= 0 && (x as usize) < F && (y as usize) < F {
            self.px[y as usize * F + x as usize] = c;
        }
    }

    fn get(&self, x: i32, y: i32) -> Px {
        if x >= 0 && y >= 0 && (x as usize) < F && (y as usize) < F {
            self.px[y as usize * F + x as usize]
        } else {
            CLEAR
        }
    }

    /// Filled ellipse, optionally rotated by `rot` radians.
    fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, c: Px) {
        let (s, co) = rot.sin_cos();
        let r = rx.max(ry).ceil() as i32 + 1;
        for y in (cy as i32 - r)..=(cy as i32 + r) {
            for x in (cx as i32 - r)..=(cx as i32 + r) {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let u = dx * co + dy * s;
                let v = -dx * s + dy * co;
                if (u / rx).powi(2) + (v / ry).powi(2) <= 1.0 {
                    self.set(x, y, c);
                }
            }
        }
    }

    fn tri(&mut self, a: (f32, f32), b: (f32, f32), c: (f32, f32), col: Px) {
        let minx = a.0.min(b.0).min(c.0).floor() as i32;
        let maxx = a.0.max(b.0).max(c.0).ceil() as i32;
        let miny = a.1.min(b.1).min(c.1).floor() as i32;
        let maxy = a.1.max(b.1).max(c.1).ceil() as i32;
        let edge = |p: (f32, f32), q: (f32, f32), x: f32, y: f32| (q.0 - p.0) * (y - p.1) - (q.1 - p.1) * (x - p.0);
        for y in miny..=maxy {
            for x in minx..=maxx {
                let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                let w0 = edge(b, c, fx, fy);
                let w1 = edge(c, a, fx, fy);
                let w2 = edge(a, b, fx, fy);
                if (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0) || (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0) {
                    self.set(x, y, col);
                }
            }
        }
    }

    fn line(&mut self, a: (f32, f32), b: (f32, f32), w: f32, c: Px) {
        let steps = ((b.0 - a.0).abs().max((b.1 - a.1).abs()) * 2.0).max(1.0) as i32;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            self.ellipse(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, w * 0.5, w * 0.5, 0.0, c);
        }
    }

    /// Recolour pixels equal to `from` inside an ellipse (shading clipped to a shape).
    fn shade(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, from: Px, to: Px) {
        let r = rx.max(ry).ceil() as i32 + 1;
        for y in (cy as i32 - r)..=(cy as i32 + r) {
            for x in (cx as i32 - r)..=(cx as i32 + r) {
                let dx = (x as f32 + 0.5 - cx) / rx;
                let dy = (y as f32 + 0.5 - cy) / ry;
                if dx * dx + dy * dy <= 1.0 && self.get(x, y) == from {
                    self.set(x, y, to);
                }
            }
        }
    }

    /// Darken the lower-right rim of an ellipse: pixels equal to `from` inside
    /// the ellipse but outside a copy shifted up-left.
    fn rim(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, from: Px, to: Px) {
        let r = rx.max(ry).ceil() as i32 + 1;
        for y in (cy as i32 - r)..=(cy as i32 + r) {
            for x in (cx as i32 - r)..=(cx as i32 + r) {
                let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                let outer = ((fx - cx) / rx).powi(2) + ((fy - cy) / ry).powi(2) <= 1.0;
                let inner = ((fx - cx + 2.0) / rx).powi(2) + ((fy - cy + 3.0) / ry).powi(2) <= 1.0;
                if outer && !inner && self.get(x, y) == from {
                    self.set(x, y, to);
                }
            }
        }
    }

    /// 1px dark outline around every opaque shape.
    fn outline(&mut self) {
        let src = self.px.clone();
        let opaque = |x: i32, y: i32| {
            x >= 0 && y >= 0 && (x as usize) < F && (y as usize) < F && src[y as usize * F + x as usize][3] > 0
        };
        for y in 0..F as i32 {
            for x in 0..F as i32 {
                if !opaque(x, y) && (opaque(x - 1, y) || opaque(x + 1, y) || opaque(x, y - 1) || opaque(x, y + 1)) {
                    self.set(x, y, INK);
                }
            }
        }
    }

    fn silhouette(&mut self, c: Px) {
        for p in &mut self.px {
            if p[3] > 0 {
                *p = c;
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Eyes {
    Open,
    Squint,
    Cross,
}

/// Per-frame pose shared by every creature.
#[derive(Clone, Copy)]
struct Pose {
    dx: f32,
    dy: f32,
    sx: f32,
    sy: f32,
    eyes: Eyes,
    mouth: bool,
    /// Foot offset for walk cycles (-1..1).
    step: f32,
    /// Animation phase 0..1 for flicker/flap.
    phase: f32,
}

fn pose(row: usize, frame: usize) -> Pose {
    let ph = frame as f32 / COLS as f32;
    let base = Pose {
        dx: 0.0,
        dy: 0.0,
        sx: 1.0,
        sy: 1.0,
        eyes: Eyes::Open,
        mouth: false,
        step: 0.0,
        phase: ph,
    };
    match row {
        // idle: gentle breathing
        0 => Pose {
            dy: [0.0, -1.0, 0.0, 1.0][frame],
            sy: [1.0, 1.03, 1.0, 0.97][frame],
            eyes: if frame == 3 { Eyes::Squint } else { Eyes::Open },
            ..base
        },
        // walk: bouncy with alternating feet
        1 => Pose {
            dy: [0.0, -2.0, 0.0, -2.0][frame],
            step: [1.0, 0.0, -1.0, 0.0][frame],
            ..base
        },
        // attack: wind back, lunge, hold, recover
        2 => Pose {
            dx: [-2.0, 3.0, 4.0, 1.0][frame],
            sx: [0.92, 1.12, 1.15, 1.03][frame],
            sy: [1.06, 0.92, 0.9, 0.98][frame],
            eyes: if frame == 0 { Eyes::Squint } else { Eyes::Open },
            mouth: frame > 0,
            ..base
        },
        // hurt: frame 0 is a white flash silhouette, then squash
        _ => Pose {
            dx: [0.0, -3.0, -2.0, -1.0][frame],
            sx: [1.0, 1.1, 1.06, 1.02][frame],
            sy: [1.0, 0.86, 0.92, 0.97][frame],
            eyes: Eyes::Cross,
            mouth: true,
            ..base
        },
    }
}

struct Palette {
    body: Px,
    light: Px,
    dark: Px,
}

fn palette(c: (u8, u8, u8)) -> Palette {
    let body = rgb(c);
    Palette {
        body,
        light: mix(body, WHITE, 0.35),
        dark: tint(body, 0.62),
    }
}

/// Body, shading, feet, eyes and mouth. Returns body centre.
fn body(cv: &mut Canvas, pal: &Palette, p: &Pose, rx: f32, ry: f32) -> (f32, f32) {
    let cx = 24.0 + p.dx;
    let cy = 27.0 + p.dy;
    // feet
    let fy = cy + ry * p.sy - 2.0;
    cv.ellipse(cx - 6.0 + p.step * 2.0, fy, 4.0, 3.0, 0.0, pal.dark);
    cv.ellipse(cx + 6.0 - p.step * 2.0, fy, 4.0, 3.0, 0.0, pal.dark);
    cv.ellipse(cx, cy, rx * p.sx, ry * p.sy, 0.0, pal.body);
    cv.rim(cx, cy, rx * p.sx, ry * p.sy, pal.body, pal.dark);
    cv.shade(cx - rx * 0.4, cy - ry * 0.45, rx * 0.32, ry * 0.22, pal.body, pal.light);
    (cx, cy)
}

fn face(cv: &mut Canvas, p: &Pose, cx: f32, cy: f32, rx: f32) {
    let ex = cx + rx * p.sx * 0.45;
    let ey = cy - 3.0;
    for (ox, big) in [(0.0, true), (-6.0, false)] {
        let x = ex + ox;
        match p.eyes {
            Eyes::Open => {
                let r = if big { 2.6 } else { 2.2 };
                cv.ellipse(x, ey, r, r + 0.6, 0.0, WHITE);
                cv.ellipse(x + 0.8, ey + 0.3, r * 0.55, r * 0.7, 0.0, INK);
            }
            Eyes::Squint => cv.line((x - 2.0, ey), (x + 2.0, ey), 1.4, INK),
            Eyes::Cross => {
                cv.line((x - 2.0, ey - 2.0), (x + 2.0, ey + 2.0), 1.2, INK);
                cv.line((x - 2.0, ey + 2.0), (x + 2.0, ey - 2.0), 1.2, INK);
            }
        }
    }
    let mx = ex - 2.0;
    let my = cy + 3.0;
    if p.mouth {
        cv.ellipse(mx, my, 2.5, 2.0, 0.0, INK);
        cv.ellipse(mx, my + 0.8, 1.5, 0.9, 0.0, [220, 80, 90, 255]);
    } else {
        cv.line((mx - 2.0, my), (mx + 1.5, my + 0.6), 1.0, INK);
    }
}

fn draw_creature(id: &str, col: (u8, u8, u8), p: &Pose) -> Canvas {
    let mut cv = Canvas::new();
    let pal = palette(col);
    let w = p.phase * std::f32::consts::TAU;
    match id {
        "emberkit" => {
            // flame tail behind the body
            let flick = 1.0 + 0.15 * w.sin();
            cv.ellipse(12.0 + p.dx, 22.0 + p.dy, 6.0 * flick, 9.0 * flick, -0.5, [230, 60, 30, 255]);
            cv.ellipse(12.5 + p.dx, 23.0 + p.dy, 4.0, 6.0 * flick, -0.5, [250, 150, 40, 255]);
            cv.ellipse(13.0 + p.dx, 25.0 + p.dy, 2.0, 3.0, -0.5, [255, 230, 120, 255]);
            let (cx, cy) = body(&mut cv, &pal, p, 12.0, 11.0);
            // ears
            cv.tri((cx - 1.0, cy - 8.0), (cx + 5.0, cy - 9.0), (cx + 3.0, cy - 17.0), pal.body);
            cv.tri((cx + 5.0, cy - 8.0), (cx + 11.0, cy - 7.0), (cx + 10.0, cy - 16.0), pal.body);
            cv.tri((cx + 1.0, cy - 9.0), (cx + 4.0, cy - 9.0), (cx + 3.0, cy - 14.0), [255, 200, 150, 255]);
            face(&mut cv, p, cx, cy, 12.0);
        }
        "tidepup" => {
            let (cx, cy) = body(&mut cv, &pal, p, 14.0, 11.0);
            cv.ellipse(cx + 2.0, cy + 4.0, 8.0 * p.sx, 5.0 * p.sy, 0.0, [215, 235, 250, 255]);
            // flippers
            let flap = 0.3 * w.sin();
            cv.ellipse(cx - 10.0, cy + 4.0, 5.0, 2.5, 0.6 + flap, pal.dark);
            cv.ellipse(cx + 11.0, cy + 6.0, 4.0, 2.0, -0.4 - flap, pal.dark);
            // droplet crest
            cv.ellipse(cx - 1.0, cy - 12.0, 3.5, 3.5, 0.0, [150, 210, 255, 255]);
            cv.tri((cx - 4.0, cy - 12.0), (cx + 2.0, cy - 12.0), (cx - 3.0, cy - 19.0), [150, 210, 255, 255]);
            face(&mut cv, p, cx, cy, 14.0);
        }
        "sproutle" => {
            let (cx, cy) = body(&mut cv, &pal, p, 13.0, 12.0);
            // spots
            cv.ellipse(cx - 6.0, cy + 1.0, 2.2, 1.8, 0.0, pal.dark);
            cv.ellipse(cx - 2.0, cy + 6.0, 1.8, 1.5, 0.0, pal.dark);
            // leaf sprout, swaying
            let sway = 0.25 * w.sin();
            let stem = (cx - 1.0, cy - 11.0);
            cv.line(stem, (stem.0, stem.1 - 5.0), 1.6, [70, 120, 50, 255]);
            cv.ellipse(stem.0 - 5.0, stem.1 - 7.0, 6.0, 2.8, 0.5 + sway, [60, 150, 60, 255]);
            cv.ellipse(stem.0 + 5.0, stem.1 - 7.0, 6.0, 2.8, -0.5 + sway, [90, 190, 80, 255]);
            face(&mut cv, p, cx, cy, 13.0);
        }
        "voltfin" => {
            // tail fin
            let wag = 2.0 * w.sin();
            cv.tri((10.0 + p.dx, 27.0 + p.dy), (3.0 + p.dx, 19.0 + p.dy + wag), (3.0 + p.dx, 35.0 + p.dy + wag), pal.dark);
            let (cx, cy) = body(&mut cv, &pal, p, 15.0, 10.0);
            // zigzag dorsal fin
            cv.tri((cx - 8.0, cy - 8.0), (cx - 3.0, cy - 17.0), (cx - 1.0, cy - 8.0), [60, 50, 30, 255]);
            cv.tri((cx - 2.0, cy - 8.0), (cx + 3.0, cy - 15.0), (cx + 5.0, cy - 8.0), [60, 50, 30, 255]);
            // stripes
            for sx in [-7.0, -2.0] {
                cv.line((cx + sx, cy - 7.0), (cx + sx - 2.0, cy + 7.0), 1.6, [120, 90, 20, 255]);
            }
            face(&mut cv, p, cx, cy, 15.0);
        }
        "pebblor" => {
            // orbiting pebbles
            for k in 0..3 {
                let a = w + k as f32 * 2.1;
                cv.ellipse(24.0 + 18.0 * a.cos(), 30.0 + 7.0 * a.sin() + p.dy, 2.5, 2.0, 0.0, [120, 110, 100, 255]);
            }
            let (cx, cy) = body(&mut cv, &pal, p, 15.0, 13.0);
            // angular top
            cv.tri((cx - 12.0, cy - 6.0), (cx - 4.0, cy - 16.0), (cx + 4.0, cy - 9.0), pal.body);
            cv.tri((cx + 2.0, cy - 10.0), (cx + 9.0, cy - 15.0), (cx + 13.0, cy - 5.0), pal.light);
            // cracks
            cv.line((cx - 9.0, cy - 2.0), (cx - 5.0, cy + 2.0), 1.2, pal.dark);
            cv.line((cx - 5.0, cy + 2.0), (cx - 7.0, cy + 7.0), 1.2, pal.dark);
            cv.line((cx + 2.0, cy - 9.0), (cx + 0.0, cy - 5.0), 1.2, pal.dark);
            face(&mut cv, p, cx, cy, 15.0);
        }
        "galeon" => {
            // wings flap
            let flap = 3.0 * w.sin();
            cv.ellipse(14.0 + p.dx, 22.0 + p.dy - flap, 9.0, 4.0, -0.7, pal.dark);
            cv.ellipse(30.0 + p.dx, 21.0 + p.dy - flap, 8.0, 3.5, 0.6, pal.light);
            let (cx, cy) = body(&mut cv, &pal, p, 12.0, 11.0);
            // crest tuft
            cv.tri((cx - 3.0, cy - 10.0), (cx + 1.0, cy - 10.0), (cx - 4.0, cy - 17.0), pal.dark);
            cv.tri((cx, cy - 10.0), (cx + 4.0, cy - 10.0), (cx + 2.0, cy - 16.0), pal.dark);
            // beak
            cv.tri((cx + 10.0, cy - 1.0), (cx + 10.0, cy + 3.0), (cx + 17.0, cy + 1.0), [245, 170, 60, 255]);
            face(&mut cv, p, cx, cy, 12.0);
        }
        _ => {
            let (cx, cy) = body(&mut cv, &pal, p, 13.0, 12.0);
            face(&mut cv, p, cx, cy, 13.0);
        }
    }
    cv.outline();
    cv
}

#[derive(Deserialize)]
struct CreatureEntry {
    color: (u8, u8, u8),
    sprite: Option<String>,
}

#[derive(Deserialize)]
struct Assets {
    creatures: HashMap<String, CreatureEntry>,
}

fn assets_dir() -> PathBuf {
    let local = PathBuf::from("assets");
    if local.is_dir() {
        local
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
    }
}

fn main() {
    let dir = assets_dir();
    let text = std::fs::read_to_string(dir.join("assets.ron")).expect("read assets.ron");
    let assets: Assets = ron::from_str(&text).expect("parse assets.ron");
    let mut ids: Vec<_> = assets.creatures.keys().cloned().collect();
    ids.sort();
    for id in ids {
        let entry = &assets.creatures[&id];
        let Some(rel) = &entry.sprite else { continue };
        let mut sheet = vec![0u8; F * COLS * F * ROWS * 4];
        let stride = F * COLS;
        for row in 0..ROWS {
            for frame in 0..COLS {
                let mut cv = draw_creature(&id, entry.color, &pose(row, frame));
                if row == 3 && frame == 0 {
                    cv.silhouette(WHITE);
                }
                for y in 0..F {
                    for x in 0..F {
                        let p = cv.px[y * F + x];
                        // macroquad's export_png flips rows (it expects GL screen
                        // data), so write the sheet bottom-up.
                        let sy = F * ROWS - 1 - (row * F + y);
                        let i = (sy * stride + frame * F + x) * 4;
                        sheet[i..i + 4].copy_from_slice(&p);
                    }
                }
            }
        }
        let img = Image {
            bytes: sheet,
            width: (F * COLS) as u16,
            height: (F * ROWS) as u16,
        };
        let out = dir.join(rel);
        std::fs::create_dir_all(out.parent().unwrap()).expect("create sprite dir");
        img.export_png(out.to_str().unwrap());
        println!("wrote {}", out.display());
    }
}
