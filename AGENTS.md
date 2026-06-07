# PlotForge Agent Rules

## Project Goal

Build PlotForge as a CLI-first Rust MVP before adding desktop UI, real model providers, media providers, or Steam integrations.

## Commands

- `cargo check --workspace`
- `cargo test --workspace`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo run -p plotforge-cli -- new demo --path examples/dynasty-embers --force`
- `cargo run -p plotforge-cli -- check examples/dynasty-embers`
- `cargo run -p plotforge-cli -- play examples/dynasty-embers --once`
- `cargo run -p plotforge-cli -- export static examples/dynasty-embers --out dist/dynasty-embers`
- `scripts/qa/full_local.sh`

## Rules

- Keep `plotforge-schema` as the only schema source of truth.
- Do not let CLI, UI, storage, or export duplicate rule/runtime logic.
- Do not add silent fallbacks. Any fallback must be visible in trace/debug output.
- Do not hardcode secrets or include provider credentials in project files or exports.
- Do not add Steam/Workshop code to the MVP core crates.
- Keep CLI/export smoke tests tempdir-based; do not mutate checked-in fixtures in CI.
- Computer Use smoke is local desktop QA only and must not become a GitHub Actions dependency.
