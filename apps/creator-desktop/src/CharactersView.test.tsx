import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { CharacterDraft, CharacterEditDocument } from "../../../contracts/plotforge";
import { CharactersView, type CharactersViewProps } from "./CharactersView";
import { StudioI18nProvider } from "./i18n";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function sampleDocument(): CharacterEditDocument {
  return {
    characters: [
      {
        id: "council-envoy",
        name: "Council Envoy",
        role: "Diplomatic liaison",
        traits: ["observant", "cautious"],
        visual_card: "ink portrait with council robes",
        voice_card: "measured formal speech",
        portrait_request: null,
      },
      {
        id: "border-guard",
        name: "Border Guard",
        role: "Northern frontier sentinel",
        traits: ["vigilant"],
        visual_card: "rough field armor",
        voice_card: "clipped military tone",
        portrait_request: {
          prompt_summary: "Generated portrait request",
          style: "ink wash",
          target_asset_slot: "portrait",
          prompt_hash: "sha256:abc",
          provider_config_hash: "sha256:cfg",
          reference_asset_ids: [],
          fallback_allowed: false,
        },
      },
    ],
  };
}

function emptyDraft(): CharacterDraft {
  return {
    id: "",
    name: "",
    role: "",
    traits_text: "",
    visual_card: "",
    voice_card: "",
  };
}

function renderView(overrides: Partial<CharactersViewProps> = {}) {
  const props: CharactersViewProps = {
    characterEditDocument: sampleDocument(),
    saving: false,
    formStatus: null,
    characterGenerationConcept: "Design a pressure-bearing character.",
    onCharacterGenerationConceptChange: vi.fn(),
    characterGenerationRoleHint: "Council Envoy",
    onCharacterGenerationRoleHintChange: vi.fn(),
    characterDraft: emptyDraft(),
    onCharacterDraftChange: vi.fn(),
    onSave: vi.fn(),
    onGenerateCharacter: vi.fn(),
    onCreateCharacterFromDraft: vi.fn(),
    onUpdateCharacter: vi.fn(),
    ...overrides,
  };
  return render(<CharactersView {...props} />, {
    wrapper: StudioI18nProvider,
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("CharactersView", () => {
  afterEach(cleanup);
  it("shows the character count subtitle", () => {
    renderView();
    expect(screen.getByText("2 records")).toBeTruthy();
  });

  it("renders collapsed character cards with name as label", () => {
    renderView();
    // Use getAllByRole to handle potential multiple matches and verify count.
    const councilEnvoyBtns = screen.getAllByRole("button", { name: /Council Envoy/ });
    expect(councilEnvoyBtns.length).toBeGreaterThanOrEqual(1);
    const borderGuardBtns = screen.getAllByRole("button", { name: /Border Guard/ });
    expect(borderGuardBtns.length).toBeGreaterThanOrEqual(1);
  });

  it("character card is collapsed by default — detail fields hidden", () => {
    renderView();
    // Character id input should not be in DOM while collapsed.
    expect(screen.queryByLabelText("Character id 1")).toBeNull();
  });

  it("expanding a character card reveals detail fields", () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Council Envoy/ }));
    expect(screen.getByLabelText("Character id 1")).toBeTruthy();
    expect(screen.getByLabelText("Character name 1")).toBeTruthy();
    expect(screen.getByLabelText("Character role 1")).toBeTruthy();
    expect(screen.getByLabelText("Character traits 1")).toBeTruthy();
    expect(screen.getByLabelText("Visual card 1")).toBeTruthy();
    expect(screen.getByLabelText("Voice card 1")).toBeTruthy();
  });

  it("expanded card shows visual/voice card placeholders", () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Council Envoy/ }));
    const visualCard = screen.getByLabelText("Visual card 1") as HTMLTextAreaElement;
    expect(visualCard.placeholder).toContain("appearance");
    const voiceCard = screen.getByLabelText("Voice card 1") as HTMLTextAreaElement;
    expect(voiceCard.placeholder).toContain("speech");
  });

  it("portrait_request details visible when card with portrait is expanded", () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Border Guard/ }));
    expect(screen.getByText("Portrait request")).toBeTruthy();
    expect(screen.getByText("Generated portrait request")).toBeTruthy();
  });

  it("calls onUpdateCharacter when a field value changes", () => {
    const onUpdateCharacter = vi.fn();
    renderView({ onUpdateCharacter });
    fireEvent.click(screen.getByRole("button", { name: /Council Envoy/ }));
    fireEvent.change(screen.getByLabelText("Character name 1"), {
      target: { value: "Senior Envoy" },
    });
    expect(onUpdateCharacter).toHaveBeenCalledWith(0, { name: "Senior Envoy" });
  });

  it("Add Character collapsible is collapsed by default", () => {
    renderView();
    // Form inputs inside the collapsible should not be visible.
    expect(screen.queryByLabelText("New character id")).toBeNull();
  });

  it("Add Character collapsible opens on click (unified entry point)", () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: "Add Character" }));
    // Manual mode is the default — manual form fields should appear.
    expect(screen.getByLabelText("New character id")).toBeTruthy();
    expect(screen.getByLabelText("New character name")).toBeTruthy();
    expect(screen.getByLabelText("New character role")).toBeTruthy();
    expect(screen.getByLabelText("New character traits")).toBeTruthy();
  });

  it("switching to AI Generate mode shows concept and role-hint fields", () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: "Add Character" }));
    fireEvent.click(screen.getByRole("button", { name: "AI Generate", pressed: false }));
    expect(screen.getByLabelText("AI character concept")).toBeTruthy();
    expect(screen.getByLabelText("Character generation role hint")).toBeTruthy();
    // Manual fields should be gone.
    expect(screen.queryByLabelText("New character id")).toBeNull();
  });

  it("switching back to Manual mode restores manual fields", () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: "Add Character" }));
    fireEvent.click(screen.getByRole("button", { name: "AI Generate", pressed: false }));
    fireEvent.click(screen.getByRole("button", { name: "Manual", pressed: false }));
    expect(screen.getByLabelText("New character id")).toBeTruthy();
    expect(screen.queryByLabelText("AI character concept")).toBeNull();
  });

  it("calls onCreateCharacterFromDraft when Create Character is clicked", () => {
    const onCreateCharacterFromDraft = vi.fn();
    renderView({ onCreateCharacterFromDraft });
    fireEvent.click(screen.getByRole("button", { name: "Add Character" }));
    fireEvent.click(screen.getByRole("button", { name: "Create Character" }));
    expect(onCreateCharacterFromDraft).toHaveBeenCalledOnce();
  });

  it("calls onGenerateCharacter when Generate Character is clicked in AI mode", () => {
    const onGenerateCharacter = vi.fn();
    renderView({ onGenerateCharacter });
    fireEvent.click(screen.getByRole("button", { name: "Add Character" }));
    fireEvent.click(screen.getByRole("button", { name: "AI Generate", pressed: false }));
    fireEvent.click(screen.getByRole("button", { name: "Generate Character" }));
    expect(onGenerateCharacter).toHaveBeenCalledOnce();
  });

  it("calls onSave when Save Characters button is clicked", () => {
    const onSave = vi.fn();
    renderView({ onSave });
    fireEvent.click(screen.getByRole("button", { name: "Save Characters" }));
    expect(onSave).toHaveBeenCalledOnce();
  });

  it("Save Characters button is disabled while saving", () => {
    renderView({ saving: true });
    const saveBtn = screen.getByRole("button", { name: "Save Characters" }) as HTMLButtonElement;
    expect(saveBtn.disabled).toBe(true);
  });

  it("shows success form status message", () => {
    renderView({
      formStatus: { section: "characters", tone: "success", message: "Characters saved." },
    });
    expect(screen.getByText("Characters saved.")).toBeTruthy();
  });

  it("does not show form status from a different section", () => {
    renderView({
      formStatus: { section: "world", tone: "success", message: "World saved." },
    });
    expect(screen.queryByText("World saved.")).toBeNull();
  });

  it("shows empty state when characterEditDocument is null", () => {
    renderView({ characterEditDocument: null });
    expect(screen.getByText("Character edit document not loaded.")).toBeTruthy();
  });

  it("manual draft form calls onCharacterDraftChange when id changes", () => {
    const onCharacterDraftChange = vi.fn();
    renderView({ onCharacterDraftChange });
    fireEvent.click(screen.getByRole("button", { name: "Add Character" }));
    fireEvent.change(screen.getByLabelText("New character id"), {
      target: { value: "regent" },
    });
    expect(onCharacterDraftChange).toHaveBeenCalledWith(
      expect.objectContaining({ id: "regent" }),
    );
  });

  it("manual form shows placeholder hints on visual/voice card fields", () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: "Add Character" }));
    const visualCard = screen.getByLabelText("New visual card") as HTMLTextAreaElement;
    expect(visualCard.placeholder).toBeTruthy();
    const voiceCard = screen.getByLabelText("New voice card") as HTMLTextAreaElement;
    expect(voiceCard.placeholder).toBeTruthy();
  });
});
