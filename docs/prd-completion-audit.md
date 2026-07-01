# PRD Completion Audit - Phase 6 / Full PRD

Audit date: 2026-06-09

Source requirements:

- `docs/plotforge_prd_final.md`
- `docs/plan/task-breakdown.md`
- `AGENTS.md`

This audit maps the PRD and task plan to repository evidence only. It does not assert any Steamworks account state, platform outcome, creator legal conclusion, real provider operation, hosted service, paid Workshop item flow, or external upload result. Phase 6 is implemented as local-first package, evidence, and adapter-boundary work.

## Audit Conclusion

Repository evidence supports the planned local-first PRD completion scope through Phase 6. The main implementation surfaces are the Rust workspace crates, generated contracts, Creator Desktop adapter, static player, example project, export/package QA scripts, Workshop package validator, Steam Submission Kit draft generator, and active Steam-facing guidance docs.

Remaining/deferred by scope:

- Real LLM, image, TTS, moderation, and vision provider calls are represented by ports, fake-first tests, and redaction boundaries; production provider integrations remain deferred.
- Steamworks SDK/API integration, platform account operations, paid Workshop item flows, cloud sharing, and hosted backend deployment remain deferred.
- The Submission Kit produces local draft evidence and review prompts only. It does not replace external forms, external review, account setup, fee handling, build upload, or creator decisions.
- `docs/plan/` and `docs/progress/` are local spec-driven tracking surfaces ignored by Git. GitHub issue/milestone state should be synchronized by the Phase PR owner during the Phase PR review/merge workflow.

## Phase And Task Evidence

| Task | Requirement | Repository evidence | Conservative status |
|---|---|---|---|
| T1.1 | Project creation service and Studio wizard | `plotforge-schema::ProjectCreationRequest`, `plotforge-storage::create_project_from_request`, `plotforge-studio::create_project`, `apps/creator-desktop/src/App.tsx`, `crates/plotforge-cli/tests/cli_smoke.rs::cli_creates_project_from_wizard_fields_and_reopens_it` | Covered by folder project creation and adapter tests. |
| T1.2 | Structured editing APIs for World, StoryCraft, Characters, State, Rules | `plotforge-storage::{read,update}_*_edit_document`, `plotforge-studio` command wrappers, generated `contracts/`, `crates/plotforge-storage/tests/project_files.rs::structured_edit_documents_roundtrip_and_persist_source_files` | Covered through Rust-backed storage/studio contracts. |
| T1.3 | Creator Desktop PRD IA and forms | `apps/creator-desktop/src/studioModel.ts`, `apps/creator-desktop/src/App.tsx`, `apps/creator-desktop/src/App.test.tsx` | Covered as a contract-backed Studio shell. |
| T1.4 | Static zip export in CLI, Studio, Export UI | `plotforge-export::{export_static_web,export_static_web_zip}`, `plotforge-studio::{export_static_project,export_static_project_zip}`, CLI `export static --zip`, `crates/plotforge-export/tests/static_export.rs`, `scripts/qa/static_export_http_smoke.py` | Covered for static local player packages and audited zips. |
| T1.5 | MVP workflow smoke alignment | `crates/plotforge-cli/tests/cli_smoke.rs::cli_runs_full_demo_flow_in_tempdir`, `scripts/qa/full_local.sh` | Covered by tempdir starter-project CLI flow; no committed default project fixture is expected. |
| T2.1 | Beat graph schema and runtime session semantics | `Beat`, `BeatNext`, `Scene.entry_beat_id` in `plotforge-schema`; `plotforge-runtime`; `crates/plotforge-runtime/tests/session_transitions.rs` | Covered for same-scene beat movement, scene changes, and explicit missing-beat errors. |
| T2.2 | Generic choice/action intent resolution | `ActionIntent` in `plotforge-schema`; `plotforge-runtime::interpret_action`; runtime tests for unsupported and ambiguous input | Covered with explicit unsupported input and no silent mutation. |
| T2.3 | Save and restore across CLI, Studio, player manifest | `RuntimeSnapshot`, `plotforge-storage::{write,read}_runtime_snapshot`, Studio play-with-save commands, static player local storage tests | Covered for local snapshots and explicit corrupted-save errors. |
| T2.4 | Static player graph navigation and responsive UI | `apps/player-web/static/player-core.js`, `apps/player-web/test/player.test.js`, `npm run player-web:qa`, HTTP smoke script | Covered for no-network DOM navigation and mobile viewport tests. |
| T2.5 | Runtime trace/debugger expansion | `RuntimeTrace` schema, `plotforge-runtime` trace fields, CLI `trace inspect`, `apps/creator-desktop/src/runtimeTraceView.tsx`, Creator Desktop tests | Covered for structured trace/debug evidence without raw provider bodies. |
| T3.1 | AI output envelope, repair/validate, reproducibility metadata | `AgentOutputEnvelope`, `ReproducibilityMetadata`, `GenerationEvidence` in schema; `plotforge-agent` tests; CLI trace assertions | Covered for fake-first generation envelopes and reproducibility fields. |
| T3.2 | Real provider port and BYO-key boundary | `TextModelProvider`, `TextProviderConfig`, credential resolver boundary, provider config hash tests, export secret exclusion tests | Covered as local port/config boundary; real external provider calls remain deferred. |
| T3.3 | World Bible AI expansion and canon/forbidden facts workflow | `WorldGenerationRequest/Report`, `plotforge-agent::generate_world_expansion`, storage/studio commands and tests | Covered with fake-first generation and structured project-file persistence. |
| T3.4 | StoryCraft generation workflows | `StoryCraftGenerationRequest/Report`, `plotforge-storycraft`, `plotforge-agent`, Studio generation command tests | Covered for generated StoryCraft plan evidence and narrative review surfaces. |
| T3.5 | Character generation and visual/voice card workflow | `Character`, `CharacterPortraitRequest`, `VisualBible`, `AudioBible`, `plotforge-agent::generate_character`, storage/studio tests | Covered for character creation and metadata persistence. |
| T3.6 | Live AI guardrails and reporting/moderation scaffolds | `AiSafetyPolicy`, `read/update_ai_safety_policy`, Creator Desktop AI safety UI/tests, `ai-usage.json` export fields | Covered as policy/evidence scaffolding; live external generation remains deferred. |
| T4.1 | Scene image persistence and reference reuse | `plotforge-agent` image provider pipeline, `plotforge-media` registry, scene background asset refs, export reachability tests | Covered with fake image provider, cached references, placeholder fallback metadata. |
| T4.2 | Visual Bible, asset browser, style controls | `VisualBible`, `VisualStyleCard`, `plotforge-storage` visual bible commands, `apps/creator-desktop` asset surfaces/tests | Covered for local visual bible and asset inspection. |
| T4.3 | TTS provider port, audio registry, visible silent fallback | `TtsRequest`, `TtsProvider`, `plotforge-agent` TTS tests, `plotforge-media`, `plotforge-job` TTS job kind | Covered as provider-port and fallback-evidence pipeline. |
| T4.4 | Player audio playback and lazy loading | `audio_refs` schema/storage/export, `apps/player-web/static/player-core.js`, `apps/player-web/test/player.test.js` | Covered for local audio refs, no autoplay, lazy loading, and missing-ref handling. |
| T5.1 | Export profile commands and UI | `ExportProfile`, `supported_export_profiles`, CLI `export profiles`, `plotforge-studio::list_export_profiles`, Creator Desktop export UI/tests | Covered for static, BYO-key, self-host, desktop, Workshop, and Submission Kit profile metadata. |
| T5.2 | Desktop runtime package draft | `plotforge-export::export_desktop_runtime_draft`, `DesktopRuntimeDraft`, CLI `export desktop`, export tests | Covered as local desktop draft package evidence, not an installer or external platform artifact. |
| T5.3 | AI usage manifest and package disclosure audit | `AiUsageManifest`, `plotforge-export::ai_usage_manifest`, `scripts/qa/static_export_http_smoke.py`, export tests | Covered for redaction-safe AI usage disclosure in static and desktop draft exports. |
| T5.4 | Export acceptance smoke matrix | `scripts/qa/full_local.sh`, `scripts/qa/static_export_http_smoke.py`, CLI smoke tests, `.github/workflows/ci.yml` jobs | Covered by repeatable local and CI gate definitions. |
| T6.1 | Local Workshop library import/load/remix/block/report/delete | `plotforge-workshop::{import,list,load,remix,block,report,delete}_workshop_library_*` and tests `imports_loads_remixes_and_deletes_local_library_items`, `block_and_report_metadata_remain_local_and_readable`, `blocked_items_reject_load_remix_and_duplicate_import_explicitly` | Covered for local/offline package library workflows. |
| T6.2 | Workshop publish draft and gated Steamworks upload port | `WorkshopPublishDraft`, `generate_workshop_publish_draft`, `write_workshop_publish_draft`, `SteamworksUploadPort`, `LocalOnlySteamworksUploadPort`, fake adapter tests | Covered as disabled-by-default, explicit-config, fake-first port. No real Steamworks call is included. |
| T6.3 | Steam Play/Creator/Developer mode shell | `apps/creator-desktop/src/App.tsx` product modes and Developer Mode boundary copy; `App.test.tsx::switches_product_modes_and_keeps_Steam_entries_local-first` | Covered as a local Studio shell without duplicating core logic. |
| T6.4 | Steam Submission Kit completion | `SteamSubmissionKitRequest/Draft`, `generate_steam_submission_kit`, `write_steam_submission_kit`, snapshot tests for checklist, AI disclosure, content warnings, asset refs, Direct checklist, safety checklist, packaging notes | Covered as deterministic local draft generation from validated package evidence. |
| T6.5 | Compliance docs and QA boundary hardening | `docs/steam-compliance-qa.md`, `docs/steam-submission-kit.md`, `README.md`, `scripts/qa/no_launch_promise_lint.py`, CI lint job | Covered for active-doc no-launch-promise boundary. |
| T6.6 | Final PRD completion audit | This file, plus repository evidence rows above | Covered as repo-scope audit artifact; remote issue/milestone closure is not asserted here. |

## PRD Requirement Evidence

| PRD area | Requirement summary | Evidence |
|---|---|---|
| Product scope and MVP core | CLI-first Rust engine, desktop creator adapter, folder project source of truth | `Cargo.toml` workspace members, `plotforge-cli`, `plotforge-storage`, `apps/creator-desktop`, README scope section |
| Project management | Create, reopen, and validate project folders | `FR-PROJ-001` evidence in `plotforge-storage`, `plotforge-studio`, CLI tests, Creator Desktop wizard tests |
| World editing | World Bible, canon, forbidden facts | `WorldEditDocument`, `world/world.md`, `world/canon.md`, `world/forbidden_facts.json`, storage/studio roundtrip tests |
| Story Craft | StoryCraft Bible, emotional arc, plot threads, narrative review | `StoryCraftState`, `plotforge-storycraft`, `story/story_craft.toml`, `story/emotional_arc.json`, `story/plot_threads.toml`, trace/debugger narrative review views |
| Characters | Character cards with arcs, visual/voice metadata | `Character`, `CharacterPortraitRequest`, `characters/*.character.toml`, storage/studio/agent tests |
| State variables | Resource and story state editing | `WorldState`, `StoryState`, `StateVariablesEditDocument`, `world/resources.toml`, `saves/initial_story_state.json` |
| Rule engine | Declarative rules only | `plotforge-rule`, `Rule`, `Condition`, `Effect`, `rules/rules.toml`, rule tests |
| Runtime | Scene/Beat progression, typed actions, rule commits, saves, trace | `plotforge-runtime`, `RuntimeSession`, `ActionIntent`, `RuntimeTrace`, `RuntimeSnapshot`, runtime tests |
| Agent pipeline | Fake-first multi-agent generation and review scaffolds | `plotforge-agent`, agent output envelopes, world/story/character generation reports, provider ports |
| Media pipeline | Image/audio asset registry, provider metadata, fallback evidence | `plotforge-media`, `plotforge-agent`, `AssetRecord`, `MediaAssetReference`, image/TTS tests, export reachability tests |
| Creator workbench | Dashboard, source editor, playtest, debugger, assets, export profiles | `apps/creator-desktop/src/App.tsx`, `studioModel.ts`, `tauriBridge.ts`, Tauri command bridge, Vitest coverage |
| Static player | No-network static player with graph navigation and local assets | `apps/player-web/static`, `apps/player-web/test/player.test.js`, `plotforge-export`, HTTP smoke script |
| Export strategy | Static Web, Desktop Runtime draft, BYO-key/self-host descriptors, Workshop and Submission Kit descriptors | `ExportProfile`, `supported_export_profiles`, CLI `export profiles`, export tests |
| Security and redaction | No secrets, provider config, raw responses, or private traces in contracts/exports/traces | `contains_secret_marker_text`, schema deny-unknown-field tests, export package audits, workshop validator checks |
| Steam/Workshop route | Workshop package metadata/library, local publish draft, mode shell, Submission Kit drafts, active-doc lint | `plotforge-workshop`, `docs/steam-compliance-qa.md`, `docs/steam-submission-kit.md`, `scripts/qa/no_launch_promise_lint.py`, Creator Desktop mode tests |
| Starter smoke project | Temporary starter projects generated by CLI align with schema and smoke flows | CLI check/play/export smoke, README commands |

## Non-Functional Evidence

| PRD NFR | Evidence |
|---|---|
| Maintainability and crate boundaries | Workspace crates align with `AGENTS.md` boundaries; Studio/Tauri adapters call Rust crates; frontend imports generated contracts rather than owning core rule/runtime logic. |
| Stability | Explicit error types in storage/runtime/export/workshop; tests cover unsupported input, corrupted saves, stale output files, unsafe paths, hash mismatches, blocked library items, and missing credentials/config. |
| Reproducibility | `run_seed`, `prompt_version`, `model_version`, `provider_config_hash`, trace ids, snapshot ids, content hashes, package hashes, and deterministic draft snapshots. |
| Performance and offline behavior | Static player is no-network; export copies reachable assets only; player audio uses `preload="none"`; SQLite cache is rebuildable and not canonical. |
| Security | Secret marker rejection, path safety checks, deny-unknown-field schemas, no provider credentials or private traces in exports/workshop packages, active Steam-facing doc lint. |

## Phase 6 Boundary Checks

| Area | Evidence | Boundary |
|---|---|---|
| Workshop package validation | `validate_workshop_package`, `workshop_metadata_snapshot_is_stable`, hash/size/path/secret tests | Local/offline metadata and file-hash validation only. |
| Workshop library | Import/load/remix/block/report/delete tests | Local library metadata and copied packages only. |
| Publish draft | `WorkshopPublishDraft`, `write_workshop_publish_draft` tests | Draft JSON keeps upload disabled and records that no Steamworks call occurred. |
| Upload port | `SteamworksUploadPort`, `LocalOnlySteamworksUploadPort`, fake adapter tests | Explicit config and credentials label are required before adapter call; default local port refuses external action. |
| Product modes | Creator Desktop Play/Creator/Developer mode UI and tests | Local shell entries only; no platform API ownership or external workflow claims. |
| Submission Kit | Eight generated Markdown drafts and snapshot/write tests | Draft support material generated from validated local evidence. |
| Active docs | README, Steam QA docs, no-launch-promise lint, CI job | Current project-facing docs stay inside the local evidence boundary. |

## Validation Evidence

Defined gates and smoke commands:

- `scripts/qa/full_local.sh` runs Rust formatting, no-launch-promise lint, workspace check/test, contract drift check, Creator Desktop QA, player-web QA, clippy, CLI tempdir smoke, desktop draft export smoke, and HTTP static export smoke.
- `.github/workflows/ci.yml` separates Rust static checks, Rust tests, Creator Desktop/player QA, CLI smoke, and export smoke.
- Targeted evidence commands listed in README include `cargo test -p plotforge-workshop`, `cargo run -p plotforge-cli -- export profiles`, `npm run creator-desktop:qa`, `npm run player-web:qa`, and `python3 scripts/qa/no_launch_promise_lint.py`.

Local validation run during this audit:

```bash
cargo fmt --all -- --check
python3 scripts/qa/no_launch_promise_lint.py
scripts/contracts/check_contracts.sh
cargo test -p plotforge-schema
cargo test -p plotforge-workshop
cargo test -p plotforge-cli --test cli_smoke
cargo test -p plotforge-studio
npm run creator-desktop:qa
npm run player-web:qa
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

Result: passed.

```bash
scripts/qa/full_local.sh
```

Result: passed after rerunning outside the file-system sandbox for the localhost HTTP smoke; the sandboxed run reached the same static export smoke and failed only when binding `127.0.0.1`.
