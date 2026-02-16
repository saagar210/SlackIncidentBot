#!/usr/bin/env bash
set -euo pipefail

cargo clean
rm -rf target/
rm -rf "${TMPDIR:-/tmp}/incident-bot-lean"
rm -rf .sqlx

echo "Removed reproducible local artifacts: target/, .sqlx, and lean temp dirs"
