import { convertFileSrc } from "@tauri-apps/api/core";
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

export function resolveScenePreviewImage({
  scene,
  loadedPath,
}: {
  scene: Scene | null | undefined;
  projectId?: string | null;
  loadedPath?: string | null;
}): string | null {
  const assetPath = scene?.background_asset ?? null;
  if (!assetPath || !loadedPath) {
    return null;
  }

  // Only resolve real image URLs when running inside the Tauri desktop
  // shell, where convertFileSrc can map a local file path onto the
  // asset:// protocol. In the HTTP dev bridge (or under jsdom in tests)
  // there is no such protocol, so the UI renders its explicit empty state.
  if (!isTauriRuntime()) {
    return null;
  }

  const fullPath = joinPath(loadedPath, assetPath);
  return convertFileSrc(fullPath);
}

function isTauriRuntime(): boolean {
  return Boolean(
    typeof window !== "undefined" &&
      (window as Window & { __TAURI_INTERNALS__?: unknown })
        .__TAURI_INTERNALS__,
  );
}

function joinPath(base: string, relative: string): string {
  if (!base) {
    return relative;
  }
  const trimmed = base.replace(/\/+$/, "");
  return `${trimmed}/${relative.replace(/^\/+/, "")}`;
}

function findBeat(scene: Scene, beatId: string | null | undefined): Beat | null {
  if (!beatId) {
    return null;
  }

  return scene.beats.find((beat) => beat.id === beatId) ?? null;
}
