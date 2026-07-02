import { demoPlayOnceReport, demoProjectData, demoReproducibilityMetadata } from "../demoStudioData";
import type { PlayOnceReport } from "../tauriBridge";

/**
 * Build a `PlayOnceReport` with an attached runtime snapshot, reused by
 * snapshot-related mock data sources so the snapshot shape stays consistent.
 */
export function playOnceReportWithSnapshot(
  playerInput: string,
  snapshotId: string,
  snapshotPath: string,
): PlayOnceReport {
  const report = demoPlayOnceReport(playerInput);
  return {
    ...report,
    snapshot: {
      id: snapshotId,
      timestamp_ms: report.trace.timestamp_ms,
      reproducibility: {
        ...demoReproducibilityMetadata,
        snapshot_id: snapshotId,
      },
      project_id: demoProjectData.game.id,
      project_version: demoProjectData.game.version,
      story_state: report.trace.story_state_after,
      world_state: report.trace.world_state_after,
      scenes: demoProjectData.scenes,
    },
    snapshot_path: snapshotPath,
  };
}

/**
 * Mock `playOnceProjectWithSave` shared across App tests: returns a report
 * with a snapshot whose id/path derive from the requested save id.
 */
export async function mockPlayOnceProjectWithSave(
  _path: string,
  playerInput: string,
  saveId: string,
): Promise<PlayOnceReport> {
  const report = demoPlayOnceReport(playerInput);
  return {
    ...report,
    snapshot: {
      id: saveId,
      timestamp_ms: report.trace.timestamp_ms,
      reproducibility: {
        ...demoReproducibilityMetadata,
        snapshot_id: saveId,
      },
      project_id: demoProjectData.game.id,
      project_version: demoProjectData.game.version,
      story_state: report.trace.story_state_after,
      world_state: report.trace.world_state_after,
      scenes: demoProjectData.scenes,
    },
    snapshot_path: `/tmp/starter-project/saves/${saveId}.runtime_snapshot.json`,
  };
}

/**
 * Mock `playOnceProjectFromSnapshot`: with a saveId, behave like
 * `mockPlayOnceProjectWithSave`; without one, return the plain report.
 */
export async function mockPlayOnceProjectFromSnapshot(
  path: string,
  playerInput: string,
  _snapshotId: string,
  saveId: string | null = null,
): Promise<PlayOnceReport> {
  return saveId
    ? mockPlayOnceProjectWithSave(path, playerInput, saveId)
    : demoPlayOnceReport(playerInput);
}

/**
 * Mock `playOnceProjectFromLatestSnapshot`: with a saveId, behave like
 * `mockPlayOnceProjectWithSave`; without one, return the plain report.
 */
export async function mockPlayOnceProjectFromLatestSnapshot(
  path: string,
  playerInput: string,
  saveId: string | null = null,
): Promise<PlayOnceReport> {
  return saveId
    ? mockPlayOnceProjectWithSave(path, playerInput, saveId)
    : demoPlayOnceReport(playerInput);
}
