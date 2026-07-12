/**
 * @fileoverview Static player core — owns manifest loading, scene/beat routing,
 * and DOM rendering. Persists progress via `player-save`, localizes chrome via
 * `player-i18n`, and renders audio via `player-audio`. Runs without external
 * network URLs and never duplicates the rule/runtime state machine.
 */

import { createSaveStore } from "./player-save.js";
import { createPlayerI18n, applyLocale, wireLanguageControl } from "./player-i18n.js";
import { renderAudio } from "./player-audio.js";

/**
 * Player i18n facade, re-exported for cross-module JSDoc references
 * (player-audio annotates its `ui` parameter with this type).
 *
 * @typedef {Object} PlayerI18n
 * @property {"en" | "zh"} locale
 * @property {(key: string, values?: Record<string, string | number>) => string} t
 */

/**
 * Load the export manifest. The fetcher is injectable so tests can supply a
 * fixture manifest without a network round-trip.
 *
 * @param {() => Promise<import("./player-types.js").ExportManifestLike>} [fetchManifest]
 * @returns {Promise<import("./player-types.js").ExportManifestLike>}
 */
export async function loadManifest(fetchManifest = defaultFetchManifest) {
  return fetchManifest();
}

/**
 * Boot the player: load the manifest and render it, or surface a localized
 * error state on the mount when loading fails.
 *
 * @param {{ root?: Document | ShadowRoot, locale?: string, fetchManifest?: () => Promise<import("./player-types.js").ExportManifestLike> }} [options]
 * @returns {Promise<void>}
 */
export async function bootPlayer(options = {}) {
  const root = options.root ?? document;
  const mount = playerRoot(root);
  const ui = createPlayerI18n(root, options.locale);
  try {
    const manifest = await loadManifest(options.fetchManifest);
    renderPlayer(manifest, root, { locale: ui.locale });
  } catch (error) {
    renderErrorState(root, mount, ui, error);
  }
}

/**
 * Render a friendly, localized error state when the manifest fails to load.
 * Clears the playing surface (choices, scene image, authored chrome) so the
 * mount never shows a broken image or stale "Loading" text, then surfaces a
 * short title plus the underlying diagnostic message. The mount's
 * `data-state="error"` drives the restrained error layout in styles.css.
 *
 * @param {Document | ShadowRoot} root
 * @param {HTMLElement} mount
 * @param {PlayerI18n} ui
 * @param {unknown} error
 * @returns {void}
 */
function renderErrorState(root, mount, ui, error) {
  mount.dataset.state = "error";
  applyLocale(root, mount, ui.locale);
  text(root, "status", ui.t("exportFailed"));
  text(root, "game-title", "");
  text(root, "scene-title", ui.t("unableToLoad"));
  text(root, "hook", "");
  text(root, "outcome", "");
  text(root, "beat", error instanceof Error ? error.message : String(error));
  field(root, "choices").replaceChildren();
  const image = field(root, "scene-image");
  image.removeAttribute("src");
  image.setAttribute("alt", "");
}

/**
 * Render a manifest onto the DOM. Resumes from localStorage when a valid save
 * exists, otherwise starts at the manifest entry scene/beat. Returns a small
 * summary used by tests and any host shell.
 *
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {Document | ShadowRoot} [root]
 * @param {{ locale?: string }} [options]
 * @returns {{ sceneKey: string, choiceCount: number }}
 */
export function renderPlayer(manifest, root = document, options = {}) {
  const mount = playerRoot(root);
  const ui = createPlayerI18n(root, options.locale);
  const saveStore = createSaveStore(root, manifest);
  const savedPosition = saveStore.load();
  const scene = savedPosition
    ? requiredSavedScene(manifest, savedPosition.sceneKey)
    : requiredEntryScene(manifest);

  const firstBeat = savedPosition
    ? requiredSavedBeat(scene, savedPosition.beatId)
    : requiredEntryBeat(scene);
  root.title = manifest.game.title;
  mount.dataset.state = "ready";
  mount.dataset.saveKey = saveStore.key;
  mount.dataset.saveState = saveStore.available ? "ready" : "unavailable";
  applyLocale(root, mount, ui.locale);
  wireLanguageControl(manifest, root, mount, ui.locale);
  text(root, "status", ui.t("staticExport"));
  text(root, "game-title", manifest.game.title);
  renderSceneBeat(manifest, scene, firstBeat, root, mount, saveStore, ui);

  updateViewportMode(root);
  return { sceneKey: scene.key, choiceCount: firstBeat?.choices.length ?? 0 };
}

/**
 * Tag the mount with a mobile/desktop viewport bucket so CSS can adapt layout.
 * The breakpoint mirrors the static player's mobile design (see styles.css).
 *
 * @param {Document | ShadowRoot} [root]
 * @returns {void}
 */
export function updateViewportMode(root = document) {
  const view = root.defaultView ?? globalThis.window;
  playerRoot(root).dataset.viewport =
    (view?.innerWidth ?? 1024) < 760 ? "mobile" : "desktop";
}

/**
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {import("./player-types.js").SceneLike} scene
 * @param {import("./player-types.js").BeatLike | null | undefined} beat
 * @param {Document | ShadowRoot} root
 * @param {HTMLElement} mount
 * @param {import("./player-save.js").PlayerSaveStore} saveStore
 * @param {PlayerI18n} ui
 * @returns {void}
 */
function renderSceneBeat(manifest, scene, beat, root, mount, saveStore, ui) {
  const image = field(root, "scene-image");
  const sceneIndex = manifest.scenes.findIndex((candidate) => candidate.key === scene.key);
  const beatIndex = scene.beats.findIndex((candidate) => candidate.id === beat?.id);
  mount.dataset.currentSceneKey = scene.key;
  if (beat?.id) {
    mount.dataset.currentBeatId = beat.id;
  } else {
    delete mount.dataset.currentBeatId;
  }
  mount.dataset.mode = "playing";
  text(root, "scene-title", scene.title);
  text(root, "hook", scene.hook);
  text(root, "outcome", "");
  text(root, "beat", beat?.text ?? "");
  text(root, "progress", progressLabel(sceneIndex, manifest.scenes.length, beatIndex, scene.beats.length, ui));
  if (scene.background_asset) {
    image.hidden = false;
    image.setAttribute("src", scene.background_asset);
    image.setAttribute("alt", ui.t("sceneArtwork", { title: scene.title }));
  } else {
    image.hidden = true;
    image.removeAttribute("src");
    image.setAttribute("alt", "");
  }
  renderAudio(manifest, scene, beat, root, mount, ui);
  saveStore.save({ sceneKey: scene.key, beatId: beat?.id ?? null });
  renderChoices(manifest, beat, scene, root, mount, saveStore, ui);
}

/**
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {import("./player-types.js").BeatLike | null | undefined} beat
 * @param {import("./player-types.js").SceneLike} scene
 * @param {Document | ShadowRoot} root
 * @param {HTMLElement} mount
 * @param {import("./player-save.js").PlayerSaveStore} saveStore
 * @param {PlayerI18n} ui
 * @returns {void}
 */
function renderChoices(manifest, beat, scene, root, mount, saveStore, ui) {
  const choices = field(root, "choices");
  choices.replaceChildren();
  for (const choice of beat?.choices ?? []) {
    const button = root.createElement("button");
    button.type = "button";
    button.className = "pf-choice";
    button.dataset.choiceId = choice.id;
    button.dataset.actionType = choice.action_type;
    button.setAttribute("aria-pressed", "false");
    button.textContent = choice.label;
    button.addEventListener("click", () => {
      for (const existing of choices.querySelectorAll("button")) {
        existing.setAttribute("aria-pressed", "false");
      }
      button.setAttribute("aria-pressed", "true");
      mount.dataset.lastChoice = choice.id;
      text(root, "status", ui.t("selectedAction", { actionType: choice.action_type }));
      text(root, "outcome", choice.dramatic_purpose);
      applyChoiceTransition(manifest, scene, beat, choice, root, mount, saveStore, ui);
    });
    choices.appendChild(button);
  }
}

/**
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {import("./player-types.js").SceneLike} scene
 * @param {import("./player-types.js").BeatLike | null | undefined} beat
 * @param {import("./player-types.js").ChoiceLike} choice
 * @param {Document | ShadowRoot} root
 * @param {HTMLElement} mount
 * @param {import("./player-save.js").PlayerSaveStore} saveStore
 * @param {PlayerI18n} ui
 * @returns {void}
 */
function applyChoiceTransition(manifest, scene, beat, choice, root, mount, saveStore, ui) {
  const transition = resolveChoiceTransition(manifest, scene, beat, choice);
  if (transition.kind === "beat") {
    renderSceneBeat(manifest, scene, transition.beat, root, mount, saveStore, ui);
    text(root, "outcome", choice.dramatic_purpose);
    return;
  }

  if (transition.kind === "scene") {
    const entry = requiredEntryBeat(transition.scene);
    renderSceneBeat(manifest, transition.scene, entry, root, mount, saveStore, ui);
    text(root, "outcome", choice.dramatic_purpose);
    return;
  }

  renderEndState(manifest, scene, beat, choice, root, mount, saveStore, ui);
}

/**
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {import("./player-types.js").SceneLike} scene
 * @param {import("./player-types.js").BeatLike | null | undefined} beat
 * @param {import("./player-types.js").ChoiceLike} choice
 * @param {Document | ShadowRoot} root
 * @param {HTMLElement} mount
 * @param {import("./player-save.js").PlayerSaveStore} saveStore
 * @param {PlayerI18n} ui
 * @returns {void}
 */
function renderEndState(manifest, scene, beat, choice, root, mount, saveStore, ui) {
  mount.dataset.mode = "ended";
  mount.dataset.currentSceneKey = scene.key;
  if (beat?.id) {
    mount.dataset.currentBeatId = beat.id;
  } else {
    delete mount.dataset.currentBeatId;
  }
  text(root, "status", ui.t("storyComplete"));
  text(root, "scene-title", scene.title);
  text(root, "hook", scene.hook);
  text(root, "outcome", choice.dramatic_purpose);
  text(root, "beat", ui.t("storyEnded"));
  const sceneIndex = manifest.scenes.findIndex((candidate) => candidate.key === scene.key);
  const beatIndex = scene.beats.findIndex((candidate) => candidate.id === beat?.id);
  text(root, "progress", progressLabel(sceneIndex, manifest.scenes.length, beatIndex, scene.beats.length, ui));
  renderAudio(manifest, scene, beat, root, mount, ui);
  field(root, "choices").replaceChildren();
  saveStore.save({ sceneKey: scene.key, beatId: beat?.id ?? null });
}

/**
 * Resolve a typed transition for a choice. The static player only renders
 * scene/beat navigation authored in the manifest; it never recomputes runtime
 * state or calls a planner.
 *
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {import("./player-types.js").SceneLike} scene
 * @param {import("./player-types.js").BeatLike | null | undefined} beat
 * @param {import("./player-types.js").ChoiceLike} choice
 * @returns {{ kind: "beat", beat: import("./player-types.js").BeatLike } | { kind: "scene", scene: import("./player-types.js").SceneLike } | { kind: "end" }}
 */
function resolveChoiceTransition(manifest, scene, beat, choice) {
  if (!beat) {
    return { kind: "end" };
  }
  if (!choice.change_scene) {
    return { kind: "beat", beat: requiredNextBeatInScene(scene, beat) };
  }
  if (beat.next?.kind === "end") {
    return { kind: "end" };
  }
  if (beat.next?.kind === "scene" || beat.next?.kind === "beat") {
    const nextScene = sceneAfter(manifest, scene);
    return nextScene ? { kind: "scene", scene: nextScene } : { kind: "end" };
  }
  return { kind: "end" };
}

async function defaultFetchManifest() {
  const response = await fetch("./game.json", { cache: "no-store" });
  if (!response.ok) {
    throw new Error(`Unable to load game.json: ${response.status}`);
  }
  return response.json();
}

/**
 * @param {Document | ShadowRoot} root
 * @returns {HTMLElement}
 */
function playerRoot(root) {
  const mount = root.querySelector("[data-player-root]");
  if (!mount) {
    throw new Error("Missing PlotForge player root.");
  }
  return mount;
}

/**
 * @param {Document | ShadowRoot} root
 * @param {string} name
 * @returns {HTMLElement}
 */
function field(root, name) {
  const element = root.querySelector(`[data-field="${name}"]`);
  if (!element) {
    throw new Error(`Missing player field: ${name}`);
  }
  return element;
}

/**
 * @param {Document | ShadowRoot} root
 * @param {string} name
 * @param {string} value
 * @returns {void}
 */
function text(root, name, value) {
  field(root, name).textContent = value;
}

/**
 * @param {import("./player-types.js").SceneLike} scene
 * @returns {import("./player-types.js").BeatLike | null}
 */
function entryBeat(scene) {
  if (typeof scene.entry_beat_id !== "string" || scene.entry_beat_id.length === 0) {
    throw new Error(`Scene is missing explicit entry_beat_id: ${scene.key}`);
  }
  return scene.beats.find((beat) => beat.id === scene.entry_beat_id) ?? null;
}

/**
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @returns {import("./player-types.js").SceneLike}
 */
function requiredEntryScene(manifest) {
  if (manifest.scenes.length === 0) {
    throw new Error("ExportManifest contains no scenes.");
  }
  const scene = sceneByKey(manifest, manifest.entry_scene);
  if (!scene) {
    throw new Error(`ExportManifest entry scene is missing: ${manifest.entry_scene}`);
  }
  return scene;
}

/**
 * @param {import("./player-types.js").SceneLike} scene
 * @returns {import("./player-types.js").BeatLike}
 */
function requiredEntryBeat(scene) {
  const beat = entryBeat(scene);
  if (!beat) {
    throw new Error(
      `Scene entry_beat_id does not match any beat: ${scene.key} -> ${scene.entry_beat_id}`,
    );
  }
  return beat;
}

/**
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {string | null | undefined} sceneKey
 * @returns {import("./player-types.js").SceneLike | null}
 */
function sceneByKey(manifest, sceneKey) {
  if (!sceneKey) {
    return null;
  }
  return manifest.scenes.find((scene) => scene.key === sceneKey) ?? null;
}

/**
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {string | null | undefined} sceneKey
 * @returns {import("./player-types.js").SceneLike}
 */
function requiredSavedScene(manifest, sceneKey) {
  const scene = sceneByKey(manifest, sceneKey);
  if (!scene) {
    throw new Error(`Invalid PlotForge player save: missing scene ${sceneKey}`);
  }
  return scene;
}

/**
 * @param {import("./player-types.js").SceneLike} scene
 * @param {string | null | undefined} beatId
 * @returns {import("./player-types.js").BeatLike | null}
 */
function beatById(scene, beatId) {
  if (!beatId) {
    return null;
  }
  return scene.beats.find((beat) => beat.id === beatId) ?? null;
}

/**
 * @param {import("./player-types.js").SceneLike} scene
 * @param {string | null | undefined} beatId
 * @returns {import("./player-types.js").BeatLike}
 */
function requiredSavedBeat(scene, beatId) {
  const beat = beatById(scene, beatId);
  if (!beat) {
    throw new Error(`Invalid PlotForge player save: missing beat ${beatId}`);
  }
  return beat;
}

/**
 * @param {import("./player-types.js").SceneLike} scene
 * @param {import("./player-types.js").BeatLike | null | undefined} beat
 * @returns {import("./player-types.js").BeatLike | null}
 */
function nextBeatInScene(scene, beat) {
  if (beat?.next?.kind !== "beat") {
    return null;
  }
  return scene.beats.find((candidate) => candidate.id === beat.next.payload) ?? null;
}

/**
 * @param {import("./player-types.js").SceneLike} scene
 * @param {import("./player-types.js").BeatLike | null | undefined} beat
 * @returns {import("./player-types.js").BeatLike}
 */
function requiredNextBeatInScene(scene, beat) {
  const nextBeat = nextBeatInScene(scene, beat);
  if (!nextBeat) {
    throw new Error(
      `Beat ${beat?.id ?? "unknown"} in scene ${scene.key} has no same-scene transition.`,
    );
  }
  return nextBeat;
}

/**
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {import("./player-types.js").SceneLike} scene
 * @returns {import("./player-types.js").SceneLike | null}
 */
function sceneAfter(manifest, scene) {
  const index = manifest.scenes.findIndex((candidate) => candidate.key === scene.key);
  if (index < 0) {
    throw new Error(`Current scene is missing from manifest: ${scene.key}`);
  }
  return manifest.scenes[index + 1] ?? null;
}

/**
 * @param {number} sceneIndex
 * @param {number} sceneCount
 * @param {number} beatIndex
 * @param {number} beatCount
 * @param {PlayerI18n} ui
 * @returns {string}
 */
function progressLabel(sceneIndex, sceneCount, beatIndex, beatCount, ui) {
  const scenePart =
    sceneIndex >= 0
      ? ui.t("sceneProgress", { current: sceneIndex + 1, total: sceneCount })
      : ui.t("scene");
  const beatPart =
    beatIndex >= 0
      ? ui.t("beatProgress", { current: beatIndex + 1, total: beatCount })
      : ui.t("beat");
  return `${scenePart} · ${beatPart}`;
}
