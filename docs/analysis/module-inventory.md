# Module Inventory

| Module | Responsibility | Current Tests | Complexity | S.U.P.E.R Score | Testing Gap |
|:--|:--|:--|:--|:--|:--|
| `plotforge-schema` | Serializable contracts | 2 roundtrip unit tests | Medium | S🟢 U🟢 P🟢 E🟢 R🟡 | Contract breadth and export manifest tests |
| `plotforge-rule` | Declarative rule engine | 2 unit tests | Medium | S🟢 U🟢 P🟢 E🟢 R🟢 | Flag conditions, errors, set resources, event dedupe |
| `plotforge-storycraft` | Narrative review and story craft fixture | 3 unit tests | Medium | S🟡 U🟢 P🟡 E🟢 R🟡 | Table-driven issue kinds and score penalties |
| `plotforge-agent` | Mock scene proposal port | 2 unit tests | Medium | S🟡 U🟡 P🟢 E🟢 R🟡 | Action/thread mapping and continue semantics |
| `plotforge-runtime` | Session, commit, trace | 2 unit tests | High | S🟡 U🟡 P🟡 E🟢 R🟡 | Continue no-advance, multi-turn, error paths, trace shape |
| `plotforge-storage` | Project files, demo generation, traces | 1 unit test | High | S🟡 U🟡 P🟡 E🟡 R🟡 | Negative filesystem paths, force behavior, fixture parity |
| `plotforge-export` | Static export package | 1 unit test | Medium | S🟢 U🟢 P🟢 E🟡 R🟡 | Manifest content, assets, secret markers, HTTP smoke |
| `plotforge-cli` | Command-line adapter | 0 tests | Medium | S🟡 U🟢 P🟡 E🟢 R🟡 | Black-box CLI E2E and stdout/exit behavior |
| `examples/dynasty-embers` | Committed demo fixture | 0 tests | Medium | S🟢 U🟡 P🟡 E🟢 R🟡 | Drift from generated template and reference integrity |
| CI / QA scripts | Repeatable verification | Basic CI only | Medium | S🟢 U🟢 P🟡 E🟢 R🟡 | Split jobs, CLI/export smoke, Computer Use runbook |

## Module Details

### CLI and Fixture Testing
- **Responsibility**: Prove README/AGENTS smoke commands work without mutating checked-in fixture state.
- **Public API**: `plotforge-cli` binary.
- **Dependencies**: temp directories, cargo-built binary, storage/runtime/export crates.
- **Complexity Rating**: High priority, medium complexity.
- **S.U.P.E.R Assessment**:
  - **S**: Keep CLI smoke separate from library logic.
  - **U**: CLI calls core crates; tests must assert behavior at the adapter edge.
  - **P**: Output files and trace JSON remain schema-defined.
  - **E**: Tests use temp dirs.
  - **R**: Desktop UI can later reuse the same fixture/export contracts.

### Static Export and Computer Use
- **Responsibility**: Verify static player package is structurally valid and visually usable.
- **Public API**: `export static`, `game.json`, `index.html`.
- **Dependencies**: local HTTP server for browser smoke; Codex Desktop Computer Use for real window interaction.
- **Complexity Rating**: Medium.
- **S.U.P.E.R Assessment**:
  - **S**: CI smoke handles files/HTTP; Computer Use handles desktop visual interaction.
  - **U**: Export reads project data and writes package; no runtime/provider calls.
  - **P**: `ExportManifest` is the contract.
  - **E**: Localhost-only, no external accounts.
  - **R**: Browser automation or future Playwright can replace manual Computer Use later.

