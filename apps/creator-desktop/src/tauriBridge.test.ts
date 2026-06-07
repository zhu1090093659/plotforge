import { describe, expect, it } from "vitest";
import {
  createStudioBridge,
  studioCommandNames,
  type ProjectCheckReport,
  type StaticExportReport,
  type StudioInvoke,
} from "./tauriBridge";

describe("createStudioBridge", () => {
  it("maps typed frontend methods to Tauri command names and arguments", async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const invoke: StudioInvoke = async <T>(command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      if (command === studioCommandNames.checkProject) {
        const result = {
          title: "Dynasty Embers",
          entry_scene: "court-crisis-001",
          scene_count: 1,
          rule_count: 3,
          character_count: 6,
        } satisfies ProjectCheckReport;
        return result as T;
      }
      if (command === studioCommandNames.exportStaticProject) {
        const result = {
          output_dir: "/tmp/export",
          files_written: ["/tmp/export/index.html"],
          allowed_files: ["index.html"],
          files_found: ["index.html"],
        } satisfies StaticExportReport;
        return result as T;
      }
      return {} as never;
    };

    const bridge = createStudioBridge(invoke);

    await bridge.checkProject("/tmp/dynasty-embers");
    await bridge.playOnceProject("/tmp/dynasty-embers", "continue");
    await bridge.exportStaticProject("/tmp/dynasty-embers", "/tmp/export");

    expect(calls).toEqual([
      {
        command: "check_project",
        args: { path: "/tmp/dynasty-embers" },
      },
      {
        command: "play_once_project",
        args: { path: "/tmp/dynasty-embers", player_input: "continue" },
      },
      {
        command: "export_static_project",
        args: { path: "/tmp/dynasty-embers", output_dir: "/tmp/export" },
      },
    ]);
  });
});
