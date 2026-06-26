/**
 * @fileoverview Player save module — owns localStorage-backed scene/beat progress
 * for the no-network static player. Mirrors the player-progress shape produced by
 * plotforge-export and is intentionally independent of i18n/audio/render concerns.
 */

/**
 * The static player's persisted progress position. Field names use the snake_case
 * shape stored in localStorage so legacy saves keep loading.
 *
 * @typedef {Object} PlayerSavePosition
 * @property {string | null} sceneKey
 * @property {string | null} beatId
 */

/**
 * LocalStorage-backed save store scoped to a single export package. Keys are
 * derived from the manifest game id/version so saves never collide between
 * projects or between versions of the same project.
 *
 * @typedef {Object} PlayerSaveStore
 * @property {string} key
 * @property {boolean} available
 * @property {() => PlayerSavePosition | null} load
 * @property {(position: PlayerSavePosition) => void} save
 */

/**
 * Build a localStorage-backed save store for a single export package.
 *
 * @param {Document | ShadowRoot} root
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @returns {PlayerSaveStore}
 */
export function createSaveStore(root, manifest) {
  const view = root.defaultView ?? globalThis.window;
  const storage = localStorageFor(view);
  const key = `plotforge:${manifest.game.id}:${manifest.game.version}:player-progress`;
  return {
    key,
    available: Boolean(storage),
    load() {
      if (!storage) {
        return null;
      }
      const raw = storage.getItem(key);
      if (!raw) {
        return null;
      }
      let parsed;
      try {
        parsed = JSON.parse(raw);
      } catch (error) {
        throw new Error("Invalid PlotForge player save: malformed JSON", {
          cause: error,
        });
      }
      if (
        parsed?.project_id !== manifest.game.id ||
        parsed?.project_version !== manifest.game.version
      ) {
        throw new Error("Invalid PlotForge player save: project mismatch");
      }
      return {
        sceneKey: typeof parsed.scene_key === "string" ? parsed.scene_key : null,
        beatId: typeof parsed.beat_id === "string" ? parsed.beat_id : null,
      };
    },
    save(position) {
      if (!storage) {
        return;
      }
      storage.setItem(
        key,
        JSON.stringify({
          project_id: manifest.game.id,
          project_version: manifest.game.version,
          scene_key: position.sceneKey,
          beat_id: position.beatId,
        }),
      );
    },
  };
}

/**
 * Resolve a localStorage handle defensively. Browsers throw on access in
 * private mode / sandboxed frames; we degrade to "unavailable" instead of
 * crashing the player.
 *
 * @param {Window | undefined} view
 * @returns {Storage | null}
 */
export function localStorageFor(view) {
  try {
    return view?.localStorage ?? null;
  } catch {
    return null;
  }
}
