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
    errorCode: withError ? "pi_agent_provider_timeout" : null,
    errorEnvVar: null,
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

  // R1: a turn with `errorCode = pi_agent_missing_credential` and an env-var
  // name must render the friendly, code-specific i18n message (naming the
  // env var) instead of the raw redacted error string. The raw message is
  // kept as a detail line so the user can still see the provider's redacted
  // text.
  it("renders the friendly missing-credential message naming the env var", () => {
    const turn: AgentTurn = {
      id: "turn-cred",
      intent: "run a turn",
      report: null,
      error: "missing provider credential in env var `OPENAI_API_KEY`",
      errorCode: "pi_agent_missing_credential",
      errorEnvVar: "OPENAI_API_KEY",
    };
    renderRail({ turns: [turn] });
    // The friendly message names the env var. The raw redacted error is also
    // rendered as a detail line, so match the friendly prefix specifically.
    expect(
      screen.getByText(/Set the env var OPENAI_API_KEY in your shell/),
    ).toBeTruthy();
  });

  // R1: a timeout turn renders the friendly provider-timeout message.
  it("renders the friendly provider-timeout message for a timeout error code", () => {
    const turn: AgentTurn = {
      id: "turn-timeout",
      intent: "run a turn",
      report: null,
      error: "text provider timed out",
      errorCode: "pi_agent_provider_timeout",
      errorEnvVar: null,
    };
    renderRail({ turns: [turn] });
    expect(screen.getByText(/Provider timed out/)).toBeTruthy();
  });

  // T1.4: rich error display for the new text_provider_* error codes. Each
  // surfaces specific guidance instead of a generic "error". The raw redacted
  // provider message is kept as a detail line under the friendly message.
  it("renders the rate-limit guidance for text_provider_rate_limit", () => {
    const turn: AgentTurn = {
      id: "turn-rate-limit",
      intent: "run a turn",
      report: null,
      error: "pi-agent provider failure: text_provider_rate_limit: 429",
      errorCode: "text_provider_rate_limit",
      errorEnvVar: null,
    };
    renderRail({ turns: [turn] });
    expect(screen.getByText(/Rate limited; retrying with backoff/)).toBeTruthy();
  });

  it("renders the content-filter guidance for text_provider_content_filtered", () => {
    const turn: AgentTurn = {
      id: "turn-content-filter",
      intent: "run a turn",
      report: null,
      error: "pi-agent provider failure: text_provider_content_filtered: content policy triggered",
      errorCode: "text_provider_content_filtered",
      errorEnvVar: null,
    };
    renderRail({ turns: [turn] });
    expect(screen.getByText(/Content policy triggered; modify your prompt/)).toBeTruthy();
  });

  it("renders the output-truncation guidance for text_provider_output_truncated", () => {
    const turn: AgentTurn = {
      id: "turn-output-truncated",
      intent: "run a turn",
      report: null,
      error: "pi-agent provider failure: text_provider_output_truncated: output truncated at max_tokens",
      errorCode: "text_provider_output_truncated",
      errorEnvVar: null,
    };
    renderRail({ turns: [turn] });
    expect(
      screen.getByText(/Output truncated; increase max_output_tokens or reduce context/),
    ).toBeTruthy();
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
