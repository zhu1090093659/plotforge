import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { JSDOM } from "jsdom";
import { describe, expect, it } from "vitest";
import { renderPlayer } from "../static/player-core.js";

const indexHtml = readStaticFile("index.html");
const playerJs = readStaticFile("player.js");
const playerCoreJs = readStaticFile("player-core.js");
const stylesCss = readStaticFile("styles.css");

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

  it("ships static player files without external network URLs", () => {
    expect(indexHtml).toContain('name="viewport"');
    expect(indexHtml).toContain('src="./player.js"');
    for (const file of [indexHtml, playerJs, playerCoreJs, stylesCss]) {
      expect(file).not.toMatch(/https?:\/\//);
      expect(file).not.toMatch(/\/\/cdn\.|\/\/unpkg\.|\/\/fonts\./);
    }
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
    generated_by: "plotforge-export 0.1.0",
  };
}

function readStaticFile(fileName) {
  return readFileSync(resolve(process.cwd(), "static", fileName), "utf8");
}
