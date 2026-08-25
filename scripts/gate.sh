#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/openbim-idm-target}"
DIST="$(mktemp -d)"
trap 'rm -rf "$DIST"' EXIT
cd "$ROOT"

step() { printf '\n==> %s\n' "$*"; "$@"; }

step cargo fmt --all -- --check
step cargo check -p openbim-idm --no-default-features
step cargo build --workspace --all-features
step cargo test --workspace --all-features
step cargo clippy --workspace --all-targets --all-features -- -D warnings
step env RUSTDOCFLAGS=-Dwarnings cargo doc -p openbim-idm --lib --all-features --no-deps
step env RUSTDOCFLAGS=-Dwarnings cargo doc -p idmxml --lib --all-features --no-deps
step python3 scripts/check-alias-purity.py
step scripts/mutation-probe.sh
step python3 scripts/check-leakage.py

step cargo package -p openbim-idm --allow-dirty --no-verify
step cargo package -p idmxml --allow-dirty --no-verify
step python3 scripts/check-leakage.py \
  "$CARGO_TARGET_DIR/package/openbim-idm-0.1.0.crate" \
  "$CARGO_TARGET_DIR/package/idmxml-0.1.0.crate"

if command -v uv >/dev/null 2>&1; then
  step uv sync --extra test
  step uv run ruff check python tests scripts
  step uv run ruff format --check python tests scripts
  step uv run maturin develop
  step uv run pytest
  step uv run maturin build --out "$DIST/python"
  mapfile -t PYTHON_ARTIFACTS < <(find "$DIST/python" -maxdepth 1 -type f -print)
  ((${#PYTHON_ARTIFACTS[@]} > 0)) || { echo "no Python artifacts built" >&2; exit 1; }
  step python3 scripts/check-leakage.py "${PYTHON_ARTIFACTS[@]}"
else
  echo "gate: uv unavailable; Python build/tests NOT RUN" >&2
  exit 1
fi

if command -v npm >/dev/null 2>&1; then
  step npm ci
  step npm run docs:build
  step python3 scripts/check-leakage.py docs/.vitepress/dist
else
  echo "gate: npm unavailable; docs build NOT RUN" >&2
  exit 1
fi

step git diff --check
echo
echo "gate: PASS"
