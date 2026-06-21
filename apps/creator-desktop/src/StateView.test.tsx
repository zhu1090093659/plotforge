import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type {
  ResourceDefinition,
  StateVariablesEditDocument,
} from "../../../contracts/plotforge";
import { StateView, type ResourceDraft, type StateViewProps } from "./StateView";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function sampleDocument(): StateVariablesEditDocument {
  return {
    resources: [
      { key: "loyalty", label: "Loyalty", initial: 50, min: 0, max: 100 },
      { key: "tension", label: "Tension", initial: 10, min: 0, max: 100 },
    ],
    initial_world_state: {
      resources: { loyalty: 60, tension: 15 },
      flags: {},
      triggered_events: [],
    },
    initial_story_state: {
      current_scene_key: "opening",
      current_beat_id: null,
      completed_scene_keys: [],
      turn: 0,
    },
  };
}

function emptyDraft(): ResourceDraft {
  return { key: "", label: "", initial: 0, min: 0, max: 100 };
}

function renderView(overrides: Partial<StateViewProps> = {}) {
  const props: StateViewProps = {
    stateVariablesEditDocument: sampleDocument(),
    saving: false,
    formStatus: null,
    newResource: emptyDraft(),
    onNewResourceChange: vi.fn(),
    onSave: vi.fn(),
    onUpdateResource: vi.fn(),
    onUpdateInitialWorldResource: vi.fn(),
    onUpdateInitialSceneKey: vi.fn(),
    onUpdateInitialTurn: vi.fn(),
    onCreateResourceFromDraft: vi.fn(),
    ...overrides,
  };
  return render(<StateView {...props} />);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("StateView", () => {
  afterEach(cleanup);

  it("shows the resource count subtitle", () => {
    renderView();
    expect(screen.getByText("2 resources")).toBeTruthy();
  });

  it("renders collapsed resource cards with label as heading", () => {
    renderView();
    const loyaltyBtns = screen.getAllByRole("button", { name: /Loyalty/ });
    expect(loyaltyBtns.length).toBeGreaterThanOrEqual(1);
    const tensionBtns = screen.getAllByRole("button", { name: /Tension/ });
    expect(tensionBtns.length).toBeGreaterThanOrEqual(1);
  });

  it("resource card is collapsed by default — detail fields hidden", () => {
    renderView();
    expect(screen.queryByLabelText("Resource key 1")).toBeNull();
  });

  it("expanding a resource card reveals all six detail fields", () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Loyalty/ }));
    expect(screen.getByLabelText("Resource key 1")).toBeTruthy();
    expect(screen.getByLabelText("Resource label 1")).toBeTruthy();
    expect(screen.getByLabelText("Resource default initial value 1")).toBeTruthy();
    expect(screen.getByLabelText("Resource world initial value 1")).toBeTruthy();
    expect(screen.getByLabelText("Resource min 1")).toBeTruthy();
    expect(screen.getByLabelText("Resource max 1")).toBeTruthy();
  });

  it("expanded card shows correct world initial value", () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Loyalty/ }));
    const worldInitInput = screen.getByLabelText(
      "Resource world initial value 1",
    ) as HTMLInputElement;
    expect(worldInitInput.value).toBe("60");
  });

  it("calls onUpdateResource when a field value changes", () => {
    const onUpdateResource = vi.fn();
    renderView({ onUpdateResource });
    fireEvent.click(screen.getByRole("button", { name: /Loyalty/ }));
    fireEvent.change(screen.getByLabelText("Resource label 1"), {
      target: { value: "Allegiance" },
    });
    expect(onUpdateResource).toHaveBeenCalledWith(0, { label: "Allegiance" });
  });

  it("calls onUpdateInitialWorldResource when world initial value changes", () => {
    const onUpdateInitialWorldResource = vi.fn();
    renderView({ onUpdateInitialWorldResource });
    fireEvent.click(screen.getByRole("button", { name: /Loyalty/ }));
    fireEvent.change(screen.getByLabelText("Resource world initial value 1"), {
      target: { value: "75" },
    });
    expect(onUpdateInitialWorldResource).toHaveBeenCalledWith("loyalty", 75);
  });

  it("Initial Story State collapsible is collapsed by default", () => {
    renderView();
    expect(screen.queryByLabelText("Initial current scene")).toBeNull();
  });

  it("Initial Story State expands on click", () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: "Initial Story State" }));
    expect(screen.getByLabelText("Initial current scene")).toBeTruthy();
    expect(screen.getByLabelText("Initial turn")).toBeTruthy();
  });

  it("Initial Story State shows correct initial values", () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: "Initial Story State" }));
    const sceneInput = screen.getByLabelText(
      "Initial current scene",
    ) as HTMLInputElement;
    expect(sceneInput.value).toBe("opening");
    const turnInput = screen.getByLabelText(
      "Initial turn",
    ) as HTMLInputElement;
    expect(turnInput.value).toBe("0");
  });

  it("calls onUpdateInitialSceneKey when scene key changes", () => {
    const onUpdateInitialSceneKey = vi.fn();
    renderView({ onUpdateInitialSceneKey });
    fireEvent.click(screen.getByRole("button", { name: "Initial Story State" }));
    fireEvent.change(screen.getByLabelText("Initial current scene"), {
      target: { value: "act-two" },
    });
    expect(onUpdateInitialSceneKey).toHaveBeenCalledWith("act-two");
  });

  it("calls onUpdateInitialTurn when turn changes", () => {
    const onUpdateInitialTurn = vi.fn();
    renderView({ onUpdateInitialTurn });
    fireEvent.click(screen.getByRole("button", { name: "Initial Story State" }));
    fireEvent.change(screen.getByLabelText("Initial turn"), {
      target: { value: "5" },
    });
    expect(onUpdateInitialTurn).toHaveBeenCalledWith(5);
  });

  it("Add Resource collapsible is collapsed by default", () => {
    renderView();
    expect(screen.queryByLabelText("New resource key")).toBeNull();
  });

  it("Add Resource collapsible opens on click", () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: "Add Resource" }));
    expect(screen.getByLabelText("New resource key")).toBeTruthy();
    expect(screen.getByLabelText("New resource label")).toBeTruthy();
    expect(screen.getByLabelText("New resource default initial value")).toBeTruthy();
    expect(screen.getByLabelText("New resource min")).toBeTruthy();
    expect(screen.getByLabelText("New resource max")).toBeTruthy();
  });

  it("calls onNewResourceChange when draft field changes", () => {
    const onNewResourceChange = vi.fn();
    renderView({ onNewResourceChange });
    fireEvent.click(screen.getByRole("button", { name: "Add Resource" }));
    fireEvent.change(screen.getByLabelText("New resource key"), {
      target: { value: "morale" },
    });
    expect(onNewResourceChange).toHaveBeenCalledWith(
      expect.objectContaining({ key: "morale" }),
    );
  });

  it("calls onCreateResourceFromDraft when Create Resource is clicked", () => {
    const onCreateResourceFromDraft = vi.fn();
    renderView({ onCreateResourceFromDraft });
    fireEvent.click(screen.getByRole("button", { name: "Add Resource" }));
    fireEvent.click(screen.getByRole("button", { name: "Create Resource" }));
    expect(onCreateResourceFromDraft).toHaveBeenCalledOnce();
  });

  it("calls onSave when Save State button is clicked", () => {
    const onSave = vi.fn();
    renderView({ onSave });
    fireEvent.click(screen.getByRole("button", { name: "Save State" }));
    expect(onSave).toHaveBeenCalledOnce();
  });

  it("Save State button is disabled while saving", () => {
    renderView({ saving: true });
    const saveBtn = screen.getByRole("button", {
      name: "Save State",
    }) as HTMLButtonElement;
    expect(saveBtn.disabled).toBe(true);
  });

  it("shows success form status message for state section", () => {
    renderView({
      formStatus: { section: "state", tone: "success", message: "State saved." },
    });
    expect(screen.getByText("State saved.")).toBeTruthy();
  });

  it("does not show form status from a different section", () => {
    renderView({
      formStatus: { section: "world", tone: "success", message: "World saved." },
    });
    expect(screen.queryByText("World saved.")).toBeNull();
  });

  it("shows empty state when stateVariablesEditDocument is null", () => {
    renderView({ stateVariablesEditDocument: null });
    expect(screen.getByText("State edit document not loaded.")).toBeTruthy();
  });
});
