import { useState } from "react";
import type { StudioDataSource } from "./studioDataSource";
import type { PlayOnceReport } from "./tauriBridge";
import { errorMessage } from "./errorMessage";

export const defaultPlaytestInput =
  "Raise emergency taxes while auditing corrupt officials.";

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
  runPlaytest(loadedPath: string): Promise<boolean>;
  resetPlaytest(): void;
}

export function usePlaytest(
  dataSource: StudioDataSource,
): PlaytestWorkspace {
  const [playtestInput, setPlaytestInput] = useState(defaultPlaytestInput);
  const [playtestSaveId, setPlaytestSaveId] = useState("save-001");
  const [playtestRestoreId, setPlaytestRestoreId] = useState("");
  const [playtestRestoreLatest, setPlaytestRestoreLatest] = useState(false);
  const [playtestReport, setPlaytestReport] = useState<PlayOnceReport | null>(
    null,
  );
  const [playtesting, setPlaytesting] = useState(false);
  const [playtestError, setPlaytestError] = useState<string | null>(null);

  async function runPlaytest(loadedPath: string): Promise<boolean> {
    const input = playtestInput.trim();
    if (!input) {
      setPlaytestError("Playtest input is required.");
      return false;
    }

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
      return true;
    } catch (source) {
      setPlaytestError(errorMessage(source));
      return false;
    } finally {
      setPlaytesting(false);
    }
  }

  function resetPlaytest() {
    setPlaytestInput(defaultPlaytestInput);
    setPlaytestSaveId("save-001");
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
