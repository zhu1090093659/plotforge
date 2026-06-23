import { useMemo, useState } from "react";
import type { ExportProfile } from "../../../contracts/plotforge";
import type { StudioDataSource } from "./studioDataSource";
import type { StaticExportReport } from "./tauriBridge";
import { errorMessage } from "./errorMessage";

export interface ExportWorkspace {
  exportDir: string;
  setExportDir: React.Dispatch<React.SetStateAction<string>>;
  archivePath: string;
  setArchivePath: React.Dispatch<React.SetStateAction<string>>;
  exportProfiles: ExportProfile[];
  selectedExportProfileId: string;
  selectedExportProfile: ExportProfile | null;
  staticExportSelected: boolean;
  exportReport: StaticExportReport | null;
  setExportReport: React.Dispatch<
    React.SetStateAction<StaticExportReport | null>
  >;
  exporting: boolean;
  exportError: string | null;
  setExportError: React.Dispatch<React.SetStateAction<string | null>>;
  runStaticZipExport(loadedPath: string): Promise<void>;
  selectExportProfile(profileId: string): void;
  resetExport(path: string, profiles: ExportProfile[]): void;
  setExportProfiles: React.Dispatch<React.SetStateAction<ExportProfile[]>>;
}

export interface UseExportOptions {
  dataSource: StudioDataSource;
  initialProjectPath: string;
}

export function useExport({
  dataSource,
  initialProjectPath,
}: UseExportOptions): ExportWorkspace {
  const [exportDir, setExportDir] = useState(
    defaultStaticExportDir(initialProjectPath),
  );
  const [archivePath, setArchivePath] = useState(
    defaultStaticArchivePath(initialProjectPath),
  );
  const [exportProfiles, setExportProfiles] = useState<ExportProfile[]>([]);
  const [selectedExportProfileId, setSelectedExportProfileId] = useState("");
  const [exportReport, setExportReport] =
    useState<StaticExportReport | null>(null);
  const [exporting, setExporting] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);

  const selectedExportProfile = useMemo(
    () =>
      exportProfiles.find(
        (profile) => profile.id === selectedExportProfileId,
      ) ?? null,
    [exportProfiles, selectedExportProfileId],
  );
  const staticExportSelected =
    selectedExportProfile?.target === "static_web";

  async function runStaticZipExport(loadedPath: string) {
    if (!staticExportSelected) {
      setExportError(
        "Selected export profile has no executable Studio command.",
      );
      return;
    }

    const outputDir = exportDir.trim();
    const zipPath = archivePath.trim();
    if (!outputDir || !zipPath) {
      setExportError("Output directory and zip archive are required.");
      return;
    }

    setExporting(true);
    setExportError(null);
    try {
      const report = await dataSource.exportStaticProjectZip(
        loadedPath,
        outputDir,
        zipPath,
      );
      setExportReport(report);
    } catch (source) {
      setExportError(errorMessage(source));
    } finally {
      setExporting(false);
    }
  }

  function selectExportProfile(profileId: string) {
    setSelectedExportProfileId(profileId);
    setExportReport(null);
    setExportError(null);
  }

  function resetExport(path: string, profiles: ExportProfile[]) {
    setExportDir(defaultStaticExportDir(path));
    setArchivePath(defaultStaticArchivePath(path));
    setExportProfiles(profiles);
    setSelectedExportProfileId((currentId) =>
      resolveExportProfileId(profiles, currentId),
    );
    setExportReport(null);
    setExportError(null);
  }

  return {
    exportDir,
    setExportDir,
    archivePath,
    setArchivePath,
    exportProfiles,
    selectedExportProfileId,
    selectedExportProfile,
    staticExportSelected,
    exportReport,
    setExportReport,
    exporting,
    exportError,
    setExportError,
    runStaticZipExport,
    selectExportProfile,
    resetExport,
    setExportProfiles,
  };
}

export function defaultStaticExportDir(projectPath: string) {
  return `${trimTrailingSlashes(projectPath)}/exports/static`;
}

export function defaultStaticArchivePath(projectPath: string) {
  return `${trimTrailingSlashes(projectPath)}/exports/static.zip`;
}

function resolveExportProfileId(
  profiles: ExportProfile[],
  currentId: string,
) {
  if (profiles.some((profile) => profile.id === currentId)) {
    return currentId;
  }
  return profiles[0]?.id ?? "";
}

function trimTrailingSlashes(path: string) {
  return path.replace(/\/+$/, "") || ".";
}
