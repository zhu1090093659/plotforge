import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AssetMaintenanceView } from "./AssetMaintenanceView";
import type { AssetCatalog } from "./studioModel";
import type { SourceFileSummary } from "./tauriBridge";

afterEach(() => {
  cleanup();
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const emptyAssetCatalog: AssetCatalog = {
  source: "records",
  items: [],
};

const sourceFiles: SourceFileSummary[] = [
  { path: "game.toml", kind: "toml", bytes: 120, editable: false },
  { path: "world/world.md", kind: "markdown", bytes: 80, editable: true },
];

const visualBible = {
  style_cards: [
    {
      id: "style-court-001",
      title: "Winter court ink wash",
      summary: "Ink wash courtyard style.",
      prompt: "Ink wash courtyard with winter lanterns",
      palette: ["bone white", "ink black"],
      tags: ["court", "winter"],
      reference_asset_ids: ["asset-style-ref-001"],
    },
  ],
};

const audioBible = {
  voice_cards: [
    {
      id: "voice-censor-001",
      title: "Court Censor",
      summary: "Dry formal court voice.",
      voice: "dry formal court voice",
      delivery: "quiet but cutting",
      sample_text: "The ledgers do not accuse by accident.",
      tags: ["court", "formal"],
      reference_asset_ids: ["asset-voice-ref-001"],
    },
  ],
};

function renderView(
  overrides: Partial<Parameters<typeof AssetMaintenanceView>[0]> = {},
) {
  const defaults = {
    assetCatalog: emptyAssetCatalog,
    sourceFiles,
    visualBible,
    audioBible,
    saving: false,
    formStatus: null,
    onUpdateVisualStyleCard: vi.fn(),
    onUpdateAudioVoiceCard: vi.fn(),
    onSaveVisualBible: vi.fn(),
    onSaveAudioBible: vi.fn(),
  };
  render(<AssetMaintenanceView {...defaults} {...overrides} />);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("AssetMaintenanceView", () => {
  it("renders the Asset Maintenance heading with record count", () => {
    renderView();
    expect(screen.getByRole("heading", { name: "Asset Maintenance" })).toBeTruthy();
    expect(screen.getByText("Asset Maintenance")).toBeTruthy();
    expect(screen.getByText("0 asset records")).toBeTruthy();
  });

  it("shows scene background fallback count when source is scene-background-fallback", () => {
    const assetCatalog: AssetCatalog = {
      source: "scene-background-fallback",
      items: [
        { source: "scene-background-fallback", path: "assets/bg-001.png" },
        { source: "scene-background-fallback", path: "assets/bg-002.png" },
      ],
    };
    renderView({ assetCatalog });
    expect(screen.getByText("2 scene background fallbacks")).toBeTruthy();
    expect(screen.getAllByText("Scene background fallback").length).toBeGreaterThan(0);
  });

  it("renders metrics: source files, visual cards, audio cards", () => {
    renderView();
    expect(screen.getByText("Source files")).toBeTruthy();
    expect(screen.getByText("2")).toBeTruthy();
    expect(screen.getByText("Visual cards")).toBeTruthy();
    expect(screen.getByText("Audio cards")).toBeTruthy();
  });

  it("renders empty panel when no assets", () => {
    renderView({ assetCatalog: emptyAssetCatalog });
    expect(
      screen.getByText("No asset records or scene background paths found."),
    ).toBeTruthy();
  });

  it("renders Visual Bible section with style card title as collapsible label", () => {
    renderView();
    expect(screen.getByText("Visual Bible")).toBeTruthy();
    expect(screen.getByRole("button", { name: /Winter court ink wash/ })).toBeTruthy();
  });

  it("renders Audio Bible section with voice card title as collapsible label", () => {
    renderView();
    expect(screen.getByText("Audio Bible")).toBeTruthy();
    expect(screen.getByRole("button", { name: /Court Censor/ })).toBeTruthy();
  });

  it("expands Visual Bible card and shows editable prompt field", () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Winter court ink wash/ }));
    expect(screen.getByLabelText("Visual style prompt 1")).toBeTruthy();
    expect(
      (screen.getByLabelText("Visual style prompt 1") as HTMLTextAreaElement).value,
    ).toBe("Ink wash courtyard with winter lanterns");
  });

  it("expands Audio Bible card and shows editable voice field", () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Court Censor/ }));
    expect(screen.getByLabelText("Audio voice 1")).toBeTruthy();
    expect(
      (screen.getByLabelText("Audio voice 1") as HTMLInputElement).value,
    ).toBe("dry formal court voice");
  });

  it("calls onUpdateVisualStyleCard when prompt field changes", () => {
    const onUpdate = vi.fn();
    renderView({ onUpdateVisualStyleCard: onUpdate });
    fireEvent.click(screen.getByRole("button", { name: /Winter court ink wash/ }));
    fireEvent.change(screen.getByLabelText("Visual style prompt 1"), {
      target: { value: "New prompt value" },
    });
    expect(onUpdate).toHaveBeenCalledWith(0, { prompt: "New prompt value" });
  });

  it("calls onUpdateAudioVoiceCard when voice field changes", () => {
    const onUpdate = vi.fn();
    renderView({ onUpdateAudioVoiceCard: onUpdate });
    fireEvent.click(screen.getByRole("button", { name: /Court Censor/ }));
    fireEvent.change(screen.getByLabelText("Audio voice 1"), {
      target: { value: "deeper resonant voice" },
    });
    expect(onUpdate).toHaveBeenCalledWith(0, { voice: "deeper resonant voice" });
  });

  it("calls onSaveVisualBible when Save Visual Bible is clicked", async () => {
    const onSave = vi.fn();
    renderView({ onSaveVisualBible: onSave });
    fireEvent.click(screen.getByRole("button", { name: "Save Visual Bible" }));
    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });
  });

  it("calls onSaveAudioBible when Save Audio Bible is clicked", async () => {
    const onSave = vi.fn();
    renderView({ onSaveAudioBible: onSave });
    fireEvent.click(screen.getByRole("button", { name: "Save Audio Bible" }));
    await waitFor(() => {
      expect(onSave).toHaveBeenCalledTimes(1);
    });
  });

  it("shows empty message when Visual Bible has no style cards", () => {
    renderView({ visualBible: { style_cards: [] } });
    expect(
      screen.getByText("No Visual Bible style cards in project data."),
    ).toBeTruthy();
  });

  it("shows empty message when Audio Bible has no voice cards", () => {
    renderView({ audioBible: { voice_cards: [] } });
    expect(
      screen.getByText("No Audio Bible voice cards in project data."),
    ).toBeTruthy();
  });

  it("disables save buttons while saving is true", () => {
    renderView({ saving: true });
    expect(
      screen.getByRole("button", { name: "Save Visual Bible" }).hasAttribute("disabled"),
    ).toBe(true);
    expect(
      screen.getByRole("button", { name: "Save Audio Bible" }).hasAttribute("disabled"),
    ).toBe(true);
  });

  it("shows formStatus success message when section is assets", () => {
    renderView({
      formStatus: {
        section: "assets",
        tone: "success",
        message: "Visual Bible saved.",
      },
    });
    expect(screen.getByText("Visual Bible saved.")).toBeTruthy();
  });

  it("does not show formStatus for a different section", () => {
    renderView({
      formStatus: {
        section: "world",
        tone: "success",
        message: "World Bible saved.",
      },
    });
    expect(screen.queryByText("World Bible saved.")).toBeNull();
  });

  it("renders asset record card with fallback badge", () => {
    const assetCatalog: AssetCatalog = {
      source: "records",
      items: [
        {
          source: "record",
          record: {
            id: "asset-voice-censor-001",
            kind: "voice",
            source: "generated",
            project_path: "assets/censor.wav",
            export_path: "assets/censor.wav",
            content_hash: "sha256:abc123",
            hash_algorithm: "sha256",
            byte_length: 2048,
            references: [],
            provider_metadata: {
              provider: "mock",
              model: "mock-v1",
              request_id: "req-001",
              prompt_hash: "sha256:prompt001",
              fallback_used: true,
            },
          },
        },
      ],
    };
    renderView({ assetCatalog });
    expect(screen.getByText("asset-voice-censor-001")).toBeTruthy();
    expect(screen.getByText("Fallback")).toBeTruthy();
    expect(screen.getByText("mock / mock-v1")).toBeTruthy();
  });
});
