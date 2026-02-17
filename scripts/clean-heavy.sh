#!/usr/bin/env bash
set -euo pipefail

rm -rf target/
rm -rf "${TMPDIR:-/tmp}/incident-bot-lean"

echo "Removed heavy build artifacts: target/ and ${TMPDIR:-/tmp}/incident-bot-lean"
