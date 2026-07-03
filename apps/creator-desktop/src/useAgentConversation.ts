import { useCallback, useRef, useState } from "react";
import type { PlayOnceReport } from "./tauriBridge";
import type { PlaytestRunResult, PlaytestWorkspace } from "./usePlaytest";

// ---------------------------------------------------------------------------
// useAgentConversation — the Cursor-style agent chat state.
//
// The right rail is now an AGENT CHAT surface (not a static evidence panel).
// This hook accumulates conversation turns: each "submit" runs a proof turn
// through the existing playtest pipeline (no view switch — the creator stays
// on the current document) and appends a turn {intent, report, error}. The
// chat input IS the playtest director-intent (single source of truth) so
// Director Mode / Command Center stay in sync.
//
// The turn is appended directly from the run result so the chat does not rely
// on React committing playtestReport/playtestError before the async submit
// continuation resumes.
// ---------------------------------------------------------------------------

export interface AgentTurn {
  id: string;
  intent: string;
  report: PlayOnceReport | null;
  error: string | null;
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
   * intent into the shared input box so the chat rail stays in sync. This is
   * the only correct path for "click a choice → submit it" because
   * `setPlaytestInput` + `submit()` would read stale state inside the same
   * tick.
   */
  submitWith(intent: string): Promise<PlaytestRunResult>;
}

export function useAgentConversation(
  playtest: PlaytestWorkspace,
  loadedPath: string,
): AgentConversationWorkspace {
  const [turns, setTurns] = useState<AgentTurn[]>([]);
  // Monotonic id counter (deterministic for tests).
  const idCounterRef = useRef(0);

  const runAndAppend = useCallback(
    async (intent: string): Promise<PlaytestRunResult> => {
      const trimmed = intent.trim();
      if (!trimmed) {
        return {
          succeeded: false,
          error: "Playtest input is required.",
        };
      }
      if (playtest.playtesting) {
        // React-state mirror of usePlaytest's synchronous `runningRef` guard.
        // Same "already running" dedup case: do not surface as a chat turn.
        return {
          succeeded: false,
          error: "A playtest turn is already running.",
          deduped: true,
        };
      }
      const result = await playtest.runPlaytest(loadedPath, trimmed);
      // A deduped concurrent submit (caught by usePlaytest's `runningRef`
      // before React state committed `playtesting=true`) is not a real
      // failure — do not append a misleading error turn for it.
      if (result.succeeded || !result.deduped) {
        idCounterRef.current += 1;
        setTurns((prev) => [
          ...prev,
          {
            id: `agent-turn-${idCounterRef.current}`,
            intent: trimmed,
            report: result.succeeded ? result.report : null,
            error: result.succeeded ? null : result.error,
          },
        ]);
      }
      return result;
    },
    [playtest, loadedPath],
  );

  const submit = useCallback(async (): Promise<PlaytestRunResult> => {
    return runAndAppend(playtest.playtestInput);
  }, [runAndAppend, playtest.playtestInput]);

  const submitWith = useCallback(
    async (intent: string): Promise<PlaytestRunResult> => {
      // Mirror the intent into the shared input box so the rail shows what
      // was submitted, then run with the argument directly (not the
      // just-dispatched state, which is not yet visible synchronously).
      playtest.setPlaytestInput(intent);
      return runAndAppend(intent);
    },
    [runAndAppend, playtest],
  );

  const setInput = playtest.setPlaytestInput;
  const canSubmit =
    Boolean(playtest.playtestInput.trim()) && !playtest.playtesting;

  return {
    input: playtest.playtestInput,
    setInput,
    turns,
    running: playtest.playtesting,
    canSubmit,
    submit,
    submitWith,
  };
}
