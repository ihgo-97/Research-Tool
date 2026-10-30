//! Minimal 2D vector so the core stays independent of any rendering crate.

use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };
    pub const X: Vec2 = Vec2 { x: 1.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn from_angle(rad: f32) -> Self {
        Self::new(rad.cos(), rad.sin())
    }

    pub fn length(self) -> f32 {
        self.length_sq().sqrt()
    }

    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn dot(self, o: Vec2) -> f32 {
        self.x * o.x + self.y * o.y
    }

    /// 2D cross product (z component).
    pub fn cross(self, o: Vec2) -> f32 {
        self.x * o.y - self.y * o.x
    }

    pub fn distance(self, o: Vec2) -> f32 {
        (self - o).length()
    }

    /// Unit vector, or zero if the input is (near) zero.
    pub fn normalize_or_zero(self) -> Self {
        let l = self.length();
        if l > 1e-6 {
            self * (1.0 / l)
        } else {
            Vec2::ZERO
        }
    }

    pub fn perp(self) -> Self {
        Self::new(-self.y, self.x)
    }

    pub fn angle(self) -> f32 {
        self.y.atan2(self.x)
    }

    pub fn rotate(self, rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self::new(self.x * c - self.y * s, self.x * s + self.y * c)
    }

    pub fn clamp_length(self, max: f32) -> Self {
        let l = self.length();
        if l > max && l > 0.0 {
            self * (max / l)
        } else {
            self
        }
    }

    pub fn lerp(self, o: Vec2, t: f32) -> Self {
        self + (o - self) * t
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
}
impl AddAssign for Vec2 {
    fn add_assign(&mut self, o: Vec2) {
        self.x += o.x;
        self.y += o.y;
    }
}
impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }
}
impl SubAssign for Vec2 {
    fn sub_assign(&mut self, o: Vec2) {
        self.x -= o.x;
        self.y -= o.y;
    }
}
impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, s: f32) -> Vec2 {
        Vec2::new(self.x * s, self.y * s)
    }
}
impl Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}

/// Smallest signed difference between two angles, in (-PI, PI].
pub fn angle_diff(a: f32, b: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let mut d = (a - b) % TAU;
    if d > PI {
        d -= TAU;
    } else if d <= -PI {
        d += TAU;
    }
    d
}
