import { describe, expect, it } from "vitest";
import { demoProjectData } from "./demoStudioData";
import { resolveSceneBeat, resolveScenePreviewImage } from "./scenePreview";

describe("scenePreview", () => {
  it("resolves the current beat before falling back to the entry beat", () => {
    const scene = demoProjectData.scenes[0];

    expect(resolveSceneBeat(scene, "court-crisis-001-beat-002")?.text)
      .toContain("war minister");
    expect(resolveSceneBeat(scene, "missing-beat")?.id).toBe(
      "court-crisis-001-beat-001",
    );
  });

  it("only maps the bundled demo image for the Dynasty Embers demo context", () => {
    const scene = demoProjectData.scenes[0];

    expect(
      resolveScenePreviewImage({
        scene,
        projectId: "dynasty-embers",
        loadedPath: "/tmp/dynasty-embers",
      }),
    ).toContain("court-crisis-001");
    expect(
      resolveScenePreviewImage({
        scene,
        projectId: "other-project",
        loadedPath: "/tmp/other-project",
      }),
    ).toBeNull();
  });
});
