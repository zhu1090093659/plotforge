import { invoke } from "@tauri-apps/api/core";
import type { ProjectData, RuntimeTrace, Scene } from "../../../contracts/plotforge";

export const studioCommandNames = {
  openProject: "open_project",
  checkProject: "check_project",
  playOnceProject: "play_once_project",
  exportStaticProject: "export_static_project",
} as const;

export interface StudioCommandError {
  code: string;
  message: string;
}

export interface ProjectCheckReport {
  title: string;
  entry_scene: string;
  scene_count: number;
  rule_count: number;
  character_count: number;
}

export interface PlayOnceReport {
  scene: Scene;
  trace: RuntimeTrace;
  trace_path: string;
  delta_summary: string[];
}

export interface StaticExportReport {
  output_dir: string;
  files_written: string[];
  allowed_files: string[];
  files_found: string[];
}

export type StudioInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;

export function createStudioBridge(invokeCommand: StudioInvoke = invoke) {
  return {
    openProject(path: string): Promise<ProjectData> {
      return invokeCommand<ProjectData>(studioCommandNames.openProject, { path });
    },
    checkProject(path: string): Promise<ProjectCheckReport> {
      return invokeCommand<ProjectCheckReport>(studioCommandNames.checkProject, {
        path,
      });
    },
    playOnceProject(path: string, playerInput: string): Promise<PlayOnceReport> {
      return invokeCommand<PlayOnceReport>(studioCommandNames.playOnceProject, {
        path,
        player_input: playerInput,
      });
    },
    exportStaticProject(
      path: string,
      outputDir: string,
    ): Promise<StaticExportReport> {
      return invokeCommand<StaticExportReport>(
        studioCommandNames.exportStaticProject,
        {
          path,
          output_dir: outputDir,
        },
      );
    },
  };
}

export const studioBridge = createStudioBridge();
