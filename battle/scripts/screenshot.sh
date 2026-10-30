#!/usr/bin/env bash
# Usage: scripts/screenshot.sh FRAME PATH [extra arena args...]
# Uses a virtual X server when no display is available.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release --quiet
if [[ -n "${DISPLAY:-}" ]]; then
    exec ./target/release/arena --screenshot "$@"
else
    exec xvfb-run -a -s "-screen 0 1280x768x24" ./target/release/arena --screenshot "$@"
fi
