import { useCallback, useRef, useState } from "react";
import type { AgentSessionConfig, PiAgentApplyResult } from "../../../contracts/plotforge";
import type { PlayOnceReport } from "./tauriBridge";
import type { StudioDataSource } from "./studioDataSource";
import type { PlaytestRunResult, PlaytestWorkspace } from "./usePlaytest";

// ---------------------------------------------------------------------------
// useAgentConversation — the agent chat state driving pi-Agent turns.
//
// The right rail's "describe a change / run a turn" now drives the
// `pi_agent_apply_run` Studio command: the pi-Agent generates a ScenePlan
// proposal via the configured provider (local mock or a real HTTP provider),
// the runtime commits it as a state change (rules + scene + turn bump), and
// the result is appended as a chat turn. The shared `playtestInput` state
// stays the single source of truth for the input box (per AGENTS.md), and
// `usePlaytest` is still the owner of snapshot-control state
// (`playtestSaveId`/`restoreId`/`restoreLatest`) used by the rail's
// "Advanced snapshot controls" — but the rail's main submit path no longer
// drives `play_once_project*`.
//
// Failure modes are explicit (never silent):
// - `pi_agent_missing_credential`: the provider's `credential_env_var` names
//   an env var that is missing or empty. The rail surfaces a friendly hint
//   naming the env var (never its value).
// - `pi_agent_provider_timeout`: the HTTP call timed out.
// - other: a redacted provider error message.
// ---------------------------------------------------------------------------

export interface AgentTurn {
  id: string;
  intent: string;
  report: PlayOnceReport | null;
  error: string | null;
  /** When the run failed with a known code, this carries the redaction-safe
   * code so the rail can render a friendlier, code-specific message. */
  errorCode: string | null;
  /** When `errorCode` is `pi_agent_missing_credential`, this carries the
   * env-var name extracted from the redacted provider message (e.g.
   * `OPENAI_API_KEY`) so the rail can surface the friendly "set {envVar}"
   * hint. The value is the env-var NAME only, never the credential value. */
  errorEnvVar: string | null;
}

export interface AgentConversationWorkspace {
  /** Director-intent input (shared with the playtest workspace). */
  input: string;
  setInput(value: string): void;
  turns: AgentTurn[];
  running: boolean;
  canSubmit: boolean;
  /** Run a turn using the current shared `input` state. */
  submit(): Promise<PlaytestRunResult>;
  /**
   * Run a turn with an explicit intent (e.g. a clicked choice label),
   * bypassing the shared-input React-state commit delay. Also mirrors the
   * intent into the shared input box so the chat rail stays in sync.
   */
  submitWith(intent: string): Promise<PlaytestRunResult>;
}

function applyResultToReport(result: PiAgentApplyResult): PlayOnceReport {
  // The pi-Agent result carries the same scene + trace + snapshot shape as a
  // `PlayOnceReport`. `delta_summary` is now surfaced by the Studio command
  // (populated from `summarize_delta(&trace.world_state_delta)` in
  // `pi_agent_apply_run`), so the rail and `TraceDebugView` render the
  // "State deltas" count chip from the same value `play_once_project*`
  // produces — no TypeScript re-implementation of `summarize_delta`.
  return {
    scene: result.scene,
    trace: result.trace,
    trace_path: result.trace_path,
    snapshot: result.snapshot ?? null,
    snapshot_path: result.snapshot_path ?? null,
    delta_summary: result.delta_summary ?? [],
  };
}

function mapApplyErrorToCode(message: string): string | null {
  if (message.includes("missing_credential")) return "pi_agent_missing_credential";
  if (message.includes("timeout")) return "pi_agent_provider_timeout";
  return null;
}

/** Extracts the env-var name from a redacted missing-credential provider
 * message. The provider error message is redaction-safe but does carry the
 * env-var NAME (never the value) inside backticks, e.g.
 * "missing provider credential in env var `OPENAI_API_KEY`". Returns the name
 * so the rail can render the friendly "set {envVar}" hint. */
function extractEnvVarName(message: string): string | null {
  const match = message.match(/`([A-Z_][A-Z0-9_]*)`/);
  return match ? match[1] : null;
}

export function useAgentConversation(
  playtest: PlaytestWorkspace,
  loadedPath: string,
  dataSource: StudioDataSource,
  agentConfig: AgentSessionConfig,
): AgentConversationWorkspace {
  const [turns, setTurns] = useState<AgentTurn[]>([]);
  // Monotonic id counter (deterministic for tests).
  const idCounterRef = useRef(0);
  // Synchronous dedup guard so a rapid double-submit (before React commits
  // `running=true`) does not spawn two provider calls.
  const runningRef = useRef(false);
  const [running, setRunning] = useState(false);

  const runAndAppend = useCallback(
    async (intent: string): Promise<PlaytestRunResult> => {
      const trimmed = intent.trim();
      if (!trimmed) {
        return {
          succeeded: false,
          error: "Playtest input is required.",
        };
      }
      if (runningRef.current) {
        return {
          succeeded: false,
          error: "A pi-Agent turn is already running.",
          deduped: true,
        };
      }
      runningRef.current = true;
      setRunning(true);
      try {
        const result = await dataSource.piAgentApplyRun({
          agent_id: agentConfig.model_id,
          run_seed: Date.now(),
          project_path: loadedPath,
          player_input: trimmed,
          save_id: playtest.playtestSaveId,
          restore_id: playtest.playtestRestoreLatest
            ? undefined
            : playtest.playtestRestoreId,
        });
        idCounterRef.current += 1;
        const report = applyResultToReport(result);
        setTurns((prev) => [
          ...prev,
          {
            id: `agent-turn-${idCounterRef.current}`,
            intent: trimmed,
            report,
            error: null,
            errorCode: null,
            errorEnvVar: null,
          },
        ]);
        // Mirror the report into the playtest workspace so TraceDebugView /
        // PlayView (which read `playtest.playtestReport`) stay in sync with
        // the latest pi-Agent turn. The playtest workspace remains the owner
        // of the report state; this hook just feeds it.
        playtest.setPlaytestReport(report);
        playtest.setPlaytestError(null);
        return { succeeded: true, report };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        const errorCode = mapApplyErrorToCode(message);
        const errorEnvVar =
          errorCode === "pi_agent_missing_credential" ? extractEnvVarName(message) : null;
        idCounterRef.current += 1;
        setTurns((prev) => [
          ...prev,
          {
            id: `agent-turn-${idCounterRef.current}`,
            intent: trimmed,
            report: null,
            error: message,
            errorCode,
            errorEnvVar,
          },
        ]);
        playtest.setPlaytestReport(null);
        playtest.setPlaytestError(message);
        return { succeeded: false, error: message };
      } finally {
        runningRef.current = false;
        setRunning(false);
      }
    },
    [dataSource, loadedPath, agentConfig.model_id, playtest.playtestSaveId, playtest.playtestRestoreId, playtest.playtestRestoreLatest],
  );

  const submit = useCallback(async (): Promise<PlaytestRunResult> => {
    return runAndAppend(playtest.playtestInput);
  }, [runAndAppend, playtest.playtestInput]);

  const submitWith = useCallback(
    async (intent: string): Promise<PlaytestRunResult> => {
      // Mirror the intent into the shared input box so the rail shows what
      // was submitted, then run with the argument directly.
      playtest.setPlaytestInput(intent);
      return runAndAppend(intent);
    },
    [runAndAppend, playtest],
  );

  const setInput = playtest.setPlaytestInput;
  const canSubmit = Boolean(playtest.playtestInput.trim()) && !running;

  return {
    input: playtest.playtestInput,
    setInput,
    turns,
    running,
    canSubmit,
    submit,
    submitWith,
  };
}
