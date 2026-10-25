#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
if command -v ffmpeg >/dev/null; then
    cargo test --locked ffmpeg_effects_render -- --ignored
fi
cargo clippy --no-default-features --all-targets -- -D warnings
cargo test --locked --no-default-features
bash -n install-desktop-app.sh uninstall-desktop-app.sh prepare-ai.sh
