//! Basic combat AI: approach / keep distance, strafe, dodge incoming hits and
//! pick moves by situation. Produces the same [`Intent`] a player would.

use crate::data::{AiProfile, ArenaDef, GameData, MoveDef, MoveShape};
use crate::hitbox::Shape;
use crate::math::Vec2;
use crate::rng::Rng;
use crate::sim::{Action, Battle, FighterView, Intent};

#[derive(Debug, Clone)]
pub struct AiController {
    profile: AiProfile,
    rng: Rng,
    think_timer: f32,
    strafe_sign: f32,
    strafe_timer: f32,
    dodge_dir: Vec2,
    dodge_timer: f32,
    move_dir: Vec2,
}

impl AiController {
    pub fn new(profile: AiProfile, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let strafe_sign = if rng.chance(0.5) { 1.0 } else { -1.0 };
        Self {
            profile,
            rng,
            think_timer: 0.0,
            strafe_sign,
            strafe_timer: 0.0,
            dodge_dir: Vec2::ZERO,
            dodge_timer: 0.0,
            move_dir: Vec2::ZERO,
        }
    }

    /// Decide this tick's intent for whoever `side` currently has on the field.
    pub fn update(&mut self, battle: &Battle, side: usize) -> Intent {
        let data = &*battle.data;
        let dt = data.dt();
        let me_v = battle.fighter(battle.active(side));
        if !me_v.fighter.active() {
            return Intent::default();
        }
        let Some(target) = nearest_enemy(battle, &me_v) else {
            return Intent::default();
        };
        let my_pos = me_v.body.pos;
        let to = target.body.pos - my_pos;
        let dist = to.length();
        let mut intent = Intent {
            aim: target.body.pos,
            ..Default::default()
        };

        self.think_timer -= dt;
        self.strafe_timer -= dt;
        self.dodge_timer -= dt;

        if self.think_timer <= 0.0 {
            self.think_timer = self.profile.reaction_time * self.rng.range(0.8, 1.2);
            if self.strafe_timer <= 0.0 {
                self.strafe_timer = self.profile.strafe_switch * self.rng.range(0.5, 1.5);
                self.strafe_sign = -self.strafe_sign;
            }

            // 1. Dodge whatever is about to hit us.
            if let Some(away) = detect_threat(battle, &me_v, self.profile.threat_lookahead) {
                if self.rng.chance(self.profile.dodge_chance) {
                    let away = steer_from_walls(away, my_pos, me_v.body.radius, battle.arena());
                    self.dodge_dir = away;
                    self.dodge_timer = 0.25;
                    intent.dash = me_v.fighter.dash_cooldown <= 0.0;
                }
            }

            // 2. Movement: hold the preferred range for the best ready move.
            let best = self.best_move(battle, &me_v, &target, false);
            let pref = match best {
                Some((slot, _)) => preferred_range(&data.moves[me_v.fighter.slots[slot].mv], target.body.radius),
                None => self.profile.kite_range,
            };
            let to_n = to.normalize_or_zero();
            let radial = if dist > pref + self.profile.range_band {
                1.0
            } else if dist < pref - self.profile.range_band {
                -1.0
            } else {
                0.0
            };
            let strafe = to_n.perp() * (self.strafe_sign * self.profile.strafe_amount);
            let mut dir = to_n * radial + strafe;
            dir = avoid_obstacles(dir, my_pos, me_v.body.radius, battle.arena());
            self.move_dir = dir.normalize_or_zero();

            // 3. Switch out of a bad matchup or when nearly fainted.
            if let Some(slot) = self.pick_switch(battle, side, &me_v, &target) {
                intent.switch_to = Some(slot);
                return intent;
            }

            // 4. Attack.
            if me_v.fighter.action == Action::Idle && self.rng.chance(self.profile.aggression) {
                if let Some((slot, _)) = self.best_move(battle, &me_v, &target, true) {
                    intent.use_move = Some(slot);
                    intent.aim = self.aim_point(data, &me_v, &target, slot);
                }
            }
        }

        intent.move_dir = if self.dodge_timer > 0.0 {
            self.dodge_dir
        } else {
            self.move_dir
        };
        intent
    }

    fn pick_switch(&mut self, battle: &Battle, side: usize, me: &FighterView, target: &FighterView) -> Option<usize> {
        let team = &battle.teams()[side];
        if team.members.len() < 2 || team.switch_cooldown > 0.0 || !self.rng.chance(self.profile.switch_chance) {
            return None;
        }
        let data = &*battle.data;
        let foe = target.fighter.creature;
        let current = matchup(data, me.fighter.creature, foe);
        let hp = me.fighter.hp / me.fighter.max_hp;
        let mut best: Option<(usize, f32)> = None;
        for (slot, &m) in team.members.iter().enumerate() {
            let f = battle.fighter(m);
            if m == me.fighter.index || !f.fighter.alive() {
                continue;
            }
            // Prefer healthy teammates with a better type matchup.
            let score = matchup(data, f.fighter.creature, foe) + 0.3 * (f.fighter.hp / f.fighter.max_hp);
            if best.is_none_or(|(_, b)| score > b) {
                best = Some((slot, score));
            }
        }
        let (slot, score) = best?;
        let bad_matchup = score - (current + 0.3 * hp) > 0.5;
        let desperate = hp < 0.2 && score > current;
        (bad_matchup || desperate).then_some(slot)
    }

    /// Highest expected-damage ready move. With `in_range_only`, only moves
    /// that can connect from the current position are considered.
    fn best_move(&self, battle: &Battle, me: &FighterView, target: &FighterView, in_range_only: bool) -> Option<(usize, f32)> {
        let data = &*battle.data;
        let dist = me.body.pos.distance(target.body.pos);
        let tc = &data.creatures[target.fighter.creature];
        let mc = &data.creatures[me.fighter.creature];
        let los = battle.line_of_sight(me.body.pos, target.body.pos, 4.0);
        let mut best: Option<(usize, f32)> = None;
        for (i, slot) in me.fighter.slots.iter().enumerate() {
            if !slot.ready() {
                continue;
            }
            let mv = &data.moves[slot.mv];
            let mut mult = data.types.multiplier(mv.element, &tc.types);
            if mc.types.contains(&mv.element) {
                mult *= data.balance.stab;
            }
            let hit_p = match mv.shape {
                MoveShape::Projectile { speed, range, count, .. } => {
                    if in_range_only && (dist > range * 0.95 || !los) {
                        continue;
                    }
                    let tof = dist / speed;
                    let p = (1.1 - tof).clamp(0.25, 1.0);
                    let closeness = (1.0 - dist / range).clamp(0.0, 1.0);
                    p * (1.0 + (count as f32 - 1.0) * closeness * 0.5)
                }
                MoveShape::Melee { range, .. } => {
                    if in_range_only && dist > range + target.body.radius - 4.0 {
                        continue;
                    }
                    0.9
                }
                MoveShape::Burst {
                    radius,
                    delay,
                    max_distance,
                    ..
                } => {
                    if max_distance <= 0.0 {
                        if in_range_only && dist > radius + target.body.radius * 0.5 {
                            continue;
                        }
                        0.85
                    } else {
                        if in_range_only && dist > max_distance + radius * 0.5 {
                            continue;
                        }
                        (1.0 - delay * 0.6).clamp(0.3, 0.8)
                    }
                }
            };
            let score = mv.power * mult * hit_p;
            if best.is_none_or(|(_, s)| score > s) {
                best = Some((i, score));
            }
        }
        best
    }

    fn aim_point(&mut self, data: &GameData, me: &FighterView, target: &FighterView, slot: usize) -> Vec2 {
        let mv = &data.moves[me.fighter.slots[slot].mv];
        let dist = me.body.pos.distance(target.body.pos);
        let lead_time = mv.windup
            + match mv.shape {
                MoveShape::Projectile { speed, .. } => dist / speed,
                MoveShape::Burst { delay, .. } => delay * 0.5,
                MoveShape::Melee { .. } => 0.0,
            };
        let predicted = target.body.pos + target.body.vel * (lead_time * self.profile.lead);
        let err = self.profile.aim_error_deg.to_radians();
        let off = (predicted - me.body.pos).rotate(self.rng.range(-err, err));
        me.body.pos + off
    }
}

/// How well creature `a` fares against `b` on types alone: best STAB-weighted
/// multiplier `a` can deal minus the best `b` can deal back.
pub fn matchup(data: &GameData, a: usize, b: usize) -> f32 {
    let best = |x: usize, y: usize| {
        let (cx, cy) = (&data.creatures[x], &data.creatures[y]);
        cx.moves
            .iter()
            .map(|&m| {
                let mv = &data.moves[m];
                let stab = if cx.types.contains(&mv.element) { data.balance.stab } else { 1.0 };
                data.types.multiplier(mv.element, &cy.types) * stab
            })
            .fold(0.0, f32::max)
    };
    best(a, b) - best(b, a)
}

pub fn nearest_enemy(battle: &Battle, me: &FighterView) -> Option<FighterView> {
    (0..battle.fighter_count())
        .map(|i| battle.fighter(i))
        .filter(|f| f.fighter.team != me.fighter.team && f.fighter.active())
        .min_by(|a, b| {
            let da = a.body.pos.distance(me.body.pos);
            let db = b.body.pos.distance(me.body.pos);
            da.total_cmp(&db)
        })
}

/// Distance from which a move is comfortable to use.
pub fn preferred_range(mv: &MoveDef, target_radius: f32) -> f32 {
    match mv.shape {
        MoveShape::Projectile { range, .. } => (range * 0.55).min(360.0),
        MoveShape::Melee { range, .. } => range * 0.7 + target_radius,
        MoveShape::Burst {
            radius, max_distance, ..
        } => {
            if max_distance <= 0.0 {
                radius * 0.6
            } else {
                max_distance * 0.7
            }
        }
    }
}

/// Direction to escape an incoming hit, if one is predicted.
pub fn detect_threat(battle: &Battle, me: &FighterView, lookahead: f32) -> Option<Vec2> {
    let data = &*battle.data;
    let pos = me.body.pos;
    let r = me.body.radius;
    for p in battle.projectiles() {
        if p.team == me.fighter.team {
            continue;
        }
        let vv = p.vel.length_sq();
        if vv <= 0.0 {
            continue;
        }
        let t = (pos - p.pos).dot(p.vel) / vv;
        if t < 0.0 || t > lookahead {
            continue;
        }
        let closest = p.pos + p.vel * t;
        let off = pos - closest;
        if off.length() < r + p.radius + 10.0 {
            let side = if off.length() > 1.0 {
                off.normalize_or_zero()
            } else {
                p.vel.perp().normalize_or_zero()
            };
            return Some(side);
        }
    }
    for a in battle.areas() {
        if a.team == me.fighter.team {
            continue;
        }
        let reach = match a.shape {
            Shape::Circle { radius } => radius,
            Shape::Arc { range, .. } => range,
        };
        if a.center.distance(pos) < reach + r + 12.0 {
            let away = (pos - a.center).normalize_or_zero();
            return Some(if away == Vec2::ZERO { Vec2::X } else { away });
        }
    }
    for i in 0..battle.fighter_count() {
        let o = battle.fighter(i);
        if o.fighter.team == me.fighter.team || !o.fighter.active() {
            continue;
        }
        if let Action::Windup { slot, .. } = o.fighter.action {
            let mv = &data.moves[o.fighter.slots[slot].mv];
            let danger = match mv.shape {
                MoveShape::Melee { range, .. } => range + 20.0,
                MoveShape::Burst {
                    radius, max_distance, ..
                } if max_distance <= 0.0 => radius + 15.0,
                _ => 0.0,
            };
            if o.body.pos.distance(pos) < danger + r {
                return Some((pos - o.body.pos).normalize_or_zero());
            }
        }
    }
    None
}

fn steer_from_walls(dir: Vec2, pos: Vec2, r: f32, a: &ArenaDef) -> Vec2 {
    let probe = pos + dir * 90.0;
    let mut d = dir;
    if probe.x < r || probe.x > a.width - r {
        d.x = -d.x;
    }
    if probe.y < r || probe.y > a.height - r {
        d.y = -d.y;
    }
    d.normalize_or_zero()
}

fn avoid_obstacles(dir: Vec2, pos: Vec2, r: f32, a: &ArenaDef) -> Vec2 {
    let margin = 70.0;
    let mut push = Vec2::ZERO;
    if pos.x < margin + r {
        push.x += 1.0;
    }
    if pos.x > a.width - margin - r {
        push.x -= 1.0;
    }
    if pos.y < margin + r {
        push.y += 1.0;
    }
    if pos.y > a.height - margin - r {
        push.y -= 1.0;
    }
    let mut d = dir + push * 0.8;
    for p in &a.pillars {
        let to = p.pos - pos;
        let dist = to.length();
        let clear = p.radius + r + 30.0;
        if dist < clear + 40.0 && d.dot(to) > 0.0 {
            // Slide around the pillar on the side we're already heading.
            let tangent = to.perp().normalize_or_zero();
            let t = if tangent.dot(d) >= 0.0 { tangent } else { -tangent };
            d = d + t * 0.9 - to.normalize_or_zero() * 0.3;
        }
    }
    d
}
