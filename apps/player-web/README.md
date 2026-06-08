# PlotForge Static Player

`apps/player-web/static/` is the source for the static export player package copied by `plotforge-export`.

The player consumes `game.json` as an `ExportManifest`, loads local assets, and runs without external network dependencies. Keep DOM behavior here and keep export packaging in `crates/plotforge-export`.
