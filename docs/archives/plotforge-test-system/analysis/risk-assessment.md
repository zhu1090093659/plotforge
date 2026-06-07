# Risk Assessment

## S.U.P.E.R Architecture Health Summary

| Principle | Status | Key Findings | Transformation Priority |
|:--|:--|:--|:--|
| **S** Single Purpose | 🟡 | Storage and test scripts can become broad if generation, validation, HTTP smoke, and UI smoke are mixed. | High |
| **U** Unidirectional Flow | 🟡 | Current core flow is mostly inward, but CLI/export tests must avoid duplicating runtime/storage logic. | Medium |
| **P** Ports over Implementation | 🟡 | Schema contracts exist, but tests do not yet lock `ProjectData`, `ExportManifest`, traces, and fixture shape broadly. | High |
| **E** Environment-Agnostic | 🟢 | Rust tests can run without providers; Computer Use must remain local-only and out of CI. | High |
| **R** Replaceable Parts | 🟡 | Export/player and future desktop UI need stable smoke tests that do not bind to implementation internals. | Medium |

**Overall Health**: _1/5 fully healthy, 4/5 directionally healthy_ — Refactoring Needed.

### S.U.P.E.R Violation Hotspots

1. **CLI has no black-box tests**: README commands can drift without CI catching it.
2. **Fixture double-source risk**: `examples/dynasty-embers` can diverge from `create_demo_project`.
3. **Storage broad responsibility**: file layout, demo data, trace writes, and asset writes share one module; negative tests are essential.
4. **Computer Use misuse risk**: desktop UI actions cannot become CI requirements or mutate external accounts.
5. **Static export visual risk**: file existence does not prove the exported player renders and responds.

## Risk Matrix

| Risk | Impact | Likelihood | Severity | Mitigation |
|:--|:--|:--|:--|:--|
| CLI smoke not automated | README commands rot | High | High | Add `assert_cmd` integration tests and `scripts/qa/full_local.sh` |
| Fixture drift | Demo works in one path but not the other | High | High | Compare generated and committed fixture semantics |
| Browser smoke overfits to file paths | False failures on CI | Medium | Medium | Use temp dirs and localhost HTTP |
| Computer Use added to CI | Non-repeatable CI | Medium | High | Keep as local runbook only |
| Secret leak in export | Security regression | Low | Critical | Add export secret marker regression test |
| Negative paths stay untested | Hidden fallback or silent success | Medium | High | Add force/missing file/bad data/error tests |

## High-Severity Risks

### CLI and Fixture Drift

The checked-in example and generated demo can diverge because they are separate file trees. The test system must validate both and compare key semantic facts: project id, entry scene, resources, rules, characters, plot threads, and exportable scene assets.

### Computer Use Boundary

Computer Use should verify a real local browser window, not replace deterministic tests. It must not upload files, log into accounts, grant permissions, or depend on coordinates when an accessibility element is available.

### Static Export Behavior

Current export tests check files exist. The expanded system must validate `game.json`, asset references, and a served HTTP smoke because the player uses `fetch("./game.json")`.

## Technical Debt

- `plotforge-cli` has no integration tests.
- CI is single-job and misses CLI/export smoke.
- No local all-in-one QA script.
- No documented Computer Use smoke procedure.

## Compatibility Concerns

- New tests must not require real providers, API keys, Tauri, Steam, or internet access.
- GitHub Actions cannot run Codex Desktop Computer Use.
- Test artifacts such as `dist/`, traces, screenshots, and local QA artifacts should stay ignored.

