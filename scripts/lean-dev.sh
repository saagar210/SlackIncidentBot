#!/usr/bin/env bash
set -euo pipefail

LEAN_TMP_ROOT="${TMPDIR:-/tmp}/incident-bot-lean"
mkdir -p "$LEAN_TMP_ROOT"
LEAN_TARGET_DIR="$(mktemp -d "$LEAN_TMP_ROOT/target.XXXXXX")"

cleanup() {
  if [ -d "$LEAN_TARGET_DIR" ]; then
    echo "[lean-dev] removing temporary build artifacts: $LEAN_TARGET_DIR"
    rm -rf "$LEAN_TARGET_DIR"
  fi
}

trap cleanup EXIT INT TERM

export CARGO_TARGET_DIR="$LEAN_TARGET_DIR"
export CARGO_INCREMENTAL=0

echo "[lean-dev] using temporary CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
echo "[lean-dev] dependency caches in ~/.cargo are preserved for faster startup"

cargo run --release "$@"
