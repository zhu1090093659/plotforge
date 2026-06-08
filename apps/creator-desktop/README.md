# PlotForge Creator Desktop

`apps/creator-desktop` is the React/TypeScript Studio frontend workspace. It consumes generated PlotForge contract types from `contracts/plotforge.d.ts` and delegates project load/check/source edit/playtest behavior through browser-preview data sources or thin Tauri commands; runtime/project mutations remain in Rust core crates.

## Commands

```bash
npm run creator-desktop:dev
npm run creator-desktop:typecheck
npm run creator-desktop:test
npm run creator-desktop:build
npm run creator-desktop:smoke
npm run creator-desktop:tauri:check
npm run creator-desktop:tauri:dev
npm run creator-desktop:qa
```

Do not hand-maintain contract shapes in this app. Regenerate `contracts/` from `plotforge-schema` when Rust schema changes.

Tauri commands are thin IPC wrappers in `src-tauri`; command behavior belongs in Rust adapter crates such as `plotforge-studio`.

`npm run creator-desktop:qa` runs typecheck, Vitest, Vite build, `scripts/qa/creator_desktop_build_smoke.py`, and Tauri check. Local interactive Browser/Computer Use smoke steps live in `scripts/qa/computer_use_creator_desktop.md` and must stay local-only.
