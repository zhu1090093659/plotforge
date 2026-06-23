import type { AssetRecord, ProjectData } from "../../../contracts/plotforge";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type AssetCatalogItem =
  | {
      source: "record";
      record: AssetRecord;
    }
  | {
      source: "scene-background-fallback";
      path: string;
    };

export interface AssetCatalog {
  items: AssetCatalogItem[];
  source: "records" | "scene-background-fallback";
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

export function projectAssetCatalog(
  project: ProjectData | null,
  assetRecords: AssetRecord[] = [],
): AssetCatalog {
  const records = assetRecords.length > 0 ? assetRecords : project?.asset_records ?? [];
  if (records.length > 0) {
    return {
      source: "records",
      items: records.map((record) => ({ source: "record", record })),
    };
  }

  const fallbackPaths = Array.from(
    new Set(project?.scenes.map((scene) => scene.background_asset).filter(Boolean) ?? []),
  );
  return {
    source: "scene-background-fallback",
    items: fallbackPaths.map((path) => ({
      source: "scene-background-fallback",
      path,
    })),
  };
}
