#!/usr/bin/env bash
# Build the wasm32 binding, generate Node glue with wasm-bindgen, and run a
# round-trip smoke test. Requires wasm-bindgen-cli matching the wasm-bindgen
# crate version in Cargo.lock, and node.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/openbim-idm-target}"
OUT="$(mktemp -d)"
trap 'rm -rf "$OUT"' EXIT

cargo build -p openbim-idm --no-default-features --features wasm \
  --target wasm32-unknown-unknown --release
wasm-bindgen --target nodejs --out-dir "$OUT" \
  "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/openbim_idm.wasm"
node scripts/wasm-smoke.cjs "$OUT"
