#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cd "$ROOT"

cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
scripts/contracts/check_contracts.sh
npm ci
npm run creator-desktop:qa
npm run player-web:qa
cargo clippy --workspace --all-targets -- -D warnings

PROJECT="$TMP/dynasty-embers"
EXPORT="$TMP/export"

cargo run -p plotforge-cli -- new demo --path "$PROJECT" --force
cargo run -p plotforge-cli -- check "$PROJECT"
cargo run -p plotforge-cli -- play "$PROJECT" --once
cargo run -p plotforge-cli -- trace inspect "$PROJECT/traces/latest.json"
cargo run -p plotforge-cli -- export static "$PROJECT" --out "$EXPORT"
python3 scripts/qa/static_export_http_smoke.py --export-dir "$EXPORT"

echo "PlotForge local QA passed."
