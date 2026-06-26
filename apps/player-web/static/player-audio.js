/**
 * @fileoverview Player audio module — owns lazy, no-network audio rendering for
 * the static player. Resolves scene/beat audio refs against the export manifest's
 * asset records and the package asset whitelist, blocks external paths, and
 * exposes resolution state on the mount dataset for tracing/testability.
 */

/**
 * Resolved audio state for a scene/beat. `status` mirrors what gets written to
 * the mount `data-audio-state` attribute.
 *
 * @typedef {{ status: "ready", path: string } | { status: "missing-asset" } | { status: "blocked-external" }} AudioResolution
 */

/**
 * Render the audio panel for a scene/beat against the manifest. Lazily creates
 * the <audio> element with `preload="none"` (no autoplay, no network), and
 * records the resolution state on the mount dataset. When no ready ref exists
 * the panel is cleared and hidden.
 *
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {import("./player-types.js").SceneLike} scene
 * @param {import("./player-types.js").BeatLike | null | undefined} beat
 * @param {Document | ShadowRoot} root
 * @param {HTMLElement} mount
 * @param {import("./player-core.js").PlayerI18n} ui
 * @returns {void}
 */
export function renderAudio(manifest, scene, beat, root, mount, ui) {
  const panel = queryField(root, "audio-panel");
  const audioReference = audioReferenceFor(manifest, scene, beat);
  if (!audioReference) {
    panel.replaceChildren();
    panel.hidden = true;
    mount.dataset.audioState = "none";
    delete mount.dataset.audioSrc;
    return;
  }

  if (audioReference.status !== "ready") {
    panel.replaceChildren();
    panel.hidden = true;
    mount.dataset.audioState = audioReference.status;
    delete mount.dataset.audioSrc;
    return;
  }

  const audioPath = audioReference.path;
  const audio = ensureAudioElement(panel, root, mount, ui);
  if (audio.getAttribute("src") !== audioPath) {
    audio.setAttribute("src", audioPath);
  }
  panel.hidden = false;
  mount.dataset.audioState = "ready";
  mount.dataset.audioSrc = audioPath;
}

/**
 * @param {HTMLElement} panel
 * @param {Document | ShadowRoot} root
 * @param {HTMLElement} mount
 * @param {import("./player-core.js").PlayerI18n} ui
 * @returns {HTMLAudioElement}
 */
function ensureAudioElement(panel, root, mount, ui) {
  const existing = panel.querySelector("audio");
  if (existing) {
    return existing;
  }
  const audio = root.createElement("audio");
  audio.controls = true;
  audio.preload = "none";
  audio.dataset.field = "scene-audio";
  audio.setAttribute("aria-label", ui.t("sceneAudio"));
  audio.addEventListener("error", () => {
    mount.dataset.audioState = "error";
  });
  panel.replaceChildren(audio);
  return audio;
}

/**
 * Beat refs take priority over scene refs; missing refs fall through to null.
 *
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {import("./player-types.js").SceneLike} scene
 * @param {import("./player-types.js").BeatLike | null | undefined} beat
 * @returns {AudioResolution | null}
 */
function audioReferenceFor(manifest, scene, beat) {
  return (
    resolveAudioRefs(manifest, beat?.audio_refs) ??
    resolveAudioRefs(manifest, scene?.audio_refs) ??
    null
  );
}

/**
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {unknown} refs
 * @returns {AudioResolution | null}
 */
function resolveAudioRefs(manifest, refs) {
  if (!Array.isArray(refs)) {
    return null;
  }
  for (const ref of refs) {
    const resolved = resolveAudioRef(manifest, ref);
    if (resolved) {
      return resolved;
    }
  }
  return null;
}

/**
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {unknown} ref
 * @returns {AudioResolution | null}
 */
function resolveAudioRef(manifest, ref) {
  if (!ref || typeof ref !== "object") {
    return null;
  }
  const candidate = /** @type {{ kind?: string, asset_id?: unknown, export_path?: unknown }} */ (ref);
  if (candidate.kind !== "audio" && candidate.kind !== "voice") {
    return null;
  }
  const record = assetRecordForReference(manifest, candidate);
  if (!record) {
    return { status: "missing-asset" };
  }
  const path = record.export_path;
  if (!isReachableAudioPath(manifest, path)) {
    return { status: "missing-asset" };
  }
  if (!isLocalAssetPath(path)) {
    return { status: "blocked-external" };
  }
  return { status: "ready", path };
}

/**
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {{ kind?: string, asset_id?: unknown, export_path?: unknown }} ref
 * @returns {import("./player-types.js").AssetRecordLike | null}
 */
function assetRecordForReference(manifest, ref) {
  const records = Array.isArray(manifest.asset_records) ? manifest.asset_records : [];
  return (
    records.find((record) => {
      const idMatches = typeof ref.asset_id === "string" && record.id === ref.asset_id;
      const pathMatches =
        typeof ref.export_path === "string" && record.export_path === ref.export_path;
      return (
        (idMatches || pathMatches) &&
        (record.kind === "audio" || record.kind === "voice")
      );
    }) ?? null
  );
}

/**
 * Reject paths absent from the package asset whitelist or outside `assets/`,
 * and reject traversal segments (`..`, `.`) before they reach a fetch.
 *
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {unknown} path
 * @returns {boolean}
 */
function isReachableAudioPath(manifest, path) {
  if (typeof path !== "string") {
    return false;
  }
  const assets = Array.isArray(manifest.assets) ? manifest.assets : [];
  if (!assets.includes(path)) {
    return false;
  }
  if (!path.startsWith("assets/")) {
    return false;
  }
  for (const part of path.split("/")) {
    if (!part || part === "." || part === "..") {
      return false;
    }
  }
  return true;
}

/**
 * Block protocol-relative (`//`) and absolute-scheme (`https:`) URLs so the
 * no-network static player can never fetch off-package audio.
 *
 * @param {unknown} path
 * @returns {boolean}
 */
function isLocalAssetPath(path) {
  if (typeof path !== "string") {
    return false;
  }
  const trimmed = path.trim();
  if (trimmed.length === 0) {
    return false;
  }
  if (trimmed.startsWith("//") || /^[a-z][a-z0-9+.-]*:/i.test(trimmed)) {
    return false;
  }
  return trimmed === path;
}

/**
 * Query a `[data-field]` element, mirroring the player-core field resolver so
 * this module stays free of a render-core import.
 *
 * @param {Document | ShadowRoot} root
 * @param {string} name
 * @returns {HTMLElement}
 */
function queryField(root, name) {
  const element = root.querySelector(`[data-field="${name}"]`);
  if (!element) {
    throw new Error(`Missing player field: ${name}`);
  }
  return element;
}
