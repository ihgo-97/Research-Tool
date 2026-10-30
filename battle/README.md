# Real-time Arena (prototype)

Top-down 2D creature battles in real time: no turns, no menus. You steer one
creature, aim with the mouse, dodge with a dash (brief invulnerability) and
fire four moves on individual cooldowns. The type chart multiplies damage. It
never decides a fight on its own.

## Run

```sh
cargo run --release                     # play: creature select -> battle
cargo run --release -- --player pebblor --enemy galeon --ai hard
cargo run --release -- --player emberkit,sproutle,voltfin --enemy tidepup,pebblor,galeon   # 3v3
```

Formats: **1v1**, or **3v3** team battles (choose on the select screen). In 3v3 one
creature per side is on the field. **Tab** switches to the next teammate and **Z/X/C**
switch to roster slots 1-3 (3s cooldown; the incoming creature gets brief
invulnerability). When a creature faints, the next one comes out automatically.

Status effects (defined in `balance.ron`, applied by moves): **burn** (damage over
time, weaker attacks), **paralysis** (slower movement and windups), **sleep** (can't act
until hit or it wears off).

Controls: **WASD** move · **mouse** aim · **LMB / RMB / Q / E** (or **1-4**) moves ·
**Space / Shift** dash · **Esc / P** pause (R restart, Enter select, X quit).
On the result screen: **R** rematch, **Enter** select, **Esc** quit.

What to look for when playtesting: see [PLAYTEST.md](PLAYTEST.md).

Sound: `cargo run --release --features audio` (Linux needs `libasound2-dev`). The
sounds are original retro-style effects synthesised by `cargo run --release --bin gen_sfx`
into `assets/sfx/`. Event sounds and per-move sounds are mapped in `assets/assets.ron`.
Without a sound device the game still runs (it logs an audio error).

## Verify from the terminal

```sh
scripts/verify.sh                       # build + clippy -D warnings + tests + headless + screenshot
cargo run --release -- --headless 50    # 50 AI-vs-AI battles per ordered matchup (36 pairs)
cargo run --release -- --headless 200 --matchup sproutle emberkit --ai hard --ai-b normal
cargo run --release -- --headless 50 --moves   # + per-move use/hit-rate/damage table
cargo run --release -- --headless 20 --team-size 3   # random 3v3 teams: team win% per creature, switches
scripts/screenshot.sh 600 out/frame.png --player voltfin --enemy tidepup --seed 5
scripts/screenshot.sh 2 out/select.png --scene select
scripts/screenshot.sh 2 out/result.png --scene result
scripts/screenshot.sh 2 out/vfx.png --scene vfx       # gallery of every move's effect
```

`--headless` prints crashes (each battle is isolated with `catch_unwind`), KO/timeout counts,
fight length, side bias, per-creature win rate / DPS / dodges, and a matchup matrix.
Exit code is non-zero if any battle crashed.

`--screenshot FRAME PATH` runs AI-vs-AI deterministically (one sim tick per frame) and
saves frame `FRAME` as PNG. Without a display it needs `xvfb-run` (the script handles it).

## Layout

```
data/          balance: types.ron, moves.ron, creatures.ron, balance.ron, arenas.ron
               feel.ron: hit-stop, shake, particles, input buffer (presentation only)
assets/        presentation only: assets.ron (names, colours, sprite sheet layout, sounds)
assets/sprites, assets/arena   generated placeholder art (cargo run --release --bin gen_sprites)
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

Style target: GBA/DS-era handheld battles. Pixel sprites drawn at an integer
`pixel_scale`, a 3/4 top-down view, light bordered HUD panels, outlined text and
move-name callouts.

Each creature sheet uses 64px frames: 4 animation frames per row, with
idle/walk/attack/hurt rows each split into down/up/side facings (side faces
right and is mirrored for left). Frame 0 of the hurt rows is the white hit flash.
The layout lives in `sheet` in `assets/assets.ron`.

The current art is original placeholder art from `crates/game/src/bin/gen_sprites/`.
It renders 3D primitive models (rounded shapes) into pixel art with quantised,
hue-shifted shading ramps and coloured outlines, plus the arena ground tiles and
rocks. Re-run it with `cargo run --release --bin gen_sprites`. To use real art,
drop in a sheet with the same layout and point the creature's `sprite` at it.
A missing or malformed sheet logs a warning and falls back to a coloured circle.

### Data rules

All balance numbers live in `data/*.ron` and are validated on load (unknown ids,
missing moves, bad values fail with a message). Display names and colours live in
`assets/assets.ron`, referenced by each entry's `asset` id.
