//! Real-time creature arena prototype.
//!
//! ```text
//! arena                                  play (creature select screen)
//! arena --headless [N]                   N AI-vs-AI battles per ordered matchup, print stats
//! arena --screenshot FRAME PATH          render FRAME ticks of an AI-vs-AI battle, save PNG
//! options:
//!   --player ID[,ID..]  --enemy ID[,ID..] creatures (data/creatures.ron ids); a list is a team
//!   --team-size N                        creatures per side (default: length of --player, else 1)
//!   --matchup A B                        headless: only this pairing
//!   --moves                              headless: also print per-move stats
//!   --ai NAME                            AI profile from data/balance.ron (default normal)
//!   --ai-b NAME                          headless: right-side AI profile (skill-gap tests)
//!   --seed N                             RNG seed (default 1)
//!   --scene select|battle|result         screenshot: which screen to capture
//!   --data DIR  --assets DIR             data/asset folders (default ./data, ./assets)
//! ```

mod app;
mod assets;
mod audio;
mod feel;
mod fx;
mod headless;
mod render;
mod sprites;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use combat::GameData;

struct Args {
    headless: Option<usize>,
    screenshot: Option<(u64, String)>,
    scene: app::Scene,
    player: Option<String>,
    enemy: Option<String>,
    matchup: Option<(String, String)>,
    team_size: Option<usize>,
    moves: bool,
    ai: String,
    ai_b: Option<String>,
    seed: u64,
    data: Option<PathBuf>,
    assets: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        headless: None,
        screenshot: None,
        scene: app::Scene::Battle,
        player: None,
        enemy: None,
        matchup: None,
        team_size: None,
        moves: false,
        ai: "normal".into(),
        ai_b: None,
        seed: 1,
        data: None,
        assets: None,
    };
    let mut it = std::env::args().skip(1).peekable();
    let val = |it: &mut std::iter::Peekable<std::iter::Skip<std::env::Args>>, flag: &str| {
        it.next().ok_or_else(|| format!("{flag} needs a value"))
    };
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--headless" => {
                let n = match it.peek() {
                    Some(v) if !v.starts_with("--") => it.next().unwrap().parse().map_err(|_| "--headless N: not a number")?,
                    _ => 20,
                };
                a.headless = Some(n);
            }
            "--screenshot" => {
                let f = val(&mut it, "--screenshot")?.parse().map_err(|_| "--screenshot FRAME: not a number")?;
                let p = val(&mut it, "--screenshot")?;
                a.screenshot = Some((f, p));
            }
            "--scene" => {
                a.scene = match val(&mut it, "--scene")?.as_str() {
                    "select" => app::Scene::Select,
                    "battle" => app::Scene::Battle,
                    "result" => app::Scene::Result,
                    s => return Err(format!("unknown scene '{s}'")),
                }
            }
            "--player" => a.player = Some(val(&mut it, "--player")?),
            "--enemy" => a.enemy = Some(val(&mut it, "--enemy")?),
            "--matchup" => {
                let x = val(&mut it, "--matchup")?;
                let y = val(&mut it, "--matchup")?;
                a.matchup = Some((x, y));
            }
            "--moves" => a.moves = true,
            "--team-size" => {
                let n: usize = val(&mut it, "--team-size")?.parse().map_err(|_| "--team-size: not a number")?;
                if n == 0 {
                    return Err("--team-size must be at least 1".into());
                }
                a.team_size = Some(n);
            }
            "--ai" => a.ai = val(&mut it, "--ai")?,
            "--ai-b" => a.ai_b = Some(val(&mut it, "--ai-b")?),
            "--seed" => a.seed = val(&mut it, "--seed")?.parse().map_err(|_| "--seed: not a number")?,
            "--data" => a.data = Some(val(&mut it, "--data")?.into()),
            "--assets" => a.assets = Some(val(&mut it, "--assets")?.into()),
            "-h" | "--help" => {
                println!("{}", include_str!("main.rs").lines().take_while(|l| l.starts_with("//!")).map(|l| l.trim_start_matches("//!").trim_start_matches(' ')).collect::<Vec<_>>().join("\n"));
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument '{other}' (try --help)")),
        }
    }
    Ok(a)
}

/// `./name` if it exists, else the copy next to the workspace root.
fn find_dir(explicit: Option<PathBuf>, name: &str) -> PathBuf {
    if let Some(p) = explicit {
        return p;
    }
    let local = PathBuf::from(name);
    if local.is_dir() {
        return local;
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(name)
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let data_dir = find_dir(args.data.clone(), "data");
    let assets_dir = find_dir(args.assets.clone(), "assets");
    let data = match GameData::load_dir(&data_dir) {
        Ok(d) => Arc::new(d),
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let assets = match assets::AssetDb::load(&assets_dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("asset error: {e}");
            return ExitCode::FAILURE;
        }
    };
    for m in assets.missing(&data) {
        eprintln!("warning: no asset entry for {m}");
    }
    for p in std::iter::once(&args.ai).chain(args.ai_b.as_ref()) {
        if data.ai_profile(p).is_none() {
            eprintln!("error: unknown AI profile '{p}'");
            return ExitCode::from(2);
        }
    }
    let creature = |id: &str| data.creature_id(id).ok_or_else(|| format!("unknown creature '{id}'"));
    let resolve = |o: &Option<String>| -> Result<Vec<usize>, String> {
        match o.as_deref() {
            Some(list) => list.split(',').map(|s| creature(s.trim())).collect(),
            None => Ok(Vec::new()),
        }
    };
    let (player, enemy) = match (resolve(&args.player), resolve(&args.enemy)) {
        (Ok(p), Ok(e)) => (p, e),
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let team_size = args.team_size.unwrap_or(player.len().max(enemy.len()).max(1));
    let n = data.creatures.len();
    let player = app::fill_roster(player, team_size, n, 0);
    let enemy = app::fill_roster(if enemy.is_empty() { vec![1 % n] } else { enemy }, team_size, n, 3);

    if let Some(per_pair) = args.headless {
        let matchup = match &args.matchup {
            Some((x, y)) => match (creature(x), creature(y)) {
                (Ok(x), Ok(y)) => Some((x, y)),
                (Err(e), _) | (_, Err(e)) => {
                    eprintln!("error: {e}");
                    return ExitCode::from(2);
                }
            },
            None => None,
        };
        let ok = headless::run(
            data,
            &assets,
            &headless::HeadlessOpts {
                per_pair,
                matchup,
                team_size,
                ai: args.ai,
                ai_b: args.ai_b,
                seed: args.seed,
                moves: args.moves,
            },
        );
        return if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE };
    }

    let feel = match feel::Feel::load(&data_dir) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("data error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let cfg = app::AppConfig {
        team_size,
        assets_dir,
        feel,
        player,
        enemy,
        ai: args.ai,
        seed: args.seed,
        screenshot: args.screenshot.map(|(frame, path)| app::Screenshot {
            frame,
            path,
            scene: args.scene,
        }),
    };
    let conf = macroquad::window::Conf {
        window_title: "Real-time Arena".into(),
        window_width: render::SCREEN_W as i32,
        window_height: render::SCREEN_H as i32,
        window_resizable: false,
        high_dpi: false,
        ..Default::default()
    };
    macroquad::Window::from_config(conf, app::run(data, assets, cfg));
    ExitCode::SUCCESS
}
