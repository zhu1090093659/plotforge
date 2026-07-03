import { act, renderHook, waitFor } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { useAgentConversation } from "./useAgentConversation";
import type { PlaytestWorkspace } from "./usePlaytest";
import { demoPlayOnceReport } from "./demoStudioData";
import type { PlayOnceReport } from "./tauriBridge";

interface HarnessOptions {
  /** Force runPlaytest to fail with this error message. */
  failWithError?: string;
  /** Start (and keep) playtesting=true so submit's guard short-circuits. */
  startPlaying?: boolean;
}

// A harness that owns a controllable playtest workspace; saveId/snapshot
// routing is orthogonal to the conversation-turn logic under test.
function useHarness(options: HarnessOptions = {}) {
  const [input, setInput] = useState("pay the army");
  const [playing, setPlaying] = useState(Boolean(options.startPlaying));
  const [report, setReport] = useState<PlayOnceReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  const runPlaytest = vi.fn(async () => {
    if (options.startPlaying) {
      // When forced into the playing state, mirror real runPlaytest which
      // would not double-enter; just return a failure so a missed guard
      // would still append an unwanted turn.
      return { succeeded: false, error: "already-running" };
    }
    setPlaying(true);
    // A macrotask yield (not a microtask) so React commits the
    // playing=true render and the edge effect records it before the
    // playing=false render. Promise.resolve() is too short — act batches
    // across microtasks and the true render never commits.
    await new Promise((resolve) => setTimeout(resolve, 0));
    if (options.failWithError) {
      setError(options.failWithError);
      setPlaying(false);
      return { succeeded: false, error: options.failWithError };
    }
    const nextReport = demoPlayOnceReport(input);
    setReport(nextReport);
    setError(null);
    setPlaying(false);
    return { succeeded: true, report: nextReport };
  });

  const playtest = {
    playtestInput: input,
    setPlaytestInput: setInput,
    playtesting: playing,
    playtestReport: report,
    playtestError: error,
    setPlaytestReport: setReport,
    setPlaytestError: setError,
    runPlaytest,
  } as unknown as PlaytestWorkspace;

  return { workspace: useAgentConversation(playtest, "/tmp/starter-project"), runPlaytest };
}

describe("useAgentConversation", () => {
  it("starts with no turns and canSubmit when input is non-empty", () => {
    const { workspace } = renderHook(() => useHarness()).result.current;
    expect(workspace.turns).toEqual([]);
    expect(workspace.canSubmit).toBe(true);
    expect(workspace.running).toBe(false);
  });

  it("appends a turn after a submitted run completes", async () => {
    const { result } = renderHook(() => useHarness());
    await act(async () => {
      await result.current.workspace.submit();
    });
    await waitFor(() => {
      expect(result.current.workspace.turns.length).toBe(1);
    });
    expect(result.current.workspace.turns[0].intent).toBe("pay the army");
    expect(result.current.workspace.turns[0].report).not.toBeNull();
    expect(result.current.workspace.turns[0].error).toBeNull();
  });

  it("does not submit when input is empty", async () => {
    const { result } = renderHook(() => useHarness());
    act(() => result.current.workspace.setInput("   "));
    expect(result.current.workspace.canSubmit).toBe(false);
    await act(async () => {
      await result.current.workspace.submit();
    });
    expect(result.current.workspace.turns.length).toBe(0);
    expect(result.current.runPlaytest).not.toHaveBeenCalled();
  });

  it("appends an error turn (report null, error set) when the run fails", async () => {
    const { result } = renderHook(() => useHarness({ failWithError: "provider_timeout" }));
    await act(async () => {
      await result.current.workspace.submit();
    });
    await waitFor(() => {
      expect(result.current.workspace.turns.length).toBe(1);
    });
    const turn = result.current.workspace.turns[0];
    expect(turn.intent).toBe("pay the army");
    expect(turn.report).toBeNull();
    expect(turn.error).toBe("provider_timeout");
  });

  it("no-ops while a run is already in progress (playtesting guard)", async () => {
    const { result } = renderHook(() => useHarness({ startPlaying: true }));
    // While playtesting is true, canSubmit is false and submit must not call
    // runPlaytest or append a turn.
    expect(result.current.workspace.canSubmit).toBe(false);
    expect(result.current.workspace.running).toBe(true);
    await act(async () => {
      await result.current.workspace.submit();
    });
    expect(result.current.workspace.turns.length).toBe(0);
    expect(result.current.runPlaytest).not.toHaveBeenCalled();
  });

  it("appends multiple turns with monotonic agent-turn-N ids", async () => {
    const { result } = renderHook(() => useHarness());
    await act(async () => {
      await result.current.workspace.submit();
    });
    await act(() => result.current.workspace.setInput("raise the levies"));
    await act(async () => {
      await result.current.workspace.submit();
    });
    await waitFor(() => {
      expect(result.current.workspace.turns.length).toBe(2);
    });
    expect(result.current.workspace.turns[0].id).toBe("agent-turn-1");
    expect(result.current.workspace.turns[0].intent).toBe("pay the army");
    expect(result.current.workspace.turns[1].id).toBe("agent-turn-2");
    expect(result.current.workspace.turns[1].intent).toBe("raise the levies");
  });
});
