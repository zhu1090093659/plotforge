import { act, renderHook, waitFor } from "@testing-library/react";
import { useRef, useState } from "react";
import { describe, expect, it, vi } from "vitest";
import type { AgentSessionConfig, PiAgentApplyResult } from "../../../contracts/plotforge";
import type { StudioDataSource } from "./studioDataSource";
import { useAgentConversation } from "./useAgentConversation";
import type { PlaytestWorkspace, PlaytestRunResult } from "./usePlaytest";
import { demoPlayOnceReport } from "./demoStudioData";
import type { PlayOnceReport } from "./tauriBridge";

interface HarnessOptions {
  /** Force piAgentApplyRun to reject with this error message. */
  failWithError?: string;
}

const defaultAgentConfig: AgentSessionConfig = {
  model_id: "local-pi",
  permission_level: "ask_every_time",
  thinking_level: "medium",
  enabled_skills: [],
  enabled_mcp_servers: [],
};

/** Build a `PiAgentApplyResult` from a demo `PlayOnceReport` so the hook can
 * convert it back into the `PlayOnceReport`-shaped turn the rail renders. */
function demoApplyResult(report: PlayOnceReport): PiAgentApplyResult {
  return {
    run: {
      descriptor: {
        agent_id: "local-pi",
        is_local_pi: true,
        capabilities: [],
      },
      reproducibility: {
        run_seed: 0,
        prompt_version: "",
        model_version: "",
        provider_config_hash: "",
        trace_id: "pi-agent-evidence-test",
      },
      trace_id: "pi-agent-evidence-test",
      evidence_summary: "",
    },
    scene_key: report.scene.key,
    scene: report.scene,
    trace: report.trace,
    trace_path: report.trace_path,
    snapshot: report.snapshot,
    snapshot_path: report.snapshot_path,
    delta_summary: report.delta_summary,
  };
}

// A harness that owns a controllable playtest workspace + a mock data source
// whose `piAgentApplyRun` resolves/rejects based on the harness options.
// The mock is created OUTSIDE the render closure so the same `vi.fn` instance
// is captured by the hook on every render (otherwise the hook's `dataSource`
// dependency would rebuild and lose the call record).
function useHarness(options: HarnessOptions = {}) {
  const [input, setInput] = useState("pay the army");
  // Keep a stable mock + data source across renders so the hook's
  // `useCallback([dataSource, ...])` does not rebuild and lose the call
  // record. `useRef` ensures the same `vi.fn` instance survives re-renders.
  const mockRef = useRef<ReturnType<typeof vi.fn> | null>(null);
  if (!mockRef.current) {
    mockRef.current = vi.fn(async () => {
      if (options.failWithError) {
        throw new Error(options.failWithError);
      }
      await new Promise((resolve) => setTimeout(resolve, 0));
      return demoApplyResult(demoPlayOnceReport(input));
    });
  }
  const piAgentApplyRun = mockRef.current;
  const dataSource = { piAgentApplyRun } as unknown as StudioDataSource;

  const playtest = {
    playtestInput: input,
    setPlaytestInput: setInput,
    playtesting: false,
    playtestReport: null,
    playtestError: null,
    setPlaytestReport: () => {},
    setPlaytestError: () => {},
    runPlaytest: vi.fn(),
  } as unknown as PlaytestWorkspace;

  return {
    workspace: useAgentConversation(
      playtest,
      "/tmp/starter-project",
      dataSource,
      defaultAgentConfig,
    ),
    piAgentApplyRun,
  };
}

describe("useAgentConversation", () => {
  it("starts with no turns and canSubmit when input is non-empty", () => {
    const { workspace } = renderHook(() => useHarness()).result.current;
    expect(workspace.turns).toEqual([]);
    expect(workspace.canSubmit).toBe(true);
    expect(workspace.running).toBe(false);
  });

  it("appends a turn after a submitted pi-Agent run completes", async () => {
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
    expect(result.current.piAgentApplyRun).toHaveBeenCalledTimes(1);
  });

  it("does not submit when input is empty", async () => {
    const { result } = renderHook(() => useHarness());
    act(() => result.current.workspace.setInput("   "));
    expect(result.current.workspace.canSubmit).toBe(false);
    await act(async () => {
      await result.current.workspace.submit();
    });
    expect(result.current.workspace.turns.length).toBe(0);
    expect(result.current.piAgentApplyRun).not.toHaveBeenCalled();
  });

  it("appends an error turn (report null, error set) when the run fails", async () => {
    const { result } = renderHook(() =>
      useHarness({ failWithError: "provider_timeout" }),
    );
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
    expect(turn.errorCode).toBe("pi_agent_provider_timeout");
  });

  it("maps a missing-credential failure to a friendly errorCode and extracts the env var", async () => {
    const { result } = renderHook(() =>
      // The redacted provider error string carries the error code
      // `text_provider_missing_credential` (which `mapApplyErrorToCode`
      // matches via `includes("missing_credential")`) and names the env var
      // (NAME only, never the value) inside backticks.
      useHarness({
        failWithError:
          "text_provider_missing_credential: missing provider credential in env var `ZAI_API_KEY`",
      }),
    );
    await act(async () => {
      await result.current.workspace.submit();
    });
    await waitFor(() => {
      expect(result.current.workspace.turns.length).toBe(1);
    });
    const turn = result.current.workspace.turns[0];
    expect(turn.errorCode).toBe("pi_agent_missing_credential");
    expect(turn.errorEnvVar).toBe("ZAI_API_KEY");
  });

  // T1.4: the new text_provider_* error codes map to distinct errorCode values
  // so the rail can render specific guidance (rate-limit / content-filter /
  // output-truncation) instead of a generic "turn failed".
  it("maps a rate-limit failure to the text_provider_rate_limit errorCode", async () => {
    const { result } = renderHook(() =>
      useHarness({
        failWithError:
          "pi-agent provider failure: text_provider_rate_limit: 429 too many requests",
      }),
    );
    await act(async () => {
      await result.current.workspace.submit();
    });
    await waitFor(() => {
      expect(result.current.workspace.turns.length).toBe(1);
    });
    expect(result.current.workspace.turns[0].errorCode).toBe(
      "text_provider_rate_limit",
    );
  });

  it("maps a content-filtered failure to the text_provider_content_filtered errorCode", async () => {
    const { result } = renderHook(() =>
      useHarness({
        failWithError:
          "pi-agent provider failure: text_provider_content_filtered: content policy triggered",
      }),
    );
    await act(async () => {
      await result.current.workspace.submit();
    });
    await waitFor(() => {
      expect(result.current.workspace.turns.length).toBe(1);
    });
    expect(result.current.workspace.turns[0].errorCode).toBe(
      "text_provider_content_filtered",
    );
  });

  it("maps an output-truncated failure to the text_provider_output_truncated errorCode", async () => {
    const { result } = renderHook(() =>
      useHarness({
        failWithError:
          "pi-agent provider failure: text_provider_output_truncated: output truncated at max_tokens",
      }),
    );
    await act(async () => {
      await result.current.workspace.submit();
    });
    await waitFor(() => {
      expect(result.current.workspace.turns.length).toBe(1);
    });
    expect(result.current.workspace.turns[0].errorCode).toBe(
      "text_provider_output_truncated",
    );
  });

  it("maps a flagged moderation preflight to its explicit rail error code", async () => {
    const { result } = renderHook(() =>
      useHarness({
        failWithError:
          'pi_agent_moderation_flagged: moderation provider flagged content in categories: ["violence"]',
      }),
    );
    await act(async () => {
      await result.current.workspace.submit();
    });
    await waitFor(() => {
      expect(result.current.workspace.turns.length).toBe(1);
    });
    expect(result.current.workspace.turns[0].errorCode).toBe(
      "pi_agent_moderation_flagged",
    );
  });

  it("maps other moderation preflight failures without pretending text ran", async () => {
    const { result } = renderHook(() =>
      useHarness({
        failWithError:
          "pi_agent_moderation_rate_limit: moderation provider rate limited",
      }),
    );
    await act(async () => {
      await result.current.workspace.submit();
    });
    await waitFor(() => {
      expect(result.current.workspace.turns.length).toBe(1);
    });
    expect(result.current.workspace.turns[0].errorCode).toBe(
      "pi_agent_moderation_failed",
    );
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

  // Finding M4: the `runningRef` synchronous dedup guard must prevent two
  // concurrent `piAgentApplyRun` calls when `submit()` is invoked twice
  // before React commits `running=true`. The second call must return
  // `{ deduped: true }` and the mock must be called exactly once.
  it("dedupes a rapid double-submit so only one provider call fires", async () => {
    const { result } = renderHook(() => useHarness());

    // Fire two submits without awaiting the first. Both promises are kept so
    // we can drain them; the guard should short-circuit the second.
    let first: PlaytestRunResult | undefined;
    let second: PlaytestRunResult | undefined;
    await act(async () => {
      const p1 = result.current.workspace.submit();
      const p2 = result.current.workspace.submit();
      [first, second] = await Promise.all([p1, p2]);
    });

    // Exactly one provider call fired.
    expect(result.current.piAgentApplyRun).toHaveBeenCalledTimes(1);
    // The first submit succeeded; the second was deduped.
    expect(first?.succeeded).toBe(true);
    expect(second).toBeDefined();
    expect(second?.succeeded).toBe(false);
    if (second && !second.succeeded) {
      expect(second.deduped).toBe(true);
    }
    // Only one turn was appended.
    expect(result.current.workspace.turns.length).toBe(1);
  });
});
