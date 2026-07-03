import { describe, expect, it } from "vitest";
import {
  getStudioSection,
  studioSectionIds,
  studioSections,
} from "./studioModel";

describe("studioModel", () => {
  it("exposes a flat, unique, non-empty set of section ids", () => {
    expect(studioSectionIds.length).toBe(studioSections.length);
    expect(new Set(studioSectionIds).size).toBe(studioSectionIds.length);
    expect(studioSectionIds.length).toBeGreaterThan(0);
  });

  it("resolves every section id via getStudioSection", () => {
    for (const id of studioSectionIds) {
      expect(() => getStudioSection(id)).not.toThrow();
      const section = getStudioSection(id);
      expect(section.id).toBe(id);
      expect(section.labelKey.length).toBeGreaterThan(0);
      expect(section.descriptionKey.length).toBeGreaterThan(0);
    }
  });

  it("throws on unknown section id", () => {
    expect(() => getStudioSection("unknown" as never)).toThrow(/Unknown Studio section/);
  });

  it("keeps the flat default-navigation section (no workflow grouping)", () => {
    expect(studioSectionIds).not.toContain("dashboard");
    expect(studioSectionIds).toContain("home");
    expect(studioSectionIds).toContain("play");
    expect(studioSectionIds).toContain("trace");
  });
});
