import { describe, expect, it } from "vitest";
import { demoProjectData } from "./demoStudioData";
import { resolveSceneBeat, resolveScenePreviewImage } from "./scenePreview";

describe("scenePreview", () => {
  it("resolves the current beat before falling back to the entry beat", () => {
    const scene = demoProjectData.scenes[0];

    expect(resolveSceneBeat(scene, "opening-scene-beat-002")?.text)
      .toContain("war minister");
    expect(resolveSceneBeat(scene, "missing-beat")?.id).toBe(
      "opening-scene-beat-001",
    );
  });

  it("does not map bundled demo images", () => {
    const scene = demoProjectData.scenes[0];

    expect(
      resolveScenePreviewImage({
        scene,
        projectId: "starter-project",
        loadedPath: "/tmp/starter-project",
      }),
    ).toBeNull();
  });
});
