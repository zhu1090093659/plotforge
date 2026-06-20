export async function loadManifest(fetchManifest = defaultFetchManifest) {
  return fetchManifest();
}

export async function bootPlayer(options = {}) {
  const root = options.root ?? document;
  const mount = playerRoot(root);
  const ui = createPlayerI18n(root, options.locale);
  try {
    const manifest = await loadManifest(options.fetchManifest);
    renderPlayer(manifest, root, { locale: ui.locale });
  } catch (error) {
    mount.dataset.state = "error";
    applyLocale(root, mount, ui.locale);
    text(root, "status", ui.t("exportFailed"));
    text(root, "scene-title", ui.t("unableToLoad"));
    text(root, "beat", error instanceof Error ? error.message : String(error));
  }
}

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

export function updateViewportMode(root = document) {
  const view = root.defaultView ?? globalThis.window;
  playerRoot(root).dataset.viewport =
    (view?.innerWidth ?? 1024) < 760 ? "mobile" : "desktop";
}

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
  image.setAttribute("src", scene.background_asset);
  image.setAttribute("alt", ui.t("sceneArtwork", { title: scene.title }));
  renderAudio(manifest, scene, beat, root, mount, ui);
  saveStore.save({ sceneKey: scene.key, beatId: beat?.id ?? null });
  renderChoices(manifest, beat, scene, root, mount, saveStore, ui);
}

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

function playerRoot(root) {
  const mount = root.querySelector("[data-player-root]");
  if (!mount) {
    throw new Error("Missing PlotForge player root.");
  }
  return mount;
}

function field(root, name) {
  const element = root.querySelector(`[data-field="${name}"]`);
  if (!element) {
    throw new Error(`Missing player field: ${name}`);
  }
  return element;
}

function text(root, name, value) {
  field(root, name).textContent = value;
}

function entryBeat(scene) {
  if (typeof scene.entry_beat_id !== "string" || scene.entry_beat_id.length === 0) {
    throw new Error(`Scene is missing explicit entry_beat_id: ${scene.key}`);
  }
  return scene.beats.find((beat) => beat.id === scene.entry_beat_id) ?? null;
}

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

function requiredEntryBeat(scene) {
  const beat = entryBeat(scene);
  if (!beat) {
    throw new Error(
      `Scene entry_beat_id does not match any beat: ${scene.key} -> ${scene.entry_beat_id}`,
    );
  }
  return beat;
}

function sceneByKey(manifest, sceneKey) {
  if (!sceneKey) {
    return null;
  }
  return manifest.scenes.find((scene) => scene.key === sceneKey) ?? null;
}

function requiredSavedScene(manifest, sceneKey) {
  const scene = sceneByKey(manifest, sceneKey);
  if (!scene) {
    throw new Error(`Invalid PlotForge player save: missing scene ${sceneKey}`);
  }
  return scene;
}

function beatById(scene, beatId) {
  if (!beatId) {
    return null;
  }
  return scene.beats.find((beat) => beat.id === beatId) ?? null;
}

function requiredSavedBeat(scene, beatId) {
  const beat = beatById(scene, beatId);
  if (!beat) {
    throw new Error(`Invalid PlotForge player save: missing beat ${beatId}`);
  }
  return beat;
}

function nextBeatInScene(scene, beat) {
  if (beat?.next?.kind !== "beat") {
    return null;
  }
  return scene.beats.find((candidate) => candidate.id === beat.next.payload) ?? null;
}

function requiredNextBeatInScene(scene, beat) {
  const nextBeat = nextBeatInScene(scene, beat);
  if (!nextBeat) {
    throw new Error(
      `Beat ${beat?.id ?? "unknown"} in scene ${scene.key} has no same-scene transition.`,
    );
  }
  return nextBeat;
}

function sceneAfter(manifest, scene) {
  const index = manifest.scenes.findIndex((candidate) => candidate.key === scene.key);
  if (index < 0) {
    throw new Error(`Current scene is missing from manifest: ${scene.key}`);
  }
  return manifest.scenes[index + 1] ?? null;
}

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

function renderAudio(manifest, scene, beat, root, mount, ui) {
  const panel = field(root, "audio-panel");
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
  const audio = ensureAudioElement(panel, root, ui);
  if (audio.getAttribute("src") !== audioPath) {
    audio.setAttribute("src", audioPath);
  }
  panel.hidden = false;
  mount.dataset.audioState = "ready";
  mount.dataset.audioSrc = audioPath;
}

function ensureAudioElement(panel, root, ui) {
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
    const mount = playerRoot(root);
    mount.dataset.audioState = "error";
  });
  panel.replaceChildren(audio);
  return audio;
}

function audioReferenceFor(manifest, scene, beat) {
  return (
    resolveAudioRefs(manifest, beat?.audio_refs) ??
    resolveAudioRefs(manifest, scene?.audio_refs) ??
    null
  );
}

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

function resolveAudioRef(manifest, ref) {
  if (!ref || typeof ref !== "object") {
    return null;
  }
  if (ref.kind !== "audio" && ref.kind !== "voice") {
    return null;
  }
  const record = assetRecordForReference(manifest, ref);
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

const playerLocaleStorageKey = "plotforge:player:locale";
const playerLocales = new Set(["en", "zh"]);

const playerMessages = {
  en: {
    exportFailed: "Export failed to load",
    unableToLoad: "Unable to load PlotForge export",
    staticExport: "Static export",
    selectedAction: ({ actionType }) => `Selected ${actionType}`,
    storyComplete: "Story complete",
    storyEnded: "This static story path has ended.",
    sceneProgress: ({ current, total }) => `Scene ${current} of ${total}`,
    beatProgress: ({ current, total }) => `Beat ${current} of ${total}`,
    scene: "Scene",
    beat: "Beat",
    sceneArtwork: ({ title }) => `${title} scene artwork`,
    sceneAudio: "Scene audio",
  },
  zh: {
    exportFailed: "导出加载失败",
    unableToLoad: "无法加载 PlotForge 导出",
    staticExport: "静态导出",
    selectedAction: ({ actionType }) => `已选择 ${actionType}`,
    storyComplete: "故事完成",
    storyEnded: "这条静态故事路径已结束。",
    sceneProgress: ({ current, total }) => `场景 ${current} / ${total}`,
    beatProgress: ({ current, total }) => `节拍 ${current} / ${total}`,
    scene: "场景",
    beat: "节拍",
    sceneArtwork: ({ title }) => `${title} 场景图`,
    sceneAudio: "场景音频",
  },
};

function createPlayerI18n(root, requestedLocale) {
  const locale = resolvePlayerLocale(root, requestedLocale);
  return {
    locale,
    t(key, values = {}) {
      const message = playerMessages[locale][key] ?? playerMessages.en[key];
      return typeof message === "function" ? message(values) : message;
    },
  };
}

function resolvePlayerLocale(root, requestedLocale) {
  if (isPlayerLocale(requestedLocale)) {
    return requestedLocale;
  }
  const view = root.defaultView ?? globalThis.window;
  const urlLocale = localeFromSearch(view?.location?.search);
  if (urlLocale) {
    return urlLocale;
  }
  const storedLocale = localStorageFor(view)?.getItem(playerLocaleStorageKey);
  if (isPlayerLocale(storedLocale)) {
    return storedLocale;
  }
  const htmlLocale = root.documentElement?.getAttribute("lang");
  if (isPlayerLocale(htmlLocale)) {
    return htmlLocale;
  }
  return view?.navigator?.language?.toLowerCase().startsWith("zh") ? "zh" : "en";
}

function localeFromSearch(search) {
  if (!search) {
    return null;
  }
  const params = new URLSearchParams(search);
  const value = params.get("lang") ?? params.get("language");
  return isPlayerLocale(value) ? value : null;
}

function isPlayerLocale(value) {
  return typeof value === "string" && playerLocales.has(value);
}

function applyLocale(root, mount, locale) {
  mount.dataset.locale = locale;
  root.documentElement?.setAttribute("lang", locale === "zh" ? "zh-CN" : "en");
}

function wireLanguageControl(manifest, root, mount, locale) {
  const select = root.querySelector('[data-field="language-select"]');
  if (!select) {
    return;
  }
  select.value = locale;
  select.addEventListener(
    "change",
    () => {
      const nextLocale = select.value;
      if (!isPlayerLocale(nextLocale)) {
        return;
      }
      const storage = localStorageFor(root.defaultView ?? globalThis.window);
      storage?.setItem(playerLocaleStorageKey, nextLocale);
      renderPlayer(manifest, root, { locale: nextLocale });
    },
    { once: true },
  );
  mount.dataset.languageControl = "ready";
}

function createSaveStore(root, manifest) {
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

function localStorageFor(view) {
  try {
    return view?.localStorage ?? null;
  } catch {
    return null;
  }
}
