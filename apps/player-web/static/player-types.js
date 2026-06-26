/**
 * @fileoverview JSDoc type aliases for the static player. These mirror the
 * `ExportManifest` family in `contracts/plotforge.d.ts` (regenerated from
 * `plotforge-schema` via `scripts/contracts/export_contracts.sh`). Keep these
 * aligned with that contract to surface schema drift early in the editor and in
 * player tests; the static player must never reimplement rule/runtime behavior.
 *
 * Suffix `Like` marks intentionally-loose aliases: the player only reads the
 * fields it renders, so optional manifest fields (e.g. `profile`,
 * `ai_usage_manifest_path`) are omitted here rather than duplicated.
 */

/**
 * @typedef {Object} GameProjectLike
 * @property {string} id
 * @property {string} title
 * @property {string} version
 * @property {string} description
 * @property {string} entry_scene
 * @property {number} run_seed
 */

/**
 * Next-transition for a beat. Mirrors `BeatNext`:
 * `{ kind: "beat"; payload: string } | { kind: "scene" } | { kind: "end" } | { kind: "none" }`.
 *
 * @typedef {{ kind: "beat", payload: string } | { kind: "scene" } | { kind: "end" } | { kind: "none" }} BeatNextLike
 */

/**
 * @typedef {Object} ChoiceLike
 * @property {string} id
 * @property {string} label
 * @property {string} action_type
 * @property {string[]} input_terms
 * @property {string} dramatic_purpose
 * @property {boolean} change_scene
 */

/**
 * Asset kind mirrors `AssetKind`.
 *
 * @typedef {"image" | "audio" | "voice" | "data"} AssetKindLike
 */

/**
 * Asset source mirrors `AssetSourceKind`.
 *
 * @typedef {"user_import" | "generated" | "placeholder" | "external"} AssetSourceKindLike
 */

/**
 * Media asset reference mirrors `MediaAssetReference`. `slot` is opaque to the
 * player; `export_path` is the package-relative path that gets fetched.
 *
 * @typedef {Object} MediaAssetReferenceLike
 * @property {string | null | undefined} asset_id
 * @property {AssetKindLike} kind
 * @property {AssetSourceKindLike} source
 * @property {string} project_path
 * @property {string} export_path
 * @property {string} slot
 */

/**
 * @typedef {Object} BeatLike
 * @property {string} id
 * @property {string} text
 * @property {string | null | undefined} speaker
 * @property {string | null | undefined} line_delivery
 * @property {MediaAssetReferenceLike[]} audio_refs
 * @property {ChoiceLike[]} choices
 * @property {BeatNextLike | null | undefined} next
 */

/**
 * @typedef {Object} SceneLike
 * @property {string} key
 * @property {string} title
 * @property {string} location
 * @property {string} dramatic_purpose
 * @property {string} hook
 * @property {string} background_asset
 * @property {MediaAssetReferenceLike[]} audio_refs
 * @property {string[]} character_ids
 * @property {Record<string, string>} plot_thread_updates
 * @property {BeatLike[]} beats
 * @property {string | null | undefined} entry_beat_id
 */

/**
 * Asset record mirrors `AssetRecord` (provider/references fields omitted — the
 * player only reads id/kind/export_path).
 *
 * @typedef {Object} AssetRecordLike
 * @property {string} id
 * @property {AssetKindLike} kind
 * @property {AssetSourceKindLike} source
 * @property {string} project_path
 * @property {string} export_path
 * @property {string} content_hash
 * @property {string} hash_algorithm
 * @property {number} byte_length
 */

/**
 * Export manifest mirrors `ExportManifest`. `profile`/`ai_usage_manifest_path`
 * are omitted because the player does not render them.
 *
 * @typedef {Object} ExportManifestLike
 * @property {GameProjectLike} game
 * @property {string} entry_scene
 * @property {SceneLike[]} scenes
 * @property {string[]} assets
 * @property {AssetRecordLike[]} asset_records
 * @property {string} generated_by
 */
