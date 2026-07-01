import type { Beat, Scene } from "../../../contracts/plotforge";

export function resolveSceneBeat(
  scene: Scene | null | undefined,
  currentBeatId: string | null | undefined,
): Beat | null {
  if (!scene) {
    return null;
  }

  return (
    findBeat(scene, currentBeatId) ??
    findBeat(scene, scene.entry_beat_id) ??
    scene.beats[0] ??
    null
  );
}

export function resolveScenePreviewImage(_: {
  scene: Scene | null | undefined;
  projectId?: string | null;
  loadedPath?: string | null;
}): string | null {
  return null;
}

function findBeat(scene: Scene, beatId: string | null | undefined): Beat | null {
  if (!beatId) {
    return null;
  }

  return scene.beats.find((beat) => beat.id === beatId) ?? null;
}
