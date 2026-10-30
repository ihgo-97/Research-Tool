#!/usr/bin/env bash
# Full check after every change: build, lint, tests, headless balance run,
# and (if a display or xvfb-run is available) a screenshot.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== build";   cargo build --release
echo "== clippy";  cargo clippy --all-targets -- -D warnings
echo "== audio feature compiles"; cargo check --release --features audio
echo "== test";    cargo test --release
echo "== headless"; ./target/release/arena --headless "${HEADLESS_N:-20}"

if [[ -n "${DISPLAY:-}" ]]; then
    ./target/release/arena --screenshot 300 out/verify.png
elif command -v xvfb-run >/dev/null; then
    xvfb-run -a -s "-screen 0 1280x768x24" ./target/release/arena --screenshot 300 out/verify.png
else
    echo "(no display / xvfb-run: screenshot skipped)"
fi
echo "== all checks passed"
