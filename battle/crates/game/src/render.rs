//! All macroquad drawing. Reads simulation state; never mutates it.

use crate::assets::{AssetDb, Rgb};
use crate::fx::Fx;
use crate::sprites::{pick_frame, Sprites};
use combat::data::{GameData, MoveShape};
use combat::hitbox::Shape;
use combat::sim::{Action, Battle, FighterView};
use combat::Outcome;
use macroquad::prelude::*;

pub const SCREEN_W: f32 = 1280.0;
pub const SCREEN_H: f32 = 768.0;
/// Top-left of the arena on screen; the arena is drawn at 1:1 scale.
pub const ARENA_X: f32 = 40.0;
pub const ARENA_Y: f32 = 64.0;
pub const HUD_Y: f32 = 676.0;

pub const PLAYER_KEYS: [&str; 4] = ["LMB / 1", "RMB / 2", "Q / 3", "E / 4"];

const BG: Color = Color::new(0.07, 0.08, 0.10, 1.0);
const PANEL: Color = Color::new(0.12, 0.13, 0.16, 1.0);
const TEXT: Color = Color::new(0.92, 0.93, 0.95, 1.0);
const DIM: Color = Color::new(0.55, 0.58, 0.63, 1.0);

pub fn rgb(c: Rgb, a: f32) -> Color {
    Color::new(c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0, a)
}

fn shade(c: Color, k: f32) -> Color {
    Color::new(c.r * k, c.g * k, c.b * k, c.a)
}

pub fn to_screen(p: combat::Vec2, shake: macroquad::math::Vec2) -> macroquad::math::Vec2 {
    vec2(ARENA_X + p.x + shake.x, ARENA_Y + p.y + shake.y)
}

pub fn text_centered(t: &str, cx: f32, y: f32, size: f32, color: Color) {
    let d = measure_text(t, None, size as u16, 1.0);
    draw_text(t, cx - d.width * 0.5, y, size, color);
}

fn type_id_color(data: &GameData, assets: &AssetDb, t: usize) -> Color {
    rgb(assets.type_color(&data.types.names[t]), 1.0)
}

fn move_color(data: &GameData, assets: &AssetDb, mv: usize) -> Color {
    type_id_color(data, assets, data.moves[mv].element)
}

/// Everything the renderer needs besides the battle itself.
pub struct Gfx<'a> {
    pub assets: &'a AssetDb,
    pub sprites: &'a Sprites,
}

pub fn draw_battle(battle: &Battle, gfx: &Gfx, fx: &Fx, player: usize) {
    let assets = gfx.assets;
    let data = &*battle.data;
    clear_background(BG);
    let shake = if fx.shake > 0.0 {
        let t = get_time() as f32 * 90.0;
        vec2(t.sin(), (t * 1.3).cos()) * fx.shake
    } else {
        vec2(0.0, 0.0)
    };

    // Floor.
    let a = &data.arena;
    draw_rectangle(ARENA_X + shake.x, ARENA_Y + shake.y, a.width, a.height, Color::new(0.16, 0.19, 0.17, 1.0));
    let grid = Color::new(1.0, 1.0, 1.0, 0.04);
    let mut x = 0.0;
    while x <= a.width {
        draw_line(ARENA_X + x + shake.x, ARENA_Y + shake.y, ARENA_X + x + shake.x, ARENA_Y + a.height + shake.y, 1.0, grid);
        x += 50.0;
    }
    let mut y = 0.0;
    while y <= a.height {
        draw_line(ARENA_X + shake.x, ARENA_Y + y + shake.y, ARENA_X + a.width + shake.x, ARENA_Y + y + shake.y, 1.0, grid);
        y += 50.0;
    }
    draw_rectangle_lines(ARENA_X + shake.x, ARENA_Y + shake.y, a.width, a.height, 3.0, Color::new(0.4, 0.45, 0.42, 1.0));
    for p in &a.pillars {
        let s = to_screen(p.pos, shake);
        draw_circle(s.x, s.y + 4.0, p.radius, Color::new(0.0, 0.0, 0.0, 0.3));
        draw_circle(s.x, s.y, p.radius, Color::new(0.36, 0.37, 0.40, 1.0));
        draw_circle_lines(s.x, s.y, p.radius, 2.0, Color::new(0.5, 0.52, 0.55, 1.0));
    }

    // Areas: telegraphs, then active hitboxes.
    for ar in battle.areas() {
        let col = move_color(data, assets, ar.mv);
        let c = to_screen(ar.center, shake);
        match ar.shape {
            Shape::Circle { radius } => {
                if !ar.is_active() {
                    let prog = 1.0 - (ar.delay / ar.delay_total.max(1e-3)).clamp(0.0, 1.0);
                    draw_circle(c.x, c.y, radius, Color::new(col.r, col.g, col.b, 0.10));
                    draw_circle(c.x, c.y, radius * prog, Color::new(col.r, col.g, col.b, 0.22));
                    draw_circle_lines(c.x, c.y, radius, 2.0, Color::new(col.r, col.g, col.b, 0.8));
                } else {
                    draw_circle(c.x, c.y, radius, Color::new(col.r, col.g, col.b, 0.55));
                    draw_circle_lines(c.x, c.y, radius, 3.0, WHITE);
                }
            }
            Shape::Arc { range, dir, half_arc } => {
                let fade = (ar.active / ar.active_total.max(1e-3)).clamp(0.2, 1.0);
                draw_arc_sector(c, range, dir.angle(), half_arc, Color::new(col.r, col.g, col.b, 0.55 * fade));
            }
        }
    }

    for p in battle.projectiles() {
        let col = move_color(data, assets, p.mv);
        let s = to_screen(p.pos, shake);
        let tail = to_screen(p.pos - p.vel * 0.04, shake);
        draw_line(tail.x, tail.y, s.x, s.y, p.radius * 1.2, Color::new(col.r, col.g, col.b, 0.35));
        draw_circle(s.x, s.y, p.radius, col);
        draw_circle(s.x, s.y, p.radius * 0.45, Color::new(1.0, 1.0, 1.0, 0.85));
    }

    // Draw back-to-front so the lower creature overlaps the upper one.
    let mut order: Vec<usize> = (0..battle.fighter_count()).collect();
    let views: Vec<FighterView> = order.iter().map(|&i| battle.fighter(i)).collect();
    order.sort_by(|&a, &b| views[a].body.pos.y.total_cmp(&views[b].body.pos.y));
    for i in order {
        let flash = fx.flash.get(i).copied().unwrap_or(0.0) > 0.0;
        draw_fighter(data, gfx, &views[i], flash, i == player, battle.time(), shake);
    }

    for p in &fx.particles {
        let s = to_screen(p.pos, shake);
        let k = (p.life / p.max_life).clamp(0.0, 1.0);
        draw_circle(s.x, s.y, p.size * (0.4 + 0.6 * k), Color::new(p.color.r, p.color.g, p.color.b, p.color.a * k));
    }

    for f in &fx.floaters {
        let s = to_screen(f.pos, shake);
        let a = Fx::floater_alpha(f);
        text_centered(&f.text, s.x + 1.0, s.y + 1.0, f.size, Color::new(0.0, 0.0, 0.0, a * 0.7));
        text_centered(&f.text, s.x, s.y, f.size, Color::new(f.color.r, f.color.g, f.color.b, a));
    }

    draw_top_bar(battle, assets, player);
    draw_bottom_hud(battle, assets, player);
}

fn draw_arc_sector(c: macroquad::math::Vec2, r: f32, angle: f32, half: f32, col: Color) {
    let steps = 16;
    let start = angle - half;
    let step = 2.0 * half / steps as f32;
    for k in 0..steps {
        let a0 = start + step * k as f32;
        let a1 = a0 + step;
        draw_triangle(c, c + vec2(a0.cos(), a0.sin()) * r, c + vec2(a1.cos(), a1.sin()) * r, col);
    }
}

fn draw_fighter(data: &GameData, gfx: &Gfx, v: &FighterView, flash: bool, is_player: bool, time: f32, shake: macroquad::math::Vec2) {
    let assets = gfx.assets;
    let c = &data.creatures[v.fighter.creature];
    let s = to_screen(v.body.pos, shake);
    let r = v.body.radius;
    let base = rgb(assets.creature_color(&c.asset), 1.0);
    let alpha = if v.fighter.iframes > 0.0 { 0.45 } else { 1.0 };
    let ring = if is_player { Color::new(1.0, 1.0, 1.0, 0.85) } else { Color::new(1.0, 0.35, 0.35, 0.85) };

    // Ground: shadow plus a team-coloured ring that also shows the hurtbox.
    draw_ellipse(s.x, s.y + r * 0.55, r * 0.95, r * 0.4, 0.0, Color::new(0.0, 0.0, 0.0, 0.3));
    draw_circle_lines(s.x, s.y, r + 2.0, 2.0, Color::new(ring.r, ring.g, ring.b, 0.55));

    let sheet = gfx.sprites.sheet(&c.asset);
    if let Action::Dash { dir, .. } = v.fighter.action {
        for k in 1..4 {
            let t = s - vec2(dir.x, dir.y) * (k as f32 * 10.0);
            draw_circle(t.x, t.y, r * (1.0 - k as f32 * 0.15), Color::new(base.r, base.g, base.b, 0.16));
        }
    }

    if let Some(tex) = sheet {
        let layout = &gfx.sprites.layout;
        let speed = v.body.vel.length();
        let (anim, frame) = pick_frame(&v.fighter.action, speed, flash, time, layout);
        let size = 2.0 * r / layout.body_fraction;
        let tint = if v.fighter.alive() {
            Color::new(1.0, 1.0, 1.0, alpha)
        } else {
            Color::new(0.55, 0.55, 0.55, 0.8)
        };
        draw_texture_ex(
            tex,
            s.x - size * 0.5,
            s.y - size * 0.5 - size * 0.06,
            tint,
            DrawTextureParams {
                dest_size: Some(vec2(size, size)),
                source: Some(gfx.sprites.source_rect(anim, frame)),
                flip_x: v.fighter.facing.x < 0.0,
                ..Default::default()
            },
        );
    } else {
        let mut col = if v.fighter.alive() { base } else { Color::new(0.3, 0.3, 0.3, 0.6) };
        col.a *= alpha;
        if flash {
            col = Color::new(1.0, 1.0, 1.0, col.a);
        }
        draw_circle(s.x, s.y, r, col);
        draw_circle_lines(s.x, s.y, r + 2.0, 2.5, ring);
    }

    // Aim marker just outside the hurtbox.
    let f = vec2(v.fighter.facing.x, v.fighter.facing.y);
    let tip = s + f * (r + 12.0);
    let side = vec2(-f.y, f.x) * 5.0;
    let back = s + f * (r + 4.0);
    if v.fighter.alive() {
        draw_triangle(tip, back + side, back - side, ring);
    }

    match v.fighter.action {
        Action::Windup { slot, t, .. } => {
            let mv = &data.moves[v.fighter.slots[slot].mv];
            let mc = move_color(data, assets, v.fighter.slots[slot].mv);
            let prog = 1.0 - (t / mv.windup.max(1e-3)).clamp(0.0, 1.0);
            draw_circle_lines(s.x, s.y, r + 6.0 + 6.0 * (1.0 - prog), 3.0, mc);
            // Show where melee arcs / self bursts will land.
            match mv.shape {
                MoveShape::Melee { range, arc_deg, .. } => {
                    draw_arc_sector(s, range, v.fighter.facing.angle(), arc_deg.to_radians() * 0.5, Color::new(mc.r, mc.g, mc.b, 0.12));
                }
                MoveShape::Burst { radius, max_distance, .. } if max_distance <= 0.0 => {
                    draw_circle_lines(s.x, s.y, radius, 1.5, Color::new(mc.r, mc.g, mc.b, 0.5));
                }
                _ => {}
            }
        }
        Action::Hitstun { .. } => {
            text_centered("*", s.x, s.y - r - 10.0, 28.0, YELLOW);
        }
        _ => {}
    }

    // Overhead HP bar.
    let w = 54.0;
    let frac = (v.fighter.hp / v.fighter.max_hp).clamp(0.0, 1.0);
    let bx = s.x - w * 0.5;
    let by = s.y - r / gfx.sprites.layout.body_fraction - 4.0;
    draw_rectangle(bx, by, w, 6.0, Color::new(0.0, 0.0, 0.0, 0.6));
    draw_rectangle(bx, by, w * frac, 6.0, hp_color(frac));
}

fn hp_color(frac: f32) -> Color {
    if frac > 0.5 {
        Color::new(0.35, 0.85, 0.40, 1.0)
    } else if frac > 0.2 {
        Color::new(0.95, 0.78, 0.25, 1.0)
    } else {
        Color::new(0.92, 0.30, 0.25, 1.0)
    }
}

fn draw_top_bar(battle: &Battle, assets: &AssetDb, player: usize) {
    let data = &*battle.data;
    draw_rectangle(0.0, 0.0, SCREEN_W, 56.0, PANEL);
    let enemy = 1 - player;
    for (slot, idx) in [(0, player), (1, enemy)] {
        let v = battle.fighter(idx);
        let c = &data.creatures[v.fighter.creature];
        let x = if slot == 0 { 40.0 } else { 760.0 };
        let w = 480.0;
        let label = if slot == 0 { "YOU" } else { "FOE" };
        draw_text(format!("{label}  {}", assets.creature_name(&c.asset)), x, 20.0, 22.0, TEXT);
        let mut tx = x + 230.0;
        for &t in &c.types {
            let name = assets.type_name(&data.types.names[t]);
            let col = type_id_color(data, assets, t);
            let tw = measure_text(name, None, 16, 1.0).width + 12.0;
            draw_rectangle(tx, 6.0, tw, 18.0, col);
            draw_text(name, tx + 6.0, 20.0, 16.0, BLACK);
            tx += tw + 6.0;
        }
        let frac = (v.fighter.hp / v.fighter.max_hp).clamp(0.0, 1.0);
        draw_rectangle(x, 30.0, w, 16.0, Color::new(0.0, 0.0, 0.0, 0.5));
        draw_rectangle(x, 30.0, w * frac, 16.0, hp_color(frac));
        draw_rectangle_lines(x, 30.0, w, 16.0, 1.0, DIM);
        let hp = format!("{:.0} / {:.0}", v.fighter.hp.ceil(), v.fighter.max_hp);
        let tw = measure_text(&hp, None, 16, 1.0).width;
        draw_text(&hp, x + w - tw - 4.0, 43.0, 16.0, TEXT);
    }
    let left = (data.balance.max_battle_seconds - battle.time()).max(0.0);
    text_centered(&format!("{:.0}", left.ceil()), SCREEN_W * 0.5, 38.0, 36.0, if left < 10.0 { RED } else { TEXT });
}

fn draw_bottom_hud(battle: &Battle, assets: &AssetDb, player: usize) {
    let data = &*battle.data;
    draw_rectangle(0.0, HUD_Y - 6.0, SCREEN_W, SCREEN_H - HUD_Y + 6.0, PANEL);
    let v = battle.fighter(player);
    let active_slot = match v.fighter.action {
        Action::Windup { slot, .. } => Some(slot),
        _ => None,
    };
    for (i, s) in v.fighter.slots.iter().enumerate() {
        let mv = &data.moves[s.mv];
        let x = 40.0 + i as f32 * 186.0;
        let (y, w, h) = (HUD_Y + 4.0, 176.0, 78.0);
        let col = move_color(data, assets, s.mv);
        draw_rectangle(x, y, w, h, Color::new(0.08, 0.09, 0.11, 1.0));
        draw_rectangle(x, y, 6.0, h, col);
        draw_text(assets.move_name(&mv.asset), x + 14.0, y + 22.0, 22.0, TEXT);
        let kind = match mv.shape {
            MoveShape::Projectile { count, .. } if count > 1 => "spread",
            MoveShape::Projectile { .. } => "shot",
            MoveShape::Melee { .. } => "melee",
            MoveShape::Burst { max_distance, .. } if max_distance > 0.0 => "aimed",
            MoveShape::Burst { .. } => "burst",
        };
        draw_text(format!("{}  pow {:.0}  {kind}", assets.type_name(&data.types.names[mv.element]), mv.power), x + 14.0, y + 42.0, 15.0, DIM);
        draw_text(PLAYER_KEYS[i], x + 14.0, y + 66.0, 18.0, Color::new(col.r, col.g, col.b, 1.0));
        if !s.ready() {
            let frac = (s.remaining / mv.cooldown).clamp(0.0, 1.0);
            draw_rectangle(x, y + h * (1.0 - frac), w, h * frac, Color::new(0.0, 0.0, 0.0, 0.6));
            text_centered(&format!("{:.1}", s.remaining), x + w - 32.0, y + 66.0, 26.0, TEXT);
        } else {
            draw_rectangle_lines(x, y, w, h, 2.0, Color::new(col.r, col.g, col.b, 0.9));
        }
        if active_slot == Some(i) {
            draw_rectangle_lines(x - 2.0, y - 2.0, w + 4.0, h + 4.0, 3.0, WHITE);
        }
    }

    // Dash.
    let x = 40.0 + 4.0 * 186.0;
    let (y, w, h) = (HUD_Y + 4.0, 110.0, 78.0);
    draw_rectangle(x, y, w, h, Color::new(0.08, 0.09, 0.11, 1.0));
    draw_text("Dash", x + 12.0, y + 22.0, 22.0, TEXT);
    draw_text("SPACE / SHIFT", x + 12.0, y + 66.0, 14.0, DIM);
    if v.fighter.dash_cooldown > 0.0 {
        let frac = (v.fighter.dash_cooldown / data.balance.dash.cooldown).clamp(0.0, 1.0);
        draw_rectangle(x, y + h * (1.0 - frac), w, h * frac, Color::new(0.0, 0.0, 0.0, 0.6));
    } else {
        draw_rectangle_lines(x, y, w, h, 2.0, WHITE);
    }

    // Opponent cooldowns (compact) so their windows are readable.
    let e = battle.fighter(1 - player);
    draw_text("FOE MOVES", 920.0, HUD_Y + 18.0, 16.0, DIM);
    for (i, s) in e.fighter.slots.iter().enumerate() {
        let mv = &data.moves[s.mv];
        let col = move_color(data, assets, s.mv);
        let x = 920.0 + i as f32 * 80.0;
        let (y, w, h) = (HUD_Y + 26.0, 74.0, 52.0);
        draw_rectangle(x, y, w, h, Color::new(col.r, col.g, col.b, 0.35));
        let name = assets.move_name(&mv.asset);
        let short: String = name.chars().take(9).collect();
        draw_text(&short, x + 4.0, y + 18.0, 15.0, TEXT);
        if !s.ready() {
            let frac = (s.remaining / mv.cooldown).clamp(0.0, 1.0);
            draw_rectangle(x, y + h * (1.0 - frac), w, h * frac, Color::new(0.0, 0.0, 0.0, 0.6));
            draw_text(format!("{:.1}", s.remaining), x + 4.0, y + 44.0, 18.0, TEXT);
        } else {
            draw_rectangle_lines(x, y, w, h, 1.5, col);
        }
    }
}

pub fn draw_pause() {
    draw_rectangle(0.0, 0.0, SCREEN_W, SCREEN_H, Color::new(0.0, 0.0, 0.0, 0.55));
    text_centered("PAUSED", SCREEN_W * 0.5, 300.0, 80.0, TEXT);
    text_centered("ESC / P  resume      R  restart      ENTER  creature select      X  quit", SCREEN_W * 0.5, 370.0, 24.0, TEXT);
}

pub fn draw_result(battle: &Battle, assets: &AssetDb, player: usize) {
    let Some(outcome) = battle.outcome() else { return };
    let data = &*battle.data;
    draw_rectangle(0.0, 0.0, SCREEN_W, SCREEN_H, Color::new(0.0, 0.0, 0.0, 0.62));
    let me = battle.fighter(player);
    let foe = battle.fighter(1 - player);
    let (title, col) = match outcome.winner() {
        Some(t) if t == me.fighter.team => ("VICTORY", Color::new(0.45, 0.9, 0.5, 1.0)),
        Some(_) => ("DEFEAT", Color::new(0.95, 0.35, 0.3, 1.0)),
        None => ("DRAW", TEXT),
    };
    let sub = match outcome {
        Outcome::Victory { .. } => "knockout",
        Outcome::Timeout { .. } => "time up - decided on remaining HP",
    };
    text_centered(title, SCREEN_W * 0.5, 260.0, 96.0, col);
    text_centered(sub, SCREEN_W * 0.5, 300.0, 24.0, DIM);
    let line = |label: &str, v: &FighterView, y: f32| {
        let c = &data.creatures[v.fighter.creature];
        let s = &v.fighter.stats;
        let t = format!(
            "{label} {:<9}  HP {:>3.0}%   damage {:>4.0}   hits {:>2}   moves {:>2}   dodges {:>2}",
            assets.creature_name(&c.asset),
            100.0 * v.fighter.hp / v.fighter.max_hp,
            s.damage_dealt,
            s.hits_landed,
            s.moves_used,
            s.dodges
        );
        text_centered(&t, SCREEN_W * 0.5, y, 22.0, TEXT);
    };
    line("You", &me, 360.0);
    line("Foe", &foe, 392.0);
    text_centered(&format!("time {:.1}s", battle.time()), SCREEN_W * 0.5, 428.0, 20.0, DIM);
    text_centered("R  rematch      ENTER  creature select      ESC  quit", SCREEN_W * 0.5, 500.0, 24.0, TEXT);
}

pub struct SelectView<'a> {
    pub focus: usize,
    pub player: usize,
    pub enemy: usize,
    pub ai: &'a str,
}

pub fn draw_select(data: &GameData, gfx: &Gfx, v: &SelectView) {
    let assets = gfx.assets;
    clear_background(BG);
    text_centered("REAL-TIME ARENA", SCREEN_W * 0.5, 70.0, 56.0, TEXT);
    text_centered("UP/DOWN choose row   LEFT/RIGHT change   ENTER fight   ESC quit", SCREEN_W * 0.5, 104.0, 20.0, DIM);

    let rows = [
        ("You", assets.creature_name(&data.creatures[v.player].asset).to_string()),
        ("Opponent", assets.creature_name(&data.creatures[v.enemy].asset).to_string()),
        ("AI", v.ai.to_string()),
    ];
    for (i, (label, value)) in rows.iter().enumerate() {
        let y = 160.0 + i as f32 * 44.0;
        let focused = i == v.focus;
        if focused {
            draw_rectangle(380.0, y - 28.0, 520.0, 38.0, PANEL);
        }
        draw_text(label, 400.0, y, 26.0, if focused { TEXT } else { DIM });
        text_centered(&format!("<  {value}  >"), 720.0, y, 28.0, if focused { YELLOW } else { TEXT });
    }

    draw_creature_card(data, gfx, v.player, v.enemy, 60.0, 310.0);
    draw_creature_card(data, gfx, v.enemy, v.player, 660.0, 310.0);
    text_centered(
        "WASD move   mouse aim   LMB/RMB/Q/E (or 1-4) moves   SPACE/SHIFT dash (brief invulnerability)",
        SCREEN_W * 0.5,
        SCREEN_H - 24.0,
        20.0,
        DIM,
    );
}

fn draw_creature_card(data: &GameData, gfx: &Gfx, id: usize, vs: usize, x: f32, y: f32) {
    let assets = gfx.assets;
    let c = &data.creatures[id];
    let other = &data.creatures[vs];
    let (w, h) = (560.0, 410.0);
    draw_rectangle(x, y, w, h, PANEL);
    let col = rgb(assets.creature_color(&c.asset), 1.0);
    if let Some(tex) = gfx.sprites.sheet(&c.asset) {
        let frame = ((get_time() as f32 * gfx.sprites.layout.fps * 0.6) as u32) % gfx.sprites.layout.columns.max(1);
        draw_texture_ex(
            tex,
            x + 12.0,
            y + 8.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(96.0, 96.0)),
                source: Some(gfx.sprites.source_rect(crate::sprites::Anim::Idle, frame)),
                ..Default::default()
            },
        );
    } else {
        draw_circle(x + 60.0, y + 60.0, c.radius * 1.5, col);
        draw_circle_lines(x + 60.0, y + 60.0, c.radius * 1.5, 2.0, WHITE);
    }
    draw_text(assets.creature_name(&c.asset), x + 120.0, y + 52.0, 34.0, TEXT);
    let mut tx = x + 120.0;
    for &t in &c.types {
        let name = assets.type_name(&data.types.names[t]);
        let tw = measure_text(name, None, 18, 1.0).width + 14.0;
        draw_rectangle(tx, y + 64.0, tw, 22.0, type_id_color(data, assets, t));
        draw_text(name, tx + 7.0, y + 81.0, 18.0, BLACK);
        tx += tw + 8.0;
    }
    let stats = [("HP", c.hp, 200.0), ("ATK", c.attack, 150.0), ("DEF", c.defense, 150.0), ("SPD", c.speed, 300.0)];
    for (i, (n, val, max)) in stats.iter().enumerate() {
        let sy = y + 118.0 + i as f32 * 26.0;
        draw_text(n, x + 24.0, sy + 14.0, 18.0, DIM);
        draw_rectangle(x + 80.0, sy, 300.0, 16.0, Color::new(0.0, 0.0, 0.0, 0.4));
        draw_rectangle(x + 80.0, sy, 300.0 * (val / max).min(1.0), 16.0, shade(col, 0.9));
        draw_text(format!("{val:.0}"), x + 392.0, sy + 14.0, 18.0, TEXT);
    }
    draw_text(
        format!("moves  (x = type multiplier vs {})", assets.creature_name(&other.asset)),
        x + 24.0,
        y + 244.0,
        16.0,
        DIM,
    );
    for (i, &m) in c.moves.iter().enumerate() {
        let mv = &data.moves[m];
        let my = y + 272.0 + i as f32 * 32.0;
        let mc = move_color(data, assets, m);
        draw_rectangle(x + 24.0, my - 16.0, 6.0, 22.0, mc);
        draw_text(format!("{}  {}", PLAYER_KEYS[i], assets.move_name(&mv.asset)), x + 38.0, my, 20.0, TEXT);
        let mut mult = data.types.multiplier(mv.element, &other.types);
        if c.types.contains(&mv.element) {
            mult *= data.balance.stab;
        }
        let mcol = if mult > 1.01 {
            Color::new(1.0, 0.65, 0.3, 1.0)
        } else if mult < 0.99 {
            DIM
        } else {
            TEXT
        };
        draw_text(format!("pow {:>3.0}  cd {:>3.1}s", mv.power, mv.cooldown), x + 280.0, my, 18.0, DIM);
        draw_text(format!("x{mult:.2}"), x + 470.0, my, 20.0, mcol);
    }
}
