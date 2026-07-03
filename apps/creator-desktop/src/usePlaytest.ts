import { useRef, useState } from "react";
import type { StudioDataSource } from "./studioDataSource";
import type { PlayOnceReport } from "./tauriBridge";
import { errorMessage } from "./errorMessage";

export const defaultPlaytestInput =
  "Raise emergency taxes while auditing corrupt officials.";

export type PlaytestRunResult =
  | { succeeded: true; report: PlayOnceReport }
  | { succeeded: false; error: string; deduped?: boolean };

export interface PlaytestWorkspace {
  playtestInput: string;
  setPlaytestInput: React.Dispatch<React.SetStateAction<string>>;
  playtestSaveId: string;
  setPlaytestSaveId: React.Dispatch<React.SetStateAction<string>>;
  playtestRestoreId: string;
  setPlaytestRestoreId: React.Dispatch<React.SetStateAction<string>>;
  playtestRestoreLatest: boolean;
  setPlaytestRestoreLatest: React.Dispatch<React.SetStateAction<boolean>>;
  playtestReport: PlayOnceReport | null;
  setPlaytestReport: React.Dispatch<React.SetStateAction<PlayOnceReport | null>>;
  playtesting: boolean;
  playtestError: string | null;
  setPlaytestError: React.Dispatch<React.SetStateAction<string | null>>;
  runPlaytest(
    loadedPath: string,
    intentOverride?: string,
  ): Promise<PlaytestRunResult>;
  resetPlaytest(): void;
}

export function usePlaytest(
  dataSource: StudioDataSource,
): PlaytestWorkspace {
  const [playtestInput, setPlaytestInput] = useState(defaultPlaytestInput);
  const [playtestSaveId, setPlaytestSaveId] = useState("");
  const [playtestRestoreId, setPlaytestRestoreId] = useState("");
  const [playtestRestoreLatest, setPlaytestRestoreLatest] = useState(false);
  const [playtestReport, setPlaytestReport] = useState<PlayOnceReport | null>(
    null,
  );
  const [playtesting, setPlaytesting] = useState(false);
  const [playtestError, setPlaytestError] = useState<string | null>(null);

  // Synchronous in-flight guard so two rapid submits (e.g. double-clicking a
  // Play choice before `playtesting` state commits) cannot both enter
  // `runPlaytest` and race the report/error state.
  const runningRef = useRef(false);

  async function runPlaytest(
    loadedPath: string,
    intentOverride?: string,
  ): Promise<PlaytestRunResult> {
    if (runningRef.current) {
      // Mark `deduped` so callers (e.g. useAgentConversation) can distinguish
      // a rejected concurrent submit from a real backend failure and skip
      // surfacing it as a chat error turn.
      return {
        succeeded: false,
        error: "A playtest turn is already running.",
        deduped: true,
      };
    }
    const input = (intentOverride ?? playtestInput).trim();
    if (!input) {
      const message = "Playtest input is required.";
      setPlaytestError(message);
      return { succeeded: false, error: message };
    }

    runningRef.current = true;
    setPlaytesting(true);
    setPlaytestError(null);
    try {
      const saveId = playtestSaveId.trim() || null;
      const restoreId = playtestRestoreId.trim();
      const report = playtestRestoreLatest
        ? await dataSource.playOnceProjectFromLatestSnapshot(
            loadedPath,
            input,
            saveId,
          )
        : restoreId
          ? await dataSource.playOnceProjectFromSnapshot(
              loadedPath,
              input,
              restoreId,
              saveId,
            )
          : saveId
            ? await dataSource.playOnceProjectWithSave(
                loadedPath,
                input,
                saveId,
              )
            : await dataSource.playOnceProject(loadedPath, input);
      setPlaytestReport(report);
      return { succeeded: true, report };
    } catch (source) {
      const message = errorMessage(source);
      setPlaytestError(message);
      return { succeeded: false, error: message };
    } finally {
      runningRef.current = false;
      setPlaytesting(false);
    }
  }

  function resetPlaytest() {
    setPlaytestInput(defaultPlaytestInput);
    setPlaytestSaveId("");
    setPlaytestRestoreId("");
    setPlaytestRestoreLatest(false);
    setPlaytestReport(null);
    setPlaytestError(null);
  }

  return {
    playtestInput,
    setPlaytestInput,
    playtestSaveId,
    setPlaytestSaveId,
    playtestRestoreId,
    setPlaytestRestoreId,
    playtestRestoreLatest,
    setPlaytestRestoreLatest,
    playtestReport,
    setPlaytestReport,
    playtesting,
    playtestError,
    setPlaytestError,
    runPlaytest,
    resetPlaytest,
  };
}
