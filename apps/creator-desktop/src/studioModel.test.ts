import { describe, expect, it } from "vitest";
import {
  agentNativeScreenReferences,
  agentNativeWorkflows,
  defaultSectionForWorkflow,
  getAgentNativeWorkflow,
  getStudioSection,
  isSectionInWorkflow,
  screenReferencesForWorkflow,
  studioSectionIds,
  workflowForSection,
} from "./studioModel";

describe("studioModel", () => {
  it("references the complete agent-native mockup screen set", () => {
    expect(agentNativeScreenReferences.map((screen) => screen.fileName)).toEqual([
      "00-agent-mesh-core.png",
      "01-project-launchpad.png",
      "02-command-center.png",
      "03-director-mode.png",
      "04-acp-bridge-setup.png",
      "05-live-build-room.png",
      "06-artifact-review.png",
      "07-playable-proof.png",
      "08-trace-debug.png",
      "09-export-package.png",
    ]);

    expect(screenReferencesForWorkflow("command").map((screen) => screen.title))
      .toEqual(["Project Launchpad", "Command Center"]);
  });

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

  it("assigns every workflow to valid default sections, sections, and screen references", () => {
    const screenIds = new Set(agentNativeScreenReferences.map((screen) => screen.id));

    for (const workflow of agentNativeWorkflows) {
      expect(workflow.sectionIds).toContain(workflow.defaultSectionId);
      expect(() => getStudioSection(workflow.defaultSectionId)).not.toThrow();

      for (const sectionId of workflow.sectionIds) {
        expect(studioSectionIds).toContain(sectionId);
      }

      for (const screenId of workflow.screenIds) {
        expect(screenIds.has(screenId)).toBe(true);
      }
    }
  });

  it("routes shared sections through their canonical workflow without hiding export-specific access", () => {
    expect(getAgentNativeWorkflow("export").label).toBe("Export Package");
    expect(defaultSectionForWorkflow("export")).toBe("export-kit");
    expect(workflowForSection("assets").id).toBe("artifacts");
    expect(isSectionInWorkflow("assets", "export")).toBe(true);
    expect(studioSectionIds).not.toContain("dashboard");
    expect(isSectionInWorkflow("launchpad", "export")).toBe(false);
  });
});
