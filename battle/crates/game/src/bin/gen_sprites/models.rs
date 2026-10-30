//! Original creature models built from SDF primitives. Model space: +x is
//! forward, +y up, z sideways; the origin is the ground under the creature.
//! `s` scales everything so the body matches the creature's hurtbox size.

// Coordinates are written as `factor * s` throughout for readability.
#![allow(clippy::neg_multiply, clippy::too_many_arguments)]

use crate::sdf::{cone, ell, sphere, v3, Finish, Model, V3};

/// Animation pose for one frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct Pose {
    /// Vertical body offset.
    pub bob: f32,
    /// Leg cycle, -1..1 (front legs forward when positive).
    pub stride: f32,
    /// Forward/back shift of the upper body (attack lunge, hurt recoil).
    pub lean: f32,
    /// Vertical squash (1 = none).
    pub squash: f32,
    /// Generic 0..1 cycle for flicker/flap/sway.
    pub phase: f32,
    pub eyes_closed: bool,
    pub mouth_open: bool,
}

/// A pair of eyes sitting on the surface of a sphere (`center`, `radius`),
/// in direction `dir` (z mirrored for the second eye).
fn eyes(m: &mut Model, center: V3, radius: f32, dir: V3, r: f32, p: &Pose) {
    let e = m.mat((20, 20, 28), Finish::Eye);
    let at = center + dir.norm() * (radius - r * 0.25);
    let ry = if p.eyes_closed { r * 0.3 } else { r * 1.2 };
    m.add_pair(ell(at, v3(r * 0.9, ry, r * 0.9)), e, 0.0);
}

fn mouth(m: &mut Model, at: V3, w: f32, p: &Pose) {
    if p.mouth_open {
        let c = m.mat((120, 30, 40), Finish::Matte);
        m.add(ell(at, v3(w * 0.5, w * 0.45, w)), c, 0.0);
    }
}

/// Four stubby legs; returns nothing, adds prims.
fn legs4(m: &mut Model, mat: usize, s: f32, x_front: f32, x_back: f32, half_w: f32, len: f32, r: f32, p: &Pose) {
    let st = p.stride * 2.2 * s;
    for (x, sign) in [(x_front, 1.0), (x_back, -1.0)] {
        for z in [half_w, -half_w] {
            // Diagonal pairs move together (trot).
            let d = if (z > 0.0) == (sign > 0.0) { st } else { -st };
            let lift = if d > 0.1 { 0.8 * s } else { 0.0 };
            m.add(cone(v3(x, len + p.bob * 0.5, z), v3(x + d, r + lift, z * 1.05), r * 1.1, r), mat, 0.0);
        }
    }
}

pub fn build(id: &str, s: f32, p: &Pose) -> Model {
    let mut m = Model::default();
    let sq = if p.squash > 0.0 { p.squash } else { 1.0 };
    let w = p.phase * std::f32::consts::TAU;
    let b = p.bob;
    let l = p.lean;
    match id {
        "emberkit" => {
            let fur = m.mat((236, 112, 48), Finish::Matte);
            let cream = m.mat((250, 226, 180), Finish::Matte);
            let sock = m.mat((150, 60, 40), Finish::Matte);
            let flame = m.mat((255, 120, 30), Finish::Glow);
            let core = m.mat((255, 232, 120), Finish::Glow);
            let nose = m.mat((40, 26, 26), Finish::Shiny);
            legs4(&mut m, sock, s, 4.0 * s, -4.5 * s, 3.6 * s, 6.0 * s, 1.7 * s, p);
            // body
            m.add(ell(v3(0.0, 8.0 * s * sq + b, 0.0), v3(8.5 * s, 6.2 * s * sq, 5.8 * s)), fur, 0.0);
            m.add(ell(v3(4.0 * s + l * 0.5, 7.0 * s + b, 0.0), v3(4.0 * s, 4.6 * s, 3.8 * s)), cream, 2.0 * s);
            // head
            let hc = v3(6.5 * s + l, 15.0 * s * sq + b, 0.0);
            m.add(sphere(hc, 5.6 * s), fur, 2.5 * s);
            m.add(ell(hc + v3(4.4 * s, -1.6 * s, 0.0), v3(2.8 * s, 2.2 * s, 2.6 * s)), cream, 1.5 * s);
            m.add(sphere(hc + v3(7.0 * s, -1.0 * s, 0.0), 1.1 * s), nose, 0.0);
            // ears
            m.add_pair(cone(hc + v3(-1.0 * s, 3.6 * s, 2.8 * s), hc + v3(-1.6 * s, 9.5 * s, 4.0 * s), 2.3 * s, 0.4 * s), fur, 0.0);
            eyes(&mut m, hc, 5.6 * s, v3(0.75, 0.2, 0.55), 1.62 * s, p);
            mouth(&mut m, hc + v3(5.6 * s, -3.2 * s, 0.0), 1.6 * s, p);
            // flame tail
            let f = 1.0 + 0.2 * w.sin();
            let sway = 1.2 * s * (w * 2.0).sin();
            let tail = v3(-7.5 * s, 9.0 * s + b, 0.0);
            m.add(cone(tail, tail + v3(-4.0 * s, 5.0 * s, 0.0), 2.4 * s, 1.8 * s), fur, 1.0 * s);
            let fb = tail + v3(-4.6 * s, 6.5 * s, 0.0);
            m.add(cone(fb, fb + v3(-1.0 * s + sway, 9.0 * s * f, 0.0), 3.6 * s, 0.5 * s), flame, 0.0);
            m.add_pair(cone(fb + v3(0.0, 0.5 * s, 1.6 * s), fb + v3(sway * 0.6, 5.5 * s * f, 2.2 * s), 1.8 * s, 0.4 * s), core, 0.0);
        }
        "tidepup" => {
            let skin = m.mat((70, 136, 226), Finish::Shiny);
            let belly = m.mat((222, 238, 250), Finish::Matte);
            let fin = m.mat((44, 92, 176), Finish::Matte);
            let crest = m.mat((140, 210, 255), Finish::Shiny);
            let nose = m.mat((30, 30, 50), Finish::Shiny);
            // body: long seal shape lying low
            m.add(ell(v3(-1.0 * s, 6.5 * s * sq + b, 0.0), v3(10.0 * s, 6.0 * s * sq, 6.4 * s)), skin, 0.0);
            m.add(ell(v3(2.0 * s, 4.8 * s + b, 0.0), v3(6.5 * s, 3.8 * s, 4.8 * s)), belly, 2.0 * s);
            let hc = v3(7.0 * s + l, 12.5 * s * sq + b, 0.0);
            m.add(sphere(hc, 5.4 * s), skin, 3.0 * s);
            m.add(ell(hc + v3(4.2 * s, -1.8 * s, 0.0), v3(2.6 * s, 2.0 * s, 3.0 * s)), belly, 1.5 * s);
            m.add(sphere(hc + v3(6.6 * s, -1.0 * s, 0.0), 1.0 * s), nose, 0.0);
            eyes(&mut m, hc, 5.4 * s, v3(0.7, 0.25, 0.6), 1.69 * s, p);
            mouth(&mut m, hc + v3(5.4 * s, -3.4 * s, 0.0), 1.6 * s, p);
            // flippers (paddle with stride)
            let fl = 0.25 + 0.2 * p.stride;
            m.add_pair(ell(v3(3.0 * s + fl * s, 2.0 * s, 6.0 * s), v3(3.2 * s, 1.0 * s, 2.2 * s)), fin, 0.0);
            m.add(ell(v3(-11.0 * s, 3.0 * s + b, 0.0), v3(2.4 * s, 1.0 * s, 4.6 * s)), fin, 0.0);
            // droplet crest
            m.add(cone(hc + v3(-1.0 * s, 4.4 * s, 0.0), hc + v3(-3.4 * s, 10.0 * s, 0.0), 2.4 * s, 0.4 * s), crest, 0.0);
        }
        "sproutle" => {
            let skin = m.mat((96, 186, 84), Finish::Matte);
            let spot = m.mat((54, 120, 60), Finish::Matte);
            let leaf = m.mat((120, 214, 90), Finish::Shiny);
            let stem = m.mat((90, 120, 50), Finish::Matte);
            let foot = m.mat((70, 140, 70), Finish::Matte);
            legs4(&mut m, foot, s, 4.0 * s, -4.0 * s, 4.6 * s, 4.0 * s, 2.0 * s, p);
            let bc = v3(l * 0.6, 9.0 * s * sq + b, 0.0);
            m.add(ell(bc, v3(8.4 * s, 7.4 * s * sq, 8.0 * s)), skin, 0.0);
            // spots on the back
            for (x, y, z) in [(-3.0, 4.5, 3.5), (-5.0, 2.0, -3.0), (0.0, 6.0, -1.5), (-6.0, 0.0, 4.5)] {
                m.add(sphere(bc + v3(x * s, y * s, z * s), 1.5 * s), spot, 0.0);
            }
            // bulging eyes on top-front
            let ec = bc + v3(5.2 * s, 4.6 * s, 3.2 * s);
            m.add_pair(sphere(ec, 2.4 * s), skin, 1.0 * s);
            eyes(&mut m, ec, 2.4 * s, v3(0.85, 0.35, 0.3), 1.62 * s, p);
            mouth(&mut m, bc + v3(7.6 * s, 0.5 * s, 0.0), 2.0 * s, p);
            // sprout
            let sway = 1.2 * s * w.sin();
            let top = bc + v3(-1.0 * s, 7.0 * s, 0.0);
            m.add(cone(top, top + v3(sway * 0.5, 4.0 * s, 0.0), 0.9 * s, 0.7 * s), stem, 0.0);
            let lt = top + v3(sway, 4.5 * s, 0.0);
            m.add_pair(cone(lt, lt + v3(1.0 * s, 1.6 * s, 6.0 * s), 0.6 * s, 2.0 * s), leaf, 0.0);
        }
        "voltfin" => {
            let skin = m.mat((246, 206, 44), Finish::Shiny);
            let belly = m.mat((252, 240, 190), Finish::Matte);
            let dark = m.mat((120, 84, 40), Finish::Matte);
            let fin = m.mat((70, 64, 96), Finish::Matte);
            let spark = m.mat((140, 230, 255), Finish::Glow);
            let hover = 3.0 * s + 1.2 * s * w.sin();
            let bc = v3(l * 0.5, 9.0 * s * sq + b + hover, 0.0);
            m.add(ell(bc, v3(9.0 * s, 6.2 * s * sq, 6.0 * s)), skin, 0.0);
            m.add(ell(bc + v3(1.5 * s, -2.4 * s, 0.0), v3(6.0 * s, 2.6 * s, 3.6 * s)), belly, 1.5 * s);
            let hc = bc + v3(6.0 * s + l * 0.5, 1.4 * s, 0.0);
            m.add(sphere(hc, 4.8 * s), skin, 2.5 * s);
            eyes(&mut m, hc, 4.8 * s, v3(0.7, 0.3, 0.6), 1.62 * s, p);
            mouth(&mut m, hc + v3(4.2 * s, -1.8 * s, 0.0), 1.4 * s, p);
            // stripes
            for x in [-4.5, -1.5] {
                m.add(ell(bc + v3(x * s, 2.6 * s, 0.0), v3(0.7 * s, 4.0 * s, 5.9 * s)), dark, 0.0);
            }
            // zigzag dorsal fin
            for (x0, h) in [(-5.0, 5.5), (-1.5, 6.5), (2.0, 5.0)] {
                let base = bc + v3(x0 * s, 5.2 * s, 0.0);
                m.add(cone(base, base + v3(-2.0 * s, h * s * 0.8, 0.0), 1.6 * s, 0.3 * s), fin, 0.0);
            }
            // tail and fin
            let tw = 1.5 * s * w.cos();
            let tb = bc + v3(-8.0 * s, 0.0, 0.0);
            m.add(cone(tb, tb + v3(-5.0 * s, 1.0 * s, tw), 2.6 * s, 1.2 * s), skin, 1.0 * s);
            m.add(ell(tb + v3(-7.0 * s, 1.0 * s, tw), v3(1.6 * s, 4.6 * s, 1.0 * s)), fin, 0.0);
            // pectoral fins
            m.add_pair(ell(bc + v3(2.0 * s, -3.0 * s, 5.6 * s), v3(2.6 * s, 0.8 * s, 1.8 * s)), fin, 0.0);
            // floating spark
            let sp = bc + v3(-2.0 * s, 8.5 * s + 1.5 * s * (w * 2.0).sin(), 0.0);
            m.add(sphere(sp, 1.1 * s), spark, 0.0);
        }
        "pebblor" => {
            let rock = m.mat((150, 132, 112), Finish::Matte);
            let dark = m.mat((104, 92, 84), Finish::Matte);
            let moss = m.mat((110, 168, 80), Finish::Matte);
            let brow = m.mat((124, 110, 96), Finish::Matte);
            let bc = v3(l * 0.5, 9.5 * s * sq + b, 0.0);
            // stubby legs
            for z in [3.6, -3.6] {
                let d = if z > 0.0 { p.stride } else { -p.stride } * 1.8 * s;
                m.add(ell(v3(d, 2.4 * s, z * s), v3(3.0 * s, 2.6 * s, 2.6 * s)), dark, 0.0);
            }
            m.add(sphere(bc, 9.2 * s), rock, 0.0);
            for (x, y, z, r) in [(-4.0, 4.0, 4.0, 4.0), (3.0, 6.0, -3.0, 3.6), (-5.0, -2.0, -5.0, 3.6), (1.0, -4.0, 6.0, 3.2), (-6.0, 5.0, -2.0, 3.0)] {
                m.add(sphere(bc + v3(x * s, y * s, z * s), r * s), rock, 1.2 * s);
            }
            for (x, y, z) in [(-3.0, 1.0, 7.0), (-6.0, -1.0, -6.0), (2.0, -5.0, -6.5)] {
                m.add(sphere(bc + v3(x * s, y * s, z * s), 1.8 * s), dark, 0.0);
            }
            m.add(ell(bc + v3(-2.0 * s, 8.5 * s, 0.0), v3(4.4 * s, 1.6 * s, 4.0 * s)), moss, 0.0);
            // arms
            let arm = 1.5 * s * p.stride;
            m.add_pair(sphere(bc + v3(2.0 * s + arm, -2.0 * s, 9.0 * s), 3.2 * s), dark, 0.0);
            // face: brow ridge + eyes
            m.add(ell(bc + v3(7.6 * s, 3.4 * s, 0.0), v3(2.0 * s, 1.3 * s, 5.4 * s)), brow, 0.6 * s);
            eyes(&mut m, bc, 10.4 * s, v3(0.92, 0.12, 0.33), 1.62 * s, p);
            mouth(&mut m, bc + v3(8.6 * s, -3.0 * s, 0.0), 2.0 * s, p);
        }
        "galeon" => {
            let feather = m.mat((136, 196, 214), Finish::Matte);
            let chest = m.mat((236, 246, 250), Finish::Matte);
            let wing = m.mat((70, 120, 170), Finish::Matte);
            let beak = m.mat((248, 176, 60), Finish::Shiny);
            let legc = m.mat((230, 150, 50), Finish::Matte);
            // legs
            for z in [2.4, -2.4] {
                let d = if z > 0.0 { p.stride } else { -p.stride } * 1.5 * s;
                m.add(cone(v3(0.0, 5.0 * s + b, z * s), v3(d + 0.6 * s, 0.6 * s, z * s), 0.8 * s, 0.7 * s), legc, 0.0);
            }
            let bc = v3(l * 0.5, 10.5 * s * sq + b, 0.0);
            m.add(ell(bc, v3(6.6 * s, 6.4 * s * sq, 5.8 * s)), feather, 0.0);
            m.add(ell(bc + v3(3.4 * s, -0.5 * s, 0.0), v3(3.6 * s, 4.6 * s, 3.8 * s)), chest, 2.0 * s);
            let hc = bc + v3(3.6 * s + l * 0.5, 6.4 * s, 0.0);
            m.add(sphere(hc, 4.4 * s), feather, 2.5 * s);
            eyes(&mut m, hc, 4.4 * s, v3(0.7, 0.25, 0.62), 1.56 * s, p);
            let open = if p.mouth_open { 0.8 * s } else { 0.0 };
            m.add(cone(hc + v3(3.6 * s, -0.6 * s + open * 0.5, 0.0), hc + v3(8.0 * s, -1.4 * s, 0.0), 1.6 * s, 0.4 * s), beak, 0.0);
            if p.mouth_open {
                m.add(cone(hc + v3(3.6 * s, -1.6 * s, 0.0), hc + v3(6.6 * s, -2.6 * s, 0.0), 1.0 * s, 0.3 * s), beak, 0.0);
            }
            // crest
            m.add(cone(hc + v3(-1.0 * s, 3.4 * s, 0.0), hc + v3(-5.0 * s, 8.0 * s, 0.0), 1.4 * s, 0.3 * s), wing, 0.0);
            m.add(cone(hc + v3(-0.5 * s, 3.8 * s, 0.0), hc + v3(-2.0 * s, 9.0 * s, 0.0), 1.2 * s, 0.3 * s), wing, 0.0);
            // wings: flap with phase
            let flap = 2.4 * s * w.sin();
            m.add_pair(ell(bc + v3(-1.5 * s, 1.5 * s + flap, 6.0 * s), v3(5.6 * s, 2.0 * s, 1.6 * s + flap.abs() * 0.3)), wing, 0.0);
            // tail feathers
            m.add(cone(bc + v3(-5.0 * s, -1.0 * s, 0.0), bc + v3(-10.0 * s, 1.5 * s, 1.5 * s), 1.8 * s, 0.6 * s), wing, 0.0);
            m.add(cone(bc + v3(-5.0 * s, -1.0 * s, 0.0), bc + v3(-10.0 * s, 1.5 * s, -1.5 * s), 1.8 * s, 0.6 * s), wing, 0.0);
        }
        _ => {
            let c = m.mat((180, 180, 180), Finish::Matte);
            m.add(sphere(v3(0.0, 8.0 * s, 0.0), 8.0 * s), c, 0.0);
            eyes(&mut m, v3(0.0, 8.0 * s, 0.0), 8.0 * s, v3(0.8, 0.2, 0.5), 1.62 * s, p);
        }
    }
    m
}

/// Boulder used for arena obstacles; `seed` varies the lumps.
pub fn rock(r: f32, seed: u32, snow: bool) -> Model {
    let mut m = Model::default();
    let stone = m.mat((128, 124, 128), Finish::Matte);
    let dark = m.mat((96, 92, 100), Finish::Matte);
    let cap = m.mat((236, 242, 250), Finish::Matte);
    let mut h = seed.wrapping_mul(2654435761);
    let mut rnd = || {
        h ^= h << 13;
        h ^= h >> 17;
        h ^= h << 5;
        (h % 1000) as f32 / 1000.0
    };
    m.add(ell(v3(0.0, r * 0.55, 0.0), v3(r, r * 0.75, r)), stone, 0.0);
    for _ in 0..5 {
        let a = rnd() * std::f32::consts::TAU;
        let d = r * (0.4 + 0.3 * rnd());
        let c = v3(a.cos() * d, r * (0.4 + 0.5 * rnd()), a.sin() * d);
        m.add(sphere(c, r * (0.35 + 0.2 * rnd())), stone, r * 0.15);
    }
    for _ in 0..3 {
        let a = rnd() * std::f32::consts::TAU;
        let c = v3(a.cos() * r * 0.85, r * 0.3, a.sin() * r * 0.85);
        m.add(sphere(c, r * 0.22), dark, 0.0);
    }
    if snow {
        m.add(ell(v3(-r * 0.15, r * 1.12, 0.0), v3(r * 0.7, r * 0.22, r * 0.6)), cap, r * 0.1);
    }
    m
}
