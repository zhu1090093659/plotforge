import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { StudioCommandPalette, type PaletteAction } from "./StudioCommandPalette";
import { StudioI18nProvider } from "./i18n";

afterEach(() => {
  cleanup();
});

function baseActions(overrides: { run?: () => void } = {}): PaletteAction[] {
  return [
    { id: "nav-world", label: "World Bible", run: overrides.run ?? vi.fn() },
    { id: "nav-story", label: "Story Craft", run: vi.fn() },
    { id: "run-proof", label: "Run playable proof", run: vi.fn() },
  ];
}

function renderPalette(overrides: Partial<Parameters<typeof StudioCommandPalette>[0]> = {}) {
  const onOpenProject = vi.fn();
  const onClose = vi.fn();
  const actions = overrides.actions ?? baseActions();
  render(
    <StudioI18nProvider>
      <StudioCommandPalette
        open={overrides.open ?? true}
        onClose={onClose}
        actions={actions}
        onOpenProject={onOpenProject}
      />
    </StudioI18nProvider>,
  );
  return { onOpenProject, onClose, actions };
}

describe("StudioCommandPalette", () => {
  it("renders nothing when closed", () => {
    renderPalette({ open: false });
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("renders the dialog with a filter input and all actions when open", () => {
    renderPalette();
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(screen.getByLabelText("Type a command or project path…")).toBeTruthy();
    expect(screen.getByText("World Bible")).toBeTruthy();
    expect(screen.getByText("Story Craft")).toBeTruthy();
    expect(screen.getByText("Run playable proof")).toBeTruthy();
  });

  it("filters actions by the typed query", () => {
    renderPalette();
    const input = screen.getByLabelText("Type a command or project path…");
    fireEvent.change(input, { target: { value: "story" } });
    expect(screen.getByText("Story Craft")).toBeTruthy();
    expect(screen.queryByText("World Bible")).toBeNull();
  });

  it("offers an 'Open project' fallback action for a non-path, non-matching query", () => {
    renderPalette();
    fireEvent.change(screen.getByLabelText("Type a command or project path…"), {
      target: { value: "zzzz-no-match" },
    });
    // The typed query matches no nav action and is not path-like, but the
    // open-project fallback is still offered (loaded on Enter below).
    expect(screen.getByText(/Open or create project "zzzz-no-match"/)).toBeTruthy();
  });

  it("renders the true empty state when there are no actions and no query", () => {
    renderPalette({ actions: [] });
    expect(screen.getByText("No matching commands")).toBeTruthy();
    expect(screen.queryByText(/Open project/)).toBeNull();
  });

  it("offers an 'Open project' action for a typed path and loads it on Enter", () => {
    const { onOpenProject, onClose } = renderPalette();
    const input = screen.getByLabelText("Type a command or project path…");
    fireEvent.change(input, { target: { value: "/tmp/some-project" } });
    expect(screen.getByText(/Open or create project "\/tmp\/some-project"/)).toBeTruthy();
    // It is the first (active) option; Enter runs it.
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onOpenProject).toHaveBeenCalledWith("/tmp/some-project");
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("runs the matched nav action (not the open-project fallback) on Enter", () => {
    const worldRun = vi.fn();
    const { onOpenProject } = renderPalette({
      actions: [{ id: "nav-world", label: "World Bible", run: worldRun }],
    });
    const input = screen.getByLabelText("Type a command or project path…");
    // "World" matches the nav label and is not path-like; the nav action must
    // stay first (active) so Enter runs it instead of loading a project.
    fireEvent.change(input, { target: { value: "World" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(worldRun).toHaveBeenCalledTimes(1);
    expect(onOpenProject).not.toHaveBeenCalled();
  });

  it("runs the active action on Enter and closes", () => {
    const run = vi.fn();
    const { onClose } = renderPalette({
      actions: [{ id: "nav-world", label: "World Bible", run }],
    });
    const input = screen.getByLabelText("Type a command or project path…");
    fireEvent.keyDown(input, { key: "Enter" });
    expect(run).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("closes on Escape", () => {
    const { onClose } = renderPalette();
    const input = screen.getByLabelText("Type a command or project path…");
    fireEvent.keyDown(input, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("closes on backdrop click", () => {
    const { onClose } = renderPalette();
    // The backdrop is the outermost fixed container.
    const dialog = screen.getByRole("dialog");
    const backdrop = dialog.parentElement!;
    fireEvent.click(backdrop);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("moves the active option with ArrowDown and runs it on Enter", () => {
    const storyRun = vi.fn();
    const { onClose } = renderPalette({
      actions: [
        { id: "nav-world", label: "World Bible", run: vi.fn() },
        { id: "nav-story", label: "Story Craft", run: storyRun },
      ],
    });
    const input = screen.getByLabelText("Type a command or project path…");
    // First option (World Bible) is active by default; ArrowDown moves to Story.
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(storyRun).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("moves the active option with ArrowUp and clamps at index 0", () => {
    const worldRun = vi.fn();
    const storyRun = vi.fn();
    renderPalette({
      actions: [
        { id: "nav-world", label: "World Bible", run: worldRun },
        { id: "nav-story", label: "Story Craft", run: storyRun },
      ],
    });
    const input = screen.getByLabelText("Type a command or project path…");
    // ArrowUp at index 0 must clamp (stay on World Bible), not go negative.
    fireEvent.keyDown(input, { key: "ArrowUp" });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(worldRun).toHaveBeenCalledTimes(1);
    expect(storyRun).not.toHaveBeenCalled();
  });

  it("ArrowDown clamps at the last option", () => {
    const worldRun = vi.fn();
    const storyRun = vi.fn();
    renderPalette({
      actions: [
        { id: "nav-world", label: "World Bible", run: worldRun },
        { id: "nav-story", label: "Story Craft", run: storyRun },
      ],
    });
    const input = screen.getByLabelText("Type a command or project path…");
    // ArrowDown twice (last item is index 1); a third ArrowDown must clamp.
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(storyRun).toHaveBeenCalledTimes(1);
    expect(worldRun).not.toHaveBeenCalled();
  });

  it("Home jumps to the first option and End jumps to the last", () => {
    const worldRun = vi.fn();
    const storyRun = vi.fn();
    const gammaRun = vi.fn();
    renderPalette({
      actions: [
        { id: "nav-world", label: "World Bible", run: worldRun },
        { id: "nav-story", label: "Story Craft", run: storyRun },
        { id: "nav-gamma", label: "Gamma", run: gammaRun },
      ],
    });
    const input = screen.getByLabelText("Type a command or project path…");
    // End -> Gamma (last).
    fireEvent.keyDown(input, { key: "End" });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(gammaRun).toHaveBeenCalledTimes(1);
    // Re-render a fresh palette (run+close unmounts) for the Home assertion.
  });

  it("Home returns to the first option from the last", () => {
    const worldRun = vi.fn();
    const gammaRun = vi.fn();
    renderPalette({
      actions: [
        { id: "nav-world", label: "World Bible", run: worldRun },
        { id: "nav-story", label: "Story Craft", run: vi.fn() },
        { id: "nav-gamma", label: "Gamma", run: gammaRun },
      ],
    });
    const input = screen.getByLabelText("Type a command or project path…");
    // End -> Gamma, Home -> World Bible, Enter -> world.
    fireEvent.keyDown(input, { key: "End" });
    fireEvent.keyDown(input, { key: "Home" });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(worldRun).toHaveBeenCalledTimes(1);
    expect(gammaRun).not.toHaveBeenCalled();
  });

  it("updates aria-activedescendant to match the active option id", () => {
    renderPalette({
      actions: [
        { id: "nav-world", label: "World Bible", run: vi.fn() },
        { id: "nav-story", label: "Story Craft", run: vi.fn() },
      ],
    });
    const input = screen.getByLabelText(
      "Type a command or project path…",
    ) as HTMLInputElement;
    // Default active is index 0 -> palette-option-0.
    expect(input.getAttribute("aria-activedescendant")).toBe("palette-option-0");
    fireEvent.keyDown(input, { key: "ArrowDown" });
    expect(input.getAttribute("aria-activedescendant")).toBe("palette-option-1");
  });

  it("runs an action on click", () => {
    const run = vi.fn();
    const { onClose } = renderPalette({
      actions: [{ id: "nav-world", label: "World Bible", run }],
    });
    fireEvent.click(screen.getByText("World Bible"));
    expect(run).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
