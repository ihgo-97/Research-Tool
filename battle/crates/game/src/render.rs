//! All macroquad drawing. Reads simulation state; never mutates it.
//!
//! Style target: GBA/DS-era handheld battles: pixel sprites at an integer
//! scale, light bordered panels, outlined text and move-name callouts.

use crate::assets::{AssetDb, Rgb};
use crate::fx::{Fx, CALLOUT_LIFE};
use crate::sprites::{pick_dir, pick_frame, Anim, Dir, Sprites};
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

const BG: Color = Color::new(0.10, 0.11, 0.16, 1.0);
const INK: Color = Color::new(0.16, 0.16, 0.22, 1.0);
const PAPER: Color = Color::new(0.97, 0.97, 0.94, 1.0);
const PAPER_EDGE: Color = Color::new(0.78, 0.80, 0.84, 1.0);
const TEXT: Color = Color::new(0.96, 0.96, 0.98, 1.0);
const DIM: Color = Color::new(0.60, 0.63, 0.70, 1.0);
/// Height at which projectiles fly and numbers pop (3/4 view illusion).
const AIR: f32 = 16.0;

pub fn rgb(c: Rgb, a: f32) -> Color {
    Color::new(c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0, a)
}

fn with_alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

fn shade(c: Color, k: f32) -> Color {
    Color::new((c.r * k).min(1.0), (c.g * k).min(1.0), (c.b * k).min(1.0), c.a)
}

pub fn to_screen(p: combat::Vec2, shake: macroquad::math::Vec2) -> macroquad::math::Vec2 {
    vec2(ARENA_X + p.x + shake.x, ARENA_Y + p.y + shake.y)
}

pub fn text_width(t: &str, size: f32) -> f32 {
    measure_text(t, None, size as u16, 1.0).width
}

/// Text with a 2px outline, the handheld-RPG look.
pub fn text_outlined(t: &str, x: f32, y: f32, size: f32, fill: Color, outline: Color) {
    let o = if size >= 40.0 { 3.0 } else if size >= 28.0 { 2.0 } else { 1.0 };
    for (dx, dy) in [(-o, 0.0), (o, 0.0), (0.0, -o), (0.0, o), (-o, -o), (o, -o), (-o, o), (o, o)] {
        draw_text(t, (x + dx).round(), (y + dy).round(), size, outline);
    }
    draw_text(t, x.round(), y.round(), size, fill);
}

pub fn text_centered(t: &str, cx: f32, y: f32, size: f32, color: Color) {
    text_outlined(t, cx - text_width(t, size) * 0.5, y, size, color, with_alpha(INK, color.a));
}

/// Light bordered panel like the classic handheld battle boxes.
fn panel(x: f32, y: f32, w: f32, h: f32) {
    draw_rectangle(x - 2.0, y - 2.0, w + 4.0, h + 4.0, INK);
    draw_rectangle(x, y, w, h, PAPER);
    draw_rectangle(x, y + h - 4.0, w, 4.0, PAPER_EDGE);
}

fn type_id_color(data: &GameData, assets: &AssetDb, t: usize) -> Color {
    rgb(assets.type_color(&data.types.names[t]), 1.0)
}

fn move_color(data: &GameData, assets: &AssetDb, mv: usize) -> Color {
    type_id_color(data, assets, data.moves[mv].element)
}

fn type_badge(data: &GameData, assets: &AssetDb, t: usize, x: f32, y: f32, size: f32) -> f32 {
    let name = assets.type_name(&data.types.names[t]).to_uppercase();
    let col = type_id_color(data, assets, t);
    let w = text_width(&name, size) + 10.0;
    draw_rectangle(x - 1.0, y - 1.0, w + 2.0, size + 2.0, shade(col, 0.5));
    draw_rectangle(x, y, w, size, col);
    text_outlined(&name, x + 5.0, y + size * 0.78, size, TEXT, shade(col, 0.45));
    w
}

fn status_badge((badge, _, col): (&str, &str, Rgb), x: f32, y: f32, size: f32) -> f32 {
    let c = rgb(col, 1.0);
    let w = text_width(badge, size) + 8.0;
    draw_rectangle(x - 1.0, y - 1.0, w + 2.0, size + 2.0, INK);
    draw_rectangle(x, y, w, size, c);
    draw_text(badge, x + 4.0, y + size * 0.82, size, INK);
    w
}

fn hp_color(frac: f32) -> Color {
    if frac > 0.5 {
        Color::new(0.35, 0.86, 0.42, 1.0)
    } else if frac > 0.2 {
        Color::new(0.97, 0.80, 0.22, 1.0)
    } else {
        Color::new(0.94, 0.32, 0.26, 1.0)
    }
}

/// GBA-style HP bar: "HP" tag, dark frame, two-tone fill.
fn hp_bar(x: f32, y: f32, w: f32, frac: f32) {
    let tag = 26.0;
    draw_rectangle(x, y, w + tag, 12.0, INK);
    draw_rectangle(x + 2.0, y + 2.0, tag - 4.0, 8.0, Color::new(0.96, 0.66, 0.20, 1.0));
    draw_text("HP", x + 4.0, y + 10.0, 12.0, INK);
    let bx = x + tag;
    draw_rectangle(bx, y + 2.0, w - 2.0, 8.0, Color::new(0.30, 0.30, 0.34, 1.0));
    let c = hp_color(frac);
    let fw = (w - 2.0) * frac.clamp(0.0, 1.0);
    draw_rectangle(bx, y + 2.0, fw, 8.0, c);
    draw_rectangle(bx, y + 2.0, fw, 3.0, shade(c, 1.25));
}

/// Everything the renderer needs besides the battle itself.
pub struct Gfx<'a> {
    pub assets: &'a AssetDb,
    pub sprites: &'a Sprites,
}

enum Drawable {
    Fighter(usize),
    Rock(usize),
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

    let a = battle.arena();
    let art = assets.arena_art.get(&a.art);
    let tex = gfx.sprites.arena(&a.art);
    let (ox, oy) = (ARENA_X + shake.x, ARENA_Y + shake.y);

    // Border bank and ground.
    let border = art.map(|b| rgb(b.border, 1.0)).unwrap_or(Color::new(0.3, 0.35, 0.3, 1.0));
    draw_rectangle(ox - 10.0, oy - 10.0, a.width + 20.0, a.height + 20.0, shade(border, 0.6));
    draw_rectangle(ox - 6.0, oy - 6.0, a.width + 12.0, a.height + 12.0, border);
    if let Some(t) = tex {
        let tile = t.ground.width() * gfx.sprites.layout.pixel_scale;
        let mut y = 0.0;
        while y < a.height {
            let mut x = 0.0;
            while x < a.width {
                let w = tile.min(a.width - x);
                let h = tile.min(a.height - y);
                let sc = gfx.sprites.layout.pixel_scale;
                draw_texture_ex(
                    &t.ground,
                    ox + x,
                    oy + y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(w, h)),
                        source: Some(Rect::new(0.0, 0.0, w / sc, h / sc)),
                        ..Default::default()
                    },
                );
                x += tile;
            }
            y += tile;
        }
    } else {
        draw_rectangle(ox, oy, a.width, a.height, Color::new(0.24, 0.30, 0.25, 1.0));
    }
    // Inner shadow along the top edge sells the "sunken arena" look.
    draw_rectangle(ox, oy, a.width, 6.0, Color::new(0.0, 0.0, 0.1, 0.12));

    // Ground-level effects: telegraphs, bursts, melee arcs.
    for ar in battle.areas() {
        let col = move_color(data, assets, ar.mv);
        let c = to_screen(ar.center, shake);
        match ar.shape {
            Shape::Circle { radius } => {
                if !ar.is_active() {
                    let prog = 1.0 - (ar.delay / ar.delay_total.max(1e-3)).clamp(0.0, 1.0);
                    let pulse = 0.5 + 0.5 * (get_time() as f32 * 18.0).sin();
                    draw_circle(c.x, c.y, radius, with_alpha(col, 0.12));
                    draw_circle(c.x, c.y, radius * prog, with_alpha(col, 0.25));
                    draw_circle_lines(c.x, c.y, radius, 3.0, with_alpha(shade(col, 0.6), 0.6 + 0.4 * pulse));
                } else {
                    draw_circle(c.x, c.y, radius, with_alpha(col, 0.55));
                    draw_circle(c.x, c.y, radius * 0.6, with_alpha(shade(col, 1.4), 0.5));
                    draw_circle_lines(c.x, c.y, radius, 4.0, WHITE);
                }
            }
            Shape::Arc { range, dir, half_arc } => {
                let fade = (ar.active / ar.active_total.max(1e-3)).clamp(0.2, 1.0);
                draw_arc_sector(c, range, dir.angle(), half_arc, with_alpha(col, 0.5 * fade));
                draw_arc_sector(c, range * 0.6, dir.angle(), half_arc * 0.8, with_alpha(shade(col, 1.5), 0.45 * fade));
            }
        }
    }

    // Projectile ground shadows first, so sprites can stand on top of them.
    let projectiles = battle.projectiles();
    for p in &projectiles {
        let s = to_screen(p.pos, shake);
        draw_ellipse(s.x, s.y, p.radius, p.radius * 0.45, 0.0, Color::new(0.0, 0.0, 0.1, 0.22));
    }

    // Fighters and rocks, back to front.
    let views: Vec<FighterView> = (0..battle.fighter_count()).map(|i| battle.fighter(i)).collect();
    let mut items: Vec<(f32, Drawable)> = views
        .iter()
        .enumerate()
        .filter(|(_, v)| !v.fighter.benched)
        .map(|(i, v)| (v.body.pos.y, Drawable::Fighter(i)))
        .collect();
    items.extend(a.pillars.iter().enumerate().map(|(i, p)| (p.pos.y, Drawable::Rock(i))));
    items.sort_by(|x, y| x.0.total_cmp(&y.0));
    for (_, d) in items {
        match d {
            Drawable::Fighter(i) => {
                let flash = fx.flash.get(i).copied().unwrap_or(0.0) > 0.0;
                draw_fighter(data, gfx, &views[i], flash, views[i].fighter.team as usize == player, battle.time(), shake);
            }
            Drawable::Rock(i) => {
                let p = &a.pillars[i];
                let s = to_screen(p.pos, shake);
                draw_ellipse(s.x, s.y + p.radius * 0.15, p.radius * 1.05, p.radius * 0.5, 0.0, Color::new(0.0, 0.0, 0.1, 0.25));
                match tex {
                    Some(t) => {
                        let n = (t.obstacles.width() / 64.0).max(1.0) as usize;
                        let src = Rect::new((i % n) as f32 * 64.0, 0.0, 64.0, 64.0);
                        // Rock art is drawn for `obstacle_radius`; scale to this pillar.
                        let k = art.map(|b| p.radius / b.obstacle_radius).unwrap_or(1.0);
                        let sc = gfx.sprites.layout.pixel_scale * k;
                        draw_texture_ex(
                            &t.obstacles,
                            (s.x - 32.0 * sc).round(),
                            (s.y - 46.0 * sc).round(),
                            WHITE,
                            DrawTextureParams {
                                dest_size: Some(vec2(64.0 * sc, 64.0 * sc)),
                                source: Some(src),
                                ..Default::default()
                            },
                        );
                    }
                    None => draw_circle(s.x, s.y, p.radius, Color::new(0.4, 0.4, 0.44, 1.0)),
                }
            }
        }
    }

    // Projectiles fly at body height above their shadow.
    for p in &projectiles {
        let col = move_color(data, assets, p.mv);
        let s = to_screen(p.pos, shake) - vec2(0.0, AIR);
        let back = p.vel.normalize_or_zero();
        let tail = s - vec2(back.x, back.y) * p.radius * 3.0;
        draw_line(tail.x, tail.y, s.x, s.y, p.radius * 1.4, with_alpha(col, 0.45));
        draw_circle(s.x, s.y, p.radius + 2.0, shade(col, 0.55));
        draw_circle(s.x, s.y, p.radius, col);
        draw_circle(s.x - p.radius * 0.25, s.y - p.radius * 0.25, p.radius * 0.45, Color::new(1.0, 1.0, 1.0, 0.9));
    }

    for p in &fx.particles {
        let s = to_screen(p.pos, shake) - vec2(0.0, AIR * 0.6);
        let k = (p.life / p.max_life).clamp(0.0, 1.0);
        let sz = (p.size * (0.4 + 0.6 * k)).max(1.5).round();
        // Square particles keep the pixel-art look.
        draw_rectangle((s.x - sz * 0.5).round(), (s.y - sz * 0.5).round(), sz, sz, with_alpha(p.color, p.color.a * k));
    }

    for f in &fx.floaters {
        let s = to_screen(f.pos, shake) - vec2(0.0, AIR * 2.0);
        let al = Fx::floater_alpha(f);
        let w = text_width(&f.text, f.size);
        text_outlined(&f.text, s.x - w * 0.5, s.y, f.size, with_alpha(f.color, al), with_alpha(INK, al));
    }

    draw_callouts(battle, fx, player);
    draw_top_bar(battle, gfx, player);
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
    let alive = v.fighter.alive();
    let team = if is_player { Color::new(0.35, 0.75, 1.0, 1.0) } else { Color::new(1.0, 0.35, 0.35, 1.0) };

    // Ground: shadow and a team ring at the creature's feet (its hurtbox).
    draw_ellipse(s.x, s.y, r * 1.05, r * 0.55, 0.0, Color::new(0.0, 0.0, 0.1, 0.28));
    if alive {
        draw_ellipse_lines(s.x, s.y, r + 3.0, (r + 3.0) * 0.55, 0.0, 2.0, with_alpha(team, 0.8));
        // Aim marker on the ground.
        let f = vec2(v.fighter.facing.x, v.fighter.facing.y * 0.55);
        let tip = s + f * (r + 14.0);
        let side = vec2(-f.y, f.x).normalize_or_zero() * 5.0;
        let back = s + f * (r + 5.0);
        draw_triangle(tip, back + side, back - side, team);
    }

    match v.fighter.action {
        Action::Windup { slot, t, .. } => {
            let mv = &data.moves[v.fighter.slots[slot].mv];
            let mc = move_color(data, assets, v.fighter.slots[slot].mv);
            let prog = 1.0 - (t / mv.windup.max(1e-3)).clamp(0.0, 1.0);
            draw_ellipse_lines(s.x, s.y, r + 8.0 + 8.0 * (1.0 - prog), (r + 8.0 + 8.0 * (1.0 - prog)) * 0.55, 0.0, 3.0, mc);
            match mv.shape {
                MoveShape::Melee { range, arc_deg, .. } => {
                    draw_arc_sector(s, range, v.fighter.facing.angle(), arc_deg.to_radians() * 0.5, with_alpha(mc, 0.14));
                }
                MoveShape::Burst { radius, max_distance, .. } if max_distance <= 0.0 => {
                    draw_circle_lines(s.x, s.y, radius, 2.0, with_alpha(mc, 0.55));
                }
                _ => {}
            }
        }
        Action::Dash { dir, .. } => {
            for k in 1..4 {
                let t = s - vec2(dir.x, dir.y) * (k as f32 * 12.0) - vec2(0.0, r * 0.6);
                draw_circle(t.x, t.y, r * (0.9 - k as f32 * 0.15), with_alpha(base, 0.18));
            }
        }
        _ => {}
    }

    let top = if let Some(tex) = gfx.sprites.sheet(&c.asset) {
        let layout = &gfx.sprites.layout;
        let (anim, frame) = pick_frame(&v.fighter.action, v.body.vel.length(), flash, time, layout);
        let (dir, flip) = pick_dir(v.fighter.facing);
        let tint = if !alive {
            Color::new(0.6, 0.6, 0.66, 0.85)
        } else if v.fighter.iframes > 0.0 {
            Color::new(1.0, 1.0, 1.0, 0.5)
        } else {
            WHITE
        };
        gfx.sprites.draw_frame(tex, gfx.sprites.source_rect(anim, dir, frame), s, flip, tint);
        s.y - (layout.anchor.1 - 14.0) * layout.pixel_scale
    } else {
        let mut col = if alive { base } else { Color::new(0.3, 0.3, 0.3, 0.6) };
        if flash {
            col = WHITE;
        }
        draw_circle(s.x, s.y - r * 0.6, r, col);
        s.y - r * 2.0
    };

    if matches!(v.fighter.action, Action::Hitstun { .. }) {
        let t = time * 10.0;
        for k in 0..3 {
            let a = t + k as f32 * 2.1;
            let p = vec2(s.x + a.cos() * 14.0, top + 6.0 + a.sin() * 4.0);
            draw_rectangle(p.x - 2.0, p.y - 2.0, 4.0, 4.0, YELLOW);
        }
    }

    // Overhead HP bar.
    if alive {
        let w = 48.0;
        let frac = (v.fighter.hp / v.fighter.max_hp).clamp(0.0, 1.0);
        let (bx, by) = ((s.x - w * 0.5).round(), (top - 10.0).round());
        draw_rectangle(bx - 1.0, by - 1.0, w + 2.0, 7.0, INK);
        draw_rectangle(bx, by, w * frac, 5.0, hp_color(frac));
        draw_rectangle(bx, by, w * frac, 2.0, shade(hp_color(frac), 1.25));
        if let Some(st) = v.fighter.status {
            status_badge(assets.status(&data.statuses[st.status].id), bx + w + 4.0, by - 4.0, 12.0);
        }
    }
}

fn draw_callouts(battle: &Battle, fx: &Fx, player: usize) {
    for (i, c) in fx.callouts.iter().enumerate() {
        let Some(c) = c else { continue };
        let mine = battle.fighter(i).fighter.team as usize == player;
        let size = 24.0;
        let w = text_width(&c.text, size) + 28.0;
        let h = 34.0;
        // Slide in from the side, fade out at the end.
        let slide = (c.t / 0.12).min(1.0);
        let fade = ((CALLOUT_LIFE - c.t) / 0.25).clamp(0.0, 1.0);
        let y = ARENA_Y + 14.0;
        let x = if mine {
            ARENA_X + 12.0 - (1.0 - slide) * (w + 20.0)
        } else {
            ARENA_X + 1200.0 - 12.0 - w + (1.0 - slide) * (w + 20.0)
        };
        draw_rectangle(x - 3.0, y - 3.0, w + 6.0, h + 6.0, with_alpha(INK, fade));
        draw_rectangle(x, y, w, h, with_alpha(PAPER, fade));
        draw_rectangle(x, y, 8.0, h, with_alpha(c.color, fade));
        draw_rectangle(x + w - 8.0, y, 8.0, h, with_alpha(c.color, fade));
        text_outlined(&c.text, x + 14.0, y + 25.0, size, with_alpha(TEXT, fade), with_alpha(shade(c.color, 0.45), fade));
    }
}

fn draw_top_bar(battle: &Battle, gfx: &Gfx, player: usize) {
    let data = &*battle.data;
    let assets = gfx.assets;
    draw_rectangle(0.0, 0.0, SCREEN_W, 52.0, BG);
    for (side, team_side) in [(0, player), (1, 1 - player)] {
        let v = battle.fighter(battle.active(team_side));
        let c = &data.creatures[v.fighter.creature];
        let (pw, ph) = (520.0, 44.0);
        let px = if side == 0 { 40.0 } else { SCREEN_W - 40.0 - pw };
        let py = 4.0;
        panel(px, py, pw, ph);
        // Portrait.
        let (pbx, pby, pbs) = if side == 0 { (px + 4.0, py + 2.0, 40.0) } else { (px + pw - 44.0, py + 2.0, 40.0) };
        draw_rectangle(pbx, pby, pbs, pbs, Color::new(0.80, 0.86, 0.94, 1.0));
        draw_rectangle_lines(pbx, pby, pbs, pbs, 2.0, INK);
        if let Some(tex) = gfx.sprites.sheet(&c.asset) {
            let src = gfx.sprites.portrait_rect();
            let k = (pbs - 4.0) / src.w.max(src.h);
            draw_texture_ex(
                tex,
                pbx + 2.0,
                pby + 2.0 + (pbs - 4.0 - src.h * k) * 0.5,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(src.w * k, src.h * k)),
                    source: Some(src),
                    flip_x: side == 1,
                    ..Default::default()
                },
            );
        }
        let name = assets.creature_name(&c.asset).to_uppercase();
        let tx = if side == 0 { px + 52.0 } else { px + 12.0 };
        let tag = if side == 0 { "YOU" } else { "FOE" };
        text_outlined(&name, tx, py + 20.0, 22.0, TEXT, INK);
        let mut bx = tx + text_width(&name, 22.0) + 12.0;
        for &t in &c.types {
            bx += type_badge(data, assets, t, bx, py + 5.0, 14.0) + 4.0;
        }
        if let Some(st) = v.fighter.status {
            bx += status_badge(assets.status(&data.statuses[st.status].id), bx, py + 5.0, 14.0) + 4.0;
        }
        let tagc = if side == 0 { Color::new(0.35, 0.75, 1.0, 1.0) } else { Color::new(1.0, 0.4, 0.4, 1.0) };
        text_outlined(tag, bx + 6.0, py + 18.0, 14.0, tagc, INK);
        // Roster orbs: creature colour, grey when fainted, ring on the active one.
        let team = &battle.teams()[team_side];
        if team.members.len() > 1 {
            let ready = team.switch_cooldown <= 0.0;
            let ox0 = if side == 0 { px + pw - 14.0 - 22.0 * (team.members.len() - 1) as f32 } else { px + 64.0 + 360.0 };
            for (k, &m) in team.members.iter().enumerate() {
                let f = battle.fighter(m);
                let cx = ox0 + k as f32 * 22.0;
                let cy = py + 14.0;
                let col = if f.fighter.alive() {
                    rgb(assets.creature_color(&data.creatures[f.fighter.creature].asset), 1.0)
                } else {
                    Color::new(0.45, 0.45, 0.5, 1.0)
                };
                draw_circle(cx, cy, 8.0, INK);
                draw_circle(cx, cy, 6.5, col);
                draw_circle(cx - 2.0, cy - 2.0, 2.0, Color::new(1.0, 1.0, 1.0, 0.7));
                if !f.fighter.alive() {
                    draw_line(cx - 5.0, cy - 5.0, cx + 5.0, cy + 5.0, 2.0, INK);
                    draw_line(cx - 5.0, cy + 5.0, cx + 5.0, cy - 5.0, 2.0, INK);
                }
                if m == team.active {
                    draw_circle_lines(cx, cy, 10.0, 2.0, if ready || side == 1 { tagc } else { DIM });
                }
            }
        }
        let frac = (v.fighter.hp / v.fighter.max_hp).clamp(0.0, 1.0);
        hp_bar(tx, py + 27.0, 330.0, frac);
        let hp = format!("{:.0}/{:.0}", v.fighter.hp.ceil(), v.fighter.max_hp);
        draw_text(&hp, tx + 366.0, py + 38.0, 18.0, INK);
    }
    let left = (battle.time_limit() - battle.time()).max(0.0);
    let t = format!("{}:{:02}", (left.ceil() as u32) / 60, (left.ceil() as u32) % 60);
    let (bw, bx) = (76.0, SCREEN_W * 0.5 - 38.0);
    panel(bx, 10.0, bw, 30.0);
    let col = if left < 10.0 { Color::new(0.95, 0.3, 0.3, 1.0) } else { INK };
    let w = text_width(&t, 24.0);
    draw_text(&t, SCREEN_W * 0.5 - w * 0.5, 32.0, 24.0, col);
}

fn draw_bottom_hud(battle: &Battle, assets: &AssetDb, player: usize) {
    let data = &*battle.data;
    draw_rectangle(0.0, HUD_Y - 6.0, SCREEN_W, SCREEN_H - HUD_Y + 6.0, BG);
    let v = battle.fighter(battle.active(player));
    let active_slot = match v.fighter.action {
        Action::Windup { slot, .. } => Some(slot),
        _ => None,
    };
    for (i, s) in v.fighter.slots.iter().enumerate() {
        let mv = &data.moves[s.mv];
        let x = 40.0 + i as f32 * 186.0;
        let (y, w, h) = (HUD_Y + 4.0, 176.0, 78.0);
        let col = move_color(data, assets, s.mv);
        draw_rectangle(x - 2.0, y - 2.0, w + 4.0, h + 4.0, INK);
        draw_rectangle(x, y, w, h, shade(col, 0.85));
        draw_rectangle(x, y, w, h * 0.45, col);
        text_outlined(assets.move_name(&mv.asset), x + 10.0, y + 24.0, 22.0, TEXT, shade(col, 0.4));
        let kind = match mv.shape {
            MoveShape::Projectile { count, .. } if count > 1 => "spread",
            MoveShape::Projectile { .. } => "shot",
            MoveShape::Melee { .. } => "melee",
            MoveShape::Burst { max_distance, .. } if max_distance > 0.0 => "aimed",
            MoveShape::Burst { .. } => "burst",
        };
        text_outlined(
            &format!("{}  POW {:.0}  {kind}", assets.type_name(&data.types.names[mv.element]).to_uppercase(), mv.power),
            x + 10.0,
            y + 46.0,
            14.0,
            TEXT,
            shade(col, 0.4),
        );
        text_outlined(PLAYER_KEYS[i], x + 10.0, y + 68.0, 16.0, Color::new(1.0, 0.95, 0.6, 1.0), shade(col, 0.35));
        if !s.ready() {
            let frac = (s.remaining / mv.cooldown).clamp(0.0, 1.0);
            draw_rectangle(x, y + h * (1.0 - frac), w, h * frac, Color::new(0.05, 0.05, 0.1, 0.6));
            text_outlined(&format!("{:.1}", s.remaining), x + w - 46.0, y + 68.0, 24.0, TEXT, INK);
        }
        if active_slot == Some(i) {
            draw_rectangle_lines(x - 4.0, y - 4.0, w + 8.0, h + 8.0, 3.0, WHITE);
        }
    }

    // Dash.
    let x = 40.0 + 4.0 * 186.0;
    let (y, w, h) = (HUD_Y + 4.0, 110.0, 78.0);
    draw_rectangle(x - 2.0, y - 2.0, w + 4.0, h + 4.0, INK);
    draw_rectangle(x, y, w, h, PAPER);
    text_outlined("DASH", x + 10.0, y + 26.0, 22.0, TEXT, INK);
    draw_text("SPACE/SHIFT", x + 10.0, y + 44.0, 14.0, INK);
    let team = &battle.teams()[player];
    if team.members.len() > 1 {
        let col = if team.switch_cooldown > 0.0 { DIM } else { INK };
        draw_text("SWITCH", x + 10.0, y + 60.0, 14.0, col);
        draw_text("TAB / Z X C", x + 10.0, y + 73.0, 13.0, col);
    }
    if v.fighter.dash_cooldown > 0.0 {
        let frac = (v.fighter.dash_cooldown / data.balance.dash.cooldown).clamp(0.0, 1.0);
        draw_rectangle(x, y + h * (1.0 - frac), w, h * frac, Color::new(0.05, 0.05, 0.1, 0.55));
    }

    // Opponent cooldowns (compact) so their windows are readable.
    let e = battle.fighter(battle.active(1 - player));
    text_outlined("FOE MOVES", 920.0, HUD_Y + 16.0, 16.0, DIM, INK);
    for (i, s) in e.fighter.slots.iter().enumerate() {
        let mv = &data.moves[s.mv];
        let col = move_color(data, assets, s.mv);
        let x = 920.0 + i as f32 * 80.0;
        let (y, w, h) = (HUD_Y + 26.0, 74.0, 52.0);
        draw_rectangle(x - 1.0, y - 1.0, w + 2.0, h + 2.0, INK);
        draw_rectangle(x, y, w, h, shade(col, 0.8));
        let name = assets.move_name(&mv.asset);
        let short: String = name.chars().take(9).collect();
        text_outlined(&short, x + 4.0, y + 18.0, 14.0, TEXT, shade(col, 0.4));
        if !s.ready() {
            let frac = (s.remaining / mv.cooldown).clamp(0.0, 1.0);
            draw_rectangle(x, y + h * (1.0 - frac), w, h * frac, Color::new(0.05, 0.05, 0.1, 0.6));
            text_outlined(&format!("{:.1}", s.remaining), x + 4.0, y + 44.0, 18.0, TEXT, INK);
        }
    }
}

pub fn draw_pause() {
    draw_rectangle(0.0, 0.0, SCREEN_W, SCREEN_H, Color::new(0.0, 0.0, 0.05, 0.55));
    text_centered("PAUSED", SCREEN_W * 0.5, 300.0, 80.0, TEXT);
    text_centered("ESC / P  resume      R  restart      ENTER  creature select      X  quit", SCREEN_W * 0.5, 370.0, 24.0, TEXT);
}

pub fn draw_result(battle: &Battle, assets: &AssetDb, player: usize) {
    let Some(outcome) = battle.outcome() else { return };
    let data = &*battle.data;
    draw_rectangle(0.0, 0.0, SCREEN_W, SCREEN_H, Color::new(0.0, 0.0, 0.05, 0.6));
    let me = battle.fighter(battle.active(player));
    let foe = battle.fighter(battle.active(1 - player));
    let (title, col) = match outcome.winner() {
        Some(t) if t == me.fighter.team => ("VICTORY!", Color::new(1.0, 0.85, 0.3, 1.0)),
        Some(_) => ("DEFEAT", Color::new(0.95, 0.35, 0.3, 1.0)),
        None => ("DRAW", TEXT),
    };
    let sub = match outcome {
        Outcome::Victory { .. } => "knockout",
        Outcome::Timeout { .. } => "time up - decided on remaining HP",
    };
    text_centered(title, SCREEN_W * 0.5, 250.0, 96.0, col);
    text_centered(sub, SCREEN_W * 0.5, 292.0, 24.0, DIM);
    let team_mode = battle.teams()[player].members.len() > 1;
    panel(250.0, 330.0, 780.0, if team_mode { 136.0 } else { 110.0 });
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
        let w = text_width(&t, 22.0);
        draw_text(&t, SCREEN_W * 0.5 - w * 0.5, y, 22.0, INK);
    };
    line("You", &me, 364.0);
    line("Foe", &foe, 396.0);
    let teams = battle.teams();
    if teams[player].members.len() > 1 {
        let left = |side: usize| teams[side].members.iter().filter(|&&m| battle.fighter(m).fighter.alive()).count();
        let t = format!(
            "team left: you {}/{}   foe {}/{}   switches: you {}  foe {}",
            left(player),
            teams[player].members.len(),
            left(1 - player),
            teams[1 - player].members.len(),
            teams[player].switches,
            teams[1 - player].switches
        );
        draw_text(&t, SCREEN_W * 0.5 - text_width(&t, 18.0) * 0.5, 452.0, 18.0, INK);
    }
    let tt = format!("time {:.1}s", battle.time());
    draw_text(&tt, SCREEN_W * 0.5 - text_width(&tt, 18.0) * 0.5, 426.0, 18.0, Color::new(0.4, 0.42, 0.5, 1.0));
    text_centered("R  rematch      ENTER  creature select      ESC  quit", SCREEN_W * 0.5, 510.0, 24.0, TEXT);
}

/// One editable row of the select screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectRow {
    Format,
    Player(usize),
    Enemy(usize),
    Arena,
    Ai,
}

pub struct SelectView<'a> {
    pub rows: &'a [SelectRow],
    pub focus: usize,
    pub team_size: usize,
    pub arena: &'a str,
    pub player: &'a [usize],
    pub enemy: &'a [usize],
    pub ai: &'a str,
}

pub fn draw_select(data: &GameData, gfx: &Gfx, v: &SelectView) {
    let assets = gfx.assets;
    clear_background(BG);
    text_centered("REAL-TIME ARENA", SCREEN_W * 0.5, 56.0, 52.0, Color::new(1.0, 0.85, 0.3, 1.0));
    text_centered("UP/DOWN choose row   LEFT/RIGHT change   ENTER fight   ESC quit", SCREEN_W * 0.5, 86.0, 18.0, DIM);

    let name = |c: usize| assets.creature_name(&data.creatures[c].asset).to_string();
    let row_h = if v.rows.len() > 5 { 22.0 } else { 34.0 };
    let top = 120.0;
    for (i, row) in v.rows.iter().enumerate() {
        let (label, value) = match *row {
            SelectRow::Format => ("Format".to_string(), format!("{0}v{0}", v.team_size)),
            SelectRow::Player(k) if v.team_size == 1 => ("You".to_string(), name(v.player[k])),
            SelectRow::Player(k) => (format!("You #{}", k + 1), name(v.player[k])),
            SelectRow::Enemy(k) if v.team_size == 1 => ("Opponent".to_string(), name(v.enemy[k])),
            SelectRow::Enemy(k) => (format!("Foe #{}", k + 1), name(v.enemy[k])),
            SelectRow::Arena => ("Arena".to_string(), v.arena.to_string()),
            SelectRow::Ai => ("AI".to_string(), v.ai.to_string()),
        };
        let y = top + i as f32 * row_h;
        let focused = i == v.focus;
        let size = if row_h < 30.0 { 22.0 } else { 26.0 };
        if focused {
            panel(380.0, y - row_h * 0.72, 520.0, row_h - 4.0);
        }
        let ink = if focused { INK } else { DIM };
        draw_text(&label, 400.0, y, size, ink);
        let t = format!("<  {value}  >");
        let w = text_width(&t, size);
        if focused {
            draw_text(&t, 720.0 - w * 0.5, y, size, Color::new(0.85, 0.35, 0.15, 1.0));
        } else {
            text_outlined(&t, 720.0 - w * 0.5, y, size, TEXT, INK);
        }
    }

    // Cards show the slot being edited on each side (leads otherwise).
    let (pk, ek) = match v.rows.get(v.focus) {
        Some(SelectRow::Player(k)) => (*k, 0),
        Some(SelectRow::Enemy(k)) => (0, *k),
        _ => (0, 0),
    };
    let card_y = top + v.rows.len() as f32 * row_h + 4.0;
    draw_creature_card(data, gfx, v.player[pk], v.enemy[ek], 60.0, card_y);
    draw_creature_card(data, gfx, v.enemy[ek], v.player[pk], 660.0, card_y);
    let hint = if v.team_size > 1 {
        "WASD move  mouse aim  LMB/RMB/Q/E moves  SPACE dash  TAB or Z/X/C switch creature"
    } else {
        "WASD move   mouse aim   LMB/RMB/Q/E (or 1-4) moves   SPACE/SHIFT dash (brief invulnerability)"
    };
    text_centered(hint, SCREEN_W * 0.5, SCREEN_H - 14.0, 18.0, DIM);
}

fn draw_creature_card(data: &GameData, gfx: &Gfx, id: usize, vs: usize, x: f32, y: f32) {
    let assets = gfx.assets;
    let c = &data.creatures[id];
    let other = &data.creatures[vs];
    let (w, h) = (560.0, 400.0);
    panel(x, y, w, h);
    let col = rgb(assets.creature_color(&c.asset), 1.0);
    // Sprite on a little pedestal.
    draw_ellipse(x + 70.0, y + 96.0, 46.0, 14.0, 0.0, Color::new(0.75, 0.80, 0.88, 1.0));
    if let Some(tex) = gfx.sprites.sheet(&c.asset) {
        let frame = ((get_time() as f32 * gfx.sprites.layout.fps * 0.6) as u32) % gfx.sprites.layout.columns.max(1);
        gfx.sprites.draw_frame(tex, gfx.sprites.source_rect(Anim::Idle, Dir::Down, frame), vec2(x + 70.0, y + 96.0), false, WHITE);
    } else {
        draw_circle(x + 70.0, y + 70.0, c.radius * 1.5, col);
    }
    text_outlined(&assets.creature_name(&c.asset).to_uppercase(), x + 140.0, y + 52.0, 34.0, TEXT, INK);
    let mut tx = x + 140.0;
    for &t in &c.types {
        tx += type_badge(data, assets, t, tx, y + 64.0, 18.0) + 8.0;
    }
    let stats = [("HP", c.hp, 200.0), ("ATK", c.attack, 150.0), ("DEF", c.defense, 150.0), ("SPD", c.speed, 300.0)];
    for (i, (n, val, max)) in stats.iter().enumerate() {
        let sy = y + 128.0 + i as f32 * 26.0;
        draw_text(n, x + 24.0, sy + 14.0, 18.0, INK);
        draw_rectangle(x + 80.0, sy, 300.0, 16.0, INK);
        draw_rectangle(x + 82.0, sy + 2.0, 296.0 * (val / max).min(1.0), 12.0, col);
        draw_text(format!("{val:.0}"), x + 392.0, sy + 14.0, 18.0, INK);
    }
    draw_text(
        format!("moves  (x = type multiplier vs {})", assets.creature_name(&other.asset)),
        x + 24.0,
        y + 250.0,
        16.0,
        Color::new(0.4, 0.42, 0.5, 1.0),
    );
    for (i, &m) in c.moves.iter().enumerate() {
        let mv = &data.moves[m];
        let my = y + 278.0 + i as f32 * 32.0;
        let mc = move_color(data, assets, m);
        draw_rectangle(x + 24.0, my - 16.0, 6.0, 22.0, mc);
        draw_text(format!("{}  {}", PLAYER_KEYS[i], assets.move_name(&mv.asset)), x + 38.0, my, 20.0, INK);
        let mut mult = data.types.multiplier(mv.element, &other.types);
        if c.types.contains(&mv.element) {
            mult *= data.balance.stab;
        }
        let mcol = if mult > 1.01 {
            Color::new(0.9, 0.45, 0.1, 1.0)
        } else if mult < 0.99 {
            Color::new(0.5, 0.52, 0.6, 1.0)
        } else {
            INK
        };
        draw_text(format!("pow {:>3.0}  cd {:>3.1}s", mv.power, mv.cooldown), x + 280.0, my, 18.0, Color::new(0.4, 0.42, 0.5, 1.0));
        draw_text(format!("x{mult:.2}"), x + 470.0, my, 20.0, mcol);
    }
}
