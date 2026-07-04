import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AgentChatRail } from "./AgentChatRail";
import type { AgentTurn } from "./useAgentConversation";
import { demoPlayOnceReport } from "./demoStudioData";
import { StudioI18nProvider } from "./i18n";

afterEach(() => {
  cleanup();
});

function makeTurn(intent: string, withError = false): AgentTurn {
  const report = withError ? null : demoPlayOnceReport(intent);
  return {
    id: `turn-${intent}`,
    intent,
    report,
    error: withError ? "provider_timeout" : null,
  };
}

function renderRail(overrides: Partial<Parameters<typeof AgentChatRail>[0]> = {}) {
  const onInputChange = vi.fn();
  const onSubmit = vi.fn();
  const onOpenTrace = vi.fn();
  render(
    <StudioI18nProvider>
      <AgentChatRail
        turns={[]}
        input="pay the army"
        onInputChange={onInputChange}
        running={false}
        canSubmit={true}
        onSubmit={onSubmit}
        onOpenTrace={onOpenTrace}
        evidence={<p>boundary-evidence-marker</p>}
        {...overrides}
      />
    </StudioI18nProvider>,
  );
  return { onInputChange, onSubmit, onOpenTrace };
}

describe("AgentChatRail", () => {
  it("shows the empty state when there are no turns", () => {
    renderRail({ turns: [] });
    expect(
      screen.getByText("No turns yet. Describe a change and run a proof turn."),
    ).toBeTruthy();
  });

  it("renders a turn's intent and agent result (trace id + scene)", () => {
    const turn = makeTurn("pay the army");
    renderRail({ turns: [turn] });
    expect(screen.getAllByText("pay the army").length).toBeGreaterThan(0);
    expect(screen.getAllByText("trace-001").length).toBeGreaterThan(0);
  });

  it("renders an error result when the turn failed", () => {
    const turn = makeTurn("bad intent", true);
    renderRail({ turns: [turn] });
    expect(screen.getByText(/provider_timeout/)).toBeTruthy();
    // An error turn has no report, so the per-turn "Open trace" button must
    // not render (it is gated on `report` being truthy).
    expect(screen.queryByRole("button", { name: "Open trace" })).toBeNull();
  });

  it("calls onSubmit when the Send button is clicked", () => {
    const { onSubmit } = renderRail();
    fireEvent.click(screen.getByRole("button", { name: "Send" }));
    expect(onSubmit).toHaveBeenCalledTimes(1);
  });

  it("disables the Send button when canSubmit is false", () => {
    renderRail({ canSubmit: false });
    expect(
      screen.getByRole("button", { name: "Send" }).hasAttribute("disabled"),
    ).toBe(true);
  });

  it("calls onInputChange when the input textarea changes", () => {
    const { onInputChange } = renderRail();
    fireEvent.change(screen.getByLabelText("Direct the agent — describe a change…"), {
      target: { value: "raise taxes" },
    });
    expect(onInputChange).toHaveBeenCalledWith("raise taxes");
  });

  it("toggles the Evidence popover open and closed", () => {
    renderRail();
    // Evidence content is hidden by default.
    expect(screen.queryByText("boundary-evidence-marker")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: /^Evidence$/ }));
    expect(screen.getByText("boundary-evidence-marker")).toBeTruthy();
    // Click again to close.
    fireEvent.click(screen.getByRole("button", { name: /^Evidence$/ }));
    expect(screen.queryByText("boundary-evidence-marker")).toBeNull();
  });

  it("closes the Evidence popover on Escape", () => {
    renderRail();
    fireEvent.click(screen.getByRole("button", { name: /^Evidence$/ }));
    expect(screen.getByText("boundary-evidence-marker")).toBeTruthy();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByText("boundary-evidence-marker")).toBeNull();
  });

  it("closes the Evidence popover on outside click", () => {
    renderRail();
    fireEvent.click(screen.getByRole("button", { name: /^Evidence$/ }));
    expect(screen.getByText("boundary-evidence-marker")).toBeTruthy();
    // mousedown outside both the popover and the toggle button closes it.
    fireEvent.mouseDown(document.body);
    expect(screen.queryByText("boundary-evidence-marker")).toBeNull();
  });

  it("does not close the Evidence popover when a click lands inside it", () => {
    renderRail();
    fireEvent.click(screen.getByRole("button", { name: /^Evidence$/ }));
    const popover = screen.getByRole("dialog");
    // A mousedown inside the popover must not close it.
    fireEvent.mouseDown(popover);
    expect(screen.getByText("boundary-evidence-marker")).toBeTruthy();
  });

  it("calls onOpenTrace when a turn's Open trace button is clicked", () => {
    const { onOpenTrace } = renderRail({ turns: [makeTurn("pay the army")] });
    fireEvent.click(screen.getByRole("button", { name: "Open trace" }));
    expect(onOpenTrace).toHaveBeenCalledTimes(1);
  });

  it("shows a running spinner label on Send while running", () => {
    renderRail({ running: true, canSubmit: false });
    // While running, Send is disabled.
    expect(
      screen.getByRole("button", { name: "Send" }).hasAttribute("disabled"),
    ).toBe(true);
  });
});
