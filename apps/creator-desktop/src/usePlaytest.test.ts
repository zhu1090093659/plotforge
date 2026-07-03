import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { StudioDataSource } from "./studioDataSource";
import { demoPlayOnceReport } from "./demoStudioData";
import { usePlaytest } from "./usePlaytest";

// A controllable data source whose playOnce* methods record their call args
// and whose resolution is gated on an externally-held deferred. This lets us
// hold a run in-flight, fire a second submit, and assert the synchronous
// `runningRef` guard rejects it before the first resolves.
interface ControllableCalls {
  playOnceProject: unknown[];
  playOnceProjectWithSave: unknown[];
  playOnceProjectFromSnapshot: unknown[];
  playOnceProjectFromLatestSnapshot: unknown[];
}

function createDeferred() {
  let resolve!: (value: void) => void;
  const promise = new Promise<void>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

function makeDataSource(reportFor = demoPlayOnceReport("continue")) {
  const calls: ControllableCalls = {
    playOnceProject: [],
    playOnceProjectWithSave: [],
    playOnceProjectFromSnapshot: [],
    playOnceProjectFromLatestSnapshot: [],
  };
  const deferreds = {
    playOnceProject: createDeferred(),
    playOnceProjectWithSave: createDeferred(),
    playOnceProjectFromSnapshot: createDeferred(),
    playOnceProjectFromLatestSnapshot: createDeferred(),
  };
  const dataSource = {
    runtimeName: "test bridge",
    playOnceProject: vi.fn(async (path: string, playerInput: string) => {
      calls.playOnceProject.push({ path, playerInput });
      await deferreds.playOnceProject.promise;
      return reportFor;
    }),
    playOnceProjectWithSave: vi.fn(
      async (path: string, playerInput: string, saveId: string) => {
        calls.playOnceProjectWithSave.push({ path, playerInput, saveId });
        await deferreds.playOnceProjectWithSave.promise;
        return reportFor;
      },
    ),
    playOnceProjectFromSnapshot: vi.fn(
      async (
        path: string,
        playerInput: string,
        snapshotId: string,
        saveId?: string | null,
      ) => {
        calls.playOnceProjectFromSnapshot.push({
          path,
          playerInput,
          snapshotId,
          saveId,
        });
        await deferreds.playOnceProjectFromSnapshot.promise;
        return reportFor;
      },
    ),
    playOnceProjectFromLatestSnapshot: vi.fn(
      async (path: string, playerInput: string, saveId?: string | null) => {
        calls.playOnceProjectFromLatestSnapshot.push({
          path,
          playerInput,
          saveId,
        });
        await deferreds.playOnceProjectFromLatestSnapshot.promise;
        return reportFor;
      },
    ),
  } as unknown as StudioDataSource;
  return { dataSource, calls, deferreds };
}

describe("usePlaytest", () => {
  it("a default run (empty saveId, no restore) routes to plain playOnceProject with no save", async () => {
    // Pins the contract: default playtestSaveId is "" so a vanilla run must
    // NOT take a save/snapshot path. Regression guard against a silent revert
    // of the playtestSaveId default to "save-001".
    const { dataSource, calls, deferreds } = makeDataSource();
    const { result } = renderHook(() => usePlaytest(dataSource));

    expect(result.current.playtestSaveId).toBe("");
    expect(result.current.playtestRestoreId).toBe("");
    expect(result.current.playtestRestoreLatest).toBe(false);

    let runResult:
      | { succeeded: boolean; error?: string; deduped?: boolean }
      | undefined;
    await act(async () => {
      deferreds.playOnceProject.resolve();
      runResult = await result.current.runPlaytest("/tmp/proj", "continue");
    });

    expect(runResult?.succeeded).toBe(true);
    expect(calls.playOnceProject).toEqual([
      { path: "/tmp/proj", playerInput: "continue" },
    ]);
    // No save/snapshot routing when saveId/restoreId/restoreLatest are all empty.
    expect(calls.playOnceProjectWithSave).toHaveLength(0);
    expect(calls.playOnceProjectFromSnapshot).toHaveLength(0);
    expect(calls.playOnceProjectFromLatestSnapshot).toHaveLength(0);
  });

  it("a synchronous second submit is deduped (runningRef guard) and only one data source call fires", async () => {
    const { dataSource, calls, deferreds } = makeDataSource();
    const { result } = renderHook(() => usePlaytest(dataSource));

    // Kick off the first run; it is held in-flight by the deferred.
    let firstRun:
      | { succeeded: boolean; error?: string; deduped?: boolean }
      | undefined;
    act(() => {
      void result.current.runPlaytest("/tmp/proj", "first").then((r) => {
        firstRun = r;
      });
    });

    // Fire a second submit before the first resolves. The synchronous
    // `runningRef` guard must reject it immediately with deduped=true, and
    // must NOT trigger a second data source call.
    let secondResult:
      | { succeeded: boolean; error?: string; deduped?: boolean }
      | undefined;
    await act(async () => {
      secondResult = await result.current.runPlaytest("/tmp/proj", "second");
    });

    expect(secondResult?.succeeded).toBe(false);
    expect(secondResult?.deduped).toBe(true);
    expect(secondResult?.error).toBe("A playtest turn is already running.");
    // Only the first run reached the data source.
    expect(calls.playOnceProject).toEqual([
      { path: "/tmp/proj", playerInput: "first" },
    ]);

    // Resolve the first run; it succeeds normally.
    await act(async () => {
      deferreds.playOnceProject.resolve();
    });
    await waitFor(() => {
      expect(firstRun?.succeeded).toBe(true);
    });
    expect(firstRun?.deduped).toBeUndefined();
  });

  it("routes to playOnceProjectWithSave when a saveId is set", async () => {
    const { dataSource, calls, deferreds } = makeDataSource();
    const { result } = renderHook(() => usePlaytest(dataSource));

    await act(async () => {
      result.current.setPlaytestSaveId("save-after-turn");
    });
    await act(async () => {
      deferreds.playOnceProjectWithSave.resolve();
      await result.current.runPlaytest("/tmp/proj", "pay the army");
    });

    expect(calls.playOnceProjectWithSave).toEqual([
      {
        path: "/tmp/proj",
        playerInput: "pay the army",
        saveId: "save-after-turn",
      },
    ]);
    expect(calls.playOnceProject).toHaveLength(0);
  });

  it("routes to playOnceProjectFromSnapshot when a restoreId is set", async () => {
    const { dataSource, calls, deferreds } = makeDataSource();
    const { result } = renderHook(() => usePlaytest(dataSource));

    await act(async () => {
      result.current.setPlaytestSaveId("save-next");
      result.current.setPlaytestRestoreId("save-prev");
    });
    await act(async () => {
      deferreds.playOnceProjectFromSnapshot.resolve();
      await result.current.runPlaytest("/tmp/proj", "raise taxes");
    });

    expect(calls.playOnceProjectFromSnapshot).toEqual([
      {
        path: "/tmp/proj",
        playerInput: "raise taxes",
        snapshotId: "save-prev",
        saveId: "save-next",
      },
    ]);
    expect(calls.playOnceProjectWithSave).toHaveLength(0);
  });

  it("routes to playOnceProjectFromLatestSnapshot when restoreLatest is checked", async () => {
    const { dataSource, calls, deferreds } = makeDataSource();
    const { result } = renderHook(() => usePlaytest(dataSource));

    await act(async () => {
      result.current.setPlaytestRestoreLatest(true);
      result.current.setPlaytestSaveId("save-after");
    });
    await act(async () => {
      deferreds.playOnceProjectFromLatestSnapshot.resolve();
      await result.current.runPlaytest("/tmp/proj", "advance the plot");
    });

    expect(calls.playOnceProjectFromLatestSnapshot).toEqual([
      {
        path: "/tmp/proj",
        playerInput: "advance the plot",
        saveId: "save-after",
      },
    ]);
    expect(calls.playOnceProjectFromSnapshot).toHaveLength(0);
  });

  it("rejects empty input (after trim) and never calls the data source", async () => {
    const { dataSource, calls } = makeDataSource();
    const { result } = renderHook(() => usePlaytest(dataSource));

    let runResult:
      | { succeeded: boolean; error?: string; deduped?: boolean }
      | undefined;
    await act(async () => {
      runResult = await result.current.runPlaytest("/tmp/proj", "   ");
    });

    expect(runResult?.succeeded).toBe(false);
    expect(runResult?.deduped).toBeUndefined();
    expect(result.current.playtestError).toBeTruthy();
    expect(calls.playOnceProject).toHaveLength(0);
  });

  it("intentOverride wins over shared playtestInput state", async () => {
    // Regression guard for the choice-click flow: an explicit intent argument
    // must be submitted verbatim, not the stale shared input.
    const { dataSource, calls, deferreds } = makeDataSource();
    const { result } = renderHook(() => usePlaytest(dataSource));

    await act(async () => {
      deferreds.playOnceProject.resolve();
      await result.current.runPlaytest("/tmp/proj", "clicked choice label");
    });

    expect(calls.playOnceProject).toEqual([
      { path: "/tmp/proj", playerInput: "clicked choice label" },
    ]);
    // The shared input is unchanged when an override is used.
    expect(result.current.playtestInput).not.toBe("clicked choice label");
  });
});
