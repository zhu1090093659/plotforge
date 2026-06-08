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
    const firstChoice = dom.window.document.querySelector("button");
    if (!firstChoice) {
      throw new Error("expected choice button");
    }
    firstChoice.click();

    expect(result).toEqual({ sceneKey: "court-crisis-001", choiceCount: 2 });
    expect(dom.window.document.title).toBe("Dynasty Embers");
    expect(dom.window.document.querySelector("[data-player-root]")?.dataset.state).toBe(
      "ready",
    );
    expect(
      dom.window.document.querySelector('[data-field="scene-title"]')?.textContent,
    ).toBe("The Red Deficit Ledger");
    expect(dom.window.document.querySelector('[data-field="beat"]')?.textContent).toBe(
      "Raise emergency funds while damaging public order.",
    );
    expect(firstChoice.getAttribute("aria-pressed")).toBe("true");
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
      dom.window.document.querySelectorAll('[data-field="choices"] button').length,
    ).toBe(2);
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
        beats: [
          {
            id: "court-crisis-001-beat-001",
            text: "The court waits for an order.",
            choices: [
              {
                id: "raise-tax",
                label: "Raise taxes",
                action_type: "raise_tax",
                dramatic_purpose: "Raise emergency funds while damaging public order.",
                change_scene: true,
              },
              {
                id: "pay-army",
                label: "Pay the army",
                action_type: "pay_army",
                dramatic_purpose: "Spend scarce treasury to buy military time.",
                change_scene: true,
              },
            ],
          },
        ],
      },
    ],
    assets: ["assets/generated/court-crisis-001.png"],
    generated_by: "plotforge-export 0.1.0",
  };
}

function readStaticFile(fileName) {
  return readFileSync(resolve(process.cwd(), "static", fileName), "utf8");
}
