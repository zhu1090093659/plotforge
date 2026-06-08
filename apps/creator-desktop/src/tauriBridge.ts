import { invoke } from "@tauri-apps/api/core";
import type {
  ProjectData,
  RuntimeTrace,
  Scene,
} from "../../../contracts/plotforge";

export const studioCommandNames = {
  openProject: "open_project",
  checkProject: "check_project",
  playOnceProject: "play_once_project",
  exportStaticProject: "export_static_project",
  listSourceFiles: "list_source_files",
  readSourceFile: "read_source_file",
  writeSourceFile: "write_source_file",
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

export type SourceFileKind = "toml" | "json" | "markdown" | "prompt";

export interface SourceFileSummary {
  path: string;
  kind: SourceFileKind;
  bytes: number;
  editable: boolean;
}

export interface SourceFileContent {
  path: string;
  kind: SourceFileKind;
  editable: boolean;
  content: string;
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
    listSourceFiles(path: string): Promise<SourceFileSummary[]> {
      return invokeCommand<SourceFileSummary[]>(
        studioCommandNames.listSourceFiles,
        { path },
      );
    },
    readSourceFile(
      path: string,
      relativePath: string,
    ): Promise<SourceFileContent> {
      return invokeCommand<SourceFileContent>(studioCommandNames.readSourceFile, {
        path,
        relative_path: relativePath,
      });
    },
    writeSourceFile(
      path: string,
      relativePath: string,
      content: string,
    ): Promise<SourceFileContent> {
      return invokeCommand<SourceFileContent>(
        studioCommandNames.writeSourceFile,
        {
          path,
          relative_path: relativePath,
          content,
        },
      );
    },
  };
}

export const studioBridge = createStudioBridge();
