//! Hitbox geometry tests.

use crate::math::{angle_diff, Vec2};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    Circle { radius: f32 },
    /// Circular sector of `range` radius, centred on `dir`, spanning `half_arc` each side.
    Arc { range: f32, dir: Vec2, half_arc: f32 },
}

pub fn circles_overlap(a: Vec2, ra: f32, b: Vec2, rb: f32) -> bool {
    a.distance(b) <= ra + rb
}

/// Does a sector at `origin` touch a circle at `c` with radius `r`?
pub fn arc_hits_circle(origin: Vec2, range: f32, dir: Vec2, half_arc: f32, c: Vec2, r: f32) -> bool {
    let to = c - origin;
    let dist = to.length();
    if dist > range + r {
        return false;
    }
    if dist <= r {
        return true; // target overlaps the origin
    }
    // Widen the arc by the angle the target's radius subtends.
    let slack = (r / dist).clamp(-1.0, 1.0).asin();
    angle_diff(to.angle(), dir.angle()).abs() <= half_arc + slack
}

pub fn shape_hits_circle(shape: &Shape, origin: Vec2, c: Vec2, r: f32) -> bool {
    match *shape {
        Shape::Circle { radius } => circles_overlap(origin, radius, c, r),
        Shape::Arc { range, dir, half_arc } => arc_hits_circle(origin, range, dir, half_arc, c, r),
    }
}

/// Does the segment a→b pass within `r` of `c`?
pub fn segment_hits_circle(a: Vec2, b: Vec2, c: Vec2, r: f32) -> bool {
    let ab = b - a;
    let len_sq = ab.length_sq();
    let t = if len_sq > 0.0 {
        ((c - a).dot(ab) / len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * t).distance(c) <= r
}
