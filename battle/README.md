# Real-time Arena (prototype)

Top-down 2D creature battles in real time: no turns, no menus. You steer one
creature, aim with the mouse, dodge with a dash (brief invulnerability) and
fire four moves on individual cooldowns. The type chart multiplies damage. It
never decides a fight on its own.

## Run

```sh
cargo run --release                     # play: creature select -> battle
cargo run --release -- --player pebblor --enemy galeon --ai hard
```

Controls: **WASD** move · **mouse** aim · **LMB / RMB / Q / E** (or **1-4**) moves ·
**Space / Shift** dash · **Esc / P** pause (R restart, Enter select, X quit).
On the result screen: **R** rematch, **Enter** select, **Esc** quit.

What to look for when playtesting: see [PLAYTEST.md](PLAYTEST.md).

Sound: `cargo run --release --features audio` (Linux needs `libasound2-dev`). Every
sound slot in `assets/assets.ron` is empty for now, so it's silent until sounds are added.

## Verify from the terminal

```sh
scripts/verify.sh                       # build + clippy -D warnings + tests + headless + screenshot
cargo run --release -- --headless 50    # 50 AI-vs-AI battles per ordered matchup (36 pairs)
cargo run --release -- --headless 200 --matchup sproutle emberkit --ai hard --ai-b normal
cargo run --release -- --headless 50 --moves   # + per-move use/hit-rate/damage table
scripts/screenshot.sh 600 out/frame.png --player voltfin --enemy tidepup --seed 5
scripts/screenshot.sh 2 out/select.png --scene select
scripts/screenshot.sh 2 out/result.png --scene result
```

`--headless` prints crashes (each battle is isolated with `catch_unwind`), KO/timeout counts,
fight length, side bias, per-creature win rate / DPS / dodges, and a matchup matrix.
Exit code is non-zero if any battle crashed.

`--screenshot FRAME PATH` runs AI-vs-AI deterministically (one sim tick per frame) and
saves frame `FRAME` as PNG. Without a display it needs `xvfb-run` (the script handles it).

## Layout

```
data/          balance: types.ron, moves.ron, creatures.ron, balance.ron, arena.ron
               feel.ron: hit-stop, shake, particles, input buffer (presentation only)
assets/        presentation only: assets.ron (names, colours, sprite sheet layout, sounds)
assets/sprites generated placeholder sprite sheets (cargo run --release --bin gen_sprites)
crates/combat  window-free core (reusable): data loading, sim, hitboxes, damage, AI
crates/game    macroquad front-end + --headless / --screenshot hooks (binary: arena)
scripts/       verify.sh, screenshot.sh
```

### Core design (`crates/combat`)

- `Battle::step(&[Intent])` advances one fixed tick (`tick_rate` in balance.ron).
  Players and AI both produce an `Intent` (move dir, aim point, dash, move slot), so
  the AI has no special powers and the sim never knows who is human.
- State is in a `hecs` world (fighters, projectiles, hit areas); `Battle::events()`
  reports hits, dodges, KOs for the renderer/audio.
- Moves are one of three data-driven shapes: `Projectile`, `Melee` (arc), `Burst`
  (circle, optionally aimed, with a telegraph delay). Fighters go
  Idle → Windup → (attack spawns) → Recovery; dash can cancel recovery.
- Nothing in the core is creature-game specific beyond naming; teams, N fighters,
  hitboxes, cooldowns, i-frames and AI carry over to the soulslike.
- Deterministic: same seed + same intents = same battle (tested).

### Art

Each creature has a 4x4 sprite sheet (48px frames; rows idle/walk/attack/hurt,
4 frames each; facing right; hurt frame 0 is the white hit flash). The current
sheets are original placeholders drawn by `crates/game/src/bin/gen_sprites.rs`.
To use real art, drop a sheet with the same layout into `assets/` and point the
creature's `sprite` at it (or change `sheet` to match the new layout). A missing
or malformed sheet logs a warning and falls back to a coloured circle.

### Data rules

All balance numbers live in `data/*.ron` and are validated on load (unknown ids,
missing moves, bad values fail with a message). Display names and colours live in
`assets/assets.ron`, referenced by each entry's `asset` id.
