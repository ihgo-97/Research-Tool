//! Windowed game: creature select -> battle -> result. Also drives the
//! deterministic `--screenshot` capture.

use crate::assets::AssetDb;
use crate::audio::Sfx;
use crate::feel::Feel;
use crate::fx::Fx;
use crate::render::{self, Gfx, SelectView, ARENA_X, ARENA_Y};
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
    pub player: usize,
    pub enemy: usize,
    pub ai: String,
    pub seed: u64,
    pub screenshot: Option<Screenshot>,
    pub assets_dir: PathBuf,
    pub feel: Feel,
}

struct BattleScreen {
    battle: Battle,
    enemy_ai: AiController,
    /// Drives the player's creature in screenshot mode.
    autoplay: Option<AiController>,
    acc: f32,
    fx: Fx,
    dash_buffer: f32,
    paused: bool,
}

enum Screen {
    Select,
    Battle(Box<BattleScreen>),
}

const PLAYER: usize = 0;

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
    let mut focus = 0usize;
    let mut seed = cfg.seed;

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
                if shot.is_none() {
                    if is_key_pressed(KeyCode::Escape) {
                        break;
                    }
                    if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                        focus = (focus + 2) % 3;
                    }
                    if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                        focus = (focus + 1) % 3;
                    }
                    let delta = if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) {
                        -1
                    } else if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) {
                        1
                    } else {
                        0
                    };
                    if delta != 0 {
                        let n = data.creatures.len() as i32;
                        match focus {
                            0 => cfg.player = (cfg.player as i32 + delta).rem_euclid(n) as usize,
                            1 => cfg.enemy = (cfg.enemy as i32 + delta).rem_euclid(n) as usize,
                            _ => ai_idx = (ai_idx as i32 + delta).rem_euclid(ai_names.len() as i32) as usize,
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
                        focus,
                        player: cfg.player,
                        enemy: cfg.enemy,
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
                    if shot.is_none()
                        && (is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::LeftShift) || is_key_pressed(KeyCode::RightShift))
                    {
                        // Edge-triggered input is polled per frame (a frame may run
                        // zero ticks) and buffered so an early press still dashes.
                        bs.dash_buffer = feel.dash_buffer;
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

fn new_battle(data: &Arc<GameData>, cfg: &AppConfig, ai: &str, seed: u64, autoplay: bool) -> BattleScreen {
    let profile = data.ai_profile(ai).expect("ai profile").clone();
    let battle = Battle::new(data.clone(), &[cfg.player, cfg.enemy], seed);
    let n = battle.fighter_count();
    BattleScreen {
        battle,
        enemy_ai: AiController::new(profile.clone(), seed.wrapping_mul(7919).wrapping_add(1)),
        autoplay: autoplay.then(|| AiController::new(profile, seed.wrapping_mul(7919).wrapping_add(2))),
        acc: 0.0,
        fx: Fx::new(n, seed),
        dash_buffer: 0.0,
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
    let me = bs.battle.fighter(PLAYER);
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
    }
}

fn step_battle(bs: &mut BattleScreen, assets: &AssetDb, feel: &Feel) {
    if bs.battle.outcome().is_some() {
        return;
    }
    let p = player_intent(bs);
    let e = bs.enemy_ai.update(&bs.battle, 1);
    bs.battle.step(&[p, e]);
    if bs.battle.events().iter().any(|ev| matches!(ev, Event::Dash { fighter } if *fighter == PLAYER)) {
        bs.dash_buffer = 0.0;
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
