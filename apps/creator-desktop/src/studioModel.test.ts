import { describe, expect, it } from "vitest";
import {
  agentNativeWorkflows,
  defaultSectionForWorkflow,
  getAgentNativeWorkflow,
  getStudioSection,
  isSectionInWorkflow,
  studioSectionIds,
  workflowForSection,
} from "./studioModel";

describe("studioModel", () => {
  it("keeps workflow and section identifiers typed, unique, and non-overlapping", () => {
    const workflowIds = agentNativeWorkflows.map((workflow) => workflow.id);

    expect(workflowIds).toEqual([
      "command",
      "game",
      "agents",
      "artifacts",
      "proof",
      "export",
    ]);
    expect(new Set(workflowIds).size).toBe(workflowIds.length);
    expect(new Set(studioSectionIds).size).toBe(studioSectionIds.length);
    const sectionIds = new Set<string>(studioSectionIds);
    expect(workflowIds.filter((id) => sectionIds.has(id))).toEqual([]);
  });

  it("assigns every workflow to valid default sections and sections", () => {
    for (const workflow of agentNativeWorkflows) {
      expect(workflow.sectionIds).toContain(workflow.defaultSectionId);
      expect(() => getStudioSection(workflow.defaultSectionId)).not.toThrow();

      for (const sectionId of workflow.sectionIds) {
        expect(studioSectionIds).toContain(sectionId);
      }
    }
  });

  it("assigns each navigable section to one workflow", () => {
    expect(getAgentNativeWorkflow("export").labelKey).toBe("workflow.export.label");
    expect(defaultSectionForWorkflow("export")).toBe("export-kit");
    expect(workflowForSection("assets").id).toBe("artifacts");
    expect(isSectionInWorkflow("assets", "export")).toBe(false);
    expect(studioSectionIds).not.toContain("dashboard");
    expect(isSectionInWorkflow("launchpad", "export")).toBe(false);

    const assignedSectionIds = agentNativeWorkflows.flatMap(
      (workflow) => workflow.sectionIds,
    );
    expect(new Set(assignedSectionIds).size).toBe(assignedSectionIds.length);
    expect(new Set(assignedSectionIds)).toEqual(new Set(studioSectionIds));
  });
});
