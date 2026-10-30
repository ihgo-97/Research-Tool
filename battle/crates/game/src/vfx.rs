//! Per-move visual effects, chosen by the `vfx` id in assets.ron. Purely
//! cosmetic: the gameplay shape (circle, arc) is still drawn underneath so
//! hitboxes stay readable.

use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};

fn alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

fn light(c: Color, k: f32) -> Color {
    Color::new(c.r + (1.0 - c.r) * k, c.g + (1.0 - c.g) * k, c.b + (1.0 - c.b) * k, c.a)
}

fn dark(c: Color, k: f32) -> Color {
    Color::new(c.r * k, c.g * k, c.b * k, c.a)
}

/// Cheap deterministic noise for flicker without per-effect state.
fn hash(x: f32) -> f32 {
    let s = (x * 12.9898).sin() * 43758.547;
    s - s.floor()
}

fn px(p: Vec2, size: f32, c: Color) {
    let s = size.max(1.0).round();
    draw_rectangle((p.x - s * 0.5).round(), (p.y - s * 0.5).round(), s, s, c);
}

fn star(c: Vec2, r: f32, rot: f32, col: Color) {
    for k in 0..5 {
        let a = rot + k as f32 * TAU / 5.0 - PI / 2.0;
        let tip = c + vec2(a.cos(), a.sin()) * r;
        let l = c + vec2((a - 0.6).cos(), (a - 0.6).sin()) * r * 0.42;
        let rr = c + vec2((a + 0.6).cos(), (a + 0.6).sin()) * r * 0.42;
        draw_triangle(tip, l, rr, col);
        draw_triangle(c, l, rr, col);
    }
}

/// Jagged polyline from `a` to `b` (lightning).
fn zigzag(a: Vec2, b: Vec2, segs: usize, jag: f32, seed: f32, thick: f32, col: Color) {
    let d = b - a;
    let n = vec2(-d.y, d.x).normalize_or_zero();
    let mut prev = a;
    for i in 1..=segs {
        let t = i as f32 / segs as f32;
        let off = if i == segs { 0.0 } else { (hash(seed + i as f32) - 0.5) * 2.0 * jag };
        let p = a + d * t + n * off;
        draw_line(prev.x, prev.y, p.x, p.y, thick, col);
        prev = p;
    }
}

/// Projectile at screen position `s` moving along `dir` (unit).
pub fn projectile(kind: &str, s: Vec2, dir: Vec2, r: f32, col: Color, t: f32) {
    let side = vec2(-dir.y, dir.x);
    match kind {
        "star" => {
            draw_line(s.x - dir.x * r * 3.0, s.y - dir.y * r * 3.0, s.x, s.y, r * 0.8, alpha(light(col, 0.6), 0.5));
            star(s, r * 1.9, t * 14.0, Color::new(0.35, 0.25, 0.1, 1.0));
            star(s, r * 1.5, t * 14.0, Color::new(1.0, 0.85, 0.3, 1.0));
            draw_circle(s.x, s.y, r * 0.35, Color::new(1.0, 1.0, 0.8, 1.0));
        }
        "flame" => {
            // Teardrop pointing backwards, flickering.
            let f = 1.0 + 0.25 * (t * 40.0).sin();
            let tail = s - dir * r * 3.2 * f;
            draw_triangle(s + side * r, s - side * r, tail, alpha(col, 0.85));
            draw_circle(s.x, s.y, r * 1.1, col);
            draw_triangle(s + side * r * 0.5, s - side * r * 0.5, s - dir * r * 1.8 * f, light(col, 0.6));
            draw_circle(s.x, s.y, r * 0.55, Color::new(1.0, 0.95, 0.6, 1.0));
        }
        "jet" => {
            // Long glossy streak.
            let tail = s - dir * r * 5.0;
            draw_line(tail.x, tail.y, s.x, s.y, r * 1.6, alpha(col, 0.55));
            draw_line(tail.x + dir.x * r * 2.0, tail.y + dir.y * r * 2.0, s.x, s.y, r * 1.1, col);
            draw_circle(s.x, s.y, r * 0.9, light(col, 0.3));
            draw_line(s.x - dir.x * r * 3.0 - side.x * r * 0.3, s.y - dir.y * r * 3.0 - side.y * r * 0.3, s.x - side.x * r * 0.3, s.y - side.y * r * 0.3, 2.0, light(col, 0.8));
        }
        "seed" => {
            // Spinning seed: an oval with a highlight.
            let a = t * 20.0;
            let ax = vec2(a.cos(), a.sin());
            let ay = vec2(-ax.y, ax.x);
            let p = |u: f32, v: f32| s + ax * u * r * 1.3 + ay * v * r * 0.7;
            draw_triangle(p(1.0, 0.0), p(0.0, 1.0), p(-1.0, 0.0), dark(col, 0.65));
            draw_triangle(p(1.0, 0.0), p(0.0, -1.0), p(-1.0, 0.0), col);
            px(p(0.2, -0.3), 2.0, light(col, 0.7));
        }
        "bolt" => {
            let tail = s - dir * r * 4.5;
            zigzag(tail, s, 4, r * 0.9, (t * 30.0).floor(), 3.0, alpha(col, 0.8));
            zigzag(tail, s, 4, r * 0.6, (t * 30.0).floor() + 7.0, 1.5, Color::new(1.0, 1.0, 0.9, 1.0));
            draw_circle(s.x, s.y, r * 0.9, light(col, 0.6));
        }
        "boulder" => {
            let a = t * 9.0;
            draw_poly(s.x, s.y, 6, r * 1.15, a.to_degrees(), dark(col, 0.45));
            draw_poly(s.x, s.y, 6, r, a.to_degrees(), col);
            draw_poly(s.x - r * 0.25, s.y - r * 0.25, 5, r * 0.45, a.to_degrees(), light(col, 0.35));
        }
        "gust" => {
            // Swirling crescents.
            for k in 0..3 {
                let a = t * 18.0 + k as f32 * TAU / 3.0;
                let c = s + vec2(a.cos(), a.sin()) * r * 0.5;
                draw_circle_lines(c.x, c.y, r * (0.7 + 0.15 * k as f32), 4.0, alpha(dark(col, 0.55), 0.9));
                draw_circle_lines(c.x, c.y, r * (0.7 + 0.15 * k as f32), 2.0, alpha(light(col, 0.4), 1.0));
            }
            draw_circle(s.x, s.y, r * 0.5, alpha(light(col, 0.7), 0.8));
        }
        _ => {
            draw_circle(s.x, s.y, r + 2.0, dark(col, 0.55));
            draw_circle(s.x, s.y, r, col);
            draw_circle(s.x - r * 0.25, s.y - r * 0.25, r * 0.45, Color::new(1.0, 1.0, 1.0, 0.9));
        }
    }
}

/// Melee swing. `prog` goes 0 -> 1 over the active window.
pub fn melee(kind: &str, c: Vec2, range: f32, angle: f32, half: f32, prog: f32, col: Color) {
    let fade = (1.0 - prog).clamp(0.0, 1.0);
    match kind {
        "slash" => {
            // Two crescent cuts sweeping across the arc.
            for (k, w) in [(0.0, 5.0), (0.18, 3.0)] {
                let p = (prog * 1.4 - k).clamp(0.0, 1.0);
                let a0 = angle - half;
                let a1 = a0 + 2.0 * half * p;
                let steps = 10;
                for i in 0..steps {
                    let u0 = a0 + (a1 - a0) * i as f32 / steps as f32;
                    let u1 = a0 + (a1 - a0) * (i + 1) as f32 / steps as f32;
                    let rr = range * (0.75 + 0.15 * (i as f32 / steps as f32));
                    let p0 = c + vec2(u0.cos(), u0.sin()) * rr;
                    let p1 = c + vec2(u1.cos(), u1.sin()) * rr;
                    draw_line(p0.x, p0.y, p1.x, p1.y, w + 3.0, alpha(dark(col, 0.45), fade));
                    draw_line(p0.x, p0.y, p1.x, p1.y, w, alpha(light(col, 0.7), fade));
                }
            }
        }
        "bite" => {
            // Upper and lower fangs closing on the target point.
            let tip = c + vec2(angle.cos(), angle.sin()) * range * 0.8;
            let gap = range * 0.35 * (1.0 - prog);
            let side = vec2(-angle.sin(), angle.cos());
            for sgn in [-1.0, 1.0] {
                let base = tip + side * sgn * (gap + 10.0);
                for k in -1..=1 {
                    let b = base + vec2(angle.cos(), angle.sin()) * (k as f32 * 9.0);
                    let f = vec2(angle.cos(), angle.sin());
                    draw_triangle(b - f * 6.0, b + f * 6.0, b - side * sgn * 13.0, alpha(Color::new(0.3, 0.1, 0.1, 1.0), fade));
                    draw_triangle(b - f * 4.0, b + f * 4.0, b - side * sgn * 10.0, alpha(WHITE, fade));
                }
            }
            for i in 0..6 {
                let a = angle + (hash(i as f32 + prog * 9.0) - 0.5) * half * 2.0;
                let p = c + vec2(a.cos(), a.sin()) * range * (0.5 + 0.4 * hash(i as f32 * 3.1));
                px(p, 4.0, alpha(col, fade));
            }
        }
        "whip" => {
            // A vine lashing out along a curve.
            let n = 14;
            let reach = range * (0.4 + 0.6 * (prog * 2.0).min(1.0));
            let mut prev = c;
            for i in 1..=n {
                let u = i as f32 / n as f32;
                let a = angle + (u * PI).sin() * half * 0.6 * (1.0 - prog * 2.0);
                let p = c + vec2(a.cos(), a.sin()) * reach * u;
                draw_line(prev.x, prev.y, p.x, p.y, 6.0 - 3.0 * u, alpha(dark(col, 0.7), fade));
                draw_line(prev.x, prev.y, p.x, p.y, 3.0 - 1.5 * u, alpha(light(col, 0.3), fade));
                if i % 4 == 0 {
                    px(p + vec2(-a.sin(), a.cos()) * 5.0, 4.0, alpha(light(col, 0.4), fade));
                }
                prev = p;
            }
        }
        _ => {
            // "impact": shockwave arcs pushing outward.
            for k in 0..3 {
                let rr = range * (0.4 + 0.25 * k as f32 + 0.3 * prog);
                let steps = 8;
                for i in 0..steps {
                    let u0 = angle - half * 0.8 + 1.6 * half * i as f32 / steps as f32;
                    let u1 = angle - half * 0.8 + 1.6 * half * (i + 1) as f32 / steps as f32;
                    let p0 = c + vec2(u0.cos(), u0.sin()) * rr;
                    let p1 = c + vec2(u1.cos(), u1.sin()) * rr;
                    let a = fade * (1.0 - 0.25 * k as f32);
                    draw_line(p0.x, p0.y, p1.x, p1.y, 6.0, alpha(dark(col, 0.4), a));
                    draw_line(p0.x, p0.y, p1.x, p1.y, 3.0, alpha(WHITE, a));
                }
            }
        }
    }
}

/// Telegraph decoration for a pending burst; `prog` 0 -> 1 until it lands.
pub fn burst_telegraph(kind: &str, c: Vec2, r: f32, prog: f32, col: Color, t: f32) {
    match kind {
        "lightning" => {
            // Target reticle closing in, storm cloud gathering above.
            let rr = r * (1.4 - 0.4 * prog);
            for k in 0..4 {
                let a = t * 2.0 + k as f32 * PI / 2.0;
                let p = c + vec2(a.cos(), a.sin()) * rr;
                let q = c + vec2(a.cos(), a.sin()) * (rr - 14.0);
                draw_line(p.x, p.y, q.x, q.y, 4.0, alpha(col, 0.9));
            }
            let cloud = c - vec2(0.0, 90.0 + 30.0 * (1.0 - prog));
            for k in 0..5 {
                let off = vec2((k as f32 - 2.0) * 14.0, (hash(k as f32) - 0.5) * 8.0);
                draw_circle(cloud.x + off.x, cloud.y + off.y, 14.0, Color::new(0.35, 0.35, 0.45, 0.35 + 0.45 * prog));
            }
        }
        "splash" => {
            // Ripples converging on the centre.
            for k in 0..3 {
                let u = (1.0 - ((prog * 1.5 + k as f32 / 3.0) % 1.0)) * r;
                draw_circle_lines(c.x, c.y, u, 2.0, alpha(light(col, 0.5), 0.7));
            }
        }
        "quake" => {
            // Ground shakes: debris jitter inside the area.
            for i in 0..10 {
                let a = hash(i as f32) * TAU;
                let d = hash(i as f32 * 1.7) * r;
                let j = (t * 50.0 + i as f32).sin() * 2.0 * prog;
                px(c + vec2(a.cos(), a.sin()) * d + vec2(j, 0.0), 4.0, alpha(dark(col, 0.6), 0.8));
            }
        }
        _ => {}
    }
}

/// Active burst; `prog` 0 -> 1 over the active window.
pub fn burst_active(kind: &str, c: Vec2, r: f32, prog: f32, col: Color, t: f32) {
    let fade = (1.0 - prog).clamp(0.0, 1.0);
    match kind {
        "flame_ring" => {
            let n = 14;
            for i in 0..n {
                let a = i as f32 * TAU / n as f32 + t;
                let base = c + vec2(a.cos(), a.sin()) * r * (0.6 + 0.4 * prog);
                let h = 26.0 * (0.6 + 0.4 * hash(i as f32 + (t * 20.0).floor()));
                draw_triangle(base - vec2(7.0, 0.0), base + vec2(7.0, 0.0), base - vec2(0.0, h), alpha(col, fade));
                draw_triangle(base - vec2(3.5, 0.0), base + vec2(3.5, 0.0), base - vec2(0.0, h * 0.55), alpha(Color::new(1.0, 0.9, 0.4, 1.0), fade));
            }
        }
        "splash" => {
            draw_circle_lines(c.x, c.y, r * (0.7 + 0.3 * prog), 6.0, alpha(light(col, 0.5), fade));
            for i in 0..16 {
                let a = i as f32 * TAU / 16.0;
                let p = c + vec2(a.cos(), a.sin()) * r * (0.5 + 0.6 * prog) - vec2(0.0, 30.0 * (prog * PI).sin() * hash(i as f32));
                px(p, 5.0, alpha(light(col, 0.6), fade));
            }
        }
        "spores" => {
            for i in 0..12 {
                let a = i as f32 * TAU / 12.0 + hash(i as f32);
                let d = r * (0.3 + 0.6 * hash(i as f32 * 2.3)) * (0.6 + 0.4 * prog);
                let p = c + vec2(a.cos(), a.sin()) * d - vec2(0.0, 12.0 * prog);
                draw_circle(p.x, p.y, 12.0 + 6.0 * hash(i as f32 * 5.0), alpha(light(col, 0.55), 0.55 * fade));
            }
        }
        "lightning" => {
            // Bolt from the sky plus a white flash on the ground.
            let top = c - vec2(0.0, 260.0);
            zigzag(top, c, 7, 18.0, (t * 25.0).floor(), 9.0, alpha(col, fade));
            zigzag(top, c, 7, 18.0, (t * 25.0).floor(), 3.0, alpha(WHITE, fade));
            draw_circle(c.x, c.y, r * 0.5 * (1.0 + prog), alpha(WHITE, 0.6 * fade));
        }
        "quake" => {
            // Cracks radiating from the centre plus dust.
            for i in 0..8 {
                let a = i as f32 * TAU / 8.0 + hash(i as f32) * 0.5;
                let len = r * (0.6 + 0.4 * hash(i as f32 * 3.3)) * (0.5 + 0.5 * prog.min(1.0) * 2.0).min(1.0);
                zigzag(c, c + vec2(a.cos(), a.sin()) * len, 4, 6.0, i as f32 * 4.0, 4.0, alpha(dark(col, 0.4), fade));
            }
            for i in 0..14 {
                let a = hash(i as f32 * 1.3) * TAU;
                let p = c + vec2(a.cos(), a.sin()) * r * hash(i as f32 * 7.7) - vec2(0.0, 20.0 * prog);
                draw_circle(p.x, p.y, 8.0, alpha(light(col, 0.4), 0.5 * fade));
            }
        }
        _ => {}
    }
}
