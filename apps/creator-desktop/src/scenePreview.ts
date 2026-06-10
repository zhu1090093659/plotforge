import type { Beat, Scene } from "../../../contracts/plotforge";

const dynastyEmbersSceneKey = "court-crisis-001";
const dynastyEmbersProjectId = "dynasty-embers";
const dynastyEmbersBackgroundAsset = "assets/generated/court-crisis-001.png";
const dynastyEmbersCourtImage = new URL(
  "../../../examples/dynasty-embers/assets/generated/court-crisis-001.png",
  import.meta.url,
).href;

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

export function resolveScenePreviewImage({
  scene,
  projectId,
  loadedPath,
}: {
  scene: Scene | null | undefined;
  projectId?: string | null;
  loadedPath?: string | null;
}): string | null {
  if (
    !scene ||
    scene.key !== dynastyEmbersSceneKey ||
    scene.background_asset !== dynastyEmbersBackgroundAsset
  ) {
    return null;
  }

  if (projectId === dynastyEmbersProjectId) {
    return dynastyEmbersCourtImage;
  }

  return loadedPath && basenameWithoutTrailingSlash(loadedPath) === dynastyEmbersProjectId
    ? dynastyEmbersCourtImage
    : null;
}

function findBeat(scene: Scene, beatId: string | null | undefined): Beat | null {
  if (!beatId) {
    return null;
  }

  return scene.beats.find((beat) => beat.id === beatId) ?? null;
}

function basenameWithoutTrailingSlash(path: string): string {
  return path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? "";
}
