import type { ProjectData } from "../../../contracts/plotforge";
import {
  demoPlayOnceReport,
  demoProjectData,
  demoProjectPath,
  demoSourceContents,
  demoSourceFiles,
} from "./demoStudioData";
import {
  studioBridge,
  type PlayOnceReport,
  type ProjectCheckReport,
  type SourceFileContent,
  type SourceFileSummary,
} from "./tauriBridge";

export interface StudioDataSource {
  runtimeName: string;
  openProject(path: string): Promise<ProjectData>;
  checkProject(path: string): Promise<ProjectCheckReport>;
  playOnceProject(path: string, playerInput: string): Promise<PlayOnceReport>;
  listSourceFiles(path: string): Promise<SourceFileSummary[]>;
  readSourceFile(path: string, relativePath: string): Promise<SourceFileContent>;
  writeSourceFile(
    path: string,
    relativePath: string,
    content: string,
  ): Promise<SourceFileContent>;
}

export function createTauriStudioDataSource(): StudioDataSource {
  return {
    runtimeName: "Tauri desktop",
    openProject: studioBridge.openProject,
    checkProject: studioBridge.checkProject,
    playOnceProject: studioBridge.playOnceProject,
    listSourceFiles: studioBridge.listSourceFiles,
    readSourceFile: studioBridge.readSourceFile,
    writeSourceFile: studioBridge.writeSourceFile,
  };
}

export function createBrowserPreviewDataSource(): StudioDataSource {
  const files = new Map(
    Object.entries(demoSourceContents).map(([path, file]) => [
      path,
      { ...file },
    ]),
  );

  return {
    runtimeName: "Browser preview",
    async openProject() {
      return structuredClone(demoProjectData);
    },
    async checkProject() {
      return {
        title: demoProjectData.game.title,
        entry_scene: demoProjectData.game.entry_scene,
        scene_count: demoProjectData.scenes.length,
        rule_count: demoProjectData.rules.length,
        character_count: demoProjectData.characters.length,
      };
    },
    async playOnceProject(_path, playerInput) {
      return demoPlayOnceReport(playerInput);
    },
    async listSourceFiles() {
      return demoSourceFiles.map((file) => ({ ...file }));
    },
    async readSourceFile(_path, relativePath) {
      const file = files.get(relativePath);
      if (!file) {
        throw new Error(`unknown demo source file: ${relativePath}`);
      }
      return { ...file };
    },
    async writeSourceFile(_path, relativePath, content) {
      const file = files.get(relativePath);
      if (!file) {
        throw new Error(`unknown demo source file: ${relativePath}`);
      }
      if (!file.editable) {
        throw new Error(`${relativePath} is not editable`);
      }
      const updated = { ...file, content };
      files.set(relativePath, updated);
      return { ...updated };
    },
  };
}

export function createDefaultStudioDataSource(): StudioDataSource {
  return isTauriRuntime()
    ? createTauriStudioDataSource()
    : createBrowserPreviewDataSource();
}

export function defaultProjectPath() {
  return demoProjectPath;
}

function isTauriRuntime() {
  return Boolean(
    typeof window !== "undefined" &&
      (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__,
  );
}
