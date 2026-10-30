//! Fixed-timestep battle simulation. No rendering, no input devices: callers
//! feed one [`Intent`] per fighter per tick and read state/events back.

use crate::damage::{self, AttackerStats};
use crate::data::{ArenaDef, CreatureId, GameData, MoveId, MoveShape, StatusId};
use crate::hitbox::{self, Shape};
use crate::math::Vec2;
use crate::rng::Rng;
use hecs::{Entity, World};
use std::sync::Arc;

/// What a controller (player input or AI) wants a fighter to do this tick.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Intent {
    /// Desired movement direction; length is clamped to 1.
    pub move_dir: Vec2,
    /// World-space point the fighter aims at.
    pub aim: Vec2,
    pub dash: bool,
    /// Move slot (0-3) to use, if any.
    pub use_move: Option<usize>,
    /// Roster slot to swap in (team battles).
    pub switch_to: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Idle,
    Windup { slot: usize, t: f32, aim: Vec2 },
    Recovery { t: f32 },
    Dash { t: f32, dir: Vec2 },
    Hitstun { t: f32 },
    Fainted,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveSlot {
    pub mv: MoveId,
    /// Seconds until usable again.
    pub remaining: f32,
}

impl MoveSlot {
    pub fn ready(&self) -> bool {
        self.remaining <= 0.0
    }

    pub fn tick(&mut self, dt: f32) {
        self.remaining = (self.remaining - dt).max(0.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Body {
    pub pos: Vec2,
    pub vel: Vec2,
    /// Externally applied velocity (knockback, lunges), decays over time.
    pub knock: Vec2,
    pub radius: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FighterStats {
    pub damage_dealt: f32,
    pub hits_landed: u32,
    pub moves_used: u32,
    pub dashes: u32,
    pub dodges: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActiveStatus {
    pub status: StatusId,
    pub remaining: f32,
    /// Fighter credited with damage over time.
    pub source: usize,
}

#[derive(Debug, Clone)]
pub struct Fighter {
    pub index: usize,
    pub team: u8,
    pub creature: CreatureId,
    pub hp: f32,
    pub max_hp: f32,
    pub slots: Vec<MoveSlot>,
    pub dash_cooldown: f32,
    pub iframes: f32,
    pub action: Action,
    pub facing: Vec2,
    pub status: Option<ActiveStatus>,
    /// Off the field (team battles): not simulated, drawn or targetable.
    pub benched: bool,
    pub stats: FighterStats,
}

impl Fighter {
    pub fn alive(&self) -> bool {
        self.action != Action::Fainted
    }

    /// On the field and able to fight.
    pub fn active(&self) -> bool {
        self.alive() && !self.benched
    }
}

/// One side of a battle: its roster and who is currently out.
#[derive(Debug, Clone)]
pub struct Team {
    /// Fighter indices, in roster order.
    pub members: Vec<usize>,
    /// Fighter index currently on the field.
    pub active: usize,
    pub switch_cooldown: f32,
    /// Counts down after the active fighter faints, then the next is sent out.
    pub send_out_in: Option<f32>,
    pub switches: u32,
}

#[derive(Debug, Clone)]
pub struct Projectile {
    pub owner: usize,
    pub team: u8,
    pub mv: MoveId,
    pub attacker: AttackerStats,
    pub pos: Vec2,
    pub vel: Vec2,
    pub radius: f32,
    pub life: f32,
    pub pierce: bool,
    hit: Vec<usize>,
    dodged: Vec<usize>,
}

/// Melee arcs and bursts: a stationary or owner-following hit area.
#[derive(Debug, Clone)]
pub struct AreaHit {
    pub owner: usize,
    pub team: u8,
    pub mv: MoveId,
    pub attacker: AttackerStats,
    pub shape: Shape,
    pub center: Vec2,
    pub follow_owner: bool,
    /// Telegraph time remaining before the area becomes dangerous.
    pub delay: f32,
    pub delay_total: f32,
    pub active: f32,
    pub active_total: f32,
    hit: Vec<usize>,
    dodged: Vec<usize>,
}

impl AreaHit {
    pub fn is_active(&self) -> bool {
        self.delay <= 0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    MoveStarted { fighter: usize, mv: MoveId },
    MoveLaunched { fighter: usize, mv: MoveId },
    Hit { attacker: usize, target: usize, mv: MoveId, damage: f32, type_mult: f32, pos: Vec2 },
    Dodged { fighter: usize, pos: Vec2 },
    Dash { fighter: usize },
    Fainted { fighter: usize },
    /// `from` was recalled (or fainted) and `to` sent out for `team`.
    SentOut { team: u8, from: usize, to: usize, voluntary: bool },
    StatusApplied { fighter: usize, status: StatusId },
    /// `broken`: ended early by taking a hit (e.g. waking from sleep).
    StatusEnded { fighter: usize, status: StatusId, broken: bool },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// A team knocked out every opponent.
    Victory { team: u8 },
    /// Time ran out; `team` had the higher remaining HP fraction (None = draw).
    Timeout { team: Option<u8> },
}

impl Outcome {
    pub fn winner(&self) -> Option<u8> {
        match *self {
            Outcome::Victory { team } => Some(team),
            Outcome::Timeout { team } => team,
        }
    }
}

/// Read-only copy of a fighter's state for AI and rendering.
#[derive(Debug, Clone)]
pub struct FighterView {
    pub body: Body,
    pub fighter: Fighter,
}

struct Launch {
    owner: usize,
    team: u8,
    mv: MoveId,
    origin: Vec2,
    radius: f32,
    aim: Vec2,
    attacker: AttackerStats,
}

struct PendingHit {
    attacker: usize,
    target: usize,
    mv: MoveId,
    stats: AttackerStats,
    push_dir: Vec2,
    pos: Vec2,
}

pub struct Battle {
    pub data: Arc<GameData>,
    world: World,
    fighters: Vec<Entity>,
    teams: Vec<Team>,
    arena: ArenaDef,
    tick: u64,
    rng: Rng,
    events: Vec<Event>,
    outcome: Option<Outcome>,
}

impl Battle {
    /// 1v1 (or free-for-all): one creature per side.
    pub fn new(data: Arc<GameData>, creatures: &[CreatureId], seed: u64) -> Self {
        let rosters: Vec<Vec<CreatureId>> = creatures.iter().map(|&c| vec![c]).collect();
        Self::with_rosters(data, &rosters, seed)
    }

    /// Team battle in the first arena: side `i` fields `rosters[i][0]` first;
    /// the rest wait on the bench.
    pub fn with_rosters(data: Arc<GameData>, rosters: &[Vec<CreatureId>], seed: u64) -> Self {
        Self::with_arena(data, rosters, 0, seed)
    }

    /// Team battle in arena `arena` (index into `GameData::arenas`).
    pub fn with_arena(data: Arc<GameData>, rosters: &[Vec<CreatureId>], arena: usize, seed: u64) -> Self {
        assert!(rosters.iter().all(|r| !r.is_empty()), "every side needs a creature");
        let arena = data.arenas[arena].clone();
        let mut world = World::new();
        let mut fighters = Vec::new();
        let mut teams = Vec::new();
        let center = Vec2::new(arena.width * 0.5, arena.height * 0.5);
        for (side, roster) in rosters.iter().enumerate() {
            let pos = arena.spawns[side % arena.spawns.len()];
            let mut members = Vec::new();
            for (k, &cid) in roster.iter().enumerate() {
                let c = &data.creatures[cid];
                let index = fighters.len();
                let body = Body {
                    pos,
                    radius: c.radius,
                    ..Default::default()
                };
                let fighter = Fighter {
                    index,
                    team: side as u8,
                    creature: cid,
                    hp: c.hp,
                    max_hp: c.hp,
                    slots: c.moves.iter().map(|&mv| MoveSlot { mv, remaining: 0.0 }).collect(),
                    dash_cooldown: 0.0,
                    iframes: 0.0,
                    action: Action::Idle,
                    facing: (center - pos).normalize_or_zero(),
                    status: None,
                    benched: k > 0,
                    stats: FighterStats::default(),
                };
                fighters.push(world.spawn((body, fighter)));
                members.push(index);
            }
            teams.push(Team {
                active: members[0],
                members,
                switch_cooldown: 0.0,
                send_out_in: None,
                switches: 0,
            });
        }
        Self {
            data,
            world,
            fighters,
            teams,
            arena,
            tick: 0,
            rng: Rng::new(seed),
            events: Vec::new(),
            outcome: None,
        }
    }

    pub fn arena(&self) -> &ArenaDef {
        &self.arena
    }

    pub fn teams(&self) -> &[Team] {
        &self.teams
    }

    /// Battle length limit: `max_battle_seconds` per creature in the largest roster.
    pub fn time_limit(&self) -> f32 {
        let size = self.teams.iter().map(|t| t.members.len()).max().unwrap_or(1);
        self.data.balance.max_battle_seconds * size as f32
    }

    /// Fighter index currently on the field for `side`.
    pub fn active(&self, side: usize) -> usize {
        self.teams[side].active
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn time(&self) -> f32 {
        self.tick as f32 * self.data.dt()
    }

    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    /// Events produced by the most recent [`Battle::step`].
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn fighter_count(&self) -> usize {
        self.fighters.len()
    }

    pub fn fighter(&self, i: usize) -> FighterView {
        let e = self.fighters[i];
        let body = *self.world.get::<&Body>(e).expect("fighter body");
        let fighter = (*self.world.get::<&Fighter>(e).expect("fighter")).clone();
        FighterView { body, fighter }
    }

    pub fn projectiles(&self) -> Vec<Projectile> {
        self.world.query::<&Projectile>().iter().cloned().collect()
    }

    pub fn areas(&self) -> Vec<AreaHit> {
        self.world.query::<&AreaHit>().iter().cloned().collect()
    }

    /// True if the straight line between two points is not blocked by a pillar.
    pub fn line_of_sight(&self, a: Vec2, b: Vec2, margin: f32) -> bool {
        !self
            .arena
            .pillars
            .iter()
            .any(|p| hitbox::segment_hits_circle(a, b, p.pos, p.radius + margin))
    }

    pub fn step(&mut self, intents: &[Intent]) {
        if self.outcome.is_some() {
            return;
        }
        self.events.clear();
        self.tick += 1;
        let data = self.data.clone();
        let dt = data.dt();

        self.update_teams(intents, dt);
        let launches = self.update_fighters(intents, dt);
        self.integrate_fighters(dt);
        for l in launches {
            self.spawn_attack(l);
        }
        let mut hits = Vec::new();
        self.update_projectiles(dt, &mut hits);
        self.update_areas(dt, &mut hits);
        self.apply_hits(hits);
        self.check_outcome();
    }

    /// Swap the active fighter of `side` for fighter `to` at the same spot.
    fn swap_in(&mut self, side: usize, to: usize, voluntary: bool) {
        let data = self.data.clone();
        let from = self.teams[side].active;
        let (pos, facing) = {
            let b = *self.world.get::<&Body>(self.fighters[from]).expect("body");
            let f = self.world.get::<&Fighter>(self.fighters[from]).expect("fighter");
            let pos = if voluntary { b.pos } else { self.arena.spawns[side % self.arena.spawns.len()] };
            (pos, f.facing)
        };
        if let Ok(q) = self.world.query_one_mut::<(&mut Body, &mut Fighter)>(self.fighters[from]) {
            q.0.vel = Vec2::ZERO;
            q.0.knock = Vec2::ZERO;
            q.1.benched = true;
            if q.1.alive() {
                q.1.action = Action::Idle;
            }
        }
        if let Ok(q) = self.world.query_one_mut::<(&mut Body, &mut Fighter)>(self.fighters[to]) {
            q.0.pos = pos;
            q.0.vel = Vec2::ZERO;
            q.0.knock = Vec2::ZERO;
            resolve_static(&mut q.0.pos, q.0.radius, &self.arena);
            q.1.benched = false;
            q.1.facing = facing;
            q.1.action = Action::Idle;
            q.1.iframes = data.balance.switching.swap_iframes;
        }
        let t = &mut self.teams[side];
        t.active = to;
        t.send_out_in = None;
        if voluntary {
            t.switch_cooldown = data.balance.switching.cooldown;
            t.switches += 1;
        }
        self.events.push(Event::SentOut {
            team: side as u8,
            from,
            to,
            voluntary,
        });
    }

    fn update_teams(&mut self, intents: &[Intent], dt: f32) {
        let sw = self.data.balance.switching.clone();
        for side in 0..self.teams.len() {
            let t = &mut self.teams[side];
            t.switch_cooldown = (t.switch_cooldown - dt).max(0.0);
            let active = t.active;
            let (active_alive, can_act) = {
                let f = self.world.get::<&Fighter>(self.fighters[active]).expect("fighter");
                let stunned = f.status.is_some_and(|s| self.data.statuses[s.status].stun);
                (f.alive(), matches!(f.action, Action::Idle | Action::Recovery { .. }) && !stunned)
            };
            let next_alive = |members: &[usize], world: &World, fighters: &[Entity]| {
                members
                    .iter()
                    .copied()
                    .find(|&m| m != active && world.get::<&Fighter>(fighters[m]).map(|f| f.alive()).unwrap_or(false))
            };
            if !active_alive {
                // Forced send-out after the faint delay.
                let t = &mut self.teams[side];
                match t.send_out_in {
                    None => t.send_out_in = Some(sw.faint_delay),
                    Some(left) if left - dt > 0.0 => t.send_out_in = Some(left - dt),
                    Some(_) => {
                        if let Some(n) = next_alive(&t.members.clone(), &self.world, &self.fighters) {
                            self.swap_in(side, n, false);
                        } else {
                            t.send_out_in = None;
                        }
                    }
                }
                continue;
            }
            // Voluntary switch.
            let Some(slot) = intents.get(side).and_then(|i| i.switch_to) else { continue };
            let t = &self.teams[side];
            let Some(&to) = t.members.get(slot) else { continue };
            let target_ok = to != active
                && self
                    .world
                    .get::<&Fighter>(self.fighters[to])
                    .map(|f| f.alive())
                    .unwrap_or(false);
            if target_ok && can_act && t.switch_cooldown <= 0.0 {
                self.swap_in(side, to, true);
            }
        }
    }

    fn update_fighters(&mut self, intents: &[Intent], dt: f32) -> Vec<Launch> {
        let data = &*self.data;
        let bal = &data.balance;
        let mut launches = Vec::new();
        let mut events = Vec::new();
        let mut dot_credit: Vec<(usize, f32)> = Vec::new();
        for (i, &e) in self.fighters.iter().enumerate() {
            let mut q = self.world.query_one::<(&mut Body, &mut Fighter)>(e);
            let (body, f) = q.get().expect("fighter components");
            let side = f.team as usize;
            let intent = if self.teams[side].active == i {
                intents.get(side).copied().unwrap_or_default()
            } else {
                Intent::default()
            };
            if f.benched {
                // Benched creatures recover their cooldowns, nothing else.
                for s in &mut f.slots {
                    s.tick(dt);
                }
                continue;
            }
            if !f.alive() {
                body.vel = Vec2::ZERO;
                continue;
            }
            let creature = &data.creatures[f.creature];
            for s in &mut f.slots {
                s.tick(dt);
            }
            f.dash_cooldown = (f.dash_cooldown - dt).max(0.0);
            f.iframes = (f.iframes - dt).max(0.0);

            // Status: damage over time, expiry.
            let mut stunned = false;
            let (mut speed_mult, mut windup_mult, mut attack_mult) = (1.0, 1.0, 1.0);
            if let Some(st) = &mut f.status {
                let def = &data.statuses[st.status];
                let dot = (f.max_hp * def.damage_per_sec * dt).min(f.hp);
                if dot > 0.0 {
                    f.hp -= dot;
                    dot_credit.push((st.source, dot));
                }
                st.remaining -= dt;
                let id = st.status;
                if f.hp <= 0.0 {
                    f.hp = 0.0;
                    f.action = Action::Fainted;
                    events.push(Event::Fainted { fighter: i });
                    body.vel = Vec2::ZERO;
                    continue;
                }
                if st.remaining <= 0.0 {
                    f.status = None;
                    events.push(Event::StatusEnded { fighter: i, status: id, broken: false });
                } else {
                    stunned = def.stun;
                    speed_mult = def.speed_mult;
                    windup_mult = def.windup_mult;
                    attack_mult = def.attack_mult;
                }
            }

            // Advance the current action.
            f.action = match f.action {
                Action::Windup { slot, t, aim } if t - dt <= 0.0 => {
                    let mv = f.slots[slot].mv;
                    launches.push(Launch {
                        owner: i,
                        team: f.team,
                        mv,
                        origin: body.pos,
                        radius: body.radius,
                        aim,
                        attacker: AttackerStats {
                            attack: creature.attack * attack_mult,
                            types: creature.types.clone(),
                        },
                    });
                    events.push(Event::MoveLaunched { fighter: i, mv });
                    f.stats.moves_used += 1;
                    let rec = data.moves[mv].recovery;
                    if rec > 0.0 {
                        Action::Recovery { t: rec }
                    } else {
                        Action::Idle
                    }
                }
                Action::Windup { slot, t, aim } => Action::Windup { slot, t: t - dt, aim },
                Action::Recovery { t } | Action::Dash { t, .. } | Action::Hitstun { t } if t - dt <= 0.0 => {
                    Action::Idle
                }
                Action::Recovery { t } => Action::Recovery { t: t - dt },
                Action::Dash { t, dir } => Action::Dash { t: t - dt, dir },
                Action::Hitstun { t } => Action::Hitstun { t: t - dt },
                a => a,
            };

            let intent = if stunned { Intent { aim: intent.aim, ..Default::default() } } else { intent };
            let move_dir = intent.move_dir.clamp_length(1.0);
            let aim_dir = (intent.aim - body.pos).normalize_or_zero();

            // Dash: from idle, or cancelling recovery if allowed.
            let can_dash = matches!(f.action, Action::Idle)
                || (bal.dash.cancels_recovery && matches!(f.action, Action::Recovery { .. }));
            if intent.dash && f.dash_cooldown <= 0.0 && can_dash {
                let dir = if move_dir.length_sq() > 0.01 {
                    move_dir.normalize_or_zero()
                } else {
                    f.facing
                };
                f.action = Action::Dash { t: bal.dash.duration, dir };
                f.iframes = bal.dash.iframes;
                f.dash_cooldown = bal.dash.cooldown;
                f.stats.dashes += 1;
                events.push(Event::Dash { fighter: i });
            }

            if let Some(slot) = intent.use_move {
                if f.action == Action::Idle && slot < f.slots.len() && f.slots[slot].ready() {
                    let mv = f.slots[slot].mv;
                    f.slots[slot].remaining = data.moves[mv].cooldown;
                    f.action = Action::Windup {
                        slot,
                        t: data.moves[mv].windup * windup_mult,
                        aim: intent.aim,
                    };
                    if aim_dir != Vec2::ZERO {
                        f.facing = aim_dir;
                    }
                    events.push(Event::MoveStarted { fighter: i, mv });
                }
            }

            if f.action == Action::Idle && aim_dir != Vec2::ZERO && !stunned {
                f.facing = aim_dir;
            }

            let speed = creature.speed * speed_mult;
            let desired = match f.action {
                Action::Idle => move_dir * speed,
                Action::Windup { .. } | Action::Recovery { .. } => move_dir * speed * bal.attack_move_mult,
                Action::Dash { dir, .. } => dir * speed * bal.dash.speed,
                Action::Hitstun { .. } | Action::Fainted => Vec2::ZERO,
            };
            if matches!(f.action, Action::Dash { .. }) {
                body.vel = desired;
            } else {
                let k = 1.0 - (-bal.acceleration * dt).exp();
                body.vel = body.vel.lerp(desired, k);
            }
        }
        self.events.extend(events);
        for (src, dmg) in dot_credit {
            if let Ok(mut f) = self.world.get::<&mut Fighter>(self.fighters[src]) {
                f.stats.damage_dealt += dmg;
            }
        }
        launches
    }

    fn integrate_fighters(&mut self, dt: f32) {
        let data = &*self.data;
        let decay = (-data.balance.knockback_decay * dt).exp();
        for &e in &self.fighters {
            let mut body = self.world.get::<&mut Body>(e).expect("body");
            let step = (body.vel + body.knock) * dt;
            body.pos += step;
            body.knock = body.knock * decay;
            let r = body.radius;
            resolve_static(&mut body.pos, r, &self.arena);
        }
        // Fighter-fighter separation.
        let n = self.fighters.len();
        for a in 0..n {
            for b in (a + 1)..n {
                let (ea, eb) = (self.fighters[a], self.fighters[b]);
                let alive = |e: Entity| self.world.get::<&Fighter>(e).map(|f| f.active()).unwrap_or(false);
                if !alive(ea) || !alive(eb) {
                    continue;
                }
                // hecs borrows per archetype column, so copy out, then write back.
                let mut ba = *self.world.get::<&Body>(ea).expect("body");
                let mut bb = *self.world.get::<&Body>(eb).expect("body");
                let d = bb.pos - ba.pos;
                let min = ba.radius + bb.radius;
                let dist = d.length();
                if dist < min {
                    let n = if dist > 1e-4 { d * (1.0 / dist) } else { Vec2::X };
                    let push = n * ((min - dist) * 0.5);
                    ba.pos -= push;
                    bb.pos += push;
                    resolve_static(&mut ba.pos, ba.radius, &self.arena);
                    resolve_static(&mut bb.pos, bb.radius, &self.arena);
                    *self.world.get::<&mut Body>(ea).expect("body") = ba;
                    *self.world.get::<&mut Body>(eb).expect("body") = bb;
                }
            }
        }
    }

    fn spawn_attack(&mut self, l: Launch) {
        let data = self.data.clone();
        let mv = &data.moves[l.mv];
        let dir = {
            let d = (l.aim - l.origin).normalize_or_zero();
            if d == Vec2::ZERO {
                self.world.get::<&Fighter>(self.fighters[l.owner]).map(|f| f.facing).unwrap_or(Vec2::X)
            } else {
                d
            }
        };
        match mv.shape {
            MoveShape::Projectile {
                speed,
                radius,
                range,
                count,
                spread_deg,
                pierce,
            } => {
                let spread = spread_deg.to_radians();
                for k in 0..count {
                    let off = if count > 1 {
                        -spread * 0.5 + spread * k as f32 / (count - 1) as f32
                    } else {
                        0.0
                    };
                    let d = dir.rotate(off);
                    self.world.spawn((Projectile {
                        owner: l.owner,
                        team: l.team,
                        mv: l.mv,
                        attacker: l.attacker.clone(),
                        pos: l.origin + d * (l.radius + radius),
                        vel: d * speed,
                        radius,
                        life: range / speed,
                        pierce,
                        hit: Vec::new(),
                        dodged: Vec::new(),
                    },));
                }
            }
            MoveShape::Melee {
                range,
                arc_deg,
                active,
                lunge,
            } => {
                if let Ok(mut b) = self.world.get::<&mut Body>(self.fighters[l.owner]) {
                    b.knock += dir * lunge;
                }
                self.world.spawn((AreaHit {
                    owner: l.owner,
                    team: l.team,
                    mv: l.mv,
                    attacker: l.attacker,
                    shape: Shape::Arc {
                        range,
                        dir,
                        half_arc: arc_deg.to_radians() * 0.5,
                    },
                    center: l.origin,
                    follow_owner: true,
                    delay: 0.0,
                    delay_total: 0.0,
                    active,
                    active_total: active,
                    hit: Vec::new(),
                    dodged: Vec::new(),
                },));
            }
            MoveShape::Burst {
                radius,
                delay,
                active,
                max_distance,
            } => {
                let center = if max_distance > 0.0 {
                    l.origin + (l.aim - l.origin).clamp_length(max_distance)
                } else {
                    l.origin
                };
                self.world.spawn((AreaHit {
                    owner: l.owner,
                    team: l.team,
                    mv: l.mv,
                    attacker: l.attacker,
                    shape: Shape::Circle { radius },
                    center,
                    follow_owner: false,
                    delay,
                    delay_total: delay,
                    active,
                    active_total: active,
                    hit: Vec::new(),
                    dodged: Vec::new(),
                },));
            }
        }
    }

    /// Living fighters as (index, team, pos, radius, iframes).
    fn targets(&self) -> Vec<(usize, u8, Vec2, f32, bool)> {
        self.fighters
            .iter()
            .filter_map(|&e| {
                let b = self.world.get::<&Body>(e).ok()?;
                let f = self.world.get::<&Fighter>(e).ok()?;
                f.active().then_some((f.index, f.team, b.pos, b.radius, f.iframes > 0.0))
            })
            .collect()
    }

    fn update_projectiles(&mut self, dt: f32, hits: &mut Vec<PendingHit>) {
        let targets = self.targets();
        let mut dead = Vec::new();
        for (e, p) in self.world.query_mut::<(Entity, &mut Projectile)>() {
            p.pos += p.vel * dt;
            p.life -= dt;
            let out = p.pos.x < 0.0 || p.pos.y < 0.0 || p.pos.x > self.arena.width || p.pos.y > self.arena.height;
            let blocked = self
                .arena
                .pillars
                .iter()
                .any(|pl| hitbox::circles_overlap(p.pos, p.radius, pl.pos, pl.radius));
            if p.life <= 0.0 || out || blocked {
                dead.push(e);
                continue;
            }
            for &(idx, team, pos, r, invuln) in &targets {
                if team == p.team || p.hit.contains(&idx) || !hitbox::circles_overlap(p.pos, p.radius, pos, r) {
                    continue;
                }
                if invuln {
                    if !p.dodged.contains(&idx) {
                        p.dodged.push(idx);
                        self.events.push(Event::Dodged { fighter: idx, pos });
                    }
                    continue;
                }
                p.hit.push(idx);
                hits.push(PendingHit {
                    attacker: p.owner,
                    target: idx,
                    mv: p.mv,
                    stats: p.attacker.clone(),
                    push_dir: p.vel.normalize_or_zero(),
                    pos: p.pos,
                });
                if !p.pierce {
                    dead.push(e);
                    break;
                }
            }
        }
        for e in dead {
            let _ = self.world.despawn(e);
        }
    }

    fn update_areas(&mut self, dt: f32, hits: &mut Vec<PendingHit>) {
        let targets = self.targets();
        let owner_pos: Vec<(Vec2, bool)> = self
            .fighters
            .iter()
            .map(|&e| {
                let b = self.world.get::<&Body>(e).map(|b| b.pos).unwrap_or_default();
                let alive = self.world.get::<&Fighter>(e).map(|f| f.active()).unwrap_or(false);
                (b, alive)
            })
            .collect();
        let mut dead = Vec::new();
        for (e, a) in self.world.query_mut::<(Entity, &mut AreaHit)>() {
            if a.follow_owner {
                let (pos, alive) = owner_pos[a.owner];
                if !alive {
                    dead.push(e);
                    continue;
                }
                a.center = pos;
            }
            if a.delay > 0.0 {
                a.delay -= dt;
                continue;
            }
            for &(idx, team, pos, r, invuln) in &targets {
                if team == a.team || a.hit.contains(&idx) || !hitbox::shape_hits_circle(&a.shape, a.center, pos, r) {
                    continue;
                }
                if invuln {
                    if !a.dodged.contains(&idx) {
                        a.dodged.push(idx);
                        self.events.push(Event::Dodged { fighter: idx, pos });
                    }
                    continue;
                }
                a.hit.push(idx);
                hits.push(PendingHit {
                    attacker: a.owner,
                    target: idx,
                    mv: a.mv,
                    stats: a.attacker.clone(),
                    push_dir: (pos - a.center).normalize_or_zero(),
                    pos,
                });
            }
            a.active -= dt;
            if a.active <= 0.0 {
                dead.push(e);
            }
        }
        for e in dead {
            let _ = self.world.despawn(e);
        }
    }

    fn apply_hits(&mut self, hits: Vec<PendingHit>) {
        let data = self.data.clone();
        for h in hits {
            let mv = &data.moves[h.mv];
            let roll = self.rng.f32();
            let te = self.fighters[h.target];
            let (dealt, type_mult) = {
                let mut q = self.world.query_one::<(&mut Body, &mut Fighter)>(te);
                let (body, f) = q.get().expect("target components");
                if !f.alive() {
                    continue;
                }
                let c = &data.creatures[f.creature];
                let r = damage::compute(mv, &h.stats, c.defense, &c.types, &data.types, &data.balance, roll);
                let dealt = r.amount.min(f.hp);
                f.hp -= r.amount;
                body.knock += h.push_dir * mv.knockback;
                if f.hp <= 0.0 {
                    f.hp = 0.0;
                    f.action = Action::Fainted;
                    self.events.push(Event::Fainted { fighter: h.target });
                } else {
                    if mv.hitstun > 0.0 {
                        f.action = Action::Hitstun { t: mv.hitstun };
                    }
                    // Hits wake sleepers (or end any break-on-hit status)...
                    if let Some(st) = f.status {
                        if dealt > 0.0 && data.statuses[st.status].break_on_hit {
                            f.status = None;
                            self.events.push(Event::StatusEnded {
                                fighter: h.target,
                                status: st.status,
                                broken: true,
                            });
                        }
                    }
                    // ...and may inflict one, if the target has none.
                    if let Some(proc_) = mv.status {
                        let def = &data.statuses[proc_.status];
                        let immune = c.types.iter().any(|t| def.immune_types.contains(t));
                        if f.status.is_none() && !immune && self.rng.chance(proc_.chance) {
                            f.status = Some(ActiveStatus {
                                status: proc_.status,
                                remaining: def.duration,
                                source: h.attacker,
                            });
                            if def.stun {
                                f.action = Action::Idle;
                            }
                            self.events.push(Event::StatusApplied {
                                fighter: h.target,
                                status: proc_.status,
                            });
                        }
                    }
                }
                (dealt, r.type_mult)
            };
            if let Ok(mut a) = self.world.get::<&mut Fighter>(self.fighters[h.attacker]) {
                a.stats.damage_dealt += dealt;
                a.stats.hits_landed += 1;
            }
            self.events.push(Event::Hit {
                attacker: h.attacker,
                target: h.target,
                mv: h.mv,
                damage: dealt,
                type_mult,
                pos: h.pos,
            });
        }
        // Count dodges (iframe evasions) per fighter.
        let dodgers: Vec<usize> = self
            .events
            .iter()
            .filter_map(|e| match e {
                Event::Dodged { fighter, .. } => Some(*fighter),
                _ => None,
            })
            .collect();
        for i in dodgers {
            if let Ok(mut f) = self.world.get::<&mut Fighter>(self.fighters[i]) {
                f.stats.dodges += 1;
            }
        }
    }

    fn check_outcome(&mut self) {
        let mut alive_teams: Vec<u8> = Vec::new();
        let mut hp_frac: Vec<(u8, f32)> = Vec::new();
        for &e in &self.fighters {
            let f = self.world.get::<&Fighter>(e).expect("fighter");
            if f.alive() && !alive_teams.contains(&f.team) {
                alive_teams.push(f.team);
            }
            let share = f.hp / f.max_hp / self.teams[f.team as usize].members.len() as f32;
            match hp_frac.iter_mut().find(|(t, _)| *t == f.team) {
                Some((_, v)) => *v += share,
                None => hp_frac.push((f.team, share)),
            }
        }
        if alive_teams.len() <= 1 {
            self.outcome = Some(match alive_teams.first() {
                Some(&team) => Outcome::Victory { team },
                None => Outcome::Timeout { team: None }, // simultaneous KO
            });
        } else if self.time() >= self.time_limit() {
            hp_frac.sort_by(|a, b| b.1.total_cmp(&a.1));
            let team = if (hp_frac[0].1 - hp_frac[1].1).abs() < 1e-4 {
                None
            } else {
                Some(hp_frac[0].0)
            };
            self.outcome = Some(Outcome::Timeout { team });
        }
    }
}

/// Keep a circle inside the arena and outside pillars.
pub fn resolve_static(pos: &mut Vec2, r: f32, a: &ArenaDef) {
    pos.x = pos.x.clamp(r, a.width - r);
    pos.y = pos.y.clamp(r, a.height - r);
    for p in &a.pillars {
        let d = *pos - p.pos;
        let min = r + p.radius;
        let dist = d.length();
        if dist < min {
            let n = if dist > 1e-4 { d * (1.0 / dist) } else { Vec2::X };
            *pos = p.pos + n * min;
        }
    }
}
