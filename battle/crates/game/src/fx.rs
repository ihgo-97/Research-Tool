//! Presentation effects driven by simulation events: floating numbers,
//! particles, hit flash, screen shake, hit-stop and sound cues.

use crate::assets::AssetDb;
use crate::feel::Feel;
use crate::render::rgb;
use combat::rng::Rng;
use combat::{Battle, Event, Vec2};
use macroquad::color::Color;

pub struct Floater {
    pub pos: Vec2,
    pub text: String,
    pub color: Color,
    pub size: f32,
    pub t: f32,
}

pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub max_life: f32,
    pub color: Color,
    pub size: f32,
}

pub struct Fx {
    pub floaters: Vec<Floater>,
    pub particles: Vec<Particle>,
    pub flash: Vec<f32>,
    pub shake: f32,
    /// Remaining freeze time; the battle does not advance while > 0.
    pub hitstop: f32,
    /// Sound cue keys (see `sounds` in assets.ron) raised since last drained.
    pub cues: Vec<&'static str>,
    rng: Rng,
}

const FLOATER_LIFE: f32 = 0.9;

impl Fx {
    pub fn new(fighters: usize, seed: u64) -> Self {
        Self {
            floaters: Vec::new(),
            particles: Vec::new(),
            flash: vec![0.0; fighters],
            shake: 0.0,
            hitstop: 0.0,
            cues: Vec::new(),
            rng: Rng::new(seed),
        }
    }

    pub fn floater_alpha(f: &Floater) -> f32 {
        (1.0 - f.t / FLOATER_LIFE).clamp(0.0, 1.0)
    }

    fn burst(&mut self, pos: Vec2, n: u32, speed: (f32, f32), life: f32, size: f32, color: Color) {
        for _ in 0..n {
            let a = self.rng.range(0.0, std::f32::consts::TAU);
            let s = self.rng.range(speed.0, speed.1);
            let l = life * self.rng.range(0.6, 1.0);
            self.particles.push(Particle {
                pos,
                vel: Vec2::from_angle(a) * s,
                life: l,
                max_life: l,
                color,
                size: size * self.rng.range(0.6, 1.2),
            });
        }
    }

    /// React to the events of the battle's latest tick.
    pub fn on_events(&mut self, battle: &Battle, assets: &AssetDb, feel: &Feel, player: usize) {
        let data = &*battle.data;
        for ev in battle.events() {
            match *ev {
                Event::MoveLaunched { .. } => self.cues.push("launch"),
                Event::Hit {
                    target,
                    mv,
                    damage,
                    type_mult,
                    pos,
                    ..
                } => {
                    let strong = type_mult > 1.01;
                    let (color, suffix) = if strong {
                        (Color::new(1.0, 0.6, 0.2, 1.0), "!")
                    } else if type_mult < 0.99 {
                        (Color::new(0.65, 0.68, 0.72, 1.0), "")
                    } else {
                        (Color::new(1.0, 1.0, 1.0, 1.0), "")
                    };
                    self.floaters.push(Floater {
                        pos,
                        text: format!("{damage:.0}{suffix}"),
                        color,
                        size: if strong { 30.0 } else { 24.0 },
                        t: 0.0,
                    });
                    if let Some(f) = self.flash.get_mut(target) {
                        *f = feel.flash_seconds;
                    }
                    if target == player {
                        self.shake = self.shake.max(feel.shake.player_hit);
                    }
                    self.hitstop = self.hitstop.max(feel.hitstop_for(damage));
                    let tc = rgb(assets.type_color(&data.types.names[data.moves[mv].element]), 1.0);
                    self.burst(pos, feel.particles.hit, (80.0, 260.0), 0.35, 4.0, tc);
                    self.cues.push(if strong { "hit_strong" } else { "hit" });
                }
                Event::Dodged { pos, .. } => {
                    self.floaters.push(Floater {
                        pos,
                        text: "dodge".into(),
                        color: Color::new(0.6, 0.9, 1.0, 1.0),
                        size: 20.0,
                        t: 0.0,
                    });
                    self.burst(pos, feel.particles.dodge, (120.0, 140.0), 0.3, 3.0, Color::new(0.6, 0.9, 1.0, 1.0));
                    self.cues.push("dodge");
                }
                Event::Dash { fighter } => {
                    let v = battle.fighter(fighter);
                    let feet = v.body.pos + Vec2::new(0.0, v.body.radius * 0.6);
                    self.burst(feet, feel.particles.dash, (20.0, 70.0), 0.4, 5.0, Color::new(0.75, 0.75, 0.7, 0.6));
                    self.cues.push("dash");
                }
                Event::Fainted { fighter } => {
                    let v = battle.fighter(fighter);
                    let c = rgb(assets.creature_color(&data.creatures[v.fighter.creature].asset), 1.0);
                    self.floaters.push(Floater {
                        pos: v.body.pos,
                        text: "KO".into(),
                        color: Color::new(1.0, 0.3, 0.3, 1.0),
                        size: 40.0,
                        t: 0.0,
                    });
                    self.burst(v.body.pos, feel.particles.ko, (60.0, 320.0), 0.8, 6.0, c);
                    self.shake = feel.shake.ko;
                    self.cues.push("ko");
                }
                Event::MoveStarted { .. } => {}
            }
        }
    }

    pub fn update(&mut self, dt: f32, feel: &Feel) {
        for f in &mut self.floaters {
            f.t += dt;
            f.pos.y -= 40.0 * dt;
        }
        self.floaters.retain(|f| f.t < FLOATER_LIFE);
        let drag = (-4.0 * dt).exp();
        for p in &mut self.particles {
            p.pos += p.vel * dt;
            p.vel = p.vel * drag;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
        for t in &mut self.flash {
            *t = (*t - dt).max(0.0);
        }
        self.shake = (self.shake - dt * feel.shake.decay).max(0.0);
    }
}
