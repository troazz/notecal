#!/usr/bin/env bash
# Every check a change must pass (the same ones CI runs). Usage: scripts/check.sh
set -euo pipefail
cd "$(dirname "$0")/.."

run() { printf '\n== %s\n' "$*"; "$@"; }

[ -d ui/node_modules ] || run bun install --cwd ui --frozen-lockfile
# The Tauri crate embeds ui/dist at compile time, so build it before cargo.
run bun run --cwd ui build
run cargo fmt --all --check
run cargo clippy --workspace --all-targets -- -D warnings
run cargo test --workspace
run bun run --cwd ui check
run bun run --cwd ui test

printf '\nAll checks passed.\n'
