# Module Inventory

## Current Modules

| Module | Responsibility | Dependencies | Files | Lines | Complexity | S.U.P.E.R Score |
|:--|:--|:--|--:|--:|:--|:--|
| Product PRD | Defines product, MVP, architecture, roadmap | None | 1 | 1970 | High | N/A |
| Repository ignore rules | Excludes local/build/secrets files | Git | 1 | 12 | Low | N/A |
| Application code | No code exists yet | None | 0 | 0 | None | N/A |

## Target MVP Modules

| Module | Responsibility | Dependencies | Files | Lines | Complexity | S.U.P.E.R Score |
|:--|:--|:--|--:|--:|:--|:--|
| Workspace Scaffold | Cargo workspace, crate layout, CI, command contracts | Cargo | TBD | TBD | Medium | S🟢 U🟢 P🟡 E🟢 R🟢 |
| `plotforge-schema` | Data structures, serde contracts, JSON roundtrip | None | TBD | TBD | Medium | S🟢 U🟢 P🟢 E🟢 R🟢 |
| `plotforge-rule` | Declarative conditions/effects and deterministic risk | `schema` | TBD | TBD | Medium | S🟢 U🟢 P🟢 E🟢 R🟢 |
| `plotforge-storycraft` | Plot threads, emotional arcs, narrative review, mock plot doctor | `schema` | TBD | TBD | High | S🟡 U🟢 P🟡 E🟢 R🟡 |
| `plotforge-agent` | Provider ports, mock scene/beat pipeline, output validation | `schema`, `storycraft` | TBD | TBD | High | S🟡 U🟡 P🟢 E🟡 R🟢 |
| `plotforge-runtime` | Session loop, action interpretation, commit boundary, trace | `schema`, `rule`, `storycraft`, `agent` ports | TBD | TBD | High | S🟡 U🟢 P🟢 E🟢 R🟡 |
| `plotforge-storage` | Folder project read/write and trace persistence | `schema`, filesystem | TBD | TBD | High | S🟡 U🟢 P🟢 E🟡 R🟡 |
| `plotforge-export` | Static web export, manifest, asset packing, secret exclusion | `schema`, `storage` | TBD | TBD | Medium | S🟢 U🟢 P🟢 E🟡 R🟡 |
| `plotforge-cli` | `new demo`, `check`, `play`, `trace inspect`, `export static` | `storage`, `runtime`, `export` | TBD | TBD | Medium | S🟡 U🟢 P🟡 E🟢 R🟢 |
| `creator-desktop` | Future Tauri/React creator workspace | Tauri, React, core ports | TBD | TBD | High | S🟡 U🟢 P🟡 E🟡 R🟡 |
| `player-web` | Static exported player | export manifest | TBD | TBD | Medium | S🟢 U🟢 P🟢 E🟢 R🟡 |
| Demo Template | Built-in `dynasty-embers` project and fixtures | `schema`, `storage` | TBD | TBD | Medium | S🟢 U🟢 P🟢 E🟢 R🟢 |

## Module Details

### `plotforge-schema`
- **Path**: `crates/plotforge-schema/`
- **Responsibility**: Own the serializable contracts for projects, world state, story state, story craft state, scenes, beats, choices, characters, rules, assets, traces, and exports.
- **Public API**: Rust structs/enums deriving `Serialize`, `Deserialize`, `Clone`, `Debug`, plus constructors for deterministic demo data.
- **Internal Dependencies**: None.
- **External Dependencies**: `serde`, `serde_json`, `toml`, optionally `schemars`.
- **Complexity Rating**: Medium.
- **Transformation Notes**: This must be the single source of truth for cross-layer contracts.
- **S.U.P.E.R Assessment**:
  - **S**: Healthy if it remains contracts only.
  - **U**: Healthy as the innermost dependency.
  - **P**: Highest priority; all boundaries depend on this.
  - **E**: No environment assumptions.
  - **R**: Enables UI/provider/export replacement.

### `plotforge-rule`
- **Path**: `crates/plotforge-rule/`
- **Responsibility**: Evaluate declarative rule conditions and effects against `WorldState`.
- **Public API**: rule evaluator, condition matcher, world delta applier.
- **Internal Dependencies**: `plotforge-schema`.
- **External Dependencies**: Minimal; deterministic hashing/random utilities if needed.
- **Complexity Rating**: Medium.
- **Transformation Notes**: Never execute arbitrary user scripts in MVP.
- **S.U.P.E.R Assessment**:
  - **S**: Single responsibility is clear.
  - **U**: Depends inward on schema only.
  - **P**: Conditions/effects must be schema-defined.
  - **E**: Deterministic via seed/config.
  - **R**: Replaceable if runtime only calls evaluator API.

### `plotforge-storycraft`
- **Path**: `crates/plotforge-storycraft/`
- **Responsibility**: Represent story quality state and run mock narrative review.
- **Public API**: plot thread registry, emotional arc initialization, narrative review checks.
- **Internal Dependencies**: `plotforge-schema`.
- **External Dependencies**: None for MVP.
- **Complexity Rating**: High.
- **Transformation Notes**: Split data structures from review strategies to avoid a broad storycraft blob.
- **S.U.P.E.R Assessment**:
  - **S**: Needs strict file/module boundaries.
  - **U**: Should produce review output, not mutate runtime state.
  - **P**: Review issues must be serializable.
  - **E**: Mock checks are deterministic.
  - **R**: Future real PlotDoctor can replace mock if output contract holds.

### `plotforge-agent`
- **Path**: `crates/plotforge-agent/`
- **Responsibility**: Provide mock and future LLM generation behind provider ports.
- **Public API**: `ScenePlanner`, `BeatWriter`, `PlotDoctor`, provider traits, mock implementations.
- **Internal Dependencies**: `plotforge-schema`, `plotforge-storycraft`.
- **External Dependencies**: None for mock MVP; HTTP/model SDKs later.
- **Complexity Rating**: High.
- **Transformation Notes**: Agent output proposes content only; it does not commit world state.
- **S.U.P.E.R Assessment**:
  - **S**: Split provider ports, mocks, validation, and prompt concerns.
  - **U**: No reverse dependency from schema/runtime into concrete providers.
  - **P**: Provider I/O must use explicit schemas.
  - **E**: Provider config from environment/config only.
  - **R**: Adapters replaceable when traits stay stable.

### `plotforge-runtime`
- **Path**: `crates/plotforge-runtime/`
- **Responsibility**: Advance sessions through Scene/Beat choices and commit validated state deltas.
- **Public API**: session engine, play step, save/load snapshot, runtime trace.
- **Internal Dependencies**: `schema`, `rule`, `storycraft`, `agent` traits.
- **External Dependencies**: None required for MVP.
- **Complexity Rating**: High.
- **Transformation Notes**: This is the `LLM proposes, Engine commits` enforcement point.
- **S.U.P.E.R Assessment**:
  - **S**: Keep session, commit, trace, save modules separate.
  - **U**: Runtime orchestrates inward dependencies; outer adapters call runtime.
  - **P**: Inputs/outputs must be serializable action and trace structs.
  - **E**: Seed/config injected.
  - **R**: Replaceable providers if runtime consumes traits and proposed outputs.

### `plotforge-storage`
- **Path**: `crates/plotforge-storage/`
- **Responsibility**: Read/write folder projects and traces.
- **Public API**: load project, write template, validate structure, persist trace.
- **Internal Dependencies**: `plotforge-schema`.
- **External Dependencies**: filesystem, `toml`, `serde_json`.
- **Complexity Rating**: High.
- **Transformation Notes**: Folder project files are source of truth; SQLite is future cache only.
- **S.U.P.E.R Assessment**:
  - **S**: Separate project files, templates, traces, cache adapters.
  - **U**: Storage serializes schema data and should not own business rules.
  - **P**: File formats must map to schema contracts.
  - **E**: Paths passed as parameters.
  - **R**: SQLite or other caches can be swapped later.

### `plotforge-export`
- **Path**: `crates/plotforge-export/`
- **Responsibility**: Create static web export with prebaked content and assets.
- **Public API**: export static web to output directory or archive.
- **Internal Dependencies**: `schema`, `storage`.
- **External Dependencies**: filesystem; zip crate if archive output is needed.
- **Complexity Rating**: Medium.
- **Transformation Notes**: MVP export must not include API keys or dynamic AI calls.
- **S.U.P.E.R Assessment**:
  - **S**: Export only packages validated content.
  - **U**: Reads project/export data, writes package.
  - **P**: Export manifest contract needed.
  - **E**: Output path configurable.
  - **R**: Static player template replaceable.

### `plotforge-cli`
- **Path**: `crates/plotforge-cli/`
- **Responsibility**: Provide MVP command surface for project creation, validation, play, trace inspection, and export.
- **Public API**: command-line interface.
- **Internal Dependencies**: `storage`, `runtime`, `export`.
- **External Dependencies**: `clap`, terminal I/O.
- **Complexity Rating**: Medium.
- **Transformation Notes**: CLI calls core modules; it must not duplicate runtime/rule logic.
- **S.U.P.E.R Assessment**:
  - **S**: Split command handlers.
  - **U**: Outer adapter calling core.
  - **P**: CLI input maps to typed command parameters.
  - **E**: Paths/options explicit.
  - **R**: Desktop can replace CLI as adapter without touching core.
