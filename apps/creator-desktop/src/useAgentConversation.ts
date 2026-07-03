import { useCallback, useRef, useState } from "react";
import type { PlayOnceReport } from "./tauriBridge";
import type { PlaytestWorkspace } from "./usePlaytest";

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
  submit(): Promise<void>;
}

export function useAgentConversation(
  playtest: PlaytestWorkspace,
  loadedPath: string,
): AgentConversationWorkspace {
  const [turns, setTurns] = useState<AgentTurn[]>([]);
  // Monotonic id counter (deterministic for tests).
  const idCounterRef = useRef(0);

  const submit = useCallback(async () => {
    const intent = playtest.playtestInput.trim();
    if (!intent || playtest.playtesting) {
      return;
    }
    const result = await playtest.runPlaytest(loadedPath);
    idCounterRef.current += 1;
    setTurns((prev) => [
      ...prev,
      {
        id: `agent-turn-${idCounterRef.current}`,
        intent,
        report: result.succeeded ? result.report : null,
        error: result.succeeded ? null : result.error,
      },
    ]);
  }, [playtest, loadedPath]);

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
  };
}
