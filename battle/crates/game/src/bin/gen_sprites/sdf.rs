//! Tiny signed-distance-field renderer that turns 3D primitive models into
//! GBA/DS-style pixel art: quantised hue-shifted shading ramps, a light
//! direction from the top-left, coloured outlines and occlusion lines.

use std::ops::{Add, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub const fn v3(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}

impl V3 {
    pub fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn norm(self) -> V3 {
        let l = self.len();
        if l > 1e-6 {
            self * (1.0 / l)
        } else {
            self
        }
    }
    fn mulv(self, o: V3) -> V3 {
        v3(self.x * o.x, self.y * o.y, self.z * o.z)
    }
    fn divv(self, o: V3) -> V3 {
        v3(self.x / o.x, self.y / o.y, self.z / o.z)
    }
}

impl Add for V3 {
    type Output = V3;
    fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for V3 {
    type Output = V3;
    fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f32> for V3 {
    type Output = V3;
    fn mul(self, s: f32) -> V3 {
        v3(self.x * s, self.y * s, self.z * s)
    }
}
impl Neg for V3 {
    type Output = V3;
    fn neg(self) -> V3 {
        v3(-self.x, -self.y, -self.z)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Shape {
    Ellipsoid { c: V3, r: V3 },
    /// Capsule whose radius tapers from `ra` at `a` to `rb` at `b`.
    RoundCone { a: V3, b: V3, ra: f32, rb: f32 },
}

impl Shape {
    fn sdf(&self, p: V3) -> f32 {
        match *self {
            Shape::Ellipsoid { c, r } => {
                let q = p - c;
                let k0 = q.divv(r).len();
                let k1 = q.divv(r.mulv(r)).len();
                if k1 < 1e-6 {
                    -r.x.min(r.y).min(r.z)
                } else {
                    k0 * (k0 - 1.0) / k1
                }
            }
            Shape::RoundCone { a, b, ra, rb } => {
                // Inigo Quilez's round cone, arbitrary orientation.
                let ba = b - a;
                let l2 = ba.dot(ba);
                let rr = ra - rb;
                let a2 = l2 - rr * rr;
                let il2 = 1.0 / l2;
                let pa = p - a;
                let y = pa.dot(ba);
                let z = y - l2;
                let xv = pa * l2 - ba * y;
                let x2 = xv.dot(xv);
                let y2 = y * y * l2;
                let z2 = z * z * l2;
                let k = rr.signum() * rr * rr * x2;
                if z.signum() * a2 * z2 > k {
                    return (x2 + z2).sqrt() * il2 - rb;
                }
                if y.signum() * a2 * y2 < k {
                    return (x2 + y2).sqrt() * il2 - ra;
                }
                ((x2 * a2 * il2).sqrt() + y * rr) * il2 - ra
            }
        }
    }
}

pub fn ell(c: V3, r: V3) -> Shape {
    Shape::Ellipsoid { c, r }
}

pub fn sphere(c: V3, r: f32) -> Shape {
    Shape::Ellipsoid { c, r: v3(r, r, r) }
}

pub fn cone(a: V3, b: V3, ra: f32, rb: f32) -> Shape {
    Shape::RoundCone { a, b, ra, rb }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Finish {
    /// Normal 4-tone shading.
    Matte,
    /// Adds a bright specular highlight tone.
    Shiny,
    /// Self-lit (flames, sparks): only the two brightest tones.
    Glow,
    /// Eye: solid near-black plus a white glint.
    Eye,
}

#[derive(Debug, Clone, Copy)]
pub struct Material {
    pub color: (u8, u8, u8),
    pub finish: Finish,
}

#[derive(Debug, Clone, Copy)]
pub struct Prim {
    pub shape: Shape,
    pub mat: usize,
    /// Smooth-union radius with everything before it (0 = hard union).
    pub blend: f32,
}

#[derive(Default)]
pub struct Model {
    pub mats: Vec<Material>,
    pub prims: Vec<Prim>,
}

impl Model {
    pub fn mat(&mut self, color: (u8, u8, u8), finish: Finish) -> usize {
        self.mats.push(Material { color, finish });
        self.mats.len() - 1
    }

    pub fn add(&mut self, shape: Shape, mat: usize, blend: f32) {
        self.prims.push(Prim { shape, mat, blend });
    }

    /// Mirror-pair helper: adds the shape and its copy with z negated.
    pub fn add_pair(&mut self, shape: Shape, mat: usize, blend: f32) {
        self.add(shape, mat, blend);
        let m = match shape {
            Shape::Ellipsoid { c, r } => Shape::Ellipsoid { c: v3(c.x, c.y, -c.z), r },
            Shape::RoundCone { a, b, ra, rb } => Shape::RoundCone {
                a: v3(a.x, a.y, -a.z),
                b: v3(b.x, b.y, -b.z),
                ra,
                rb,
            },
        };
        self.add(m, mat, blend);
    }

    /// Distance and material at a model-space point.
    fn eval(&self, p: V3) -> (f32, usize) {
        let mut d = f32::INFINITY;
        let mut m = 0;
        for prim in &self.prims {
            let di = prim.shape.sdf(p);
            if prim.blend > 0.0 && d.is_finite() {
                let k = prim.blend;
                let h = (0.5 + 0.5 * (di - d) / k).clamp(0.0, 1.0);
                let nd = di + (d - di) * h - k * h * (1.0 - h);
                if di < d {
                    m = prim.mat;
                }
                d = nd;
            } else if di < d {
                d = di;
                m = prim.mat;
            }
        }
        (d, m)
    }
}

/// Which way the creature faces on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    /// Toward the viewer (screen down).
    Down,
    Up,
    /// Screen right; left is drawn by mirroring.
    Side,
}

/// World point -> model space (model faces +x).
fn to_model(p: V3, f: Facing) -> V3 {
    match f {
        Facing::Side => p,
        Facing::Down => v3(p.z, p.y, -p.x),
        Facing::Up => v3(-p.z, p.y, p.x),
    }
}

pub struct Camera {
    pub size: usize,
    /// Frame pixel where the model origin (ground under the creature) lands.
    pub anchor: (f32, f32),
    /// Downward viewing angle in degrees (0 = side view, 90 = straight down).
    pub elevation_deg: f32,
}

#[derive(Clone, Copy)]
struct Hit {
    depth: f32,
    mat: usize,
    tone: usize,
}

/// 5-step ramp: outline, shadow, mid, light, highlight.
pub fn ramp(c: (u8, u8, u8)) -> [[u8; 4]; 5] {
    let (h, s, v) = rgb_to_hsv(c);
    let shift = |h: f32, toward: f32, amt: f32| {
        let mut d = toward - h;
        if d > 180.0 {
            d -= 360.0;
        }
        if d < -180.0 {
            d += 360.0;
        }
        (h + d * amt).rem_euclid(360.0)
    };
    let mk = |h: f32, s: f32, v: f32| {
        let (r, g, b) = hsv_to_rgb(h, s.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
        [r, g, b, 255]
    };
    [
        mk(shift(h, 250.0, 0.35), (s + 0.25).min(1.0), v * 0.28),
        mk(shift(h, 250.0, 0.18), (s + 0.12).min(1.0), v * 0.68),
        mk(h, s, v),
        mk(shift(h, 55.0, 0.12), s * 0.82, (v * 1.14).min(1.0)),
        mk(shift(h, 55.0, 0.2), s * 0.35, 1.0),
    ]
}

fn rgb_to_hsv(c: (u8, u8, u8)) -> (f32, f32, f32) {
    let (r, g, b) = (c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max == 0.0 { 0.0 } else { d / max };
    (h, s, max)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let f = |u: f32| ((u + m) * 255.0).round() as u8;
    (f(r), f(g), f(b))
}

/// Render one frame as RGBA pixels (size*size).
pub fn render(model: &Model, cam: &Camera, facing: Facing) -> Vec<[u8; 4]> {
    let n = cam.size;
    let e = cam.elevation_deg.to_radians();
    let view = v3(0.0, -e.sin(), -e.cos());
    let up = v3(0.0, e.cos(), -e.sin());
    let right = v3(1.0, 0.0, 0.0);
    let light = v3(-0.55, 0.9, 0.6).norm();
    let ramps: Vec<_> = model.mats.iter().map(|m| ramp(m.color)).collect();

    let mut hits: Vec<Option<Hit>> = vec![None; n * n];
    for j in 0..n {
        for i in 0..n {
            let sx = i as f32 + 0.5 - cam.anchor.0;
            let sy = cam.anchor.1 - (j as f32 + 0.5);
            let origin = right * sx + up * sy - view * 120.0;
            let mut t = 0.0;
            for _ in 0..160 {
                let p = origin + view * t;
                let (d, mat) = model.eval(to_model(p, facing));
                if d < 0.02 {
                    let pm = to_model(p, facing);
                    let eps = 0.05;
                    let g = |dx: V3| model.eval(pm + dx).0 - model.eval(pm - dx).0;
                    let nm = v3(g(v3(eps, 0.0, 0.0)), g(v3(0.0, eps, 0.0)), g(v3(0.0, 0.0, eps))).norm();
                    // Normal back to world space (inverse of to_model).
                    let nw = match facing {
                        Facing::Side => nm,
                        Facing::Down => v3(-nm.z, nm.y, nm.x),
                        Facing::Up => v3(nm.z, nm.y, -nm.x),
                    };
                    let tone = shade(nw, light, view, model.mats[mat].finish);
                    hits[j * n + i] = Some(Hit { depth: t, mat, tone });
                    break;
                }
                t += d.max(0.01);
                if t > 260.0 {
                    break;
                }
            }
        }
    }

    let mut px = vec![[0u8; 4]; n * n];
    for k in 0..n * n {
        if let Some(h) = hits[k] {
            px[k] = match model.mats[h.mat].finish {
                Finish::Eye if h.tone == 4 => [255, 255, 255, 255],
                Finish::Eye => [24, 20, 30, 255],
                _ => ramps[h.mat][h.tone],
            };
        }
    }
    // Occlusion lines: the farther surface along a depth jump gets an outline tone.
    for j in 0..n {
        for i in 0..n {
            let Some(h) = hits[j * n + i] else { continue };
            for (di, dj) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let (ni, nj) = (i as i32 + di, j as i32 + dj);
                if ni < 0 || nj < 0 || ni >= n as i32 || nj >= n as i32 {
                    continue;
                }
                if let Some(o) = hits[nj as usize * n + ni as usize] {
                    let jump = h.depth - o.depth;
                    let differs = o.mat != h.mat || jump > 4.0;
                    if jump > 2.5 && differs && model.mats[h.mat].finish != Finish::Eye {
                        px[j * n + i] = ramps[h.mat][0];
                    }
                }
            }
        }
    }
    // Silhouette outline drawn just outside the shape, coloured per material.
    let src = px.clone();
    for j in 0..n {
        for i in 0..n {
            if hits[j * n + i].is_some() {
                continue;
            }
            let mut best: Option<(usize, f32)> = None;
            for (di, dj) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let (ni, nj) = (i as i32 + di, j as i32 + dj);
                if ni < 0 || nj < 0 || ni >= n as i32 || nj >= n as i32 {
                    continue;
                }
                if let Some(o) = hits[nj as usize * n + ni as usize] {
                    if best.is_none_or(|(_, d)| o.depth < d) {
                        best = Some((o.mat, o.depth));
                    }
                }
            }
            if let Some((m, _)) = best {
                px[j * n + i] = match model.mats[m].finish {
                    Finish::Eye => [24, 20, 30, 255],
                    _ => ramps[m][0],
                };
            }
        }
    }
    let _ = src;
    px
}

fn shade(n: V3, light: V3, view: V3, finish: Finish) -> usize {
    let lam = n.dot(light);
    let half = 0.5 * lam + 0.5;
    let spec = {
        let h = (light - view).norm();
        n.dot(h).max(0.0).powf(24.0)
    };
    match finish {
        Finish::Glow => {
            if half > 0.8 {
                3
            } else {
                2
            }
        }
        Finish::Eye => {
            if spec > 0.55 {
                4
            } else {
                1
            }
        }
        _ => {
            if finish == Finish::Shiny && spec > 0.5 {
                4
            } else if half < 0.32 {
                1
            } else if half < 0.66 {
                2
            } else {
                3
            }
        }
    }
}
