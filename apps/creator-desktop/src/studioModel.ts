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

export const agentNativeScreenIds = [
  "agent-mesh-core",
  "project-launchpad",
  "command-center",
  "director-mode",
  "acp-bridge-setup",
  "live-build-room",
  "artifact-review",
  "playable-proof",
  "trace-debug",
  "export-package",
] as const;

export type AgentNativeScreenId = (typeof agentNativeScreenIds)[number];

export interface AgentNativeScreenReference {
  id: AgentNativeScreenId;
  title: string;
  fileName: string;
}

export const agentNativeScreenReferences: readonly AgentNativeScreenReference[] = [
  {
    id: "agent-mesh-core",
    title: "Agent Mesh Core",
    fileName: "00-agent-mesh-core.png",
  },
  {
    id: "project-launchpad",
    title: "Project Launchpad",
    fileName: "01-project-launchpad.png",
  },
  {
    id: "command-center",
    title: "Command Center",
    fileName: "02-command-center.png",
  },
  {
    id: "director-mode",
    title: "Director Mode",
    fileName: "03-director-mode.png",
  },
  {
    id: "acp-bridge-setup",
    title: "ACP Bridge Setup",
    fileName: "04-acp-bridge-setup.png",
  },
  {
    id: "live-build-room",
    title: "Live Build Room",
    fileName: "05-live-build-room.png",
  },
  {
    id: "artifact-review",
    title: "Artifact Review",
    fileName: "06-artifact-review.png",
  },
  {
    id: "playable-proof",
    title: "Playable Proof",
    fileName: "07-playable-proof.png",
  },
  {
    id: "trace-debug",
    title: "Trace Debug",
    fileName: "08-trace-debug.png",
  },
  {
    id: "export-package",
    title: "Export Package",
    fileName: "09-export-package.png",
  },
];

export const studioSectionIds = [
  "launchpad",
  "agent-mesh",
  "world",
  "story",
  "characters",
  "state",
  "rules",
  "assets",
  "playtest",
  "debugger",
  "export-kit",
] as const;

export type StudioSectionId = (typeof studioSectionIds)[number];

export const agentNativeWorkflowIds = [
  "command",
  "game",
  "agents",
  "artifacts",
  "proof",
  "export",
] as const;

export type AgentNativeWorkflowId = (typeof agentNativeWorkflowIds)[number];

export interface StudioSection {
  id: StudioSectionId;
  label: string;
  description: string;
  status: "ready" | "next" | "later";
  icon: LucideIcon;
}

export const studioSections: StudioSection[] = [
  {
    id: "launchpad",
    label: "Launchpad",
    description: "Director intent, project health, artifact summary",
    status: "ready",
    icon: Gauge,
  },
  {
    id: "agent-mesh",
    label: "Agent Mesh",
    description: "ACP capability map, approvals, and local boundaries",
    status: "ready",
    icon: Network,
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
    id: "export-kit",
    label: "Export",
    description: "Static web and package profiles",
    status: "ready",
    icon: ShipWheel,
  },
];

export interface AgentNativeWorkflow {
  id: AgentNativeWorkflowId;
  label: string;
  shortLabel: string;
  description: string;
  status: "ready" | "next";
  icon: LucideIcon;
  defaultSectionId: StudioSectionId;
  sectionIds: readonly StudioSectionId[];
  screenIds: readonly AgentNativeScreenId[];
}

export const agentNativeWorkflows: readonly AgentNativeWorkflow[] = [
  {
    id: "command",
    label: "Command Center",
    shortLabel: "Command",
    description: "Director intent, project launch, active run summary",
    status: "ready",
    icon: Gauge,
    defaultSectionId: "launchpad",
    sectionIds: ["launchpad", "world", "story"],
    screenIds: ["project-launchpad", "command-center"],
  },
  {
    id: "game",
    label: "Director Mode",
    shortLabel: "Game",
    description: "Playable scene preview, creative direction, runtime loop",
    status: "ready",
    icon: Play,
    defaultSectionId: "playtest",
    sectionIds: ["playtest", "world", "story", "characters", "state", "rules"],
    screenIds: ["director-mode"],
  },
  {
    id: "agents",
    label: "Agent Mesh",
    shortLabel: "Agents",
    description: "Local capability map, ACP setup, approval boundaries",
    status: "ready",
    icon: Network,
    defaultSectionId: "agent-mesh",
    sectionIds: ["agent-mesh"],
    screenIds: ["agent-mesh-core", "acp-bridge-setup"],
  },
  {
    id: "artifacts",
    label: "Artifact Review",
    shortLabel: "Artifacts",
    description: "Agent proposals, changed assets, validation evidence",
    status: "ready",
    icon: Boxes,
    defaultSectionId: "assets",
    sectionIds: ["assets", "world", "story", "characters", "rules"],
    screenIds: ["live-build-room", "artifact-review"],
  },
  {
    id: "proof",
    label: "Playable Proof",
    shortLabel: "Proof",
    description: "Trace-visible playtest evidence and reproducibility",
    status: "ready",
    icon: Bug,
    defaultSectionId: "debugger",
    sectionIds: ["playtest", "debugger"],
    screenIds: ["playable-proof", "trace-debug"],
  },
  {
    id: "export",
    label: "Export Package",
    shortLabel: "Export",
    description: "Local package readiness, manifests, disclosure drafts",
    status: "ready",
    icon: ShipWheel,
    defaultSectionId: "export-kit",
    sectionIds: ["export-kit", "assets", "debugger"],
    screenIds: ["export-package"],
  },
];

export function getAgentNativeWorkflow(
  id: AgentNativeWorkflowId,
): AgentNativeWorkflow {
  const workflow = agentNativeWorkflows.find((candidate) => candidate.id === id);
  if (!workflow) {
    throw new Error(`Unknown agent-native workflow: ${id}`);
  }
  return workflow;
}

export function getStudioSection(id: StudioSectionId): StudioSection {
  const section = studioSections.find((candidate) => candidate.id === id);
  if (!section) {
    throw new Error(`Unknown Studio section: ${id}`);
  }
  return section;
}

export function getAgentNativeScreenReference(
  id: AgentNativeScreenId,
): AgentNativeScreenReference {
  const reference = agentNativeScreenReferences.find(
    (candidate) => candidate.id === id,
  );
  if (!reference) {
    throw new Error(`Unknown agent-native screen reference: ${id}`);
  }
  return reference;
}

export function screenReferencesForWorkflow(
  workflowId: AgentNativeWorkflowId,
): AgentNativeScreenReference[] {
  return getAgentNativeWorkflow(workflowId).screenIds.map((screenId) =>
    getAgentNativeScreenReference(screenId),
  );
}

export function workflowForSection(
  sectionId: StudioSectionId,
): AgentNativeWorkflow {
  const workflow = agentNativeWorkflows.find((candidate) =>
    candidate.sectionIds.includes(sectionId),
  );
  if (!workflow) {
    throw new Error(`Studio section is not assigned to a workflow: ${sectionId}`);
  }
  return workflow;
}

export function defaultSectionForWorkflow(
  workflowId: AgentNativeWorkflowId,
): StudioSectionId {
  return getAgentNativeWorkflow(workflowId).defaultSectionId;
}

export function isSectionInWorkflow(
  sectionId: StudioSectionId,
  workflowId: AgentNativeWorkflowId,
): boolean {
  return getAgentNativeWorkflow(workflowId).sectionIds.includes(sectionId);
}

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
