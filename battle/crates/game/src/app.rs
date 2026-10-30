//! Windowed game: creature select -> battle -> result. Also drives the
//! deterministic `--screenshot` capture.

use crate::assets::AssetDb;
use crate::audio::Sfx;
use crate::feel::Feel;
use crate::fx::Fx;
use crate::render::{self, Gfx, SelectRow, SelectView, ARENA_X, ARENA_Y};
use crate::sprites::Sprites;
use combat::ai::AiController;
use combat::{Battle, Event, GameData, Intent};
use macroquad::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    Select,
    Battle,
    /// Battle fast-forwarded to its end, showing the result screen.
    Result,
}

pub struct Screenshot {
    pub frame: u64,
    pub path: String,
    pub scene: Scene,
}

pub struct AppConfig {
    /// Both rosters have this many creatures (1 = classic 1v1).
    pub team_size: usize,
    pub player: Vec<usize>,
    pub enemy: Vec<usize>,
    pub ai: String,
    pub seed: u64,
    pub screenshot: Option<Screenshot>,
    pub assets_dir: PathBuf,
    pub feel: Feel,
}

/// Team sizes offered on the select screen.
const FORMATS: [usize; 2] = [1, 3];

struct BattleScreen {
    battle: Battle,
    enemy_ai: AiController,
    /// Drives the player's side in screenshot mode.
    autoplay: Option<AiController>,
    acc: f32,
    fx: Fx,
    dash_buffer: f32,
    /// Roster slot requested by a switch key, consumed on the next tick.
    switch_request: Option<usize>,
    paused: bool,
}

enum Screen {
    Select,
    Battle(Box<BattleScreen>),
}

/// The player is always side 0.
const PLAYER: usize = 0;

/// Fill a roster up to `n` creatures, keeping picks and avoiding duplicates.
pub fn fill_roster(mut picks: Vec<usize>, n: usize, creatures: usize, offset: usize) -> Vec<usize> {
    picks.truncate(n);
    let mut k = offset;
    while picks.len() < n.min(creatures) {
        let c = k % creatures;
        if !picks.contains(&c) {
            picks.push(c);
        }
        k += 1;
    }
    picks
}

fn select_rows(cfg: &AppConfig) -> Vec<SelectRow> {
    let mut rows = vec![SelectRow::Format];
    rows.extend((0..cfg.team_size).map(SelectRow::Player));
    rows.extend((0..cfg.team_size).map(SelectRow::Enemy));
    rows.push(SelectRow::Ai);
    rows
}

pub async fn run(data: Arc<GameData>, assets: AssetDb, mut cfg: AppConfig) {
    let (sprites, warn_s) = Sprites::load(&cfg.assets_dir, &assets);
    let (sfx, warn_a) = Sfx::load(&cfg.assets_dir, &assets).await;
    for w in warn_s.iter().chain(&warn_a) {
        eprintln!("warning: {w}");
    }
    let gfx = Gfx {
        assets: &assets,
        sprites: &sprites,
    };
    let feel = cfg.feel.clone();
    let mut ai_names: Vec<String> = data.balance.ai_profiles.keys().cloned().collect();
    ai_names.sort_by_key(|n| match n.as_str() {
        "easy" => 0,
        "normal" => 1,
        "hard" => 2,
        _ => 3,
    });
    let mut ai_idx = ai_names.iter().position(|n| *n == cfg.ai).unwrap_or(0);
    let mut focus = 1usize;
    let mut seed = cfg.seed;
    let n = data.creatures.len();

    let shot = cfg.screenshot.take();
    let mut screen = match shot.as_ref().map(|s| s.scene) {
        Some(Scene::Battle) | Some(Scene::Result) => {
            let mut b = new_battle(&data, &cfg, &ai_names[ai_idx], seed, true);
            if shot.as_ref().map(|s| s.scene) == Some(Scene::Result) {
                fast_forward(&mut b, &assets, &feel);
            }
            Screen::Battle(Box::new(b))
        }
        _ => Screen::Select,
    };
    let mut frame: u64 = 0;

    loop {
        let dt = get_frame_time().min(0.1);
        match &mut screen {
            Screen::Select => {
                let rows = select_rows(&cfg);
                focus = focus.min(rows.len() - 1);
                if shot.is_none() {
                    if is_key_pressed(KeyCode::Escape) {
                        break;
                    }
                    if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                        focus = (focus + rows.len() - 1) % rows.len();
                    }
                    if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                        focus = (focus + 1) % rows.len();
                    }
                    let delta: i32 = if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) {
                        -1
                    } else if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) {
                        1
                    } else {
                        0
                    };
                    if delta != 0 {
                        // Cycle a roster slot, skipping creatures already on that team.
                        let cycle = |roster: &mut Vec<usize>, k: usize| {
                            let mut c = roster[k];
                            for _ in 0..n {
                                c = (c as i32 + delta).rem_euclid(n as i32) as usize;
                                if !roster.iter().enumerate().any(|(j, &o)| j != k && o == c) {
                                    break;
                                }
                            }
                            roster[k] = c;
                        };
                        match rows[focus] {
                            SelectRow::Format => {
                                let i = FORMATS.iter().position(|&f| f == cfg.team_size).unwrap_or(0) as i32;
                                cfg.team_size = FORMATS[(i + delta).rem_euclid(FORMATS.len() as i32) as usize];
                                cfg.player = fill_roster(cfg.player.clone(), cfg.team_size, n, 0);
                                cfg.enemy = fill_roster(cfg.enemy.clone(), cfg.team_size, n, 3);
                            }
                            SelectRow::Player(k) => cycle(&mut cfg.player, k),
                            SelectRow::Enemy(k) => cycle(&mut cfg.enemy, k),
                            SelectRow::Ai => ai_idx = (ai_idx as i32 + delta).rem_euclid(ai_names.len() as i32) as usize,
                        }
                    }
                    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
                        seed = seed.wrapping_add(1);
                        screen = Screen::Battle(Box::new(new_battle(&data, &cfg, &ai_names[ai_idx], seed, false)));
                        next_frame().await;
                        continue;
                    }
                }
                render::draw_select(
                    &data,
                    &gfx,
                    &SelectView {
                        rows: &rows,
                        focus,
                        team_size: cfg.team_size,
                        player: &cfg.player,
                        enemy: &cfg.enemy,
                        ai: &ai_names[ai_idx],
                    },
                );
            }
            Screen::Battle(bs) => {
                let done = bs.battle.outcome().is_some();
                if shot.is_none() {
                    let pause_key = is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::P);
                    if done && is_key_pressed(KeyCode::Escape) {
                        break;
                    }
                    if !done && pause_key {
                        bs.paused = !bs.paused;
                    }
                    if (done || bs.paused) && is_key_pressed(KeyCode::R) {
                        seed = seed.wrapping_add(1);
                        **bs = new_battle(&data, &cfg, &ai_names[ai_idx], seed, false);
                    } else if (done || bs.paused) && is_key_pressed(KeyCode::Enter) {
                        screen = Screen::Select;
                        next_frame().await;
                        continue;
                    } else if bs.paused && is_key_pressed(KeyCode::X) {
                        break;
                    }
                }
                // In screenshot mode time advances exactly one tick per frame so
                // captures are deterministic.
                let frame_dt = if shot.is_some() { data.dt() } else { dt };
                if !bs.paused {
                    if shot.is_none() {
                        poll_edge_input(bs, &feel);
                    }
                    if bs.fx.hitstop > 0.0 {
                        bs.fx.hitstop = (bs.fx.hitstop - frame_dt).max(0.0);
                    } else if shot.is_some() {
                        step_battle(bs, &assets, &feel);
                    } else {
                        bs.acc += dt;
                        while bs.acc >= data.dt() && bs.fx.hitstop <= 0.0 {
                            bs.acc -= data.dt();
                            step_battle(bs, &assets, &feel);
                        }
                        if bs.fx.hitstop > 0.0 {
                            bs.acc = 0.0;
                        }
                    }
                    bs.dash_buffer = (bs.dash_buffer - frame_dt).max(0.0);
                    bs.fx.update(frame_dt, &feel);
                }
                sfx.play_cues(&mut bs.fx.cues);
                render::draw_battle(&bs.battle, &gfx, &bs.fx, PLAYER);
                render::draw_result(&bs.battle, &assets, PLAYER);
                if bs.paused {
                    render::draw_pause();
                }
            }
        }

        if let Some(s) = &shot {
            if frame >= s.frame {
                save_screenshot(&s.path);
                break;
            }
        }
        frame += 1;
        next_frame().await;
    }
}

/// Edge-triggered keys are polled per frame (a frame may run zero ticks)
/// and buffered so an early press still registers.
fn poll_edge_input(bs: &mut BattleScreen, feel: &Feel) {
    if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::LeftShift) || is_key_pressed(KeyCode::RightShift) {
        bs.dash_buffer = feel.dash_buffer;
    }
    let team = &bs.battle.teams()[PLAYER];
    let direct = [KeyCode::Z, KeyCode::X, KeyCode::C];
    for (slot, key) in direct.iter().enumerate() {
        if slot < team.members.len() && is_key_pressed(*key) {
            bs.switch_request = Some(slot);
        }
    }
    if is_key_pressed(KeyCode::Tab) {
        // Next living teammate after the active one.
        let pos = team.members.iter().position(|&m| m == team.active).unwrap_or(0);
        bs.switch_request = (1..team.members.len())
            .map(|k| (pos + k) % team.members.len())
            .find(|&slot| bs.battle.fighter(team.members[slot]).fighter.alive());
    }
}

fn new_battle(data: &Arc<GameData>, cfg: &AppConfig, ai: &str, seed: u64, autoplay: bool) -> BattleScreen {
    let profile = data.ai_profile(ai).expect("ai profile").clone();
    let n = data.creatures.len();
    let rosters = [
        fill_roster(cfg.player.clone(), cfg.team_size, n, 0),
        fill_roster(cfg.enemy.clone(), cfg.team_size, n, 3),
    ];
    let battle = Battle::with_rosters(data.clone(), &rosters, seed);
    let fighters = battle.fighter_count();
    BattleScreen {
        battle,
        enemy_ai: AiController::new(profile.clone(), seed.wrapping_mul(7919).wrapping_add(1)),
        autoplay: autoplay.then(|| AiController::new(profile, seed.wrapping_mul(7919).wrapping_add(2))),
        acc: 0.0,
        fx: Fx::new(fighters, seed),
        dash_buffer: 0.0,
        switch_request: None,
        paused: false,
    }
}

fn fast_forward(bs: &mut BattleScreen, assets: &AssetDb, feel: &Feel) {
    while bs.battle.outcome().is_none() {
        step_battle(bs, assets, feel);
    }
    bs.fx = Fx::new(bs.battle.fighter_count(), 0);
}

fn player_intent(bs: &mut BattleScreen) -> Intent {
    if let Some(ai) = &mut bs.autoplay {
        return ai.update(&bs.battle, PLAYER);
    }
    let me = bs.battle.fighter(bs.battle.active(PLAYER));
    let mut dir = combat::Vec2::ZERO;
    if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
        dir.y -= 1.0;
    }
    if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
        dir.y += 1.0;
    }
    if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
        dir.x -= 1.0;
    }
    if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
        dir.x += 1.0;
    }
    let (mx, my) = mouse_position();
    let aim = combat::Vec2::new(mx - ARENA_X, my - ARENA_Y);

    let held = [
        is_mouse_button_down(MouseButton::Left) || is_key_down(KeyCode::Key1),
        is_mouse_button_down(MouseButton::Right) || is_key_down(KeyCode::Key2),
        is_key_down(KeyCode::Q) || is_key_down(KeyCode::Key3),
        is_key_down(KeyCode::E) || is_key_down(KeyCode::Key4),
    ];
    let use_move = (0..me.fighter.slots.len()).find(|&i| held.get(i).copied().unwrap_or(false) && me.fighter.slots[i].ready());

    Intent {
        move_dir: dir.normalize_or_zero(),
        aim,
        dash: bs.dash_buffer > 0.0,
        use_move,
        switch_to: bs.switch_request,
    }
}

fn step_battle(bs: &mut BattleScreen, assets: &AssetDb, feel: &Feel) {
    if bs.battle.outcome().is_some() {
        return;
    }
    let p = player_intent(bs);
    let e = bs.enemy_ai.update(&bs.battle, 1);
    bs.battle.step(&[p, e]);
    let active = bs.battle.active(PLAYER);
    for ev in bs.battle.events() {
        match *ev {
            Event::Dash { fighter } if fighter == active => bs.dash_buffer = 0.0,
            Event::SentOut { team, .. } if team as usize == PLAYER => bs.switch_request = None,
            _ => {}
        }
    }
    // A request the sim refused (cooldown, mid-attack) expires instead of firing later.
    if bs.battle.teams()[PLAYER].switch_cooldown > 0.0 {
        bs.switch_request = None;
    }
    bs.fx.on_events(&bs.battle, assets, feel, PLAYER);
    bs.fx.emit_trails(&bs.battle, assets);
    bs.fx.emit_status(&bs.battle, assets);
}

fn save_screenshot(path: &str) {
    let mut img = get_screen_data();
    // The framebuffer's alpha channel is meaningless here; force opaque.
    for px in img.bytes.chunks_mut(4) {
        px[3] = 255;
    }
    if let Some(dir) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    img.export_png(path);
    println!("screenshot saved: {path} ({}x{})", img.width, img.height);
}

#[cfg(test)]
mod tests {
    use super::fill_roster;

    #[test]
    fn rosters_fill_without_duplicates() {
        assert_eq!(fill_roster(vec![2], 3, 6, 0), vec![2, 0, 1]);
        assert_eq!(fill_roster(vec![0, 1, 2, 3], 3, 6, 0), vec![0, 1, 2]);
        assert_eq!(fill_roster(vec![], 3, 6, 3), vec![3, 4, 5]);
        assert_eq!(fill_roster(vec![5], 1, 6, 0), vec![5]);
    }
}
