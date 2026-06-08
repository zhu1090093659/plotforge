#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cd "$ROOT"

cargo fmt --all -- --check
python3 scripts/qa/no_launch_promise_lint.py
cargo check --workspace
cargo test --workspace
cargo test -p plotforge-cli --test cli_smoke cli_runs_workshop_local_flow_in_tempdir
scripts/contracts/check_contracts.sh
npm ci
npm run creator-desktop:qa
npm run player-web:qa
cargo clippy --workspace --all-targets -- -D warnings

PROJECT="$TMP/dynasty-embers"
EXPORT="$TMP/export"
DESKTOP_EXPORT="$TMP/desktop-export"
EXPORT_ZIP="$TMP/dynasty-embers-static.zip"
UNPACKED_EXPORT="$TMP/unpacked-export"

cargo run -p plotforge-cli -- new demo --path "$PROJECT" --force
cargo run -p plotforge-cli -- check "$PROJECT"
cargo run -p plotforge-cli -- export profiles
cargo run -p plotforge-cli -- play "$PROJECT" --once
cargo run -p plotforge-cli -- trace inspect "$PROJECT/traces/latest.json"
cargo run -p plotforge-cli -- export static "$PROJECT" --out "$EXPORT" --zip "$EXPORT_ZIP"
cargo run -p plotforge-cli -- export desktop "$PROJECT" --out "$DESKTOP_EXPORT"
python3 scripts/qa/static_export_http_smoke.py --export-dir "$EXPORT"
python3 -c 'import pathlib,sys,zipfile; out=pathlib.Path(sys.argv[2]); out.mkdir(parents=True, exist_ok=True); zipfile.ZipFile(sys.argv[1]).extractall(out)' "$EXPORT_ZIP" "$UNPACKED_EXPORT"
python3 scripts/qa/static_export_http_smoke.py --export-dir "$UNPACKED_EXPORT"

echo "PlotForge local QA passed."
