# Third-party code

No source code has been copied into this repository. Everything below is a
crates.io dependency, pulled in by `Cargo.toml` and wrapped behind our own
modules so it can be swapped:

| Crate | Version | License | Source | Used for | Wrapped by |
|---|---|---|---|---|---|
| macroquad | 0.4.16 | MIT OR Apache-2.0 | https://github.com/not-fl3/macroquad | window, input, 2D drawing, PNG screenshot | `crates/game/src/{app,render}.rs` only. The `combat` crate never sees it |
| hecs | 0.11.2 | MIT OR Apache-2.0 | https://github.com/Ralith/hecs | ECS storage for fighters, projectiles, hit areas | `crates/combat/src/sim.rs` (public API is `Battle`) |
| serde | 1.0 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde | deserialising data/asset files | `data.rs`, `assets.rs` |
| ron | 0.12.3 | MIT OR Apache-2.0 | https://github.com/ron-rs/ron | RON file format for `data/` and `assets/` | `data.rs`, `assets.rs` |

macroquad is built with `default-features = false` (no audio backend yet).

Review notes (2026-10-30): the sources of macroquad, miniquad, quad-rand, fontdue,
hecs and ron were grepped for `std::process::Command`, `std::net`, `TcpStream`,
`UdpSocket`, `reqwest`, `hyper`: no matches. All four direct dependencies
build cleanly on stable Rust 1.97 and are actively maintained.

## Transitive dependencies

All are permissive. Most are MIT and/or Apache-2.0. These fall outside the
MIT/Apache/BSD allow-list and are **pending your OK**:

| Crate | License | Pulled in by | Notes |
|---|---|---|---|
| foldhash 0.1 / 0.2 | Zlib | hashbrown (via hecs/ron) | Zlib is permissive, similar to MIT |
| zlib-rs 0.6 | Zlib | flate2 (via image/png, used for PNG screenshots) | same |
| unicode-ident 1.0 | (MIT OR Apache-2.0) AND Unicode-3.0 | proc-macro2 (serde_derive), **build-time only** | Unicode data-file license, permissive |
| byteorder 1.5 | Unlicense OR MIT | image | MIT applies |
| bytemuck, miniz_oxide, fontdue, adler2 | include Zlib/0BSD as one option | macroquad / image | MIT option applies |

Regenerate the full list with:
`cargo metadata --format-version 1 | python3 -c "import json,sys;[print(p['name'],p['version'],p['license']) for p in json.load(sys.stdin)['packages']]"`

## Data references

The creature, move and type designs are original placeholders. Their names
and numbers are not taken from any game's data. The type chart follows the
familiar elemental pattern (water beats fire, and so on). If Pokémon Showdown
data (MIT) is used later as a reference for stats or moves, it will be
recorded here with the commit/URL it came from.
