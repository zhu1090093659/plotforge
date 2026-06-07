# PlotForge Creator Desktop

`apps/creator-desktop` is the React/TypeScript Studio frontend workspace. It is currently a Vite shell that consumes generated PlotForge contract types from `contracts/plotforge.d.ts`; runtime/project mutations remain in Rust core crates and future Tauri commands.

## Commands

```bash
npm run creator-desktop:dev
npm run creator-desktop:typecheck
npm run creator-desktop:test
npm run creator-desktop:build
npm run creator-desktop:qa
```

Do not hand-maintain contract shapes in this app. Regenerate `contracts/` from `plotforge-schema` when Rust schema changes.
