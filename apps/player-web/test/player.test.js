import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { JSDOM } from "jsdom";
import { describe, expect, it } from "vitest";
import { renderPlayer, bootPlayer } from "../static/player-core.js";
import { createSaveStore } from "../static/player-save.js";
import { createPlayerI18n, applyLocale } from "../static/player-i18n.js";
import { renderAudio } from "../static/player-audio.js";

const indexHtml = readStaticFile("index.html");
const playerJs = readStaticFile("player.js");
const playerCoreJs = readStaticFile("player-core.js");
const playerTypesJs = readStaticFile("player-types.js");
const playerSaveJs = readStaticFile("player-save.js");
const playerI18nJs = readStaticFile("player-i18n.js");
const playerAudioJs = readStaticFile("player-audio.js");
const stylesCss = readStaticFile("styles.css");

const playerSourceFiles = [
  indexHtml,
  playerJs,
  playerCoreJs,
  playerTypesJs,
  playerSaveJs,
  playerI18nJs,
  playerAudioJs,
  stylesCss,
];

describe("PlotForge static player", () => {
  it("renders ExportManifest and handles DOM choice clicks", () => {
    const dom = new JSDOM(indexHtml, {
      url: "http://127.0.0.1:4173/",
      pretendToBeVisual: true,
    });

    const result = renderPlayer(sampleManifest(), dom.window.document);
    const continueChoice = dom.window.document.querySelector(
      '[data-action-type="continue"]',
    );
    if (!continueChoice) {
      throw new Error("expected continue choice button");
    }
    continueChoice.click();

    expect(result).toEqual({ sceneKey: "court-crisis-001", choiceCount: 3 });
    expect(dom.window.document.title).toBe("Dynasty Embers");
    expect(dom.window.document.querySelector("[data-player-root]")?.dataset.state).toBe(
      "ready",
    );
    expect(
      dom.window.document.querySelector('[data-field="scene-title"]')?.textContent,
    ).toBe("The Red Deficit Ledger");
    expect(
      dom.window.document.querySelector('[data-field="progress"]')?.textContent,
    ).toBe("Scene 1 of 2 · Beat 2 of 2");
    expect(
      dom.window.document.querySelector('[data-field="outcome"]')?.textContent,
    ).toBe("Stay in the council scene.");
    expect(dom.window.document.querySelector('[data-field="beat"]')?.textContent).toBe(
      "The war minister asks whether delay is now policy or merely fear.",
    );
    expect(continueChoice.getAttribute("aria-pressed")).toBe("true");
    expect(
      dom.window.document.querySelector("[data-player-root]")?.dataset.currentBeatId,
    ).toBe("court-crisis-001-beat-002");
    expect(
      dom.window.localStorage.getItem(
        "plotforge:dynasty-embers:0.1.0:player-progress",
      ),
    ).toContain("court-crisis-001-beat-002");
    expect(
      dom.window.document.querySelectorAll('[data-field="choices"] button').length,
    ).toBe(1);
  });

  it("renders static player chrome in Chinese and persists language changes", () => {
    const dom = new JSDOM(indexHtml, {
      url: "http://127.0.0.1:4173/",
      pretendToBeVisual: true,
    });

    renderPlayer(sampleManifest(), dom.window.document, { locale: "zh" });
    const continueChoice = dom.window.document.querySelector(
      '[data-action-type="continue"]',
    );
    if (!continueChoice) {
      throw new Error("expected continue choice button");
    }
    continueChoice.click();

    expect(dom.window.document.documentElement.lang).toBe("zh-CN");
    expect(dom.window.document.querySelector("[data-player-root]")?.dataset.locale).toBe(
      "zh",
    );
    expect(
      dom.window.document.querySelector('[data-field="status"]')?.textContent,
    ).toBe("已选择 continue");
    expect(
      dom.window.document.querySelector('[data-field="progress"]')?.textContent,
    ).toBe("场景 1 / 2 · 节拍 2 / 2");

    const languageSelect = dom.window.document.querySelector(
      '[data-field="language-select"]',
    );
    if (!languageSelect) {
      throw new Error("expected language select");
    }
    languageSelect.value = "en";
    languageSelect.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
    expect(
      dom.window.localStorage.getItem("plotforge:player:locale"),
    ).toBe("en");
    expect(dom.window.document.documentElement.lang).toBe("en");
  });


  it("navigates scene graph choices and ends a completed static path", () => {
    const dom = new JSDOM(indexHtml, {
      url: "http://127.0.0.1:4173/",
      pretendToBeVisual: true,
    });

    renderPlayer(sampleManifest(), dom.window.document);
    const raiseTaxChoice = dom.window.document.querySelector(
      '[data-choice-id="raise-tax"]',
    );
    if (!raiseTaxChoice) {
      throw new Error("expected raise-tax choice button");
    }
    raiseTaxChoice.click();

    expect(
      dom.window.document.querySelector("[data-player-root]")?.dataset.currentSceneKey,
    ).toBe("tax-riot-002");
    expect(
      dom.window.document.querySelector("[data-player-root]")?.dataset.currentBeatId,
    ).toBe("tax-riot-002-beat-001");
    expect(
      dom.window.document.querySelector("[data-player-root]")?.dataset.mode,
    ).toBe("playing");
    expect(
      dom.window.document.querySelector('[data-field="scene-title"]')?.textContent,
    ).toBe("Provincial Tax Riot");
    expect(
      dom.window.document.querySelector('[data-field="outcome"]')?.textContent,
    ).toBe("Raise emergency funds while damaging public order.");
    expect(
      dom.window.document.querySelector('[data-field="progress"]')?.textContent,
    ).toBe("Scene 2 of 2 · Beat 1 of 1");
    expect(
      dom.window.localStorage.getItem(
        "plotforge:dynasty-embers:0.1.0:player-progress",
      ),
    ).toContain("tax-riot-002-beat-001");

    const holdLineChoice = dom.window.document.querySelector(
      '[data-choice-id="hold-line"]',
    );
    if (!holdLineChoice) {
      throw new Error("expected hold-line choice button");
    }
    holdLineChoice.click();

    expect(
      dom.window.document.querySelector("[data-player-root]")?.dataset.mode,
    ).toBe("ended");
    expect(dom.window.document.querySelector('[data-field="status"]')?.textContent).toBe(
      "Story complete",
    );
    expect(dom.window.document.querySelector('[data-field="beat"]')?.textContent).toBe(
      "This static story path has ended.",
    );
    expect(
      dom.window.document.querySelectorAll('[data-field="choices"] button').length,
    ).toBe(0);
  });

  it("resumes saved local player progress from localStorage", () => {
    const dom = new JSDOM(indexHtml, {
      url: "http://127.0.0.1:4173/",
      pretendToBeVisual: true,
    });
    dom.window.localStorage.setItem(
      "plotforge:dynasty-embers:0.1.0:player-progress",
      JSON.stringify({
        project_id: "dynasty-embers",
        project_version: "0.1.0",
        scene_key: "court-crisis-001",
        beat_id: "court-crisis-001-beat-002",
      }),
    );

    const result = renderPlayer(sampleManifest(), dom.window.document);

    expect(result).toEqual({ sceneKey: "court-crisis-001", choiceCount: 1 });
    expect(dom.window.document.querySelector('[data-field="beat"]')?.textContent).toBe(
      "The war minister asks whether delay is now policy or merely fear.",
    );
    expect(
      dom.window.document.querySelector("[data-player-root]")?.dataset.saveState,
    ).toBe("ready");
  });

  it("fails explicitly for corrupted local player progress", () => {
    const dom = new JSDOM(indexHtml, {
      url: "http://127.0.0.1:4173/",
      pretendToBeVisual: true,
    });
    dom.window.localStorage.setItem(
      "plotforge:dynasty-embers:0.1.0:player-progress",
      "{not-json",
    );

    expect(() => renderPlayer(sampleManifest(), dom.window.document)).toThrow(
      "Invalid PlotForge player save: malformed JSON",
    );
  });

  it("fails explicitly when manifest entry scene is missing", () => {
    const dom = new JSDOM(indexHtml, {
      url: "http://127.0.0.1:4173/",
      pretendToBeVisual: true,
    });
    const manifest = {
      ...sampleManifest(),
      entry_scene: "missing-scene",
    };

    expect(() => renderPlayer(manifest, dom.window.document)).toThrow(
      "ExportManifest entry scene is missing: missing-scene",
    );
  });

  it("fails explicitly when manifest entry scene has no entry beat id", () => {
    const dom = new JSDOM(indexHtml, {
      url: "http://127.0.0.1:4173/",
      pretendToBeVisual: true,
    });
    const manifest = sampleManifest();
    const { entry_beat_id: _entryBeatId, ...entrySceneWithoutEntryBeat } =
      manifest.scenes[0];
    const invalidManifest = {
      ...manifest,
      scenes: [entrySceneWithoutEntryBeat, ...manifest.scenes.slice(1)],
    };

    expect(() => renderPlayer(invalidManifest, dom.window.document)).toThrow(
      "Scene is missing explicit entry_beat_id: court-crisis-001",
    );
  });

  it("fails explicitly when manifest entry beat id points to a missing beat", () => {
    const dom = new JSDOM(indexHtml, {
      url: "http://127.0.0.1:4173/",
      pretendToBeVisual: true,
    });
    const manifest = sampleManifest();
    const invalidManifest = {
      ...manifest,
      scenes: [
        {
          ...manifest.scenes[0],
          entry_beat_id: "missing-entry-beat",
        },
        ...manifest.scenes.slice(1),
      ],
    };

    expect(() => renderPlayer(invalidManifest, dom.window.document)).toThrow(
      "Scene entry_beat_id does not match any beat: court-crisis-001 -> missing-entry-beat",
    );
  });

  it("marks mobile viewport mode without changing manifest behavior", () => {
    const dom = new JSDOM(indexHtml, { pretendToBeVisual: true });
    Object.defineProperty(dom.window, "innerWidth", {
      configurable: true,
      value: 390,
    });

    renderPlayer(sampleManifest(), dom.window.document);

    expect(dom.window.document.querySelector("[data-player-root]")?.dataset.viewport).toBe(
      "mobile",
    );
    expect(
      dom.window.document.querySelector("[data-player-root]")?.dataset.saveState,
    ).toBe("unavailable");
    expect(
      dom.window.document.querySelectorAll('[data-field="choices"] button').length,
    ).toBe(3);
  });

  it("keeps player readable and clickable when no audio refs exist", () => {
    const dom = new JSDOM(indexHtml, {
      url: "http://127.0.0.1:4173/",
      pretendToBeVisual: true,
    });

    renderPlayer(sampleManifest(), dom.window.document);

    expect(dom.window.document.querySelector("[data-player-root]")?.dataset.audioState).toBe(
      "none",
    );
    expect(dom.window.document.querySelector('[data-field="scene-audio"]')).toBeNull();

    const continueChoice = dom.window.document.querySelector(
      '[data-choice-id="continue-council"]',
    );
    if (!continueChoice) {
      throw new Error("expected continue choice button");
    }
    continueChoice.click();

    expect(dom.window.document.querySelector('[data-field="beat"]')?.textContent).toBe(
      "The war minister asks whether delay is now policy or merely fear.",
    );
  });

  it("lazy loads local audio refs only for the active scene and beat", () => {
    const dom = new JSDOM(indexHtml, {
      url: "http://127.0.0.1:4173/",
      pretendToBeVisual: true,
    });
    const manifest = sampleManifest();
    manifest.scenes[0].audio_refs = [
      {
        asset_id: "asset-audio-court-theme",
        kind: "audio",
        source: "generated",
        project_path: "assets/audio/court-theme.ogg",
        export_path: "assets/audio/court-theme.ogg",
        slot: "scene_audio",
      },
    ];
    manifest.scenes[0].beats[1].audio_refs = [
      {
        asset_id: "asset-audio-court-beat-002",
        kind: "audio",
        source: "generated",
        project_path: "assets/audio/court-beat-002.ogg",
        export_path: "assets/audio/court-beat-002.ogg",
        slot: "narration",
      },
    ];
    manifest.asset_records.push({
      kind: "audio",
      id: "asset-audio-court-theme",
      source: "generated",
      project_path: "assets/audio/court-theme.ogg",
      export_path: "assets/audio/court-theme.ogg",
      content_hash: "0".repeat(64),
      hash_algorithm: "sha256",
      byte_length: 12,
      references: [],
    });
    manifest.asset_records.push({
      kind: "audio",
      id: "asset-audio-court-beat-002",
      source: "generated",
      project_path: "assets/audio/court-beat-002.ogg",
      export_path: "assets/audio/court-beat-002.ogg",
      content_hash: "1".repeat(64),
      hash_algorithm: "sha256",
      byte_length: 12,
      references: [],
    });
    manifest.assets.push("assets/audio/court-theme.ogg");
    manifest.assets.push("assets/audio/court-beat-002.ogg");

    renderPlayer(manifest, dom.window.document);

    const mount = dom.window.document.querySelector("[data-player-root]");
    const audio = dom.window.document.querySelector('[data-field="scene-audio"]');
    expect(audio).not.toBeNull();
    expect(audio?.getAttribute("preload")).toBe("none");
    expect(audio?.hasAttribute("autoplay")).toBe(false);
    expect(audio?.getAttribute("src")).toBe("assets/audio/court-theme.ogg");
    expect(mount?.dataset.audioSrc).toBe("assets/audio/court-theme.ogg");

    const continueChoice = dom.window.document.querySelector(
      '[data-choice-id="continue-council"]',
    );
    if (!continueChoice) {
      throw new Error("expected continue choice button");
    }
    continueChoice.click();

    const updatedAudio = dom.window.document.querySelector('[data-field="scene-audio"]');
    expect(updatedAudio?.getAttribute("src")).toBe("assets/audio/court-beat-002.ogg");
    expect(mount?.dataset.audioSrc).toBe("assets/audio/court-beat-002.ogg");
  });

  it("does not load audio refs missing from exported asset records", () => {
    const dom = new JSDOM(indexHtml, {
      url: "http://127.0.0.1:4173/",
      pretendToBeVisual: true,
    });
    const manifest = sampleManifest();
    manifest.scenes[0].beats[0].audio_refs = [
      {
        asset_id: "missing-audio",
        kind: "audio",
        source: "generated",
        project_path: "assets/audio/missing.ogg",
        export_path: "assets/audio/missing.ogg",
        slot: "narration",
      },
    ];

    renderPlayer(manifest, dom.window.document);

    expect(dom.window.document.querySelector("[data-player-root]")?.dataset.audioState).toBe(
      "missing-asset",
    );
    expect(dom.window.document.querySelector('[data-field="scene-audio"]')).toBeNull();

    const continueChoice = dom.window.document.querySelector(
      '[data-choice-id="continue-council"]',
    );
    if (!continueChoice) {
      throw new Error("expected continue choice button");
    }
    continueChoice.click();

    expect(dom.window.document.querySelector("[data-player-root]")?.dataset.currentBeatId).toBe(
      "court-crisis-001-beat-002",
    );
  });

  it("ignores non-schema audio fields instead of guessing local paths", () => {
    const dom = new JSDOM(indexHtml, {
      url: "http://127.0.0.1:4173/",
      pretendToBeVisual: true,
    });
    const manifest = sampleManifest();
    manifest.scenes[0].audio_ref = "assets/audio/legacy-scene.ogg";
    manifest.scenes[0].beats[0].audio_asset = "assets/audio/legacy-beat.ogg";
    manifest.assets.push("assets/audio/legacy-scene.ogg");
    manifest.assets.push("assets/audio/legacy-beat.ogg");

    renderPlayer(manifest, dom.window.document);

    expect(dom.window.document.querySelector("[data-player-root]")?.dataset.audioState).toBe(
      "none",
    );
    expect(dom.window.document.querySelector('[data-field="scene-audio"]')).toBeNull();
  });

  it("renders a friendly error state when the manifest fails to load", async () => {
    const dom = new JSDOM(indexHtml, {
      url: "http://127.0.0.1:4173/",
      pretendToBeVisual: true,
    });
    const fetchManifest = () =>
      Promise.reject(new Error("Unable to load game.json: 404"));

    await bootPlayer({
      root: dom.window.document,
      locale: "en",
      fetchManifest,
    });

    const doc = dom.window.document;
    expect(doc.querySelector("[data-player-root]")?.dataset.state).toBe(
      "error",
    );
    expect(doc.querySelector('[data-field="status"]')?.textContent).toBe(
      "Export failed to load",
    );
    expect(doc.querySelector('[data-field="scene-title"]')?.textContent).toBe(
      "Unable to load PlotForge export",
    );
    expect(doc.querySelector('[data-field="beat"]')?.textContent).toBe(
      "Unable to load game.json: 404",
    );
    // Authored chrome cleared so no stale "Loading" text or broken image remains.
    expect(doc.querySelector('[data-field="game-title"]')?.textContent).toBe("");
    expect(doc.querySelector('[data-field="hook"]')?.textContent).toBe("");
    expect(doc.querySelector('[data-field="outcome"]')?.textContent).toBe("");
    expect(
      doc.querySelectorAll('[data-field="choices"] button').length,
    ).toBe(0);
    expect(
      doc.querySelector('[data-field="scene-image"]')?.hasAttribute("src"),
    ).toBe(false);
  });

  it("renders a friendly localized error state in Chinese", async () => {
    const dom = new JSDOM(indexHtml, {
      url: "http://127.0.0.1:4173/",
      pretendToBeVisual: true,
    });
    const fetchManifest = () =>
      Promise.reject(new Error("Unable to load game.json: 404"));

    await bootPlayer({
      root: dom.window.document,
      locale: "zh",
      fetchManifest,
    });

    const doc = dom.window.document;
    expect(doc.querySelector('[data-field="status"]')?.textContent).toBe(
      "导出加载失败",
    );
    expect(doc.querySelector('[data-field="scene-title"]')?.textContent).toBe(
      "无法加载 PlotForge 导出",
    );
    expect(doc.documentElement.lang).toBe("zh-CN");
  });

  it("ships static player files without external network URLs", () => {
    expect(indexHtml).toContain('name="viewport"');
    expect(indexHtml).toContain('src="./player.js"');
    expect(indexHtml).toContain('data-field="language-select"');
    for (const file of playerSourceFiles) {
      expect(file).not.toMatch(/https?:\/\//);
      expect(file).not.toMatch(/\/\/cdn\.|\/\/unpkg\.|\/\/fonts\./);
    }
  });
});

describe("player-save module", () => {
  it("scopes the save key to the game id and version", () => {
    const dom = newJSDOM();
    const store = createSaveStore(dom.window.document, sampleManifest());

    expect(store.key).toBe("plotforge:dynasty-embers:0.1.0:player-progress");
    expect(store.available).toBe(true);
  });

  it("round-trips a scene/beat position through localStorage", () => {
    const dom = newJSDOM();
    const store = createSaveStore(dom.window.document, sampleManifest());

    store.save({ sceneKey: "tax-riot-002", beatId: "tax-riot-002-beat-001" });
    expect(store.load()).toEqual({
      sceneKey: "tax-riot-002",
      beatId: "tax-riot-002-beat-001",
    });
  });

  it("returns null when no save exists", () => {
    const dom = newJSDOM();
    const store = createSaveStore(dom.window.document, sampleManifest());

    expect(store.load()).toBeNull();
  });

  it("rejects saves written for a different project or version", () => {
    const dom = newJSDOM();
    const store = createSaveStore(dom.window.document, sampleManifest());
    dom.window.localStorage.setItem(
      "plotforge:dynasty-embers:0.1.0:player-progress",
      JSON.stringify({
        project_id: "other-game",
        project_version: "0.1.0",
        scene_key: "court-crisis-001",
        beat_id: "court-crisis-001-beat-001",
      }),
    );

    expect(() => store.load()).toThrow(
      "Invalid PlotForge player save: project mismatch",
    );
  });

  it("reports the store unavailable when localStorage is absent", () => {
    const dom = newJSDOM();
    // Drop localStorage so the store degrades gracefully.
    Object.defineProperty(dom.window, "localStorage", {
      configurable: true,
      get() {
        throw new Error("localStorage blocked");
      },
    });

    const store = createSaveStore(dom.window.document, sampleManifest());
    expect(store.available).toBe(false);
    expect(store.load()).toBeNull();
    expect(() => store.save({ sceneKey: "x", beatId: null })).not.toThrow();
  });
});

describe("player-i18n module", () => {
  it("translates chrome keys in English and Chinese", () => {
    const dom = newJSDOM();
    const en = createPlayerI18n(dom.window.document, "en");
    const zh = createPlayerI18n(dom.window.document, "zh");

    expect(en.t("storyComplete")).toBe("Story complete");
    expect(zh.t("storyComplete")).toBe("故事完成");
    expect(en.t("sceneProgress", { current: 1, total: 2 })).toBe("Scene 1 of 2");
    expect(zh.t("sceneProgress", { current: 1, total: 2 })).toBe("场景 1 / 2");
  });

  it("falls back to English for unknown keys", () => {
    const dom = newJSDOM();
    const zh = createPlayerI18n(dom.window.document, "zh");
    expect(zh.t("not-a-real-key")).toBeUndefined();
  });

  it("resolves the locale from the URL ?lang= parameter", () => {
    const dom = new JSDOM(indexHtml, {
      url: "http://127.0.0.1:4173/?lang=zh",
      pretendToBeVisual: true,
    });
    expect(createPlayerI18n(dom.window.document).locale).toBe("zh");
  });

  it("persists the chosen locale and reflects it on <html lang>", () => {
    const dom = newJSDOM();
    const mount = dom.window.document.querySelector("[data-player-root]");
    applyLocale(dom.window.document, mount, "zh");
    expect(mount.dataset.locale).toBe("zh");
    expect(dom.window.document.documentElement.lang).toBe("zh-CN");
  });
});

describe("player-audio module", () => {
  it("renders a lazy audio element for a ready, package-local audio ref", () => {
    const dom = newJSDOM();
    const manifest = manifestWithAudio();
    const mount = dom.window.document.querySelector("[data-player-root]");
    const ui = createPlayerI18n(dom.window.document, "en");
    const scene = manifest.scenes[0];
    const beat = scene.beats[0];

    renderAudio(manifest, scene, beat, dom.window.document, mount, ui);

    const audio = dom.window.document.querySelector('[data-field="scene-audio"]');
    expect(audio).not.toBeNull();
    expect(audio.getAttribute("preload")).toBe("none");
    expect(audio.hasAttribute("autoplay")).toBe(false);
    expect(audio.getAttribute("src")).toBe("assets/audio/court-theme.ogg");
    expect(mount.dataset.audioState).toBe("ready");
    expect(mount.dataset.audioSrc).toBe("assets/audio/court-theme.ogg");
  });

  it("reports missing-asset when the audio ref is absent from asset records", () => {
    const dom = newJSDOM();
    const manifest = sampleManifest();
    manifest.scenes[0].beats[0].audio_refs = [
      {
        asset_id: "missing-audio",
        kind: "audio",
        source: "generated",
        project_path: "assets/audio/missing.ogg",
        export_path: "assets/audio/missing.ogg",
        slot: "narration",
      },
    ];
    const mount = dom.window.document.querySelector("[data-player-root]");
    const ui = createPlayerI18n(dom.window.document, "en");

    renderAudio(manifest, manifest.scenes[0], manifest.scenes[0].beats[0], dom.window.document, mount, ui);

    expect(mount.dataset.audioState).toBe("missing-asset");
    expect(dom.window.document.querySelector('[data-field="scene-audio"]')).toBeNull();
  });

  it("blocks external protocol-relative audio paths", () => {
    const dom = newJSDOM();
    const manifest = sampleManifest();
    manifest.scenes[0].beats[0].audio_refs = [
      {
        asset_id: "external-audio",
        kind: "audio",
        source: "external",
        project_path: "//cdn.example/track.ogg",
        export_path: "//cdn.example/track.ogg",
        slot: "narration",
      },
    ];
    manifest.asset_records.push({
      kind: "audio",
      id: "external-audio",
      source: "external",
      project_path: "//cdn.example/track.ogg",
      export_path: "//cdn.example/track.ogg",
      content_hash: "2".repeat(64),
      hash_algorithm: "sha256",
      byte_length: 0,
      references: [],
    });
    manifest.assets.push("//cdn.example/track.ogg");
    const mount = dom.window.document.querySelector("[data-player-root]");
    const ui = createPlayerI18n(dom.window.document, "en");

    renderAudio(manifest, manifest.scenes[0], manifest.scenes[0].beats[0], dom.window.document, mount, ui);

    expect(mount.dataset.audioState).toBe("missing-asset");
    expect(dom.window.document.querySelector('[data-field="scene-audio"]')).toBeNull();
  });

  it("clears the panel and hides it when no audio refs exist", () => {
    const dom = newJSDOM();
    const manifest = sampleManifest();
    const mount = dom.window.document.querySelector("[data-player-root]");
    const ui = createPlayerI18n(dom.window.document, "en");

    renderAudio(manifest, manifest.scenes[0], manifest.scenes[0].beats[0], dom.window.document, mount, ui);

    expect(mount.dataset.audioState).toBe("none");
    const panel = dom.window.document.querySelector('[data-field="audio-panel"]');
    expect(panel.hidden).toBe(true);
    expect(panel.children.length).toBe(0);
  });
});

function sampleManifest() {
  return {
    game: {
      id: "dynasty-embers",
      title: "Dynasty Embers",
      version: "0.1.0",
      description: "A court crisis demo.",
      entry_scene: "court-crisis-001",
      run_seed: 7,
    },
    entry_scene: "court-crisis-001",
    scenes: [
      {
        key: "court-crisis-001",
        title: "The Red Deficit Ledger",
        location: "Qianqing Palace",
        dramatic_purpose: "Force a tradeoff.",
        hook: "The ledger arrives with a fresh red deficit mark.",
        background_asset: "assets/generated/court-crisis-001.png",
        character_ids: [],
        plot_thread_updates: {},
        entry_beat_id: "court-crisis-001-beat-001",
        beats: [
          {
            id: "court-crisis-001-beat-001",
            text: "The court waits for an order.",
            choices: [
              {
                id: "continue-council",
                label: "Hear one more minister",
                action_type: "continue",
                input_terms: ["continue", "hear", "minister"],
                dramatic_purpose: "Stay in the council scene.",
                change_scene: false,
              },
              {
                id: "raise-tax",
                label: "Raise taxes",
                action_type: "raise_tax",
                input_terms: ["raise", "tax", "levy"],
                dramatic_purpose: "Raise emergency funds while damaging public order.",
                change_scene: true,
              },
              {
                id: "pay-army",
                label: "Pay the army",
                action_type: "pay_army",
                input_terms: ["pay", "army"],
                dramatic_purpose: "Spend scarce treasury to buy military time.",
                change_scene: true,
              },
            ],
            next: { kind: "beat", payload: "court-crisis-001-beat-002" },
          },
          {
            id: "court-crisis-001-beat-002",
            text: "The war minister asks whether delay is now policy or merely fear.",
            choices: [
              {
                id: "raise-tax",
                label: "Raise taxes",
                action_type: "raise_tax",
                input_terms: ["raise", "tax", "levy"],
                dramatic_purpose: "Raise emergency funds while damaging public order.",
                change_scene: true,
              },
            ],
            next: { kind: "scene" },
          },
        ],
      },
      {
        key: "tax-riot-002",
        title: "Provincial Tax Riot",
        location: "Shandong courier yard",
        dramatic_purpose: "Show the first consequence of emergency taxation.",
        hook: "A courier arrives with ash on his sleeves and a tax seal split in half.",
        background_asset: "assets/generated/tax-riot-002.png",
        character_ids: [],
        plot_thread_updates: {},
        entry_beat_id: "tax-riot-002-beat-001",
        beats: [
          {
            id: "tax-riot-002-beat-001",
            text: "Smoke from the eastern counties turns the levy into a legitimacy crisis.",
            choices: [
              {
                id: "hold-line",
                label: "Hold the levy line",
                action_type: "hold_line",
                input_terms: ["hold", "line"],
                dramatic_purpose: "End the static sample path with a hard public-order cost.",
                change_scene: true,
              },
            ],
            next: { kind: "end" },
          },
        ],
      },
    ],
    assets: [
      "assets/generated/court-crisis-001.png",
      "assets/generated/tax-riot-002.png",
    ],
    asset_records: [],
    generated_by: "plotforge-export 0.1.0",
  };
}

function readStaticFile(fileName) {
  return readFileSync(resolve(process.cwd(), "static", fileName), "utf8");
}

function newJSDOM() {
  return new JSDOM(indexHtml, {
    url: "http://127.0.0.1:4173/",
    pretendToBeVisual: true,
  });
}

function manifestWithAudio() {
  const manifest = sampleManifest();
  manifest.scenes[0].beats[0].audio_refs = [
    {
      asset_id: "asset-audio-court-theme",
      kind: "audio",
      source: "generated",
      project_path: "assets/audio/court-theme.ogg",
      export_path: "assets/audio/court-theme.ogg",
      slot: "scene_audio",
    },
  ];
  manifest.asset_records.push({
    kind: "audio",
    id: "asset-audio-court-theme",
    source: "generated",
    project_path: "assets/audio/court-theme.ogg",
    export_path: "assets/audio/court-theme.ogg",
    content_hash: "0".repeat(64),
    hash_algorithm: "sha256",
    byte_length: 12,
    references: [],
  });
  manifest.assets.push("assets/audio/court-theme.ogg");
  return manifest;
}
