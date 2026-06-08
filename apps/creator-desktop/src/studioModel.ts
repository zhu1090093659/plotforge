import type { LucideIcon } from "lucide-react";
import type { AssetRecord, ProjectData } from "../../../contracts/plotforge";
import {
  Boxes,
  Bug,
  Gauge,
  KeyRound,
  Map,
  Network,
  Play,
  ScrollText,
  ShipWheel,
  Users,
} from "lucide-react";

export interface StudioSection {
  id: string;
  label: string;
  description: string;
  status: "ready" | "next" | "later";
  icon: LucideIcon;
}

export const studioSections: StudioSection[] = [
  {
    id: "dashboard",
    label: "Dashboard",
    description: "Project health, source files, recent checks",
    status: "ready",
    icon: Gauge,
  },
  {
    id: "world",
    label: "World Bible",
    description: "Canon, forbidden facts, setting notes",
    status: "next",
    icon: Map,
  },
  {
    id: "story",
    label: "Story Craft",
    description: "Promises, hooks, reversals, emotional arc",
    status: "next",
    icon: ScrollText,
  },
  {
    id: "characters",
    label: "Characters",
    description: "Roles, arcs, visual and voice cards",
    status: "ready",
    icon: Users,
  },
  {
    id: "state",
    label: "State",
    description: "Resources, flags, triggered events",
    status: "ready",
    icon: Network,
  },
  {
    id: "rules",
    label: "Rules",
    description: "Declarative conditions and effects",
    status: "ready",
    icon: KeyRound,
  },
  {
    id: "assets",
    label: "Assets",
    description: "Generated and imported files",
    status: "ready",
    icon: Boxes,
  },
  {
    id: "playtest",
    label: "Playtest",
    description: "Local runtime preview",
    status: "ready",
    icon: Play,
  },
  {
    id: "debugger",
    label: "Debugger",
    description: "Trace, diagnostics, review notes",
    status: "ready",
    icon: Bug,
  },
  {
    id: "export",
    label: "Export",
    description: "Static web and package profiles",
    status: "ready",
    icon: ShipWheel,
  },
];

export const sourceFiles = [
  "game.toml",
  "world/world.md",
  "world/canon.md",
  "world/forbidden_facts.json",
  "story/story_craft.toml",
  "characters/*.character.toml",
  "rules/rules.toml",
  "scenes/*.scene.json",
];

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
