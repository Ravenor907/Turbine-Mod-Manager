#!/bin/sh
# Runs the engine's tests on its own (the engine needs no outside crates, so this works without a network).
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
cp -r "$root/engine" "$tmp/engine"
sed -i -e 's/^version.workspace = true/version = "0.0.0"/' -e 's/^edition.workspace = true/edition = "2024"/' \
  -e '/^license.workspace = true/d' -e '/^repository.workspace = true/d' "$tmp/engine/Cargo.toml"
cd "$tmp/engine"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/turbine-engine-target}" cargo test --offline --release -q "$@"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/turbine-engine-target}" cargo clippy --offline --all-targets -q -- -D warnings
rm -rf "$tmp"
