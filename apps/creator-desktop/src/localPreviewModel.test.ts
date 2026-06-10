import { describe, expect, it } from "vitest";
import {
  defaultLocalPreviewState,
  localPreviewSummary,
} from "./localPreviewModel";

describe("localPreviewModel", () => {
  it("names agent mesh and approval data as local preview state only", () => {
    expect(defaultLocalPreviewState.source).toBe("local-preview-only");
    expect(defaultLocalPreviewState.authoritativeProjectState).toBe(false);
    expect(defaultLocalPreviewState.usesStudioDataSourcePort).toBe(false);
    expect(defaultLocalPreviewState.networkEnabled).toBe(false);
    expect(defaultLocalPreviewState.workers.map((worker) => worker.id)).toEqual([
      "director-agent",
      "codex-worker",
      "claude-code-worker",
      "story-agent",
      "rules-agent",
      "asset-agent",
      "playtest-agent",
      "export-agent",
    ]);
    expect(defaultLocalPreviewState.workers.map((worker) => worker.kind))
      .toContain("external-worker");
    expect(defaultLocalPreviewState.capabilities.map((capability) => capability.id))
      .toEqual([
        "StoryCraft.review",
        "Rules.patch",
        "Runtime.playtest",
        "Assets.generate",
        "Export.package",
      ]);
    expect(defaultLocalPreviewState.approvals.length).toBeGreaterThan(0);
    expect(defaultLocalPreviewState.buildRun.timeline.map((step) => step.id))
      .toEqual(["plan", "generate", "patch", "validate", "playtest", "review"]);
    expect(defaultLocalPreviewState.artifactBundle.changes.map((change) => change.id))
      .toEqual(["story-contract", "rule-patch", "scene-asset", "risk-notes"]);
    expect(
      defaultLocalPreviewState.artifactBundle.validationEvidence.map(
        (evidence) => evidence.id,
      ),
    ).toEqual(["schema-valid", "links-valid", "determinism", "safety-boundary"]);
    expect(defaultLocalPreviewState.capabilityPolicy.allowedPaths).toContain(
      "/projects/dynasty-embers/",
    );
    expect(defaultLocalPreviewState.capabilityPolicy.blockedPaths).toContain(
      "/config/provider-keys/",
    );
  });

  it("summarizes preview evidence without creating a project truth source", () => {
    expect(localPreviewSummary(defaultLocalPreviewState)).toEqual({
      workerCount: 8,
      externalWorkerCount: 2,
      plotforgeSubagentCount: 5,
      capabilityCount: 5,
      approvalRequiredCapabilityCount: 4,
      pendingApprovalCount: 1,
      boundaryCount: 3,
      buildStepCount: 6,
      artifactChangeCount: 4,
      validationCheckCount: 4,
      isAuthoritativeProjectState: false,
      usesStudioDataSourcePort: false,
      networkEnabled: false,
    });
  });
});
